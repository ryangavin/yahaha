//! The mock's sound catalog (#117): what `src/session/sounds.rs` does, built from the
//! mock's fonts, plugins and sound library with the engine's own rules
//! (`yahaha::api::SoundPrefs`). No audio. The twin of `app/src/lib/api/mock-sounds.ts`.

use yahaha::api::*;
use yahaha::patches::PatchSource;

use super::sound::presets;

/// The made-up sampler whose AU presets the mock lists (as mock-plugins.ts): a user
/// preset folder from the start, factory presets once the browser expands it.
pub const MOCK_PRESETS_ID: &str = "aumu Smp7 Fake";

/// Its user presets' folder.
const MOCK_PRESET_DIR: &str = "/Users/mock/Library/Audio/Presets/Fake Instruments/Sampler Deluxe";

/// The catalog's settings, its revision and the audition.
pub struct MockSounds {
    prefs: SoundPrefs,
    audition: Option<(String, f64)>,
    /// What the catalog was last built from, and its revision.
    key: String,
    revision: u64,
    /// The plugins' presets (`listPluginPresets` adds the factory ones).
    presets: Vec<PluginPresetList>,
}

impl Default for MockSounds {
    fn default() -> MockSounds {
        let user = |name: &str, folder: Option<&str>| PluginPresetEntry {
            key: format!("u:{MOCK_PRESET_DIR}/{}{name}.aupreset", folder.map_or(String::new(), |f| format!("{f}/"))),
            name: name.into(),
            folder: folder.map(Into::into),
        };
        MockSounds {
            prefs: SoundPrefs::default(),
            audition: None,
            key: String::new(),
            revision: 0,
            presets: vec![PluginPresetList {
                plugin: MOCK_PRESETS_ID.into(),
                listed: false,
                presets: vec![user("Arco Strings", None), user("Upright Piano", Some("Pianos"))],
                error: None,
            }],
        }
    }
}

/// What a `SoundsCmd` asks of the rest of the mock.
pub enum Then {
    Nothing,
    Run(Vec<AppCmd>),
    /// Add a preset to the library, then give keyboard part `.1` the patch added.
    AddThenAssign(AppCmd, u8),
    /// Part `.0` plays the preset just saved (key `.1`, name `.2`).
    PresetSaved(u8, String, String),
}

impl MockSounds {
    fn fonts(st: &AppState) -> Vec<(String, Vec<Preset>)> {
        st.io.sound_fonts.iter().map(|f| (f.clone(), presets(f))).collect()
    }

    pub fn catalog(&self, st: &AppState) -> SoundCatalog {
        let fonts = Self::fonts(st);
        let fonts: Vec<(&str, &[Preset])> = fonts.iter().map(|(f, p)| (f.as_str(), p.as_slice())).collect();
        let patches: Vec<_> = st.sound_library.patches.iter().map(|p| p.patch.clone()).collect();
        SoundCatalog {
            revision: self.revision,
            entries: self.prefs.entries(&fonts, &st.plugins.list, &self.presets, &patches),
            recents: self.prefs.recents.clone(),
            fonts: font_summaries(&fonts),
        }
    }

    /// Preset `key` of plugin `id`, if the mock lists it.
    pub fn preset(&self, id: &str, key: &str) -> Option<&PluginPresetEntry> {
        self.presets.iter().find(|l| l.plugin == id)?.presets.iter().find(|p| p.key == key)
    }

    fn known(&self, st: &AppState, id: &str) -> bool {
        if let Some((file, bank, program)) = parse_preset_id(id) {
            st.io.sound_fonts.iter().any(|f| f == file) && presets(file).iter().any(|p| p.bank == bank && p.program == program)
        } else if let Some((plugin, preset)) = parse_plugin_id(id) {
            match preset {
                None => st.plugins.list.iter().any(|p| p.id == plugin),
                Some(key) => self.preset(plugin, key).is_some(),
            }
        } else if let Some(patch) = id.strip_prefix("saved:") {
            st.sound_library.patches.iter().any(|p| p.patch.id == patch)
        } else {
            false
        }
    }

    pub fn cmd(&mut self, st: &AppState, c: SoundsCmd) -> Result<Then, String> {
        let no = |id: &str| format!("no sound {id}");
        match c {
            SoundsCmd::SetSoundFavourite { id, on } => {
                if !self.known(st, &id) {
                    return Err(no(&id));
                }
                if let Some(patch) = id.strip_prefix("saved:") {
                    return Ok(Then::Run(vec![SoundLibraryCmd::SetPatchFavourite { id: patch.into(), favourite: on }.into()]));
                }
                if on {
                    self.prefs.favourites.insert(id);
                } else {
                    self.prefs.favourites.remove(&id);
                }
            }
            SoundsCmd::AuditionSound { id } => {
                if !self.known(st, &id) {
                    return Err(no(&id));
                }
                if st.transport.running {
                    return Err("Stop the band to audition a sound".into());
                }
                self.audition = Some((id, 3000.0));
            }
            SoundsCmd::StopSoundAudition => self.audition = None,
            SoundsCmd::AssignSound { part, id } => {
                if part > 3 {
                    return Err(format!("no keyboard part {part} (0-3)"));
                }
                if !self.known(st, &id) {
                    return Err(no(&id));
                }
                let then = if let Some(patch) = id.strip_prefix("saved:") {
                    Then::Run(vec![SoundLibraryCmd::SetPartPatch { part, id: Some(patch.into()) }.into()])
                } else if let Some((plugin, preset)) = parse_plugin_id(&id) {
                    match preset {
                        Some(key) => Then::Run(vec![PluginCmd::SetPartPluginPreset { part, id: plugin.into(), preset: key.into() }.into()]),
                        None => Then::Run(vec![PluginCmd::SetPartPlugin { part, id: plugin.into(), state: None }.into()]),
                    }
                } else {
                    let (file, bank, program) = parse_preset_id(&id).ok_or_else(|| no(&id))?;
                    if st.io.sound_font_file.as_deref() == Some(file) && bank == 0 {
                        Then::Run(vec![PartsCmd::SetPartVoice { part, program }.into()])
                    } else {
                        let existing = st.sound_library.patches.iter().find(|p| {
                            matches!(&p.patch.source, PatchSource::SoundFont { file: f, bank: b, program: q } if f == file && *b == bank && *q == program)
                        });
                        match existing {
                            Some(p) => Then::Run(vec![SoundLibraryCmd::SetPartPatch { part, id: Some(p.patch.id.clone()) }.into()]),
                            None => Then::AddThenAssign(SoundLibraryCmd::AddPresetAsPatch { file: file.into(), bank, program, name: None }.into(), part),
                        }
                    }
                };
                self.prefs.push_recent(id);
                return Ok(then);
            }
            SoundsCmd::SetSoundCategory { id, category } => {
                if !self.known(st, &id) {
                    return Err(no(&id));
                }
                if let Some(patch) = id.strip_prefix("saved:") {
                    let p = st.sound_library.patches.iter().find(|p| p.patch.id == patch).map(|p| p.patch.clone()).ok_or_else(|| no(&id))?;
                    let fields = PatchFields { name: p.name, category, tags: p.tags, favourite: p.favourite, source: p.source };
                    return Ok(Then::Run(vec![SoundLibraryCmd::UpdatePatch { id: p.id, patch: fields }.into()]));
                }
                if !id.starts_with("au:") {
                    return Err("a preset's category is its GM family".into());
                }
                self.prefs.sound_categories.insert(id, category);
            }
            // The mock itself adds the patch (`MockSession::rule_patch`), as a map rule's.
            // The session mock does these itself (mock.rs `sounds_cmd`).
            SoundsCmd::AddToMySounds { .. } | SoundsCmd::ReplacePartSound { .. } => {}
            SoundsCmd::ListPluginPresets { id } => {
                let Some((plugin, None)) = parse_plugin_id(&id) else { return Err(format!("{id} is not a plugin")) };
                let Some(entry) = st.plugins.list.iter().find(|p| p.id == plugin) else {
                    return Err(format!("no instrument Audio Unit {plugin} is installed"));
                };
                let at = match self.presets.iter().position(|l| l.plugin == plugin) {
                    Some(i) => i,
                    None => {
                        self.presets.push(PluginPresetList { plugin: plugin.into(), listed: false, presets: vec![], error: None });
                        self.presets.len() - 1
                    }
                };
                let l = &mut self.presets[at];
                if !l.listed && l.error.is_none() && let Some(e) = &entry.last_error {
                    // A plugin that does not load cannot list its presets (the engine's
                    // listing fails the same way): the browser stops waiting.
                    l.error = Some(e.clone());
                } else if !l.listed && l.error.is_none() {
                    l.listed = true;
                    if plugin == MOCK_PRESETS_ID {
                        let factory = ["Init", "Bright Grand", "Brass Stabs"]
                            .iter()
                            .enumerate()
                            .map(|(n, name)| PluginPresetEntry { key: format!("f:{n}"), name: (*name).into(), folder: None });
                        l.presets.splice(0..0, factory);
                    }
                }
            }
            SoundsCmd::SavePartAsPluginPreset { part, name, category, overwrite } => {
                if part > 3 {
                    return Err(format!("no keyboard part {part} (0-3)"));
                }
                let name = name.trim();
                if name.is_empty() {
                    return Err("name the preset".into());
                }
                let kp = &st.keyboard_parts[part as usize];
                let Some(pl) = kp.plugin.as_ref().filter(|p| p.status == PluginStatus::Playing) else {
                    return Err("the part plays its SoundFont voice, not a plugin".into());
                };
                let file: String = name.chars().map(|c| if matches!(c, '/' | ':') { '-' } else { c }).collect();
                let key = format!("u:/Users/mock/Library/Audio/Presets/{}/{}/{file}.aupreset", pl.manufacturer, pl.name);
                let at = match self.presets.iter().position(|l| l.plugin == pl.id) {
                    Some(i) => i,
                    None => {
                        self.presets.push(PluginPresetList { plugin: pl.id.clone(), listed: false, presets: vec![], error: None });
                        self.presets.len() - 1
                    }
                };
                let list = &mut self.presets[at].presets;
                if !overwrite && list.iter().any(|p| p.key == key) {
                    return Err(format!("a preset called {file} already exists: save under another name, or replace it"));
                }
                list.retain(|p| p.key != key);
                list.push(PluginPresetEntry { key: key.clone(), name: file.clone(), folder: None });
                self.prefs.sound_categories.insert(plugin_preset_id(&pl.id, &key), category);
                return Ok(Then::PresetSaved(part, key, file));
            }
        }
        Ok(Then::Nothing)
    }

    /// The library patch a program map rule gets for catalog entry `id` (#117): a saved
    /// sound's own or the library's patch for the preset or plugin (`Ok(Ok(id))`), else
    /// the command that adds it (`Ok(Err(cmd))`; the patch is then `lastAdded`). An id
    /// without a catalog prefix is a patch id already.
    pub fn patch_for(&self, st: &AppState, id: &str) -> Result<Result<String, AppCmd>, String> {
        if let Some(patch) = id.strip_prefix("saved:") {
            return Ok(Ok(patch.into()));
        }
        let source = if let Some((file, bank, program)) = parse_preset_id(id) {
            PatchSource::SoundFont { file: file.into(), bank, program }
        } else if let Some((plugin, preset)) = parse_plugin_id(id) {
            // The one plugin sound for the preset (its origin finds it again): a user
            // preset's keeps the file's settings (the mock has none to read); a factory
            // preset's state is captured when it plays, so it starts empty.
            let origin = preset.and_then(yahaha::patches::PluginOrigin::from_preset_key).unwrap_or_default();
            let state = match &origin {
                yahaha::patches::PluginOrigin::File { .. } => "bW9jaw==".to_string(),
                _ => String::new(),
            };
            PatchSource::Plugin { component_id: plugin.into(), state, origin }
        } else {
            return Ok(Ok(id.into()));
        };
        if !self.known(st, id) {
            return Err(format!("no sound {id}"));
        }
        let same = |p: &&PatchInfo| match &source {
            // Or the sound with exactly its settings (Save as… with an .aupreset), as the
            // session's.
            PatchSource::Plugin { component_id, origin, state } if !origin.is_user() => {
                p.patch.source.same_plugin_origin(component_id, origin)
                    || (!state.is_empty() && matches!(&p.patch.source, PatchSource::Plugin { component_id: c, state: s, .. } if c == component_id && s == state))
            }
            _ => p.patch.source == source,
        };
        if let Some(p) = st.sound_library.patches.iter().find(same) {
            return Ok(Ok(p.patch.id.clone()));
        }
        Ok(Err(match source {
            PatchSource::SoundFont { file, bank, program } => SoundLibraryCmd::AddPresetAsPatch { file, bank, program, name: None }.into(),
            source @ PatchSource::Plugin { .. } => {
                let (plugin, preset) = parse_plugin_id(id).ok_or_else(|| format!("no sound {id}"))?;
                let e = st.plugins.list.iter().find(|p| p.id == plugin).ok_or_else(|| format!("no sound {id}"))?;
                let pid = format!("au:{plugin}");
                let mut category = self.prefs.plugin_category(&pid, &e.name, &e.manufacturer);
                let mut name = e.name.clone();
                if let Some(q) = preset.and_then(|k| self.preset(plugin, k)) {
                    category = self.prefs.preset_category(id, &q.name, q.folder.as_deref(), category);
                    name = format!("{} · {}", e.name, q.name);
                }
                let patch = PatchFields { name, category, tags: vec![], favourite: false, source };
                SoundLibraryCmd::CreatePatch { patch }.into()
            }
        }))
    }

    pub fn advance(&mut self, ms: f64, running: bool) {
        if let Some((_, left)) = self.audition.as_mut() {
            *left -= ms;
            if *left <= 0.0 || running {
                self.audition = None;
            }
        }
    }

    /// `state.sounds`: a new revision whenever what the catalog is built from changed.
    pub fn derive(&mut self, st: &mut AppState) {
        let patches: Vec<_> = st.sound_library.patches.iter().map(|p| &p.patch).collect();
        let key = serde_json::to_string(&(&st.io.sound_fonts, &st.io.sound_font_file, &st.plugins.list, patches, &self.prefs, &self.presets)).unwrap_or_default();
        if key != self.key || self.revision == 0 {
            self.key = key;
            self.revision += 1;
        }
        let presets: usize = st.io.sound_fonts.iter().map(|f| presets(f).len()).sum();
        let plugin_presets: usize = self.presets.iter().filter(|l| st.plugins.list.iter().any(|p| p.id == l.plugin)).map(|l| l.presets.len()).sum();
        st.sounds = SoundsState {
            revision: self.revision,
            count: (presets + st.plugins.list.len() + plugin_presets + st.sound_library.patches.len()) as u32,
            scanning: st.plugins.scanning,
            auditioning: self.audition.as_ref().map(|(id, _)| id.clone()),
            // The mock lists at once.
            listing_presets: Vec::new(),
        };
    }
}

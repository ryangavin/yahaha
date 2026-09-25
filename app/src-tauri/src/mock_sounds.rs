//! The mock's sound catalog (#117): what `src/session/sounds.rs` does, built from the
//! mock's fonts, plugins and sound library with the engine's own rules
//! (`yahaha::api::SoundPrefs`). No audio. The twin of `app/src/lib/api/mock-sounds.ts`.

use yahaha::api::*;
use yahaha::patches::PatchSource;

use super::sound::presets;

/// The catalog's settings, its revision and the audition.
#[derive(Default)]
pub struct MockSounds {
    prefs: SoundPrefs,
    audition: Option<(String, f64)>,
    /// What the catalog was last built from, and its revision.
    key: String,
    revision: u64,
}

/// What a `SoundsCmd` asks of the rest of the mock.
pub enum Then {
    Nothing,
    Run(Vec<AppCmd>),
    /// Add a preset to the library, then give keyboard part `.1` the patch added.
    AddThenAssign(AppCmd, u8),
}

impl MockSounds {
    fn fonts(st: &AppState) -> Vec<(String, Vec<Preset>)> {
        st.io.sound_fonts.iter().map(|f| (f.clone(), presets(f))).collect()
    }

    pub fn catalog(&self, st: &AppState) -> SoundCatalog {
        let fonts = Self::fonts(st);
        let fonts: Vec<(&str, &[Preset])> = fonts.iter().map(|(f, p)| (f.as_str(), p.as_slice())).collect();
        let patches: Vec<_> = st.sound_library.patches.iter().map(|p| p.patch.clone()).collect();
        SoundCatalog { revision: self.revision, entries: self.prefs.entries(&fonts, &st.plugins.list, &patches), recents: self.prefs.recents.clone() }
    }

    fn known(&self, st: &AppState, id: &str) -> bool {
        if let Some((file, bank, program)) = parse_preset_id(id) {
            st.io.sound_fonts.iter().any(|f| f == file) && presets(file).iter().any(|p| p.bank == bank && p.program == program)
        } else if let Some(plugin) = id.strip_prefix("au:") {
            st.plugins.list.iter().any(|p| p.id == plugin)
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
                } else if let Some(plugin) = id.strip_prefix("au:") {
                    Then::Run(vec![PluginCmd::SetPartPlugin { part, id: plugin.into(), state: None }.into()])
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
                    let fields = PatchFields { name: p.name, category, tags: p.tags, favourite: p.favourite, source: p.source, defaults: p.defaults };
                    return Ok(Then::Run(vec![SoundLibraryCmd::UpdatePatch { id: p.id, patch: fields }.into()]));
                }
                if !id.starts_with("au:") {
                    return Err("a preset's category is its GM family".into());
                }
                self.prefs.sound_categories.insert(id, category);
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
        } else if let Some(plugin) = id.strip_prefix("au:") {
            PatchSource::Plugin { component_id: plugin.into(), state: String::new() }
        } else {
            return Ok(Ok(id.into()));
        };
        if !self.known(st, id) {
            return Err(format!("no sound {id}"));
        }
        if let Some(p) = st.sound_library.patches.iter().find(|p| p.patch.source == source) {
            return Ok(Ok(p.patch.id.clone()));
        }
        Ok(Err(match source {
            PatchSource::SoundFont { file, bank, program } => SoundLibraryCmd::AddPresetAsPatch { file, bank, program, name: None }.into(),
            source @ PatchSource::Plugin { .. } => {
                let e = st.plugins.list.iter().find(|p| format!("au:{}", p.id) == id).ok_or_else(|| format!("no sound {id}"))?;
                let category = self.prefs.plugin_category(id, &e.name, &e.manufacturer);
                let patch = PatchFields { name: e.name.clone(), category, tags: vec![], favourite: false, source, defaults: Default::default() };
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
        let key = serde_json::to_string(&(&st.io.sound_fonts, &st.io.sound_font_file, &st.plugins.list, patches, &self.prefs)).unwrap_or_default();
        if key != self.key || self.revision == 0 {
            self.key = key;
            self.revision += 1;
        }
        let presets: usize = st.io.sound_fonts.iter().map(|f| presets(f).len()).sum();
        st.sounds = SoundsState {
            revision: self.revision,
            count: (presets + st.plugins.list.len() + st.sound_library.patches.len()) as u32,
            scanning: st.plugins.scanning,
            auditioning: self.audition.as_ref().map(|(id, _)| id.clone()),
        };
    }
}

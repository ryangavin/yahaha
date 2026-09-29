//! The mock's sound library (#103): what `src/session/sound_library.rs` does, closely
//! enough for the Sound Library drawer, with the engine's own rules (`yahaha::patches`).
//! No audio. The twin of `app/src/lib/api/mock-sound-library.ts`.

use std::collections::BTreeMap;
use yahaha::api::*;
use yahaha::patches::{self, AutoFill, Category, Patch, PatchDefaults, PatchSource, PluginOrigin, ProgramMap, SoundLibrary, SoundTag};

const SF2: &str = "GeneralUser-GS.sf2";
const FONTS: [&str; 2] = [SF2, "FluidR3_GM.sf2"];

fn sf(id: &str, name: &str, bank: u16, program: u8) -> Patch {
    Patch {
        id: id.into(),
        name: name.into(),
        category: Category::guess(bank, program),
        tags: vec![],
        favourite: false,
        source: PatchSource::SoundFont { file: SF2.into(), bank, program },
        defaults: PatchDefaults::default(),
    }
}

/// The mock's library: a few patches, a small map, the parts' own patches, the style maps.
pub struct MockSound {
    patches: Vec<Patch>,
    map: ProgramMap,
    style_maps: BTreeMap<String, ProgramMap>,
    pub parts: [Option<String>; 4],
    /// The plugin patch whose plugin each part was given (`part_plugins`).
    plugin_parts: [Option<String>; 4],
    port: bool,
    audition: Option<(String, f64)>,
    browse: Option<SoundFontBrowse>,
    last_added: Option<String>,
    /// The Sound a Plugins-tab plugin plays on each part (a preset's, or Save as…'s).
    pub plugin_sound: [Option<SoundTag>; 4],
    /// The mock plugin window's one knob on each part, and where its sound left it: the
    /// part is edited while they differ (the session compares state fingerprints, O3).
    knob: [i32; 4],
    saved_knob: [i32; 4],
    /// The `.aupreset` names `exportSoundPreset` wrote (a second export of one needs
    /// `overwrite`, as the real preset folder does).
    exported: Vec<String>,
}

impl Default for MockSound {
    fn default() -> MockSound {
        let mut patches = vec![
            sf("stage-grand", "Stage Grand", 0, 0),
            sf("warm-rhodes", "Warm Rhodes", 0, 4),
            sf("finger-bass", "Finger Bass", 0, 33),
            sf("studio-kit", "Studio Kit", 128, 0),
            sf("silk-strings", "Silk Strings", 0, 48),
            sf("brass-section", "Brass Section", 0, 61),
            sf("soft-pad", "Soft Pad", 0, 89),
            Patch {
                source: PatchSource::plugin("aumu dls  appl", String::new()),
                category: Category::EPiano,
                ..sf("keys-au", "Keys (AU)", 0, 4)
            },
        ];
        patches[0].favourite = true;
        patches[0].tags = vec!["bright".into()];
        patches[1].favourite = true;
        patches[1].defaults = PatchDefaults { volume: Some(96), pan: None, reverb: Some(40), chorus: Some(30), octave: 0 };
        patches[2].defaults = PatchDefaults { volume: Some(100), pan: Some(64), reverb: Some(10), chorus: None, octave: 0 };
        patches[4].tags = vec!["warm".into()];
        let mut map = ProgramMap::default();
        for (f, id) in [(0, "stage-grand"), (4, "finger-bass"), (5, "silk-strings"), (6, "silk-strings"), (7, "brass-section"), (11, "soft-pad")] {
            map.set_family(f, Some(id.into()));
        }
        map.set_override(4, Some("warm-rhodes".into()));
        map.set_override(5, Some("warm-rhodes".into()));
        map.drums = Some("studio-kit".into());
        MockSound { patches, map, style_maps: BTreeMap::new(), parts: Default::default(), plugin_parts: Default::default(), port: false, audition: None, browse: None, last_added: None, plugin_sound: Default::default(), knob: [0; 4], saved_knob: [0; 4], exported: Vec::new() }
    }
}

/// A mock font's presets: the GM set on bank 0; GeneralUser also has kits (so it is the
/// most GM-complete: the main font, and the auto-fill's first choice).
pub fn presets(file: &str) -> Vec<Preset> {
    let gm: Vec<Preset> = (0..128u8).map(|p| Preset { bank: 0, program: p, name: format!("{}{}", gm_name(p), if file == SF2 { "" } else { " (Fluid)" }) }).collect();
    let kits = ["Standard", "Room", "Power", "Electronic", "Jazz", "Brush"].iter().enumerate().map(|(i, n)| Preset { bank: 128, program: i as u8 * 8, name: n.to_string() });
    let kits: Vec<Preset> = if file == SF2 { kits.collect() } else { Vec::new() };
    gm.into_iter().chain(kits).collect()
}

/// The GM map's auto-fill from the mock's fonts (D4), as the session builds it.
fn auto_fill() -> AutoFill {
    AutoFill::build(&FONTS.map(|f| (f.to_string(), presets(f))))
}

impl MockSound {
    fn has(&self, id: &Option<String>) -> bool {
        id.as_ref().is_none_or(|id| self.patches.iter().any(|p| &p.id == id))
    }

    fn at(&self, id: &str) -> Option<usize> {
        self.patches.iter().position(|p| p.id == id)
    }

    fn add(&mut self, mut p: Patch) {
        p.id = patches::new_id(&p.name, self.patches.iter().map(|q| q.id.as_str()));
        self.last_added = Some(p.id.clone());
        self.patches.push(p);
    }

    /// A GM voice was picked for a part: its own patch goes.
    pub fn part_voice(&mut self, part: usize) {
        if let Some(p) = self.parts.get_mut(part) {
            *p = None;
        }
    }

    /// A plugin picked (or, while a plugin patch plays, cleared) on the Plugins tab: the
    /// part's own patch goes.
    pub fn part_plugin(&mut self, part: usize, picked: bool) {
        let p = part & 3;
        if picked || self.plugin_parts[p].is_some() {
            self.plugin_parts[p] = None;
            self.parts[p] = None;
        }
        self.plugin_sound[p] = None;
        (self.knob[p], self.saved_knob[p]) = (0, 0);
    }

    /// A plugin preset picked on part `part`: its one library sound (added once, found
    /// again by its origin), as the session's `link_voice_sound`.
    pub fn preset_sound(&mut self, part: usize, component: &str, key: &str, name: &str) {
        let Some(origin) = PluginOrigin::from_preset_key(key) else { return };
        let i = match self.patches.iter().position(|p| p.source.same_plugin_origin(component, &origin)) {
            Some(i) => i,
            None => {
                let source = PatchSource::Plugin { component_id: component.into(), state: String::new(), origin };
                self.add(Patch { id: String::new(), name: name.into(), category: Category::SynthLead, tags: vec![], favourite: false, source, defaults: PatchDefaults::default() });
                self.patches.len() - 1
            }
        };
        self.plugin_sound[part & 3] = Some(self.patches[i].tag());
    }

    /// The mock plugin window turned its knob to `value`: the part shows as edited at once
    /// unless that is where its sound left it (as the session's reads while a window is open).
    pub fn plugin_window(&mut self, part: usize, value: i32) {
        self.knob[part & 3] = value;
    }

    /// The demo window's edit: the knob one step off its sound's value, or back onto it.
    pub fn plugin_window_demo(&self, part: usize) -> i32 {
        let p = part & 3;
        if self.knob[p] == self.saved_knob[p] { self.saved_knob[p] + 1 } else { self.saved_knob[p] }
    }

    /// The library sound part `p` plays through its plugin, if any.
    fn plugin_sound_id(&self, p: usize) -> Option<String> {
        self.plugin_parts[p].clone().or_else(|| self.plugin_sound[p].as_ref().map(|t| t.id.strip_prefix("saved:").unwrap_or(&t.id).to_string()))
    }

    /// Whether part `part` plays a plugin the Plugins tab picked (not a plugin patch's).
    pub fn own_plugin(&self, part: usize) -> bool {
        self.plugin_parts[part & 3].is_none()
    }

    /// The parts' plugins to change for their own plugin patches, as the session's
    /// `sync_part_plugins` does: (part, Some((component id, state)) to load, None to clear).
    pub fn part_plugins(&mut self) -> Vec<(usize, Option<(String, String)>)> {
        let mut out = Vec::new();
        for p in 0..4 {
            let want = self.parts[p].as_ref().and_then(|id| self.at(id)).and_then(|i| match &self.patches[i].source {
                PatchSource::Plugin { component_id, state, .. } => Some((self.patches[i].id.clone(), (component_id.clone(), state.clone()))),
                PatchSource::SoundFont { .. } => None,
            });
            if want.as_ref().map(|w| &w.0) == self.plugin_parts[p].as_ref() {
                continue;
            }
            let had = self.plugin_parts[p].take().is_some();
            match want {
                Some((id, voice)) => {
                    self.plugin_parts[p] = Some(id);
                    out.push((p, Some(voice)));
                }
                None if had => out.push((p, None)),
                None => {}
            }
        }
        out
    }

    fn map_mut(&mut self, style: bool, key: &str) -> &mut ProgramMap {
        if style { self.style_maps.entry(key.to_string()).or_default() } else { &mut self.map }
    }

    /// Runs a command on `st`; an error message if it is refused.
    pub fn cmd(&mut self, st: &mut AppState, c: SoundLibraryCmd) -> Option<String> {
        let nope = |id: &str| Some(format!("no patch {id} in the sound library"));
        let key = st.sound_library.style_key.clone();
        let fields = |id: String, f: PatchFields| Patch { id, name: f.name, category: f.category, tags: f.tags, favourite: f.favourite, source: f.source, defaults: f.defaults.clamped() };
        match c {
            SoundLibraryCmd::CreatePatch { patch } => self.add(fields(String::new(), patch)),
            SoundLibraryCmd::UpdatePatch { id, patch } => {
                let Some(i) = self.at(&id) else { return nope(&id) };
                let name = if patch.name.trim().is_empty() { self.patches[i].name.clone() } else { patch.name.clone() };
                self.patches[i] = Patch { name, ..fields(id, patch) };
            }
            SoundLibraryCmd::DeletePatch { id } => {
                let Some(i) = self.at(&id) else { return nope(&id) };
                self.patches.remove(i);
                self.map.forget(&id);
                for m in self.style_maps.values_mut() {
                    m.forget(&id);
                }
                for p in &mut self.parts {
                    if p.as_deref() == Some(id.as_str()) {
                        *p = None;
                    }
                }
            }
            SoundLibraryCmd::DuplicatePatch { id } => {
                let Some(i) = self.at(&id) else { return nope(&id) };
                let mut p = self.patches[i].clone();
                p.name = format!("{} copy", p.name);
                p.id = patches::new_id(&p.name, self.patches.iter().map(|q| q.id.as_str()));
                self.last_added = Some(p.id.clone());
                self.patches.insert(i + 1, p);
            }
            SoundLibraryCmd::MovePatch { id, to } => {
                let Some(i) = self.at(&id) else { return nope(&id) };
                let p = self.patches.remove(i);
                let to = (to as usize).min(self.patches.len());
                self.patches.insert(to, p);
            }
            SoundLibraryCmd::SetPatchFavourite { id, favourite } => {
                let Some(i) = self.at(&id) else { return nope(&id) };
                self.patches[i].favourite = favourite;
            }
            SoundLibraryCmd::SaveSound { part } => {
                // Save (O3): over the user's own sound it plays, else Save as….
                let p = (part & 3) as usize;
                let kp = &st.keyboard_parts[p];
                let current = if kp.plugin.is_some() { self.plugin_sound_id(p) } else { self.parts[p].clone() };
                let own = current.as_deref().and_then(|id| self.at(id)).filter(|&i| match &self.patches[i].source {
                    PatchSource::Plugin { component_id, origin, .. } => origin.is_user() && kp.plugin.as_ref().is_some_and(|q| q.id == *component_id),
                    PatchSource::SoundFont { .. } => kp.plugin.is_none(),
                });
                let Some(i) = own else { return self.cmd(st, SoundLibraryCmd::SaveSoundAs { part, name: None }) };
                let (volume, octave) = (kp.volume, kp.octave);
                let q = &mut self.patches[i];
                q.defaults.volume = Some(volume);
                q.defaults.octave = octave;
                self.saved_knob[p] = self.knob[p];
            }
            SoundLibraryCmd::SaveSoundAs { part, name } | SoundLibraryCmd::SavePartAsPatch { part, name } => {
                // What the part plays: its plugin, else its own patch, else the patch the
                // map sends its GM voice to, else its GM voice (as the session's).
                let kp = &st.keyboard_parts[(part & 3) as usize];
                let key = st.style.path.rsplit('/').next().unwrap_or_default().to_string();
                let own = self.parts[(part & 3) as usize].clone().filter(|_| !kp.plays_bass);
                let plays = own
                    .or_else(|| patches::resolve(&self.map, self.style_maps.get(&key), false, kp.program).patch.map(str::to_string))
                    .and_then(|id| self.at(&id))
                    .map(|i| self.patches[i].clone());
                let plugin = kp.plugin.as_ref().filter(|p| p.status != PluginStatus::Failed).map(|p| {
                    let source = PatchSource::plugin(p.id.clone(), String::new());
                    match plays.clone() {
                        Some(q) if matches!(&q.source, PatchSource::Plugin { component_id, .. } if *component_id == p.id) => Patch { source, ..q },
                        q => Patch {
                            id: String::new(),
                            name: p.name.clone(),
                            category: q.map_or_else(|| Category::guess(0, kp.program), |q| q.category),
                            tags: vec![],
                            favourite: false,
                            source,
                            defaults: PatchDefaults::default(),
                        },
                    }
                });
                let mut p = plugin.or(plays).unwrap_or_else(|| Patch {
                    id: String::new(),
                    name: gm_name(kp.program).into(),
                    category: Category::guess(0, kp.program),
                    tags: vec![],
                    favourite: false,
                    source: PatchSource::SoundFont { file: SF2.into(), bank: 0, program: kp.program },
                    defaults: PatchDefaults::default(),
                });
                p.defaults.volume = Some(kp.volume);
                p.defaults.octave = kp.octave;
                if let Some(n) = name.filter(|n| !n.trim().is_empty()) {
                    p.name = n;
                }
                let plugin = matches!(p.source, PatchSource::Plugin { .. });
                self.add(p);
                // A part playing a plugin plays the new sound, not edited (O3).
                let i = (part & 3) as usize;
                if plugin && let Some(new) = self.patches.last() {
                    if self.plugin_parts[i].is_some() {
                        self.parts[i] = Some(new.id.clone());
                        self.plugin_parts[i] = Some(new.id.clone());
                    } else {
                        self.plugin_sound[i] = Some(new.tag());
                    }
                    self.saved_knob[i] = self.knob[i];
                }
            }
            SoundLibraryCmd::AddPresetAsPatch { file, bank, program, name } => {
                if !FONTS.contains(&file.as_str()) {
                    return Some(format!("no SoundFont {file} in the SoundFont folder"));
                }
                let name = name.filter(|n| !n.trim().is_empty()).unwrap_or_else(|| {
                    presets(&file).into_iter().find(|p| p.bank == bank && p.program == program).map_or_else(|| format!("{file} {bank}:{}", program + 1), |p| p.name)
                });
                self.add(Patch { id: String::new(), name, category: Category::guess(bank, program), tags: vec![], favourite: false, source: PatchSource::SoundFont { file, bank, program }, defaults: PatchDefaults::default() });
            }
            SoundLibraryCmd::AuditionPatch { id } => {
                if st.transport.running {
                    return Some("Stop the band to audition a sound".into());
                }
                let Some(i) = self.at(&id) else { return nope(&id) };
                if let Some(why) = patches::unavailable_reason(&self.patches[i], &FONTS.map(String::from)) {
                    return Some(format!("{}: {why}", self.patches[i].name));
                }
                self.audition = Some((id, 3000.0));
            }
            SoundLibraryCmd::AuditionPreset { .. } => {
                if st.transport.running {
                    return Some("Stop the band to audition a sound".into());
                }
                self.audition = Some(("preset".into(), 3000.0));
            }
            SoundLibraryCmd::StopPatchAudition => self.audition = None,
            SoundLibraryCmd::SetFamilyRule { family, patch, style } => {
                if family >= 16 {
                    return Some(format!("no GM family {family} (0-15)"));
                }
                if !self.has(&patch) {
                    return nope(patch.as_deref().unwrap_or(""));
                }
                self.map_mut(style, &key).set_family(family as usize, patch);
            }
            SoundLibraryCmd::SetProgramOverride { program, patch, style } => {
                if !self.has(&patch) {
                    return nope(patch.as_deref().unwrap_or(""));
                }
                self.map_mut(style, &key).set_override(program, patch);
            }
            SoundLibraryCmd::SetDrumRule { patch, style } => {
                if !self.has(&patch) {
                    return nope(patch.as_deref().unwrap_or(""));
                }
                self.map_mut(style, &key).drums = patch;
            }
            SoundLibraryCmd::ClearStyleMap => {
                self.style_maps.remove(&key);
            }
            SoundLibraryCmd::SetPartPatch { part, id } => {
                if !self.has(&id) {
                    return nope(id.as_deref().unwrap_or(""));
                }
                let p = (part & 3) as usize;
                if let Some(d) = id.as_ref().and_then(|i| self.at(i)).map(|i| self.patches[i].defaults) {
                    let kp = &mut st.keyboard_parts[p];
                    if let Some(v) = d.volume {
                        kp.volume = v;
                    }
                    kp.octave = d.octave;
                }
                self.parts[p] = id;
            }
            SoundLibraryCmd::SetPortSendsMapped { on } => self.port = on,
            SoundLibraryCmd::BrowseSoundFont { file } => {
                self.browse = match file {
                    None => None,
                    Some(f) if FONTS.contains(&f.as_str()) => Some(SoundFontBrowse { presets: presets(&f), file: f, error: None }),
                    Some(f) => return Some(format!("no SoundFont {f} in the SoundFont folder")),
                }
            }
            SoundLibraryCmd::ImportSoundLibrary { path, .. } => return Some(format!("{path}: the mock has no files to import")),
            SoundLibraryCmd::ExportSoundLibrary { .. } => {}
            SoundLibraryCmd::ExportSoundPreset { id, overwrite } => {
                let Some(i) = self.at(&id) else { return nope(&id) };
                let p = &self.patches[i];
                match &p.source {
                    PatchSource::Plugin { state, .. } if state.is_empty() => return Some(format!("{} has no settings yet: play it once first", p.name)),
                    PatchSource::Plugin { .. } => {}
                    PatchSource::SoundFont { .. } => return Some(format!("{} is a SoundFont preset, not a plugin sound", p.name)),
                }
                if self.exported.contains(&p.name) && !overwrite {
                    return Some(format!("a preset called {} already exists: save under another name, or replace it", p.name));
                }
                if !self.exported.contains(&p.name) {
                    self.exported.push(p.name.clone());
                }
            }
        }
        None
    }

    /// Time passes: an audition ends by itself, and not while the band plays.
    pub fn advance(&mut self, ms: f64, running: bool) {
        if let Some((_, left)) = self.audition.as_mut() {
            *left -= ms;
            if *left <= 0.0 || running {
                self.audition = None;
            }
        }
    }

    /// The state, and the keyboard parts' patch and voice names.
    pub fn derive(&self, st: &mut AppState, gm: &[String]) {
        let key = st.style.path.rsplit('/').next().unwrap_or_default().to_string();
        let style = self.style_maps.get(&key);
        let name = |id: Option<&str>| id.and_then(|id| self.patches.iter().find(|p| p.id == id)).map(|p| p.name.clone());
        let usage = st
            .mixer
            .style_parts
            .iter()
            .enumerate()
            .filter_map(|(i, p)| {
                let v = p.voice.as_ref()?;
                let drums = i < 2 || v.kit;
                let r = patches::resolve(&self.map, style, drums, v.program);
                Some(ProgramUse {
                    channel: p.channel,
                    part: p.name.clone(),
                    msb: v.bank_msb,
                    lsb: v.bank_lsb,
                    program: v.program,
                    gm_program: v.program,
                    voice: v.label.clone(),
                    drums,
                    patch: r.patch.map(str::to_string),
                    rule: r.rule,
                    from_style: r.from_style,
                    plays: name(r.patch).unwrap_or_else(|| v.label.clone()),
                })
            })
            .collect();
        let tag = |id: Option<&str>| id.and_then(|id| self.patches.iter().find(|p| p.id == id)).map(Patch::tag);
        for (i, p) in st.keyboard_parts.iter_mut().enumerate() {
            p.patch = self.parts[i].clone();
            // Now playing (O3): the plugin's sound, else its own or the map's patch.
            (p.sound, p.sound_edited) = if p.plugin.is_some() {
                let s = match &self.plugin_parts[i] {
                    Some(id) => tag(Some(id)),
                    None => self.plugin_sound[i].clone(),
                };
                let edited = self.knob[i] != self.saved_knob[i] && s.is_some();
                (s, edited)
            } else {
                let own = self.parts[i].clone().filter(|_| !p.plays_bass);
                (tag(own.as_deref().or(patches::resolve(&self.map, style, false, p.program).patch)), false)
            };
            if p.plays_bass {
                continue;
            }
            let own = name(self.parts[i].as_deref());
            let mapped = name(patches::resolve(&self.map, style, false, p.program).patch);
            p.voice_name = own.or(mapped).unwrap_or_else(|| gm[p.program as usize].clone());
        }
        let fonts = FONTS.map(String::from);
        // The GM map for the style playing, through the engine's own resolution.
        let lib = SoundLibrary { patches: self.patches.clone(), map: self.map.clone(), style_maps: self.style_maps.clone(), ..SoundLibrary::default() };
        let gm_map = patches::gm_map_rows(&lib, style.map(|_| key.as_str()), &auto_fill());
        st.sound_library = SoundLibraryState {
            patches: self
                .patches
                .iter()
                .map(|p| {
                    let note = patches::unavailable_reason(p, &fonts);
                    PatchInfo { patch: p.clone(), available: note.is_none(), note }
                })
                .collect(),
            categories: Category::ALL.iter().map(|&c| CategoryInfo { id: c, label: c.label().into() }).collect(),
            families: patches::FAMILY_NAMES.iter().map(|s| s.to_string()).collect(),
            map: self.map.clone(),
            style_map: style.cloned().unwrap_or_default(),
            style_key: key,
            usage,
            port_sends_mapped: self.port,
            auditioning: self.audition.as_ref().map(|a| a.0.clone()),
            browse: self.browse.clone(),
            file: Some("/Users/me/Documents/yahaha/sound-library.json".into()),
            extra_sound_fonts: vec![],
            last_added: self.last_added.clone(),
            gm_map,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `exportSoundPreset` as the engine: a plugin sound with a state exports, a second
    /// export of the name needs `overwrite` (#307); no state, a SoundFont sound or an
    /// unknown id are refused.
    #[test]
    fn a_plugin_sound_exports_as_an_aupreset() {
        let (mut m, mut st) = (MockSound::default(), AppState::default());
        let export = |m: &mut MockSound, st: &mut AppState, id: &str, overwrite: bool| m.cmd(st, SoundLibraryCmd::ExportSoundPreset { id: id.into(), overwrite });
        assert!(export(&mut m, &mut st, "keys-au", false).unwrap().contains("no settings yet"));
        assert!(export(&mut m, &mut st, "stage-grand", false).unwrap().contains("not a plugin sound"));
        assert!(export(&mut m, &mut st, "nope", false).is_some());
        let i = m.at("keys-au").unwrap();
        m.patches[i].source = PatchSource::plugin("aumu Smp7 Fake", "c2FtcGxlciBkZWx1eGU=");
        assert_eq!(export(&mut m, &mut st, "keys-au", false), None);
        assert!(export(&mut m, &mut st, "keys-au", false).unwrap().contains("already exists"));
        assert_eq!(export(&mut m, &mut st, "keys-au", true), None);
    }
}

//! The mock's sound library (#103): what `src/session/sound_library.rs` does, closely
//! enough for the Sound Library drawer, with the engine's own rules (`yahaha::patches`).
//! No audio. The twin of `app/src/lib/api/mock-sound-library.ts`.

use std::collections::BTreeMap;
use yahaha::api::*;
use yahaha::patches::{self, Category, Patch, PatchDefaults, PatchSource, ProgramMap};

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
                source: PatchSource::Plugin { component_id: "aumu dls  appl".into(), state: String::new() },
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
        MockSound { patches, map, style_maps: BTreeMap::new(), parts: Default::default(), plugin_parts: Default::default(), port: false, audition: None, browse: None, last_added: None }
    }
}

fn presets(file: &str) -> Vec<Preset> {
    let gm: Vec<Preset> = (0..128u8).map(|p| Preset { bank: 0, program: p, name: format!("{}{}", gm_name(p), if file == SF2 { "" } else { " (Fluid)" }) }).collect();
    let kits = ["Standard", "Room", "Power", "Electronic", "Jazz", "Brush"].iter().enumerate().map(|(i, n)| Preset { bank: 128, program: i as u8 * 8, name: n.to_string() });
    gm.into_iter().chain(kits).collect()
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
                PatchSource::Plugin { component_id, state } => Some((self.patches[i].id.clone(), (component_id.clone(), state.clone()))),
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
            SoundLibraryCmd::SavePartAsPatch { part, name } => {
                let kp = &st.keyboard_parts[(part & 3) as usize];
                let own = self.parts[(part & 3) as usize].clone().and_then(|id| self.at(&id)).map(|i| self.patches[i].clone());
                let mut p = own.unwrap_or_else(|| Patch {
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
                self.add(p);
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
        for (i, p) in st.keyboard_parts.iter_mut().enumerate() {
            p.patch = self.parts[i].clone();
            if p.plays_bass {
                continue;
            }
            let own = name(self.parts[i].as_deref());
            let mapped = name(patches::resolve(&self.map, style, false, p.program).patch);
            p.voice_name = own.or(mapped).unwrap_or_else(|| gm[p.program as usize].clone());
        }
        let fonts = FONTS.map(String::from);
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
        };
    }
}

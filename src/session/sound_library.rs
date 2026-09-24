//! The sound library (#103) on the control side: the patches and maps (saved to the data
//! folder), the route table the synth and the port read (`patches::Routes`, in `Shared`),
//! the SoundFonts the synth needs for them, the keyboard parts' own patches, auditions and
//! the SoundFont preset browser.
//!
//! Everything that resolves the map runs here, off the real-time threads: when a style is
//! handed to the engine (its bank of the table is written first), when the engine takes it
//! over (`promote_style`), and after any edit. The synth and the engine thread only read
//! the table. docs/sound-library.md has the behaviour.

use super::{Control, RackLoad};
use crate::api::{
    voice_label, CategoryInfo, CmdError, PatchFields, PatchInfo, ProgramUse, SoundFontBrowse, SoundLibraryCmd, SoundLibraryState,
    STYLE_PART_NAMES,
};
use crate::engine::Prepared;
use crate::patches::route::{AUDITION, AUDITION_CHANNEL, MAX_FONTS};
use crate::patches::{self, Category, Patch, PatchDefaults, PatchSource, ProgramMap, Route, Routes, SoundLibrary};
use crate::{parts, synth};
use rtrb::Producer;
use rustysynth::SoundFont;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering::Relaxed;
use std::sync::{mpsc, Arc};

/// How long a library audition waits for its SoundFont to load before giving up.
const AUDITION_WAIT_NS: u64 = 15_000_000_000;

/// An audition: what it plays, and how far it got.
struct Audition {
    /// The patch id, or "preset".
    label: String,
    route: Option<Route>,
    /// The SoundFont it needs loaded first (None: plays the fallback / nothing to wait for).
    font: Option<String>,
    drums: bool,
    volume: u8,
    requested: u64,
    started: Option<u64>,
    step: usize,
}

/// A melodic audition: an arpeggio up, then the chord held. (ms, message) on the audition
/// channel, note-offs included.
const MELODIC: [(u64, [u8; 3]); 13] = [
    (0, [0x90, 60, 96]),
    (230, [0x80, 60, 0]),
    (250, [0x90, 64, 96]),
    (480, [0x80, 64, 0]),
    (500, [0x90, 67, 96]),
    (730, [0x80, 67, 0]),
    (750, [0x90, 72, 100]),
    (980, [0x80, 72, 0]),
    (1000, [0x90, 60, 90]),
    (1000, [0x90, 64, 90]),
    (1000, [0x90, 67, 90]),
    (2400, [0xB0, 123, 0]),
    (2800, [0x80, 60, 0]),
];

/// A drum kit audition: a bar of kick, snare and hi-hat, then a crash.
const DRUMS: [(u64, [u8; 3]); 14] = [
    (0, [0x90, 36, 110]),
    (0, [0x90, 42, 90]),
    (250, [0x90, 42, 70]),
    (500, [0x90, 38, 105]),
    (500, [0x90, 42, 90]),
    (750, [0x90, 42, 70]),
    (1000, [0x90, 36, 110]),
    (1000, [0x90, 42, 90]),
    (1250, [0x90, 36, 90]),
    (1250, [0x90, 42, 70]),
    (1500, [0x90, 38, 105]),
    (1750, [0x90, 38, 80]),
    (2000, [0x90, 49, 110]),
    (2000, [0x90, 36, 110]),
];

/// The control side's sound library.
pub(super) struct SoundLib {
    lib: SoundLibrary,
    /// Where it is saved (None: nowhere).
    path: Option<PathBuf>,
    /// The file there could not be read: it is never saved over (the text says why).
    locked: Option<String>,
    data_dir: Option<PathBuf>,
    /// Each keyboard part's own patch (Right 1, Right 2, Right 3, Left).
    part_patch: [Option<String>; parts::COUNT],
    /// Font ids: `fonts[id]` is the SoundFont file (stable for the session).
    fonts: Vec<String>,
    /// SoundFonts parsed for racks, by file (control side only).
    cache: HashMap<String, Arc<SoundFont>>,
    /// The SoundFonts of the rack the synth plays: the main one first.
    rack_fonts: Vec<String>,
    /// The SoundFonts of the rack being loaded.
    loading: Option<Vec<String>>,
    /// The table bank, style key and program list of the style playing.
    cur_bank: u8,
    cur_key: String,
    cur_usage: Vec<(u8, u8, u8, u8)>,
    /// The same for a style handed to the engine that it hasn't taken over yet (its tag).
    pending: Option<(u8, String, Vec<(u8, u8, u8, u8)>, u64)>,
    audition: Option<Audition>,
    /// The channels the map put on a plugin (the patch id), through #91's
    /// `assign_channel_plugin`.
    plugin_channels: [Option<String>; 16],
    /// Auditions to the synth (the control side's ring), when it runs.
    pub(super) audition_tx: Option<Producer<synth::Msg>>,
    browse: Option<SoundFontBrowse>,
    last_added: Option<String>,
}

impl SoundLib {
    /// Open the library in `data_dir` (none: an empty one, never saved).
    pub(super) fn open(data_dir: Option<&Path>) -> SoundLib {
        let path = data_dir.map(|d| d.join(patches::FILE_NAME));
        let (lib, locked) = match path.as_deref().map(SoundLibrary::load) {
            Some(Ok(lib)) => (lib, None),
            Some(Err(e)) => (SoundLibrary::default(), Some(format!("{e:#}"))),
            None => (SoundLibrary::default(), None),
        };
        SoundLib {
            lib,
            path,
            locked,
            data_dir: data_dir.map(Path::to_path_buf),
            part_patch: Default::default(),
            fonts: Vec::new(),
            cache: HashMap::new(),
            rack_fonts: Vec::new(),
            loading: None,
            cur_bank: 0,
            cur_key: String::new(),
            cur_usage: Vec::new(),
            pending: None,
            audition: None,
            plugin_channels: Default::default(),
            audition_tx: None,
            browse: None,
            last_added: None,
        }
    }

    /// Why the library file was not loaded, if it wasn't (for the message line).
    pub(super) fn load_error(&self) -> Option<&str> {
        self.locked.as_deref()
    }

    /// The font id of a SoundFont file (a new one if it has none yet).
    pub(super) fn font_id(&mut self, file: &str) -> Option<u8> {
        if let Some(i) = self.fonts.iter().position(|f| f == file) {
            return Some(i as u8);
        }
        if self.fonts.len() >= MAX_FONTS {
            return None;
        }
        self.fonts.push(file.to_string());
        Some(self.fonts.len() as u8 - 1)
    }

    /// The synth starts on `main`.
    pub(super) fn synth_started(&mut self, main: &str) {
        self.rack_fonts = vec![main.to_string()];
    }

    fn patch(&self, id: &str) -> Option<&Patch> {
        self.lib.patch(id)
    }

    /// How the synth plays a patch: its SoundFont preset if the file is in the SoundFont
    /// folder. A plugin patch has no route until plugin hosting plays channels (#91): it
    /// plays the fallback. TODO(#91): `Source::Plugin(slot)` once a channel can be routed
    /// to a plugin rack slot.
    fn route_of(&mut self, id: &str, avail: &[String]) -> Option<Route> {
        let (file, bank, program) = match &self.patch(id)?.source {
            PatchSource::SoundFont { file, bank, program } if avail.contains(file) => (file.clone(), *bank, *program),
            _ => return None,
        };
        Some(Route::sound_font(self.font_id(&file)?, bank, program))
    }

    /// Write table bank `bank` for the style stored under `key`.
    fn write_bank(&mut self, routes: &Routes, bank: u8, key: &str, avail: &[String]) {
        let style = self.lib.style_maps.get(key);
        let ids: Vec<Option<String>> =
            (0..128u8).map(|p| patches::resolve(&self.lib.map, style, false, p).patch.map(str::to_string)).collect();
        let drum = patches::resolve(&self.lib.map, style, true, 0).patch.map(str::to_string);
        let mut prog = [None; 128];
        for (p, id) in ids.iter().enumerate() {
            prog[p] = id.as_deref().and_then(|id| self.route_of(id, avail));
        }
        let drum = drum.as_deref().and_then(|id| self.route_of(id, avail));
        routes.write_bank(bank, &prog, drum);
    }

    fn write_parts(&mut self, routes: &Routes, avail: &[String]) {
        let ids = self.part_patch.clone();
        let r = ids.map(|id| id.as_deref().and_then(|id| self.route_of(id, avail)));
        routes.set_parts(r);
    }

    /// Rewrite everything the table holds: the playing style's bank, a pending style's, the
    /// parts'.
    fn write_all(&mut self, routes: &Routes, avail: &[String]) {
        let (bank, key) = (self.cur_bank, self.cur_key.clone());
        self.write_bank(routes, bank, &key, avail);
        if let Some((b, k, _, _)) = self.pending.clone() {
            self.write_bank(routes, b, &k, avail);
        }
        self.write_parts(routes, avail);
        routes.port_mapped.store(self.lib.port_sends_mapped, Relaxed);
    }

    /// A style about to play: its program list, and its parts' levels from the patches
    /// they resolve to where the style sets none (the patch's CC7, which the mixer shows).
    fn prepare(&mut self, p: &mut Prepared, key: &str) -> Vec<(u8, u8, u8, u8)> {
        let style = self.lib.style_maps.get(key);
        for part in 0..8 {
            if p.mix_set & 1 << part != 0 {
                continue;
            }
            let ch = 8 + part as u8;
            let Some((msb, _, pc)) = p.voices[ch as usize] else { continue };
            let drum = patches::is_drum(ch, msb);
            let r = patches::resolve(&self.lib.map, style, drum, patches::map_program(ch, msb, pc));
            if let Some(v) = r.patch.and_then(|id| self.lib.patch(id)).and_then(|q| q.defaults.volume) {
                p.mix[part] = v;
            }
        }
        p.program_changes()
    }

    /// The SoundFonts the synth should have: the main one, then every other one a library
    /// patch (or the audition) plays, if it is in the folder.
    fn wanted_fonts(&self, main: &str, avail: &[String]) -> Vec<String> {
        let mut extra: Vec<String> = self
            .lib
            .patches
            .iter()
            .filter_map(|p| match &p.source {
                PatchSource::SoundFont { file, .. } => Some(file.clone()),
                PatchSource::Plugin { .. } => None,
            })
            .chain(self.audition.as_ref().and_then(|a| a.font.clone()))
            .filter(|f| f != main && avail.contains(f))
            .collect();
        extra.sort();
        extra.dedup();
        std::iter::once(main.to_string()).chain(extra).collect()
    }

    /// Start loading a rack with `fonts` (the main one first) from `dir`, on a thread of its
    /// own: SoundFonts already parsed are reused.
    fn spawn_rack(&mut self, dir: &Path, fonts: Vec<String>, sample_rate: u32) -> Option<mpsc::Receiver<RackLoad>> {
        let mut jobs = Vec::new();
        for f in &fonts {
            let id = self.font_id(f)?;
            jobs.push((id, f.clone(), self.cache.get(f).cloned()));
        }
        let dir = dir.to_path_buf();
        let (tx, rx) = mpsc::channel();
        let spawned = std::thread::Builder::new().name("yahaha-sf2".into()).spawn(move || {
            let load = || -> Result<RackLoaded, String> {
                let mut fonts = Vec::new();
                for (id, file, cached) in jobs {
                    let font = match cached {
                        Some(f) => f,
                        None => {
                            let mut r = std::fs::File::open(dir.join(&file)).map_err(|e| format!("{file}: {e}"))?;
                            Arc::new(SoundFont::new(&mut r).map_err(|e| format!("{file}: {e:?}"))?)
                        }
                    };
                    fonts.push((id, file, font));
                }
                let with: Vec<(u8, Arc<SoundFont>)> = fonts.iter().map(|(id, _, f)| (*id, f.clone())).collect();
                let rack = synth::Rack::with_fonts(&with, sample_rate as i32).map_err(|e| format!("{e:#}"))?;
                Ok((Box::new(rack), fonts.into_iter().map(|(_, f, font)| (f, font)).collect()))
            };
            let _ = tx.send(load());
        });
        spawned.ok()?;
        self.loading = Some(fonts);
        Some(rx)
    }
}

/// A loaded rack and the SoundFonts in it (file, font), for the cache.
pub(super) type RackLoaded = (Box<synth::Rack>, Vec<(String, Arc<SoundFont>)>);

impl Control {
    fn avail_fonts(&self) -> Vec<String> {
        if self.sf_dir.is_some() { self.sound_fonts.clone() } else { Vec::new() }
    }

    /// Rewrite the route table, and save the library (after an edit).
    fn sound_library_changed(&mut self) {
        let avail = self.avail_fonts();
        let routes = self.shared.routes.clone();
        self.sound.write_all(&routes, &avail);
        self.sync_channel_routes();
        if let (Some(path), None) = (&self.sound.path, &self.sound.locked)
            && let Err(e) = self.sound.lib.save(path)
        {
            self.say(format!("Sound library not saved: {e:#}"), true);
        }
    }

    fn sl_fail<T>(&mut self, text: impl Into<String>) -> Result<T, CmdError> {
        Err(self.fail(text).unwrap_err())
    }

    fn need_patch(&mut self, id: &str) -> Result<usize, CmdError> {
        match self.sound.lib.index_of(id) {
            Some(i) => Ok(i),
            None => self.sl_fail(format!("no patch {id} in the sound library")),
        }
    }

    fn need_patch_or_none(&mut self, id: &Option<String>) -> Result<(), CmdError> {
        if let Some(id) = id {
            self.need_patch(id)?;
        }
        Ok(())
    }

    /// A SoundFont file name from a client: one in the SoundFont folder.
    fn need_sound_font(&mut self, file: &str) -> Result<(), CmdError> {
        self.list_sound_fonts();
        if file.contains('/') || file.contains('\\') || !self.sound_fonts.iter().any(|f| f == file) {
            return self.sl_fail(format!("no SoundFont {file} in the SoundFont folder"));
        }
        Ok(())
    }

    fn add_patch(&mut self, mut p: Patch) -> Result<(), CmdError> {
        if self.sound.lib.patches.len() >= patches::MAX_PATCHES {
            return self.sl_fail(format!("the sound library is full ({} patches)", patches::MAX_PATCHES));
        }
        p.id = patches::new_id(&p.name, self.sound.lib.patches.iter().map(|q| q.id.as_str()));
        p.defaults = p.defaults.clamped();
        self.sound.last_added = Some(p.id.clone());
        self.sound.lib.patches.push(p);
        self.sound_library_changed();
        Ok(())
    }

    /// The map a rule command edits: the global one, or the current style's own.
    fn map_mut(&mut self, style: bool) -> Result<&mut ProgramMap, CmdError> {
        if !style {
            return Ok(&mut self.sound.lib.map);
        }
        if self.sound.cur_key.is_empty() {
            return self.sl_fail("no style loaded");
        }
        let key = self.sound.cur_key.clone();
        Ok(self.sound.lib.style_maps.entry(key).or_default())
    }

    pub(super) fn sound_library_cmd(&mut self, c: SoundLibraryCmd) -> Result<(), CmdError> {
        match c {
            SoundLibraryCmd::CreatePatch { patch } => return self.add_patch(from_fields(String::new(), patch)),
            SoundLibraryCmd::UpdatePatch { id, patch } => {
                let i = self.need_patch(&id)?;
                let mut p = from_fields(id, patch);
                p.defaults = p.defaults.clamped();
                if p.name.trim().is_empty() {
                    p.name = self.sound.lib.patches[i].name.clone();
                }
                self.sound.lib.patches[i] = p;
            }
            SoundLibraryCmd::DeletePatch { id } => {
                let i = self.need_patch(&id)?;
                self.sound.lib.patches.remove(i);
                self.sound.lib.map.forget(&id);
                for m in self.sound.lib.style_maps.values_mut() {
                    m.forget(&id);
                }
                self.sound.lib.style_maps.retain(|_, m| !m.is_empty());
                for p in &mut self.sound.part_patch {
                    if p.as_deref() == Some(id.as_str()) {
                        *p = None;
                    }
                }
            }
            SoundLibraryCmd::DuplicatePatch { id } => {
                let i = self.need_patch(&id)?;
                let mut p = self.sound.lib.patches[i].clone();
                p.name = format!("{} copy", p.name);
                p.id = patches::new_id(&p.name, self.sound.lib.patches.iter().map(|q| q.id.as_str()));
                self.sound.last_added = Some(p.id.clone());
                self.sound.lib.patches.insert(i + 1, p);
            }
            SoundLibraryCmd::MovePatch { id, to } => {
                let i = self.need_patch(&id)?;
                let p = self.sound.lib.patches.remove(i);
                let to = (to as usize).min(self.sound.lib.patches.len());
                self.sound.lib.patches.insert(to, p);
            }
            SoundLibraryCmd::SetPatchFavourite { id, favourite } => {
                let i = self.need_patch(&id)?;
                self.sound.lib.patches[i].favourite = favourite;
            }
            SoundLibraryCmd::SavePartAsPatch { part, name } => return self.save_part_as_patch(part as usize & 3, name),
            SoundLibraryCmd::AddPresetAsPatch { file, bank, program, name } => {
                self.need_sound_font(&file)?;
                let name = name.filter(|n| !n.trim().is_empty()).unwrap_or_else(|| self.preset_name(&file, bank, program));
                let p = Patch {
                    id: String::new(),
                    name,
                    category: Category::guess(bank, program),
                    tags: Vec::new(),
                    favourite: false,
                    source: PatchSource::SoundFont { file, bank, program: program & 127 },
                    defaults: PatchDefaults::default(),
                };
                return self.add_patch(p);
            }
            SoundLibraryCmd::AuditionPatch { id } => {
                let i = self.need_patch(&id)?;
                let p = self.sound.lib.patches[i].clone();
                if let Some(why) = patches::unavailable_reason(&p, &self.avail_fonts()) {
                    return self.sl_fail(format!("{}: {why}", p.name));
                }
                let PatchSource::SoundFont { file, bank, program } = p.source else {
                    return self.sl_fail(format!("{}: a plugin patch auditions on a part (pick it for Right 1)", p.name));
                };
                return self.start_audition(id, file, bank, program, p.defaults.volume);
            }
            SoundLibraryCmd::AuditionPreset { file, bank, program } => {
                self.need_sound_font(&file)?;
                return self.start_audition("preset".into(), file, bank, program, None);
            }
            SoundLibraryCmd::StopPatchAudition => {
                self.stop_patch_audition();
                return Ok(());
            }
            SoundLibraryCmd::SetFamilyRule { family, patch, style } => {
                if family >= 16 {
                    return self.sl_fail(format!("no GM family {family} (0-15)"));
                }
                self.need_patch_or_none(&patch)?;
                self.map_mut(style)?.set_family(family as usize, patch);
            }
            SoundLibraryCmd::SetProgramOverride { program, patch, style } => {
                if program > 127 {
                    return self.sl_fail(format!("no program {program} (0-127)"));
                }
                self.need_patch_or_none(&patch)?;
                self.map_mut(style)?.set_override(program, patch);
            }
            SoundLibraryCmd::SetDrumRule { patch, style } => {
                self.need_patch_or_none(&patch)?;
                self.map_mut(style)?.drums = patch;
            }
            SoundLibraryCmd::ClearStyleMap => {
                let key = self.sound.cur_key.clone();
                self.sound.lib.style_maps.remove(&key);
            }
            SoundLibraryCmd::SetPartPatch { part, id } => return self.set_part_patch(part as usize & 3, id),
            SoundLibraryCmd::SetPortSendsMapped { on } => self.sound.lib.port_sends_mapped = on,
            SoundLibraryCmd::BrowseSoundFont { file } => {
                self.sound.browse = match file {
                    None => None,
                    Some(file) => {
                        self.need_sound_font(&file)?;
                        let path = self.sf_dir.as_ref().map(|d| d.join(&file)).unwrap_or_default();
                        Some(match patches::sf2::presets(&path) {
                            Ok(presets) => SoundFontBrowse { file, presets, error: None },
                            Err(e) => SoundFontBrowse { file, presets: Vec::new(), error: Some(format!("{e:#}")) },
                        })
                    }
                };
                return Ok(());
            }
            SoundLibraryCmd::ImportSoundLibrary { path, replace, maps } => return self.import_sound_library(&path, replace, maps),
            SoundLibraryCmd::ExportSoundLibrary { path } => return self.export_sound_library(path),
        }
        self.sound_library_changed();
        Ok(())
    }

    /// A preset's name as the SoundFont gives it (the file's presets are read once more).
    fn preset_name(&self, file: &str, bank: u16, program: u8) -> String {
        let listed = self.sound.browse.as_ref().filter(|b| b.file == file).map(|b| b.presets.clone());
        let presets = listed.or_else(|| self.sf_dir.as_ref().and_then(|d| patches::sf2::presets(&d.join(file)).ok())).unwrap_or_default();
        presets
            .iter()
            .find(|p| p.bank == bank && p.program == program)
            .map(|p| p.name.clone())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| format!("{} {bank}:{}", Path::new(file).file_stem().unwrap_or_default().to_string_lossy(), program + 1))
    }

    fn save_part_as_patch(&mut self, part: usize, name: Option<String>) -> Result<(), CmdError> {
        let kp = self.shared.parts.clone();
        let mut p = match self.sound.part_patch[part].clone().and_then(|id| self.sound.lib.patch(&id).cloned()) {
            Some(own) => own,
            None => {
                let Some(file) = self.sf_file.clone() else { return self.sl_fail("the synth plays no SoundFont") };
                let program = kp.channel_program(part);
                Patch {
                    id: String::new(),
                    name: crate::api::gm_name(program).to_string(),
                    category: Category::guess(0, program),
                    tags: Vec::new(),
                    favourite: false,
                    source: PatchSource::SoundFont { file, bank: 0, program },
                    defaults: PatchDefaults::default(),
                }
            }
        };
        p.defaults.volume = Some(kp.volume(part));
        p.defaults.octave = kp.octave[part].load(Relaxed).clamp(-2, 2);
        if let Some(n) = name.filter(|n| !n.trim().is_empty()) {
            p.name = n;
        }
        self.add_patch(p)
    }

    /// A keyboard part plays a library patch (or its GM voice again): its defaults (volume,
    /// octave, pan and sends) go to the part as CCs, as a voice selection does.
    fn set_part_patch(&mut self, part: usize, id: Option<String>) -> Result<(), CmdError> {
        self.need_patch_or_none(&id)?;
        let defaults = id.as_deref().and_then(|i| self.sound.lib.patch(i)).map(|p| p.defaults);
        self.sound.part_patch[part] = id;
        let kp = self.shared.parts.clone();
        if let Some(d) = defaults {
            if let Some(v) = d.volume {
                kp.set_volume(part, v);
            }
            kp.octave[part].store(d.octave.clamp(-2, 2), Relaxed);
            kp.set_fx(part, [d.pan, d.reverb, d.chorus]);
            self.wake_engine();
        }
        self.sound_library_changed();
        Ok(())
    }

    /// A GM voice was picked for keyboard part `part` (the voice list, an OTS): it no
    /// longer plays its own patch.
    pub(super) fn sound_library_part_voice(&mut self, part: usize) {
        if self.sound.part_patch[part & 3].take().is_some() {
            let (avail, routes) = (self.avail_fonts(), self.shared.routes.clone());
            self.sound.write_parts(&routes, &avail);
        }
    }

    fn import_sound_library(&mut self, path: &str, replace: bool, maps: bool) -> Result<(), CmdError> {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => return self.sl_fail(format!("{path}: {e}")),
        };
        let other = match SoundLibrary::from_json(&text) {
            Ok(l) => l,
            Err(e) => return self.sl_fail(format!("{path}: {e:#}")),
        };
        let n = other.patches.len();
        if replace {
            self.sound.lib = other;
            self.sound.part_patch = Default::default();
        } else {
            let added = self.sound.lib.merge(other, maps);
            self.say(format!("Imported {added} of {n} patches"), false);
        }
        // A library file that could not be read before is replaced by a good one now.
        self.sound.locked = None;
        self.sound_library_changed();
        Ok(())
    }

    fn export_sound_library(&mut self, path: Option<String>) -> Result<(), CmdError> {
        let path = match path.map(PathBuf::from).or_else(|| self.sound.data_dir.as_ref().map(|d| d.join("sound-library-export.json"))) {
            Some(p) => p,
            None => return self.sl_fail("no data folder to export to"),
        };
        match self.sound.lib.save(&path) {
            Ok(()) => {
                self.say(format!("Sound library exported to {}", path.display()), false);
                Ok(())
            }
            Err(e) => self.sl_fail(format!("{e:#}")),
        }
    }

    // ----- the per-channel routes (#91's `ChannelRoutes`) -----

    /// The patch a channel plays by the map now: a Style part's by its setup voice, a
    /// keyboard part's own patch or its GM voice through the map.
    fn channel_patch(&self, ch: u8) -> Option<String> {
        let lib = &self.sound.lib;
        let style = lib.style_maps.get(&self.sound.cur_key);
        if let Some(p) = parts::part_of_channel(ch) {
            if let Some(own) = self.sound.part_patch[p].clone() {
                return Some(own);
            }
            let prog = self.shared.parts.channel_program(p);
            return patches::resolve(&lib.map, style, false, prog).patch.map(str::to_string);
        }
        let (msb, _, pc) = self.info.voices[ch as usize & 15]?;
        let drum = patches::is_drum(ch, msb);
        let prog = if drum { pc } else { patches::map_program(ch, msb, pc) };
        patches::resolve(&lib.map, style, drum, prog).patch.map(str::to_string)
    }

    /// Bring the synth's per-channel route table (`SynthControl::routes`, #91) to the map,
    /// off the real-time threads: each Style part and keyboard part on the SoundFont slot
    /// its patch plays from (`Source::SoundFont(n)`, n = its slot in the rack, 0 = the main
    /// font), or, for a plugin patch, on a plugin (`assign_channel_plugin`). A channel the
    /// map put on a plugin goes back to the SoundFont when it no longer resolves to one.
    /// A keyboard part's own plugin (`SetPartPlugin`, #91) is left alone.
    ///
    /// Program changes inside a section still pick their SoundFont on the audio thread,
    /// from the per-program table (`patches::Routes`): they can't be known here in time.
    pub(super) fn sync_channel_routes(&mut self) {
        if self.synth.is_none() {
            return;
        }
        let channels: Vec<u8> = (0..parts::COUNT as u8).map(|p| parts::CHANNEL[p as usize]).chain(8..16).collect();
        for ch in channels {
            let patch = self.channel_patch(ch).and_then(|id| self.sound.lib.patch(&id).cloned());
            let mine = self.sound.plugin_channels[ch as usize].clone();
            match patch.as_ref().map(|p| (&p.id, &p.source)) {
                Some((id, PatchSource::Plugin { component_id, state })) if parts::part_of_channel(ch).is_none() => {
                    if mine.as_deref() == Some(id.as_str()) {
                        continue;
                    }
                    // TODO(#91): the plugin state format is #91's (ClassInfo bytes, base64).
                    let voice = super::PluginVoice { id: component_id.clone(), state: crate::api::base64_decode(state) };
                    match self.assign_channel_plugin(ch, voice) {
                        Ok(()) => self.sound.plugin_channels[ch as usize] = Some(id.clone()),
                        // No plugin host (this build, or no synth): the SoundFont fallback.
                        Err(_) => self.sound.plugin_channels[ch as usize] = None,
                    }
                }
                other => {
                    let n = match other {
                        Some((_, PatchSource::SoundFont { file, .. })) => {
                            self.sound.rack_fonts.iter().position(|f| f == file).unwrap_or(0).min(crate::route::MAX_FONT as usize) as u8
                        }
                        _ => 0,
                    };
                    if mine.is_some() {
                        self.sound.plugin_channels[ch as usize] = None;
                        self.route_channel_sound_font(ch, n);
                    } else if let Some(sy) = &self.synth
                        && sy.control.routes.source(ch) != crate::route::Source::Plugin
                    {
                        sy.control.routes.set(ch, crate::route::Source::SoundFont(n));
                    }
                }
            }
        }
    }

    // ----- auditions -----

    fn start_audition(&mut self, label: String, file: String, bank: u16, program: u8, volume: Option<u8>) -> Result<(), CmdError> {
        if self.snap.running {
            return self.sl_fail("Stop the band to audition a sound");
        }
        self.stop_patch_audition();
        let Some(id) = self.sound.font_id(&file) else { return self.sl_fail("too many SoundFonts") };
        let main = self.sf_file.as_deref() == Some(file.as_str());
        self.sound.audition = Some(Audition {
            label,
            route: Some(Route::sound_font(id, bank, program & 127)),
            font: (!main).then_some(file),
            drums: bank >= 128,
            volume: volume.unwrap_or(100).min(127),
            requested: self.clock_ns,
            started: None,
            step: 0,
        });
        let now = self.clock_ns;
        self.pump_audition(now);
        Ok(())
    }

    fn audition_push(&mut self, m: [u8; 3]) {
        if let Some(tx) = self.sound.audition_tx.as_mut() {
            let _ = tx.push(m);
        }
    }

    fn stop_patch_audition(&mut self) {
        if let Some(a) = self.sound.audition.take()
            && a.started.is_some()
        {
            self.audition_push([0xB0 | AUDITION_CHANNEL, 123, 0]);
            self.audition_push([AUDITION, 0, 0]);
            self.shared.routes.set_audition(None);
        }
    }

    fn pump_audition(&mut self, now: u64) {
        let Some(a) = self.sound.audition.as_ref() else { return };
        if self.snap.running {
            self.stop_patch_audition();
            return;
        }
        let (started, drums, step, route, volume, requested) = (a.started, a.drums, a.step, a.route, a.volume, a.requested);
        let Some(start) = started else {
            // Waiting for its SoundFont to join the synth's rack.
            let ready = a.font.as_ref().is_none_or(|f| self.sound.rack_fonts.contains(f)) || self.synth.is_none();
            if ready {
                self.shared.routes.set_audition(route);
                self.audition_push([AUDITION, 1, 0]);
                self.audition_push([0xB0 | AUDITION_CHANNEL, 7, volume]);
                if let Some(a) = self.sound.audition.as_mut() {
                    a.started = Some(now);
                }
            } else if now.saturating_sub(requested) > AUDITION_WAIT_NS {
                self.sound.audition = None;
                self.say("The SoundFont to audition did not load", true);
            }
            return;
        };
        let script: &[(u64, [u8; 3])] = if drums { &DRUMS } else { &MELODIC };
        let (mut step, t) = (step, now.saturating_sub(start) / 1_000_000);
        while step < script.len() && script[step].0 <= t {
            let (_, m) = script[step];
            self.audition_push([m[0] | AUDITION_CHANNEL, m[1], m[2]]);
            step += 1;
        }
        if step >= script.len() && t >= script[script.len() - 1].0 + 400 {
            self.stop_patch_audition();
        } else if let Some(a) = self.sound.audition.as_mut() {
            a.step = step;
        }
    }

    // ----- styles, the rack, the pump -----

    /// A style is being handed to the engine: its bank of the table (the one the playing
    /// style doesn't use), its parts' levels from their patches where the style sets none,
    /// and its program list.
    pub(super) fn sound_library_on_style(&mut self, p: &mut Prepared, path: &Path) {
        let key = patches::style_key(path);
        let bank = match &self.sound.pending {
            Some((b, ..)) => *b,
            None => 1 - self.sound.cur_bank % 2,
        };
        let usage = self.sound.prepare(p, &key);
        p.route_bank = bank;
        let (avail, routes) = (self.avail_fonts(), self.shared.routes.clone());
        self.sound.write_bank(&routes, bank, &key, &avail);
        self.sound.pending = Some((bank, key, usage, p.tag));
    }

    /// The first style, before the engine has it (bank 0).
    pub(super) fn sound_library_first_style(sound: &mut SoundLib, routes: &Routes, p: &mut Prepared, path: &Path, avail: &[String]) {
        let key = patches::style_key(path);
        sound.cur_usage = sound.prepare(p, &key);
        p.route_bank = 0;
        sound.cur_bank = 0;
        sound.cur_key = key.clone();
        sound.write_bank(routes, 0, &key, avail);
        sound.write_parts(routes, avail);
        routes.port_mapped.store(sound.lib.port_sends_mapped, Relaxed);
    }

    /// The engine took over the style handed to it (`promote_style`).
    pub(super) fn sound_library_promoted(&mut self, tag: u64) {
        if let Some((bank, key, usage, t)) = self.sound.pending.take() {
            if t != tag {
                // Another style was handed over since: it is the one still pending.
                self.sound.pending = Some((bank, key, usage, t));
                return;
            }
            self.sound.cur_bank = bank;
            self.sound.cur_key = key;
            self.sound.cur_usage = usage;
            self.shared.routes.current.store(bank, Relaxed);
            self.sync_channel_routes();
        }
    }

    /// Start loading another rack for the synth: `main` and the library's SoundFonts.
    pub(super) fn sound_library_rack(&mut self, main: &str) -> Option<mpsc::Receiver<RackLoad>> {
        let sy = self.synth.as_ref()?;
        let sample_rate = sy.info.sample_rate;
        let dir = self.sf_dir.clone()?;
        let avail = self.avail_fonts();
        let fonts = self.sound.wanted_fonts(main, &avail);
        self.sound.spawn_rack(&dir, fonts, sample_rate)
    }

    /// A rack finished loading: its SoundFonts are what the synth plays (once it is
    /// swapped in), and they stay parsed for the next rack.
    pub(super) fn sound_library_loaded(&mut self, fonts: Vec<(String, Arc<SoundFont>)>) {
        self.sound.rack_fonts = fonts.iter().map(|(f, _)| f.clone()).collect();
        self.sound.cache = fonts.into_iter().collect();
        self.sound.loading = None;
        self.sync_channel_routes();
    }

    /// Each pump: a rack with the SoundFonts the library needs, and the audition.
    pub(super) fn pump_sound_library(&mut self, now: u64) {
        if self.sf_load.is_none() && self.sf_ready.is_none() {
            self.sound.loading = None;
        }
        let can_swap = self.synth.as_ref().is_some_and(|s| s.swap.is_some());
        if can_swap
            && self.sound.loading.is_none()
            && let Some(main) = self.sf_file.clone()
        {
            let avail = self.avail_fonts();
            if self.sound.wanted_fonts(&main, &avail) != self.sound.rack_fonts
                && let Some(rx) = self.sound_library_rack(&main)
            {
                self.sf_load = Some((main, rx));
            }
        }
        self.pump_audition(now);
    }

    // ----- state -----

    /// A keyboard part's own patch and what its channel plays (for `KeyboardPart`).
    pub(super) fn part_sound(&self, part: usize) -> (Option<String>, Option<String>) {
        let kp = &self.shared.parts;
        let own = self.sound.part_patch[part].clone();
        let plays_bass = part == parts::LEFT && kp.manual_bass.load(Relaxed);
        let name = |id: &str| self.sound.lib.patch(id).map(|p| p.name.clone());
        if let Some(id) = own.as_deref().filter(|_| !plays_bass) {
            return (own.clone(), name(id));
        }
        let style = self.sound.lib.style_maps.get(&self.sound.cur_key);
        let r = patches::resolve(&self.sound.lib.map, style, false, kp.channel_program(part));
        (own, r.patch.and_then(name))
    }

    pub(super) fn sound_library_state(&self) -> SoundLibraryState {
        let avail = self.avail_fonts();
        let lib = &self.sound.lib;
        let style = lib.style_maps.get(&self.sound.cur_key);
        let name = |id: &str| lib.patch(id).map(|p| p.name.clone());
        let usage = self
            .sound
            .cur_usage
            .iter()
            .map(|&(ch, msb, lsb, program)| {
                let drums = patches::is_drum(ch, msb);
                let gm_program = if drums { program } else { patches::map_program(ch, msb, program) };
                let r = patches::resolve(&lib.map, style, drums, gm_program);
                let voice = voice_label(ch, Some((msb, lsb, program)));
                ProgramUse {
                    channel: ch + 1,
                    part: STYLE_PART_NAMES[(ch as usize).saturating_sub(8) & 7].to_string(),
                    msb,
                    lsb,
                    program,
                    gm_program,
                    plays: r.patch.and_then(name).unwrap_or_else(|| voice.clone()),
                    voice,
                    drums,
                    patch: r.patch.map(str::to_string),
                    rule: r.rule,
                    from_style: r.from_style,
                }
            })
            .collect();
        SoundLibraryState {
            patches: lib
                .patches
                .iter()
                .map(|p| {
                    let note = patches::unavailable_reason(p, &avail);
                    PatchInfo { patch: p.clone(), available: note.is_none(), note }
                })
                .collect(),
            categories: Category::ALL.iter().map(|&c| CategoryInfo { id: c, label: c.label().to_string() }).collect(),
            families: patches::FAMILY_NAMES.iter().map(|s| s.to_string()).collect(),
            map: lib.map.clone(),
            style_map: style.cloned().unwrap_or_default(),
            style_key: self.sound.cur_key.clone(),
            usage,
            port_sends_mapped: lib.port_sends_mapped,
            auditioning: self.sound.audition.as_ref().map(|a| a.label.clone()),
            browse: self.sound.browse.clone(),
            file: self.sound.path.as_ref().map(|p| p.display().to_string()),
            extra_sound_fonts: self.sound.rack_fonts.iter().skip(1).cloned().collect(),
            last_added: self.sound.last_added.clone(),
        }
    }
}

fn from_fields(id: String, f: PatchFields) -> Patch {
    Patch { id, name: f.name, category: f.category, tags: f.tags, favourite: f.favourite, source: f.source, defaults: f.defaults }
}

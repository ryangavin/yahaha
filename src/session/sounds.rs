//! The sound catalog (#117, api/sounds.rs): every SoundFont preset in the folder, every
//! instrument plugin and every saved sound, as one list for the Sound Browser.
//!
//! The list is built on demand (`Session::sound_catalog`) and cached until its `revision`
//! moves. Each publish works out a cheap fingerprint of what the list is made of (the
//! folder, the plugins, the patches, the favourites); a new one bumps the revision and
//! sends `Event::SoundsChanged`. Favourites, Recents and plugin categories are saved in
//! `sound-settings.json` next to the default sound set.

use super::sound_set::write_key;
use super::Control;
use crate::api::{
    CmdError, PartsCmd, PatchFields, PluginCmd, SoundCatalog, SoundEntry, SoundLibraryCmd, SoundPluginInfo,
    SoundSource, SoundsCmd, SoundsState,
};
use crate::patches::sf2::{self, Preset};
use crate::patches::{Category, PatchSource};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;
use std::sync::Arc;

/// How many Recents are kept.
pub const MAX_RECENTS: usize = 20;

/// The control side's catalog state.
#[derive(Debug, Default)]
pub(super) struct Sounds {
    /// Favourite presets and plugins (a saved sound's is its patch's).
    favourites: BTreeSet<String>,
    /// Most recent first.
    recents: Vec<String>,
    /// Plugin categories the user set, by entry id.
    categories: BTreeMap<String, Category>,
    /// Each font's presets, read once per file (the folder as last listed).
    presets: HashMap<String, Vec<Preset>>,
    /// The fingerprint of what the catalog is built from, and its revision.
    key: u64,
    revision: u64,
    /// The catalog last built (its revision inside).
    cache: Option<Arc<SoundCatalog>>,
}

impl Sounds {
    /// Read the favourites, Recents and categories saved in `sound-settings.json`.
    pub(super) fn open(file: Option<&Path>) -> Sounds {
        let v: serde_json::Value =
            file.and_then(|f| std::fs::read_to_string(f).ok()).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        let get = |k: &str| v.get(k).cloned().unwrap_or_default();
        Sounds {
            favourites: serde_json::from_value(get("favourites")).unwrap_or_default(),
            recents: serde_json::from_value(get("recents")).unwrap_or_default(),
            categories: serde_json::from_value(get("soundCategories")).unwrap_or_default(),
            ..Sounds::default()
        }
    }
}

/// A preset's catalog id.
pub fn preset_id(file: &str, bank: u16, program: u8) -> String {
    format!("sf:{file}:{bank}:{program}")
}

/// A preset id's file, bank and program. The file may hold ':' itself: bank and program
/// are the last two fields.
pub fn parse_preset_id(id: &str) -> Option<(&str, u16, u8)> {
    let rest = id.strip_prefix("sf:")?;
    let (rest, program) = rest.rsplit_once(':')?;
    let (file, bank) = rest.rsplit_once(':')?;
    Some((file, bank.parse().ok()?, program.parse().ok().filter(|&p: &u8| p < 128)?))
}

/// A plugin's likely category, from its name and maker: most instrument plugins are
/// synths, so anything else is Synth Lead.
pub fn plugin_category(name: &str, maker: &str) -> Category {
    let n = format!("{name} {maker}").to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| n.contains(w));
    if has(&["rhodes", "wurli", "e.piano", "epiano", "electric piano", "e-piano", "ep-", "clav"]) {
        Category::EPiano
    } else if has(&["piano", "grand", "keyscape", "pianoteq"]) {
        Category::Piano
    } else if has(&["organ", "b-3", "b3", "hammond", "vox continental", "farfisa"]) {
        Category::Organ
    } else if has(&["drum", "perc", "kit", "beat", "battery", "808", "909"]) {
        Category::DrumsPerc
    } else if has(&["bass"]) {
        Category::Bass
    } else if has(&["guitar", "strum"]) {
        Category::Guitar
    } else if has(&["string", "violin", "cello", "orchestra", "symphon"]) {
        Category::Strings
    } else if has(&["brass", "trumpet", "horn", "trombone"]) {
        Category::Brass
    } else if has(&["sax", "flute", "clarinet", "oboe", "wind"]) {
        Category::SaxWoodwind
    } else if has(&["choir", "vocal", "voice", "vox"]) {
        Category::Choir
    } else if has(&["pad", "ambient", "atmos"]) {
        Category::Pad
    } else {
        Category::SynthLead
    }
}

impl Control {
    /// Each publish: when what the catalog is made of changed, a new revision (the caller
    /// sends `SoundsChanged`). Returns the new revision.
    pub(super) fn sounds_touch(&mut self) -> Option<u64> {
        // Presets of files that came into the folder; forget those that left.
        let fonts = self.sound_fonts.clone();
        self.sounds.presets.retain(|f, _| fonts.contains(f));
        for f in &fonts {
            if !self.sounds.presets.contains_key(f) {
                let presets = self.sf_dir.as_ref().and_then(|d| sf2::presets(&d.join(f)).ok()).unwrap_or_default();
                self.sounds.presets.insert(f.clone(), presets);
            }
        }
        let mut h = DefaultHasher::new();
        fonts.hash(&mut h);
        self.sf_file.hash(&mut h);
        for p in self.plugins_state().list {
            (p.id, p.name, p.manufacturer, p.format, p.last_error).hash(&mut h);
        }
        for p in self.sound_patches() {
            (&p.id, &p.name, p.category as u8, p.favourite).hash(&mut h);
            serde_json::to_string(&p.source).unwrap_or_default().hash(&mut h);
        }
        let s = &self.sounds;
        (&s.favourites, &s.recents).hash(&mut h);
        for (id, c) in &s.categories {
            (id, *c as u8).hash(&mut h);
        }
        let key = h.finish();
        if key == self.sounds.key && self.sounds.revision > 0 {
            return None;
        }
        self.sounds.key = key;
        self.sounds.revision += 1;
        Some(self.sounds.revision)
    }

    /// The catalog, built again only when its revision moved.
    pub(super) fn sound_catalog(&mut self) -> Arc<SoundCatalog> {
        if let Some(c) = &self.sounds.cache
            && c.revision == self.sounds.revision
        {
            return c.clone();
        }
        let c = Arc::new(self.build_catalog());
        self.sounds.cache = Some(c.clone());
        c
    }

    fn build_catalog(&self) -> SoundCatalog {
        let s = &self.sounds;
        let recent = |id: &str| s.recents.iter().any(|r| r == id);
        let mut entries = Vec::new();
        for f in &self.sound_fonts {
            for p in s.presets.get(f).into_iter().flatten() {
                let id = preset_id(f, p.bank, p.program);
                entries.push(SoundEntry {
                    name: if p.name.trim().is_empty() { format!("{f} {}:{}", p.bank, p.program) } else { p.name.trim().to_string() },
                    category: Category::guess(p.bank, p.program),
                    source: SoundSource::SoundFont,
                    detail: f.clone(),
                    favourite: s.favourites.contains(&id),
                    recent: recent(&id),
                    plugin: None,
                    id,
                });
            }
        }
        for p in self.plugins_state().list {
            let id = format!("au:{}", p.id);
            entries.push(SoundEntry {
                category: s.categories.get(&id).copied().unwrap_or_else(|| plugin_category(&p.name, &p.manufacturer)),
                source: SoundSource::Plugin,
                detail: p.manufacturer,
                favourite: s.favourites.contains(&id),
                recent: recent(&id),
                plugin: Some(SoundPluginInfo { format: p.format, last_error: p.last_error }),
                name: p.name,
                id,
            });
        }
        for p in self.sound_patches() {
            let id = format!("saved:{}", p.id);
            let detail = match &p.source {
                PatchSource::SoundFont { file, .. } => file.clone(),
                PatchSource::Plugin { component_id, .. } => component_id.clone(),
            };
            entries.push(SoundEntry {
                name: p.name.clone(),
                category: p.category,
                source: SoundSource::Saved,
                detail,
                favourite: p.favourite,
                recent: recent(&id),
                plugin: None,
                id,
            });
        }
        SoundCatalog { revision: s.revision, entries, recents: s.recents.clone() }
    }

    pub(super) fn sounds_state(&self) -> SoundsState {
        let presets: usize = self.sound_fonts.iter().filter_map(|f| self.sounds.presets.get(f)).map(Vec::len).sum();
        let plugins = self.plugins_state();
        let auditioning = self.sound_audition().map(|l| if l.starts_with("sf:") { l.to_string() } else { format!("saved:{l}") });
        SoundsState {
            revision: self.sounds.revision,
            count: (presets + plugins.list.len() + self.sound_patches().len()) as u32,
            scanning: plugins.scanning,
            auditioning: auditioning.filter(|a| a != "saved:preset"),
        }
    }

    pub(super) fn sounds_cmd(&mut self, c: SoundsCmd) -> Result<(), CmdError> {
        match c {
            SoundsCmd::SetSoundFavourite { id, on } => {
                if let Some(patch) = id.strip_prefix("saved:") {
                    return self.sound_library_cmd(SoundLibraryCmd::SetPatchFavourite { id: patch.into(), favourite: on });
                }
                self.need_sound(&id)?;
                if on {
                    self.sounds.favourites.insert(id);
                } else {
                    self.sounds.favourites.remove(&id);
                }
                self.save_sounds("favourites", serde_json::to_value(&self.sounds.favourites));
            }
            SoundsCmd::AuditionSound { id } => {
                if let Some(patch) = id.strip_prefix("saved:") {
                    return self.sound_library_cmd(SoundLibraryCmd::AuditionPatch { id: patch.into() });
                }
                let Some((file, bank, program)) = parse_preset_id(&id) else {
                    return self.fail(format!("{}: a plugin auditions on a part (assign it)", self.sound_name(&id)));
                };
                let file = file.to_string();
                self.need_sound(&id)?;
                return self.start_audition(id, file, bank, program, None);
            }
            SoundsCmd::StopSoundAudition => return self.sound_library_cmd(SoundLibraryCmd::StopPatchAudition),
            SoundsCmd::AssignSound { part, id } => return self.assign_sound(part, id),
            SoundsCmd::SetSoundCategory { id, category } => {
                if let Some(patch) = id.strip_prefix("saved:") {
                    let Some(p) = self.sound_patches().iter().find(|p| p.id == patch).cloned() else {
                        return self.fail(format!("no sound {id}"));
                    };
                    let fields = PatchFields { name: p.name, category, tags: p.tags, favourite: p.favourite, source: p.source, defaults: p.defaults };
                    return self.sound_library_cmd(SoundLibraryCmd::UpdatePatch { id: p.id, patch: fields });
                }
                if !id.starts_with("au:") {
                    return self.fail("a preset's category is its GM family");
                }
                self.need_sound(&id)?;
                self.sounds.categories.insert(id, category);
                self.save_sounds("soundCategories", serde_json::to_value(&self.sounds.categories));
            }
        }
        Ok(())
    }

    /// `AssignSound`: the part plays it through the command its source has.
    fn assign_sound(&mut self, part: u8, id: String) -> Result<(), CmdError> {
        if part > 3 {
            return self.fail(format!("no keyboard part {part} (0-3)"));
        }
        if let Some(patch) = id.strip_prefix("saved:") {
            self.sound_library_cmd(SoundLibraryCmd::SetPartPatch { part, id: Some(patch.into()) })?;
        } else if let Some(plugin) = id.strip_prefix("au:") {
            self.need_sound(&id)?;
            self.plugins_cmd(PluginCmd::SetPartPlugin { part, id: plugin.into(), state: None })?;
        } else if let Some((file, bank, program)) = parse_preset_id(&id) {
            let file = file.to_string();
            self.need_sound(&id)?;
            if self.sf_file.as_deref() == Some(file.as_str()) && bank == 0 {
                self.parts_cmd(PartsCmd::SetPartVoice { part, program })?;
            } else {
                // Another font's preset plays as a saved sound: the one already in the
                // library, else a new one.
                let existing = self.sound_patches().iter().find(|p| {
                    matches!(&p.source, PatchSource::SoundFont { file: f, bank: b, program: q } if *f == file && *b == bank && *q == program)
                });
                let patch = match existing {
                    Some(p) => p.id.clone(),
                    None => {
                        self.sound_library_cmd(SoundLibraryCmd::AddPresetAsPatch { file, bank, program, name: None })?;
                        self.sound_last_added().unwrap_or_default().to_string()
                    }
                };
                self.sound_library_cmd(SoundLibraryCmd::SetPartPatch { part, id: Some(patch) })?;
            }
        } else {
            return self.fail(format!("no sound {id}"));
        }
        self.sounds.recents.retain(|r| *r != id);
        self.sounds.recents.insert(0, id);
        self.sounds.recents.truncate(MAX_RECENTS);
        self.save_sounds("recents", serde_json::to_value(&self.sounds.recents));
        Ok(())
    }

    /// A preset or plugin id that is in the catalog.
    fn need_sound(&mut self, id: &str) -> Result<(), CmdError> {
        let known = if let Some((file, bank, program)) = parse_preset_id(id) {
            self.sounds.presets.get(file).is_some_and(|v| v.iter().any(|p| p.bank == bank && p.program == program))
        } else if let Some(plugin) = id.strip_prefix("au:") {
            self.plugins_state().list.iter().any(|p| p.id == plugin)
        } else {
            false
        };
        if known { Ok(()) } else { self.fail(format!("no sound {id}")) }
    }

    fn sound_name(&self, id: &str) -> String {
        let plugin = id.strip_prefix("au:").unwrap_or(id);
        self.plugins_state().list.into_iter().find(|p| p.id == plugin).map_or_else(|| id.to_string(), |p| p.name)
    }

    fn save_sounds(&mut self, key: &str, value: serde_json::Result<serde_json::Value>) {
        let r = value.map_err(anyhow::Error::from).and_then(|v| write_key(self.sound_set.file(), key, v));
        if let Err(e) = r {
            self.say(format!("The sound browser settings were not saved: {e:#}"), true);
        }
    }
}

#[cfg(test)]
#[path = "sounds_tests.rs"]
mod tests;

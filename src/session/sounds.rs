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
use crate::api::{parse_preset_id, CmdError, PartsCmd, PatchFields, PluginCmd, SoundCatalog, SoundLibraryCmd, SoundPrefs, SoundsCmd, SoundsState};
use crate::patches::sf2::{self, Preset};
use crate::patches::{Category, PatchSource};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;
use std::sync::Arc;

/// The control side's catalog state.
#[derive(Debug, Default)]
pub(super) struct Sounds {
    /// Favourites, Recents and plugin categories (saved).
    prefs: SoundPrefs,
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
        let prefs = file.and_then(|f| std::fs::read_to_string(f).ok()).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        Sounds { prefs, ..Sounds::default() }
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
        serde_json::to_string(&self.sounds.prefs).unwrap_or_default().hash(&mut h);
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
        let fonts: Vec<(&str, &[Preset])> =
            self.sound_fonts.iter().map(|f| (f.as_str(), s.presets.get(f).map_or(&[][..], Vec::as_slice))).collect();
        let entries = s.prefs.entries(&fonts, &self.plugins_state().list, self.sound_patches());
        SoundCatalog { revision: s.revision, entries, recents: s.prefs.recents.clone() }
    }

    pub(super) fn sounds_state(&self) -> SoundsState {
        let presets: usize = self.sound_fonts.iter().filter_map(|f| self.sounds.presets.get(f)).map(Vec::len).sum();
        let plugins = self.plugins_state();
        let auditioning =
            self.sound_audition().map(|l| if l.starts_with("sf:") || l.starts_with("au:") { l.to_string() } else { format!("saved:{l}") });
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
                    self.sounds.prefs.favourites.insert(id);
                } else {
                    self.sounds.prefs.favourites.remove(&id);
                }
                self.save_sounds("favourites", serde_json::to_value(&self.sounds.prefs.favourites));
            }
            SoundsCmd::AuditionSound { id } => {
                if let Some(patch) = id.strip_prefix("saved:") {
                    return self.sound_library_cmd(SoundLibraryCmd::AuditionPatch { id: patch.into() });
                }
                if let Some(plugin) = id.strip_prefix("au:") {
                    self.need_sound(&id)?;
                    let (name, drums) = (self.sound_name(&id), self.plugin_category_of(&id) == Category::DrumsPerc);
                    let voice = super::PluginVoice { id: plugin.to_string(), state: None };
                    return self.start_plugin_audition(id, &name, voice, drums, None);
                }
                let Some((file, bank, program)) = parse_preset_id(&id) else { return self.fail(format!("no sound {id}")) };
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
                self.sounds.prefs.sound_categories.insert(id, category);
                self.save_sounds("soundCategories", serde_json::to_value(&self.sounds.prefs.sound_categories));
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
            self.need_sound(&id)?;
            if self.sf_file.as_deref() == Some(file) && bank == 0 {
                self.parts_cmd(PartsCmd::SetPartVoice { part, program })?;
            } else {
                let patch = self.patch_for_sound(&id)?;
                self.sound_library_cmd(SoundLibraryCmd::SetPartPatch { part, id: Some(patch) })?;
            }
        } else {
            return self.fail(format!("no sound {id}"));
        }
        self.sounds.prefs.push_recent(id);
        self.save_sounds("recents", serde_json::to_value(&self.sounds.prefs.recents));
        Ok(())
    }

    /// The library patch that plays catalog entry `id`: a saved sound's own, else the
    /// library's patch for that preset or plugin (a plugin's with its default preset),
    /// added once. An id without a catalog prefix is a patch id already.
    pub(super) fn patch_for_sound(&mut self, id: &str) -> Result<String, CmdError> {
        if let Some(patch) = id.strip_prefix("saved:") {
            return Ok(patch.to_string());
        }
        let source = if let Some((file, bank, program)) = parse_preset_id(id) {
            PatchSource::SoundFont { file: file.to_string(), bank, program }
        } else if let Some(plugin) = id.strip_prefix("au:") {
            PatchSource::Plugin { component_id: plugin.to_string(), state: String::new() }
        } else {
            return Ok(id.to_string());
        };
        self.need_sound(id)?;
        if let Some(p) = self.sound_patches().iter().find(|p| p.source == source) {
            return Ok(p.id.clone());
        }
        match source {
            PatchSource::SoundFont { file, bank, program } => {
                self.sound_library_cmd(SoundLibraryCmd::AddPresetAsPatch { file, bank, program, name: None })?
            }
            source @ PatchSource::Plugin { .. } => {
                let (name, category) = (self.sound_name(id), self.plugin_category_of(id));
                let patch = PatchFields { name, category, tags: Vec::new(), favourite: false, source, defaults: Default::default() };
                self.sound_library_cmd(SoundLibraryCmd::CreatePatch { patch })?
            }
        }
        Ok(self.sound_last_added().unwrap_or_default().to_string())
    }

    /// A program map rule naming a catalog entry (#117) names its library patch.
    pub(super) fn rule_patch(&mut self, patch: Option<String>) -> Result<Option<String>, CmdError> {
        patch.map(|id| self.patch_for_sound(&id)).transpose()
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

    /// A plugin's category: the user's, else the guess.
    fn plugin_category_of(&self, id: &str) -> Category {
        let plugin = id.strip_prefix("au:").unwrap_or(id);
        let list = self.plugins_state().list;
        let p = list.iter().find(|p| p.id == plugin);
        self.sounds.prefs.plugin_category(id, p.map_or("", |p| &p.name), p.map_or("", |p| &p.manufacturer))
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

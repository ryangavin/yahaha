//! The sound catalog (#117, api/sounds.rs): every SoundFont preset in the folder, every
//! instrument plugin and every saved sound, as one list for the Sound Browser.
//!
//! The list is built on demand (`Session::sound_catalog`) and cached until its `revision`
//! moves. Each publish works out a cheap fingerprint of what the list is made of (the
//! folder, the plugins, the patches, the favourites); a new one bumps the revision and
//! sends `Event::SoundsChanged`. Favourites, Recents and plugin categories are saved in
//! `sound-settings.json` (session/gm_auto.rs).

use super::gm_auto::write_key;
use super::Control;
use crate::api::{parse_plugin_id, parse_preset_id, CmdError, PartsCmd, PatchFields, PluginCmd, SoundCatalog, SoundLibraryCmd, SoundPrefs, SoundsCmd, SoundsState};
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
    /// The font list `presets` was brought up to date with: it is looked at again only
    /// when the folder's list differs.
    fonts: Vec<String>,
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
        // Presets of files that came into the folder; forget those that left. Only when the
        // folder's list changed: a file's presets are read once.
        if self.sounds.fonts != self.sound_fonts {
            self.sounds.presets.retain(|f, _| self.sound_fonts.contains(f));
            for f in &self.sound_fonts {
                if !self.sounds.presets.contains_key(f) {
                    let presets = self.sf_dir.as_ref().and_then(|d| sf2::presets(&d.join(f)).ok()).unwrap_or_default();
                    self.sounds.presets.insert(f.clone(), presets);
                }
            }
            self.sounds.fonts.clone_from(&self.sound_fonts);
        }
        // Only what the catalog shows is hashed, in place: never a plugin sound's state
        // (MBs for a sampler, and not in the catalog), and nothing serialised (#134).
        let mut h = DefaultHasher::new();
        self.sound_fonts.hash(&mut h);
        self.sf_file.hash(&mut h);
        self.hash_plugins_for_catalog(&mut h);
        for p in self.sound_patches() {
            (&p.id, &p.name, p.category as u8, p.favourite).hash(&mut h);
            match &p.source {
                PatchSource::SoundFont { file, .. } => file.hash(&mut h),
                PatchSource::Plugin { component_id, .. } => component_id.hash(&mut h),
            }
        }
        let prefs = &self.sounds.prefs;
        (&prefs.favourites, &prefs.recents, &prefs.sound_categories).hash(&mut h);
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
        let entries = s.prefs.entries(&fonts, &self.plugins_state().list, &self.plugin_preset_lists(), self.sound_patches());
        SoundCatalog { revision: s.revision, entries, recents: s.prefs.recents.clone(), fonts: crate::api::font_summaries(&fonts) }
    }

    pub(super) fn sounds_state(&self) -> SoundsState {
        let presets: usize = self.sound_fonts.iter().filter_map(|f| self.sounds.presets.get(f)).map(Vec::len).sum();
        let (plugins, plugin_presets, scanning) = self.catalog_plugin_counts();
        SoundsState {
            revision: self.sounds.revision,
            count: (presets + plugins + plugin_presets + self.sound_patches().len()) as u32,
            scanning,
            listing_presets: self.plugin_presets_listing().into_iter().map(|id| format!("au:{id}")).collect(),
        }
    }

    /// The plugins the catalog lists, their presets, and whether a scan runs, counted in
    /// place: exactly `plugins_state().list.len()`, the presets in all of
    /// `plugin_preset_lists()` (a plugin's factory presets plus its `.aupreset` files) and
    /// `plugins_state().scanning`, with no list built.
    #[cfg(feature = "plugins")]
    fn catalog_plugin_counts(&self) -> (usize, usize, bool) {
        let list = &self.plugins.list;
        let presets = list.iter().map(|p| p.factory_presets.as_ref().map_or(0, Vec::len) + p.user_presets.len()).sum();
        (list.len(), presets, self.plugins.scan_rx.is_some())
    }

    #[cfg(not(feature = "plugins"))]
    fn catalog_plugin_counts(&self) -> (usize, usize, bool) {
        (0, 0, false)
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
            SoundsCmd::AssignSound { part, id } => return self.assign_sound(part, id),
            SoundsCmd::ReplacePartSound { part, id } => return self.replace_part_sound(part, id),
            SoundsCmd::SetSoundCategory { id, category } => {
                if let Some(patch) = id.strip_prefix("saved:") {
                    let Some(p) = self.sound_patches().iter().find(|p| p.id == patch).cloned() else {
                        return self.fail(format!("no sound {id}"));
                    };
                    let fields = PatchFields { name: p.name, category, tags: p.tags, favourite: p.favourite, source: p.source };
                    return self.sound_library_cmd(SoundLibraryCmd::UpdatePatch { id: p.id, patch: fields });
                }
                if !id.starts_with("au:") {
                    return self.fail("a preset's category is its GM family");
                }
                self.need_sound(&id)?;
                self.set_preset_category(id, category);
            }
            SoundsCmd::ListPluginPresets { id } => {
                let Some((plugin, None)) = parse_plugin_id(&id) else { return self.fail(format!("{id} is not a plugin")) };
                if let Err(e) = self.list_plugin_presets(plugin) {
                    return self.fail(e);
                }
            }
            SoundsCmd::AddToMySounds { id } => {
                if id.starts_with("saved:") {
                    return Ok(());
                }
                if !id.starts_with("sf:") && !id.starts_with("au:") {
                    return self.fail(format!("no sound {id}"));
                }
                self.patch_for_sound(&id)?;
            }
            SoundsCmd::SavePartAsPluginPreset { part, name, category, overwrite } => {
                if part > 3 {
                    return self.fail(format!("no keyboard part {part} (0-3)"));
                }
                if let Err(e) = self.save_part_as_preset(crate::parts::CHANNEL[part as usize], &name, category, overwrite) {
                    return self.fail(e);
                }
            }
        }
        Ok(())
    }

    /// File catalog entry `id` (a plugin or one of its presets) under `category`: the
    /// user's choice, kept in `sound-settings.json` (the `.aupreset` itself is not touched).
    pub(super) fn set_preset_category(&mut self, id: String, category: Category) {
        self.sounds.prefs.sound_categories.insert(id, category);
        self.save_sounds("soundCategories", serde_json::to_value(&self.sounds.prefs.sound_categories));
    }

    /// `ReplacePartSound`: the part plays sound `id` (as `AssignSound`) and keeps its mix,
    /// exactly as a rack part holds it (docs/racks.md: swapping a sound never touches the
    /// mix). Library › Replace… sends it for a part whose plugin is missing.
    fn replace_part_sound(&mut self, part: u8, id: String) -> Result<(), CmdError> {
        if part > 3 {
            return self.fail(format!("no keyboard part {part} (0-3)"));
        }
        let mix = self.capture_rack_part(part as usize, false);
        self.assign_sound(part, id)?;
        if let Err(e) = self.apply_rack_mix(part as usize, &mix) {
            return self.fail(e);
        }
        self.wake_engine();
        Ok(())
    }

    /// `AssignSound`: the part plays it through the command its source has.
    fn assign_sound(&mut self, part: u8, id: String) -> Result<(), CmdError> {
        if part > 3 {
            return self.fail(format!("no keyboard part {part} (0-3)"));
        }
        if let Some(patch) = id.strip_prefix("saved:") {
            self.sound_library_cmd(SoundLibraryCmd::SetPartPatch { part, id: Some(patch.into()) })?;
        } else if let Some((plugin, preset)) = parse_plugin_id(&id) {
            self.need_sound(&id)?;
            match preset {
                Some(key) => self.plugins_cmd(PluginCmd::SetPartPluginPreset { part, id: plugin.into(), preset: key.into() })?,
                None => self.plugins_cmd(PluginCmd::SetPartPlugin { part, id: plugin.into(), state: None })?,
            }
        } else if let Some((file, bank, program)) = parse_preset_id(&id) {
            self.need_sound(&id)?;
            if self.sf_file.as_deref() == Some(file) && bank == 0 {
                // The part's GM voice: `setPartVoice` ends a plugin picked for the part,
                // as a SoundFont patch does (`set_part_patch`).
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
        } else if let Some((plugin, preset)) = parse_plugin_id(id) {
            // A preset as a patch is the one plugin sound for it (its origin finds it
            // again): an `.aupreset` keeps the file's settings, read now; a factory
            // preset's are captured the first time it plays (docs/sound-browser.md).
            let origin = preset.and_then(crate::patches::PluginOrigin::from_preset_key).unwrap_or_default();
            let state = match preset {
                None => String::new(),
                Some(key) => match self.preset_voice(plugin, key).map(|v| v.state) {
                    Ok(Some(st)) => crate::api::base64_encode(&st),
                    Ok(None) => String::new(),
                    Err(e) => {
                        self.fail(e)?;
                        return Ok(String::new());
                    }
                },
            };
            PatchSource::Plugin { component_id: plugin.to_string(), state, origin }
        } else {
            return Ok(id.to_string());
        };
        self.need_sound(id)?;
        // The library's sound for that preset, or one with exactly the preset's settings:
        // an `.aupreset` written by Save as… is the sound saved with it, not a second one.
        let same = |p: &&crate::patches::Patch| match &source {
            PatchSource::Plugin { component_id, origin, state } if !origin.is_user() => {
                p.source.same_plugin_origin(component_id, origin)
                    || matches!(&p.source, PatchSource::Plugin { component_id: c, state: s, .. } if c == component_id && same_settings(s, state))
            }
            _ => p.source == source,
        };
        if let Some(p) = self.sound_patches().iter().find(same) {
            return Ok(p.id.clone());
        }
        match source {
            PatchSource::SoundFont { file, bank, program } => {
                self.sound_library_cmd(SoundLibraryCmd::AddPresetAsPatch { file, bank, program, name: None })?
            }
            source @ PatchSource::Plugin { .. } => {
                let (name, category) = (self.sound_name(id), self.plugin_category_of(id));
                let patch = PatchFields { name, category, tags: Vec::new(), favourite: false, source };
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
        } else if let Some((plugin, preset)) = parse_plugin_id(id) {
            match preset {
                None => self.plugins_state().list.iter().any(|p| p.id == plugin),
                Some(key) => self.plugin_preset_lists().iter().any(|l| l.plugin == plugin && l.presets.iter().any(|p| p.key == key)),
            }
        } else {
            false
        };
        if known { Ok(()) } else { self.fail(format!("no sound {id}")) }
    }

    /// A plugin's (or plugin preset's) category: the user's, else the guess.
    fn plugin_category_of(&self, id: &str) -> Category {
        let (plugin, preset) = parse_plugin_id(id).unwrap_or((id, None));
        let list = self.plugins_state().list;
        let p = list.iter().find(|p| p.id == plugin);
        let pid = format!("au:{plugin}");
        let cat = self.sounds.prefs.plugin_category(&pid, p.map_or("", |p| &p.name), p.map_or("", |p| &p.manufacturer));
        match preset.and_then(|k| self.plugin_preset(plugin, k)) {
            Some(q) => self.sounds.prefs.preset_category(id, &q.name, q.folder.as_deref(), cat),
            None => cat,
        }
    }

    fn plugin_preset(&self, plugin: &str, key: &str) -> Option<crate::api::PluginPresetEntry> {
        self.plugin_preset_lists().into_iter().find(|l| l.plugin == plugin)?.presets.into_iter().find(|p| p.key == key)
    }

    /// A plugin's name, or "Plugin · Preset" for one of its presets.
    fn sound_name(&self, id: &str) -> String {
        let (plugin, preset) = parse_plugin_id(id).unwrap_or((id, None));
        let name = self.plugins_state().list.into_iter().find(|p| p.id == plugin).map_or_else(|| plugin.to_string(), |p| p.name);
        match preset.and_then(|k| self.plugin_preset(plugin, k)) {
            Some(q) => format!("{name} · {}", q.name),
            None => name,
        }
    }

    /// A font preset's name, as the catalog lists it (None for a preset not in the folder,
    /// or one with no name).
    pub(super) fn font_preset_name(&self, f: &crate::patches::FontPreset) -> Option<String> {
        let p = self.sounds.presets.get(&f.file)?.iter().find(|p| p.bank == f.bank && p.program == f.program)?;
        Some(p.name.trim().to_string()).filter(|n| !n.is_empty())
    }

    fn save_sounds(&mut self, key: &str, value: serde_json::Result<serde_json::Value>) {
        let r = value.map_err(anyhow::Error::from).and_then(|v| write_key(self.sound_settings.as_deref(), key, v));
        if let Err(e) = r {
            self.say(format!("The sound browser settings were not saved: {e:#}"), true);
        }
    }
}

/// Whether two plugin states (base64) are the same settings: the same bytes, or the same
/// property list in another form (an `.aupreset` is the XML form of the state it saved).
/// An empty state (the plugin's default, or a factory preset not captured yet) is none.
fn same_settings(a: &str, b: &str) -> bool {
    if a.is_empty() || b.is_empty() {
        return false;
    }
    if a == b {
        return true;
    }
    #[cfg(feature = "plugins")]
    if let (Some(a), Some(b)) = (crate::api::base64_decode(a), crate::api::base64_decode(b)) {
        return crate::plugin::presets::same_settings(&a, &b);
    }
    false
}

#[cfg(test)]
#[path = "sounds_tests.rs"]
mod tests;

/// The counts `sounds_state` takes in place, and the presets read only when the font list
/// changes: each against what the full lists say.
#[cfg(test)]
mod cache_tests {
    use super::*;
    use crate::patches::sf2::tiny_sound_font;
    use crate::session::{Options, Session};

    /// `SoundsState::count` as the full lists give it (what `sounds_state` built before).
    fn listed_count(ctl: &Control) -> u32 {
        let presets: usize = ctl.sound_fonts.iter().filter_map(|f| ctl.sounds.presets.get(f)).map(Vec::len).sum();
        let plugin_presets: usize = ctl.plugin_preset_lists().iter().map(|l| l.presets.len()).sum();
        (presets + ctl.plugins_state().list.len() + plugin_presets + ctl.sound_patches().len()) as u32
    }

    fn offline(tag: &str) -> (Session, std::path::PathBuf) {
        let data = std::env::temp_dir().join(format!("yahaha-sounds-cache-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&data);
        let sf = data.join("sf");
        std::fs::create_dir_all(&sf).unwrap();
        std::fs::write(sf.join("A.sf2"), tiny_sound_font(&[(0, 0, "Piano"), (0, 33, "Finger Bass")])).unwrap();
        let style = crate::session::testing::style_path();
        let opts = Options { paths: vec![style], data_dir: Some(data.clone()), sound_font_dir: Some(sf), ..Options::default() };
        (Session::offline(opts).unwrap(), data)
    }

    #[test]
    fn fonts_are_read_again_only_when_the_list_changes() {
        let (s, data) = offline("fonts");
        let mut ctl = s.inner.lock();
        ctl.sounds_touch();
        assert_eq!(ctl.sounds_state().count, 2);
        assert_eq!(ctl.sounds_state().count, listed_count(&ctl));
        // A font comes into the folder: its presets are read, and the revision moves.
        std::fs::write(data.join("sf/B.sf2"), tiny_sound_font(&[(0, 88, "Warm Pad"), (0, 89, "Choir")])).unwrap();
        ctl.sound_fonts.push("B.sf2".into());
        assert!(ctl.sounds_touch().is_some());
        assert_eq!(ctl.sounds_state().count, 4);
        assert_eq!(ctl.sounds_state().count, listed_count(&ctl));
        // Unchanged: the file is not read again (a changed file on disk stays as read).
        std::fs::write(data.join("sf/B.sf2"), tiny_sound_font(&[(0, 88, "Warm Pad")])).unwrap();
        assert!(ctl.sounds_touch().is_none());
        assert_eq!(ctl.sounds.presets["B.sf2"].len(), 2);
        // It leaves: forgotten, and the count drops.
        ctl.sound_fonts.retain(|f| f != "B.sf2");
        assert!(ctl.sounds_touch().is_some());
        assert!(!ctl.sounds.presets.contains_key("B.sf2"));
        assert_eq!(ctl.sounds_state().count, 2);
        // Back again: read afresh.
        ctl.sound_fonts.push("B.sf2".into());
        assert!(ctl.sounds_touch().is_some());
        assert_eq!(ctl.sounds.presets["B.sf2"].len(), 1);
        assert_eq!(ctl.sounds_state().count, listed_count(&ctl));
        drop(ctl);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[cfg(feature = "plugins")]
    #[test]
    fn plugin_counts_match_the_listed_presets() {
        use crate::plugin::{FactoryPreset, PluginFormat, PluginId, PluginInfo, UserPreset};
        let info = |id: &str, factory: Option<usize>, user: usize| PluginInfo {
            id: PluginId::parse(id).unwrap(),
            name: "Sampler Deluxe".into(),
            manufacturer: "Fake Instruments".into(),
            version: 0x10000,
            format: PluginFormat::Au2,
            requires_async: false,
            can_load_in_process: false,
            sandbox_safe: true,
            last_load: None,
            in_process: false,
            factory_presets: factory.map(|n| (0..n as i32).map(|number| FactoryPreset { number, name: format!("F{number}") }).collect()),
            user_presets: (0..user)
                .map(|i| UserPreset { name: format!("U{i}"), path: format!("/nowhere/U{i}.aupreset").into(), folder: None })
                .collect(),
        };
        let (s, data) = offline("plugins");
        let mut ctl = s.inner.lock();
        ctl.sounds_touch();
        let base = ctl.sounds_state().count;
        ctl.plugins.list = vec![info("aumu Smp7 Fake", Some(3), 2), info("aumu Org1 Fake", None, 0), info("aumu Pad1 Fake", Some(0), 1)];
        // A failed listing lists the plugin with no presets: it adds none.
        ctl.plugins.listing_failed.insert("aumu Org1 Fake".into(), "timed out".into());
        assert_eq!(ctl.sounds_state().count, base + 3 + 3 + 2 + 1);
        assert_eq!(ctl.sounds_state().count, listed_count(&ctl));
        assert_eq!(ctl.sounds_state().scanning, ctl.plugins_state().scanning);
        let (_tx, rx) = std::sync::mpsc::channel();
        ctl.plugins.scan_rx = Some(rx);
        assert!(ctl.sounds_state().scanning && ctl.plugins_state().scanning);
        ctl.plugins.scan_rx = None;
        drop(ctl);
        let _ = std::fs::remove_dir_all(&data);
    }
}

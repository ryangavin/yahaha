//! New and missing plugins (docs/racks.md, "Plugins coming and going").
//!
//! After each plugin scan (the start-up scan, or `rescanPlugins`) the installed list is
//! compared with the plugins yahaha has seen before, kept in `<data>/known-plugins.json`:
//!
//! - A plugin found for the first time is **new** until it is opened (`markPluginSeen`) or
//!   plays on a part. The first scan ever (no file yet) only records what is there, so
//!   nothing is new on a fresh install.
//! - A plugin seen before that isn't installed now is **missing**. It stays in the file
//!   with its name and vendor, so it can still be shown, and so reinstalling it doesn't
//!   make it new.
//!
//! The state also says how many of the user's racks (`<data>/Racks`) and library sounds
//! use each plugin, and which racks need attention (a part's sound is on a missing
//! plugin). The racks folder is read after each scan and whenever it changes (looked at
//! every 2 s). Nothing here writes a rack or a sound. A keyboard part whose plugin is
//! missing is silent (session/plugins.rs).

use super::Control;
use crate::api::{MissingPlugin, PluginsState, RackAttention};
use crate::data_files::write_atomic;
use crate::patches::PatchSource;
use crate::racks::{self, Rack, SoundRef};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

/// The file in the data folder.
pub(super) const FILE: &str = "known-plugins.json";
/// Its `format` field.
const FORMAT: &str = "yahaha.known-plugins";
/// The version this build reads and writes.
const VERSION: u32 = 1;
/// How often the racks folder is looked at for changes.
const RACKS_EVERY_NS: u64 = 2_000_000_000;

/// A plugin yahaha has seen installed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Known {
    /// "aumu Xf2X XFER".
    pub(super) id: String,
    /// Its name and vendor as last scanned.
    pub(super) name: String,
    #[serde(default)]
    pub(super) manufacturer: String,
    /// Found by a scan for the first time, and not opened or played since.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub(super) new: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct KnownFile {
    format: String,
    version: u32,
    plugins: Vec<Known>,
}

/// What a user rack's parts play.
#[derive(Debug)]
struct RackUse {
    id: String,
    name: String,
    sounds: Vec<SoundRef>,
}

/// The control side's record of the plugins seen, and the racks that use them.
#[derive(Debug, Default)]
pub(super) struct Presence {
    /// `<data>/known-plugins.json` (None: nothing is saved).
    file: Option<PathBuf>,
    known: Vec<Known>,
    /// There is a record of what was installed before (the file, or an earlier scan):
    /// a plugin a scan finds that isn't in it is new.
    baseline: bool,
    /// The file is a newer yahaha's: it is never saved over.
    locked: bool,
    /// A scan is in, so what isn't in the plugin list is not installed.
    pub(super) scanned: bool,
    racks_dir: Option<PathBuf>,
    racks: Vec<RackUse>,
    /// The racks folder as last read (its files, their sizes and times).
    racks_key: Option<u64>,
    racks_ns: u64,
}

impl Presence {
    /// The record in data folder `data` (None: kept in memory only).
    pub(super) fn open(data: Option<&Path>) -> Presence {
        let mut p = Presence { file: data.map(|d| d.join(FILE)), racks_dir: data.map(racks::dir), ..Presence::default() };
        let Some(text) = p.file.as_deref().and_then(|f| std::fs::read_to_string(f).ok()) else { return p };
        let v: Value = serde_json::from_str(&text).unwrap_or_default();
        let version = v.get("version").and_then(Value::as_u64).unwrap_or(0);
        if v.get("format").and_then(Value::as_str) == Some(FORMAT) && version > VERSION as u64 {
            // A newer yahaha's: its plugins still count as seen, and it is left as it is.
            p.locked = true;
            p.baseline = true;
            if let Some(list) = v.get("plugins").cloned() {
                p.known = serde_json::from_value(list).unwrap_or_default();
            }
            return p;
        }
        // A damaged file is only a record of what was installed: it starts again.
        if let Ok(f) = serde_json::from_value::<KnownFile>(v)
            && f.format == FORMAT
        {
            p.known = f.plugins;
            p.baseline = true;
        }
        p
    }

    /// A scan found `installed` (id, name, vendor): the plugins not seen before are new
    /// (none on the very first scan), and the names of those seen are brought up to date.
    #[cfg_attr(not(feature = "plugins"), allow(dead_code))]
    pub(super) fn note_scan(&mut self, installed: &[(String, String, String)]) -> anyhow::Result<()> {
        let first = !self.baseline;
        let mut changed = first;
        for (id, name, manufacturer) in installed {
            match self.known.iter_mut().find(|k| k.id == *id) {
                Some(k) => {
                    if k.name != *name || k.manufacturer != *manufacturer {
                        (k.name, k.manufacturer) = (name.clone(), manufacturer.clone());
                        changed = true;
                    }
                }
                None => {
                    self.known.push(Known { id: id.clone(), name: name.clone(), manufacturer: manufacturer.clone(), new: !first });
                    changed = true;
                }
            }
        }
        self.baseline = true;
        self.scanned = true;
        if changed { self.save() } else { Ok(()) }
    }

    /// Plugin `id` was opened or played: it is no longer new. Returns whether it was.
    pub(super) fn mark_seen(&mut self, id: &str) -> anyhow::Result<bool> {
        match self.known.iter_mut().find(|k| k.id == id && k.new) {
            Some(k) => {
                k.new = false;
                self.save().map(|_| true)
            }
            None => Ok(false),
        }
    }

    pub(super) fn is_new(&self, id: &str) -> bool {
        self.known.iter().any(|k| k.id == id && k.new)
    }

    /// The plugin as it was last installed.
    pub(super) fn known(&self, id: &str) -> Option<&Known> {
        self.known.iter().find(|k| k.id == id)
    }

    fn save(&self) -> anyhow::Result<()> {
        let Some(path) = self.file.as_deref().filter(|_| !self.locked) else { return Ok(()) };
        let f = KnownFile { format: FORMAT.into(), version: VERSION, plugins: self.known.clone() };
        write_atomic(path, &serde_json::to_string_pretty(&f)?)
    }

    /// Read the racks folder again if it changed since it was last read (or `force`).
    pub(super) fn refresh_racks(&mut self, force: bool) {
        let Some(dir) = self.racks_dir.as_deref() else { return };
        let files = racks::list(dir);
        let mut h = DefaultHasher::new();
        for f in &files {
            let m = std::fs::metadata(f).ok();
            (f, m.as_ref().map(|m| m.len()), m.and_then(|m| m.modified().ok())).hash(&mut h);
        }
        let key = h.finish();
        if !force && self.racks_key == Some(key) {
            return;
        }
        self.racks_key = Some(key);
        // A rack that can't be read (a newer yahaha's, damaged) uses nothing we can tell.
        self.racks = files
            .iter()
            .filter_map(|f| Rack::load(f).ok())
            .map(|r| RackUse { id: r.id, name: r.name, sounds: r.parts.into_iter().map(|p| p.sound).collect() })
            .collect();
    }
}

impl Control {
    /// A plugin scan is in: new and missing plugins, and the racks read again.
    #[cfg_attr(not(feature = "plugins"), allow(dead_code))]
    pub(super) fn note_plugin_scan(&mut self, installed: &[(String, String, String)]) {
        if let Err(e) = self.presence.note_scan(installed) {
            self.say(format!("The list of plugins seen was not saved: {e:#}"), true);
        }
        self.presence.refresh_racks(true);
    }

    /// `markPluginSeen`, or plugin `id` played: it is no longer new.
    pub(super) fn mark_plugin_seen(&mut self, id: &str) {
        if let Err(e) = self.presence.mark_seen(id) {
            self.say(format!("The list of plugins seen was not saved: {e:#}"), true);
        }
    }

    /// Now and then: the racks folder read again if it changed.
    pub(super) fn pump_plugin_presence(&mut self, now: u64) {
        if now.saturating_sub(self.presence.racks_ns) >= RACKS_EVERY_NS {
            self.presence.racks_ns = now;
            self.presence.refresh_racks(false);
        }
    }

    /// The plugins as the app sees them: installed ones with whether each is new and how
    /// many racks and sounds use it; the missing ones; the racks that need attention.
    pub(super) fn plugins_app_state(&self) -> PluginsState {
        let mut st = self.plugins_state();
        // The plugin each library sound plays, and how many sounds play each plugin.
        let lib: HashMap<&str, &str> = self
            .sound_patches()
            .iter()
            .filter_map(|p| match &p.source {
                PatchSource::Plugin { component_id, .. } => Some((p.id.as_str(), component_id.as_str())),
                PatchSource::SoundFont { .. } => None,
            })
            .collect();
        let mut sounds: HashMap<&str, u32> = HashMap::new();
        for c in lib.values() {
            *sounds.entry(c).or_default() += 1;
        }
        fn plays<'a>(lib: &HashMap<&'a str, &'a str>, s: &'a SoundRef) -> Option<&'a str> {
            match s {
                SoundRef::Plugin { component } => Some(component.as_str()),
                SoundRef::Library { id } => lib.get(id.as_str()).copied(),
                SoundRef::Font { .. } => None,
            }
        }
        let component = |s| plays(&lib, s);
        // How many racks play each plugin (a rack counts once, however many parts).
        let mut racks: HashMap<&str, u32> = HashMap::new();
        for r in &self.presence.racks {
            let used: BTreeSet<&str> = r.sounds.iter().filter_map(component).collect();
            for c in used {
                *racks.entry(c).or_default() += 1;
            }
        }
        let count = |m: &HashMap<&str, u32>, id: &str| m.get(id).copied().unwrap_or(0);
        for e in &mut st.list {
            e.new = self.presence.is_new(&e.id);
            e.racks = count(&racks, &e.id);
            e.sounds = count(&sounds, &e.id);
        }
        // Before the first scan, nothing is known to be missing.
        if !self.presence.scanned {
            return st;
        }
        let installed: HashSet<&str> = st.list.iter().map(|e| e.id.as_str()).collect();
        let gone: BTreeSet<&str> = self
            .presence
            .known
            .iter()
            .map(|k| k.id.as_str())
            .chain(racks.keys().copied())
            .chain(sounds.keys().copied())
            .filter(|id| !installed.contains(id))
            .collect();
        let mut missing: Vec<MissingPlugin> = gone
            .into_iter()
            .map(|id| {
                let k = self.presence.known(id);
                MissingPlugin {
                    id: id.to_string(),
                    name: k.map_or_else(|| id.to_string(), |k| k.name.clone()),
                    manufacturer: k.map(|k| k.manufacturer.clone()).unwrap_or_default(),
                    racks: count(&racks, id),
                    sounds: count(&sounds, id),
                }
            })
            .collect();
        missing.sort_by_cached_key(|m| (m.name.to_lowercase(), m.id.clone()));
        let needs_attention: Vec<RackAttention> = self
            .presence
            .racks
            .iter()
            .filter_map(|r| {
                let parts: Vec<u8> =
                    (0..r.sounds.len()).filter(|&p| component(&r.sounds[p]).is_some_and(|c| !installed.contains(c))).map(|p| p as u8).collect();
                (!parts.is_empty()).then(|| RackAttention { id: r.id.clone(), name: r.name.clone(), parts })
            })
            .collect();
        (st.missing, st.needs_attention) = (missing, needs_attention);
        st
    }
}

#[cfg(test)]
#[path = "plugin_presence_tests.rs"]
mod tests;

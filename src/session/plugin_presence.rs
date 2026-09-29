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
use crate::api::{MissingPlugin, PluginEntry, PluginsState, RackAttention};
use crate::data_files::write_atomic;
use crate::patches::{Patch, PatchSource};
use crate::racks::{self, Rack, SoundRef};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cell::RefCell;
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

/// What a user rack's parts play, and where its file is (the rack commands and the
/// racks list read it from here).
#[derive(Debug)]
pub(super) struct RackUse {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) sounds: Vec<SoundRef>,
    pub(super) on: Vec<bool>,
    pub(super) path: PathBuf,
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
    /// Moves whenever `known`, `scanned` or `racks` may have changed: part of the key of
    /// `usage`.
    rev: u64,
    /// `rev` as the last `pump_plugin_presence` saw it.
    pumped_rev: u64,
    /// What `plugins_app_state` adds to the plugin list, as last worked out.
    usage: RefCell<Option<Usage>>,
}

/// What the app state adds to the plugin list (whether each is new, what uses it, the
/// missing plugins, the racks that need attention), and exactly what it was built from:
/// the presence revision, the library's plugin sounds and the installed ids, in order.
#[derive(Debug)]
struct Usage {
    rev: u64,
    /// Each library sound on a plugin: its id and the plugin, in the library's order.
    lib: Vec<(String, String)>,
    /// The installed plugins' ids, in the list's order.
    installed: Vec<String>,
    /// For each of those: new, racks, sounds.
    rows: Vec<(bool, u32, u32)>,
    missing: Vec<MissingPlugin>,
    needs_attention: Vec<RackAttention>,
}

/// Each library sound on a plugin: its id and the plugin's.
fn plugin_sounds(patches: &[Patch]) -> impl Iterator<Item = (&str, &str)> {
    patches.iter().filter_map(|p| match &p.source {
        PatchSource::Plugin { component_id, .. } => Some((p.id.as_str(), component_id.as_str())),
        PatchSource::SoundFont { .. } => None,
    })
}

impl Usage {
    /// Built from exactly these inputs.
    fn fits(&self, rev: u64, patches: &[Patch], list: &[PluginEntry]) -> bool {
        self.rev == rev
            && self.installed.len() == list.len()
            && self.installed.iter().zip(list).all(|(a, e)| *a == e.id)
            && plugin_sounds(patches).eq(self.lib.iter().map(|(a, b)| (a.as_str(), b.as_str())))
    }
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
        self.rev += 1;
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
                self.rev += 1;
                self.save().map(|_| true)
            }
            None => Ok(false),
        }
    }

    #[cfg(test)]
    pub(super) fn is_new(&self, id: &str) -> bool {
        self.known.iter().any(|k| k.id == id && k.new)
    }

    /// The plugin as it was last installed.
    #[cfg_attr(not(feature = "plugins"), allow(dead_code))]
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
        self.rev += 1;
        // A rack that can't be read (a newer yahaha's, damaged) uses nothing we can tell.
        self.racks = files
            .iter()
            .filter_map(|f| Rack::load(f).ok().map(|r| (f, r)))
            .map(|(f, r)| RackUse {
                id: r.id,
                name: r.name,
                on: r.parts.iter().map(|p| p.on).collect(),
                sounds: r.parts.into_iter().map(|p| p.sound).collect(),
                path: f.clone(),
            })
            .collect();
    }

    /// The racks folder (None: no data folder, so racks can't be saved).
    pub(super) fn racks_dir(&self) -> Option<&Path> {
        self.racks_dir.as_deref()
    }

    /// The user's racks as last read.
    pub(super) fn racks(&self) -> &[RackUse] {
        &self.racks
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

    /// Now and then: the racks folder read again if it changed (looked at every
    /// `RACKS_EVERY_NS`). Returns whether the record or the racks changed since the last
    /// pump (here or by a command): the plugin list's `new`, `racks`, `missing` and
    /// `needsAttention`, and the racks lists, show them. Nothing here runs in the
    /// background.
    pub(super) fn pump_plugin_presence(&mut self, now: u64) -> bool {
        // The first pump reads them at once, so the racks list is there from the start.
        if self.presence.racks_key.is_none() || now.saturating_sub(self.presence.racks_ns) >= RACKS_EVERY_NS {
            self.presence.racks_ns = now;
            self.presence.refresh_racks(false);
        }
        let changed = self.presence.rev != self.presence.pumped_rev;
        self.presence.pumped_rev = self.presence.rev;
        changed
    }

    /// The plugins as the app sees them: installed ones with whether each is new and how
    /// many racks and sounds use it; the missing ones; the racks that need attention. What
    /// is added to the list is worked out again only when what it is made of changed.
    pub(super) fn plugins_app_state(&self) -> PluginsState {
        let mut st = self.plugins_state();
        let (rev, patches) = (self.presence.rev, self.sound_patches());
        let fresh = self.presence.usage.borrow().as_ref().is_some_and(|u| u.fits(rev, patches, &st.list));
        if !fresh {
            let u = self.plugin_usage(&st.list);
            *self.presence.usage.borrow_mut() = Some(u);
        }
        let usage = self.presence.usage.borrow();
        let Some(u) = usage.as_ref() else { return st };
        for (e, &(new, racks, sounds)) in st.list.iter_mut().zip(&u.rows) {
            (e.new, e.racks, e.sounds) = (new, racks, sounds);
        }
        st.missing.clone_from(&u.missing);
        st.needs_attention.clone_from(&u.needs_attention);
        st
    }

    /// What `plugins_app_state` adds to installed plugins `list`.
    fn plugin_usage(&self, list: &[PluginEntry]) -> Usage {
        let patches = self.sound_patches();
        let mut u = Usage {
            rev: self.presence.rev,
            lib: plugin_sounds(patches).map(|(a, b)| (a.to_string(), b.to_string())).collect(),
            installed: list.iter().map(|e| e.id.clone()).collect(),
            rows: Vec::new(),
            missing: Vec::new(),
            needs_attention: Vec::new(),
        };
        // The plugin each library sound plays, and how many sounds play each plugin.
        let lib: HashMap<&str, &str> = plugin_sounds(patches).collect();
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
        let new: HashSet<&str> = self.presence.known.iter().filter(|k| k.new).map(|k| k.id.as_str()).collect();
        u.rows = list.iter().map(|e| (new.contains(e.id.as_str()), count(&racks, &e.id), count(&sounds, &e.id))).collect();
        // Before the first scan, nothing is known to be missing.
        if !self.presence.scanned {
            return u;
        }
        // The first record of each id, as `Presence::known` finds it.
        let mut known: HashMap<&str, &Known> = HashMap::new();
        for k in &self.presence.known {
            known.entry(k.id.as_str()).or_insert(k);
        }
        let installed: HashSet<&str> = list.iter().map(|e| e.id.as_str()).collect();
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
                let k = known.get(id).copied();
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
        (u.missing, u.needs_attention) = (missing, needs_attention);
        u
    }
}

#[cfg(test)]
#[path = "plugin_presence_tests.rs"]
mod tests;

/// The cached additions to the plugin list follow every change of what they are made of,
/// and the pump says when the record or the racks changed.
#[cfg(test)]
mod cache_tests {
    use super::*;
    use crate::patches::Category;
    use crate::session::testing::{data_dir, session_in};

    const SAMPLER: &str = "aumu Smp7 Fake";
    const ORGAN: &str = "aumu Org1 Fake";
    const PADS: &str = "aumu Pad1 Fake";

    /// The state as built now, checked against one built with no cache.
    fn check(ctl: &Control) -> PluginsState {
        let cached = ctl.plugins_app_state();
        *ctl.presence.usage.borrow_mut() = None;
        assert_eq!(cached, ctl.plugins_app_state(), "the cache went stale");
        cached
    }

    fn missing(st: &PluginsState, id: &str) -> Option<(u32, u32)> {
        st.missing.iter().find(|m| m.id == id).map(|m| (m.racks, m.sounds))
    }

    fn installed(id: &str, name: &str) -> (String, String, String) {
        (id.into(), name.into(), "Fake Instruments".into())
    }

    #[test]
    fn plugin_usage_follows_the_library_racks_and_record() {
        let d = data_dir("presence-cache");
        let s = session_in(&d);
        let mut rack = s.capture_rack("One");
        rack.parts[2].sound = SoundRef::Library { id: "keys".into() };
        let mut ctl = s.inner.lock();
        let t = 1_000_000_000_000_000;
        ctl.pump_plugin_presence(t);
        assert!(!ctl.pump_plugin_presence(t), "nothing changed");
        // Before a scan nothing is missing; after one, a plugin gone is.
        assert!(check(&ctl).missing.is_empty());
        ctl.presence.note_scan(&[installed(SAMPLER, "Sampler Deluxe")]).unwrap();
        ctl.presence.note_scan(&[]).unwrap();
        assert!(ctl.pump_plugin_presence(t), "a scan is a change");
        assert_eq!(missing(&check(&ctl), SAMPLER), Some((0, 0)));
        // A library sound on it.
        let sound = Patch {
            id: "keys".into(),
            name: "Deluxe Keys".into(),
            category: Category::Piano,
            tags: Vec::new(),
            favourite: false,
            source: PatchSource::plugin(SAMPLER, ""),
        };
        ctl.sound.lib.patches.push(sound);
        assert_eq!(missing(&check(&ctl), SAMPLER), Some((0, 1)));
        // The sound moved to another plugin, never installed here.
        let PatchSource::Plugin { component_id, .. } = &mut ctl.sound.lib.patches.last_mut().unwrap().source else { unreachable!() };
        *component_id = ORGAN.into();
        let st = check(&ctl);
        assert_eq!((missing(&st, SAMPLER), missing(&st, ORGAN)), (Some((0, 0)), Some((0, 1))));
        // A rack playing that sound, seen at the next look at the folder.
        rack.save(&racks::path_for(&racks::dir(&d), "One")).unwrap();
        assert!(!ctl.pump_plugin_presence(t + 1), "not looked at yet");
        assert!(ctl.pump_plugin_presence(t + RACKS_EVERY_NS));
        let st = check(&ctl);
        assert_eq!(missing(&st, ORGAN), Some((1, 1)));
        assert_eq!(st.needs_attention.iter().map(|r| (r.name.as_str(), r.parts.clone())).collect::<Vec<_>>(), [("One", vec![2u8])]);
        assert!(!ctl.pump_plugin_presence(t + 2 * RACKS_EVERY_NS), "the folder is as it was");
        // The sound gone: the rack's part plays nothing we know.
        ctl.sound.lib.patches.retain(|p| p.id != "keys");
        let st = check(&ctl);
        assert_eq!((missing(&st, ORGAN), st.needs_attention.len()), (None, 0));
        // A new plugin, then seen.
        ctl.presence.note_scan(&[installed(PADS, "Pad Machine")]).unwrap();
        ctl.pump_plugin_presence(t + 2 * RACKS_EVERY_NS);
        assert!(ctl.presence.mark_seen(PADS).unwrap());
        assert!(ctl.pump_plugin_presence(t + 2 * RACKS_EVERY_NS), "no longer new");
        check(&ctl);
        drop(ctl);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// With installed plugins: `new` and the counts on each row follow the record and the
    /// list (a scan replaces it).
    #[cfg(feature = "plugins")]
    #[test]
    fn rows_follow_the_installed_list() {
        use crate::plugin::{PluginFormat, PluginId, PluginInfo};
        let info = |id: &str, name: &str| PluginInfo {
            id: PluginId::parse(id).unwrap(),
            name: name.into(),
            manufacturer: "Fake Instruments".into(),
            version: 0x10000,
            format: PluginFormat::Au2,
            requires_async: false,
            can_load_in_process: false,
            sandbox_safe: true,
            last_load: None,
            in_process: false,
            factory_presets: None,
            user_presets: Vec::new(),
        };
        let d = data_dir("presence-cache-rows");
        let s = session_in(&d);
        let mut ctl = s.inner.lock();
        ctl.presence.note_scan(&[installed(SAMPLER, "Sampler Deluxe")]).unwrap();
        ctl.presence.note_scan(&[installed(SAMPLER, "Sampler Deluxe"), installed(PADS, "Pad Machine")]).unwrap();
        ctl.plugins.list = vec![info(PADS, "Pad Machine"), info(SAMPLER, "Sampler Deluxe")];
        let row = |st: &PluginsState, id: &str| st.list.iter().find(|e| e.id == id).map(|e| (e.new, e.racks, e.sounds));
        let st = check(&ctl);
        assert_eq!((row(&st, PADS), row(&st, SAMPLER)), (Some((true, 0, 0)), Some((false, 0, 0))));
        assert!(st.missing.is_empty());
        // The same ids in another order: the rows go with their plugins.
        ctl.plugins.list.reverse();
        let st = check(&ctl);
        assert_eq!((row(&st, PADS), row(&st, SAMPLER)), (Some((true, 0, 0)), Some((false, 0, 0))));
        ctl.presence.mark_seen(PADS).unwrap();
        assert_eq!(row(&check(&ctl), PADS), Some((false, 0, 0)));
        // A sound on the sampler, then the sampler uninstalled.
        ctl.sound.lib.patches.push(Patch {
            id: "keys".into(),
            name: "Deluxe Keys".into(),
            category: Category::Piano,
            tags: Vec::new(),
            favourite: false,
            source: PatchSource::plugin(SAMPLER, ""),
        });
        assert_eq!(row(&check(&ctl), SAMPLER), Some((false, 0, 1)));
        ctl.plugins.list.retain(|p| p.id.to_string() != SAMPLER);
        let st = check(&ctl);
        assert_eq!((row(&st, SAMPLER), missing(&st, SAMPLER)), (None, Some((0, 1))));
        drop(ctl);
        let _ = std::fs::remove_dir_all(&d);
    }
}

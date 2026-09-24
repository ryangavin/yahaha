//! The sound library on disk: one versioned JSON file, `sound-library.json` in the data
//! folder, with import and export.
//!
//! Versions:
//!
//! - no `version` field, a bare array: a list of patches (the simplest thing to write by
//!   hand or share). It becomes a library with those patches and empty maps.
//! - 1: `{ "version": 1, "patches": [...], "map": {...}, "styleMaps": {...},
//!   "portSendsMapped": false }`, this module's [`SoundLibrary`].
//!
//! A file from a newer yahaha (a higher version) is refused rather than half read, and
//! never overwritten.

use super::{new_id, Patch, ProgramMap};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// The format version this build writes.
pub const VERSION: u32 = 1;

/// The file name in the data folder.
pub const FILE_NAME: &str = "sound-library.json";

/// How many patches a library holds at most. It is meant to be small (about 20): the
/// limit only stops a runaway import.
pub const MAX_PATCHES: usize = 256;

/// The library: patches in the user's order, the global program map, the per-style maps
/// and the port setting.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundLibrary {
    pub version: u32,
    #[serde(default)]
    pub patches: Vec<Patch>,
    #[serde(default)]
    pub map: ProgramMap,
    /// Per-style maps, by the style's file name (not in the style file, which stays the
    /// style's own).
    #[serde(default)]
    pub style_maps: BTreeMap<String, ProgramMap>,
    /// The `yahaha` MIDI port gets the mapped bank and program instead of the style's own
    /// program changes.
    #[serde(default)]
    pub port_sends_mapped: bool,
}

impl Default for SoundLibrary {
    fn default() -> SoundLibrary {
        SoundLibrary { version: VERSION, patches: Vec::new(), map: ProgramMap::default(), style_maps: BTreeMap::new(), port_sends_mapped: false }
    }
}

impl SoundLibrary {
    /// Parse a library file of any version this build knows, migrated to the current one.
    pub fn from_json(text: &str) -> Result<SoundLibrary> {
        let v: serde_json::Value = serde_json::from_str(text).context("not JSON")?;
        let mut lib = match &v {
            serde_json::Value::Array(_) => {
                let patches: Vec<Patch> = serde_json::from_value(v).context("a patch list")?;
                SoundLibrary { patches, ..SoundLibrary::default() }
            }
            serde_json::Value::Object(o) => {
                let version = o.get("version").and_then(|x| x.as_u64()).context("no version")?;
                if version > VERSION as u64 {
                    bail!("made by a newer yahaha (format {version}; this one reads up to {VERSION})");
                }
                if version == 0 {
                    bail!("format 0 is not a sound library");
                }
                serde_json::from_value(v).context("a sound library")?
            }
            _ => bail!("not a sound library"),
        };
        lib.version = VERSION;
        lib.normalize();
        Ok(lib)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Load from `path`; a missing file is an empty library.
    pub fn load(path: &Path) -> Result<SoundLibrary> {
        match std::fs::read_to_string(path) {
            Ok(text) => SoundLibrary::from_json(&text).with_context(|| format!("{}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(SoundLibrary::default()),
            Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
        }
    }

    /// Save to `path` (written to a temporary file first, then renamed over it).
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, self.to_json()).with_context(|| format!("writing {}", tmp.display()))?;
        std::fs::rename(&tmp, path).with_context(|| format!("saving {}", path.display()))
    }

    pub fn patch(&self, id: &str) -> Option<&Patch> {
        self.patches.iter().find(|p| p.id == id)
    }

    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.patches.iter().position(|p| p.id == id)
    }

    /// Ids unique and non-empty, names non-empty, defaults in range, every rule naming a
    /// patch that exists.
    pub fn normalize(&mut self) {
        self.patches.truncate(MAX_PATCHES);
        let mut seen: Vec<String> = Vec::new();
        for p in &mut self.patches {
            if p.name.trim().is_empty() {
                p.name = "Patch".into();
            }
            if p.id.is_empty() || seen.contains(&p.id) {
                p.id = new_id(&p.name, seen.iter().map(String::as_str));
            }
            p.defaults = p.defaults.clamped();
            seen.push(p.id.clone());
        }
        let ids: Vec<String> = seen;
        let prune = |m: &mut ProgramMap| {
            m.normalize();
            let gone: Vec<String> = m
                .families
                .iter()
                .flatten()
                .chain(m.overrides.iter().map(|o| &o.patch))
                .chain(m.drums.iter())
                .filter(|id| !ids.contains(id))
                .cloned()
                .collect();
            for id in gone {
                m.forget(&id);
            }
        };
        prune(&mut self.map);
        for m in self.style_maps.values_mut() {
            prune(m);
        }
        self.style_maps.retain(|_, m| !m.is_empty());
    }

    /// Add patches from another library (an import): each keeps its id unless this one has
    /// it already, when it gets a new one (and the imported maps follow). With `maps`, the
    /// imported maps' rules are added too (theirs win where both have one). Returns how
    /// many patches were added.
    pub fn merge(&mut self, other: SoundLibrary, maps: bool) -> usize {
        let mut other = other;
        let room = MAX_PATCHES.saturating_sub(self.patches.len());
        let incoming: Vec<Patch> = std::mem::take(&mut other.patches).into_iter().take(room).collect();
        // Every imported patch's new id first, unique against both libraries (and the
        // ids handed out so far), then the imported maps rewritten once through the table:
        // renaming one at a time would move an earlier patch's rules onto a later one
        // whose original id a new id happens to equal.
        let mut taken: Vec<String> = self.patches.iter().map(|p| p.id.clone()).chain(incoming.iter().map(|p| p.id.clone())).collect();
        let mut table: Vec<(String, String)> = Vec::new();
        let mut added = Vec::new();
        for mut p in incoming {
            let old = p.id.clone();
            if self.patch(&old).is_some() || table.iter().any(|(_, n)| *n == old) {
                p.id = new_id(&p.name, taken.iter().map(String::as_str));
                taken.push(p.id.clone());
            }
            table.push((old, p.id.clone()));
            added.push(p);
        }
        let n = added.len();
        self.patches.extend(added);
        if maps {
            let rewrite = |m: &ProgramMap| {
                let to = |id: &str| table.iter().find(|(o, _)| o == id).map(|(_, n)| n.clone());
                ProgramMap {
                    families: m.families.clone().map(|f| f.and_then(|id| to(&id))),
                    overrides: m.overrides.iter().filter_map(|o| Some(super::ProgramOverride { program: o.program, patch: to(&o.patch)? })).collect(),
                    drums: m.drums.as_deref().and_then(to),
                }
            };
            merge_map(&mut self.map, &rewrite(&other.map));
            for (k, m) in &other.style_maps {
                let m = rewrite(m);
                merge_map(self.style_maps.entry(k.clone()).or_default(), &m);
            }
        }
        self.normalize();
        n
    }
}

fn merge_map(into: &mut ProgramMap, from: &ProgramMap) {
    for (f, p) in from.families.iter().enumerate() {
        if p.is_some() {
            into.families[f] = p.clone();
        }
    }
    for o in &from.overrides {
        into.set_override(o.program, Some(o.patch.clone()));
    }
    if from.drums.is_some() {
        into.drums = from.drums.clone();
    }
}

/// The key a style's own map is stored under: its file name.
pub fn style_key(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
}

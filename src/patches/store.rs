//! The sound library on disk: one versioned JSON file, `sound-library.json` in the data
//! folder, with import and export.
//!
//! Versions:
//!
//! - no `version` field, a bare array: a list of patches (the simplest thing to write by
//!   hand or share). It becomes a library with those patches and empty maps.
//! - 1: `{ "version": 1, "patches": [...], "map": {...}, "styleMaps": {...},
//!   "portSendsMapped": false }`.
//! - 2: the same, and a plugin patch's source may carry its `origin` (a factory preset by
//!   number, or an `.aupreset` file by path; docs/sound-browser.md). A version 1 file
//!   reads as it is: every plugin patch in it is a `user` sound. Version 2 is written so a
//!   version-1 build refuses the file instead of saving over it and dropping the origins.
//!
//! Export writes a bundle (`{ "kind": "yahaha-sound-bundle", "fonts": [...], "library":
//! {...} }`): the library as above plus the SoundFont file names it plays. Import reads a
//! bundle or any library file.
//!
//! A file from a newer yahaha (a higher version) is refused rather than half read, and
//! never overwritten.

use super::{new_id, Patch, PatchSource, PluginOrigin, ProgramMap, SoundTag};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// The format version this build writes.
pub const VERSION: u32 = 2;

/// The file name in the data folder.
pub const FILE_NAME: &str = "sound-library.json";

/// The `kind` of an export bundle (`SoundLibrary::to_bundle_json`).
pub const BUNDLE_KIND: &str = "yahaha-sound-bundle";

/// An export bundle: `{ "kind": "yahaha-sound-bundle", "fonts": [file names], "library":
/// {...} }`. SoundFonts are referenced by file name, never copied.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Bundle {
    kind: String,
    fonts: Vec<String>,
    library: SoundLibrary,
}

/// How many patches a library holds at most. Every plugin sound lives here (D1: factory
/// presets once played, imported `.aupreset` files), so it is no longer small; the limit
/// only stops a runaway import.
pub const MAX_PATCHES: usize = 4096;

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

    /// The SoundFont files the library's patches play, sorted, each once.
    pub fn fonts_used(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .patches
            .iter()
            .filter_map(|p| match &p.source {
                PatchSource::SoundFont { file, .. } => Some(file.clone()),
                PatchSource::Plugin { .. } => None,
            })
            .collect();
        v.sort();
        v.dedup();
        v
    }

    /// The library as an export bundle (D5): its metadata and maps, every plugin sound's
    /// state (it is in the patches), and the SoundFonts it plays, by file name only.
    pub fn to_bundle_json(&self) -> String {
        let b = Bundle { kind: BUNDLE_KIND.into(), fonts: self.fonts_used(), library: self.clone() };
        serde_json::to_string_pretty(&b).unwrap_or_default()
    }

    /// Read an export bundle, or any library file [`SoundLibrary::from_json`] reads: the
    /// library, and the SoundFont files it names (the bundle's list and every file a
    /// patch plays).
    pub fn read_bundle(text: &str) -> Result<(SoundLibrary, Vec<String>)> {
        let v: serde_json::Value = serde_json::from_str(text).context("not JSON")?;
        let (lib, mut fonts) = match v.get("kind").and_then(|k| k.as_str()) {
            Some(BUNDLE_KIND) => {
                let listed: Vec<String> = v.get("fonts").cloned().map(serde_json::from_value).transpose().context("the bundle's fonts")?.unwrap_or_default();
                let inner = v.get("library").context("the bundle has no library")?.to_string();
                (SoundLibrary::from_json(&inner)?, listed)
            }
            Some(k) => bail!("not a sound bundle ({k})"),
            None => (SoundLibrary::from_json(text)?, Vec::new()),
        };
        fonts.extend(lib.fonts_used());
        fonts.sort();
        fonts.dedup();
        Ok((lib, fonts))
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

    /// The plugin sound for `component`'s preset from `origin` (a factory preset or an
    /// `.aupreset` file), if the library has it. A `user` origin never matches: two sounds
    /// made in yahaha are two sounds.
    pub fn plugin_sound(&self, component: &str, origin: &PluginOrigin) -> Option<&Patch> {
        if origin.is_user() {
            return None;
        }
        self.patches.iter().find(|p| p.source.same_plugin_origin(component, origin))
    }

    /// The library's sound for a plugin preset, added if it has none (the migration of a
    /// part that played a preset before the library held plugin sounds, and the first play
    /// of a preset). `state` (base64) is its state if known: an `.aupreset`'s, read from
    /// the file; a factory preset's is left empty to be captured when it plays. Returns
    /// its id, or None when the library is full.
    pub fn add_plugin_preset(&mut self, component: &str, origin: PluginOrigin, name: &str, category: super::Category, state: Option<String>) -> Option<String> {
        if let Some(p) = self.plugin_sound(component, &origin) {
            return Some(p.id.clone());
        }
        if self.patches.len() >= MAX_PATCHES {
            return None;
        }
        let name = if name.trim().is_empty() { "Sound".to_string() } else { name.to_string() };
        let id = new_id(&name, self.patches.iter().map(|p| p.id.as_str()));
        let source = PatchSource::Plugin { component_id: component.to_string(), state: state.unwrap_or_default(), origin };
        self.patches.push(Patch { id: id.clone(), name, category, tags: Vec::new(), favourite: false, source, defaults: Default::default() });
        Some(id)
    }

    /// Capture a factory preset's state the first time it plays: patch `id` takes `state`
    /// (base64) if it is a plugin sound still waiting for one. Returns whether it changed
    /// (the caller saves the library). A sound with a state keeps it: later edits are the
    /// user's to save (Save / Save as…).
    pub fn capture_state(&mut self, id: &str, state: &str) -> bool {
        let Some(p) = self.patches.iter_mut().find(|p| p.id == id) else { return false };
        if state.is_empty() || !p.awaits_capture() {
            return false;
        }
        if let PatchSource::Plugin { state: s, .. } = &mut p.source {
            *s = state.to_string();
        }
        true
    }

    /// The sound a recalled plugin state is (a Registration or OTS record from before
    /// records named their sound): the library's plugin sound with exactly that instrument
    /// and state (base64), if there is one.
    pub fn tag_for_state(&self, component: &str, state: &str) -> Option<SoundTag> {
        if state.is_empty() {
            return None;
        }
        self.patches
            .iter()
            .find(|p| matches!(&p.source, PatchSource::Plugin { component_id, state: s, .. } if component_id == component && s == state))
            .map(Patch::tag)
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

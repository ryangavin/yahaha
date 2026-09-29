//! Style racks' file (docs/racks.md, "Styles and OTS"): `<data>/style-racks.json`, format
//! `yahaha.style-racks`, version 1. Per style file name (`Soul Ballad.sty`), which of its
//! OTS buttons 1-4 load one of the user's racks (by id) instead of the style's own One Touch
//! Setting. The style file itself is never touched.
//!
//! The write is atomic. A file of a newer version than this build knows is refused on read
//! and never saved over. Fields this build does not know are kept and written back.

use crate::data_files::write_atomic;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The file's name in the data folder.
pub const FILE: &str = "style-racks.json";
/// Its `format` field.
pub const FORMAT: &str = "yahaha.style-racks";
/// The version this build reads and writes.
pub const VERSION: u32 = 1;
/// OTS buttons per style.
pub const SLOTS: usize = 4;

/// One style's OTS buttons: `[i]` is the rack id OTS i+1 loads, or None for the style's own.
pub type Slots = [Option<String>; SLOTS];

/// Every style's OTS buttons that load a user rack.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StyleRacks {
    /// By style file name. A style with every slot its own is left out.
    pub styles: BTreeMap<String, Slots>,
    /// Fields a newer build wrote: kept, and written back as they were.
    pub other: Map<String, Value>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileForm {
    format: String,
    version: u32,
    /// Style file name to its OTS 1-4 in order; missing ones are the style's own, extra
    /// ones ignored.
    #[serde(default)]
    styles: BTreeMap<String, Vec<Option<String>>>,
    #[serde(flatten)]
    other: Map<String, Value>,
}

impl StyleRacks {
    /// Read the file's text. Another format, or a newer version, is refused.
    pub fn from_json(text: &str) -> Result<StyleRacks> {
        let v: Value = serde_json::from_str(text)?;
        check_header(&v)?;
        let f: FileForm = serde_json::from_value(v)?;
        let mut s = StyleRacks { other: f.other, ..StyleRacks::default() };
        for (style, ids) in f.styles {
            let mut slots = Slots::default();
            for (i, id) in ids.into_iter().take(SLOTS).enumerate() {
                slots[i] = id.filter(|id| !id.is_empty());
            }
            if slots.iter().any(Option::is_some) {
                s.styles.insert(style, slots);
            }
        }
        Ok(s)
    }

    pub fn to_json(&self) -> String {
        let f = FileForm {
            format: FORMAT.into(),
            version: VERSION,
            styles: self.styles.iter().map(|(k, v)| (k.clone(), v.to_vec())).collect(),
            other: self.other.clone(),
        };
        serde_json::to_string_pretty(&f).expect("style racks serialize")
    }

    /// Read `path`. Ok(None): there is no file yet.
    pub fn load(path: &Path) -> Result<Option<StyleRacks>> {
        match std::fs::read_to_string(path) {
            Ok(text) => StyleRacks::from_json(&text).map(Some).with_context(|| format!("reading {}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
        }
    }

    /// Write to `path`, atomically. A file already there that a newer yahaha made (or of
    /// another format) is never saved over: the save is refused.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Ok(text) = std::fs::read_to_string(path)
            && let Ok(v) = serde_json::from_str::<Value>(&text)
        {
            check_header(&v).with_context(|| format!("not saving over {}", path.display()))?;
        }
        write_atomic(path, &self.to_json())
    }

    /// The rack OTS `index` (0-3) of style `style` loads, if it isn't the style's own.
    pub fn get(&self, style: &str, index: usize) -> Option<&str> {
        self.styles.get(style)?.get(index)?.as_deref()
    }

    /// OTS `index` of style `style` loads rack `id` (None: the style's own again).
    pub fn set(&mut self, style: &str, index: usize, id: Option<String>) {
        if index >= SLOTS {
            return;
        }
        let slots = self.styles.entry(style.to_string()).or_default();
        slots[index] = id;
        if slots.iter().all(Option::is_none) {
            self.styles.remove(style);
        }
    }

    /// Put every OTS that names rack `id` back to its style's own. True if any did.
    pub fn forget(&mut self, id: &str) -> bool {
        let mut any = false;
        for slots in self.styles.values_mut() {
            for s in slots.iter_mut() {
                if s.as_deref() == Some(id) {
                    *s = None;
                    any = true;
                }
            }
        }
        self.styles.retain(|_, s| s.iter().any(Option::is_some));
        any
    }
}

fn check_header(v: &Value) -> Result<()> {
    let format = v.get("format").and_then(Value::as_str).unwrap_or_default();
    anyhow::ensure!(format == FORMAT, "not yahaha style racks (format {format:?})");
    let version = v.get("version").and_then(Value::as_u64).unwrap_or(0);
    anyhow::ensure!(version <= VERSION as u64, "made by a newer yahaha (style racks version {version})");
    Ok(())
}

/// The file in data folder `data`.
pub fn path(data: &Path) -> PathBuf {
    data.join(FILE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_keeps_unknown_fields() {
        let mut s = StyleRacks::default();
        s.set("Soul Ballad.sty", 1, Some("r1".into()));
        s.set("Soul Ballad.sty", 3, Some("r2".into()));
        s.set("Pop.sty", 0, Some("r1".into()));
        s.other.insert("later".into(), Value::from(3));
        let back = StyleRacks::from_json(&s.to_json()).unwrap();
        assert_eq!(back, s);
        assert_eq!((back.get("Soul Ballad.sty", 1), back.get("Soul Ballad.sty", 0)), (Some("r1"), None));
        assert_eq!(back.get("Soul Ballad.sty", 3), Some("r2"));
        assert_eq!(back.get("Other.sty", 0), None);
        let v: Value = serde_json::from_str(&s.to_json()).unwrap();
        assert_eq!((v["format"].as_str(), v["version"].as_u64()), (Some(FORMAT), Some(1)));
        assert_eq!(v["styles"]["Pop.sty"], serde_json::json!(["r1", null, null, null]));
    }

    #[test]
    fn the_style_s_own_again_drops_the_style() {
        let mut s = StyleRacks::default();
        s.set("Pop.sty", 2, Some("r1".into()));
        s.set("Pop.sty", 2, None);
        assert!(s.styles.is_empty());
        s.set("Pop.sty", 9, Some("r1".into()));
        assert!(s.styles.is_empty(), "no OTS 10");
        let short = StyleRacks::from_json(r#"{"format":"yahaha.style-racks","version":1,"styles":{"A.sty":["x"],"B.sty":[null,""]}}"#).unwrap();
        assert_eq!((short.get("A.sty", 0), short.get("A.sty", 1)), (Some("x"), None));
        assert!(!short.styles.contains_key("B.sty"));
    }

    #[test]
    fn a_newer_file_is_refused_and_never_saved_over() {
        let dir = std::env::temp_dir().join(format!("yahaha-style-racks-newer-{}", std::process::id()));
        let path = dir.join(FILE);
        let newer = r#"{"format":"yahaha.style-racks","version":2,"styles":{}}"#;
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, newer).unwrap();
        assert!(StyleRacks::load(&path).is_err());
        assert!(StyleRacks::default().save(&path).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), newer);
        assert!(StyleRacks::from_json(r#"{"format":"yahaha.quick-racks","version":1}"#).is_err());
        assert_eq!(StyleRacks::load(&dir.join("none.json")).unwrap(), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn forgetting_a_rack_gives_its_ots_back_to_the_style() {
        let mut s = StyleRacks::default();
        s.set("A.sty", 0, Some("r1".into()));
        s.set("B.sty", 1, Some("r1".into()));
        s.set("B.sty", 2, Some("r2".into()));
        assert!(s.forget("r1"));
        assert_eq!((s.get("A.sty", 0), s.get("B.sty", 1), s.get("B.sty", 2)), (None, None, Some("r2")));
        assert!(!s.styles.contains_key("A.sty"));
        assert!(!s.forget("r1"));
    }
}

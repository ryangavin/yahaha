//! Quick Racks' file (docs/racks.md): `<data>/quick-racks.json`, format
//! `yahaha.quick-racks`, version 1. Banks A-H of eight buttons, each a rack id or empty.
//!
//! The write is atomic. A file of a newer version than this build knows is refused on read
//! and never saved over. Fields this build does not know are kept and written back.

use crate::data_files::write_atomic;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::path::Path;

/// The file's name in the data folder.
pub const FILE: &str = "quick-racks.json";
/// Its `format` field.
pub const FORMAT: &str = "yahaha.quick-racks";
/// The version this build reads and writes.
pub const VERSION: u32 = 1;
/// Banks A-H.
pub const BANKS: usize = 8;
/// Buttons per bank.
pub const SLOTS: usize = 8;

/// Every Quick Rack button: `banks[bank][slot]` is a rack id, or None when empty.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct QuickRacks {
    pub banks: [[Option<String>; SLOTS]; BANKS],
    /// Fields a newer build wrote: kept, and written back as they were.
    pub other: Map<String, Value>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileForm {
    format: String,
    version: u32,
    /// Banks in order, each its buttons in order; missing ones are empty, extra ones ignored.
    #[serde(default)]
    banks: Vec<Vec<Option<String>>>,
    #[serde(flatten)]
    other: Map<String, Value>,
}

impl QuickRacks {
    /// Read the file's text. Another format, or a newer version, is refused.
    pub fn from_json(text: &str) -> Result<QuickRacks> {
        let v: Value = serde_json::from_str(text)?;
        check_header(&v)?;
        let f: FileForm = serde_json::from_value(v)?;
        let mut q = QuickRacks { other: f.other, ..QuickRacks::default() };
        for (b, bank) in f.banks.iter().take(BANKS).enumerate() {
            for (s, id) in bank.iter().take(SLOTS).enumerate() {
                q.banks[b][s] = id.clone().filter(|id| !id.is_empty());
            }
        }
        Ok(q)
    }

    pub fn to_json(&self) -> String {
        let f = FileForm {
            format: FORMAT.into(),
            version: VERSION,
            banks: self.banks.iter().map(|b| b.to_vec()).collect(),
            other: self.other.clone(),
        };
        serde_json::to_string_pretty(&f).expect("quick racks serialize")
    }

    /// Read `path`. Ok(None): there is no file yet.
    pub fn load(path: &Path) -> Result<Option<QuickRacks>> {
        match std::fs::read_to_string(path) {
            Ok(text) => QuickRacks::from_json(&text).map(Some).with_context(|| format!("reading {}", path.display())),
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

    /// Button `slot` of bank `bank`, if it holds a rack.
    pub fn get(&self, bank: usize, slot: usize) -> Option<&str> {
        self.banks.get(bank)?.get(slot)?.as_deref()
    }

    /// Empty every button that names rack `id`. True if any did.
    pub fn forget(&mut self, id: &str) -> bool {
        let mut any = false;
        for b in self.banks.iter_mut().flatten() {
            if b.as_deref() == Some(id) {
                *b = None;
                any = true;
            }
        }
        any
    }
}

fn check_header(v: &Value) -> Result<()> {
    let format = v.get("format").and_then(Value::as_str).unwrap_or_default();
    anyhow::ensure!(format == FORMAT, "not yahaha Quick Racks (format {format:?})");
    let version = v.get("version").and_then(Value::as_u64).unwrap_or(0);
    anyhow::ensure!(version <= VERSION as u64, "made by a newer yahaha (Quick Racks version {version})");
    Ok(())
}

/// A bank's letter: "A" for 0.
pub fn bank_letter(bank: usize) -> char {
    (b'A' + bank.min(BANKS - 1) as u8) as char
}

/// A button's label: "A1" for bank 0, slot 0; "C3" for bank 2, slot 2.
pub fn label(bank: usize, slot: usize) -> String {
    format!("{}{}", bank_letter(bank), slot + 1)
}

/// The file in data folder `data`.
pub fn path(data: &Path) -> std::path::PathBuf {
    data.join(FILE)
}

/// A captured rack's name, from its sounds (hold Sound + tap an empty Quick Rack pad,
/// docs/eyes-free.md): the sounds of the parts that are on, in part order (Right 1-3, then
/// Left), each once, joined with " + ": "Rhodes Soft + Strings". Blank names are skipped.
/// None when no part names a sound.
pub fn name_from_sounds<'a>(sounds: impl IntoIterator<Item = &'a str>) -> Option<String> {
    let mut names: Vec<&str> = Vec::new();
    for s in sounds.into_iter().map(str::trim).filter(|s| !s.is_empty()) {
        if !names.iter().any(|n| n.eq_ignore_ascii_case(s)) {
            names.push(s);
        }
    }
    (!names.is_empty()).then(|| names.join(" + "))
}

/// `name`, or the first of "`name` 2", "`name` 3", ... that `taken` says is free.
pub fn unique_name(name: &str, taken: impl Fn(&str) -> bool) -> String {
    (1..)
        .map(|n| if n == 1 { name.to_string() } else { format!("{name} {n}") })
        .find(|n| !taken(n))
        .expect("a free name")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_keeps_unknown_fields() {
        let mut q = QuickRacks::default();
        q.banks[0][0] = Some("r1".into());
        q.banks[7][7] = Some("r2".into());
        q.other.insert("later".into(), Value::from(3));
        let back = QuickRacks::from_json(&q.to_json()).unwrap();
        assert_eq!(back, q);
        assert_eq!(back.get(7, 7), Some("r2"));
        assert_eq!(back.get(0, 1), None);
        let v: Value = serde_json::from_str(&q.to_json()).unwrap();
        assert_eq!((v["format"].as_str(), v["version"].as_u64()), (Some(FORMAT), Some(1)));
        assert_eq!(v["banks"].as_array().unwrap().len(), BANKS);
    }

    #[test]
    fn short_banks_read_as_empty_buttons() {
        let q = QuickRacks::from_json(r#"{"format":"yahaha.quick-racks","version":1,"banks":[["a",null,""]]}"#).unwrap();
        assert_eq!((q.get(0, 0), q.get(0, 1), q.get(0, 2), q.get(1, 0)), (Some("a"), None, None, None));
    }

    #[test]
    fn a_newer_file_is_refused_and_never_saved_over() {
        let dir = std::env::temp_dir().join(format!("yahaha-quick-newer-{}", std::process::id()));
        let path = dir.join(FILE);
        let newer = r#"{"format":"yahaha.quick-racks","version":2,"banks":[]}"#;
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, newer).unwrap();
        assert!(QuickRacks::load(&path).is_err());
        assert!(QuickRacks::default().save(&path).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), newer);
        assert!(QuickRacks::from_json(r#"{"format":"yahaha.rack","version":1}"#).is_err());
        assert_eq!(QuickRacks::load(&dir.join("none.json")).unwrap(), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_captured_rack_is_named_from_its_sounds() {
        assert_eq!(name_from_sounds(["Rhodes Soft", "Strings"]).as_deref(), Some("Rhodes Soft + Strings"));
        // Each sound once (case aside), blanks skipped, in the order given.
        assert_eq!(name_from_sounds(["Strings", " ", "Rhodes Soft", "strings", "Fretless"]).as_deref(), Some("Strings + Rhodes Soft + Fretless"));
        assert_eq!(name_from_sounds(["  Piano "]).as_deref(), Some("Piano"));
        assert_eq!(name_from_sounds([]), None);
        assert_eq!(name_from_sounds(["", " "]), None);
    }

    #[test]
    fn unique_names_count_up_from_2() {
        assert_eq!(unique_name("Piano", |_| false), "Piano");
        let taken = ["Piano", "Piano 2"];
        assert_eq!(unique_name("Piano", |n| taken.contains(&n)), "Piano 3");
    }

    #[test]
    fn forgetting_a_rack_empties_its_buttons() {
        let mut q = QuickRacks::default();
        q.banks[0][1] = Some("r1".into());
        q.banks[3][4] = Some("r1".into());
        q.banks[3][5] = Some("r2".into());
        assert!(q.forget("r1"));
        assert_eq!((q.get(0, 1), q.get(3, 4), q.get(3, 5)), (None, None, Some("r2")));
        assert!(!q.forget("r1"));
    }
}

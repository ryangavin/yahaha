//! The Playlist (OM p.100-103): a set list of records, each linking to a Registration
//! bank file (optionally recalling one of its buttons) or, in yahaha, straight to a style
//! file. Saved as one JSON file (`<name>.playlist.json`).

use super::{file_stem, list_files, write_atomic};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const PLAYLIST_FORMAT: &str = "yahaha.playlist";
pub const PLAYLIST_EXT: &str = ".playlist.json";
pub const PLAYLIST_VERSION: u32 = 1;
/// Records per playlist file (OM p.133).
pub const MAX_RECORDS: usize = 2500;

/// What a record loads.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum RecordTarget {
    /// A Registration bank file; with `regist`, that button (0-9) is recalled after the
    /// bank loads (the Genos record Action "Load Regist Memory").
    Bank {
        path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        regist: Option<u8>,
    },
    /// A style file, loaded as the style browser would.
    Style { path: String },
}

impl RecordTarget {
    pub fn path(&self) -> &str {
        match self {
            RecordTarget::Bank { path, .. } | RecordTarget::Style { path } => path,
        }
    }
}

/// One Playlist record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub name: String,
    #[serde(flatten)]
    pub target: RecordTarget,
}

/// The Playlist display order (OM p.102). Up/Down, Delete and Add are disabled while the
/// list is sorted; saving keeps the displayed order and returns to Normal.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlaylistSort {
    #[default]
    Normal,
    AToZ,
    ZToA,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub format: String,
    pub version: u32,
    pub name: String,
    pub records: Vec<Record>,
}

impl Default for Playlist {
    fn default() -> Playlist {
        Playlist::new("New Playlist")
    }
}

impl Playlist {
    pub fn new(name: &str) -> Playlist {
        Playlist { format: PLAYLIST_FORMAT.into(), version: PLAYLIST_VERSION, name: name.into(), records: Vec::new() }
    }

    pub fn from_json(text: &str) -> Result<Playlist> {
        let mut p: Playlist = serde_json::from_str(text)?;
        anyhow::ensure!(p.format == PLAYLIST_FORMAT, "not a yahaha playlist (format {:?})", p.format);
        anyhow::ensure!(p.version <= PLAYLIST_VERSION, "made by a newer yahaha (playlist version {})", p.version);
        p.records.truncate(MAX_RECORDS);
        Ok(p)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a playlist serializes")
    }

    pub fn load(path: &Path) -> Result<Playlist> {
        let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        Playlist::from_json(&text).with_context(|| format!("reading {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        write_atomic(path, &self.to_json())
    }

    /// Record indices in display order.
    pub fn order(&self, sort: PlaylistSort) -> Vec<usize> {
        let mut v: Vec<usize> = (0..self.records.len()).collect();
        let key = |i: &usize| self.records[*i].name.to_lowercase();
        match sort {
            PlaylistSort::Normal => {}
            PlaylistSort::AToZ => v.sort_by_key(key),
            PlaylistSort::ZToA => {
                v.sort_by_key(key);
                v.reverse();
            }
        }
        v
    }

    /// Put the records in display order (what Save does while sorted).
    pub fn apply_order(&mut self, sort: PlaylistSort) {
        let order = self.order(sort);
        let old = std::mem::take(&mut self.records);
        let mut slots: Vec<Option<Record>> = old.into_iter().map(Some).collect();
        self.records = order.into_iter().filter_map(|i| slots[i].take()).collect();
    }

    /// Add a record at the end; false when the playlist is full.
    pub fn push(&mut self, r: Record) -> bool {
        if self.records.len() >= MAX_RECORDS {
            return false;
        }
        self.records.push(r);
        true
    }

    /// Move record `i` by `delta` places (Up/Down). False if it can't move.
    pub fn move_record(&mut self, i: usize, delta: i32) -> bool {
        let n = self.records.len() as i64;
        let j = i as i64 + delta as i64;
        if i as i64 >= n || j < 0 || j >= n {
            return false;
        }
        let r = self.records.remove(i);
        self.records.insert(j as usize, r);
        true
    }
}

/// A record target's file, with a relative path taken from the playlist's folder.
pub fn resolve(target: &str, playlist_dir: Option<&Path>) -> PathBuf {
    let p = PathBuf::from(target);
    match playlist_dir {
        Some(d) if p.is_relative() => d.join(p),
        _ => p,
    }
}

pub fn list_playlists(dir: &Path) -> Vec<PathBuf> {
    list_files(dir, PLAYLIST_EXT)
}

pub fn playlist_name(path: &Path) -> String {
    file_stem(path, PLAYLIST_EXT)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(name: &str) -> Record {
        Record { name: name.into(), target: RecordTarget::Style { path: format!("/s/{name}.sty") } }
    }

    #[test]
    fn records_serialize_flat() {
        let r = Record { name: "Opener".into(), target: RecordTarget::Bank { path: "Set.regist.json".into(), regist: Some(2) } };
        let j = serde_json::to_string(&r).unwrap();
        assert_eq!(j, r#"{"name":"Opener","kind":"bank","path":"Set.regist.json","regist":2}"#);
        assert_eq!(serde_json::from_str::<Record>(&j).unwrap(), r);
        let s: Record = serde_json::from_str(r#"{"name":"x","kind":"style","path":"a.sty"}"#).unwrap();
        assert_eq!(s.target.path(), "a.sty");
    }

    #[test]
    fn sorting_and_saving_in_display_order() {
        let mut p = Playlist::new("gig");
        for n in ["b", "C", "a"] {
            p.push(rec(n));
        }
        assert_eq!(p.order(PlaylistSort::Normal), [0, 1, 2]);
        assert_eq!(p.order(PlaylistSort::AToZ), [2, 0, 1]);
        assert_eq!(p.order(PlaylistSort::ZToA), [1, 0, 2]);
        p.apply_order(PlaylistSort::AToZ);
        assert_eq!(p.records.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), ["a", "b", "C"]);
        let back = Playlist::from_json(&p.to_json()).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn moving_records() {
        let mut p = Playlist::new("gig");
        for n in ["a", "b", "c"] {
            p.push(rec(n));
        }
        assert!(p.move_record(0, 1));
        assert!(!p.move_record(0, -1));
        assert!(!p.move_record(2, 1));
        assert!(p.move_record(2, -2));
        assert_eq!(p.records.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), ["c", "b", "a"]);
    }

    #[test]
    fn relative_targets_resolve_against_the_playlist_folder() {
        assert_eq!(resolve("x.regist.json", Some(Path::new("/p"))), PathBuf::from("/p/x.regist.json"));
        assert_eq!(resolve("/a/x.sty", Some(Path::new("/p"))), PathBuf::from("/a/x.sty"));
    }
}

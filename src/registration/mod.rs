//! Registration Memory, Registration Freeze, Registration Sequence and the Playlist: the
//! file formats and the logic that needs no session (Genos OM p.96-103, RM p.113-119).
//!
//! - A **bank** ([`Bank`]) is ten Registration Memory buttons plus the bank's Registration
//!   Sequence, saved as one JSON file (`<name>.regist.json`). yahaha does not read or write
//!   Yamaha's `.rgt`.
//! - A **memory** ([`Memory`]) is the panel as it was memorized: a name, the groups that were
//!   memorized, and one JSON section per registrable feature (`"style"`, `"parts"`, ...). The
//!   session's registrables (src/session/registration/sections.rs) capture and recall the
//!   sections; this module only stores them, so a feature added later brings its own
//!   section and older files simply lack it. Sections this build does not know are kept
//!   as they are.
//! - [`Group`]s are the Genos Freeze groups (the Data List's "Freeze Group" column), used
//!   both for Memorize (which groups a button stores) and Freeze (which groups a recall
//!   leaves alone).
//! - The Registration Sequence is in [`sequence`], the Playlist in [`playlist`].
//!
//! docs/registration.md has the Genos behaviour and the decisions made where the manuals
//! are silent.

pub mod playlist;
pub mod sequence;

pub use playlist::{Playlist, PlaylistSort, Record, RecordTarget};
pub use sequence::{SeqMove, Sequence, SequenceEnd};

use anyhow::{Context, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Registration Memory buttons per bank (OM p.133).
pub const BUTTONS: usize = 10;

/// A bank file's `format` field.
pub const BANK_FORMAT: &str = "yahaha.registration-bank";
/// A bank file's name ends with this.
pub const BANK_EXT: &str = ".regist.json";
pub const BANK_VERSION: u32 = 1;

/// A Registration Freeze / Memorize group (Genos Data List, "Freeze Group" column). Only
/// the groups yahaha has (or plans) features for; the Genos's Line Out, Song, Text and
/// Vocal Harmony groups are out of scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Group {
    /// The Style, its section, Sync Start/Stop, Stop ACMP, OTS Link, the Style part mixer,
    /// and (as on the Genos) the Left part, split point and fingering.
    Style,
    /// The Right 1-3 parts: voice, on/off, volume, octave.
    Voice,
    /// Keyboard Harmony / Arpeggio.
    HarmonyArp,
    /// The Multi Pad bank.
    MultiPad,
    Tempo,
    Transpose,
    ChordLooper,
    LiveControl,
    /// The Assignable settings: today the Fade In/Out and Fade Out Hold times (Data List:
    /// Freeze group "Assignable Buttons"; #107).
    Assignable,
}

impl Group {
    pub const ALL: [Group; 9] = [
        Group::Style,
        Group::Voice,
        Group::HarmonyArp,
        Group::MultiPad,
        Group::Tempo,
        Group::Transpose,
        Group::ChordLooper,
        Group::LiveControl,
        Group::Assignable,
    ];

    /// Display name, as the Genos lists it.
    pub fn name(self) -> &'static str {
        match self {
            Group::Style => "Style",
            Group::Voice => "Voice",
            Group::HarmonyArp => "Keyboard Harmony/Arpeggio",
            Group::MultiPad => "Multi Pad",
            Group::Tempo => "Tempo",
            Group::Transpose => "Transpose",
            Group::ChordLooper => "Chord Looper",
            Group::LiveControl => "Live Control",
            Group::Assignable => "Assignable",
        }
    }

    fn bit(self) -> u16 {
        1 << (self as u16)
    }
}

/// A set of [`Group`]s. On the wire and in files: a list, e.g. `["style","tempo"]`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Groups(u16);

impl Groups {
    pub const NONE: Groups = Groups(0);

    pub fn all() -> Groups {
        Group::ALL.into_iter().collect()
    }

    pub fn has(self, g: Group) -> bool {
        self.0 & g.bit() != 0
    }

    pub fn set(&mut self, g: Group, on: bool) {
        if on {
            self.0 |= g.bit();
        } else {
            self.0 &= !g.bit();
        }
    }

    pub fn with(mut self, g: Group) -> Groups {
        self.set(g, true);
        self
    }

    /// The groups in `self` but not in `other`.
    pub fn minus(self, other: Groups) -> Groups {
        Groups(self.0 & !other.0)
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn list(self) -> Vec<Group> {
        Group::ALL.into_iter().filter(|&g| self.has(g)).collect()
    }
}

impl FromIterator<Group> for Groups {
    fn from_iter<I: IntoIterator<Item = Group>>(it: I) -> Groups {
        let mut s = Groups::NONE;
        for g in it {
            s.set(g, true);
        }
        s
    }
}

impl Serialize for Groups {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.list().serialize(s)
    }
}

impl<'de> Deserialize<'de> for Groups {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Groups, D::Error> {
        // A group this build doesn't know (a newer file) is skipped, not an error.
        let v = Vec::<serde_json::Value>::deserialize(d)?;
        Ok(v.into_iter().filter_map(|g| serde_json::from_value::<Group>(g).ok()).collect())
    }
}

/// Which voice a keyboard part plays. Tagged by `kind`, so other voice sources (plugin
/// instruments, phase 2 of #35) are new variants and old files stay valid.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum VoiceRef {
    /// A General MIDI voice of the built-in synth / the MIDI port: program 0-127 on bank
    /// MSB/LSB (0/0 today).
    Gm {
        program: u8,
        #[serde(default)]
        bank_msb: u8,
        #[serde(default)]
        bank_lsb: u8,
    },
}

impl VoiceRef {
    pub fn gm(program: u8) -> VoiceRef {
        VoiceRef::Gm { program: program & 127, bank_msb: 0, bank_lsb: 0 }
    }

    /// The GM program to play, when this build can play the voice.
    pub fn program(&self) -> Option<u8> {
        match self {
            VoiceRef::Gm { program, .. } => Some(*program & 127),
        }
    }
}

/// One Registration Memory button's contents.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(from = "MemoryFile", into = "MemoryFile")]
pub struct Memory {
    pub name: String,
    /// The groups memorized (the Memory window's checkboxes). A recall changes only these.
    pub groups: Groups,
    /// Memorized groups this build does not know (a newer build's, e.g. `footPedals`):
    /// written back as they were, so saving the bank here does not forget them.
    pub other_groups: Vec<String>,
    /// One entry per registrable feature, by its key.
    pub sections: BTreeMap<String, serde_json::Value>,
}

/// A memory as the bank file has it: `groups` is one list, known and unknown names alike.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MemoryFile {
    #[serde(default)]
    name: String,
    #[serde(default)]
    groups: Vec<serde_json::Value>,
    #[serde(default)]
    sections: BTreeMap<String, serde_json::Value>,
}

impl From<MemoryFile> for Memory {
    fn from(f: MemoryFile) -> Memory {
        let mut groups = Groups::NONE;
        let mut other_groups = Vec::new();
        for g in f.groups {
            match serde_json::from_value::<Group>(g.clone()) {
                Ok(g) => groups.set(g, true),
                Err(_) => {
                    if let serde_json::Value::String(s) = g
                        && !other_groups.contains(&s)
                    {
                        other_groups.push(s);
                    }
                }
            }
        }
        Memory { name: f.name, groups, other_groups, sections: f.sections }
    }
}

impl From<Memory> for MemoryFile {
    fn from(m: Memory) -> MemoryFile {
        let mut groups: Vec<serde_json::Value> = m.groups.list().into_iter().map(|g| serde_json::to_value(g).expect("a group serializes")).collect();
        groups.extend(m.other_groups.into_iter().map(serde_json::Value::String));
        MemoryFile { name: m.name, groups, sections: m.sections }
    }
}

/// A Registration Memory bank: ten buttons and the bank's Registration Sequence.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bank {
    pub format: String,
    pub version: u32,
    pub name: String,
    /// Always `BUTTONS` long; None = an empty button.
    #[serde(deserialize_with = "ten")]
    pub memories: Vec<Option<Memory>>,
    #[serde(default)]
    pub sequence: Sequence,
    /// Search tags (RM p.117). Kept, not used yet.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

fn ten<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Option<Memory>>, D::Error> {
    let mut v = Vec::<Option<Memory>>::deserialize(d)?;
    v.resize(BUTTONS, None);
    Ok(v)
}

impl Default for Bank {
    fn default() -> Bank {
        Bank::new("New Bank")
    }
}

impl Bank {
    pub fn new(name: &str) -> Bank {
        Bank {
            format: BANK_FORMAT.into(),
            version: BANK_VERSION,
            name: name.into(),
            memories: vec![None; BUTTONS],
            sequence: Sequence::default(),
            tags: Vec::new(),
        }
    }

    pub fn from_json(text: &str) -> Result<Bank> {
        let b: Bank = serde_json::from_str(text)?;
        anyhow::ensure!(b.format == BANK_FORMAT, "not a yahaha registration bank (format {:?})", b.format);
        anyhow::ensure!(b.version <= BANK_VERSION, "made by a newer yahaha (bank version {})", b.version);
        Ok(b)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a bank serializes")
    }

    pub fn load(path: &Path) -> Result<Bank> {
        let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        Bank::from_json(&text).with_context(|| format!("reading {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        write_atomic(path, &self.to_json())
    }

    /// Buttons with data, as a bit mask (bit 0 = button 1).
    pub fn stored_mask(&self) -> u16 {
        self.memories.iter().enumerate().filter(|(_, m)| m.is_some()).fold(0, |a, (i, _)| a | 1 << i)
    }
}

/// The bank files in `dir`, sorted by name (the Registration Bank Selection display's
/// order; REGIST BANK -/+ and the sequence's "Next" follow it).
pub fn list_banks(dir: &Path) -> Vec<PathBuf> {
    list_files(dir, BANK_EXT)
}

/// A bank's name from its file name ("Ballads.regist.json" -> "Ballads").
pub fn bank_name(path: &Path) -> String {
    file_stem(path, BANK_EXT)
}

/// A file name for a bank or playlist called `name` (characters a file name can't have
/// become `_`).
pub fn file_name(name: &str, ext: &str) -> String {
    let clean: String = name.trim().chars().map(|c| if matches!(c, '/' | '\\' | ':' | '\0') { '_' } else { c }).collect();
    let clean = clean.trim_start_matches('.');
    format!("{}{ext}", if clean.is_empty() { "Untitled" } else { clean })
}

/// The file `file` (a name from `file_name`) names in `dir`, as it is on disk, or None if
/// there is none. On a case-insensitive file system (APFS, the Mac's default; NTFS)
/// "gig.regist.json" opens the existing "Gig.regist.json": this returns that entry, so a
/// save compares with, and renames, the file that is really there.
pub fn existing_file(dir: &Path, file: &str) -> Option<PathBuf> {
    let want = dir.join(file);
    if !want.exists() {
        return None;
    }
    let names: Vec<String> = std::fs::read_dir(dir)
        .map(|rd| rd.filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned())).collect())
        .unwrap_or_default();
    if names.iter().any(|n| n == file) {
        return Some(want);
    }
    let lower = file.to_lowercase();
    Some(names.into_iter().find(|n| n.to_lowercase() == lower).map_or(want, |n| dir.join(n)))
}

/// Where a save as `file` in `dir` goes, given the file in use (`own`): Ok(path), after
/// renaming an existing file whose name differs only in case (the same file on a
/// case-insensitive file system) to the spelling asked for; `SaveClash::Exists` when it belongs
/// to something else and `overwrite` is not set.
pub fn save_target(dir: &Path, file: &str, own: Option<&Path>, overwrite: bool) -> Result<PathBuf, SaveClash> {
    let path = dir.join(file);
    let Some(existing) = existing_file(dir, file) else { return Ok(path) };
    if own != Some(existing.as_path()) && !overwrite {
        return Err(SaveClash::Exists);
    }
    if existing != path {
        std::fs::rename(&existing, &path).map_err(|e| SaveClash::Rename(e.to_string()))?;
    }
    Ok(path)
}

/// Why `save_target` refused.
#[derive(Debug)]
pub enum SaveClash {
    /// Another bank's or playlist's file has that name.
    Exists,
    /// Renaming the file to the new case failed.
    Rename(String),
}

pub(crate) fn list_files(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.is_file() && p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.ends_with(ext) && !n.starts_with('.')))
                .collect()
        })
        .unwrap_or_default();
    v.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()));
    v
}

pub(crate) fn file_stem(path: &Path, ext: &str) -> String {
    let n = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    n.strip_suffix(ext).map(str::to_string).unwrap_or(n)
}

/// Write via a temporary file and a rename, so a crash never leaves half a file.
pub(crate) fn write_atomic(path: &Path, text: &str) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, text).with_context(|| format!("writing {}", tmp.display()))?;
    std::fs::rename(&tmp, path).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_round_trip_as_a_list_and_skip_unknown() {
        let g = Groups::NONE.with(Group::Style).with(Group::Tempo);
        assert_eq!(serde_json::to_string(&g).unwrap(), r#"["style","tempo"]"#);
        let back: Groups = serde_json::from_str(r#"["tempo","vocalHarmony","style"]"#).unwrap();
        assert_eq!(back, g);
        assert_eq!(Groups::all().list().len(), Group::ALL.len());
        assert!(!Groups::all().minus(g).has(Group::Style));
    }

    #[test]
    fn unknown_groups_survive_a_round_trip() {
        // A newer build memorized Foot Pedals: this build recalls what it knows and writes
        // the rest back as it was (review N1).
        let text = r#"{"format":"yahaha.registration-bank","version":1,"name":"x","memories":[
            {"name":"A","groups":["style","footPedals","tempo"],"sections":{"footPedals":{"p":1}}}]}"#;
        let b = Bank::from_json(text).unwrap();
        let m = b.memories[0].as_ref().unwrap();
        assert_eq!(m.groups, Groups::NONE.with(Group::Style).with(Group::Tempo));
        assert_eq!(m.other_groups, ["footPedals"]);
        let again = Bank::from_json(&b.to_json()).unwrap();
        assert_eq!(again, b);
        let v: serde_json::Value = serde_json::from_str(&b.to_json()).unwrap();
        assert_eq!(v["memories"][0]["groups"], serde_json::json!(["style", "tempo", "footPedals"]));
    }

    #[test]
    fn bank_round_trips_and_pads_to_ten_buttons() {
        let mut b = Bank::new("Set 1");
        let mut m = Memory { name: "Ballad".into(), groups: Groups::all(), ..Memory::default() };
        m.sections.insert("tempo".into(), serde_json::json!({ "bpm": 72.0 }));
        m.sections.insert("fromTheFuture".into(), serde_json::json!({ "x": 1 }));
        b.memories[3] = Some(m);
        let text = b.to_json();
        let back = Bank::from_json(&text).unwrap();
        assert_eq!(back, b);
        assert_eq!(back.stored_mask(), 1 << 3);
        // A short list is padded; a wrong format is refused.
        let short = r#"{"format":"yahaha.registration-bank","version":1,"name":"x","memories":[null]}"#;
        assert_eq!(Bank::from_json(short).unwrap().memories.len(), BUTTONS);
        assert!(Bank::from_json(r#"{"format":"other","version":1,"name":"x","memories":[]}"#).is_err());
    }

    #[test]
    fn voice_ref_is_tagged_by_kind() {
        let v = VoiceRef::gm(48);
        assert_eq!(serde_json::to_string(&v).unwrap(), r#"{"kind":"gm","program":48,"bankMsb":0,"bankLsb":0}"#);
        let back: VoiceRef = serde_json::from_str(r#"{"kind":"gm","program":5}"#).unwrap();
        assert_eq!(back.program(), Some(5));
        assert!(serde_json::from_str::<VoiceRef>(r#"{"kind":"plugin","id":"x"}"#).is_err());
    }

    #[test]
    fn file_names_and_listing() {
        assert_eq!(file_name("My/Set: 1", BANK_EXT), "My_Set_ 1.regist.json");
        assert_eq!(file_name("  ", BANK_EXT), "Untitled.regist.json");
        let dir = std::env::temp_dir().join(format!("yahaha-reg-list-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        Bank::new("b").save(&dir.join("b.regist.json")).unwrap();
        Bank::new("a").save(&dir.join("A.regist.json")).unwrap();
        std::fs::write(dir.join("notes.txt"), "x").unwrap();
        let v = list_banks(&dir);
        assert_eq!(v.iter().map(|p| bank_name(p)).collect::<Vec<_>>(), ["A", "b"]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

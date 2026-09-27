//! Registration Memory, Registration Freeze, Registration Sequence and the Playlist: the
//! file formats and the logic that needs no session (Genos OM p.96-103, RM p.113-119).
//!
//! - A **bank file** ([`Bank`]) holds Snapshots (the Genos's Registration Memory buttons)
//!   in **snapshot banks of eight** (Bank A: Snapshots 1-8, Bank B: the next eight, ...;
//!   the controllers have eights), plus the file's Registration Sequence, saved as one JSON
//!   file (`<name>.regist.json`). A snapshot is addressed by its index in the file
//!   (`bank * 8 + slot`). yahaha does not read or write Yamaha's `.rgt`.
//! - Version 1 files had ten buttons: they load as Bank A = buttons 1-8 and Bank B =
//!   buttons 9-10 (the same indices, so sequences and playlist records still point at the
//!   same registrations), and save as version 2.
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

/// Snapshots per snapshot bank (the controllers' eight pads).
pub const SLOTS: usize = 8;
/// Snapshot banks a bank file holds at most (A-H).
pub const MAX_BANKS: usize = 8;
/// Snapshots a bank file holds at most: indices are 0..MAX_SLOTS.
pub const MAX_SLOTS: usize = SLOTS * MAX_BANKS;

/// The name of snapshot bank `bank` (0-based): 'A', 'B', ...
pub fn bank_letter(bank: usize) -> char {
    (b'A' + (bank.min(25) as u8)) as char
}

/// A snapshot's short label: "A1" for index 0, "B2" for index 9.
pub fn snapshot_label(index: usize) -> String {
    format!("{}{}", bank_letter(index / SLOTS), index % SLOTS + 1)
}

/// A bank file's `format` field.
pub const BANK_FORMAT: &str = "yahaha.registration-bank";
/// A bank file's name ends with this.
pub const BANK_EXT: &str = ".regist.json";
/// 2: snapshot banks of eight (a list of whole banks). 1: ten buttons per file.
pub const BANK_VERSION: u32 = 2;

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
    /// An instrument plugin on the part (#91's `setPartPlugin`, #104): its id
    /// (`"aumu dls  appl"`), its name for Regist Bank Info, its full state (base64; none =
    /// its default preset), and the GM voice the part has underneath, which a build or a
    /// Mac without the plugin plays instead.
    Plugin {
        id: String,
        #[serde(default)]
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        state: Option<String>,
        #[serde(default)]
        program: u8,
        /// The library Sound the state is (docs/sound-browser.md). A registration saved
        /// before sounds had ids has none; `SoundLibrary::tag_for_state` finds it again.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sound: Option<crate::patches::SoundTag>,
    },
}

impl VoiceRef {
    pub fn gm(program: u8) -> VoiceRef {
        VoiceRef::Gm { program: program & 127, bank_msb: 0, bank_lsb: 0 }
    }

    /// The GM program to play, when this build can play the voice: a plugin's is the GM
    /// voice underneath it.
    pub fn program(&self) -> Option<u8> {
        match self {
            VoiceRef::Gm { program, .. } | VoiceRef::Plugin { program, .. } => Some(*program & 127),
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

/// A bank file: its snapshots, in snapshot banks of eight, and its Registration Sequence.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bank {
    pub format: String,
    pub version: u32,
    pub name: String,
    /// Every snapshot, by index (None = empty): whole snapshot banks, just enough to hold
    /// the last stored one (at least one bank). Change it with `set`, which keeps it so.
    #[serde(deserialize_with = "slots")]
    pub memories: Vec<Option<Memory>>,
    #[serde(default)]
    pub sequence: Sequence,
    /// Search tags (RM p.117). Kept, not used yet.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

fn slots<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Option<Memory>>, D::Error> {
    let mut v = Vec::<Option<Memory>>::deserialize(d)?;
    normalize(&mut v);
    Ok(v)
}

/// Whole snapshot banks, just enough for the last stored snapshot (at least one, at most
/// `MAX_BANKS`). A version 1 file's ten buttons become Bank A and, when button 9 or 10 is
/// stored, Bank B.
fn normalize(v: &mut Vec<Option<Memory>>) {
    v.truncate(MAX_SLOTS);
    let used = v.iter().rposition(Option::is_some).map_or(0, |i| i + 1);
    v.resize(used.div_ceil(SLOTS).max(1) * SLOTS, None);
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
            memories: vec![None; SLOTS],
            sequence: Sequence::default(),
            tags: Vec::new(),
        }
    }

    pub fn from_json(text: &str) -> Result<Bank> {
        let mut b: Bank = serde_json::from_str(text)?;
        anyhow::ensure!(b.format == BANK_FORMAT, "not a yahaha registration bank (format {:?})", b.format);
        anyhow::ensure!(b.version <= BANK_VERSION, "made by a newer yahaha (bank version {})", b.version);
        // An older file is this version once read (its ten buttons are Banks A and B), so
        // it saves as one: an older build then refuses it rather than dropping Bank B.
        b.version = BANK_VERSION;
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

    /// Snapshots with data, as a bit mask (bit 0 = snapshot A1).
    pub fn stored_mask(&self) -> u64 {
        self.memories.iter().enumerate().filter(|(_, m)| m.is_some()).fold(0, |a, (i, _)| a | 1 << i)
    }

    /// Snapshot banks the file holds (1..=MAX_BANKS).
    pub fn banks(&self) -> usize {
        self.memories.len() / SLOTS
    }

    /// The snapshot at `index`, if stored.
    pub fn get(&self, index: usize) -> Option<&Memory> {
        self.memories.get(index).and_then(Option::as_ref)
    }

    /// The snapshot at `index`, if stored, to change.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Memory> {
        self.memories.get_mut(index).and_then(Option::as_mut)
    }

    /// Store (Some) or empty (None) snapshot `index` (below MAX_SLOTS; others are
    /// ignored), growing or shrinking the file by whole snapshot banks.
    pub fn set(&mut self, index: usize, m: Option<Memory>) {
        if index >= MAX_SLOTS {
            return;
        }
        if index >= self.memories.len() {
            self.memories.resize(index + 1, None);
        }
        self.memories[index] = m;
        normalize(&mut self.memories);
    }
}

/// Regist +/- without a sequence (`RegistrationCmd::StepRegist`): the next (`delta` > 0) or
/// previous snapshot in `stored` (a `Bank::stored_mask`) after `from` (None: none recalled
/// yet, so + is the first stored snapshot and - the last), across the file's snapshot
/// banks. None at either end, or with nothing stored.
pub fn step_stored(stored: u64, from: Option<u8>, delta: i8) -> Option<u8> {
    let has = |b: usize| stored >> b & 1 != 0;
    match (delta.signum(), from.map(usize::from)) {
        (1, None) => (0..MAX_SLOTS).find(|&b| has(b)),
        (1, Some(f)) => (f + 1..MAX_SLOTS).find(|&b| has(b)),
        (-1, None) => (0..MAX_SLOTS).rev().find(|&b| has(b)),
        (-1, Some(f)) => (0..f.min(MAX_SLOTS)).rev().find(|&b| has(b)),
        _ => None,
    }
    .map(|b| b as u8)
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
    fn regist_plus_minus_without_a_sequence_steps_the_stored_buttons() {
        let stored = 1 << 1 | 1 << 4 | 1 << 7;
        assert_eq!(step_stored(stored, None, 1), Some(1), "+ from none: the first stored");
        assert_eq!(step_stored(stored, None, -1), Some(7), "- from none: the last stored");
        assert_eq!(step_stored(stored, Some(1), 1), Some(4), "empty buttons are skipped");
        assert_eq!(step_stored(stored, Some(4), -1), Some(1));
        assert_eq!(step_stored(stored, Some(2), 1), Some(4), "from an empty button too");
        assert_eq!(step_stored(stored, Some(7), 1), None, "it stops at the end");
        assert_eq!(step_stored(stored, Some(1), -1), None, "and at the start");
        assert_eq!(step_stored(stored, Some(4), 0), None);
        assert_eq!(step_stored(0, None, 1), None);
    }

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
    fn snapshot_labels() {
        assert_eq!(snapshot_label(0), "A1");
        assert_eq!(snapshot_label(7), "A8");
        assert_eq!(snapshot_label(9), "B2");
        assert_eq!(snapshot_label(MAX_SLOTS - 1), "H8");
    }

    #[test]
    fn stepping_crosses_snapshot_banks() {
        let stored = 1u64 << 6 | 1 << 12;
        assert_eq!(step_stored(stored, Some(6), 1), Some(12), "A7 -> B5");
        assert_eq!(step_stored(stored, Some(12), -1), Some(6));
        assert_eq!(step_stored(stored, None, -1), Some(12));
    }

    /// A version 1 file (ten buttons) loads losslessly: buttons 1-8 are Bank A, 9-10 flow
    /// into Bank B at the same indices (so the sequence and a playlist record still point
    /// at them), and it saves as version 2 and loads back the same.
    #[test]
    fn a_ten_button_bank_migrates_into_snapshot_banks_and_round_trips() {
        let mem = |name: &str, bpm: f64| {
            serde_json::json!({ "name": name, "groups": ["style", "tempo"], "sections": { "tempo": { "bpm": bpm } } })
        };
        let v1 = serde_json::json!({
            "format": BANK_FORMAT, "version": 1, "name": "Gig",
            "memories": [mem("One", 90.0), null, null, null, null, null, null, mem("Eight", 100.0), mem("Nine", 110.0), mem("Ten", 120.0)],
            "sequence": { "steps": [0, 8, 9, 7], "end": "top" },
        });
        let b = Bank::from_json(&v1.to_string()).unwrap();
        assert_eq!(b.version, BANK_VERSION);
        assert_eq!(b.banks(), 2, "Bank A and Bank B");
        assert_eq!(b.memories.len(), 2 * SLOTS);
        let name = |i: usize| b.get(i).map(|m| m.name.clone());
        assert_eq!(name(0).as_deref(), Some("One"));
        assert_eq!(name(7).as_deref(), Some("Eight"), "A8");
        assert_eq!(name(8).as_deref(), Some("Nine"), "old button 9 is B1");
        assert_eq!(name(9).as_deref(), Some("Ten"), "old button 10 is B2");
        assert!(b.memories[10..].iter().all(Option::is_none));
        assert_eq!(b.sequence.clone().clean().steps, [0, 8, 9, 7], "the sequence still points at them");
        assert_eq!(b.stored_mask(), 1 | 1 << 7 | 1 << 8 | 1 << 9);
        // Saved as version 2, and back.
        let text = b.to_json();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["version"], 2);
        assert_eq!(v["memories"].as_array().unwrap().len(), 16);
        let back = Bank::from_json(&text).unwrap();
        assert_eq!(back, b);
        // Every stored snapshot is exactly what the version 1 file held, contents and all.
        for i in [0, 7, 8, 9] {
            let orig: Memory = serde_json::from_value(v1["memories"][i].clone()).unwrap();
            assert_eq!(back.get(i), Some(&orig));
        }
        // A version 1 file with buttons 9 and 10 empty is just Bank A.
        let short = serde_json::json!({ "format": BANK_FORMAT, "version": 1, "name": "x",
            "memories": [mem("One", 90.0), null, null, null, null, null, null, null, null, null] });
        assert_eq!(Bank::from_json(&short.to_string()).unwrap().banks(), 1);
    }

    #[test]
    fn storing_grows_and_clearing_shrinks_by_whole_banks() {
        let mut b = Bank::new("x");
        assert_eq!(b.banks(), 1);
        b.set(17, Some(Memory::default()));
        assert_eq!((b.banks(), b.memories.len()), (3, 24), "C2 needs Banks A-C");
        b.set(17, None);
        assert_eq!(b.banks(), 1);
        b.set(MAX_SLOTS, Some(Memory::default()));
        assert_eq!(b.banks(), 1, "beyond Bank H is refused");
    }

    #[test]
    fn bank_round_trips_and_pads_to_whole_snapshot_banks() {
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
        assert_eq!(Bank::from_json(short).unwrap().memories.len(), SLOTS);
        assert!(Bank::from_json(r#"{"format":"other","version":1,"name":"x","memories":[]}"#).is_err());
    }

    #[test]
    fn voice_ref_is_tagged_by_kind() {
        let v = VoiceRef::gm(48);
        assert_eq!(serde_json::to_string(&v).unwrap(), r#"{"kind":"gm","program":48,"bankMsb":0,"bankLsb":0}"#);
        let back: VoiceRef = serde_json::from_str(r#"{"kind":"gm","program":5}"#).unwrap();
        assert_eq!(back.program(), Some(5));
        assert!(serde_json::from_str::<VoiceRef>(r#"{"kind":"clap","id":"x"}"#).is_err());
        let p: VoiceRef = serde_json::from_str(r#"{"kind":"plugin","id":"aumu dls  appl","program":4}"#).unwrap();
        assert_eq!(p, VoiceRef::Plugin { id: "aumu dls  appl".into(), name: String::new(), state: None, program: 4, sound: None });
        assert_eq!(p.program(), Some(4));
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

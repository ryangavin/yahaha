//! Racks (docs/racks.md): what's under the player's hands, saved, recalled and swapped as
//! a whole. A [`Rack`] holds the four keyboard parts (Right 1-3 and Left), each with its
//! sound and mix, plus the split point, Harmony/Arpeggio, the keyboard transpose and the
//! controller map.
//!
//! This module is the model and its file, `Racks/<name>.rack.json` in the data folder
//! (format `yahaha.rack`, version 1). The session captures a rack from what plays and
//! applies one back (src/session/racks.rs). Nothing here depends on the session or on
//! Registration Memory.
//!
//! - A part's sound is a reference ([`SoundRef`]): a library sound by id, or a SoundFont
//!   preset. A plugin sound that differs from its saved sound carries the difference as the
//!   part's `edited_state`, never written into the sound.
//! - Mix (level, pan, sends, octave, tone, bend range, on/off) belongs to the rack part,
//!   never to the sound.
//! - Every write is atomic. A file made by a newer yahaha (a higher version) is refused and
//!   never saved over. Fields this build does not know are kept and written back.

pub mod quick;
mod settings;
pub mod style_racks;
#[cfg(test)]
mod tests;

pub use settings::{HarmonyArpReg, ToneReg};

use crate::data_files::{file_name, file_stem, list_files, write_atomic};
pub use crate::fx::part_eq::PartEq;
use anyhow::{Context, Result};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

/// A rack file's `format` field.
pub const FORMAT: &str = "yahaha.rack";
/// The rack file version this build reads and writes.
pub const VERSION: u32 = 1;
/// A rack file's name ends with this.
pub const EXT: &str = ".rack.json";
/// The folder in the data folder that holds the user's racks.
pub const DIR: &str = "Racks";

/// The four keyboard parts, in `parts` order: Right 1, Right 2, Right 3, Left.
pub const PARTS: usize = 4;

/// One rack.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rack {
    pub format: String,
    pub version: u32,
    /// Stable: Quick Racks and style racks name a rack by it, so a rename keeps them.
    pub id: String,
    pub name: String,
    /// Right 1, Right 2, Right 3, Left.
    pub parts: [RackPart; PARTS],
    /// The split point (a MIDI note): Left plays below it.
    pub split: u8,
    pub harmony_arp: HarmonyArpReg,
    /// Keyboard transpose, in semitones (Master transpose is not the rack's).
    pub transpose: i8,
    /// What faders 1-4 and knobs 1-8 do on the Rack knob page.
    #[serde(default)]
    pub controls: ControlMap,
    /// Fields a newer build wrote: kept, and written back as they were.
    #[serde(flatten)]
    pub other: Map<String, Value>,
}

/// One keyboard part of a rack: what plays, and how it is mixed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RackPart {
    pub on: bool,
    pub sound: SoundRef,
    /// A plugin sound's state (base64) when it differs from the saved sound's (an edit not
    /// saved yet), or the state of a plugin that is no library sound. None: the sound as
    /// saved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edited_state: Option<String>,
    /// The GM program the part has underneath a library or plugin sound: what its channel
    /// is set to, and what plays where the sound can't. None for a font preset (its
    /// program is the sound's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_program: Option<u8>,
    /// CC7.
    pub volume: u8,
    /// CC10 (64 = centre).
    pub pan: u8,
    /// CC91.
    pub reverb: u8,
    /// CC93.
    pub chorus: u8,
    /// CC94 (the Variation block).
    pub variation: u8,
    /// -2..=2.
    pub octave: i8,
    /// The voice settings the panel or an OTS set (filter, EG, vibrato, portamento, XG
    /// part parameters). Empty: the voice's own.
    #[serde(default, skip_serializing_if = "ToneReg::is_empty")]
    pub tone: ToneReg,
    /// Pitch Bend Range in semitones.
    pub bend_range: u8,
    /// The channel-strip EQ (#247). Missing (a rack saved before it, or flat): flat, and a
    /// flat one is left out of the file.
    #[serde(default, skip_serializing_if = "PartEq::is_default")]
    pub eq: PartEq,
    /// Fields a newer build wrote: kept, and written back as they were.
    #[serde(flatten)]
    pub other: Map<String, Value>,
}

/// What a rack part plays, by reference: saving a sound changes every rack that uses it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum SoundRef {
    /// A sound in the sound library, by its id (a plugin sound, or a font preset added to
    /// the library).
    Library { id: String },
    /// A SoundFont preset: its file name in the SoundFont folder, bank (128 = drum kits)
    /// and program. The GM voices are the main font's bank 0 presets.
    Font { file: String, bank: u16, program: u8 },
    /// A plugin picked with no library sound (its default preset, or settings of its
    /// own): its component id. Its settings are the part's `edited_state`.
    Plugin { component: String },
}

/// What a controller does on the Rack knob page.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ControlTarget {
    /// Nothing.
    #[default]
    None,
    /// A keyboard part's level (CC7); `part` 0-3 is Right 1, Right 2, Right 3, Left.
    PartLevel { part: u8 },
    /// A keyboard part's pan (CC10).
    PartPan { part: u8 },
    /// A keyboard part's reverb send (CC91).
    PartReverb { part: u8 },
    /// A keyboard part's chorus send (CC93).
    PartChorus { part: u8 },
    /// The HARMONY/ARPEGGIO switch.
    HarmonyArp,
    /// The split point.
    SplitPoint,
    /// The Keyboard Harmony volume.
    HarmonyVolume,
    /// The metronome's volume.
    MetronomeVolume,
    /// The tempo (knobs only: a fader has no tempo range).
    Tempo,
    /// A target this build doesn't know (a newer build's), kept verbatim so it is written
    /// back unchanged. It does nothing here.
    #[serde(untagged)]
    Unknown(Value),
}

impl ControlTarget {
    /// A target this build knows, with a part (if any) of 0-3.
    pub fn is_known(&self) -> bool {
        match self {
            ControlTarget::Unknown(_) => false,
            ControlTarget::PartLevel { part } | ControlTarget::PartPan { part } | ControlTarget::PartReverb { part } | ControlTarget::PartChorus { part } => {
                (*part as usize) < PARTS
            }
            _ => true,
        }
    }
}

/// The rack's controller map: faders 1-4 and knobs 1-8.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "SavedMap", into = "MapOut")]
pub struct ControlMap {
    pub faders: [ControlTarget; 4],
    pub knobs: [ControlTarget; 8],
}

impl Default for ControlMap {
    /// Today's Parts knob page and Panel faders: the four keyboard parts' levels on faders
    /// 1-4 and on knobs 1-4; Harmony volume, the metronome's volume, none and the tempo on
    /// knobs 5-8.
    fn default() -> ControlMap {
        let level = |p: u8| ControlTarget::PartLevel { part: p };
        let rest = [ControlTarget::HarmonyVolume, ControlTarget::MetronomeVolume, ControlTarget::None, ControlTarget::Tempo];
        let knobs = std::array::from_fn(|k| if k < 4 { level(k as u8) } else { rest[k - 4].clone() });
        ControlMap { faders: std::array::from_fn(|p| level(p as u8)), knobs }
    }
}

/// A controller in the controller map.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RackControl {
    Fader,
    Knob,
}

impl ControlMap {
    /// Controller `index` (0-based) now does `target`. Err: why not (a target this build
    /// doesn't know, the tempo on a fader, no such controller); the map is unchanged.
    pub fn set(&mut self, control: RackControl, index: u8, target: ControlTarget) -> Result<(), String> {
        if !target.is_known() {
            return Err(format!("no controller target {}", serde_json::to_string(&target).unwrap_or_default()));
        }
        if control == RackControl::Fader && target == ControlTarget::Tempo {
            return Err("a fader can't set the tempo: put it on a knob".into());
        }
        let (slots, name) = match control {
            RackControl::Fader => (&mut self.faders[..], "fader"),
            RackControl::Knob => (&mut self.knobs[..], "knob"),
        };
        let n = slots.len();
        let slot = slots.get_mut(index as usize).ok_or_else(|| format!("no {name} {} (1-{n})", index as usize + 1))?;
        *slot = target;
        Ok(())
    }

    /// The map every rack had before the map could be edited (and before Harmony volume,
    /// the metronome and the tempo were targets): the default, with none on knobs 5-8.
    fn first_default() -> ControlMap {
        let mut m = ControlMap::default();
        for k in &mut m.knobs[4..] {
            *k = ControlTarget::None;
        }
        m
    }
}

/// A controller map as a file has it.
#[derive(Deserialize)]
struct SavedMap {
    /// [`MAP_VERSION`] when this build (or a newer one) wrote it; missing (0) before.
    #[serde(default)]
    version: u32,
    #[serde(deserialize_with = "targets")]
    faders: [ControlTarget; 4],
    #[serde(deserialize_with = "targets")]
    knobs: [ControlTarget; 8],
}

impl From<SavedMap> for ControlMap {
    /// A map saved before it could be edited is that build's default, whose knobs 5-8 did
    /// nothing only because Harmony volume, the metronome and the tempo were no targets
    /// yet: it reads as today's default, so the Rack page keeps doing what the Parts page
    /// did.
    /// Only an unmarked map (written before the map had a version) is migrated: a marked
    /// one with none on knobs 5-8 was set that way on purpose.
    fn from(s: SavedMap) -> ControlMap {
        let m = ControlMap { faders: s.faders, knobs: s.knobs };
        if s.version == 0 && m == ControlMap::first_default() { ControlMap::default() } else { m }
    }
}

/// The controller map's version, written with every map so a later load knows it needs
/// no migration.
const MAP_VERSION: u32 = 1;

/// A controller map as it's written.
#[derive(Serialize)]
struct MapOut {
    version: u32,
    faders: [ControlTarget; 4],
    knobs: [ControlTarget; 8],
}

impl From<ControlMap> for MapOut {
    fn from(m: ControlMap) -> MapOut {
        MapOut { version: MAP_VERSION, faders: m.faders, knobs: m.knobs }
    }
}

/// A list of targets, read leniently: a target this build doesn't know (a newer build's)
/// is kept verbatim as [`ControlTarget::Unknown`], a missing one is none, and extras are
/// dropped.
fn targets<'de, D: Deserializer<'de>, const N: usize>(d: D) -> Result<[ControlTarget; N], D::Error> {
    let v = Vec::<Value>::deserialize(d)?;
    Ok(std::array::from_fn(|i| {
        v.get(i).map(|t| serde_json::from_value(t.clone()).unwrap_or_else(|_| ControlTarget::Unknown(t.clone()))).unwrap_or_default()
    }))
}

impl Rack {
    /// Read a rack file's text. A file of another format, or of a newer version than this
    /// build knows, is refused.
    pub fn from_json(text: &str) -> Result<Rack> {
        let v: Value = serde_json::from_str(text)?;
        check_header(&v)?;
        let mut r: Rack = serde_json::from_value(v)?;
        r.version = VERSION;
        Ok(r)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a rack serializes")
    }

    pub fn load(path: &Path) -> Result<Rack> {
        let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        Rack::from_json(&text).with_context(|| format!("reading {}", path.display()))
    }

    /// Write the rack to `path`, atomically. A file already there that a newer yahaha made
    /// (or that is no rack) is never saved over: the save is refused.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Ok(text) = std::fs::read_to_string(path)
            && let Ok(v) = serde_json::from_str::<Value>(&text)
        {
            check_header(&v).with_context(|| format!("not saving over {}", path.display()))?;
        }
        let mut r = self.clone();
        r.format = FORMAT.into();
        r.version = VERSION;
        write_atomic(path, &r.to_json())
    }
}

/// A rack file's format and version: ours, and no newer than this build.
fn check_header(v: &Value) -> Result<()> {
    let format = v.get("format").and_then(Value::as_str).unwrap_or_default();
    anyhow::ensure!(format == FORMAT, "not a yahaha rack (format {format:?})");
    let version = v.get("version").and_then(Value::as_u64).unwrap_or(0);
    anyhow::ensure!(version <= VERSION as u64, "made by a newer yahaha (rack version {version})");
    Ok(())
}

/// The racks folder in data folder `data`.
pub fn dir(data: &Path) -> PathBuf {
    data.join(DIR)
}

/// Where a rack called `name` is saved in `dir`.
pub fn path_for(dir: &Path, name: &str) -> PathBuf {
    dir.join(file_name(name, EXT))
}

/// The rack files in `dir`, sorted by name.
pub fn list(dir: &Path) -> Vec<PathBuf> {
    list_files(dir, EXT)
}

/// A rack's name from its file name ("Ballad.rack.json" -> "Ballad").
pub fn name_of(path: &Path) -> String {
    file_stem(path, EXT)
}

/// A new rack id: unique on this machine (the time, and a count within this run).
pub fn new_id() -> String {
    use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
    static COUNT: AtomicU32 = AtomicU32::new(0);
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_micros() as u64);
    format!("r{t:x}-{:x}", COUNT.fetch_add(1, Relaxed))
}

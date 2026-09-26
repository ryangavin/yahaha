//! Chord Looper (Genos CHORD LOOPER, RM p.14-19): record a chord progression and loop it;
//! eight memories. docs/chord-looper.md.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum LooperCmd {
    /// CHORD LOOPER [REC/STOP]. Playing: recording starts at the next bar line. Stopped:
    /// Sync Start turns on and the first chord starts the style and the recording. While
    /// recording: stops recording (the style plays on). Pressed again while armed: cancels.
    LooperRec,
    /// CHORD LOOPER [ON/OFF]. Recording: stops recording, and the loop starts at the next
    /// bar line. With a sequence: the loop starts at the next bar line (stopped: when the
    /// style starts). Looping: stops the loop at once; the keyboard's chords take over.
    LooperOnOff,
    /// Select memory `index` (0-7). One that holds a sequence replaces the current one;
    /// while looping, at the next bar line.
    SelectLooperMemory { index: u8 },
    /// Store the current sequence in memory `index` (0-7) (the Genos [Memory] button).
    StoreLooperMemory { index: u8 },
    /// Clear memory `index` (0-7).
    ClearLooperMemory { index: u8 },
    /// Clear all eight memories: a new, unsaved bank ("New Bank"). The current sequence
    /// stays.
    NewLooperBank,
    /// Save the bank: to its own file (`name` null), or as a file named `name` in the
    /// data folder's ChordLooper folder, which becomes its file. A name another bank's file
    /// has is refused unless `overwrite`.
    SaveLooperBank {
        name: Option<String>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        overwrite: bool,
    },
    /// Load a bank file (`banks` lists them): its memories replace the eight. Nothing is
    /// selected; a loop that plays goes on.
    LoadLooperBank { path: String },
}

/// Where the Chord Looper is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LooperMode {
    /// Not recording or looping (ON/OFF lit blue when there is a sequence).
    #[default]
    Off,
    /// REC/STOP flashing: recording starts at the next bar line (stopped: with the first
    /// chord).
    RecArmed,
    /// REC/STOP lit: recording the chords played.
    Recording,
    /// ON/OFF flashing: the loop starts at the next bar line (stopped: when the style
    /// starts).
    LoopArmed,
    /// ON/OFF lit: the loop plays the chords; the keyboard's are ignored.
    Looping,
}

/// One chord change of a sequence.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoopChord {
    /// Bar of the sequence, 1-based.
    pub bar: u32,
    /// Beat in the bar, 1-based, in quarter notes (2.5 = the "and" of beat 2).
    pub beat: f64,
    /// The chord as fingered, e.g. "Cm7".
    pub chord: String,
}

/// One of the eight memories.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LooperMemory {
    /// "CLD_001" and on, named as stored; None when empty.
    pub name: Option<String>,
    pub bars: u32,
    pub chords: Vec<LoopChord>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LooperState {
    pub mode: LooperMode,
    /// There is a sequence to loop (the latest recording, or a memory selected).
    pub has_data: bool,
    /// Recording: the bar being recorded; looping: the loop's bar playing (1-based).
    pub bar: Option<u32>,
    /// Recording: bars so far; otherwise the sequence's length.
    pub bars: u32,
    /// The current sequence's chord changes.
    pub chords: Vec<LoopChord>,
    /// The memory selected (0-7), if any.
    pub memory: Option<u8>,
    /// A memory selected while looping, waiting for the next bar line.
    pub pending_memory: Option<u8>,
    /// Memories 1-8 (always 8).
    pub memories: Vec<LooperMemory>,
    /// The bank's name ("New Bank" until it is saved or loaded).
    #[serde(default)]
    pub bank_name: String,
    /// Its file (null: not saved; the memories are still kept for the next session).
    #[serde(default)]
    pub bank_path: Option<String>,
    /// The bank files in the ChordLooper folder, by name.
    #[serde(default)]
    pub banks: Vec<super::BankFile>,
}

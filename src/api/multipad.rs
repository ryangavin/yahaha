//! Multi Pads (docs/genos-features.md §7, docs/multipad.md): the bank, the four pads,
//! Synchro Stop, and the bank files the library found.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum MultiPadCmd {
    /// Load a bank from the bank list (`MultiPadBankEntry::id`). Pads playing stop.
    LoadMultiPad { id: usize },
    /// Load a `.pad` file by path (added to the bank list if it isn't in it).
    LoadMultiPadPath { path: String },
    /// Unload the bank: the pads go dark.
    ClearMultiPad,
    /// Press pad `pad` (0-3): it plays from the top (a playing pad restarts), at once when
    /// the band is stopped, at the next bar line while it plays. Pads in Synchro Start
    /// standby start with it.
    TriggerMultiPad { pad: u8 },
    /// STOP + pad: stop one pad now.
    StopMultiPad { pad: u8 },
    /// STOP: stop every pad, and cancel Synchro Start standby.
    StopAllMultiPads,
    /// SELECT + pad: toggle the pad's Synchro Start standby. Armed pads start on the next
    /// chord in the chord section or when the band starts (at the next bar line while it
    /// plays).
    ArmMultiPad { pad: u8 },
    /// Override the pad's Repeat flag (from the bank file) until the next bank loads.
    SetMultiPadRepeat { pad: u8, on: bool },
    /// Override the pad's Chord Match flag until the next bank loads.
    SetMultiPadChordMatch { pad: u8, on: bool },
    /// Multi Pad Synchro Stop (Style Setting): stop repeating pads when the band stops
    /// (`styleStop`) and when an Ending starts (`ending`).
    SetMultiPadSynchroStop { style_stop: bool, ending: bool },
}

/// A pad's lamp, as on the Genos panel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PadLamp {
    /// No data: off.
    #[default]
    Empty,
    /// Has data: blue.
    Ready,
    /// Synchro Start standby: red, flashing.
    Armed,
    /// Pressed while the band plays, waiting for the next bar line.
    Queued,
    /// Playing: red.
    Playing,
}

/// Multi Pads.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiPadState {
    /// The bank loaded (None: none).
    pub bank: Option<MultiPadBank>,
    /// A bank is on its way to the engine (a moment after `LoadMultiPad`).
    pub loading: bool,
    /// Pads 1-4 (always 4).
    pub pads: Vec<MultiPadPad>,
    pub synchro_stop: MultiPadSynchroStop,
    /// The `.pad` files in the style folders (`library.roots`), folder then name; refreshed
    /// by `RescanLibrary`.
    pub banks: Vec<MultiPadBankEntry>,
}

/// The loaded bank.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiPadBank {
    /// Its bank list id (`MultiPadBankEntry::id`).
    pub id: usize,
    /// The file name without `.pad`.
    pub name: String,
    pub path: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiPadPad {
    /// 0-3.
    pub index: u8,
    /// From the bank file ("" for an empty pad).
    pub name: String,
    pub lamp: PadLamp,
    /// Loops until stopped (else plays once).
    pub repeat: bool,
    /// Follows the chord (else plays as written).
    pub chord_match: bool,
    /// The MIDI channel it plays on, 1-based: pads 1-4 on channels 5-8.
    pub channel: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiPadSynchroStop {
    pub style_stop: bool,
    pub ending: bool,
}

impl Default for MultiPadSynchroStop {
    fn default() -> Self {
        let d = crate::engine::SynchroStop::default();
        MultiPadSynchroStop { style_stop: d.style_stop, ending: d.ending }
    }
}

/// A bank file the library found.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiPadBankEntry {
    /// Stable for the session: `LoadMultiPad { id }`.
    pub id: usize,
    pub name: String,
    /// Folder relative to the scanned root, `/`-separated.
    pub folder: String,
    pub path: String,
}

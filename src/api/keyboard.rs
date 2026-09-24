//! The keys held and the chord, for the app's keyboard strip.

use serde::{Deserialize, Serialize};

/// The keyboard as the key strip draws it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardState {
    /// The keys held, from any keyboard source, low to high.
    pub held: Vec<HeldNote>,
    /// Split Point (Left): keys at or below it play the Left part (the same split as
    /// `chord.split`; yahaha has one).
    pub left_split: u8,
    /// Pitch classes (0-11, C = 0) of the chord as fingered (`chord.fingered`), root
    /// first; empty for none.
    pub chord_tones: Vec<u8>,
    /// Its bass (pitch class): the root, or the slash / on-bass note. None: no chord.
    pub chord_bass: Option<u8>,
    /// The keys chord detection reads, as [lo, hi] MIDI notes (inclusive): up to the split
    /// in Lower, above it in Upper (Fingered*), every key in the Full Keyboard types.
    pub detection: [u8; 2],
}

/// A key held.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeldNote {
    /// The MIDI note as played (before Keyboard transpose and the parts' octaves).
    pub note: u8,
    /// The side of the split it went to when pressed.
    pub zone: Zone,
    /// The keyboard parts sounding it (0-3 = Right 1, Right 2, Right 3, Left); empty for
    /// a key that only gives the chord.
    pub parts: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Zone {
    /// At or below the split: the Left part, the chord section in Lower.
    #[default]
    Left,
    Right,
}

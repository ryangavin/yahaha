//! Chord detection, split, transpose.

use crate::fingering::Fingering;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ChordCmd {
    /// Select a fingering type.
    SetFingering { fingering: Fingering },
    /// Step to the next fingering type (display order, wrapping).
    NextFingering,
    /// Chord Detection Area: Upper (true) or Lower. Selecting Upper turns Manual Bass on.
    SetUpper { on: bool },
    ToggleUpper,
    /// The Manual Bass setting. Only changes in Upper (ignored in Lower).
    SetManualBass { on: bool },
    ToggleManualBass,
    /// Split point, a MIDI note (clamped to 24-96). Keys at or below it are the left hand.
    SetSplit { note: u8 },
    /// Move the split point by `delta` keys.
    MoveSplit { delta: i8 },
    /// Keyboard and Master transpose in semitones (each clamped to -12..=12).
    SetTranspose { keyboard: i8, master: i8 },
    /// Add to the Keyboard and Master transpose.
    StepTranspose { keyboard: i8, master: i8 },
    /// Keyboard and Master transpose back to 0.
    ResetTranspose,
    /// The chord-settle window, in ms (clamped to 0-`CHORD_SETTLE_MAX_MS`): while the style
    /// plays, a chord change reaches the accompaniment once the chord has held still this
    /// long, so a rolled chord is followed once (docs/genos-features.md, Chord settle).
    SetChordSettle { ms: u32 },
}

/// Chord detection.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChordState {
    /// The chord the style follows (after Keyboard transpose), e.g. "Am7/G".
    pub name: Option<String>,
    /// The chord as fingered, before Keyboard transpose.
    pub fingered: Option<String>,
    pub fingering: Fingering,
    /// Display name, e.g. "Fingered On Bass".
    pub fingering_name: String,
    /// Chord Detection Area = Upper (the chord comes from the right hand, as Fingered*).
    pub upper: bool,
    /// The Manual Bass setting.
    pub manual_bass: bool,
    /// Manual Bass in effect (Upper and the setting on): the left hand plays the Style's
    /// Bass voice and the Style's Bass part is muted.
    pub manual_bass_active: bool,
    /// Split point, a MIDI note: keys at or below it are the left hand.
    pub split: u8,
    /// The split point in Yamaha octave numbering (C3 = 60), e.g. "F#2".
    pub split_name: String,
    /// Keyboard transpose (the keys and the chord), semitones -12..=12.
    pub transpose_keyboard: i8,
    /// Master transpose (everything that sounds but drum kits), semitones -12..=12.
    pub transpose_master: i8,
    /// The chord-settle window, in ms (`SetChordSettle`).
    pub settle_ms: u32,
}

//! Keyboard parts (Right 1-3, Left).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PartsCmd {
    /// Turn a part on/off. Left is refused under Manual Bass (it plays the bass then).
    SetPartOn { part: u8, on: bool },
    TogglePart { part: u8 },
    /// The part the voice commands (`StepVoice`) and the Launchkey voice pads edit.
    SelectPart { part: u8 },
    /// Set a part's voice (GM program 0-127).
    SetPartVoice { part: u8, program: u8 },
    /// Previous/next voice for the selected part.
    StepVoice { delta: i8 },
    /// A part's volume (its CC7, 0-127). The Launchkey fader picks it up.
    SetPartVolume { part: u8, volume: u8 },
    /// A part's octave shift (-2..=2).
    SetPartOctave { part: u8, octave: i8 },
}

/// A keyboard part: Right 1, Right 2, Right 3 or Left.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardPart {
    /// "Right 1", "Right 2", "Right 3", "Left".
    pub name: String,
    /// MIDI channel, 1-based (Right 1 = 1, Left = 2, Right 2 = 3, Right 3 = 4).
    pub channel: u8,
    /// The part's on/off switch.
    pub on: bool,
    /// It sounds: on, or Left playing the bass under Manual Bass.
    pub sounding: bool,
    /// The part the voice commands edit.
    pub selected: bool,
    /// Volume (its CC7), 0-127.
    pub volume: u8,
    /// The Launchkey fader has moved but not yet reached `volume` (soft takeover).
    pub waiting: bool,
    /// Where its Launchkey fader (Panel page, faders 1-4) physically is; None if it hasn't
    /// moved.
    pub fader: Option<u8>,
    /// The part's own voice, a GM program 0-127.
    pub program: u8,
    /// What its channel plays: its voice, or the Style's Bass voice under Manual Bass.
    pub voice_name: String,
    /// Left playing the Style's Bass voice (Manual Bass).
    pub plays_bass: bool,
    /// The octave setting, -2..=2 (not applied while `plays_bass`).
    pub octave: i8,
    /// Its own sound library patch (`setPartPatch`), if it has one; else its GM voice
    /// plays, through the program map (`voiceName` names the patch it resolves to).
    #[serde(default)]
    pub patch: Option<String>,
}

//! Controllers: pedals, the pitch-bend and modulation wheels, and the pedals' assignable
//! functions (`crate::controllers`).

use crate::controllers::{ControlType, Function, PedalSetup, Range};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ControllersCmd {
    /// Set up pedal `pedal` (0-2): the CC it listens for on the keyboards (null: none), its
    /// function (`Function`, e.g. "sustain", "startStop", "fillUp"), Control Type
    /// ("holdA", "holdB", "toggle"), reversed polarity, and Range for Pitch Bend ("upper",
    /// "lower", "full").
    SetPedal {
        pedal: u8,
        cc: Option<u8>,
        function: Function,
        #[serde(default)]
        control_type: ControlType,
        #[serde(default)]
        reverse: bool,
        #[serde(default)]
        range: Range,
    },
    /// Pedal `pedal` takes the CC of the next control change a keyboard sends (a pedal
    /// press); null stops learning.
    LearnPedal { pedal: Option<u8> },
    /// Which controllers reach keyboard part `part` (0-3): the pedal switches (sustain,
    /// sostenuto, soft), pitch bend, modulation.
    SetPartControllers { part: u8, sustain: bool, pitch_bend: bool, modulation: bool },
    /// Keyboard part `part`'s Pitch Bend Range, 0-12 semitones.
    SetBendRange { part: u8, semitones: u8 },
    /// Run an assignable function now, as a pedal press would.
    TriggerFunction { function: Function },
}

impl ControllersCmd {
    /// A `SetPedal`'s setup.
    pub fn pedal_setup(&self) -> Option<PedalSetup> {
        match *self {
            ControllersCmd::SetPedal { cc, function, control_type, reverse, range, .. } => {
                Some(PedalSetup { cc: cc.filter(|&c| c < 128), function, control_type, reverse, range })
            }
            _ => None,
        }
    }
}

/// Controllers.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllersState {
    /// The pedals (always 3).
    pub pedals: Vec<PedalState>,
    /// The pedal learning its CC (0-2), if any.
    pub learning: Option<u8>,
    /// Right 1, Right 2, Right 3, Left (always 4): what reaches each.
    pub parts: Vec<PartControllers>,
    /// The pedal switches in effect now.
    pub sustain: bool,
    pub sostenuto: bool,
    pub soft: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PedalState {
    /// The CC it listens for, or null.
    pub cc: Option<u8>,
    pub function: Function,
    pub control_type: ControlType,
    pub reverse: bool,
    pub range: Range,
    /// Held down now.
    pub down: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartControllers {
    /// The pedal switches (sustain, sostenuto, soft) reach it.
    pub sustain: bool,
    pub pitch_bend: bool,
    pub modulation: bool,
    /// Pitch Bend Range in semitones, 0-12.
    pub bend_range: u8,
}

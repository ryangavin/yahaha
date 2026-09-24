//! Metronome (Genos Menu > Metronome, RM p.39): a click on each beat, a bell on beat 1.
//! It sounds on the built-in synth only (a click voice, not a MIDI part), never on the
//! MIDI port.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum MetronomeCmd {
    ToggleMetronome,
    SetMetronome { on: bool },
    /// The click's volume, 0-127.
    SetMetronomeVolume { volume: u8 },
    /// A bell on the first beat of each bar.
    SetMetronomeBell { on: bool },
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetronomeState {
    pub on: bool,
    /// 0-127.
    pub volume: u8,
    pub bell: bool,
    /// The built-in synth is running (the only place the click sounds).
    pub audible: bool,
}

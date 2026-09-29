//! The effect blocks' parameters (#236): each one a whole number in its own unit, with a
//! range, a name and how it reads. The control side sets them (`FxControl::params`, one
//! atomic each); the audio thread reads them once per buffer and glides to them, so a
//! change never clicks. A type change puts its block's parameters back to that type's
//! defaults (the Genos loads a type with its own settings), on the control side.

use serde::{Deserialize, Serialize};

/// An effect parameter. Its value is a whole number in the unit `Param::spec` gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[repr(u8)]
pub enum Param {
    /// Reverb decay time (RT60), tenths of a second: 3-100 (0.3-10 s).
    ReverbTime = 0,
    /// Reverb pre-delay, ms: 0-200.
    PreDelay = 1,
    /// Reverb tone: the tank's high cut, 100 Hz steps: 10-200 (1-20 kHz). Lower is darker
    /// (more high-frequency damping).
    ReverbTone = 2,
    /// Delay tempo sync: 1 = the time is `DelayNote` at the style tempo, 0 = `DelayTime`.
    DelaySync = 3,
    /// Delay note value: an index into `super::delay::NOTES` (1/16 ... 1/2), 0-7.
    DelayNote = 4,
    /// Delay free time (tempo sync off), ms: 10-2000.
    DelayTime = 5,
    /// Delay feedback, %: 0-90.
    DelayFeedback = 6,
    /// Delay tone: the high cut on the repeats, 100 Hz steps: 10-200 (1-20 kHz).
    DelayTone = 7,
    /// Delay ping-pong: 1 = the repeats alternate left and right.
    PingPong = 8,
    /// Chorus rate: the LFO speed, 0.01 Hz steps: 5-500 (0.05-5 Hz).
    ChorusRate = 9,
    /// Chorus depth: how far the taps swing, 0.1 ms steps: 0-50 (0-5 ms).
    ChorusDepth = 10,
}

/// How many parameters there are (`FxControl::params`).
pub const PARAMS: usize = 11;

/// A parameter's block, range and names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spec {
    /// The bus it belongs to (`super::REVERB`, `CHORUS`, `VARIATION`).
    pub block: usize,
    pub min: u16,
    pub max: u16,
    /// "Time".
    pub name: &'static str,
    /// Up to 8 characters, for a knob: "RevTime".
    pub short: &'static str,
    /// With its block, for a knob: "Reverb Time".
    pub full: &'static str,
    /// How far a knob step moves it.
    pub step: u16,
}

impl Param {
    pub const ALL: [Param; PARAMS] = [
        Param::ReverbTime,
        Param::PreDelay,
        Param::ReverbTone,
        Param::DelaySync,
        Param::DelayNote,
        Param::DelayTime,
        Param::DelayFeedback,
        Param::DelayTone,
        Param::PingPong,
        Param::ChorusRate,
        Param::ChorusDepth,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn spec(self) -> Spec {
        let s = |block, min, max, name, short, full, step| Spec { block, min, max, name, short, full, step };
        match self {
            Param::ReverbTime => s(super::REVERB, 3, 100, "Time", "RevTime", "Reverb Time", 1),
            Param::PreDelay => s(super::REVERB, 0, 200, "Pre-delay", "PreDly", "Reverb Pre-delay", 2),
            Param::ReverbTone => s(super::REVERB, 10, 200, "Tone", "RevTone", "Reverb Tone", 2),
            Param::DelaySync => s(super::VARIATION, 0, 1, "Tempo sync", "DlySync", "Delay Tempo Sync", 1),
            Param::DelayNote => s(super::VARIATION, 0, super::delay::NOTES.len() as u16 - 1, "Note", "DlyNote", "Delay Note", 1),
            Param::DelayTime => s(super::VARIATION, 10, 2000, "Time", "DlyTime", "Delay Time", 10),
            Param::DelayFeedback => s(super::VARIATION, 0, 90, "Feedback", "DlyFdbk", "Delay Feedback", 2),
            Param::DelayTone => s(super::VARIATION, 10, 200, "Tone", "DlyTone", "Delay Tone", 2),
            Param::PingPong => s(super::VARIATION, 0, 1, "Ping-pong", "PingPong", "Delay Ping-pong", 1),
            Param::ChorusRate => s(super::CHORUS, 5, 500, "Rate", "ChoRate", "Chorus Rate", 2),
            Param::ChorusDepth => s(super::CHORUS, 0, 50, "Depth", "ChoDepth", "Chorus Depth", 1),
        }
    }

    /// The parameters of bus `block`, in order.
    pub fn of_block(block: usize) -> impl Iterator<Item = Param> {
        Param::ALL.into_iter().filter(move |p| p.spec().block == block)
    }

    /// `v` in range.
    pub fn clamp(self, v: u16) -> u16 {
        let s = self.spec();
        v.clamp(s.min, s.max)
    }

    /// `v` as it reads: "2.4 s", "22 ms", "4.5 kHz".
    pub fn display(self, v: u16) -> String {
        let v = self.clamp(v);
        match self {
            Param::ReverbTime => format!("{:.1} s", v as f32 / 10.0),
            Param::PreDelay => format!("{v} ms"),
            Param::ReverbTone | Param::DelayTone => format!("{:.1} kHz", v as f32 / 10.0),
            Param::DelaySync | Param::PingPong => if v != 0 { "On" } else { "Off" }.into(),
            Param::DelayNote => super::delay::NOTES[v as usize].1.into(),
            Param::DelayTime => format!("{v} ms"),
            Param::DelayFeedback => format!("{v}%"),
            Param::ChorusRate => format!("{:.2} Hz", v as f32 / 100.0),
            Param::ChorusDepth => format!("{:.1} ms", v as f32 / 10.0),
        }
    }
}

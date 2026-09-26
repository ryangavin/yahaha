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
}

/// How many parameters there are (`FxControl::params`).
pub const PARAMS: usize = 3;

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
}

impl Param {
    pub const ALL: [Param; PARAMS] = [Param::ReverbTime, Param::PreDelay, Param::ReverbTone];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn spec(self) -> Spec {
        let s = |block, min, max, name, short| Spec { block, min, max, name, short };
        match self {
            Param::ReverbTime => s(super::REVERB, 3, 100, "Time", "RevTime"),
            Param::PreDelay => s(super::REVERB, 0, 200, "Pre-delay", "PreDly"),
            Param::ReverbTone => s(super::REVERB, 10, 200, "Tone", "RevTone"),
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
            Param::ReverbTone => format!("{:.1} kHz", v as f32 / 10.0),
        }
    }
}

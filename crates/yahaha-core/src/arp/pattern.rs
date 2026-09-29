//! The arpeggio pattern format: a loop of steps, each picking notes from the held chord.
//!
//! Timing is written at [`PATTERN_PPQ`] (480 ticks per quarter note) and scaled to the
//! engine's resolution at playback, so a pattern reads the same at any PPQ.

use std::borrow::Cow;

/// Ticks per quarter note that pattern step lengths and strum spreads are written in.
pub const PATTERN_PPQ: u32 = 480;

/// Common step lengths at [`PATTERN_PPQ`].
pub mod len {
    pub const QUARTER: u16 = 480;
    pub const EIGHTH: u16 = 240;
    pub const EIGHTH_TRIPLET: u16 = 160;
    pub const SIXTEENTH: u16 = 120;
    pub const SIXTEENTH_TRIPLET: u16 = 80;
    pub const THIRTY_SECOND: u16 = 60;
}

/// Where a pattern belongs in the browser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Category {
    /// Walks up, down or both through the held notes.
    UpDown,
    /// Seeded random picks: the same every time the pattern restarts.
    Random,
    /// The held notes in the order they were pressed.
    AsPlayed,
    /// The whole chord at once, in a rhythm.
    ChordStab,
    /// Fixed figures over the chord tones (Alberti, waltz, fingerpicking).
    BrokenChord,
    /// The chord rolled across "strings" in a strumming rhythm.
    Guitar,
    /// Synth sequencer lines: a note or two with octave jumps and accents.
    Sequence,
}

/// How a [`Sel::Walk`] step moves through the held notes (spread over the pattern's
/// octave range).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Motion {
    Up,
    Down,
    /// Up then down, not repeating the top and bottom notes.
    UpDown,
    /// Down then up, not repeating the top and bottom notes.
    DownUp,
    /// A seeded random pick that never repeats the previous note (when there is a choice).
    Random,
    /// The order the keys were pressed.
    AsPlayed,
}

/// The order [`Sel::Idx`] and [`Sel::Top`] count the held notes in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Sort {
    /// Lowest pitch first.
    Pitch,
    /// First pressed first.
    Played,
}

/// Which held notes a step plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Sel {
    /// Silence.
    Rest,
    /// The next note of the pattern's [`Motion`].
    Walk,
    /// The i-th held note counting from the bottom (in the pattern's [`Sort`] order).
    /// Past the last note it wraps and goes up an octave: with three notes, 3 is the
    /// first note an octave up.
    Idx(u8),
    /// The i-th held note counting from the top. Past the first note it wraps and goes
    /// down an octave.
    Top(u8),
    /// Every held note at once.
    All,
    /// Every held note rolled across the chord, `spread` ticks (at [`PATTERN_PPQ`])
    /// apart. `up: false` is a downstroke in the guitar sense (low string first, so low
    /// to high); `up: true` is an upstroke (high to low).
    Strum { up: bool, spread: u16 },
}

/// One step of a pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Step {
    pub sel: Sel,
    /// Octave shift added to every note of the step.
    pub oct: i8,
    /// Velocity 1-127 (used by the Original velocity mode, and as the accent in Thru).
    pub vel: u8,
    /// Note length as a percent of the step length. Over 100 overlaps the next step.
    pub gate: u8,
}

impl Step {
    pub const fn new(sel: Sel, oct: i8, vel: u8, gate: u8) -> Step {
        Step { sel, oct, vel, gate }
    }
    pub const REST: Step = Step::new(Sel::Rest, 0, 0, 0);
}

/// An arpeggio pattern: a loop of equally long steps.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Pattern {
    pub name: Cow<'static, str>,
    pub category: Category,
    /// Step length in ticks at [`PATTERN_PPQ`].
    pub step_len: u16,
    /// 50 plays straight; higher delays every second step, 67 is about a triplet shuffle,
    /// 75 is a dotted feel. Clamped to 50..=75.
    pub swing: u8,
    pub motion: Motion,
    /// Octaves a [`Sel::Walk`] spans (1 = just the held notes).
    pub octaves: u8,
    pub sort: Sort,
    /// Seed for [`Motion::Random`]; the sequence restarts with the pattern.
    pub seed: u32,
    pub steps: Cow<'static, [Step]>,
}

impl Pattern {
    /// Length of one loop in ticks at [`PATTERN_PPQ`].
    pub fn loop_len(&self) -> u32 {
        self.step_len as u32 * self.steps.len() as u32
    }

    /// Checks a pattern can be played: at least one step, a nonzero step length, and
    /// octave and velocity values in range.
    pub fn validate(&self) -> Result<(), String> {
        if self.steps.is_empty() {
            return Err(format!("{}: no steps", self.name));
        }
        if self.step_len == 0 {
            return Err(format!("{}: zero step length", self.name));
        }
        if !(1..=4).contains(&self.octaves) {
            return Err(format!("{}: octaves must be 1-4", self.name));
        }
        for (i, s) in self.steps.iter().enumerate() {
            if s.sel != Sel::Rest && !(1..=127).contains(&s.vel) {
                return Err(format!("{}: step {i} velocity out of range", self.name));
            }
            if s.sel != Sel::Rest && s.gate == 0 {
                return Err(format!("{}: step {i} has zero gate", self.name));
            }
            if !(-3..=3).contains(&s.oct) {
                return Err(format!("{}: step {i} octave out of range", self.name));
            }
        }
        Ok(())
    }
}

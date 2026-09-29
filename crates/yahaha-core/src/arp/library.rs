//! The starter pattern library. Every pattern here is our own, written for yahaha. None
//! is taken or transcribed from Yamaha's preset arpeggio data (see docs/arpeggio.md).

use super::pattern::{len, Category, Motion, Pattern, Sel, Sort, Step};
use std::borrow::Cow;

const R: Step = Step::REST;

/// A walk step.
const fn w(vel: u8, gate: u8) -> Step {
    Step::new(Sel::Walk, 0, vel, gate)
}
/// The i-th note from the bottom, shifted `oct` octaves.
const fn i(idx: u8, oct: i8, vel: u8, gate: u8) -> Step {
    Step::new(Sel::Idx(idx), oct, vel, gate)
}
/// The i-th note from the top.
const fn t(idx: u8, oct: i8, vel: u8, gate: u8) -> Step {
    Step::new(Sel::Top(idx), oct, vel, gate)
}
/// The whole chord.
const fn a(vel: u8, gate: u8) -> Step {
    Step::new(Sel::All, 0, vel, gate)
}
/// A downstroke (low to high).
const fn dn(spread: u16, vel: u8, gate: u8) -> Step {
    Step::new(Sel::Strum { up: false, spread }, 0, vel, gate)
}
/// An upstroke (high to low).
const fn up(spread: u16, vel: u8, gate: u8) -> Step {
    Step::new(Sel::Strum { up: true, spread }, 0, vel, gate)
}

#[allow(clippy::too_many_arguments)]
const fn p(
    name: &'static str,
    category: Category,
    step_len: u16,
    swing: u8,
    motion: Motion,
    octaves: u8,
    seed: u32,
    steps: &'static [Step],
) -> Pattern {
    Pattern {
        name: Cow::Borrowed(name),
        category,
        step_len,
        swing,
        motion,
        octaves,
        sort: Sort::Pitch,
        seed,
        steps: Cow::Borrowed(steps),
    }
}

use Category::*;

/// The starter library, grouped by category.
#[rustfmt::skip]
pub static PATTERNS: &[Pattern] = &[
    // --- Up & Down ---------------------------------------------------------------
    p("Climb 16", UpDown, len::SIXTEENTH, 50, Motion::Up, 1, 0,
        &[w(100, 80), w(64, 60), w(80, 60), w(64, 60)]),
    p("Fall 16", UpDown, len::SIXTEENTH, 50, Motion::Down, 1, 0,
        &[w(100, 80), w(64, 60), w(80, 60), w(64, 60)]),
    p("Peak 8", UpDown, len::EIGHTH, 50, Motion::UpDown, 2, 0,
        &[w(96, 85), w(72, 85)]),
    p("Valley Triplet", UpDown, len::EIGHTH_TRIPLET, 50, Motion::DownUp, 1, 0,
        &[w(100, 70), w(70, 70), w(76, 70)]),
    p("Sky Ladder 32", UpDown, len::THIRTY_SECOND, 50, Motion::Up, 3, 0,
        &[w(90, 50), w(60, 50), w(70, 50), w(60, 50)]),
    // --- Random ------------------------------------------------------------------
    p("Dice 16", Random, len::SIXTEENTH, 50, Motion::Random, 1, 0x2545_F491,
        &[w(96, 70), w(70, 50), w(84, 60), w(64, 40)]),
    p("Scatter Octaves", Random, len::EIGHTH, 50, Motion::Random, 2, 0x9E37_79B9,
        &[w(100, 90), w(80, 90)]),
    // --- As Played ---------------------------------------------------------------
    p("Echo Order 8", AsPlayed, len::EIGHTH, 50, Motion::AsPlayed, 1, 0,
        &[w(100, 90), w(80, 90)]),
    p("Shuffle Order 16", AsPlayed, len::SIXTEENTH, 62, Motion::AsPlayed, 2, 0,
        &[w(100, 70), w(70, 60)]),
    // --- Chord stabs -------------------------------------------------------------
    p("Four Stabs", ChordStab, len::QUARTER, 50, Motion::Up, 1, 0,
        &[a(110, 40), a(90, 40), a(100, 40), a(90, 40)]),
    p("Offbeat Pump", ChordStab, len::EIGHTH, 50, Motion::Up, 1, 0,
        &[R, a(104, 45)]),
    p("Syncopated Hits", ChordStab, len::SIXTEENTH, 50, Motion::Up, 1, 0,
        &[a(112, 60), R, R, a(96, 60), R, R, a(104, 60), R,
          R, R, a(96, 60), R, a(100, 60), R, R, R]),
    p("Gated Pad 16", ChordStab, len::SIXTEENTH, 50, Motion::Up, 1, 0,
        &[a(104, 85), a(64, 35), a(84, 60), a(64, 35)]),
    // --- Broken chords -----------------------------------------------------------
    p("Alberti 16", BrokenChord, len::SIXTEENTH, 50, Motion::Up, 1, 0,
        &[i(0, 0, 96, 90), t(0, 0, 72, 90), i(1, 0, 80, 90), t(0, 0, 72, 90)]),
    p("Waltz Broken", BrokenChord, len::QUARTER, 50, Motion::Up, 1, 0,
        &[i(0, -1, 100, 90), a(72, 55), a(72, 55)]),
    p("Rolling Eights", BrokenChord, len::EIGHTH, 50, Motion::Up, 1, 0,
        &[i(0, 0, 100, 95), i(1, 0, 76, 95), i(2, 0, 80, 95), i(3, 0, 84, 95),
          i(4, 0, 88, 95), i(3, 0, 80, 95), i(2, 0, 76, 95), i(1, 0, 72, 95)]),
    p("Thumb Pick", BrokenChord, len::EIGHTH, 55, Motion::Up, 1, 0,
        &[i(0, -1, 100, 80), t(0, 0, 70, 60), i(1, -1, 90, 80), t(1, 0, 66, 60),
          i(0, -1, 96, 80), t(0, 0, 70, 60), i(1, -1, 88, 80), t(1, 0, 66, 60)]),
    // --- Guitar strums -----------------------------------------------------------
    p("Strum Quarters", Guitar, len::QUARTER, 50, Motion::Up, 1, 0,
        &[dn(20, 110, 90), dn(20, 90, 90), dn(20, 100, 90), dn(20, 90, 90)]),
    p("Campfire Strum", Guitar, len::EIGHTH, 50, Motion::Up, 1, 0,
        &[dn(18, 110, 95), R, dn(18, 96, 95), up(14, 80, 90),
          R, up(14, 84, 90), dn(18, 100, 95), up(14, 80, 90)]),
    p("Muted Sixteens", Guitar, len::SIXTEENTH, 54, Motion::Up, 1, 0,
        &[dn(8, 108, 40), up(6, 70, 25), dn(8, 90, 25), up(6, 72, 25)]),
    // --- Sequences ---------------------------------------------------------------
    p("Octave Pulse", Sequence, len::SIXTEENTH, 50, Motion::Up, 1, 0,
        &[i(0, 0, 104, 60), i(0, 1, 76, 50), i(0, 0, 88, 60), i(0, 0, 70, 50),
          i(0, 1, 96, 60), i(0, 0, 70, 50), i(0, 1, 86, 60), i(0, 0, 72, 50)]),
    p("Root Fifth Seq", Sequence, len::SIXTEENTH, 50, Motion::Up, 1, 0,
        &[i(0, 0, 108, 70), i(0, 0, 70, 40), t(0, 0, 90, 70), i(0, 1, 80, 40),
          i(0, 0, 100, 70), t(0, 0, 76, 40), i(1, 0, 90, 70), i(0, 1, 84, 40)]),
    p("Pluck Line", Sequence, len::SIXTEENTH, 58, Motion::Up, 1, 0,
        &[i(0, 0, 110, 30), R, i(2, 0, 84, 30), i(1, 1, 72, 30),
          R, i(0, 1, 96, 30), i(2, 0, 80, 30), R]),
];

/// Looks up a library pattern by name (case-insensitive).
pub fn find(name: &str) -> Option<&'static Pattern> {
    PATTERNS.iter().find(|p| p.name.eq_ignore_ascii_case(name))
}

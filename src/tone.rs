//! A keyboard part's tone controllers (#238): the CC numbers and their indices.
//!
//! Plain constants with no dependencies, below both `sff` (which reads them from the OTS
//! tracks) and `parts` (which holds them per part and re-exports them at `parts::*`).
//! They go to the core crate in the workspace split (#337).

/// The controllers `Parts::tone` holds (#238): filter cutoff and resonance, EG attack, decay
/// and release, vibrato rate, depth and delay (all relative: 64 = the voice's own), then
/// portamento switch and time. The Genos receives all of them on the keyboard parts (Data
/// List, MIDI Data Format: Control Change).
pub const TONE_CC: [u8; TONE] = [74, 71, 73, 75, 72, 76, 77, 78, 65, 5];
pub const TONE: usize = 10;
/// `Parts::tone` indices.
pub const CUTOFF: usize = 0;
pub const RESONANCE: usize = 1;
pub const ATTACK: usize = 2;
pub const DECAY: usize = 3;
pub const RELEASE: usize = 4;
pub const VIBRATO_RATE: usize = 5;
pub const VIBRATO_DEPTH: usize = 6;
pub const VIBRATO_DELAY: usize = 7;
pub const PORTAMENTO: usize = 8;
pub const PORTAMENTO_TIME: usize = 9;
/// What a voice change puts the `TONE_CC` controllers a part has set back to: the new
/// voice's own filter, EG and vibrato (64), portamento off and time 0 (XG defaults).
pub const TONE_NEUTRAL: [u8; TONE] = [64, 64, 64, 64, 64, 64, 64, 64, 0, 0];

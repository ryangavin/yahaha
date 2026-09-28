//! Keyboard parts, the Genos model: Right 1, Right 2, Right 3 and Left (OM p.48).
//!
//! The old `crate::parts::...` paths, re-exported (#337): the plain data (part ids,
//! channels, fader page and layer, send controllers) lives in `parts_data`, the tone
//! controllers in `tone`, both headed for the core crate; the `Parts` state needs the
//! engine and sff and lives in `engine`. Modules headed below the engine that need only
//! the data import `parts_data` or `tone` directly.

pub use crate::engine::Parts;
pub use crate::parts_data::*;
pub use crate::tone::{
    ATTACK, CUTOFF, DECAY, PORTAMENTO, PORTAMENTO_TIME, RELEASE, RESONANCE, TONE, TONE_CC, TONE_NEUTRAL,
    VIBRATO_DELAY, VIBRATO_DEPTH, VIBRATO_RATE,
};

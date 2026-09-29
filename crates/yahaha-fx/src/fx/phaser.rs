//! A stereo phaser (the mixer rework): an allpass chain swept by an LFO, with feedback.
//! The insert kind `InsertKind::Phaser` and the send kind `SendKind::Phaser` both play it.
//! Allocation-free: its state is fixed-size arrays.

/// A phaser's state (audio thread).
pub struct Phaser {
    #[allow(dead_code)]
    rate: f32,
}

impl Phaser {
    /// A phaser for `rate` Hz.
    pub fn new(rate: f32) -> Phaser {
        Phaser { rate }
    }

    /// Clear its state (the allpass memories, the feedback, the LFO phase).
    pub fn reset(&mut self) {}

    /// Its settings, once per buffer: `depth` 0-127, `rate_centihz` in hundredths of a
    /// hertz (5-500), `feedback_pct` 0-90 (the `kinds` PHASER specs).
    pub fn set(&mut self, _depth: u16, _rate_centihz: u16, _feedback_pct: u16) {}

    /// One stereo frame through it: the phased signal (the dry and the allpass chain
    /// summed, the notches). Stub: unchanged.
    #[inline]
    pub fn tick(&mut self, l: f32, r: f32) -> (f32, f32) {
        (l, r)
    }
}

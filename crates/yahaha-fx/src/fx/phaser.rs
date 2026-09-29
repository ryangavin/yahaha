//! A stereo phaser (the mixer rework): an allpass chain swept by an LFO, with feedback.
//! The insert kind `InsertKind::Phaser` and the send kind `SendKind::Phaser` both play it.
//!
//! Each side runs [`STAGES`] first-order allpass stages whose corner sweeps, on an
//! exponential (musical) scale, from [`LOW_HZ`] up [`OCTAVES`] octaves and back, with a
//! raised-cosine LFO; the right side's LFO runs a quarter cycle (90 degrees) ahead of the
//! left's, for width. The chain's output feeds back into its input. Summed with the dry
//! signal it cancels at the frequencies where the chain turns the phase by 180 degrees:
//! the moving notches.
//!
//! Allocation-free and panic-free: its state is fixed-size arrays, and the coefficients
//! are recomputed every [`BLOCK`] frames. Feedback stays below 1 (at most 0.9), so the
//! loop is stable and the output finite for every setting.

use std::f32::consts::{PI, TAU};

/// Allpass stages per side (3 notches).
pub const STAGES: usize = 6;
/// The sweep's lowest corner (Hz).
pub const LOW_HZ: f32 = 200.0;
/// The sweep's span, in octaves above [`LOW_HZ`] (200 Hz .. 3.2 kHz).
pub const OCTAVES: f32 = 4.0;
/// Frames between coefficient updates.
const BLOCK: u32 = 8;
/// How long depth and feedback take to glide across their whole range (s), so a
/// setting change never clicks.
const GLIDE_S: f32 = 0.02;

/// `v` moved toward `to` by at most `step`, landing on it exactly.
#[inline]
fn toward(v: f32, to: f32, step: f32) -> f32 {
    if (to - v).abs() <= step { to } else { v + step.copysign(to - v) }
}

/// A phaser's state (audio thread).
pub struct Phaser {
    rate: f32,
    /// The LFO's phase (0..1), the left side's; the right runs a quarter ahead.
    phase: f32,
    /// The LFO's step per frame.
    step: f32,
    /// 0 = dry .. 1 = the full notches (`depth` / 127), and where `set` asked it to go.
    depth: f32,
    depth_to: f32,
    /// The chain's output fed back into its input (0-0.9), and where it is going.
    feedback: f32,
    feedback_to: f32,
    /// The most depth and feedback move per frame (a whole range in [`GLIDE_S`]).
    glide: f32,
    /// Each side's stage memories.
    z: [[f32; STAGES]; 2],
    /// Each side's last chain output (the feedback).
    last: [f32; 2],
    /// Each side's allpass coefficient, and the frames until it is next computed.
    coef: [f32; 2],
    until: u32,
}

impl Phaser {
    /// A phaser for `rate` Hz, at its default settings (depth 64, 0.50 Hz, 40%).
    pub fn new(rate: f32) -> Phaser {
        let rate = rate.max(1.0);
        let mut p = Phaser {
            rate,
            phase: 0.0,
            step: 0.0,
            depth: 0.0,
            depth_to: 0.0,
            feedback: 0.0,
            feedback_to: 0.0,
            glide: 1.0 / (GLIDE_S * rate),
            z: [[0.0; STAGES]; 2],
            last: [0.0; 2],
            coef: [0.0; 2],
            until: 0,
        };
        p.set(64, 50, 40);
        p.reset();
        p
    }

    /// Clear its state (the allpass memories, the feedback, the LFO phase); depth and
    /// feedback jump to their settings instead of gliding. Its settings stay.
    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.z = [[0.0; STAGES]; 2];
        self.last = [0.0; 2];
        self.until = 0;
        self.depth = self.depth_to;
        self.feedback = self.feedback_to;
    }

    /// Its settings, once per buffer: `depth` 0-127, `rate_centihz` in hundredths of a
    /// hertz (5-500), `feedback_pct` 0-90 (the `kinds` PHASER specs). Out-of-range values
    /// are clamped. Depth and feedback glide to their new values over up to 20 ms.
    pub fn set(&mut self, depth: u16, rate_centihz: u16, feedback_pct: u16) {
        self.depth_to = depth.min(127) as f32 / 127.0;
        self.step = rate_centihz.clamp(5, 500) as f32 / 100.0 / self.rate;
        self.feedback_to = feedback_pct.min(90) as f32 / 100.0;
    }

    /// The allpass coefficient for LFO phase `phase` (0..1).
    fn coef_at(&self, phase: f32) -> f32 {
        let lfo = 0.5 * (1.0 - (TAU * phase).cos());
        let hz = (LOW_HZ * (OCTAVES * lfo).exp2()).min(self.rate * 0.45);
        let t = (PI * hz / self.rate).tan();
        (t - 1.0) / (t + 1.0)
    }

    /// One stereo frame through it: the phased signal (the dry and the allpass chain
    /// summed, the notches), at about the input's level.
    #[inline]
    pub fn tick(&mut self, l: f32, r: f32) -> (f32, f32) {
        if self.until == 0 {
            self.coef = [self.coef_at(self.phase), self.coef_at((self.phase + 0.25).fract())];
            self.until = BLOCK;
        }
        self.until -= 1;
        self.phase = (self.phase + self.step).fract();
        self.depth = toward(self.depth, self.depth_to, self.glide);
        self.feedback = toward(self.feedback, self.feedback_to, self.glide);
        let mut out = [l, r];
        for ((o, z), (last, &a)) in out.iter_mut().zip(&mut self.z).zip(self.last.iter_mut().zip(&self.coef)) {
            let dry = *o;
            let mut v = dry + self.feedback * *last;
            for s in z.iter_mut() {
                let y = a * v + *s;
                *s = v - a * y;
                v = y;
            }
            *last = v;
            *o = (dry + self.depth * v) / (1.0 + self.depth);
        }
        (out[0], out[1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 48_000.0;

    fn sine(hz: f32, n: usize) -> Vec<f32> {
        (0..n).map(|i| (TAU * hz * i as f32 / RATE).sin() * 0.5).collect()
    }

    /// The RMS of each 10 ms window of the left side.
    fn windows(p: &mut Phaser, x: &[f32]) -> Vec<f32> {
        let y: Vec<f32> = x.iter().map(|&v| p.tick(v, v).0).collect();
        y.chunks(480).map(|c| (c.iter().map(|v| v * v).sum::<f32>() / c.len() as f32).sqrt()).collect()
    }

    /// A fixed sine's level rises and falls as the notches sweep past it.
    #[test]
    fn the_notches_sweep() {
        let mut p = Phaser::new(RATE);
        p.set(127, 200, 0);
        let w = windows(&mut p, &sine(800.0, 24_000));
        let (lo, hi) = w.iter().fold((f32::MAX, 0f32), |(a, b), &v| (a.min(v), b.max(v)));
        assert!(hi > 3.0 * lo, "{lo} .. {hi}");
        // At depth 0 it is the dry signal.
        let mut p = Phaser::new(RATE);
        p.set(0, 200, 60);
        p.reset();
        let x = sine(800.0, 4800);
        assert!(x.iter().all(|&v| p.tick(v, v) == (v, v)));
    }

    /// The two sides sweep a quarter cycle apart.
    #[test]
    fn the_sides_differ() {
        let mut p = Phaser::new(RATE);
        let x = sine(700.0, 24_000);
        let diff = x.iter().map(|&v| p.tick(v, v)).map(|(a, b)| (a - b).abs()).fold(0f32, f32::max);
        assert!(diff > 0.05, "{diff}");
    }

    /// Feedback deepens the notches' peaks; every setting stays finite and bounded.
    #[test]
    fn feedback_is_stable() {
        for (d, r, f) in [(127, 5, 90), (127, 500, 90), (64, 50, 40), (127, 500, 0), (200, 9000, 900)] {
            let mut p = Phaser::new(RATE);
            p.set(d, r, f);
            let mut seed = 7u32;
            for _ in 0..24_000 {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let v = (seed >> 8) as f32 / (1u32 << 24) as f32 - 0.5;
                let (a, b) = p.tick(v, -v);
                assert!(a.is_finite() && b.is_finite() && a.abs() < 20.0 && b.abs() < 20.0, "{d} {r} {f}: {a} {b}");
            }
        }
        let peak = |f: u16| {
            let mut p = Phaser::new(RATE);
            p.set(127, 100, f);
            windows(&mut p, &sine(500.0, 48_000)).into_iter().fold(0f32, f32::max)
        };
        assert!(peak(90) > 1.2 * peak(0), "{} vs {}", peak(90), peak(0));
    }

    /// A depth or feedback change glides: no step in the output much steeper than the
    /// input's own.
    #[test]
    fn a_setting_change_does_not_click() {
        let mut p = Phaser::new(RATE);
        p.set(0, 100, 0);
        p.reset();
        let x = sine(110.0, 9600);
        let mut y = Vec::with_capacity(x.len());
        for (i, c) in x.chunks(64).enumerate() {
            if i == 75 {
                p.set(127, 100, 90);
            }
            y.extend(c.iter().map(|&v| p.tick(v, v).0));
        }
        let steepest = |v: &[f32]| v.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0f32, f32::max);
        assert!(steepest(&y) < 2.0 * steepest(&x), "{} vs {}", steepest(&y), steepest(&x));
    }

    /// Reset clears its memories: the same input plays the same output again.
    #[test]
    fn reset_restarts_it() {
        let mut p = Phaser::new(RATE);
        let x = sine(300.0, 2000);
        let a: Vec<_> = x.iter().map(|&v| p.tick(v, v)).collect();
        p.reset();
        let b: Vec<_> = x.iter().map(|&v| p.tick(v, v)).collect();
        assert_eq!(a, b);
    }
}

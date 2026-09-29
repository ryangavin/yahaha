//! The Chorus block: copies of the input read from a delay line whose length drifts, so
//! they are slightly detuned against the dry sound.
//!
//! The input is summed to mono and each side reads its own two taps, their LFOs a
//! quarter turn apart between the sides, which spreads the sound across the stereo field.
//! The types (Genos System Chorus names): Chorus (two voices, medium depth), Celeste
//! (slower, subtler: a gentle detune), Flanger (short delay with feedback: the sweeping
//! comb).
//!
//! Its parameters (#236, `super::Param`): the rate (the first LFO's speed; the second
//! keeps its type's ratio to it) and the depth (how far the taps swing, ms). A type
//! starts them at its own values. A rate change keeps each LFO's phase and only changes
//! its speed; a depth change glides; neither clicks.

use super::line::{Lfo, Line};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ChorusType {
    Chorus = 0,
    Celeste = 1,
    Flanger = 2,
}

impl ChorusType {
    pub const ALL: [ChorusType; 3] = [ChorusType::Chorus, ChorusType::Celeste, ChorusType::Flanger];

    pub fn from_u8(v: u8) -> ChorusType {
        Self::ALL.get(v as usize).copied().unwrap_or(ChorusType::Chorus)
    }

    /// The type's own parameters, in their units (`super::Param`): rate (0.01 Hz), depth
    /// (0.1 ms).
    pub fn defaults(self) -> [u16; 2] {
        let p = self.params();
        [(p.hz[0] * 100.0).round() as u16, (p.depth_ms * 10.0).round() as u16]
    }

    fn params(self) -> Params {
        match self {
            ChorusType::Chorus => Params { base_ms: [11.0, 16.5], depth_ms: 2.2, hz: [0.55, 0.83], feedback: 0.0 },
            ChorusType::Celeste => Params { base_ms: [7.5, 10.0], depth_ms: 0.9, hz: [0.29, 0.41], feedback: 0.0 },
            ChorusType::Flanger => Params { base_ms: [2.2, 2.9], depth_ms: 1.8, hz: [0.21, 0.21], feedback: 0.55 },
        }
    }
}

#[derive(Clone, Copy)]
struct Params {
    /// The two taps' centre delays.
    base_ms: [f32; 2],
    /// The Depth parameter's default.
    depth_ms: f32,
    /// The two LFOs' rates: the first is the Rate parameter's default.
    hz: [f32; 2],
    feedback: f32,
}

const MAX_MS: f32 = 25.0;
/// How fast a depth change glides (time constant, s).
const GLIDE_S: f32 = 0.03;

/// The Chorus block.
pub struct Chorus {
    rate: f32,
    kind: Option<ChorusType>,
    /// The rate (Hz) and depth (ms) last applied.
    applied: (f32, f32),
    line: Line,
    /// Each tap's centre delay (samples).
    base: [f32; 2],
    /// The second LFO's rate over the first's (the type's).
    ratio: f32,
    /// The depth now and where it is going (samples).
    depth: f32,
    depth_to: f32,
    glide: f32,
    lfo: [Lfo; 2],
    feedback: f32,
    last: f32,
}

impl Chorus {
    pub fn new(rate: f32) -> Chorus {
        let mut c = Chorus {
            rate,
            kind: None,
            applied: (0.0, 0.0),
            line: Line::new((MAX_MS * rate / 1000.0) as usize + 2),
            base: [0.0; 2],
            ratio: 1.0,
            depth: 0.0,
            depth_to: 0.0,
            glide: 1.0 - (-1.0 / (GLIDE_S * rate)).exp(),
            lfo: [Lfo::new(0.5, rate, 0.0); 2],
            feedback: 0.0,
            last: 0.0,
        };
        c.set_type(ChorusType::Chorus);
        c
    }

    /// The type, and its rate (Hz) and depth (ms). A new type moves the taps (the line
    /// keeps playing) and takes these at once; a change on the same type glides.
    pub fn set(&mut self, t: ChorusType, rate_hz: f32, depth_ms: f32) {
        let new_type = self.kind != Some(t);
        if new_type {
            self.set_type(t);
        }
        let p = (rate_hz.clamp(0.01, 10.0), depth_ms.clamp(0.0, 10.0));
        if new_type || p != self.applied {
            self.applied = p;
            for (i, l) in self.lfo.iter_mut().enumerate() {
                l.set_hz(if i == 0 { p.0 } else { p.0 * self.ratio }, self.rate);
            }
            // The taps never read ahead of the line's write.
            self.depth_to = (p.1 * self.rate / 1000.0).min(self.base[0] - 1.0).max(0.0);
            if new_type {
                self.depth = self.depth_to;
            }
        }
    }

    /// Change the type with its own rate and depth (the line keeps playing; the taps
    /// move).
    pub fn set_type(&mut self, t: ChorusType) {
        if self.kind == Some(t) {
            return;
        }
        self.kind = Some(t);
        let p = t.params();
        let ms = self.rate / 1000.0;
        for i in 0..2 {
            self.base[i] = p.base_ms[i] * ms;
            self.lfo[i] = Lfo::new(p.hz[i], self.rate, i as f32 * 2.1);
        }
        self.ratio = p.hz[1] / p.hz[0];
        self.feedback = p.feedback;
        self.last = 0.0;
        self.applied = (p.hz[0], p.depth_ms);
        self.depth_to = (p.depth_ms * ms).min(self.base[0] - 1.0).max(0.0);
        self.depth = self.depth_to;
    }

    /// One stereo frame in, one out (wet only).
    #[inline]
    pub fn tick(&mut self, l: f32, r: f32) -> (f32, f32) {
        self.depth += (self.depth_to - self.depth) * self.glide;
        let x = (l + r) * 0.5;
        self.line.write(x + self.last * self.feedback);
        let (s0, c0) = self.lfo[0].tick();
        let (s1, c1) = self.lfo[1].tick();
        let ([b0, b1], d) = (self.base, self.depth);
        // Left: sines; right: cosines (a quarter turn later).
        let out_l = (self.line.read_frac(b0 + d * s0) + self.line.read_frac(b1 + d * s1)) * 0.5;
        let out_r = (self.line.read_frac(b0 + d * c0) + self.line.read_frac(b1 + d * c1)) * 0.5;
        self.last = (out_l + out_r) * 0.5;
        (out_l, out_r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tone through the chorus at this rate (Hz) and depth (ms).
    fn render(rate_hz: f32, depth_ms: f32) -> Vec<f32> {
        let mut c = Chorus::new(48_000.0);
        c.set(ChorusType::Chorus, rate_hz, depth_ms);
        (0..96_000)
            .map(|i| {
                let x = (i as f32 * 440.0 * std::f32::consts::TAU / 48_000.0).sin() * 0.3;
                c.tick(x, x).0
            })
            .collect()
    }

    /// How far the chorused sound moves away from the unmoving (depth 0) one over frames
    /// `a..b`: the detune's size.
    fn moved(rate_hz: f32, depth_ms: f32, a: usize, b: usize) -> f32 {
        let still = render(rate_hz, 0.0);
        let x = render(rate_hz, depth_ms);
        (x[a..b].iter().zip(&still[a..b]).map(|(p, q)| (p - q) * (p - q)).sum::<f32>() / (b - a) as f32).sqrt()
    }

    /// #236: the depth sets how far the taps swing (none at 0).
    #[test]
    fn the_depth_sets_the_swing() {
        let (none, some, deep) = (moved(0.55, 0.0, 0, 96_000), moved(0.55, 0.05, 0, 96_000), moved(0.55, 0.2, 0, 96_000));
        assert!(none == 0.0 && some > 0.001 && deep > some * 2.5, "{none} < {some} < {deep}");
    }

    /// #236: the rate sets both LFOs' speed (the second keeps the type's ratio), and a
    /// change keeps where they are in their turn.
    #[test]
    fn the_rate_sets_the_lfos_and_keeps_their_phase() {
        let mut c = Chorus::new(48_000.0);
        for _ in 0..1000 {
            c.tick(0.0, 0.0);
        }
        let before = c.lfo.map(|l| l.phase());
        c.set(ChorusType::Chorus, 2.0, 2.2);
        let hz = c.lfo.map(|l| l.hz(48_000.0));
        assert!((hz[0] - 2.0).abs() < 1e-3 && (hz[1] - 2.0 * 0.83 / 0.55).abs() < 1e-3, "{hz:?}");
        assert_eq!(c.lfo.map(|l| l.phase()), before, "no jump");
    }

    /// Turning the rate or the depth while it plays: no jump in the output.
    #[test]
    fn a_change_does_not_click() {
        let mut c = Chorus::new(48_000.0);
        let tone = |i: usize| (i as f32 * 220.0 * std::f32::consts::TAU / 48_000.0).sin() * 0.3;
        let (mut prev, mut worst) = (0f32, 0f32);
        for i in 0..96_000 {
            match i {
                24_000 => c.set(ChorusType::Chorus, 4.0, 5.0),
                48_000 => c.set(ChorusType::Chorus, 0.1, 0.2),
                72_000 => c.set(ChorusType::Chorus, 2.0, 3.0),
                _ => {}
            }
            let (l, _) = c.tick(tone(i), tone(i));
            if i > 1000 {
                worst = worst.max((l - prev).abs());
            }
            prev = l;
        }
        // A 220 Hz sine at 0.3 moves at most ~0.009 a sample; a jump would be far more.
        assert!(worst < 0.02, "{worst}");
    }
}

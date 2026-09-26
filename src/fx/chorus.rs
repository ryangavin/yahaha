//! The Chorus block: copies of the input read from a delay line whose length drifts, so
//! they are slightly detuned against the dry sound.
//!
//! The input is summed to mono and each side reads its own two taps, their LFOs a
//! quarter turn apart between the sides, which spreads the sound across the stereo field.
//! The types (Genos System Chorus names): Chorus (two voices, medium depth), Celeste
//! (slower, subtler: a gentle detune), Flanger (short delay with feedback: the sweeping
//! comb).

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
    depth_ms: f32,
    hz: [f32; 2],
    feedback: f32,
}

const MAX_MS: f32 = 25.0;

/// The Chorus block.
pub struct Chorus {
    rate: f32,
    kind: Option<ChorusType>,
    line: Line,
    /// Per tap: (base, depth) in samples.
    taps: [(f32, f32); 2],
    lfo: [Lfo; 2],
    feedback: f32,
    last: f32,
}

impl Chorus {
    pub fn new(rate: f32) -> Chorus {
        let mut c = Chorus {
            rate,
            kind: None,
            line: Line::new((MAX_MS * rate / 1000.0) as usize + 2),
            taps: [(0.0, 0.0); 2],
            lfo: [Lfo::new(0.5, rate, 0.0); 2],
            feedback: 0.0,
            last: 0.0,
        };
        c.set_type(ChorusType::Chorus);
        c
    }

    /// Change the type (the line keeps playing; the taps move).
    pub fn set_type(&mut self, t: ChorusType) {
        if self.kind == Some(t) {
            return;
        }
        self.kind = Some(t);
        let p = t.params();
        let ms = self.rate / 1000.0;
        for i in 0..2 {
            self.taps[i] = (p.base_ms[i] * ms, p.depth_ms * ms);
            self.lfo[i] = Lfo::new(p.hz[i], self.rate, i as f32 * 2.1);
        }
        self.feedback = p.feedback;
        self.last = 0.0;
    }

    /// One stereo frame in, one out (wet only).
    #[inline]
    pub fn tick(&mut self, l: f32, r: f32) -> (f32, f32) {
        let x = (l + r) * 0.5;
        self.line.write(x + self.last * self.feedback);
        let (s0, c0) = self.lfo[0].tick();
        let (s1, c1) = self.lfo[1].tick();
        let [(b0, d0), (b1, d1)] = self.taps;
        // Left: sines; right: cosines (a quarter turn later).
        let out_l = (self.line.read_frac(b0 + d0 * s0) + self.line.read_frac(b1 + d1 * s1)) * 0.5;
        let out_r = (self.line.read_frac(b0 + d0 * c0) + self.line.read_frac(b1 + d1 * c1)) * 0.5;
        self.last = (out_l + out_r) * 0.5;
        (out_l, out_r)
    }
}

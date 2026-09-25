//! The Reverb block: an 8-line feedback delay network (FDN).
//!
//! Input: a pre-delay, a bandwidth lowpass and four allpass diffusers per side (the
//! input section of Dattorro's plate), so the tail starts dense. The tank: eight delay
//! lines of mutually prime lengths, mixed through an 8x8 Hadamard matrix (every line
//! feeds every other, energy-preserving), each with a one-pole lowpass (high frequencies
//! die sooner, like air and walls) and a feedback gain that sets its RT60. Four lines'
//! lengths drift slowly (a fraction of a millisecond), which breaks up the metallic ring
//! a static network has. The left output is taken from the even lines, the right from
//! the odd ones, so the two sides are decorrelated.
//!
//! The types (Genos System Reverb names): Hall (large, long, soft top), Room (small,
//! short, early), Stage (between, brighter), Plate (dense, bright, no pre-delay).

use super::line::{Allpass, Lfo, Line, OnePole};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ReverbType {
    Hall = 0,
    Room = 1,
    Stage = 2,
    Plate = 3,
}

impl ReverbType {
    pub const ALL: [ReverbType; 4] = [ReverbType::Hall, ReverbType::Room, ReverbType::Stage, ReverbType::Plate];

    pub fn from_u8(v: u8) -> ReverbType {
        Self::ALL.get(v as usize).copied().unwrap_or(ReverbType::Hall)
    }

    /// The decay time (s, -60 dB at mid frequencies).
    pub fn rt60(self) -> f32 {
        self.params().rt60
    }

    fn params(self) -> Params {
        match self {
            ReverbType::Hall => Params { size: 1.0, rt60: 2.4, predelay_ms: 22.0, bandwidth_hz: 7000.0, damping_hz: 4500.0, diffusion: 0.70, depth: 6.0 },
            ReverbType::Room => Params { size: 0.42, rt60: 0.85, predelay_ms: 4.0, bandwidth_hz: 9000.0, damping_hz: 6000.0, diffusion: 0.62, depth: 3.0 },
            ReverbType::Stage => Params { size: 0.75, rt60: 1.7, predelay_ms: 12.0, bandwidth_hz: 9500.0, damping_hz: 6500.0, diffusion: 0.68, depth: 5.0 },
            ReverbType::Plate => Params { size: 0.55, rt60: 1.8, predelay_ms: 0.5, bandwidth_hz: 12000.0, damping_hz: 9000.0, diffusion: 0.76, depth: 4.0 },
        }
    }
}

#[derive(Clone, Copy)]
struct Params {
    /// Scales the tank's line lengths (1.0 = the longest, the Hall).
    size: f32,
    rt60: f32,
    predelay_ms: f32,
    /// The input lowpass.
    bandwidth_hz: f32,
    /// The tank's lowpass.
    damping_hz: f32,
    /// The input diffusers' coefficient.
    diffusion: f32,
    /// How far the modulated lines drift (samples at 48 kHz).
    depth: f32,
}

/// The tank's line lengths at size 1.0, ms (mutually prime at 48 kHz).
const TANK_MS: [f32; 8] = [31.71, 37.23, 41.13, 45.91, 53.77, 59.83, 67.19, 79.33];
/// The input diffusers' lengths, ms, per side (Dattorro's, a little apart left and right).
const DIFFUSER_MS: [[f32; 4]; 2] = [[4.771, 3.595, 12.73, 9.307], [4.987, 3.821, 12.13, 8.863]];
const MAX_PREDELAY_MS: f32 = 50.0;

/// The Reverb block.
pub struct Reverb {
    rate: f32,
    kind: Option<ReverbType>,
    pre: [Line; 2],
    predelay: usize,
    bandwidth: [OnePole; 2],
    diffusers: [[Allpass; 4]; 2],
    tank: [Line; 8],
    len: [f32; 8],
    gain: [f32; 8],
    damping: [OnePole; 8],
    lfo: Lfo,
    depth: f32,
    out_gain: f32,
}

impl Reverb {
    pub fn new(rate: f32) -> Reverb {
        let ms = |m: f32| (m * rate / 1000.0).round() as usize;
        let depth_max = 8.0 * rate / 48_000.0;
        let diffuser = |side: usize, i: usize| Allpass::new(ms(DIFFUSER_MS[side][i]), 0.7);
        let mut r = Reverb {
            rate,
            kind: None,
            pre: [Line::new(ms(MAX_PREDELAY_MS)), Line::new(ms(MAX_PREDELAY_MS))],
            predelay: 0,
            bandwidth: [OnePole::new(8000.0, rate); 2],
            diffusers: std::array::from_fn(|side| std::array::from_fn(|i| diffuser(side, i))),
            tank: std::array::from_fn(|i| Line::new(ms(TANK_MS[i]) + depth_max as usize + 2)),
            len: [0.0; 8],
            gain: [0.0; 8],
            damping: [OnePole::new(5000.0, rate); 8],
            lfo: Lfo::new(0.37, rate, 0.0),
            depth: 0.0,
            out_gain: 0.0,
        };
        r.set_type(ReverbType::Hall);
        r
    }

    /// Change the type: the tank starts again from silence (a type change is a new room).
    /// Nothing to do if it already has this type.
    pub fn set_type(&mut self, t: ReverbType) {
        if self.kind == Some(t) {
            return;
        }
        self.kind = Some(t);
        let p = t.params();
        let rate = self.rate;
        self.predelay = ((p.predelay_ms * rate / 1000.0) as usize).min(self.pre[0].capacity());
        for b in &mut self.bandwidth {
            b.set(p.bandwidth_hz, rate);
        }
        self.depth = p.depth * rate / 48_000.0;
        let mut energy = 0.0;
        #[allow(clippy::needless_range_loop)] // five arrays by line index
        for i in 0..8 {
            let len = (TANK_MS[i] * p.size * rate / 1000.0).max(8.0);
            self.len[i] = len.min(self.tank[i].capacity() as f32 - self.depth - 2.0);
            // Per pass round the loop: -60 dB over rt60 seconds.
            self.gain[i] = 10f32.powf(-3.0 * self.len[i] / (p.rt60 * rate));
            self.damping[i].set(p.damping_hz, rate);
            energy += 1.0 / (1.0 - self.gain[i] * self.gain[i]);
        }
        // Normalise the tail's level: a longer, bigger room builds up more energy.
        self.out_gain = 1.4 / (energy / 8.0).sqrt();
        for d in self.diffusers.iter_mut().flatten() {
            d.g = p.diffusion;
        }
        self.clear();
    }

    fn clear(&mut self) {
        for l in self.pre.iter_mut().chain(self.tank.iter_mut()) {
            l.clear();
        }
        for d in self.diffusers.iter_mut().flatten() {
            d.clear();
        }
        for f in self.bandwidth.iter_mut().chain(self.damping.iter_mut()) {
            f.clear();
        }
    }

    /// One stereo frame in, one out (wet only).
    #[inline]
    pub fn tick(&mut self, l: f32, r: f32) -> (f32, f32) {
        // Input: pre-delay, bandwidth, diffusion.
        let mut x = [l, r];
        #[allow(clippy::needless_range_loop)] // four arrays by side
        for s in 0..2 {
            self.pre[s].write(x[s]);
            let mut v = self.bandwidth[s].tick(self.pre[s].read(self.predelay));
            for d in &mut self.diffusers[s] {
                v = d.tick(v);
            }
            x[s] = v;
        }
        // The tank's outputs, four of them drifting.
        let (sn, cs) = self.lfo.tick();
        let drift = [sn, cs, -sn, -cs];
        let mut y = [0f32; 8];
        for i in 0..8 {
            let d = if i < 4 { self.len[i] + self.depth * (1.0 + drift[i]) } else { self.len[i] };
            y[i] = self.damping[i].tick(self.tank[i].read_frac(d)) * self.gain[i];
        }
        let out_l = (y[0] - y[2] + y[4] - y[6]) * self.out_gain;
        let out_r = (y[1] - y[3] + y[5] - y[7]) * self.out_gain;
        // Mix every line into every other (Hadamard, energy-preserving) and feed the
        // input in: the left side to the even lines, the right to the odd.
        hadamard8(&mut y);
        for (i, v) in y.iter_mut().enumerate() {
            let input = if i % 2 == 0 { x[0] } else { x[1] };
            let sign = if i & 2 == 0 { 1.0 } else { -1.0 };
            self.tank[i].write(*v + input * sign * 0.5);
        }
        (out_l, out_r)
    }
}

/// The 8x8 Hadamard transform, scaled to be orthonormal (1/sqrt(8)).
#[inline]
fn hadamard8(v: &mut [f32; 8]) {
    let mut h = 1;
    while h < 8 {
        for i in (0..8).step_by(h * 2) {
            for j in i..i + h {
                let (a, b) = (v[j], v[j + h]);
                v[j] = a + b;
                v[j + h] = a - b;
            }
        }
        h *= 2;
    }
    const S: f32 = 0.353_553_4;
    for x in v.iter_mut() {
        *x *= S;
    }
}

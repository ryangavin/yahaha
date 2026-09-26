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
//! short, early), Stage (between, brighter), Plate (dense, bright, no pre-delay). A type
//! sets the room's size and character; its decay time, pre-delay and tone are parameters
//! (#236, `super::Param`), which start at the type's own values. A parameter change glides
//! (the feedback gains and the output level over about 50 ms; the pre-delay crossfades
//! to its new tap in 30 ms), so turning
//! one while the tail rings never clicks.

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

    /// The type's own decay time (s, -60 dB at mid frequencies): its Time parameter's
    /// default.
    pub fn rt60(self) -> f32 {
        self.params().rt60
    }

    /// The type's own parameters, in their units (`super::Param`): decay time (tenths of
    /// a second), pre-delay (ms), tone (100 Hz).
    pub fn defaults(self) -> [u16; 3] {
        let p = self.params();
        [(p.rt60 * 10.0).round() as u16, p.predelay_ms.round() as u16, (p.damping_hz / 100.0).round() as u16]
    }

    fn params(self) -> Params {
        match self {
            ReverbType::Hall => Params { size: 1.0, rt60: 2.4, predelay_ms: 22.0, bandwidth_hz: 7000.0, damping_hz: 4500.0, diffusion: 0.70, depth: 6.0 },
            ReverbType::Room => Params { size: 0.42, rt60: 0.9, predelay_ms: 4.0, bandwidth_hz: 9000.0, damping_hz: 6000.0, diffusion: 0.62, depth: 3.0 },
            ReverbType::Stage => Params { size: 0.75, rt60: 1.7, predelay_ms: 12.0, bandwidth_hz: 9500.0, damping_hz: 6500.0, diffusion: 0.68, depth: 5.0 },
            ReverbType::Plate => Params { size: 0.55, rt60: 1.8, predelay_ms: 1.0, bandwidth_hz: 12000.0, damping_hz: 9000.0, diffusion: 0.76, depth: 4.0 },
        }
    }
}

#[derive(Clone, Copy)]
struct Params {
    /// Scales the tank's line lengths (1.0 = the longest, the Hall).
    size: f32,
    /// The Time parameter's default.
    rt60: f32,
    /// The Pre-delay parameter's default.
    predelay_ms: f32,
    /// The input lowpass.
    bandwidth_hz: f32,
    /// The Tone parameter's default: the tank's lowpass.
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
/// The longest pre-delay (the Pre-delay parameter's top).
pub const MAX_PREDELAY_MS: f32 = 200.0;
/// How fast a parameter change glides (time constant, s).
const GLIDE_S: f32 = 0.05;
/// How long a pre-delay change crossfades (s).
const FADE_S: f32 = 0.03;

/// The Reverb block.
pub struct Reverb {
    rate: f32,
    kind: Option<ReverbType>,
    /// The decay time, pre-delay and tone last applied (s, ms, Hz).
    applied: (f32, f32, f32),
    pre: [Line; 2],
    /// The pre-delay tap (samples), and while it moves the new one it crossfades to
    /// (`fade` 0..1): a moving tap would bend the pitch, a jump would click.
    predelay: f32,
    predelay_to: f32,
    fade: f32,
    bandwidth: [OnePole; 2],
    diffusers: [[Allpass; 4]; 2],
    tank: [Line; 8],
    len: [f32; 8],
    /// Each line's feedback gain now and where it is going.
    gain: [f32; 8],
    gain_to: [f32; 8],
    damping: [OnePole; 8],
    lfo: Lfo,
    depth: f32,
    out_gain: f32,
    out_gain_to: f32,
    /// The per-sample glide step, and the samples left gliding (0: at the targets).
    glide: f32,
    gliding: u32,
    /// The pre-delay crossfade's step per sample.
    fade_step: f32,
}

impl Reverb {
    pub fn new(rate: f32) -> Reverb {
        let ms = |m: f32| (m * rate / 1000.0).round() as usize;
        let depth_max = 8.0 * rate / 48_000.0;
        let diffuser = |side: usize, i: usize| Allpass::new(ms(DIFFUSER_MS[side][i]), 0.7);
        let mut r = Reverb {
            rate,
            kind: None,
            applied: (0.0, 0.0, 0.0),
            pre: [Line::new(ms(MAX_PREDELAY_MS) + 2), Line::new(ms(MAX_PREDELAY_MS) + 2)],
            predelay: 0.0,
            predelay_to: 0.0,
            fade: 1.0,
            bandwidth: [OnePole::new(8000.0, rate); 2],
            diffusers: std::array::from_fn(|side| std::array::from_fn(|i| diffuser(side, i))),
            tank: std::array::from_fn(|i| Line::new(ms(TANK_MS[i]) + depth_max as usize + 2)),
            len: [0.0; 8],
            gain: [0.0; 8],
            gain_to: [0.0; 8],
            damping: [OnePole::new(5000.0, rate); 8],
            lfo: Lfo::new(0.37, rate, 0.0),
            depth: 0.0,
            out_gain: 0.0,
            out_gain_to: 0.0,
            glide: 1.0 - (-1.0 / (GLIDE_S * rate)).exp(),
            gliding: 0,
            fade_step: 1.0 / (FADE_S * rate),
        };
        r.set_type(ReverbType::Hall);
        r
    }

    /// The type, and its decay time (s), pre-delay (ms) and tone (Hz). A new type starts
    /// from silence (a type change is a new room) with these at once; a parameter change
    /// on the same type glides. Nothing to do if nothing changed.
    pub fn set(&mut self, t: ReverbType, rt60: f32, predelay_ms: f32, tone_hz: f32) {
        let new_type = self.kind != Some(t);
        if new_type {
            self.set_type(t);
        }
        let p = (rt60.clamp(0.1, 30.0), predelay_ms.clamp(0.0, MAX_PREDELAY_MS), tone_hz.clamp(200.0, 22_000.0));
        if new_type || p != self.applied {
            self.applied = p;
            self.retarget(new_type);
        }
    }

    /// Change the type with its own parameters: the tank starts again from silence.
    /// Nothing to do if it already has this type.
    pub fn set_type(&mut self, t: ReverbType) {
        if self.kind == Some(t) {
            return;
        }
        self.kind = Some(t);
        let p = t.params();
        let rate = self.rate;
        for b in &mut self.bandwidth {
            b.set(p.bandwidth_hz, rate);
        }
        self.depth = p.depth * rate / 48_000.0;
        #[allow(clippy::needless_range_loop)] // two arrays by line index
        for i in 0..8 {
            let len = (TANK_MS[i] * p.size * rate / 1000.0).max(8.0);
            self.len[i] = len.min(self.tank[i].capacity() as f32 - self.depth - 2.0);
        }
        for d in self.diffusers.iter_mut().flatten() {
            d.g = p.diffusion;
        }
        self.clear();
        self.applied = (p.rt60, p.predelay_ms, p.damping_hz);
        self.retarget(true);
    }

    /// Aim the gains, the output level, the pre-delay and the tone at `applied`: at once
    /// (`snap`) or gliding there.
    fn retarget(&mut self, snap: bool) {
        let (rt60, predelay_ms, tone_hz) = self.applied;
        let rate = self.rate;
        let to = (predelay_ms * rate / 1000.0).min(self.pre[0].capacity() as f32 - 2.0);
        if to != self.predelay_to {
            // Crossfade from wherever the output mostly is now.
            if self.fade >= 0.5 {
                self.predelay = self.predelay_to;
            }
            self.predelay_to = to;
            self.fade = 0.0;
        }
        let mut energy = 0.0;
        for i in 0..8 {
            // Per pass round the loop: -60 dB over rt60 seconds.
            let g = 10f32.powf(-3.0 * self.len[i] / (rt60 * rate));
            self.gain_to[i] = g;
            energy += 1.0 / (1.0 - g * g);
            // The tone: a one-pole's coefficient moves without a jump in its output.
            self.damping[i].set(tone_hz, rate);
        }
        // Normalise the tail's level: a longer, bigger room builds up more energy.
        self.out_gain_to = 1.4 / (energy / 8.0).sqrt();
        if snap {
            self.snap();
        } else {
            // Long enough to arrive: e^-10 of the step is left.
            self.gliding = (10.0 * GLIDE_S * rate) as u32;
        }
    }

    fn snap(&mut self) {
        self.gain = self.gain_to;
        self.out_gain = self.out_gain_to;
        self.predelay = self.predelay_to;
        self.fade = 1.0;
        self.gliding = 0;
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
        if self.gliding > 0 {
            self.gliding -= 1;
            let k = self.glide;
            for (g, to) in self.gain.iter_mut().zip(&self.gain_to) {
                *g += (to - *g) * k;
            }
            self.out_gain += (self.out_gain_to - self.out_gain) * k;
            if self.gliding == 0 {
                self.snap();
            }
        }
        // Input: pre-delay, bandwidth, diffusion.
        let fade = self.fade;
        if fade < 1.0 {
            self.fade = (fade + self.fade_step).min(1.0);
            if self.fade >= 1.0 {
                self.predelay = self.predelay_to;
            }
        }
        let mut x = [l, r];
        #[allow(clippy::needless_range_loop)] // four arrays by side
        for s in 0..2 {
            self.pre[s].write(x[s]);
            let mut pre = self.pre[s].read(self.predelay as usize);
            if fade < 1.0 {
                pre += (self.pre[s].read(self.predelay_to as usize) - pre) * fade;
            }
            let mut v = self.bandwidth[s].tick(pre);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    /// Noise in for a second, then silence: how long the tail takes to fall 30 dB below
    /// its level as the input stopped (s), measured in 20 ms windows.
    fn decay_30db(rt60: f32, predelay_ms: f32, tone_hz: f32) -> f32 {
        let rate = 48_000.0;
        let mut r = Reverb::new(rate);
        r.set(ReverbType::Hall, rt60, predelay_ms, tone_hz);
        let mut seed = 1u32;
        let mut noise = || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / (1u32 << 24) as f32 - 0.5
        };
        let total = 48_000 * 12;
        let mut out = Vec::with_capacity(total);
        for i in 0..total {
            let x = if i < 48_000 { noise() } else { 0.0 };
            out.push(r.tick(x, x).0);
        }
        let win = 960;
        let start = rms(&out[48_000 - win..48_000]);
        let k = (0..(total - 48_000) / win).find(|&k| rms(&out[48_000 + k * win..48_000 + (k + 1) * win]) < start * 10f32.powf(-1.5)).unwrap();
        k as f32 * win as f32 / rate
    }

    /// The Time parameter sets how long the tail rings: 30 dB down takes about half the
    /// RT60, and four times the time is about four times the tail.
    #[test]
    fn the_tail_follows_the_time() {
        let short = decay_30db(1.0, 0.0, 4500.0);
        let long = decay_30db(4.0, 0.0, 4500.0);
        let ratio = long / short;
        assert!((0.3..0.8).contains(&short), "1 s: 30 dB after {short} s");
        assert!((3.0..5.0).contains(&ratio), "4 s against 1 s: {long} vs {short}");
    }

    /// The Pre-delay parameter holds the reverb back: the first sound comes that much
    /// later than with none (the tank's own shortest path comes first either way).
    #[test]
    fn the_pre_delay_holds_the_reverb_back() {
        let first = |ms: f32| {
            let mut r = Reverb::new(48_000.0);
            r.set(ReverbType::Hall, 2.4, ms, 4500.0);
            // Let the change settle (it crossfades from the Hall's own 22 ms).
            for _ in 0..9600 {
                r.tick(0.0, 0.0);
            }
            (0..48_000usize).find(|&i| {
                let x = if i == 0 { 1.0 } else { 0.0 };
                r.tick(x, x).0.abs() > 1e-4
            })
        };
        let base = first(0.0).unwrap();
        for ms in [50usize, 150, 200] {
            let f = first(ms as f32).unwrap();
            assert!(f.abs_diff(base + ms * 48) <= 2, "{ms} ms: first sound at {f}, {base} with none");
        }
    }

    /// The Tone parameter darkens the tail: less of a high tone rings on.
    #[test]
    fn a_lower_tone_is_darker() {
        let tail = |tone: f32, hz: f32| {
            let mut r = Reverb::new(48_000.0);
            r.set(ReverbType::Hall, 2.4, 0.0, tone);
            let mut out = Vec::new();
            for i in 0..48_000 {
                let x = if i < 24_000 { (i as f32 * hz * std::f32::consts::TAU / 48_000.0).sin() * 0.3 } else { 0.0 };
                out.push(r.tick(x, x).0);
            }
            rms(&out[30_000..40_000])
        };
        let (bright, dark) = (tail(12_000.0, 5000.0), tail(1500.0, 5000.0));
        assert!(dark < bright * 0.3, "5 kHz rings on less with the tone down: {dark} vs {bright}");
        let (low_b, low_d) = (tail(12_000.0, 200.0), tail(1500.0, 200.0));
        assert!(low_d > low_b * 0.5, "and a low tone much the same: {low_d} vs {low_b}");
    }

    /// Turning a parameter while the reverb rings glides: the output moves no faster just
    /// after a change than once it has settled at the new setting (a click would be a
    /// jump right at the change).
    #[test]
    fn a_parameter_change_does_not_click() {
        let mut r = Reverb::new(48_000.0);
        r.set(ReverbType::Hall, 2.4, 22.0, 4500.0);
        let tone = |i: usize| (i as f32 * 220.0 * std::f32::consts::TAU / 48_000.0).sin() * 0.3;
        let changes = [(48_000, (8.0, 150.0, 2000.0)), (96_000, (0.5, 0.0, 12_000.0)), (144_000, (3.0, 60.0, 6000.0))];
        let mut out = Vec::new();
        for i in 0..192_000 {
            if let Some((_, (t, p, h))) = changes.iter().find(|c| c.0 == i) {
                r.set(ReverbType::Hall, *t, *p, *h);
            }
            out.push(r.tick(tone(i), tone(i)).0);
        }
        let steepest = |a: usize, b: usize| out[a..b].windows(2).map(|w| (w[1] - w[0]).abs()).fold(0f32, f32::max);
        for (c, p) in changes {
            let (at, settled) = (steepest(c, c + 960), steepest(c + 24_000, c + 48_000));
            assert!(at < settled * 1.5 + 1e-4, "{p:?}: {at} just after the change, {settled} settled");
        }
    }
}

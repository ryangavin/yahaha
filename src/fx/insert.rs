//! Insertion effects (#269): one effect on one part's own signal, before its sends and the
//! mix: a Style part's as the style's XG Insertion Effect SysEx asks for
//! (`xg::StyleInserts`), a keyboard part's from its own insert slot ([`PartInsert`], set
//! by the player, an OTS or a rack). Whatever plays the part runs it: the SoundFont rack on
//! the part's stem ([`ChannelInserts`]), the plugin rack on the plugin's output.
//!
//! The kinds are the ones the corpus styles use most (see `xg::insert_kind`):
//! - [`InsertKind::Distortion`]: the amp simulators, overdrives and distortions (British
//!   Combo, US Combo, Stereo Amp Sim, V Distortion...): a soft clipper between a low cut
//!   and a cabinet-like high cut, its drive from the type (clean, crunch or lead);
//! - [`InsertKind::Compressor`]: Uni Comp, Multi Band Comp, VCM Compressor;
//! - [`InsertKind::AutoWah`]: Auto Wah, Tempo Auto Wah, VCM Auto/Pedal Wah: a resonant
//!   low-pass swept by the part's own envelope;
//! - [`InsertKind::Tremolo`]: a volume tremolo at a 1/8 note of the style tempo;
//! - [`InsertKind::Rotary`]: a rotary speaker, slow or fast (`FxControl::rotary_fast`,
//!   gliding between them), horn and drum, with a little Doppler.
//!
//! The effect sees the part as if at full volume: `process` divides the part's level out
//! before it and puts it back after, so a fader or expression move doesn't change how hard
//! it drives (the Genos inserts before the part's volume). A kind change fades the part's
//! dry sound in and out over 20 ms, never clicking. Everything is allocated in
//! [`Insert::new`]; [`Insert::process`] never allocates.

use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;

/// An insertion effect yahaha plays, on the wire (the app API, a rack file): an
/// [`InsertKind`] other than None.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InsertEffect {
    #[default]
    Distortion,
    Compressor,
    AutoWah,
    Tremolo,
    Rotary,
}

impl InsertEffect {
    pub const fn kind(self) -> InsertKind {
        match self {
            InsertEffect::Distortion => InsertKind::Distortion,
            InsertEffect::Compressor => InsertKind::Compressor,
            InsertEffect::AutoWah => InsertKind::AutoWah,
            InsertEffect::Tremolo => InsertKind::Tremolo,
            InsertEffect::Rotary => InsertKind::Rotary,
        }
    }
}

impl From<InsertKind> for InsertEffect {
    fn from(k: InsertKind) -> InsertEffect {
        match k {
            InsertKind::Distortion | InsertKind::None => InsertEffect::Distortion,
            InsertKind::Compressor => InsertEffect::Compressor,
            InsertKind::AutoWah => InsertEffect::AutoWah,
            InsertKind::Tremolo => InsertEffect::Tremolo,
            InsertKind::Rotary => InsertEffect::Rotary,
        }
    }
}

/// A keyboard part's insert slot (Genos Mixer > Effect: Insertion Effect On/Off, Type and
/// Depth): its effect, whether it plays, and its amount (0-127, `InsertSettings::amount`).
/// Off, the part's audio is bit-identical to no insert at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PartInsert {
    pub effect: InsertEffect,
    pub on: bool,
    pub amount: u8,
}

impl Default for PartInsert {
    fn default() -> PartInsert {
        PartInsert::OFF
    }
}

impl PartInsert {
    /// Off, a distortion at the middle amount: what a part has until something sets it.
    pub const OFF: PartInsert = PartInsert { effect: InsertEffect::Distortion, on: false, amount: 64 };

    /// What plays: its effect's kind when on, else None.
    pub fn kind(&self) -> InsertKind {
        if self.on { self.effect.kind() } else { InsertKind::None }
    }

    /// Exactly `OFF` (a rack leaves such a slot out of its file).
    pub fn is_default(&self) -> bool {
        *self == PartInsert::OFF
    }

    /// As a `u32` for an atomic: effect, on, amount (`from_bits` reads it back).
    pub const fn to_bits(self) -> u32 {
        let amount = if self.amount > 127 { 127 } else { self.amount };
        self.effect.kind() as u32 | (self.on as u32) << 8 | (amount as u32) << 16
    }

    pub fn from_bits(v: u32) -> PartInsert {
        PartInsert { effect: InsertEffect::from(InsertKind::from_u8(v as u8)), on: (v >> 8) & 1 == 1, amount: ((v >> 16) as u8).min(127) }
    }
}

/// An insertion effect's kind. `as u8` on the control atomics (`FxControl::insert`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum InsertKind {
    #[default]
    None = 0,
    Distortion = 1,
    Compressor = 2,
    AutoWah = 3,
    Tremolo = 4,
    Rotary = 5,
}

impl InsertKind {
    pub const ALL: [InsertKind; 6] = [InsertKind::None, InsertKind::Distortion, InsertKind::Compressor, InsertKind::AutoWah, InsertKind::Tremolo, InsertKind::Rotary];

    pub fn from_u8(v: u8) -> InsertKind {
        InsertKind::ALL.get(v as usize).copied().unwrap_or_default()
    }

    pub fn name(self) -> &'static str {
        match self {
            InsertKind::None => "None",
            InsertKind::Distortion => "Distortion",
            InsertKind::Compressor => "Compressor",
            InsertKind::AutoWah => "Auto Wah",
            InsertKind::Tremolo => "Tremolo",
            InsertKind::Rotary => "Rotary",
        }
    }
}

/// How long a kind change fades (s).
const FADE_S: f32 = 0.02;
/// The rotary's Doppler line (s): the longest swing of its taps.
const ROTARY_LINE_S: f32 = 0.004;
/// The rotary's horn and drum rates (Hz), slow and fast (a Leslie 122's chorale and
/// tremolo speeds).
const HORN_HZ: [f32; 2] = [0.8, 6.7];
const DRUM_HZ: [f32; 2] = [0.67, 5.7];
/// How long the rotary takes to reach fast, and to come back to slow (s).
const ROTARY_UP_S: f32 = 1.0;
const ROTARY_DOWN_S: f32 = 2.0;

/// A one-pole coefficient for cutoff `hz` at `rate`.
fn one_pole(hz: f32, rate: f32) -> f32 {
    1.0 - (-TAU * hz / rate).exp()
}

/// A smoothing coefficient for time constant `s` at `rate`.
fn time_coef(s: f32, rate: f32) -> f32 {
    1.0 - (-1.0 / (s * rate)).exp()
}

/// A soft clipper: linear near 0, at most 1.
#[inline]
fn shape(x: f32) -> f32 {
    x / (1.0 + x.abs())
}

/// One Style part's insertion effect (audio thread).
pub struct Insert {
    rate: f32,
    /// The kind playing, and the one the control side asks for.
    kind: InsertKind,
    /// 0 = dry, 1 = the effect: fades on a kind change.
    mix: f32,
    fade: f32,
    // Filters, one state per side.
    lo: [f32; 2],
    hi: [f32; 2],
    svf: [[f32; 2]; 2],
    env: f32,
    /// LFO phases (0..1): tremolo / rotary horn, rotary drum.
    phase: [f32; 2],
    /// The rotary's Doppler lines, one per side, and their write position.
    line: [Vec<f32>; 2],
    pos: usize,
    /// The rotary's speed, 0 = slow .. 1 = fast, gliding toward `InsertSettings::fast`.
    spin: f32,
    /// The glide per sample: speeding up, slowing down.
    spin_up: f32,
    spin_down: f32,
}

/// What the control side asks of an insert, read once per buffer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InsertSettings {
    pub kind: InsertKind,
    /// 0-127: the distortion's drive, the compressor's squeeze, the wah's sensitivity, the
    /// tremolo's and rotary's depth.
    pub amount: u8,
    /// The style tempo (BPM): the tremolo's rate.
    pub bpm: f32,
    /// The rotary at its fast speed (the Leslie switch): it speeds up and slows down
    /// gradually, as a real rotor does.
    pub fast: bool,
}

impl Insert {
    /// An insert for `rate` Hz (allocates the rotary's lines: call it off the audio thread).
    pub fn new(rate: f32) -> Insert {
        let n = (ROTARY_LINE_S * rate) as usize + 4;
        Insert {
            rate,
            kind: InsertKind::None,
            mix: 0.0,
            fade: 1.0 / (FADE_S * rate),
            lo: [0.0; 2],
            hi: [0.0; 2],
            svf: [[0.0; 2]; 2],
            env: 0.0,
            phase: [0.0, 0.25],
            line: [vec![0.0; n], vec![0.0; n]],
            pos: 0,
            spin: 0.0,
            spin_up: 1.0 / (ROTARY_UP_S * rate),
            spin_down: 1.0 / (ROTARY_DOWN_S * rate),
        }
    }

    /// Whether it does anything: an effect in place, or fading out of one.
    pub fn active(&self, want: InsertKind) -> bool {
        self.kind != InsertKind::None || want != InsertKind::None
    }

    fn reset(&mut self) {
        self.lo = [0.0; 2];
        self.hi = [0.0; 2];
        self.svf = [[0.0; 2]; 2];
        self.env = 0.0;
        self.phase = [0.0, 0.25];
        for l in &mut self.line {
            l.fill(0.0);
        }
        self.pos = 0;
    }

    /// Run the part's signal (`left`/`right`, its channel's own mix) through the effect in
    /// place. `level` is the part's own gain in that mix (volume x expression x master; 0:
    /// the effect runs on it as is). RT-safe.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], level: f32, s: &InsertSettings) {
        let n = left.len().min(right.len());
        let g = if level > 1e-4 { level } else { 1.0 };
        let (inv, amount) = (1.0 / g, s.amount.min(127) as f32 / 127.0);
        let rate = self.rate;
        // Distortion: pre gain 1 (clean) .. 40 (lead); the output kept near the input's
        // loudness.
        let pre = 1.0 + 39.0 * amount * amount;
        let post = 1.0 / shape(pre).max(0.05) * 0.7;
        let lo_c = one_pole(90.0, rate);
        let hi_c = one_pole(5500.0 - 2500.0 * amount, rate);
        // Compressor: threshold -12 .. -30 dB, ratio 4, attack 3 ms, release 150 ms.
        let thr = 10f32.powf((-12.0 - 18.0 * amount) / 20.0);
        let (att, rel) = (time_coef(0.003, rate), time_coef(0.15, rate));
        let makeup = (1.0 / thr).powf(0.75 * 0.5);
        // Auto wah: envelope follower, a resonant low-pass from 350 Hz to 3 kHz.
        let (wa, wr) = (time_coef(0.004, rate), time_coef(0.09, rate));
        let sens = 2.0 + 10.0 * amount;
        // Tremolo: a 1/8 note; rotary: horn 0.8 Hz, drum 0.67 Hz (the slow speed).
        let trem_hz = (s.bpm.clamp(20.0, 400.0) / 60.0 * 2.0).min(12.0);
        let depth = 0.2 + 0.6 * amount;
        let line_len = self.line[0].len();
        for k in 0..n {
            // The kind changes only at dry: fade out, swap, fade in.
            if self.kind != s.kind {
                self.mix -= self.fade;
                if self.mix <= 0.0 {
                    self.mix = 0.0;
                    self.kind = s.kind;
                    self.reset();
                }
            } else if self.kind != InsertKind::None && self.mix < 1.0 {
                self.mix = (self.mix + self.fade).min(1.0);
            }
            let (dl, dr) = (left[k], right[k]);
            if self.kind == InsertKind::None || self.mix == 0.0 {
                continue;
            }
            let x = [dl * inv, dr * inv];
            let mut y = x;
            match self.kind {
                InsertKind::Distortion => {
                    for (c, v) in y.iter_mut().enumerate() {
                        self.lo[c] += lo_c * (*v - self.lo[c]);
                        let shaped = shape((*v - self.lo[c]) * pre) * post;
                        self.hi[c] += hi_c * (shaped - self.hi[c]);
                        *v = self.hi[c];
                    }
                }
                InsertKind::Compressor => {
                    let peak = x[0].abs().max(x[1].abs());
                    let c = if peak > self.env { att } else { rel };
                    self.env += c * (peak - self.env);
                    let gain = if self.env > thr { (thr / self.env).powf(0.75) } else { 1.0 };
                    for v in &mut y {
                        *v *= gain * makeup;
                    }
                }
                InsertKind::AutoWah => {
                    let peak = (x[0].abs() + x[1].abs()) * 0.5;
                    let c = if peak > self.env { wa } else { wr };
                    self.env += c * (peak - self.env);
                    let hz = 350.0 * (1.0 + sens * self.env).min(8.6);
                    let f = 2.0 * (std::f32::consts::PI * hz.min(rate * 0.2) / rate).sin();
                    let q = 0.25;
                    for (c, v) in y.iter_mut().enumerate() {
                        let [low, band] = &mut self.svf[c];
                        *low += f * *band;
                        let high = *v - *low - q * *band;
                        *band += f * high;
                        *v = *low * 0.8 + *band * 0.4;
                    }
                }
                InsertKind::Tremolo => {
                    self.phase[0] = (self.phase[0] + trem_hz / rate).fract();
                    let m = 1.0 - depth * 0.5 * (1.0 - (TAU * self.phase[0]).cos());
                    for v in &mut y {
                        *v *= m;
                    }
                }
                InsertKind::Rotary => {
                    self.spin = if s.fast { (self.spin + self.spin_up).min(1.0) } else { (self.spin - self.spin_down).max(0.0) };
                    let horn_hz = HORN_HZ[0] + (HORN_HZ[1] - HORN_HZ[0]) * self.spin;
                    let drum_hz = DRUM_HZ[0] + (DRUM_HZ[1] - DRUM_HZ[0]) * self.spin;
                    self.phase[0] = (self.phase[0] + horn_hz / rate).fract();
                    self.phase[1] = (self.phase[1] + drum_hz / rate).fract();
                    let (h, d) = ((TAU * self.phase[0]).sin(), (TAU * self.phase[1]).sin());
                    let mono = 0.5 * (x[0] + x[1]);
                    for c in 0..2 {
                        self.line[c][self.pos] = mono;
                    }
                    // The horn swings its tap (Doppler) and pans; the drum pans the other way.
                    for (c, v) in y.iter_mut().enumerate() {
                        let side = if c == 0 { 1.0 } else { -1.0 };
                        let delay = (0.5 + 0.45 * side * h) * (line_len - 3) as f32 + 1.0;
                        let back = self.pos as f32 + line_len as f32 - delay;
                        let i = back.floor();
                        let fr = back - i;
                        let (a, b) = (self.line[c][i as usize % line_len], self.line[c][(i as usize + 1) % line_len]);
                        let horn = a + (b - a) * fr;
                        let am_h = 1.0 - depth * 0.5 * (1.0 - side * h);
                        let am_d = 1.0 - depth * 0.35 * (1.0 + side * d);
                        *v = 0.6 * horn * am_h + 0.4 * mono * am_d;
                    }
                    self.pos = (self.pos + 1) % line_len;
                }
                InsertKind::None => {}
            }
            let m = self.mix;
            left[k] = dl + (y[0] * g - dl) * m;
            right[k] = dr + (y[1] * g - dr) * m;
        }
    }
}

impl InsertSettings {
    /// No effect.
    pub const NONE: InsertSettings = InsertSettings { kind: InsertKind::None, amount: 64, bpm: 120.0, fast: false };

    /// Every channel's insert as the control side has it (read once per buffer): the Style
    /// parts' (channels 9-16) from the style (`FxControl::insert`), none on the others;
    /// the caller puts the keyboard parts' own in (`PartInsert`, `with`).
    pub fn channels(ctl: &super::FxControl) -> [InsertSettings; 16] {
        use std::sync::atomic::Ordering::Relaxed;
        let bpm = ctl.tempo.load(Relaxed) as f32 / 100.0;
        let fast = ctl.rotary_fast.load(Relaxed);
        let mut out = [InsertSettings { bpm, fast, ..InsertSettings::NONE }; 16];
        for (p, s) in out[super::BAND_CHANNELS].iter_mut().enumerate() {
            s.kind = InsertKind::from_u8(ctl.insert[p].load(Relaxed));
            s.amount = ctl.insert_amount[p].load(Relaxed);
        }
        out
    }

    /// These settings (their tempo and rotary speed) with a keyboard part's slot.
    pub fn with(self, slot: PartInsert) -> InsertSettings {
        InsertSettings { kind: slot.kind(), amount: slot.amount, ..self }
    }
}

/// Every MIDI channel's insert on the SoundFont side, each run on its part's stem by the
/// synth's rack (audio thread; allocated in `new`): the Style parts' (channels 9-16) and
/// the keyboard parts' (channels 1-4).
pub struct ChannelInserts {
    slots: [Insert; 16],
    settings: [InsertSettings; 16],
}

impl ChannelInserts {
    pub fn new(rate: f32) -> ChannelInserts {
        ChannelInserts { slots: std::array::from_fn(|_| Insert::new(rate)), settings: [InsertSettings::NONE; 16] }
    }

    /// Take the settings for this buffer (`InsertSettings::channels`).
    pub fn set(&mut self, settings: &[InsertSettings; 16]) {
        self.settings = *settings;
    }

    /// The channels (bit = channel) whose stem runs through `process` before the mix.
    pub fn mask(&self) -> u16 {
        let mut m = 0;
        for (ch, (slot, s)) in self.slots.iter().zip(&self.settings).enumerate() {
            if slot.active(s.kind) {
                m |= 1 << ch;
            }
        }
        m
    }

    /// Run channel `channel`'s stem (`left`/`right`) through its effect in place. `level`
    /// is the part's gain in the mix (volume x expression, squared, x the master volume).
    pub fn process(&mut self, channel: usize, left: &mut [f32], right: &mut [f32], level: f32) {
        let Some(slot) = self.slots.get_mut(channel) else { return };
        // The top view (#296) shows the Style parts' inserts.
        let band = channel.checked_sub(super::BAND_CHANNELS.start).filter(|&p| p < 8);
        let perf = &crate::perf::PERF;
        let t0 = band.is_some_and(|_| perf.on()).then(crate::rt::host_now);
        slot.process(left, right, level, &self.settings[channel]);
        // Its time and output peak, atomics only.
        if let (Some(t0), Some(p)) = (t0, band) {
            perf.insert[p].add(crate::rt::host_to_ns(crate::rt::host_now().wrapping_sub(t0)));
            let peak = left.iter().chain(right.iter()).fold(0f32, |m, x| m.max(x.abs()));
            crate::perf::Perf::peak(&perf.insert_peak[p], peak);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 48_000.0;

    fn sine(hz: f32, amp: f32, n: usize) -> Vec<f32> {
        (0..n).map(|i| (TAU * hz * i as f32 / RATE).sin() * amp).collect()
    }

    fn run(kind: InsertKind, amount: u8, input: &[f32], level: f32) -> Vec<f32> {
        let mut ins = Insert::new(RATE);
        let s = InsertSettings { kind, amount, bpm: 120.0, fast: false };
        let (mut l, mut r) = (input.iter().map(|x| x * level).collect::<Vec<_>>(), input.iter().map(|x| x * level).collect::<Vec<_>>());
        for (a, b) in l.chunks_mut(64).zip(r.chunks_mut(64)) {
            ins.process(a, b, level, &s);
        }
        l
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    /// The level of harmonic `h` of `hz` in `x` (a single-bin DFT).
    fn harmonic(x: &[f32], hz: f32, h: f32) -> f32 {
        let (mut re, mut im) = (0f32, 0f32);
        for (i, v) in x.iter().enumerate() {
            let a = TAU * hz * h * i as f32 / RATE;
            re += v * a.cos();
            im += v * a.sin();
        }
        (re * re + im * im).sqrt() / x.len() as f32
    }

    #[test]
    fn none_passes_the_part_untouched() {
        let x = sine(220.0, 0.5, 4800);
        assert_eq!(run(InsertKind::None, 64, &x, 1.0), x);
    }

    /// The distortion adds odd harmonics, more with more drive, at about the input's
    /// loudness; the part's level doesn't change how hard it drives.
    #[test]
    fn distortion_adds_harmonics_by_its_drive() {
        let x = sine(220.0, 0.5, 48_000);
        let clean = run(InsertKind::Distortion, 10, &x, 1.0);
        let lead = run(InsertKind::Distortion, 120, &x, 1.0);
        let third = |y: &[f32]| harmonic(&y[4800..], 220.0, 3.0) / harmonic(&y[4800..], 220.0, 1.0);
        assert!(third(&lead) > 2.0 * third(&clean), "{} vs {}", third(&lead), third(&clean));
        assert!(third(&lead) > 0.1);
        let (li, lo) = (rms(&x[4800..]), rms(&lead[4800..]));
        assert!(lo > li * 0.3 && lo < li * 2.0, "loudness {lo} vs {li}");
        // At a quarter of the level, the same shape a quarter as loud.
        let quiet = run(InsertKind::Distortion, 120, &x, 0.25);
        assert!((third(&quiet) - third(&lead)).abs() < 0.02, "the fader doesn't change the drive");
        assert!((rms(&quiet[4800..]) / lo - 0.25).abs() < 0.02);
    }

    /// The compressor evens loud and soft: 20 dB in becomes much less out.
    #[test]
    fn the_compressor_evens_the_level() {
        let loud = rms(&run(InsertKind::Compressor, 64, &sine(220.0, 0.9, 48_000), 1.0)[24_000..]);
        let soft = rms(&run(InsertKind::Compressor, 64, &sine(220.0, 0.09, 48_000), 1.0)[24_000..]);
        assert!(loud / soft < 5.0, "{loud} / {soft}");
    }

    /// The wah opens with the part's envelope: a loud note is brighter than a soft one.
    #[test]
    fn the_wah_follows_the_envelope() {
        let bright = |amp: f32| {
            let y = run(InsertKind::AutoWah, 64, &sine(1500.0, amp, 24_000), 1.0);
            rms(&y[12_000..]) / amp
        };
        assert!(bright(0.8) > 1.5 * bright(0.05), "{} vs {}", bright(0.8), bright(0.05));
    }

    /// The tremolo moves the level at a 1/8 note (4 Hz at 120 BPM).
    #[test]
    fn the_tremolo_pulses_at_the_tempo() {
        let y = run(InsertKind::Tremolo, 100, &vec![0.5; 48_000], 1.0);
        let lows = y.windows(2).skip(4800).filter(|w| w[0] > w[1] && w[1] < 0.3 && w[0] >= 0.3).count();
        assert!((3..=5).contains(&lows), "{lows} dips in 0.9 s");
    }

    /// The rotary makes the two sides differ and moves the pitch a little.
    #[test]
    fn the_rotary_spins() {
        let mut ins = Insert::new(RATE);
        let s = InsertSettings { kind: InsertKind::Rotary, amount: 64, bpm: 120.0, fast: false };
        let x = sine(440.0, 0.5, 96_000);
        let (mut l, mut r) = (x.clone(), x.clone());
        for (a, b) in l.chunks_mut(64).zip(r.chunks_mut(64)) {
            ins.process(a, b, 1.0, &s);
        }
        let diff = l[4800..].iter().zip(&r[4800..]).map(|(a, b)| (a - b).abs()).fold(0f32, f32::max);
        assert!(diff > 0.1, "the sides differ: {diff}");
        assert!(l.iter().chain(&r).all(|v| v.is_finite() && v.abs() < 2.0));
    }

    /// The fast speed spins the horn quicker: the sides cross over far more often, and
    /// the rotor glides there instead of jumping.
    #[test]
    fn the_rotary_goes_fast() {
        let crossings = |fast: bool| {
            let mut ins = Insert::new(RATE);
            let s = InsertSettings { kind: InsertKind::Rotary, amount: 100, bpm: 120.0, fast };
            let x = vec![0.5; 3 * RATE as usize];
            let (mut l, mut r) = (x.clone(), x.clone());
            for (a, b) in l.chunks_mut(64).zip(r.chunks_mut(64)) {
                ins.process(a, b, 1.0, &s);
            }
            let d: Vec<f32> = l.iter().zip(&r).map(|(a, b)| a - b).skip(2 * RATE as usize).collect();
            (d.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count(), ins.spin)
        };
        let ((slow, s0), (fast, s1)) = (crossings(false), crossings(true));
        assert_eq!((s0, s1), (0.0, 1.0));
        assert!(slow <= 2 && fast >= 5, "slow {slow}, fast {fast} in 1 s");
        let mut ins = Insert::new(RATE);
        let (mut a, mut b) = (vec![0.5; 4800], vec![0.5; 4800]);
        ins.process(&mut a, &mut b, 1.0, &InsertSettings { kind: InsertKind::Rotary, amount: 64, bpm: 120.0, fast: true });
        assert!(ins.spin > 0.0 && ins.spin < 0.2, "it glides: {}", ins.spin);
    }

    /// A kind change fades, never jumps: the steepest step in the output stays near the
    /// input's own.
    #[test]
    fn a_kind_change_does_not_click() {
        let mut ins = Insert::new(RATE);
        let x = sine(110.0, 0.5, 48_000);
        let (mut l, mut r) = (x.clone(), x.clone());
        let kinds = [InsertKind::None, InsertKind::Distortion, InsertKind::Tremolo, InsertKind::None, InsertKind::Compressor];
        for (i, (a, b)) in l.chunks_mut(64).zip(r.chunks_mut(64)).enumerate() {
            let kind = kinds[(i / 150) % kinds.len()];
            ins.process(a, b, 1.0, &InsertSettings { kind, amount: 90, bpm: 120.0, fast: false });
        }
        // Around each change (the fade, 20 ms) no step is steeper than the steepest the
        // effects make anyway.
        let steps: Vec<f32> = l.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
        let near = |i: usize| (1..kinds.len()).any(|c| (i as isize - (c * 150 * 64) as isize).unsigned_abs() < 2000);
        let (mut steady, mut change) = (0f32, 0f32);
        for (i, &s) in steps.iter().enumerate() {
            if near(i) {
                change = change.max(s);
            } else {
                steady = steady.max(s);
            }
        }
        assert!(change <= steady * 1.2 + 1e-4, "{change} vs {steady}");
    }
}

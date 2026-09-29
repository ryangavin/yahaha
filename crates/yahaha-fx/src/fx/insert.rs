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
//!   gliding between them), horn and drum, with a little Doppler;
//! - [`InsertKind::Phaser`]: a stereo allpass-chain phaser ([`super::Phaser`]), for a
//!   keyboard part's or a strip's slot (no XG type maps to it).
//!
//! Each kind has 2-4 named settings (`kinds::InsertType::settings`, documented there),
//! read once per buffer with [`InsertSettings::value`]: the first is the old single
//! `amount`, and at the others' defaults a kind sounds bit-for-bit as it did with only the
//! amount (a style's insert and an old slot carry only that).
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
    /// A stereo phaser (the mixer rework, [`super::Phaser`]).
    Phaser,
}

impl InsertEffect {
    pub const fn kind(self) -> InsertKind {
        match self {
            InsertEffect::Distortion => InsertKind::Distortion,
            InsertEffect::Compressor => InsertKind::Compressor,
            InsertEffect::AutoWah => InsertKind::AutoWah,
            InsertEffect::Tremolo => InsertKind::Tremolo,
            InsertEffect::Rotary => InsertKind::Rotary,
            InsertEffect::Phaser => InsertKind::Phaser,
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
            InsertKind::Phaser => InsertEffect::Phaser,
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
    /// A stereo phaser ([`super::Phaser`]); no XG type maps to it.
    Phaser = 6,
}

impl InsertKind {
    pub const ALL: [InsertKind; 7] =
        [InsertKind::None, InsertKind::Distortion, InsertKind::Compressor, InsertKind::AutoWah, InsertKind::Tremolo, InsertKind::Rotary, InsertKind::Phaser];

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
            InsertKind::Phaser => "Phaser",
        }
    }
}

/// `InsertSettings::rest`: this setting at the kind's default.
pub const KIND_DEFAULT: u16 = u16::MAX;

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

/// One kind's settings (`InsertSettings::value`) as the DSP uses them, worked out once
/// per buffer. At a kind's default settings every field is exactly what the insert used
/// before it had them (only `amount`), so an old slot sounds bit-for-bit the same.
#[derive(Clone, Copy)]
struct Params {
    // Distortion: pre gain, post gain (with Output), the low cut's and the tone
    // high cut's coefficients.
    pre: f32,
    post: f32,
    lo_c: f32,
    hi_c: f32,
    // Compressor: threshold, attack and release coefficients, makeup (with Output).
    thr: f32,
    att: f32,
    rel: f32,
    makeup: f32,
    // Auto wah: the follower's attack and release, sensitivity, base frequency (Hz), the
    // filter's damping and its band-pass share.
    wa: f32,
    wr: f32,
    sens: f32,
    wah_hz: f32,
    wah_q: f32,
    wah_band: f32,
    // Tremolo: its rate (Hz) and squareness (0 = the cosine). Tremolo and rotary: depth.
    trem_hz: f32,
    trem_sq: f32,
    depth: f32,
    // Rotary: the drive's pre and post gains (pre 1: clean), the horn's and drum's share.
    rot_pre: f32,
    rot_post: f32,
    horn: f32,
    drum: f32,
    /// Phaser: depth, rate (centihertz), feedback (%), for `Phaser::set`.
    phaser: [u16; 3],
}

/// `v` (0-127) as a factor: 64 = 1, each 32 steps an octave (x0.25 .. about x4).
fn octaves_from_64(v: u16) -> f32 {
    if v == 64 { 1.0 } else { ((v as f32 - 64.0) / 32.0).exp2() }
}

/// An Output setting (0-127) as a gain: 100 = unity, linear.
fn output_gain(v: u16) -> f32 {
    if v == 100 { 1.0 } else { v as f32 / 100.0 }
}

impl Params {
    /// `s`'s kind's settings at `rate` (every kind's fields are filled: the unused ones
    /// from their defaults, cheaply).
    fn of(s: &InsertSettings, rate: f32) -> Params {
        let amount = s.amount.min(127) as f32 / 127.0;
        let v = |i: usize| s.value(i);
        let kind = s.kind;
        // Distortion: pre gain 1 (clean) .. 40 (lead); the output kept near the input's
        // loudness, times Output; Tone moves the cabinet-like high cut by up to two
        // octaves either way.
        let pre = 1.0 + 39.0 * amount * amount;
        let (tone, dist_out) = if kind == InsertKind::Distortion { (octaves_from_64(v(1)), output_gain(v(2))) } else { (1.0, 1.0) };
        let post = 1.0 / shape(pre).max(0.05) * 0.7 * dist_out;
        let hi_c = one_pole((5500.0 - 2500.0 * amount) * tone, rate);
        // Compressor: threshold -12 .. -30 dB, ratio 4, Attack (ms), Release (ms), makeup
        // times Output.
        let thr = 10f32.powf((-12.0 - 18.0 * amount) / 20.0);
        let (att_ms, rel_ms, comp_out) = if kind == InsertKind::Compressor { (v(1), v(2), output_gain(v(3))) } else { (3, 150, 1.0) };
        let (att, rel) = (time_coef(att_ms as f32 / 1000.0, rate), time_coef(rel_ms as f32 / 1000.0, rate));
        let makeup = (1.0 / thr).powf(0.75 * 0.5) * comp_out;
        // Auto wah: envelope follower, a resonant low-pass from Frequency (32: 350 Hz) up;
        // Resonance lowers its damping (64: 0.25) and the band-pass share follows, so
        // the peak rises without the level jumping.
        let (reso, freq) = if kind == InsertKind::AutoWah { (octaves_from_64(v(1)), octaves_from_64(v(2) + 32)) } else { (1.0, 1.0) };
        let wah_q = (0.25 / reso).max(0.06);
        let wah_band = 0.4 * (wah_q / 0.25).sqrt();
        // Tremolo: Note of the style tempo (1/8 by default), capped at 12 Hz; Shape.
        let bpm = s.bpm.clamp(20.0, 400.0);
        let (beats, sq) = if kind == InsertKind::Tremolo { (super::NOTES.get(v(1) as usize).map_or(0.5, |n| n.0), v(2)) } else { (0.5, 0) };
        let trem_hz = (bpm / 60.0 / beats).min(12.0);
        let trem_sq = sq as f32 / 127.0 * 20.0;
        // Rotary: Drive's clipper (0: clean) and Balance (64: horn 0.6, drum 0.4).
        let (drive, bal) = if kind == InsertKind::Rotary { (v(1), v(2)) } else { (0, 64) };
        let d = drive as f32 / 127.0;
        let rot_pre = 1.0 + 15.0 * d * d;
        let rot_post = 1.0 / rot_pre.sqrt();
        let (horn, drum) = if bal <= 64 {
            let t = bal as f32 / 64.0;
            (0.6 * t, 0.4 + 0.6 * (1.0 - t))
        } else {
            let t = (bal - 64) as f32 / 63.0;
            (0.6 + 0.4 * t, 0.4 * (1.0 - t))
        };
        let phaser = if kind == InsertKind::Phaser { [v(0), v(1), v(2)] } else { [64, 50, 40] };
        Params {
            pre,
            post,
            lo_c: one_pole(90.0, rate),
            hi_c,
            thr,
            att,
            rel,
            makeup,
            wa: time_coef(0.004, rate),
            wr: time_coef(0.09, rate),
            sens: 2.0 + 10.0 * amount,
            wah_hz: 350.0 * freq,
            wah_q,
            wah_band,
            trem_hz,
            trem_sq,
            depth: 0.2 + 0.6 * amount,
            rot_pre,
            rot_post,
            horn,
            drum,
            phaser,
        }
    }
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
    /// The phaser kind's.
    phaser: super::Phaser,
    /// The playing kind's settings as of the last buffer that asked for it: what it
    /// fades out with after a kind change.
    playing: Params,
}

/// What the control side asks of an insert, read once per buffer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InsertSettings {
    pub kind: InsertKind,
    /// Setting 1 (`kinds::InsertType::settings()[0]`), 0-127: the distortion's drive, the
    /// compressor's squeeze, the wah's sensitivity, the tremolo's, rotary's and phaser's
    /// depth.
    pub amount: u8,
    /// Settings 2-4, in the kind's own units (`kinds::InsertType::settings()[1..]`);
    /// [`KIND_DEFAULT`] (what a style's insert and an old single slot carry) plays the
    /// kind's default, which sounds as the insert did before it had them.
    pub rest: [u16; 3],
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
            phaser: super::Phaser::new(rate),
            playing: Params::of(&InsertSettings::NONE, rate),
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
        self.phaser.reset();
    }

    /// Run the part's signal (`left`/`right`, its channel's own mix) through the effect in
    /// place. `level` is the part's own gain in that mix (volume x expression x master; 0:
    /// the effect runs on it as is). RT-safe.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], level: f32, s: &InsertSettings) {
        let n = left.len().min(right.len());
        let g = if level > 1e-4 { level } else { 1.0 };
        let inv = 1.0 / g;
        let rate = self.rate;
        // The settings, once per buffer: the wanted kind's, and while a change fades the
        // playing kind out, that kind's as they last were (the wanted kind's settings
        // mean something else to it).
        let want = Params::of(s, rate);
        if self.kind == s.kind {
            self.playing = want;
        }
        let old = self.playing;
        if s.kind == InsertKind::Phaser {
            self.phaser.set(want.phaser[0], want.phaser[1], want.phaser[2]);
        }
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
            let p = if self.kind == s.kind { &want } else { &old };
            let x = [dl * inv, dr * inv];
            let mut y = x;
            match self.kind {
                InsertKind::Distortion => {
                    for (c, v) in y.iter_mut().enumerate() {
                        self.lo[c] += p.lo_c * (*v - self.lo[c]);
                        let shaped = shape((*v - self.lo[c]) * p.pre) * p.post;
                        self.hi[c] += p.hi_c * (shaped - self.hi[c]);
                        *v = self.hi[c];
                    }
                }
                InsertKind::Compressor => {
                    let peak = x[0].abs().max(x[1].abs());
                    let c = if peak > self.env { p.att } else { p.rel };
                    self.env += c * (peak - self.env);
                    let gain = if self.env > p.thr { (p.thr / self.env).powf(0.75) } else { 1.0 };
                    for v in &mut y {
                        *v *= gain * p.makeup;
                    }
                }
                InsertKind::AutoWah => {
                    let peak = (x[0].abs() + x[1].abs()) * 0.5;
                    let c = if peak > self.env { p.wa } else { p.wr };
                    self.env += c * (peak - self.env);
                    let hz = p.wah_hz * (1.0 + p.sens * self.env).min(8.6);
                    let f = 2.0 * (std::f32::consts::PI * hz.min(rate * 0.2) / rate).sin();
                    let q = p.wah_q;
                    for (c, v) in y.iter_mut().enumerate() {
                        let [low, band] = &mut self.svf[c];
                        *low += f * *band;
                        let high = *v - *low - q * *band;
                        *band += f * high;
                        *v = *low * 0.8 + *band * p.wah_band;
                    }
                }
                InsertKind::Tremolo => {
                    self.phase[0] = (self.phase[0] + p.trem_hz / rate).fract();
                    let cos = (TAU * self.phase[0]).cos();
                    // Shape 0: the smooth cosine; up, clipped toward a square.
                    let w = if p.trem_sq > 0.0 { shape(cos * p.trem_sq) / shape(p.trem_sq) } else { cos };
                    let m = 1.0 - p.depth * 0.5 * (1.0 - w);
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
                    // Drive 0: clean; up, the amp in front of the speaker clips.
                    let xin = if p.rot_pre > 1.0 { [shape(x[0] * p.rot_pre) * p.rot_post, shape(x[1] * p.rot_pre) * p.rot_post] } else { x };
                    let mono = 0.5 * (xin[0] + xin[1]);
                    for line in &mut self.line {
                        if let Some(w) = line.get_mut(self.pos) {
                            *w = mono;
                        }
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
                        let am_h = 1.0 - p.depth * 0.5 * (1.0 - side * h);
                        let am_d = 1.0 - p.depth * 0.35 * (1.0 + side * d);
                        *v = p.horn * horn * am_h + p.drum * mono * am_d;
                    }
                    self.pos = (self.pos + 1) % line_len;
                }
                InsertKind::Phaser => {
                    let (a, b) = self.phaser.tick(x[0], x[1]);
                    y = [a, b];
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
    pub const NONE: InsertSettings = InsertSettings { kind: InsertKind::None, amount: 64, rest: [KIND_DEFAULT; 3], bpm: 120.0, fast: false };

    /// Setting `i` (0-3) in its unit, clamped to the kind's range: `amount` for 0, and
    /// the kind's default for a setting at [`KIND_DEFAULT`] or one the kind hasn't.
    pub fn value(&self, i: usize) -> u16 {
        let specs = super::InsertType::from(self.kind).settings();
        let Some(spec) = specs.get(i) else { return 0 };
        let v = if i == 0 { self.amount as u16 } else { self.rest[i - 1] };
        if v == KIND_DEFAULT { spec.default } else { spec.clamp(v) }
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 48_000.0;

    fn sine(hz: f32, amp: f32, n: usize) -> Vec<f32> {
        (0..n).map(|i| (TAU * hz * i as f32 / RATE).sin() * amp).collect()
    }

    fn run(kind: InsertKind, amount: u8, input: &[f32], level: f32) -> Vec<f32> {
        let mut ins = Insert::new(RATE);
        let s = InsertSettings { kind, amount, ..InsertSettings::NONE };
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
        let s = InsertSettings { kind: InsertKind::Rotary, amount: 64, ..InsertSettings::NONE };
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
            let s = InsertSettings { kind: InsertKind::Rotary, amount: 100, fast, ..InsertSettings::NONE };
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
        ins.process(&mut a, &mut b, 1.0, &InsertSettings { kind: InsertKind::Rotary, amount: 64, fast: true, ..InsertSettings::NONE });
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
            ins.process(a, b, 1.0, &InsertSettings { kind, amount: 90, ..InsertSettings::NONE });
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

    /// A sine plus a little noise, loud for its first third and soft after (so an
    /// envelope rises and falls), left and right different, for `n` frames.
    fn signal(n: usize) -> (Vec<f32>, Vec<f32>) {
        let mut seed = 1u32;
        let l: Vec<f32> = (0..n)
            .map(|i| {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let amp = if i < n / 3 { 1.0 } else { 0.15 };
                ((TAU * 330.0 * i as f32 / RATE).sin() * 0.4 + (seed >> 8) as f32 / (1u32 << 24) as f32 * 0.2 - 0.1) * amp
            })
            .collect();
        let r = l.iter().rev().copied().collect();
        (l, r)
    }

    /// `input` through kind `kind` with `amount` and settings 2-4 `rest`, in 64-frame
    /// buffers at level 0.8.
    fn run_rest(kind: InsertKind, amount: u8, rest: [u16; 3], input: &(Vec<f32>, Vec<f32>)) -> (Vec<f32>, Vec<f32>) {
        let mut ins = Insert::new(RATE);
        let s = InsertSettings { kind, amount, rest, bpm: 133.0, ..InsertSettings::NONE };
        let (mut l, mut r): (Vec<f32>, Vec<f32>) = (input.0.iter().map(|v| v * 0.8).collect(), input.1.iter().map(|v| v * 0.8).collect());
        for (a, b) in l.chunks_mut(64).zip(r.chunks_mut(64)) {
            ins.process(a, b, 0.8, &s);
        }
        (l, r)
    }

    /// Settings at `KIND_DEFAULT` (a style's insert, an old single-amount slot) play
    /// exactly as the same settings given explicitly at the kind's defaults.
    #[test]
    fn kind_default_is_the_explicit_default() {
        let x = signal(4800);
        for kind in &InsertKind::ALL[1..] {
            let d = super::super::InsertType::from(*kind).defaults();
            for amount in [20, 90] {
                assert_eq!(run_rest(*kind, amount, [KIND_DEFAULT; 3], &x), run_rest(*kind, amount, [d[1], d[2], d[3]], &x), "{kind:?}");
            }
        }
    }

    /// Every setting after the first changes the sound: each at its far end from its
    /// default differs clearly from the default.
    #[test]
    fn every_setting_changes_the_sound() {
        let x = signal(9600);
        for kind in &InsertKind::ALL[1..] {
            let specs = super::super::InsertType::from(*kind).settings();
            let base = run_rest(*kind, 90, [KIND_DEFAULT; 3], &x);
            for (i, spec) in specs.iter().enumerate().skip(1) {
                let far = if spec.default - spec.min > spec.max - spec.default { spec.min } else { spec.max };
                let mut rest = [KIND_DEFAULT; 3];
                rest[i - 1] = far;
                let y = run_rest(*kind, 90, rest, &x);
                assert!(y.0.iter().chain(&y.1).all(|v| v.is_finite()), "{kind:?} {}", spec.name);
                let diff = base.0.iter().zip(&y.0).chain(base.1.iter().zip(&y.1)).map(|(a, b)| (a - b).abs()).fold(0f32, f32::max);
                assert!(diff > 0.02, "{kind:?} {} at {far}: {diff}", spec.name);
            }
        }
    }

    /// The phaser insert sweeps notches through the part: a fixed sine's level rises and
    /// falls over time, and stays finite.
    #[test]
    fn the_phaser_sweeps_notches() {
        let x = sine(800.0, 0.5, 24_000);
        let (l, _) = run_rest(InsertKind::Phaser, 127, [200, 0, KIND_DEFAULT], &(x.clone(), x));
        assert!(l.iter().all(|v| v.is_finite()));
        let w: Vec<f32> = l[1920..].chunks(480).map(rms).collect();
        let (lo, hi) = w.iter().fold((f32::MAX, 0f32), |(a, b), &v| (a.min(v), b.max(v)));
        assert!(hi > 3.0 * lo, "{lo} .. {hi}");
    }

    /// The phaser at depth 0 is the dry part, bit for bit; switched to None it is again
    /// once its fade has passed.
    #[test]
    fn the_phaser_bypasses_cleanly() {
        let x = sine(440.0, 0.5, 4800);
        let mut ins = Insert::new(RATE);
        let (mut l, mut r) = (x.clone(), x.clone());
        for (a, b) in l.chunks_mut(64).zip(r.chunks_mut(64)) {
            ins.process(a, b, 1.0, &InsertSettings { kind: InsertKind::Phaser, amount: 0, ..InsertSettings::NONE });
        }
        assert_eq!((&l, &r), (&x, &x));
        let (mut l, mut r) = (x.clone(), x.clone());
        let s = InsertSettings { kind: InsertKind::Phaser, amount: 127, ..InsertSettings::NONE };
        for (a, b) in l.chunks_mut(64).zip(r.chunks_mut(64)) {
            ins.process(a, b, 1.0, &s);
        }
        assert_ne!(l, x, "it phases at full depth");
        let (mut l, mut r) = (x.clone(), x.clone());
        for (a, b) in l.chunks_mut(64).zip(r.chunks_mut(64)) {
            ins.process(a, b, 1.0, &InsertSettings::NONE);
        }
        assert_eq!(l[1920..], x[1920..]);
        assert_eq!(r[1920..], x[1920..]);
    }

    /// Switching to and from the phaser fades, never jumps.
    #[test]
    fn a_phaser_change_does_not_click() {
        let mut ins = Insert::new(RATE);
        let x = sine(110.0, 0.5, 48_000);
        let (mut l, mut r) = (x.clone(), x.clone());
        let kinds = [InsertKind::None, InsertKind::Phaser, InsertKind::None, InsertKind::Phaser, InsertKind::Tremolo];
        for (i, (a, b)) in l.chunks_mut(64).zip(r.chunks_mut(64)).enumerate() {
            let kind = kinds[(i / 150) % kinds.len()];
            ins.process(a, b, 1.0, &InsertSettings { kind, amount: 127, rest: [300, 80, KIND_DEFAULT], ..InsertSettings::NONE });
        }
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

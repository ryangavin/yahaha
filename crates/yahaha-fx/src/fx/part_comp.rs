//! A channel strip's compressor (the mixer rework): after the strip's EQ, before its
//! inserts. Its settings mirror the Master Compressor's ([`super::master::MasterComp`]):
//! a type ([`CompPreset`]) that brings its parameters, which can then be edited.
//!
//! The settings ([`PartComp`]), their control-side cell ([`PartCompCell`], atomics only)
//! and the compressor on the audio thread ([`PartCompDsp`]): the Master Compressor's
//! algorithm (`master::CompCore`) at a threshold, ratio, attack, release and make-up of
//! its own. Like the inserts it sees the part as if at full volume (the part's level is
//! divided out of its detector), so a fader move doesn't change how hard it compresses.
//! Turning it on or off glides (a few ms); off and settled it isn't run at all, so the
//! stem is bit-identical to one without it.

use super::master::CompPreset;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

/// Threshold range, dB.
pub const THRESHOLD_DB: (i8, i8) = (-48, 0);
/// Ratio range, tenths (1.0:1 to 20.0:1).
pub const RATIO: (u8, u8) = (10, 200);
/// Attack range, ms.
pub const ATTACK_MS: (u16, u16) = (1, 100);
/// Release range, ms.
pub const RELEASE_MS: (u16, u16) = (10, 1000);
/// Make-up gain range, dB.
pub const MAKEUP_DB: (i8, i8) = (0, 24);

/// One of a strip compressor's parameters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PartCompParam {
    /// dB, -48..=0.
    Threshold,
    /// Tenths, 10..=200 (1.0:1 to 20.0:1).
    Ratio,
    /// ms, 1..=100.
    Attack,
    /// ms, 10..=1000.
    Release,
    /// dB, 0..=24.
    Makeup,
}

/// A strip compressor's settings. Off (the default), the strip plays as with none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PartComp {
    pub on: bool,
    /// The type the parameters started from.
    pub preset: CompPreset,
    /// dB.
    pub threshold: i8,
    /// Tenths: 40 = 4.0:1.
    pub ratio: u8,
    /// ms.
    pub attack: u16,
    /// ms.
    pub release: u16,
    /// dB.
    pub makeup: i8,
}

impl Default for PartComp {
    /// Off, Natural.
    fn default() -> PartComp {
        PartComp::of(false, CompPreset::Natural)
    }
}

/// Each type's threshold, ratio, attack, release and make-up.
fn preset_params(p: CompPreset) -> (i8, u8, u16, u16, i8) {
    match p {
        // Unity make-up on every type (mixer rule: a part's level is CC7 plus master);
        // the player adds make-up explicitly.
        CompPreset::Natural => (-18, 25, 10, 200, 0),
        CompPreset::Rich => (-20, 20, 30, 400, 0),
        CompPreset::Punchy => (-24, 60, 5, 120, 0),
        CompPreset::Electronic => (-22, 40, 3, 100, 0),
        CompPreset::Loud => (-30, 80, 2, 150, 0),
    }
}

impl PartComp {
    /// Type `preset` at its own parameters.
    pub fn of(on: bool, preset: CompPreset) -> PartComp {
        let (threshold, ratio, attack, release, makeup) = preset_params(preset);
        PartComp { on, preset, threshold, ratio, attack, release, makeup }
    }

    pub fn clamped(self) -> PartComp {
        PartComp {
            threshold: self.threshold.clamp(THRESHOLD_DB.0, THRESHOLD_DB.1),
            ratio: self.ratio.clamp(RATIO.0, RATIO.1),
            attack: self.attack.clamp(ATTACK_MS.0, ATTACK_MS.1),
            release: self.release.clamp(RELEASE_MS.0, RELEASE_MS.1),
            makeup: self.makeup.clamp(MAKEUP_DB.0, MAKEUP_DB.1),
            ..self
        }
    }

    /// The parameters differ from their type's.
    pub fn edited(&self) -> bool {
        (self.threshold, self.ratio, self.attack, self.release, self.makeup) != preset_params(self.preset)
    }

    /// Off at Natural's parameters (a rack leaves it out of its file).
    pub fn is_default(&self) -> bool {
        *self == PartComp::default()
    }

    /// One parameter to `value`, clamped to its range.
    pub fn set(&mut self, param: PartCompParam, value: i16) {
        let c = |lo: i32, hi: i32| (value as i32).clamp(lo, hi);
        match param {
            PartCompParam::Threshold => self.threshold = c(THRESHOLD_DB.0 as i32, THRESHOLD_DB.1 as i32) as i8,
            PartCompParam::Ratio => self.ratio = c(RATIO.0 as i32, RATIO.1 as i32) as u8,
            PartCompParam::Attack => self.attack = c(ATTACK_MS.0 as i32, ATTACK_MS.1 as i32) as u16,
            PartCompParam::Release => self.release = c(RELEASE_MS.0 as i32, RELEASE_MS.1 as i32) as u16,
            PartCompParam::Makeup => self.makeup = c(MAKEUP_DB.0 as i32, MAKEUP_DB.1 as i32) as i8,
        }
    }

    /// As a `u64` for an atomic ([`PartCompCell`]); `from_bits` reads it back (the type
    /// isn't carried: the audio thread needs only the parameters).
    pub const fn to_bits(self) -> u64 {
        self.on as u64 | (self.threshold as u8 as u64) << 8 | (self.ratio as u64) << 16 | (self.attack as u64) << 24 | (self.release as u64) << 40 | (self.makeup as u8 as u64) << 56
    }

    pub fn from_bits(v: u64) -> PartComp {
        PartComp {
            on: v & 1 == 1,
            preset: CompPreset::Natural,
            threshold: (v >> 8) as u8 as i8,
            ratio: (v >> 16) as u8,
            attack: (v >> 24) as u16,
            release: (v >> 40) as u16,
            makeup: (v >> 56) as u8 as i8,
        }
        .clamped()
    }
}

/// One strip compressor's settings from the control side to the audio thread: one atomic,
/// no locks.
pub struct PartCompCell(AtomicU64);

impl Default for PartCompCell {
    fn default() -> PartCompCell {
        PartCompCell::new()
    }
}

impl PartCompCell {
    pub const fn new() -> PartCompCell {
        PartCompCell(AtomicU64::new(PartComp::of_natural_off().to_bits()))
    }

    pub fn set(&self, c: &PartComp) {
        self.0.store(c.clamped().to_bits(), Relaxed);
    }

    /// Its settings (the type reads as Natural).
    pub fn get(&self) -> PartComp {
        PartComp::from_bits(self.0.load(Relaxed))
    }
}

impl PartComp {
    /// `PartComp::default()` as a const.
    const fn of_natural_off() -> PartComp {
        PartComp { on: false, preset: CompPreset::Natural, threshold: -18, ratio: 25, attack: 10, release: 200, makeup: 0 }
    }
}

/// How long the make-up gain glides to a new value (turning on or off, or a new make-up),
/// and the longest the gain change takes to come back when turned off (s).
const GLIDE_S: f32 = 0.005;
/// The knee's width, dB (as the Master Compressor's, `master::comp_curve`).
const KNEE_DB: f32 = 6.0;

/// A strip compressor on the audio thread: the Master Compressor's algorithm
/// (`master::CompCore`) at a [`PartComp`]'s parameters, on one part's stem. Off and
/// settled, it isn't run: the samples are left as they are.
#[derive(Clone, Copy, Debug)]
pub struct PartCompDsp {
    sample_rate: f32,
    /// The gain change now, dB (at most 0).
    gr_db: f32,
    /// The make-up gain now (linear), the one it glides to, its step per sample and the
    /// samples left in the glide.
    makeup: f32,
    makeup_target: f32,
    makeup_step: f32,
    glide_left: u32,
    /// Off and settled: not run.
    idle: bool,
}

impl Default for PartCompDsp {
    fn default() -> PartCompDsp {
        PartCompDsp::new(48_000.0)
    }
}

impl PartCompDsp {
    pub fn new(sample_rate: f32) -> PartCompDsp {
        let sample_rate = if sample_rate.is_finite() && sample_rate > 1_000.0 { sample_rate } else { 48_000.0 };
        PartCompDsp { sample_rate, gr_db: 0.0, makeup: 1.0, makeup_target: 1.0, makeup_step: 0.0, glide_left: 0, idle: true }
    }

    /// Whether it runs: on, or gliding back after being turned off.
    pub fn active(&self, c: &PartComp) -> bool {
        c.on || !self.idle
    }

    /// The gain change now, dB (0 or less).
    pub fn gain_reduction_db(&self) -> f32 {
        self.gr_db
    }

    /// Run `left`/`right` through it in place, at `c`. `level` is the part's gain in the
    /// mix: it is divided out of what the detector sees, so the part is compressed as if
    /// at full volume and a fader move doesn't change how hard. Off and settled, the
    /// samples aren't touched. Never allocates, locks or panics.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], level: f32, c: &PartComp) {
        let on = c.on;
        if !on && self.idle {
            return;
        }
        let n = left.len().min(right.len());
        if n == 0 {
            return;
        }
        let c = c.clamped();
        let sr = self.sample_rate;
        let glide_k = super::master::comp_coef(GLIDE_S * 1000.0, sr);
        let (ka, kr) = (super::master::comp_coef(c.attack as f32, sr), super::master::comp_coef(c.release as f32, sr));
        // Off: back within the glide, or the release if that's faster.
        let kr = if on { kr } else { kr.min(glide_k) };
        let target = if on { super::master::db_gain(c.makeup as f32) } else { 1.0 };
        if target != self.makeup_target {
            let steps = ((GLIDE_S * sr) as u32).max(1);
            self.makeup_target = target;
            self.makeup_step = (target - self.makeup) / steps as f32;
            self.glide_left = steps;
        }
        let g = if level.is_finite() && level > 1e-4 { level } else { 1.0 };
        let core = super::master::CompCore { on, threshold: c.threshold as f32, ratio: c.ratio as f32 / 10.0, knee: KNEE_DB, ka, kr, detect: 1.0 / g };
        let (m0, dm, left_n) = (self.makeup, self.makeup_step, self.glide_left as usize);
        core.run(&mut self.gr_db, &mut left[..n], &mut right[..n], |i| if i < left_n { m0 + dm * (i + 1) as f32 } else { target });
        if n >= left_n {
            self.makeup = target;
            self.glide_left = 0;
        } else {
            self.makeup = m0 + dm * n as f32;
            self.glide_left -= n as u32;
        }
        if on {
            self.idle = false;
        } else if self.gr_db > -1e-4 && self.glide_left == 0 {
            // Back at unity: stop, so the stem is its own again.
            self.gr_db = 0.0;
            self.makeup = 1.0;
            self.idle = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_off_at_natural() {
        let c = PartComp::default();
        assert!(!c.on && !c.edited() && c.is_default());
        assert_eq!(PartComp::of_natural_off(), c);
        assert_eq!(PartCompCell::new().get(), c);
    }

    #[test]
    fn a_parameter_is_clamped_and_marks_it_edited() {
        let mut c = PartComp::of(true, CompPreset::Punchy);
        c.set(PartCompParam::Ratio, 999);
        assert_eq!(c.ratio, RATIO.1);
        c.set(PartCompParam::Threshold, -100);
        assert_eq!(c.threshold, THRESHOLD_DB.0);
        assert!(c.edited());
    }

    #[test]
    fn the_cell_carries_every_parameter() {
        let mut c = PartComp::of(true, CompPreset::Loud);
        c.set(PartCompParam::Release, 777);
        let cell = PartCompCell::new();
        cell.set(&c);
        assert_eq!(cell.get(), PartComp { preset: CompPreset::Natural, ..c });
    }

    fn sine(amp: f32, n: usize) -> Vec<f32> {
        (0..n).map(|i| (std::f32::consts::TAU * 220.0 * i as f32 / 48_000.0).sin() * amp).collect()
    }

    /// `x` through a fresh compressor at `c` and `level`, 256 samples a buffer.
    fn run(c: &PartComp, x: &[f32], level: f32) -> Vec<f32> {
        let mut d = PartCompDsp::new(48_000.0);
        let (mut l, mut r) = (x.to_vec(), x.to_vec());
        for (lb, rb) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            d.process(lb, rb, level, c);
        }
        l
    }

    fn rms_db(v: &[f32]) -> f32 {
        10.0 * (v.iter().map(|s| s * s).sum::<f32>() / v.len() as f32).log10()
    }

    #[test]
    fn on_it_evens_out_loud_and_soft() {
        let c = PartComp { threshold: -30, ratio: 80, attack: 2, release: 50, makeup: 0, ..PartComp::of(true, CompPreset::Loud) };
        let n = 9_600;
        let (loud, soft) = (run(&c, &sine(1.0, n), 1.0), run(&c, &sine(0.1, n), 1.0));
        let diff = rms_db(&loud[n / 2..]) - rms_db(&soft[n / 2..]);
        assert!(diff < 8.0, "a 20 dB difference became {diff} dB");
        assert!(rms_db(&loud[n / 2..]) < rms_db(&sine(1.0, n)[n / 2..]) - 10.0);
        // Off: untouched.
        let x = sine(1.0, n);
        assert_eq!(run(&PartComp { on: false, ..c }, &x, 1.0), x);
    }

    #[test]
    fn the_parts_level_is_divided_out() {
        let c = PartComp::of(true, CompPreset::Punchy);
        let x = sine(0.8, 4_800);
        let full = run(&c, &x, 1.0);
        let quarter: Vec<f32> = x.iter().map(|s| s * 0.25).collect();
        let got = run(&c, &quarter, 0.25);
        for (a, b) in full.iter().zip(&got) {
            assert!((a * 0.25 - b).abs() < 1e-6, "{a} {b}");
        }
    }

    #[test]
    fn it_round_trips_as_json() {
        let c = PartComp::of(true, CompPreset::Rich);
        let j = serde_json::to_string(&c).unwrap();
        assert_eq!(j, r#"{"on":true,"preset":"rich","threshold":-20,"ratio":20,"attack":30,"release":400,"makeup":0}"#);
        assert_eq!(serde_json::from_str::<PartComp>(&j).unwrap(), c);
    }

    /// Every type has unity make-up: switching a strip's compressor on adds no gain.
    #[test]
    fn every_preset_has_unity_makeup() {
        for p in [CompPreset::Natural, CompPreset::Rich, CompPreset::Punchy, CompPreset::Electronic, CompPreset::Loud] {
            assert_eq!(preset_params(p).4, 0, "{p:?}");
            assert_eq!(PartComp::of(true, p).makeup, 0, "{p:?}");
        }
        assert_eq!(PartComp::default().makeup, 0);
    }
}

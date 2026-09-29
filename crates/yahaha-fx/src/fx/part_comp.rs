//! A channel strip's compressor (the mixer rework): after the strip's EQ, before its
//! inserts. Its settings mirror the Master Compressor's ([`super::master::MasterComp`]):
//! a type ([`CompPreset`]) that brings its parameters, which can then be edited.
//!
//! Stub: the settings, their control-side cell ([`PartCompCell`], atomics only) and a DSP
//! that passes the signal through ([`PartCompDsp`]). The compressor itself is the mixer
//! rework's lane A.

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
        CompPreset::Natural => (-18, 25, 10, 200, 3),
        CompPreset::Rich => (-20, 20, 30, 400, 3),
        CompPreset::Punchy => (-24, 60, 5, 120, 6),
        CompPreset::Electronic => (-22, 40, 3, 100, 5),
        CompPreset::Loud => (-30, 80, 2, 150, 9),
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
        PartComp { on: false, preset: CompPreset::Natural, threshold: -18, ratio: 25, attack: 10, release: 200, makeup: 3 }
    }
}

/// A strip compressor on the audio thread. Stub: it passes the signal through.
#[derive(Default)]
pub struct PartCompDsp {
    _sample_rate: f32,
}

impl PartCompDsp {
    pub fn new(sample_rate: f32) -> PartCompDsp {
        PartCompDsp { _sample_rate: sample_rate }
    }

    /// Run `left`/`right` through it in place, at `c`. Never allocates. Stub: unchanged.
    pub fn process(&mut self, _left: &mut [f32], _right: &mut [f32], _c: &PartComp) {}
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

    #[test]
    fn it_round_trips_as_json() {
        let c = PartComp::of(true, CompPreset::Rich);
        let j = serde_json::to_string(&c).unwrap();
        assert_eq!(j, r#"{"on":true,"preset":"rich","threshold":-20,"ratio":20,"attack":30,"release":400,"makeup":3}"#);
        assert_eq!(serde_json::from_str::<PartComp>(&j).unwrap(), c);
    }
}

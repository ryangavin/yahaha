//! A keyboard part's channel-strip EQ (#247): a low shelf and a high shelf, as the Genos
//! Mixer's Part EQ (RM p.130: High and Low for each part) and the XG multi part EQ (Data
//! List, MULTI PART 72H-77H: bass and treble gain, -12 to +12 dB; bass frequency 32 Hz to
//! 2 kHz, treble 500 Hz to 16 kHz).
//!
//! yahaha plays it on the part's audio, whatever plays the part: the SoundFont rack on the
//! part's stem (`synth::Rack`), the plugin rack on the plugin's output
//! (`plugin::PluginRack`). An OTS's XG part EQ is mapped onto it ([`PartEq::from_xg`]).
//!
//! **Mixer rule.** It is a tone control, not a level: a band at 0 dB is not run at all, so
//! a flat EQ leaves the part's audio bit-identical, and a shelf only boosts or cuts the
//! band the channel strip shows, by the gain it shows. Nothing else changes the level.
//!
//! **Threads.** The control side sets an EQ ([`EqCell::set`]) and computes its
//! coefficients there; the audio thread takes them once a buffer ([`EqCell::read`], a
//! sequence lock: no lock, no allocation, never waits) and runs them ([`EqDsp`]).

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering::{Acquire, Relaxed, Release}, fence};

/// The most a band boosts or cuts, in dB.
pub const MAX_GAIN_DB: i8 = 12;
/// The low band's frequency range and default (Hz): XG EQ BASS FREQUENCY 04H-28H, default
/// 0CH.
pub const LOW_HZ: (u16, u16) = (32, 2_000);
pub const LOW_DEFAULT_HZ: u16 = 80;
/// The high band's (XG EQ TREBLE FREQUENCY 1CH-3AH, default 36H).
pub const HIGH_HZ: (u16, u16) = (500, 16_000);
pub const HIGH_DEFAULT_HZ: u16 = 10_000;

/// XG EQ frequencies (Data List, Table#3), by data value 0-60. 0 and 60 are "thru" in the
/// table; the part EQ never uses them (its ranges are 04H-28H and 1CH-3AH).
const XG_FREQ: [u16; 61] = [
    20, 22, 25, 28, 32, 36, 40, 45, 50, 56, 63, 70, 80, 90, 100, 110, 125, 140, 160, 180, 200, 225, 250, 280, 315, 355, 400, 450, 500, 560,
    630, 700, 800, 900, 1_000, 1_100, 1_200, 1_400, 1_600, 1_800, 2_000, 2_200, 2_500, 2_800, 3_200, 3_600, 4_000, 4_500, 5_000, 5_600,
    6_300, 7_000, 8_000, 9_000, 10_000, 11_000, 12_000, 14_000, 16_000, 18_000, 20_000,
];

/// The XG multi part parameters (block 08H) of the part EQ.
const XG_BASS_GAIN: u8 = 0x72;
const XG_TREBLE_GAIN: u8 = 0x73;
const XG_BASS_FREQ: u8 = 0x76;
const XG_TREBLE_FREQ: u8 = 0x77;

/// A part's EQ: each shelf's gain (dB, -12..=12) and corner frequency (Hz).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PartEq {
    pub low_gain: i8,
    pub low_freq: u16,
    pub high_gain: i8,
    pub high_freq: u16,
}

impl Default for PartEq {
    fn default() -> PartEq {
        PartEq::FLAT
    }
}

impl PartEq {
    /// Both bands at 0 dB, at the XG default frequencies.
    pub const FLAT: PartEq = PartEq { low_gain: 0, low_freq: LOW_DEFAULT_HZ, high_gain: 0, high_freq: HIGH_DEFAULT_HZ };

    /// Both bands at 0 dB: the EQ is out of the signal (whatever its frequencies).
    pub fn is_flat(&self) -> bool {
        self.low_gain == 0 && self.high_gain == 0
    }

    /// Exactly `FLAT` (a rack leaves such an EQ out of its file).
    pub fn is_default(&self) -> bool {
        *self == PartEq::FLAT
    }

    /// Within the ranges: gains to +-12 dB, frequencies to their band's.
    pub fn clamped(self) -> PartEq {
        PartEq {
            low_gain: self.low_gain.clamp(-MAX_GAIN_DB, MAX_GAIN_DB),
            low_freq: self.low_freq.clamp(LOW_HZ.0, LOW_HZ.1),
            high_gain: self.high_gain.clamp(-MAX_GAIN_DB, MAX_GAIN_DB),
            high_freq: self.high_freq.clamp(HIGH_HZ.0, HIGH_HZ.1),
        }
    }

    /// The part EQ that XG multi part parameters (hh, nn, vv) set, over `FLAT` (a parameter
    /// they leave out is at its XG default); None when they set none of it.
    ///
    /// Gains: 40H is 0 dB, a step is 1 dB, clamped to +-12 dB (the XG range is 34H-4CH;
    /// the Genos Data List gives 00H-7FH as -12..+12 dB without a curve, so a value outside
    /// 34H-4CH is read as the nearest end). Frequencies: Table#3, clamped to the band's
    /// range.
    pub fn from_xg(items: impl IntoIterator<Item = (u8, u8, u8)>) -> Option<PartEq> {
        let mut eq = PartEq::FLAT;
        let mut any = false;
        let gain = |v: u8| (v as i16 - 64).clamp(-(MAX_GAIN_DB as i16), MAX_GAIN_DB as i16) as i8;
        let freq = |v: u8| XG_FREQ[(v as usize).min(XG_FREQ.len() - 1)];
        for (hh, nn, vv) in items {
            if hh != 0x08 {
                continue;
            }
            match nn {
                XG_BASS_GAIN => eq.low_gain = gain(vv),
                XG_TREBLE_GAIN => eq.high_gain = gain(vv),
                XG_BASS_FREQ => eq.low_freq = freq(vv),
                XG_TREBLE_FREQ => eq.high_freq = freq(vv),
                _ => continue,
            }
            any = true;
        }
        any.then(|| eq.clamped())
    }

    fn pack(self) -> u64 {
        (self.low_gain as u8 as u64) | (self.high_gain as u8 as u64) << 8 | (self.low_freq as u64) << 16 | (self.high_freq as u64) << 32
    }

    const fn packed_flat() -> u64 {
        (LOW_DEFAULT_HZ as u64) << 16 | (HIGH_DEFAULT_HZ as u64) << 32
    }

    fn unpack(v: u64) -> PartEq {
        PartEq { low_gain: v as u8 as i8, high_gain: (v >> 8) as u8 as i8, low_freq: (v >> 16) as u16, high_freq: (v >> 32) as u16 }
    }
}

/// The biquads of an EQ at a sample rate: per band b0, b1, b2, a1, a2 (a0 = 1), and
/// whether the band runs (a band at 0 dB does not).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EqCoeffs {
    pub bands: [[f32; 5]; 2],
    pub on: [bool; 2],
}

const IDENTITY: [f32; 5] = [1.0, 0.0, 0.0, 0.0, 0.0];

impl Default for EqCoeffs {
    fn default() -> EqCoeffs {
        EqCoeffs::FLAT
    }
}

impl EqCoeffs {
    pub const FLAT: EqCoeffs = EqCoeffs { bands: [IDENTITY; 2], on: [false; 2] };

    /// `eq`'s shelves at `sample_rate` (RBJ cookbook shelves, slope 1). Not for the audio
    /// thread (it calls `powf`, `sin` and `cos`), though it allocates nothing.
    pub fn new(eq: PartEq, sample_rate: f32) -> EqCoeffs {
        let eq = eq.clamped();
        let sr = if sample_rate.is_finite() && sample_rate > 1_000.0 { sample_rate } else { 48_000.0 };
        let band = |gain: i8, hz: u16, high: bool| -> Option<[f32; 5]> {
            (gain != 0).then(|| shelf(gain as f64, (hz as f64).min(0.45 * sr as f64), sr as f64, high))
        };
        let low = band(eq.low_gain, eq.low_freq, false);
        let high = band(eq.high_gain, eq.high_freq, true);
        EqCoeffs { bands: [low.unwrap_or(IDENTITY), high.unwrap_or(IDENTITY)], on: [low.is_some(), high.is_some()] }
    }

    pub fn is_flat(&self) -> bool {
        !self.on[0] && !self.on[1]
    }
}

/// An RBJ shelf with slope 1: `gain_db` below (`high`: above) `hz`.
fn shelf(gain_db: f64, hz: f64, sr: f64, high: bool) -> [f32; 5] {
    let a = 10f64.powf(gain_db / 40.0);
    let w0 = 2.0 * std::f64::consts::PI * hz / sr;
    let (sin, cos) = w0.sin_cos();
    let alpha = sin / 2.0 * std::f64::consts::SQRT_2;
    let k = 2.0 * a.sqrt() * alpha;
    let (b0, b1, b2, a0, a1, a2) = if high {
        (
            a * ((a + 1.0) + (a - 1.0) * cos + k),
            -2.0 * a * ((a - 1.0) + (a + 1.0) * cos),
            a * ((a + 1.0) + (a - 1.0) * cos - k),
            (a + 1.0) - (a - 1.0) * cos + k,
            2.0 * ((a - 1.0) - (a + 1.0) * cos),
            (a + 1.0) - (a - 1.0) * cos - k,
        )
    } else {
        (
            a * ((a + 1.0) - (a - 1.0) * cos + k),
            2.0 * a * ((a - 1.0) - (a + 1.0) * cos),
            a * ((a + 1.0) - (a - 1.0) * cos - k),
            (a + 1.0) + (a - 1.0) * cos + k,
            -2.0 * ((a - 1.0) + (a + 1.0) * cos),
            (a + 1.0) + (a - 1.0) * cos - k,
        )
    };
    [b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0].map(|x| x as f32)
}

/// Coefficient slots in `EqCell`: 5 per band, then the bands that run (bit = band).
const SLOTS: usize = 11;

/// One part's EQ, shared: its settings, and its coefficients behind a sequence lock the
/// audio thread reads without waiting. See the module docs.
pub struct EqCell {
    settings: AtomicU64,
    /// Even: the coefficients are whole. Odd: a writer is changing them.
    seq: AtomicU32,
    coef: [AtomicU32; SLOTS],
}

impl Default for EqCell {
    fn default() -> EqCell {
        EqCell::new()
    }
}

impl EqCell {
    /// Flat.
    pub const fn new() -> EqCell {
        const ONE: u32 = 0x3F80_0000; // 1.0f32
        let mut coef = [const { AtomicU32::new(0) }; SLOTS];
        coef[0] = AtomicU32::new(ONE);
        coef[5] = AtomicU32::new(ONE);
        EqCell { settings: AtomicU64::new(PartEq::packed_flat()), seq: AtomicU32::new(0), coef }
    }

    /// The EQ as last set.
    pub fn get(&self) -> PartEq {
        PartEq::unpack(self.settings.load(Relaxed))
    }

    /// Set the EQ (clamped) and publish its coefficients at `sample_rate`. Control side
    /// only: it computes them here, and waits for another writer.
    pub fn set(&self, eq: PartEq, sample_rate: f32) {
        let eq = eq.clamped();
        self.settings.store(eq.pack(), Relaxed);
        self.publish(&EqCoeffs::new(eq, sample_rate));
    }

    /// Publish the coefficients again, for a new sample rate.
    pub fn recompute(&self, sample_rate: f32) {
        self.publish(&EqCoeffs::new(self.get(), sample_rate));
    }

    fn publish(&self, c: &EqCoeffs) {
        // Take the lock: from even (whole) to odd (being written).
        let mut s = self.seq.load(Relaxed);
        loop {
            if s & 1 == 1 {
                std::hint::spin_loop();
                s = self.seq.load(Relaxed);
                continue;
            }
            match self.seq.compare_exchange_weak(s, s.wrapping_add(1), Acquire, Relaxed) {
                Ok(_) => break,
                Err(now) => s = now,
            }
        }
        fence(Release);
        for (b, band) in c.bands.iter().enumerate() {
            for (i, &x) in band.iter().enumerate() {
                self.coef[5 * b + i].store(x.to_bits(), Relaxed);
            }
        }
        self.coef[10].store(c.on[0] as u32 | (c.on[1] as u32) << 1, Relaxed);
        self.seq.store(s.wrapping_add(2), Release);
    }

    /// Audio thread: the coefficients, if they were published since `seen` (a value this
    /// call returned before; start from `EqCell::UNSEEN`) and are whole now. A read that
    /// meets a writer returns None and the next one tries again. RT-safe.
    #[inline]
    pub fn read(&self, seen: &mut u32) -> Option<EqCoeffs> {
        let s1 = self.seq.load(Acquire);
        if s1 == *seen || s1 & 1 == 1 {
            return None;
        }
        let x = |i: usize| f32::from_bits(self.coef[i].load(Relaxed));
        let on = self.coef[10].load(Relaxed);
        let c = EqCoeffs { bands: [std::array::from_fn(x), std::array::from_fn(|i| x(5 + i))], on: [on & 1 != 0, on & 2 != 0] };
        fence(Acquire);
        if self.seq.load(Relaxed) != s1 {
            return None;
        }
        *seen = s1;
        Some(c)
    }

    /// A `seen` for `read` that no publication matches (the sequence is even when whole).
    pub const UNSEEN: u32 = u32::MAX;
}

/// A part's EQ on the audio thread: its coefficients and each band's filter state per
/// side (transposed direct form II). Nothing here allocates, locks or panics.
#[derive(Clone, Copy, Debug, Default)]
pub struct EqDsp {
    c: EqCoeffs,
    /// [band][side][state].
    z: [[[f32; 2]; 2]; 2],
}

impl EqDsp {
    pub const fn new() -> EqDsp {
        EqDsp { c: EqCoeffs::FLAT, z: [[[0.0; 2]; 2]; 2] }
    }

    /// New coefficients. A band that starts or stops running starts from rest.
    pub fn set(&mut self, c: &EqCoeffs) {
        for b in 0..2 {
            if !c.on[b] || !self.c.on[b] {
                self.z[b] = [[0.0; 2]; 2];
            }
        }
        self.c = *c;
    }

    /// Whether any band runs: when not, `process` leaves the audio as it is.
    #[inline]
    pub fn active(&self) -> bool {
        !self.c.is_flat()
    }

    /// Forget the filters' state (the part went silent).
    pub fn clear(&mut self) {
        self.z = [[[0.0; 2]; 2]; 2];
    }

    /// Run the EQ on a stereo block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for b in 0..2 {
            if !self.c.on[b] {
                continue;
            }
            let [b0, b1, b2, a1, a2] = self.c.bands[b];
            for (side, x) in [&mut *left, &mut *right].into_iter().enumerate() {
                let [mut z1, mut z2] = self.z[b][side];
                for s in x.iter_mut() {
                    let i = *s;
                    let y = b0 * i + z1;
                    z1 = b1 * i - a1 * y + z2;
                    z2 = b2 * i - a2 * y;
                    *s = y;
                }
                // No denormals ringing on in the silence after a note.
                if z1.abs() < 1e-20 {
                    z1 = 0.0;
                }
                if z2.abs() < 1e-20 {
                    z2 = 0.0;
                }
                self.z[b][side] = [z1, z2];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(hz: f32, n: usize) -> Vec<f32> {
        (0..n).map(|i| (2.0 * std::f32::consts::PI * hz * i as f32 / 48_000.0).sin() * 0.5).collect()
    }

    /// The gain in dB `eq` gives a sine at `hz` (after it settles).
    fn gain_db(eq: PartEq, hz: f32) -> f32 {
        let mut d = EqDsp::new();
        d.set(&EqCoeffs::new(eq, 48_000.0));
        let x = sine(hz, 48_000);
        let (mut l, mut r) = (x.clone(), x.clone());
        d.process(&mut l, &mut r);
        let rms = |v: &[f32]| (v.iter().map(|s| s * s).sum::<f32>() / v.len() as f32).sqrt();
        20.0 * (rms(&l[24_000..]) / rms(&x[24_000..])).log10()
    }

    #[test]
    fn a_flat_eq_leaves_the_audio_bit_identical() {
        let x = sine(440.0, 1024);
        for eq in [PartEq::FLAT, PartEq { low_freq: 500, high_freq: 2_000, ..PartEq::FLAT }] {
            let mut d = EqDsp::new();
            d.set(&EqCoeffs::new(eq, 48_000.0));
            assert!(!d.active());
            let (mut l, mut r) = (x.clone(), x.clone());
            d.process(&mut l, &mut r);
            assert_eq!(l, x);
            assert_eq!(r, x);
        }
    }

    #[test]
    fn the_shelves_boost_and_cut_their_bands_by_their_gain() {
        let bass = PartEq { low_gain: 12, low_freq: 200, ..PartEq::FLAT };
        assert!((gain_db(bass, 40.0) - 12.0).abs() < 0.5, "{}", gain_db(bass, 40.0));
        assert!(gain_db(bass, 5_000.0).abs() < 0.2);
        let treble = PartEq { high_gain: -12, high_freq: 2_000, ..PartEq::FLAT };
        assert!((gain_db(treble, 15_000.0) + 12.0).abs() < 0.8, "{}", gain_db(treble, 15_000.0));
        assert!(gain_db(treble, 100.0).abs() < 0.2);
        // Half the gain at the corner.
        assert!((gain_db(bass, 200.0) - 6.0).abs() < 1.0, "{}", gain_db(bass, 200.0));
    }

    #[test]
    fn xg_part_eq_maps_onto_the_part_eq() {
        assert_eq!(PartEq::from_xg([(0x08, 0x05, 0), (0x08, 0x18, 30)]), None, "no EQ parameter");
        let eq = PartEq::from_xg([(0x08, 0x72, 0x46), (0x08, 0x73, 0x3A), (0x08, 0x76, 0x14), (0x08, 0x77, 0x30)]).unwrap();
        assert_eq!(eq, PartEq { low_gain: 6, low_freq: 200, high_gain: -6, high_freq: 5_000 });
        // One parameter: the others at their XG defaults (80 Hz, 10 kHz, 0 dB).
        assert_eq!(PartEq::from_xg([(0x08, 0x73, 0x44)]), Some(PartEq { high_gain: 4, ..PartEq::FLAT }));
        // Outside the ranges: the nearest end.
        let eq = PartEq::from_xg([(0x08, 0x72, 0x7F), (0x08, 0x73, 0x00), (0x08, 0x76, 0x7F), (0x08, 0x77, 0x00)]).unwrap();
        assert_eq!(eq, PartEq { low_gain: 12, low_freq: 2_000, high_gain: -12, high_freq: 500 });
        // The XG defaults are the flat EQ.
        assert_eq!(PartEq::from_xg([(0x08, 0x72, 0x40), (0x08, 0x73, 0x40), (0x08, 0x76, 0x0C), (0x08, 0x77, 0x36)]), Some(PartEq::FLAT));
    }

    #[test]
    fn the_cell_hands_over_what_was_set_once() {
        let cell = EqCell::new();
        let mut seen = EqCell::UNSEEN;
        assert_eq!(cell.read(&mut seen), Some(EqCoeffs::FLAT));
        assert_eq!(cell.read(&mut seen), None, "nothing new");
        let eq = PartEq { low_gain: -3, low_freq: 100, high_gain: 5, high_freq: 8_000 };
        cell.set(eq, 44_100.0);
        assert_eq!(cell.get(), eq);
        assert_eq!(cell.read(&mut seen), Some(EqCoeffs::new(eq, 44_100.0)));
        assert_eq!(cell.read(&mut seen), None);
        cell.set(PartEq { low_gain: 40, low_freq: 1, high_gain: -40, high_freq: 60_000 }, 48_000.0);
        assert_eq!(cell.get(), PartEq { low_gain: 12, low_freq: 32, high_gain: -12, high_freq: 16_000 }, "clamped");
    }

    #[test]
    fn a_missing_field_reads_as_flat() {
        let eq: PartEq = serde_json::from_str(r#"{"lowGain":3}"#).unwrap();
        assert_eq!(eq, PartEq { low_gain: 3, ..PartEq::FLAT });
        assert_eq!(serde_json::to_string(&PartEq::FLAT).unwrap(), r#"{"lowGain":0,"lowFreq":80,"highGain":0,"highFreq":10000}"#);
    }
}

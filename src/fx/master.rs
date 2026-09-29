//! The Master Compressor and the Master EQ (Genos RM p.130-131 and p.136, OM p.106): the
//! last stage on the master bus, after the effect returns and before the output's safety
//! clipper (`synth::soft_clip`). As on the Genos, the metronome click doesn't go through
//! them (RM p.131, p.136: "cannot be applied to ... the metronome sound").
//!
//! - **Compressor**: on/off, a type (Natural, Rich, Punchy, Electronic, Loud) and the
//!   three parameters the Genos shows: Compression (threshold, ratio and knee together),
//!   Texture (how light it is: its attack and release) and Output (the level after it).
//! - **EQ**: eight bands, each with its gain (-12..+12 dB), centre frequency and Q; the
//!   leftmost and rightmost bands can be shelves (the Data List's MULTI EQ ranges: band 1
//!   32 Hz-2 kHz, bands 2-7 100 Hz-10 kHz, band 8 500 Hz-16 kHz; Q 0.1-12.0). A type
//!   (Flat, Mellow, Bright, Loudness, Powerful) sets every band.
//!
//! **Mixer rule.** Both are tone on the master, shown with their settings; they may boost
//! (an EQ band, the compressor's Output). Off, they are not run at all: the output is
//! bit-identical to the master bus without them.
//!
//! **Threads.** The control side sets them ([`MasterControl`]): the compressor's
//! parameters as plain atomics, the EQ's coefficients computed there and handed over
//! behind a sequence lock, as the part EQ's (`part_eq::EqCell`). The audio thread
//! ([`MasterDsp`]) reads them once a buffer and never allocates, locks or waits. The
//! compressor's gain glides (its attack and release; the Output over a buffer), so a
//! change of type or parameter never clicks; switched off, it glides back to unity and
//! then stops running.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicI8, AtomicU8, AtomicU32, Ordering::{Acquire, Relaxed, Release}, fence};

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

/// The Master EQ's bands.
pub const EQ_BANDS: usize = 8;
/// The most a band boosts or cuts, in dB.
pub const EQ_MAX_GAIN_DB: i8 = 12;
/// Each band's centre frequency range (Hz), from the Data List's MULTI EQ: band 1 32 Hz-2
/// kHz, the middle bands 100 Hz-10 kHz, band 8 500 Hz-16 kHz.
pub const EQ_FREQ_RANGE: [(u16, u16); EQ_BANDS] = [(32, 2_000), (100, 10_000), (100, 10_000), (100, 10_000), (100, 10_000), (100, 10_000), (100, 10_000), (500, 16_000)];
/// The bands' frequencies as the Genos starts them (RM p.130).
pub const EQ_DEFAULT_FREQ: [u16; EQ_BANDS] = [80, 250, 500, 630, 800, 1_000, 4_000, 8_000];
/// Q in tenths: 0.1-12.0 (Data List).
pub const EQ_Q_RANGE: (u8, u8) = (1, 120);
/// A band's Q as the types set it: 0.7.
pub const EQ_DEFAULT_Q: u8 = 7;
/// The compressor's Output range, in dB.
pub const COMP_OUTPUT_DB: (i8, i8) = (-12, 12);

/// One Master EQ band.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct EqBand {
    /// dB, -12..=12.
    pub gain: i8,
    /// The centre (a shelf's corner) frequency, Hz, in the band's range (`EQ_FREQ_RANGE`).
    pub freq: u16,
    /// Q in tenths (1-120: 0.1-12.0); higher is narrower. A shelf has none (slope 1).
    pub q: u8,
    /// A shelf rather than a peak/dip: the leftmost and rightmost bands only.
    pub shelf: bool,
}

impl Default for EqBand {
    fn default() -> EqBand {
        EqBand { gain: 0, freq: 1_000, q: EQ_DEFAULT_Q, shelf: false }
    }
}

impl EqBand {
    /// Band `i` within its ranges.
    pub fn clamped(self, i: usize) -> EqBand {
        let (lo, hi) = EQ_FREQ_RANGE[i.min(EQ_BANDS - 1)];
        EqBand {
            gain: self.gain.clamp(-EQ_MAX_GAIN_DB, EQ_MAX_GAIN_DB),
            freq: self.freq.clamp(lo, hi),
            q: self.q.clamp(EQ_Q_RANGE.0, EQ_Q_RANGE.1),
            shelf: self.shelf && (i == 0 || i == EQ_BANDS - 1),
        }
    }

    /// 32 bits for `EqCell`: gain, Q, frequency (15 bits: at most 16 kHz), shelf.
    fn pack(self) -> u32 {
        self.gain as u8 as u32 | (self.q as u32) << 8 | (self.freq as u32 & 0x7FFF) << 16 | (self.shelf as u32) << 31
    }

    fn unpack(v: u32) -> EqBand {
        EqBand { gain: v as u8 as i8, q: (v >> 8) as u8, freq: ((v >> 16) & 0x7FFF) as u16, shelf: v >> 31 == 1 }
    }
}

/// A Master EQ type (RM p.131). The Genos's speaker types (HS7, HS8, STAGEPAS) and its
/// User types aren't here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EqPreset {
    /// Every band at 0 dB.
    #[default]
    Flat,
    /// The highs a little down.
    Mellow,
    /// The highs up.
    Bright,
    /// The lows and highs up.
    Loudness,
    /// Every band up a little, the ends more.
    Powerful,
}

impl EqPreset {
    pub const ALL: [EqPreset; 5] = [EqPreset::Flat, EqPreset::Mellow, EqPreset::Bright, EqPreset::Loudness, EqPreset::Powerful];

    pub fn name(self) -> &'static str {
        match self {
            EqPreset::Flat => "Flat",
            EqPreset::Mellow => "Mellow",
            EqPreset::Bright => "Bright",
            EqPreset::Loudness => "Loudness",
            EqPreset::Powerful => "Powerful",
        }
    }

    /// Its bands: the default frequencies, Q 0.7, the edge bands as shelves, and its gains.
    pub fn bands(self) -> [EqBand; EQ_BANDS] {
        let gains: [i8; EQ_BANDS] = match self {
            EqPreset::Flat => [0; EQ_BANDS],
            EqPreset::Mellow => [0, 0, 0, 0, 0, 0, -2, -4],
            EqPreset::Bright => [0, 0, 0, 0, 0, 0, 2, 4],
            EqPreset::Loudness => [4, 1, 0, 0, 0, 0, 2, 4],
            EqPreset::Powerful => [4, 2, 1, 1, 1, 1, 2, 3],
        };
        std::array::from_fn(|i| EqBand { gain: gains[i], freq: EQ_DEFAULT_FREQ[i], q: EQ_DEFAULT_Q, shelf: i == 0 || i == EQ_BANDS - 1 })
    }
}

/// The Master EQ's settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MasterEq {
    pub on: bool,
    /// The type the bands started from (`setMasterEqPreset`); an edited band leaves it.
    pub preset: EqPreset,
    pub bands: [EqBand; EQ_BANDS],
}

impl Default for MasterEq {
    /// Off, Flat.
    fn default() -> MasterEq {
        MasterEq { on: false, preset: EqPreset::Flat, bands: EqPreset::Flat.bands() }
    }
}

impl MasterEq {
    pub fn clamped(self) -> MasterEq {
        MasterEq { bands: std::array::from_fn(|i| self.bands[i].clamped(i)), ..self }
    }

    /// The bands differ from their type's.
    pub fn edited(&self) -> bool {
        self.bands != self.preset.bands()
    }
}

/// A Master Compressor type (RM p.136). The Genos's User types aren't here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CompPreset {
    /// Moderate.
    #[default]
    Natural,
    /// Gentle and slow, for acoustic instruments and jazz.
    Rich,
    /// Heavy and fast, for rock.
    Punchy,
    /// For electronic dance music.
    Electronic,
    /// The most, for energetic music.
    Loud,
}

impl CompPreset {
    pub const ALL: [CompPreset; 5] = [CompPreset::Natural, CompPreset::Rich, CompPreset::Punchy, CompPreset::Electronic, CompPreset::Loud];

    pub fn name(self) -> &'static str {
        match self {
            CompPreset::Natural => "Natural",
            CompPreset::Rich => "Rich",
            CompPreset::Punchy => "Punchy",
            CompPreset::Electronic => "Electronic",
            CompPreset::Loud => "Loud",
        }
    }

    /// Its Compression (%), Texture (%) and Output (dB).
    pub fn params(self) -> (u8, u8, i8) {
        match self {
            CompPreset::Natural => (30, 50, 1),
            CompPreset::Rich => (45, 30, 2),
            CompPreset::Punchy => (70, 80, 4),
            CompPreset::Electronic => (60, 65, 3),
            CompPreset::Loud => (85, 45, 6),
        }
    }
}

/// The Master Compressor's settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MasterComp {
    pub on: bool,
    /// The type the parameters started from (`setMasterCompressorPreset`).
    pub preset: CompPreset,
    /// 0-100 %: 0 compresses nothing; more lowers the threshold and raises the ratio
    /// (`comp_curve`).
    pub compression: u8,
    /// 0-100 %: higher is lighter (a faster attack and release, `comp_times`).
    pub texture: u8,
    /// The level after it, -12..=12 dB.
    pub output: i8,
}

impl Default for MasterComp {
    /// Off, Natural.
    fn default() -> MasterComp {
        MasterComp::of(false, CompPreset::Natural)
    }
}

impl MasterComp {
    /// Type `preset` at its own parameters.
    pub fn of(on: bool, preset: CompPreset) -> MasterComp {
        let (compression, texture, output) = preset.params();
        MasterComp { on, preset, compression, texture, output }
    }

    pub fn clamped(self) -> MasterComp {
        MasterComp { compression: self.compression.min(100), texture: self.texture.min(100), output: self.output.clamp(COMP_OUTPUT_DB.0, COMP_OUTPUT_DB.1), ..self }
    }

    /// The parameters differ from their type's.
    pub fn edited(&self) -> bool {
        (self.compression, self.texture, self.output) != self.preset.params()
    }
}

/// Both, as saved (`<data>/master-effects.json`) and shown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MasterSettings {
    pub compressor: MasterComp,
    pub eq: MasterEq,
}

/// The compressor's static curve for `compression` (0-100 %): threshold (dBFS), ratio and
/// knee width (dB). 0 % is a ratio of 1: nothing.
pub fn comp_curve(compression: u8) -> (f32, f32, f32) {
    let c = compression.min(100) as f32;
    (-3.0 - 0.27 * c, 1.0 + 0.07 * c, 6.0)
}

/// The compressor's attack and release (ms) for `texture` (0-100 %): 0 is heavy and smooth
/// (30 ms, 500 ms), 100 light and lively (1 ms, 60 ms).
pub fn comp_times(texture: u8) -> (f32, f32) {
    let t = texture.min(100) as f32;
    (30.0 - 0.29 * t, 500.0 - 4.4 * t)
}

/// The gain change (dB, at most 0) the static curve gives a level of `x_db`.
#[inline]
fn curve_db(x_db: f32, threshold: f32, ratio: f32, knee: f32) -> f32 {
    let over = x_db - threshold;
    let slope = 1.0 / ratio - 1.0;
    if 2.0 * over <= -knee {
        0.0
    } else if 2.0 * over.abs() <= knee {
        let a = over + knee / 2.0;
        slope * a * a / (2.0 * knee)
    } else {
        slope * over
    }
}

// ---------------------------------------------------------------------------
// The EQ's coefficients
// ---------------------------------------------------------------------------

/// The biquads of a Master EQ at a sample rate: per band b0, b1, b2, a1, a2 (a0 = 1), and
/// which bands run (bit = band; a band at 0 dB, or every band with the EQ off, does not).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EqCoeffs {
    pub bands: [[f32; 5]; EQ_BANDS],
    pub on: u8,
}

const IDENTITY: [f32; 5] = [1.0, 0.0, 0.0, 0.0, 0.0];

impl EqCoeffs {
    pub const FLAT: EqCoeffs = EqCoeffs { bands: [IDENTITY; EQ_BANDS], on: 0 };

    /// `eq` at `sample_rate`. Not for the audio thread (`powf`, `sin`, `cos`), though it
    /// allocates nothing.
    pub fn new(eq: &MasterEq, sample_rate: f32) -> EqCoeffs {
        if !eq.on {
            return EqCoeffs::FLAT;
        }
        let eq = eq.clamped();
        let sr = if sample_rate.is_finite() && sample_rate > 1_000.0 { sample_rate as f64 } else { 48_000.0 };
        let mut c = EqCoeffs::FLAT;
        for (i, b) in eq.bands.iter().enumerate() {
            if b.gain == 0 {
                continue;
            }
            let hz = (b.freq as f64).min(0.45 * sr);
            c.bands[i] = if b.shelf { super::part_eq::shelf(b.gain as f64, hz, sr, i != 0) } else { peak(b.gain as f64, hz, b.q as f64 / 10.0, sr) };
            c.on |= 1 << i;
        }
        c
    }
}

/// An RBJ peaking EQ: `gain_db` at `hz`, bandwidth `q`.
fn peak(gain_db: f64, hz: f64, q: f64, sr: f64) -> [f32; 5] {
    let a = 10f64.powf(gain_db / 40.0);
    let w0 = 2.0 * std::f64::consts::PI * hz / sr;
    let (sin, cos) = w0.sin_cos();
    let alpha = sin / (2.0 * q.max(0.1));
    let a0 = 1.0 + alpha / a;
    [(1.0 + alpha * a) / a0, (-2.0 * cos) / a0, (1.0 - alpha * a) / a0, (-2.0 * cos) / a0, (1.0 - alpha / a) / a0].map(|x| x as f32)
}

/// Coefficient slots: 5 per band, then the bands that run.
const SLOTS: usize = 5 * EQ_BANDS + 1;

/// The Master EQ, shared: its settings, the sample rate, and its coefficients behind a
/// sequence lock the audio thread reads without waiting (as `part_eq::EqCell`).
struct EqCell {
    on: AtomicBool,
    bands: [AtomicU32; EQ_BANDS],
    /// f32 bits.
    rate: AtomicU32,
    /// Even: the coefficients are whole. Odd: a writer is changing them.
    seq: AtomicU32,
    coef: [AtomicU32; SLOTS],
}

impl EqCell {
    /// Off and flat, at 48 kHz.
    fn new() -> EqCell {
        let flat = MasterEq::default();
        EqCell {
            on: AtomicBool::new(false),
            bands: flat.bands.map(|b| AtomicU32::new(b.pack())),
            rate: AtomicU32::new(48_000f32.to_bits()),
            seq: AtomicU32::new(0),
            coef: std::array::from_fn(|i| AtomicU32::new(if i < 5 * EQ_BANDS && i % 5 == 0 { 1f32.to_bits() } else { 0 })),
        }
    }

    /// The EQ as last set (its type isn't kept here).
    fn get(&self) -> (bool, [EqBand; EQ_BANDS]) {
        (self.on.load(Relaxed), std::array::from_fn(|i| EqBand::unpack(self.bands[i].load(Relaxed))))
    }

    fn set(&self, eq: &MasterEq) {
        let eq = eq.clamped();
        if self.get() == (eq.on, eq.bands) {
            return;
        }
        self.on.store(eq.on, Relaxed);
        for (a, b) in self.bands.iter().zip(eq.bands) {
            a.store(b.pack(), Relaxed);
        }
        self.publish(&EqCoeffs::new(&eq, f32::from_bits(self.rate.load(Relaxed))));
    }

    fn set_rate(&self, sample_rate: f32) {
        self.rate.store(sample_rate.to_bits(), Relaxed);
        let (on, bands) = self.get();
        self.publish(&EqCoeffs::new(&MasterEq { on, bands, preset: EqPreset::Flat }, sample_rate));
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
        self.coef[SLOTS - 1].store(c.on as u32, Relaxed);
        self.seq.store(s.wrapping_add(2), Release);
    }

    /// Audio thread: the coefficients, if they were published since `seen` and are whole
    /// now (as `part_eq::EqCell::read`). RT-safe.
    #[inline]
    fn read(&self, seen: &mut u32) -> Option<EqCoeffs> {
        let s1 = self.seq.load(Acquire);
        if s1 == *seen || s1 & 1 == 1 {
            return None;
        }
        let x = |i: usize| f32::from_bits(self.coef[i].load(Relaxed));
        let c = EqCoeffs { bands: std::array::from_fn(|b| std::array::from_fn(|i| x(5 * b + i))), on: self.coef[SLOTS - 1].load(Relaxed) as u8 };
        fence(Acquire);
        if self.seq.load(Relaxed) != s1 {
            return None;
        }
        *seen = s1;
        Some(c)
    }
}

// ---------------------------------------------------------------------------
// The control side
// ---------------------------------------------------------------------------

/// The master chain's settings, turned by the control side and read by the audio thread
/// once per buffer (in `FxControl::master`).
pub struct MasterControl {
    pub comp_on: AtomicBool,
    pub compression: AtomicU8,
    pub texture: AtomicU8,
    pub output: AtomicI8,
    eq: EqCell,
}

impl Default for MasterControl {
    fn default() -> MasterControl {
        MasterControl::new()
    }
}

impl MasterControl {
    /// Both off.
    pub fn new() -> MasterControl {
        let c = MasterComp::default();
        MasterControl { comp_on: AtomicBool::new(false), compression: AtomicU8::new(c.compression), texture: AtomicU8::new(c.texture), output: AtomicI8::new(c.output), eq: EqCell::new() }
    }

    /// The compressor's settings (plain stores).
    pub fn set_compressor(&self, c: &MasterComp) {
        let c = c.clamped();
        self.compression.store(c.compression, Relaxed);
        self.texture.store(c.texture, Relaxed);
        self.output.store(c.output, Relaxed);
        self.comp_on.store(c.on, Relaxed);
    }

    /// The EQ's settings: its coefficients are computed and published here when they
    /// changed. Control side only (it may wait for another writer).
    pub fn set_eq(&self, eq: &MasterEq) {
        self.eq.set(eq);
    }

    /// The EQ as last set: on, and its bands (clamped).
    pub fn eq(&self) -> (bool, [EqBand; EQ_BANDS]) {
        self.eq.get()
    }

    /// The output's sample rate: the EQ's coefficients again at it. Off the audio thread.
    pub fn set_sample_rate(&self, sample_rate: f32) {
        self.eq.set_rate(sample_rate);
    }
}

// ---------------------------------------------------------------------------
// The audio thread
// ---------------------------------------------------------------------------

/// The Master EQ on the audio thread: coefficients and each band's filter state per side
/// (transposed direct form II).
#[derive(Clone, Copy, Debug)]
struct EqDsp {
    c: EqCoeffs,
    /// [band][side][state].
    z: [[[f32; 2]; 2]; EQ_BANDS],
}

impl EqDsp {
    fn set(&mut self, c: &EqCoeffs) {
        // A band that starts or stops running starts from rest.
        for b in 0..EQ_BANDS {
            if (c.on >> b) & 1 == 0 || (self.c.on >> b) & 1 == 0 {
                self.z[b] = [[0.0; 2]; 2];
            }
        }
        self.c = *c;
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for b in 0..EQ_BANDS {
            if (self.c.on >> b) & 1 == 0 {
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

/// The Master Compressor on the audio thread: a stereo-linked peak detector, the static
/// curve, the gain change smoothed by the attack and release, and the Output gliding.
#[derive(Clone, Copy, Debug)]
struct CompDsp {
    sample_rate: f32,
    /// The gain change now, dB (at most 0).
    gr_db: f32,
    /// The Output gain now (linear).
    makeup: f32,
    /// Off and back at unity: not run.
    idle: bool,
}

impl CompDsp {
    fn process(&mut self, left: &mut [f32], right: &mut [f32], ctl: &MasterControl) {
        let on = ctl.comp_on.load(Relaxed);
        if !on && self.idle {
            return;
        }
        let n = left.len().min(right.len());
        if n == 0 {
            return;
        }
        let (threshold, ratio, knee) = comp_curve(ctl.compression.load(Relaxed));
        let (attack, release) = comp_times(ctl.texture.load(Relaxed));
        let coef = |ms: f32| (-1.0 / (ms.max(0.1) * 0.001 * self.sample_rate)).exp();
        let (ka, kr) = (coef(attack), coef(release));
        let target = if on { db_gain(ctl.output.load(Relaxed).clamp(COMP_OUTPUT_DB.0, COMP_OUTPUT_DB.1) as f32) } else { 1.0 };
        let (m0, dm) = (self.makeup, (target - self.makeup) / n as f32);
        for i in 0..n {
            let (l, r) = (left[i], right[i]);
            let want = if on {
                let x = l.abs().max(r.abs()).max(1e-9);
                curve_db(20.0 * x.log10(), threshold, ratio, knee)
            } else {
                0.0
            };
            let k = if want < self.gr_db { ka } else { kr };
            self.gr_db = want + k * (self.gr_db - want);
            let g = db_gain(self.gr_db) * (m0 + dm * (i + 1) as f32);
            left[i] = l * g;
            right[i] = r * g;
        }
        self.makeup = target;
        if on {
            self.idle = false;
        } else if self.gr_db > -1e-4 {
            // Back at unity: stop, so the output is the bus's own again.
            self.gr_db = 0.0;
            self.makeup = 1.0;
            self.idle = true;
        }
    }
}

#[inline]
fn db_gain(db: f32) -> f32 {
    (db * (std::f32::consts::LN_10 / 20.0)).exp()
}

/// The master chain on the audio thread: the compressor, then the EQ. Nothing here
/// allocates, locks or panics.
#[derive(Clone, Copy, Debug)]
pub struct MasterDsp {
    comp: CompDsp,
    eq: EqDsp,
    eq_seen: u32,
}

impl MasterDsp {
    pub fn new(sample_rate: f32) -> MasterDsp {
        let sample_rate = if sample_rate.is_finite() && sample_rate > 1_000.0 { sample_rate } else { 48_000.0 };
        MasterDsp {
            comp: CompDsp { sample_rate, gr_db: 0.0, makeup: 1.0, idle: true },
            eq: EqDsp { c: EqCoeffs::FLAT, z: [[[0.0; 2]; 2]; EQ_BANDS] },
            eq_seen: u32::MAX,
        }
    }

    /// Run the chain on a stereo block in place, at `ctl`'s settings. Both off: the block
    /// is left as it is.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], ctl: &MasterControl) {
        if let Some(c) = ctl.eq.read(&mut self.eq_seen) {
            self.eq.set(&c);
        }
        self.comp.process(left, right, ctl);
        if self.eq.c.on != 0 {
            self.eq.process(left, right);
        }
    }

    /// The compressor's gain change now, dB (0 or less): the Genos's "GR".
    pub fn gain_reduction_db(&self) -> f32 {
        self.comp.gr_db
    }

    /// Whether anything runs: the compressor (on, or gliding back) or an EQ band.
    pub fn active(&self) -> bool {
        !self.comp.idle || self.eq.c.on != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48_000.0;

    fn sine(hz: f32, amp: f32, n: usize) -> Vec<f32> {
        (0..n).map(|i| (2.0 * std::f32::consts::PI * hz * i as f32 / SR).sin() * amp).collect()
    }

    fn noise(n: usize, amp: f32) -> Vec<f32> {
        let mut v = 0x1234_5678u32;
        (0..n)
            .map(|_| {
                v ^= v << 13;
                v ^= v >> 17;
                v ^= v << 5;
                (v as f32 / u32::MAX as f32 * 2.0 - 1.0) * amp
            })
            .collect()
    }

    fn rms(v: &[f32]) -> f32 {
        (v.iter().map(|s| s * s).sum::<f32>() / v.len().max(1) as f32).sqrt()
    }

    /// Run `x` (both sides) through the chain in 128-frame buffers.
    fn run(dsp: &mut MasterDsp, ctl: &MasterControl, x: &[f32]) -> (Vec<f32>, Vec<f32>) {
        let (mut l, mut r) = (x.to_vec(), x.to_vec());
        for (a, b) in l.chunks_mut(128).zip(r.chunks_mut(128)) {
            dsp.process(a, b, ctl);
        }
        (l, r)
    }

    fn ctl_at(rate: f32) -> MasterControl {
        let c = MasterControl::new();
        c.set_sample_rate(rate);
        c
    }

    #[test]
    fn both_off_leave_the_audio_bit_identical() {
        let x = noise(48_000, 0.9);
        let ctl = ctl_at(SR);
        let mut dsp = MasterDsp::new(SR);
        assert_eq!(run(&mut dsp, &ctl, &x), (x.clone(), x.clone()));
        assert!(!dsp.active());
        // Settings changed while off don't run either: a type, parameters, EQ bands.
        let comp = MasterComp { on: false, ..MasterComp::of(false, CompPreset::Loud) };
        ctl.set_compressor(&comp);
        ctl.set_eq(&MasterEq { on: false, preset: EqPreset::Loudness, bands: EqPreset::Loudness.bands() });
        assert_eq!(run(&mut dsp, &ctl, &x), (x.clone(), x.clone()));
        // An EQ on at Flat runs no band.
        ctl.set_eq(&MasterEq { on: true, ..MasterEq::default() });
        assert_eq!(run(&mut dsp, &ctl, &x), (x.clone(), x.clone()));
        // On, then off again: once it has glided back, the audio is untouched again.
        ctl.set_compressor(&MasterComp::of(true, CompPreset::Punchy));
        ctl.set_eq(&MasterEq { on: true, preset: EqPreset::Bright, bands: EqPreset::Bright.bands() });
        assert_ne!(run(&mut dsp, &ctl, &x).0, x);
        ctl.set_compressor(&MasterComp::of(false, CompPreset::Punchy));
        ctl.set_eq(&MasterEq { on: false, preset: EqPreset::Bright, bands: EqPreset::Bright.bands() });
        for _ in 0..4 {
            run(&mut dsp, &ctl, &x);
        }
        assert!(!dsp.active(), "settled");
        assert_eq!(run(&mut dsp, &ctl, &x), (x.clone(), x));
    }

    #[test]
    fn the_compressor_reduces_a_loud_signal_and_leaves_a_quiet_one() {
        let ctl = ctl_at(SR);
        // Output 0 dB, to see the gain change alone.
        ctl.set_compressor(&MasterComp { output: 0, ..MasterComp::of(true, CompPreset::Loud) });
        let mut dsp = MasterDsp::new(SR);
        let loud = sine(440.0, 0.9, 48_000);
        let (l, r) = run(&mut dsp, &ctl, &loud);
        let db = 20.0 * (rms(&l[24_000..]) / rms(&loud[24_000..])).log10();
        assert!(db < -6.0, "a loud signal comes down: {db} dB");
        assert_eq!(l, r, "linked: both sides alike");
        assert!(dsp.gain_reduction_db() < -6.0);
        // Well under the threshold (-26 dBFS for Loud): untouched, near enough.
        let mut dsp = MasterDsp::new(SR);
        let quiet = sine(440.0, 0.005, 48_000);
        let (l, _) = run(&mut dsp, &ctl, &quiet);
        let db = 20.0 * (rms(&l[24_000..]) / rms(&quiet[24_000..])).log10();
        assert!(db.abs() < 0.05, "a quiet one passes: {db} dB");
        // More Compression, more gain change.
        let at = |c: u8| {
            ctl.set_compressor(&MasterComp { compression: c, output: 0, ..MasterComp::of(true, CompPreset::Natural) });
            let mut dsp = MasterDsp::new(SR);
            let (l, _) = run(&mut dsp, &ctl, &loud);
            rms(&l[24_000..])
        };
        assert!(at(0) > at(40) && at(40) > at(90), "{} {} {}", at(0), at(40), at(90));
        assert!((at(0) / rms(&loud[24_000..]) - 1.0).abs() < 1e-4, "0% compresses nothing");
    }

    #[test]
    fn the_output_sets_the_level_after_it_and_glides() {
        let ctl = ctl_at(SR);
        ctl.set_compressor(&MasterComp { compression: 0, output: 6, ..MasterComp::of(true, CompPreset::Natural) });
        let mut dsp = MasterDsp::new(SR);
        let x = sine(440.0, 0.1, 4_800);
        let (l, _) = run(&mut dsp, &ctl, &x);
        let db = 20.0 * (rms(&l[2_400..]) / rms(&x[2_400..])).log10();
        assert!((db - 6.0).abs() < 0.05, "{db}");
        // The first buffer ramps up rather than jumping.
        let early = l[10] / x[10];
        assert!(early > 1.0 && early < 1.2, "{early}");
    }

    #[test]
    fn texture_sets_how_fast_it_lets_go() {
        // A loud burst then quiet: a light texture recovers sooner than a heavy one.
        let mut x = sine(440.0, 0.9, 24_000);
        x.extend(sine(440.0, 0.05, 24_000));
        let recovered = |texture: u8| {
            let ctl = ctl_at(SR);
            ctl.set_compressor(&MasterComp { compression: 80, texture, output: 0, ..MasterComp::of(true, CompPreset::Natural) });
            let mut dsp = MasterDsp::new(SR);
            let (l, _) = run(&mut dsp, &ctl, &x);
            rms(&l[24_000 + 2_400..24_000 + 4_800]) / rms(&x[24_000 + 2_400..24_000 + 4_800])
        };
        assert!(recovered(100) > recovered(0) * 1.2, "{} vs {}", recovered(100), recovered(0));
    }

    /// The gain in dB the EQ gives a sine at `hz`.
    fn eq_gain_db(eq: MasterEq, hz: f32) -> f32 {
        let ctl = ctl_at(SR);
        ctl.set_eq(&eq);
        let mut dsp = MasterDsp::new(SR);
        let x = sine(hz, 0.25, 48_000);
        let (l, _) = run(&mut dsp, &ctl, &x);
        20.0 * (rms(&l[24_000..]) / rms(&x[24_000..])).log10()
    }

    #[test]
    fn each_band_boosts_and_cuts_at_its_frequency() {
        let mut eq = MasterEq { on: true, ..MasterEq::default() };
        eq.bands[3] = EqBand { gain: 9, freq: 1_000, q: 20, shelf: false };
        assert!((eq_gain_db(eq, 1_000.0) - 9.0).abs() < 0.3, "{}", eq_gain_db(eq, 1_000.0));
        assert!(eq_gain_db(eq, 100.0).abs() < 0.3, "narrow: far away untouched");
        eq.bands[3].gain = -12;
        assert!((eq_gain_db(eq, 1_000.0) + 12.0).abs() < 0.3, "{}", eq_gain_db(eq, 1_000.0));
        // The edge bands as shelves: all below (above) the corner.
        let mut eq = MasterEq { on: true, ..MasterEq::default() };
        eq.bands[0] = EqBand { gain: 6, freq: 200, q: 7, shelf: true };
        eq.bands[7] = EqBand { gain: -6, freq: 4_000, q: 7, shelf: true };
        assert!((eq_gain_db(eq, 40.0) - 6.0).abs() < 0.4, "{}", eq_gain_db(eq, 40.0));
        assert!((eq_gain_db(eq, 15_000.0) + 6.0).abs() < 0.6, "{}", eq_gain_db(eq, 15_000.0));
        assert!(eq_gain_db(eq, 1_000.0).abs() < 0.5, "{}", eq_gain_db(eq, 1_000.0));
        // Off: nothing, whatever the bands.
        eq.on = false;
        assert_eq!(eq_gain_db(eq, 40.0), 0.0);
    }

    #[test]
    fn the_types_shape_the_tone_as_named() {
        let at = |p: EqPreset, hz: f32| eq_gain_db(MasterEq { on: true, preset: p, bands: p.bands() }, hz);
        assert!(at(EqPreset::Mellow, 12_000.0) < -2.0);
        assert!(at(EqPreset::Bright, 12_000.0) > 2.0);
        assert!(at(EqPreset::Loudness, 40.0) > 2.0 && at(EqPreset::Loudness, 12_000.0) > 2.0);
        assert!(at(EqPreset::Powerful, 1_000.0) > 0.5);
        assert!(at(EqPreset::Flat, 1_000.0) == 0.0);
    }

    #[test]
    fn settings_clamp_and_round_trip_through_the_cell() {
        let ctl = MasterControl::new();
        let mut eq = MasterEq { on: true, ..MasterEq::default() };
        eq.bands[0] = EqBand { gain: 40, freq: 5, q: 0, shelf: true };
        eq.bands[3] = EqBand { gain: -40, freq: 60_000, q: 200, shelf: true };
        ctl.set_eq(&eq);
        let (on, bands) = ctl.eq();
        assert!(on);
        assert_eq!(bands[0], EqBand { gain: 12, freq: 32, q: 1, shelf: true });
        assert_eq!(bands[3], EqBand { gain: -12, freq: 10_000, q: 120, shelf: false }, "a middle band is never a shelf");
        assert_eq!(bands[7], EqBand { gain: 0, freq: 8_000, q: 7, shelf: true });
        let mut seen = u32::MAX;
        let c = ctl.eq.read(&mut seen).unwrap();
        assert_eq!(c.on, 0b1001);
        assert_eq!(ctl.eq.read(&mut seen), None, "nothing new");
        ctl.set_eq(&eq);
        assert_eq!(ctl.eq.read(&mut seen), None, "the same again publishes nothing");
        let comp = MasterComp { on: true, preset: CompPreset::Rich, compression: 200, texture: 150, output: 90 }.clamped();
        assert_eq!((comp.compression, comp.texture, comp.output), (100, 100, 12));
    }

    #[test]
    fn a_missing_field_reads_as_its_default() {
        let s: MasterSettings = serde_json::from_str(r#"{"compressor":{"on":true}}"#).unwrap();
        assert_eq!(s.compressor, MasterComp::of(true, CompPreset::Natural));
        assert_eq!(s.eq, MasterEq::default());
        let s: MasterSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(s, MasterSettings::default());
        assert!(!MasterComp::default().edited() && !MasterEq::default().edited());
    }
}

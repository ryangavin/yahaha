//! The shared effect bus (#204): Genos-style System Effects on the audio thread.
//!
//! Every part feeds the bus through its own sends, SoundFont and plugin parts alike: the
//! Reverb block from CC91, the Chorus block from CC93 (and, from the Variation block,
//! CC94). A send taps the part's dry mix after its volume, expression and pan (post
//! fader, as on the Genos), so a part's sends follow its fader. Each block's output comes
//! back into the mix at its return level (Genos: 0-127, 64 = 0 dB, 127 = +6 dB), before
//! the master fader's clipper.
//!
//! The SoundFont synthesizers' own reverb and chorus are off: the bus replaces them
//! (`Synthesizer::set_internal_effects`), so a send is not heard twice. `FxControl::legacy`
//! turns them back on and the bus off, for a before/after comparison (`YAHAHA_FX=legacy
//! yahaha render ...`).
//!
//! The blocks: Reverb ([`Reverb`], Hall/Room/Stage/Plate), Chorus ([`Chorus`]), and
//! Variation, a stereo delay synced to the style tempo ([`Delay`]: 1/8, dotted 1/8, 1/4,
//! ping-pong). The control side keeps `FxControl::tempo` at the style's tempo.
//!
//! [`FxBus`] allocates everything in [`FxBus::new`]; [`FxBus::process_add`] never
//! allocates, locks or blocks (`tests/synth_no_alloc.rs`). A block with no input whose
//! output has died away is skipped, so an idle bus costs next to nothing.

use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering::Relaxed};

mod chorus;
mod delay;
mod line;
mod reverb;

pub use chorus::{Chorus, ChorusType};
pub use delay::{Delay, DelayType};
pub use reverb::{Reverb, ReverbType};

/// The send buses: Reverb (CC91), Chorus (CC93), Variation (CC94).
pub const BUSES: usize = rustysynth::SEND_BUSES;
pub const REVERB: usize = 0;
pub const CHORUS: usize = 1;
pub const VARIATION: usize = 2;

/// The controller that sends a part to each bus.
pub const SEND_CC: [u8; BUSES] = [91, 93, 94];

/// A part's sends before any controller sets them: GM's (reverb 40, chorus 0) and no
/// Variation.
pub const DEFAULT_SENDS: [u8; BUSES] = [40, 0, 0];

/// A return level at unity gain (Genos: 64 = 0 dB).
pub const RETURN_UNITY: u8 = 64;

/// The gain of a send controller value: linear, 127 = the whole dry signal.
#[inline]
pub fn send_gain(cc: u8) -> f32 {
    cc.min(127) as f32 / 127.0
}

/// The gain of a return level: linear, 64 = 0 dB, 127 = +6 dB, 0 = off (Genos, DL p.130).
#[inline]
pub fn return_gain(level: u8) -> f32 {
    level.min(127) as f32 / RETURN_UNITY as f32
}

/// The bus's settings, turned by the control side and read by the audio thread once per
/// buffer: plain atomics.
pub struct FxControl {
    pub reverb_type: AtomicU8,
    pub reverb_return: AtomicU8,
    pub chorus_type: AtomicU8,
    pub chorus_return: AtomicU8,
    pub variation_type: AtomicU8,
    pub variation_return: AtomicU8,
    /// The style tempo the delay follows: BPM x 100.
    pub tempo: AtomicU32,
    /// The SoundFont's own reverb and chorus instead of the bus (the sound before #204).
    pub legacy: AtomicBool,
}

impl FxControl {
    pub fn new() -> FxControl {
        FxControl {
            reverb_type: AtomicU8::new(ReverbType::Hall as u8),
            reverb_return: AtomicU8::new(RETURN_UNITY),
            chorus_type: AtomicU8::new(ChorusType::Chorus as u8),
            chorus_return: AtomicU8::new(RETURN_UNITY),
            variation_type: AtomicU8::new(DelayType::DottedEighth as u8),
            variation_return: AtomicU8::new(RETURN_UNITY),
            tempo: AtomicU32::new(12_000),
            legacy: AtomicBool::new(false),
        }
    }
}

impl FxControl {
    /// Follow the style tempo (BPM).
    pub fn set_tempo(&self, bpm: f64) {
        self.tempo.store((bpm.clamp(1.0, 1000.0) * 100.0).round() as u32, Relaxed);
    }
}

impl Default for FxControl {
    fn default() -> Self {
        FxControl::new()
    }
}

/// Output below this (linear) with no input: the block has died away.
const IDLE_LEVEL: f32 = 1e-5;

/// One block's idle tracking and return gain ramp.
struct Block {
    /// The block has had no input and rendered nothing audible for `hold` frames: skip
    /// it until input comes.
    idle: bool,
    quiet: u32,
    /// The longest gap in its output while it still rings (the delay's silence between
    /// two repeats).
    hold: u32,
    gain: f32,
}

impl Block {
    fn new(hold: f32, rate: f32) -> Block {
        Block { idle: true, quiet: 0, hold: (hold * rate) as u32, gain: 0.0 }
    }
}

/// The effect blocks and their state (audio thread).
pub struct FxBus {
    reverb: Reverb,
    chorus: Chorus,
    delay: Delay,
    blocks: [Block; BUSES],
}

impl FxBus {
    /// The bus for `sample_rate` (allocates its delay lines: call it off the audio thread).
    pub fn new(sample_rate: u32) -> FxBus {
        let rate = sample_rate.max(8000) as f32;
        FxBus {
            reverb: Reverb::new(rate),
            chorus: Chorus::new(rate),
            delay: Delay::new(rate),
            blocks: [Block::new(0.1, rate), Block::new(0.05, rate), Block::new(delay::MAX_SECONDS + 0.1, rate)],
        }
    }

    /// Run the blocks on `n` frames of send buses (`sends`: bus b's left side at
    /// `2 * b * n`, its right side at `(2 * b + 1) * n`) and **add** their returns into
    /// `left` / `right`. RT-safe.
    pub fn process_add(&mut self, sends: &[f32], n: usize, left: &mut [f32], right: &mut [f32], ctl: &FxControl) {
        let n = n.min(left.len()).min(right.len()).min(sends.len() / (2 * BUSES));
        if n == 0 {
            return;
        }
        let bus = |b: usize| (&sends[2 * b * n..(2 * b + 1) * n], &sends[(2 * b + 1) * n..(2 * b + 2) * n]);
        self.reverb.set_type(ReverbType::from_u8(ctl.reverb_type.load(Relaxed)));
        self.chorus.set_type(ChorusType::from_u8(ctl.chorus_type.load(Relaxed)));
        let bpm = ctl.tempo.load(Relaxed) as f32 / 100.0;
        self.delay.set(DelayType::from_u8(ctl.variation_type.load(Relaxed)), bpm);
        let returns = [ctl.reverb_return.load(Relaxed), ctl.chorus_return.load(Relaxed), ctl.variation_return.load(Relaxed)];
        for (b, block) in self.blocks.iter_mut().enumerate() {
            let (il, ir) = bus(b);
            let input = il.iter().chain(ir).any(|x| *x != 0.0);
            let target = return_gain(returns[b]);
            if block.idle {
                // Waking up: no ramp from a stale gain.
                block.gain = target;
                if !input {
                    continue;
                }
            }
            let (g0, dg) = (block.gain, (target - block.gain) / n as f32);
            block.gain = target;
            let mut peak = 0f32;
            for k in 0..n {
                let (wl, wr) = match b {
                    REVERB => self.reverb.tick(il[k], ir[k]),
                    CHORUS => self.chorus.tick(il[k], ir[k]),
                    _ => self.delay.tick(il[k], ir[k]),
                };
                peak = peak.max(wl.abs()).max(wr.abs());
                let g = g0 + dg * (k + 1) as f32;
                left[k] += wl * g;
                right[k] += wr * g;
            }
            block.quiet = if input || peak >= IDLE_LEVEL { 0 } else { block.quiet.saturating_add(n as u32) };
            block.idle = block.quiet > block.hold;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(bus: &mut FxBus, ctl: &FxControl, input: impl Fn(usize) -> [f32; 2], b: usize, frames: usize) -> (Vec<f32>, Vec<f32>) {
        let n = 64;
        let (mut l, mut r) = (Vec::new(), Vec::new());
        let mut sends = vec![0f32; 2 * BUSES * n];
        let mut done = 0;
        while done < frames {
            sends.fill(0.0);
            for k in 0..n {
                let [a, c] = input(done + k);
                sends[2 * b * n + k] = a;
                sends[(2 * b + 1) * n + k] = c;
            }
            let (mut ol, mut or) = (vec![0f32; n], vec![0f32; n]);
            bus.process_add(&sends, n, &mut ol, &mut or, ctl);
            l.extend(ol);
            r.extend(or);
            done += n;
        }
        (l, r)
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    /// A deterministic noise source.
    fn noise(seed: u32) -> impl Fn(usize) -> [f32; 2] {
        move |i| {
            let h = |x: u32| {
                let mut v = x.wrapping_mul(0x9E37_79B9) ^ seed;
                v ^= v >> 15;
                v = v.wrapping_mul(0x85EB_CA6B);
                v ^= v >> 13;
                (v as f32 / u32::MAX as f32) * 2.0 - 1.0
            };
            [h(2 * i as u32) * 0.25, h(2 * i as u32 + 1) * 0.25]
        }
    }

    #[test]
    fn return_levels_follow_the_genos_scale() {
        assert_eq!(return_gain(0), 0.0);
        assert_eq!(return_gain(64), 1.0);
        assert!((20.0 * return_gain(127).log10() - 6.0).abs() < 0.1, "127 = +6 dB");
        assert_eq!(send_gain(127), 1.0);
    }

    /// Each reverb type rings on after its input stops, for about its decay time, and
    /// then dies away; every type sits at a similar level, and the two sides differ
    /// (a stereo image, not two copies).
    #[test]
    fn every_reverb_type_rings_and_dies_away() {
        let mut levels = Vec::new();
        for t in [ReverbType::Hall, ReverbType::Room, ReverbType::Stage, ReverbType::Plate] {
            let ctl = FxControl::new();
            ctl.reverb_type.store(t as u8, Relaxed);
            let mut bus = FxBus::new(48_000);
            let src = noise(1);
            let (l, r) = run(&mut bus, &ctl, |i| if i < 24_000 { src(i) } else { [0.0; 2] }, REVERB, 48_000 * 8);
            let steady = rms(&l[12_000..24_000]);
            levels.push(steady);
            let after = |ms: usize| rms(&l[24_000 + ms * 48..24_000 + (ms + 50) * 48]);
            // RT60: 60 dB down after the decay time. At 0.5 s the tail is still there;
            // long after its decay time it is gone.
            let rt = t.rt60();
            assert!(after(100) > steady * 0.05, "{t:?} rings on");
            let gone = ((rt * 1.6) * 1000.0) as usize;
            assert!(after(gone) < steady * 0.01, "{t:?} dies away ({} vs {steady})", after(gone));
            let corr = l[12_000..24_000].iter().zip(&r[12_000..24_000]).map(|(a, b)| a * b).sum::<f32>()
                / (rms(&l[12_000..24_000]) * rms(&r[12_000..24_000]) * 12_000.0);
            assert!(corr.abs() < 0.5, "{t:?}: the sides are decorrelated ({corr})");
            assert!(l.iter().chain(&r).all(|x| x.is_finite() && x.abs() < 4.0));
        }
        eprintln!("reverb levels {levels:?}");
        let (lo, hi) = levels.iter().fold((f32::MAX, 0f32), |(a, b), &x| (a.min(x), b.max(x)));
        assert!(hi / lo < 2.0, "the types sit at similar levels: {levels:?}");
        // Wet about as loud as a fully sent dry signal (0.25 peak noise: RMS ~0.144).
        assert!(levels.iter().all(|&x| (0.03..0.3).contains(&x)), "{levels:?}");
    }

    /// No input: the bus adds nothing and does no work. The return level scales the wet
    /// signal (0 = off).
    #[test]
    fn silence_stays_silent_and_the_return_scales_the_wet() {
        let ctl = FxControl::new();
        let mut bus = FxBus::new(48_000);
        let (l, r) = run(&mut bus, &ctl, |_| [0.0; 2], REVERB, 4800);
        assert!(l.iter().chain(&r).all(|x| *x == 0.0));
        assert!(bus.blocks.iter().all(|b| b.idle));

        let src = noise(7);
        let level = |ret: u8, b: usize| {
            let ctl = FxControl::new();
            ctl.reverb_return.store(ret, Relaxed);
            ctl.chorus_return.store(ret, Relaxed);
            ctl.variation_return.store(ret, Relaxed);
            let mut bus = FxBus::new(48_000);
            let (l, _) = run(&mut bus, &ctl, &src, b, 24_000);
            rms(&l[12_000..])
        };
        for b in [REVERB, CHORUS, VARIATION] {
            let unity = level(64, b);
            assert!(unity > 0.01, "bus {b} sounds");
            assert_eq!(level(0, b), 0.0, "bus {b} return 0 = off");
            assert!((level(127, b) / unity - 127.0 / 64.0).abs() < 0.02, "bus {b} +6 dB");
        }
    }

    /// The Variation block's delay follows the tempo the control side sets: a click
    /// comes back a dotted 1/8 later (the default type), at 120 and at 100 BPM, and keeps
    /// repeating across buffers with silence between the repeats.
    #[test]
    fn the_delay_follows_the_tempo() {
        for (bpm, want) in [(120.0, 18_000), (100.0, 21_600)] {
            let ctl = FxControl::new();
            ctl.set_tempo(bpm);
            let mut bus = FxBus::new(48_000);
            let (l, _) = run(&mut bus, &ctl, |i| if i == 0 { [1.0, 1.0] } else { [0.0; 2] }, VARIATION, 48_000 * 3);
            let loud: Vec<usize> = (0..l.len()).filter(|&i| l[i].abs() > 0.05).collect();
            assert!(loud.first().is_some_and(|&i| i.abs_diff(want) <= 2), "{bpm}: {:?}", &loud[..loud.len().min(4)]);
            assert!(loud.iter().any(|&i| i.abs_diff(2 * want) <= 4), "{bpm}: a second repeat");
        }
    }

    /// The chorus is a modulated copy of its input: as loud as it, and moving.
    #[test]
    fn every_chorus_type_sounds() {
        for t in [ChorusType::Chorus, ChorusType::Celeste, ChorusType::Flanger] {
            let ctl = FxControl::new();
            ctl.chorus_type.store(t as u8, Relaxed);
            let mut bus = FxBus::new(48_000);
            let tone = |i: usize| {
                let x = (i as f32 * 440.0 * std::f32::consts::TAU / 48_000.0).sin() * 0.3;
                [x, x]
            };
            let (l, r) = run(&mut bus, &ctl, tone, CHORUS, 48_000);
            let level = rms(&l[4800..]);
            assert!((0.08..0.5).contains(&level), "{t:?}: {level}");
            assert!(l[4800..].iter().zip(&r[4800..]).any(|(a, b)| (a - b).abs() > 0.01), "{t:?}: stereo");
            assert!(l.iter().chain(&r).all(|x| x.is_finite()));
        }
    }

    /// The bus's CPU cost: reverb, chorus and delay all running on 10 s at 48 kHz, in 64-frame
    /// buffers (`cargo test --release --lib fx::tests::cost -- --ignored --nocapture`).
    #[test]
    #[ignore]
    fn cost() {
        let ctl = FxControl::new();
        let mut bus = FxBus::new(48_000);
        let n = 64;
        let src = noise(3);
        let mut sends = vec![0f32; 2 * BUSES * n];
        for k in 0..n {
            let [a, c] = src(k);
            for b in 0..BUSES {
                sends[2 * b * n + k] = a;
                sends[(2 * b + 1) * n + k] = c;
            }
        }
        let (mut l, mut r) = (vec![0f32; n], vec![0f32; n]);
        let buffers = 48_000 * 10 / n;
        let t = std::time::Instant::now();
        for _ in 0..buffers {
            bus.process_add(&sends, n, &mut l, &mut r, &ctl);
        }
        let s = t.elapsed().as_secs_f64();
        eprintln!("fx bus: {:.1} ms per 10 s of audio ({:.2}% of real time, {:.1} us per 64-frame buffer)", s * 1e3, s * 10.0, s * 1e6 / buffers as f64);
    }
}

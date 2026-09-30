//! Golden tests: the refactors of #464 (8826f593, "Mixer strips lane A") left the default
//! sound bit-identical, and a channel's strip send reaches send 4 end to end.
//!
//! Each golden test holds a reference copy of the code as it was before #464
//! (`git show 8826f593^:crates/yahaha-fx/src/fx/insert.rs` and `.../fx/master.rs`),
//! trimmed to the process loop, and runs it next to today's code on the same input,
//! asserting every output sample is bit-for-bit the same. The reference copies must never
//! be "fixed" to follow today's code: they are the old sound.

use std::f32::consts::TAU;
use std::sync::atomic::Ordering::Relaxed;

use super::master::{MasterComp, MasterControl, MasterDsp, comp_curve, comp_times, CompPreset, COMP_OUTPUT_DB};
use super::{BUSES, FxBus, FxControl, Insert, InsertEffect, InsertKind, InsertSettings, PartInsert, SENDS, SendKind, SendSlot};

const RATE: f32 = 48_000.0;

/// A deterministic stereo test signal: a sine and noise, loud and soft by turns (so the
/// compressor and wah move), the two sides different.
fn input(n: usize) -> (Vec<f32>, Vec<f32>) {
    let mut seed = 0x2545_f491u32;
    let mut noise = move || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (seed >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0
    };
    let (mut l, mut r) = (Vec::with_capacity(n), Vec::with_capacity(n));
    for i in 0..n {
        let env = if (i / 1200) % 3 == 0 { 0.9 } else { 0.08 };
        let t = i as f32 / RATE;
        l.push(env * (0.6 * (TAU * 220.0 * t).sin() + 0.3 * noise()));
        r.push(env * (0.6 * (TAU * 331.0 * t).sin() + 0.3 * noise()));
    }
    (l, r)
}

/// Buffer sizes the tests cycle through: a buffer boundary anywhere.
const BUFS: [usize; 5] = [64, 128, 37, 256, 1];

// ---------------------------------------------------------------------------
// Inserts: the pre-#464 `Insert` (8826f593^:crates/yahaha-fx/src/fx/insert.rs)
// ---------------------------------------------------------------------------

mod old_insert {
    use super::{InsertKind, TAU};

    const FADE_S: f32 = 0.02;
    const ROTARY_LINE_S: f32 = 0.004;
    const HORN_HZ: [f32; 2] = [0.8, 6.7];
    const DRUM_HZ: [f32; 2] = [0.67, 5.7];
    const ROTARY_UP_S: f32 = 1.0;
    const ROTARY_DOWN_S: f32 = 2.0;

    fn one_pole(hz: f32, rate: f32) -> f32 {
        1.0 - (-TAU * hz / rate).exp()
    }

    fn time_coef(s: f32, rate: f32) -> f32 {
        1.0 - (-1.0 / (s * rate)).exp()
    }

    #[inline]
    fn shape(x: f32) -> f32 {
        x / (1.0 + x.abs())
    }

    /// The old `InsertSettings`: one amount, no other settings.
    #[derive(Clone, Copy)]
    pub struct Settings {
        pub kind: InsertKind,
        pub amount: u8,
        pub bpm: f32,
        pub fast: bool,
    }

    pub struct Insert {
        rate: f32,
        kind: InsertKind,
        mix: f32,
        fade: f32,
        lo: [f32; 2],
        hi: [f32; 2],
        svf: [[f32; 2]; 2],
        env: f32,
        phase: [f32; 2],
        line: [Vec<f32>; 2],
        pos: usize,
        spin: f32,
        spin_up: f32,
        spin_down: f32,
    }

    impl Insert {
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

        pub fn process(&mut self, left: &mut [f32], right: &mut [f32], level: f32, s: &Settings) {
            let n = left.len().min(right.len());
            let g = if level > 1e-4 { level } else { 1.0 };
            let (inv, amount) = (1.0 / g, s.amount.min(127) as f32 / 127.0);
            let rate = self.rate;
            let pre = 1.0 + 39.0 * amount * amount;
            let post = 1.0 / shape(pre).max(0.05) * 0.7;
            let lo_c = one_pole(90.0, rate);
            let hi_c = one_pole(5500.0 - 2500.0 * amount, rate);
            let thr = 10f32.powf((-12.0 - 18.0 * amount) / 20.0);
            let (att, rel) = (time_coef(0.003, rate), time_coef(0.15, rate));
            let makeup = (1.0 / thr).powf(0.75 * 0.5);
            let (wa, wr) = (time_coef(0.004, rate), time_coef(0.09, rate));
            let sens = 2.0 + 10.0 * amount;
            let trem_hz = (s.bpm.clamp(20.0, 400.0) / 60.0 * 2.0).min(12.0);
            let depth = 0.2 + 0.6 * amount;
            let line_len = self.line[0].len();
            for k in 0..n {
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
                    // Before #464 the phaser played dry; it has its own DSP now, so it is
                    // not compared.
                    InsertKind::None | InsertKind::Phaser => {}
                }
                let m = self.mix;
                left[k] = dl + (y[0] * g - dl) * m;
                right[k] = dr + (y[1] * g - dr) * m;
            }
        }
    }
}

/// One step of an insert run: this many frames at these settings.
#[derive(Clone, Copy)]
struct Step {
    frames: usize,
    kind: InsertKind,
    amount: u8,
    fast: bool,
}

/// Run `steps` through today's `Insert` (its settings as a keyboard slot gives them:
/// `InsertSettings::with(PartInsert)`, every other setting at the kind's default) and
/// through the pre-#464 one, on the same input at part level `level`, and assert every
/// sample is bit-identical.
fn assert_insert_matches(steps: &[Step], level: f32, bpm: f32) {
    let total: usize = steps.iter().map(|s| s.frames).sum();
    let (il, ir) = input(total);
    let (mut nl, mut nr): (Vec<f32>, Vec<f32>) = (il.iter().map(|x| x * level).collect(), ir.iter().map(|x| x * level).collect());
    let (mut ol, mut or) = (nl.clone(), nr.clone());
    let mut new = Insert::new(RATE);
    let mut old = old_insert::Insert::new(RATE);
    let mut at = 0;
    let mut b = 0;
    for st in steps {
        let slot = PartInsert { effect: InsertEffect::from(st.kind), on: st.kind != InsertKind::None, amount: st.amount };
        let s = InsertSettings { bpm, fast: st.fast, ..InsertSettings::NONE }.with(slot);
        assert_eq!(s.kind, st.kind);
        let os = old_insert::Settings { kind: st.kind, amount: st.amount, bpm, fast: st.fast };
        let end = at + st.frames;
        while at < end {
            let len = BUFS[b % BUFS.len()].min(end - at);
            b += 1;
            new.process(&mut nl[at..at + len], &mut nr[at..at + len], level, &s);
            old.process(&mut ol[at..at + len], &mut or[at..at + len], level, &os);
            at += len;
        }
    }
    for i in 0..total {
        assert!(
            nl[i].to_bits() == ol[i].to_bits() && nr[i].to_bits() == or[i].to_bits(),
            "{:?}: sample {i} differs: ({}, {}) now, ({}, {}) before #464",
            steps.iter().map(|s| (s.kind, s.amount, s.fast)).collect::<Vec<_>>(),
            nl[i],
            nr[i],
            ol[i],
            or[i]
        );
    }
    // The effect did something (a golden test of a pass-through would prove nothing).
    if steps.iter().any(|s| s.kind != InsertKind::None) {
        assert!(nl.iter().zip(&il).any(|(y, x)| (y - x * level).abs() > 1e-4), "{:?} changed nothing", steps[0].kind);
    }
}

/// Kind `kind` at its defaults (only its amount set), at a few amounts and part levels,
/// is bit-identical to the pre-#464 insert.
fn kind_matches(kind: InsertKind, frames: usize) {
    for amount in [0, 64, 127] {
        for level in [1.0, 0.3] {
            assert_insert_matches(&[Step { frames, kind, amount, fast: false }], level, 120.0);
        }
    }
}

#[test]
fn insert_none_matches_pre_464() {
    let (il, ir) = input(4800);
    let (mut l, mut r) = (il.clone(), ir.clone());
    let mut new = Insert::new(RATE);
    new.process(&mut l, &mut r, 0.5, &InsertSettings::NONE);
    assert_eq!((l, r), (il, ir));
}

#[test]
fn insert_distortion_matches_pre_464() {
    kind_matches(InsertKind::Distortion, 4800);
}

#[test]
fn insert_compressor_matches_pre_464() {
    kind_matches(InsertKind::Compressor, 4800);
}

#[test]
fn insert_auto_wah_matches_pre_464() {
    kind_matches(InsertKind::AutoWah, 4800);
}

#[test]
fn insert_tremolo_matches_pre_464() {
    kind_matches(InsertKind::Tremolo, 4800);
    // Another tempo: the rate follows it the same way.
    assert_insert_matches(&[Step { frames: 4800, kind: InsertKind::Tremolo, amount: 90, fast: false }], 1.0, 97.5);
}

#[test]
fn insert_rotary_matches_pre_464() {
    kind_matches(InsertKind::Rotary, 4800);
    // Speeding up and slowing down again.
    let rot = |fast| Step { frames: 4800, kind: InsertKind::Rotary, amount: 100, fast };
    assert_insert_matches(&[rot(true), rot(true), rot(false)], 0.7, 120.0);
}

/// Kind changes (each fading out and in over 20 ms), at one amount, match too.
#[test]
fn insert_kind_changes_match_pre_464() {
    let kinds = [InsertKind::Distortion, InsertKind::Tremolo, InsertKind::None, InsertKind::Compressor, InsertKind::Rotary, InsertKind::AutoWah];
    let steps: Vec<Step> = kinds.iter().map(|&kind| Step { frames: 1600, kind, amount: 90, fast: false }).collect();
    assert_insert_matches(&steps, 0.8, 120.0);
}

// ---------------------------------------------------------------------------
// The Master Compressor: the pre-#464 `CompDsp` (8826f593^:.../fx/master.rs)
// ---------------------------------------------------------------------------

mod old_master {
    use super::{COMP_OUTPUT_DB, MasterControl, Relaxed, comp_curve, comp_times};

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

    #[inline]
    fn db_gain(db: f32) -> f32 {
        (db * (std::f32::consts::LN_10 / 20.0)).exp()
    }

    pub struct CompDsp {
        pub sample_rate: f32,
        pub gr_db: f32,
        pub makeup: f32,
        pub idle: bool,
    }

    impl CompDsp {
        pub fn process(&mut self, left: &mut [f32], right: &mut [f32], ctl: &MasterControl) {
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
                self.gr_db = 0.0;
                self.makeup = 1.0;
                self.idle = true;
            }
        }
    }
}

/// The Master Compressor (EQ off) through a run of settings: off, each type, an Output
/// and a Compression change, off again (gliding back and going idle), on again. Every
/// sample, and the gain change after every buffer, is bit-identical to the pre-#464 one.
#[test]
fn master_compressor_matches_pre_464() {
    let ctl = MasterControl::new();
    ctl.set_sample_rate(RATE);
    let mut new = MasterDsp::new(RATE);
    let mut old = old_master::CompDsp { sample_rate: RATE, gr_db: 0.0, makeup: 1.0, idle: true };
    let settings: Vec<MasterComp> = {
        let mut v = vec![MasterComp::default()];
        v.extend(CompPreset::ALL.iter().map(|&p| MasterComp::of(true, p)));
        v.push(MasterComp { output: -7, ..MasterComp::of(true, CompPreset::Loud) });
        v.push(MasterComp { compression: 0, texture: 100, ..MasterComp::of(true, CompPreset::Natural) });
        v.push(MasterComp { compression: 100, texture: 0, output: 12, ..MasterComp::of(true, CompPreset::Natural) });
        v.push(MasterComp::of(false, CompPreset::Punchy));
        v.push(MasterComp::of(true, CompPreset::Punchy));
        v
    };
    let per = 3600;
    let (il, ir) = input(per * settings.len());
    let (mut nl, mut nr) = (il.clone(), ir.clone());
    let (mut ol, mut or) = (il.clone(), ir.clone());
    let (mut at, mut b) = (0, 0);
    let mut went_idle = false;
    for c in &settings {
        ctl.set_compressor(c);
        let end = at + per;
        while at < end {
            let len = BUFS[b % BUFS.len()].min(end - at);
            b += 1;
            new.process(&mut nl[at..at + len], &mut nr[at..at + len], &ctl);
            old.process(&mut ol[at..at + len], &mut or[at..at + len], &ctl);
            assert_eq!(new.gain_reduction_db().to_bits(), old.gr_db.to_bits(), "{c:?}: gain change after frame {}", at + len);
            assert_eq!(new.active(), !old.idle, "{c:?}: idle after frame {}", at + len);
            went_idle |= !c.on && old.idle;
            at += len;
        }
    }
    assert!(went_idle, "the off stretch glides back to idle");
    for i in 0..at {
        assert!(nl[i].to_bits() == ol[i].to_bits() && nr[i].to_bits() == or[i].to_bits(), "sample {i}: ({}, {}) now, ({}, {}) before #464", nl[i], nr[i], ol[i], or[i]);
    }
    assert!(nl.iter().zip(&il).any(|(y, x)| (y - x).abs() > 1e-3), "it compressed");
}

// ---------------------------------------------------------------------------
// A strip send reaches send 4
// ---------------------------------------------------------------------------

/// Channel `ch`'s stem (`stem`, both sides) through its strip sends to sends 4-6 (at
/// `FxControl::strip_send_gains`, as the synth's rack adds each stem into the send
/// buffers), then `FxBus::process_add_slots` on all [`SENDS`] slots: the returns.
fn strip_send_returns(ctl: &FxControl, ch: usize, stem: &[f32]) -> (Vec<f32>, Vec<f32>) {
    let n = 64;
    let mut bus = FxBus::new(RATE as u32);
    let gains = ctl.strip_send_gains();
    let mut sends = vec![0f32; 2 * SENDS * n];
    let (mut l, mut r) = (Vec::new(), Vec::new());
    for chunk in stem.chunks(n) {
        sends.fill(0.0);
        for (i, &g) in gains[ch].iter().enumerate() {
            let s = BUSES + i;
            for (k, x) in chunk.iter().enumerate() {
                sends[2 * s * n + k] += g * x;
                sends[(2 * s + 1) * n + k] += g * x;
            }
        }
        let (mut ol, mut or) = (vec![0f32; n], vec![0f32; n]);
        bus.process_add_slots(&sends, SENDS, n, &mut ol, &mut or, ctl);
        l.extend_from_slice(&ol[..chunk.len()]);
        r.extend_from_slice(&or[..chunk.len()]);
    }
    (l, r)
}

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
}

/// Channel 3's strip send to send 4 (slot 3) plays send 4's delay: audible, in
/// proportion to the send level, and nothing at all with no send to it (none, or only
/// to send 5, which has no kind).
#[test]
fn strip_send_drives_send_4() {
    const CH: usize = 3;
    let with = |send4: u8, send5: u8| {
        let ctl = FxControl::new();
        // Send 4: a 1/8 delay (250 ms at 120 BPM), its return at +3 dB-ish.
        ctl.sends[0].set(&SendSlot { return_level: 90, ..SendSlot::of(SendKind::Eighth) });
        ctl.strip_send[CH][0].store(send4, Relaxed);
        ctl.strip_send[CH][1].store(send5, Relaxed);
        // Another channel sends to send 4, but plays nothing here.
        ctl.strip_send[CH + 1][0].store(127, Relaxed);
        ctl
    };
    // A 50 ms burst, then silence: the delay's repeat comes back after it.
    let stem: Vec<f32> = (0..19_200).map(|i| if i < 2400 { 0.5 * (TAU * 440.0 * i as f32 / RATE).sin() } else { 0.0 }).collect();
    let (full, fr) = strip_send_returns(&with(127, 0), CH, &stem);
    let (quarter, _) = strip_send_returns(&with(32, 0), CH, &stem);
    assert!(full.iter().chain(&fr).all(|x| x.is_finite()));
    let (a, b) = (rms(&full), rms(&quarter));
    assert!(a > 0.01, "send 4 returns the channel: rms {a}");
    assert!((a / b - 127.0 / 32.0).abs() < 0.05, "the return follows the send level: {a} / {b}");
    // Nothing comes back until the delay's time has passed: it is the delay, not a leak.
    assert!(full[..2400].iter().all(|x| x.abs() < 1e-6), "no dry leak");
    // No send to send 4: exactly nothing, even with a send to send 5 (no kind there).
    let (none, nr) = strip_send_returns(&with(0, 127), CH, &stem);
    assert!(none.iter().chain(&nr).all(|x| *x == 0.0), "no send, no return");
}

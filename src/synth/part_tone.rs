//! A part's live tone controls (#346 step 3), played in yahaha on the part's stem and
//! through rustysynth's standard controllers, so they need nothing of rustysynth beyond
//! what upstream has:
//! - **CC74 cutoff, CC71 resonance:** a low-pass filter on the part's stem
//!   ([`StemFilter`]). It moves on the notes already sounding, gliding (no clicks). At
//!   CC74 64 and up it is out of the signal, so a part with its cutoff at 64 sounds exactly
//!   as without the filter, and it adds no gain. Above 64 a high shelf brightens the stem
//!   instead: 0 dB at 64, up to +6 dB at 127 above a one-pole split at 2.5 kHz, gliding
//!   the same way, with nothing added below the split. Below 64 its cutoff is 20 kHz x 2^((v -
//!   64) / 10), down to about 240 Hz at 0. (The vendored rustysynth scaled each voice's
//!   own cutoff, 16 steps an octave; from 20 kHz that scale hardly darkens a voice whose
//!   own filter is already low, so the stem's spans more.) CC71 raises (lowers) the
//!   filter's resonance by 0.2 dB a step, while CC74 is below 64.
//! - **CC77 vibrato depth:** added to the player's mod wheel (CC1), 2 steps of CC1 a step:
//!   rustysynth's mod wheel is 50 cents of vibrato at 127, so CC77 127 adds about half a
//!   semitone, as the vendored patch did. Below 64 it takes the mod wheel's vibrato away,
//!   not the voice's own.
//! - **CC78 vibrato delay:** above 64, each note-on holds the mod wheel's vibrato (CC1 and
//!   CC77) off for 20 ms a step, then fades it in over `DELAY_FADE_S`. Below 64 it does
//!   nothing (a voice's own vibrato delay can't be shortened from outside).
//! - **CC76 vibrato rate:** not played. Upstream rustysynth's vibrato runs at the voice's
//!   own rate, and CC1 only scales its depth.
//!
//! Everything is allocated with the rack; nothing here allocates, locks or panics.

/// The mod wheel, and the tone controllers played here.
const MODULATION: i32 = 1;
const RESONANCE: i32 = 71;
const CUTOFF: i32 = 74;
const VIBRATO_RATE: i32 = 76;
const VIBRATO_DEPTH: i32 = 77;
const VIBRATO_DELAY: i32 = 78;
const RESET_ALL_CONTROLLERS: i32 = 121;

/// Steps of CC74 an octave of cutoff.
const STEPS_PER_OCTAVE: f32 = 10.0;
/// The cutoff at CC74 64 (the top of the range; the filter is out of the signal there).
const TOP_HZ: f32 = 20_000.0;
/// The filter's resonance at CC71 64 (Butterworth: no peak), and dB a CC71 step.
const Q_NEUTRAL: f32 = std::f32::consts::FRAC_1_SQRT_2;
const RESONANCE_DB_PER_STEP: f32 = 0.2;
/// CC1 steps a CC77 step (CC1 127 is 50 cents of vibrato; CC77 +63, about 49).
const DEPTH_PER_STEP: i32 = 2;
/// Seconds of vibrato delay a CC78 step above 64, and how long the vibrato then fades in.
const DELAY_PER_STEP_S: f32 = 0.02;
const DELAY_FADE_S: f32 = 0.1;
/// The filter's settings glide to a change with this time constant (s).
const GLIDE_S: f32 = 0.01;
/// The high shelf at CC74 127 (dB above the split), and where it splits (Hz).
const SHELF_DB_AT_127: f32 = 6.0;
const SHELF_HZ: f32 = 2_500.0;
/// Frames between two updates of the filter's coefficients.
const SUB: usize = 16;

/// A stereo low-pass filter on a part's stem: a state-variable filter (the trapezoidal,
/// "zero-delay feedback" form, stable while its cutoff moves), mixed in from the dry
/// signal so it can come in and go out without a click.
pub(super) struct StemFilter {
    sample_rate: f32,
    /// log2 of the highest cutoff (`TOP_HZ`, kept below Nyquist).
    top: f32,
    /// Per-`SUB` glide factor towards the targets.
    glide: f32,
    /// Where the settings go: log2 of the cutoff (Hz), 1 / Q, the wet share (0: dry).
    to_oct: f32,
    to_k: f32,
    to_mix: f32,
    /// Where they are now.
    oct: f32,
    k: f32,
    mix: f32,
    /// The two integrators' state, per side.
    s1: [f32; 2],
    s2: [f32; 2],
    /// The high shelf (CC74 above 64): its one-pole split coefficient, the boost above the
    /// split as a linear gain minus 1 (target and now), and the split's low side, per side.
    shelf_a: f32,
    to_boost: f32,
    boost: f32,
    lp: [f32; 2],
}

impl StemFilter {
    pub(super) fn new(sample_rate: f32) -> StemFilter {
        let top = TOP_HZ.min(0.45 * sample_rate).log2();
        let glide = 1.0 - (-(SUB as f32) / (GLIDE_S * sample_rate)).exp();
        StemFilter {
            sample_rate,
            top,
            glide,
            to_oct: top,
            to_k: 1.0 / Q_NEUTRAL,
            to_mix: 0.0,
            oct: top,
            k: 1.0 / Q_NEUTRAL,
            mix: 0.0,
            s1: [0.0; 2],
            s2: [0.0; 2],
            shelf_a: 1.0 - (-2.0 * std::f32::consts::PI * SHELF_HZ.min(0.45 * sample_rate) / sample_rate).exp(),
            to_boost: 0.0,
            boost: 0.0,
            lp: [0.0; 2],
        }
    }

    /// Set CC74 and CC71 (7-bit). The filter glides there while it plays.
    pub(super) fn set(&mut self, cutoff: u8, resonance: u8) {
        if cutoff < 64 {
            self.to_oct = self.top + (cutoff as f32 - 64.0) / STEPS_PER_OCTAVE;
            let db = (resonance as f32 - 64.0) * RESONANCE_DB_PER_STEP;
            self.to_k = 1.0 / (Q_NEUTRAL * 10f32.powf(db / 20.0));
            self.to_mix = 1.0;
        } else {
            self.to_oct = self.top;
            self.to_k = 1.0 / Q_NEUTRAL;
            self.to_mix = 0.0;
        }
        let db = (cutoff as f32 - 64.0).max(0.0) * SHELF_DB_AT_127 / 63.0;
        self.to_boost = if db > 0.0 { 10f32.powf(db / 20.0) - 1.0 } else { 0.0 };
        if self.mix == 0.0 {
            // Out of the signal: no cutoff to glide from (the wet share still glides in).
            self.oct = self.to_oct;
            self.k = self.to_k;
        }
    }

    /// Whether it changes the signal now (it is in, or on its way in or out).
    #[inline]
    pub(super) fn active(&self) -> bool {
        self.mix > 0.0 || self.to_mix > 0.0 || self.boost > 0.0 || self.to_boost > 0.0
    }

    /// The part fell silent: forget what rang, and take the settings as they are.
    pub(super) fn clear(&mut self) {
        self.s1 = [0.0; 2];
        self.s2 = [0.0; 2];
        self.oct = self.to_oct;
        self.k = self.to_k;
        self.mix = self.to_mix;
        self.lp = [0.0; 2];
        self.boost = self.to_boost;
    }

    /// Filter the stem in place. Out of the signal, it leaves the stem untouched.
    pub(super) fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        if !self.active() {
            return;
        }
        let n = left.len().min(right.len());
        let mut i = 0;
        while i < n {
            let end = (i + SUB).min(n);
            self.oct += self.glide * (self.to_oct - self.oct);
            self.k += self.glide * (self.to_k - self.k);
            self.mix += self.glide * (self.to_mix - self.mix);
            if (self.mix - self.to_mix).abs() < 1e-3 {
                self.mix = self.to_mix;
            }
            self.boost += self.glide * (self.to_boost - self.boost);
            if (self.boost - self.to_boost).abs() < 1e-4 {
                self.boost = self.to_boost;
            }
            let hz = self.oct.exp2();
            let g = (std::f32::consts::PI * hz / self.sample_rate).tan();
            let a1 = 1.0 / (1.0 + g * (g + self.k));
            let a2 = g * a1;
            let a3 = g * a2;
            let mix = self.mix;
            let lowpass = mix > 0.0 || self.to_mix > 0.0;
            for (side, x) in [&mut left[i..end], &mut right[i..end]].into_iter().enumerate() {
                if !lowpass {
                    continue;
                }
                let (mut s1, mut s2) = (self.s1[side], self.s2[side]);
                for v in x.iter_mut() {
                    let v3 = *v - s2;
                    let v1 = a1 * s1 + a2 * v3;
                    let v2 = s2 + a2 * s1 + a3 * v3;
                    s1 = 2.0 * v1 - s1;
                    s2 = 2.0 * v2 - s2;
                    *v += mix * (v2 - *v);
                }
                // No denormals as a tail dies away.
                self.s1[side] = if s1.abs() < 1e-20 { 0.0 } else { s1 };
                self.s2[side] = if s2.abs() < 1e-20 { 0.0 } else { s2 };
            }
            // The high shelf: the signal plus `boost` times what lies above the split (the
            // signal less its one-pole low side), so below the split it adds nothing.
            let (a, boost) = (self.shelf_a, self.boost);
            if boost > 0.0 || self.to_boost > 0.0 {
                for (side, x) in [&mut left[i..end], &mut right[i..end]].into_iter().enumerate() {
                    let mut lp = self.lp[side];
                    for v in x.iter_mut() {
                        lp += a * (*v - lp);
                        *v += boost * (*v - lp);
                    }
                    self.lp[side] = if lp.abs() < 1e-20 { 0.0 } else { lp };
                }
            }
            i = end;
        }
        if self.mix == 0.0 {
            // Fully out again: start from rest next time.
            self.s1 = [0.0; 2];
            self.s2 = [0.0; 2];
        }
        if self.boost == 0.0 {
            self.lp = [0.0; 2];
        }
    }
}

/// A part's tone controllers and the mod wheel, as the part plays them (see the module
/// docs), and its stem filter.
pub(super) struct PartTone {
    cutoff: u8,
    resonance: u8,
    depth: u8,
    delay: u8,
    /// The player's mod wheel (CC1).
    wheel: u8,
    /// The CC1 the part's synthesizers have (what [`PartTone::modulation`] last gave).
    sent: u8,
    /// Frames since the last note-on (saturating; 0 before the first, so CC78 holds the
    /// first note's vibrato off too).
    since_on: u32,
    /// Frames a step of CC78 above 64 holds the vibrato off, and the fade-in's length.
    frames_per_step: f32,
    fade_frames: f32,
    pub(super) filter: StemFilter,
}

impl PartTone {
    pub(super) fn new(sample_rate: f32) -> PartTone {
        PartTone {
            cutoff: 64,
            resonance: 64,
            depth: 64,
            delay: 64,
            wheel: 0,
            sent: 0,
            since_on: 0,
            frames_per_step: DELAY_PER_STEP_S * sample_rate,
            fade_frames: DELAY_FADE_S * sample_rate,
            filter: StemFilter::new(sample_rate),
        }
    }

    /// Follow a message to the part. True if it is the tone's own (the part's synthesizers
    /// don't get it: the mod wheel goes to them as [`PartTone::modulation`] says).
    #[inline]
    pub(super) fn follow(&mut self, st: i32, d1: i32, d2: i32) -> bool {
        let v = (d2 & 127) as u8;
        match st {
            0x90 if d2 > 0 => {
                self.since_on = 0;
                false
            }
            0xB0 => match d1 {
                MODULATION => {
                    self.wheel = v;
                    true
                }
                CUTOFF | RESONANCE => {
                    if d1 == CUTOFF {
                        self.cutoff = v;
                    } else {
                        self.resonance = v;
                    }
                    self.filter.set(self.cutoff, self.resonance);
                    true
                }
                VIBRATO_DEPTH => {
                    self.depth = v;
                    true
                }
                VIBRATO_DELAY => {
                    self.delay = v;
                    true
                }
                VIBRATO_RATE => true,
                // rustysynth sets its mod wheel to 0 (the part sends it again after); the
                // sound controllers stay as they are (GM2 RP-015, XG).
                RESET_ALL_CONTROLLERS => {
                    self.wheel = 0;
                    self.sent = 0;
                    false
                }
                _ => false,
            },
            _ => false,
        }
    }

    /// The mod wheel the part's synthesizers should have now (CC1, 7-bit), if it isn't
    /// what they have: the player's, CC77's depth, and CC78's delay after a note-on.
    #[inline]
    pub(super) fn modulation(&mut self) -> Option<i32> {
        let full = (self.wheel as i32 + DEPTH_PER_STEP * (self.depth as i32 - 64)).clamp(0, 127);
        let now = if self.delay > 64 && full > 0 {
            let wait = (self.delay - 64) as f32 * self.frames_per_step;
            let t = self.since_on as f32 - wait;
            if t <= 0.0 { 0 } else { (full as f32 * (t / self.fade_frames).min(1.0)).round() as i32 }
        } else {
            full
        };
        (now != self.sent as i32).then(|| {
            self.sent = now as u8;
            now
        })
    }

    /// `frames` frames have been rendered.
    #[inline]
    pub(super) fn advance(&mut self, frames: usize) {
        self.since_on = self.since_on.saturating_add(frames as u32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 48_000.0;

    /// A bright test tone: a square wave at 440 Hz.
    fn square(n: usize) -> Vec<f32> {
        (0..n).map(|i| if (i * 440 * 2 / RATE as usize) % 2 == 0 { 0.5 } else { -0.5 }).collect()
    }

    fn hf(x: &[f32]) -> f32 {
        x.windows(2).map(|w| (w[1] - w[0]).powi(2)).sum()
    }

    #[test]
    fn at_64_the_filter_is_out_of_the_signal() {
        let mut f = StemFilter::new(RATE);
        for (c, r) in [(64, 64), (64, 127)] {
            f.set(c, r);
            let (mut l, mut r) = (square(4800), square(4800));
            f.process(&mut l, &mut r);
            assert!(l == square(4800) && r == square(4800), "bit-identical");
        }
        assert!(!f.active());
    }

    /// Above 64 a high shelf brightens, gliding in and out, and adds nothing at DC.
    #[test]
    fn above_64_a_shelf_brightens_and_comes_back_out() {
        let mut f = StemFilter::new(RATE);
        let open = hf(&square(4800)[2400..]);
        f.set(127, 64);
        let (mut l, mut r) = (square(4800), square(4800));
        f.process(&mut l, &mut r);
        let bright = hf(&l[2400..]);
        assert!(bright > open * 1.5, "brighter: {bright} vs {open}");
        let mut dc = vec![0.5f32; 4800];
        f.process(&mut dc.clone(), &mut dc);
        assert!((dc[4799] - 0.5).abs() < 1e-3, "no broadband gain: {}", dc[4799]);
        f.set(64, 64);
        for _ in 0..10 {
            let (mut l, mut r) = (square(4800), square(4800));
            f.process(&mut l, &mut r);
        }
        assert!(!f.active(), "out again once it has glided back");
        let (mut l, mut r) = (square(4800), square(4800));
        f.process(&mut l, &mut r);
        assert!(l == square(4800), "and bit-identical");
    }

    #[test]
    fn a_lower_cutoff_is_darker_and_comes_back_out() {
        let mut f = StemFilter::new(RATE);
        let (mut l, mut r) = (square(4800), square(4800));
        f.process(&mut l, &mut r);
        let open = hf(&l[2400..]);
        f.set(10, 64);
        let (mut l, mut r) = (square(4800), square(4800));
        f.process(&mut l, &mut r);
        assert!(hf(&l[2400..]) < open * 0.2, "darker: {} vs {open}", hf(&l[2400..]));
        f.set(64, 64);
        for _ in 0..10 {
            let (mut l, mut r) = (square(4800), square(4800));
            f.process(&mut l, &mut r);
        }
        assert!(!f.active(), "out again once it has glided back");
        let (mut l, mut r) = (square(4800), square(4800));
        f.process(&mut l, &mut r);
        assert!(l == square(4800), "and bit-identical");
    }

    #[test]
    fn the_mod_wheel_takes_depth_and_delay() {
        let mut t = PartTone::new(RATE);
        assert_eq!(t.modulation(), None, "nothing to send at rest");
        t.follow(0xB0, 1, 40);
        assert_eq!(t.modulation(), Some(40));
        t.follow(0xB0, 77, 80);
        assert_eq!(t.modulation(), Some(72), "CC77 +16: 32 more");
        t.follow(0xB0, 77, 0);
        assert_eq!(t.modulation(), Some(0), "not below 0");
        t.follow(0xB0, 77, 127);
        t.follow(0xB0, 1, 0);
        assert_eq!(t.modulation(), Some(126), "CC77 alone");
        // CC78 +10: 0.2 s off after each note-on, then a fade in.
        t.follow(0xB0, 78, 74);
        assert_eq!(t.modulation(), Some(0), "no note yet: the first note is held off too");
        t.follow(0x90, 60, 100);
        assert_eq!(t.modulation(), None, "held off");
        t.advance(9000);
        assert_eq!(t.modulation(), None, "still");
        t.advance(600 + 2400);
        assert_eq!(t.modulation(), Some(63), "half way in");
        t.advance(2400);
        assert_eq!(t.modulation(), Some(126), "all in");
        // Reset All Controllers: rustysynth's mod wheel is 0 again; the depth stays.
        assert!(!t.follow(0xB0, 121, 0));
        assert_eq!(t.modulation(), Some(126));
        assert!(t.follow(0xB0, 76, 0), "CC76 is taken (not played)");
    }
}

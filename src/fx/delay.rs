//! The Variation block: a stereo delay synced to the style tempo (#204).
//!
//! The Genos's Variation block can hold many effect types; here it is the tempo delay
//! (Genos: "Tempo Delay", "Tempo Echo", "Tempo Cross"). The types:
//!
//! - 1/8, dotted 1/8 and 1/4: each side repeats its own input after that note length;
//! - ping-pong: the repeats alternate between the sides, an 1/8 apart (a repeat on the
//!   left, then the right, then the left...).
//!
//! The repeats fall away at `FEEDBACK` each, and a lowpass in the loop darkens each one
//! (like a tape or analogue echo). A tempo change glides the delay time to the new length
//! over a few tens of milliseconds rather than jumping, so it never clicks.

use super::line::{Line, OnePole};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DelayType {
    Eighth = 0,
    DottedEighth = 1,
    Quarter = 2,
    PingPong = 3,
}

impl DelayType {
    pub const ALL: [DelayType; 4] = [DelayType::Eighth, DelayType::DottedEighth, DelayType::Quarter, DelayType::PingPong];

    pub fn from_u8(v: u8) -> DelayType {
        Self::ALL.get(v as usize).copied().unwrap_or(DelayType::DottedEighth)
    }

    /// The delay time in quarter notes (beats).
    pub fn beats(self) -> f32 {
        match self {
            DelayType::Eighth | DelayType::PingPong => 0.5,
            DelayType::DottedEighth => 0.75,
            DelayType::Quarter => 1.0,
        }
    }
}

/// How much of each repeat comes back.
pub const FEEDBACK: f32 = 0.38;
/// The longest delay (s): a 1/4 at 30 BPM. Slower tempos repeat at this.
pub const MAX_SECONDS: f32 = 2.0;
/// The feedback loop's lowpass.
const DAMPING_HZ: f32 = 5000.0;

/// The Variation block's delay.
pub struct Delay {
    rate: f32,
    kind: DelayType,
    lines: [Line; 2],
    damping: [OnePole; 2],
    /// The delay time now and where it is going (samples).
    time: f32,
    target: f32,
    /// Per-sample glide towards `target`.
    glide: f32,
}

impl Delay {
    pub fn new(rate: f32) -> Delay {
        let max = (MAX_SECONDS * rate) as usize + 2;
        let mut d = Delay {
            rate,
            kind: DelayType::DottedEighth,
            lines: [Line::new(max), Line::new(max)],
            damping: [OnePole::new(DAMPING_HZ, rate); 2],
            time: 0.0,
            target: 0.0,
            glide: 1.0 - (-1.0 / (0.03 * rate)).exp(),
        };
        d.set(DelayType::DottedEighth, 120.0);
        d.time = d.target;
        d
    }

    /// The type and the tempo (BPM). A type change starts from silence; a tempo change
    /// glides.
    pub fn set(&mut self, t: DelayType, bpm: f32) {
        let bpm = if bpm.is_finite() && bpm > 0.0 { bpm } else { 120.0 };
        let seconds = (t.beats() * 60.0 / bpm).min(MAX_SECONDS);
        self.target = (seconds * self.rate).clamp(2.0, self.lines[0].capacity() as f32 - 2.0);
        if t != self.kind {
            self.kind = t;
            for l in &mut self.lines {
                l.clear();
            }
            for f in &mut self.damping {
                f.clear();
            }
            self.time = self.target;
        }
    }

    /// One stereo frame in, one out (wet only).
    #[inline]
    pub fn tick(&mut self, l: f32, r: f32) -> (f32, f32) {
        self.time += (self.target - self.time) * self.glide;
        let yl = self.lines[0].read_frac(self.time - 1.0);
        let yr = self.lines[1].read_frac(self.time - 1.0);
        let fl = self.damping[0].tick(yl) * FEEDBACK;
        let fr = self.damping[1].tick(yr) * FEEDBACK;
        if self.kind == DelayType::PingPong {
            // Both sides' input starts on the left; each repeat crosses over.
            self.lines[0].write((l + r) * 0.5 + fr);
            self.lines[1].write(fl / FEEDBACK);
        } else {
            self.lines[0].write(l + fl);
            self.lines[1].write(r + fr);
        }
        (yl, yr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An impulse through the delay: where the repeats land (frame, side).
    fn echoes(t: DelayType, bpm: f32) -> Vec<(usize, usize, f32)> {
        let mut d = Delay::new(48_000.0);
        d.set(t, bpm);
        d.time = d.target;
        let mut out = Vec::new();
        let mut last = [None::<usize>; 2];
        for i in 0..48_000 * 3 {
            let x = if i == 0 { 1.0 } else { 0.0 };
            let (l, r) = d.tick(x, x);
            for (side, v) in [l, r].into_iter().enumerate() {
                if v.abs() > 0.02 && last[side].is_none_or(|f| f + 100 < i) {
                    out.push((i, side, v));
                }
                if v.abs() > 0.02 {
                    last[side] = Some(i);
                }
            }
        }
        out
    }

    #[test]
    fn repeats_fall_on_the_beat_divisions() {
        // 120 BPM: a beat is 24000 frames at 48 kHz.
        for (t, len) in [(DelayType::Eighth, 12_000), (DelayType::DottedEighth, 18_000), (DelayType::Quarter, 24_000)] {
            let e = echoes(t, 120.0);
            let lefts: Vec<_> = e.iter().filter(|x| x.1 == 0).collect();
            assert!(lefts.len() >= 3, "{t:?}: {e:?}");
            for (k, x) in lefts.iter().take(3).enumerate() {
                assert!(x.0.abs_diff(len * (k + 1)) <= 2, "{t:?} repeat {k} at {} (want {})", x.0, len * (k + 1));
            }
            // Each repeat quieter than the one before.
            assert!(lefts[1].2.abs() < lefts[0].2.abs() && lefts[2].2.abs() < lefts[1].2.abs());
            assert!(e.iter().any(|x| x.1 == 1 && x.0.abs_diff(len) <= 2), "{t:?}: both sides");
        }
        // Another tempo, another length: 1/4 at 90 BPM is 2/3 s.
        let e = echoes(DelayType::Quarter, 90.0);
        assert!(e[0].0.abs_diff(32_000) <= 2, "{e:?}");
    }

    #[test]
    fn ping_pong_alternates_the_sides() {
        let e = echoes(DelayType::PingPong, 120.0);
        let firsts: Vec<_> = e.iter().take(4).map(|x| (x.0 / 1000, x.1)).collect();
        assert_eq!(firsts, vec![(12, 0), (24, 1), (36, 0), (48, 1)], "{e:?}");
    }

    /// A tempo change glides the delay time: no jump in the output.
    #[test]
    fn a_tempo_change_does_not_click() {
        let mut d = Delay::new(48_000.0);
        d.set(DelayType::Quarter, 120.0);
        let tone = |i: usize| (i as f32 * 220.0 * std::f32::consts::TAU / 48_000.0).sin() * 0.3;
        let mut prev = 0.0;
        let mut worst = 0f32;
        for i in 0..96_000 {
            if i == 48_000 {
                d.set(DelayType::Quarter, 140.0);
            }
            let (l, _) = d.tick(tone(i), tone(i));
            if i > 30_000 {
                worst = worst.max((l - prev).abs());
            }
            prev = l;
        }
        // A 220 Hz sine at this level moves at most ~0.009 a sample; a jump would be far more.
        assert!(worst < 0.05, "{worst}");
    }
}

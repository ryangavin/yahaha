//! The Variation block: a stereo delay synced to the style tempo (#204).
//!
//! The Genos's Variation block can hold many effect types; here it is the tempo delay
//! (Genos: "Tempo Delay", "Tempo Echo", "Tempo Cross"). Its parameters (#236,
//! `super::Param`):
//!
//! - the time: a note value at the style tempo (1/16 to 1/2, with triplets and dotted
//!   notes; `NOTES`), or with tempo sync off a free time in ms;
//! - the feedback: how much of each repeat comes back (0-90%);
//! - the tone: a lowpass in the loop, so each repeat is darker than the last (like a tape
//!   or analogue echo);
//! - ping-pong: the repeats alternate between the sides (a repeat on the left, then the
//!   right, then the left...) instead of each side repeating its own input.
//!
//! The types (1/8, dotted 1/8, 1/4, ping-pong) are starting points: each sets the note
//! value and the ping-pong switch (`DelayType::defaults`), and the control side puts them
//! there on a type change. Nothing jumps: a tempo nudge glides the delay time over a few
//! tens of milliseconds, a new note value or time crossfades to its new tap, the feedback moves smoothly, and the ping-pong switch
//! crossfades its routing, so nothing clicks.

use super::line::{Line, OnePole};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DelayType {
    Eighth = 0,
    DottedEighth = 1,
    Quarter = 2,
    PingPong = 3,
}

/// The note values the time can be (beats, the name), shortest first.
pub const NOTES: [(f32, &str); 8] = [
    (0.25, "1/16"),
    (1.0 / 3.0, "1/8T"),
    (0.5, "1/8"),
    (2.0 / 3.0, "1/4T"),
    (0.75, "1/8."),
    (1.0, "1/4"),
    (1.5, "1/4."),
    (2.0, "1/2"),
];

/// The type-independent parameters' defaults: sync on, 375 ms free time, feedback 38%,
/// tone 5 kHz (the sound of #219).
pub const DEFAULT_MS: u16 = 375;
pub const DEFAULT_FEEDBACK: u16 = 38;
pub const DEFAULT_TONE: u16 = 50;

impl DelayType {
    pub const ALL: [DelayType; 4] = [DelayType::Eighth, DelayType::DottedEighth, DelayType::Quarter, DelayType::PingPong];

    pub fn from_u8(v: u8) -> DelayType {
        Self::ALL.get(v as usize).copied().unwrap_or(DelayType::DottedEighth)
    }

    /// The type's own parameters, in their units (`super::Param`): sync, note (`NOTES`
    /// index), free time (ms), feedback (%), tone (100 Hz), ping-pong.
    pub fn defaults(self) -> [u16; 6] {
        let (note, pp) = match self {
            DelayType::Eighth => (2, 0),
            DelayType::DottedEighth => (4, 0),
            DelayType::Quarter => (5, 0),
            DelayType::PingPong => (2, 1),
        };
        [1, note, DEFAULT_MS, DEFAULT_FEEDBACK, DEFAULT_TONE, pp]
    }
}

/// The longest delay (s): a 1/2 at 60 BPM, a 1/4 at 30. Longer times repeat at this.
pub const MAX_SECONDS: f32 = 2.0;
/// How fast the feedback and the ping-pong routing glide (time constant, s).
const GLIDE_S: f32 = 0.03;
/// A time change bigger than this share of the time now crossfades instead of gliding.
const JUMP: f32 = 0.25;
/// How long that crossfade takes (s).
const FADE_S: f32 = 0.04;

/// The delay's settings, as the bus reads them from the parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    /// The time follows the tempo (`note`), or is `ms`.
    pub sync: bool,
    /// A `NOTES` index.
    pub note: usize,
    pub ms: f32,
    /// 0..0.9.
    pub feedback: f32,
    pub tone_hz: f32,
    pub ping_pong: bool,
}

impl Settings {
    /// Settings from the parameter values (`DelayType::defaults` order).
    pub fn from_params(v: [u16; 6]) -> Settings {
        Settings {
            sync: v[0] != 0,
            note: (v[1] as usize).min(NOTES.len() - 1),
            ms: v[2] as f32,
            feedback: (v[3].min(90) as f32) / 100.0,
            tone_hz: v[4] as f32 * 100.0,
            ping_pong: v[5] != 0,
        }
    }
}

/// The Variation block's delay.
pub struct Delay {
    rate: f32,
    lines: [Line; 2],
    damping: [OnePole; 2],
    /// The settings and tempo last applied.
    applied: Option<(Settings, f32)>,
    /// The delay time now and where it is going (samples). A small change (a tempo
    /// nudge) glides there; a big one (a new note value) jumps the tap and crossfades from
    /// the old one (`from`, `fade` 0..1), since gliding that far would sweep the echoes'
    /// pitch.
    time: f32,
    target: f32,
    from: f32,
    fade: f32,
    fade_step: f32,
    /// Feedback now and target; the ping-pong routing now (0 = straight, 1 = crossed)
    /// and target.
    feedback: f32,
    feedback_to: f32,
    cross: f32,
    cross_to: f32,
    /// Per-sample glide towards the targets.
    glide: f32,
}

impl Delay {
    pub fn new(rate: f32) -> Delay {
        let max = (MAX_SECONDS * rate) as usize + 2;
        let mut d = Delay {
            rate,
            lines: [Line::new(max), Line::new(max)],
            damping: [OnePole::new(DEFAULT_TONE as f32 * 100.0, rate); 2],
            applied: None,
            time: 0.0,
            target: 0.0,
            from: 0.0,
            fade: 1.0,
            fade_step: 1.0 / (FADE_S * rate),
            feedback: 0.0,
            feedback_to: 0.0,
            cross: 0.0,
            cross_to: 0.0,
            glide: 1.0 - (-1.0 / (GLIDE_S * rate)).exp(),
        };
        d.set(Settings::from_params(DelayType::DottedEighth.defaults()), 120.0);
        d.snap();
        d
    }

    /// The settings and the tempo (BPM). Everything glides there; nothing to do if
    /// nothing changed.
    pub fn set(&mut self, s: Settings, bpm: f32) {
        let bpm = if bpm.is_finite() && bpm > 0.0 { bpm } else { 120.0 };
        if self.applied == Some((s, bpm)) {
            return;
        }
        self.applied = Some((s, bpm));
        let seconds = if s.sync { NOTES[s.note.min(NOTES.len() - 1)].0 * 60.0 / bpm } else { s.ms / 1000.0 };
        self.target = (seconds.min(MAX_SECONDS) * self.rate).clamp(2.0, self.lines[0].capacity() as f32 - 2.0);
        if (self.target - self.time).abs() > JUMP * self.time {
            // From wherever the output mostly is now.
            if self.fade >= 0.5 {
                self.from = self.time;
            }
            self.time = self.target;
            self.fade = 0.0;
        }
        self.feedback_to = s.feedback.clamp(0.0, 0.9);
        self.cross_to = if s.ping_pong { 1.0 } else { 0.0 };
        for f in &mut self.damping {
            f.set(s.tone_hz.clamp(200.0, 22_000.0), self.rate);
        }
    }

    /// At the targets at once.
    fn snap(&mut self) {
        self.time = self.target;
        self.fade = 1.0;
        self.feedback = self.feedback_to;
        self.cross = self.cross_to;
    }

    /// One stereo frame in, one out (wet only).
    #[inline]
    pub fn tick(&mut self, l: f32, r: f32) -> (f32, f32) {
        let k = self.glide;
        self.time += (self.target - self.time) * k;
        self.feedback += (self.feedback_to - self.feedback) * k;
        self.cross += (self.cross_to - self.cross) * k;
        let mut yl = self.lines[0].read_frac(self.time - 1.0);
        let mut yr = self.lines[1].read_frac(self.time - 1.0);
        if self.fade < 1.0 {
            let f = self.fade;
            let ol = self.lines[0].read_frac(self.from - 1.0);
            let or = self.lines[1].read_frac(self.from - 1.0);
            yl = ol + (yl - ol) * f;
            yr = or + (yr - or) * f;
            self.fade = (f + self.fade_step).min(1.0);
        }
        let dl = self.damping[0].tick(yl);
        let dr = self.damping[1].tick(yr);
        let (fl, fr) = (dl * self.feedback, dr * self.feedback);
        // Straight: each side repeats its own. Ping-pong: both sides' input starts on the
        // left, and each repeat crosses over (the left's first repeat whole, as the
        // input to the right). The routing crossfades between the two.
        let c = self.cross;
        let (sl, sr) = (l + fl, r + fr);
        let (pl, pr) = ((l + r) * 0.5 + fr, dl);
        self.lines[0].write(sl + (pl - sl) * c);
        self.lines[1].write(sr + (pr - sr) * c);
        (yl, yr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(t: DelayType) -> Settings {
        Settings::from_params(t.defaults())
    }

    /// An impulse through the delay: where the repeats land (frame, side, level).
    fn echoes_with(s: Settings, bpm: f32) -> Vec<(usize, usize, f32)> {
        let mut d = Delay::new(48_000.0);
        d.set(s, bpm);
        d.snap();
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

    fn echoes(t: DelayType, bpm: f32) -> Vec<(usize, usize, f32)> {
        echoes_with(settings(t), bpm)
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

    /// #236: every note value, triplets and dotted ones, at the tempo; with sync off, the
    /// free time in ms, whatever the tempo.
    #[test]
    fn the_time_is_a_note_value_or_free() {
        for (i, (beats, name)) in NOTES.iter().enumerate() {
            let s = Settings { note: i, ..settings(DelayType::Eighth) };
            let want = (beats * 24_000.0).round() as usize;
            let e = echoes_with(s, 120.0);
            assert!(e[0].0.abs_diff(want) <= 2, "{name}: first repeat at {} (want {want})", e[0].0);
        }
        for bpm in [80.0, 140.0] {
            let s = Settings { sync: false, ms: 250.0, ..settings(DelayType::Eighth) };
            let e = echoes_with(s, bpm);
            assert!(e[0].0.abs_diff(12_000) <= 2, "250 ms at {bpm}: {}", e[0].0);
        }
    }

    /// #236: the feedback sets how fast the repeats fall away (0: a single repeat), and
    /// the tone how dark they get.
    #[test]
    fn feedback_and_tone_shape_the_repeats() {
        let run = |feedback: f32, tone: f32| echoes_with(Settings { feedback, tone_hz: tone, ..settings(DelayType::Quarter) }, 120.0);
        let lefts = |e: &[(usize, usize, f32)]| e.iter().filter(|x| x.1 == 0).map(|x| x.2.abs()).collect::<Vec<_>>();
        assert_eq!(lefts(&run(0.0, 5000.0)).len(), 1, "no feedback: one repeat");
        let (low, high) = (lefts(&run(0.2, 20_000.0)), lefts(&run(0.8, 20_000.0)));
        assert!(high.len() >= low.len() + 2, "more feedback, more repeats: {} vs {}", high.len(), low.len());
        assert!((high[1] / high[0] - 0.8).abs() < 0.1, "each repeat at the feedback: {high:?}");
        // A dark tone takes the edge off each repeat: its peak falls faster.
        let dark = lefts(&run(0.8, 1000.0));
        assert!(dark[1] < high[1] * 0.6, "{dark:?} vs {high:?}");
    }

    #[test]
    fn ping_pong_alternates_the_sides() {
        let e = echoes(DelayType::PingPong, 120.0);
        let firsts: Vec<_> = e.iter().take(4).map(|x| (x.0 / 1000, x.1)).collect();
        assert_eq!(firsts, vec![(12, 0), (24, 1), (36, 0), (48, 1)], "{e:?}");
        // The switch on any note value: 1/4 ping-pong.
        let e = echoes_with(Settings { ping_pong: true, ..settings(DelayType::Quarter) }, 120.0);
        let firsts: Vec<_> = e.iter().take(2).map(|x| (x.0 / 1000, x.1)).collect();
        assert_eq!(firsts, vec![(24, 0), (48, 1)], "{e:?}");
    }

    /// A tempo change, a new time, feedback, tone or the ping-pong switch while it
    /// repeats: it glides, with no jump in the output.
    #[test]
    fn a_change_does_not_click() {
        let mut d = Delay::new(48_000.0);
        let base = settings(DelayType::Quarter);
        d.set(base, 120.0);
        let tone = |i: usize| (i as f32 * 220.0 * std::f32::consts::TAU / 48_000.0).sin() * 0.3;
        let mut prev = 0.0;
        let mut worst = 0f32;
        for i in 0..192_000 {
            match i {
                48_000 => d.set(base, 140.0),
                72_000 => d.set(Settings { note: 1, feedback: 0.8, ..base }, 140.0),
                96_000 => d.set(Settings { note: 1, feedback: 0.8, ping_pong: true, tone_hz: 2000.0, ..base }, 140.0),
                120_000 => d.set(Settings { sync: false, ms: 90.0, feedback: 0.1, ..base }, 140.0),
                _ => {}
            }
            let (l, _) = d.tick(tone(i), tone(i));
            if i > 30_000 {
                worst = worst.max((l - prev).abs());
            }
            prev = l;
        }
        // A 220 Hz sine at this level moves at most ~0.009 a sample (0.05 with the
        // feedback stacking repeats); a jump would be far more.
        assert!(worst < 0.08, "{worst}");
    }
}

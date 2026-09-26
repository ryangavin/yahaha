//! TEMPO −/+ held down (OM p.46): "holding down either button changes the value
//! continuously". The Launchkey's tempo buttons report press and release, so the engine
//! repeats the step itself while one is held (`live::Cmd::TempoHold`), on engine
//! nanoseconds, band running or not.
//!
//! The manual gives neither the step nor the repeat rate. yahaha steps 1 BPM (the Genos
//! panel's step), first repeats after `REPEAT_DELAY_MS`, then speeds up
//! (`repeat_interval_ms`): slow enough at first to stop on the tempo wanted, fast enough
//! to cross 5-500 BPM in a few seconds. The app's tempo buttons use the same schedule.

use super::*;

/// One press of TEMPO − or +, in BPM.
pub const TEMPO_STEP: f64 = 1.0;

/// How long a tempo button is held before it starts repeating.
pub const REPEAT_DELAY_MS: u64 = 400;

/// A hold longer than this stops repeating: a release lost with the controller unplugged
/// must not run the tempo to the end of its range.
pub const MAX_HOLD_MS: u64 = 30_000;

/// The time to the next step after `repeats` repeats: 10 steps a second at first, 20 after
/// the first 8, 40 after 24 more (about a second and a half in).
pub const fn repeat_interval_ms(repeats: u32) -> u64 {
    match repeats {
        0..8 => 100,
        8..32 => 50,
        _ => 25,
    }
}

/// Engine-side auto-repeat state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct TempoRepeat {
    /// −1 or +1 while a button is held, else 0.
    dir: i8,
    since: u64,
    next: u64,
    repeats: u32,
}

impl Engine {
    /// One tempo step down (`dir` < 0) or up. A ritardando slows from the stepped tempo.
    pub(super) fn tempo_step(&mut self, dir: i8, now: u64) {
        let d = TEMPO_STEP * dir.signum() as f64;
        self.rit_tempo(d);
        self.set_bpm_internal(self.bpm + d, now);
    }

    /// A tempo button went down (`dir` −1 or +1: one step now, then repeating while it is
    /// held) or up (0: stop repeating).
    pub fn tempo_hold(&mut self, dir: i8, now: u64) {
        let dir = dir.signum();
        self.features.tempo_repeat = TempoRepeat::default();
        if dir != 0 {
            self.tempo_step(dir, now);
            let next = now + REPEAT_DELAY_MS * 1_000_000;
            self.features.tempo_repeat = TempoRepeat { dir, since: now, next, repeats: 0 };
        }
    }

    /// When the held button steps next.
    pub(super) fn tempo_repeat_deadline(&self) -> Option<u64> {
        let r = &self.features.tempo_repeat;
        (r.dir != 0).then_some(r.next)
    }

    /// A held tempo button's step is due. One step per wake: a late wake does not jump.
    pub(super) fn tempo_repeat_wake(&mut self, now: u64) {
        let r = self.features.tempo_repeat;
        if r.dir == 0 || now < r.next {
            return;
        }
        if now.saturating_sub(r.since) > MAX_HOLD_MS * 1_000_000 {
            self.features.tempo_repeat = TempoRepeat::default();
            return;
        }
        self.tempo_step(r.dir, now);
        let repeats = r.repeats + 1;
        let next = now + repeat_interval_ms(repeats) * 1_000_000;
        self.features.tempo_repeat = TempoRepeat { repeats, next, ..r };
    }

    /// TEMPO − and + pressed together: the tempo the style came with (OM p.46). A
    /// ritardando slows from it.
    pub(super) fn reset_tempo(&mut self, now: u64) {
        self.features.tempo_repeat = TempoRepeat::default();
        self.set_bpm_internal(self.style.bpm, now);
        self.rit_retempo(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: u64 = 1_000_000;

    struct Null;
    impl Sink for Null {
        fn send(&mut self, _: &[u8]) {}
    }

    /// SlowWalker (4/4, 75 BPM), stopped.
    fn engine() -> Option<Engine> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
        if !p.exists() {
            eprintln!("corpus missing; skipping");
            return None;
        }
        Some(Engine::new(Box::new(Prepared::new(&crate::sff::Style::load(&p).unwrap()))))
    }

    /// Run the engine as the engine loop does, waking at its deadlines, up to `to`.
    fn run(e: &mut Engine, from: u64, to: u64) {
        let mut now = from;
        e.process(now, &mut Null);
        while let Some(d) = e.next_deadline() {
            if d > to {
                break;
            }
            now = d.max(now + 1);
            e.process(now, &mut Null);
        }
    }

    fn bpm(e: &Engine) -> f64 {
        e.snapshot(0).bpm
    }

    #[test]
    fn one_press_is_one_bpm() {
        let Some(mut e) = engine() else { return };
        e.button(Button::TempoUp, 0, &mut Null);
        assert_eq!(bpm(&e), 76.0);
        e.button(Button::TempoDown, 0, &mut Null);
        e.button(Button::TempoDown, 0, &mut Null);
        assert_eq!(bpm(&e), 74.0);
    }

    /// Held: one step at once, the next after the delay, then faster and faster; the
    /// release stops it.
    #[test]
    fn a_held_button_repeats_and_speeds_up() {
        let Some(mut e) = engine() else { return };
        e.tempo_hold(1, 0);
        assert_eq!(bpm(&e), 76.0);
        run(&mut e, 0, (REPEAT_DELAY_MS - 1) * MS);
        assert_eq!(bpm(&e), 76.0, "no repeat before the delay");
        run(&mut e, REPEAT_DELAY_MS * MS, REPEAT_DELAY_MS * MS);
        assert_eq!(bpm(&e), 77.0);
        // The first second of repeats (10 a second), then faster.
        let t = REPEAT_DELAY_MS * MS;
        run(&mut e, t, t + 700 * MS);
        assert_eq!(bpm(&e), 77.0 + 7.0);
        run(&mut e, t + 700 * MS, t + 2_000 * MS);
        let fast = bpm(&e);
        assert!(fast > 84.0 + 13.0 * 1.5, "speeds up: {fast}");
        e.tempo_hold(0, t + 2_000 * MS);
        run(&mut e, t + 2_000 * MS, t + 5_000 * MS);
        assert_eq!(bpm(&e), fast, "the release stops it");
        assert_eq!(e.next_deadline(), None);
        // Down, held: down to the floor and no further.
        e.tempo_hold(-1, 10_000 * MS);
        run(&mut e, 10_000 * MS, 30_000 * MS);
        assert_eq!(bpm(&e), MIN_BPM);
    }

    /// A release that never comes: the repeat gives up.
    #[test]
    fn a_lost_release_stops_repeating() {
        let Some(mut e) = engine() else { return };
        e.tempo_hold(-1, 0);
        run(&mut e, 0, (MAX_HOLD_MS + 1_000) * MS);
        assert_eq!(e.next_deadline(), None);
    }

    /// −/+ together: the style's own tempo, stopped or playing, and a ritardando slows from it.
    #[test]
    fn reset_restores_the_style_tempo() {
        let Some(mut e) = engine() else { return };
        for _ in 0..5 {
            e.button(Button::TempoUp, 0, &mut Null);
        }
        e.tempo_hold(1, 0);
        e.button(Button::TempoReset, 0, &mut Null);
        assert_eq!(bpm(&e), 75.0);
        run(&mut e, 0, 2_000 * MS);
        assert_eq!(bpm(&e), 75.0, "the reset ends a repeat");
        // Playing.
        e.button(Button::StartStop, 0, &mut Null);
        e.button(Button::SetTempo(120), 0, &mut Null);
        e.button(Button::TempoReset, 10 * MS, &mut Null);
        assert_eq!(bpm(&e), 75.0);
    }
}

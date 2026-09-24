//! The metronome (Genos Menu > Metronome, RM p.39): a click on every beat, with a bell on
//! the first beat of the bar. While the band plays it clicks on the style's beat lines
//! (the `on_beat` hook, woken exactly by `hook_deadline`); while stopped it runs free at
//! the tempo, in the style's time signature, from when it was turned on or the band
//! stopped.
//!
//! The click is `Sink::click`: the built-in synth's click voice (`crate::click`), never
//! the MIDI port. Its volume is the synth's (`SynthControl::click_volume`).

use super::*;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Metronome {
    on: bool,
    bell: bool,
    /// Stopped: when the next free-running click is due (0: from the next wake, one beat on).
    idle_next: u64,
    /// Stopped: the beat of the bar that click is.
    idle_beat: u32,
}

impl Engine {
    /// Metronome on/off and the bell on beat 1.
    pub fn set_metronome(&mut self, on: bool, bell: bool, now: u64) {
        let m = &mut self.features.metronome;
        if on && !m.on {
            // Turned on while stopped: a bar starts now.
            m.idle_next = now.max(1);
            m.idle_beat = 0;
        }
        m.on = on;
        m.bell = bell;
    }

    fn beat_ns(&self) -> u64 {
        (60e9 / self.bpm.max(MIN_BPM)) as u64
    }

    /// A beat line while playing (the `on_beat` hook).
    pub(super) fn metronome_beat(&mut self, beat: u32, sink: &mut impl Sink) {
        let m = self.features.metronome;
        if m.on {
            sink.click(m.bell && beat == 0);
        }
    }

    /// The band stopped: the free-running clicks go on a beat later.
    pub(super) fn metronome_on_stop(&mut self) {
        let m = &mut self.features.metronome;
        m.idle_next = 0;
        m.idle_beat = 0;
    }

    /// Playing: the next beat line, when the metronome needs to be woken for it.
    pub(super) fn metronome_line(&self) -> Option<f64> {
        self.features.metronome.on.then_some(self.lines.next)
    }

    /// Stopped: when the next free-running click is due.
    pub(super) fn metronome_idle_deadline(&self) -> Option<u64> {
        let m = &self.features.metronome;
        m.on.then_some(m.idle_next)
    }

    /// Stopped: the free-running clicks due by `now`.
    pub(super) fn metronome_idle(&mut self, now: u64, sink: &mut impl Sink) {
        if !self.features.metronome.on {
            return;
        }
        let period = self.beat_ns().max(1);
        let beats = (self.style.tpb / self.style.ppq.max(1)).max(1);
        let m = &mut self.features.metronome;
        if m.idle_next == 0 {
            m.idle_next = now + period;
            return;
        }
        if now >= m.idle_next {
            sink.click(m.bell && m.idle_beat == 0);
            m.idle_beat = (m.idle_beat + 1) % beats;
            m.idle_next += period;
            // Woken late (more than a beat): carry on from now rather than click a burst.
            if m.idle_next <= now {
                m.idle_next = now + period;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Clicks(Vec<(u64, bool)>, u64);
    impl Sink for Clicks {
        fn send(&mut self, _: &[u8]) {}
        fn click(&mut self, accent: bool) {
            self.0.push((self.1, accent));
        }
    }

    fn engine() -> Option<Engine> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
        if !p.exists() {
            eprintln!("corpus missing; skipping");
            return None;
        }
        Some(Engine::new(Box::new(Prepared::new(&Style::load(&p).unwrap()))))
    }

    /// Drive the engine as its thread does: wake at each deadline.
    fn run(e: &mut Engine, s: &mut Clicks, mut now: u64, to: u64) {
        while now < to {
            s.1 = now;
            e.process(now, s);
            now = e.next_deadline().unwrap_or(to).max(now + 1);
        }
    }

    #[test]
    fn clicks_on_the_beat_with_a_bell_on_one() {
        let Some(mut e) = engine() else { return };
        let mut s = Clicks::default();
        e.set_metronome(true, true, 0);
        e.set_chord(crate::parse_chord("C").unwrap(), 0, &mut s);
        let bar = e.ns_at_bar(1);
        let beats = (e.style.tpb / e.style.ppq) as usize;
        run(&mut e, &mut s, 0, 2 * bar + 1);
        assert_eq!(s.0.len(), 2 * beats + 1, "{:?}", s.0);
        let beat = bar / beats as u64;
        for (i, &(t, accent)) in s.0.iter().enumerate() {
            assert!(t.abs_diff(i as u64 * beat) <= 2, "click {i} at {t}");
            assert_eq!(accent, i % beats == 0);
        }
    }

    #[test]
    fn runs_free_while_stopped() {
        let Some(mut e) = engine() else { return };
        let mut s = Clicks::default();
        e.button(Button::SetTempo(120), 0, &mut s);
        e.set_metronome(true, false, 1_000);
        run(&mut e, &mut s, 1_000, 2_000_000_000);
        // 120 BPM: a click every 500 ms from when it was turned on.
        let times: Vec<_> = s.0.iter().map(|c| c.0).collect();
        assert_eq!(times, [1_000, 500_001_000, 1_000_001_000, 1_500_001_000]);
        assert!(s.0.iter().all(|c| !c.1), "no bell when it is off");
        e.set_metronome(false, false, 2_000_000_000);
        s.0.clear();
        run(&mut e, &mut s, 2_000_000_000, 4_000_000_000);
        assert!(s.0.is_empty());
    }

    #[test]
    fn tempo_range_is_5_to_500() {
        let Some(mut e) = engine() else { return };
        e.button(Button::SetTempo(4), 0, &mut Clicks::default());
        assert_eq!(e.snapshot(0).bpm, 5.0);
        e.button(Button::SetTempo(501), 0, &mut Clicks::default());
        assert_eq!(e.snapshot(0).bpm, 500.0);
        e.button(Button::TempoUp, 0, &mut Clicks::default());
        assert_eq!(e.snapshot(0).bpm, 500.0);
    }

    /// Tap Tempo covers the whole range: taps 10 s apart give 6 BPM, 0.125 s apart 480;
    /// a pause past 12 s forgets the taps, and a sudden change starts a fresh average.
    #[test]
    fn tap_tempo_reaches_5_to_500() {
        let Some(mut e) = engine() else { return };
        let tap = |e: &mut Engine, t: u64| e.button(Button::TapTempo, t, &mut Clicks::default());
        for i in 0..3 {
            tap(&mut e, 1 + i * 10_000_000_000);
        }
        assert!((e.snapshot(0).bpm - 6.0).abs() < 1e-6, "{}", e.snapshot(0).bpm);
        let t0 = 100_000_000_000;
        for i in 0..4 {
            tap(&mut e, t0 + i * 125_000_000);
        }
        assert!((e.snapshot(0).bpm - 480.0).abs() < 1e-6, "{}", e.snapshot(0).bpm);
        // 120 BPM, then quarter-speed taps: the new tempo at once, not an average.
        let t1 = 200_000_000_000;
        for i in 0..3 {
            tap(&mut e, t1 + i * 500_000_000);
        }
        assert!((e.snapshot(0).bpm - 120.0).abs() < 1e-6);
        tap(&mut e, t1 + 1_000_000_000 + 2_000_000_000);
        assert!((e.snapshot(0).bpm - 30.0).abs() < 1e-6, "{}", e.snapshot(0).bpm);
        // A tap after 13 s starts again: no tempo from it alone.
        tap(&mut e, t1 + 16_000_000_000);
        assert!((e.snapshot(0).bpm - 30.0).abs() < 1e-6);
    }
}

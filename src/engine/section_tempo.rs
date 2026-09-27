//! Tempo changes written inside a section (#243): mostly the ritardando curves at the end
//! of Endings and Intros (71 corpus styles, most from the SX900 "for Genos" set). The
//! parser keeps them (`Style::timing_changes`); `Prepared::section_tempo` turns them into
//! tempo ratios against the style's own tempo (its bar-1 tempo).
//!
//! The manuals don't say whether the Genos plays them; the owner decided it does, relative
//! to the tempo playing: an Ending written to slow from 120 to 90 slows from 100 to 75 when
//! the player plays at 100. The panel tempo comes back when the section ends or the band
//! stops. `StyleSettings::section_tempo` turns it off (default on).
//!
//! The player's own tempo moves during such a section (TEMPO −/+, TAP, a tempo set) move
//! the tempo the curve is read against. A ritardando from pressing ENDING again takes over
//! from the tempo reached (ritardando.rs); the written curve stops until the section ends.

use super::*;

/// Engine-side state: the section whose written tempo changes are playing.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct SectionTempo {
    pub(super) active: bool,
    /// The panel tempo the ratios scale (and that comes back at the end).
    base: f64,
    /// The ratio in effect.
    ratio: f64,
    /// The section slot and its entry on the timeline.
    slot: usize,
    sec_start: f64,
    /// The next change to apply.
    next: usize,
}

impl Engine {
    /// A section began (at the start, or a section change): the previous one's tempo comes
    /// back, and this one's written changes, if any, begin.
    pub(super) fn section_tempo_enter(&mut self, now: u64) {
        self.section_tempo_end(now);
        if !self.features.settings.section_tempo || self.style.section_tempo[self.cur].is_empty() {
            return;
        }
        self.features.section_tempo = SectionTempo { active: true, base: self.bpm, ratio: 1.0, slot: self.cur, sec_start: self.sec_start, next: 0 };
        self.section_tempo_wake(now);
    }

    /// The section's written changes end: the panel tempo comes back.
    pub(super) fn section_tempo_end(&mut self, now: u64) {
        let s = self.features.section_tempo;
        if s.active {
            self.features.section_tempo.active = false;
            self.set_bpm_internal(s.base, now);
        }
    }

    /// A new style took over: its tempo is the one to keep (Change Behavior set it).
    pub(super) fn section_tempo_drop(&mut self) {
        self.features.section_tempo.active = false;
    }

    /// The written changes due by `now`, in order.
    pub(super) fn section_tempo_wake(&mut self, now: u64) {
        let s = self.features.section_tempo;
        if !s.active || !self.running || self.features.rit.active || s.slot != self.cur {
            return;
        }
        let changes = &self.style.section_tempo[s.slot];
        let t = self.tick_at(now) - s.sec_start + 1e-6;
        let mut next = s.next;
        let mut ratio = s.ratio;
        while let Some(&(tick, r)) = changes.get(next) {
            if tick > t {
                break;
            }
            ratio = r;
            next += 1;
        }
        if next != s.next {
            self.features.section_tempo.next = next;
            self.features.section_tempo.ratio = ratio;
            self.set_bpm_internal(s.base * ratio, now);
        }
    }

    /// The tick of the next written change, for `hook_deadline`.
    pub(super) fn section_tempo_deadline(&self) -> Option<f64> {
        let s = &self.features.section_tempo;
        if !s.active || self.features.rit.active || s.slot != self.cur {
            return None;
        }
        self.style.section_tempo[s.slot].get(s.next).map(|&(tick, _)| s.sec_start + tick)
    }

    /// The player set the tempo during a section with written changes: it becomes the
    /// tempo the curve is read against (`base`), from the ratio in effect.
    pub(super) fn section_tempo_retempo(&mut self) {
        let s = &mut self.features.section_tempo;
        if s.active && s.ratio > 0.0 {
            s.base = self.bpm / s.ratio;
        }
    }

    /// The tempo the section's curve is read against (the panel tempo), when one plays.
    pub fn section_tempo_base(&self) -> Option<f64> {
        let s = &self.features.section_tempo;
        s.active.then_some(s.base)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sff::{Style, Timing, TimingChange};

    const MS: u64 = 1_000_000;

    struct Null;
    impl Sink for Null {
        fn send(&mut self, _: &[u8]) {}
    }

    /// SlowWalker (75 BPM) with a written ritardando in Ending A's second bar: 60 BPM at its
    /// start, 50 BPM halfway through it. (The corpus styles with such curves are not all
    /// in every checkout; this one is.)
    fn style() -> Option<Style> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
        if !p.exists() {
            eprintln!("corpus missing; skipping");
            return None;
        }
        let mut s = Style::load(&p).unwrap();
        let bar = s.ticks_per_bar();
        let name = SectionId::Ending(0).name();
        s.timing_changes = vec![
            TimingChange { section: name.clone(), tick: bar, change: Timing::Tempo(1_000_000) },
            TimingChange { section: name.clone(), tick: bar + bar / 2, change: Timing::Tempo(1_200_000) },
            TimingChange { section: name, tick: bar, change: Timing::TimeSig(3, 4) },
        ];
        Some(s)
    }

    fn run(e: &mut Engine, from: u64, to: u64) {
        let mut now = from;
        while now < to {
            e.process(now, &mut Null);
            now = e.next_deadline().unwrap_or(to).clamp(now + 1, now + 5 * MS).min(to);
        }
        e.process(to, &mut Null);
    }

    #[test]
    fn prepared_keeps_tempo_ratios_per_section() {
        let Some(s) = style() else { return };
        let p = Prepared::new(&s);
        let bar = p.tpb as f64;
        let e = &p.section_tempo[slot_of(SectionId::Ending(0))];
        assert_eq!(e.len(), 2, "time signatures are not tempo");
        assert_eq!(e[0], (bar, 0.8));
        assert!((e[1].1 - 50.0 / 75.0).abs() < 1e-9);
        assert!(p.section_tempo[slot_of(SectionId::Main(0))].is_empty());
    }

    /// Played at 100 BPM: the Ending slows to 80 and ~66.7 (the written 60 and 50, scaled
    /// by 100/75), and the panel tempo comes back when the band stops.
    #[test]
    fn an_ending_plays_its_written_ritardando_relative_to_the_tempo() {
        let Some(s) = style() else { return };
        let mut e = Engine::new(Box::new(Prepared::new(&s)));
        e.set_chord(Chord::new(0, 0), 0, &mut Null);
        e.button(Button::SetTempo(100), 0, &mut Null);
        e.button(Button::Ending(0), 10 * MS, &mut Null);
        let bar1 = e.ns_at(e.style.tpb as f64);
        run(&mut e, 0, bar1 + MS);
        assert_eq!(e.snapshot(0).cur, Some(SectionId::Ending(0)));
        assert_eq!(e.bpm, 100.0, "bar 1 of the Ending: as written, the tempo playing");
        let bar2 = e.ns_at(e.sec_start + e.style.tpb as f64);
        run(&mut e, bar1 + MS, bar2 + MS);
        assert!((e.bpm - 80.0).abs() < 1e-6, "{}", e.bpm);
        let half = e.ns_at(e.sec_start + 1.5 * e.style.tpb as f64);
        run(&mut e, bar2 + MS, half + MS);
        assert!((e.bpm - 100.0 * 50.0 / 75.0).abs() < 1e-6, "{}", e.bpm);
        run(&mut e, half + MS, half + 10_000 * MS);
        assert!(!e.running);
        assert_eq!(e.bpm, 100.0, "the panel tempo comes back");
    }

    /// The setting off: the tempo stays.
    #[test]
    fn the_setting_off_keeps_the_tempo() {
        let Some(s) = style() else { return };
        let mut e = Engine::new(Box::new(Prepared::new(&s)));
        e.set_style_settings(StyleSettings { section_tempo: false, ..StyleSettings::default() });
        e.set_chord(Chord::new(0, 0), 0, &mut Null);
        e.button(Button::Ending(0), 10 * MS, &mut Null);
        let end = e.ns_at(3.0 * e.style.tpb as f64);
        let mut seen = Vec::new();
        let mut now = 0;
        while now < end {
            e.process(now, &mut Null);
            seen.push(e.bpm);
            now += 20 * MS;
        }
        assert!(seen.iter().all(|&b| b == 75.0));
    }

    /// TEMPO + during the curve moves the tempo it is read against.
    #[test]
    fn a_tempo_change_during_the_curve_moves_its_base() {
        let Some(s) = style() else { return };
        let mut e = Engine::new(Box::new(Prepared::new(&s)));
        e.set_chord(Chord::new(0, 0), 0, &mut Null);
        e.button(Button::Ending(0), 10 * MS, &mut Null);
        let bar1 = e.ns_at(e.style.tpb as f64);
        run(&mut e, 0, bar1 + MS);
        let bar2 = e.ns_at(e.sec_start + e.style.tpb as f64);
        run(&mut e, bar1 + MS, bar2 + MS);
        assert!((e.bpm - 60.0).abs() < 1e-6);
        e.button(Button::TempoUp, bar2 + 2 * MS, &mut Null);
        assert!((e.section_tempo_base().unwrap() - 61.0 / 0.8).abs() < 1e-6);
    }
}

//! Engine extension points: where a feature that lives in the engine plugs in.
//!
//! Each hook is a plain method, called at one fixed point of the engine's work and in a
//! fixed order; none of them does anything yet. A feature adds one call to its own
//! function (in its own module) to the hook it needs, and keeps its engine-side state in
//! one field of [`Features`]. No trait objects, no registry: the calls are static and
//! inline away while the bodies are empty.
//!
//! The rules the engine's own code keeps apply here too: deterministic (time is the `now`
//! passed in, never the wall clock), and no allocation or freeing (the engine runs on the
//! real-time thread; `tests/engine_no_alloc.rs` checks it). An `Engine` also plays style
//! previews (`live::Audition`): hooks run there too.
//!
//! | Hook | When |
//! |---|---|
//! | `on_start` | START (or Sync Start): the first section is set up, nothing of it played yet |
//! | `on_stop` | the band stopped: every note is off (not called when already stopped) |
//! | `on_bar`, `on_beat` | `process` reached a bar line / beat (quarter note) of the section playing |
//! | `before_section_change` | at a section boundary, before anything changes (the old notes still sound) |
//! | `after_section_change` | after it: the new section is set up, its first events not yet played |
//! | `on_chord` | the chord the style follows changed (a new chord, or a Keyboard transpose) |
//! | `on_style_loaded` | a new style took over (at once when stopped, at the bar line when playing) |
//! | `hook_deadline` | a tick by which a feature needs `process` to run (see below) |
//! | `on_due` | `process` reached the tick `hook_due` named (a feature's own timed action) |
//!
//! Timing: bar and beat hooks run when `process` passes the line, before the pattern's
//! events at that tick, and after a section change at that tick (they see the new
//! section). `process` runs at every event and boundary; a line between two events is seen
//! at the next one. A feature that must act exactly on the line (a metronome click)
//! returns the next line from `hook_deadline` so the engine wakes for it. A feature that
//! acts between lines (the Chord Looper's chord changes) names the tick in `hook_due`:
//! `process` calls `on_due` there, after a line at the same tick and before the pattern's
//! events at it.
//!
//! "When does a queued section change happen" is not a hook but a policy:
//! `Engine::change_point` (sections.rs), and `Engine::follow_on` for what plays when a
//! section ends with nothing queued.

use super::*;

/// Engine-side state of the features that plug into the hooks: one field per feature,
/// its own `Copy`/fixed-size struct (no heap: `Engine::new` builds it on the control
/// side, but the engine thread must never grow it).
#[derive(Default)]
pub(super) struct Features {
    /// Chord Looper (looper.rs). Boxed: its sequences are a few KB.
    pub(super) looper: Box<super::looper::Looper>,
    /// Metronome (metronome.rs).
    pub(super) metronome: super::metronome::Metronome,
    /// The Style part soloed, 0-7 (mixer.rs).
    pub(super) solo: Option<u8>,
}

/// The next beat line the bar and beat hooks wait for: a tick on the section's timeline
/// (as `sec_start`), and its bar (0-based in this pass of the section) and beat (0-based
/// in the bar).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Lines {
    pub(super) next: f64,
    pub(super) bar: u32,
    pub(super) beat: u32,
}

/// What ran, for tests.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Hook {
    Start,
    Stop,
    Bar(u32),
    Beat(u32, u32),
    BeforeSection { from: usize, to: usize },
    AfterSection { from: usize, to: usize },
    Chord,
    StyleLoaded,
}

impl Engine {
    #[cfg(test)]
    fn log(&mut self, h: Hook) {
        self.hook_log.push(h);
    }

    // ----- the hooks -----

    /// The band started (`start`): the first section is set up at position 0, nothing of
    /// it played yet.
    #[inline]
    pub(super) fn on_start(&mut self, _now: u64, _sink: &mut impl Sink) {
        #[cfg(test)]
        self.log(Hook::Start);
    }

    /// The band stopped: every note is off.
    #[inline]
    pub(super) fn on_stop(&mut self, _sink: &mut impl Sink) {
        #[cfg(test)]
        self.log(Hook::Stop);
        self.looper_on_stop();
        self.metronome_on_stop();
    }

    /// Bar `bar` (0-based in this pass of the section) begins; `on_beat` for its first beat
    /// follows.
    #[inline]
    pub(super) fn on_bar(&mut self, _bar: u32, now: u64, sink: &mut impl Sink) {
        #[cfg(test)]
        self.log(Hook::Bar(_bar));
        self.looper_on_bar(now, sink);
    }

    /// Beat `beat` (a quarter note, 0-based in the bar) of bar `bar` begins.
    #[inline]
    pub(super) fn on_beat(&mut self, _bar: u32, beat: u32, _now: u64, sink: &mut impl Sink) {
        #[cfg(test)]
        self.log(Hook::Beat(_bar, beat));
        self.metronome_beat(beat, sink);
    }

    /// A section boundary at tick `_at`: section slot `self.cur` hands over to slot `_to`
    /// (`usize::MAX`: the band stops; the same slot: a Main repeating). Nothing has changed
    /// yet: the old section's notes still sound.
    #[inline]
    pub(super) fn before_section_change(&mut self, _to: usize, _at: f64, _now: u64, _sink: &mut impl Sink) {
        #[cfg(test)]
        {
            let from = self.cur;
            self.log(Hook::BeforeSection { from, to: _to });
        }
    }

    /// The section changed from slot `_from` (to `self.cur`, which may be the same slot
    /// repeating): its setup has gone out, its first events have not played.
    #[inline]
    pub(super) fn after_section_change(&mut self, _from: usize, _now: u64, _sink: &mut impl Sink) {
        #[cfg(test)]
        {
            let to = self.cur;
            self.log(Hook::AfterSection { from: _from, to });
        }
    }

    /// The chord the style follows (`self.chord`) changed from `_prev`, and the band has
    /// followed it (started, re-voiced, Stop ACMP sounded).
    #[inline]
    pub(super) fn on_chord(&mut self, _prev: Option<Chord>, _now: u64, _sink: &mut impl Sink) {
        #[cfg(test)]
        self.log(Hook::Chord);
    }

    /// A new style (`self.style`) took over: its setup has gone out.
    #[inline]
    pub(super) fn on_style_loaded(&mut self, _now: u64, _sink: &mut impl Sink) {
        #[cfg(test)]
        self.log(Hook::StyleLoaded);
    }

    /// A tick (on the section's timeline) by which a feature needs `process` to run, if
    /// any: `next_deadline` wakes the engine for it. The metronome's next beat line, the
    /// Chord Looper's next chord change; with neither, the engine wakes only for pattern
    /// events and boundaries.
    #[inline]
    pub(super) fn hook_deadline(&self) -> Option<f64> {
        match (self.metronome_line(), self.hook_due()) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }

    /// The tick of a feature's next timed action between lines, for `on_due`.
    #[inline]
    pub(super) fn hook_due(&self) -> Option<f64> {
        self.looper_due()
    }

    /// `process` reached the tick `t` that `hook_due` named. Must move `hook_due` on.
    #[inline]
    pub(super) fn on_due(&mut self, t: f64, now: u64, sink: &mut impl Sink) {
        self.looper_play_due(t, now, sink);
    }

    // ----- the bar and beat lines -----

    /// Aim the bar and beat hooks at the first line at or after tick `t` of the section
    /// playing (a section change, a start, a style swap).
    pub(super) fn lines_from(&mut self, t: f64) {
        let (tpb, ppq) = (self.style.tpb.max(1) as f64, self.style.ppq.max(1) as f64);
        let rel = (t - self.sec_start).max(0.0);
        let mut bar = (rel / tpb + 1e-9).floor();
        let mut beat = ((rel - bar * tpb) / ppq - 1e-6).ceil().max(0.0);
        if beat * ppq >= tpb - 1e-6 {
            bar += 1.0;
            beat = 0.0;
        }
        self.lines = Lines { next: self.sec_start + bar * tpb + beat * ppq, bar: bar as u32, beat: beat as u32 };
    }

    /// The line `self.lines` has come: run its hooks and aim at the next one.
    pub(super) fn beat_line(&mut self, now: u64, sink: &mut impl Sink) {
        let Lines { bar, beat, .. } = self.lines;
        if beat == 0 {
            self.on_bar(bar, now, sink);
        }
        self.on_beat(bar, beat, now, sink);
        let (tpb, ppq) = (self.style.tpb.max(1) as f64, self.style.ppq.max(1) as f64);
        let off = (beat + 1) as f64 * ppq;
        let bar_start = self.sec_start + bar as f64 * tpb;
        self.lines = if off < tpb - 1e-6 {
            Lines { next: bar_start + off, bar, beat: beat + 1 }
        } else {
            Lines { next: bar_start + tpb, bar: bar + 1, beat: 0 }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Nop;
    impl Sink for Nop {
        fn send(&mut self, _: &[u8]) {}
    }

    fn engine(name: &str) -> Option<Engine> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2").join(name);
        if !p.exists() {
            eprintln!("corpus missing; skipping");
            return None;
        }
        Some(Engine::new(Box::new(Prepared::new(&Style::load(&p).unwrap()))))
    }

    /// Run the engine from `from` to `to` in 5 ms steps; after each, every line up to now
    /// has had its hooks.
    fn play(e: &mut Engine, from: u64, to: u64) {
        let mut now = from;
        while now < to {
            e.process(now, &mut Nop);
            if e.running {
                assert!(e.lines.next > e.tick_at(now) - 1e-6, "a line up to now was missed");
            }
            now += 5_000_000;
        }
    }

    #[test]
    fn hooks_run_in_order() {
        let Some(mut e) = engine("SlowWalker.T552.sty") else { return };
        let bar = e.ns_at_bar(1);
        e.set_chord(crate::parse_chord("C").unwrap(), 0, &mut Nop);
        assert_eq!(e.hook_log[..4], [Hook::Start, Hook::Bar(0), Hook::Beat(0, 0), Hook::Chord]);
        play(&mut e, 0, bar + bar / 3);
        // Main B in bar 2 (Auto Fill on): its fill at the next beat, then Main B.
        e.button(Button::Main(1), bar + bar / 3, &mut Nop);
        play(&mut e, bar + bar / 3, 5 * bar);
        e.set_chord(crate::parse_chord("F").unwrap(), 5 * bar, &mut Nop);
        e.button(Button::StartStop, 5 * bar, &mut Nop);
        e.stop(&mut Nop); // already stopped: no second Stop

        let log = &e.hook_log;
        let pos = |h: Hook| log.iter().position(|&x| x == h).unwrap_or_else(|| panic!("{h:?} missing: {log:?}"));
        let fill = slot_of(SectionId::Fill(1));
        let main_b = slot_of(SectionId::Main(1));
        let to_fill = pos(Hook::BeforeSection { from: 4, to: fill });
        assert_eq!(log[to_fill + 1], Hook::AfterSection { from: 4, to: fill });
        // The fill came in mid-bar: its first line is a beat of its bar, not a bar line.
        assert!(matches!(log[to_fill + 2], Hook::Beat(0, b) if b > 0), "{:?}", &log[to_fill..]);
        let to_b = pos(Hook::BeforeSection { from: fill, to: main_b });
        assert_eq!(log[to_b + 1..to_b + 4], [Hook::AfterSection { from: fill, to: main_b }, Hook::Bar(0), Hook::Beat(0, 0)]);
        assert_eq!(log.iter().filter(|&&h| h == Hook::Stop).count(), 1);
        assert_eq!(log.last(), Some(&Hook::Stop));
        assert_eq!(log.iter().filter(|&&h| h == Hook::Chord).count(), 2);
        // Bars and beats count on between section changes, each bar line with its first
        // beat.
        let mut last: Option<(u32, u32)> = None;
        for w in log.windows(2) {
            match w {
                [Hook::Bar(b), next] => assert_eq!(*next, Hook::Beat(*b, 0)),
                [Hook::AfterSection { .. } | Hook::Start, _] => last = None,
                _ => {}
            }
            if let Hook::Beat(b, k) = w[1] {
                if let Some((pb, pk)) = last {
                    assert!((b, k) == (pb, pk + 1) || (b, k) == (pb + 1, 0), "{:?} after {:?}", (b, k), (pb, pk));
                }
                last = Some((b, k));
            }
        }
    }

    #[test]
    fn a_style_change_is_reported() {
        let (Some(mut e), Some(other)) = (engine("SlowWalker.T552.sty"), engine("TickingAway.T162.sty")) else { return };
        let _old = e.load(other.style, 0, &mut Nop);
        assert_eq!(e.hook_log, [Hook::StyleLoaded]);
    }

    /// Every queued change asks `change_point`: sections at the next bar, fills at the
    /// next beat (counting from their bar).
    #[test]
    fn change_points() {
        let Some(mut e) = engine("SlowWalker.T552.sty") else { return };
        e.set_chord(crate::parse_chord("C").unwrap(), 0, &mut Nop);
        let (tpb, ppq) = (e.style.tpb as f64, e.style.ppq as f64);
        let now = e.ns_at(tpb + 1.5 * ppq);
        assert_eq!(e.change_point(Change::Section, now), (2.0 * tpb, 2.0 * tpb));
        assert_eq!(e.change_point(Change::Style, now), (2.0 * tpb, 2.0 * tpb));
        assert_eq!(e.change_point(Change::Fill, now), (tpb + 2.0 * ppq, tpb));
    }
}

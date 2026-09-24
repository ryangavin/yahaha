//! Chart player: the band takes its chords (and its Main sections) from an iReal Pro chart
//! expanded into bars ([`ChartPlan`], built on the control side by `session/chart.rs`),
//! in time, instead of from the left hand. docs/ireal.md ("Chart player") has the rules.
//!
//! - One chart bar is one bar of the style. A chart bar's chords go in on their beats; a
//!   beat the style's bar doesn't have (a 4/4 chart on a 3/4 style) lands on its last beat.
//! - Chart sections A-D play Main A-D; each section mark restarts its Main at its first bar,
//!   so the style's phrases line up with the chart's. With Auto Fill on, the bar before a
//!   section mark plays the new Main's fill.
//! - An Intro (the chart's setting, or the one the player pressed) plays before the first
//!   bar; after the last bar the Ending plays (or the band stops, with no Ending). A loop
//!   range plays over and over instead, until the player stops or presses an Ending.
//! - A chord the player plays overrides the chart until the next bar line, where the chart
//!   takes over again.
//!
//! Real-time: the plan arrives in a `Box` built on the control side (`live::EngineIo::charts`),
//! and the one it replaces goes back out to be freed there (`Engine::set_chart` returns it).
//! Nothing here allocates.

use super::*;

/// Chords a plan bar holds; more on one bar are dropped (iReal writes at most 4 cells a
/// beat, so 8 is already generous).
pub const CHART_CHORDS: usize = 8;

/// One chart bar as the engine plays it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanBar {
    /// Main 0-3 (A-D) this bar plays.
    pub main: u8,
    /// A section mark (or the top of a chorus) is on this bar: its Main starts again here,
    /// and a fill leads into it.
    pub section_start: bool,
    /// The chord in effect as the bar begins (its beat-1 chord, or the one held from
    /// before); None before the chart's first chord.
    pub enter: Option<Chord>,
    /// Chords on beats (0-based beat, chord), in beat order; N.C. is `theory::CANCEL`.
    pub chords: [(u8, Chord); CHART_CHORDS],
    pub n: u8,
}

impl PlanBar {
    pub const EMPTY: PlanBar = PlanBar { main: 0, section_start: false, enter: None, chords: [(0, Chord::new(0, 0)); CHART_CHORDS], n: 0 };

    /// The chord in effect at `beat`, where `last` means the style's bar ends at this beat
    /// (chords on later beats of the chart bar land here).
    pub fn chord_at(&self, beat: u32, last: bool) -> Option<Chord> {
        self.chords[..self.n as usize]
            .iter()
            .rev()
            .find(|(b, _)| last || *b as u32 <= beat)
            .map(|&(_, c)| c)
            .or(self.enter)
    }
}

/// A chart expanded for playing: built (and freed) on the control side.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChartPlan {
    /// Identifies the plan in snapshots (`Snapshot::chart_tag`).
    pub tag: u64,
    pub bars: Vec<PlanBar>,
    /// The chart's tempo: set when the plan arrives with the band stopped.
    pub bpm: Option<f64>,
    /// A new song (not the same one with another chorus count): a band that plays starts it
    /// from its first bar at the next bar line.
    pub fresh: bool,
}

/// The Main a chart section plays: A-D are Main A-D; any other section (a verse, the
/// chart's own intro) keeps the Main before it.
pub fn section_main(section: Option<char>, prev: u8) -> u8 {
    match section {
        Some(c @ 'A'..='D') => c as u8 - b'A',
        _ => prev,
    }
}

impl ChartPlan {
    /// The plan for an expanded chart (`ireal::expand`). A bar with a section mark, and the
    /// top of each chorus after the first, starts a section.
    pub fn from_bars(bars: &[crate::ireal::Bar], tag: u64, bpm: Option<f64>) -> ChartPlan {
        let mut main = 0;
        let mut held: Option<Chord> = None;
        let out = bars
            .iter()
            .enumerate()
            .map(|(i, b)| {
                main = section_main(b.section, main);
                let new_chorus = i > 0 && bars[i - 1].chorus != b.chorus;
                let mut p = PlanBar { main, section_start: (b.section_start || new_chorus) && i > 0, enter: held, ..PlanBar::EMPTY };
                for c in b.chords.iter().take(CHART_CHORDS) {
                    p.chords[p.n as usize] = (c.beat, c.chord);
                    p.n += 1;
                    held = Some(c.chord);
                }
                if let Some(&(0, c)) = p.chords[..p.n as usize].first() {
                    p.enter = Some(c);
                }
                p
            })
            .collect();
        ChartPlan { tag, bars: out, bpm, fresh: false }
    }
}

/// Chart player settings (a `live::Cmd`, so `Copy`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChartSettings {
    /// Chart mode: the band follows the chart while it plays.
    pub on: bool,
    /// Intro 0-2 (A-C) before the chart; None: straight into the first bar.
    pub intro: Option<u8>,
    /// Ending 0-2 after the chart; None: the band stops at the end of the last bar.
    pub ending: Option<u8>,
    /// Plan bars [start, end) played over and over instead of ending.
    pub loop_range: Option<(u32, u32)>,
}

/// The chart player's engine-side state (one field of `hooks::Features`).
#[derive(Default)]
pub(super) struct ChartPlayer {
    plan: Option<Box<ChartPlan>>,
    settings: ChartSettings,
    /// The plan bar playing; None before the first (stopped, or in the Intro).
    bar: Option<u32>,
    /// The player's chord is in charge until the next bar line.
    overridden: bool,
    /// The chord the chart last gave the band (as written, before Keyboard transpose): a
    /// chord change to anything else is the player's.
    applied: Option<Chord>,
    /// The time of the last start: a Sync Start chord only starts the band.
    start_ns: Option<u64>,
    /// The section change the chart queued for the next bar line (a Main, a fill, the
    /// Ending or the stop), and the Main selected before it: a new plan or new settings
    /// take it back.
    owned: Option<(Queued, u8)>,
}

impl Engine {
    /// The chart is driving the band: chart mode on, and a plan with bars.
    #[inline]
    fn chart_active(&self) -> bool {
        let c = &self.features.chart;
        c.settings.on && c.plan.as_ref().is_some_and(|p| !p.bars.is_empty())
    }

    #[inline]
    fn plan_bar(&self, i: u32) -> Option<&PlanBar> {
        self.features.chart.plan.as_ref()?.bars.get(i as usize)
    }

    /// The bar after plan bar `i` (following the loop), None after the last.
    fn chart_next(&self, i: u32) -> Option<u32> {
        let c = &self.features.chart;
        let n = c.plan.as_ref().map_or(0, |p| p.bars.len()) as u32;
        if let Some((a, _)) = c.settings.loop_range.filter(|&(a, b)| a < b && b <= n && i + 1 >= b) {
            return Some(a);
        }
        (i + 1 < n).then_some(i + 1)
    }

    /// A new plan; returns the one it replaces (for the caller to free off the real-time
    /// thread). A band that plays keeps its bar (clamped to the new plan); a stopped one
    /// takes the chart's tempo.
    pub fn set_chart(&mut self, plan: Box<ChartPlan>, now: u64) -> Option<Box<ChartPlan>> {
        if let (false, Some(bpm)) = (self.running, plan.bpm) {
            self.set_bpm_internal(bpm, now);
        }
        // What the old plan queued for the next bar line is not the new plan's.
        self.chart_unqueue();
        let len = plan.bars.len() as u32;
        let fresh = plan.fresh;
        let c = &mut self.features.chart;
        if fresh {
            // A new song starts from its first bar.
            c.bar = None;
            c.overridden = false;
        } else if let Some(b) = c.bar {
            c.bar = (len > 0).then(|| b.min(len - 1));
        }
        let old = c.plan.replace(plan);
        if fresh && self.chart_active() && matches!(id_of(self.cur), SectionId::Intro(_)) {
            // The Intro goes on to the new song's first Main.
            if let Some(b) = self.plan_bar(0) {
                self.main = b.main;
            }
        }
        self.chart_requeue(now);
        old
    }

    /// New chart settings. Turning chart mode off hands the chords back to the player (and
    /// takes back what the chart queued); new settings (a loop, an Ending) queue the next
    /// bar line again.
    pub fn set_chart_settings(&mut self, s: ChartSettings, now: u64) {
        self.chart_unqueue();
        let c = &mut self.features.chart;
        c.settings = s;
        if !s.on {
            c.bar = None;
            c.overridden = false;
        }
        self.chart_requeue(now);
    }

    /// Take back the change the chart queued, if it is still the one queued (the player may
    /// have queued another since), and the Main it selected.
    fn chart_unqueue(&mut self) {
        let Some((q, main)) = self.features.chart.owned.take() else { return };
        if self.queued.is_some_and(|cur| id_of(cur.slot) == id_of(q.slot) || cur.slot == usize::MAX && q.slot == usize::MAX) {
            self.queued = None;
            self.main = main;
        }
    }

    /// Queue the next bar line's change again (a new plan or new settings mid-bar), while
    /// the chart plays a Main, fill or break.
    fn chart_requeue(&mut self, now: u64) {
        if !self.running || !self.chart_active() || self.queued.is_some() {
            return;
        }
        if matches!(id_of(self.cur), SectionId::Intro(_) | SectionId::Ending(_)) {
            return;
        }
        let end = self.next_bar(now);
        self.chart_queue(end);
    }

    /// (tag of the plan, bar playing, player override) for the snapshot.
    pub(super) fn chart_pos(&self) -> (u64, Option<u32>, bool) {
        let c = &self.features.chart;
        let active = self.running && self.chart_active();
        (c.plan.as_ref().map_or(0, |p| p.tag), c.bar.filter(|_| active), c.overridden && active)
    }

    /// Give the band a chart chord (as written; Keyboard transpose moves it as it moves a
    /// played chord).
    fn chart_chord(&mut self, c: Option<Chord>, now: u64, sink: &mut impl Sink) {
        let Some(c) = c else { return };
        if self.played == Some(c) {
            self.features.chart.applied = Some(c);
            return;
        }
        self.features.chart.applied = Some(c);
        self.set_chord(c, now, sink);
    }

    // ----- hooks -----

    /// `on_start`: the Intro (unless the player chose one), the first bar's Main, and the
    /// first chord, so a START with no chord held plays the song's.
    pub(super) fn chart_start(&mut self, now: u64, sink: &mut impl Sink) {
        let c = &mut self.features.chart;
        c.bar = None;
        c.overridden = false;
        c.owned = None;
        c.start_ns = Some(now);
        if !self.chart_active() {
            return;
        }
        let Some(&first) = self.plan_bar(0) else { return };
        self.main = first.main;
        let intro = self.features.chart.settings.intro.and_then(|i| self.style.resolve(i as usize));
        match (id_of(self.cur), intro) {
            (SectionId::Intro(_), _) => {}
            (_, Some(slot)) => self.cur = slot,
            (_, None) => {
                if let Some(slot) = self.style.resolve(4 + first.main as usize) {
                    self.cur = slot;
                }
            }
        }
        let first_chord = self.features.chart.plan.as_ref().and_then(|p| p.bars.iter().find_map(|b| b.enter));
        self.chart_chord(first_chord, now, sink);
    }

    /// `on_stop`.
    pub(super) fn chart_stop(&mut self) {
        let c = &mut self.features.chart;
        c.bar = None;
        c.overridden = false;
        c.owned = None;
    }

    /// `on_bar`: the chart moves on a bar (outside an Intro or Ending), the player's
    /// override ends, and the next section change is queued for its bar line.
    pub(super) fn chart_bar(&mut self, bar: u32, now: u64, sink: &mut impl Sink) {
        if !self.chart_active() {
            return;
        }
        let was_overridden = std::mem::take(&mut self.features.chart.overridden);
        match id_of(self.cur) {
            SectionId::Ending(_) => return,
            SectionId::Intro(_) => {
                if was_overridden {
                    let c = self.plan_bar(0).and_then(|b| b.enter);
                    self.chart_chord(c, now, sink);
                }
                return;
            }
            _ => {}
        }
        let idx = match self.features.chart.bar {
            None => 0,
            Some(i) => match self.chart_next(i) {
                Some(n) => n,
                None => return,
            },
        };
        self.features.chart.bar = Some(idx);
        let tpb = self.style.tpb.max(1) as f64;
        let end = self.sec_start + (bar as f64 + 1.0) * tpb;
        self.features.chart.owned = None;
        self.chart_queue(end);
    }

    /// Queue the change for the bar line at tick `end`, after the plan bar playing (None:
    /// before the first): the next bar's Main when a section starts there (a section mark,
    /// the top of a chorus, the top of the loop, the chart's first bar), else, with Auto
    /// Fill on, the fill when one starts the bar after; the Ending (or the stop) after the
    /// last bar.
    fn chart_queue(&mut self, end: f64) {
        let cur = self.features.chart.bar;
        let n1 = match cur {
            None => Some(0),
            Some(i) => self.chart_next(i),
        };
        let n2 = n1.and_then(|n| self.chart_next(n));
        // A section starts on bar `to` coming from `from`: its mark, or a jump back.
        let starts = |e: &Engine, from: Option<u32>, to: u32| -> bool {
            from.is_none_or(|f| to <= f) || e.plan_bar(to).is_some_and(|b| b.section_start)
        };
        let (Some(b1), b2) = (n1.and_then(|n| self.plan_bar(n)).copied(), n2.and_then(|n| self.plan_bar(n)).copied()) else {
            // The last bar: the Ending (or the stop) at its end.
            let ending = self.features.chart.settings.ending.and_then(|e| self.style.resolve(13 + e as usize));
            self.chart_own(Queued { slot: ending.unwrap_or(usize::MAX), at: end, sec_start: end }, self.main);
            return;
        };
        let (n1, n2) = (n1.unwrap_or(0), n2.unwrap_or(0));
        if starts(self, cur, n1) {
            // The new section's Main from its first bar (a fill queued for this bar hands
            // over to it as well).
            if let Some(slot) = self.style.resolve(4 + b1.main as usize) {
                let before = self.main;
                self.main = b1.main;
                self.chart_own(Queued { slot, at: end, sec_start: end }, before);
            }
        } else if let Some(b2) = b2.filter(|_| starts(self, Some(n1), n2) && self.auto_fill) {
            // Auto Fill: the next bar is the new section's fill.
            if let Some(fill) = self.style.resolve(8 + b2.main as usize) {
                let before = self.main;
                self.main = b2.main;
                self.chart_own(Queued { slot: fill, at: end, sec_start: end }, before);
            }
        }
    }

    fn chart_own(&mut self, q: Queued, main_before: u8) {
        self.queued = Some(q);
        self.features.chart.owned = Some((q, main_before));
    }

    /// `on_beat`: the chart's chord for this beat, unless the player has taken over.
    pub(super) fn chart_beat(&mut self, beat: u32, now: u64, sink: &mut impl Sink) {
        if !self.chart_active() || self.features.chart.overridden {
            return;
        }
        let Some(i) = self.features.chart.bar else { return };
        if matches!(id_of(self.cur), SectionId::Intro(_) | SectionId::Ending(_)) {
            return;
        }
        let beats = (self.style.tpb / self.style.ppq.max(1)).max(1);
        let c = self.plan_bar(i).and_then(|b| b.chord_at(beat, beat + 1 >= beats));
        self.chart_chord(c, now, sink);
    }

    /// `on_chord`: a chord that isn't the chart's is the player's; it holds until the next
    /// bar line. A Sync Start chord only starts the band.
    pub(super) fn chart_chord_changed(&mut self, now: u64) {
        let c = &mut self.features.chart;
        if !self.running || !c.settings.on || self.played == c.applied || c.start_ns == Some(now) {
            return;
        }
        c.overridden = true;
    }

    /// `hook_deadline`: every beat line while the chart plays, so its chords land on time.
    #[inline]
    pub(super) fn chart_deadline(&self) -> Option<f64> {
        (self.running && self.chart_active()).then_some(self.lines.next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ireal::{expand, parse_chart};

    struct Nop;
    impl Sink for Nop {
        fn send(&mut self, _: &[u8]) {}
    }

    fn engine() -> Option<Engine> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
        if !p.exists() {
            eprintln!("corpus missing; skipping");
            return None;
        }
        Some(Engine::new(Box::new(Prepared::new(&Style::load(&p).unwrap()))))
    }

    fn plan(chart: &str, choruses: u32) -> Box<ChartPlan> {
        Box::new(ChartPlan::from_bars(&expand(&parse_chart(chart), choruses), 1, None))
    }

    fn settings(intro: Option<u8>, ending: Option<u8>) -> ChartSettings {
        ChartSettings { on: true, intro, ending, loop_range: None }
    }

    /// What the band played at a moment: the plan bar, the section, the chord, the beat.
    #[derive(Debug, Clone, PartialEq)]
    struct Line {
        bar: Option<u32>,
        section: String,
        chord: String,
        beat: u32,
        /// The bar of the section playing.
        sbar: u32,
    }

    /// Play from `from` for `bars` bars (of the style) in 1/8-beat steps, noting each change.
    fn play(e: &mut Engine, from: u64, bars: u32) -> (Vec<Line>, u64) {
        let bar = e.ns_at_bar(1) - e.ns_at_bar(0);
        let beat = bar / (e.style.tpb / e.style.ppq) as u64;
        let mut out: Vec<Line> = Vec::new();
        let mut now = from;
        let end = from + bars as u64 * bar;
        while now < end && (e.running || out.is_empty()) {
            e.process(now, &mut Nop);
            let s = e.snapshot(now);
            let l = Line {
                bar: s.chart_bar,
                section: s.cur.map_or("-".into(), |c| c.name()),
                chord: s.played.map_or("-".into(), |c| c.name()),
                beat: s.beat,
                sbar: s.bar,
            };
            if out.last() != Some(&l) {
                out.push(l);
            }
            now += beat / 8;
        }
        (out, now)
    }

    fn start(e: &mut Engine) {
        e.button(Button::StartStop, 0, &mut Nop);
    }

    fn at(lines: &[Line], bar: u32) -> Vec<Line> {
        lines.iter().filter(|l| l.bar == Some(bar)).cloned().collect()
    }

    /// Sections A/B play Main A/B, a fill leads into the section mark (Auto Fill on), and
    /// the chart's chords land on their beats.
    #[test]
    fn sections_fills_and_chords_follow_the_chart() {
        let Some(mut e) = engine() else { return };
        e.set_chart(plan("*A[C |D-7 G7 |C |C ]*B[F |F |G |G Z", 1), 0);
        e.set_chart_settings(settings(None, None), 0);
        start(&mut e);
        let (lines, _) = play(&mut e, 0, 10);
        assert_eq!(at(&lines, 0)[0].section, "Main A");
        assert_eq!(at(&lines, 0)[0].chord, "C");
        // D-7 on beat 1, G7 on beat 3 of bar 2.
        let b1 = at(&lines, 1);
        assert_eq!((b1[0].chord.as_str(), b1[0].beat), ("Dm7", 0), "{b1:?}");
        assert!(b1.iter().any(|l| l.chord == "G7" && l.beat == 2), "{b1:?}");
        // The bar before B is Main B's fill; B starts Main B.
        assert!(at(&lines, 3).iter().all(|l| l.section == "Fill In BB"), "{:?}", at(&lines, 3));
        assert!(at(&lines, 4).iter().all(|l| l.section == "Main B"), "{:?}", at(&lines, 4));
        assert_eq!(at(&lines, 4)[0].chord, "F");
        // No Ending: the band stops at the end of the last bar.
        assert!(!e.running);
        assert_eq!(lines.iter().filter_map(|l| l.bar).max(), Some(7));
    }

    /// Without Auto Fill the section changes on its bar line, with no fill.
    #[test]
    fn no_fill_without_auto_fill() {
        let Some(mut e) = engine() else { return };
        e.button(Button::AutoFill, 0, &mut Nop);
        e.set_chart(plan("*A[C |C ]*B[F |F Z", 1), 0);
        e.set_chart_settings(settings(None, None), 0);
        start(&mut e);
        let (lines, _) = play(&mut e, 0, 5);
        assert!(lines.iter().all(|l| !l.section.starts_with("Fill")), "{lines:?}");
        assert!(at(&lines, 2).iter().all(|l| l.section == "Main B"));
    }

    /// An Intro before the chart (the song's first chord already sounding), an Ending after.
    #[test]
    fn intro_before_and_ending_after() {
        let Some(mut e) = engine() else { return };
        e.set_chart(plan("*A[F |B-7 E7 Z", 1), 0);
        e.set_chart_settings(settings(Some(0), Some(0)), 0);
        start(&mut e);
        let (lines, _) = play(&mut e, 0, 20);
        assert_eq!(lines[0].section, "Intro A");
        assert_eq!(lines[0].chord, "F");
        assert!(lines.iter().filter(|l| l.section == "Intro A").all(|l| l.bar.is_none()));
        let first = lines.iter().position(|l| l.bar == Some(0)).unwrap();
        assert_eq!(lines[first].section, "Main A");
        let end = lines.iter().position(|l| l.section == "Ending A").expect("an Ending");
        assert!(lines[..end].iter().any(|l| l.bar == Some(1) && l.chord == "E7"));
        assert!(!e.running, "the Ending stopped the band");
    }

    /// A chord from the player takes over until the next bar line; then the chart goes on.
    #[test]
    fn the_left_hand_overrides_until_the_next_bar() {
        let Some(mut e) = engine() else { return };
        e.set_chart(plan("*A[C |C |F |F Z", 1), 0);
        e.set_chart_settings(settings(None, None), 0);
        start(&mut e);
        let bar = e.ns_at_bar(1);
        // Mid bar 2 the player plays Ab.
        let t = bar + bar / 2;
        let _ = play(&mut e, 0, 1);
        e.process(t, &mut Nop);
        e.set_chord(crate::parse_chord("Ab").unwrap(), t, &mut Nop);
        let s = e.snapshot(t);
        assert!(s.chart_override);
        assert_eq!(s.chart_bar, Some(1));
        assert_eq!(s.played.unwrap().name(), "Ab");
        let (lines, _) = play(&mut e, t, 2);
        assert!(at(&lines, 1).iter().all(|l| l.chord == "Ab"), "{lines:?}");
        assert_eq!(at(&lines, 2)[0].chord, "F");
        assert!(!e.snapshot(e.ns_at_bar(2) + 1).chart_override);
    }

    /// A Sync Start chord starts the band on the chart's chord, with no override.
    #[test]
    fn a_sync_start_chord_only_starts() {
        let Some(mut e) = engine() else { return };
        e.set_chart(plan("*A[E-7 |A7 Z", 1), 0);
        e.set_chart_settings(settings(None, None), 0);
        e.set_chord(crate::parse_chord("C").unwrap(), 0, &mut Nop);
        let s = e.snapshot(0);
        assert!(s.running);
        assert!(!s.chart_override);
        assert_eq!(s.played.unwrap().name(), "Em7");
    }

    /// Keyboard transpose moves the chart's chords.
    #[test]
    fn keyboard_transpose_applies_to_the_chart() {
        let Some(mut e) = engine() else { return };
        e.set_chart(plan("*A[C |F Z", 1), 0);
        e.set_chart_settings(settings(None, None), 0);
        e.set_transpose(Transpose::new(2, 0), 0, &mut Nop);
        start(&mut e);
        let s = e.snapshot(0);
        assert_eq!(s.chord.unwrap().name(), "D");
        assert!(!s.chart_override, "a transpose is not the player's chord");
        let bar = e.ns_at_bar(1);
        let _ = play(&mut e, 0, 1);
        e.process(bar + bar / 4, &mut Nop);
        assert_eq!(e.chord.unwrap().name(), "G");
        e.set_transpose(Transpose::new(-1, 0), bar + bar / 2, &mut Nop);
        assert_eq!(e.chord.unwrap().name(), "E");
        assert!(!e.snapshot(bar + bar / 2).chart_override);
    }

    /// A loop plays its bars over and over, with no Ending.
    #[test]
    fn a_loop_repeats() {
        let Some(mut e) = engine() else { return };
        e.set_chart(plan("*A[C |D |E |F Z", 1), 0);
        e.set_chart_settings(ChartSettings { loop_range: Some((1, 3)), ..settings(None, Some(0)) }, 0);
        start(&mut e);
        let (lines, _) = play(&mut e, 0, 9);
        let mut bars: Vec<u32> = lines.iter().filter_map(|l| l.bar).collect();
        bars.dedup();
        assert_eq!(bars, [0, 1, 2, 1, 2, 1, 2, 1, 2]);
        assert!(e.running);
    }

    /// Choruses: the form plays again, with a fill into the top of each chorus.
    #[test]
    fn choruses_play_the_form_again() {
        let Some(mut e) = engine() else { return };
        e.set_chart(plan("*A[C |G Z", 2), 0);
        e.set_chart_settings(settings(None, None), 0);
        start(&mut e);
        let (lines, _) = play(&mut e, 0, 6);
        assert_eq!(lines.iter().filter_map(|l| l.bar).max(), Some(3));
        assert!(at(&lines, 1).iter().all(|l| l.section == "Fill In AA"), "{lines:?}");
    }

    /// Chart mode off: the chart does nothing.
    #[test]
    fn off_leaves_the_left_hand_in_charge() {
        let Some(mut e) = engine() else { return };
        e.set_chart(plan("*A[C |G Z", 1), 0);
        e.set_chord(crate::parse_chord("Bb").unwrap(), 0, &mut Nop);
        let (lines, _) = play(&mut e, 0, 3);
        assert!(lines.iter().all(|l| l.chord == "Bb" && l.bar.is_none()), "{lines:?}");
    }

    /// The top of a loop restarts its Main: a whole-song loop on an AB chart goes back to
    /// Main A (with Auto Fill, through its fill), and a loop inside a section restarts the
    /// section's phrase on each pass.
    #[test]
    fn the_loop_top_restarts_its_main() {
        let Some(mut e) = engine() else { return };
        e.set_chart(plan("*A[C |C ]*B[F |F Z", 1), 0);
        e.set_chart_settings(ChartSettings { loop_range: Some((0, 4)), ..settings(None, None) }, 0);
        start(&mut e);
        let (lines, _) = play(&mut e, 0, 9);
        let pass2 = lines.iter().position(|l| l.bar == Some(3)).unwrap();
        let top: Vec<_> = lines[pass2..].iter().filter(|l| l.bar == Some(0)).collect();
        assert!(!top.is_empty() && top.iter().all(|l| l.section == "Main A"), "{lines:?}");
        assert_eq!(top[0].sbar, 0, "{lines:?}");
        assert!(at(&lines, 3).iter().all(|l| l.section == "Fill In AA"), "{lines:?}");

        // Auto Fill off: a loop of three bars in a long section restarts the Main each pass.
        let Some(mut e) = engine() else { return };
        e.button(Button::AutoFill, 0, &mut Nop);
        e.set_chart(plan("*A[C |D |E |F |G |A |B |C Z", 1), 0);
        e.set_chart_settings(ChartSettings { loop_range: Some((0, 3)), ..settings(None, None) }, 0);
        start(&mut e);
        let (lines, _) = play(&mut e, 0, 10);
        let tops: Vec<_> = lines.iter().filter(|l| l.bar == Some(0)).map(|l| (l.sbar, l.section.clone())).collect();
        assert!(tops.len() > 3, "{lines:?}");
        assert!(tops.iter().all(|(b, s)| *b == 0 && s == "Main A"), "{tops:?}");
    }

    /// Chart mode off in the last bar: the stop the chart queued for its end goes too.
    #[test]
    fn chart_mode_off_takes_back_the_queued_stop() {
        let Some(mut e) = engine() else { return };
        e.set_chart(plan("*A[C |F Z", 1), 0);
        e.set_chart_settings(settings(None, Some(0)), 0);
        start(&mut e);
        let bar = e.ns_at_bar(1);
        let (_, now) = play(&mut e, 0, 1);
        let t = now + bar / 4;
        e.process(t, &mut Nop);
        assert_eq!(e.snapshot(t).chart_bar, Some(1));
        assert!(matches!(e.snapshot(t).queued, Some(SectionId::Ending(0))));
        e.set_chart_settings(ChartSettings { on: false, ..settings(None, Some(0)) }, t);
        assert!(e.snapshot(t).queued.is_none());
        let (lines, _) = play(&mut e, t, 3);
        assert!(e.running, "{lines:?}");
        assert!(lines.iter().all(|l| l.section == "Main A"), "{lines:?}");
    }

    /// More choruses (the same song, a longer plan) in the last bar: the band plays on
    /// into the next chorus instead of the old plan's Ending; a loop set there does too.
    #[test]
    fn a_new_plan_or_loop_in_the_last_bar_replans_its_end() {
        let Some(mut e) = engine() else { return };
        e.set_chart(plan("*A[C |F Z", 1), 0);
        e.set_chart_settings(settings(None, Some(0)), 0);
        start(&mut e);
        let bar = e.ns_at_bar(1);
        let (_, now) = play(&mut e, 0, 1);
        let t = now + bar / 4;
        e.process(t, &mut Nop);
        assert_eq!(e.snapshot(t).chart_bar, Some(1));
        let old = e.set_chart(plan("*A[C |F Z", 2), t);
        assert!(old.is_some());
        assert!(!matches!(e.snapshot(t).queued, Some(SectionId::Ending(_))));
        let (lines, now) = play(&mut e, t, 2);
        assert!(e.running, "{lines:?}");
        assert!(at(&lines, 2).iter().all(|l| l.section == "Main A" && l.chord == "C"), "{lines:?}");
        // In the new last bar, a loop over the song replaces the Ending queued for its end.
        let t = now;
        e.process(t, &mut Nop);
        assert_eq!(e.snapshot(t).chart_bar, Some(3));
        e.set_chart_settings(ChartSettings { loop_range: Some((0, 4)), ..settings(None, Some(0)) }, t);
        let (lines, _) = play(&mut e, t, 2);
        assert!(e.running, "{lines:?}");
        assert!(lines.iter().any(|l| l.bar == Some(0)), "{lines:?}");
        assert!(lines.iter().all(|l| !l.section.starts_with("Ending")), "{lines:?}");
    }

    /// A new song mid-play starts from its first bar (its Main) at the next bar line.
    #[test]
    fn a_new_song_mid_play_starts_at_its_first_bar() {
        let Some(mut e) = engine() else { return };
        e.button(Button::AutoFill, 0, &mut Nop);
        e.set_chart(plan("*A[C |C |C ]*B[F |F |F |F Z", 1), 0);
        e.set_chart_settings(settings(None, None), 0);
        start(&mut e);
        let bar = e.ns_at_bar(1);
        let (_, now) = play(&mut e, 0, 4);
        let t = now + bar / 4;
        e.process(t, &mut Nop);
        assert_eq!(e.snapshot(t).chart_bar, Some(4));
        assert_eq!(e.snapshot(t).cur, Some(SectionId::Main(1)));
        let mut song = plan("*A[G |D Z", 1);
        song.fresh = true;
        song.tag = 2;
        e.set_chart(song, t);
        let (lines, _) = play(&mut e, t, 2);
        let first = lines.iter().position(|l| l.bar == Some(0)).expect("the new song's first bar");
        assert_eq!((lines[first].section.as_str(), lines[first].chord.as_str(), lines[first].sbar), ("Main A", "G", 0), "{lines:?}");
    }

    #[test]
    fn plan_bars_carry_sections_and_held_chords() {
        let p = plan("*A[C |x |D-7 G7 ]*B[G7 |*C C Z", 1);
        let mains: Vec<_> = p.bars.iter().map(|b| (b.main, b.section_start)).collect();
        assert_eq!(mains, [(0, false), (0, false), (0, false), (1, true), (2, true)]);
        assert_eq!(p.bars[1].enter.unwrap().name(), "C");
        assert_eq!(p.bars[2].chord_at(1, false).unwrap().name(), "Dm7");
        assert_eq!(p.bars[2].chord_at(2, false).unwrap().name(), "G7");
        // A 4/4 chart bar on a 2-beat style bar: beat 3's chord lands on the last beat.
        assert_eq!(p.bars[2].chord_at(1, true).unwrap().name(), "G7");
        assert_eq!(section_main(Some('V'), 2), 2);
    }
}

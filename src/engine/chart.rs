//! Chart player: the band takes its chords (and its Main sections) from an iReal Pro chart
//! expanded into bars ([`ChartPlan`], built on the control side by `session/chart.rs`),
//! in time, instead of from the left hand. docs/ireal.md ("Chart player") has the rules.
//!
//! - One chart bar is one bar of the style. A chart bar's chords go in at their place in
//!   the bar: a chord on beat `b` of an `n`-beat chart bar at `b/n` of the style's bar,
//!   whatever the chart's beat unit (a 2/2 chart's half-bar chord on quarter 3 of a 4/4
//!   style, a 6/8 chart's second eighth at 1/6 of the bar). The engine wakes at each chord
//!   (`hook_due`), so a chord between the style's quarter lines lands on time; the
//!   lead-sheet band draws it at the same place.
//! - Chart sections A-D play Main A-D; each section mark restarts its Main at its first bar,
//!   so the style's phrases line up with the chart's. With Auto Fill on, the bar before a
//!   section mark plays the new Main's fill.
//! - An Intro (the chart's setting, or the one the player pressed) plays before the first
//!   bar; after the last bar the Ending plays (or the band stops, with no Ending). A loop
//!   range plays over and over instead, until the player stops or presses an Ending.
//! - A chord the player plays overrides the chart until the next bar line, where the chart
//!   takes over again. A chord played in the last eighth of a bar (half a beat before the
//!   line) is an anticipation of the next bar: it holds through that bar as well.
//!
//! Real-time: the plan arrives in a `Box` built on the control side (`live::EngineIo::charts`),
//! and the one it replaces goes back out to be freed there (`Engine::set_chart` returns it).
//! Nothing here allocates.

use super::*;

/// Chords a plan bar holds; more on one bar are dropped (iReal writes at most 4 cells a
/// beat, so 8 is already generous).
pub const CHART_CHORDS: usize = 8;

/// A player's chord this close to the next bar line (in beats) is meant for the bar after
/// it: the override carries over that line.
pub const ANTICIPATE_BEATS: f64 = 0.5;

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
    /// Chords at their place in the bar (the fraction of the bar before them, 0 <= x < 1,
    /// from the chart's beat and time signature), in order; N.C. is `theory::CANCEL`.
    pub chords: [(f32, Chord); CHART_CHORDS],
    pub n: u8,
}

impl PlanBar {
    pub const EMPTY: PlanBar = PlanBar { main: 0, section_start: false, enter: None, chords: [(0.0, Chord::new(0, 0)); CHART_CHORDS], n: 0 };

    /// The chord in effect at `pos` (a fraction of the bar).
    pub fn chord_at(&self, pos: f32) -> Option<Chord> {
        self.chords[..self.n as usize].iter().rev().find(|(p, _)| *p <= pos + POS_EPS).map(|&(_, c)| c).or(self.enter)
    }

    /// The place of the first chord after `pos` in this bar.
    fn next_after(&self, pos: f32) -> Option<f32> {
        self.chords[..self.n as usize].iter().map(|&(p, _)| p).find(|&p| p > pos + POS_EPS)
    }
}

/// Two chord places closer than this are the same place.
const POS_EPS: f32 = 1e-4;

/// Where a chord on 0-based `beat` of a bar of `beats` beats sits: the fraction of the bar
/// before it (in [0, 1)).
pub fn bar_pos(beat: u8, beats: u8) -> f32 {
    (beat as f32 / beats.max(1) as f32).clamp(0.0, 1.0 - 1e-3)
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
                    p.chords[p.n as usize] = (bar_pos(c.beat, b.time.0), c.chord);
                    p.n += 1;
                    held = Some(c.chord);
                }
                if let Some(&(_, c)) = p.chords[..p.n as usize].first().filter(|(x, _)| *x <= POS_EPS) {
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
    /// The bar line (tick) the player's override carries over: their chord came just
    /// before it (`ANTICIPATE_BEATS`).
    carry: Option<f64>,
    /// The chord the chart last gave the band (as written, before Keyboard transpose): a
    /// chord change to anything else is the player's.
    applied: Option<Chord>,
    /// The time of the last start: a Sync Start chord only starts the band.
    start_ns: Option<u64>,
    /// The section change the chart queued for the next bar line (a Main, a fill, the
    /// Ending or the stop), and the Main selected before it: a new plan or new settings
    /// take it back.
    owned: Option<(Queued, u8)>,
    /// How far into the bar playing (a fraction) the chart's chords have gone in.
    done: f32,
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
        let due = self.chart_unqueue(now);
        let len = plan.bars.len() as u32;
        let fresh = plan.fresh;
        let c = &mut self.features.chart;
        if fresh {
            // A new song starts from its first bar.
            c.bar = None;
            c.overridden = false;
            c.carry = None;
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
        self.chart_requeue(now, due);
        old
    }

    /// New chart settings. Turning chart mode off hands the chords back to the player (and
    /// takes back what the chart queued); new settings (a loop, an Ending) queue the next
    /// bar line again.
    pub fn set_chart_settings(&mut self, s: ChartSettings, now: u64) {
        let due = self.chart_unqueue(now);
        let c = &mut self.features.chart;
        c.settings = s;
        if !s.on {
            c.bar = None;
            c.overridden = false;
            c.carry = None;
        }
        self.chart_requeue(now, due);
    }

    /// Take back the change the chart queued, if it is still the one queued (the player may
    /// have queued another since), and the Main it selected. Returns its bar line if that
    /// line has come already (`now` is a hair past it, and `process` hasn't got to it): the
    /// change for that line is planned again, not moved to the line after.
    fn chart_unqueue(&mut self, now: u64) -> Option<f64> {
        let (q, main) = self.features.chart.owned.take()?;
        if self.queued.is_some_and(|cur| id_of(cur.slot) == id_of(q.slot) || cur.slot == usize::MAX && q.slot == usize::MAX) {
            self.queued = None;
            self.main = main;
            return (self.running && q.at <= self.tick_at(now) + 1e-6).then_some(q.at);
        }
        None
    }

    /// Queue the change again (a new plan or new settings), while the chart plays a Main,
    /// fill or break: for the bar line `due` when it has come and `process` hasn't got to it
    /// yet, else for the next bar line.
    fn chart_requeue(&mut self, now: u64, due: Option<f64>) {
        if !self.running || !self.chart_active() || self.queued.is_some() {
            return;
        }
        if matches!(id_of(self.cur), SectionId::Intro(_) | SectionId::Ending(_)) {
            return;
        }
        let end = due.unwrap_or_else(|| self.next_bar(now));
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
        c.carry = None;
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
        c.carry = None;
        c.owned = None;
    }

    /// `on_bar`: the chart moves on a bar (outside an Intro or Ending), the player's
    /// override ends, and the next section change is queued for its bar line.
    pub(super) fn chart_bar(&mut self, bar: u32, now: u64, sink: &mut impl Sink) {
        if !self.chart_active() {
            return;
        }
        let tpb = self.style.tpb.max(1) as f64;
        let line = self.sec_start + bar as f64 * tpb;
        // An override ends here, unless the player's chord came just before this line (an
        // anticipation of this bar): then it holds through this bar too.
        let c = &mut self.features.chart;
        let carried = c.overridden && c.carry.take().is_some_and(|t| (t - line).abs() < 1.0);
        let was_overridden = std::mem::replace(&mut c.overridden, carried) && !carried;
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
        // A change the chart queued for a later line (settings changed just before this
        // one) is replaced by this bar's; one for this line has happened.
        match self.features.chart.owned {
            Some((q, _)) if q.at > line + 1e-6 => {
                let _ = self.chart_unqueue(now);
            }
            _ => self.features.chart.owned = None,
        }
        // A change the player queued for a later line (not the chart's) stays.
        if self.queued.is_none() {
            self.chart_queue(line + tpb);
        }
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

    /// The plan bar whose chords go in now: the chart plays a Main, fill or break, and the
    /// player hasn't taken over.
    fn chart_giving(&self) -> Option<&PlanBar> {
        let c = &self.features.chart;
        if !self.running || !self.chart_active() || c.overridden || matches!(id_of(self.cur), SectionId::Intro(_) | SectionId::Ending(_)) {
            return None;
        }
        self.plan_bar(c.bar?)
    }

    /// `on_beat`: a bar's first beat gives the chord it begins with (its own, or the one
    /// held); the rest go in at their places (`chart_due`).
    pub(super) fn chart_beat(&mut self, beat: u32, now: u64, sink: &mut impl Sink) {
        if beat != 0 {
            return;
        }
        self.features.chart.done = 0.0;
        let c = self.chart_giving().and_then(|b| b.chord_at(0.0));
        self.chart_chord(c, now, sink);
    }

    /// The tick of the bar line the bar and beat lines are in (`self.lines` aims at the
    /// next line).
    fn line_bar_start(&self) -> f64 {
        let (tpb, ppq) = (self.style.tpb.max(1) as f64, self.style.ppq.max(1) as f64);
        let l = self.lines;
        if l.beat == 0 {
            l.next - tpb
        } else {
            l.next - l.beat as f64 * ppq
        }
    }

    /// `hook_due`: the tick of the chart's next chord in the bar playing, and its place.
    #[inline]
    pub(super) fn chart_due(&self) -> Option<(f64, f32)> {
        let pos = self.chart_giving()?.next_after(self.features.chart.done)?;
        Some((self.line_bar_start() + pos as f64 * self.style.tpb.max(1) as f64, pos))
    }

    /// `on_due`: the chord at `pos` in the bar playing goes in.
    pub(super) fn chart_at(&mut self, pos: f32, now: u64, sink: &mut impl Sink) {
        self.features.chart.done = pos;
        let c = self.chart_giving().and_then(|b| b.chord_at(pos));
        self.chart_chord(c, now, sink);
    }

    /// `on_chord`: a chord that isn't the chart's is the player's; it holds until the next
    /// bar line, or, played within `ANTICIPATE_BEATS` of that line, through the bar after
    /// it. A Sync Start chord only starts the band.
    pub(super) fn chart_chord_changed(&mut self, now: u64) {
        let c = &self.features.chart;
        if !self.running || !c.settings.on || self.played == c.applied || c.start_ns == Some(now) {
            return;
        }
        let line = self.next_bar(now);
        let early = line - self.tick_at(now) <= ANTICIPATE_BEATS * self.style.ppq.max(1) as f64 + 1e-6;
        let c = &mut self.features.chart;
        c.overridden = true;
        c.carry = early.then_some(line);
    }

    /// `hook_deadline`: every beat line while the chart plays (its bar lines queue the
    /// sections), and its next chord.
    #[inline]
    pub(super) fn chart_deadline(&self) -> Option<f64> {
        let line = (self.running && self.chart_active()).then_some(self.lines.next)?;
        Some(self.chart_due().map_or(line, |(t, _)| t.min(line)))
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

    /// A chord played just before a bar line (an anticipation) holds through the next bar;
    /// the chart takes over at the line after it. One played earlier than half a beat
    /// before the line ends at it, as before.
    #[test]
    fn an_anticipated_chord_holds_through_the_next_bar() {
        for (early_beats, carries) in [(0.02, true), (0.45, true), (0.6, false), (2.0, false)] {
            let Some(mut e) = engine() else { return };
            e.set_chart(plan("*A[C |C |F |G |A- Z", 1), 0);
            e.set_chart_settings(settings(None, None), 0);
            start(&mut e);
            let bar = e.ns_at_bar(1) - e.ns_at_bar(0);
            let beat = bar / (e.style.tpb / e.style.ppq) as u64;
            // Just before bar 3 (the chart's F) the player plays Ab.
            let t = e.ns_at_bar(2) - (early_beats * beat as f64) as u64;
            let _ = play(&mut e, 0, 1);
            e.process(t, &mut Nop);
            e.set_chord(crate::parse_chord("Ab").unwrap(), t, &mut Nop);
            assert_eq!(e.snapshot(t).chart_bar, Some(1), "{early_beats}");
            let (lines, _) = play(&mut e, t, 3);
            let b2 = at(&lines, 2);
            if carries {
                assert!(b2.iter().all(|l| l.chord == "Ab"), "{early_beats}: {b2:?}");
            } else {
                assert_eq!(b2[0].chord, "F", "{early_beats}: {b2:?}");
            }
            assert_eq!(at(&lines, 3)[0].chord, "G", "{early_beats}: {lines:?}");
        }
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

    /// The chord playing just before and just after `frac` of the style's bar `bar`.
    fn around(e: &mut Engine, bar: u32, frac: f64) -> (String, String) {
        let len = (e.ns_at_bar(1) - e.ns_at_bar(0)) as f64;
        let at = e.ns_at_bar(bar) as f64 + frac * len;
        let mut name = |t: f64| {
            e.process(t as u64, &mut Nop);
            e.snapshot(t as u64).played.map_or("-".into(), |c| c.name())
        };
        (name(at - len / 100.0), name(at + len / 200.0))
    }

    /// A chord goes in at its place in the bar, whatever the chart's beat unit: a 2/2 or
    /// 12/8 chart's half-bar chord at half the style's bar (quarter 3 of 4/4), a 6/8 chart's
    /// second eighth at a sixth of it (between the style's quarter lines).
    #[test]
    fn chords_land_at_their_place_in_the_bar() {
        for (chart, frac, name) in [
            ("T22*A[C D |C Z", 0.5, "D"),
            ("T12*A[C D |C Z", 0.5, "D"),
            ("T68*A[C  D  |C Z", 0.5, "D"),
            ("T68*A[C,D,E,F,G,A|C Z", 1.0 / 6.0, "D"),
            ("T68*A[C,D,E,F,G,A|C Z", 5.0 / 6.0, "A"),
            ("T44*A[C D |C Z", 0.5, "D"),
        ] {
            let Some(mut e) = engine() else { return };
            e.set_chart(plan(chart, 1), 0);
            e.set_chart_settings(settings(None, None), 0);
            start(&mut e);
            let (before, after) = around(&mut e, 0, frac);
            assert_ne!(before, name, "{chart}: {name} early");
            assert_eq!(after, name, "{chart}: {name} not in at {frac} of the bar");
            // The next bar's chord still comes on its bar line.
            assert_eq!(around(&mut e, 1, 0.0).1, "C", "{chart}");
        }
    }

    /// New settings or a new plan a hair after a bar line, before `process` has got to it:
    /// the change the chart queued for that line still happens there.
    #[test]
    fn a_command_just_after_the_line_keeps_its_section_change() {
        for new_plan in [false, true] {
            let Some(mut e) = engine() else { return };
            e.button(Button::AutoFill, 0, &mut Nop);
            e.set_chart(plan("*A[C |C ]*B[F |F Z", 1), 0);
            e.set_chart_settings(settings(None, None), 0);
            start(&mut e);
            let line = e.ns_at_bar(2);
            let (_, _) = play(&mut e, 0, 1);
            e.process(line - 1_000_000, &mut Nop);
            assert_eq!(e.snapshot(line - 1_000_000).queued, Some(SectionId::Main(1)));
            // The line has come; the engine hasn't woken for it yet.
            let t = line + 1_000;
            if new_plan {
                let _ = e.set_chart(plan("*A[C |C ]*B[F |F Z", 2), t);
            } else {
                e.set_chart_settings(ChartSettings { loop_range: Some((0, 4)), ..settings(None, None) }, t);
            }
            let (lines, _) = play(&mut e, t, 1);
            assert!(at(&lines, 2).iter().all(|l| l.section == "Main B" && l.chord == "F"), "new plan {new_plan}: {lines:?}");
            assert_eq!(at(&lines, 2)[0].sbar, 0, "{lines:?}");
        }
    }

    /// An Ending the player presses a hair after a bar line, before `process` has got to
    /// it, is not replaced by the chart's next change.
    #[test]
    fn an_ending_pressed_just_after_the_line_plays() {
        let Some(mut e) = engine() else { return };
        e.button(Button::AutoFill, 0, &mut Nop);
        e.set_chart(plan("*A[C |C ]*B[F |F |F |F Z", 1), 0);
        e.set_chart_settings(settings(None, None), 0);
        start(&mut e);
        let line = e.ns_at_bar(2);
        let _ = play(&mut e, 0, 1);
        e.process(line - 1_000_000, &mut Nop);
        let t = line + 1_000;
        e.button(Button::Ending(0), t, &mut Nop);
        // And new settings in the same window.
        e.set_chart_settings(ChartSettings { loop_range: Some((0, 6)), ..settings(None, None) }, t + 1_000);
        let (lines, _) = play(&mut e, t, 3);
        assert!(lines.iter().any(|l| l.section == "Ending A"), "{lines:?}");
    }

    #[test]
    fn plan_bars_carry_sections_and_held_chords() {
        let p = plan("*A[C |x |D-7 G7 ]*B[G7 |*C C Z", 1);
        let mains: Vec<_> = p.bars.iter().map(|b| (b.main, b.section_start)).collect();
        assert_eq!(mains, [(0, false), (0, false), (0, false), (1, true), (2, true)]);
        assert_eq!(p.bars[1].enter.unwrap().name(), "C");
        assert_eq!(p.bars[2].chord_at(0.25).unwrap().name(), "Dm7");
        assert_eq!(p.bars[2].chord_at(0.5).unwrap().name(), "G7");
        assert_eq!(p.bars[1].chord_at(0.9).unwrap().name(), "C");
        assert_eq!(section_main(Some('V'), 2), 2);
    }
}

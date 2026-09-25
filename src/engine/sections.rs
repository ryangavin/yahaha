//! Section changes: when a queued change happens, and what happens at the boundary.

use super::*;

/// What kind of change is being queued (`Engine::change_point`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    /// Into a Main (A-D): Section Change Timing "To Main".
    Main,
    /// Into Intro or Ending slot `.0`: Section Change Timing "Inside Intro/Ending" while an
    /// Intro or Ending plays, else the next bar line.
    IntroEnding(usize),
    /// A Fill In or Break.
    Fill,
    /// A fill under Half Bar Fill In (asked for on the first beat of a bar): from the
    /// middle of that bar.
    HalfBar,
    /// The band stops (an Ending the style doesn't have).
    Stop,
    /// Another style takes over while the band plays (follows "To Main").
    Style,
}

impl Engine {
    /// The section's next boundary before its end (`section_end`): a queued section, or a
    /// style change (`swap` true), else the section's end; and whether events at it still
    /// belong to this section (at its real end they do; at a Retrigger loop's end they
    /// are the rest of the section, which does not play).
    pub(super) fn boundary(&self) -> (f64, bool, bool) {
        let (sec_end, real_end) = self.section_end();
        let (b, inclusive) = match self.queued {
            Some(q) if q.at < sec_end => (q.at, false),
            _ => (sec_end, real_end),
        };
        match &self.pending {
            Some(p) if p.at <= b + 1e-6 => (p.at, false, true),
            _ => (b, inclusive, false),
        }
    }

    /// When a change asked for at `now` takes effect: the tick it starts at, and the tick
    /// the section it brings counts its bars from (both on the section's timeline). The
    /// one policy for section-change timing: every queued change asks here (Intro, Main
    /// and Ending buttons, fills and breaks, the stop an Ending the style lacks makes, and
    /// a style change while playing).
    ///
    /// Fills and breaks start at the next beat and play the rest of that bar, aligned so
    /// the fill's beat matches the bar position. Mains and style changes follow Section
    /// Change Timing "To Main" (`MainTiming`; Auto Fill In on makes a Main change Next
    /// Bar). Changing from an Intro or Ending to another follows "Inside Intro/Ending"
    /// (`IntroEndingTiming`), except Intro to Intro (always Next Bar) and into Ending I;
    /// those, a change from a Main or a Fill into an Intro or Ending, and the stop wait for
    /// the next bar line, as they always have (the manual's "conventional rules").
    ///
    /// A Half Bar Fill (Half Bar Fill In, asked for on the first beat of a bar) starts at
    /// the middle of that bar (half its notated beats, rounded down: beat 3 of 4/4, beat 2
    /// of 3/4, the 4th eighth of 6/8; `Prepared::half_bar`), aligned as a fill; asked for
    /// after the middle, it is a fill from the next beat.
    pub(super) fn change_point(&self, change: Change, now: u64) -> (f64, f64) {
        let timing = self.features.settings;
        match change {
            Change::Fill => self.next_beat(now),
            Change::Main => match timing.main_timing {
                MainTiming::Immediate if !self.auto_fill => self.next_beat(now),
                _ => self.bar_or_now(now),
            },
            // Decision (owner-confirmed, as the Genos does): a style change while an Ending
            // plays waits for the Ending to end, even in its first beat; the band stops
            // there with the new style loaded. Section Change Timing applies only while a
            // Main plays; from an Intro, a Fill or the Break the change waits for the
            // next bar line. An Ending pressed but still waiting for its change point
            // counts as playing (#111): the style waits for that Ending's end too, and the
            // Ending plays in the old style.
            Change::Style if self.queued_ending_end().is_some() => {
                let end = self.queued_ending_end().unwrap_or_default();
                (end, end)
            }
            Change::Style => match id_of(self.cur) {
                SectionId::Ending(_) => {
                    let at = self.sec_start + self.style.sections[self.cur].as_ref().map_or(0, |s| s.len) as f64;
                    (at, at)
                }
                SectionId::Main(_) => match timing.main_timing {
                    MainTiming::Immediate => self.next_beat(now),
                    MainTiming::NextBar => self.bar_or_now(now),
                },
                _ => {
                    let at = self.next_bar(now);
                    (at, at)
                }
            },
            Change::IntroEnding(to) => {
                let from = id_of(self.cur);
                let inside = matches!(from, SectionId::Intro(_) | SectionId::Ending(_));
                if !inside || to == slot_of(SectionId::Ending(0)) {
                    let at = self.next_bar(now);
                    return (at, at);
                }
                let intro_to_intro = matches!((from, id_of(to)), (SectionId::Intro(_), SectionId::Intro(_)));
                match timing.intro_ending_timing {
                    IntroEndingTiming::EndOfSection if !intro_to_intro => {
                        let at = self.sec_start + self.style.sections[self.cur].as_ref().map_or(0, |s| s.len) as f64;
                        (at, at)
                    }
                    _ => self.bar_or_now(now),
                }
            }
            Change::Stop => {
                let at = self.next_bar(now);
                (at, at)
            }
            Change::HalfBar => {
                let t = self.tick_at(now);
                let tpb = self.style.tpb.max(1) as f64;
                let bar_start = self.sec_start + ((t - self.sec_start) / tpb).floor() * tpb;
                let half = self.style.half_bar as f64;
                // Asked for after the middle (not on beat 1): the next beat, as a fill.
                if bar_start + half + 1e-6 >= t {
                    (bar_start + half, bar_start)
                } else {
                    self.next_beat(now)
                }
            }
        }
    }

    /// The tick a queued Ending (pressed, not playing yet) ends at, on the section
    /// timeline: its start plus its length in the style playing.
    pub(super) fn queued_ending_end(&self) -> Option<f64> {
        let q = self.queued?;
        if !(13..=15).contains(&q.slot) {
            return None;
        }
        let len = self.style.sections[q.slot].as_ref()?.len as f64;
        Some(q.sec_start + len)
    }

    /// What plays when the section playing ends with no change queued for its end: a Main
    /// repeats (or becomes the Main selected meanwhile), an Intro, Fill or Break goes on to
    /// the Main, an Ending stops the band (`usize::MAX`).
    pub(super) fn follow_on(&self) -> usize {
        let main_slot = self.style.resolve(4 + self.main as usize).unwrap_or(4);
        match id_of(self.cur) {
            SectionId::Main(_) => main_slot,
            SectionId::Ending(_) => usize::MAX,
            _ => main_slot,
        }
    }

    /// The next bar line after `now`.
    pub(super) fn next_bar(&self, now: u64) -> f64 {
        let t = self.tick_at(now);
        let tpb = self.style.tpb as f64;
        let pos = t - self.sec_start;
        self.sec_start + ((pos / tpb).floor() + 1.0) * tpb
    }

    /// Queue section `slot` (an Intro, a Main, an Ending) for its change point.
    pub(super) fn queue_at_bar(&mut self, slot: usize, now: u64) {
        let change = if matches!(id_of(slot), SectionId::Main(_)) { Change::Main } else { Change::IntroEnding(slot) };
        let (at, sec_start) = self.change_point(change, now);
        self.set_queued(Queued { slot, at, sec_start }, now);
    }

    /// Queue the band's stop (an Ending the style doesn't have).
    pub(super) fn queue_stop_at_bar(&mut self, now: u64) {
        let (at, sec_start) = self.change_point(Change::Stop, now);
        self.set_queued(Queued { slot: usize::MAX, at, sec_start }, now);
    }

    /// Queue a fill or break.
    pub(super) fn queue_fill(&mut self, slot: usize, now: u64) {
        self.queue_change(slot, Change::Fill, now);
    }

    /// Queue section `slot` for the change point of a `change`.
    pub(super) fn queue_change(&mut self, slot: usize, change: Change, now: u64) {
        let (at, sec_start) = self.change_point(change, now);
        self.set_queued(Queued { slot, at, sec_start }, now);
    }

    /// Queue `q` in place of whatever was queued. A style change waiting for a queued
    /// Ending that this replaces (#175) no longer waits for it: it comes when a style
    /// chosen now would (the new change, if it is an Ending, is waited for in turn).
    fn set_queued(&mut self, q: Queued, now: u64) {
        let was_ending = self.queued_ending_end().is_some();
        self.queued = Some(q);
        if was_ending && self.pending.is_some() {
            let (at, _) = self.change_point(Change::Style, now);
            if let Some(p) = self.pending.as_mut() {
                p.at = at;
            }
        }
    }

    pub(super) fn seek(&mut self, pos: f64) {
        let sec = self.style.sections[self.cur].as_ref().unwrap();
        self.entry = pos;
        self.ev_idx = sec.events.partition_point(|e| (e.tick as f64) < pos);
    }

    /// A section boundary at tick `at`: the section queued for it, else `follow_on`, takes
    /// over.
    pub(super) fn transition(&mut self, at: f64, now: u64, sink: &mut impl Sink) {
        if self.retrigger_wraps(at) {
            let at = self.retrigger_catch_up(at, now);
            self.restart_section(at, now, sink);
            return;
        }
        let sec_end = self.section_end().0;
        let (next, start) = match self.queued.take() {
            Some(q) if q.at <= sec_end + 1e-6 => (q.slot, q.sec_start),
            q => {
                self.queued = q;
                (self.follow_on(), at)
            }
        };
        let from = self.cur;
        self.before_section_change(next, at, now, sink);
        self.notes_off(true, sink);
        if next == usize::MAX {
            self.stop(sink);
            self.sync_armed = true;
            return;
        }
        let change = next != self.cur;
        self.cur = next;
        self.sec_start = start;
        self.seek(at - start);
        // A section repeating itself is not a change: its own pattern carries on.
        if change {
            let own_voice = self.own_voice(at - start);
            // An Ending's fade-out (CC11) ends the song, not the section that follows it:
            // a Main, Break or Intro pressed during the Ending starts at full expression,
            // as it would after a start (#122). The setup's own CC11 still has the last
            // word (reapply_init), and so does the new section's at its entry.
            if matches!(id_of(from), SectionId::Ending(_)) {
                let own = self.own_expression(at - start);
                self.reset_expression(own, sink);
            }
            self.reapply_init(own_voice, sink);
        }
        self.chase(sink);
        self.lines_from(at);
        self.after_section_change(from, now, sink);
    }

    /// Channels whose current section sends a program change up to section tick `entry`
    /// (inclusive): at the section change they take that voice, not the SInt's.
    pub(super) fn own_voice(&self, entry: f64) -> u16 {
        let sec = self.style.sections[self.cur].as_ref().unwrap();
        let mut m = 0u16;
        for e in sec.events.iter().take_while(|e| e.tick as f64 <= entry + 1e-6) {
            if let (PKind::Pc { .. }, Some(r)) = (e.kind, sec.rules[e.src as usize & 15].as_ref()) {
                m |= 1 << (r.dest_ch & 15);
            }
        }
        m
    }

    /// Channels whose current section sets expression (CC11) up to section tick `entry`
    /// (inclusive): they take the section's own value at the section change.
    pub(super) fn own_expression(&self, entry: f64) -> u16 {
        let sec = self.style.sections[self.cur].as_ref().unwrap();
        let mut m = 0u16;
        for e in sec.events.iter().take_while(|e| e.tick as f64 <= entry + 1e-6) {
            if let (PKind::Cc { cc: 11, .. }, Some(r)) = (e.kind, sec.rules[e.src as usize & 15].as_ref()) {
                m |= 1 << (r.dest_ch & 15);
            }
        }
        m
    }

    /// A section entered mid-bar (a Fill or Break at the next beat) skips its events before
    /// the entry point. Its notes stay skipped, but its voice and controllers are brought to
    /// where the pattern has them at the entry, so the part plays the section's voice from
    /// there on. Bank selects, program changes and (N)RPN messages go in order; for the
    /// other controllers and the pitch bend only the value in effect at the entry matters,
    /// so each goes once, and only if it differs from what the channel has: an expression
    /// swell or a bend before the entry is not replayed as a burst.
    pub(super) fn chase(&mut self, sink: &mut impl Sink) {
        if self.ev_idx == 0 {
            return;
        }
        let mut last_cc = [[UNSENT; 128]; 16];
        let mut last_bend: [Option<(u8, u8)>; 16] = [None; 16];
        for i in 0..self.ev_idx {
            let sec = self.style.sections[self.cur].as_ref().unwrap();
            let e = sec.events[i];
            let Some(dest) = sec.rules[e.src as usize].as_ref().map(|r| r.dest_ch) else { continue };
            let d = dest as usize & 15;
            match e.kind {
                PKind::Cc { cc: cc @ (0 | 32), val } => {
                    if self.mirror.cc[d][cc as usize] != val {
                        self.mirror.send(sink, &[0xB0 | dest, cc, val]);
                    }
                }
                PKind::Cc { cc: 6 | 38 | 96..=101, .. } => self.emit_control(dest, e.kind, sink),
                PKind::Cc { cc, val } => last_cc[d][cc as usize & 127] = val,
                PKind::Pc { prog } => {
                    let want = (self.mirror.cc[d][0], self.mirror.cc[d][32], prog);
                    if self.mirror.voice[d] != Some(want) {
                        self.mirror.send(sink, &[0xC0 | dest, prog]);
                        self.pattern_pc |= 1 << d;
                    }
                }
                PKind::Bend { lo, hi } => last_bend[d] = Some((lo, hi)),
                PKind::On { .. } | PKind::Off { .. } => {}
            }
        }
        for d in 0..16u8 {
            for cc in 0..128u8 {
                let val = last_cc[d as usize][cc as usize];
                if val == UNSENT {
                    continue;
                }
                if cc == 7 && (8..16).contains(&d) {
                    let p = d as usize - 8;
                    if self.user_set & (1 << p) == 0 && (self.mixer[p] != val || self.mirror.cc[d as usize][7] != self.faded(val)) {
                        self.pattern_volume(d, val, sink);
                    }
                } else if self.mirror.cc[d as usize][cc as usize] != val {
                    self.mirror.send(sink, &[0xB0 | d, cc, val]);
                }
            }
            if let Some((lo, hi)) = last_bend[d as usize] {
                let v = (hi as u16) << 7 | lo as u16;
                if follows_chords(d) {
                    if self.pat_bend[d as usize] != v {
                        self.pat_bend[d as usize] = v;
                        self.send_bend(d, sink);
                    }
                } else if self.mirror.bend[d as usize] != Some(v) {
                    self.mirror.send(sink, &[0xE0 | d, lo, hi]);
                }
            }
        }
    }
}

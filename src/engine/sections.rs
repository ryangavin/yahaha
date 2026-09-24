//! Section changes: when a queued change happens, and what happens at the boundary.

use super::*;

/// What kind of change is being queued (`Engine::change_point`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    /// An Intro, Main or Ending.
    Section,
    /// A Fill In or Break.
    Fill,
    /// A fill under Half Bar Fill In (asked for on the first beat of a bar): from the
    /// middle of that bar.
    HalfBar,
    /// The band stops (an Ending the style doesn't have).
    Stop,
    /// Another style takes over while the band plays.
    Style,
}

impl Engine {
    /// The section's next boundary before `sec_end`: a queued section, or a style change
    /// (`swap` true), else the section's end; and whether events at it still belong to
    /// this section.
    pub(super) fn boundary(&self, sec_end: f64) -> (f64, bool, bool) {
        let (b, inclusive) = match self.queued {
            Some(q) if q.at < sec_end => (q.at, false),
            _ => (sec_end, true),
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
    /// Sections, stops and style changes wait for the next bar line. Fills and breaks
    /// start at the next beat and play the rest of that bar, aligned so the fill's beat
    /// matches the bar position. A Half Bar Fill starts at the middle of the bar asked in
    /// (the beat at or before half its length: beat 3 of 4, beat 2 of 3), aligned the
    /// same way.
    pub(super) fn change_point(&self, change: Change, now: u64) -> (f64, f64) {
        match change {
            Change::Section | Change::Stop | Change::Style => {
                let at = self.next_bar(now);
                (at, at)
            }
            Change::Fill => {
                let t = self.tick_at(now);
                let ppq = self.style.ppq as f64;
                let tpb = self.style.tpb as f64;
                let pos = t - self.sec_start;
                let beat = (pos / ppq).ceil() * ppq;
                let at = self.sec_start + beat;
                let bar_start = self.sec_start + (beat / tpb).floor() * tpb;
                (at, bar_start)
            }
            Change::HalfBar => {
                let t = self.tick_at(now);
                let ppq = self.style.ppq.max(1) as f64;
                let tpb = self.style.tpb.max(1) as f64;
                let bar_start = self.sec_start + ((t - self.sec_start) / tpb).floor() * tpb;
                let half = ((tpb / ppq / 2.0).floor() * ppq).max(ppq);
                // Asked for after the middle (not on beat 1): the next beat, as a fill.
                let at = if bar_start + half + 1e-6 >= t { bar_start + half } else { return self.change_point(Change::Fill, now) };
                (at, bar_start)
            }
        }
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
        let (at, sec_start) = self.change_point(Change::Section, now);
        self.queued = Some(Queued { slot, at, sec_start });
    }

    /// Queue the band's stop (an Ending the style doesn't have).
    pub(super) fn queue_stop_at_bar(&mut self, now: u64) {
        let (at, sec_start) = self.change_point(Change::Stop, now);
        self.queued = Some(Queued { slot: usize::MAX, at, sec_start });
    }

    /// Queue a fill or break.
    pub(super) fn queue_fill(&mut self, slot: usize, now: u64) {
        self.queue_change(slot, Change::Fill, now);
    }

    /// Queue section `slot` for the change point of a `change`.
    pub(super) fn queue_change(&mut self, slot: usize, change: Change, now: u64) {
        let (at, sec_start) = self.change_point(change, now);
        self.queued = Some(Queued { slot, at, sec_start });
    }

    pub(super) fn seek(&mut self, pos: f64) {
        let sec = self.style.sections[self.cur].as_ref().unwrap();
        self.entry = pos;
        self.ev_idx = sec.events.partition_point(|e| (e.tick as f64) < pos);
    }

    /// A section boundary at tick `at`: the section queued for it, else `follow_on`, takes
    /// over.
    pub(super) fn transition(&mut self, at: f64, now: u64, sink: &mut impl Sink) {
        let sec_end = self.sec_start + self.style.sections[self.cur].as_ref().map_or(0, |s| s.len) as f64;
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
                    if self.user_set & (1 << p) == 0 && (self.mixer[p] != val || self.mirror.cc[d as usize][7] != val) {
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

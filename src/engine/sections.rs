//! Section changes: when a queued change happens, and what happens at the boundary.

use super::*;

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

    pub(super) fn next_bar(&self, now: u64) -> f64 {
        let t = self.tick_at(now);
        let tpb = self.style.tpb as f64;
        let pos = t - self.sec_start;
        self.sec_start + ((pos / tpb).floor() + 1.0) * tpb
    }

    pub(super) fn queue_at_bar(&mut self, slot: usize, now: u64) {
        let at = self.next_bar(now);
        self.queued = Some(Queued { slot, at, sec_start: at });
    }

    pub(super) fn queue_stop_at_bar(&mut self, now: u64) {
        let at = self.next_bar(now);
        self.queued = Some(Queued { slot: usize::MAX, at, sec_start: at });
    }

    /// Fills and breaks start at the next beat and play the rest of the bar, aligned so the
    /// fill's beat matches the bar position.
    pub(super) fn queue_fill(&mut self, slot: usize, now: u64) {
        let t = self.tick_at(now);
        let ppq = self.style.ppq as f64;
        let tpb = self.style.tpb as f64;
        let pos = t - self.sec_start;
        let beat = (pos / ppq).ceil() * ppq;
        let at = self.sec_start + beat;
        let bar_start = self.sec_start + (beat / tpb).floor() * tpb;
        self.queued = Some(Queued { slot, at, sec_start: bar_start });
    }

    pub(super) fn seek(&mut self, pos: f64) {
        let sec = self.style.sections[self.cur].as_ref().unwrap();
        self.entry = pos;
        self.ev_idx = sec.events.partition_point(|e| (e.tick as f64) < pos);
    }

    pub(super) fn transition(&mut self, at: f64, now: u64, sink: &mut impl Sink) {
        self.notes_off(true, sink);
        let sec_end = self.sec_start + self.style.sections[self.cur].as_ref().map_or(0, |s| s.len) as f64;
        let (next, start) = match self.queued.take() {
            Some(q) if q.at <= sec_end + 1e-6 => (q.slot, q.sec_start),
            q => {
                self.queued = q;
                let main_slot = self.style.resolve(4 + self.main as usize).unwrap_or(4);
                match id_of(self.cur) {
                    SectionId::Main(_) => (main_slot, at),
                    SectionId::Ending(_) => (usize::MAX, at),
                    _ => (main_slot, at),
                }
            }
        };
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
        let _ = now;
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

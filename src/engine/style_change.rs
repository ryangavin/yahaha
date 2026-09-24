//! Style changes: at once when stopped, at the next bar line when playing.

use super::*;

impl Engine {
    /// A style change from the player (the session's `LoadStyle`): stopped, the new style
    /// takes over at once; playing, at the next bar line, keeping the section and the bar
    /// position, as on a Genos (`swap_style`). A later change before that bar line replaces
    /// this one. Styles the engine no longer needs go to `take_retired`.
    pub fn change_style(&mut self, style: Box<Prepared>, now: u64, sink: &mut impl Sink) {
        if !self.running {
            if let Some(p) = self.pending.take() {
                self.retire(p.style);
            }
            let old = self.load(style, now, sink);
            self.retire(old);
            return;
        }
        let at = self.next_bar(now);
        if let Some(p) = self.pending.replace(PendingStyle { style, at }) {
            self.retire(p.style);
        }
    }

    /// Keep a style the engine is done with until the caller takes it (never freed here).
    pub(super) fn retire(&mut self, style: Box<Prepared>) {
        match self.retired.iter_mut().find(|r| r.is_none()) {
            Some(slot) => *slot = Some(style),
            // Cannot happen while the caller drains after every call; dropping here would
            // free on the real-time thread, so the oldest is handed back first.
            None => {
                self.retired.rotate_left(1);
                self.retired[3] = Some(style);
            }
        }
    }

    /// A style the engine no longer uses (replaced or never played), to free elsewhere.
    pub fn take_retired(&mut self) -> Option<Box<Prepared>> {
        self.retired.iter_mut().find_map(|r| r.take())
    }

    /// A style change is waiting for the next bar line.
    pub fn style_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// Bar line `at` (a tick of the style playing) has come: the pending style takes over.
    /// The section carries on in the new style: the same section if it has it, else the
    /// nearest of the same kind (Main D -> Main C), else the Main in use; a section queued
    /// for this bar line (a Main, an Ending) starts here instead, and one that ends here
    /// hands over as it would have (a Fill to its Main, an Ending stops the band). A
    /// section that carries on keeps its bar position (bar 3 of Main A goes on in bar 3 of
    /// the new Main A, wrapping at its length). The tempo stays, re-timed to the new
    /// style's resolution; the new style's setup and part levels go out, as on any load.
    pub(super) fn swap_style(&mut self, at: f64, now: u64, sink: &mut impl Sink) {
        let Some(p) = self.pending.take() else { return };
        let old_len = self.style.sections[self.cur].as_ref().map_or(0, |s| s.len) as f64;
        let sec_end = self.sec_start + old_len;
        let old_tpb = self.style.tpb.max(1) as f64;
        let old_ppq = self.style.ppq.max(1) as f64;
        // The section that plays from `at` on, in the old style's terms, and how far into it.
        let (slot, pos_old) = match self.queued {
            Some(q) if q.at <= at + 1e-6 => {
                self.queued = None;
                (q.slot, at - q.sec_start)
            }
            _ if at >= sec_end - 1e-6 => match id_of(self.cur) {
                SectionId::Ending(_) => (usize::MAX, 0.0),
                _ => (4 + self.main as usize, 0.0),
            },
            _ => (self.cur, at - self.sec_start),
        };
        // A later queued change (none is, in practice: sections queue for this bar line).
        let later = self.queued.take();
        let ns_b = self.ns_at(at);
        self.notes_off(false, sink);
        let old = std::mem::replace(&mut self.style, p.style);
        self.user_set = 0;
        for i in 0..8 {
            let v = self.style.mix[i];
            self.set_mixer(i, v);
        }
        self.retire(old);
        if slot == usize::MAX {
            // The Ending ended here: stopped, with the new style at its own tempo, as a
            // load while stopped leaves it.
            self.running = false;
            self.sync_armed = true;
            self.set_bpm_internal(self.style.bpm, ns_b);
            self.send_init(sink);
            return;
        }
        let new_slot = if self.style.has(slot) { Some(slot) } else { self.style.resolve(slot) };
        let Some(new_slot) = new_slot.or_else(|| self.style.resolve(4 + self.main as usize)) else {
            self.running = false;
            self.send_init(sink);
            return;
        };
        if let SectionId::Main(m) = id_of(new_slot) {
            self.main = m;
        }
        let (tpb, ppq) = (self.style.tpb.max(1) as f64, self.style.ppq.max(1) as f64);
        let len = self.style.sections[new_slot].as_ref().map_or(0, |s| s.len) as f64;
        // Whole bars, then the rest in beats.
        let bars = (pos_old / old_tpb).floor();
        let rest = (pos_old - bars * old_tpb) / old_ppq * ppq;
        let mut pos = bars * tpb + rest;
        if len > 0.0 {
            pos = pos.rem_euclid(len);
        }
        // Re-time: tick `pos` of the new section at the bar line's time, same tempo.
        self.cur = new_slot;
        self.anchor_ns = ns_b;
        self.anchor_tick = pos;
        self.sec_start = 0.0;
        self.ns_per_tick = 60e9 / (self.bpm * ppq);
        self.seek(pos);
        if let Some(q) = later {
            let t = |x: f64| pos + (x - at) / old_ppq * ppq;
            self.queued = Some(Queued { slot: if q.slot == usize::MAX { q.slot } else { self.style.resolve(q.slot).unwrap_or(new_slot) }, at: t(q.at), sec_start: t(q.sec_start) });
        }
        self.send_init(sink);
        self.chase(sink);
        let _ = now;
    }

    /// Swap in a new style at once; returns the old one so the caller can free it off the
    /// RT thread. Playing, the band restarts the new style's Main at once (tests use it;
    /// the session goes through `change_style`, which waits for the bar line).
    pub fn load(&mut self, style: Box<Prepared>, now: u64, sink: &mut impl Sink) -> Box<Prepared> {
        self.all_off(sink);
        let old = std::mem::replace(&mut self.style, style);
        self.queued = None;
        // A new style brings its own part levels (no parameter lock): the faders move to
        // them and the player's earlier moves are forgotten.
        self.user_set = 0;
        for p in 0..8 {
            let v = self.style.mix[p];
            self.set_mixer(p, v);
        }
        // Stopped, the new style's tempo; running, the same tempo re-timed to the new
        // style's resolution (ticks per quarter differ between styles: 480, 960, 1920).
        let bpm = if self.running { self.bpm } else { self.style.bpm };
        self.set_bpm_internal(bpm, now);
        self.send_init(sink);
        if self.running {
            // Continue from the next bar of the equivalent section.
            let slot = self.style.resolve(slot_of(SectionId::Main(self.main))).unwrap_or(4);
            let t = self.tick_at(now);
            self.cur = slot;
            self.sec_start = t;
            self.seek(0.0);
        }
        old
    }
}

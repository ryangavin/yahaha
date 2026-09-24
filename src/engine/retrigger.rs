//! Style Section Reset (TAP TEMPO while the style plays, OM p.46) and Style Retrigger
//! (RM p.147): both rewind the section playing to its top, at once.
//!
//! - **Section Reset**: the section starts again from its first beat at the tap; the bar
//!   grid moves with it: a change queued for a bar line waits for the new grid's next
//!   one, a fill for its next beat.
//! - **Retrigger**: while on, each chord played in a Main restarts the Main at the chord,
//!   and from then on only its head plays, `4 / rate` beats long (a whole note .. a 32nd),
//!   repeating, until a section change or Retrigger goes off (then the section plays on
//!   from where the head is). Only Mains retrigger.

use super::*;

/// Engine-side Retrigger state.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Retrigger {
    /// Style Retrigger on (RtgOnOff).
    pub(super) on: bool,
    /// A chord has started the head loop in the Main playing.
    pub(super) looping: bool,
}

impl Engine {
    /// Retrigger on/off (`Button::Retrigger`). Off, the Main plays on from where the head
    /// loop is.
    pub(super) fn toggle_retrigger(&mut self) {
        let r = &mut self.features.retrigger;
        r.on = !r.on;
        r.looping &= r.on;
    }

    /// Retrigger is on.
    pub fn retrigger_on(&self) -> bool {
        self.features.retrigger.on
    }

    /// The loop length in ticks while the head loops, if it does.
    fn retrigger_len(&self) -> Option<f64> {
        let r = self.features.retrigger;
        if !(r.on && r.looping && self.running && matches!(id_of(self.cur), SectionId::Main(_))) {
            return None;
        }
        let rate = self.features.settings.retrigger_rate.max(1) as f64;
        Some(4.0 * self.style.ppq as f64 / rate)
    }

    /// Where the section playing ends, and whether that is its real end (false: the
    /// Retrigger head loop's end, where it starts again).
    pub(super) fn section_end(&self) -> (f64, bool) {
        let len = self.style.sections[self.cur].as_ref().map_or(0, |s| s.len) as f64;
        match self.retrigger_len() {
            Some(l) if l < len - 1e-6 => (self.sec_start + l, false),
            _ => (self.sec_start + len, true),
        }
    }

    /// A boundary at `at` is the Retrigger loop coming round (no change is due there):
    /// the head starts again.
    pub(super) fn retrigger_wraps(&self, at: f64) -> bool {
        self.retrigger_len().is_some() && !self.queued.is_some_and(|q| q.at <= at + 1e-6)
    }

    /// A chord was played (`set_chord`; `was_running`: the band played before it). With
    /// Retrigger on, a Main starts again at the chord and its head loops.
    pub(super) fn retrigger_chord(&mut self, was_running: bool, now: u64, sink: &mut impl Sink) {
        if !(self.features.retrigger.on && was_running && self.running && matches!(id_of(self.cur), SectionId::Main(_))) {
            return;
        }
        self.features.retrigger.looping = true;
        self.reset_section(now, sink);
    }

    /// Style Section Reset: the section playing starts again from its top at `now`; a
    /// change queued for the old bar grid moves to the new one.
    pub(super) fn reset_section(&mut self, now: u64, sink: &mut impl Sink) {
        if !self.running {
            return;
        }
        let at = self.tick_at(now);
        self.restart_section(at, now, sink);
        let (tpb, ppq) = (self.style.tpb.max(1) as f64, self.style.ppq.max(1) as f64);
        if let Some(q) = self.queued.as_mut() {
            let fill = q.slot != usize::MAX && matches!(id_of(q.slot), SectionId::Fill(_) | SectionId::Break);
            *q = if fill {
                // A fill at the next beat.
                Queued { at: at + ppq.min(tpb), sec_start: at, ..*q }
            } else {
                // A section (or the stop): the new grid's next bar line.
                Queued { at: at + tpb, sec_start: at + tpb, ..*q }
            };
        }
        if let Some(p) = self.pending.as_mut() {
            p.at = at + tpb;
        }
    }

    /// The section playing starts again from its top at tick `at` (Section Reset, a
    /// Retrigger chord or its loop coming round): its notes end, its events play again.
    /// A section repeating itself: the section hooks run, its setup is not sent again.
    pub(super) fn restart_section(&mut self, at: f64, now: u64, sink: &mut impl Sink) {
        let slot = self.cur;
        self.before_section_change(slot, at, now, sink);
        self.notes_off(true, sink);
        self.sec_start = at;
        self.seek(0.0);
        self.lines_from(at);
        self.after_section_change(slot, now, sink);
    }

    /// After a section change: a new section ends the head loop until the next chord.
    #[inline]
    pub(super) fn retrigger_after_section(&mut self, from: usize) {
        if from != self.cur {
            self.features.retrigger.looping = false;
        }
    }

    /// The band stopped: no head loop.
    #[inline]
    pub(super) fn retrigger_on_stop(&mut self) {
        self.features.retrigger.looping = false;
    }
}

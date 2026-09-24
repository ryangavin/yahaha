//! ENDING/rit. (OM p.66): pressing the Ending that is playing again slows the tempo down
//! gradually to the end of the ending.
//!
//! The manuals don't say how much or how. yahaha slows it linearly, from the press to the
//! ending's last tick, to `RIT_END` of the tempo, in sixteenth-note steps. The tempo comes
//! back when the band stops (or the ending gives way to a Main).

use super::*;

/// The tempo at the end of a ritardando, as a fraction of the tempo it started from.
pub const RIT_END: f64 = 0.65;

/// Engine-side ritardando state.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Rit {
    pub(super) active: bool,
    /// The ending slot it slows.
    slot: usize,
    /// The tempo it started from (and goes back to).
    base: f64,
    /// Ticks (section timeline) it runs between.
    from: f64,
    to: f64,
    /// The next step's tick.
    next: f64,
}

impl Engine {
    /// The Ending playing was pressed again: slow down to its end.
    pub(super) fn start_rit(&mut self, now: u64) {
        if self.features.rit.active || !self.running {
            return;
        }
        let from = self.tick_at(now);
        let to = self.section_end().0;
        if to <= from + 1e-6 {
            return;
        }
        let step = self.style.ppq.max(4) as f64 / 4.0;
        self.features.rit = Rit { active: true, slot: self.cur, base: self.bpm, from, to, next: from + step };
    }

    /// A ritardando is slowing the band.
    pub fn ritardando(&self) -> bool {
        self.features.rit.active
    }

    /// The tempo buttons move the tempo a ritardando comes back to.
    pub(super) fn rit_tempo(&mut self, delta: f64) {
        if self.features.rit.active {
            self.features.rit.base += delta;
        }
    }

    /// The tempo where the ritardando has got to at `now`.
    pub(super) fn rit_wake(&mut self, now: u64) {
        let r = self.features.rit;
        if !r.active || !self.running {
            return;
        }
        let t = self.tick_at(now);
        let p = ((t - r.from) / (r.to - r.from)).clamp(0.0, 1.0);
        let bpm = r.base * (1.0 - (1.0 - RIT_END) * p);
        if (bpm - self.bpm).abs() > 1e-9 {
            self.set_bpm_internal(bpm, now);
        }
        let step = self.style.ppq.max(4) as f64 / 4.0;
        let t = self.tick_at(now);
        self.features.rit.next = r.from + ((t - r.from) / step + 1e-9).floor() * step + step;
    }

    /// The tick of the ritardando's next step, while it runs.
    pub(super) fn rit_deadline(&self) -> Option<f64> {
        let r = &self.features.rit;
        (r.active && self.running && r.next < r.to).then_some(r.next)
    }

    /// The ritardando ends and the tempo comes back (at `now`; stopped, any time).
    pub(super) fn end_rit(&mut self, now: u64) {
        let r = self.features.rit;
        if r.active {
            self.features.rit.active = false;
            self.set_bpm_internal(r.base, now);
        }
    }

    /// After a section change: leaving the ending ends its ritardando.
    #[inline]
    pub(super) fn rit_after_section(&mut self, now: u64) {
        if self.features.rit.active && self.cur != self.features.rit.slot {
            self.end_rit(now);
        }
    }
}

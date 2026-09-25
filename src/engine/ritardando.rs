//! ENDING/rit. (OM p.66): pressing the Ending that is playing again slows the tempo down
//! gradually to the end of the ending.
//!
//! The manuals don't say how much or how. yahaha slows it linearly, from the press to the
//! ending's last tick, to `RIT_END` of the tempo: set at every engine wake, and at least every
//! sixteenth note (`rit_deadline`). The tempo comes
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
    pub(super) base: f64,
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

    /// A tempo set outright during a ritardando (TAP TEMPO, with Style Section Reset off):
    /// it becomes the tempo the ritardando slows from and comes back to, and the band
    /// slows on from it at once.
    pub(super) fn rit_retempo(&mut self, now: u64) {
        if self.features.rit.active {
            self.features.rit.base = self.bpm;
            self.rit_wake(now);
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

    /// The ritardando ends without its tempo coming back (a new style's tempo takes over).
    pub(super) fn rit_drop(&mut self) {
        self.features.rit.active = false;
    }

    /// A style swapped in while the band plays (`swap_style`, which has re-anchored the
    /// timeline on the new style's ticks): an Ending still playing slows on from the tempo
    /// reached to the same end tempo at its new end; anything else ends the ritardando (the
    /// tempo comes back).
    pub(super) fn rit_rebase(&mut self) {
        let r = self.features.rit;
        if !r.active {
            return;
        }
        let now = self.anchor_ns;
        if !matches!(id_of(self.cur), SectionId::Ending(_)) {
            self.end_rit(now);
            return;
        }
        let from = self.tick_at(now);
        let to = self.section_end().0;
        if to <= from + 1e-6 {
            self.end_rit(now);
            return;
        }
        // How far down it has slowed (0 = base, 1 = the end tempo), and the tick where a
        // ritardando reaching that at `from` and the end tempo at `to` would have started.
        let done = ((1.0 - self.bpm / r.base) / (1.0 - RIT_END)).clamp(0.0, 1.0 - 1e-6);
        let start = (from - done * to) / (1.0 - done);
        let step = self.style.ppq.max(4) as f64 / 4.0;
        let next = start + ((from - start) / step + 1e-9).floor() * step + step;
        self.features.rit = Rit { active: true, slot: self.cur, base: r.base, from: start, to, next };
    }

    /// After a section change: leaving the ending ends its ritardando.
    #[inline]
    pub(super) fn rit_after_section(&mut self, now: u64) {
        if self.features.rit.active && self.cur != self.features.rit.slot {
            self.end_rit(now);
        }
    }
}

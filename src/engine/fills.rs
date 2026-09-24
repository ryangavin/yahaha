//! Main presses and the assignable fill functions: Fill Up / Down / Self (RM p.142) and
//! Half Bar Fill In.

use super::*;

/// Half Bar Fill In state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Fills {
    /// Half Bar Fill In: a Main change (or a fill) asked for on the first beat of a bar
    /// plays a fill from the middle of that bar, then the Main at the next bar line.
    pub(super) half_bar: bool,
}

impl Engine {
    /// Main A-D (`i`) pressed, or a fill function aimed at it (`force_fill`). Stopped, it
    /// only selects the Main the band starts on. On a Main: the Main playing plays its own
    /// fill; another Main is entered through its fill when Auto Fill is on (or a fill is
    /// forced), else at the next bar. On an Ending the Main comes at the next bar; during an
    /// Intro, fill or break the selected Main follows when it ends.
    pub(super) fn press_main(&mut self, i: u8, force_fill: bool, now: u64) {
        self.main = i;
        if !self.running {
            return;
        }
        match id_of(self.cur) {
            SectionId::Main(m) => {
                let half = self.half_bar_due(now);
                let fill = if i == m || self.auto_fill || force_fill || half { self.style.resolve(8 + i as usize) } else { None };
                match fill {
                    Some(f) if half => self.queue_change(f, Change::HalfBar, now),
                    Some(f) => self.queue_fill(f, now),
                    None if i != m => {
                        if let Some(slot) = self.style.resolve(4 + i as usize) {
                            self.queue_at_bar(slot, now)
                        }
                    }
                    None => {}
                }
            }
            SectionId::Ending(_) => {
                if let Some(slot) = self.style.resolve(4 + i as usize) {
                    self.queue_at_bar(slot, now)
                }
            }
            // Intro / fill / break: they flow into self.main when done.
            _ => {}
        }
    }

    /// Fill Up / Down / Self: a fill, then Main `target` (as a Main press with the fill
    /// forced, Auto Fill or not).
    pub(super) fn fill_to(&mut self, target: u8, now: u64) {
        self.press_main(target, true, now);
    }

    /// The Main next to the selected one, to the right (`up`) or left, skipping Mains the
    /// style lacks; the selected Main itself at the end of the row (no wrap).
    pub(super) fn neighbour_main(&self, up: bool) -> u8 {
        let mut j = self.main as i32;
        loop {
            j += if up { 1 } else { -1 };
            if !(0..4).contains(&j) {
                return self.main;
            }
            if self.style.has(4 + j as usize) {
                return j as u8;
            }
        }
    }

    /// Half Bar Fill In applies to a change asked for now: it is on, a Main plays, and
    /// `now` is within the first beat of a bar.
    pub(super) fn half_bar_due(&self, now: u64) -> bool {
        if !self.features.fills.half_bar || !self.running || !matches!(id_of(self.cur), SectionId::Main(_)) {
            return false;
        }
        let pos = (self.tick_at(now) - self.sec_start).max(0.0);
        pos.rem_euclid(self.style.tpb.max(1) as f64) < self.style.ppq.max(1) as f64
    }

    /// Half Bar Fill In on or off.
    pub fn half_bar_fill(&self) -> bool {
        self.features.fills.half_bar
    }
}

//! Style Setting rules the engine keeps: what a style change does to the tempo, the part
//! on/off states and the Main (Change Behavior, RM p.12-13), and how Stop Accompaniment
//! sounds (RM p.11).

use super::*;

/// Change Behavior for the tempo and the Style part on/off states (RM p.12-13).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChangeRule {
    /// Always keep the old style's value.
    Lock,
    /// Keep it while the band plays; take the new style's when stopped.
    #[default]
    Hold,
    /// Always take the new style's (its tempo; every part on).
    Reset,
}

impl ChangeRule {
    /// The new style's value applies to a change made now.
    pub fn resets(self, running: bool) -> bool {
        match self {
            ChangeRule::Lock => false,
            ChangeRule::Hold => !running,
            ChangeRule::Reset => true,
        }
    }
}

/// Style Setting > Change Behavior.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChangeRules {
    pub tempo: ChangeRule,
    pub parts: ChangeRule,
    /// Section Set: the Main (0-3) a style loaded while stopped selects, or None to keep
    /// the one selected. A Main the style lacks becomes the nearest it has.
    pub section: Option<u8>,
}

/// Stop Accompaniment: what a chord played with the band stopped (Sync Start off) sounds
/// on (RM p.11). The chord is recognised in every mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StopAcmp {
    /// Silent.
    #[default]
    Off,
    /// The style's own Bass and Pad voices.
    Style,
    /// Fixed Bass and Pad voices, whatever the style.
    Fixed,
}

/// The Fixed mode's voices (GM programs, bank 0): Fingered Bass and Warm Pad.
pub const FIXED_BASS_PROGRAM: u8 = 33;
pub const FIXED_PAD_PROGRAM: u8 = 89;

/// Stop Accompaniment's engine state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StopAcmpState {
    /// The mode the toggle turns back on.
    pub(super) last: StopAcmp,
    /// The Fixed voices went out on the Bass and Pad channels since the style's setup last
    /// did: the style's voices must go back before they sound again.
    pub(super) fixed_sent: bool,
}

impl Default for StopAcmpState {
    fn default() -> StopAcmpState {
        StopAcmpState { last: StopAcmp::Style, fixed_sent: false }
    }
}

/// The Style Bass and Pad channels (0-based: MIDI 11 and 14).
const PAD_CH: u8 = 13;

impl Engine {
    /// New Change Behavior rules, for the next style change.
    pub fn set_change_rules(&mut self, r: ChangeRules) {
        self.features.rules = ChangeRules { section: r.section.map(|s| s.min(3)), ..r };
    }

    pub fn change_rules(&self) -> ChangeRules {
        self.features.rules
    }

    /// The tempo after a style change to `self.style` (already the new style).
    pub(super) fn tempo_after_change(&self) -> f64 {
        if self.features.rules.tempo.resets(self.running) {
            self.style.bpm
        } else {
            self.bpm
        }
    }

    /// The Style part on/off states and the Main after a style change to `self.style`:
    /// Part On/Off turns every part on unless it keeps them; Section Set (stopped only)
    /// picks the Main.
    pub(super) fn parts_after_change(&mut self) {
        if self.features.rules.parts.resets(self.running) {
            self.parts = 0xFF;
        }
        if let (false, Some(m)) = (self.running, self.features.rules.section) {
            self.main = self.style.resolve(4 + m as usize).map_or(m, |s| (s - 4) as u8);
        }
    }

    /// OTS recall turns Sync Start on (OM p.47): stopped, the next chord starts the band.
    /// Playing, nothing changes (Sync Start pressed while playing would stop the band).
    pub fn sync_start_on(&mut self) {
        if !self.running {
            self.sync_armed = true;
        }
    }

    /// Stop Accompaniment on/off (`Button::StopAcmp`): Off <-> the mode last on.
    pub(super) fn toggle_stop_acmp(&mut self, sink: &mut impl Sink) {
        let m = if self.stop_acmp == StopAcmp::Off { self.features.stop_acmp.last } else { StopAcmp::Off };
        self.set_stop_acmp(m, sink);
    }

    /// A new Stop Accompaniment mode. What it sounded stops; leaving Fixed puts the style's
    /// Bass and Pad voices back.
    pub(super) fn set_stop_acmp(&mut self, m: StopAcmp, sink: &mut impl Sink) {
        if m == self.stop_acmp {
            return;
        }
        self.off_where(sink, |n| n.src == STOP_ACMP_SRC);
        if m != StopAcmp::Off {
            self.features.stop_acmp.last = m;
        }
        self.stop_acmp = m;
        if m != StopAcmp::Fixed {
            self.restore_stop_acmp_voices(sink);
        }
    }

    /// The Bass and Pad channels get the voices the mode sounds on: Fixed sends the fixed
    /// voices (bank 0), Style the style's own if Fixed replaced them.
    pub(super) fn stop_acmp_voices(&mut self, sink: &mut impl Sink) {
        if self.stop_acmp != StopAcmp::Fixed {
            self.restore_stop_acmp_voices(sink);
            return;
        }
        for (ch, prog) in [(BASS_CH, FIXED_BASS_PROGRAM), (PAD_CH, FIXED_PAD_PROGRAM)] {
            let d = ch as usize;
            if self.mirror.voice[d] != Some((0, 0, prog)) {
                self.mirror.send(sink, &[0xB0 | ch, 0, 0]);
                self.mirror.send(sink, &[0xB0 | ch, 32, 0]);
                self.mirror.send(sink, &[0xC0 | ch, prog]);
            }
        }
        self.features.stop_acmp.fixed_sent = true;
    }

    /// The style's whole setup just went out (a start, a style load): the Fixed voices are
    /// gone from the Bass and Pad channels.
    #[inline]
    pub(super) fn stop_acmp_setup_sent(&mut self) {
        self.features.stop_acmp.fixed_sent = false;
    }

    /// The style's Bass and Pad voices again after the Fixed ones (only what differs goes
    /// out). Stopped only: a start sends the whole setup anyway.
    fn restore_stop_acmp_voices(&mut self, sink: &mut impl Sink) {
        if !self.features.stop_acmp.fixed_sent || self.running {
            return;
        }
        self.features.stop_acmp.fixed_sent = false;
        self.reapply_init(0, sink);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_follow_the_manual() {
        use ChangeRule::*;
        assert_eq!([Lock, Hold, Reset].map(|r| r.resets(true)), [false, false, true]);
        assert_eq!([Lock, Hold, Reset].map(|r| r.resets(false)), [false, true, true]);
    }
}

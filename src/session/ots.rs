//! One Touch Settings and OTS Link.

use super::Control;
use crate::api::{gm_name, CmdError, OtsCmd, OtsLinkTiming, OtsPart, OtsSetting, OtsState};
use crate::live::Cmd;
use crate::sff::SectionId;
use std::sync::atomic::Ordering::Relaxed;

impl Control {
    pub(super) fn ots_cmd(&mut self, c: OtsCmd) -> Result<(), CmdError> {
        let parts = self.shared.parts.clone();
        match c {
            OtsCmd::RecallOts { index } => self.recall_ots(index),
            OtsCmd::SetOtsLink { on } => parts.ots_link.store(on, Relaxed),
            OtsCmd::ToggleOtsLink => {
                parts.ots_link.fetch_xor(true, Relaxed);
            }
            OtsCmd::SetOtsLinkTiming { timing } => self.ots_timing = timing,
        }
        Ok(())
    }

    /// Recall a One Touch Setting into the keyboard parts; the engine thread sends the new
    /// volumes as CC7 on its next wake. A recall also turns Sync Start on (ACMP is always
    /// on here), so the next chord starts a stopped band (OM p.47; DL: OTS stores "ACMP
    /// on, Sync Start on"). The engine ignores it while the band plays.
    fn recall_ots(&mut self, index: u8) {
        if let Some(o) = self.info.ots.get(index as usize) {
            self.shared.parts.apply_ots(o, index + 1);
            // The parts the OTS gives a voice play it (through the program map), not their
            // own library patch (#103).
            let voiced: Vec<usize> = o.parts.iter().enumerate().filter(|(_, q)| q.voice.is_some_and(|v| v.0 < 126)).map(|(p, _)| p).collect();
            for p in voiced {
                self.sound_library_part_voice(p);
            }
            if self.engine_cmd(Cmd::SyncStartOn).is_err() {
                self.wake_engine();
            }
        }
    }

    /// OTS Link (every pump, on the latest snapshot): Main A-D recall One Touch Settings
    /// 1-4 when the Main changes, on a style change, and when Link is switched on. When the
    /// Main "changes" is OTS Link Timing: Immediate, as it is pressed (`main`, which moves at
    /// once); At Main Section Change, when that Main starts playing (the section playing;
    /// an Intro, fill or break in between changes nothing). Stopped, both follow the press,
    /// except that stopping the band is not itself a change: a Main pressed but not yet
    /// played when the band stops waits for the band to start it or for a Main press (the
    /// same Main again included). Switching the timing while it waits changes nothing
    /// either: it waits under both. The OTS number is the button's: a Main the style lacks
    /// plays its neighbour, and recalls the pressed button's OTS under both timings.
    pub(super) fn pump_ots_link(&mut self) {
        let s = self.snap;
        let at_change = self.ots_timing == OtsLinkTiming::MainChange;
        let stop_key = (s.main, s.main_presses);
        if at_change && self.ots_was_running && !s.running {
            self.ots_stop_main = Some(stop_key);
        }
        if s.running || self.ots_stop_main != Some(stop_key) {
            self.ots_stop_main = None;
        }
        self.ots_was_running = s.running;
        let held = self.last_ots_key.filter(|k| k.0 == self.cur).map_or(s.main, |k| k.1);
        let main = match (at_change, s.running, s.cur) {
            (true, true, Some(SectionId::Main(m))) if resolve_main(&self.info.has, s.main) == Some(m) => s.main,
            (true, true, Some(SectionId::Main(m))) => m,
            (true, true, _) => held,
            (_, false, _) if self.ots_stop_main.is_some() => held,
            _ => s.main,
        };
        let key = (self.cur, main);
        let link = self.shared.parts.ots_link.load(Relaxed);
        // A Registration recall settling: its voices, not the OTS of its section.
        if self.registration_holds_ots() {
            self.last_ots_key = Some(key);
            self.last_link = link;
            return;
        }
        let due = link && (self.last_ots_key != Some(key) || !self.last_link);
        if due && (main as usize) < self.info.ots.len() {
            self.recall_ots(main);
        }
        self.last_ots_key = Some(key);
        self.last_link = link;
    }

    pub(super) fn ots_state(&self) -> OtsState {
        let kp = &self.shared.parts;
        OtsState {
            settings: self
                .info
                .ots
                .iter()
                .take(4)
                .enumerate()
                .map(|(i, o)| OtsSetting {
                    name: format!("OTS {}", i + 1),
                    parts: o
                        .parts
                        .iter()
                        .map(|q| {
                            let program = q.voice.filter(|v| v.0 < 126).map(|v| v.2);
                            OtsPart {
                                on: q.on,
                                program,
                                voice_name: program.map_or("drum kit", gm_name).to_string(),
                                volume: q.volume,
                                octave: q.octave,
                            }
                        })
                        .collect(),
                })
                .collect(),
            applied: kp.ots_applied.load(Relaxed),
            link: kp.ots_link.load(Relaxed),
            link_timing: self.ots_timing,
        }
    }
}

/// The Main that plays for Main button `i` (the engine's rule: the nearest the style has,
/// left first).
fn resolve_main(has: &[bool], i: u8) -> Option<u8> {
    let i = i as i32;
    (0..4).flat_map(|d| [i - d, i + d]).filter(|j| (0..4).contains(j)).find(|&j| has.get(4 + j as usize) == Some(&true)).map(|j| j as u8)
}

#[cfg(test)]
mod tests {
    use super::resolve_main;

    #[test]
    fn resolve_main_matches_the_engine_rule() {
        let has = |mains: [bool; 4]| {
            let mut h = [false; 8];
            h[4..8].copy_from_slice(&mains);
            h
        };
        assert_eq!(resolve_main(&has([true, true, true, false]), 3), Some(2));
        assert_eq!(resolve_main(&has([true, false, true, true]), 1), Some(0), "left first");
        assert_eq!(resolve_main(&has([false, false, false, true]), 0), Some(3));
        assert_eq!(resolve_main(&has([false; 4]), 0), None);
    }
}

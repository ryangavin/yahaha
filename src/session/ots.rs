//! One Touch Settings and OTS Link.

use super::Control;
use crate::api::{gm_name, CmdError, OtsCmd, OtsPart, OtsSetting, OtsState};
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
        }
        Ok(())
    }

    /// Recall a One Touch Setting into the keyboard parts; the engine thread sends the new
    /// volumes as CC7 on its next wake.
    fn recall_ots(&mut self, index: u8) {
        if let Some(o) = self.info.ots.get(index as usize) {
            self.shared.parts.apply_ots(o, index + 1);
            // The parts the OTS gives a voice play it (through the program map), not their
            // own library patch (#103).
            let voiced: Vec<usize> = o.parts.iter().enumerate().filter(|(_, q)| q.voice.is_some_and(|v| v.0 < 126)).map(|(p, _)| p).collect();
            for p in voiced {
                self.sound_library_part_voice(p);
            }
            self.wake_engine();
        }
    }

    /// OTS Link (every pump, on the latest snapshot): Main A-D recall One Touch Settings
    /// 1-4, when the Main changes, on a style change, and when Link is switched on.
    pub(super) fn pump_ots_link(&mut self) {
        let s = self.snap;
        let key = (self.cur, s.main);
        let link = self.shared.parts.ots_link.load(Relaxed);
        let due = link && (self.last_ots_key != Some(key) || !self.last_link);
        if due && (s.main as usize) < self.info.ots.len() {
            self.recall_ots(s.main);
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
        }
    }
}

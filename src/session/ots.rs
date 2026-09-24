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
            if self.engine_cmd(Cmd::SyncStartOn).is_err() {
                self.wake_engine();
            }
        }
    }

    /// OTS Link (every pump, on the latest snapshot): Main A-D recall One Touch Settings
    /// 1-4 when the Main changes, on a style change, and when Link is switched on. When the
    /// Main "changes" is OTS Link Timing: Immediate, as it is pressed (`main`, which moves at
    /// once); At Main Section Change, when that Main starts playing (the section playing;
    /// an Intro, fill or break in between changes nothing). Stopped, both follow the press.
    pub(super) fn pump_ots_link(&mut self) {
        let s = self.snap;
        let main = match (self.ots_timing, s.running, s.cur) {
            (OtsLinkTiming::MainChange, true, Some(SectionId::Main(m))) => m,
            (OtsLinkTiming::MainChange, true, _) => self.last_ots_key.filter(|k| k.0 == self.cur).map_or(s.main, |k| k.1),
            _ => s.main,
        };
        let key = (self.cur, main);
        let link = self.shared.parts.ots_link.load(Relaxed);
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

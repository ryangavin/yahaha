//! Sections and transport: every command is an engine button.

use super::{Control, View};
use crate::api::{CmdError, TransportCmd, TransportState};
use crate::launchkey::Page;
use crate::live::Cmd;

impl Control {
    pub(super) fn transport_cmd(&mut self, c: TransportCmd) -> Result<(), CmdError> {
        self.engine_cmd(Cmd::Button(c.button()))
    }

    pub(super) fn transport_state(&self, v: &View) -> TransportState {
        let s = &self.snap;
        TransportState {
            running: s.running,
            sync_start: s.sync_armed,
            sync_stop: s.sync_stop,
            sync_stop_available: self.shared.sync_stop_allowed(),
            auto_fill: s.auto_fill,
            stop_acmp: s.stop_acmp,
            section: s.cur.map(|c| c.name()),
            queued: s.queued.map(|q| q.name()),
            pending_intro: s.pending_intro,
            main: s.main,
            bar: s.bar + 1,
            beat: s.beat + 1,
            beats_per_bar: self.info.timesig.0,
            section_bars: (s.section_bars > 0).then_some(s.section_bars),
            tempo: s.bpm,
            lamps: self.pads(&v.pnl, Page::Sections),
            half_bar_fill: s.half_bar_fill,
            stop_acmp_mode: s.stop_acmp_mode.into(),
        }
    }
}

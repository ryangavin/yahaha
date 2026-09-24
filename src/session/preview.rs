//! Style preview (`AuditionStyle`): an engine of its own, played on the engine thread
//! beside the stopped band (`live::Audition`), and the style waiting for the bar line.

use super::Control;
use crate::api::{AuditionState, CmdError, PreviewCmd, PreviewState};
use crate::engine::Engine;
use crate::live::{Audition, Cmd};

/// The chords a style preview plays, one a bar.
pub const AUDITION_CHORDS: [&str; 4] = ["C", "Am", "F", "G7"];

impl Control {
    pub(super) fn preview_cmd(&mut self, c: PreviewCmd) -> Result<(), CmdError> {
        match c {
            PreviewCmd::AuditionStyle { id } => self.audition(id),
            PreviewCmd::StopAudition => self.engine_cmd(Cmd::StopAudition),
        }
    }

    /// A style preview (`AuditionStyle`), for the engine thread.
    fn audition(&mut self, id: usize) -> Result<(), CmdError> {
        if self.snap.running {
            return self.fail("Stop the band to preview a style");
        }
        let (p, _) = self.load_entry(id)?;
        let engine = Engine::new(p);
        let chords = AUDITION_CHORDS.map(|c| crate::parse_chord(c).expect("audition chord"));
        if self.audition_tx.push(Box::new(Audition { engine, id: id as u32, chords })).is_err() {
            return Err(CmdError::Busy);
        }
        self.wake_engine();
        Ok(())
    }

    pub(super) fn preview_state(&self) -> PreviewState {
        PreviewState {
            audition: self.snap.audition.map(|a| AuditionState {
                id: a.id as usize,
                bar: a.bar,
                bars: a.bars,
                chord: AUDITION_CHORDS.get(a.chord as usize).map(|c| c.to_string()),
            }),
            queued: self.pending_style.as_ref().map(|p| p.0),
        }
    }
}

//! Chord detection, split, transpose.

use super::{Control, View};
use crate::api::{note_name, ChordCmd, ChordState, CmdError};
use crate::engine::Transpose;
use crate::fingering::Fingering;
use crate::live::Cmd;
use std::sync::atomic::Ordering::Relaxed;

impl Control {
    pub(super) fn chord_cmd(&mut self, c: ChordCmd) -> Result<(), CmdError> {
        match c {
            ChordCmd::SetFingering { fingering } => {
                self.shared.fingering.store(fingering.to_u8(), Relaxed);
                self.wake_engine();
            }
            ChordCmd::NextFingering => {
                let f = Fingering::from_u8(self.shared.fingering.load(Relaxed)).next();
                self.shared.fingering.store(f.to_u8(), Relaxed);
                self.wake_engine();
            }
            ChordCmd::SetUpper { on } => {
                if on != self.shared.upper.load(Relaxed) {
                    self.set_upper(on);
                }
            }
            ChordCmd::ToggleUpper => {
                let on = !self.shared.upper.load(Relaxed);
                self.set_upper(on);
            }
            // Manual Bass is only available in Upper mode.
            ChordCmd::SetManualBass { on } => {
                if self.shared.upper.load(Relaxed) {
                    self.shared.manual_bass.store(on, Relaxed);
                    self.sync_manual_bass();
                }
            }
            ChordCmd::ToggleManualBass => {
                if self.shared.upper.load(Relaxed) {
                    let v = !self.shared.manual_bass.load(Relaxed);
                    self.shared.manual_bass.store(v, Relaxed);
                    self.sync_manual_bass();
                }
            }
            ChordCmd::SetSplit { note } => self.shared.split.store(note.clamp(24, 96), Relaxed),
            ChordCmd::MoveSplit { delta } => {
                let s = self.shared.split.load(Relaxed) as i16 + delta as i16;
                self.shared.split.store(s.clamp(24, 96) as u8, Relaxed);
            }
            ChordCmd::SetTranspose { keyboard, master } => return self.set_transpose(Transpose::new(keyboard, master)),
            ChordCmd::StepTranspose { keyboard, master } => {
                let t = self.transpose;
                return self.set_transpose(Transpose::new(t.keyboard.saturating_add(keyboard), t.master.saturating_add(master)));
            }
            ChordCmd::ResetTranspose => return self.set_transpose(Transpose::default()),
        }
        Ok(())
    }

    /// Push the effective Manual Bass state (Upper mode and the setting both on) to the
    /// engine, which mutes the Style's Bass part, and to the keyboard parts, where the Left
    /// part takes the Style's Bass voice and fader.
    pub(super) fn sync_manual_bass(&mut self) {
        let on = self.shared.manual_bass();
        if self.ui_tx.push(Cmd::ManualBass(on)).is_ok() {
            self.wake_engine();
        }
        self.shared.parts.set_manual_bass(on);
    }

    /// Played notes and the engine must agree, so the key shift only changes once the
    /// engine has the command.
    pub(super) fn set_transpose(&mut self, t: Transpose) -> Result<(), CmdError> {
        let t = Transpose::new(t.keyboard, t.master);
        self.engine_cmd(Cmd::Transpose(t))?;
        self.shared.key_shift.store(t.keys(), Relaxed);
        self.transpose = t;
        Ok(())
    }

    fn set_upper(&mut self, on: bool) {
        self.shared.upper.store(on, Relaxed);
        // Selecting Upper turns Manual Bass on, its default there.
        if on {
            self.shared.manual_bass.store(true, Relaxed);
        }
        self.sync_manual_bass();
        self.wake_engine();
    }

    pub(super) fn chord_state(&self, v: &View) -> ChordState {
        let s = &self.snap;
        ChordState {
            name: s.chord.map(|c| c.name()),
            fingered: s.played.map(|c| c.name()),
            fingering: v.fingering,
            fingering_name: v.fingering.name().to_string(),
            upper: v.upper,
            manual_bass: self.shared.manual_bass.load(Relaxed),
            manual_bass_active: v.manual_bass_active,
            split: v.split,
            split_name: note_name(v.split),
            transpose_keyboard: s.transpose.keyboard,
            transpose_master: s.transpose.master,
        }
    }
}

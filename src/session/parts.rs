//! Keyboard parts (Right 1-3, Left).

use super::{Control, View};
use crate::api::{gm_name, CmdError, KeyboardPart, PartsCmd};
use crate::parts;
use std::sync::atomic::Ordering::Relaxed;

impl Control {
    pub(super) fn parts_cmd(&mut self, c: PartsCmd) -> Result<(), CmdError> {
        let parts = self.shared.parts.clone();
        match c {
            PartsCmd::SetPartOn { part, on } => return self.set_part_on(part, on),
            PartsCmd::TogglePart { part } => {
                let on = !parts.is_on((part & 3) as usize);
                return self.set_part_on(part, on);
            }
            PartsCmd::SelectPart { part } => parts.select(part as usize),
            PartsCmd::SetPartVoice { part, program } => parts.set_program((part & 3) as usize, program),
            PartsCmd::StepVoice { delta } => parts.step_program(delta as i32),
            PartsCmd::SetPartVolume { part, volume } => {
                parts.set_volume((part & 3) as usize, volume);
                self.wake_engine();
            }
            PartsCmd::SetPartOctave { part, octave } => parts.octave[(part & 3) as usize].store(octave.clamp(-2, 2), Relaxed),
        }
        Ok(())
    }

    fn set_part_on(&mut self, part: u8, on: bool) -> Result<(), CmdError> {
        let p = (part & 3) as usize;
        if self.shared.parts.is_on(p) != on && !self.shared.parts.toggle(p) {
            return self.fail("Left plays the bass under Manual Bass: turn Manual Bass off [D] to switch Left");
        }
        // The engine thread gives a part switched on the held pedal and wheels, and one
        // switched off their release (`Controllers::sync`).
        self.wake_engine();
        Ok(())
    }

    pub(super) fn keyboard_parts_state(&self, v: &View) -> Vec<KeyboardPart> {
        let kp = &self.shared.parts;
        (0..parts::COUNT)
            .map(|p| {
                let plays_bass = p == parts::LEFT && kp.manual_bass.load(Relaxed);
                KeyboardPart {
                    name: parts::NAMES[p].to_string(),
                    channel: parts::CHANNEL[p] + 1,
                    on: kp.is_on(p),
                    sounding: if p == parts::LEFT { kp.left_audible() } else { kp.audible(p) },
                    selected: kp.selected() == p,
                    volume: kp.volume(p),
                    waiting: kp.waiting(p),
                    program: kp.program[p].load(Relaxed),
                    voice_name: gm_name(kp.channel_program(p)).to_string(),
                    plays_bass,
                    octave: kp.octave[p].load(Relaxed).clamp(-2, 2),
                    fader: v.fader_hw[p],
                }
            })
            .collect()
    }
}

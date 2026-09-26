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
            // A GM voice selected replaces the part's voice (#179): a plugin picked for the
            // part ends (`end_picked_plugin`), and its own library patch goes, with a plugin
            // that patch brought (`sound_library_part_voice`).
            PartsCmd::SetPartVoice { part, program } => {
                let p = (part & 3) as usize;
                parts.set_program(p, program);
                self.end_picked_plugin(p);
                self.sound_library_part_voice(p);
                // The voice settings back to neutral (#238): the engine thread sends them.
                self.wake_engine();
            }
            PartsCmd::StepVoice { delta } => {
                parts.step_program(delta as i32);
                let p = parts.selected();
                self.end_picked_plugin(p);
                self.sound_library_part_voice(p);
                // The voice settings back to neutral (#238): the engine thread sends them.
                self.wake_engine();
            }
            PartsCmd::SetPartVolume { part, volume } => {
                parts.set_volume((part & 3) as usize, volume);
                self.wake_engine();
            }
            PartsCmd::SetPartOctave { part, octave } => parts.octave[(part & 3) as usize].store(octave.clamp(-2, 2), Relaxed),
            // The engine thread sends it as the part's CC, to the port and the synth.
            PartsCmd::SetPartPan { part, pan } => {
                parts.set_fx((part & 3) as usize, [Some(pan), None, None, None]);
                self.wake_engine();
            }
            PartsCmd::SetPartSend { part, send, value } => {
                let mut fx = [None; parts::FX];
                fx[send.index()] = Some(value);
                parts.set_fx((part & 3) as usize, fx);
                self.wake_engine();
            }
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
                // Its own sound library patch, and the patch its channel plays (#103).
                let (patch, plays) = self.part_sound(p);
                KeyboardPart {
                    name: parts::NAMES[p].to_string(),
                    channel: parts::CHANNEL[p] + 1,
                    on: kp.is_on(p),
                    sounding: if p == parts::LEFT { kp.left_audible() } else { kp.audible(p) },
                    selected: kp.selected() == p,
                    volume: kp.volume(p),
                    waiting: kp.waiting(p),
                    program: kp.program[p].load(Relaxed),
                    voice_name: plays.unwrap_or_else(|| gm_name(kp.channel_program(p)).to_string()),
                    plays_bass,
                    octave: kp.octave[p].load(Relaxed).clamp(-2, 2),
                    pan: kp.fx(p)[parts::PAN],
                    reverb: kp.fx(p)[parts::REVERB],
                    chorus: kp.fx(p)[parts::CHORUS],
                    variation: kp.fx(p)[parts::VARIATION],
                    fader: v.fader_hw[p],
                    plugin: self.channel_plugin_state(parts::CHANNEL[p]),
                    patch,
                }
            })
            .collect()
    }
}

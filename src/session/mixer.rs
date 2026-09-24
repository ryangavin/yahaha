//! The mixer: Style part mute and levels, the fader page, the synth's master and mute.

use super::{Control, View};
use crate::api::{voice_label, CmdError, MixerCmd, MixerState, StylePart, Voice, STYLE_PART_NAMES};
use crate::live::Cmd;
use std::sync::atomic::Ordering::Relaxed;

impl Control {
    pub(super) fn mixer_cmd(&mut self, c: MixerCmd) -> Result<(), CmdError> {
        if let Some(b) = c.button() {
            return self.engine_cmd(Cmd::Button(b));
        }
        let parts = self.shared.parts.clone();
        match c {
            MixerCmd::SetStylePartVolume { part, volume } => return self.engine_cmd(Cmd::StyleVolume(part & 7, volume.min(127))),
            MixerCmd::SetFaderPage { page } => {
                if parts.fader_page() != page {
                    parts.set_fader_page(page);
                    self.wake_engine();
                }
            }
            MixerCmd::ToggleFaderPage => {
                parts.toggle_fader_page();
                self.wake_engine();
            }
            MixerCmd::SetMasterVolume { volume } => match &self.synth {
                Some(s) => {
                    let v = volume.min(127);
                    s.control.master.store(v, Relaxed);
                    // The fader has to reach the new level before it takes over again.
                    let hw = self.shared.master_hw.load(Relaxed);
                    s.control.master_waiting.store(crate::engine::Takeover::at(hw, v).waiting(), Relaxed);
                    self.shared.master_moved.store(true, Relaxed);
                }
                None => return self.fail("the synth is off"),
            },
            MixerCmd::SetSynthMuted { on } => {
                if let Some(s) = &self.synth {
                    s.control.muted.store(on, Relaxed);
                }
            }
            MixerCmd::ToggleSynthMute => {
                if let Some(s) = &self.synth {
                    s.control.muted.fetch_xor(true, Relaxed);
                }
            }
            MixerCmd::SetStyleSolo { part } => return self.engine_cmd(Cmd::StyleSolo(part.map(|p| p & 7))),
            MixerCmd::SetPartSolo { part } => {
                parts.set_solo(part.map(|p| (p & 3) as usize));
            }
            MixerCmd::StyleTrackMute { order, value } => return self.engine_cmd(Cmd::StyleParts(order.mask(value))),
            // An engine button, handled above.
            MixerCmd::ToggleStylePart { .. } => {}
        }
        Ok(())
    }

    pub(super) fn mixer_state(&self, v: &View) -> MixerState {
        let s = &self.snap;
        MixerState {
            fader_page: self.shared.parts.fader_page(),
            style_parts: (0..8u8)
                .map(|p| {
                    // Manual Bass mutes the Style's Bass part (its voice moves to the left hand).
                    let mb = p == 2 && v.manual_bass_active;
                    let voice = self.info.voices[8 + p as usize];
                    StylePart {
                        name: STYLE_PART_NAMES[p as usize].to_string(),
                        channel: 9 + p,
                        on: s.parts & (1 << p) != 0 && !mb,
                        muted_by_manual_bass: mb,
                        volume: s.volumes[p as usize],
                        waiting: s.pickup & (1 << p) != 0,
                        fader: v.fader_hw[p as usize],
                        voice: voice.map(|(msb, lsb, program)| Voice {
                            bank_msb: msb,
                            bank_lsb: lsb,
                            program,
                            kit: msb >= 126 || p < 2,
                            label: voice_label(8 + p, voice),
                        }),
                    }
                })
                .collect(),
            master: self.synth.as_ref().map(|s| s.control.master.load(Relaxed)),
            master_waiting: self.synth.as_ref().is_some_and(|s| s.control.master_waiting.load(Relaxed)),
            style_solo: s.style_solo,
            part_solo: self.shared.parts.solo().map(|p| p as u8),
        }
    }
}

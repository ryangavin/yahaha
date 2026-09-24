//! Controllers: pedal setup, the parts the wheels and pedals reach, Pitch Bend Range, and
//! the assignable functions the control side runs (`crate::controllers`).

use super::Control;
use crate::api::{AppCmd, ChordCmd, CmdError, ControllersCmd, ControllersState, OtsCmd, PartControllers, PartsCmd, PedalState};
use crate::controllers::{Effect, Function, PartTargets, PEDALS, SOFT, SOSTENUTO, SUSTAIN};
use crate::fingering::Fingering;
use crate::parts;
use std::sync::atomic::Ordering::Relaxed;

impl Control {
    pub(super) fn controllers_cmd(&mut self, c: ControllersCmd) -> Result<(), CmdError> {
        let ctl = &self.shared.controllers;
        match c {
            ControllersCmd::SetPedal { pedal, .. } => {
                if pedal as usize >= PEDALS {
                    return self.fail(format!("there is no pedal {}", pedal as usize + 1));
                }
                if let Some(p) = c.pedal_setup() {
                    ctl.set_pedal(pedal as usize, p);
                }
            }
            ControllersCmd::LearnPedal { pedal } => ctl.learn(pedal.map(|p| p as usize).filter(|&p| p < PEDALS)),
            ControllersCmd::SetPartControllers { part, sustain, pitch_bend, modulation } => {
                ctl.set_part((part & 3) as usize, PartTargets { sustain, pitch_bend, modulation });
            }
            ControllersCmd::SetBendRange { part, semitones } => ctl.set_bend_range((part & 3) as usize, semitones),
            ControllersCmd::TriggerFunction { function } => return self.run_function(function),
        }
        // The engine thread sends the parts what changed.
        self.wake_engine();
        Ok(())
    }

    /// Run an assignable function (a pedal press, or `TriggerFunction`).
    pub(super) fn run_function(&mut self, f: Function) -> Result<(), CmdError> {
        let info = f.info();
        if !info.available {
            return self.fail(format!("{} is not in yahaha yet", info.name));
        }
        let cmd: AppCmd = match f.effect() {
            Effect::Nothing => return Ok(()),
            Effect::Engine(b) => b.into(),
            Effect::Switch(b) => {
                self.shared.controllers.toggle_switch(b);
                self.wake_engine();
                return Ok(());
            }
            Effect::Modulation | Effect::PitchBend => return self.fail(format!("{} needs a foot controller (an expression pedal)", info.name)),
            Effect::Control => match f {
                Function::OtsLink => OtsCmd::ToggleOtsLink.into(),
                Function::Ots1 | Function::Ots2 | Function::Ots3 | Function::Ots4 => {
                    OtsCmd::RecallOts { index: f as u8 - Function::Ots1 as u8 }.into()
                }
                Function::OtsNext | Function::OtsPrev => {
                    let n = self.info.ots.len().min(4) as u8;
                    if n == 0 {
                        return self.fail("this style has no One Touch Settings");
                    }
                    // `ots_applied` is 1-based (0: none yet). Wraps round, both ways.
                    let applied = self.shared.parts.ots_applied.load(Relaxed).min(n);
                    let index = match (f, applied) {
                        (Function::OtsNext, a) => a % n,
                        (_, 0) => n - 1,
                        (_, a) => (a + n - 2) % n,
                    };
                    OtsCmd::RecallOts { index }.into()
                }
                Function::TransposeUp => ChordCmd::StepTranspose { keyboard: 1, master: 0 }.into(),
                Function::TransposeDown => ChordCmd::StepTranspose { keyboard: -1, master: 0 }.into(),
                Function::Right1OnOff => PartsCmd::TogglePart { part: parts::RIGHT1 as u8 }.into(),
                Function::Right2OnOff => PartsCmd::TogglePart { part: parts::RIGHT2 as u8 }.into(),
                Function::Right3OnOff => PartsCmd::TogglePart { part: parts::RIGHT3 as u8 }.into(),
                Function::LeftOnOff => PartsCmd::TogglePart { part: parts::LEFT as u8 }.into(),
                Function::FingeredOnBass => {
                    let now = Fingering::from_u8(self.shared.fingering.load(Relaxed));
                    let fingering = if now == Fingering::FingeredOnBass { Fingering::Fingered } else { Fingering::FingeredOnBass };
                    ChordCmd::SetFingering { fingering }.into()
                }
                _ => return self.fail(format!("{} can't be run here", info.name)),
            },
        };
        self.apply(cmd)
    }

    pub(super) fn controllers_state(&self) -> ControllersState {
        let ctl = &self.shared.controllers;
        let down = ctl.down();
        let sw = ctl.switches();
        ControllersState {
            pedals: (0..PEDALS)
                .map(|i| {
                    let p = ctl.pedal(i);
                    PedalState { cc: p.cc, function: p.function, control_type: p.control_type, reverse: p.reverse, range: p.range, down: down >> i & 1 != 0 }
                })
                .collect(),
            learning: ctl.learning().map(|i| i as u8),
            parts: (0..parts::COUNT)
                .map(|p| {
                    let t = ctl.part_targets(p);
                    PartControllers { sustain: t.sustain, pitch_bend: t.pitch_bend, modulation: t.modulation, bend_range: ctl.bend_range(p) }
                })
                .collect(),
            sustain: sw & SUSTAIN != 0,
            sostenuto: sw & SOSTENUTO != 0,
            soft: sw & SOFT != 0,
        }
    }
}

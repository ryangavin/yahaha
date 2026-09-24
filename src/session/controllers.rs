//! Controllers: pedal setup, the parts the wheels and pedals reach, Pitch Bend Range, and
//! the assignable functions the control side runs (`crate::controllers`).

use super::Control;
use crate::api::{function_run, function_set, CmdError, ControllersCmd, ControllersState, FunctionRun};
use crate::controllers::{control_switch_sets, reset_release, Function, PEDALS};
use crate::fingering::Fingering;
use std::sync::atomic::Ordering::Relaxed;

impl Control {
    pub(super) fn controllers_cmd(&mut self, c: ControllersCmd) -> Result<(), CmdError> {
        let ctl = &self.shared.controllers;
        let before = match c {
            ControllersCmd::SetPedal { pedal, .. } => Some((pedal as usize % PEDALS, ctl.pedal(pedal as usize % PEDALS), ctl.down())),
            _ => None,
        };
        match c.apply_setting(&self.shared.controllers) {
            Ok(true) => {
                // A Hold pedal on Kbd Harmony/Arpeggio or Arpeggio Hold: the control side
                // keeps those switches, so it sets them where the new setup puts them.
                if let Some((i, old, old_down)) = before {
                    let ctl = &self.shared.controllers;
                    let new_down = ctl.down() >> i & 1 != 0;
                    for (f, on) in control_switch_sets(old, ctl.pedal(i), old_down >> i & 1 != 0, new_down).into_iter().flatten() {
                        if let Some(cmd) = function_set(f, on) {
                            self.apply(cmd)?;
                        }
                    }
                }
            }
            Ok(false) => {
                if let ControllersCmd::TriggerFunction { function } = c {
                    return self.run_function(function);
                }
            }
            Err(e) => return self.fail(e),
        }
        // The engine thread sends the parts what changed.
        self.wake_engine();
        Ok(())
    }

    /// A reset on the engine thread (Panic, a keyboard unplugged) let go of the pedals:
    /// the switches the control side keeps for a Hold pedal (Kbd Harmony/Arpeggio,
    /// Arpeggio Hold) go off where the pedal was keeping them on, as the pedal switches did
    /// in the reset. The pedal's own release sends no edge after a reset, so without this
    /// they would stay on with the pedal up.
    pub(super) fn pump_pedal_releases(&mut self) {
        let Some(down) = self.shared.controllers.take_reset_releases() else { return };
        for i in 0..PEDALS {
            if let Some(cmd) = reset_release(self.shared.controllers.pedal(i), down >> i & 1 != 0).and_then(|f| function_set(f, false)) {
                let _ = self.apply(cmd);
            }
        }
    }

    /// Run an assignable function (a pedal press, or `TriggerFunction`).
    pub(super) fn run_function(&mut self, f: Function) -> Result<(), CmdError> {
        let fingering = Fingering::from_u8(self.shared.fingering.load(Relaxed));
        let ots = self.info.ots.len().min(4) as u8;
        match function_run(f, fingering, ots, self.shared.parts.ots_applied.load(Relaxed)) {
            Ok(FunctionRun::Nothing) => Ok(()),
            Ok(FunctionRun::Cmd(c)) => self.apply(c),
            Ok(FunctionRun::Switch(b)) => {
                self.shared.controllers.toggle_switch(b);
                self.wake_engine();
                Ok(())
            }
            Err(e) => self.fail(e),
        }
    }

    pub(super) fn controllers_state(&self) -> ControllersState {
        ControllersState::of(&self.shared.controllers)
    }
}

#[cfg(test)]
mod tests {
    use crate::api::*;
    use crate::controllers::{ControlType, Function};
    use crate::session::{Options, Port, Session};
    use std::path::Path;

    fn offline() -> Option<Session> {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
        if !p.exists() {
            eprintln!("corpus missing; skipping");
            return None;
        }
        Some(Session::offline(Options { paths: vec![p], ..Options::default() }).unwrap())
    }

    fn pedal(pedal: u8, cc: u8, function: Function) -> ControllersCmd {
        ControllersCmd::SetPedal { pedal, cc: Some(cc), function, control_type: Default::default(), reverse: false, range: Default::default() }
    }

    /// Messages on the keyboard parts' channels (1-4).
    fn keyboard_msgs(s: &Session) -> Vec<[u8; 3]> {
        s.take_output().into_iter().filter(|m| m[0] & 0x0F < 4 && m[0] < 0xF0).collect()
    }

    /// A bar of SlowWalker (75 bpm, 4/4: 3.2 s), and a little.
    const BAR: u64 = 3_300_000_000;

    #[test]
    fn a_pedal_runs_the_style() {
        let Some(s) = offline() else { return };
        s.send(pedal(1, 66, Function::StartStop)).unwrap();
        s.send(pedal(2, 67, Function::FillUp)).unwrap();
        s.midi_in(Port::Keys, &[0xB0, 66, 127]);
        let st = s.state();
        assert!(st.transport.running, "Start/Stop from the pedal");
        assert!(st.controllers.pedals[1].down);
        s.midi_in(Port::Keys, &[0xB0, 66, 0]);
        assert!(s.state().transport.running, "nothing on release");
        // Fill Up from Main A: Main B's fill, then Main B.
        s.advance(BAR / 3);
        s.midi_in(Port::Keys, &[0xB0, 67, 127]);
        assert_eq!(s.state().transport.queued.as_deref(), Some("Fill In BB"));
        s.advance(2 * BAR);
        assert_eq!(s.state().transport.section.as_deref(), Some("Main B"));
        s.midi_in(Port::Keys, &[0xB0, 67, 0]);
        s.midi_in(Port::Keys, &[0xB0, 66, 127]);
        assert!(!s.state().transport.running);
    }

    #[test]
    fn control_side_functions_from_a_pedal_and_from_software() {
        let Some(s) = offline() else { return };
        s.send(pedal(1, 66, Function::OtsNext)).unwrap();
        s.midi_in(Port::Keys, &[0xB0, 66, 127]);
        assert_eq!(s.state().ots.applied, 1, "OTS + from none: OTS 1");
        s.midi_in(Port::Keys, &[0xB0, 66, 0]);
        s.midi_in(Port::Keys, &[0xB0, 66, 127]);
        assert_eq!(s.state().ots.applied, 2);
        s.send(ControllersCmd::TriggerFunction { function: Function::OtsPrev }).unwrap();
        assert_eq!(s.state().ots.applied, 1);
        let r2 = s.state().keyboard_parts[1].on;
        s.send(ControllersCmd::TriggerFunction { function: Function::Right2OnOff }).unwrap();
        assert_ne!(s.state().keyboard_parts[1].on, r2);
        s.send(ControllersCmd::TriggerFunction { function: Function::FingeredOnBass }).unwrap();
        let f = s.state().chord.fingering;
        s.send(ControllersCmd::TriggerFunction { function: Function::FingeredOnBass }).unwrap();
        assert_ne!(s.state().chord.fingering, f, "it alternates");
        let e = s.send(ControllersCmd::TriggerFunction { function: Function::RegistBankNext }).unwrap_err();
        assert!(matches!(e, CmdError::Failed(t) if t.contains("not in yahaha yet")));
        s.send(ControllersCmd::TriggerFunction { function: Function::Sustain }).unwrap();
        assert!(s.state().controllers.sustain);
    }

    /// Kbd Harmony/Arpeggio On/Off and Arpeggio Hold as pedal functions (RM p.141): a Hold
    /// A pedal holds the arpeggio while it is down, a Toggle pedal switches Harmony/Arpeggio.
    #[test]
    fn a_pedal_holds_the_arpeggio_and_switches_harmony() {
        let Some(s) = offline() else { return };
        let set = |function, control_type| ControllersCmd::SetPedal { pedal: 1, cc: Some(66), function, control_type, reverse: false, range: Default::default() };
        s.send(set(Function::ArpHold, ControlType::HoldA)).unwrap();
        assert!(!s.state().harmony_arp.arp.pedal_hold);
        s.midi_in(Port::Keys, &[0xB0, 66, 127]);
        assert!(s.state().harmony_arp.arp.pedal_hold, "Hold A: on while the pedal is down");
        s.midi_in(Port::Keys, &[0xB0, 66, 0]);
        assert!(!s.state().harmony_arp.arp.pedal_hold);
        s.send(set(Function::ArpHold, ControlType::HoldB)).unwrap();
        assert!(s.state().harmony_arp.arp.pedal_hold, "Hold B picked with the pedal up: on at once");
        s.midi_in(Port::Keys, &[0xB0, 66, 127]);
        assert!(!s.state().harmony_arp.arp.pedal_hold, "Hold B: off while down");
        s.send(set(Function::KbdHarmonyArp, ControlType::Toggle)).unwrap();
        s.midi_in(Port::Keys, &[0xB0, 66, 0]);
        assert!(!s.state().harmony_arp.on);
        s.midi_in(Port::Keys, &[0xB0, 66, 127]);
        assert!(s.state().harmony_arp.on, "Toggle: a press switches it");
        s.midi_in(Port::Keys, &[0xB0, 66, 0]);
        assert!(s.state().harmony_arp.on, "and it stays on after release");
        s.send(ControllersCmd::TriggerFunction { function: Function::KbdHarmonyArp }).unwrap();
        assert!(!s.state().harmony_arp.on);
        s.send(ControllersCmd::TriggerFunction { function: Function::ArpHold }).unwrap();
        let st = s.state();
        assert!(st.harmony_arp.arp.pedal_hold, "Try: Arpeggio Hold (the function) switches");
        assert!(!st.harmony_arp.arp.hold, "and leaves the Hold setting");
        // A Hold A pedal held down when it is given another function lets its switch go.
        s.send(set(Function::KbdHarmonyArp, ControlType::HoldA)).unwrap();
        s.midi_in(Port::Keys, &[0xB0, 66, 127]);
        assert!(s.state().harmony_arp.on);
        s.send(set(Function::StartStop, ControlType::HoldA)).unwrap();
        assert!(!s.state().harmony_arp.on, "re-picked while held: Harmony/Arpeggio off");
    }

    #[test]
    fn sustain_follows_the_parts_switches() {
        let Some(s) = offline() else { return };
        s.send(PartsCmd::SetPartOn { part: 1, on: false }).unwrap();
        s.take_output();
        // Right 1 is on: the pedal reaches it only.
        s.midi_in(Port::Keys, &[0xB0, 64, 127]);
        let out = keyboard_msgs(&s);
        assert!(out.contains(&[0xB0, 64, 127]) && !out.contains(&[0xB2, 64, 127]), "{out:?}");
        assert!(s.state().controllers.sustain);
        // Right 2 comes on while the pedal is down: it is sustained too.
        s.send(PartsCmd::SetPartOn { part: 1, on: true }).unwrap();
        assert!(keyboard_msgs(&s).contains(&[0xB2, 64, 127]));
        // Right 1 goes off: released.
        s.send(PartsCmd::SetPartOn { part: 0, on: false }).unwrap();
        assert!(keyboard_msgs(&s).contains(&[0xB0, 64, 0]));
        // Left is chosen out of the sustain.
        s.send(ControllersCmd::SetPartControllers { part: 3, sustain: false, pitch_bend: true, modulation: false }).unwrap();
        s.send(ChordCmd::SetUpper { on: true }).unwrap(); // Left sounds (Manual Bass)
        assert!(s.state().keyboard_parts[3].sounding);
        assert!(!keyboard_msgs(&s).contains(&[0xB1, 64, 127]));
        s.midi_in(Port::Keys, &[0xB0, 64, 0]);
        assert!(keyboard_msgs(&s).contains(&[0xB2, 64, 0]));
    }

    /// A Toggle sustain latched on, then the pedal given another function in Settings: the
    /// sustain lets go (it would otherwise stay on with nothing left to turn it off).
    #[test]
    fn a_pedal_given_another_function_releases_its_sustain() {
        let Some(s) = offline() else { return };
        let toggle = |function| ControllersCmd::SetPedal { pedal: 0, cc: Some(64), function, control_type: crate::controllers::ControlType::Toggle, reverse: false, range: Default::default() };
        s.send(toggle(Function::Sustain)).unwrap();
        s.midi_in(Port::Keys, &[0xB0, 64, 127]);
        s.midi_in(Port::Keys, &[0xB0, 64, 0]);
        assert!(s.state().controllers.sustain, "latched");
        s.take_output();
        s.send(toggle(Function::OtsNext)).unwrap();
        assert!(!s.state().controllers.sustain);
        assert!(keyboard_msgs(&s).contains(&[0xB0, 64, 0]), "Right 1 released");
    }

    #[test]
    fn a_held_pedal_survives_the_band_and_panic_releases_it() {
        let Some(s) = offline() else { return };
        s.midi_in(Port::Keys, &[0xB0, 64, 127]);
        s.midi_in(Port::Keys, &[0xE0, 0, 0x60]);
        s.take_output();
        // Start, a section change, a style reload and stop never touch the keyboard parts.
        s.send(TransportCmd::StartStop).unwrap();
        s.advance(BAR);
        s.send(TransportCmd::Main { index: 2 }).unwrap();
        s.advance(2 * BAR);
        s.send(LibraryCmd::StepStyle { delta: 0 }).unwrap();
        s.advance(BAR);
        s.send(TransportCmd::StartStop).unwrap();
        assert_eq!(keyboard_msgs(&s), Vec::<[u8; 3]>::new());
        assert!(s.state().controllers.sustain);
        // Panic: the pedal and bend released before All Notes Off, and they stay released.
        s.send(SystemCmd::Panic).unwrap();
        let out = keyboard_msgs(&s);
        let at = |m: [u8; 3]| out.iter().position(|x| *x == m);
        assert!(at([0xB0, 64, 0]).is_some() && at([0xB0, 64, 0]) < at([0xB0, 123, 0]), "{out:?}");
        assert!(at([0xE0, 0, 0x40]).is_some());
        assert!(!s.state().controllers.sustain);
        s.send(PartsCmd::SetPartOn { part: 2, on: true }).unwrap();
        assert!(!keyboard_msgs(&s).contains(&[0xB3, 64, 127]), "not sent again after panic");
    }

    /// A CC a pedal can't use (the CC7 rule drops 0/7/32 before the pedals see them; 121 is
    /// Reset All Controllers; 1 is the wheel) is refused, not silently dead.
    #[test]
    fn a_pedal_refuses_ccs_it_could_never_hear() {
        let Some(s) = offline() else { return };
        for cc in [0, 1, 7, 32, 121] {
            let e = s.send(pedal(1, cc, Function::StartStop)).unwrap_err();
            assert!(matches!(&e, CmdError::Failed(t) if t.contains(&format!("CC {cc}"))), "{e:?}");
            assert_eq!(s.state().controllers.pedals[1].cc, Some(66), "unchanged");
        }
        s.send(pedal(1, 85, Function::StartStop)).unwrap();
        s.midi_in(Port::Keys, &[0xB0, 85, 127]);
        assert!(s.state().transport.running);
    }

    /// Transpose +/− from a pedal is the TRANSPOSE buttons (RM p.144): Master transpose.
    #[test]
    fn transpose_pedal_moves_master_transpose() {
        let Some(s) = offline() else { return };
        s.send(pedal(1, 66, Function::TransposeUp)).unwrap();
        s.midi_in(Port::Keys, &[0xB0, 66, 127]);
        let c = s.state().chord.clone();
        assert_eq!((c.transpose_master, c.transpose_keyboard), (1, 0));
    }

    /// Hold B picked in Settings with the pedal up: Sustain comes on at once, as the tooltip
    /// says, and pressing the pedal lets go.
    #[test]
    fn hold_b_sustain_is_on_while_the_pedal_is_up() {
        let Some(s) = offline() else { return };
        s.take_output();
        let hold_b = ControllersCmd::SetPedal { pedal: 0, cc: Some(64), function: Function::Sustain, control_type: crate::controllers::ControlType::HoldB, reverse: false, range: Default::default() };
        s.send(hold_b).unwrap();
        assert!(s.state().controllers.sustain);
        assert!(keyboard_msgs(&s).contains(&[0xB0, 64, 127]));
        s.midi_in(Port::Keys, &[0xB0, 64, 127]);
        assert!(!s.state().controllers.sustain);
        assert!(keyboard_msgs(&s).contains(&[0xB0, 64, 0]));
    }

    #[test]
    fn bend_range_and_learn() {
        let Some(s) = offline() else { return };
        s.take_output();
        s.send(ControllersCmd::SetBendRange { part: 0, semitones: 7 }).unwrap();
        let out = keyboard_msgs(&s);
        assert!(out.windows(3).any(|w| w == [[0xB0, 101, 0], [0xB0, 100, 0], [0xB0, 6, 7]]), "{out:?}");
        assert_eq!(s.state().controllers.parts[0].bend_range, 7);
        s.send(ControllersCmd::LearnPedal { pedal: Some(2) }).unwrap();
        assert_eq!(s.state().controllers.learning, Some(2));
        s.midi_in(Port::Keys, &[0xB0, 85, 127]);
        let st = s.state();
        assert_eq!((st.controllers.learning, st.controllers.pedals[2].cc), (None, Some(85)));
    }
}

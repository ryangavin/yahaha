//! Controllers: pedal setup, the parts the wheels and pedals reach, Pitch Bend Range, and
//! the assignable functions the control side runs (`crate::controllers`).

use super::Control;
use crate::api::{function_run, CmdError, ControllersCmd, ControllersState, FunctionRun};
use crate::controllers::Function;
use crate::fingering::Fingering;
use std::sync::atomic::Ordering::Relaxed;

impl Control {
    pub(super) fn controllers_cmd(&mut self, c: ControllersCmd) -> Result<(), CmdError> {
        match c.apply_setting(&self.shared.controllers) {
            Ok(true) => {}
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
    use crate::controllers::Function;
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
        let e = s.send(ControllersCmd::TriggerFunction { function: Function::RegistNext }).unwrap_err();
        assert!(matches!(e, CmdError::Failed(t) if t.contains("not in yahaha yet")));
        s.send(ControllersCmd::TriggerFunction { function: Function::Sustain }).unwrap();
        assert!(s.state().controllers.sustain);
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

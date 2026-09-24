//! Metronome: on/off and the bell go to the engine (engine/metronome.rs), which times the
//! clicks; the volume goes to the built-in synth's click voice (`crate::click`).

use super::Control;
use crate::api::{CmdError, MetronomeCmd, MetronomeState};
use crate::live::Cmd;
use std::sync::atomic::Ordering::Relaxed;

pub(super) struct MetronomeCtl {
    on: bool,
    volume: u8,
    bell: bool,
}

impl Default for MetronomeCtl {
    fn default() -> MetronomeCtl {
        // The Genos sounds the bell by default.
        MetronomeCtl { on: false, volume: crate::click::DEFAULT_VOLUME, bell: true }
    }
}

impl Control {
    pub(super) fn metronome_cmd(&mut self, c: MetronomeCmd) -> Result<(), CmdError> {
        let m = &mut self.metronome;
        let (on, bell) = (m.on, m.bell);
        match c {
            MetronomeCmd::ToggleMetronome => m.on = !m.on,
            MetronomeCmd::SetMetronome { on } => m.on = on,
            MetronomeCmd::SetMetronomeBell { on } => m.bell = on,
            MetronomeCmd::SetMetronomeVolume { volume } => {
                m.volume = volume.min(127);
                self.pump_metronome();
                return Ok(());
            }
        }
        let (new_on, new_bell) = (m.on, m.bell);
        if (on, bell) != (new_on, new_bell) {
            if let Err(e) = self.engine_cmd(Cmd::Metronome { on: new_on, bell: new_bell }) {
                self.metronome.on = on;
                self.metronome.bell = bell;
                return Err(e);
            }
        }
        Ok(())
    }

    /// The click voice's volume follows the setting (also after the synth restarts).
    pub(super) fn pump_metronome(&mut self) {
        if let Some(s) = &self.synth {
            if s.control.click_volume.load(Relaxed) != self.metronome.volume {
                s.control.click_volume.store(self.metronome.volume, Relaxed);
            }
        }
    }

    pub(super) fn metronome_state(&self) -> MetronomeState {
        let m = &self.metronome;
        MetronomeState { on: m.on, volume: m.volume, bell: m.bell, audible: self.synth.is_some() }
    }
}

//! The live rack's controller map (docs/racks.md, "The model"): what Launchkey faders 1-4
//! (Panel page, Volume layer) and knobs 1-8 (the Rack knob page) do while the rack is
//! loaded.
//!
//! - **Edit.** `setRackControl` changes one controller's target in the live rack; the live
//!   rack's change tracking marks it modified and Save rack keeps it.
//! - **Hand-off.** Whenever the map changes (an edit, a rack loaded) the control side
//!   resolves it once: the Rack page's knob functions into `Knobs` (the knobs already run
//!   here, from the input thread's actions), and each Panel fader's route into
//!   `Parts::rack_fader` atomics, which the input thread reads with no allocation or lock.
//!   A fader on its own part's level stays on the input thread with soft takeover, as
//!   before racks; a fader with any other target comes here as `moveRackFader`.

use super::Control;
use crate::api::{CmdError, RackControl};
use crate::knobs::{fader_command, fader_routes};
use crate::racks::{ControlMap, ControlTarget};

impl Control {
    /// The live rack's controller map is now `m`: the knobs and the input thread follow it.
    pub(super) fn set_rack_controls(&mut self, m: ControlMap) {
        self.knobs.set_rack(&m);
        self.shared.parts.set_rack_faders(fader_routes(&m));
        self.rack_controls = m;
    }

    /// `setRackControl`.
    pub(super) fn set_rack_control(&mut self, control: RackControl, index: u8, target: ControlTarget) -> Result<(), CmdError> {
        let mut m = self.rack_controls.clone();
        if let Err(e) = m.set(control, index, target) {
            return self.fail(e);
        }
        self.set_rack_controls(m);
        Ok(())
    }

    /// `moveRackFader`: fader `fader` at `v` runs its target's command.
    pub(super) fn move_rack_fader(&mut self, fader: u8, v: u8) -> Result<(), CmdError> {
        let Some(t) = self.rack_controls.faders.get(fader as usize) else {
            return self.fail(format!("no fader {} (1-{})", fader as usize + 1, self.rack_controls.faders.len()));
        };
        match fader_command(t, v, self.harmony_arp.on) {
            Some(cmd) => self.apply(cmd),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
#[path = "rack_controls_tests.rs"]
mod tests;

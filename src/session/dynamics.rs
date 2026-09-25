//! Style Dynamics Control, Touch and Accent (#180): the session keeps the settings, hands
//! the engine the whole set on each change (`Cmd::Dynamics`), and tells the input thread
//! whether to send chord-section strikes (`Shared::strikes`). The level in effect is the
//! engine's (Touch moves it): the snapshot's.

use super::Control;
use crate::api::{CmdError, DynamicsCmd, DynamicsState};
use crate::engine::DynamicsSettings;
use crate::live::Cmd;
use std::sync::atomic::Ordering::Relaxed;

impl Control {
    /// The settings in effect: the session's, with the engine's level.
    fn dynamics_now(&self) -> DynamicsSettings {
        DynamicsSettings { level: self.snap.dynamics, ..self.dynamics }
    }

    pub(super) fn dynamics_cmd(&mut self, c: DynamicsCmd) -> Result<(), CmdError> {
        let s = c.apply(self.dynamics_now());
        self.engine_cmd(Cmd::Dynamics(s))?;
        self.dynamics = s;
        // The level shows at once, before the engine's next snapshot.
        self.snap.dynamics = s.level;
        self.shared.strikes.store(s.wants_strikes(), Relaxed);
        Ok(())
    }

    pub(super) fn dynamics_state(&self) -> DynamicsState {
        self.dynamics_now().into()
    }
}

#[cfg(test)]
#[path = "dynamics_tests.rs"]
mod tests;

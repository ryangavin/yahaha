//! Style settings: Section Change Timing, Synchro Stop Window, fade times, Section Reset,
//! Retrigger length. The session keeps them and hands the engine the whole set on each
//! change (`Cmd::StyleSettings`).

use super::Control;
use crate::api::{CmdError, StyleSettingsCmd};
use crate::live::Cmd;

impl Control {
    pub(super) fn style_settings_cmd(&mut self, c: StyleSettingsCmd) -> Result<(), CmdError> {
        let s = c.apply(self.style_settings);
        self.engine_cmd(Cmd::StyleSettings(s))?;
        self.style_settings = s;
        Ok(())
    }
}

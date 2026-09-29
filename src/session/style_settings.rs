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

    /// Swing back to 0 (as written): on every style load.
    pub(super) fn swing_reset(&mut self) {
        if self.style_settings.swing != 0 {
            let _ = self.style_settings_cmd(StyleSettingsCmd::SetSwing { amount: 0 });
        }
    }
}

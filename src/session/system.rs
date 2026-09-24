//! Panic and the message line.

use super::Control;
use crate::api::{CmdError, SystemCmd};
use crate::live::Cmd;

impl Control {
    pub(super) fn system_cmd(&mut self, c: SystemCmd) -> Result<(), CmdError> {
        match c {
            SystemCmd::Panic => return self.engine_cmd(Cmd::Panic),
            SystemCmd::ClearMessage => self.message = None,
        }
        Ok(())
    }
}

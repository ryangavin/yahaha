//! Parameter Lock (Genos RM p.163): a locked group changes only from the panel, never from
//! Registration Memory, One Touch Setting or Playlist recall.
//!
//! Every recall that sets an item of a lock group asks `param_locked` first (the chord
//! registrable in registration/sections.rs: split point, fingering). A One Touch Setting
//! sets no item of any lock group (Data List: the OTS column is X for the split points,
//! the fingering type and the Chord Detection Area), so OTS recall has nothing to check.
//! The lock state is a setup setting, kept with Registration Sequence On/Off in the
//! Registration folder's `setup.json` (never in a bank).

use super::Control;
use crate::api::{CmdError, LockItem, ParamLockCmd, ParamLockState};

impl Control {
    pub(super) fn param_lock_cmd(&mut self, c: ParamLockCmd) -> Result<(), CmdError> {
        match c {
            ParamLockCmd::SetParamLock { item, on } => {
                self.reg.locks.set(item, on);
                match self.reg.save_setup() {
                    Ok(()) => Ok(()),
                    Err(e) => self.fail(format!("saving Parameter Lock: {e:#}")),
                }
            }
        }
    }

    /// The lock for `item`: a recall must leave the group's items as they are.
    pub(super) fn param_locked(&self, item: LockItem) -> bool {
        self.reg.locks.get(item)
    }

    pub(super) fn param_lock_state(&self) -> ParamLockState {
        self.reg.locks
    }
}

#[cfg(test)]
#[path = "param_lock_tests.rs"]
mod tests;

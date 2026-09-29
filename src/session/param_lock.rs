//! Parameter Lock (Genos RM p.163): a locked group changes only from the panel, never from
//! a rack or One Touch Setting recall.
//!
//! Every recall that sets an item of a lock group asks `param_locked` first (a rack's split
//! point, session/racks.rs). A One Touch Setting sets no item of any lock group (Data List:
//! the OTS column is X for the split points, the fingering type and the Chord Detection
//! Area), so OTS recall has nothing to check.
//!
//! The lock state is a setup setting, kept in `<data>/param-locks.json` (never in a rack).
//! Before racks it was kept in the Registration folder's `setup.json`: while
//! `param-locks.json` doesn't exist yet, the locks are read from there (never written).

use super::Control;
use crate::api::{CmdError, LockItem, ParamLockCmd, ParamLockState};
use std::path::{Path, PathBuf};

/// The lock state's file in the data folder.
const FILE: &str = "param-locks.json";
/// Where the locks were kept before racks, under `paramLocks` (read only).
const OLD_FILE: &str = "Registration/setup.json";

/// The Parameter Lock state and where it is saved.
#[derive(Default)]
pub(super) struct ParamLocks {
    /// None: not saved (sessions without a data folder).
    path: Option<PathBuf>,
    state: ParamLockState,
}

impl ParamLocks {
    /// The locks saved in `data_dir` (all unlocked when there are none).
    pub(super) fn load(data_dir: Option<&Path>) -> ParamLocks {
        let Some(dir) = data_dir else { return ParamLocks::default() };
        let read = |p: &Path| std::fs::read_to_string(p).ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok());
        let path = dir.join(FILE);
        let state = if path.exists() {
            read(&path)
        } else {
            read(&dir.join(OLD_FILE)).and_then(|v| v.get("paramLocks").cloned())
        };
        let state = state.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default();
        ParamLocks { path: Some(path), state }
    }

    fn save(&self) -> anyhow::Result<()> {
        let Some(path) = &self.path else { return Ok(()) };
        crate::data_files::write_atomic(path, &serde_json::to_string_pretty(&self.state)?)
    }
}

impl Control {
    pub(super) fn param_lock_cmd(&mut self, c: ParamLockCmd) -> Result<(), CmdError> {
        match c {
            ParamLockCmd::SetParamLock { item, on } => {
                self.locks.state.set(item, on);
                match self.locks.save() {
                    Ok(()) => Ok(()),
                    Err(e) => self.fail(format!("saving Parameter Lock: {e:#}")),
                }
            }
        }
    }

    /// The lock for `item`: a recall must leave the group's items as they are.
    pub(super) fn param_locked(&self, item: LockItem) -> bool {
        self.locks.state.get(item)
    }

    pub(super) fn param_lock_state(&self) -> ParamLockState {
        self.locks.state
    }
}

#[cfg(test)]
#[path = "param_lock_tests.rs"]
mod tests;

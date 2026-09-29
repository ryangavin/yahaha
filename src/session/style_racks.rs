//! Style racks (docs/racks.md, "Styles and OTS"): per style, any of OTS buttons 1-4 can
//! load one of the user's racks instead of the style's own One Touch Setting. The choice is
//! kept by style file name in `<data>/style-racks.json`; the style file isn't touched.
//!
//! - **Recall.** OTS N (the app, Launchkey pad page 3, a pedal, and OTS Link at a Main
//!   change) loads the rack chosen for it, if one is and it still exists; otherwise the
//!   style's own OTS, as before. From the app it goes through the switching guard, as
//!   `loadRack`; the hardware and OTS Link, which have no dialog, keep unsaved changes as a
//!   "Recovered: <name>" rack and switch.
//! - Loading a style never changes the rack by itself; only OTS Link (off by default) does.
//! - Deleting a rack gives every OTS that loaded it back to its style.

use super::Control;
use crate::api::{AppCmd, CmdError, OtsRack, RackCmd};
use crate::live::Cmd;
use crate::racks::style_racks::{self, StyleRacks, SLOTS};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering::Relaxed;

/// The control side's style racks.
#[derive(Default)]
pub(super) struct StyleRacksCtl {
    /// Where they are kept (None: no data folder, so they can't be changed).
    path: Option<PathBuf>,
    racks: StyleRacks,
    /// The file could not be read (a newer yahaha's, or damaged): it is never saved over.
    load_error: Option<String>,
    /// An OTS recall from the app waiting on the unsaved-changes prompt: OTS `.0` (0-3)
    /// counts as recalled once rack `.1` is loaded.
    pending: Option<(u8, String)>,
}

impl StyleRacksCtl {
    pub(super) fn open(data: Option<&Path>) -> StyleRacksCtl {
        let path = data.map(style_racks::path);
        let (racks, load_error) = match path.as_deref().map(StyleRacks::load) {
            Some(Ok(s)) => (s.unwrap_or_default(), None),
            Some(Err(e)) => (StyleRacks::default(), Some(format!("{e:#}"))),
            None => (StyleRacks::default(), None),
        };
        StyleRacksCtl { path, racks, load_error, pending: None }
    }

    pub(super) fn load_error(&self) -> Option<&str> {
        self.load_error.as_deref()
    }

    fn read_only(&self) -> bool {
        self.path.is_none() || self.load_error.is_some()
    }
}

impl Control {
    /// The loaded style's key in `style-racks.json`: its file name.
    fn style_key(&self) -> String {
        self.info.path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
    }

    /// The rack OTS `index` of the loaded style loads, if one is chosen and still exists.
    pub(super) fn style_rack_for(&mut self, index: u8) -> Option<String> {
        let id = self.style_racks.racks.get(&self.style_key(), index as usize)?.to_string();
        self.presence.refresh_racks(false);
        self.presence.racks().iter().any(|r| r.id == id).then_some(id)
    }

    /// `setOtsRack` / `clearOtsRack`.
    pub(super) fn set_ots_rack(&mut self, index: u8, id: Option<String>) -> Result<(), CmdError> {
        let n = self.info.ots.len().min(SLOTS);
        if index as usize >= n {
            return self.fail(format!("{} has no OTS {}", self.info.name, index + 1));
        }
        if let Some(e) = match (&self.style_racks.path, &self.style_racks.load_error) {
            (None, _) => Some("Style racks can't be changed: there is no data folder".to_string()),
            (_, Some(e)) => Some(format!("Style racks can't be changed: {e}")),
            _ => None,
        } {
            return self.fail(e);
        }
        let name = match &id {
            Some(id) => {
                self.presence.refresh_racks(false);
                match self.presence.racks().iter().find(|r| r.id == *id) {
                    Some(r) => Some(r.name.clone()),
                    None => return self.fail(format!("no rack {id}")),
                }
            }
            None => None,
        };
        let key = self.style_key();
        if self.style_racks.racks.get(&key, index as usize) == id.as_deref() {
            return Ok(());
        }
        let mut s = self.style_racks.racks.clone();
        s.set(&key, index as usize, id);
        let path = self.style_racks.path.clone().expect("checked above");
        if let Err(e) = s.save(&path) {
            return self.fail(format!("Style racks were not saved: {e:#}"));
        }
        self.style_racks.racks = s;
        let text = match name {
            Some(n) => format!("With {} loaded, OTS {} now loads {n}", self.info.name, index + 1),
            None => format!("OTS {} is back to {}'s own", index + 1, self.info.name),
        };
        self.say(text, false);
        Ok(())
    }

    /// OTS `index` loads rack `id`: from the app through the switching guard (the recall
    /// counts once the prompt's switch is made), from the hardware or OTS Link keeping
    /// unsaved changes as a Recovered rack. Loaded, it counts as that OTS recalled and turns
    /// Sync Start on, as any OTS recall does.
    pub(super) fn recall_style_rack(&mut self, index: u8, id: String, unattended: bool) -> Result<(), CmdError> {
        self.style_racks.pending = None;
        let r = if unattended {
            self.switch_rack_unattended(Some(&id))
        } else {
            self.apply(AppCmd::Rack(RackCmd::LoadRack { id: id.clone(), discard: false }))
        };
        match r {
            Ok(()) => self.style_rack_recalled(index),
            Err(CmdError::UnsavedChanges) => self.style_racks.pending = Some((index, id)),
            Err(_) => {}
        }
        r
    }

    fn style_rack_recalled(&mut self, index: u8) {
        self.shared.parts.ots_applied.store(index + 1, Relaxed);
        if self.engine_cmd(Cmd::SyncStartOn).is_err() {
            self.wake_engine();
        }
    }

    /// After a rack command: an OTS recall waiting on the prompt counts once its rack is
    /// loaded (and is dropped by any other switch or by Keep editing); deleting a rack gives
    /// every OTS that loaded it back to its style.
    pub(super) fn style_racks_after_rack_cmd(&mut self, c: &RackCmd, ok: bool) {
        if let Some((index, id)) = self.style_racks.pending.clone() {
            let loaded = ok && self.live_rack.id.as_deref() == Some(id.as_str()) && !self.live_rack.modified && self.live_rack.prompt.is_none();
            if loaded {
                self.style_racks.pending = None;
                self.style_rack_recalled(index);
            } else if matches!(c, RackCmd::LoadRack { .. } | RackCmd::NewRack { .. } | RackCmd::DismissRackPrompt) {
                self.style_racks.pending = None;
            }
        }
        if let RackCmd::DeleteRack { id } = c
            && ok
            && let Some(path) = self.style_racks.path.clone()
            && self.style_racks.load_error.is_none()
        {
            let mut s = self.style_racks.racks.clone();
            if s.forget(id) {
                match s.save(&path) {
                    Ok(()) => self.style_racks.racks = s,
                    Err(e) => self.say(format!("Style racks were not saved: {e:#}"), true),
                }
            }
        }
    }

    /// The loaded style's OTS buttons, as `ots.racks`.
    pub(super) fn ots_racks_state(&self) -> Vec<OtsRack> {
        let key = self.style_key();
        let racks = self.presence.racks();
        (0..self.info.ots.len().min(SLOTS))
            .map(|i| match self.style_racks.racks.get(&key, i) {
                Some(id) => {
                    let found = racks.iter().find(|r| r.id == id);
                    OtsRack { rack: Some(id.to_string()), name: found.map(|r| r.name.clone()).unwrap_or_default(), missing: found.is_none() }
                }
                None => OtsRack::default(),
            })
            .collect()
    }

    pub(super) fn style_racks_read_only(&self) -> bool {
        self.style_racks.read_only()
    }
}

#[cfg(test)]
#[path = "style_racks_tests.rs"]
mod tests;

//! Quick Racks (docs/racks.md): the one-press rack buttons on the bar, Launchkey pad page 4
//! and the pedals, banks A-H of eight, kept in `<data>/quick-racks.json`. They replace
//! Registrations.
//!
//! - **Press** loads the button's rack as `loadRack` does, through the switching guard. From
//!   the Launchkey or a pedal (`Control::hardware`), which have no dialog, the switch goes
//!   ahead and unsaved changes are kept as a "Recovered: <name>" rack.
//! - **Store** arms; the next press stores the live rack on that button. A rack with
//!   unsaved changes, or one never saved, is saved first: the button waits
//!   (`storeWaiting`) until `saveRack` / `saveRackAs` succeeds, then takes the saved rack.
//!   From the hardware there is no save flow, so Store there needs a saved rack.
//! - A button names a rack by id, so a rename keeps it; deleting a rack empties its buttons.

use super::{Control, Session};
use crate::api::{CmdError, QuickRackButton, QuickRackCmd, QuickRacksState, RackCmd};
use crate::launchkey::{Action, QuickPanel};
use crate::racks::quick::{self, QuickRacks, BANKS, SLOTS};
use std::path::{Path, PathBuf};

/// The control side's Quick Racks.
#[derive(Default)]
pub(super) struct QuickCtl {
    /// Where they are kept (None: no data folder, so they can't be changed).
    path: Option<PathBuf>,
    racks: QuickRacks,
    /// The bank on view.
    bank: u8,
    /// Store is armed.
    store: bool,
    /// A button (bank, slot) waiting for the live rack to be saved.
    waiting: Option<(u8, u8)>,
    /// The file could not be read (a newer yahaha's, or damaged): it is never saved over.
    load_error: Option<String>,
}

impl QuickCtl {
    pub(super) fn open(data: Option<&Path>) -> QuickCtl {
        let path = data.map(quick::path);
        let (racks, load_error) = match path.as_deref().map(QuickRacks::load) {
            Some(Ok(q)) => (q.unwrap_or_default(), None),
            Some(Err(e)) => (QuickRacks::default(), Some(format!("{e:#}"))),
            None => (QuickRacks::default(), None),
        };
        QuickCtl { path, racks, load_error, ..QuickCtl::default() }
    }

    pub(super) fn load_error(&self) -> Option<&str> {
        self.load_error.as_deref()
    }

    fn read_only(&self) -> bool {
        self.path.is_none() || self.load_error.is_some()
    }

    /// Button `slot` of the bank on view, with 8 and 9 running on into the next bank.
    fn resolve(&self, slot: u8) -> Option<(u8, u8)> {
        let i = self.bank as usize * SLOTS + slot as usize;
        (slot < 10 && i < BANKS * SLOTS).then(|| ((i / SLOTS) as u8, (i % SLOTS) as u8))
    }

    fn get(&self, bank: u8, slot: u8) -> Option<&str> {
        self.racks.get(bank as usize, slot as usize)
    }
}

impl Control {
    pub(super) fn quick_rack_cmd(&mut self, c: QuickRackCmd) -> Result<(), CmdError> {
        match c {
            QuickRackCmd::PressQuickRack { slot, discard } => {
                let Some((bank, slot)) = self.quick.resolve(slot) else {
                    return self.fail(format!("no Quick Rack {}", slot as usize + 1));
                };
                if self.quick.store {
                    return self.store_quick(bank, slot);
                }
                self.load_quick(bank, slot, discard)
            }
            QuickRackCmd::StepQuickRackBank { delta } => {
                self.quick.bank = (self.quick.bank as i16 + delta.signum() as i16).clamp(0, BANKS as i16 - 1) as u8;
                Ok(())
            }
            QuickRackCmd::ToggleQuickRackStore => {
                self.quick.store = !self.quick.store;
                self.quick.waiting = None;
                Ok(())
            }
            // As Store then the button (lane C builds hold Sound + tap on it).
            QuickRackCmd::StoreRack { slot } => {
                if slot as usize >= SLOTS {
                    return self.fail(format!("no Quick Rack {}", slot as usize + 1));
                }
                self.quick.store = true;
                self.quick.waiting = None;
                self.store_quick(self.quick.bank, slot)
            }
            QuickRackCmd::ClearQuickRack { bank, slot } => {
                if bank as usize >= BANKS || slot as usize >= SLOTS {
                    return self.fail(format!("no Quick Rack {bank}:{slot}"));
                }
                if self.quick.get(bank, slot).is_none() {
                    return Ok(());
                }
                self.change_quick(|q| q.banks[bank as usize][slot as usize] = None)
            }
            QuickRackCmd::StepQuickRack { delta, discard } => {
                let bank = self.quick.bank;
                let stored: Vec<u8> = (0..SLOTS as u8).filter(|&s| self.quick.get(bank, s).is_some()).collect();
                let lit = self.lit_slot(bank);
                let to = match (lit.and_then(|l| stored.iter().position(|&s| s == l)), delta.signum()) {
                    (_, 0) => None,
                    (None, d) if d > 0 => stored.first().copied(),
                    (None, _) => stored.last().copied(),
                    (Some(i), d) if d > 0 => stored.get(i + 1).copied(),
                    (Some(i), _) => i.checked_sub(1).map(|i| stored[i]),
                };
                match to {
                    Some(s) => self.load_quick(bank, s, discard),
                    None if stored.is_empty() => self.fail(format!("Bank {} has no racks", quick::bank_letter(bank as usize))),
                    None => Ok(()), // at the end already
                }
            }
        }
    }

    /// The first button of `bank` holding the live rack.
    fn lit_slot(&self, bank: u8) -> Option<u8> {
        let id = self.live_rack.id.as_deref()?;
        (0..SLOTS as u8).find(|&s| self.quick.get(bank, s) == Some(id))
    }

    /// Load button (`bank`, `slot`)'s rack: through the guard from the app, or keeping a
    /// Recovered rack from the hardware.
    fn load_quick(&mut self, bank: u8, slot: u8, discard: bool) -> Result<(), CmdError> {
        let label = quick::label(bank as usize, slot as usize);
        let Some(id) = self.quick.get(bank, slot).map(str::to_string) else {
            return self.fail(format!("Quick Rack {label} is empty"));
        };
        self.presence.refresh_racks(false);
        if !self.presence.racks().iter().any(|r| r.id == id) {
            return self.fail(format!("Quick Rack {label}'s rack is gone"));
        }
        self.quick.waiting = None;
        if self.hardware { self.switch_rack_unattended(Some(&id)) } else { self.rack_cmd(RackCmd::LoadRack { id, discard }) }
    }

    /// Store armed and button (`bank`, `slot`) pressed: the live rack goes on it, once saved.
    fn store_quick(&mut self, bank: u8, slot: u8) -> Result<(), CmdError> {
        if let Some(e) = self.quick_refusal() {
            self.quick.store = false;
            return self.fail(e);
        }
        let saved = self.live_rack.id.clone().filter(|id| !self.live_rack.modified && self.presence.racks().iter().any(|r| r.id == *id));
        match saved {
            Some(id) => self.put_quick(bank, slot, id),
            None if self.hardware => {
                self.quick.store = false;
                self.fail(format!("Save {} first: Store puts a saved rack on the button", self.live_rack.name))
            }
            None => {
                self.quick.waiting = Some((bank, slot));
                self.say(format!("Save the rack first; then it goes on Quick Rack {}", quick::label(bank as usize, slot as usize)), false);
                Ok(())
            }
        }
    }

    fn put_quick(&mut self, bank: u8, slot: u8, id: String) -> Result<(), CmdError> {
        self.quick.store = false;
        self.quick.waiting = None;
        self.change_quick(|q| q.banks[bank as usize][slot as usize] = Some(id))?;
        self.say(format!("Stored {} on Quick Rack {}", self.live_rack.name, quick::label(bank as usize, slot as usize)), false);
        Ok(())
    }

    /// Why Quick Racks can't be changed now, if they can't.
    fn quick_refusal(&self) -> Option<String> {
        match (&self.quick.path, &self.quick.load_error) {
            (None, _) => Some("Quick Racks can't be changed: there is no data folder".into()),
            (_, Some(e)) => Some(format!("Quick Racks can't be changed: {e}")),
            _ => None,
        }
    }

    /// Change the buttons and save them. If the file can't be written, nothing changes.
    fn change_quick(&mut self, f: impl FnOnce(&mut QuickRacks)) -> Result<(), CmdError> {
        if let Some(e) = self.quick_refusal() {
            return self.fail(e);
        }
        let mut q = self.quick.racks.clone();
        f(&mut q);
        let path = self.quick.path.clone().expect("checked above");
        if let Err(e) = q.save(&path) {
            return self.fail(format!("Quick Racks were not saved: {e:#}"));
        }
        self.quick.racks = q;
        Ok(())
    }

    /// After a rack command: a button waiting for the save takes the saved rack; deleting
    /// a rack empties its buttons; loading another rack or dismissing the prompt lets a
    /// waiting Store go.
    pub(super) fn quick_after_rack_cmd(&mut self, c: &RackCmd, ok: bool) {
        match c {
            RackCmd::SaveRack { .. } | RackCmd::SaveRackAs { .. } if ok => {
                if let (Some((bank, slot)), Some(id)) = (self.quick.waiting, self.live_rack.id.clone()) {
                    let _ = self.put_quick(bank, slot, id);
                }
            }
            RackCmd::DeleteRack { id } if ok => {
                if self.quick.racks.banks.iter().flatten().any(|b| b.as_deref() == Some(id.as_str())) {
                    let _ = self.change_quick(|q| {
                        q.forget(id);
                    });
                }
            }
            RackCmd::LoadRack { .. } | RackCmd::NewRack { .. } | RackCmd::RevertRack if ok => self.quick.waiting = None,
            RackCmd::DismissRackPrompt => self.quick.waiting = None,
            _ => {}
        }
    }

    /// A Launchkey pad or button, or a pedal: its command, run as the hardware (no dialog).
    pub(super) fn apply_hardware(&mut self, a: Action) -> Result<(), CmdError> {
        self.hardware = true;
        let r = self.apply(a.into());
        self.hardware = false;
        r
    }

    pub(super) fn quick_racks_state(&self) -> QuickRacksState {
        let q = &self.quick;
        let racks = self.presence.racks();
        let live = self.live_rack.id.as_deref();
        let buttons = (0..SLOTS as u8)
            .map(|s| {
                let id = q.get(q.bank, s);
                let found = id.and_then(|id| racks.iter().find(|r| r.id == id));
                QuickRackButton {
                    rack: id.map(str::to_string),
                    name: found.map(|r| r.name.clone()).unwrap_or_default(),
                    missing: id.is_some() && found.is_none(),
                    loaded: id.is_some() && id == live,
                }
            })
            .collect();
        QuickRacksState { bank: q.bank, buttons, store: q.store, store_waiting: q.waiting.filter(|w| w.0 == q.bank).map(|w| w.1), read_only: q.read_only() }
    }

    /// Page 4 of the Launchkey: the bank on view.
    pub(super) fn quick_panel(&self) -> QuickPanel {
        let q = &self.quick;
        let live = self.live_rack.id.as_deref();
        let mut p = QuickPanel { bank: q.bank, store: q.store, ..QuickPanel::default() };
        for s in 0..SLOTS {
            if let Some(id) = q.get(q.bank, s as u8) {
                p.stored |= 1 << s;
                if Some(id) == live {
                    p.loaded |= 1 << s;
                }
            }
        }
        p
    }
}

impl Session {
    /// Run a Launchkey action (a pad, a button, or a pedal's `Action::Assign`) as the
    /// hardware does: what has a dialog in the app goes ahead without one (a rack switch
    /// keeps unsaved changes as a Recovered rack).
    pub fn hardware(&self, a: Action) -> Result<(), CmdError> {
        let mut ctl = self.inner.lock();
        let r = ctl.apply_hardware(a);
        if ctl.offline.is_some() {
            drop(ctl);
            self.settle();
        } else {
            // Pump and publish now, as `send` does, so `state()` straight after shows the
            // action, follow-ups included. The control thread publishes again only if the
            // engine's snapshot then changes (the engine wakes it itself).
            let now = crate::rt::now_ns();
            ctl.pump(now);
            self.inner.publish(&mut ctl, now);
        }
        r
    }
}

#[cfg(test)]
#[path = "quick_racks_tests.rs"]
mod tests;

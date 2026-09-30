//! Quick Racks in the dev mock (docs/app-api.md › Quick Racks), kept in memory: press
//! (through the rack guard), Store (waiting for the save when the rack is unsaved), bank
//! −/+, clear, previous/next rack, `storeRack`, the Racks pad page. As mock-quick-racks.ts.
//! The mock has no hardware, so every press takes the app's path.

use super::MockSession;
use yahaha::api::*;
use yahaha::launchkey::{self as lk, Page, QuickPanel};

const BANKS: usize = 8;
const SLOTS: usize = 8;

#[derive(Default)]
pub(super) struct MockQuick {
    banks: [[Option<String>; SLOTS]; BANKS],
    bank: u8,
    store: bool,
    waiting: Option<(u8, u8)>,
}

fn label(bank: u8, slot: u8) -> String {
    format!("{}{}", (b'A' + bank) as char, slot + 1)
}

impl MockQuick {
    fn get(&self, bank: u8, slot: u8) -> Option<&str> {
        self.banks[bank as usize][slot as usize].as_deref()
    }

    /// The bank the Racks page shows, for the live rack `live`.
    fn panel(&self, live: Option<&str>) -> QuickPanel {
        let mut p = QuickPanel { bank: self.bank, store: self.store, ..QuickPanel::default() };
        for s in 0..SLOTS as u8 {
            if let Some(id) = self.get(self.bank, s) {
                p.stored |= 1 << s;
                if Some(id) == live {
                    p.loaded |= 1 << s;
                }
            }
        }
        p
    }

    /// `quickRacks` and, on the Racks page, the pads, from the racks list `racks`.
    pub(super) fn fill(&self, st: &mut AppState, racks: &[RackEntry]) {
        let live = st.live_rack.id.clone();
        let buttons = (0..SLOTS as u8)
            .map(|s| {
                let id = self.get(self.bank, s);
                let found = id.and_then(|id| racks.iter().find(|r| r.id == id));
                QuickRackButton {
                    rack: id.map(str::to_string),
                    name: found.map(|r| r.name.clone()).unwrap_or_default(),
                    missing: id.is_some() && found.is_none(),
                    loaded: id.is_some() && id == live.as_deref(),
                }
            })
            .collect();
        st.quick_racks = QuickRacksState {
            bank: self.bank,
            buttons,
            store: self.store,
            store_waiting: self.waiting.filter(|w| w.0 == self.bank).map(|w| w.1),
            read_only: false,
        };
        if st.pads.page == Page::Racks {
            let panel = super::lk_panel(st, self.panel(live.as_deref()));
            st.pads.pads = lk::racks_looks(&panel)
                .iter()
                .map(|(note, look)| Pad {
                    note: *note,
                    label: look.label.into(),
                    key: look.key.into(),
                    rgb: [look.rgb.0, look.rgb.1, look.rgb.2],
                    level: look.level,
                    anim: look.anim,
                    action: lk::pad_action(Page::Racks, Layer::None, *note).map(AppCmd::from),
                    palette: None,
                })
                .collect();
        }
    }

    /// The first button of the bank on view holding rack `live`.
    fn lit(&self, live: Option<&str>) -> Option<u8> {
        let live = live?;
        (0..SLOTS as u8).find(|&s| self.get(self.bank, s) == Some(live))
    }
}

impl MockSession {
    pub(super) fn quick_rack_cmd(&mut self, c: QuickRackCmd) {
        match c {
            QuickRackCmd::PressQuickRack { slot, discard } => {
                let i = self.quick.bank as usize * SLOTS + slot as usize;
                if slot >= 10 || i >= BANKS * SLOTS {
                    return self.message(format!("no Quick Rack {}", slot as usize + 1), true);
                }
                let (bank, slot) = ((i / SLOTS) as u8, (i % SLOTS) as u8);
                if self.quick.store {
                    return self.store_quick(bank, slot);
                }
                self.load_quick(bank, slot, discard);
            }
            QuickRackCmd::StepQuickRackBank { delta } => {
                self.quick.bank = (self.quick.bank as i16 + delta.signum() as i16).clamp(0, BANKS as i16 - 1) as u8;
            }
            QuickRackCmd::ToggleQuickRackStore => {
                self.quick.store = !self.quick.store;
                self.quick.waiting = None;
            }
            // Store, then the button, in one command (hold Sound + tap a Racks pad).
            QuickRackCmd::StoreRack { slot } => {
                if slot as usize >= SLOTS {
                    return self.message(format!("no Quick Rack {}", slot as usize + 1), true);
                }
                self.quick.store = true;
                self.quick.waiting = None;
                self.store_quick(self.quick.bank, slot);
            }
            QuickRackCmd::ClearQuickRack { bank, slot } => {
                if bank as usize >= BANKS || slot as usize >= SLOTS {
                    return self.message(format!("no Quick Rack {bank}:{slot}"), true);
                }
                self.quick.banks[bank as usize][slot as usize] = None;
            }
            QuickRackCmd::StepQuickRack { delta, discard } => {
                let bank = self.quick.bank;
                let stored: Vec<u8> = (0..SLOTS as u8).filter(|&s| self.quick.get(bank, s).is_some()).collect();
                if stored.is_empty() {
                    return self.message(format!("Bank {} has no racks", (b'A' + bank) as char), true);
                }
                let lit = self.quick.lit(self.state.live_rack.id.as_deref());
                let to = match (lit.and_then(|l| stored.iter().position(|&s| s == l)), delta.signum()) {
                    (_, 0) => None,
                    (None, d) if d > 0 => stored.first().copied(),
                    (None, _) => stored.last().copied(),
                    (Some(i), d) if d > 0 => stored.get(i + 1).copied(),
                    (Some(i), _) => i.checked_sub(1).map(|i| stored[i]),
                };
                if let Some(s) = to {
                    self.load_quick(bank, s, discard);
                }
            }
        }
    }

    fn load_quick(&mut self, bank: u8, slot: u8, discard: bool) {
        let Some(id) = self.quick.get(bank, slot).map(str::to_string) else {
            return self.message(format!("Quick Rack {} is empty", label(bank, slot)), true);
        };
        if !self.racks.entries().iter().any(|r| r.id == id) {
            return self.message(format!("Quick Rack {}'s rack is gone", label(bank, slot)), true);
        }
        self.quick.waiting = None;
        self.rack_cmd(RackCmd::LoadRack { id, discard });
    }

    fn store_quick(&mut self, bank: u8, slot: u8) {
        let lr = &self.state.live_rack;
        match lr.id.clone().filter(|id| !lr.modified && self.racks.entries().iter().any(|r| r.id == *id)) {
            Some(id) => self.put_quick(bank, slot, id),
            None => {
                self.quick.waiting = Some((bank, slot));
                self.message(format!("Save the rack first; then it goes on Quick Rack {}", label(bank, slot)), false);
            }
        }
    }

    fn put_quick(&mut self, bank: u8, slot: u8, id: String) {
        self.quick.banks[bank as usize][slot as usize] = Some(id);
        self.quick.store = false;
        self.quick.waiting = None;
        let name = self.state.live_rack.name.clone();
        self.message(format!("Stored {name} on Quick Rack {}", label(bank, slot)), false);
    }

    /// After a rack command, as the session does: a waiting button takes the saved rack;
    /// deleting a rack empties its buttons; a load or dismissing the prompt lets a waiting
    /// Store go.
    pub(super) fn quick_after_rack_cmd(&mut self, c: &RackCmd) {
        match c {
            RackCmd::SaveRack { .. } | RackCmd::SaveRackAs { .. } => {
                let lr = &self.state.live_rack;
                if let (Some((bank, slot)), Some(id), false) = (self.quick.waiting, lr.id.clone(), lr.modified || lr.prompt.is_some()) {
                    self.put_quick(bank, slot, id);
                }
            }
            RackCmd::DeleteRack { id } if !self.racks.entries().iter().any(|r| r.id == *id) => {
                for b in self.quick.banks.iter_mut().flatten() {
                    if b.as_deref() == Some(id.as_str()) {
                        *b = None;
                    }
                }
            }
            RackCmd::LoadRack { .. } | RackCmd::NewRack { .. } | RackCmd::RevertRack | RackCmd::DismissRackPrompt => self.quick.waiting = None,
            _ => {}
        }
    }
}

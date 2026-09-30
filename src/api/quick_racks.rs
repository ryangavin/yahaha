//! Quick Racks (docs/racks.md): the one-press buttons on the bar, Launchkey pad page 4 and
//! the pedals. Banks A-H of eight buttons, each one of the user's racks or empty, kept in
//! `<data>/quick-racks.json`. They replace Registrations.

use serde::{Deserialize, Serialize};

/// The Quick Racks commands. `slot` is a button of the bank on view, 0-7.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum QuickRackCmd {
    /// Press a button. With Store armed, store the live rack on it (a rack with unsaved
    /// changes, or one never saved, waits for the save: `quickRacks.storeWaiting`).
    /// Otherwise load its rack as `loadRack` does, with the same guard and `discard`. Slots
    /// 8 and 9 run on into the next bank's 1 and 2 (the Regist 9-10 pedal functions).
    PressQuickRack {
        slot: u8,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        discard: bool,
    },
    /// Bank -/+: view the previous/next bank (A-H; it stops at either end).
    StepQuickRackBank { delta: i8 },
    /// Store: arm (or disarm) it for the next button press.
    ToggleQuickRackStore,
    /// Store the live rack on button `slot` (0-7) of the bank on view, overwriting what is
    /// there, as Store then the button does (hold Sound + tap a Racks pad).
    StoreRack { slot: u8 },
    /// Empty button `slot` of bank `bank` (0 = A).
    ClearQuickRack { bank: u8, slot: u8 },
    /// Previous/next rack in the bank on view: the stored button before/after the lit one
    /// (from none: + the first, - the last; it stops at either end), loaded as
    /// `pressQuickRack` loads.
    StepQuickRack {
        delta: i8,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        discard: bool,
    },
}

/// Quick Racks, as the bar and pad page 4 show them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickRacksState {
    /// The bank on view, 0-based (0 = A).
    pub bank: u8,
    /// The eight buttons of the bank on view.
    pub buttons: Vec<QuickRackButton>,
    /// Store is armed: the next button press stores the live rack.
    pub store: bool,
    /// A button of the bank on view (0-7) waiting for the live rack to be saved
    /// (`saveRack` / `saveRackAs`) before it is stored there; null when none.
    pub store_waiting: Option<u8>,
    /// Quick Racks can't be changed: the file is from a newer yahaha, or there is no data
    /// folder.
    pub read_only: bool,
}

/// One button.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickRackButton {
    /// The rack's id; null when empty.
    pub rack: Option<String>,
    /// The rack's name; empty when the button is empty or its rack is gone.
    pub name: String,
    /// It names a rack that isn't in `racks` any more.
    pub missing: bool,
    /// Its rack is the live rack's (`liveRack.id`): lit.
    pub loaded: bool,
}

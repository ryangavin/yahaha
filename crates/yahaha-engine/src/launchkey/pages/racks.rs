//! The Racks page (orange): Quick Racks 1-8 of the bank on view on the top row (docs/racks.md),
//! OTS 1-4, bank -/+ and Store on the bottom. Holding Sound shows it from any page
//! (`Layer::Sound`). Rack -/+ left the pads: Shift + Track < / > keep them.

use crate::launchkey::{look_led, page_look, Action, Anim, Led, Level, Look, Page, Panel, BLUE, C_QUICK_LOADED, C_QUICK_STORED, DIM_BLUE, DIM_ORANGE, DIM_RED, ORANGE, QUICK_BANKS, RED};

pub fn pad_action(note: u8) -> Option<Action> {
    Some(match note {
        96..=103 => Action::QuickRack(note - 96),
        112..=115 => Action::Ots(note - 112),
        116 => Action::QuickRackBank(-1),
        117 => Action::QuickRackBank(1),
        118 => Action::QuickRackStore,
        // 119 is dark (spare).
        _ => return None,
    })
}

const QUICK_LABELS: [&str; 8] = ["QUICK 1", "QUICK 2", "QUICK 3", "QUICK 4", "QUICK 5", "QUICK 6", "QUICK 7", "QUICK 8"];
const QUICK_KEYS: [&str; 8] = ["⇧Q", "⇧W", "⇧E", "⇧R", "⇧T", "⇧Y", "⇧U", "⇧I"];

/// The Racks page's pads (the app's dev mock builds its Racks page from this too).
pub fn looks(p: &Panel) -> [(u8, Look); 16] {
    let q = &p.quick;
    let pl = |label, key, available, on| page_look(Page::Racks, label, key, available, on);
    let button = |i: u8| -> Look {
        let (label, key) = (QUICK_LABELS[i as usize], QUICK_KEYS[i as usize]);
        let stored = q.stored & (1 << i) != 0;
        let look = |rgb, level, anim| Look { label, key, rgb, level, anim };
        if q.store {
            // Armed: every button waits to be stored onto.
            look(C_QUICK_LOADED, Level::Bright, Anim::Flash)
        } else if stored && q.loaded & (1 << i) != 0 {
            look(C_QUICK_LOADED, Level::Bright, Anim::Solid)
        } else if stored {
            look(C_QUICK_STORED, Level::Bright, Anim::Solid)
        } else {
            look(C_QUICK_STORED, Level::Off, Anim::Solid)
        }
    };
    let ots = |n: u8, label, key| pl(label, key, n < p.ots_count, p.ots_applied == n + 1);
    let store = if q.store {
        Look { label: "STORE", key: "F5", rgb: C_QUICK_LOADED, level: Level::Bright, anim: Anim::Flash }
    } else {
        pl("STORE", "F5", true, false)
    };
    [
        (96, button(0)),
        (97, button(1)),
        (98, button(2)),
        (99, button(3)),
        (100, button(4)),
        (101, button(5)),
        (102, button(6)),
        (103, button(7)),
        (112, ots(0, "OTS 1", "⇧1")),
        (113, ots(1, "OTS 2", "⇧2")),
        (114, ots(2, "OTS 3", "⇧3")),
        (115, ots(3, "OTS 4", "⇧4")),
        (116, pl("BANK -", "⇧O", q.bank > 0, false)),
        (117, pl("BANK +", "⇧P", q.bank < QUICK_BANKS - 1, false)),
        (118, store),
        // Spare: no label, no action.
        (119, pl("", "", false, false)),
    ]
}

/// The Racks page in palette mode: the Registration lamp colours on the Quick Rack
/// buttons (red = the loaded rack, blue = a rack, off = empty; flashing red while Store is
/// armed), orange on the rest.
pub fn leds(p: &Panel) -> [(u8, Led); 16] {
    looks(p).map(|(note, look)| {
        let (bright, dim) = match look.rgb {
            C_QUICK_LOADED => (RED, DIM_RED),
            C_QUICK_STORED => (BLUE, DIM_BLUE),
            _ => (ORANGE, DIM_ORANGE),
        };
        (note, look_led(&look, bright, dim))
    })
}

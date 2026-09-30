//! The Chord page (cyan): the mid-song chord switches on the bottom row; the top row is dark.
//! The set-and-forget ones (fingering type, Upper) are on the Setup page.

use crate::engine::{Button, Snapshot, Transpose};
use crate::launchkey::{page_leds, page_look, Action, Led, Look, Page, Panel};

pub fn pad_action(note: u8) -> Option<Action> {
    Some(match note {
        112 => Action::ToggleManualBass,
        113 => Action::Button(Button::StopAcmp),
        114 => Action::Split(-1),
        115 => Action::Split(1),
        116 => Action::Transpose { keyboard: -1, master: 0 },
        117 => Action::Transpose { keyboard: 1, master: 0 },
        118 => Action::TransposeReset,
        119 => Action::Button(Button::Retrigger),
        _ => return None,
    })
}

pub fn looks(s: &Snapshot, p: &Panel) -> [(u8, Look); 16] {
    let pl = |label, key, available, on| page_look(Page::Chord, label, key, available, on);
    let dark = pl("", "", false, false);
    let t = s.transpose;
    [
        (96, dark),
        (97, dark),
        (98, dark),
        (99, dark),
        (100, dark),
        (101, dark),
        (102, dark),
        (103, dark),
        // Manual Bass is only available in Upper.
        (112, pl("MAN BASS", "D", p.upper, p.manual_bass)),
        (113, pl("STOP ACMP", "h", true, s.stop_acmp)),
        (114, pl("SPLIT -", "[", true, false)),
        (115, pl("SPLIT +", "]", true, false)),
        (116, pl("KBD TR -", ";", true, t.keyboard < 0)),
        (117, pl("KBD TR +", "'", true, t.keyboard > 0)),
        (118, pl("TR RESET", "/", true, t != Transpose::default())),
        (119, pl("RETRIG", "R", true, s.retrigger)),
    ]
}

pub fn leds(s: &Snapshot, p: &Panel) -> [(u8, Led); 16] {
    page_leds(Page::Chord, looks(s, p))
}

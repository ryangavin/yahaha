//! The Setup page (pink), last by default: the set-and-forget switches, each saved in
//! settings. Fingering type 1-7 and Upper on the top row; OTS Link and the Stop ACMP mode
//! on the bottom.

use crate::engine::{Button, Snapshot, StopAcmp};
use crate::launchkey::{page_leds, page_look, Action, Led, Look, Page, Panel};
use yahaha_core::fingering::Fingering;

pub fn pad_action(note: u8) -> Option<Action> {
    Some(match note {
        96..=102 => Action::Fingering(Fingering::ALL[(note - 96) as usize]),
        103 => Action::ToggleUpper,
        112 => Action::ToggleOtsLink,
        113 => Action::Button(Button::SetStopAcmp(StopAcmp::Style)),
        114 => Action::Button(Button::SetStopAcmp(StopAcmp::Fixed)),
        _ => return None,
    })
}

const FINGERING_LABELS: [&str; 7] = ["SINGLE", "FINGERED", "ON BASS", "MULTI", "AI FING", "FULL KBD", "AI FULL"];

pub fn looks(s: &Snapshot, p: &Panel) -> [(u8, Look); 16] {
    let pl = |label, key, available, on| page_look(Page::Setup, label, key, available, on);
    // No key selects a type directly (`f` steps through them), so the hint says "pad".
    let fing = |i: usize| pl(FINGERING_LABELS[i], "pad", true, p.fingering == Fingering::ALL[i]);
    let dark = pl("", "", false, false);
    [
        (96, fing(0)),
        (97, fing(1)),
        (98, fing(2)),
        (99, fing(3)),
        (100, fing(4)),
        (101, fing(5)),
        (102, fing(6)),
        (103, pl("UPPER", "d", true, p.upper)),
        (112, pl("OTS LINK", "F10", true, p.ots_link)),
        (113, pl("ACMP STYLE", "pad", true, s.stop_acmp_mode == StopAcmp::Style)),
        (114, pl("ACMP FIXED", "pad", true, s.stop_acmp_mode == StopAcmp::Fixed)),
        (115, dark),
        (116, dark),
        (117, dark),
        (118, dark),
        (119, dark),
    ]
}

pub fn leds(s: &Snapshot, p: &Panel) -> [(u8, Led); 16] {
    page_leds(Page::Setup, looks(s, p))
}

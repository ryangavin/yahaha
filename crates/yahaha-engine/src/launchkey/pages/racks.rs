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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::launchkey::{QuickPanel, C_PAGE_RACKS, OFF};

    const PADS: [u8; 16] = [96, 97, 98, 99, 100, 101, 102, 103, 112, 113, 114, 115, 116, 117, 118, 119];

    fn panel(quick: QuickPanel, ots_count: u8, ots_applied: u8) -> Panel {
        Panel { page: Page::Racks, quick, ots_count, ots_applied, ..Panel::default() }
    }

    fn look(p: &Panel, note: u8) -> Look {
        looks(p).into_iter().find(|(n, _)| *n == note).expect("a pad").1
    }

    fn led(p: &Panel, note: u8) -> Led {
        leds(p).into_iter().find(|(n, _)| *n == note).expect("a pad").1
    }

    /// Every note off the 16 pads, the middle notes 104-111 included, does nothing.
    #[test]
    fn notes_off_the_pads_do_nothing() {
        for n in 0..=127u8 {
            if !PADS.contains(&n) {
                assert_eq!(pad_action(n), None, "note {n}");
            }
        }
    }

    /// `looks` and `leds` cover the 16 pads in pad order, and every pad but the spare acts.
    #[test]
    fn looks_and_leds_cover_the_pads_in_order() {
        let p = panel(QuickPanel::default(), 4, 0);
        assert_eq!(looks(&p).map(|(n, _)| n), PADS);
        assert_eq!(leds(&p).map(|(n, _)| n), PADS);
        for n in PADS {
            assert_eq!(pad_action(n).is_some(), n != 119, "note {n}");
        }
    }

    /// Each Quick Rack pad reads its own bit: its label and key, blue when stored, red
    /// when loaded, off when empty; a loaded bit on an empty button stays off.
    #[test]
    fn each_quick_rack_pad_reads_its_own_bit() {
        for i in 0..8u8 {
            let note = 96 + i;
            let bit = 1 << i;
            let stored = panel(QuickPanel { stored: bit, ..QuickPanel::default() }, 0, 0);
            let l = look(&stored, note);
            assert_eq!((l.label, l.key), (QUICK_LABELS[i as usize], QUICK_KEYS[i as usize]));
            assert_eq!((l.rgb, l.level, l.anim), (C_QUICK_STORED, Level::Bright, Anim::Solid), "Quick {i} stored");
            assert_eq!(led(&stored, note), Led::Solid(BLUE));
            for other in (96..=103).filter(|&n| n != note) {
                assert_eq!(look(&stored, other).level, Level::Off, "Quick {i} lights only its own pad, not {other}");
            }

            let loaded = panel(QuickPanel { stored: bit, loaded: bit, ..QuickPanel::default() }, 0, 0);
            let l = look(&loaded, note);
            assert_eq!((l.rgb, l.level, l.anim), (C_QUICK_LOADED, Level::Bright, Anim::Solid), "Quick {i} loaded");
            assert_eq!(led(&loaded, note), Led::Solid(RED));

            let ghost = panel(QuickPanel { loaded: bit, ..QuickPanel::default() }, 0, 0);
            assert_eq!(look(&ghost, note).level, Level::Off, "Quick {i}: loaded but empty stays off");
            assert_eq!(led(&ghost, note), Led::Solid(OFF));
        }
    }

    /// The bits are of the bank on view: the bank number alone doesn't change the top row.
    #[test]
    fn top_row_ignores_the_bank_number() {
        let quick = QuickPanel { stored: 0b1100_0011, loaded: 0b10, bank: 0, store: false };
        let top = |bank| looks(&panel(QuickPanel { bank, ..quick }, 0, 0))[..8].to_vec();
        for bank in 1..QUICK_BANKS {
            assert_eq!(top(bank), top(0), "bank {bank}");
        }
    }

    /// Store armed: all eight flash red, empty and loaded alike, and so does Store; the
    /// bottom row's other pads are unchanged. Unarmed, Store is dim orange.
    #[test]
    fn store_armed_flashes_every_quick_rack() {
        let quick = QuickPanel { stored: 0b0000_0101, loaded: 0b0000_0100, bank: 3, store: false };
        let idle = panel(quick, 2, 1);
        let armed = panel(QuickPanel { store: true, ..quick }, 2, 1);
        for note in 96..=103 {
            let l = look(&armed, note);
            assert_eq!((l.rgb, l.level, l.anim), (C_QUICK_LOADED, Level::Bright, Anim::Flash), "note {note}");
            assert_eq!(led(&armed, note), Led::Flash(DIM_RED, RED), "note {note}");
        }
        let store = look(&armed, 118);
        assert_eq!((store.label, store.key, store.rgb, store.anim), ("STORE", "F5", C_QUICK_LOADED, Anim::Flash));
        for note in [112, 113, 114, 115, 116, 117, 119] {
            assert_eq!(look(&armed, note), look(&idle, note), "note {note}");
        }
        let unarmed = look(&idle, 118);
        assert_eq!((unarmed.rgb, unarmed.level, unarmed.anim), (C_PAGE_RACKS, Level::Dim, Anim::Solid));
        assert_eq!(led(&idle, 118), Led::Solid(DIM_ORANGE));
    }

    /// OTS n is available only below the style's count and on only when it was the one
    /// recalled; a recalled number past the count never lights a dark pad.
    #[test]
    fn ots_pads_follow_count_and_applied() {
        for count in 0..=4u8 {
            for applied in 0..=5u8 {
                let p = panel(QuickPanel::default(), count, applied);
                for n in 0..4u8 {
                    let l = look(&p, 112 + n);
                    let (want, want_led) = match (n < count, applied == n + 1) {
                        (false, _) => (Level::Off, Led::Solid(OFF)),
                        (true, true) => (Level::Bright, Led::Solid(ORANGE)),
                        (true, false) => (Level::Dim, Led::Solid(DIM_ORANGE)),
                    };
                    assert_eq!((l.rgb, l.level, l.anim), (C_PAGE_RACKS, want, Anim::Solid), "count {count} applied {applied} OTS {n}");
                    assert_eq!(led(&p, 112 + n), want_led, "count {count} applied {applied} OTS {n}");
                }
            }
        }
        let p = panel(QuickPanel::default(), 4, 0);
        let labels: Vec<_> = (112..=115).map(|n| look(&p, n)).map(|l| (l.label, l.key)).collect();
        assert_eq!(labels, [("OTS 1", "⇧1"), ("OTS 2", "⇧2"), ("OTS 3", "⇧3"), ("OTS 4", "⇧4")]);
    }

    /// Bank - is dark only on the first bank, Bank + only on the last; both dim between.
    #[test]
    fn bank_buttons_stop_at_the_ends() {
        for bank in 0..QUICK_BANKS {
            let p = panel(QuickPanel { bank, ..QuickPanel::default() }, 0, 0);
            let (minus, plus) = (look(&p, 116), look(&p, 117));
            assert_eq!(minus.level, if bank == 0 { Level::Off } else { Level::Dim }, "bank {bank}");
            assert_eq!(plus.level, if bank == QUICK_BANKS - 1 { Level::Off } else { Level::Dim }, "bank {bank}");
            assert_eq!((minus.label, minus.key, plus.label, plus.key), ("BANK -", "⇧O", "BANK +", "⇧P"));
        }
    }

    /// The spare stays dark and unlabelled whatever the panel holds.
    #[test]
    fn spare_is_always_dark() {
        let full = QuickPanel { stored: 0xff, loaded: 0x01, bank: 4, store: true };
        for p in [panel(QuickPanel::default(), 0, 0), panel(full, 4, 4)] {
            let l = look(&p, 119);
            assert_eq!((l.label, l.key, l.level), ("", "", Level::Off));
            assert_eq!(led(&p, 119), Led::Solid(OFF));
        }
    }

    /// The palette follows each look's colour: loaded red, stored blue, everything else
    /// (only ever the page's orange) orange, each at the look's level and animation.
    #[test]
    fn leds_map_colour_by_look() {
        let quick = QuickPanel { stored: 0b0110_1011, loaded: 0b0000_1000, bank: 5, store: false };
        for store in [false, true] {
            let p = panel(QuickPanel { store, ..quick }, 3, 2);
            for ((note, l), (led_note, got)) in looks(&p).into_iter().zip(leds(&p)) {
                assert_eq!(note, led_note);
                let (bright, dim) = match l.rgb {
                    C_QUICK_LOADED => (RED, DIM_RED),
                    C_QUICK_STORED => (BLUE, DIM_BLUE),
                    rgb => {
                        assert_eq!(rgb, C_PAGE_RACKS, "note {note}: only the Quick Rack lamps leave the page colour");
                        (ORANGE, DIM_ORANGE)
                    }
                };
                let want = match (l.level, l.anim) {
                    (Level::Off, _) => Led::Solid(OFF),
                    (Level::Dim, _) => Led::Solid(dim),
                    (Level::Bright, Anim::Solid) => Led::Solid(bright),
                    (Level::Bright, Anim::Flash) => Led::Flash(dim, bright),
                    (Level::Bright, Anim::Pulse) => Led::Pulse(bright),
                };
                assert_eq!(got, want, "store {store} note {note}");
            }
        }
    }
}

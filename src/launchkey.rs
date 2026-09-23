//! Novation Launchkey MK4 in DAW mode: pads and buttons as arranger controls, with LEDs.
//!
//! Pad layout (DAW port, channel 1 notes):
//!   top    96..103: Intro I  Intro II  Intro III  SyncStart | Ending I  Ending II  Ending III  AutoFill
//!   bottom 112..119: Main A   Main B    Main C     Main D    | Break     Tap       SyncStop    Start/Stop
//! Buttons (CC): 115 Play = Start/Stop, 116 Stop, 104/105 (right arrows) = tempo +/-.

use crate::engine::{slot_of, Button, Snapshot};
use crate::sff::SectionId;

pub const ENTER_DAW: [u8; 3] = [0x9F, 0x0C, 0x7F];
pub const EXIT_DAW: [u8; 3] = [0x9F, 0x0C, 0x00];

pub fn pad_button(note: u8) -> Option<Button> {
    Some(match note {
        96 => Button::Intro(0),
        97 => Button::Intro(1),
        98 => Button::Intro(2),
        99 => Button::SyncStart,
        100 => Button::Ending(0),
        101 => Button::Ending(1),
        102 => Button::Ending(2),
        103 => Button::AutoFill,
        112 => Button::Main(0),
        113 => Button::Main(1),
        114 => Button::Main(2),
        115 => Button::Main(3),
        116 => Button::Break,
        117 => Button::TapTempo,
        118 => Button::SyncStop,
        119 => Button::StartStop,
        _ => return None,
    })
}

pub fn cc_button(cc: u8) -> Option<Button> {
    Some(match cc {
        115 => Button::StartStop,
        116 => Button::Stop,
        104 => Button::TempoUp,
        105 => Button::TempoDown,
        _ => return None,
    })
}

// Novation palette indices.
const OFF: u8 = 0;
const WHITE: u8 = 3;
const DIM_WHITE: u8 = 1;
const RED: u8 = 5;
const DIM_RED: u8 = 7;
const ORANGE: u8 = 9;
const YELLOW: u8 = 13;
const DIM_YELLOW: u8 = 15;
const GREEN: u8 = 21;
const DIM_GREEN: u8 = 23;
const CYAN: u8 = 37;
const BLUE: u8 = 45;
const PURPLE: u8 = 53;
const DIM_PURPLE: u8 = 55;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Led {
    Solid(u8),
    Flash(u8, u8),
    Pulse(u8),
}

/// Desired LED state for all 16 pads, given engine state and which sections exist.
pub fn pad_leds(s: &Snapshot, has: &[bool]) -> [(u8, Led); 16] {
    let cur = s.cur;
    let queued = s.queued;
    let sec = |id: SectionId, on: u8, dim: u8| -> Led {
        if !has[slot_of(id)] {
            return Led::Solid(OFF);
        }
        if queued == Some(id) {
            Led::Flash(dim, on)
        } else if cur == Some(id) {
            Led::Solid(on)
        } else {
            Led::Solid(dim)
        }
    };
    let main = |i: u8| -> Led {
        let id = SectionId::Main(i);
        if !has[slot_of(id)] {
            return Led::Solid(OFF);
        }
        let fill = SectionId::Fill(i);
        if queued == Some(id) || queued == Some(fill) || cur == Some(fill) {
            Led::Flash(DIM_GREEN, GREEN)
        } else if cur == Some(id) || (s.main == i && !matches!(cur, Some(SectionId::Main(_)))) {
            Led::Solid(GREEN)
        } else {
            Led::Solid(DIM_GREEN)
        }
    };
    let intro = |i: u8| -> Led {
        if s.pending_intro == Some(i) && has[slot_of(SectionId::Intro(i))] {
            Led::Pulse(YELLOW)
        } else {
            sec(SectionId::Intro(i), YELLOW, DIM_YELLOW)
        }
    };
    [
        (96, intro(0)),
        (97, intro(1)),
        (98, intro(2)),
        (99, if s.sync_armed { Led::Pulse(ORANGE) } else { Led::Solid(OFF) }),
        (100, sec(SectionId::Ending(0), RED, DIM_RED)),
        (101, sec(SectionId::Ending(1), RED, DIM_RED)),
        (102, sec(SectionId::Ending(2), RED, DIM_RED)),
        (103, Led::Solid(if s.auto_fill { BLUE } else { OFF })),
        (112, main(0)),
        (113, main(1)),
        (114, main(2)),
        (115, main(3)),
        (116, sec(SectionId::Break, PURPLE, DIM_PURPLE)),
        (117, if s.running && s.beat == 0 { Led::Solid(WHITE) } else { Led::Solid(DIM_WHITE) }),
        (118, Led::Solid(if s.sync_stop { CYAN } else { OFF })),
        (119, if s.running { Led::Solid(GREEN) } else { Led::Solid(DIM_RED) }),
    ]
}

/// MIDI messages that set one pad's LED.
pub fn led_msgs(note: u8, led: Led, out: &mut Vec<[u8; 3]>) {
    match led {
        Led::Solid(c) => out.push([0x90, note, c]),
        Led::Flash(a, b) => {
            out.push([0x90, note, a]);
            out.push([0x91, note, b]);
        }
        Led::Pulse(c) => out.push([0x92, note, c]),
    }
}

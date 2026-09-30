//! Page 1, Sections: the original layout, unchanged (fixed first in every page order).

use crate::engine::{slot_of, Button, Snapshot};
use crate::launchkey::{
    landing, Action, Anim, Led, Level, Look, BLUE, C_BREAK, C_ENDING, C_FILL, C_IDLE, C_INTRO, C_MAIN, C_RUN, C_STOPSYNC, C_SYNC, C_TAP, CYAN,
    DIM_GREEN, DIM_PURPLE, DIM_RED, DIM_WHITE, DIM_YELLOW, GREEN, OFF, ORANGE, PURPLE, RED, WHITE, YELLOW,
};
use yahaha_sff::sff::SectionId;

/// Page 1 (Sections) pad buttons: the original layout, unchanged.
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

pub fn pad_action(note: u8) -> Option<Action> {
    pad_button(note).map(Action::Button)
}

/// Page 1 in palette mode, given engine state and which sections exist.
pub fn leds(s: &Snapshot, has: &[bool]) -> [(u8, Led); 16] {
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
        } else if landing(s) == Some(i) {
            // Where the fill lands, when that's another Main (#282).
            Led::Pulse(GREEN)
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

pub fn looks(s: &Snapshot, has: &[bool]) -> [(u8, Look); 16] {
    let l = |label, key, rgb, level, anim| Look { label, key, rgb, level, anim };
    let sec = |id: SectionId, label, key, rgb| -> Look {
        if !has[slot_of(id)] {
            l(label, key, rgb, Level::Off, Anim::Solid)
        } else if s.queued == Some(id) {
            l(label, key, rgb, Level::Bright, Anim::Flash)
        } else if s.cur == Some(id) {
            l(label, key, rgb, Level::Bright, Anim::Solid)
        } else {
            l(label, key, rgb, Level::Dim, Anim::Solid)
        }
    };
    let main = |i: u8, label, key| -> Look {
        let id = SectionId::Main(i);
        let fill = SectionId::Fill(i);
        if !has[slot_of(id)] {
            l(label, key, C_MAIN, Level::Off, Anim::Solid)
        } else if s.queued == Some(id) || s.queued == Some(fill) || s.cur == Some(fill) {
            l(label, key, C_MAIN, Level::Bright, Anim::Flash)
        } else if landing(s) == Some(i) {
            l(label, key, C_MAIN, Level::Bright, Anim::Pulse)
        } else if s.cur == Some(id) || (s.main == i && !matches!(s.cur, Some(SectionId::Main(_)))) {
            l(label, key, C_MAIN, Level::Bright, Anim::Solid)
        } else {
            l(label, key, C_MAIN, Level::Dim, Anim::Solid)
        }
    };
    let intro = |i: u8, label, key| -> Look {
        if s.pending_intro == Some(i) && has[slot_of(SectionId::Intro(i))] {
            l(label, key, C_INTRO, Level::Bright, Anim::Pulse)
        } else {
            sec(SectionId::Intro(i), label, key, C_INTRO)
        }
    };
    let toggle = |on: bool, label, key, rgb| l(label, key, rgb, if on { Level::Bright } else { Level::Dim }, Anim::Solid);
    [
        (96, intro(0, "INTRO 1", "q")),
        (97, intro(1, "INTRO 2", "w")),
        (98, intro(2, "INTRO 3", "e")),
        (99, if s.sync_armed { l("SYNC ST", "y", C_SYNC, Level::Bright, Anim::Pulse) } else { toggle(false, "SYNC ST", "y", C_SYNC) }),
        (100, sec(SectionId::Ending(0), "ENDING 1", "i", C_ENDING)),
        (101, sec(SectionId::Ending(1), "ENDING 2", "o", C_ENDING)),
        (102, sec(SectionId::Ending(2), "ENDING 3", "p", C_ENDING)),
        (103, toggle(s.auto_fill, "AUTOFILL", "u", C_FILL)),
        (112, main(0, "MAIN A", "1")),
        (113, main(1, "MAIN B", "2")),
        (114, main(2, "MAIN C", "3")),
        (115, main(3, "MAIN D", "4")),
        (116, sec(SectionId::Break, "BREAK", "g", C_BREAK)),
        (117, toggle(s.running && s.beat == 0, "TAP", "t", C_TAP)),
        (118, toggle(s.sync_stop, "SYNC STP", "j", C_STOPSYNC)),
        (119, if s.running { toggle(true, "START", "spc", C_RUN) } else { toggle(true, "STOP", "spc", C_IDLE) }),
    ]
}

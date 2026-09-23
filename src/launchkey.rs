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

/// Faders (DAW mode, Volume): CC 5..=12 are faders 1-8, CC 13 is the master fader.
pub const FADER_CC: std::ops::RangeInclusive<u8> = 5..=13;
/// Buttons under the faders: CC 37..=44 under faders 1-8, CC 45 under the master fader.
pub const FADER_BTN_CC: std::ops::RangeInclusive<u8> = 37..=45;

/// Side buttons left of the pads: top = Left voice on/off, bottom = OTS Link.
pub const LEFT_BTN_CC: u8 = 106;
pub const OTS_LINK_BTN_CC: u8 = 107;

/// Side button lights (palette colour on ch 1, plus brightness on ch 4 in case they're
/// single-colour LEDs).
pub fn side_button_msgs(left_on: bool, ots_link: bool, out: &mut Vec<[u8; 3]>) {
    out.push([0xB0, LEFT_BTN_CC, if left_on { 21 } else { 23 }]);
    out.push([0xB3, LEFT_BTN_CC, if left_on { 127 } else { 16 }]);
    out.push([0xB0, OTS_LINK_BTN_CC, if ots_link { 13 } else { 15 }]);
    out.push([0xB3, OTS_LINK_BTN_CC, if ots_link { 127 } else { 16 }]);
}

/// Palette colours for the fader buttons: voice slots and layer mode.
pub fn fader_button_msgs(active: u8, layer_mode: bool, out: &mut Vec<[u8; 3]>) {
    for i in 0..8u8 {
        let c = if active & (1 << i) != 0 { 45 } else { 47 }; // blue / dim blue
        out.push([0xB0, 37 + i, c]);
    }
    out.push([0xB0, 45, if layer_mode { 9 } else { 11 }]); // orange / dim orange
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

// ---------------------------------------------------------------------------
// RGB look model: one description drives both the hardware pads and the on-screen map.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
    Off,
    Dim,
    Bright,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Anim {
    Solid,
    /// Alternates dim/bright every half beat: queued, waiting for the bar/beat.
    Flash,
    /// Breathes over two beats: armed, waiting for you.
    Pulse,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Look {
    pub label: &'static str,
    pub key: &'static str,
    /// Full-brightness colour, 0..=127 per channel.
    pub rgb: (u8, u8, u8),
    pub level: Level,
    pub anim: Anim,
}

pub const C_INTRO: (u8, u8, u8) = (127, 95, 0);
pub const C_MAIN: (u8, u8, u8) = (0, 127, 16);
pub const C_ENDING: (u8, u8, u8) = (127, 0, 0);
pub const C_BREAK: (u8, u8, u8) = (90, 0, 127);
pub const C_SYNC: (u8, u8, u8) = (127, 45, 0);
pub const C_FILL: (u8, u8, u8) = (0, 45, 127);
pub const C_TAP: (u8, u8, u8) = (100, 100, 100);
pub const C_STOPSYNC: (u8, u8, u8) = (0, 110, 110);
pub const C_RUN: (u8, u8, u8) = (0, 127, 0);
pub const C_IDLE: (u8, u8, u8) = (127, 0, 0);

/// Brightness of "dim" relative to full.
const DIM: f32 = 0.18;

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

/// Colour at a point in time. `beats` is a free-running beat clock (fractional).
pub fn rgb_at(look: &Look, beats: f64) -> (u8, u8, u8) {
    let k = match (look.level, look.anim) {
        (Level::Off, _) => 0.0,
        (Level::Dim, _) => DIM,
        (Level::Bright, Anim::Solid) => 1.0,
        (Level::Bright, Anim::Flash) => {
            if beats.fract() < 0.5 {
                1.0
            } else {
                DIM
            }
        }
        (Level::Bright, Anim::Pulse) => {
            // Two-beat triangle between 25% and 100%.
            let p = (beats / 2.0).fract() as f32;
            let tri = if p < 0.5 { p * 2.0 } else { 2.0 - p * 2.0 };
            0.25 + 0.75 * tri
        }
    };
    let f = |c: u8| ((c as f32 * k).round() as u8).min(127);
    (f(look.rgb.0), f(look.rgb.1), f(look.rgb.2))
}

/// SysEx that sets a pad to an RGB colour (0..=127 per channel). Regular (non-Mini) SKU.
pub fn rgb_sysex(pad: u8, (r, g, b): (u8, u8, u8)) -> [u8; 13] {
    [0xF0, 0x00, 0x20, 0x29, 0x02, 0x14, 0x01, 0x43, pad, r, g, b, 0xF7]
}

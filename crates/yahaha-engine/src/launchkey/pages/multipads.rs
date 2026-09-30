//! The Multi Pads page (yellow): Multi Pads 1-4, STOP, SELECT + pad (Synchro Start) and
//! STOP + pad (#196).

use crate::engine::{PadCmd, Snapshot};
use crate::launchkey::{
    look_led, page_look, Action, Anim, Led, Level, Look, Page, BLUE, C_PAD_PLAYING, C_PAD_QUEUED, C_PAD_READY, DIM_BLUE, DIM_ORANGE, DIM_RED, DIM_YELLOW,
    ORANGE, RED, YELLOW,
};
use crate::multipad::PadState;

pub fn pad_action(note: u8) -> Option<Action> {
    Some(match note {
        96..=99 => Action::MultiPad(PadCmd::Trigger(note - 96)),
        100 => Action::MultiPad(PadCmd::StopAll),
        112..=115 => Action::MultiPad(PadCmd::Arm(note - 112)),
        116..=119 => Action::MultiPad(PadCmd::Stop(note - 116)),
        _ => return None,
    })
}

const MULTIPAD_LABELS: [&str; 4] = ["PAD 1", "PAD 2", "PAD 3", "PAD 4"];
const MULTIPAD_KEYS: [&str; 4] = ["Z", "X", "C", "V"];
const SELECT_PAD_LABELS: [&str; 4] = ["SELECT 1", "SELECT 2", "SELECT 3", "SELECT 4"];
const STOP_PAD_LABELS: [&str; 4] = ["STOP 1", "STOP 2", "STOP 3", "STOP 4"];

pub fn looks(s: &Snapshot) -> [(u8, Look); 16] {
    let pl = |label, key, available, on| page_look(Page::MultiPads, label, key, available, on);
    let st = s.multipad.states;
    let has = |i: usize| st[i] != PadState::Empty;
    let sounding = |i: usize| matches!(st[i], PadState::Playing | PadState::Queued);
    let lamp = |i: usize| -> Look {
        let look = |rgb, level, anim| Look { label: MULTIPAD_LABELS[i], key: MULTIPAD_KEYS[i], rgb, level, anim };
        match st[i] {
            PadState::Empty => look(C_PAD_READY, Level::Off, Anim::Solid),
            PadState::Ready => look(C_PAD_READY, Level::Bright, Anim::Solid),
            PadState::Armed => look(C_PAD_PLAYING, Level::Bright, Anim::Flash),
            PadState::Queued => look(C_PAD_QUEUED, Level::Bright, Anim::Flash),
            PadState::Playing => look(C_PAD_PLAYING, Level::Bright, Anim::Solid),
        }
    };
    // SELECT + pad arms it: lit while it waits in standby.
    let select = |i: usize| -> Look {
        let armed = st[i] == PadState::Armed;
        Look { anim: if armed { Anim::Flash } else { Anim::Solid }, ..pl(SELECT_PAD_LABELS[i], "pad", has(i), armed) }
    };
    let stop = |i: usize| pl(STOP_PAD_LABELS[i], "pad", has(i), sounding(i));
    let busy = (0..4).any(|i| sounding(i) || st[i] == PadState::Armed);
    let none = |_| pl("", "", false, false);
    [
        (96, lamp(0)),
        (97, lamp(1)),
        (98, lamp(2)),
        (99, lamp(3)),
        (100, pl("STOP", "B", (0..4).any(has), busy)),
        (101, none(())),
        (102, none(())),
        (103, none(())),
        (112, select(0)),
        (113, select(1)),
        (114, select(2)),
        (115, select(3)),
        (116, stop(0)),
        (117, stop(1)),
        (118, stop(2)),
        (119, stop(3)),
    ]
}

/// The Multi Pads page in palette mode: the Genos Multi Pad lamps on pads 1-4 (blue =
/// data, red = playing, flashing red = Synchro Start standby, flashing orange = waiting for
/// the bar line, off = empty), yellow on the rest.
pub fn leds(s: &Snapshot) -> [(u8, Led); 16] {
    looks(s).map(|(note, look)| {
        let (bright, dim) = match look.rgb {
            C_PAD_READY => (BLUE, DIM_BLUE),
            C_PAD_PLAYING => (RED, DIM_RED),
            C_PAD_QUEUED => (ORANGE, DIM_ORANGE),
            _ => (YELLOW, DIM_YELLOW),
        };
        (note, look_led(&look, bright, dim))
    })
}

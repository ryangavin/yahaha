//! Novation Launchkey MK4 in DAW mode: pads and buttons as arranger controls, with LEDs.
//!
//! The 16 pads have five pages, switched with the Pad Bank ▲/▼ buttons left of the pads
//! (DAW port, channel 1 notes; top row 96..103, bottom row 112..119):
//!
//!   1 Sections     Intro I  Intro II  Intro III  SyncStart | Ending I  Ending II  Ending III  AutoFill
//!                  Main A   Main B    Main C     Main D    | Break     Tap       SyncStop    Start/Stop
//!   2 Chord/Setup  Single   Fingered  On Bass    Multi     | AI Fing.  Full Kbd  AI Full     Upper
//!                  ManBass  StopAcmp  Split -    Split +   | Kbd tr -  Kbd tr +  Tr reset    Retrigger
//!   3 OTS/Parts    OTS 1    OTS 2     OTS 3      OTS 4     | OTS Link  Fade      Voice -/+
//!                  Right 1  Right 2   Right 3    Left      | Select R1 Select R2 Select R3   Select Left
//!   4 Registration Regist 1 Regist 2  Regist 3   Regist 4  | Regist 5  Regist 6  Regist 7    Regist 8
//!                  Regist 9 Regist 10 Bank -     Bank +    | Memory    Freeze    Regist -    Regist +
//!   5 Multi Pads   Pad 1    Pad 2     Pad 3      Pad 4     | STOP      -         -           -
//!                  Select 1 Select 2  Select 3   Select 4  | Stop 1    Stop 2    Stop 3      Stop 4
//!
//! Buttons (CC in DAW mode; numbers from the MK4 Programmer's Reference Guide v3.0, p.9,
//! Figure 3): 115 Play = Start/Stop, 116 Stop, 104 (Scene Launch >) / 105 (Function) =
//! tempo +/-, 106/107 (Pad Bank ▲/▼) = page up/down, Shift + ▲/▼ = Left on/off / OTS Link,
//! 103/102 (< Track / Track >) = previous/next style (Shift: previous/next Playlist record),
//! 63 = Shift. Shift + Play = Style Section Reset, Shift + Stop = Fade In/Out, Shift +
//! Scene Launch / Function = Retrigger length shorter / longer.
//!
//! The 8 encoders are knobs on Knob Assign pages (`knobs.rs`), stepped with the encoder
//! page buttons ▲/▼ (CC 51/52) right of them, like the Genos KNOB ASSIGN button. yahaha
//! turns the encoders' relative output on when it enters DAW mode.
//!
//! Faders have two pages, like the Genos Mixer's Panel and Style tabs; the button under
//! the master fader switches them (see `parts`). Panel: faders 1-4 = Right 1, Right 2,
//! Right 3, Left volumes, their buttons = part on/off (Shift: select the part), button 5
//! HARMONY/ARPEGGIO, 6 plugin reload, 7 LEFT HOLD, 8 CHORD LOOPER ON/OFF (Shift: REC/STOP).
//! Style:
//! faders 1-8 = the Style parts, their buttons = part mute. Master is always master.

use crate::engine::{slot_of, Button, FadeState, PadCmd, Snapshot, Transpose};
use crate::fingering::Fingering;
use crate::multipad::PadState;
use crate::parts::{self, FaderPage};
use crate::sff::SectionId;

pub const ENTER_DAW: [u8; 3] = [0x9F, 0x0C, 0x7F];
pub const EXIT_DAW: [u8; 3] = [0x9F, 0x0C, 0x00];
/// Feature control 45h, DAW Encoder Relative output on / off (Programmer's Reference
/// Guide v3.0, p.22): the encoders send steps, not positions.
pub const ENCODERS_RELATIVE: [u8; 3] = [0xB6, 0x45, 0x7F];
pub const ENCODERS_ABSOLUTE: [u8; 3] = [0xB6, 0x45, 0x00];

/// The encoders send on channel 16 (pp.12-13): CC 21-28 in the Plugin, Mixer and Sends
/// modes (relative once `ENCODERS_RELATIVE` is sent), CC 85-92 in the Transport mode
/// (always relative). Relative: 64 = no move, 65 = one step clockwise, 63 = one step back.
pub const ENCODER_STATUS: u8 = 0xBF;
pub const ENCODER_CC: std::ops::RangeInclusive<u8> = 21..=28;
pub const ENCODER_TRANSPORT_CC: std::ops::RangeInclusive<u8> = 85..=92;

/// An encoder message: (knob 0-7, steps; positive = clockwise). Touch events (channel 15)
/// and the other channels are not encoder turns.
pub fn encoder(status: u8, cc: u8, value: u8) -> Option<(u8, i8)> {
    if status != ENCODER_STATUS {
        return None;
    }
    let knob = if ENCODER_CC.contains(&cc) {
        cc - ENCODER_CC.start()
    } else if ENCODER_TRANSPORT_CC.contains(&cc) {
        cc - ENCODER_TRANSPORT_CC.start()
    } else {
        return None;
    };
    let delta = (value & 0x7F) as i8 - 64;
    (delta != 0).then_some((knob, delta))
}

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

/// A pad note in DAW mode (either row).
pub fn is_pad(note: u8) -> bool {
    (96..=103).contains(&note) || (112..=119).contains(&note)
}

/// Faders (DAW mode, Volume): CC 5..=12 are faders 1-8, CC 13 is the master fader.
pub const FADER_CC: std::ops::RangeInclusive<u8> = 5..=13;
pub const MASTER_FADER_CC: u8 = 13;
/// Buttons under the faders: CC 37..=44 under faders 1-8, CC 45 under the master fader.
pub const FADER_BTN_CC: std::ops::RangeInclusive<u8> = 37..=45;

/// Button CCs in DAW mode (Programmer's Reference Guide v3.0, p.9, Figure 3).
pub const SHIFT_CC: u8 = 63;
pub const TRACK_LEFT_CC: u8 = 103;
pub const TRACK_RIGHT_CC: u8 = 102;
/// The encoder page buttons ▲ / ▼, right of the encoders: the Knob Assign page.
pub const KNOB_UP_CC: u8 = 51;
pub const KNOB_DOWN_CC: u8 = 52;
/// Pad Bank ▲ / ▼, left of the top / bottom pad row.
pub const PAD_UP_CC: u8 = 106;
pub const PAD_DOWN_CC: u8 = 107;
/// Scene Launch > and Function, right of the top / bottom pad row.
pub const SCENE_CC: u8 = 104;
pub const FUNCTION_CC: u8 = 105;
pub const PLAY_CC: u8 = 115;
pub const STOP_CC: u8 = 116;
/// Channel 7 CCs are mode reports and feature-control replies (Programmer's Reference
/// Guide v3.0, pp.10 and 21), not button presses.
pub const FEATURE_CH_STATUS: u8 = 0xB6;
/// Pad mode report on channel 7 (p.10); 2 = DAW layout.
pub const PAD_MODE_CC: u8 = 29;

/// A Launchkey control the player just touched or moved, as the input thread records it
/// for the display (`live::Shared::touched`, packed with a sequence number so a second
/// press of the same control counts). The control side works out what it did from the
/// state: the pads, the surface (buttons, fader buttons, faders) and the knobs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Touch {
    /// A pad (its note) on the current page.
    Pad(u8),
    /// A button (its CC), with Shift held or not.
    Button { cc: u8, shift: bool },
    /// Fader 1-8 (0-7) or the master fader (8).
    Fader(u8),
    /// The button under fader 1-8 (0-7) or under the master fader (8).
    FaderButton { index: u8, shift: bool },
    /// Encoder 1-8 (0-7).
    Knob(u8),
}

impl Touch {
    /// Packed with `seq` for an atomic: seq in bits 16-31, the kind in 8-11, Shift in bit
    /// 7, the index in 0-6. Never 0 (0 = nothing touched yet).
    pub fn pack(self, seq: u16) -> u32 {
        let (kind, shift, i) = match self {
            Touch::Pad(n) => (1, false, n),
            Touch::Button { cc, shift } => (2, shift, cc),
            Touch::Fader(i) => (3, false, i),
            Touch::FaderButton { index, shift } => (4, shift, index),
            Touch::Knob(i) => (5, false, i),
        };
        (seq as u32) << 16 | kind << 8 | (shift as u32) << 7 | (i & 0x7F) as u32
    }

    pub fn unpack(v: u32) -> Option<Touch> {
        let (i, shift) = ((v & 0x7F) as u8, v & 0x80 != 0);
        Some(match (v >> 8) & 0xF {
            1 => Touch::Pad(i),
            2 => Touch::Button { cc: i, shift },
            3 => Touch::Fader(i),
            4 => Touch::FaderButton { index: i, shift },
            5 => Touch::Knob(i),
            _ => return None,
        })
    }
}

/// The display target yahaha writes to: the Global temporary display (Programmer's
/// Reference Guide v3.0, p.17), which goes back to the normal screen by itself after the
/// Launchkey's display timeout.
pub const DISPLAY_TARGET: u8 = 0x21;
/// Arrangement 2: three lines, Title, Name and Value (p.18).
const DISPLAY_TITLE_NAME_VALUE: u8 = 2;
/// Characters per line yahaha sends (the display is 128 pixels wide).
pub const DISPLAY_CHARS: usize = 16;

fn display_header(cmd: u8, target: u8, out: &mut Vec<u8>) {
    out.extend_from_slice(&[0xF0, 0x00, 0x20, 0x29, 0x02, 0x14, cmd, target]);
}

/// Configure a display target (SysEx 04h): `config` 0 cancels it, 7Fh brings it up.
pub fn display_config(target: u8, config: u8) -> Vec<u8> {
    let mut m = Vec::with_capacity(10);
    display_header(0x04, target, &mut m);
    m.extend_from_slice(&[config & 0x7F, 0xF7]);
    m
}

/// Set a text field of a display target (SysEx 06h). The text is made printable: the
/// display takes ASCII 20h-7Eh, plus a flat sign at 1Dh.
pub fn display_field(target: u8, field: u8, text: &str) -> Vec<u8> {
    let mut m = Vec::with_capacity(12 + DISPLAY_CHARS);
    display_header(0x06, target, &mut m);
    m.push(field);
    m.extend(display_chars(text));
    m.push(0xF7);
    m
}

/// `text` as the display's characters, at most `DISPLAY_CHARS`.
pub fn display_chars(text: &str) -> impl Iterator<Item = u8> + '_ {
    text.chars()
        .filter_map(|c| match c {
            ' '..='~' => Some(c as u8),
            '♭' => Some(0x1D),
            '▲' => Some(b'^'),
            '▼' => Some(b'v'),
            '◀' => Some(b'<'),
            '▶' | '▸' | '→' => Some(b'>'),
            '−' | '–' | '—' => Some(b'-'),
            '·' => Some(b'.'),
            _ => None,
        })
        .take(DISPLAY_CHARS)
}

/// What the display shows for a touched control: a title (where it is), the function's
/// name and its value, brought up at once.
pub fn display_msgs(title: &str, name: &str, value: &str) -> [Vec<u8>; 5] {
    [
        display_config(DISPLAY_TARGET, DISPLAY_TITLE_NAME_VALUE),
        display_field(DISPLAY_TARGET, 0, title),
        display_field(DISPLAY_TARGET, 1, name),
        display_field(DISPLAY_TARGET, 2, value),
        display_config(DISPLAY_TARGET, 0x7F),
    ]
}

/// On entering DAW mode: the faders' and encoders' own temporary displays (the raw CC
/// value) off, as yahaha shows what they do instead (targets 05h-0Dh and 15h-1Ch, config
/// with the "on change" and "on touch" bits clear).
pub fn analogue_displays_off() -> impl Iterator<Item = Vec<u8>> {
    FADER_CC.chain(ENCODER_CC).map(|t| display_config(t, DISPLAY_TITLE_NAME_VALUE))
}

/// Pad pages.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Page {
    #[default]
    Sections,
    ChordSetup,
    OtsParts,
    /// Registration Memory buttons 1-10, banks, Memory, Freeze, the Registration Sequence.
    Registration,
    /// Multi Pads 1-4, STOP, SELECT + pad (Synchro Start) and STOP + pad (#196).
    MultiPads,
}

impl Page {
    pub const ALL: [Page; 5] = [Page::Sections, Page::ChordSetup, Page::OtsParts, Page::Registration, Page::MultiPads];

    pub fn name(self) -> &'static str {
        match self {
            Page::Sections => "Sections",
            Page::ChordSetup => "Chord/Setup",
            Page::OtsParts => "OTS/Parts",
            Page::Registration => "Registration",
            Page::MultiPads => "Multi Pads",
        }
    }

    pub fn to_u8(self) -> u8 {
        self as u8
    }

    pub fn from_u8(v: u8) -> Page {
        Page::ALL[(v as usize).min(Page::ALL.len() - 1)]
    }

    /// The page `d` steps away, stopping at the first and last (Pad Bank ▲/▼).
    pub fn step(self, d: i8) -> Page {
        Page::from_u8((self as i8).saturating_add(d).max(0) as u8)
    }

    /// The page `d` steps away, wrapping around (the terminal's Tab / Shift+Tab).
    pub fn cycle(self, d: i8) -> Page {
        let n = Page::ALL.len() as i16;
        Page::from_u8((self as i16 + d as i16).rem_euclid(n) as u8)
    }

    /// The page's LED identity: RGB for the pads, plus bright and dim palette colours.
    /// Page 1 keeps its per-section pad colours; white is its Pad Bank button colour.
    pub fn colour(self) -> ((u8, u8, u8), u8, u8) {
        match self {
            Page::Sections => (C_TAP, WHITE, DIM_WHITE),
            Page::ChordSetup => (C_PAGE_CHORD, CYAN, DIM_CYAN),
            Page::OtsParts => (C_PAGE_OTS, PINK, DIM_PINK),
            Page::Registration => (C_PAGE_REGIST, ORANGE, DIM_ORANGE),
            Page::MultiPads => (C_PAGE_PADS, YELLOW, DIM_YELLOW),
        }
    }
}

/// What a pad or button does. Engine buttons go straight to the engine from the MIDI
/// thread; everything else runs on the session's control side as the `AppCmd` its
/// keyboard shortcut sends (`impl From<Action> for AppCmd`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Button(Button),
    /// Select a fingering type (the `f` key steps through them: `NextFingering`).
    Fingering(Fingering),
    NextFingering,
    /// Chord Detection Area Lower/Upper (`d`).
    ToggleUpper,
    /// Manual Bass (`D`), Upper only.
    ToggleManualBass,
    /// Split point down/up one key (`[` `]`).
    Split(i8),
    /// Transpose steps: Keyboard (`;` `'`) and Master (`:` `"`).
    Transpose { keyboard: i8, master: i8 },
    /// Keyboard and Master transpose back to 0 (`/`).
    TransposeReset,
    /// Recall One Touch Setting 1-4 (0-based; `shift+1`-`4`).
    Ots(u8),
    /// OTS Link on/off (`F10`).
    ToggleOtsLink,
    /// Keyboard part on/off: 0-2 = Right 1-3, 3 = Left (`5`-`8`; `l` for Left).
    PartOnOff(u8),
    /// Select the keyboard part the voice keys edit (`F1`-`F4`).
    SelectPart(u8),
    /// Previous/next voice for the selected part (`9` `0`).
    PartVoice(i8),
    /// Fader page Panel/Style (`F9`; the button under the master fader).
    ToggleFaderPage,
    /// Previous/next style (`←` `→`).
    Style(i8),
    /// Style Retrigger length shorter (+1) / longer (-1) (`}` `{`).
    RetriggerRate(i8),
    /// A Registration Memory button 1-10 (0-based; `Q`-`P` with Shift): recall, or
    /// memorize while Memory is armed.
    Regist(u8),
    /// The MEMORY button (`F5`): the next Regist button memorizes.
    RegistMemory,
    /// FREEZE on/off (`F6`).
    RegistFreeze,
    /// REGIST BANK -/+ (`F11` `F12`).
    RegistBank(i8),
    /// Regist -/+: the Registration Sequence (`F7` `F8`).
    RegistSeq(i8),
    /// Previous/next Playlist record (`<` `>`; Shift + Track < / >).
    Playlist(i8),
    /// A pedal's assignable function that the control side runs (`controllers.rs`).
    Assign(crate::controllers::Function),
    /// A Hold A / Hold B pedal sets a control-side switch on or off (`controllers::Fire::set`).
    AssignSet(crate::controllers::Function, bool),
    /// The HARMONY/ARPEGGIO button: the selected Harmony type or arpeggio on/off (`J`,
    /// fader button 5 on the Panel fader page).
    ToggleHarmonyArp,
    /// Load the selected part's plugin again after it stopped or failed to load (`s`,
    /// fader button 6 on the Panel fader page).
    ReloadPlugin,
    /// A Multi Pad button (page 5; `Z X C V`, `B`): straight to the engine, as a section
    /// pad, so a press is not held up by the control side.
    MultiPad(PadCmd),
    /// An encoder turned: knob 0-7, steps (positive: clockwise).
    Knob(u8, i8),
    /// The Knob Assign page up (-1) / down (+1): the encoder page buttons.
    KnobPage(i8),
}

/// What a pad does on a page.
pub fn pad_action(page: Page, note: u8) -> Option<Action> {
    Some(match (page, note) {
        (Page::Sections, _) => return pad_button(note).map(Action::Button),
        (Page::ChordSetup, 96..=102) => Action::Fingering(Fingering::ALL[(note - 96) as usize]),
        (Page::ChordSetup, 103) => Action::ToggleUpper,
        (Page::ChordSetup, 112) => Action::ToggleManualBass,
        (Page::ChordSetup, 113) => Action::Button(Button::StopAcmp),
        (Page::ChordSetup, 114) => Action::Split(-1),
        (Page::ChordSetup, 115) => Action::Split(1),
        (Page::ChordSetup, 116) => Action::Transpose { keyboard: -1, master: 0 },
        (Page::ChordSetup, 117) => Action::Transpose { keyboard: 1, master: 0 },
        (Page::ChordSetup, 118) => Action::TransposeReset,
        (Page::ChordSetup, 119) => Action::Button(Button::Retrigger),
        (Page::OtsParts, 96..=99) => Action::Ots(note - 96),
        (Page::OtsParts, 100) => Action::ToggleOtsLink,
        (Page::OtsParts, 101) => Action::Button(Button::Fade),
        (Page::OtsParts, 102) => Action::PartVoice(-1),
        (Page::OtsParts, 103) => Action::PartVoice(1),
        (Page::OtsParts, 112..=115) => Action::PartOnOff(note - 112),
        (Page::OtsParts, 116..=119) => Action::SelectPart(note - 116),
        (Page::Registration, 96..=103) => Action::Regist(note - 96),
        (Page::Registration, 112..=113) => Action::Regist(note - 112 + 8),
        (Page::Registration, 114) => Action::RegistBank(-1),
        (Page::Registration, 115) => Action::RegistBank(1),
        (Page::Registration, 116) => Action::RegistMemory,
        (Page::Registration, 117) => Action::RegistFreeze,
        (Page::Registration, 118) => Action::RegistSeq(-1),
        (Page::Registration, 119) => Action::RegistSeq(1),
        (Page::MultiPads, 96..=99) => Action::MultiPad(PadCmd::Trigger(note - 96)),
        (Page::MultiPads, 100) => Action::MultiPad(PadCmd::StopAll),
        (Page::MultiPads, 112..=115) => Action::MultiPad(PadCmd::Arm(note - 112)),
        (Page::MultiPads, 116..=119) => Action::MultiPad(PadCmd::Stop(note - 116)),
        _ => return None,
    })
}

/// The TEMPO buttons (Function −, Scene +; without Shift): their direction. They act on
/// release too (a held button repeats), so the input thread handles them before
/// `cc_control`.
pub fn tempo_button(cc: u8) -> Option<i8> {
    match cc {
        SCENE_CC => Some(1),
        FUNCTION_CC => Some(-1),
        _ => None,
    }
}

/// What a pressed button (CC) does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Control {
    /// Pad page up (-1) / down (+1).
    Page(i8),
    Act(Action),
}

/// The button CCs, with Shift held or not. Shift itself, the faders and the fader
/// buttons are handled by the caller. None = not ours.
pub fn cc_control(cc: u8, shift: bool) -> Option<Control> {
    let act = |a| Some(Control::Act(a));
    match cc {
        PLAY_CC if shift => act(Action::Button(Button::SectionReset)),
        STOP_CC if shift => act(Action::Button(Button::Fade)),
        SCENE_CC if shift => act(Action::RetriggerRate(1)),
        FUNCTION_CC if shift => act(Action::RetriggerRate(-1)),
        PLAY_CC => act(Action::Button(Button::StartStop)),
        STOP_CC => act(Action::Button(Button::Stop)),
        SCENE_CC => act(Action::Button(Button::TempoUp)),
        FUNCTION_CC => act(Action::Button(Button::TempoDown)),
        // Shift + Track < / >: the Playlist, a set list's previous/next song.
        TRACK_LEFT_CC if shift => act(Action::Playlist(-1)),
        TRACK_RIGHT_CC if shift => act(Action::Playlist(1)),
        TRACK_LEFT_CC => act(Action::Style(-1)),
        TRACK_RIGHT_CC => act(Action::Style(1)),
        // Shift + Pad Bank ▲/▼: the toggles these buttons had before pages (also on page 3).
        PAD_UP_CC if shift => act(Action::PartOnOff(parts::LEFT as u8)),
        PAD_DOWN_CC if shift => act(Action::ToggleOtsLink),
        KNOB_UP_CC => act(Action::KnobPage(-1)),
        KNOB_DOWN_CC => act(Action::KnobPage(1)),
        PAD_UP_CC => Some(Control::Page(-1)),
        PAD_DOWN_CC => Some(Control::Page(1)),
        _ => None,
    }
}

/// Button lights (palette colour on ch 1, plus brightness on ch 4 in case they're
/// single-colour LEDs): Pad Bank ▲/▼ in the page's colour where there is a page to go to,
/// Track < / > when there is another style to go to.
pub fn nav_button_msgs(page: Page, styles: bool, out: &mut Vec<[u8; 3]>) {
    let (_, c, _) = page.colour();
    let up = page != Page::ALL[0];
    let down = page != Page::ALL[Page::ALL.len() - 1];
    for (cc, on, colour) in [
        (PAD_UP_CC, up, c),
        (PAD_DOWN_CC, down, c),
        (TRACK_LEFT_CC, styles, WHITE),
        (TRACK_RIGHT_CC, styles, WHITE),
    ] {
        out.push([0xB0, cc, if on { colour } else { OFF }]);
        out.push([0xB3, cc, if on { 127 } else { 0 }]);
    }
}

/// On exit: the Pad Bank, Track and fader button lights off.
pub fn buttons_off_msgs(out: &mut Vec<[u8; 3]>) {
    for cc in [PAD_UP_CC, PAD_DOWN_CC, TRACK_LEFT_CC, TRACK_RIGHT_CC].into_iter().chain(FADER_BTN_CC) {
        out.push([0xB0, cc, OFF]);
        out.push([0xB3, cc, 0]);
    }
}

/// The fader button (0-based, under fader 5) that is the HARMONY/ARPEGGIO switch on the
/// Panel fader page. Every pad on every page is taken; Panel buttons 5-8 were dark.
pub const HARM_ARP_FADER_BTN: u8 = 4;

/// The fader button (0-based, under fader 6) that reloads the selected part's plugin on the
/// Panel fader page. It lights red while that plugin has stopped or failed to load.
pub const PLUGIN_FADER_BTN: u8 = 5;

/// The fader button (0-based, under fader 7) that is LEFT HOLD on the Panel fader page
/// (#202): lit orange while it is on.
pub const LEFT_HOLD_FADER_BTN: u8 = 6;

/// The fader button (0-based, under fader 8) that is the CHORD LOOPER on the Panel fader
/// page: ON/OFF, and with Shift REC/STOP (#201).
pub const LOOPER_FADER_BTN: u8 = 7;

/// The Chord Looper as its fader button shows it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LooperLamp {
    /// Nothing recorded: dark.
    #[default]
    Empty,
    /// A sequence to loop: dim green.
    Ready,
    /// REC armed (dim red) or recording (red).
    RecArmed,
    Recording,
    /// ON/OFF armed (dim yellow: the loop starts at the next bar line) or looping (green).
    LoopArmed,
    Looping,
}

/// What the Panel page's own fader buttons (5-8) show.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PanelLamps {
    /// The HARMONY/ARPEGGIO switch (button 5).
    pub harmony_arp: bool,
    /// The selected part's plugin needs a reload (button 6 red).
    pub plugin_fault: bool,
    /// Left Hold (button 7).
    pub left_hold: bool,
    /// The Chord Looper (button 8).
    pub looper: LooperLamp,
}

/// Palette colours for the fader buttons. Panel page (blue): Right 1-3 and Left lit while
/// on (`parts_on`, bit = part), button 5 (purple) lit while HARMONY/ARPEGGIO is on,
/// button 6 red while the selected part's plugin needs a reload, button 7 (orange) lit
/// while Left Hold is on, button 8 the Chord Looper (`LooperLamp`). Style page (green): the Style parts lit while they
/// play (`style_on`). The master button shows the page's colour.
pub fn fader_button_msgs(page: FaderPage, parts_on: u8, style_on: u8, lamps: PanelLamps, out: &mut Vec<[u8; 3]>) {
    let (on, n, (bright, dim)) = match page {
        FaderPage::Panel => (parts_on, parts::COUNT as u8, (BLUE, DIM_BLUE)),
        FaderPage::Style => (style_on, 8, (GREEN, DIM_GREEN)),
    };
    for i in 0..8u8 {
        let c = if page == FaderPage::Panel && i == HARM_ARP_FADER_BTN {
            if lamps.harmony_arp { PURPLE } else { DIM_PURPLE }
        } else if page == FaderPage::Panel && i == PLUGIN_FADER_BTN {
            if lamps.plugin_fault { RED } else { OFF }
        } else if page == FaderPage::Panel && i == LEFT_HOLD_FADER_BTN {
            if lamps.left_hold { ORANGE } else { DIM_ORANGE }
        } else if page == FaderPage::Panel && i == LOOPER_FADER_BTN {
            match lamps.looper {
                LooperLamp::Empty => OFF,
                LooperLamp::Ready => DIM_GREEN,
                LooperLamp::RecArmed => DIM_RED,
                LooperLamp::Recording => RED,
                LooperLamp::LoopArmed => DIM_YELLOW,
                LooperLamp::Looping => GREEN,
            }
        } else if i >= n {
            OFF
        } else if on & (1 << i) != 0 {
            bright
        } else {
            dim
        };
        out.push([0xB0, 37 + i, c]);
    }
    out.push([0xB0, 45, bright]);
}

/// The Style parts the Style-page fader buttons show as playing (`style_on` of
/// `fader_button_msgs`): Manual Bass mutes the Style's Bass part in the engine, so it
/// shows off, as on screen.
pub fn style_lit(parts: u8, manual_bass: bool) -> u8 {
    if manual_bass { parts & !(1 << 2) } else { parts }
}

/// The button LEDs as `nav_button_msgs` and `fader_button_msgs` set them: (CC, palette
/// colour) for Pad Bank ▲/▼, Track ◀/▶, the fader buttons and the master fader button.
pub fn button_colours(page: Page, styles: bool, fader_page: FaderPage, parts_on: u8, style_on: u8, lamps: PanelLamps) -> Vec<(u8, u8)> {
    let mut msgs = Vec::new();
    nav_button_msgs(page, styles, &mut msgs);
    fader_button_msgs(fader_page, parts_on, style_on, lamps, &mut msgs);
    // Channel 1 carries the colour (channel 4 the brightness, for single-colour LEDs).
    msgs.iter().filter(|m| m[0] == 0xB0).map(|m| (m[1], m[2])).collect()
}

/// What a Novation palette colour yahaha uses looks like: its full colour (0-127 per
/// channel, as `Look::rgb`) and its level. Colours yahaha never sends read as off.
pub fn palette_colour(c: u8) -> ((u8, u8, u8), Level) {
    let (rgb, bright) = match c {
        WHITE => ((127, 127, 127), true),
        DIM_WHITE => ((127, 127, 127), false),
        RED => ((127, 0, 0), true),
        DIM_RED => ((127, 0, 0), false),
        ORANGE => ((127, 60, 0), true),
        DIM_ORANGE => ((127, 60, 0), false),
        YELLOW => ((127, 127, 0), true),
        DIM_YELLOW => ((127, 127, 0), false),
        GREEN => ((0, 127, 0), true),
        DIM_GREEN => ((0, 127, 0), false),
        CYAN => ((0, 100, 127), true),
        DIM_CYAN => ((0, 100, 127), false),
        BLUE => ((0, 0, 127), true),
        DIM_BLUE => ((0, 0, 127), false),
        PURPLE => ((90, 0, 127), true),
        DIM_PURPLE => ((90, 0, 127), false),
        PINK => ((127, 0, 70), true),
        DIM_PINK => ((127, 0, 70), false),
        _ => return ((0, 0, 0), Level::Off),
    };
    (rgb, if bright { Level::Bright } else { Level::Dim })
}

/// Panel state outside the engine snapshot that pages 2 and 3 show.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Panel {
    pub page: Page,
    pub fingering: Fingering,
    pub upper: bool,
    /// The Manual Bass setting (in effect only in Upper).
    pub manual_bass: bool,
    /// One Touch Settings in the style, and the last one recalled (1-based, 0 = none).
    pub ots_count: u8,
    pub ots_applied: u8,
    pub ots_link: bool,
    /// The HARMONY/ARPEGGIO switch.
    pub harmony_arp: bool,
    /// The selected part's plugin stopped working or failed to load (fader button 6 red).
    pub plugin_fault: bool,
    /// Left Hold (fader button 7).
    pub left_hold: bool,
    /// The Chord Looper (fader button 8).
    pub looper: LooperLamp,
    /// Keyboard parts that are on (bit = `parts::RIGHT1`..`LEFT`), and the selected one.
    pub parts_on: u8,
    pub selected: u8,
    /// Registration Memory, for page 4.
    pub regist: RegistPanel,
}

/// Registration Memory as page 4 shows it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct RegistPanel {
    /// Buttons holding a registration (bit 0 = button 1).
    pub stored: u16,
    /// The button last recalled or memorized, 1-based (0 = none).
    pub selected: u8,
    /// MEMORY armed.
    pub memory: bool,
    pub freeze: bool,
    /// The Registration Sequence is on and has steps.
    pub sequence: bool,
    /// There are bank files to step to.
    pub banks: bool,
}

impl Panel {
    /// What the Panel page's own fader buttons show.
    pub fn lamps(&self) -> PanelLamps {
        PanelLamps { harmony_arp: self.harmony_arp, plugin_fault: self.plugin_fault, left_hold: self.left_hold, looper: self.looper }
    }
}

impl Default for Panel {
    fn default() -> Panel {
        Panel {
            page: Page::Sections,
            fingering: Fingering::FingeredOnBass,
            upper: false,
            manual_bass: true,
            ots_count: 0,
            ots_applied: 0,
            ots_link: false,
            harmony_arp: false,
            plugin_fault: false,
            left_hold: false,
            looper: LooperLamp::Empty,
            parts_on: 1 << parts::RIGHT1,
            selected: parts::RIGHT1 as u8,
            regist: RegistPanel::default(),
        }
    }
}

// Novation palette indices.
const OFF: u8 = 0;
const WHITE: u8 = 3;
const DIM_WHITE: u8 = 1;
const RED: u8 = 5;
const DIM_RED: u8 = 7;
const ORANGE: u8 = 9;
const DIM_ORANGE: u8 = 11;
const YELLOW: u8 = 13;
const DIM_YELLOW: u8 = 15;
const GREEN: u8 = 21;
const DIM_GREEN: u8 = 23;
const CYAN: u8 = 37;
const DIM_CYAN: u8 = 39;
const BLUE: u8 = 45;
const DIM_BLUE: u8 = 47;
const PURPLE: u8 = 53;
const DIM_PURPLE: u8 = 55;
const PINK: u8 = 57;
const DIM_PINK: u8 = 59;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Led {
    Solid(u8),
    Flash(u8, u8),
    Pulse(u8),
}

/// Desired LED state for all 16 pads (palette mode) on the current page.
pub fn pad_leds(s: &Snapshot, has: &[bool], panel: &Panel) -> [(u8, Led); 16] {
    if panel.page == Page::Sections {
        return section_leds(s, has);
    }
    if panel.page == Page::Registration {
        return regist_leds(&panel.regist);
    }
    if panel.page == Page::MultiPads {
        return multipad_leds(s);
    }
    let (_, bright, dim) = panel.page.colour();
    looks(s, has, panel).map(|(note, look)| {
        let led = match (look.level, look.anim) {
            (Level::Off, _) => Led::Solid(OFF),
            (Level::Dim, _) => Led::Solid(dim),
            (Level::Bright, Anim::Solid) => Led::Solid(bright),
            (Level::Bright, Anim::Flash) => Led::Flash(dim, bright),
            (Level::Bright, Anim::Pulse) => Led::Pulse(bright),
        };
        (note, led)
    })
}

/// Page 1 in palette mode, given engine state and which sections exist.
fn section_leds(s: &Snapshot, has: &[bool]) -> [(u8, Led); 16] {
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

/// Page 4 in palette mode: the Genos lamp colours on the buttons (red = selected, blue =
/// stored, off = empty; flashing red while Memory is armed), orange on the rest.
fn regist_leds(r: &RegistPanel) -> [(u8, Led); 16] {
    let button = |i: u8| -> Led {
        let stored = r.stored & (1 << i) != 0;
        if r.memory {
            Led::Flash(DIM_RED, RED)
        } else if r.selected == i + 1 && stored {
            Led::Solid(RED)
        } else if stored {
            Led::Solid(BLUE)
        } else {
            Led::Solid(OFF)
        }
    };
    let tog = |avail: bool, on: bool| Led::Solid(match (avail, on) {
        (false, _) => OFF,
        (true, true) => ORANGE,
        (true, false) => DIM_ORANGE,
    });
    [
        (96, button(0)),
        (97, button(1)),
        (98, button(2)),
        (99, button(3)),
        (100, button(4)),
        (101, button(5)),
        (102, button(6)),
        (103, button(7)),
        (112, button(8)),
        (113, button(9)),
        (114, tog(r.banks, false)),
        (115, tog(r.banks, false)),
        (116, if r.memory { Led::Flash(DIM_RED, RED) } else { tog(true, false) }),
        (117, tog(true, r.freeze)),
        (118, tog(r.sequence, false)),
        (119, tog(r.sequence, false)),
    ]
}

/// Page 5 in palette mode: the Genos Multi Pad lamps on pads 1-4 (blue = data, red =
/// playing, flashing red = Synchro Start standby, flashing orange = waiting for the bar
/// line, off = empty), yellow on the rest.
fn multipad_leds(s: &Snapshot) -> [(u8, Led); 16] {
    multipad_looks(s).map(|(note, look)| {
        let (bright, dim) = match look.rgb {
            C_PAD_READY => (BLUE, DIM_BLUE),
            C_PAD_PLAYING => (RED, DIM_RED),
            C_PAD_QUEUED => (ORANGE, DIM_ORANGE),
            _ => (YELLOW, DIM_YELLOW),
        };
        let led = match (look.level, look.anim) {
            (Level::Off, _) => Led::Solid(OFF),
            (Level::Dim, _) => Led::Solid(dim),
            (Level::Bright, Anim::Solid) => Led::Solid(bright),
            (Level::Bright, Anim::Flash) => Led::Flash(dim, bright),
            (Level::Bright, Anim::Pulse) => Led::Pulse(bright),
        };
        (note, led)
    })
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

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Level {
    #[default]
    Off,
    Dim,
    Bright,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Anim {
    #[default]
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
/// Page identities: every pad on page 2 is cyan, every pad on page 3 magenta.
pub const C_PAGE_CHORD: (u8, u8, u8) = (0, 100, 127);
pub const C_PAGE_OTS: (u8, u8, u8) = (127, 0, 70);
/// Page 4: orange, with the Registration buttons in the Genos lamp colours.
pub const C_PAGE_REGIST: (u8, u8, u8) = (127, 60, 0);
/// Registration lamps: red = selected, blue = stored (OM p.97).
pub const C_REGIST_SELECTED: (u8, u8, u8) = (127, 0, 0);
pub const C_REGIST_STORED: (u8, u8, u8) = (0, 40, 127);
/// Page 5: yellow, with the Multi Pads in the Genos lamp colours (blue = data, red =
/// playing; OM p.75) and amber while a press waits for the bar line.
pub const C_PAGE_PADS: (u8, u8, u8) = (127, 127, 0);
pub const C_PAD_READY: (u8, u8, u8) = (0, 40, 127);
pub const C_PAD_PLAYING: (u8, u8, u8) = (127, 0, 0);
pub const C_PAD_QUEUED: (u8, u8, u8) = (127, 60, 0);

/// Brightness of "dim" relative to full.
const DIM: f32 = 0.18;

/// Pad looks for the current page.
pub fn looks(s: &Snapshot, has: &[bool], panel: &Panel) -> [(u8, Look); 16] {
    match panel.page {
        Page::Sections => section_looks(s, has),
        Page::ChordSetup => chord_looks(s, panel),
        Page::OtsParts => ots_looks(s, panel),
        Page::Registration => regist_looks(&panel.regist),
        Page::MultiPads => multipad_looks(s),
    }
}

fn section_looks(s: &Snapshot, has: &[bool]) -> [(u8, Look); 16] {
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

/// A page 2/3 pad in the page's colour: bright when on, dim when off, dark when unavailable.
fn page_look(page: Page, label: &'static str, key: &'static str, available: bool, on: bool) -> Look {
    let level = match (available, on) {
        (false, _) => Level::Off,
        (true, true) => Level::Bright,
        (true, false) => Level::Dim,
    };
    Look { label, key, rgb: page.colour().0, level, anim: Anim::Solid }
}

const FINGERING_LABELS: [&str; 7] = ["SINGLE", "FINGERED", "ON BASS", "MULTI", "AI FING", "FULL KBD", "AI FULL"];

fn chord_looks(s: &Snapshot, p: &Panel) -> [(u8, Look); 16] {
    let pl = |label, key, available, on| page_look(Page::ChordSetup, label, key, available, on);
    // No key selects a type directly (`f` steps through them), so the hint says "pad".
    let fing = |i: usize| pl(FINGERING_LABELS[i], "pad", true, p.fingering == Fingering::ALL[i]);
    let t = s.transpose;
    [
        (96, fing(0)),
        (97, fing(1)),
        (98, fing(2)),
        (99, fing(3)),
        (100, fing(4)),
        (101, fing(5)),
        (102, fing(6)),
        (103, pl("UPPER", "d", true, p.upper)),
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

pub const PART_LABELS: [&str; 4] = ["RIGHT 1", "RIGHT 2", "RIGHT 3", "LEFT"];
const PART_KEYS: [&str; 4] = ["5", "6", "7", "8/l"];
pub const SELECT_LABELS: [&str; 4] = ["EDIT R1", "EDIT R2", "EDIT R3", "EDIT L"];
const SELECT_KEYS: [&str; 4] = ["F1", "F2", "F3", "F4"];

fn ots_looks(s: &Snapshot, p: &Panel) -> [(u8, Look); 16] {
    let pl = |label, key, available, on| page_look(Page::OtsParts, label, key, available, on);
    let ots = |n: u8, label, key| pl(label, key, n < p.ots_count, p.ots_applied == n + 1);
    let part = |i: usize| pl(PART_LABELS[i], PART_KEYS[i], true, p.parts_on & (1 << i) != 0);
    let select = |i: usize| pl(SELECT_LABELS[i], SELECT_KEYS[i], true, p.selected as usize == i);
    [
        (96, ots(0, "OTS 1", "⇧1")),
        (97, ots(1, "OTS 2", "⇧2")),
        (98, ots(2, "OTS 3", "⇧3")),
        (99, ots(3, "OTS 4", "⇧4")),
        (100, pl("OTS LINK", "F10", true, p.ots_link)),
        (101, pl("FADE", "F", true, s.fade != FadeState::Off)),
        (102, pl("VOICE -", "9", true, false)),
        (103, pl("VOICE +", "0", true, false)),
        (112, part(0)),
        (113, part(1)),
        (114, part(2)),
        (115, part(3)),
        (116, select(0)),
        (117, select(1)),
        (118, select(2)),
        (119, select(3)),
    ]
}

const REGIST_LABELS: [&str; 10] = ["REGIST 1", "REGIST 2", "REGIST 3", "REGIST 4", "REGIST 5", "REGIST 6", "REGIST 7", "REGIST 8", "REGIST 9", "REGIST 10"];
const REGIST_KEYS: [&str; 10] = ["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P"];

fn regist_looks(r: &RegistPanel) -> [(u8, Look); 16] {
    let pl = |label, key, available, on| page_look(Page::Registration, label, key, available, on);
    let button = |i: u8| -> Look {
        let (label, key) = (REGIST_LABELS[i as usize], REGIST_KEYS[i as usize]);
        let stored = r.stored & (1 << i) != 0;
        let look = |rgb, level, anim| Look { label, key, rgb, level, anim };
        if r.memory {
            // Armed: every button waits to be memorized into.
            look(C_REGIST_SELECTED, Level::Bright, Anim::Flash)
        } else if stored && r.selected == i + 1 {
            look(C_REGIST_SELECTED, Level::Bright, Anim::Solid)
        } else if stored {
            look(C_REGIST_STORED, Level::Bright, Anim::Solid)
        } else {
            look(C_REGIST_STORED, Level::Off, Anim::Solid)
        }
    };
    let memory = if r.memory {
        Look { label: "MEMORY", key: "F5", rgb: C_REGIST_SELECTED, level: Level::Bright, anim: Anim::Flash }
    } else {
        pl("MEMORY", "F5", true, false)
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
        (112, button(8)),
        (113, button(9)),
        (114, pl("BANK -", "F11", r.banks, false)),
        (115, pl("BANK +", "F12", r.banks, false)),
        (116, memory),
        (117, pl("FREEZE", "F6", true, r.freeze)),
        (118, pl("REGIST -", "F7", r.sequence, false)),
        (119, pl("REGIST +", "F8", r.sequence, false)),
    ]
}

const MULTIPAD_LABELS: [&str; 4] = ["PAD 1", "PAD 2", "PAD 3", "PAD 4"];
const MULTIPAD_KEYS: [&str; 4] = ["Z", "X", "C", "V"];
const SELECT_PAD_LABELS: [&str; 4] = ["SELECT 1", "SELECT 2", "SELECT 3", "SELECT 4"];
const STOP_PAD_LABELS: [&str; 4] = ["STOP 1", "STOP 2", "STOP 3", "STOP 4"];

fn multipad_looks(s: &Snapshot) -> [(u8, Look); 16] {
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

/// Colour at a point in time. `beats` is a free-running beat clock (fractional).
pub fn rgb_at(look: &Look, beats: f64) -> (u8, u8, u8) {
    lit(look.rgb, look.level, look.anim, beats)
}

/// Colour at a point in time for a pad's full colour, level and animation (a `Look`, or
/// an `api::Pad`).
pub fn lit(rgb: (u8, u8, u8), level: Level, anim: Anim, beats: f64) -> (u8, u8, u8) {
    let k = match (level, anim) {
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
    (f(rgb.0), f(rgb.1), f(rgb.2))
}

/// SysEx that sets a pad to an RGB colour (0..=127 per channel). Regular (non-Mini) SKU.
pub fn rgb_sysex(pad: u8, (r, g, b): (u8, u8, u8)) -> [u8; 13] {
    [0xF0, 0x00, 0x20, 0x29, 0x02, 0x14, 0x01, 0x43, pad, r, g, b, 0xF7]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap() -> Snapshot {
        Snapshot {
            running: false, sync_armed: false, sync_stop: false, auto_fill: false, cur: None, queued: None,
            pending_intro: None, main: 0, bar: 0, beat: 0, chord: None, bpm: 120.0, parts: 0xFF, volumes: [100; 8], user_set: 0, pickup: 0,
            stop_acmp: false, stop_acmp_mode: crate::engine::StopAcmp::Off, half_bar_fill: false, main_presses: 0, transpose: Transpose::default(), played: None, anchor_ns: 0, anchor_beats: 0.0, style_tag: 0,
            style_pending: false, section_bars: 0, audition: None, fade: FadeState::Off, retrigger: false, ritardando: false,
            looper: Default::default(), style_solo: None,
            multipad: Default::default(), chart_tag: 0, chart_bar: None, chart_override: false, dynamics: 64,
        }
    }

    /// Every colour the LED functions send has a look, and the button colours are the
    /// channel-1 half of the messages.
    #[test]
    fn palette_colours_and_button_leds() {
        for c in [WHITE, DIM_WHITE, RED, DIM_RED, ORANGE, DIM_ORANGE, YELLOW, DIM_YELLOW, GREEN, DIM_GREEN, CYAN, DIM_CYAN, BLUE, DIM_BLUE, PURPLE, DIM_PURPLE, PINK, DIM_PINK] {
            assert_ne!(palette_colour(c).1, Level::Off, "{c}");
        }
        assert_eq!(palette_colour(OFF).1, Level::Off);
        let b = button_colours(Page::Sections, true, FaderPage::Panel, 0b0001, 0xFF, PanelLamps::default());
        assert!(b.contains(&(PAD_UP_CC, OFF)) && b.contains(&(PAD_DOWN_CC, WHITE)));
        assert!(b.contains(&(TRACK_LEFT_CC, WHITE)));
        assert!(b.contains(&(37, BLUE)) && b.contains(&(38, DIM_BLUE)) && b.contains(&(41, DIM_PURPLE)) && b.contains(&(42, OFF)) && b.contains(&(45, BLUE)));
        assert_eq!(b.len(), 4 + 9);
    }

    const PADS: [u8; 16] = [96, 97, 98, 99, 100, 101, 102, 103, 112, 113, 114, 115, 116, 117, 118, 119];

    /// Encoders: relative steps on channel 16 in any encoder mode; touch events (channel
    /// 15) and other channels are not turns.
    #[test]
    fn encoders_are_relative_knobs() {
        assert_eq!(encoder(0xBF, 21, 65), Some((0, 1)));
        assert_eq!(encoder(0xBF, 28, 60), Some((7, -4)));
        assert_eq!(encoder(0xBF, 85, 63), Some((0, -1)), "Transport encoder mode");
        assert_eq!(encoder(0xBF, 92, 70), Some((7, 6)));
        assert_eq!(encoder(0xBF, 21, 64), None, "no move");
        assert_eq!(encoder(0xBE, 85, 127), None, "touch on");
        assert_eq!(encoder(0xB0, 21, 65), None);
        assert_eq!(encoder(0xBF, 13, 65), None, "the master fader");
        assert_eq!(encoder(0xBF, 29, 65), None);
    }

    #[test]
    fn touches_pack_and_unpack() {
        for t in [
            Touch::Pad(119),
            Touch::Button { cc: SHIFT_CC, shift: true },
            Touch::Button { cc: KNOB_DOWN_CC, shift: false },
            Touch::Fader(8),
            Touch::FaderButton { index: 3, shift: true },
            Touch::Knob(7),
        ] {
            let v = t.pack(0xFFFF);
            assert_ne!(v, 0);
            assert_eq!(Touch::unpack(v), Some(t));
            assert_ne!(t.pack(1), t.pack(2), "a second press of the same control counts");
        }
        assert_eq!(Touch::unpack(0), None);
    }

    /// The display SysEx (Programmer's Reference Guide v3.0, pp.17-19): configure the
    /// Global temporary display for Title/Name/Value, the three fields, then bring it up.
    #[test]
    fn display_sysex() {
        let m = display_msgs("Pads Sections", "MAIN B", "Fill In BB");
        assert_eq!(m[0], [0xF0, 0x00, 0x20, 0x29, 0x02, 0x14, 0x04, 0x21, 0x02, 0xF7]);
        assert_eq!(m[2], [&[0xF0, 0x00, 0x20, 0x29, 0x02, 0x14, 0x06, 0x21, 0x01][..], b"MAIN B", &[0xF7]].concat());
        assert_eq!(m[4], [0xF0, 0x00, 0x20, 0x29, 0x02, 0x14, 0x04, 0x21, 0x7F, 0xF7]);
        // Printable ASCII only, and short.
        let f = display_field(DISPLAY_TARGET, 2, "PAGE ▲ B♭ ✓ a very long value indeed");
        assert_eq!(&f[9..f.len() - 1], b"PAGE ^ B\x1D  a ver");
        assert!(f[9..f.len() - 1].iter().all(|&c| c < 0x80));
        let off: Vec<_> = analogue_displays_off().collect();
        assert_eq!(off.len(), 9 + 8);
        assert_eq!(off[0], display_config(0x05, 2));
        assert_eq!(off[9], display_config(0x15, 2));
    }

    #[test]
    fn page_1_is_the_original_layout() {
        for n in 0..=127u8 {
            assert_eq!(pad_action(Page::Sections, n), pad_button(n).map(Action::Button), "note {n}");
        }
        assert_eq!(pad_action(Page::Sections, 96), Some(Action::Button(Button::Intro(0))));
        assert_eq!(pad_action(Page::Sections, 119), Some(Action::Button(Button::StartStop)));
    }

    #[test]
    fn page_2_chord_setup() {
        let p = Page::ChordSetup;
        for (i, f) in Fingering::ALL.iter().enumerate() {
            assert_eq!(pad_action(p, 96 + i as u8), Some(Action::Fingering(*f)));
        }
        assert_eq!(pad_action(p, 96), Some(Action::Fingering(Fingering::SingleFinger)));
        assert_eq!(pad_action(p, 102), Some(Action::Fingering(Fingering::AiFullKeyboard)));
        assert_eq!(pad_action(p, 103), Some(Action::ToggleUpper));
        assert_eq!(pad_action(p, 112), Some(Action::ToggleManualBass));
        assert_eq!(pad_action(p, 113), Some(Action::Button(Button::StopAcmp)));
        assert_eq!(pad_action(p, 114), Some(Action::Split(-1)));
        assert_eq!(pad_action(p, 115), Some(Action::Split(1)));
        assert_eq!(pad_action(p, 116), Some(Action::Transpose { keyboard: -1, master: 0 }));
        assert_eq!(pad_action(p, 117), Some(Action::Transpose { keyboard: 1, master: 0 }));
        assert_eq!(pad_action(p, 118), Some(Action::TransposeReset));
        assert_eq!(pad_action(p, 119), Some(Action::Button(Button::Retrigger)));
        assert_eq!(pad_action(p, 60), None);
    }

    #[test]
    fn page_3_ots_parts() {
        let p = Page::OtsParts;
        for n in 0..4u8 {
            assert_eq!(pad_action(p, 96 + n), Some(Action::Ots(n)));
        }
        assert_eq!(pad_action(p, 100), Some(Action::ToggleOtsLink));
        assert_eq!(pad_action(p, 101), Some(Action::Button(Button::Fade)));
        assert_eq!(pad_action(p, 102), Some(Action::PartVoice(-1)));
        assert_eq!(pad_action(p, 103), Some(Action::PartVoice(1)));
        for n in 0..4u8 {
            assert_eq!(pad_action(p, 112 + n), Some(Action::PartOnOff(n)));
            assert_eq!(pad_action(p, 116 + n), Some(Action::SelectPart(n)));
        }
        assert_eq!(pad_action(p, 104), None);
    }

    #[test]
    fn page_4_registration() {
        let p = Page::Registration;
        for n in 0..8u8 {
            assert_eq!(pad_action(p, 96 + n), Some(Action::Regist(n)));
        }
        assert_eq!(pad_action(p, 112), Some(Action::Regist(8)));
        assert_eq!(pad_action(p, 113), Some(Action::Regist(9)));
        assert_eq!(pad_action(p, 114), Some(Action::RegistBank(-1)));
        assert_eq!(pad_action(p, 115), Some(Action::RegistBank(1)));
        assert_eq!(pad_action(p, 116), Some(Action::RegistMemory));
        assert_eq!(pad_action(p, 117), Some(Action::RegistFreeze));
        assert_eq!(pad_action(p, 118), Some(Action::RegistSeq(-1)));
        assert_eq!(pad_action(p, 119), Some(Action::RegistSeq(1)));
        // Shift + Track < / > step the Playlist.
        assert_eq!(cc_control(TRACK_LEFT_CC, true), Some(Control::Act(Action::Playlist(-1))));
        assert_eq!(cc_control(TRACK_RIGHT_CC, true), Some(Control::Act(Action::Playlist(1))));
    }

    /// Page 5: pads 1-4 and STOP; SELECT + pad and STOP + pad on the bottom row.
    #[test]
    fn page_5_multi_pads() {
        let p = Page::MultiPads;
        for n in 0..4u8 {
            assert_eq!(pad_action(p, 96 + n), Some(Action::MultiPad(PadCmd::Trigger(n))));
            assert_eq!(pad_action(p, 112 + n), Some(Action::MultiPad(PadCmd::Arm(n))));
            assert_eq!(pad_action(p, 116 + n), Some(Action::MultiPad(PadCmd::Stop(n))));
        }
        assert_eq!(pad_action(p, 100), Some(Action::MultiPad(PadCmd::StopAll)));
        for n in 101..=103 {
            assert_eq!(pad_action(p, n), None);
        }
    }

    /// Page 5 lamps as on the Genos: blue = data, red = playing, flashing red = standby,
    /// off = empty; amber flashing while a press waits for the bar line.
    #[test]
    fn page_5_lamps() {
        let mut s = snap();
        s.multipad.states = [PadState::Ready, PadState::Playing, PadState::Armed, PadState::Empty];
        let panel = Panel { page: Page::MultiPads, ..Panel::default() };
        let l = looks(&s, &[true; 32], &panel);
        assert_eq!((l[0].1.rgb, l[0].1.level, l[0].1.anim), (C_PAD_READY, Level::Bright, Anim::Solid));
        assert_eq!((l[1].1.rgb, l[1].1.anim), (C_PAD_PLAYING, Anim::Solid));
        assert_eq!((l[2].1.rgb, l[2].1.anim), (C_PAD_PLAYING, Anim::Flash));
        assert_eq!(l[3].1.level, Level::Off);
        assert_eq!(l[4].1.level, Level::Bright, "STOP lit while a pad plays");
        assert_eq!(l[5].1.level, Level::Off);
        assert_eq!((l[10].1.level, l[10].1.anim), (Level::Bright, Anim::Flash), "SELECT 3: pad 3 in standby");
        assert_eq!(l[8].1.level, Level::Dim);
        assert_eq!(l[11].1.level, Level::Off, "no data on pad 4");
        assert_eq!((l[12].1.level, l[13].1.level), (Level::Dim, Level::Bright), "STOP 2 lit: pad 2 plays");
        let leds = pad_leds(&s, &[true; 32], &panel);
        assert_eq!((leds[0].1, leds[1].1, leds[2].1, leds[3].1), (Led::Solid(BLUE), Led::Solid(RED), Led::Flash(DIM_RED, RED), Led::Solid(OFF)));
        assert_eq!((leds[4].1, leds[8].1), (Led::Solid(YELLOW), Led::Solid(DIM_YELLOW)));
        s.multipad.states = [PadState::Queued, PadState::Ready, PadState::Ready, PadState::Ready];
        assert_eq!(pad_leds(&s, &[true; 32], &panel)[0].1, Led::Flash(DIM_ORANGE, ORANGE));
        s.multipad.states = [PadState::Ready; 4];
        assert_eq!(looks(&s, &[true; 32], &panel)[4].1.level, Level::Dim, "nothing plays");
    }

    /// Page 4 lamps as on the Genos: red = selected, blue = stored, off = empty; all
    /// flashing while Memory is armed.
    #[test]
    fn page_4_lamps() {
        let regist = RegistPanel { stored: 0b101, selected: 3, memory: false, freeze: true, sequence: false, banks: true };
        let panel = Panel { page: Page::Registration, regist, ..Panel::default() };
        let l = looks(&snap(), &[true; 32], &panel);
        assert_eq!((l[0].1.rgb, l[0].1.level), (C_REGIST_STORED, Level::Bright));
        assert_eq!(l[1].1.level, Level::Off);
        assert_eq!((l[2].1.rgb, l[2].1.level), (C_REGIST_SELECTED, Level::Bright));
        assert_eq!(l[13].1.level, Level::Bright); // Freeze on
        assert_eq!(l[14].1.level, Level::Off); // no sequence
        let leds = pad_leds(&snap(), &[true; 32], &panel);
        assert_eq!(leds[0].1, Led::Solid(BLUE));
        assert_eq!(leds[1].1, Led::Solid(OFF));
        assert_eq!(leds[2].1, Led::Solid(RED));
        let armed = Panel { regist: RegistPanel { memory: true, ..regist }, ..panel };
        assert!(looks(&snap(), &[true; 32], &armed)[..10].iter().all(|(_, l)| l.anim == Anim::Flash));
        assert_eq!(pad_leds(&snap(), &[true; 32], &armed)[12].1, Led::Flash(DIM_RED, RED));
    }

    #[test]
    fn buttons_and_page_switching() {
        assert_eq!(cc_control(103, false), Some(Control::Act(Action::Style(-1))));
        assert_eq!(cc_control(102, false), Some(Control::Act(Action::Style(1))));
        assert_eq!(cc_control(106, false), Some(Control::Page(-1)));
        assert_eq!(cc_control(107, false), Some(Control::Page(1)));
        assert_eq!(cc_control(106, true), Some(Control::Act(Action::PartOnOff(3))));
        assert_eq!(cc_control(107, true), Some(Control::Act(Action::ToggleOtsLink)));
        // Tempo and transport stay where they were.
        assert_eq!(cc_control(104, false), Some(Control::Act(Action::Button(Button::TempoUp))));
        assert_eq!(cc_control(105, false), Some(Control::Act(Action::Button(Button::TempoDown))));
        assert_eq!(cc_control(115, false), Some(Control::Act(Action::Button(Button::StartStop))));
        assert_eq!(cc_control(116, false), Some(Control::Act(Action::Button(Button::Stop))));
        assert_eq!(cc_control(SHIFT_CC, false), None);
        assert_eq!(cc_control(51, false), Some(Control::Act(Action::KnobPage(-1))));
        assert_eq!(cc_control(52, true), Some(Control::Act(Action::KnobPage(1))));
        assert_eq!(cc_control(53, false), None);

        // ▲/▼ stop at the ends; Tab wraps.
        assert_eq!(Page::Sections.step(-1), Page::Sections);
        assert_eq!(Page::Sections.step(1), Page::ChordSetup);
        assert_eq!(Page::ChordSetup.step(1), Page::OtsParts);
        assert_eq!(Page::OtsParts.step(1), Page::Registration);
        assert_eq!(Page::Registration.step(1), Page::MultiPads);
        assert_eq!(Page::MultiPads.step(1), Page::MultiPads);
        assert_eq!(Page::OtsParts.step(-1), Page::ChordSetup);
        assert_eq!(Page::MultiPads.cycle(1), Page::Sections);
        assert_eq!(Page::Sections.cycle(-1), Page::MultiPads);
        for p in Page::ALL {
            assert_eq!(Page::from_u8(p.to_u8()), p);
        }
        assert_eq!(Page::from_u8(200), Page::MultiPads);
    }

    /// Every page lights all 16 pads in the same order, so the LED cache keyed by index
    /// updates exactly the pads that change on a page switch.
    #[test]
    fn every_page_covers_the_pads_in_order() {
        let has = [true; crate::engine::NUM_SLOTS];
        for page in Page::ALL {
            let panel = Panel { page, ..Panel::default() };
            let notes: Vec<u8> = looks(&snap(), &has, &panel).iter().map(|(n, _)| *n).collect();
            assert_eq!(notes, PADS, "{page:?}");
            let notes: Vec<u8> = pad_leds(&snap(), &has, &panel).iter().map(|(n, _)| *n).collect();
            assert_eq!(notes, PADS, "{page:?}");
        }
    }

    #[test]
    fn page_2_leds() {
        let has = [true; crate::engine::NUM_SLOTS];
        let mut s = snap();
        let mut panel = Panel { page: Page::ChordSetup, fingering: Fingering::MultiFinger, ..Panel::default() };
        let lk = |s: &Snapshot, p: &Panel| looks(s, &has, p).map(|(_, l)| l);
        let leds = |s: &Snapshot, p: &Panel| pad_leds(s, &has, p).map(|(_, l)| l);

        let l = lk(&s, &panel);
        assert!(l.iter().all(|l| l.rgb == C_PAGE_CHORD), "one colour for the page");
        let bright: Vec<usize> = (0..7).filter(|&i| l[i].level == Level::Bright).collect();
        assert_eq!(bright, vec![3], "only the selected fingering type is lit");
        assert_eq!(l[7].level, Level::Dim, "Lower");
        assert_eq!(l[8].level, Level::Off, "Manual Bass is unavailable in Lower");
        assert_eq!(l[15].level, Level::Dim, "Retrigger off");
        assert_eq!(leds(&s, &panel)[3], Led::Solid(CYAN));
        assert_eq!(leds(&s, &panel)[0], Led::Solid(DIM_CYAN));
        assert_eq!(leds(&s, &panel)[15], Led::Solid(DIM_CYAN));

        panel.upper = true;
        s.stop_acmp = true;
        s.transpose = Transpose::new(-2, 0);
        s.retrigger = true;
        let l = lk(&s, &panel);
        assert_eq!(l[15].level, Level::Bright, "Retrigger on");
        assert_eq!(l[7].level, Level::Bright, "Upper");
        assert_eq!(l[8].level, Level::Bright, "Manual Bass on");
        assert_eq!(l[9].level, Level::Bright, "Stop ACMP");
        assert_eq!((l[12].level, l[13].level, l[14].level), (Level::Bright, Level::Dim, Level::Bright));
        panel.manual_bass = false;
        assert_eq!(lk(&s, &panel)[8].level, Level::Dim);
    }

    #[test]
    fn page_3_leds() {
        let has = [true; crate::engine::NUM_SLOTS];
        let mut s = snap();
        let mut panel = Panel { page: Page::OtsParts, ..Panel::default() };
        let lk = |s: &Snapshot, p: &Panel| looks(s, &has, p).map(|(_, l)| l);

        // A style without OTS: those pads are dark. Right 1 is on and selected.
        let l = lk(&s, &panel);
        assert!(l.iter().all(|l| l.rgb == C_PAGE_OTS), "one colour for the page");
        assert!(l[..4].iter().all(|l| l.level == Level::Off));
        assert_eq!(l[5].level, Level::Dim, "Fade off");
        s.fade = FadeState::Armed;
        assert_eq!(lk(&s, &panel)[5].level, Level::Bright, "Fade armed");
        s.fade = FadeState::Off;
        assert_eq!(l[8..12].iter().map(|l| l.level).collect::<Vec<_>>(), [Level::Bright, Level::Dim, Level::Dim, Level::Dim]);
        assert_eq!(l[12..].iter().map(|l| l.level).collect::<Vec<_>>(), [Level::Bright, Level::Dim, Level::Dim, Level::Dim]);

        panel.ots_count = 3;
        panel.ots_applied = 2;
        panel.ots_link = true;
        panel.parts_on = 0b1011;
        panel.selected = 3;
        let l = lk(&s, &panel);
        assert_eq!(l[..4].iter().map(|l| l.level).collect::<Vec<_>>(), [Level::Dim, Level::Bright, Level::Dim, Level::Off]);
        assert_eq!(l[4].level, Level::Bright, "Link on");
        assert_eq!(l[8..12].iter().map(|l| l.level).collect::<Vec<_>>(), [Level::Bright, Level::Bright, Level::Dim, Level::Bright]);
        assert_eq!(l[15].level, Level::Bright, "Left selected");
        let leds = pad_leds(&s, &has, &panel).map(|(_, l)| l);
        assert_eq!((leds[1], leds[0], leds[3]), (Led::Solid(PINK), Led::Solid(DIM_PINK), Led::Solid(OFF)));
    }

    #[test]
    fn fader_buttons_follow_the_page() {
        let mut out = Vec::new();
        fader_button_msgs(FaderPage::Panel, 0b1001, 0xFF, PanelLamps::default(), &mut out);
        assert_eq!(out[..4], [[0xB0, 37, BLUE], [0xB0, 38, DIM_BLUE], [0xB0, 39, DIM_BLUE], [0xB0, 40, BLUE]]);
        assert_eq!(out[4], [0xB0, 41, DIM_PURPLE], "button 5: HARMONY/ARPEGGIO off");
        assert_eq!([out[5][2], out[7][2]], [OFF, OFF], "button 6 dark (no plugin to reload), 8 no loop");
        assert_eq!(out[6], [0xB0, 43, DIM_ORANGE], "button 7: Left Hold off");
        assert_eq!(out[8], [0xB0, 45, BLUE]);
        out.clear();
        let lit = PanelLamps { harmony_arp: true, plugin_fault: true, left_hold: true, looper: LooperLamp::Looping };
        fader_button_msgs(FaderPage::Panel, 0b1001, 0xFF, lit, &mut out);
        assert_eq!(out[4], [0xB0, 41, PURPLE], "button 5: HARMONY/ARPEGGIO on");
        assert_eq!(out[5], [0xB0, 42, RED], "button 6: the selected part's plugin needs a reload");
        assert_eq!(out[6], [0xB0, 43, ORANGE], "button 7: Left Hold on");
        assert_eq!(out[7], [0xB0, 44, GREEN], "button 8: the Chord Looper loops");
        out.clear();
        fader_button_msgs(FaderPage::Panel, 0, 0, PanelLamps { looper: LooperLamp::Recording, ..lit }, &mut out);
        assert_eq!(out[7], [0xB0, 44, RED], "button 8: recording");
        out.clear();
        // The Style page's button 5 is the Style's fifth part, whatever the switch.
        fader_button_msgs(FaderPage::Style, 0b1001, !(1 << 5), lit, &mut out);
        assert_eq!(out[5], [0xB0, 42, DIM_GREEN], "Pad muted");
        assert!(out.iter().enumerate().all(|(i, m)| i == 5 || m[2] == GREEN));
    }

    #[test]
    fn page_1_leds_unchanged_by_panel() {
        let has = [true; crate::engine::NUM_SLOTS];
        let s = Snapshot { running: true, cur: Some(SectionId::Main(1)), main: 1, auto_fill: true, ..snap() };
        let a = Panel::default();
        let b = Panel { upper: true, ots_count: 4, ots_applied: 1, parts_on: 0b1111, ..Panel::default() };
        assert_eq!(pad_leds(&s, &has, &a), pad_leds(&s, &has, &b));
        assert_eq!(looks(&s, &has, &a), looks(&s, &has, &b));
        assert_eq!(pad_leds(&s, &has, &a)[9], (113, Led::Solid(GREEN)));
    }

    #[test]
    fn nav_buttons_show_where_you_can_go() {
        let mut out = Vec::new();
        nav_button_msgs(Page::Sections, true, &mut out);
        assert!(out.contains(&[0xB0, PAD_UP_CC, OFF]) && out.contains(&[0xB0, PAD_DOWN_CC, WHITE]));
        assert!(out.contains(&[0xB0, TRACK_LEFT_CC, WHITE]) && out.contains(&[0xB0, TRACK_RIGHT_CC, WHITE]));
        out.clear();
        nav_button_msgs(Page::ChordSetup, false, &mut out);
        assert!(out.contains(&[0xB0, PAD_UP_CC, CYAN]) && out.contains(&[0xB0, PAD_DOWN_CC, CYAN]));
        assert!(out.contains(&[0xB0, TRACK_LEFT_CC, OFF]));
        out.clear();
        nav_button_msgs(Page::OtsParts, true, &mut out);
        assert!(out.contains(&[0xB0, PAD_UP_CC, PINK]) && out.contains(&[0xB0, PAD_DOWN_CC, PINK]));
        out.clear();
        nav_button_msgs(Page::Registration, true, &mut out);
        assert!(out.contains(&[0xB0, PAD_UP_CC, ORANGE]) && out.contains(&[0xB0, PAD_DOWN_CC, ORANGE]));
        out.clear();
        nav_button_msgs(Page::MultiPads, true, &mut out);
        assert!(out.contains(&[0xB0, PAD_UP_CC, YELLOW]) && out.contains(&[0xB0, PAD_DOWN_CC, OFF]));
        out.clear();
        buttons_off_msgs(&mut out);
        for cc in [PAD_UP_CC, PAD_DOWN_CC, TRACK_LEFT_CC, TRACK_RIGHT_CC, 37, 45] {
            assert!(out.contains(&[0xB0, cc, OFF]) && out.contains(&[0xB3, cc, 0]), "{cc}");
        }
    }
}

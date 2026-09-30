//! Novation Launchkey MK4 in DAW mode: pads and buttons as arranger controls, with LEDs.
//!
//! The 16 pads have five pages, switched with the Pad Bank ▲/▼ buttons left of the pads
//! (DAW port, channel 1 notes; top row 96..103, bottom row 112..119). Page 1 is fixed; pages
//! 2-5 come in the player's order (`PageOrder`, set in Settings; the default below). The
//! per-page tables live in `launchkey/pages/`.
//!
//!   1 Sections     Intro I  Intro II  Intro III  SyncStart | Ending I  Ending II  Ending III  AutoFill
//!                  Main A   Main B    Main C     Main D    | Break     Tap       SyncStop    Start/Stop
//!   2 Racks        Quick 1  Quick 2   Quick 3    Quick 4   | Quick 5   Quick 6   Quick 7     Quick 8
//!                  OTS 1    OTS 2     OTS 3      OTS 4     | Bank -    Bank +    Store       -
//!   3 Chord        -        -         -          -         | -         -         -           -
//!                  ManBass  StopAcmp  Split -    Split +   | Kbd tr -  Kbd tr +  Tr reset    Retrigger
//!   4 Multi Pads   Pad 1    Pad 2     Pad 3      Pad 4     | STOP      -         -           -
//!                  Select 1 Select 2  Select 3   Select 4  | Stop 1    Stop 2    Stop 3      Stop 4
//!   5 Setup        Single   Fingered  On Bass    Multi     | AI Fing.  Full Kbd  AI Full     Upper
//!                  OTS Link Acmp Style Acmp Fixed -        | -         -         -           -
//!
//! Holding Sound (fader button 6) shows the Racks page on the pads from any page
//! (`Layer::Sound`); release goes back.
//!
//! Buttons (CC in DAW mode; numbers from the MK4 Programmer's Reference Guide v3.0, p.9,
//! Figure 3): 115 Play = Start/Stop, 116 Stop, 104 (Scene Launch >) / 105 (Function) =
//! tempo +/-, 106/107 (Pad Bank ▲/▼) = page up/down, Shift + ▲/▼ = Left on/off / OTS Link,
//! 103/102 (< Track / Track >) = previous/next style (Shift: previous/next Quick Rack),
//! 63 = Shift. Shift + Play = Style Section Reset, Shift + Stop = Fade In/Out, Shift +
//! Scene Launch / Function = Retrigger length shorter / longer.
//!
//! The 8 encoders are knobs on Knob Assign pages (`knobs.rs`), stepped with the encoder
//! page buttons ▲/▼ (CC 51/52) right of them, like the Genos KNOB ASSIGN button. yahaha
//! turns the encoders' relative output on when it enters DAW mode. Shift + ▼ is [ACMP];
//! Shift + ▲ is Organ Rotary Slow/Fast, and ▲ is lit while the rotary is fast.
//!
//! Faders have two pages, like the Genos Mixer's Panel and Style tabs; the button under
//! the master fader switches them (see `parts`). Panel: faders 1-4 = Right 1, Right 2,
//! Right 3, Left volumes, their buttons = part on/off on a tap (hold + turn a knob: swap
//! mode, `Layer::Swap`; Shift: select the part), button 5 HARMONY/ARPEGGIO, 6 SOUND (hold),
//! 7 LEFT HOLD, 8 CHORD LOOPER ON/OFF (Shift: REC/STOP). Style: faders 1-8 = the Style
//! parts, their buttons = part mute, except button 6, which is SOUND on both pages (Shift +
//! 6 mutes part 6, Pad). Master is always master.

use crate::engine::{Button, PadCmd, Snapshot};
use yahaha_core::fingering::Fingering;
use crate::parts::{self, FaderLayer, FaderPage};
use yahaha_sff::sff::SectionId;

/// The per-page pad tables: each page's pad actions, palette LEDs and looks.
mod pages {
    pub mod chord;
    pub mod multipads;
    pub mod racks;
    pub mod sections;
    pub mod setup;
}

pub use pages::sections::pad_button;

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

/// Pad pages. Page 1 (Sections) is fixed; the order of pages 2-5 is the player's
/// (`PageOrder`).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Page {
    #[default]
    Sections,
    /// Quick Racks 1-8 of the bank on view, OTS 1-4, bank -/+ and Store (docs/racks.md).
    /// It took over the OTS/Parts and Quick Racks pages, and before them the Snapshots
    /// page (`otsParts`, `quickRacks` and `registration`, still read).
    #[serde(alias = "otsParts", alias = "quickRacks", alias = "registration")]
    Racks,
    /// The mid-song chord switches (`chordSetup`, the Chord/Setup page, still read).
    #[serde(alias = "chordSetup")]
    Chord,
    /// Multi Pads 1-4, STOP, SELECT + pad (Synchro Start) and STOP + pad (#196).
    MultiPads,
    /// The set-and-forget switches: fingering type, Upper, OTS Link, Stop ACMP mode.
    Setup,
}

impl Page {
    pub const ALL: [Page; 5] = [Page::Sections, Page::Racks, Page::Chord, Page::MultiPads, Page::Setup];

    pub fn name(self) -> &'static str {
        match self {
            Page::Sections => "Sections",
            Page::Racks => "Racks",
            Page::Chord => "Chord",
            Page::MultiPads => "Multi Pads",
            Page::Setup => "Setup",
        }
    }

    pub fn to_u8(self) -> u8 {
        self as u8
    }

    /// The page with discriminant `v`; out of range reads as the last page.
    pub fn from_u8(v: u8) -> Page {
        Page::ALL[(v as usize).min(Page::ALL.len() - 1)]
    }

    /// The page's LED identity: RGB for the pads, plus bright and dim palette colours.
    /// Page 1 keeps its per-section pad colours; white is its Pad Bank button colour.
    pub fn colour(self) -> ((u8, u8, u8), u8, u8) {
        match self {
            Page::Sections => (C_TAP, WHITE, DIM_WHITE),
            Page::Racks => (C_PAGE_RACKS, ORANGE, DIM_ORANGE),
            Page::Chord => (C_PAGE_CHORD, CYAN, DIM_CYAN),
            Page::MultiPads => (C_PAGE_PADS, YELLOW, DIM_YELLOW),
            Page::Setup => (C_PAGE_SETUP, PINK, DIM_PINK),
        }
    }
}

/// The order of pad pages 2-5: Sections always first, then up to four of the others, each
/// at most once (a page left out isn't on the pads, bar Racks under a Sound hold). Packed
/// in a u16 for an atomic: the count in bits 0-2, then each page's discriminant in 3 bits.
/// Everything here is total and allocation-free: the input thread walks it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PageOrder(u16);

impl Default for PageOrder {
    fn default() -> PageOrder {
        PageOrder::DEFAULT
    }
}

impl PageOrder {
    /// Racks, Chord, Multi Pads, Setup.
    pub const DEFAULT: PageOrder = PageOrder(4 | 1 << 3 | 2 << 6 | 3 << 9 | 4 << 12);

    const MOVABLE: usize = Page::ALL.len() - 1;

    /// Pages 2.. in this order. None when it names Sections, names a page twice, or names
    /// more than four pages. Fewer pages leave the rest out.
    pub fn new(pages: &[Page]) -> Option<PageOrder> {
        if pages.len() > Self::MOVABLE {
            return None;
        }
        let (mut bits, mut seen) = (pages.len() as u16, 0u8);
        for (i, &p) in pages.iter().enumerate() {
            let v = p.to_u8();
            if p == Page::Sections || seen & (1 << v) != 0 {
                return None;
            }
            seen |= 1 << v;
            bits |= (v as u16) << (3 + 3 * i);
        }
        Some(PageOrder(bits))
    }

    fn count(self) -> usize {
        (self.0 & 7) as usize
    }

    fn slot(self, i: usize) -> Page {
        Page::from_u8(((self.0 >> (3 + 3 * i)) & 7) as u8)
    }

    /// The page at position `i` (0 = Sections); `i` is below `len()`.
    fn at(self, i: usize) -> Page {
        if i == 0 { Page::Sections } else { self.slot(i - 1) }
    }

    /// Pages 2.. in order (no Sections).
    pub fn movable(self) -> impl Iterator<Item = Page> {
        (0..self.count()).map(move |i| self.slot(i))
    }

    /// Sections, then `movable()`.
    pub fn pages(self) -> impl Iterator<Item = Page> {
        std::iter::once(Page::Sections).chain(self.movable())
    }

    /// The number of pages, Sections included (1..=5; never empty).
    #[allow(clippy::len_without_is_empty)]
    pub fn len(self) -> usize {
        1 + self.count()
    }

    /// The page's position (0 = Sections); None when it's left out.
    pub fn position(self, page: Page) -> Option<usize> {
        if page == Page::Sections {
            return Some(0);
        }
        self.movable().position(|p| p == page).map(|i| i + 1)
    }

    pub fn contains(self, page: Page) -> bool {
        self.position(page).is_some()
    }

    /// The page `d` steps away, stopping at the first and last (Pad Bank ▲/▼). From a page
    /// left out, it steps from Sections.
    pub fn step(self, page: Page, d: i8) -> Page {
        let pos = self.position(page).unwrap_or(0) as i16;
        self.at((pos + d as i16).clamp(0, self.len() as i16 - 1) as usize)
    }

    /// The page `d` steps away, wrapping around (the terminal's Tab / Shift+Tab). From a
    /// page left out, it steps from Sections.
    pub fn cycle(self, page: Page, d: i8) -> Page {
        let pos = self.position(page).unwrap_or(0) as i16;
        self.at((pos + d as i16).rem_euclid(self.len() as i16) as usize)
    }

    pub fn to_bits(self) -> u16 {
        self.0
    }

    /// The order `to_bits` packed; anything else (a count over four, Sections, a page out
    /// of range or twice, stray bits) reads as `DEFAULT`.
    pub fn from_bits(b: u16) -> PageOrder {
        let n = (b & 7) as usize;
        if n > Self::MOVABLE || b >> (3 + 3 * n) != 0 {
            return PageOrder::DEFAULT;
        }
        let mut seen = 0u8;
        for i in 0..n {
            let v = (b >> (3 + 3 * i)) & 7;
            if v == 0 || v as usize >= Page::ALL.len() || seen & (1 << v) != 0 {
                return PageOrder::DEFAULT;
            }
            seen |= 1 << v;
        }
        PageOrder(b)
    }
}

/// A held control's layer (docs/eyes-free.md): Sound held (the pads show the Racks page),
/// or a Panel fader button 1-4 held with a knob turned (swap mode for that part).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Layer {
    #[default]
    None,
    Sound,
    Swap { part: u8 },
}

impl Layer {
    /// Packed for an atomic: None 0, Sound 1, Swap 2 + part.
    pub fn to_u8(self) -> u8 {
        match self {
            Layer::None => 0,
            Layer::Sound => 1,
            Layer::Swap { part } => 2u8.saturating_add(part),
        }
    }

    pub fn from_u8(v: u8) -> Layer {
        match v {
            0 => Layer::None,
            1 => Layer::Sound,
            v => Layer::Swap { part: v - 2 },
        }
    }

    /// The page the pads show: Racks while Sound is held, otherwise `page`.
    pub fn pads(self, page: Page) -> Page {
        if self == Layer::Sound { Page::Racks } else { page }
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
    /// Quick Rack 1-8 of the bank on view (0-based; `Q`-`I` with Shift): load its rack, or
    /// store the live rack on it while Store is armed.
    QuickRack(u8),
    /// Quick Racks bank -/+ (`O` `P` with Shift).
    QuickRackBank(i8),
    /// Quick Racks STORE (`F5`): the next Quick Rack button stores.
    QuickRackStore,
    /// Previous/next rack in the bank on view (`F7` `F8`; Shift + Track < / >).
    QuickRackStep(i8),
    /// A pedal's assignable function that the control side runs (`controllers.rs`).
    Assign(crate::controllers::Function),
    /// A Hold A / Hold B pedal sets a control-side switch on or off (`controllers::Fire::set`).
    AssignSet(crate::controllers::Function, bool),
    /// The HARMONY/ARPEGGIO button: the selected Harmony type or arpeggio on/off (`J`,
    /// fader button 5 on the Panel fader page).
    ToggleHarmonyArp,
    /// Load the selected part's plugin again after it stopped or failed to load (`s`, and
    /// the app; fader button 6 is Sound now).
    ReloadPlugin,
    /// A Multi Pad button (page 5; `Z X C V`, `B`): straight to the engine, as a section
    /// pad, so a press is not held up by the control side.
    MultiPad(PadCmd),
    /// An encoder turned: knob 0-7, steps (positive: clockwise).
    Knob(u8, i8),
    /// The Knob Assign page up (-1) / down (+1): the encoder page buttons.
    KnobPage(i8),
    /// Panel fader 1-4 (0-3) moved to a value (0-127) in the Volume layer, where the live
    /// rack's controller map gives it something other than its own part's level.
    RackFader(u8, u8),
    /// Swap mode: step keyboard part `part`'s sound by `step` numbers (PartsCmd::SwapSound).
    SwapSound { part: u8, step: i8 },
    /// Store the live rack on Quick Rack `slot` (0-7) of the bank on view
    /// (QuickRackCmd::StoreRack).
    StoreRack(u8),
}

/// What a pad does on a page under a layer: the pads of `layer.pads(page)`.
pub fn pad_action(page: Page, layer: Layer, note: u8) -> Option<Action> {
    match layer.pads(page) {
        Page::Sections => pages::sections::pad_action(note),
        Page::Racks => pages::racks::pad_action(note),
        Page::Chord => pages::chord::pad_action(note),
        Page::MultiPads => pages::multipads::pad_action(note),
        Page::Setup => pages::setup::pad_action(note),
    }
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
        // Shift + Track < / >: the previous/next Quick Rack in the bank (it was the
        // Playlist's previous/next song).
        TRACK_LEFT_CC if shift => act(Action::QuickRackStep(-1)),
        TRACK_RIGHT_CC if shift => act(Action::QuickRackStep(1)),
        TRACK_LEFT_CC => act(Action::Style(-1)),
        TRACK_RIGHT_CC => act(Action::Style(1)),
        // Shift + Pad Bank ▲/▼: the toggles these buttons had before pages (OTS Link is on
        // the Setup page too).
        PAD_UP_CC if shift => act(Action::PartOnOff(parts::LEFT as u8)),
        PAD_DOWN_CC if shift => act(Action::ToggleOtsLink),
        // Shift + encoder page ▼: [ACMP] on/off (#266). Every pad is taken.
        KNOB_DOWN_CC if shift => act(Action::Button(Button::Acmp)),
        // Shift + encoder page ▲: Organ Rotary Slow/Fast (Genos RM p.140), as a pedal set
        // to it runs it (`controllers::Function::RotaryFast`).
        KNOB_UP_CC if shift => act(Action::Assign(crate::controllers::Function::RotaryFast)),
        KNOB_UP_CC => act(Action::KnobPage(-1)),
        KNOB_DOWN_CC => act(Action::KnobPage(1)),
        PAD_UP_CC => Some(Control::Page(-1)),
        PAD_DOWN_CC => Some(Control::Page(1)),
        _ => None,
    }
}

/// Button lights (palette colour on ch 1, plus brightness on ch 4 in case they're
/// single-colour LEDs): Pad Bank ▲/▼ in the page's colour where `order` has a page to go
/// to, Track < / > when there is another style to go to.
pub fn nav_button_msgs(page: Page, order: PageOrder, styles: bool, out: &mut Vec<[u8; 3]>) {
    let (_, c, _) = page.colour();
    let up = order.step(page, -1) != page;
    let down = order.step(page, 1) != page;
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

/// The encoder page ▲ light: lit while the rotary is fast (Shift + ▲ switches it; the
/// Genos has no lamp for it, the Launchkey has one to spare). ▼ stays dark.
pub fn knob_button_msgs(rotary_fast: bool, out: &mut Vec<[u8; 3]>) {
    out.push([0xB0, KNOB_UP_CC, if rotary_fast { WHITE } else { OFF }]);
    out.push([0xB3, KNOB_UP_CC, if rotary_fast { 127 } else { 0 }]);
}

/// On exit: the Pad Bank, Track, encoder page and fader button lights off.
pub fn buttons_off_msgs(out: &mut Vec<[u8; 3]>) {
    for cc in [PAD_UP_CC, PAD_DOWN_CC, TRACK_LEFT_CC, TRACK_RIGHT_CC, KNOB_UP_CC].into_iter().chain(FADER_BTN_CC) {
        out.push([0xB0, cc, OFF]);
        out.push([0xB3, cc, 0]);
    }
}

/// The fader button (0-based, under fader 5) that is the HARMONY/ARPEGGIO switch on the
/// Panel fader page. Every pad on every page is taken; Panel buttons 5-8 were dark.
pub const HARM_ARP_FADER_BTN: u8 = 4;

/// The fader button (0-based, under fader 6) that is SOUND on both fader pages: hold it
/// and the pads show the Racks page (`Layer::Sound`). Dim white, white while held. On the
/// Style page Shift + it mutes Style part 6 (Pad).
pub const SOUND_FADER_BTN: u8 = 5;

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

/// What the Panel page's own fader buttons (5-8) show, and Sound (button 6 on both pages).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PanelLamps {
    /// The HARMONY/ARPEGGIO switch (button 5).
    pub harmony_arp: bool,
    /// Sound is held (button 6 white).
    pub sound: bool,
    /// Left Hold (button 7).
    pub left_hold: bool,
    /// The Chord Looper (button 8).
    pub looper: LooperLamp,
}

/// The Panel fader page's colour (bright, dim) in each fader layer, so the player sees
/// which layer the faders control: VOL blue, PAN yellow, REV cyan, CHO pink, DLY white.
pub fn layer_colour(layer: FaderLayer) -> (u8, u8) {
    match layer {
        FaderLayer::Volume => (BLUE, DIM_BLUE),
        FaderLayer::Pan => (YELLOW, DIM_YELLOW),
        FaderLayer::Reverb => (CYAN, DIM_CYAN),
        FaderLayer::Chorus => (PINK, DIM_PINK),
        FaderLayer::Delay => (WHITE, DIM_WHITE),
    }
}

/// Palette colours for the fader buttons. Panel page (the fader layer's colour,
/// `layer_colour`): Right 1-3 and Left lit while on (`parts_on`, bit = part), button 5
/// (purple) lit while HARMONY/ARPEGGIO is on, button 7 (orange) lit while Left Hold is on,
/// button 8 the Chord Looper (`LooperLamp`). Style page (green, whatever the layer): the
/// Style parts lit while they play (`style_on`). On both pages button 6 is Sound: dim
/// white, white while held. The master button shows the page's colour.
pub fn fader_button_msgs(page: FaderPage, layer: FaderLayer, parts_on: u8, style_on: u8, lamps: PanelLamps, out: &mut Vec<[u8; 3]>) {
    let (on, n, (bright, dim)) = match page {
        FaderPage::Panel => (parts_on, parts::COUNT as u8, layer_colour(layer)),
        FaderPage::Style => (style_on, 8, (GREEN, DIM_GREEN)),
    };
    for i in 0..8u8 {
        let c = if i == SOUND_FADER_BTN {
            if lamps.sound { WHITE } else { DIM_WHITE }
        } else if page == FaderPage::Panel && i == HARM_ARP_FADER_BTN {
            if lamps.harmony_arp { PURPLE } else { DIM_PURPLE }
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
#[allow(clippy::too_many_arguments)]
pub fn button_colours(page: Page, order: PageOrder, styles: bool, fader_page: FaderPage, layer: FaderLayer, parts_on: u8, style_on: u8, lamps: PanelLamps) -> Vec<(u8, u8)> {
    let mut msgs = Vec::new();
    nav_button_msgs(page, order, styles, &mut msgs);
    fader_button_msgs(fader_page, layer, parts_on, style_on, lamps, &mut msgs);
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

/// Panel state outside the engine snapshot that the pages and buttons show.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Panel {
    /// The page on view; the pads show `shown()`.
    pub page: Page,
    /// The held control's layer (Sound: the pads show Racks).
    pub layer: Layer,
    /// The pad page order (Pad Bank ▲/▼ lights).
    pub order: PageOrder,
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
    /// Left Hold (fader button 7).
    pub left_hold: bool,
    /// The Chord Looper (fader button 8).
    pub looper: LooperLamp,
    /// Keyboard parts that are on (bit = `parts::RIGHT1`..`LEFT`), and the selected one.
    pub parts_on: u8,
    pub selected: u8,
    /// Quick Racks, for the Racks page.
    pub quick: QuickPanel,
    /// Organ Rotary Slow/Fast is at fast (the encoder page ▲ light).
    pub rotary_fast: bool,
}

/// The Quick Racks bank on view, as the Racks page shows it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct QuickPanel {
    /// Its buttons that hold a rack (bit 0 = Quick Rack 1).
    pub stored: u8,
    /// Its buttons whose rack is the live rack's: lit.
    pub loaded: u8,
    /// The bank on view (0 = A).
    pub bank: u8,
    /// Store armed.
    pub store: bool,
}

impl Panel {
    /// The page the pads show: `layer.pads(page)`.
    pub fn shown(&self) -> Page {
        self.layer.pads(self.page)
    }

    /// What the Panel page's own fader buttons, and Sound, show.
    pub fn lamps(&self) -> PanelLamps {
        PanelLamps { harmony_arp: self.harmony_arp, sound: self.layer == Layer::Sound, left_hold: self.left_hold, looper: self.looper }
    }
}

impl Default for Panel {
    fn default() -> Panel {
        Panel {
            page: Page::Sections,
            layer: Layer::None,
            order: PageOrder::DEFAULT,
            fingering: Fingering::FingeredOnBass,
            upper: false,
            manual_bass: true,
            ots_count: 0,
            ots_applied: 0,
            ots_link: false,
            harmony_arp: false,
            left_hold: false,
            looper: LooperLamp::Empty,
            parts_on: 1 << parts::RIGHT1,
            selected: parts::RIGHT1 as u8,
            quick: QuickPanel::default(),
            rotary_fast: false,
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

/// Desired LED state for all 16 pads (palette mode) on the page the pads show
/// (`Panel::shown`).
pub fn pad_leds(s: &Snapshot, has: &[bool], panel: &Panel) -> [(u8, Led); 16] {
    match panel.shown() {
        Page::Sections => pages::sections::leds(s, has),
        Page::Racks => pages::racks::leds(panel),
        Page::Chord => pages::chord::leds(s, panel),
        Page::MultiPads => pages::multipads::leds(s),
        Page::Setup => pages::setup::leds(s, panel),
    }
}

/// A look as a palette LED in the given bright and dim colours.
fn look_led(look: &Look, bright: u8, dim: u8) -> Led {
    match (look.level, look.anim) {
        (Level::Off, _) => Led::Solid(OFF),
        (Level::Dim, _) => Led::Solid(dim),
        (Level::Bright, Anim::Solid) => Led::Solid(bright),
        (Level::Bright, Anim::Flash) => Led::Flash(dim, bright),
        (Level::Bright, Anim::Pulse) => Led::Pulse(bright),
    }
}

/// A page's looks as palette LEDs in the page's colour.
fn page_leds(page: Page, looks: [(u8, Look); 16]) -> [(u8, Led); 16] {
    let (_, bright, dim) = page.colour();
    looks.map(|(note, look)| (note, look_led(&look, bright, dim)))
}

/// The Main (0-3) a fill or the Break, queued or playing, lands on (#282): the Main
/// selected. None when no fill is queued or playing.
pub fn landing(s: &Snapshot) -> Option<u8> {
    let fill_like = |x: Option<SectionId>| matches!(x, Some(SectionId::Fill(_) | SectionId::Break));
    (s.running && (fill_like(s.queued) || fill_like(s.cur))).then_some(s.main)
}

/// Quick Racks banks: A-H.
pub const QUICK_BANKS: u8 = 8;

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
/// Page identities: every pad on the Chord page is cyan, every pad on the Setup page
/// magenta.
pub const C_PAGE_CHORD: (u8, u8, u8) = (0, 100, 127);
pub const C_PAGE_SETUP: (u8, u8, u8) = (127, 0, 70);
/// The Racks page: orange, with the Quick Rack buttons in the Genos Registration lamp
/// colours.
pub const C_PAGE_RACKS: (u8, u8, u8) = (127, 60, 0);
/// Quick Rack lamps: red = the loaded rack, blue = a rack (the Registration lamps, OM p.97).
pub const C_QUICK_LOADED: (u8, u8, u8) = (127, 0, 0);
pub const C_QUICK_STORED: (u8, u8, u8) = (0, 40, 127);
/// The Multi Pads page: yellow, with the Multi Pads in the Genos lamp colours (blue = data,
/// red = playing; OM p.75) and amber while a press waits for the bar line.
pub const C_PAGE_PADS: (u8, u8, u8) = (127, 127, 0);
pub const C_PAD_READY: (u8, u8, u8) = (0, 40, 127);
pub const C_PAD_PLAYING: (u8, u8, u8) = (127, 0, 0);
pub const C_PAD_QUEUED: (u8, u8, u8) = (127, 60, 0);

/// Brightness of "dim" relative to full.
const DIM: f32 = 0.18;

/// Pad looks for the current page.
pub fn looks(s: &Snapshot, has: &[bool], panel: &Panel) -> [(u8, Look); 16] {
    match panel.shown() {
        Page::Sections => pages::sections::looks(s, has),
        Page::Racks => pages::racks::looks(panel),
        Page::Chord => pages::chord::looks(s, panel),
        Page::MultiPads => pages::multipads::looks(s),
        Page::Setup => pages::setup::looks(s, panel),
    }
}

/// The Racks page's pads, from the panel alone (the app's dev mock builds its Racks page
/// from this too).
pub fn racks_looks(p: &Panel) -> [(u8, Look); 16] {
    pages::racks::looks(p)
}

/// A page's pad in the page's colour: bright when on, dim when off, dark when unavailable.
fn page_look(page: Page, label: &'static str, key: &'static str, available: bool, on: bool) -> Look {
    let level = match (available, on) {
        (false, _) => Level::Off,
        (true, true) => Level::Bright,
        (true, false) => Level::Dim,
    };
    Look { label, key, rgb: page.colour().0, level, anim: Anim::Solid }
}

/// The keyboard parts' names (Right 1-3, Left), for the display and the app.
pub const PART_LABELS: [&str; 4] = ["RIGHT 1", "RIGHT 2", "RIGHT 3", "LEFT"];
pub const SELECT_LABELS: [&str; 4] = ["EDIT R1", "EDIT R2", "EDIT R3", "EDIT L"];

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
    use crate::engine::{StopAcmp, Transpose};
    use crate::multipad::PadState;

    fn snap() -> Snapshot {
        Snapshot {
            running: false, sync_armed: false, sync_stop: false, auto_fill: false, cur: None, queued: None,
            pending_intro: None, main: 0, bar: 0, beat: 0, chord: None, bpm: 120.0, parts: 0xFF, volumes: [100; 8], user_set: 0, pickup: 0, send_pickup: 0,
            stop_acmp: false, stop_acmp_mode: crate::engine::StopAcmp::Off, half_bar_fill: false, main_presses: 0, transpose: Transpose::default(), played: None, anchor_ns: 0, anchor_beats: 0.0, style_tag: 0,
            style_pending: false, section_bars: 0, audition: None, fade: crate::engine::FadeState::Off, retrigger: false, ritardando: false,
            looper: Default::default(), style_solo: None,
            multipad: Default::default(), chart_tag: 0, chart_bar: None, chart_override: false, dynamics: 64,
            style_sends: [[40, 0, 0]; 8], style_send_own: [[255; 3]; 8], acmp: true, unison: false, unison_latched: false, unison_type: Default::default(),
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
        let b = button_colours(Page::Sections, PageOrder::DEFAULT, true, FaderPage::Panel, FaderLayer::Volume, 0b0001, 0xFF, PanelLamps::default());
        assert!(b.contains(&(PAD_UP_CC, OFF)) && b.contains(&(PAD_DOWN_CC, WHITE)));
        assert!(b.contains(&(TRACK_LEFT_CC, WHITE)));
        assert!(b.contains(&(37, BLUE)) && b.contains(&(38, DIM_BLUE)) && b.contains(&(41, DIM_PURPLE)) && b.contains(&(42, DIM_WHITE)) && b.contains(&(45, BLUE)));
        assert_eq!(b.len(), 4 + 9);
    }

    /// Each fader layer lights the Panel page's part buttons and master button in its own
    /// colour, bright while the part is on and dim while off; the Style page stays green.
    #[test]
    fn fader_layers_have_their_own_colour() {
        let expect = [
            (FaderLayer::Volume, BLUE, DIM_BLUE),
            (FaderLayer::Pan, YELLOW, DIM_YELLOW),
            (FaderLayer::Reverb, CYAN, DIM_CYAN),
            (FaderLayer::Chorus, PINK, DIM_PINK),
            (FaderLayer::Delay, WHITE, DIM_WHITE),
        ];
        for (layer, bright, dim) in expect {
            let b = button_colours(Page::Sections, PageOrder::DEFAULT, true, FaderPage::Panel, layer, 0b0001, 0xFF, PanelLamps::default());
            assert!(b.contains(&(37, bright)) && b.contains(&(38, dim)) && b.contains(&(40, dim)) && b.contains(&(45, bright)), "{layer:?}");
            assert!(b.contains(&(41, DIM_PURPLE)), "{layer:?}: button 5 keeps its own colour");
            assert_ne!(palette_colour(bright).1, palette_colour(dim).1, "{layer:?}: on and off still read");
            let s = button_colours(Page::Sections, PageOrder::DEFAULT, true, FaderPage::Style, layer, 0, 0xFF, PanelLamps::default());
            assert!(s.contains(&(37, GREEN)) && s.contains(&(45, GREEN)), "{layer:?}: Style page");
        }
        let hues: std::collections::HashSet<_> = expect.iter().map(|e| palette_colour(e.1).0).collect();
        assert_eq!(hues.len(), 5, "every layer has a different hue");
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
            assert_eq!(pad_action(Page::Sections, Layer::None, n), pad_button(n).map(Action::Button), "note {n}");
        }
        assert_eq!(pad_action(Page::Sections, Layer::None, 96), Some(Action::Button(Button::Intro(0))));
        assert_eq!(pad_action(Page::Sections, Layer::None, 119), Some(Action::Button(Button::StartStop)));
    }

    fn act(page: Page, note: u8) -> Option<Action> {
        pad_action(page, Layer::None, note)
    }

    /// The Chord page: the top row is dark; the bottom row is the old Chord/Setup page's.
    #[test]
    fn chord_page_actions() {
        let p = Page::Chord;
        for n in 96..=103 {
            assert_eq!(act(p, n), None, "pad {n} is dark");
        }
        assert_eq!(act(p, 112), Some(Action::ToggleManualBass));
        assert_eq!(act(p, 113), Some(Action::Button(Button::StopAcmp)));
        assert_eq!(act(p, 114), Some(Action::Split(-1)));
        assert_eq!(act(p, 115), Some(Action::Split(1)));
        assert_eq!(act(p, 116), Some(Action::Transpose { keyboard: -1, master: 0 }));
        assert_eq!(act(p, 117), Some(Action::Transpose { keyboard: 1, master: 0 }));
        assert_eq!(act(p, 118), Some(Action::TransposeReset));
        assert_eq!(act(p, 119), Some(Action::Button(Button::Retrigger)));
        assert_eq!(act(p, 60), None);
    }

    /// The Setup page: fingering types 1-7 and Upper on top; OTS Link and the Stop ACMP
    /// mode below; the rest dark.
    #[test]
    fn setup_page_actions() {
        let p = Page::Setup;
        for (i, f) in Fingering::ALL.iter().enumerate() {
            assert_eq!(act(p, 96 + i as u8), Some(Action::Fingering(*f)));
        }
        assert_eq!(act(p, 96), Some(Action::Fingering(Fingering::SingleFinger)));
        assert_eq!(act(p, 102), Some(Action::Fingering(Fingering::AiFullKeyboard)));
        assert_eq!(act(p, 103), Some(Action::ToggleUpper));
        assert_eq!(act(p, 112), Some(Action::ToggleOtsLink));
        assert_eq!(act(p, 113), Some(Action::Button(Button::SetStopAcmp(StopAcmp::Style))));
        assert_eq!(act(p, 114), Some(Action::Button(Button::SetStopAcmp(StopAcmp::Fixed))));
        for n in 115..=119 {
            assert_eq!(act(p, n), None, "pad {n} is dark");
        }
    }

    /// The Racks page: Quick Racks 1-8 on top; OTS 1-4, bank -/+, Store and a spare below.
    /// Rack -/+ left the pads for Shift + Track < / >.
    #[test]
    fn racks_page_actions() {
        let p = Page::Racks;
        for i in 0..8 {
            assert_eq!(act(p, 96 + i), Some(Action::QuickRack(i)));
        }
        for n in 0..4 {
            assert_eq!(act(p, 112 + n), Some(Action::Ots(n)));
        }
        assert_eq!(act(p, 116), Some(Action::QuickRackBank(-1)));
        assert_eq!(act(p, 117), Some(Action::QuickRackBank(1)));
        assert_eq!(act(p, 118), Some(Action::QuickRackStore));
        assert_eq!(act(p, 119), None, "the spare is dark");
        assert_eq!(act(p, 104), None);
        assert_eq!(cc_control(TRACK_LEFT_CC, true), Some(Control::Act(Action::QuickRackStep(-1))));
        assert_eq!(cc_control(TRACK_RIGHT_CC, true), Some(Control::Act(Action::QuickRackStep(1))));
    }

    /// Holding Sound: the pads act as the Racks page's from every page. Swap mode leaves
    /// the page's own pads.
    #[test]
    fn sound_layer_pads_are_the_racks_page() {
        for page in Page::ALL {
            for n in 0..=127u8 {
                assert_eq!(pad_action(page, Layer::Sound, n), act(Page::Racks, n), "{page:?} note {n}");
                assert_eq!(pad_action(page, Layer::Swap { part: 1 }, n), act(page, n), "{page:?} note {n}");
            }
        }
    }

    /// The wire names, and the old names still read.
    #[test]
    fn page_wire_names() {
        let names = ["sections", "racks", "chord", "multiPads", "setup"];
        for (p, n) in Page::ALL.iter().zip(names) {
            assert_eq!(serde_json::to_string(p).unwrap(), format!("\"{n}\""));
            assert_eq!(serde_json::from_str::<Page>(&format!("\"{n}\"")).unwrap(), *p);
        }
        for (old, p) in [("otsParts", Page::Racks), ("quickRacks", Page::Racks), ("registration", Page::Racks), ("chordSetup", Page::Chord)] {
            assert_eq!(serde_json::from_str::<Page>(&format!("\"{old}\"")).unwrap(), p, "{old}");
        }
        assert_eq!(Page::ALL.map(Page::name), ["Sections", "Racks", "Chord", "Multi Pads", "Setup"]);
        for p in Page::ALL {
            assert_eq!(Page::from_u8(p.to_u8()), p);
        }
        assert_eq!(Page::from_u8(200), Page::Setup);
        assert_eq!(Page::ALL.map(|p| p.colour().1), [WHITE, ORANGE, CYAN, YELLOW, PINK]);
    }

    /// Page order: `new` refuses Sections, duplicates and more than four pages; fewer
    /// leave pages out; step stops at the ends, cycle wraps; bits round trip, garbage
    /// reads as the default.
    #[test]
    fn page_order() {
        use Page::*;
        let d = PageOrder::DEFAULT;
        assert_eq!(PageOrder::default(), d);
        assert_eq!(d.pages().collect::<Vec<_>>(), [Sections, Racks, Chord, MultiPads, Setup]);
        assert_eq!(d.movable().collect::<Vec<_>>(), [Racks, Chord, MultiPads, Setup]);
        assert_eq!(PageOrder::new(&[Racks, Chord, MultiPads, Setup]), Some(d));
        assert_eq!(d.len(), 5);

        assert_eq!(PageOrder::new(&[Sections, Racks]), None, "Sections is fixed first");
        assert_eq!(PageOrder::new(&[Racks, Chord, Racks]), None, "a duplicate");
        assert_eq!(PageOrder::new(&[Racks, Chord, MultiPads, Setup, Setup]), None, "too many");

        let o = PageOrder::new(&[Setup, Racks]).unwrap();
        assert_eq!(o.pages().collect::<Vec<_>>(), [Sections, Setup, Racks]);
        assert_eq!(o.len(), 3);
        assert_eq!((o.position(Sections), o.position(Setup), o.position(Racks), o.position(Chord)), (Some(0), Some(1), Some(2), None));
        assert!(o.contains(Setup) && !o.contains(MultiPads));
        assert_eq!(o.step(Sections, 1), Setup);
        assert_eq!(o.step(Setup, 1), Racks);
        assert_eq!(o.step(Racks, 1), Racks, "stops at the last");
        assert_eq!(o.step(Sections, -1), Sections, "stops at the first");
        assert_eq!(o.step(Racks, 100), Racks);
        assert_eq!(o.step(Racks, -128), Sections);
        assert_eq!(o.step(Chord, 1), Setup, "a page left out steps from Sections");
        assert_eq!(o.step(Chord, -1), Sections);
        assert_eq!(o.cycle(Racks, 1), Sections);
        assert_eq!(o.cycle(Sections, -1), Racks);
        assert_eq!(o.cycle(Setup, 3), Setup);
        assert_eq!(o.cycle(Sections, 127), o.pages().nth(127 % 3).unwrap());
        assert_eq!(o.cycle(Sections, -128), o.pages().nth((-128i16).rem_euclid(3) as usize).unwrap());

        let only = PageOrder::new(&[]).unwrap();
        assert_eq!((only.len(), only.step(Sections, 1), only.cycle(Sections, -1)), (1, Sections, Sections));
        assert_eq!(only.cycle(Racks, i8::MIN), Sections);

        for o in [d, o, only, PageOrder::new(&[MultiPads]).unwrap(), PageOrder::new(&[Chord, Setup, MultiPads, Racks]).unwrap()] {
            assert_eq!(PageOrder::from_bits(o.to_bits()), o);
        }
        // Garbage: a count over four, Sections or an out-of-range page, a duplicate, stray bits.
        for b in [7, 5, 1, 1 | 5 << 3, 2 | 1 << 3 | 1 << 6, 1 | 1 << 3 | 1 << 6, u16::MAX, 0x8000] {
            assert_eq!(PageOrder::from_bits(b), d, "{b:#x}");
        }
        for b in 0..=u16::MAX {
            let o = PageOrder::from_bits(b);
            assert!((1..=5).contains(&o.len()), "{b:#x}");
            assert!(o.contains(o.cycle(Setup, i8::MAX)) && o.contains(o.step(Chord, i8::MIN)), "{b:#x}");
        }
    }

    /// Layers: packed for an atomic, the JSON shapes, and the page the pads show.
    #[test]
    fn layers() {
        for l in [Layer::None, Layer::Sound, Layer::Swap { part: 0 }, Layer::Swap { part: 3 }, Layer::Swap { part: 253 }] {
            assert_eq!(Layer::from_u8(l.to_u8()), l);
        }
        for v in 0..=u8::MAX {
            assert_eq!(Layer::from_u8(v).to_u8(), v);
        }
        assert_eq!(Layer::default(), Layer::None);
        for (l, j) in [(Layer::None, r#"{"type":"none"}"#), (Layer::Sound, r#"{"type":"sound"}"#), (Layer::Swap { part: 0 }, r#"{"type":"swap","part":0}"#)] {
            assert_eq!(serde_json::to_string(&l).unwrap(), j);
            assert_eq!(serde_json::from_str::<Layer>(j).unwrap(), l);
        }
        for p in Page::ALL {
            assert_eq!(Layer::Sound.pads(p), Page::Racks);
            assert_eq!(Layer::None.pads(p), p);
            assert_eq!(Layer::Swap { part: 2 }.pads(p), p);
        }
    }

    /// Multi Pads: pads 1-4 and STOP; SELECT + pad and STOP + pad on the bottom row.
    #[test]
    fn multi_pads_page_actions() {
        let p = Page::MultiPads;
        for n in 0..4u8 {
            assert_eq!(act(p, 96 + n), Some(Action::MultiPad(PadCmd::Trigger(n))));
            assert_eq!(act(p, 112 + n), Some(Action::MultiPad(PadCmd::Arm(n))));
            assert_eq!(act(p, 116 + n), Some(Action::MultiPad(PadCmd::Stop(n))));
        }
        assert_eq!(act(p, 100), Some(Action::MultiPad(PadCmd::StopAll)));
        for n in 101..=103 {
            assert_eq!(act(p, n), None);
        }
    }

    /// Multi Pads lamps as on the Genos: blue = data, red = playing, flashing red = standby,
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

    /// Racks page lamps: red = the loaded rack, blue = a rack, off = empty; all flashing
    /// while Store is armed; OTS 1-4 dark past the style's count and bright on the one
    /// recalled; the spare dark.
    #[test]
    fn racks_page_lamps() {
        let quick = QuickPanel { stored: 0b101, loaded: 0b100, bank: 0, store: false };
        let panel = Panel { page: Page::Racks, quick, ..Panel::default() };
        let l = looks(&snap(), &[true; 32], &panel);
        assert_eq!(l, racks_looks(&panel), "the dev mock's Racks page is the same");
        assert_eq!((l[0].1.rgb, l[0].1.level), (C_QUICK_STORED, Level::Bright));
        assert_eq!(l[1].1.level, Level::Off);
        assert_eq!((l[2].1.rgb, l[2].1.level), (C_QUICK_LOADED, Level::Bright));
        assert!(l[8..12].iter().all(|(_, l)| l.level == Level::Off), "a style without OTS");
        assert_eq!((l[15].1.level, l[15].1.label), (Level::Off, ""), "the spare is dark");
        assert_eq!(l[12].1.level, Level::Off, "Bank - is dark on Bank A");
        assert_eq!(l[13].1.level, Level::Dim, "Bank + is lit below Bank H");
        assert_eq!((l[8].1.label, l[8].1.key, l[12].1.label, l[13].1.key, l[14].1.label, l[14].1.key), ("OTS 1", "⇧1", "BANK -", "⇧P", "STORE", "F5"));
        assert!(l[8..].iter().all(|(_, l)| l.rgb == C_PAGE_RACKS), "the bottom row in the page's colour");
        let leds = pad_leds(&snap(), &[true; 32], &panel);
        assert_eq!(leds[0].1, Led::Solid(BLUE));
        assert_eq!(leds[1].1, Led::Solid(OFF));
        assert_eq!(leds[2].1, Led::Solid(RED));
        assert_eq!((leds[12].1, leds[13].1, leds[14].1, leds[15].1), (Led::Solid(OFF), Led::Solid(DIM_ORANGE), Led::Solid(DIM_ORANGE), Led::Solid(OFF)));

        let ots = Panel { ots_count: 3, ots_applied: 2, ..panel };
        let l = looks(&snap(), &[true; 32], &ots);
        assert_eq!(l[8..12].iter().map(|(_, l)| l.level).collect::<Vec<_>>(), [Level::Dim, Level::Bright, Level::Dim, Level::Off]);
        let leds = pad_leds(&snap(), &[true; 32], &ots);
        assert_eq!((leds[8].1, leds[9].1, leds[11].1), (Led::Solid(DIM_ORANGE), Led::Solid(ORANGE), Led::Solid(OFF)));

        let armed = Panel { quick: QuickPanel { store: true, ..quick }, ..panel };
        assert!(looks(&snap(), &[true; 32], &armed)[..8].iter().all(|(_, l)| l.anim == Anim::Flash));
        assert_eq!(pad_leds(&snap(), &[true; 32], &armed)[14].1, Led::Flash(DIM_RED, RED), "Store flashes red while armed");
        assert_eq!(pad_leds(&snap(), &[true; 32], &armed)[0].1, Led::Flash(DIM_RED, RED));
        let last = Panel { quick: QuickPanel { bank: QUICK_BANKS - 1, ..quick }, ..panel };
        assert_eq!(looks(&snap(), &[true; 32], &last)[13].1.level, Level::Off, "Bank + stops at H");
    }

    /// Holding Sound draws the Racks page from any page, on the screen and the pads.
    #[test]
    fn sound_layer_draws_racks() {
        let has = [true; crate::engine::NUM_SLOTS];
        let quick = QuickPanel { stored: 0b11, loaded: 0b1, bank: 2, store: false };
        let racks = Panel { page: Page::Racks, quick, ots_count: 2, ..Panel::default() };
        for page in Page::ALL {
            let held = Panel { page, layer: Layer::Sound, ..racks };
            assert_eq!(held.shown(), Page::Racks);
            assert_eq!(looks(&snap(), &has, &held), looks(&snap(), &has, &racks), "{page:?}");
            assert_eq!(pad_leds(&snap(), &has, &held), pad_leds(&snap(), &has, &racks), "{page:?}");
            assert!(held.lamps().sound);
            let swap = Panel { page, layer: Layer::Swap { part: 0 }, ..racks };
            assert_eq!(swap.shown(), page);
            assert!(!swap.lamps().sound);
        }
    }

    /// Encoder page ▲ is lit while the rotary is fast, dark while slow, and dark on exit.
    #[test]
    fn encoder_page_up_lights_for_rotary_fast() {
        let mut out = Vec::new();
        knob_button_msgs(true, &mut out);
        assert_eq!(out, [[0xB0, KNOB_UP_CC, WHITE], [0xB3, KNOB_UP_CC, 127]]);
        out.clear();
        knob_button_msgs(false, &mut out);
        assert_eq!(out, [[0xB0, KNOB_UP_CC, OFF], [0xB3, KNOB_UP_CC, 0]]);
        out.clear();
        buttons_off_msgs(&mut out);
        assert!(out.contains(&[0xB0, KNOB_UP_CC, OFF]));
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
        assert_eq!(cc_control(52, false), Some(Control::Act(Action::KnobPage(1))));
        // Shift + ▲: Organ Rotary Slow/Fast.
        assert_eq!(cc_control(51, true), Some(Control::Act(Action::Assign(crate::controllers::Function::RotaryFast))));
        // Shift + ▼: [ACMP] (#266).
        assert_eq!(cc_control(52, true), Some(Control::Act(Action::Button(Button::Acmp))));
        assert_eq!(cc_control(53, false), None);
    }

    /// Every page lights all 16 pads in the same order, so the LED cache keyed by index
    /// updates exactly the pads that change on a page switch; under a Sound hold too.
    #[test]
    fn every_page_covers_the_pads_in_order() {
        let has = [true; crate::engine::NUM_SLOTS];
        for page in Page::ALL {
            for layer in [Layer::None, Layer::Sound, Layer::Swap { part: 3 }] {
                let panel = Panel { page, layer, ..Panel::default() };
                let notes: Vec<u8> = looks(&snap(), &has, &panel).iter().map(|(n, _)| *n).collect();
                assert_eq!(notes, PADS, "{page:?} {layer:?}");
                let notes: Vec<u8> = pad_leds(&snap(), &has, &panel).iter().map(|(n, _)| *n).collect();
                assert_eq!(notes, PADS, "{page:?} {layer:?}");
            }
        }
    }

    #[test]
    fn chord_page_lamps() {
        let has = [true; crate::engine::NUM_SLOTS];
        let mut s = snap();
        let mut panel = Panel { page: Page::Chord, ..Panel::default() };
        let lk = |s: &Snapshot, p: &Panel| looks(s, &has, p).map(|(_, l)| l);
        let leds = |s: &Snapshot, p: &Panel| pad_leds(s, &has, p).map(|(_, l)| l);

        let l = lk(&s, &panel);
        assert!(l.iter().all(|l| l.rgb == C_PAGE_CHORD), "one colour for the page");
        assert!(l[..8].iter().all(|l| l.level == Level::Off && l.label.is_empty()), "the top row is dark");
        assert!(leds(&s, &panel)[..8].iter().all(|l| *l == Led::Solid(OFF)));
        assert_eq!(l[8].level, Level::Off, "Manual Bass is unavailable in Lower");
        assert_eq!(l[15].level, Level::Dim, "Retrigger off");
        assert_eq!(leds(&s, &panel)[15], Led::Solid(DIM_CYAN));

        panel.upper = true;
        s.stop_acmp = true;
        s.transpose = Transpose::new(-2, 0);
        s.retrigger = true;
        let l = lk(&s, &panel);
        assert_eq!(l[15].level, Level::Bright, "Retrigger on");
        assert_eq!(l[8].level, Level::Bright, "Manual Bass on");
        assert_eq!(l[9].level, Level::Bright, "Stop ACMP");
        assert_eq!((l[12].level, l[13].level, l[14].level), (Level::Bright, Level::Dim, Level::Bright));
        assert_eq!(leds(&s, &panel)[9], Led::Solid(CYAN));
        panel.manual_bass = false;
        assert_eq!(lk(&s, &panel)[8].level, Level::Dim);
    }

    #[test]
    fn setup_page_lamps() {
        let has = [true; crate::engine::NUM_SLOTS];
        let mut s = snap();
        let mut panel = Panel { page: Page::Setup, fingering: Fingering::MultiFinger, ..Panel::default() };
        let lk = |s: &Snapshot, p: &Panel| looks(s, &has, p).map(|(_, l)| l);
        let leds = |s: &Snapshot, p: &Panel| pad_leds(s, &has, p).map(|(_, l)| l);

        let l = lk(&s, &panel);
        assert!(l.iter().all(|l| l.rgb == C_PAGE_SETUP), "one colour for the page");
        let bright: Vec<usize> = (0..7).filter(|&i| l[i].level == Level::Bright).collect();
        assert_eq!(bright, vec![3], "only the selected fingering type is lit");
        assert_eq!(l[7].level, Level::Dim, "Lower");
        assert_eq!((l[8].level, l[9].level, l[10].level), (Level::Dim, Level::Dim, Level::Dim), "OTS Link off, Stop ACMP mode Off");
        assert_eq!((l[8].label, l[9].label, l[10].label), ("OTS LINK", "ACMP STYLE", "ACMP FIXED"));
        assert!(l[11..].iter().all(|l| l.level == Level::Off && l.label.is_empty()), "115-119 dark");
        assert_eq!((leds(&s, &panel)[3], leds(&s, &panel)[0], leds(&s, &panel)[11]), (Led::Solid(PINK), Led::Solid(DIM_PINK), Led::Solid(OFF)));

        panel.upper = true;
        panel.ots_link = true;
        s.stop_acmp_mode = StopAcmp::Style;
        let l = lk(&s, &panel);
        assert_eq!((l[7].level, l[8].level, l[9].level, l[10].level), (Level::Bright, Level::Bright, Level::Bright, Level::Dim));
        s.stop_acmp_mode = StopAcmp::Fixed;
        let l = lk(&s, &panel);
        assert_eq!((l[9].level, l[10].level), (Level::Dim, Level::Bright));
    }

    #[test]
    fn fader_buttons_follow_the_page() {
        let mut out = Vec::new();
        fader_button_msgs(FaderPage::Panel, FaderLayer::Volume, 0b1001, 0xFF, PanelLamps::default(), &mut out);
        assert_eq!(out[..4], [[0xB0, 37, BLUE], [0xB0, 38, DIM_BLUE], [0xB0, 39, DIM_BLUE], [0xB0, 40, BLUE]]);
        assert_eq!(out[4], [0xB0, 41, DIM_PURPLE], "button 5: HARMONY/ARPEGGIO off");
        assert_eq!(out[5], [0xB0, 42, DIM_WHITE], "button 6: Sound, not held");
        assert_eq!(out[7][2], OFF, "button 8: no loop");
        assert_eq!(out[6], [0xB0, 43, DIM_ORANGE], "button 7: Left Hold off");
        assert_eq!(out[8], [0xB0, 45, BLUE]);
        out.clear();
        let lit = PanelLamps { harmony_arp: true, sound: true, left_hold: true, looper: LooperLamp::Looping };
        fader_button_msgs(FaderPage::Panel, FaderLayer::Volume, 0b1001, 0xFF, lit, &mut out);
        assert_eq!(out[4], [0xB0, 41, PURPLE], "button 5: HARMONY/ARPEGGIO on");
        assert_eq!(out[5], [0xB0, 42, WHITE], "button 6: Sound held");
        assert_eq!(out[6], [0xB0, 43, ORANGE], "button 7: Left Hold on");
        assert_eq!(out[7], [0xB0, 44, GREEN], "button 8: the Chord Looper loops");
        out.clear();
        fader_button_msgs(FaderPage::Panel, FaderLayer::Volume, 0, 0, PanelLamps { looper: LooperLamp::Recording, ..lit }, &mut out);
        assert_eq!(out[7], [0xB0, 44, RED], "button 8: recording");
        out.clear();
        // The Style page's button 5 is the Style's fifth part, whatever the switch; button
        // 6 is Sound on this page too, whatever part 6 (Pad) does.
        fader_button_msgs(FaderPage::Style, FaderLayer::Volume, 0b1001, !(1 << 4), lit, &mut out);
        assert_eq!(out[4], [0xB0, 41, DIM_GREEN], "part 5 muted");
        assert_eq!(out[5], [0xB0, 42, WHITE], "button 6: Sound held");
        assert!(out.iter().enumerate().all(|(i, m)| i == 4 || i == 5 || m[2] == GREEN));
        out.clear();
        fader_button_msgs(FaderPage::Style, FaderLayer::Volume, 0, 0xFF, PanelLamps::default(), &mut out);
        assert_eq!(out[5], [0xB0, 42, DIM_WHITE], "button 6: Sound, not held");
        assert_eq!(SOUND_FADER_BTN, 5);
        let held = Panel { layer: Layer::Sound, ..Panel::default() };
        assert!(held.lamps().sound && !Panel::default().lamps().sound);
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
        let d = PageOrder::DEFAULT;
        nav_button_msgs(Page::Sections, d, true, &mut out);
        assert!(out.contains(&[0xB0, PAD_UP_CC, OFF]) && out.contains(&[0xB0, PAD_DOWN_CC, WHITE]));
        assert!(out.contains(&[0xB0, TRACK_LEFT_CC, WHITE]) && out.contains(&[0xB0, TRACK_RIGHT_CC, WHITE]));
        out.clear();
        nav_button_msgs(Page::Chord, d, false, &mut out);
        assert!(out.contains(&[0xB0, PAD_UP_CC, CYAN]) && out.contains(&[0xB0, PAD_DOWN_CC, CYAN]));
        assert!(out.contains(&[0xB0, TRACK_LEFT_CC, OFF]));
        out.clear();
        nav_button_msgs(Page::Racks, d, true, &mut out);
        assert!(out.contains(&[0xB0, PAD_UP_CC, ORANGE]) && out.contains(&[0xB0, PAD_DOWN_CC, ORANGE]));
        out.clear();
        nav_button_msgs(Page::MultiPads, d, true, &mut out);
        assert!(out.contains(&[0xB0, PAD_UP_CC, YELLOW]) && out.contains(&[0xB0, PAD_DOWN_CC, YELLOW]));
        out.clear();
        nav_button_msgs(Page::Setup, d, true, &mut out);
        assert!(out.contains(&[0xB0, PAD_UP_CC, PINK]) && out.contains(&[0xB0, PAD_DOWN_CC, OFF]));
        out.clear();
        // A custom order: Multi Pads last, Setup second.
        let o = PageOrder::new(&[Page::Setup, Page::MultiPads]).unwrap();
        nav_button_msgs(Page::MultiPads, o, true, &mut out);
        assert!(out.contains(&[0xB0, PAD_UP_CC, YELLOW]) && out.contains(&[0xB0, PAD_DOWN_CC, OFF]));
        out.clear();
        nav_button_msgs(Page::Setup, o, true, &mut out);
        assert!(out.contains(&[0xB0, PAD_UP_CC, PINK]) && out.contains(&[0xB0, PAD_DOWN_CC, PINK]));
        out.clear();
        let only = PageOrder::new(&[]).unwrap();
        nav_button_msgs(Page::Sections, only, true, &mut out);
        assert!(out.contains(&[0xB0, PAD_UP_CC, OFF]) && out.contains(&[0xB0, PAD_DOWN_CC, OFF]));
        let b = button_colours(Page::Setup, o, true, FaderPage::Panel, FaderLayer::Volume, 0, 0, PanelLamps::default());
        assert!(b.contains(&(PAD_UP_CC, PINK)) && b.contains(&(PAD_DOWN_CC, PINK)));
        out.clear();
        buttons_off_msgs(&mut out);
        for cc in [PAD_UP_CC, PAD_DOWN_CC, TRACK_LEFT_CC, TRACK_RIGHT_CC, 37, 45] {
            assert!(out.contains(&[0xB0, cc, OFF]) && out.contains(&[0xB3, cc, 0]), "{cc}");
        }
    }
}

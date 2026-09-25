//! Controllers: the sustain pedal and other footswitches, the pitch-bend and modulation
//! wheels, and the assignable functions a pedal can run (Genos RM p.138-144, OM p.64).
//!
//! # The model
//!
//! - **Pedals** ([`PEDALS`] of them, like the Genos's three jacks): each listens for one
//!   control change from the keyboard (the Launchkey's sustain jack sends CC 64) and runs
//!   one [`Function`] from the table ([`FUNCTIONS`]). A pedal has a Control Type
//!   ([`ControlType`]: Toggle, Hold A, Hold B) for the functions that have one, a polarity
//!   switch, and a Range ([`Range`]) for Pitch Bend.
//! - **Pedal switches**: Sustain, Sostenuto and Soft. Each is one logical on/off
//!   ([`SUSTAIN`], [`SOSTENUTO`], [`SOFT`]) that reaches the keyboard parts chosen for it
//!   (per part, [`Controllers::set_part`]) as CC 64 / 66 / 67.
//! - **Wheels**: pitch bend and modulation (CC 1) from the keyboard reach the parts chosen
//!   for them, with each part's Pitch Bend Range (0-12 semitones, RPN 0).
//!
//! # What each part's channel gets
//!
//! A part's channel always holds exactly what applies to it now: the pedal switch, the
//! modulation and the bend when the part sounds (`Parts::audible_mask`: a solo counts) and is chosen for
//! them, else released / 0 / centre. [`Controllers::sync`] sends the difference between
//! that and what the channel was last sent (`sent_*`), so:
//!
//! - a part switched off while the pedal is down is released (its notes stop ringing, and
//!   nothing is left stuck when it comes back on);
//! - a part switched on while the pedal is down is sustained from then on, as on the Genos
//!   (the pedal is down; the part joins it);
//! - a part switched off while bent goes back to centre, and one switched on while bent
//!   gets the bend.
//!
//! Two threads call `sync`: the MIDI input thread (right after the message that changed
//! something, so a pedal and the notes in the same packet keep their order) and the
//! engine thread on every wake (for the part switches and settings, which change on the
//! control side). They send through different output buffers, flushed at different
//! times, so only one of them may send at a time: each [`Controllers::claim`]s the right
//! before `sync` and [`Controllers::release`]s it after its flush. A thread that can't
//! claim it doesn't wait: it leaves a note, and the holder syncs again before it lets go.
//! Otherwise a change one thread sent could reach the synth after a newer one the other
//! thread flushed first, and the channel would keep the old value while `sent_*` says
//! the new one. The Pitch Bend Range goes out from the engine thread only
//! ([`Controllers::sync_ranges`]): an RPN is five messages that must not interleave with
//! another thread's.
//!
//! # Safety: nothing stuck
//!
//! [`Controllers::reset`] puts every pedal switch and wheel back to neutral and sends that
//! to every keyboard part, whatever it was sent before. It runs on Panic, when a MIDI source
//! that moved a pedal or wheel is disconnected (its release will never come,
//! [`Controllers::touched`]), and when the keyboard sends Reset All Controllers (CC 121).
//! Style changes, section changes and Stop never touch the keyboard parts' channels
//! (the band plays on 9-16), so a held pedal carries across them unchanged. After a reset
//! every pedal counts as up, and a Hold B switch stays off until its pedal is next pressed
//! and released (a Panic doesn't turn a sustain straight back on).
//!
//! # More than one keyboard
//!
//! Each keyboard source keeps its own pedal edges (the input thread's `edges`, one byte
//! per source). A pedal is down while any source holds it: two keyboards with a sustain
//! pedal each on CC 64 keep Sustain on until both are up. A press on any source fires a
//! trigger function and flips a Toggle.
//!
//! All of this is atomics: the input thread, the engine thread and the control side read
//! and write it without waiting, and nothing here allocates.

use crate::engine::Button;
use crate::parts::{self, COUNT};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, AtomicU8, Ordering::Relaxed, Ordering::SeqCst};

/// How many pedals (the Genos has three jacks).
pub const PEDALS: usize = 3;
/// No CC: the pedal listens to nothing.
pub const NO_CC: u8 = 0xFF;
/// `Controllers::learn`: no pedal is learning.
const NOT_LEARNING: u8 = 0xFF;

/// Pedal switch bits (`Controllers::switches`).
pub const SUSTAIN: u8 = 1;
pub const SOSTENUTO: u8 = 2;
pub const SOFT: u8 = 4;
/// The CC each switch bit sends, in bit order.
const SWITCH_CC: [(u8, u8); 3] = [(SUSTAIN, 64), (SOSTENUTO, 66), (SOFT, 67)];

/// Pitch bend at rest (14 bits).
pub const BEND_CENTRE: u16 = 8192;
/// Genos default Pitch Bend Range, in semitones.
pub const DEFAULT_BEND_RANGE: u8 = 2;
/// The largest Pitch Bend Range a keyboard part takes (RM p.140: 0-12).
pub const MAX_BEND_RANGE: u8 = 12;

/// Pedal default: listen to these CCs (the GM sustain, sostenuto and soft pedals), running
/// Sustain, Sostenuto and Soft. A keyboard with those pedals works as any GM instrument
/// until a pedal is given something else.
pub const DEFAULT_PEDALS: [(u8, Function); PEDALS] = [(64, Function::Sustain), (66, Function::Sostenuto), (67, Function::Soft)];

/// What an assignable function is, for the pedal that runs it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    /// On while held (or toggled): the pedal's Control Type applies.
    Switch,
    /// Runs once when the pedal goes down.
    Trigger,
    /// Follows the pedal's position (a foot controller, "*" in the manual).
    Continuous,
}

/// The manual's groups, for the picker.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Category {
    Voice,
    Style,
    Ots,
    Registration,
    Overall,
}

/// Every function a pedal can be given. The order is the table's ([`FUNCTIONS`]) and the
/// packed value's; add new ones at the end.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[repr(u8)]
pub enum Function {
    /// No function: the pedal does nothing.
    #[default]
    None,
    Sustain,
    Sostenuto,
    Soft,
    Modulation,
    PitchBend,
    StartStop,
    SyncStart,
    SyncStop,
    Intro1,
    Intro2,
    Intro3,
    MainA,
    MainB,
    MainC,
    MainD,
    FillDown,
    FillSelf,
    FillBreak,
    FillUp,
    Ending1,
    Ending2,
    Ending3,
    AutoFill,
    StopAcmp,
    OtsLink,
    Ots1,
    Ots2,
    Ots3,
    Ots4,
    OtsNext,
    OtsPrev,
    RegistBankNext,
    RegistBankPrev,
    TempoUp,
    TempoDown,
    TapTempo,
    TransposeUp,
    TransposeDown,
    Right1OnOff,
    Right2OnOff,
    Right3OnOff,
    LeftOnOff,
    FingeredOnBass,
    FadeInOut,
    /// Kbd Harmony/Arpeggio On/Off: the HARMONY/ARPEGGIO button (RM p.141).
    KbdHarmonyArp,
    /// Arpeggio Hold (RM p.141; OM p.57 points arpeggio players to it).
    ArpHold,
    /// Style Section Reset (OM p.67): yahaha's own row. The Genos reaches it only through
    /// TAP TEMPO while a style plays; here Tap sets the tempo by default (#128), so Section
    /// Reset gets a pedal or button of its own.
    SectionReset,
    /// Style Dynamics Control (RM p.142, a foot controller "*" function): the pedal's
    /// position is the Dynamics level (engine/dynamics.rs, #180).
    DynamicsControl,
}

/// One row of the assignable-function table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionInfo {
    pub id: Function,
    /// As the Genos names it (RM p.139-144), except yahaha's own rows: Stop Acmp On/Off
    /// (the Genos list has Acmp On/Off, which yahaha doesn't have) and one row per part for
    /// the Genos's single Part On/Off (see docs/controllers.md).
    pub name: &'static str,
    pub category: Category,
    pub kind: Kind,
    /// yahaha has it. A pedal can hold one that isn't (so a setup can name it), and it
    /// does nothing but say so.
    pub available: bool,
}

const fn f(id: Function, name: &'static str, category: Category, kind: Kind) -> FunctionInfo {
    FunctionInfo { id, name, category, kind, available: true }
}

use Category::*;
use Kind::*;

/// The assignable functions, in `Function` order: the Genos live-play list (RM p.139-144)
/// as far as yahaha has the feature. app/src/lib/api/assignable-functions.json is this table
/// as the app reads it (a test keeps the two equal).
pub const FUNCTIONS: [FunctionInfo; 49] = [
    f(Function::None, "No Assign", Overall, Trigger),
    f(Function::Sustain, "Sustain", Voice, Switch),
    f(Function::Sostenuto, "Sostenuto", Voice, Switch),
    f(Function::Soft, "Soft", Voice, Switch),
    f(Function::Modulation, "Modulation", Voice, Continuous),
    f(Function::PitchBend, "Pitch Bend", Voice, Continuous),
    f(Function::StartStop, "Style Start/Stop", Style, Trigger),
    f(Function::SyncStart, "Synchro Start On/Off", Style, Trigger),
    f(Function::SyncStop, "Synchro Stop On/Off", Style, Trigger),
    f(Function::Intro1, "Intro 1", Style, Trigger),
    f(Function::Intro2, "Intro 2", Style, Trigger),
    f(Function::Intro3, "Intro 3", Style, Trigger),
    f(Function::MainA, "Main A", Style, Trigger),
    f(Function::MainB, "Main B", Style, Trigger),
    f(Function::MainC, "Main C", Style, Trigger),
    f(Function::MainD, "Main D", Style, Trigger),
    f(Function::FillDown, "Fill Down", Style, Trigger),
    f(Function::FillSelf, "Fill Self", Style, Trigger),
    f(Function::FillBreak, "Fill Break", Style, Trigger),
    f(Function::FillUp, "Fill Up", Style, Trigger),
    f(Function::Ending1, "Ending 1", Style, Trigger),
    f(Function::Ending2, "Ending 2", Style, Trigger),
    f(Function::Ending3, "Ending 3", Style, Trigger),
    f(Function::AutoFill, "Auto Fill In On/Off", Style, Trigger),
    f(Function::StopAcmp, "Stop Acmp On/Off", Style, Trigger),
    f(Function::OtsLink, "OTS Link On/Off", Ots, Trigger),
    f(Function::Ots1, "One Touch Setting 1", Ots, Trigger),
    f(Function::Ots2, "One Touch Setting 2", Ots, Trigger),
    f(Function::Ots3, "One Touch Setting 3", Ots, Trigger),
    f(Function::Ots4, "One Touch Setting 4", Ots, Trigger),
    f(Function::OtsNext, "One Touch Setting +", Ots, Trigger),
    f(Function::OtsPrev, "One Touch Setting −", Ots, Trigger),
    f(Function::RegistBankNext, "Registration Bank +", Registration, Trigger),
    f(Function::RegistBankPrev, "Registration Bank −", Registration, Trigger),
    f(Function::TempoUp, "Tempo +", Overall, Trigger),
    f(Function::TempoDown, "Tempo −", Overall, Trigger),
    f(Function::TapTempo, "Tap Tempo", Overall, Trigger),
    f(Function::TransposeUp, "Transpose +", Overall, Trigger),
    f(Function::TransposeDown, "Transpose −", Overall, Trigger),
    f(Function::Right1OnOff, "Right 1 On/Off", Overall, Trigger),
    f(Function::Right2OnOff, "Right 2 On/Off", Overall, Trigger),
    f(Function::Right3OnOff, "Right 3 On/Off", Overall, Trigger),
    f(Function::LeftOnOff, "Left On/Off", Overall, Trigger),
    f(Function::FingeredOnBass, "Fingered/Fingered On Bass", Style, Trigger),
    f(Function::FadeInOut, "Fade In/Out", Style, Trigger),
    f(Function::KbdHarmonyArp, "Kbd Harmony/Arpeggio On/Off", Voice, Switch),
    f(Function::ArpHold, "Arpeggio Hold", Voice, Switch),
    f(Function::SectionReset, "Style Section Reset", Style, Trigger),
    f(Function::DynamicsControl, "Dynamics Control", Style, Continuous),
];

/// What running a function means, for the input thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Nothing (No Assign).
    Nothing,
    /// A pedal switch bit (`SUSTAIN`, ...).
    Switch(u8),
    Modulation,
    PitchBend,
    /// The Dynamics level: straight to the engine (`Cmd::DynamicsLevel`).
    Dynamics,
    /// An engine button: straight to the engine.
    Engine(Button),
    /// Anything else: the control side runs it (`ControllersCmd::Trigger`).
    Control,
    /// A switch the control side keeps (Kbd Harmony/Arpeggio On/Off, Arpeggio Hold): a
    /// Toggle pedal runs it on each press (`Fire::control`), a Hold A or Hold B pedal sets
    /// it on or off as it goes down and up (`Fire::set`).
    ControlSwitch,
}

impl Function {
    pub fn info(self) -> &'static FunctionInfo {
        &FUNCTIONS[self as usize]
    }

    pub fn from_u8(v: u8) -> Function {
        FUNCTIONS.get(v as usize).map_or(Function::None, |i| i.id)
    }

    pub fn kind(self) -> Kind {
        self.info().kind
    }

    pub fn effect(self) -> Effect {
        use Function as F;
        match self {
            F::None => Effect::Nothing,
            F::Sustain => Effect::Switch(SUSTAIN),
            F::Sostenuto => Effect::Switch(SOSTENUTO),
            F::Soft => Effect::Switch(SOFT),
            F::Modulation => Effect::Modulation,
            F::PitchBend => Effect::PitchBend,
            F::DynamicsControl => Effect::Dynamics,
            F::StartStop => Effect::Engine(Button::StartStop),
            F::SyncStart => Effect::Engine(Button::SyncStart),
            F::SyncStop => Effect::Engine(Button::SyncStop),
            F::Intro1 | F::Intro2 | F::Intro3 => Effect::Engine(Button::Intro(self as u8 - F::Intro1 as u8)),
            F::MainA | F::MainB | F::MainC | F::MainD => Effect::Engine(Button::Main(self as u8 - F::MainA as u8)),
            F::FillDown => Effect::Engine(Button::FillDown),
            F::FillSelf => Effect::Engine(Button::FillSelf),
            F::FillUp => Effect::Engine(Button::FillUp),
            F::FillBreak => Effect::Engine(Button::Break),
            F::Ending1 | F::Ending2 | F::Ending3 => Effect::Engine(Button::Ending(self as u8 - F::Ending1 as u8)),
            F::AutoFill => Effect::Engine(Button::AutoFill),
            F::StopAcmp => Effect::Engine(Button::StopAcmp),
            F::TempoUp => Effect::Engine(Button::TempoUp),
            F::TempoDown => Effect::Engine(Button::TempoDown),
            F::TapTempo => Effect::Engine(Button::TapTempo),
            F::SectionReset => Effect::Engine(Button::SectionReset),
            // The FADE IN/OUT button (OM p.67): stopped, arms a fade in; playing, fades
            // out to the stop.
            F::FadeInOut => Effect::Engine(Button::Fade),
            F::KbdHarmonyArp | F::ArpHold => Effect::ControlSwitch,
            _ => Effect::Control,
        }
    }
}

/// How a pedal drives a Switch function (RM p.139).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ControlType {
    /// On while the pedal is held (the default; how a sustain pedal works).
    #[default]
    HoldA,
    /// Off while the pedal is held, on when it is up.
    HoldB,
    /// Each press switches it on or off.
    Toggle,
}

/// The Range of a continuous function (RM p.139): which half of the bend the pedal sweeps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Range {
    /// Up: heel = centre, toe = the top of the range.
    #[default]
    Upper,
    /// Down: heel = centre, toe = the bottom.
    Lower,
    /// The whole bend: heel = bottom, toe = top.
    Full,
}

/// One pedal's setup.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PedalSetup {
    /// The CC it listens for on the keyboard sources (`None`: nothing).
    pub cc: Option<u8>,
    pub function: Function,
    pub control_type: ControlType,
    /// Reversed polarity: pressed reads as up (a pedal that works the other way round).
    pub reverse: bool,
    pub range: Range,
}

impl Default for PedalSetup {
    fn default() -> PedalSetup {
        PedalSetup { cc: None, function: Function::None, control_type: ControlType::HoldA, reverse: false, range: Range::Upper }
    }
}

impl PedalSetup {
    fn pack(self) -> u32 {
        let ct = match self.control_type {
            ControlType::HoldA => 0,
            ControlType::HoldB => 1,
            ControlType::Toggle => 2,
        };
        let range = match self.range {
            Range::Upper => 0,
            Range::Lower => 1,
            Range::Full => 2,
        };
        self.cc.filter(|&c| c < 128).unwrap_or(NO_CC) as u32 | (self.function as u32) << 8 | ct << 16 | (self.reverse as u32) << 18 | range << 20
    }

    fn unpack(v: u32) -> PedalSetup {
        let cc = (v & 0xFF) as u8;
        PedalSetup {
            cc: (cc < 128).then_some(cc),
            function: Function::from_u8((v >> 8) as u8),
            control_type: match (v >> 16) & 3 {
                1 => ControlType::HoldB,
                2 => ControlType::Toggle,
                _ => ControlType::HoldA,
            },
            reverse: v >> 18 & 1 != 0,
            range: match (v >> 20) & 3 {
                1 => Range::Lower,
                2 => Range::Full,
                _ => Range::Upper,
            },
        }
    }

    /// Where a continuous pedal at `v` (0-127, polarity applied) puts the bend (14 bits).
    pub fn bend_at(self, v: u8) -> u16 {
        let v = v.min(127) as u32;
        match self.range {
            Range::Upper => (BEND_CENTRE as u32 + v * 8191 / 127) as u16,
            Range::Lower => (BEND_CENTRE as u32 - v * 8192 / 127) as u16,
            Range::Full => (v * 16383 / 127) as u16,
        }
    }
}

/// Which parts a controller reaches, per part (bit = part index).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartTargets {
    pub sustain: bool,
    pub pitch_bend: bool,
    pub modulation: bool,
}

/// Default part targets: the pedal switches and pitch bend on all four parts (the RM:
/// Sustain "all notes played on the keyboard"; the joystick X axis bends all parts),
/// modulation on Right 1-3 (the joystick Y axis, OM p.70).
pub const DEFAULT_SUSTAIN_PARTS: u8 = 0b1111;
pub const DEFAULT_BEND_PARTS: u8 = 0b1111;
pub const DEFAULT_MOD_PARTS: u8 = 1 << parts::RIGHT1 | 1 << parts::RIGHT2 | 1 << parts::RIGHT3;

/// The controllers' settings and live state (see the module docs). One per session, in
/// `live::Shared`.
pub struct Controllers {
    pedal: [AtomicU32; PEDALS],
    sustain_parts: AtomicU8,
    bend_parts: AtomicU8,
    mod_parts: AtomicU8,
    bend_range: [AtomicU8; COUNT],
    /// The pedal switches that are on (`SUSTAIN` | `SOSTENUTO` | `SOFT`).
    switches: AtomicU8,
    /// The modulation wheel (or pedal), 0-127.
    modulation: AtomicU8,
    /// The pitch bend (wheel or pedal), 14 bits.
    bend: AtomicU16,
    /// The pedals held down, as last read (bit = pedal), for the screen.
    down: AtomicU8,
    /// What each part's channel was last sent.
    sent_switches: [AtomicU8; COUNT],
    sent_mod: [AtomicU8; COUNT],
    sent_bend: [AtomicU16; COUNT],
    /// 0xFF: never sent (the first engine wake sends it).
    sent_range: [AtomicU8; COUNT],
    /// Keyboard source slots that moved a pedal or wheel since the last reset.
    touched: AtomicU16,
    /// The pedal waiting to learn its CC (`NOT_LEARNING`: none).
    learn: AtomicU8,
    /// Pedals whose edge the input thread must forget (bit = pedal): set by a reset or a
    /// changed setup, so the next press is a press even if the last read was "down".
    forget: AtomicU8,
    /// A reset happened (`RESET_SEEN`) and the pedals that were down then (bit = pedal),
    /// for the control side: it lets go of the switches it keeps for a Hold pedal
    /// (`Effect::ControlSwitch`), as `reset` does the pedal switches
    /// (`take_reset_releases`, `reset_release`).
    reset_seen: AtomicU16,
    /// Left Hold (OM p.49): the Left part's channel holds as if its sustain pedal were
    /// down (`set_left_hold`).
    left_hold: AtomicBool,
    /// Bumped to let go of what Left Hold holds (a new Left key, a stop:
    /// `release_left_hold`); `sync` re-pedals Left when it moved since `sent_left_release`.
    left_release: AtomicU8,
    sent_left_release: AtomicU8,
    /// A thread holds the right to send the parts' controllers (`claim`).
    busy: AtomicBool,
    /// A thread wanted to sync while the other held it: the holder syncs again.
    pending: AtomicBool,
}

impl Default for Controllers {
    fn default() -> Controllers {
        Controllers::new()
    }
}

impl Controllers {
    pub fn new() -> Controllers {
        Controllers {
            pedal: DEFAULT_PEDALS.map(|(cc, function)| AtomicU32::new(PedalSetup { cc: Some(cc), function, ..PedalSetup::default() }.pack())),
            sustain_parts: AtomicU8::new(DEFAULT_SUSTAIN_PARTS),
            bend_parts: AtomicU8::new(DEFAULT_BEND_PARTS),
            mod_parts: AtomicU8::new(DEFAULT_MOD_PARTS),
            bend_range: [const { AtomicU8::new(DEFAULT_BEND_RANGE) }; COUNT],
            switches: AtomicU8::new(0),
            modulation: AtomicU8::new(0),
            bend: AtomicU16::new(BEND_CENTRE),
            down: AtomicU8::new(0),
            sent_switches: [const { AtomicU8::new(0) }; COUNT],
            sent_mod: [const { AtomicU8::new(0) }; COUNT],
            sent_bend: [const { AtomicU16::new(BEND_CENTRE) }; COUNT],
            sent_range: [const { AtomicU8::new(0xFF) }; COUNT],
            touched: AtomicU16::new(0),
            learn: AtomicU8::new(NOT_LEARNING),
            forget: AtomicU8::new(0),
            reset_seen: AtomicU16::new(0),
            left_hold: AtomicBool::new(false),
            left_release: AtomicU8::new(0),
            sent_left_release: AtomicU8::new(0),
            busy: AtomicBool::new(false),
            pending: AtomicBool::new(false),
        }
    }

    // ----- settings (control side) -----

    pub fn pedal(&self, i: usize) -> PedalSetup {
        PedalSetup::unpack(self.pedal[i % PEDALS].load(Relaxed))
    }

    /// Set pedal `i`. When its function or CC changes, whatever the old setup was driving is
    /// released first (a switch it held or latched, a wheel it moved), so a pedal re-picked
    /// while down, latched by Toggle or mid-sweep never leaves a stuck pedal or bend. The
    /// pedal then counts as up until it is next pressed.
    ///
    /// A Hold A or Hold B switch follows the pedal's position, so when the function, CC or
    /// Control Type changes it is set to where the pedal is now: Hold B picked with the
    /// pedal up turns the switch on (RM p.139: Hold B "turns the function off and keeps it
    /// inactive while holding down"), and Hold A turned to Hold B while the pedal is held
    /// turns it off.
    pub fn set_pedal(&self, i: usize, p: PedalSetup) {
        let i = i % PEDALS;
        let old = PedalSetup::unpack(self.pedal[i].swap(p.pack(), Relaxed));
        let rebound = old.function != p.function || old.cc != p.cc;
        if rebound {
            let bit = 1u8 << i;
            self.down.fetch_and(!bit, Relaxed);
            self.forget.fetch_or(bit, Relaxed);
            match old.function.effect() {
                Effect::Switch(b) => {
                    if !self.kept_on_by_another(i, b) {
                        self.set_switch(b, false);
                    }
                }
                Effect::Modulation => self.modulation.store(0, Relaxed),
                Effect::PitchBend => self.bend.store(BEND_CENTRE, Relaxed),
                // A Dynamics Control pedal given another job leaves the level where it is.
                Effect::Nothing | Effect::Engine(_) | Effect::Control | Effect::ControlSwitch | Effect::Dynamics => {}
            }
        }
        let retyped = rebound || old.control_type != p.control_type;
        if let (Effect::Switch(b), true) = (p.function.effect(), retyped && p.control_type != ControlType::Toggle) {
            let down = self.down.load(Relaxed) >> i & 1 != 0;
            self.hold(i, b, p.control_type, down);
        }
    }

    /// Set switch `b` where Hold pedal `i` (Hold A or Hold B, `down` or up) puts it: off
    /// only if no other pedal keeps it on.
    fn hold(&self, i: usize, b: u8, ct: ControlType, down: bool) {
        let on = (ct == ControlType::HoldB) != down;
        if on || !self.kept_on_by_another(i, b) {
            self.set_switch(b, on);
        }
    }

    /// A pedal other than `i` on switch `b` keeps it on: a Hold A pedal held down, or a
    /// Hold B pedal up.
    fn kept_on_by_another(&self, i: usize, b: u8) -> bool {
        let down = self.down.load(Relaxed);
        (0..PEDALS).any(|j| {
            let q = self.pedal(j);
            let d = down >> j & 1 != 0;
            j != i
                && q.function.effect() == Effect::Switch(b)
                && match q.control_type {
                    ControlType::HoldA => d,
                    ControlType::HoldB => !d,
                    ControlType::Toggle => false,
                }
        })
    }

    pub fn part_targets(&self, part: usize) -> PartTargets {
        let bit = |m: &AtomicU8| m.load(Relaxed) >> (part & 3) & 1 != 0;
        PartTargets { sustain: bit(&self.sustain_parts), pitch_bend: bit(&self.bend_parts), modulation: bit(&self.mod_parts) }
    }

    pub fn set_part(&self, part: usize, t: PartTargets) {
        let b = 1u8 << (part & 3);
        let set = |m: &AtomicU8, on: bool| {
            if on {
                m.fetch_or(b, Relaxed);
            } else {
                m.fetch_and(!b, Relaxed);
            }
        };
        set(&self.sustain_parts, t.sustain);
        set(&self.bend_parts, t.pitch_bend);
        set(&self.mod_parts, t.modulation);
    }

    pub fn bend_range(&self, part: usize) -> u8 {
        self.bend_range[part & 3].load(Relaxed)
    }

    pub fn set_bend_range(&self, part: usize, semitones: u8) {
        self.bend_range[part & 3].store(semitones.min(MAX_BEND_RANGE), Relaxed);
    }

    /// Pedal `i` learns its CC from the next control change a keyboard sends (None: stop).
    pub fn learn(&self, i: Option<usize>) {
        self.learn.store(i.map_or(NOT_LEARNING, |i| (i % PEDALS) as u8), Relaxed);
    }

    pub fn learning(&self) -> Option<usize> {
        let v = self.learn.load(Relaxed);
        (v != NOT_LEARNING).then_some(v as usize)
    }

    // ----- live state -----

    pub fn switches(&self) -> u8 {
        self.switches.load(Relaxed)
    }

    pub fn modulation(&self) -> u8 {
        self.modulation.load(Relaxed)
    }

    pub fn bend(&self) -> u16 {
        self.bend.load(Relaxed)
    }

    /// The pedals held down (bit = pedal).
    pub fn down(&self) -> u8 {
        self.down.load(Relaxed)
    }

    /// The control side: whether a reset (Panic, a keyboard unplugged) happened since the
    /// last call, with the pedals that were down then (bit = pedal). Each pedal's
    /// control-side switch is then let go of ([`reset_release`]).
    pub fn take_reset_releases(&self) -> Option<u8> {
        let w = self.reset_seen.swap(0, Relaxed);
        (w & RESET_SEEN != 0).then_some(w as u8)
    }

    /// Keyboard source slot `slot` moved a pedal or wheel since the last reset.
    pub fn touched(&self, slot: usize) -> bool {
        self.touched.load(Relaxed) >> (slot & 15) & 1 != 0
    }

    /// Anything away from neutral: a pedal switch on, the modulation up or a bend.
    pub fn active(&self) -> bool {
        self.switches() != 0 || self.modulation() != 0 || self.bend() != BEND_CENTRE
    }

    // ----- the input thread -----

    /// The input thread: a control change `cc` = `v` from keyboard source `slot`. Returns
    /// what it did, for the input thread to carry out (`Handled::Fire`) or to send the
    /// message on unchanged (`Handled::Pass`). Pedal edges are kept in `edges` (the input
    /// thread's own: one byte per keyboard source slot, bit = pedal).
    pub fn control_change(&self, slot: usize, cc: u8, v: u8, edges: &mut [u8]) -> Handled {
        let learning = self.learn.load(Relaxed);
        if learning != NOT_LEARNING && learnable(cc) && v >= 64 {
            let i = learning as usize % PEDALS;
            self.set_pedal(i, PedalSetup { cc: Some(cc), ..self.pedal(i) });
            self.learn.store(NOT_LEARNING, Relaxed);
            return Handled::Learned;
        }
        let forget = self.forget.swap(0, Relaxed);
        if forget != 0 {
            for e in edges.iter_mut() {
                *e &= !forget;
            }
        }
        let src = slot % edges.len().max(1);
        if cc == 121 {
            // Reset All Controllers: everything this module keeps back to neutral, and the
            // message still goes to the parts (it resets expression, pressure and the rest).
            self.switches.store(0, Relaxed);
            self.modulation.store(0, Relaxed);
            self.bend.store(BEND_CENTRE, Relaxed);
            return Handled::SyncAndPass;
        }
        let mut claimed = false;
        let mut fire = Fire::default();
        for i in 0..PEDALS {
            let p = self.pedal(i);
            if p.cc != Some(cc) {
                continue;
            }
            claimed = true;
            self.touch(slot);
            let pressed = (v >= 64) != p.reverse;
            let bit = 1u8 << i;
            // `was`: this source had it down. The pedal is down while any source holds it.
            let was = edges.get(src).is_some_and(|e| e & bit != 0);
            if let Some(e) = edges.get_mut(src) {
                if pressed {
                    *e |= bit;
                } else {
                    *e &= !bit;
                }
            }
            let others = edges.iter().enumerate().any(|(s, e)| s != src && e & bit != 0);
            let (was_down, is_down) = (was || others, pressed || others);
            if was_down != is_down {
                if is_down {
                    self.down.fetch_or(bit, Relaxed);
                } else {
                    self.down.fetch_and(!bit, Relaxed);
                }
                fire.shown = true;
            }
            let pos = if p.reverse { 127 - v.min(127) } else { v.min(127) };
            match p.function.effect() {
                Effect::Nothing => {}
                Effect::Switch(b) => match p.control_type {
                    ControlType::Toggle => {
                        if pressed && !was {
                            self.switches.fetch_xor(b, Relaxed);
                            fire.sync = true;
                        }
                    }
                    ct => {
                        if was_down != is_down {
                            self.hold(i, b, ct, is_down);
                            fire.sync = true;
                        }
                    }
                },
                Effect::Modulation => {
                    self.modulation.store(pos, Relaxed);
                    fire.sync = true;
                }
                Effect::PitchBend => {
                    self.bend.store(p.bend_at(pos), Relaxed);
                    fire.sync = true;
                }
                Effect::Dynamics => fire.dynamics = Some(pos),
                Effect::Engine(b) if pressed && !was => fire.engine = Some(b),
                Effect::Control if pressed && !was => fire.control = Some(p.function),
                Effect::ControlSwitch => match p.control_type {
                    ControlType::Toggle => {
                        if pressed && !was {
                            fire.control = Some(p.function);
                        }
                    }
                    ct => {
                        if was_down != is_down {
                            fire.set = Some((p.function, (ct == ControlType::HoldB) != is_down));
                        }
                    }
                },
                Effect::Engine(_) | Effect::Control => {}
            }
        }
        if claimed {
            return Handled::Fire(fire);
        }
        // Not a pedal's: the GM meaning of the wheel and pedal CCs, so they still go
        // through the parts model (part on/off, targets); anything else passes.
        match cc {
            1 => {
                self.touch(slot);
                self.modulation.store(v.min(127), Relaxed);
                Handled::Sync
            }
            64 | 66 | 67 => {
                self.touch(slot);
                let b = match cc {
                    64 => SUSTAIN,
                    66 => SOSTENUTO,
                    _ => SOFT,
                };
                self.set_switch(b, v >= 64);
                Handled::Sync
            }
            _ => Handled::Pass,
        }
    }

    /// The input thread: pitch bend `v` (14 bits) from keyboard source `slot`.
    pub fn pitch_bend(&self, slot: usize, v: u16) {
        self.touch(slot);
        self.bend.store(v.min(16383), Relaxed);
    }

    fn touch(&self, slot: usize) {
        self.touched.fetch_or(1 << (slot & 15), Relaxed);
    }

    /// Left Hold on or off (the control side).
    pub fn set_left_hold(&self, on: bool) {
        self.left_hold.store(on, Relaxed);
    }

    pub fn left_hold(&self) -> bool {
        self.left_hold.load(Relaxed)
    }

    /// Let go of the Left notes Left Hold holds, at the next `sync`: the input thread when
    /// a key sounds on Left (the next chord), the engine thread when the style stops. The
    /// keys still down keep sounding (a sustain release only ends released notes).
    pub fn release_left_hold(&self) {
        self.left_release.fetch_add(1, Relaxed);
    }

    /// Tests: how many releases were asked for (wrapping).
    #[cfg(test)]
    pub fn left_releases(&self) -> u8 {
        self.left_release.load(Relaxed)
    }

    /// Switch a pedal switch bit on or off from software (`TriggerFunction`).
    pub fn toggle_switch(&self, b: u8) {
        self.switches.fetch_xor(b, Relaxed);
    }

    fn set_switch(&self, b: u8, on: bool) {
        if on {
            self.switches.fetch_or(b, Relaxed);
        } else {
            self.switches.fetch_and(!b, Relaxed);
        }
    }

    // ----- output (input and engine threads) -----

    /// Take the right to send the parts' controllers, before `sync`; `release` it after
    /// the flush. False: the other thread has it (nothing waits). That thread syncs again
    /// before it lets go, so this change still goes out, after what it has in hand.
    pub fn claim(&self) -> bool {
        self.pending.store(true, SeqCst);
        if self.busy.compare_exchange(false, true, SeqCst, SeqCst).is_ok() {
            // This sync covers whoever asked (a swap, so it sees what they changed first).
            self.pending.swap(false, SeqCst);
            true
        } else {
            false
        }
    }

    /// Give back the right `claim` took, after the flush. True: the other thread wanted to
    /// sync meanwhile: claim, sync and flush again.
    pub fn release(&self) -> bool {
        self.busy.store(false, SeqCst);
        self.pending.load(SeqCst)
    }

    /// Send each keyboard part what applies to it now, where it differs from what it was
    /// sent. `sounding`: the parts that sound (`Parts::audible_mask`, which follows a solo).
    pub fn sync(&self, sounding: u8, out: &mut impl FnMut(&[u8])) {
        let (switches, modulation, bend) = (self.switches(), self.modulation(), self.bend());
        let (sus, pb, md) = (self.sustain_parts.load(Relaxed), self.bend_parts.load(Relaxed), self.mod_parts.load(Relaxed));
        for p in 0..COUNT {
            let ch = parts::CHANNEL[p];
            let on = sounding >> p & 1 != 0;
            let reaches = |mask: u8| on && mask >> p & 1 != 0;
            let pedal = if reaches(sus) { switches } else { 0 };
            // Left Hold: Left's channel holds while it sounds, unless the pedal already
            // sustains it.
            let held = p == parts::LEFT && on && self.left_hold.load(Relaxed) && pedal & SUSTAIN == 0;
            let want = if held { pedal | SUSTAIN } else { pedal };
            let mut was = self.sent_switches[p].swap(want, Relaxed);
            if p == parts::LEFT {
                // A release asked for since the last sync: re-pedal, so the notes held
                // stop and the keys going down now are held next.
                let rel = self.left_release.load(Relaxed);
                if self.sent_left_release.swap(rel, Relaxed) != rel && held && was != UNKNOWN_SWITCHES && was & SUSTAIN != 0 {
                    out(&[0xB0 | ch, 64, 0]);
                    was &= !SUSTAIN;
                }
            }
            let diff = if was == UNKNOWN_SWITCHES { u8::MAX } else { was ^ want };
            for (b, cc) in SWITCH_CC {
                if diff & b != 0 {
                    out(&[0xB0 | ch, cc, if want & b != 0 { 127 } else { 0 }]);
                }
            }
            let want = if reaches(md) { modulation } else { 0 };
            if self.sent_mod[p].swap(want, Relaxed) != want {
                out(&[0xB0 | ch, 1, want]);
            }
            let want = if reaches(pb) { bend } else { BEND_CENTRE };
            if self.sent_bend[p].swap(want, Relaxed) != want {
                out(&[0xE0 | ch, (want & 0x7F) as u8, (want >> 7) as u8]);
            }
        }
    }

    /// The engine thread: each part's Pitch Bend Range (RPN 0), where it changed.
    pub fn sync_ranges(&self, out: &mut impl FnMut(&[u8])) {
        for p in 0..COUNT {
            let want = self.bend_range(p);
            if self.sent_range[p].swap(want, Relaxed) != want {
                let ch = 0xB0 | parts::CHANNEL[p];
                for m in [[ch, 101, 0], [ch, 100, 0], [ch, 6, want], [ch, 38, 0], [ch, 101, 127], [ch, 100, 127]] {
                    out(&m);
                }
            }
        }
    }

    /// Everything back to neutral: pedal switches off, modulation 0, bend centred, sent to
    /// every keyboard part whatever it was sent before (Panic, a disconnected source).
    ///
    /// The other thread may hold a sync of its own not yet flushed, which would land after
    /// this: so every part is marked as holding nothing known, and the next `sync` (by
    /// whichever thread claims it) sends neutral once more after it.
    pub fn reset(&self, out: &mut impl FnMut(&[u8])) {
        self.switches.store(0, Relaxed);
        self.modulation.store(0, Relaxed);
        self.bend.store(BEND_CENTRE, Relaxed);
        self.touched.store(0, Relaxed);
        // The pedals count as up until pressed again: the next press is a press. The
        // control side lets go of what those that were down held (`take_reset_releases`).
        let was_down = self.down.swap(0, Relaxed);
        self.reset_seen.fetch_or(RESET_SEEN | was_down as u16, Relaxed);
        self.forget.store(u8::MAX, Relaxed);
        for p in 0..COUNT {
            let ch = parts::CHANNEL[p];
            self.sent_switches[p].store(UNKNOWN_SWITCHES, Relaxed);
            self.sent_mod[p].store(UNKNOWN_MOD, Relaxed);
            self.sent_bend[p].store(UNKNOWN_BEND, Relaxed);
            for (_, cc) in SWITCH_CC {
                out(&[0xB0 | ch, cc, 0]);
            }
            out(&[0xB0 | ch, 1, 0]);
            out(&[0xE0 | ch, 0x00, 0x40]);
        }
    }
}

/// `sent_*` after a reset: no value a part can want, so the next `sync` sends it all.
const UNKNOWN_SWITCHES: u8 = 0xFF;
const UNKNOWN_MOD: u8 = 0xFF;
const UNKNOWN_BEND: u16 = 0xFFFF;

/// Why a pedal can't listen for `cc` (None: it can). Bank select and volume belong to the
/// parts and never reach the pedals (the CC7 principle), CC 121 is Reset All Controllers,
/// the modulation wheel is a wheel, and data entry, (N)RPN selection and the other channel
/// mode messages are not pedals. Learn skips them and `SetPedal` refuses them.
pub fn pedal_cc_refused(cc: u8) -> Option<&'static str> {
    Some(match cc {
        0 | 32 => "bank select",
        1 => "the modulation wheel",
        6 | 38 => "data entry",
        7 => "volume",
        98..=101 => "(N)RPN selection",
        121 => "Reset All Controllers",
        120..=127 => "a channel mode message",
        128.. => "not a control change",
        _ => return None,
    })
}

fn learnable(cc: u8) -> bool {
    pedal_cc_refused(cc).is_none()
}

/// What a control change did (`Controllers::control_change`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handled {
    /// Not ours: send it on as before.
    Pass,
    /// A switch or wheel changed: `sync`.
    Sync,
    /// `sync`, then send the message on to all the parts too (Reset All Controllers).
    SyncAndPass,
    /// A pedal's: sync if `sync`, and run what it fired.
    Fire(Fire),
    /// A pedal learned its CC: the screen shows it.
    Learned,
}

/// What a pedal message asks the input thread to do.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Fire {
    pub sync: bool,
    /// A pedal went down or up (the screen shows it).
    pub shown: bool,
    pub engine: Option<Button>,
    pub control: Option<Function>,
    /// A control-side switch a Hold A or Hold B pedal sets on or off (`Effect::ControlSwitch`).
    pub set: Option<(Function, bool)>,
    /// A Dynamics Control pedal moved: the Dynamics level (`Effect::Dynamics`).
    pub dynamics: Option<u8>,
}

/// What a pedal's new setup (`Controllers::set_pedal` from `old` to `new`) does to the
/// control-side switches (`Effect::ControlSwitch`), which the control side keeps and so
/// sets itself: the switch a Hold pedal was keeping on goes off when the pedal is given
/// another function or CC (`old_down`: the pedal was down before), and a Hold A or Hold B
/// switch follows where the pedal is now (`new_down`), as the pedal switches do: Hold B
/// picked with the pedal up turns it on.
pub fn control_switch_sets(old: PedalSetup, new: PedalSetup, old_down: bool, new_down: bool) -> [Option<(Function, bool)>; 2] {
    let rebound = old.function != new.function || old.cc != new.cc;
    let retyped = rebound || old.control_type != new.control_type;
    let hold_on = |ct: ControlType, down: bool| (ct == ControlType::HoldB) != down;
    let held = |p: PedalSetup| p.function.effect() == Effect::ControlSwitch && p.control_type != ControlType::Toggle;
    let release = (rebound && held(old) && hold_on(old.control_type, old_down)).then_some((old.function, false));
    let follow = (retyped && held(new)).then(|| (new.function, hold_on(new.control_type, new_down)));
    [release, follow]
}

/// The control-side switch (`Effect::ControlSwitch`) a reset turns off for pedal setup `p`
/// (`was_down`: the pedal was down at the reset): the one a Hold pedal was keeping on, a
/// Hold A pedal's while down, a Hold B pedal's while up. As with the pedal switches
/// (`reset` turns them all off), the pedal then counts as up and a Hold B switch stays off
/// until the pedal is next pressed and released: a reset never turns anything on. None for
/// a Toggle pedal (a press switched it, as the panel button would; the reset leaves it)
/// and for other functions.
pub fn reset_release(p: PedalSetup, was_down: bool) -> Option<Function> {
    let held_on = match p.control_type {
        ControlType::HoldA => was_down,
        ControlType::HoldB => !was_down,
        ControlType::Toggle => false,
    };
    (p.function.effect() == Effect::ControlSwitch && held_on).then_some(p.function)
}

/// `Controllers::reset_seen`: a reset happened (the low bits are the pedals down then).
const RESET_SEEN: u16 = 1 << 8;

#[cfg(test)]
mod tests {
    use super::*;

    fn sent(c: &Controllers, sounding: u8) -> Vec<[u8; 3]> {
        let mut v = Vec::new();
        c.sync(sounding, &mut |m| v.push([m[0], m[1], m[2]]));
        v
    }

    #[test]
    fn table_is_in_enum_order_and_ids_are_unique() {
        for (i, info) in FUNCTIONS.iter().enumerate() {
            assert_eq!(info.id as usize, i, "{:?}", info.id);
            assert_eq!(Function::from_u8(i as u8), info.id);
        }
        assert_eq!(Function::from_u8(200), Function::None);
    }

    #[test]
    fn the_app_table_is_this_table() {
        let json = serde_json::to_string_pretty(&FUNCTIONS[..]).unwrap() + "\n";
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/app/src/lib/api/assignable-functions.json");
        if std::env::var("YAHAHA_WRITE_FIXTURES").is_ok() {
            std::fs::write(path, &json).unwrap();
        }
        let file = std::fs::read_to_string(path).expect("app/src/lib/api/assignable-functions.json");
        assert_eq!(file, json, "regenerate with YAHAHA_WRITE_FIXTURES=1 cargo test the_app_table");
    }

    #[test]
    fn pedal_setup_packs() {
        for function in FUNCTIONS.map(|i| i.id) {
            for control_type in [ControlType::HoldA, ControlType::HoldB, ControlType::Toggle] {
                for range in [Range::Upper, Range::Lower, Range::Full] {
                    for (cc, reverse) in [(Some(64), false), (None, true), (Some(0), true), (Some(127), false)] {
                        let p = PedalSetup { cc, function, control_type, reverse, range };
                        assert_eq!(PedalSetup::unpack(p.pack()), p);
                    }
                }
            }
        }
    }

    #[test]
    fn sustain_follows_the_pedal_on_the_parts_that_sound() {
        let c = Controllers::new();
        let mut e = [0u8; 4];
        assert_eq!(c.control_change(0, 64, 127, &mut e), Handled::Fire(Fire { sync: true, shown: true, ..Fire::default() }));
        // Right 1 and Left sound: ch 1 and ch 2.
        assert_eq!(sent(&c, 0b1001), vec![[0xB0, 64, 127], [0xB1, 64, 127]]);
        assert!(sent(&c, 0b1001).is_empty(), "sent once");
        // Right 2 comes on while the pedal is down: it joins; Left goes off: released.
        assert_eq!(sent(&c, 0b0011), vec![[0xB2, 64, 127], [0xB1, 64, 0]]);
        c.control_change(0, 64, 0, &mut e);
        assert_eq!(sent(&c, 0b0011), vec![[0xB0, 64, 0], [0xB2, 64, 0]]);
    }

    /// Left Hold (OM p.49, #202): Left's channel holds as if its sustain pedal were down;
    /// a new Left key lets go of what was held (a re-pedal before the key's note-on), and
    /// so does a stop. The pedal's own sustain wins over the re-pedal.
    #[test]
    fn left_hold_holds_the_left_part_until_the_next_left_key() {
        let c = Controllers::new();
        c.set_left_hold(true);
        assert!(c.left_hold());
        assert_eq!(sent(&c, 0b1001), vec![[0xB1, 64, 127]], "Left only");
        // A new Left key: re-pedal.
        c.release_left_hold();
        assert_eq!(sent(&c, 0b1001), vec![[0xB1, 64, 0], [0xB1, 64, 127]]);
        assert!(sent(&c, 0b1001).is_empty(), "once");
        // Left off: nothing held; on again: held again, with nothing to let go.
        assert_eq!(sent(&c, 0b0001), vec![[0xB1, 64, 0]]);
        c.release_left_hold();
        assert_eq!(sent(&c, 0b1001), vec![[0xB1, 64, 127]]);
        // The sustain pedal down on Left: a new Left key doesn't let go of what it sustains.
        let mut e = [0u8; 4];
        c.control_change(0, 64, 127, &mut e);
        assert_eq!(sent(&c, 0b1001), vec![[0xB0, 64, 127]]);
        c.release_left_hold();
        assert!(sent(&c, 0b1001).is_empty());
        c.control_change(0, 64, 0, &mut e);
        assert_eq!(sent(&c, 0b1001), vec![[0xB0, 64, 0]], "Left stays held by Left Hold");
        // Left Hold off: let go.
        c.set_left_hold(false);
        assert_eq!(sent(&c, 0b1001), vec![[0xB1, 64, 0]]);
        c.release_left_hold();
        assert!(sent(&c, 0b1001).is_empty(), "off: nothing to re-pedal");
    }

    #[test]
    fn part_targets_choose_who_gets_what() {
        let c = Controllers::new();
        c.set_part(parts::LEFT, PartTargets { sustain: false, pitch_bend: true, modulation: true });
        let mut e = [0u8; 4];
        c.control_change(0, 64, 127, &mut e);
        c.control_change(0, 1, 90, &mut e);
        c.pitch_bend(0, 16383);
        let all = sent(&c, 0b1111);
        // Left (ch 2) gets modulation and bend but no sustain.
        assert!(!all.contains(&[0xB1, 64, 127]));
        assert!(all.contains(&[0xB1, 1, 90]));
        assert!(all.contains(&[0xE1, 0x7F, 0x7F]));
        // Right 1 gets all three.
        for m in [[0xB0, 64, 127], [0xB0, 1, 90], [0xE0, 0x7F, 0x7F]] {
            assert!(all.contains(&m), "{m:?}");
        }
        // Right 2 (ch 3): modulation reaches Right 1-3 by default.
        assert!(all.contains(&[0xB2, 1, 90]));
        // A part going off returns to neutral.
        assert_eq!(sent(&c, 0b0111), vec![[0xB1, 1, 0], [0xE1, 0x00, 0x40]]);
    }

    #[test]
    fn modulation_defaults_to_the_right_parts() {
        let c = Controllers::new();
        let mut e = [0u8; 4];
        c.control_change(0, 1, 50, &mut e);
        let all = sent(&c, 0b1111);
        assert!(!all.iter().any(|m| m[0] == 0xB1), "Left gets no modulation by default: {all:?}");
    }

    #[test]
    fn control_types() {
        let c = Controllers::new();
        let mut e = [0u8; 4];
        c.set_pedal(0, PedalSetup { control_type: ControlType::Toggle, ..c.pedal(0) });
        c.control_change(0, 64, 127, &mut e);
        c.control_change(0, 64, 0, &mut e);
        assert_eq!(c.switches(), SUSTAIN, "toggle: stays on after release");
        c.control_change(0, 64, 127, &mut e);
        assert_eq!(c.switches(), 0);
        c.control_change(0, 64, 0, &mut e);
        c.set_pedal(0, PedalSetup { control_type: ControlType::HoldB, ..c.pedal(0) });
        c.control_change(0, 64, 127, &mut e);
        assert_eq!(c.switches(), 0);
        c.control_change(0, 64, 0, &mut e);
        assert_eq!(c.switches(), SUSTAIN, "hold B: on while up");
    }

    #[test]
    fn reverse_polarity_and_repeated_values() {
        let c = Controllers::new();
        let mut e = [0u8; 4];
        c.set_pedal(0, PedalSetup { reverse: true, ..c.pedal(0) });
        c.control_change(0, 64, 0, &mut e);
        assert_eq!(c.switches(), SUSTAIN);
        assert_eq!(c.down(), 1);
        // A half-pedal stream of values on the same side changes nothing.
        for v in [10, 20, 30] {
            assert_eq!(c.control_change(0, 64, v, &mut e), Handled::Fire(Fire::default()));
        }
        c.control_change(0, 64, 127, &mut e);
        assert_eq!(c.switches(), 0);
    }

    /// Kbd Harmony/Arpeggio On/Off and Arpeggio Hold take a Control Type (RM p.141): Toggle
    /// runs the function on each press, Hold A sets it on while the pedal is down and Hold B
    /// off while it is down; the control side keeps the switch, so the pedal asks it.
    #[test]
    fn harmony_and_arpeggio_hold_pedals_follow_their_control_type() {
        let c = Controllers::new();
        let mut e = [0u8; 4];
        assert_eq!(Function::KbdHarmonyArp.kind(), Kind::Switch);
        assert_eq!(Function::ArpHold.info().name, "Arpeggio Hold");
        let hold = |ct| PedalSetup { cc: Some(66), function: Function::ArpHold, control_type: ct, ..PedalSetup::default() };
        c.set_pedal(1, hold(ControlType::HoldA));
        let Handled::Fire(f) = c.control_change(0, 66, 127, &mut e) else { panic!() };
        assert_eq!((f.set, f.control), (Some((Function::ArpHold, true)), None));
        let Handled::Fire(f) = c.control_change(0, 66, 127, &mut e) else { panic!() };
        assert_eq!(f.set, None, "nothing while held");
        let Handled::Fire(f) = c.control_change(0, 66, 0, &mut e) else { panic!() };
        assert_eq!(f.set, Some((Function::ArpHold, false)));
        c.set_pedal(1, hold(ControlType::HoldB));
        let Handled::Fire(f) = c.control_change(0, 66, 127, &mut e) else { panic!() };
        assert_eq!(f.set, Some((Function::ArpHold, false)), "hold B: off while down");
        c.control_change(0, 66, 0, &mut e);
        c.set_pedal(1, PedalSetup { function: Function::KbdHarmonyArp, ..hold(ControlType::Toggle) });
        let Handled::Fire(f) = c.control_change(0, 66, 127, &mut e) else { panic!() };
        assert_eq!((f.set, f.control), (None, Some(Function::KbdHarmonyArp)), "toggle: a press runs it");
        let Handled::Fire(f) = c.control_change(0, 66, 0, &mut e) else { panic!() };
        assert_eq!((f.set, f.control), (None, None), "nothing on release");
        // What a new setup does: Hold B picked with the pedal up turns it on; a Hold A pedal
        // held when it is given another function lets its switch go.
        let (a, b) = (hold(ControlType::HoldA), hold(ControlType::HoldB));
        assert_eq!(control_switch_sets(a, b, false, false), [None, Some((Function::ArpHold, true))]);
        let other = PedalSetup { function: Function::StartStop, ..a };
        assert_eq!(control_switch_sets(a, other, true, false), [Some((Function::ArpHold, false)), None]);
        assert_eq!(control_switch_sets(a, other, false, false), [None, None], "it was off");
        assert_eq!(control_switch_sets(a, a, true, true), [None, None], "no change");
        let toggle = hold(ControlType::Toggle);
        assert_eq!(control_switch_sets(other, toggle, false, false), [None, None], "a toggle leaves it as it is");
    }

    /// A reset (Panic, a keyboard unplugged) lets go of the pedals: those that were down
    /// are reported once to the control side, whose switches then follow the pedal up.
    #[test]
    fn a_reset_reports_the_pedals_it_let_go() {
        let c = Controllers::new();
        let mut e = [0u8; 4];
        let hold = |cc, function, control_type| PedalSetup { cc: Some(cc), function, control_type, ..PedalSetup::default() };
        c.set_pedal(0, hold(64, Function::ArpHold, ControlType::HoldA));
        c.set_pedal(1, hold(66, Function::KbdHarmonyArp, ControlType::HoldB));
        c.set_pedal(2, hold(67, Function::ArpHold, ControlType::HoldA));
        c.control_change(0, 64, 127, &mut e);
        c.control_change(0, 66, 127, &mut e);
        assert_eq!(c.down(), 0b011);
        assert_eq!(c.take_reset_releases(), None, "nothing before a reset");
        c.reset(&mut |_| {});
        assert_eq!(c.take_reset_releases(), Some(0b011), "the pedals that were down, not pedal 3");
        assert_eq!(c.take_reset_releases(), None, "once");
        c.reset(&mut |_| {});
        assert_eq!(c.take_reset_releases(), Some(0), "a reset with every pedal up");
        // What each lets go of: what a Hold pedal was keeping on.
        let (a, b) = (c.pedal(0), c.pedal(1));
        assert_eq!(reset_release(a, true), Some(Function::ArpHold), "Hold A, down: on, so off");
        assert_eq!(reset_release(a, false), None, "Hold A, up: not the pedal's");
        assert_eq!(reset_release(b, false), Some(Function::KbdHarmonyArp), "Hold B, up: on, so off");
        assert_eq!(reset_release(b, true), None, "Hold B, down: off already");
        assert_eq!(reset_release(hold(64, Function::ArpHold, ControlType::Toggle), true), None, "a toggle leaves it");
        assert_eq!(reset_release(hold(64, Function::Sustain, ControlType::HoldA), true), None, "a pedal switch: reset does it");
        // Released after the reset: no edge (it counts as up already).
        let f = c.control_change(0, 64, 0, &mut e);
        assert!(matches!(f, Handled::Fire(Fire { set: None, .. })), "{f:?}");
    }

    #[test]
    fn a_pedal_fires_its_function_once_per_press() {
        let c = Controllers::new();
        let mut e = [0u8; 4];
        c.set_pedal(1, PedalSetup { cc: Some(66), function: Function::StartStop, ..PedalSetup::default() });
        c.set_pedal(2, PedalSetup { cc: Some(67), function: Function::OtsNext, ..PedalSetup::default() });
        let Handled::Fire(f) = c.control_change(0, 66, 127, &mut e) else { panic!() };
        assert_eq!(f.engine, Some(Button::StartStop));
        let Handled::Fire(f) = c.control_change(0, 66, 127, &mut e) else { panic!() };
        assert_eq!(f.engine, None, "no repeat while held");
        let Handled::Fire(f) = c.control_change(0, 66, 0, &mut e) else { panic!() };
        assert_eq!(f.engine, None, "nothing on release");
        let Handled::Fire(f) = c.control_change(0, 67, 127, &mut e) else { panic!() };
        assert_eq!(f.control, Some(Function::OtsNext));
        // CC 66 is Pedal 2's now: it no longer works as sostenuto.
        assert_eq!(c.switches(), 0);
    }

    #[test]
    fn unassigned_gm_pedals_still_work_and_other_ccs_pass() {
        let c = Controllers::new();
        let mut e = [0u8; 4];
        c.set_pedal(0, PedalSetup { cc: Some(65), function: Function::FillUp, ..PedalSetup::default() });
        assert_eq!(c.control_change(0, 64, 127, &mut e), Handled::Sync);
        assert_eq!(c.switches(), SUSTAIN);
        assert_eq!(c.control_change(0, 11, 100, &mut e), Handled::Pass);
        assert_eq!(c.control_change(0, 121, 0, &mut e), Handled::SyncAndPass);
        assert_eq!(c.switches(), 0);
    }

    #[test]
    fn continuous_pedals() {
        let p = PedalSetup { range: Range::Upper, ..PedalSetup::default() };
        assert_eq!((p.bend_at(0), p.bend_at(127)), (BEND_CENTRE, 16383));
        let p = PedalSetup { range: Range::Lower, ..p };
        assert_eq!((p.bend_at(0), p.bend_at(127)), (BEND_CENTRE, 0));
        let p = PedalSetup { range: Range::Full, ..p };
        assert_eq!((p.bend_at(0), p.bend_at(127)), (0, 16383));
        let c = Controllers::new();
        let mut e = [0u8; 4];
        c.set_pedal(2, PedalSetup { cc: Some(4), function: Function::Modulation, ..PedalSetup::default() });
        c.control_change(0, 4, 77, &mut e);
        assert_eq!(c.modulation(), 77);
    }

    /// Dynamics Control (#180): a foot controller whose position is the Dynamics level,
    /// every move (Reverse flips it).
    #[test]
    fn a_dynamics_control_pedal_sends_its_position() {
        let c = Controllers::new();
        let mut e = [0u8; 4];
        assert_eq!(Function::DynamicsControl.kind(), Kind::Continuous);
        c.set_pedal(2, PedalSetup { cc: Some(4), function: Function::DynamicsControl, ..PedalSetup::default() });
        for v in [0u8, 30, 64, 127] {
            let Handled::Fire(f) = c.control_change(0, 4, v, &mut e) else { panic!() };
            assert_eq!(f.dynamics, Some(v));
            assert_eq!(f.engine, None);
        }
        c.set_pedal(2, PedalSetup { cc: Some(4), function: Function::DynamicsControl, reverse: true, ..PedalSetup::default() });
        let Handled::Fire(f) = c.control_change(0, 4, 100, &mut e) else { panic!() };
        assert_eq!(f.dynamics, Some(27));
        assert_eq!(c.modulation(), 0, "not the modulation");
    }

    #[test]
    fn learn_takes_the_next_pressed_cc() {
        let c = Controllers::new();
        let mut e = [0u8; 4];
        c.learn(Some(1));
        assert_eq!(c.control_change(0, 1, 127, &mut e), Handled::Sync, "the mod wheel is not learnt");
        assert_eq!(c.control_change(0, 85, 0, &mut e), Handled::Pass, "a release is not learnt");
        assert_eq!(c.control_change(0, 85, 127, &mut e), Handled::Learned);
        assert_eq!(c.pedal(1).cc, Some(85));
        assert_eq!(c.learning(), None);
    }

    #[test]
    fn reset_releases_everything_and_forgets_sources() {
        let c = Controllers::new();
        let mut e = [0u8; 4];
        c.control_change(3, 64, 127, &mut e);
        c.pitch_bend(3, 0);
        assert!(c.touched(3) && !c.touched(2) && c.active());
        sent(&c, 0b0001);
        let mut v = Vec::new();
        c.reset(&mut |m| v.push(m.to_vec()));
        assert_eq!(v.len(), 4 * 5);
        assert!(v.contains(&vec![0xB0, 64, 0]) && v.contains(&vec![0xE3, 0, 0x40]));
        assert!(!c.active() && !c.touched(3));
        // Neutral once more on the next sync (the other thread may have had a sync of its
        // own in hand), then nothing.
        assert_eq!(sent(&c, 0b0001).len(), 4 * 5);
        assert!(sent(&c, 0b0001).is_empty());
    }

    #[test]
    fn bend_ranges_go_out_once_as_rpn_0() {
        let c = Controllers::new();
        let mut v = Vec::new();
        c.sync_ranges(&mut |m| v.push([m[0], m[1], m[2]]));
        assert_eq!(v.len(), 4 * 6);
        assert_eq!(&v[..3], &[[0xB0, 101, 0], [0xB0, 100, 0], [0xB0, 6, 2]]);
        v.clear();
        c.set_bend_range(parts::LEFT, 40);
        c.sync_ranges(&mut |m| v.push([m[0], m[1], m[2]]));
        assert_eq!(v[2], [0xB1, 6, 12], "clamped to 12");
        assert_eq!(v.len(), 6);
    }

    #[test]
    fn a_pedal_given_a_new_setup_releases_what_it_drove() {
        let mut e = [0u8; 4];
        // Held: re-picked while down.
        let c = Controllers::new();
        c.control_change(0, 64, 127, &mut e);
        assert_eq!(c.switches(), SUSTAIN);
        c.set_pedal(0, PedalSetup { function: Function::StartStop, ..c.pedal(0) });
        assert_eq!((c.switches(), c.down()), (0, 0));
        assert_eq!(c.control_change(0, 64, 0, &mut e), Handled::Fire(Fire::default()), "the release does nothing");
        assert_eq!(c.control_change(0, 64, 127, &mut e), Handled::Fire(Fire { shown: true, engine: Some(Button::StartStop), ..Fire::default() }));
        // Toggle: latched on, then re-picked with the pedal up.
        let c = Controllers::new();
        let mut e = [0u8; 4];
        c.set_pedal(0, PedalSetup { control_type: ControlType::Toggle, ..c.pedal(0) });
        c.control_change(0, 64, 127, &mut e);
        c.control_change(0, 64, 0, &mut e);
        assert_eq!(c.switches(), SUSTAIN, "latched");
        c.set_pedal(0, PedalSetup { function: Function::None, ..c.pedal(0) });
        assert_eq!(c.switches(), 0);
        // A new CC (Learn) releases too; a new Control Type alone does not.
        let c = Controllers::new();
        let mut e = [0u8; 4];
        c.control_change(0, 64, 127, &mut e);
        c.set_pedal(0, PedalSetup { reverse: true, ..c.pedal(0) });
        assert_eq!(c.switches(), SUSTAIN);
        c.learn(Some(0));
        assert_eq!(c.control_change(0, 85, 127, &mut e), Handled::Learned);
        assert_eq!(c.switches(), 0);
        // Another pedal held on the same switch keeps it on.
        let c = Controllers::new();
        let mut e = [0u8; 4];
        c.set_pedal(1, PedalSetup { cc: Some(80), function: Function::Sustain, ..PedalSetup::default() });
        c.control_change(0, 64, 127, &mut e);
        c.control_change(0, 80, 127, &mut e);
        c.set_pedal(0, PedalSetup { function: Function::None, ..c.pedal(0) });
        assert_eq!(c.switches(), SUSTAIN);
        // A Pitch Bend pedal mid-sweep: the bend centres.
        let c = Controllers::new();
        let mut e = [0u8; 4];
        c.set_pedal(2, PedalSetup { cc: Some(4), function: Function::PitchBend, ..PedalSetup::default() });
        c.control_change(0, 4, 127, &mut e);
        assert_eq!(c.bend(), 16383);
        assert_eq!(sent(&c, 0b0001), vec![[0xE0, 0x7F, 0x7F]]);
        c.set_pedal(2, PedalSetup { function: Function::None, ..c.pedal(2) });
        assert_eq!(sent(&c, 0b0001), vec![[0xE0, 0, 0x40]], "centred on the next sync");
        c.control_change(0, 4, 0, &mut e);
        assert_eq!(c.bend(), BEND_CENTRE);
    }

    #[test]
    fn after_a_reset_the_next_press_is_a_press() {
        let c = Controllers::new();
        let mut e = [0u8; 4];
        c.control_change(0, 64, 127, &mut e);
        c.reset(&mut |_| {});
        assert_eq!(c.down(), 0, "the lamp goes out");
        c.control_change(0, 64, 127, &mut e);
        assert_eq!((c.switches(), c.down()), (SUSTAIN, 1), "not swallowed");
    }

    /// The synth's view: each keyboard channel's CC 64, from messages in the order they
    /// were flushed.
    struct Wire([u8; 4]);

    impl Wire {
        fn flush(&mut self, buf: &mut Vec<[u8; 3]>) {
            for m in buf.drain(..) {
                if m[0] & 0xF0 == 0xB0 && m[1] == 64 && (m[0] & 0x0F) < 4 {
                    self.0[(m[0] & 0x0F) as usize] = m[2];
                }
            }
        }
    }

    /// One thread's sync as live.rs does it: claim, sync into its own buffer.
    fn sync_into(c: &Controllers, sounding: u8, buf: &mut Vec<[u8; 3]>) -> bool {
        let owned = c.claim();
        if owned {
            c.sync(sounding, &mut |m| buf.push([m[0], m[1], m[2]]));
        }
        owned
    }

    /// After the flush: release, and sync and flush again while the other thread asked.
    fn release_into(c: &Controllers, sounding: u8, buf: &mut Vec<[u8; 3]>, wire: &mut Wire) {
        while c.release() {
            if !sync_into(c, sounding, buf) {
                break;
            }
            wire.flush(buf);
        }
    }

    /// Input and engine threads send through their own buffers, flushed at different times.
    /// Whatever the interleaving, the channel ends up with what `sent_*` says.
    #[test]
    fn two_threads_never_leave_the_channel_behind() {
        // The input thread syncs a pedal press (Right 1 sounds); before it flushes, the
        // engine switches Right 1 off and flushes first.
        let c = Controllers::new();
        let (mut input, mut engine, mut wire) = (Vec::new(), Vec::new(), Wire([0; 4]));
        let mut e = [0u8; 4];
        c.control_change(0, 64, 127, &mut e);
        assert!(sync_into(&c, 0b0001, &mut input));
        assert!(!sync_into(&c, 0b0000, &mut engine), "the input thread has it");
        wire.flush(&mut engine);
        wire.flush(&mut input);
        release_into(&c, 0b0000, &mut input, &mut wire);
        assert_eq!(wire.0[0], 0, "Right 1 is off: released on the wire, as sent_* says");
        assert!(sent(&c, 0b0000).is_empty());

        // The other way round: the engine syncs a part coming on; the pedal is released on
        // the input thread before the engine flushes.
        let c = Controllers::new();
        let (mut input, mut engine, mut wire) = (Vec::new(), Vec::new(), Wire([0; 4]));
        let mut e = [0u8; 4];
        c.control_change(0, 64, 127, &mut e);
        assert!(sync_into(&c, 0b0011, &mut engine));
        c.control_change(0, 64, 0, &mut e);
        assert!(!sync_into(&c, 0b0011, &mut input));
        wire.flush(&mut input);
        wire.flush(&mut engine);
        release_into(&c, 0b0011, &mut engine, &mut wire);
        assert_eq!(wire.0, [0; 4], "the release went out after the press");

        // A Panic on the engine thread while the input thread holds a press not yet flushed.
        let c = Controllers::new();
        let (mut input, mut engine, mut wire) = (Vec::new(), Vec::new(), Wire([0; 4]));
        let mut e = [0u8; 4];
        c.control_change(0, 64, 127, &mut e);
        assert!(sync_into(&c, 0b0001, &mut input));
        c.reset(&mut |m| engine.push([m[0], m[1], m[2]]));
        assert!(!sync_into(&c, 0b0001, &mut engine));
        wire.flush(&mut engine);
        wire.flush(&mut input);
        assert_eq!(wire.0[0], 127, "the stale press landed after the reset");
        release_into(&c, 0b0001, &mut input, &mut wire);
        assert_eq!(wire.0[0], 0, "and neutral follows it");
    }

    /// RM p.139: Hold B "turns the function off and keeps it inactive while holding down",
    /// so it is on whenever the pedal is up, from the moment it is picked.
    #[test]
    fn hold_b_follows_the_pedal_position() {
        let hold_b = |c: &Controllers| PedalSetup { control_type: ControlType::HoldB, ..c.pedal(0) };
        // Picked with the pedal up: on at once.
        let c = Controllers::new();
        c.set_pedal(0, hold_b(&c));
        assert_eq!(c.switches(), SUSTAIN);
        let mut e = [0u8; 4];
        c.control_change(0, 64, 127, &mut e);
        assert_eq!(c.switches(), 0);
        c.control_change(0, 64, 0, &mut e);
        assert_eq!(c.switches(), SUSTAIN);
        // Hold A to Hold B while held: off now, on at the release.
        let c = Controllers::new();
        let mut e = [0u8; 4];
        c.control_change(0, 64, 127, &mut e);
        assert_eq!(c.switches(), SUSTAIN);
        c.set_pedal(0, hold_b(&c));
        assert_eq!(c.switches(), 0, "held down: off");
        c.control_change(0, 64, 0, &mut e);
        assert_eq!(c.switches(), SUSTAIN, "up: on");
        // Back to Hold A with the pedal up: off.
        c.set_pedal(0, PedalSetup { control_type: ControlType::HoldA, ..c.pedal(0) });
        assert_eq!(c.switches(), 0);
        // A Hold B pedal that is up keeps the switch on when a Hold A pedal on it is released.
        let c = Controllers::new();
        let mut e = [0u8; 4];
        c.set_pedal(1, PedalSetup { cc: Some(80), function: Function::Sustain, control_type: ControlType::HoldB, ..PedalSetup::default() });
        c.control_change(0, 64, 127, &mut e);
        c.control_change(0, 64, 0, &mut e);
        assert_eq!(c.switches(), SUSTAIN);
    }

    /// Two keyboards, each with a sustain pedal on CC 64: Sustain stays on until both are up.
    #[test]
    fn a_pedal_is_down_while_any_keyboard_holds_it() {
        let c = Controllers::new();
        let mut e = [0u8; 4];
        c.control_change(0, 64, 127, &mut e);
        c.control_change(1, 64, 127, &mut e);
        assert_eq!(c.control_change(0, 64, 0, &mut e), Handled::Fire(Fire::default()), "B still holds it");
        assert_eq!((c.switches(), c.down()), (SUSTAIN, 1));
        c.control_change(1, 64, 0, &mut e);
        assert_eq!((c.switches(), c.down()), (0, 0));
        // A trigger fires for a press on either keyboard.
        c.set_pedal(1, PedalSetup { cc: Some(66), function: Function::TempoUp, ..PedalSetup::default() });
        let Handled::Fire(f) = c.control_change(0, 66, 127, &mut e) else { panic!() };
        assert_eq!(f.engine, Some(Button::TempoUp));
        let Handled::Fire(f) = c.control_change(1, 66, 127, &mut e) else { panic!() };
        assert_eq!(f.engine, Some(Button::TempoUp));
        // A reset forgets every keyboard's edges.
        c.reset(&mut |_| {});
        c.control_change(0, 64, 127, &mut e);
        c.control_change(0, 64, 0, &mut e);
        assert_eq!(c.switches(), 0);
    }

    #[test]
    fn pedal_ccs_learn_can_take_are_the_ones_setup_can() {
        for cc in 0..=255u8 {
            assert_eq!(learnable(cc), pedal_cc_refused(cc).is_none());
        }
        for cc in [0, 1, 6, 7, 32, 38, 99, 121, 123, 128] {
            assert!(pedal_cc_refused(cc).is_some(), "{cc}");
        }
        for cc in [4, 11, 64, 66, 67, 85, 119] {
            assert!(pedal_cc_refused(cc).is_none(), "{cc}");
        }
    }
}

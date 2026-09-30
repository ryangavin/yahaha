//! Knob Assign pages for the Launchkey's 8 encoders (#197): the Genos LIVE CONTROL knobs
//! (OM p.62-63, RM p.145-148). A page gives each knob a function; KNOB ASSIGN (the
//! encoder page buttons) steps through the pages. The knobs are relative, as the Genos's
//! are: a turn moves the value from where it is now (OM p.63), whoever set it last.
//!
//! This is the pure model: the session hands it the values in effect (`Now`) and runs the
//! command a turn gives back, the same command the app's control for it sends.

use crate::api::{AppCmd, ChordCmd, DynamicsCmd, FxBlock, FxCmd, KnobState, KnobsState, HarmonyArpCmd, PartSend, MetronomeCmd, MixerCmd, PartsCmd, StripCmd, StyleSettingsCmd, TrackMuteOrder, TransportCmd};
use crate::engine::RETRIGGER_RATES;
use crate::fx::{INSERT_SLOTS, INSERT_VALUES, KnobSpec, Param, SENDS};
use crate::parts::FaderRoute;
use crate::racks::{ControlMap, ControlTarget};
use serde::{Deserialize, Serialize};

/// A Knob Assign page.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum KnobPage {
    /// The Style's live functions: Dynamics, Retrigger, Track Mute A/B, tempo.
    #[default]
    Style,
    /// The live rack's controller map (docs/racks.md): its knobs 1-8. The default map is
    /// the page this was before racks: the keyboard parts' volumes, Harmony, the metronome,
    /// tempo. (A `parts` page from before racks opens this one.)
    #[serde(alias = "parts")]
    Rack,
    /// The keyboard parts' pan, tempo.
    Pan,
    /// One page per effect block: knobs 1-4 the keyboard parts' sends to it (Right 1-3,
    /// Left; CC91), 5-8 its most useful parameters, the return level last. Reverb: time,
    /// pre-delay, tone, return. (An `effects` page from before these pages opens this one.)
    #[serde(alias = "effects")]
    Reverb,
    /// The keyboard parts' chorus sends (CC93); the chorus's rate and depth, its return.
    Chorus,
    /// The keyboard parts' delay sends (CC94, the Variation block); the delay's time,
    /// feedback and tone (its high cut), its return. (An `fx` page opens this one.)
    #[serde(alias = "fx")]
    Delay,
}

impl KnobPage {
    pub const ALL: [KnobPage; 6] = [KnobPage::Style, KnobPage::Rack, KnobPage::Pan, KnobPage::Reverb, KnobPage::Chorus, KnobPage::Delay];

    pub fn name(self) -> &'static str {
        match self {
            KnobPage::Style => "Style",
            KnobPage::Rack => "Rack",
            KnobPage::Pan => "Pan",
            KnobPage::Reverb => "Reverb",
            KnobPage::Chorus => "Chorus",
            KnobPage::Delay => "Delay",
        }
    }

    pub fn index(self) -> usize {
        self as usize
    }

    /// The page `d` steps away, stopping at the first and last.
    pub fn step(self, d: i8) -> KnobPage {
        KnobPage::ALL[(self as i16 + d as i16).clamp(0, KnobPage::ALL.len() as i16 - 1) as usize]
    }

    /// Knobs 1-8. Tempo is knob 8 on the Style and Pan pages (and the default Rack page);
    /// on an effect page knobs 1-4 are the parts' sends to it and knob 8 its return. The
    /// Rack page's are the default controller map's (`Knobs::function` has the live rack's).
    pub fn functions(self) -> [KnobFn; 8] {
        use KnobFn::*;
        match self {
            KnobPage::Style => [Dynamics, RetriggerRate, RetriggerOnOff, TrackMuteA, TrackMuteB, Swing, None, Tempo],
            KnobPage::Rack => rack_functions(&ControlMap::default()),
            KnobPage::Pan => [PartPan(0), PartPan(1), PartPan(2), PartPan(3), FxReturn(0), FxReturn(1), FxReturn(2), Tempo],
            KnobPage::Reverb => [
                PartReverb(0),
                PartReverb(1),
                PartReverb(2),
                PartReverb(3),
                FxParam(Param::ReverbTime),
                FxParam(Param::PreDelay),
                FxParam(Param::ReverbTone),
                FxReturn(0),
            ],
            // The chorus has no feedback parameter: knob 7 is unassigned, so the return
            // stays on knob 8 as on the other effect pages.
            KnobPage::Chorus => [
                PartChorus(0),
                PartChorus(1),
                PartChorus(2),
                PartChorus(3),
                FxParam(Param::ChorusRate),
                FxParam(Param::ChorusDepth),
                None,
                FxReturn(1),
            ],
            KnobPage::Delay => [
                PartDelay(0),
                PartDelay(1),
                PartDelay(2),
                PartDelay(3),
                DelayTime,
                FxParam(Param::DelayFeedback),
                FxParam(Param::DelayTone),
                FxReturn(2),
            ],
        }
    }
}

/// What a knob does (RM p.146-148 names in the comments).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KnobFn {
    /// No Assign (`---`).
    None,
    /// DynCtrl: the Style Dynamics level, 0-127.
    Dynamics,
    /// RtgRate: the Retrigger length; turning right makes it shorter.
    RetriggerRate,
    /// RtgOnOff: right turns Retrigger on, left off.
    RetriggerOnOff,
    /// StyMuteA / StyMuteB: fully left one Style part plays; turning right adds the others.
    TrackMuteA,
    TrackMuteB,
    /// Tempo (Master Tempo), 1 BPM a step.
    Tempo,
    /// Swing, 0-100 % (engine/swing.rs), 2 % a step. Not a Genos Live Control function.
    Swing,
    /// Mixer Volume of a keyboard part (0-3: Right 1-3, Left): its CC7.
    PartVolume(u8),
    /// HarmVol: the Keyboard Harmony volume.
    HarmonyVolume,
    /// The metronome's volume.
    MetronomeVolume,
    /// Mixer Pan of a keyboard part (0-3): its CC10, 64 = centre.
    PartPan(u8),
    /// Mixer Reverb / Chorus / Delay depth of a keyboard part (0-3): its CC91 / CC93 /
    /// CC94 (the Variation block, the tempo delay).
    PartReverb(u8),
    PartChorus(u8),
    PartDelay(u8),
    /// The effect bus's return level (#204) of block 0-2: Reverb (the Genos Ambience
    /// knob's job here), Chorus, Variation (the delay).
    FxReturn(u8),
    /// An effect parameter (#236), in its own unit and step (`Param::spec`).
    FxParam(Param),
    /// The delay's time: its note value with tempo sync on (a step every 3 knob steps),
    /// its free time in ms with it off.
    DelayTime,
    /// The HARMONY/ARPEGGIO switch: right turns it on, left off (stepped, as Retrigger
    /// On/Off).
    HarmonyArp,
    /// The split point, a semitone a step.
    SplitPoint,
    /// A keyboard part's (0-3) insert slot (0-1) on or off: right turns it on, left off
    /// (stepped, as Retrigger On/Off). `setStripInsertOn`.
    InsertOn(u8, u8),
    /// Setting (0-3) of a keyboard part's (0-3) insert slot (0-1), across its kind's
    /// range. `setStripInsertSetting`; nothing when the kind has no such setting.
    InsertSetting(u8, u8, u8),
    /// A keyboard part's (0-3) level to added send effect 4-6 (3-5). `setStripSend`;
    /// nothing while that send isn't there. (Sends 1-3 are `PartReverb` / `PartChorus` /
    /// `PartDelay`.)
    PartSend(u8, u8),
    /// The rotary speaker's speed: right fast, left slow (stepped). `setRotaryFast`.
    RotaryFast,
}

/// One insert slot as the knobs and faders read it: on or off, its values, and its kind's
/// settings (none for an empty slot).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct InsertNow {
    pub on: bool,
    pub values: [u16; INSERT_VALUES],
    pub specs: &'static [KnobSpec],
}

/// The keyboard parts' channel strips as the knobs and faders read them (the strips of
/// `api::Strips`), and the rotary speed. Default: nothing known, so an insert setting or a
/// send 4-6 does nothing.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct StripNow {
    /// Each keyboard part's insert slots 1-2.
    pub inserts: [[InsertNow; INSERT_SLOTS]; 4],
    /// Each keyboard part's level to sends 1-6.
    pub sends: [[u8; SENDS]; 4],
    /// The send effects there are (3-6): a send past them has no level to set.
    pub send_count: u8,
    /// The rotary inserts at their fast speed (`setRotaryFast`).
    pub rotary_fast: bool,
}

impl StripNow {
    fn insert(&self, part: u8, slot: u8) -> &InsertNow {
        &self.inserts[(part & 3) as usize][(slot as usize).min(INSERT_SLOTS - 1)]
    }

    /// Insert setting `setting` of a part's slot: its spec and value, if the kind has it.
    fn setting(&self, part: u8, slot: u8, setting: u8) -> Option<(&'static KnobSpec, u16)> {
        let i = self.insert(part, slot);
        let spec = i.specs.get(setting as usize)?;
        Some((spec, spec.clamp(i.values[setting as usize])))
    }

    /// A part's level to send `send`, if that send is there.
    fn send(&self, part: u8, send: u8) -> Option<u8> {
        (send < self.send_count).then(|| self.sends[(part & 3) as usize][(send as usize).min(SENDS - 1)])
    }
}

/// How far a knob step moves an insert setting: its range in about 64 steps (2 for a
/// 0-127 setting, as the levels).
fn setting_step(s: &KnobSpec) -> i32 {
    ((s.max - s.min) as u32).div_ceil(64).max(1) as i32
}

/// Where an insert setting sits in its range, 0-127.
fn setting_level(s: &KnobSpec, v: u16) -> u8 {
    ((s.clamp(v) - s.min) as u32 * 127 / (s.max - s.min).max(1) as u32) as u8
}

/// What a controller map target does on a knob. A target this build doesn't know does
/// nothing.
pub fn rack_function(t: &ControlTarget) -> KnobFn {
    match *t {
        ControlTarget::PartLevel { part } => KnobFn::PartVolume(part & 3),
        ControlTarget::PartPan { part } => KnobFn::PartPan(part & 3),
        ControlTarget::PartReverb { part } => KnobFn::PartReverb(part & 3),
        ControlTarget::PartChorus { part } => KnobFn::PartChorus(part & 3),
        ControlTarget::HarmonyArp => KnobFn::HarmonyArp,
        ControlTarget::SplitPoint => KnobFn::SplitPoint,
        ControlTarget::HarmonyVolume => KnobFn::HarmonyVolume,
        ControlTarget::MetronomeVolume => KnobFn::MetronomeVolume,
        ControlTarget::Tempo => KnobFn::Tempo,
        ControlTarget::PartSend { part, send: 0 } => KnobFn::PartReverb(part & 3),
        ControlTarget::PartSend { part, send: 1 } => KnobFn::PartChorus(part & 3),
        ControlTarget::PartSend { part, send: 2 } | ControlTarget::PartDelay { part } => KnobFn::PartDelay(part & 3),
        ControlTarget::PartSend { part, send } => KnobFn::PartSend(part & 3, send.min(SENDS as u8 - 1)),
        ControlTarget::PartInsertOn { part, slot } => KnobFn::InsertOn(part & 3, slot & 1),
        ControlTarget::PartInsertSetting { part, slot, setting } => KnobFn::InsertSetting(part & 3, slot & 1, setting & 3),
        ControlTarget::RotaryFast => KnobFn::RotaryFast,
        ControlTarget::None | ControlTarget::Unknown(_) => KnobFn::None,
    }
}

/// The command a fader at `v` (0-127) runs for controller map target `t`
/// (`moveRackFader`): a level, pan or send set to `v`, a switch (Harmony/Arp, an insert
/// slot, the rotary's speed) on from 64 (None when it is that already: `harmony_arp` and
/// `strips` say what they are now), the split point and an insert setting across their
/// range. None for none, the tempo, a send or setting that isn't there, and a target this
/// build doesn't know.
pub fn fader_command_at(t: &ControlTarget, v: u8, harmony_arp: bool, strips: &StripNow) -> Option<AppCmd> {
    let v = v.min(127);
    let on = v >= 64;
    Some(match *t {
        ControlTarget::PartLevel { part } => PartsCmd::SetPartVolume { part, volume: v }.into(),
        ControlTarget::PartPan { part } => PartsCmd::SetPartPan { part, pan: v }.into(),
        ControlTarget::PartReverb { part } => PartsCmd::SetPartSend { part, send: PartSend::Reverb, value: v }.into(),
        ControlTarget::PartChorus { part } => PartsCmd::SetPartSend { part, send: PartSend::Chorus, value: v }.into(),
        ControlTarget::HarmonyArp => {
            if on == harmony_arp {
                return None;
            }
            HarmonyArpCmd::SetHarmonyArpOn { on }.into()
        }
        ControlTarget::SplitPoint => ChordCmd::SetSplit { note: split_at(v) }.into(),
        ControlTarget::HarmonyVolume => HarmonyArpCmd::SetHarmonyVolume { volume: v }.into(),
        ControlTarget::MetronomeVolume => MetronomeCmd::SetMetronomeVolume { volume: v }.into(),
        ControlTarget::PartSend { part, send: 0 } => PartsCmd::SetPartSend { part, send: PartSend::Reverb, value: v }.into(),
        ControlTarget::PartSend { part, send: 1 } => PartsCmd::SetPartSend { part, send: PartSend::Chorus, value: v }.into(),
        ControlTarget::PartSend { part, send: 2 } | ControlTarget::PartDelay { part } => {
            PartsCmd::SetPartSend { part, send: PartSend::Variation, value: v }.into()
        }
        ControlTarget::PartSend { part, send } => {
            strips.send(part, send)?;
            StripCmd::SetStripSend { strip: part & 3, send, level: v }.into()
        }
        ControlTarget::PartInsertOn { part, slot } => {
            if on == strips.insert(part, slot).on {
                return None;
            }
            StripCmd::SetStripInsertOn { strip: part & 3, slot: slot & 1, on }.into()
        }
        ControlTarget::PartInsertSetting { part, slot, setting } => {
            let (s, now) = strips.setting(part, slot, setting)?;
            let value = s.min + (v as u32 * (s.max - s.min) as u32 / 127) as u16;
            if value == now {
                return None;
            }
            StripCmd::SetStripInsertSetting { strip: part & 3, slot: slot & 1, setting, value }.into()
        }
        ControlTarget::RotaryFast => {
            if on == strips.rotary_fast {
                return None;
            }
            FxCmd::SetRotaryFast { on }.into()
        }
        ControlTarget::Tempo | ControlTarget::None | ControlTarget::Unknown(_) => return None,
    })
}

/// What each Panel fader 1-4 does on the input thread for controller map `m`: its own
/// part's level stays there; none (and the tempo, which no fader has) does nothing; any
/// other target goes to the control side.
pub fn fader_routes(m: &ControlMap) -> [FaderRoute; 4] {
    std::array::from_fn(|f| match &m.faders[f] {
        ControlTarget::PartLevel { part } if *part as usize == f => FaderRoute::Own,
        ControlTarget::None | ControlTarget::Tempo | ControlTarget::Unknown(_) => FaderRoute::Off,
        _ => FaderRoute::Control,
    })
}

/// The Rack page's knobs 1-8 for controller map `m`.
pub fn rack_functions(m: &ControlMap) -> [KnobFn; 8] {
    std::array::from_fn(|k| rack_function(&m.knobs[k]))
}

/// The split point's range (`ChordCmd::SetSplit`).
pub const SPLIT_MIN: u8 = 24;
pub const SPLIT_MAX: u8 = 96;
/// The split point a double-click puts back (the Genos default, F#2).
pub const DEFAULT_SPLIT: u8 = 54;

/// Knob steps per Retrigger length, and per Retrigger on/off switch.
const RTG_STEPS: i16 = 3;
/// A Track Mute knob's position moves this much per step (0-127: about four steps per part).
const MUTE_STEP: i16 = 4;
/// Levels (0-127) move this much per step.
const LEVEL_STEP: i16 = 2;
/// The tempo range a knob turns through (the engine's).
const MIN_BPM: i32 = 5;
const MAX_BPM: i32 = 500;

impl KnobFn {
    /// The wire name of the function.
    pub fn id(self) -> &'static str {
        match self {
            KnobFn::None => "none",
            KnobFn::Dynamics => "dynamics",
            KnobFn::RetriggerRate => "retriggerRate",
            KnobFn::RetriggerOnOff => "retriggerOnOff",
            KnobFn::TrackMuteA => "trackMuteA",
            KnobFn::TrackMuteB => "trackMuteB",
            KnobFn::Tempo => "tempo",
            KnobFn::Swing => "swing",
            KnobFn::PartVolume(_) => "partVolume",
            KnobFn::HarmonyVolume => "harmonyVolume",
            KnobFn::MetronomeVolume => "metronomeVolume",
            KnobFn::PartPan(_) => "partPan",
            KnobFn::PartReverb(_) => "partReverb",
            KnobFn::PartChorus(_) => "partChorus",
            KnobFn::PartDelay(_) => "partDelay",
            KnobFn::FxReturn(_) => "fxReturn",
            KnobFn::FxParam(_) => "fxParam",
            KnobFn::DelayTime => "delayTime",
            KnobFn::HarmonyArp => "harmonyArp",
            KnobFn::SplitPoint => "splitPoint",
            KnobFn::InsertOn(..) => "insertOn",
            KnobFn::InsertSetting(..) => "insertSetting",
            KnobFn::PartSend(..) => "partSend",
            KnobFn::RotaryFast => "rotaryFast",
        }
    }

    /// A short name, up to 8 characters (the Launchkey display's eight-name page).
    pub fn short(self) -> &'static str {
        match self {
            KnobFn::None => "---",
            KnobFn::Dynamics => "DynCtrl",
            KnobFn::RetriggerRate => "RtgRate",
            KnobFn::RetriggerOnOff => "RtgOnOff",
            KnobFn::TrackMuteA => "StyMuteA",
            KnobFn::TrackMuteB => "StyMuteB",
            KnobFn::Tempo => "Tempo",
            KnobFn::Swing => "Swing",
            KnobFn::PartVolume(p) => ["Right1", "Right2", "Right3", "Left"][(p & 3) as usize],
            KnobFn::HarmonyVolume => "HarmVol",
            KnobFn::MetronomeVolume => "MetroVol",
            KnobFn::PartPan(p) => ["PanR1", "PanR2", "PanR3", "PanL"][(p & 3) as usize],
            KnobFn::PartReverb(p) => ["RevR1", "RevR2", "RevR3", "RevL"][(p & 3) as usize],
            KnobFn::PartChorus(p) => ["ChoR1", "ChoR2", "ChoR3", "ChoL"][(p & 3) as usize],
            KnobFn::PartDelay(p) => ["DlyR1", "DlyR2", "DlyR3", "DlyL"][(p & 3) as usize],
            KnobFn::FxReturn(b) => ["RevRtn", "ChoRtn", "DlyRtn"][(b as usize).min(2)],
            KnobFn::FxParam(p) => p.spec().short,
            KnobFn::DelayTime => "DlyTime",
            KnobFn::HarmonyArp => "HarmArp",
            KnobFn::SplitPoint => "Split",
            KnobFn::InsertOn(p, s) => INSERT_ON_SHORT[(p & 3) as usize][(s & 1) as usize],
            KnobFn::InsertSetting(p, s, i) => INSERT_SETTING_SHORT[(p & 3) as usize][(s & 1) as usize][(i & 3) as usize],
            KnobFn::PartSend(p, s) => SEND_SHORT[(p & 3) as usize][(s as usize).clamp(3, SENDS - 1) - 3],
            KnobFn::RotaryFast => "Rotary",
        }
    }

    /// The full name.
    pub fn name(self) -> &'static str {
        match self {
            KnobFn::None => "No Assign",
            KnobFn::Dynamics => "Dynamics Control",
            KnobFn::RetriggerRate => "Retrigger Rate",
            KnobFn::RetriggerOnOff => "Retrigger On/Off",
            KnobFn::TrackMuteA => "Style Track Mute A",
            KnobFn::TrackMuteB => "Style Track Mute B",
            KnobFn::Tempo => "Tempo",
            KnobFn::Swing => "Swing",
            KnobFn::PartVolume(p) => ["Right 1 Volume", "Right 2 Volume", "Right 3 Volume", "Left Volume"][(p & 3) as usize],
            KnobFn::HarmonyVolume => "Harmony Volume",
            KnobFn::MetronomeVolume => "Metronome Volume",
            KnobFn::PartPan(p) => ["Right 1 Pan", "Right 2 Pan", "Right 3 Pan", "Left Pan"][(p & 3) as usize],
            KnobFn::PartReverb(p) => ["Right 1 Reverb", "Right 2 Reverb", "Right 3 Reverb", "Left Reverb"][(p & 3) as usize],
            KnobFn::PartChorus(p) => ["Right 1 Chorus", "Right 2 Chorus", "Right 3 Chorus", "Left Chorus"][(p & 3) as usize],
            KnobFn::PartDelay(p) => ["Right 1 Delay", "Right 2 Delay", "Right 3 Delay", "Left Delay"][(p & 3) as usize],
            KnobFn::FxReturn(b) => ["Reverb Return", "Chorus Return", "Delay Return"][(b as usize).min(2)],
            KnobFn::FxParam(p) => p.spec().full,
            KnobFn::DelayTime => "Delay Time",
            KnobFn::HarmonyArp => "Harmony/Arpeggio",
            KnobFn::SplitPoint => "Split Point",
            KnobFn::InsertOn(p, s) => INSERT_ON_NAME[(p & 3) as usize][(s & 1) as usize],
            KnobFn::InsertSetting(p, s, i) => INSERT_SETTING_NAME[(p & 3) as usize][(s & 1) as usize][(i & 3) as usize],
            KnobFn::PartSend(p, s) => SEND_NAME[(p & 3) as usize][(s as usize).clamp(3, SENDS - 1) - 3],
            KnobFn::RotaryFast => "Rotary Fast/Slow",
        }
    }
}

/// The strip functions' names, by part (Right 1-3, Left), slot and setting.
const INSERT_ON_SHORT: [[&str; 2]; 4] = [["R1 Ins1", "R1 Ins2"], ["R2 Ins1", "R2 Ins2"], ["R3 Ins1", "R3 Ins2"], ["L Ins1", "L Ins2"]];
const INSERT_ON_NAME: [[&str; 2]; 4] = [
    ["Right 1 Insert 1 On/Off", "Right 1 Insert 2 On/Off"],
    ["Right 2 Insert 1 On/Off", "Right 2 Insert 2 On/Off"],
    ["Right 3 Insert 1 On/Off", "Right 3 Insert 2 On/Off"],
    ["Left Insert 1 On/Off", "Left Insert 2 On/Off"],
];
const INSERT_SETTING_SHORT: [[[&str; 4]; 2]; 4] = [
    [["R1 I1.1", "R1 I1.2", "R1 I1.3", "R1 I1.4"], ["R1 I2.1", "R1 I2.2", "R1 I2.3", "R1 I2.4"]],
    [["R2 I1.1", "R2 I1.2", "R2 I1.3", "R2 I1.4"], ["R2 I2.1", "R2 I2.2", "R2 I2.3", "R2 I2.4"]],
    [["R3 I1.1", "R3 I1.2", "R3 I1.3", "R3 I1.4"], ["R3 I2.1", "R3 I2.2", "R3 I2.3", "R3 I2.4"]],
    [["L I1.1", "L I1.2", "L I1.3", "L I1.4"], ["L I2.1", "L I2.2", "L I2.3", "L I2.4"]],
];
const INSERT_SETTING_NAME: [[[&str; 4]; 2]; 4] = [
    [
        ["Right 1 Insert 1 Setting 1", "Right 1 Insert 1 Setting 2", "Right 1 Insert 1 Setting 3", "Right 1 Insert 1 Setting 4"],
        ["Right 1 Insert 2 Setting 1", "Right 1 Insert 2 Setting 2", "Right 1 Insert 2 Setting 3", "Right 1 Insert 2 Setting 4"],
    ],
    [
        ["Right 2 Insert 1 Setting 1", "Right 2 Insert 1 Setting 2", "Right 2 Insert 1 Setting 3", "Right 2 Insert 1 Setting 4"],
        ["Right 2 Insert 2 Setting 1", "Right 2 Insert 2 Setting 2", "Right 2 Insert 2 Setting 3", "Right 2 Insert 2 Setting 4"],
    ],
    [
        ["Right 3 Insert 1 Setting 1", "Right 3 Insert 1 Setting 2", "Right 3 Insert 1 Setting 3", "Right 3 Insert 1 Setting 4"],
        ["Right 3 Insert 2 Setting 1", "Right 3 Insert 2 Setting 2", "Right 3 Insert 2 Setting 3", "Right 3 Insert 2 Setting 4"],
    ],
    [
        ["Left Insert 1 Setting 1", "Left Insert 1 Setting 2", "Left Insert 1 Setting 3", "Left Insert 1 Setting 4"],
        ["Left Insert 2 Setting 1", "Left Insert 2 Setting 2", "Left Insert 2 Setting 3", "Left Insert 2 Setting 4"],
    ],
];
/// Sends 4-6.
const SEND_SHORT: [[&str; 3]; 4] = [["R1 Snd4", "R1 Snd5", "R1 Snd6"], ["R2 Snd4", "R2 Snd5", "R2 Snd6"], ["R3 Snd4", "R3 Snd5", "R3 Snd6"], ["L Snd4", "L Snd5", "L Snd6"]];
const SEND_NAME: [[&str; 3]; 4] = [
    ["Right 1 Send 4", "Right 1 Send 5", "Right 1 Send 6"],
    ["Right 2 Send 4", "Right 2 Send 5", "Right 2 Send 6"],
    ["Right 3 Send 4", "Right 3 Send 5", "Right 3 Send 6"],
    ["Left Send 4", "Left Send 5", "Left Send 6"],
];

/// The values in effect, as the control side knows them.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Now {
    pub dynamics: u8,
    pub retrigger: bool,
    pub retrigger_rate: u8,
    /// Swing, 0-100 %.
    pub swing: u8,
    pub bpm: f64,
    pub part_volume: [u8; 4],
    pub harmony_volume: u8,
    pub metronome_volume: u8,
    /// Each keyboard part's pan and sends (`parts::Parts::fx`).
    pub part_fx: [[u8; crate::parts::FX]; 4],
    /// The effect bus's return levels: Reverb, Chorus, Variation (#204).
    pub fx_return: [u8; 3],
    /// The effect parameters (#236, `Param::index`).
    pub fx_params: [u16; crate::fx::PARAMS],
    /// The effect parameters' defaults for each block's current type (what a reset goes to).
    pub fx_defaults: [u16; crate::fx::PARAMS],
    /// The HARMONY/ARPEGGIO switch.
    pub harmony_arp: bool,
    /// The split point (a MIDI note).
    pub split: u8,
}

/// A keyboard part's, and the Harmony's, volume a double-click puts back (the Genos default).
pub const DEFAULT_VOLUME: u8 = 100;

/// A knob as it reads now: its value as text, and where it is (0-127) if it has a
/// position (the Genos LED ring; Tempo has none).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Reading {
    pub value: String,
    pub level: Option<u8>,
}

/// The knobs: the page, the Rack page's functions, and what a turn carries over.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Knobs {
    pub page: KnobPage,
    /// The Rack page's knobs, from the live rack's controller map (`set_rack`).
    rack: [KnobFn; 8],
    /// Steps turned toward the next switch of a stepped function (Retrigger), per knob.
    acc: [i16; 8],
    /// The Track Mute A and B knob positions (0-127). They set the Style parts' switches
    /// and keep no value of their own, so the knob is where it was last turned to.
    mute: [u8; 2],
}

impl Default for Knobs {
    fn default() -> Knobs {
        Knobs { page: KnobPage::default(), rack: rack_functions(&ControlMap::default()), acc: [0; 8], mute: [127; 2] }
    }
}

impl Knobs {
    pub fn set_page(&mut self, page: KnobPage) {
        if page != self.page {
            self.page = page;
            self.acc = [0; 8];
        }
    }

    /// The live rack's controller map changed: the Rack page's knobs do what it says. A
    /// knob whose function changed starts its stepped count again.
    pub fn set_rack(&mut self, m: &ControlMap) {
        let rack = rack_functions(m);
        if self.page == KnobPage::Rack {
            for k in 0..8 {
                if rack[k] != self.rack[k] {
                    self.acc[k] = 0;
                }
            }
        }
        self.rack = rack;
    }

    /// The function of knob `knob` (0-7) on the page.
    pub fn function(&self, knob: u8) -> KnobFn {
        let k = knob as usize;
        if k >= 8 {
            return KnobFn::None;
        }
        if self.page == KnobPage::Rack { self.rack[k] } else { self.page.functions()[k] }
    }

    /// Knob `knob` (0-7) turned `delta` steps (positive: clockwise): the command that
    /// makes the change, or None when nothing changes.
    pub fn turn_at(&mut self, knob: u8, delta: i8, now: &Now, strips: &StripNow) -> Option<AppCmd> {
        let k = knob as usize;
        let f = self.function(knob);
        let d = delta as i16;
        let level = |v: u8| (v as i16 + d * LEVEL_STEP).clamp(0, 127) as u8;
        let cmd: AppCmd = match f {
            KnobFn::None => return None,
            KnobFn::Dynamics => DynamicsCmd::SetDynamics { level: level(now.dynamics) }.into(),
            KnobFn::RetriggerRate => {
                let steps = self.stepped(k, d)?;
                StyleSettingsCmd::StepRetriggerRate { delta: steps }.into()
            }
            KnobFn::RetriggerOnOff => {
                let steps = self.stepped(k, d)?;
                if (steps > 0) == now.retrigger {
                    return None;
                }
                TransportCmd::ToggleRetrigger.into()
            }
            KnobFn::TrackMuteA | KnobFn::TrackMuteB => {
                let (i, order) = if f == KnobFn::TrackMuteA { (0, TrackMuteOrder::A) } else { (1, TrackMuteOrder::B) };
                let v = (self.mute[i] as i16 + d * MUTE_STEP).clamp(0, 127) as u8;
                if v == self.mute[i] {
                    return None;
                }
                self.mute[i] = v;
                MixerCmd::StyleTrackMute { order, value: v }.into()
            }
            KnobFn::Tempo => {
                let bpm = (now.bpm.round() as i32 + d as i32).clamp(MIN_BPM, MAX_BPM);
                TransportCmd::SetTempo { bpm: bpm as u16 }.into()
            }
            KnobFn::Swing => {
                let v = (now.swing as i16 + d * 2).clamp(0, 100) as u8;
                if v == now.swing {
                    return None;
                }
                StyleSettingsCmd::SetSwing { amount: v }.into()
            }
            KnobFn::PartVolume(p) => PartsCmd::SetPartVolume { part: p, volume: level(now.part_volume[(p & 3) as usize]) }.into(),
            KnobFn::HarmonyVolume => HarmonyArpCmd::SetHarmonyVolume { volume: level(now.harmony_volume) }.into(),
            KnobFn::MetronomeVolume => MetronomeCmd::SetMetronomeVolume { volume: level(now.metronome_volume) }.into(),
            KnobFn::PartPan(p) => PartsCmd::SetPartPan { part: p, pan: level(now.part_fx[(p & 3) as usize][0]) }.into(),
            KnobFn::PartReverb(p) => {
                PartsCmd::SetPartSend { part: p, send: PartSend::Reverb, value: level(now.part_fx[(p & 3) as usize][1]) }.into()
            }
            KnobFn::PartChorus(p) => {
                PartsCmd::SetPartSend { part: p, send: PartSend::Chorus, value: level(now.part_fx[(p & 3) as usize][2]) }.into()
            }
            KnobFn::PartDelay(p) => {
                PartsCmd::SetPartSend { part: p, send: PartSend::Variation, value: level(now.part_fx[(p & 3) as usize][3]) }.into()
            }
            KnobFn::FxReturn(b) => {
                let b = (b as usize).min(2);
                FxCmd::SetEffectReturn { block: FxBlock::ALL[b], level: level(now.fx_return[b]) }.into()
            }
            KnobFn::FxParam(p) => {
                let v = now.fx_params[p.index()];
                let to = p.clamp((v as i32 + d as i32 * p.spec().step as i32).clamp(0, u16::MAX as i32) as u16);
                if to == v {
                    return None;
                }
                fx_param(p, to)
            }
            KnobFn::DelayTime => {
                if now.fx_params[Param::DelaySync.index()] != 0 {
                    let steps = self.stepped(k, d)?;
                    let v = now.fx_params[Param::DelayNote.index()];
                    let to = Param::DelayNote.clamp((v as i16 + steps as i16).max(0) as u16);
                    if to == v {
                        return None;
                    }
                    fx_param(Param::DelayNote, to)
                } else {
                    let p = Param::DelayTime;
                    let v = now.fx_params[p.index()];
                    let to = p.clamp((v as i32 + d as i32 * p.spec().step as i32).max(0) as u16);
                    if to == v {
                        return None;
                    }
                    fx_param(p, to)
                }
            }
            KnobFn::HarmonyArp => {
                let steps = self.stepped(k, d)?;
                if (steps > 0) == now.harmony_arp {
                    return None;
                }
                HarmonyArpCmd::ToggleHarmonyArp.into()
            }
            KnobFn::SplitPoint => {
                let to = (now.split as i16 + d).clamp(SPLIT_MIN as i16, SPLIT_MAX as i16) as u8;
                if to == now.split {
                    return None;
                }
                ChordCmd::SetSplit { note: to }.into()
            }
            KnobFn::InsertOn(p, s) => {
                let steps = self.stepped(k, d)?;
                let on = steps > 0;
                if on == strips.insert(p, s).on {
                    return None;
                }
                StripCmd::SetStripInsertOn { strip: p, slot: s, on }.into()
            }
            KnobFn::InsertSetting(p, s, i) => {
                let (spec, v) = strips.setting(p, s, i)?;
                let to = spec.clamp((v as i32 + d as i32 * setting_step(spec)).clamp(0, u16::MAX as i32) as u16);
                if to == v {
                    return None;
                }
                StripCmd::SetStripInsertSetting { strip: p, slot: s, setting: i, value: to }.into()
            }
            KnobFn::PartSend(p, s) => StripCmd::SetStripSend { strip: p, send: s, level: level(strips.send(p, s)?) }.into(),
            KnobFn::RotaryFast => {
                let steps = self.stepped(k, d)?;
                if (steps > 0) == strips.rotary_fast {
                    return None;
                }
                FxCmd::SetRotaryFast { on: steps > 0 }.into()
            }
        };
        Some(cmd)
    }

    /// Knob `knob` (0-7) put back to its default (a double-click in the app): the command
    /// that makes the change, or None when it is there already. Dynamics goes to max,
    /// sends dry, pan centre, returns unity, Tempo the style's, effect parameters and
    /// insert settings the current type's own, switches off (the rotary slow).
    pub fn reset_at(&mut self, knob: u8, now: &Now, strips: &StripNow) -> Option<AppCmd> {
        let k = knob as usize;
        let f = self.function(knob);
        let d = crate::engine::StyleSettings::default();
        let cmd: AppCmd = match f {
            KnobFn::None => return None,
            KnobFn::Dynamics => (now.dynamics != 127).then(|| DynamicsCmd::SetDynamics { level: 127 }.into())?,
            KnobFn::RetriggerRate => {
                self.acc[k] = 0;
                (now.retrigger_rate != d.retrigger_rate).then(|| StyleSettingsCmd::SetRetriggerRate { rate: d.retrigger_rate }.into())?
            }
            KnobFn::RetriggerOnOff => {
                self.acc[k] = 0;
                now.retrigger.then(|| TransportCmd::ToggleRetrigger.into())?
            }
            KnobFn::TrackMuteA | KnobFn::TrackMuteB => {
                let (i, order) = if f == KnobFn::TrackMuteA { (0, TrackMuteOrder::A) } else { (1, TrackMuteOrder::B) };
                if self.mute[i] == 127 {
                    return None;
                }
                self.mute[i] = 127;
                MixerCmd::StyleTrackMute { order, value: 127 }.into()
            }
            KnobFn::Tempo => TransportCmd::ResetTempo.into(),
            KnobFn::Swing => (now.swing != d.swing).then(|| StyleSettingsCmd::SetSwing { amount: d.swing }.into())?,
            KnobFn::PartVolume(p) => PartsCmd::SetPartVolume { part: p, volume: DEFAULT_VOLUME }.into(),
            KnobFn::HarmonyVolume => HarmonyArpCmd::SetHarmonyVolume { volume: DEFAULT_VOLUME }.into(),
            KnobFn::MetronomeVolume => MetronomeCmd::SetMetronomeVolume { volume: crate::click::DEFAULT_VOLUME }.into(),
            KnobFn::PartPan(p) => PartsCmd::SetPartPan { part: p, pan: 64 }.into(),
            KnobFn::PartReverb(p) => PartsCmd::SetPartSend { part: p, send: PartSend::Reverb, value: 0 }.into(),
            KnobFn::PartChorus(p) => PartsCmd::SetPartSend { part: p, send: PartSend::Chorus, value: 0 }.into(),
            KnobFn::PartDelay(p) => PartsCmd::SetPartSend { part: p, send: PartSend::Variation, value: 0 }.into(),
            KnobFn::FxReturn(b) => FxCmd::SetEffectReturn { block: FxBlock::ALL[(b as usize).min(2)], level: crate::fx::RETURN_UNITY }.into(),
            KnobFn::FxParam(p) => fx_param(p, now.fx_defaults[p.index()]),
            KnobFn::DelayTime => {
                self.acc[k] = 0;
                let p = if now.fx_params[Param::DelaySync.index()] != 0 { Param::DelayNote } else { Param::DelayTime };
                fx_param(p, now.fx_defaults[p.index()])
            }
            KnobFn::HarmonyArp => {
                self.acc[k] = 0;
                now.harmony_arp.then(|| HarmonyArpCmd::ToggleHarmonyArp.into())?
            }
            KnobFn::SplitPoint => (now.split != DEFAULT_SPLIT).then(|| ChordCmd::SetSplit { note: DEFAULT_SPLIT }.into())?,
            KnobFn::InsertOn(p, s) => {
                self.acc[k] = 0;
                strips.insert(p, s).on.then(|| StripCmd::SetStripInsertOn { strip: p, slot: s, on: false }.into())?
            }
            KnobFn::InsertSetting(p, s, i) => {
                let (spec, v) = strips.setting(p, s, i)?;
                (v != spec.default).then(|| StripCmd::SetStripInsertSetting { strip: p, slot: s, setting: i, value: spec.default }.into())?
            }
            KnobFn::PartSend(p, s) => (strips.send(p, s)? != 0).then(|| StripCmd::SetStripSend { strip: p, send: s, level: 0 }.into())?,
            KnobFn::RotaryFast => {
                self.acc[k] = 0;
                strips.rotary_fast.then(|| FxCmd::SetRotaryFast { on: false }.into())?
            }
        };
        Some(cmd)
    }

    /// A stepped function's knob turned `d`: the whole switches (±1) it has turned
    /// through, if any. Turning back starts the count again.
    fn stepped(&mut self, k: usize, d: i16) -> Option<i8> {
        let a = &mut self.acc[k];
        if (*a > 0 && d < 0) || (*a < 0 && d > 0) {
            *a = 0;
        }
        *a += d;
        let n = *a / RTG_STEPS;
        *a -= n * RTG_STEPS;
        (n != 0).then_some(n.clamp(-6, 6) as i8)
    }

    /// The page and its knobs as the state shows them.
    pub fn state_at(&self, now: &Now, strips: &StripNow) -> KnobsState {
        let knobs = (0..8u8)
            .map(|k| {
                let (f, r) = (self.function(k), self.read_at(self.function(k), now, strips));
                KnobState { function: f.id().into(), name: f.name().into(), short: f.short().into(), value: r.value, level: r.level }
            })
            .collect();
        let page = self.page;
        KnobsState { page, page_name: page.name().into(), page_number: page.index() as u8 + 1, page_count: KnobPage::ALL.len() as u8, knobs }
    }

    /// Function `f` as it reads now (on a knob, or on a fader the controller map gives it).
    /// An insert setting reads with its name ("Drive 64"); a setting or send that isn't
    /// there reads "---".
    pub fn read_at(&self, f: KnobFn, now: &Now, strips: &StripNow) -> Reading {
        let r = |value: String, level: Option<u8>| Reading { value, level };
        let switch = |on: bool, yes: &str, no: &str| r(if on { yes } else { no }.into(), Some(if on { 127 } else { 0 }));
        match f {
            KnobFn::None => r(String::new(), None),
            KnobFn::Dynamics => r(now.dynamics.to_string(), Some(now.dynamics)),
            KnobFn::RetriggerRate => {
                let i = RETRIGGER_RATES.iter().position(|&x| x == now.retrigger_rate).unwrap_or(0);
                r(format!("1/{}", now.retrigger_rate), Some((i * 127 / (RETRIGGER_RATES.len() - 1)) as u8))
            }
            KnobFn::RetriggerOnOff => r(if now.retrigger { "On" } else { "Off" }.into(), Some(if now.retrigger { 127 } else { 0 })),
            f @ (KnobFn::TrackMuteA | KnobFn::TrackMuteB) => {
                let v = self.mute[(f == KnobFn::TrackMuteB) as usize];
                let n = TrackMuteOrder::A.mask(v).count_ones();
                r(if n == 8 { "All".into() } else { format!("{n} of 8") }, Some(v))
            }
            KnobFn::Tempo => r(format!("{} BPM", now.bpm.round() as i32), None),
            KnobFn::Swing => r(format!("{}%", now.swing), Some((now.swing.min(100) as u16 * 127 / 100) as u8)),
            KnobFn::PartVolume(p) => {
                let v = now.part_volume[(p & 3) as usize];
                r(v.to_string(), Some(v))
            }
            KnobFn::HarmonyVolume => r(now.harmony_volume.to_string(), Some(now.harmony_volume)),
            KnobFn::MetronomeVolume => r(now.metronome_volume.to_string(), Some(now.metronome_volume)),
            KnobFn::PartPan(p) => {
                let v = now.part_fx[(p & 3) as usize][0];
                r(pan_text(v), Some(v))
            }
            f @ (KnobFn::PartReverb(p) | KnobFn::PartChorus(p) | KnobFn::PartDelay(p)) => {
                let send = match f {
                    KnobFn::PartReverb(_) => crate::parts::REVERB,
                    KnobFn::PartChorus(_) => crate::parts::CHORUS,
                    _ => crate::parts::VARIATION,
                };
                let v = now.part_fx[(p & 3) as usize][send];
                r(v.to_string(), Some(v))
            }
            KnobFn::FxParam(p) => param_reading(p, now),
            KnobFn::DelayTime => param_reading(if now.fx_params[Param::DelaySync.index()] != 0 { Param::DelayNote } else { Param::DelayTime }, now),
            KnobFn::FxReturn(b) => {
                let v = now.fx_return[(b as usize).min(2)];
                r(v.to_string(), Some(v))
            }
            KnobFn::HarmonyArp => r(if now.harmony_arp { "On" } else { "Off" }.into(), Some(if now.harmony_arp { 127 } else { 0 })),
            KnobFn::SplitPoint => r(crate::api::note_name(now.split), Some(split_level(now.split))),
            KnobFn::InsertOn(p, s) => switch(strips.insert(p, s).on, "On", "Off"),
            KnobFn::InsertSetting(p, s, i) => match strips.setting(p, s, i) {
                Some((spec, v)) => r(format!("{} {}", spec.short, spec.display(v)), Some(setting_level(spec, v))),
                None => r("---".into(), None),
            },
            KnobFn::PartSend(p, s) => match strips.send(p, s) {
                Some(v) => r(v.to_string(), Some(v)),
                None => r("---".into(), None),
            },
            KnobFn::RotaryFast => switch(strips.rotary_fast, "Fast", "Slow"),
        }
    }
}

/// Where split point `n` sits in its range, 0-127.
pub fn split_level(n: u8) -> u8 {
    ((n.clamp(SPLIT_MIN, SPLIT_MAX) - SPLIT_MIN) as u16 * 127 / (SPLIT_MAX - SPLIT_MIN) as u16) as u8
}

/// The split point a fader at `v` (0-127) puts it at.
pub fn split_at(v: u8) -> u8 {
    SPLIT_MIN + (v.min(127) as u16 * (SPLIT_MAX - SPLIT_MIN) as u16 / 127) as u8
}

/// A pan as the Genos shows it: L63 … C … R63.
pub fn pan_text(v: u8) -> String {
    match v.min(127) as i16 - 64 {
        0 => "C".into(),
        d if d < 0 => format!("L{}", -d),
        d => format!("R{d}"),
    }
}

/// The command that sets effect parameter `p` to `v` (#236).
fn fx_param(p: Param, v: u16) -> AppCmd {
    FxCmd::SetEffectParam { block: FxBlock::ALL[p.spec().block], param: p, value: v }.into()
}

/// An effect parameter as a knob reads: its value as text, and where it sits in its range.
fn param_reading(p: Param, now: &Now) -> Reading {
    let s = p.spec();
    let v = p.clamp(now.fx_params[p.index()]);
    let level = ((v - s.min) as u32 * 127 / (s.max - s.min).max(1) as u32) as u8;
    Reading { value: p.display(v), level: Some(level) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> Now {
        Now {
            dynamics: 64,
            retrigger: false,
            retrigger_rate: 8,
            swing: 0,
            bpm: 120.0,
            part_volume: [100, 90, 80, 70],
            harmony_volume: 100,
            metronome_volume: 64,
            part_fx: [[64, 40, 0, 0], [30, 50, 10, 0], [100, 0, 0, 0], [64, 127, 5, 0]],
            fx_return: [64, 40, 0],
            fx_params: crate::fx::default_params(),
            fx_defaults: crate::fx::default_params(),
            harmony_arp: false,
            split: 54,
        }
    }

    // The knob calls with no strips known, for the tests that don't use them.
    fn turn(k: &mut Knobs, knob: u8, delta: i8, now: &Now) -> Option<AppCmd> {
        k.turn_at(knob, delta, now, &StripNow::default())
    }

    fn reset(k: &mut Knobs, knob: u8, now: &Now) -> Option<AppCmd> {
        k.reset_at(knob, now, &StripNow::default())
    }

    fn reading(k: &Knobs, knob: u8, now: &Now) -> Reading {
        k.read_at(k.function(knob), now, &StripNow::default())
    }

    /// A double-click puts a knob back: Dynamics to max, sends dry, pan centre, effect
    /// parameters their type's own.
    #[test]
    fn a_reset_goes_to_the_default() {
        let mut k = Knobs::default();
        assert_eq!(reset(&mut k, 0, &now()), Some(DynamicsCmd::SetDynamics { level: 127 }.into()));
        assert_eq!(reset(&mut k, 0, &Now { dynamics: 127, ..now() }), None);
        assert_eq!(reset(&mut k, 6, &now()), None);
        assert_eq!(reset(&mut k, 7, &now()), Some(TransportCmd::ResetTempo.into()));
        k.set_page(KnobPage::Pan);
        assert_eq!(reset(&mut k, 1, &now()), Some(PartsCmd::SetPartPan { part: 1, pan: 64 }.into()));
        k.set_page(KnobPage::Reverb);
        assert_eq!(reset(&mut k, 3, &now()), Some(PartsCmd::SetPartSend { part: 3, send: PartSend::Reverb, value: 0 }.into()));
        let mut p = crate::fx::default_params();
        p[Param::ReverbTime.index()] = 90;
        assert_eq!(reset(&mut k, 4, &Now { fx_params: p, ..now() }), Some(fx_param(Param::ReverbTime, crate::fx::default_params()[Param::ReverbTime.index()])));
    }

    #[test]
    fn pages_step_and_stop_at_the_ends() {
        assert_eq!(KnobPage::Style.step(-1), KnobPage::Style);
        assert_eq!(KnobPage::Style.step(1), KnobPage::Rack);
        assert_eq!(KnobPage::Pan.step(1), KnobPage::Reverb);
        assert_eq!(KnobPage::Reverb.step(1), KnobPage::Chorus);
        assert_eq!(KnobPage::Chorus.step(1), KnobPage::Delay);
        assert_eq!(KnobPage::Delay.step(1), KnobPage::Delay);
        for p in KnobPage::ALL {
            assert!(p.functions().iter().all(|f| f.short().len() <= 8));
        }
        for p in [KnobPage::Style, KnobPage::Rack, KnobPage::Pan] {
            assert_eq!(p.functions()[7], KnobFn::Tempo, "tempo is knob 8 on {p:?}");
        }
        // One page per effect: the four parts' sends to it, then its parameters and return.
        for (b, p) in [KnobPage::Reverb, KnobPage::Chorus, KnobPage::Delay].into_iter().enumerate() {
            let f = p.functions();
            assert_eq!(f[7], KnobFn::FxReturn(b as u8), "{p:?}'s return is knob 8");
            for (k, f) in f[..4].iter().enumerate() {
                let k = k as u8;
                assert_eq!(*f, [KnobFn::PartReverb(k), KnobFn::PartChorus(k), KnobFn::PartDelay(k)][b], "{p:?}");
            }
            for f in &f[4..7] {
                let param_block = match f {
                    KnobFn::FxParam(q) => Some(q.spec().block),
                    KnobFn::DelayTime => Some(crate::fx::VARIATION),
                    _ => None,
                };
                assert!(param_block.is_none_or(|x| x == b), "{p:?}: {f:?} is its own block's");
            }
        }
        // A page saved under the old names opens its successor.
        assert_eq!(serde_json::from_str::<KnobPage>("\"effects\"").unwrap(), KnobPage::Reverb);
        assert_eq!(serde_json::from_str::<KnobPage>("\"fx\"").unwrap(), KnobPage::Delay);
        assert_eq!(serde_json::from_str::<KnobPage>("\"parts\"").unwrap(), KnobPage::Rack);
    }

    /// The knobs move the value from where it is now (OM p.63).
    #[test]
    fn levels_move_from_the_value_in_effect() {
        let mut k = Knobs::default();
        let n = Now { dynamics: 100, ..now() };
        assert_eq!(turn(&mut k, 0, 3, &n), Some(DynamicsCmd::SetDynamics { level: 106 }.into()));
        assert_eq!(turn(&mut k, 0, -64, &n), Some(DynamicsCmd::SetDynamics { level: 0 }.into()));
        assert_eq!(turn(&mut k, 7, -2, &now()), Some(TransportCmd::SetTempo { bpm: 118 }.into()));
        assert_eq!(turn(&mut k, 7, 1, &Now { bpm: 500.0, ..now() }), Some(TransportCmd::SetTempo { bpm: 500 }.into()));
        assert_eq!(turn(&mut k, 5, 1, &now()), Some(StyleSettingsCmd::SetSwing { amount: 2 }.into()), "knob 6 is Swing");
        assert_eq!(turn(&mut k, 5, -1, &now()), None, "swing stops at 0");
        assert_eq!(turn(&mut k, 6, 1, &now()), None, "knob 7 is unassigned on the Style page");
        k.set_page(KnobPage::Rack);
        assert_eq!(turn(&mut k, 1, -1, &now()), Some(PartsCmd::SetPartVolume { part: 1, volume: 88 }.into()));
        assert_eq!(turn(&mut k, 4, 1, &now()), Some(HarmonyArpCmd::SetHarmonyVolume { volume: 102 }.into()));
        assert_eq!(turn(&mut k, 5, 1, &now()), Some(MetronomeCmd::SetMetronomeVolume { volume: 66 }.into()));
    }

    /// The Rack page with the default controller map is the Parts page it replaced: part
    /// volumes, Harmony volume, the metronome, nothing, the tempo.
    #[test]
    fn the_default_rack_page_is_the_parts_page() {
        use KnobFn::{HarmonyVolume, MetronomeVolume, PartVolume, Tempo};
        let parts = [PartVolume(0), PartVolume(1), PartVolume(2), PartVolume(3), HarmonyVolume, MetronomeVolume, KnobFn::None, Tempo];
        let mut k = Knobs::default();
        k.set_page(KnobPage::Rack);
        assert_eq!(std::array::from_fn::<_, 8, _>(|i| k.function(i as u8)), parts);
        let shorts: Vec<_> = (0..8).map(|i| k.function(i).short()).collect();
        assert_eq!(shorts, ["Right1", "Right2", "Right3", "Left", "HarmVol", "MetroVol", "---", "Tempo"]);
        assert_eq!(turn(&mut k, 7, 1, &now()), Some(TransportCmd::SetTempo { bpm: 121 }.into()));
        assert_eq!(turn(&mut k, 6, 1, &now()), None);
    }

    /// A knob on the Rack page does what the controller map says: its command, its name,
    /// its reading.
    #[test]
    fn the_rack_page_follows_the_controller_map() {
        let mut m = ControlMap::default();
        m.knobs[0] = ControlTarget::PartPan { part: 2 };
        m.knobs[1] = ControlTarget::HarmonyArp;
        m.knobs[2] = ControlTarget::SplitPoint;
        m.knobs[3] = ControlTarget::Unknown(serde_json::json!({ "kind": "pluginMacro" }));
        let mut k = Knobs::default();
        k.set_page(KnobPage::Rack);
        k.set_rack(&m);
        assert_eq!(turn(&mut k, 0, 1, &now()), Some(PartsCmd::SetPartPan { part: 2, pan: 102 }.into()));
        assert_eq!((k.function(0).short(), k.function(1).short(), k.function(2).short(), k.function(3).short()), ("PanR3", "HarmArp", "Split", "---"));
        assert_eq!(turn(&mut k, 1, 2, &now()), None, "stepped");
        assert_eq!(turn(&mut k, 1, 1, &now()), Some(HarmonyArpCmd::ToggleHarmonyArp.into()), "right: on");
        assert_eq!(turn(&mut k, 1, 3, &Now { harmony_arp: true, ..now() }), None, "already on");
        assert_eq!(reading(&k, 1, &Now { harmony_arp: true, ..now() }).value, "On");
        assert_eq!(turn(&mut k, 2, -2, &now()), Some(ChordCmd::SetSplit { note: 52 }.into()));
        assert_eq!(turn(&mut k, 2, -1, &Now { split: SPLIT_MIN, ..now() }), None);
        assert_eq!(reading(&k, 2, &now()), Reading { value: "F#2".into(), level: Some(52) });
        assert_eq!(reset(&mut k, 2, &Now { split: 60, ..now() }), Some(ChordCmd::SetSplit { note: DEFAULT_SPLIT }.into()));
        assert_eq!(turn(&mut k, 3, 5, &now()), None, "a target this build doesn't know does nothing");
        // Other pages don't change.
        k.set_page(KnobPage::Pan);
        assert_eq!(k.function(0), KnobFn::PartPan(0));
        assert_eq!((split_at(0), split_at(127)), (SPLIT_MIN, SPLIT_MAX));
    }

    /// The strips (the mixer rework): Right 2's insert 2 is a distortion, on; Left's insert 1
    /// empty; one added send (send 4) at 30 for Right 1; the rotary slow.
    fn strips() -> StripNow {
        use crate::fx::InsertType;
        let mut s = StripNow { send_count: 4, ..StripNow::default() };
        let kind = InsertType::Distortion;
        let mut values = kind.defaults();
        values[1] = 40;
        s.inserts[1][1] = InsertNow { on: true, values, specs: kind.settings() };
        s.sends[0][3] = 30;
        s
    }

    /// A knob on an insert setting sends `setStripInsertSetting` for its strip, slot and
    /// setting, from the value in effect and across the kind's range; on an insert slot,
    /// a send 4-6 or the rotary speed, their own commands.
    #[test]
    fn strip_targets_on_the_rack_page() {
        use crate::fx::InsertType;
        let mut m = ControlMap::default();
        m.knobs[0] = ControlTarget::PartInsertSetting { part: 1, slot: 1, setting: 1 };
        m.knobs[1] = ControlTarget::PartInsertOn { part: 1, slot: 1 };
        m.knobs[2] = ControlTarget::PartSend { part: 0, send: 3 };
        m.knobs[3] = ControlTarget::RotaryFast;
        m.knobs[4] = ControlTarget::PartInsertSetting { part: 3, slot: 0, setting: 0 };
        m.knobs[5] = ControlTarget::PartSend { part: 0, send: 5 };
        m.knobs[6] = ControlTarget::PartDelay { part: 2 };
        let mut k = Knobs::default();
        k.set_page(KnobPage::Rack);
        k.set_rack(&m);
        let st = strips();
        let spec = InsertType::Distortion.settings()[1];
        let step = setting_step(&spec) as u16;
        assert_eq!(
            k.turn_at(0, 2, &now(), &st),
            Some(StripCmd::SetStripInsertSetting { strip: 1, slot: 1, setting: 1, value: spec.clamp(40 + 2 * step) }.into())
        );
        assert_eq!(k.turn_at(0, -127, &now(), &st), Some(StripCmd::SetStripInsertSetting { strip: 1, slot: 1, setting: 1, value: spec.min }.into()));
        assert_eq!(k.read_at(k.function(0), &now(), &st).value, format!("{} {}", spec.short, spec.display(40)));
        // The strip's Tone is at 40, not its default (64): a reset sends the default.
        assert_ne!(spec.default, 40, "the test needs a setting away from its default");
        assert_eq!(k.reset_at(0, &now(), &st), Some(StripCmd::SetStripInsertSetting { strip: 1, slot: 1, setting: 1, value: spec.default }.into()));
        // At its default already: nothing to send.
        let mut at_default = st;
        at_default.inserts[1][1].values[1] = spec.default;
        assert_eq!(k.reset_at(0, &now(), &at_default), None);
        assert_eq!((k.function(0).id(), k.function(0).short(), k.function(0).name()), ("insertSetting", "R2 I2.2", "Right 2 Insert 2 Setting 2"));
        // An empty slot has no settings: nothing to turn.
        assert_eq!(k.turn_at(4, 3, &now(), &st), None);
        assert_eq!(k.read_at(k.function(4), &now(), &st), Reading { value: "---".into(), level: None });
        // The slot's switch: stepped, left off.
        assert_eq!(k.turn_at(1, 3, &now(), &st), None, "already on");
        assert_eq!(k.turn_at(1, -3, &now(), &st), Some(StripCmd::SetStripInsertOn { strip: 1, slot: 1, on: false }.into()));
        assert_eq!(k.read_at(k.function(1), &now(), &st).value, "On");
        // Send 4, there; send 6, not.
        assert_eq!(k.turn_at(2, 1, &now(), &st), Some(StripCmd::SetStripSend { strip: 0, send: 3, level: 32 }.into()));
        assert_eq!(k.reset_at(2, &now(), &st), Some(StripCmd::SetStripSend { strip: 0, send: 3, level: 0 }.into()));
        assert_eq!(k.turn_at(5, 1, &now(), &st), None);
        assert_eq!((k.function(2).short(), k.function(5).name()), ("R1 Snd4", "Right 1 Send 6"));
        // The rotary: right fast, left slow.
        assert_eq!(k.turn_at(3, 3, &now(), &st), Some(FxCmd::SetRotaryFast { on: true }.into()));
        let fast = StripNow { rotary_fast: true, ..st };
        assert_eq!(k.turn_at(3, 3, &now(), &fast), None);
        assert_eq!(k.read_at(KnobFn::RotaryFast, &now(), &fast), Reading { value: "Fast".into(), level: Some(127) });
        assert_eq!(k.reset_at(3, &now(), &fast), Some(FxCmd::SetRotaryFast { on: false }.into()));
        // The delay send is the Variation send.
        assert_eq!(k.turn_at(6, 1, &now(), &st), Some(PartsCmd::SetPartSend { part: 2, send: PartSend::Variation, value: 2 }.into()));
    }

    /// A fader on a strip target: an insert setting across its range, a switch on from
    /// 64 (nothing when it is that already), a send 4-6 level while that send is there.
    #[test]
    fn strip_targets_on_a_fader() {
        use crate::fx::InsertType;
        let st = strips();
        let spec = InsertType::Distortion.settings()[1];
        let f = |t, v, s: &StripNow| fader_command_at(&t, v, false, s);
        assert_eq!(f(ControlTarget::PartInsertSetting { part: 1, slot: 1, setting: 1 }, 127, &st), Some(StripCmd::SetStripInsertSetting { strip: 1, slot: 1, setting: 1, value: spec.max }.into()));
        assert_eq!(f(ControlTarget::PartInsertSetting { part: 1, slot: 1, setting: 1 }, 0, &st), Some(StripCmd::SetStripInsertSetting { strip: 1, slot: 1, setting: 1, value: spec.min }.into()));
        assert_eq!(f(ControlTarget::PartInsertSetting { part: 3, slot: 0, setting: 0 }, 90, &st), None, "empty slot");
        assert_eq!(f(ControlTarget::PartInsertOn { part: 1, slot: 1 }, 100, &st), None, "already on");
        assert_eq!(f(ControlTarget::PartInsertOn { part: 1, slot: 1 }, 10, &st), Some(StripCmd::SetStripInsertOn { strip: 1, slot: 1, on: false }.into()));
        assert_eq!(f(ControlTarget::PartSend { part: 0, send: 3 }, 77, &st), Some(StripCmd::SetStripSend { strip: 0, send: 3, level: 77 }.into()));
        assert_eq!(f(ControlTarget::PartSend { part: 0, send: 4 }, 77, &st), None, "send 5 isn't there");
        assert_eq!(f(ControlTarget::RotaryFast, 127, &st), Some(FxCmd::SetRotaryFast { on: true }.into()));
        assert_eq!(f(ControlTarget::RotaryFast, 0, &st), None, "already slow");
        assert_eq!(f(ControlTarget::PartDelay { part: 1 }, 50, &st), Some(PartsCmd::SetPartSend { part: 1, send: PartSend::Variation, value: 50 }.into()));
        let m = ControlMap { faders: [ControlTarget::RotaryFast, ControlTarget::PartInsertOn { part: 0, slot: 0 }, ControlTarget::PartSend { part: 2, send: 3 }, ControlTarget::PartLevel { part: 3 }], ..ControlMap::default() };
        assert_eq!(fader_routes(&m), [FaderRoute::Control, FaderRoute::Control, FaderRoute::Control, FaderRoute::Own]);
    }

    /// Retrigger steps once per few knob steps; turning back starts the count again.
    #[test]
    fn retrigger_knobs_step() {
        let mut k = Knobs::default();
        assert_eq!(turn(&mut k, 1, 1, &now()), None);
        assert_eq!(turn(&mut k, 1, 1, &now()), None);
        assert_eq!(turn(&mut k, 1, 1, &now()), Some(StyleSettingsCmd::StepRetriggerRate { delta: 1 }.into()), "right: shorter");
        assert_eq!(turn(&mut k, 1, 2, &now()), None);
        assert_eq!(turn(&mut k, 1, -1, &now()), None, "turning back starts again");
        assert_eq!(turn(&mut k, 1, -2, &now()), Some(StyleSettingsCmd::StepRetriggerRate { delta: -1 }.into()));
        assert_eq!(turn(&mut k, 1, 7, &now()), Some(StyleSettingsCmd::StepRetriggerRate { delta: 2 }.into()));
        // On/Off: right turns it on, and further right leaves it on.
        assert_eq!(turn(&mut k, 2, 3, &now()), Some(TransportCmd::ToggleRetrigger.into()));
        assert_eq!(turn(&mut k, 2, 3, &Now { retrigger: true, ..now() }), None);
        assert_eq!(turn(&mut k, 2, -3, &Now { retrigger: true, ..now() }), Some(TransportCmd::ToggleRetrigger.into()));
    }

    /// Track Mute A/B start fully right (every part on) and turn parts off going left.
    #[test]
    fn track_mute_knobs() {
        let mut k = Knobs::default();
        assert_eq!(reading(&k, 3, &now()).value, "All");
        assert_eq!(turn(&mut k, 3, 1, &now()), None, "already fully right");
        assert_eq!(turn(&mut k, 3, -4, &now()), Some(MixerCmd::StyleTrackMute { order: TrackMuteOrder::A, value: 111 }.into()));
        assert_eq!(turn(&mut k, 4, -40, &now()), Some(MixerCmd::StyleTrackMute { order: TrackMuteOrder::B, value: 0 }.into()));
        assert_eq!(reading(&k, 4, &now()), Reading { value: "1 of 8".into(), level: Some(0) });
        assert_eq!(reading(&k, 3, &now()).level, Some(111));
    }

    /// Pan and the effect sends, on the Pan and Effects pages (#198's per-part controls).
    #[test]
    fn pan_and_effect_knobs() {
        let mut k = Knobs::default();
        k.set_page(KnobPage::Pan);
        assert_eq!(turn(&mut k, 1, -3, &now()), Some(PartsCmd::SetPartPan { part: 1, pan: 24 }.into()));
        assert_eq!(reading(&k, 0, &now()), Reading { value: "C".into(), level: Some(64) });
        assert_eq!(reading(&k, 1, &now()).value, "L34");
        assert_eq!(reading(&k, 2, &now()).value, "R36");
        assert_eq!(k.function(7), KnobFn::Tempo);
        // Knobs 5-7: the effect bus's return levels (#204).
        assert_eq!(turn(&mut k, 4, -2, &now()), Some(FxCmd::SetEffectReturn { block: FxBlock::Reverb, level: 60 }.into()));
        assert_eq!(turn(&mut k, 6, 1, &now()), Some(FxCmd::SetEffectReturn { block: FxBlock::Variation, level: 2 }.into()));
        assert_eq!(reading(&k, 5, &now()), Reading { value: "40".into(), level: Some(40) });
        assert_eq!((k.function(4).short(), k.function(6).name()), ("RevRtn", "Delay Return"));
        k.set_page(KnobPage::Reverb);
        assert_eq!(turn(&mut k, 3, 1, &now()), Some(PartsCmd::SetPartSend { part: 3, send: PartSend::Reverb, value: 127 }.into()));
        assert_eq!(reading(&k, 0, &now()), Reading { value: "40".into(), level: Some(40) });
        assert_eq!(turn(&mut k, 7, -2, &now()), Some(FxCmd::SetEffectReturn { block: FxBlock::Reverb, level: 60 }.into()));
        k.set_page(KnobPage::Chorus);
        assert_eq!(turn(&mut k, 1, 2, &now()), Some(PartsCmd::SetPartSend { part: 1, send: PartSend::Chorus, value: 14 }.into()));
        assert_eq!(reading(&k, 3, &now()), Reading { value: "5".into(), level: Some(5) });
        assert_eq!(reading(&k, 7, &now()), Reading { value: "40".into(), level: Some(40) });
        k.set_page(KnobPage::Delay);
        assert_eq!(turn(&mut k, 0, 1, &now()), Some(PartsCmd::SetPartSend { part: 0, send: PartSend::Variation, value: 2 }.into()));
        assert_eq!(reading(&k, 0, &now()), Reading { value: "0".into(), level: Some(0) });
        assert_eq!((k.function(2).short(), k.function(3).name()), ("DlyR3", "Left Delay"));
        assert_eq!(turn(&mut k, 7, 1, &now()), Some(FxCmd::SetEffectReturn { block: FxBlock::Variation, level: 2 }.into()));
        assert_eq!(pan_text(0), "L64");
        assert_eq!(pan_text(127), "R63");
    }

    #[test]
    fn readings() {
        let k = Knobs::default();
        assert_eq!(reading(&k, 0, &now()), Reading { value: "64".into(), level: Some(64) });
        assert_eq!(reading(&k, 1, &now()), Reading { value: "1/8".into(), level: Some(76) });
        assert_eq!(reading(&k, 2, &now()).value, "Off");
        assert_eq!(reading(&k, 5, &Now { swing: 50, ..now() }), Reading { value: "50%".into(), level: Some(63) });
        assert_eq!(reading(&k, 6, &now()), Reading { value: String::new(), level: None });
        assert_eq!(reading(&k, 7, &Now { bpm: 97.6, ..now() }).value, "98 BPM");
    }

    /// #236: the effect pages turn the effect parameters in their own steps; the delay
    /// time knob steps the note value with tempo sync on and the ms with it off.
    #[test]
    fn effect_parameter_knobs() {
        use crate::api::FxParam;
        let mut k = Knobs::default();
        let set = |block, param, value| Some(AppCmd::from(FxCmd::SetEffectParam { block, param, value }));
        k.set_page(KnobPage::Reverb);
        assert_eq!(turn(&mut k, 4, 1, &now()), set(FxBlock::Reverb, FxParam::ReverbTime, 25));
        assert_eq!(turn(&mut k, 5, -20, &now()), set(FxBlock::Reverb, FxParam::PreDelay, 0));
        assert_eq!(reading(&k, 4, &now()), Reading { value: "2.4 s".into(), level: Some(27) });
        assert_eq!((k.function(4).short(), k.function(4).name(), k.function(6).short()), ("RevTime", "Reverb Time", "RevTone"));
        // At an end nothing changes.
        let mut p = crate::fx::default_params();
        p[FxParam::ReverbTime.index()] = 100;
        assert_eq!(turn(&mut k, 4, 1, &Now { fx_params: p, ..now() }), None);
        k.set_page(KnobPage::Chorus);
        assert_eq!(turn(&mut k, 4, -1, &now()), set(FxBlock::Chorus, FxParam::ChorusRate, 53));
        assert_eq!(turn(&mut k, 5, 1, &now()), set(FxBlock::Chorus, FxParam::ChorusDepth, 23));
        assert_eq!(turn(&mut k, 6, 1, &now()), None, "no chorus feedback: unassigned");
        k.set_page(KnobPage::Delay);
        assert_eq!(turn(&mut k, 5, 3, &now()), set(FxBlock::Variation, FxParam::DelayFeedback, 44));
        assert_eq!(reading(&k, 5, &now()).value, "38%");
        assert_eq!(k.function(6).short(), "DlyTone");
        // Delay time: the note value, a step every 3 knob steps (1/8. -> 1/4).
        assert_eq!(k.function(4).short(), "DlyTime");
        assert_eq!(reading(&k, 4, &now()).value, "1/8.");
        assert_eq!(turn(&mut k, 4, 2, &now()), None);
        assert_eq!(turn(&mut k, 4, 1, &now()), set(FxBlock::Variation, FxParam::DelayNote, 5));
        // With tempo sync off: ms, 10 a step.
        let mut p = crate::fx::default_params();
        p[FxParam::DelaySync.index()] = 0;
        let free = Now { fx_params: p, ..now() };
        assert_eq!(reading(&k, 4, &free).value, "375 ms");
        assert_eq!(turn(&mut k, 4, -2, &free), set(FxBlock::Variation, FxParam::DelayTime, 355));
    }
}

//! Keyboard Harmony / Arpeggio: the HARMONY/ARPEGGIO switch, the type and the detail
//! settings (spec §6; docs/harmony.md, docs/arpeggio.md).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum HarmonyArpCmd {
    /// The HARMONY/ARPEGGIO button: the selected type on or off.
    ToggleHarmonyArp,
    SetHarmonyArpOn { on: bool },
    /// Select Keyboard Harmony type `index` (`LibraryList.harmonyTypes`, Data List order).
    SetHarmonyType { index: u8 },
    /// Select arpeggio pattern `index` (`LibraryList.arpPatterns`).
    SetArpPattern { index: u8 },
    /// Step through the types and then the patterns, as one list (wrapping).
    StepHarmonyArpType { delta: i8 },
    /// Volume (HrmArpVol), 0-127: the level of the added notes and of the arpeggio.
    SetHarmonyVolume { volume: u8 },
    /// Echo, Tremolo and Trill speed.
    SetHarmonySpeed { speed: HarmonySpeed },
    /// Which Right parts sound the effect (and the arpeggio).
    SetHarmonyAssign { assign: HarmonyAssign },
    /// Harmony category: harmonise only melody notes of the current chord.
    SetChordNoteOnly { on: bool },
    /// Touch Limit (Minimum Velocity), 1-127: the effect sounds only for keys played at
    /// least this hard.
    SetTouchLimit { velocity: u8 },
    /// Arpeggio Quantize.
    SetArpQuantize { quantize: ArpQuantize },
    /// The Arpeggio Hold setting (RM p.41: the pattern plays on after the keys go up until
    /// the switch goes off).
    SetArpHold { on: bool },
    ToggleArpHold,
    /// The Arpeggio Hold pedal function (RM p.141), apart from the setting: the pattern
    /// plays on while it is on and stops when it goes off. A Hold A / Hold B pedal sets it,
    /// a Toggle pedal (and the function's Try) switches it.
    SetArpPedalHold { on: bool },
    ToggleArpPedalHold,
    /// Where the arpeggio's velocities come from; `velocity` (1-127) is used by `fixed`.
    SetArpVelocity { mode: ArpVelocityMode, velocity: u8 },
    /// Keep Key On: the pattern clock runs on through a full release.
    SetArpKeepKeyOn { on: bool },
}

/// The HARMONY/ARPEGGIO switch and its settings.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HarmonyArpState {
    /// The HARMONY/ARPEGGIO switch.
    pub on: bool,
    /// Which list the selected type is in.
    pub mode: HarmonyArpMode,
    /// The selected Harmony type (index into `LibraryList::harmony_types`), kept while an
    /// arpeggio is selected.
    pub harmony_type: u8,
    /// The selected arpeggio pattern (index into `LibraryList::arp_patterns`).
    pub arp_pattern: u8,
    /// The selected type's name, e.g. "Standard Duet 1" or "Climb 16".
    pub type_name: String,
    /// Its category: "Harmony", "Echo", or the arpeggio pattern's ("Up & Down", ...).
    pub category: String,
    pub volume: u8,
    pub speed: HarmonySpeed,
    pub assign: HarmonyAssign,
    pub chord_note_only: bool,
    pub touch_limit: u8,
    pub arp: ArpSettings,
}

/// A type or pattern in the lists (`LibraryList::harmony_types`, `arp_patterns`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HarmonyTypeInfo {
    pub name: String,
    pub category: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArpSettings {
    pub quantize: ArpQuantize,
    /// The Hold setting.
    pub hold: bool,
    /// The Arpeggio Hold pedal function is on (a pedal holding it, or switched on).
    #[serde(default)]
    pub pedal_hold: bool,
    pub velocity: ArpVelocityMode,
    /// The velocity `fixed` plays at.
    pub fixed_velocity: u8,
    pub keep_key_on: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HarmonyArpMode {
    #[default]
    Harmony,
    Arpeggio,
}

/// Echo-category speed as a note value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum HarmonySpeed {
    #[serde(rename = "1/4")]
    Quarter,
    #[serde(rename = "1/6")]
    QuarterTriplet,
    #[default]
    #[serde(rename = "1/8")]
    Eighth,
    #[serde(rename = "1/12")]
    EighthTriplet,
    #[serde(rename = "1/16")]
    Sixteenth,
    #[serde(rename = "1/32")]
    ThirtySecond,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HarmonyAssign {
    #[default]
    Auto,
    Multi,
    Right1,
    Right2,
    Right3,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArpQuantize {
    #[default]
    Off,
    Eighth,
    Sixteenth,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArpVelocityMode {
    /// The pattern's own accents.
    #[default]
    Original,
    /// The velocity each key was played with.
    Thru,
    /// Every note at `fixedVelocity`.
    Fixed,
}

/// Every Keyboard Harmony type, in Data List order, with its category ("Harmony", "Echo").
pub fn harmony_type_options() -> Vec<HarmonyTypeInfo> {
    crate::harmony::ALL_TYPES
        .iter()
        .map(|&t| HarmonyTypeInfo { name: t.name().to_string(), category: harmony_category_name(t).to_string() })
        .collect()
}

/// Every arpeggio pattern, with its category ("Up & Down", ...).
pub fn arp_pattern_options() -> Vec<HarmonyTypeInfo> {
    crate::arp::library::PATTERNS
        .iter()
        .map(|p| HarmonyTypeInfo { name: p.name.to_string(), category: arp_category_name(p.category).to_string() })
        .collect()
}

pub fn harmony_category_name(t: crate::harmony::HarmonyType) -> &'static str {
    match t.category() {
        crate::harmony::Category::Harmony => "Harmony",
        crate::harmony::Category::Echo => "Echo",
    }
}

pub fn arp_category_name(c: crate::arp::Category) -> &'static str {
    use crate::arp::Category as C;
    match c {
        C::UpDown => "Up & Down",
        C::Random => "Random",
        C::AsPlayed => "As Played",
        C::ChordStab => "Chord Stab",
        C::BrokenChord => "Broken Chord",
        C::Guitar => "Guitar",
        C::Sequence => "Sequence",
    }
}

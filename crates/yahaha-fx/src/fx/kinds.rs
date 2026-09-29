//! The channel strip's effect lists and control-side settings (the mixer rework): every
//! strip (4 keyboard parts, 8 Style parts) runs EQ, compressor, insert 1, insert 2, then
//! its sends to up to [`SENDS`] send effects, then pan and level.
//!
//! - [`InsertType`]: what an insert slot plays, by a stable name ("distortion",
//!   "autoWah", ...), [`InsertType::None`] when empty. Each kind has 2-4 named settings
//!   ([`InsertType::settings`], [`KnobSpec`]); the first means what the old single
//!   `amount` meant, so an old slot keeps sounding the same.
//! - [`SendKind`]: what a send effect plays: the reverb, chorus and delay types the three
//!   buses play today, and a phaser.
//! - [`InsertSlot`], [`SendSlot`]: one slot's control settings, as the app API, a rack
//!   file and the session hold them.
//!
//! Both kinds read an unknown name (a newer build's) as `Unknown(name)` and write it back
//! unchanged; an unknown insert plays dry and an unknown send is silent. Nothing here runs
//! on the audio thread.

use serde::{Deserialize, Serialize};

/// Insert slots per strip.
pub const INSERT_SLOTS: usize = 2;
/// Settings per insert slot (a kind uses the first 2-4).
pub const INSERT_VALUES: usize = 4;
/// Send effects: 1-3 always there, fed by the style's reverb, chorus and delay sends
/// (the buses, `super::BUSES`); 4-6 added by the player, the rack's.
pub const SENDS: usize = 6;
/// Parameters per send effect (a kind uses the first few; the delay uses all six).
pub const SEND_PARAMS: usize = 6;

/// How a [`KnobSpec`]'s value reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    /// A plain number: "64".
    Plain,
    /// Milliseconds: "12 ms".
    Ms,
    /// Hundredths of a hertz: "0.50 Hz".
    CentiHz,
    /// Percent: "40%".
    Percent,
    /// A note value, an index into `super::NOTES`: "1/8".
    Note,
    /// 0 = Off, 1 = On.
    Switch,
    /// A bus parameter, read as it reads there (`super::Param::display`).
    Bus(super::Param),
}

/// A setting's range, default and names (like `super::Spec` for the bus parameters).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KnobSpec {
    /// "Drive".
    pub name: &'static str,
    /// Up to 8 characters, for a knob: "Drive".
    pub short: &'static str,
    pub min: u16,
    pub max: u16,
    pub default: u16,
    pub unit: Unit,
}

impl KnobSpec {
    const fn new(name: &'static str, short: &'static str, min: u16, max: u16, default: u16, unit: Unit) -> KnobSpec {
        KnobSpec { name, short, min, max, default, unit }
    }

    /// `v` in range.
    pub fn clamp(&self, v: u16) -> u16 {
        v.clamp(self.min, self.max)
    }

    /// `v` as it reads: "64", "12 ms", "0.50 Hz", "40%", "1/8", "On".
    pub fn display(&self, v: u16) -> String {
        let v = self.clamp(v);
        match self.unit {
            Unit::Plain => v.to_string(),
            Unit::Ms => format!("{v} ms"),
            Unit::CentiHz => format!("{:.2} Hz", v as f32 / 100.0),
            Unit::Percent => format!("{v}%"),
            Unit::Note => super::NOTES.get(v as usize).map_or_else(|| v.to_string(), |n| n.1.to_string()),
            Unit::Switch => if v != 0 { "On" } else { "Off" }.into(),
            Unit::Bus(p) => p.display(v),
        }
    }
}

/// The eighth note's index in `super::NOTES`.
const EIGHTH: u16 = 2;

/// The distortion's settings:
/// - Drive (0-127): the soft clipper's pre gain, 1 (clean) .. 40 (lead), with the output
///   kept near the input's loudness; more drive also lowers the cabinet-like high cut
///   (5.5 kHz .. 3 kHz).
/// - Tone (0-127): moves that high cut, 32 steps an octave; 64 leaves it where Drive puts
///   it (0: two octaves down, 127: about two up).
/// - Output (0-127): the effect's output gain, linear; 100 = unity.
const DISTORTION: [KnobSpec; 3] =
    [KnobSpec::new("Drive", "Drive", 0, 127, 64, Unit::Plain), KnobSpec::new("Tone", "Tone", 0, 127, 64, Unit::Plain), KnobSpec::new("Output", "Output", 0, 127, 100, Unit::Plain)];
/// The compressor's settings:
/// - Squeeze (0-127): the threshold, -12 dB .. -30 dB (ratio 4), with makeup gain to
///   match.
/// - Attack (1-80 ms) and Release (10-1000 ms): the level follower's times (defaults 3 ms
///   and 150 ms).
/// - Output (0-127): the output gain after makeup, linear; 100 = unity.
const COMPRESSOR: [KnobSpec; 4] = [
    KnobSpec::new("Squeeze", "Squeeze", 0, 127, 64, Unit::Plain),
    KnobSpec::new("Attack", "Attack", 1, 80, 3, Unit::Ms),
    KnobSpec::new("Release", "Release", 10, 1000, 150, Unit::Ms),
    KnobSpec::new("Output", "Output", 0, 127, 100, Unit::Plain),
];
/// The auto wah's settings:
/// - Sensitivity (0-127): how far the part's envelope opens the filter.
/// - Resonance (0-127): the filter's peak, 32 steps an octave of its Q; 64 is the
///   default peak, 0 flat, 127 sharp.
/// - Frequency (0-127): where the sweep starts (the filter closed), 32 steps an octave:
///   0 = 175 Hz, 32 = 350 Hz, 127 = about 2.7 kHz.
const AUTO_WAH: [KnobSpec; 3] = [
    KnobSpec::new("Sensitivity", "Sens", 0, 127, 64, Unit::Plain),
    KnobSpec::new("Resonance", "Reso", 0, 127, 64, Unit::Plain),
    KnobSpec::new("Frequency", "Freq", 0, 127, 32, Unit::Plain),
];
/// The tremolo's settings:
/// - Depth (0-127): how far the level dips, 20% .. 80%.
/// - Note (a note value, `super::NOTES`, 1/16 .. 1/2): one LFO cycle per note at the
///   style tempo (at most 12 Hz); 1/8 by default.
/// - Shape (0-127): 0 = a smooth sine, up to a nearly square on/off.
const TREMOLO: [KnobSpec; 3] =
    [KnobSpec::new("Depth", "Depth", 0, 127, 64, Unit::Plain), KnobSpec::new("Note", "Note", 0, 7, EIGHTH, Unit::Note), KnobSpec::new("Shape", "Shape", 0, 127, 0, Unit::Plain)];
/// The rotary speaker's settings:
/// - Depth (0-127): how far the horn and drum move the level (and the sides apart).
/// - Drive (0-127): the amp in front of the speaker; 0 = clean, up to a soft overdrive.
/// - Balance (0-127): horn against drum; 0 = drum only, 64 = horn 60% and drum 40%,
///   127 = horn only.
const ROTARY: [KnobSpec; 3] =
    [KnobSpec::new("Depth", "Depth", 0, 127, 64, Unit::Plain), KnobSpec::new("Drive", "Drive", 0, 127, 0, Unit::Plain), KnobSpec::new("Balance", "Balance", 0, 127, 64, Unit::Plain)];
/// The phaser's settings (the insert's and the send's, `super::Phaser`):
/// - Depth (0-127): how deep the notches are; 0 = dry, 127 = the dry and the allpass
///   chain in equal parts.
/// - Rate (0.05-5.00 Hz, in hundredths): the sweep's speed (200 Hz .. 3.2 kHz and back;
///   the right side a quarter cycle ahead of the left).
/// - Feedback (0-90%): the chain's output fed back into it, sharpening the peaks.
const PHASER: [KnobSpec; 3] = [
    KnobSpec::new("Depth", "Depth", 0, 127, 64, Unit::Plain),
    KnobSpec::new("Rate", "Rate", 5, 500, 50, Unit::CentiHz),
    KnobSpec::new("Feedback", "Feedback", 0, 90, 40, Unit::Percent),
];

/// What an insert slot plays. On the wire and in a rack file, a stable name.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum InsertType {
    /// An empty slot.
    #[default]
    None,
    Distortion,
    Compressor,
    AutoWah,
    Tremolo,
    Rotary,
    Phaser,
    /// A kind this build doesn't know (a newer build's), kept by name and written back
    /// unchanged. It plays dry.
    Unknown(String),
}

impl InsertType {
    /// Every kind this build plays, None first.
    pub const ALL: [InsertType; 7] =
        [InsertType::None, InsertType::Distortion, InsertType::Compressor, InsertType::AutoWah, InsertType::Tremolo, InsertType::Rotary, InsertType::Phaser];

    /// The stable name: "none", "distortion", "compressor", "autoWah", "tremolo",
    /// "rotary", "phaser", or an unknown kind's own.
    pub fn id(&self) -> &str {
        match self {
            InsertType::None => "none",
            InsertType::Distortion => "distortion",
            InsertType::Compressor => "compressor",
            InsertType::AutoWah => "autoWah",
            InsertType::Tremolo => "tremolo",
            InsertType::Rotary => "rotary",
            InsertType::Phaser => "phaser",
            InsertType::Unknown(s) => s,
        }
    }

    /// As the app shows it: "Auto Wah" (an unknown kind by its name).
    pub fn name(&self) -> &str {
        match self {
            InsertType::Unknown(s) => s,
            k => k.kind().name(),
        }
    }

    /// What the audio thread plays for it (an unknown kind: nothing).
    pub fn kind(&self) -> super::InsertKind {
        use super::InsertKind as K;
        match self {
            InsertType::None | InsertType::Unknown(_) => K::None,
            InsertType::Distortion => K::Distortion,
            InsertType::Compressor => K::Compressor,
            InsertType::AutoWah => K::AutoWah,
            InsertType::Tremolo => K::Tremolo,
            InsertType::Rotary => K::Rotary,
            InsertType::Phaser => K::Phaser,
        }
    }

    /// Its settings, in order (none for None or an unknown kind). The first is what the
    /// old single `amount` was.
    pub fn settings(&self) -> &'static [KnobSpec] {
        match self {
            InsertType::None | InsertType::Unknown(_) => &[],
            InsertType::Distortion => &DISTORTION,
            InsertType::Compressor => &COMPRESSOR,
            InsertType::AutoWah => &AUTO_WAH,
            InsertType::Tremolo => &TREMOLO,
            InsertType::Rotary => &ROTARY,
            InsertType::Phaser => &PHASER,
        }
    }

    /// Its settings at their defaults (unused values 0).
    pub fn defaults(&self) -> [u16; INSERT_VALUES] {
        let mut v = [0; INSERT_VALUES];
        for (o, s) in v.iter_mut().zip(self.settings()) {
            *o = s.default;
        }
        v
    }
}

impl From<super::InsertKind> for InsertType {
    fn from(k: super::InsertKind) -> InsertType {
        use super::InsertKind as K;
        match k {
            K::None => InsertType::None,
            K::Distortion => InsertType::Distortion,
            K::Compressor => InsertType::Compressor,
            K::AutoWah => InsertType::AutoWah,
            K::Tremolo => InsertType::Tremolo,
            K::Rotary => InsertType::Rotary,
            K::Phaser => InsertType::Phaser,
        }
    }
}

impl From<String> for InsertType {
    fn from(s: String) -> InsertType {
        InsertType::ALL.into_iter().find(|k| k.id() == s).unwrap_or(InsertType::Unknown(s))
    }
}

impl From<InsertType> for String {
    fn from(k: InsertType) -> String {
        match k {
            InsertType::Unknown(s) => s,
            k => k.id().to_string(),
        }
    }
}

/// One insert slot's settings on the control side: its kind, on or off, and its values
/// (by [`InsertType::settings`]; the first is the old `amount`). Empty and off by default.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct InsertSlot {
    pub kind: InsertType,
    pub on: bool,
    pub values: [u16; INSERT_VALUES],
}

impl Default for InsertSlot {
    fn default() -> InsertSlot {
        InsertSlot { kind: InsertType::None, on: false, values: [0; INSERT_VALUES] }
    }
}

impl InsertSlot {
    /// Kind `kind`, on, at its defaults.
    pub fn of(kind: InsertType) -> InsertSlot {
        InsertSlot { values: kind.defaults(), kind, on: true }
    }

    /// Empty and off (a rack leaves such a slot out of its file).
    pub fn is_default(&self) -> bool {
        *self == InsertSlot::default()
    }

    /// A new kind: its settings go back to that kind's defaults (as the Genos loads a type
    /// with its own settings). On/off is unchanged.
    pub fn set_kind(&mut self, kind: InsertType) {
        self.values = kind.defaults();
        self.kind = kind;
    }

    /// Setting `i` to `v`, clamped. Err: the kind has no setting `i`.
    pub fn set_value(&mut self, i: usize, v: u16) -> Result<(), String> {
        let spec = self.kind.settings().get(i).ok_or_else(|| format!("{} has no setting {}", self.kind.name(), i + 1))?;
        self.values[i] = spec.clamp(v);
        Ok(())
    }

    /// The old single-slot shape (`super::PartInsert`): the kind (an empty or unknown slot
    /// as an off distortion), on, and the first value as the amount.
    pub fn to_part_insert(&self) -> super::PartInsert {
        let effect = super::InsertEffect::from(self.kind.kind());
        let known = self.kind.kind() != super::InsertKind::None;
        super::PartInsert { effect, on: self.on && known, amount: if known { self.values[0].min(127) as u8 } else { super::PartInsert::OFF.amount } }
    }

    /// From the old single-slot shape: its effect with its amount as the first value, the
    /// other values at the kind's defaults. An off slot at its defaults is an empty slot.
    pub fn from_part_insert(p: super::PartInsert) -> InsertSlot {
        if p.is_default() {
            return InsertSlot::default();
        }
        let kind = InsertType::from(p.effect.kind());
        let mut values = kind.defaults();
        values[0] = p.amount as u16;
        InsertSlot { kind, on: p.on, values }
    }

    /// What the audio thread plays for it at `bpm`, fast or slow: the first value as the
    /// amount (off: nothing).
    pub fn settings(&self, bpm: f32, fast: bool) -> super::InsertSettings {
        let kind = if self.on { self.kind.kind() } else { super::InsertKind::None };
        let rest = [self.values[1], self.values[2], self.values[3]];
        super::InsertSettings { kind, amount: self.values[0].min(127) as u8, rest, bpm, fast }
    }
}

/// What a send effect plays. On the wire and in a rack file, a stable name (the three
/// buses' types by the names `FxType` gives them).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum SendKind {
    Hall,
    Room,
    Stage,
    Plate,
    Chorus,
    Celeste,
    Flanger,
    /// Delay 1/8.
    Eighth,
    /// Delay dotted 1/8.
    DottedEighth,
    /// Delay 1/4.
    Quarter,
    /// Delay 1/8, alternating left and right.
    PingPong,
    Phaser,
    /// A kind this build doesn't know (a newer build's), kept by name and written back
    /// unchanged. It is silent.
    Unknown(String),
}

impl SendKind {
    /// Every kind this build plays.
    pub const ALL: [SendKind; 12] = [
        SendKind::Hall,
        SendKind::Room,
        SendKind::Stage,
        SendKind::Plate,
        SendKind::Chorus,
        SendKind::Celeste,
        SendKind::Flanger,
        SendKind::Eighth,
        SendKind::DottedEighth,
        SendKind::Quarter,
        SendKind::PingPong,
        SendKind::Phaser,
    ];

    /// The stable name: "hall", "dottedEighth", "phaser", or an unknown kind's own.
    pub fn id(&self) -> &str {
        match self {
            SendKind::Hall => "hall",
            SendKind::Room => "room",
            SendKind::Stage => "stage",
            SendKind::Plate => "plate",
            SendKind::Chorus => "chorus",
            SendKind::Celeste => "celeste",
            SendKind::Flanger => "flanger",
            SendKind::Eighth => "eighth",
            SendKind::DottedEighth => "dottedEighth",
            SendKind::Quarter => "quarter",
            SendKind::PingPong => "pingPong",
            SendKind::Phaser => "phaser",
            SendKind::Unknown(s) => s,
        }
    }

    /// As the app shows it: "Hall", "Delay 1/8." (an unknown kind by its name).
    pub fn name(&self) -> &str {
        match self {
            SendKind::Hall => "Hall",
            SendKind::Room => "Room",
            SendKind::Stage => "Stage",
            SendKind::Plate => "Plate",
            SendKind::Chorus => "Chorus",
            SendKind::Celeste => "Celeste",
            SendKind::Flanger => "Flanger",
            SendKind::Eighth => "Delay 1/8",
            SendKind::DottedEighth => "Delay 1/8.",
            SendKind::Quarter => "Delay 1/4",
            SendKind::PingPong => "Ping-Pong",
            SendKind::Phaser => "Phaser",
            SendKind::Unknown(s) => s,
        }
    }

    /// The bus that plays it today (`super::REVERB`, `CHORUS`, `VARIATION`) and its type
    /// number there (`super::ReverbType as u8` etc.); None for a phaser or an unknown kind.
    pub fn bus(&self) -> Option<(usize, u8)> {
        Some(match self {
            SendKind::Hall => (super::REVERB, 0),
            SendKind::Room => (super::REVERB, 1),
            SendKind::Stage => (super::REVERB, 2),
            SendKind::Plate => (super::REVERB, 3),
            SendKind::Chorus => (super::CHORUS, 0),
            SendKind::Celeste => (super::CHORUS, 1),
            SendKind::Flanger => (super::CHORUS, 2),
            SendKind::Eighth => (super::VARIATION, 0),
            SendKind::DottedEighth => (super::VARIATION, 1),
            SendKind::Quarter => (super::VARIATION, 2),
            SendKind::PingPong => (super::VARIATION, 3),
            SendKind::Phaser | SendKind::Unknown(_) => return None,
        })
    }

    /// Bus `block`'s type number `index` (the inverse of [`SendKind::bus`]).
    pub fn of_bus(block: usize, index: u8) -> Option<SendKind> {
        SendKind::ALL.into_iter().find(|k| k.bus() == Some((block, index)))
    }

    /// Its parameters, in order: a bus kind's are its bus's (`super::Param::of_block`),
    /// a phaser's its own; none for an unknown kind.
    pub fn params(&self) -> Vec<KnobSpec> {
        match self.bus() {
            Some((block, _)) => {
                let d = self.defaults();
                super::Param::of_block(block)
                    .zip(d)
                    .map(|(p, default)| {
                        let s = p.spec();
                        KnobSpec { name: s.name, short: s.short, min: s.min, max: s.max, default, unit: Unit::Bus(p) }
                    })
                    .collect()
            }
            None if *self == SendKind::Phaser => PHASER.to_vec(),
            None => Vec::new(),
        }
    }

    /// Its parameters at the kind's defaults (unused ones 0).
    pub fn defaults(&self) -> [u16; SEND_PARAMS] {
        let mut v = [0; SEND_PARAMS];
        match self.bus() {
            Some((block, index)) => {
                let mut all = super::default_params();
                super::type_defaults(block, index, &mut all);
                for (o, p) in v.iter_mut().zip(super::Param::of_block(block)) {
                    *o = all[p.index()];
                }
            }
            None => {
                for (o, s) in v.iter_mut().zip(self.params()) {
                    *o = s.default;
                }
            }
        }
        v
    }
}

impl From<String> for SendKind {
    fn from(s: String) -> SendKind {
        SendKind::ALL.into_iter().find(|k| k.id() == s).unwrap_or(SendKind::Unknown(s))
    }
}

impl From<SendKind> for String {
    fn from(k: SendKind) -> String {
        match k {
            SendKind::Unknown(s) => s,
            k => k.id().to_string(),
        }
    }
}

/// One send effect's settings on the control side: its kind, its parameters (by
/// [`SendKind::params`]) and its return level (0-127, 64 = 0 dB, as the buses').
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendSlot {
    pub kind: SendKind,
    #[serde(default)]
    pub params: [u16; SEND_PARAMS],
    #[serde(default = "unity")]
    pub return_level: u8,
}

fn unity() -> u8 {
    super::RETURN_UNITY
}

impl SendSlot {
    /// Kind `kind` at its defaults, returning at 0 dB.
    pub fn of(kind: SendKind) -> SendSlot {
        SendSlot { params: kind.defaults(), kind, return_level: super::RETURN_UNITY }
    }

    /// A new kind: its parameters go back to that kind's defaults. The return is unchanged.
    pub fn set_kind(&mut self, kind: SendKind) {
        self.params = kind.defaults();
        self.kind = kind;
    }

    /// Parameter `i` to `v`, clamped. Err: the kind has no parameter `i`.
    pub fn set_param(&mut self, i: usize, v: u16) -> Result<(), String> {
        let spec = *self.kind.params().get(i).ok_or_else(|| format!("{} has no parameter {}", self.kind.name(), i + 1))?;
        self.params[i] = spec.clamp(v);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_types_round_trip_by_name() {
        for k in InsertType::ALL {
            let j = serde_json::to_string(&k).unwrap();
            assert_eq!(j, format!("\"{}\"", k.id()));
            assert_eq!(serde_json::from_str::<InsertType>(&j).unwrap(), k);
        }
        assert_eq!(serde_json::to_string(&InsertType::AutoWah).unwrap(), "\"autoWah\"");
    }

    #[test]
    fn an_unknown_insert_type_is_kept_and_plays_dry() {
        let k: InsertType = serde_json::from_str("\"ringModulator\"").unwrap();
        assert_eq!(k, InsertType::Unknown("ringModulator".into()));
        assert_eq!(serde_json::to_string(&k).unwrap(), "\"ringModulator\"");
        assert_eq!(k.kind(), crate::fx::InsertKind::None);
        assert!(k.settings().is_empty());
    }

    #[test]
    fn send_kinds_round_trip_by_name() {
        for k in SendKind::ALL {
            let j = serde_json::to_string(&k).unwrap();
            assert_eq!(serde_json::from_str::<SendKind>(&j).unwrap(), k);
            if let Some((b, i)) = k.bus() {
                assert_eq!(SendKind::of_bus(b, i), Some(k.clone()));
            }
        }
        assert_eq!(serde_json::to_string(&SendKind::DottedEighth).unwrap(), "\"dottedEighth\"");
    }

    #[test]
    fn an_unknown_send_kind_is_kept() {
        let s: SendSlot = serde_json::from_str(r#"{"kind":"shimmer","params":[1,2,3,4,5,6],"returnLevel":70}"#).unwrap();
        assert_eq!(s.kind, SendKind::Unknown("shimmer".into()));
        assert_eq!(serde_json::to_string(&s).unwrap(), r#"{"kind":"shimmer","params":[1,2,3,4,5,6],"returnLevel":70}"#);
        assert!(s.kind.params().is_empty());
    }

    #[test]
    fn every_insert_kind_has_two_to_four_settings() {
        for k in &InsertType::ALL[1..] {
            let n = k.settings().len();
            assert!((2..=INSERT_VALUES).contains(&n), "{k:?}: {n}");
            for s in k.settings() {
                assert!(s.min <= s.default && s.default <= s.max, "{k:?} {}", s.name);
            }
        }
    }

    #[test]
    fn the_first_value_is_the_old_amount() {
        use crate::fx::{InsertEffect, PartInsert};
        let old = PartInsert { effect: InsertEffect::Rotary, on: true, amount: 90 };
        let slot = InsertSlot::from_part_insert(old);
        assert_eq!((slot.kind.clone(), slot.on, slot.values[0]), (InsertType::Rotary, true, 90));
        assert_eq!(slot.to_part_insert(), old);
        assert_eq!(slot.settings(120.0, false).amount, 90);
        assert_eq!(InsertSlot::from_part_insert(PartInsert::OFF), InsertSlot::default());
    }

    #[test]
    fn a_kind_change_resets_its_values() {
        let mut s = InsertSlot::of(InsertType::Distortion);
        s.set_value(0, 500).unwrap();
        assert_eq!(s.values[0], 127, "clamped");
        assert!(s.set_value(3, 1).is_err(), "a distortion has three settings");
        s.set_kind(InsertType::Compressor);
        assert_eq!(s.values, InsertType::Compressor.defaults());
        let mut d = SendSlot::of(SendKind::Hall);
        assert_eq!(d.kind.params().len(), 3);
        d.set_kind(SendKind::PingPong);
        assert_eq!(d.params, SendKind::PingPong.defaults());
        assert_eq!(d.params[5], 1, "ping-pong on");
    }
}

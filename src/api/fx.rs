//! The shared effect bus (#204; `crate::fx`): each System Effect block's type and return
//! level, its band send (#236), the scale on every Style part's send to it, and its Multi
//! Pad send (#267), the same for the pads. The
//! keyboard parts' sends to it are `setPartSend` (CC91/93/94).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum FxCmd {
    /// A block's effect type: one of the block's own (`EffectBlockState::types`).
    SetEffectType { block: FxBlock, effect: FxType },
    /// A block's return level, 0-127 (64 = 0 dB, 127 = +6 dB, 0 = off).
    SetEffectReturn { block: FxBlock, level: u8 },
    /// A block's band send (#236): every Style part's send to it scaled, 0-127 % (100 =
    /// as the style wrote it, 0 = none of the band).
    SetBandSend { block: FxBlock, level: u8 },
    /// A block's Multi Pad send (#267): every Multi Pad's send to it scaled, 0-127 % (100
    /// = as the pad wrote it, 0 = none of the pads).
    SetPadSend { block: FxBlock, level: u8 },
    /// One of a block's parameters (#236; `EffectBlockState::params`), in its own unit,
    /// clamped to its range. A parameter of another block is refused.
    SetEffectParam { block: FxBlock, param: FxParam, value: u16 },
    /// Whether the block follows the style's own effect type (#237): on, it takes the
    /// style's type now and at every style change; `setEffectType` turns it off (the
    /// player's own choice stays).
    SetFollowStyle { block: FxBlock, on: bool },
    /// The style's insertion effects (#269, `EffectsState::inserts`) on or off, all
    /// together.
    SetInsertsOn { on: bool },
    /// One Style part's insertion effect (0-7) on or off; until the next style.
    SetPartInsertOn { part: u8, on: bool },
    /// One Style part's insertion effect amount (0-127: the distortion's drive, the
    /// compressor's squeeze, the wah's sensitivity, the tremolo's and rotary's depth); until
    /// the next style, which brings its own. A part with no insert is refused.
    SetPartInsertAmount { part: u8, amount: u8 },
    /// Every rotary insert at its fast speed or its slow one (the Leslie switch).
    SetRotaryFast { on: bool },
    /// The Master Compressor on or off (`EffectsState::master`).
    SetMasterCompressorOn { on: bool },
    /// The Master Compressor's type: its Compression, Texture and Output come with it.
    SetMasterCompressorPreset { preset: CompPreset },
    /// One Master Compressor parameter, clamped to its range: `compression` and `texture`
    /// 0-100 (%), `output` -12..12 (dB).
    SetMasterCompressorParam { param: CompParam, value: i16 },
    /// The Master EQ on or off.
    SetMasterEqOn { on: bool },
    /// The Master EQ's type: every band's gain, frequency, Q and shape come with it.
    SetMasterEqPreset { preset: EqPreset },
    /// One Master EQ band (0-7), clamped to its ranges (`crate::fx::master`). `shelf`
    /// only on bands 0 and 7.
    SetMasterEqBand { band: u8, gain: i8, freq: u16, q: u8, shelf: bool },
}

pub use crate::fx::master::{CompPreset, EqBand, EqPreset, MasterComp, MasterEq, MasterSettings};

/// A Master Compressor parameter (Genos RM p.136).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CompParam {
    /// 0-100 %: threshold, ratio and knee together.
    Compression,
    /// 0-100 %: higher is lighter (a faster attack and release).
    Texture,
    /// -12..12 dB: the level after it.
    Output,
}

/// The app API's view of [`MasterSettings`]. A trait, not an inherent impl, because
/// `MasterSettings` lives in yahaha-fx, which knows nothing of the API types.
pub trait MasterSettingsExt {
    /// A Master Compressor or Master EQ command applied here: None if `c` is another
    /// effects command, else whether it was taken (an error for a band out of range).
    /// The session and the dev mock both play the commands through it.
    fn apply(&mut self, c: &FxCmd) -> Option<Result<(), String>>;
    /// As the state shows them.
    fn state(&self) -> MasterFxState;
    /// The settings a state shows (the dev mock keeps only the state).
    fn from_state(s: &MasterFxState) -> MasterSettings;
}

impl MasterSettingsExt for MasterSettings {
    fn apply(&mut self, c: &FxCmd) -> Option<Result<(), String>> {
        let comp = &mut self.compressor;
        let eq = &mut self.eq;
        match *c {
            FxCmd::SetMasterCompressorOn { on } => comp.on = on,
            FxCmd::SetMasterCompressorPreset { preset } => *comp = MasterComp::of(comp.on, preset),
            FxCmd::SetMasterCompressorParam { param, value } => {
                match param {
                    CompParam::Compression => comp.compression = value.clamp(0, 100) as u8,
                    CompParam::Texture => comp.texture = value.clamp(0, 100) as u8,
                    CompParam::Output => comp.output = value.clamp(crate::fx::master::COMP_OUTPUT_DB.0 as i16, crate::fx::master::COMP_OUTPUT_DB.1 as i16) as i8,
                }
                *comp = comp.clamped();
            }
            FxCmd::SetMasterEqOn { on } => eq.on = on,
            FxCmd::SetMasterEqPreset { preset } => {
                eq.preset = preset;
                eq.bands = preset.bands();
            }
            FxCmd::SetMasterEqBand { band, gain, freq, q, shelf } => {
                let i = band as usize;
                if i >= crate::fx::master::EQ_BANDS {
                    return Some(Err(format!("the Master EQ has no band {band} (0-7)")));
                }
                eq.bands[i] = EqBand { gain, freq, q, shelf }.clamped(i);
            }
            _ => return None,
        }
        Some(Ok(()))
    }

    fn state(&self) -> MasterFxState {
        let (c, e) = (self.compressor.clamped(), self.eq.clamped());
        MasterFxState {
            compressor: MasterCompState { on: c.on, preset: c.preset, compression: c.compression, texture: c.texture, output: c.output, edited: c.edited() },
            eq: MasterEqState { on: e.on, preset: e.preset, bands: e.bands.to_vec(), edited: e.edited() },
        }
    }

    fn from_state(s: &MasterFxState) -> MasterSettings {
        let c = &s.compressor;
        let mut eq = MasterEq { on: s.eq.on, preset: s.eq.preset, ..MasterEq::default() };
        for (i, b) in s.eq.bands.iter().take(crate::fx::master::EQ_BANDS).enumerate() {
            eq.bands[i] = *b;
        }
        MasterSettings { compressor: MasterComp { on: c.on, preset: c.preset, compression: c.compression, texture: c.texture, output: c.output }, eq }
    }
}

/// The Master Compressor and Master EQ, on the whole mix after the effect returns.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterFxState {
    pub compressor: MasterCompState,
    pub eq: MasterEqState,
}

impl Default for MasterFxState {
    /// Both off: the Compressor at Natural, the EQ Flat.
    fn default() -> MasterFxState {
        MasterSettings::default().state()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterCompState {
    pub on: bool,
    /// The type the parameters started from.
    pub preset: CompPreset,
    /// 0-100 %.
    pub compression: u8,
    /// 0-100 %.
    pub texture: u8,
    /// -12..12 dB.
    pub output: i8,
    /// The parameters differ from the type's.
    pub edited: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterEqState {
    pub on: bool,
    /// The type the bands started from.
    pub preset: EqPreset,
    /// The eight bands, low to high.
    pub bands: Vec<EqBand>,
    /// The bands differ from the type's.
    pub edited: bool,
}

/// An effect parameter (#236): the bus's own (`crate::fx::Param`).
pub use crate::fx::Param as FxParam;

/// A System Effect block.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FxBlock {
    /// CC91.
    Reverb,
    /// CC93.
    Chorus,
    /// CC94: the tempo delay.
    Variation,
}

impl FxBlock {
    pub const ALL: [FxBlock; 3] = [FxBlock::Reverb, FxBlock::Chorus, FxBlock::Variation];
    /// Each block's type before anything sets it (Genos: Hall, Chorus; here the dotted 1/8
    /// delay).
    pub const DEFAULT_TYPES: [FxType; 3] = [FxType::Hall, FxType::Chorus, FxType::DottedEighth];

    pub fn index(self) -> usize {
        self as usize
    }

    /// Type `t`'s number within the block (`crate::fx::ReverbType as u8` etc.; 0 if it is
    /// another block's).
    pub fn type_index(self, t: FxType) -> u8 {
        self.types().iter().position(|x| *x == t).unwrap_or(0) as u8
    }

    pub fn name(self) -> &'static str {
        match self {
            FxBlock::Reverb => "Reverb",
            FxBlock::Chorus => "Chorus",
            FxBlock::Variation => "Variation",
        }
    }

    /// The types it offers, in the bus's own order (`crate::fx::ReverbType` etc.).
    pub fn types(self) -> &'static [FxType] {
        match self {
            FxBlock::Reverb => &[FxType::Hall, FxType::Room, FxType::Stage, FxType::Plate],
            FxBlock::Chorus => &[FxType::Chorus, FxType::Celeste, FxType::Flanger],
            FxBlock::Variation => &[FxType::Eighth, FxType::DottedEighth, FxType::Quarter, FxType::PingPong],
        }
    }
}

/// An effect type (each block has its own; see `FxBlock::types`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FxType {
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
}

impl FxType {
    pub fn name(self) -> &'static str {
        match self {
            FxType::Hall => "Hall",
            FxType::Room => "Room",
            FxType::Stage => "Stage",
            FxType::Plate => "Plate",
            FxType::Chorus => "Chorus",
            FxType::Celeste => "Celeste",
            FxType::Flanger => "Flanger",
            FxType::Eighth => "Delay 1/8",
            FxType::DottedEighth => "Delay 1/8.",
            FxType::Quarter => "Delay 1/4",
            FxType::PingPong => "Ping-Pong",
        }
    }
}

/// The effect blocks.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectsState {
    /// Reverb, Chorus, Variation.
    pub blocks: Vec<EffectBlockState>,
    /// The loaded style's insertion effects (#269), one per Style part at most.
    #[serde(default)]
    pub inserts: Vec<InsertState>,
    /// Whether they play (`setInsertsOn`).
    #[serde(default = "yes")]
    pub inserts_on: bool,
    /// The rotary inserts at their fast speed (`setRotaryFast`).
    #[serde(default)]
    pub rotary_fast: bool,
    /// The Master Compressor and Master EQ (both off by default).
    #[serde(default)]
    pub master: MasterFxState,
}

fn yes() -> bool {
    true
}

/// A style's insertion effect on one of its parts (#269).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InsertState {
    /// The Style part, 0-7 (Rhythm 1 ... Phrase 2).
    pub part: u8,
    /// "Chord 1".
    pub part_name: String,
    /// The XG type: "British Combo Classic".
    pub name: String,
    /// What plays it here; null: nothing near it, the part plays dry.
    pub effect: Option<InsertEffect>,
    /// This part's insert on (`setPartInsertOn`; the style's inserts also need `insertsOn`).
    #[serde(default = "yes")]
    pub on: bool,
    /// Its amount, 0-127 (`setPartInsertAmount`): the style's, or the player's.
    #[serde(default = "mid")]
    pub amount: u8,
}

fn mid() -> u8 {
    64
}

/// An insertion effect yahaha plays (`crate::fx::InsertKind`).
pub use crate::fx::InsertEffect;

impl EffectsState {
    /// The blocks with these types, return levels and band sends (by `FxBlock::index`).
    pub fn new(effect: [FxType; 3], returns: [u8; 3], band: [u8; 3], params: [u16; crate::fx::PARAMS]) -> EffectsState {
        let blocks = FxBlock::ALL
            .iter()
            .map(|&b| {
                let effect = effect[b.index()];
                EffectBlockState {
                    block: b,
                    name: b.name().into(),
                    effect,
                    effect_name: effect.name().into(),
                    types: b.types().iter().map(|&t| FxOption { effect: t, name: t.name().into() }).collect(),
                    return_level: returns[b.index()],
                    band_send: band[b.index()],
                    pad_send: crate::fx::PAD_SEND_DEFAULT[b.index()],
                    params: FxParamState::of_block(b, effect, &params),
                    style_effect: None,
                    follow_style: true,
                }
            })
            .collect();
        EffectsState { blocks, inserts: Vec::new(), inserts_on: true, rotary_fast: false, master: MasterFxState::default() }
    }

    /// As a session starts: Hall, Chorus, the dotted 1/8 delay, every return 64 (0 dB);
    /// the band's reverb as written, no band chorus or delay.
    pub fn initial() -> EffectsState {
        EffectsState::new(FxBlock::DEFAULT_TYPES, [crate::fx::RETURN_UNITY; 3], crate::fx::BAND_SEND_DEFAULT, crate::fx::default_params())
    }
}

/// One block.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectBlockState {
    pub block: FxBlock,
    /// "Reverb".
    pub name: String,
    pub effect: FxType,
    /// "Hall".
    pub effect_name: String,
    /// The types it offers.
    pub types: Vec<FxOption>,
    /// 0-127, 64 = 0 dB.
    pub return_level: u8,
    /// The band send (#236): every Style part's send to this block scaled, 0-127 % (100 =
    /// as written). Reverb 100, Chorus 0, Variation 0 until something sets it.
    pub band_send: u8,
    /// The Multi Pad send (#267): every Multi Pad's send to this block scaled, 0-127 % (100
    /// = as written). Reverb 100, Chorus 0, Variation 0 until something sets it.
    pub pad_send: u8,
    /// Its parameters (#236), in order. Reverb: time, pre-delay, tone. Chorus: rate,
    /// depth. Variation: tempo sync, note, time (ms), feedback, tone, ping-pong.
    pub params: Vec<FxParamState>,
    /// The loaded style's own type for this block (#237), null if the style sets none.
    pub style_effect: Option<StyleEffectState>,
    /// The block takes the style's type at each style change (#237, `setFollowStyle`).
    pub follow_style: bool,
}

/// A style's own effect type (#237).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleEffectState {
    /// The XG type's name: "Real Medium Hall", "Tempo Cross 1" ("XG 96/0" for one yahaha
    /// has no name for).
    pub name: String,
    /// The block's type it plays as; null: nothing near it here, so the block's default.
    pub effect: Option<FxType>,
}

/// One effect parameter as the app shows it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FxParamState {
    pub param: FxParam,
    /// "Time".
    pub name: String,
    /// In the parameter's own unit (see docs/app-api.md › Effects), `min..=max`.
    pub value: u16,
    pub min: u16,
    pub max: u16,
    /// The value the block's type starts it at (a type change goes back to it).
    pub default: u16,
    /// The value as it reads: "2.4 s".
    pub display: String,
}

impl FxParamState {
    /// Block `b`'s parameters at `values`, with type `effect`'s defaults.
    pub fn of_block(b: FxBlock, effect: FxType, values: &[u16; crate::fx::PARAMS]) -> Vec<FxParamState> {
        let mut defaults = crate::fx::default_params();
        crate::fx::type_defaults(b.index(), b.type_index(effect), &mut defaults);
        FxParam::of_block(b.index())
            .map(|p| {
                let s = p.spec();
                let value = p.clamp(values[p.index()]);
                FxParamState { param: p, name: s.name.into(), value, min: s.min, max: s.max, default: defaults[p.index()], display: p.display(value) }
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FxOption {
    pub effect: FxType,
    pub name: String,
}

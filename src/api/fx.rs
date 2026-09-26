//! The shared effect bus (#204; `crate::fx`): each System Effect block's type and return
//! level, and its band send (#236), the scale on every Style part's send to it. The
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
    /// One of a block's parameters (#236; `EffectBlockState::params`), in its own unit,
    /// clamped to its range. A parameter of another block is refused.
    SetEffectParam { block: FxBlock, param: FxParam, value: u16 },
    /// Whether the block follows the style's own effect type (#237): on, it takes the
    /// style's type now and at every style change; `setEffectType` turns it off (the
    /// player's own choice stays).
    SetFollowStyle { block: FxBlock, on: bool },
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
}

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
                    params: FxParamState::of_block(b, effect, &params),
                    style_effect: None,
                    follow_style: true,
                }
            })
            .collect();
        EffectsState { blocks }
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

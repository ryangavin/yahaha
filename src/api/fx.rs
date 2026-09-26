//! The shared effect bus (#204; `crate::fx`): each System Effect block's type and return
//! level. The parts' sends to it are `setPartSend` (CC91/93/94).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum FxCmd {
    /// A block's effect type: one of the block's own (`EffectBlockState::types`).
    SetEffectType { block: FxBlock, effect: FxType },
    /// A block's return level, 0-127 (64 = 0 dB, 127 = +6 dB, 0 = off).
    SetEffectReturn { block: FxBlock, level: u8 },
}

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

    pub fn index(self) -> usize {
        self as usize
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
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FxOption {
    pub effect: FxType,
    pub name: String,
}

//! Style Setting > Change Behavior (RM p.12-13): what changing the style does to the
//! tempo, the Style part on/off states and the Main section.

use crate::engine::{ChangeRule, ChangeRules};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum StyleChangeCmd {
    /// Change Behavior > Tempo.
    SetTempoChange { rule: ChangeRuleMode },
    /// Change Behavior > Part On/Off.
    SetPartsChange { rule: ChangeRuleMode },
    /// Change Behavior > Section Set: the Main (0-3 = A-D) a style chosen while stopped
    /// starts on, or null (Off) to keep the Main selected.
    SetSectionSet { section: Option<u8> },
    /// The assignable "Style Tempo Lock/Reset": Tempo Reset -> Lock, anything else -> Reset.
    ToggleStyleTempoLock,
    /// The assignable "Style Tempo Hold/Reset": Tempo Reset -> Hold, anything else -> Reset.
    ToggleStyleTempoHold,
}

/// Lock / Hold / Reset.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChangeRuleMode {
    /// Always keep the old style's value.
    Lock,
    /// Keep it while the band plays; take the new style's when stopped.
    #[default]
    Hold,
    /// Always take the new style's (its tempo; every part on).
    Reset,
}

impl From<ChangeRuleMode> for ChangeRule {
    fn from(m: ChangeRuleMode) -> ChangeRule {
        match m {
            ChangeRuleMode::Lock => ChangeRule::Lock,
            ChangeRuleMode::Hold => ChangeRule::Hold,
            ChangeRuleMode::Reset => ChangeRule::Reset,
        }
    }
}

impl From<ChangeRule> for ChangeRuleMode {
    fn from(m: ChangeRule) -> ChangeRuleMode {
        match m {
            ChangeRule::Lock => ChangeRuleMode::Lock,
            ChangeRule::Hold => ChangeRuleMode::Hold,
            ChangeRule::Reset => ChangeRuleMode::Reset,
        }
    }
}

/// Change Behavior.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleChangeState {
    pub tempo: ChangeRuleMode,
    pub parts: ChangeRuleMode,
    /// The Main (0-3) a style chosen while stopped starts on; null = Off (keep it).
    pub section_set: Option<u8>,
}

impl From<StyleChangeState> for ChangeRules {
    fn from(s: StyleChangeState) -> ChangeRules {
        ChangeRules { tempo: s.tempo.into(), parts: s.parts.into(), section: s.section_set }
    }
}

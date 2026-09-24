//! Style Setting > Change Behavior: the control side keeps the settings (for the state)
//! and hands the engine a copy, which applies them at the next style change.

use super::Control;
use crate::api::{ChangeRuleMode, CmdError, StyleChangeCmd};
use crate::live::Cmd;

impl Control {
    pub(super) fn style_change_cmd(&mut self, c: StyleChangeCmd) -> Result<(), CmdError> {
        let mut s = self.style_change;
        let flip = |cur: ChangeRuleMode, to: ChangeRuleMode| if cur == ChangeRuleMode::Reset { to } else { ChangeRuleMode::Reset };
        match c {
            StyleChangeCmd::SetTempoChange { rule } => s.tempo = rule,
            StyleChangeCmd::SetPartsChange { rule } => s.parts = rule,
            StyleChangeCmd::SetSectionSet { section } => s.section_set = section.map(|m| m.min(3)),
            StyleChangeCmd::ToggleStyleTempoLock => s.tempo = flip(s.tempo, ChangeRuleMode::Lock),
            StyleChangeCmd::ToggleStyleTempoHold => s.tempo = flip(s.tempo, ChangeRuleMode::Hold),
        }
        self.engine_cmd(Cmd::ChangeRules(s.into()))?;
        self.style_change = s;
        Ok(())
    }
}

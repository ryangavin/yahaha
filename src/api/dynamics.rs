//! Style Dynamics Control (Genos2: Menu > Style Setting > Dynamics Control; the Live
//! Control / Assignable "Dynamics Control"), Touch and Accent (#180; engine/dynamics.rs).
//!
//! - **Dynamics level** (0-127, 64 = as written): scales the velocity of every Style note.
//! - **Dynamics Control** (Style Setting, on/off): whether the level acts on the Style.
//! - **Touch**: each key struck in the chord section sets the level from its velocity.
//! - **Accent**: a chord-section key struck at or above the threshold plays the Main's fill.
//!
//! All of them are System settings, not stored in Registration (Genos Data List, Parameter
//! Chart: Dynamics Control is System only).

use crate::engine::DynamicsSettings;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum DynamicsCmd {
    /// Style Setting > Dynamics Control: the level may act on the Style (on) or the Style
    /// plays as written (off).
    SetDynamicsControl { on: bool },
    /// The Dynamics level, 0-127 (64: as written).
    SetDynamics { level: u8 },
    /// The Dynamics level moved by `delta` (clamped to 0-127).
    StepDynamics { delta: i8 },
    /// Touch: chord-section strikes set the level.
    SetDynamicsTouch { on: bool },
    ToggleDynamicsTouch,
    /// Accent: a hard chord-section strike plays the Main's fill.
    SetAccent { on: bool },
    ToggleAccent,
    /// The Accent threshold, a velocity 1-127.
    SetAccentThreshold { velocity: u8 },
}

/// Style Dynamics Control, Touch and Accent.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DynamicsState {
    /// Style Setting > Dynamics Control.
    pub control: bool,
    /// The level in effect, 0-127 (64: as written); Touch moves it.
    pub level: u8,
    pub touch: bool,
    pub accent: bool,
    /// The Accent threshold (velocity).
    pub accent_threshold: u8,
}

impl Default for DynamicsState {
    fn default() -> DynamicsState {
        DynamicsSettings::default().into()
    }
}

impl From<DynamicsSettings> for DynamicsState {
    fn from(s: DynamicsSettings) -> DynamicsState {
        DynamicsState { control: s.control, level: s.level, touch: s.touch, accent: s.accent, accent_threshold: s.accent_min }
    }
}

impl DynamicsCmd {
    /// The settings `s` with this command applied.
    pub fn apply(&self, s: DynamicsSettings) -> DynamicsSettings {
        let mut s = s;
        match *self {
            DynamicsCmd::SetDynamicsControl { on } => s.control = on,
            DynamicsCmd::SetDynamics { level } => s.level = level,
            DynamicsCmd::StepDynamics { delta } => s.level = (s.level as i16 + delta as i16).clamp(0, 127) as u8,
            DynamicsCmd::SetDynamicsTouch { on } => s.touch = on,
            DynamicsCmd::ToggleDynamicsTouch => s.touch = !s.touch,
            DynamicsCmd::SetAccent { on } => s.accent = on,
            DynamicsCmd::ToggleAccent => s.accent = !s.accent,
            DynamicsCmd::SetAccentThreshold { velocity } => s.accent_min = velocity,
        }
        s.clamped()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_apply_and_clamp() {
        let s = DynamicsSettings::default();
        assert_eq!(DynamicsCmd::StepDynamics { delta: -100 }.apply(s).level, 0);
        assert_eq!(DynamicsCmd::StepDynamics { delta: 100 }.apply(s).level, 127);
        assert_eq!(DynamicsCmd::SetDynamics { level: 200 }.apply(s).level, 127);
        assert_eq!(DynamicsCmd::SetAccentThreshold { velocity: 0 }.apply(s).accent_min, 1);
        assert!(DynamicsCmd::ToggleAccent.apply(s).accent);
        assert!(DynamicsCmd::ToggleDynamicsTouch.apply(s).touch);
        assert!(!DynamicsCmd::SetDynamicsControl { on: false }.apply(s).control);
    }
}

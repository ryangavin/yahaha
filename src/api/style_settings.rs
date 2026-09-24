//! Style settings: Section Change Timing, Synchro Stop Window, Fade In/Out times, Tap
//! Tempo's Style Section Reset and the Style Retrigger length (Genos Menu › Style Setting,
//! Metronome › Tap Tempo, Assignable › Fade In/Out, Live Control › RtgRate).

use crate::engine::{IntroEndingTiming, MainTiming, StyleSettings};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum StyleSettingsCmd {
    /// Section Change Timing, To Main [A]-[D] (and a style change while playing):
    /// `"immediate"` (the next beat) or `"nextBar"`.
    SetMainTiming { timing: MainTiming },
    /// Section Change Timing, Inside Intro/Ending: `"nextBar"` or `"endOfSection"`.
    SetIntroEndingTiming { timing: IntroEndingTiming },
    /// Synchro Stop Window in ms (0 = Off, up to 5000).
    SetSyncStopWindow { ms: u16 },
    /// Fade In Time in ms (0-20000).
    SetFadeInTime { ms: u16 },
    /// Fade Out Time in ms (0-20000).
    SetFadeOutTime { ms: u16 },
    /// Fade Out Hold Time in ms (0-5000).
    SetFadeHoldTime { ms: u16 },
    /// Tap Tempo › Style Section Reset: TAP TEMPO while the style plays rewinds the
    /// section (on) or sets the tempo (off).
    SetSectionReset { on: bool },
    /// Style Retrigger length: 1, 2, 4, 8, 16 or 32 (a whole note .. a 32nd; other values
    /// snap down to one of these).
    SetRetriggerRate { rate: u8 },
    /// Style Retrigger length `delta` steps along 1, 2, 4, 8, 16, 32 (positive: shorter).
    StepRetriggerRate { delta: i8 },
}

/// The Style settings in use.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleSettingsState {
    pub main_timing: MainTiming,
    pub intro_ending_timing: IntroEndingTiming,
    /// Synchro Stop Window in ms; 0 = Off.
    pub sync_stop_window_ms: u16,
    pub fade_in_ms: u16,
    pub fade_out_ms: u16,
    pub fade_hold_ms: u16,
    pub section_reset: bool,
    /// 1, 2, 4, 8, 16 or 32.
    pub retrigger_rate: u8,
}

impl Default for StyleSettingsState {
    fn default() -> StyleSettingsState {
        StyleSettings::default().into()
    }
}

impl From<StyleSettings> for StyleSettingsState {
    fn from(s: StyleSettings) -> StyleSettingsState {
        StyleSettingsState {
            main_timing: s.main_timing,
            intro_ending_timing: s.intro_ending_timing,
            sync_stop_window_ms: s.sync_stop_window_ms,
            fade_in_ms: s.fade_in_ms,
            fade_out_ms: s.fade_out_ms,
            fade_hold_ms: s.fade_hold_ms,
            section_reset: s.section_reset,
            retrigger_rate: s.retrigger_rate,
        }
    }
}

impl StyleSettingsCmd {
    /// The settings `s` with this command applied.
    pub fn apply(&self, s: StyleSettings) -> StyleSettings {
        let mut s = s;
        match *self {
            StyleSettingsCmd::SetMainTiming { timing } => s.main_timing = timing,
            StyleSettingsCmd::SetIntroEndingTiming { timing } => s.intro_ending_timing = timing,
            StyleSettingsCmd::SetSyncStopWindow { ms } => s.sync_stop_window_ms = ms,
            StyleSettingsCmd::SetFadeInTime { ms } => s.fade_in_ms = ms,
            StyleSettingsCmd::SetFadeOutTime { ms } => s.fade_out_ms = ms,
            StyleSettingsCmd::SetFadeHoldTime { ms } => s.fade_hold_ms = ms,
            StyleSettingsCmd::SetSectionReset { on } => s.section_reset = on,
            StyleSettingsCmd::SetRetriggerRate { rate } => s.retrigger_rate = rate,
            StyleSettingsCmd::StepRetriggerRate { delta } => s.retrigger_rate = StyleSettings::step_rate(s.retrigger_rate, delta),
        }
        s.clamped()
    }
}

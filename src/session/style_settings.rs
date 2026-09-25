//! Style settings: Section Change Timing, Synchro Stop Window, fade times, Section Reset,
//! Retrigger length. The session keeps them and hands the engine the whole set on each
//! change (`Cmd::StyleSettings`).

use super::Control;
use crate::api::{CmdError, StyleSettingsCmd};
use crate::engine::{MainTiming, StyleControls};
use crate::live::Cmd;
use crate::registration::{Group, Groups};
use serde::{Deserialize, Serialize};
use serde_json::Value;

impl Control {
    pub(super) fn style_settings_cmd(&mut self, c: StyleSettingsCmd) -> Result<(), CmdError> {
        let s = c.apply(self.style_settings);
        self.engine_cmd(Cmd::StyleSettings(s))?;
        self.style_settings = s;
        Ok(())
    }
}

// ----- Registration (#107) -----

/// The Style settings a registration stores (Genos Data List, Parameter Chart: the
/// Registration column). Group Style: Style Retrigger On/Off and Rate, Synchro Stop Window,
/// Tap Tempo's Style Section Reset, and (Genos2 Reference Manual, Style Setting) Section
/// Change Timing To Main. Group Assignable: Fade In Time, Fade Out Time, Fade Out Hold Time.
/// Not stored: Inside Intro/Ending timing (the manual names only To Main as loaded by a
/// Registration). Every field is optional: a bank from an earlier build, or a memory
/// without that group, leaves the setting alone.
#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StyleSettingsReg {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    main_timing: Option<MainTiming>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    retrigger: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    retrigger_rate: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sync_stop_window_ms: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    section_reset: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fade_in_ms: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fade_out_ms: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fade_hold_ms: Option<u16>,
}

pub(super) fn style_settings_capture(c: &Control, g: Groups) -> Option<Value> {
    let (style, assign) = (g.has(Group::Style), g.has(Group::Assignable));
    if !style && !assign {
        return None;
    }
    let s = c.style_settings;
    let r = StyleSettingsReg {
        main_timing: style.then_some(s.main_timing),
        retrigger: style.then_some(c.snap.retrigger),
        retrigger_rate: style.then_some(s.retrigger_rate),
        sync_stop_window_ms: style.then_some(s.sync_stop_window_ms),
        section_reset: style.then_some(s.section_reset),
        fade_in_ms: assign.then_some(s.fade_in_ms),
        fade_out_ms: assign.then_some(s.fade_out_ms),
        fade_hold_ms: assign.then_some(s.fade_hold_ms),
    };
    serde_json::to_value(&r).ok()
}

pub(super) fn style_settings_recall(c: &mut Control, v: &Value, g: Groups) -> Result<(), String> {
    let r: StyleSettingsReg = serde_json::from_value(v.clone()).map_err(|e| format!("registration styleSettings: {e}"))?;
    let (style, assign) = (g.has(Group::Style), g.has(Group::Assignable));
    let mut s = c.style_settings;
    if style {
        s.main_timing = r.main_timing.unwrap_or(s.main_timing);
        s.retrigger_rate = r.retrigger_rate.unwrap_or(s.retrigger_rate);
        s.sync_stop_window_ms = r.sync_stop_window_ms.unwrap_or(s.sync_stop_window_ms);
        s.section_reset = r.section_reset.unwrap_or(s.section_reset);
    }
    if assign {
        s.fade_in_ms = r.fade_in_ms.unwrap_or(s.fade_in_ms);
        s.fade_out_ms = r.fade_out_ms.unwrap_or(s.fade_out_ms);
        s.fade_hold_ms = r.fade_hold_ms.unwrap_or(s.fade_hold_ms);
    }
    let s = s.clamped();
    if s != c.style_settings {
        c.engine_cmd(Cmd::StyleSettings(s)).map_err(|e| e.to_string())?;
        c.style_settings = s;
    }
    // Retrigger on/off is the engine's switch: a state, which it compares with its own.
    if let Some(on) = r.retrigger.filter(|_| style) {
        let set = StyleControls { retrigger: Some(on), ..StyleControls::default() };
        c.engine_cmd(Cmd::StyleControls(set)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

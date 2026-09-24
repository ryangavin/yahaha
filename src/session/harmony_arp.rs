//! Keyboard Harmony / Arpeggio: the switch, the type and the detail settings. The control
//! side keeps them (`Control::harmony_arp`) and publishes them to the real-time threads as
//! one packed word (`Shared::kbd_fx`); the input thread's processor and the engine thread's
//! `live::KbdFx` pick it up on their next key or wake (see `live/kbdfx.rs`).

use super::Control;
use crate::api::{
    arp_category_name, harmony_category_name, ArpQuantize, ArpSettings, ArpVelocityMode, CmdError, HarmonyArpCmd,
    HarmonyArpMode, HarmonyArpState, HarmonyAssign, HarmonySpeed,
};
use crate::arp::{library::PATTERNS, Quantize, Velocity};
use crate::harmony::{Assign, EchoSpeed, ALL_TYPES};
use crate::live::{type_index, FxMode};
use crate::registration::{Group, Groups};
use std::sync::atomic::Ordering::Release;

impl Control {
    pub(super) fn harmony_arp_cmd(&mut self, c: HarmonyArpCmd) -> Result<(), CmdError> {
        let h = &mut self.harmony_arp;
        match c {
            HarmonyArpCmd::ToggleHarmonyArp => h.on = !h.on,
            HarmonyArpCmd::SetHarmonyArpOn { on } => h.on = on,
            HarmonyArpCmd::SetHarmonyType { index } => {
                let Some(&t) = ALL_TYPES.get(index as usize) else {
                    return self.fail(format!("no Harmony type {index} (0-{})", ALL_TYPES.len() - 1));
                };
                h.mode = FxMode::Harmony;
                h.harmony.ty = t;
            }
            HarmonyArpCmd::SetArpPattern { index } => {
                if index as usize >= PATTERNS.len() {
                    return self.fail(format!("no arpeggio pattern {index} (0-{})", PATTERNS.len() - 1));
                }
                h.mode = FxMode::Arpeggio;
                h.pattern = index;
            }
            HarmonyArpCmd::StepHarmonyArpType { delta } => {
                let n = (ALL_TYPES.len() + PATTERNS.len()) as i32;
                let cur = match h.mode {
                    FxMode::Harmony => type_index(h.harmony.ty) as i32,
                    FxMode::Arpeggio => ALL_TYPES.len() as i32 + h.pattern as i32,
                };
                let next = (cur + delta as i32).rem_euclid(n) as usize;
                if next < ALL_TYPES.len() {
                    h.mode = FxMode::Harmony;
                    h.harmony.ty = ALL_TYPES[next];
                } else {
                    h.mode = FxMode::Arpeggio;
                    h.pattern = (next - ALL_TYPES.len()) as u8;
                }
            }
            HarmonyArpCmd::SetHarmonyVolume { volume } => h.harmony.volume = volume.min(127),
            HarmonyArpCmd::SetHarmonySpeed { speed } => h.harmony.speed = speed_of(speed),
            HarmonyArpCmd::SetHarmonyAssign { assign } => h.harmony.assign = assign_of(assign),
            HarmonyArpCmd::SetChordNoteOnly { on } => h.harmony.chord_note_only = on,
            HarmonyArpCmd::SetTouchLimit { velocity } => h.harmony.min_velocity = velocity.clamp(1, 127),
            HarmonyArpCmd::SetArpQuantize { quantize } => {
                h.quantize = match quantize {
                    ArpQuantize::Off => Quantize::Off,
                    ArpQuantize::Eighth => Quantize::Eighth,
                    ArpQuantize::Sixteenth => Quantize::Sixteenth,
                }
            }
            HarmonyArpCmd::SetArpHold { on } => h.hold = on,
            HarmonyArpCmd::ToggleArpHold => h.hold = !h.hold,
            HarmonyArpCmd::SetArpPedalHold { on } => h.pedal_hold = on,
            HarmonyArpCmd::ToggleArpPedalHold => h.pedal_hold = !h.pedal_hold,
            HarmonyArpCmd::SetArpVelocity { mode, velocity } => {
                h.velocity = match mode {
                    ArpVelocityMode::Original => Velocity::Original,
                    ArpVelocityMode::Thru => Velocity::Thru,
                    ArpVelocityMode::Fixed => Velocity::Fixed(velocity.clamp(1, 127)),
                }
            }
            HarmonyArpCmd::SetArpKeepKeyOn { on } => h.keep_key_on = on,
        }
        self.shared.kbd_fx.store(self.harmony_arp.pack(), Release);
        self.wake_engine();
        Ok(())
    }

    pub(super) fn harmony_arp_state(&self) -> HarmonyArpState {
        let h = &self.harmony_arp;
        let pattern = &PATTERNS[(h.pattern as usize).min(PATTERNS.len() - 1)];
        let (type_name, category) = match h.mode {
            FxMode::Harmony => (h.harmony.ty.name().to_string(), harmony_category_name(h.harmony.ty).to_string()),
            FxMode::Arpeggio => (pattern.name.to_string(), arp_category_name(pattern.category).to_string()),
        };
        let (velocity, fixed_velocity) = match h.velocity {
            Velocity::Original => (ArpVelocityMode::Original, 100),
            Velocity::Thru => (ArpVelocityMode::Thru, 100),
            Velocity::Fixed(v) => (ArpVelocityMode::Fixed, v),
        };
        HarmonyArpState {
            on: h.on,
            mode: match h.mode {
                FxMode::Harmony => HarmonyArpMode::Harmony,
                FxMode::Arpeggio => HarmonyArpMode::Arpeggio,
            },
            harmony_type: type_index(h.harmony.ty),
            arp_pattern: h.pattern,
            type_name,
            category,
            volume: h.harmony.volume,
            speed: match h.harmony.speed {
                EchoSpeed::Quarter => HarmonySpeed::Quarter,
                EchoSpeed::QuarterTriplet => HarmonySpeed::QuarterTriplet,
                EchoSpeed::Eighth => HarmonySpeed::Eighth,
                EchoSpeed::EighthTriplet => HarmonySpeed::EighthTriplet,
                EchoSpeed::Sixteenth => HarmonySpeed::Sixteenth,
                EchoSpeed::ThirtySecond => HarmonySpeed::ThirtySecond,
            },
            assign: match h.harmony.assign {
                Assign::Auto => HarmonyAssign::Auto,
                Assign::Multi => HarmonyAssign::Multi,
                Assign::Right1 => HarmonyAssign::Right1,
                Assign::Right2 => HarmonyAssign::Right2,
                Assign::Right3 => HarmonyAssign::Right3,
            },
            chord_note_only: h.harmony.chord_note_only,
            touch_limit: h.harmony.min_velocity,
            arp: ArpSettings {
                quantize: match h.quantize {
                    Quantize::Off => ArpQuantize::Off,
                    Quantize::Eighth => ArpQuantize::Eighth,
                    Quantize::Sixteenth => ArpQuantize::Sixteenth,
                },
                hold: h.hold,
                pedal_hold: h.pedal_hold,
                velocity,
                fixed_velocity,
                keep_key_on: h.keep_key_on,
            },
        }
    }
}

// ----- Registration (group Keyboard Harmony/Arpeggio) -----

/// The `harmonyArp` section of a Registration Memory (Data List, Freeze group "Keyboard
/// Harmony/Arpeggio"): the HARMONY/ARPEGGIO switch, the selected type and the detail
/// settings. The type and pattern are stored by name, so a list that grows or reorders
/// still finds them. The Arpeggio Hold pedal function (`pedalHold`) is not stored: it is
/// the pedal's, not a setting.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct HarmonyArpReg {
    on: bool,
    mode: HarmonyArpMode,
    harmony_type: String,
    arp_pattern: String,
    volume: u8,
    speed: HarmonySpeed,
    assign: HarmonyAssign,
    chord_note_only: bool,
    touch_limit: u8,
    arp_quantize: ArpQuantize,
    arp_hold: bool,
    arp_velocity: ArpVelocityMode,
    arp_fixed_velocity: u8,
    arp_keep_key_on: bool,
}

pub(super) fn harmony_arp_capture(c: &Control, g: Groups) -> Option<serde_json::Value> {
    if !g.has(Group::HarmonyArp) {
        return None;
    }
    let s = c.harmony_arp_state();
    let h = &c.harmony_arp;
    let r = HarmonyArpReg {
        on: s.on,
        mode: s.mode,
        harmony_type: h.harmony.ty.name().to_string(),
        arp_pattern: PATTERNS[(h.pattern as usize).min(PATTERNS.len() - 1)].name.to_string(),
        volume: s.volume,
        speed: s.speed,
        assign: s.assign,
        chord_note_only: s.chord_note_only,
        touch_limit: s.touch_limit,
        arp_quantize: s.arp.quantize,
        arp_hold: s.arp.hold,
        arp_velocity: s.arp.velocity,
        arp_fixed_velocity: s.arp.fixed_velocity,
        arp_keep_key_on: s.arp.keep_key_on,
    };
    serde_json::to_value(&r).ok()
}

/// Recall the `harmonyArp` section. A type or pattern this build doesn't have is reported;
/// the other settings are still recalled and the selection stays as it is.
pub(super) fn harmony_arp_recall(c: &mut Control, v: &serde_json::Value, g: Groups) -> Result<(), String> {
    if !g.has(Group::HarmonyArp) {
        return Ok(());
    }
    let r: HarmonyArpReg = serde_json::from_value(v.clone()).map_err(|e| format!("registration harmonyArp: {e}"))?;
    let ty = ALL_TYPES.iter().position(|t| t.name() == r.harmony_type).map(|i| i as u8);
    let pattern = PATTERNS.iter().position(|p| p.name == r.arp_pattern).map(|i| i as u8);
    let mut err = None;
    let mut cmds = vec![
        HarmonyArpCmd::SetHarmonyVolume { volume: r.volume },
        HarmonyArpCmd::SetHarmonySpeed { speed: r.speed },
        HarmonyArpCmd::SetHarmonyAssign { assign: r.assign },
        HarmonyArpCmd::SetChordNoteOnly { on: r.chord_note_only },
        HarmonyArpCmd::SetTouchLimit { velocity: r.touch_limit },
        HarmonyArpCmd::SetArpQuantize { quantize: r.arp_quantize },
        HarmonyArpCmd::SetArpHold { on: r.arp_hold },
        HarmonyArpCmd::SetArpVelocity { mode: r.arp_velocity, velocity: r.arp_fixed_velocity },
        HarmonyArpCmd::SetArpKeepKeyOn { on: r.arp_keep_key_on },
    ];
    // Both selections, the one in use last (it sets the mode).
    let (harmony, arp) = (ty.map(|index| HarmonyArpCmd::SetHarmonyType { index }), pattern.map(|index| HarmonyArpCmd::SetArpPattern { index }));
    let (first, last, missing) = match r.mode {
        HarmonyArpMode::Harmony => (arp, harmony, ty.is_none().then_some(&r.harmony_type)),
        HarmonyArpMode::Arpeggio => (harmony, arp, pattern.is_none().then_some(&r.arp_pattern)),
    };
    if let Some(name) = missing {
        err = Some(format!("Harmony/Arpeggio type not found: {name}"));
    }
    cmds.extend(first);
    cmds.extend(last);
    cmds.push(HarmonyArpCmd::SetHarmonyArpOn { on: r.on });
    for cmd in cmds {
        c.harmony_arp_cmd(cmd).map_err(|e| e.to_string())?;
    }
    err.map_or(Ok(()), Err)
}

fn speed_of(s: HarmonySpeed) -> EchoSpeed {
    match s {
        HarmonySpeed::Quarter => EchoSpeed::Quarter,
        HarmonySpeed::QuarterTriplet => EchoSpeed::QuarterTriplet,
        HarmonySpeed::Eighth => EchoSpeed::Eighth,
        HarmonySpeed::EighthTriplet => EchoSpeed::EighthTriplet,
        HarmonySpeed::Sixteenth => EchoSpeed::Sixteenth,
        HarmonySpeed::ThirtySecond => EchoSpeed::ThirtySecond,
    }
}

fn assign_of(a: HarmonyAssign) -> Assign {
    match a {
        HarmonyAssign::Auto => Assign::Auto,
        HarmonyAssign::Multi => Assign::Multi,
        HarmonyAssign::Right1 => Assign::Right1,
        HarmonyAssign::Right2 => Assign::Right2,
        HarmonyAssign::Right3 => Assign::Right3,
    }
}

#[cfg(test)]
#[path = "harmony_arp_tests.rs"]
mod tests;

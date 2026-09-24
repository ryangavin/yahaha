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
                velocity,
                fixed_velocity,
                keep_key_on: h.keep_key_on,
            },
        }
    }
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

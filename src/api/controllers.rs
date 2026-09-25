//! Controllers: pedals, the pitch-bend and modulation wheels, and the pedals' assignable
//! functions (`crate::controllers`).

use crate::controllers::{ControlType, Controllers, Function, PartTargets, PedalSetup, Range, PEDALS, SOFT, SOSTENUTO, SUSTAIN};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ControllersCmd {
    /// Set up pedal `pedal` (0-2): the CC it listens for on the keyboards (null: none), its
    /// function (`Function`, e.g. "sustain", "startStop", "fillUp"), Control Type
    /// ("holdA", "holdB", "toggle"), reversed polarity, and Range for Pitch Bend ("upper",
    /// "lower", "full").
    SetPedal {
        pedal: u8,
        cc: Option<u8>,
        function: Function,
        #[serde(default)]
        control_type: ControlType,
        #[serde(default)]
        reverse: bool,
        #[serde(default)]
        range: Range,
    },
    /// Pedal `pedal` takes the CC of the next control change a keyboard sends (a pedal
    /// press); null stops learning.
    LearnPedal { pedal: Option<u8> },
    /// Which controllers reach keyboard part `part` (0-3): the pedal switches (sustain,
    /// sostenuto, soft), pitch bend, modulation.
    SetPartControllers { part: u8, sustain: bool, pitch_bend: bool, modulation: bool },
    /// Keyboard part `part`'s Pitch Bend Range, 0-12 semitones.
    SetBendRange { part: u8, semitones: u8 },
    /// Run an assignable function now, as a pedal press would.
    TriggerFunction { function: Function },
}

impl ControllersCmd {
    /// Apply a setting (`SetPedal`, `LearnPedal`, `SetPartControllers`, `SetBendRange`) to
    /// `c`: Ok(true) if it was one, Ok(false) for `TriggerFunction` (the caller runs it),
    /// Err for a pedal that doesn't exist. The session and the app's Rust mock share it.
    pub fn apply_setting(&self, c: &Controllers) -> Result<bool, String> {
        match *self {
            ControllersCmd::SetPedal { pedal, cc, function, control_type, reverse, range } => {
                if pedal as usize >= PEDALS {
                    return Err(format!("there is no pedal {}", pedal as usize + 1));
                }
                if let Some(why) = cc.and_then(|cc| crate::controllers::pedal_cc_refused(cc).map(|w| (cc, w))) {
                    return Err(format!("a pedal can't use CC {}: it is {}", why.0, why.1));
                }
                c.set_pedal(pedal as usize, PedalSetup { cc, function, control_type, reverse, range });
            }
            ControllersCmd::LearnPedal { pedal } => c.learn(pedal.map(|p| p as usize).filter(|&p| p < PEDALS)),
            ControllersCmd::SetPartControllers { part, sustain, pitch_bend, modulation } => {
                c.set_part((part & 3) as usize, PartTargets { sustain, pitch_bend, modulation });
            }
            ControllersCmd::SetBendRange { part, semitones } => c.set_bend_range((part & 3) as usize, semitones),
            ControllersCmd::TriggerFunction { .. } => return Ok(false),
        }
        Ok(true)
    }
}

/// What running assignable function `f` means (a pedal press, `TriggerFunction`), given the
/// fingering type and the style's One Touch Settings (`ots_count`, and `ots_applied`
/// 1-based, 0 = none yet):
///
/// - `Ok(FunctionRun::Cmd(c))`: run command `c`;
/// - `Ok(FunctionRun::Switch(bit))`: switch a pedal switch (`controllers::SUSTAIN`, ...) on or off;
/// - `Ok(FunctionRun::Nothing)`: No Assign;
/// - `Err(text)`: it can't run (not in yahaha yet, needs a foot controller, no OTS).
///
/// The session and the app's Rust mock share it.
pub fn function_run(f: Function, fingering: crate::fingering::Fingering, ots_count: u8, ots_applied: u8) -> Result<FunctionRun, String> {
    use crate::controllers::Effect;
    use crate::fingering::Fingering as Fg;
    use crate::parts;
    let info = f.info();
    if !info.available {
        return Err(format!("{} is not in yahaha yet", info.name));
    }
    let cmd: super::AppCmd = match f.effect() {
        Effect::Nothing => return Ok(FunctionRun::Nothing),
        Effect::Engine(b) => b.into(),
        Effect::Switch(b) => return Ok(FunctionRun::Switch(b)),
        Effect::Modulation | Effect::PitchBend | Effect::Dynamics => return Err(format!("{} needs a foot controller (an expression pedal)", info.name)),
        // A press (a Toggle pedal, `TriggerFunction`) switches it; Hold pedals set it
        // (`function_set`).
        Effect::ControlSwitch => match f {
            Function::KbdHarmonyArp => super::HarmonyArpCmd::ToggleHarmonyArp.into(),
            Function::ArpHold => super::HarmonyArpCmd::ToggleArpPedalHold.into(),
            _ => return Err(format!("{} can't be run here", info.name)),
        },
        Effect::Control => match f {
            Function::OtsLink => super::OtsCmd::ToggleOtsLink.into(),
            Function::Ots1 | Function::Ots2 | Function::Ots3 | Function::Ots4 => super::OtsCmd::RecallOts { index: f as u8 - Function::Ots1 as u8 }.into(),
            Function::OtsNext | Function::OtsPrev => {
                let n = ots_count.min(4);
                if n == 0 {
                    return Err("this style has no One Touch Settings".into());
                }
                // Wraps round, both ways; from none, + is OTS 1 and − the last.
                let index = match (f, ots_applied.min(n)) {
                    (Function::OtsNext, a) => a % n,
                    (_, 0) => n - 1,
                    (_, a) => (a + n - 2) % n,
                };
                super::OtsCmd::RecallOts { index }.into()
            }
            // RM p.144: "Same as the TRANSPOSE [+]/[−] buttons", which transpose the overall
            // pitch (OM p.61): Master transpose.
            // The REGIST BANK [+]/[−] buttons (RM p.144).
            Function::RegistBankNext => super::RegistrationCmd::StepRegistBank { delta: 1 }.into(),
            Function::RegistBankPrev => super::RegistrationCmd::StepRegistBank { delta: -1 }.into(),
            Function::TransposeUp => super::ChordCmd::StepTranspose { keyboard: 0, master: 1 }.into(),
            Function::TransposeDown => super::ChordCmd::StepTranspose { keyboard: 0, master: -1 }.into(),
            Function::Right1OnOff => super::PartsCmd::TogglePart { part: parts::RIGHT1 as u8 }.into(),
            Function::Right2OnOff => super::PartsCmd::TogglePart { part: parts::RIGHT2 as u8 }.into(),
            Function::Right3OnOff => super::PartsCmd::TogglePart { part: parts::RIGHT3 as u8 }.into(),
            Function::LeftOnOff => super::PartsCmd::TogglePart { part: parts::LEFT as u8 }.into(),
            Function::FingeredOnBass => {
                let fingering = if fingering == Fg::FingeredOnBass { Fg::Fingered } else { Fg::FingeredOnBass };
                super::ChordCmd::SetFingering { fingering }.into()
            }
            _ => return Err(format!("{} can't be run here", info.name)),
        },
    };
    Ok(FunctionRun::Cmd(cmd))
}

/// The command that sets control-side switch `f` (`Effect::ControlSwitch`) on or off, as a
/// Hold A or Hold B pedal does (`controllers::Fire::set`, `controllers::control_switch_sets`).
/// None for a function that isn't one.
pub fn function_set(f: Function, on: bool) -> Option<super::AppCmd> {
    match f {
        Function::KbdHarmonyArp => Some(super::HarmonyArpCmd::SetHarmonyArpOn { on }.into()),
        Function::ArpHold => Some(super::HarmonyArpCmd::SetArpPedalHold { on }.into()),
        _ => None,
    }
}

/// What running an assignable function comes to (`function_run`).
#[derive(Clone, Debug, PartialEq)]
pub enum FunctionRun {
    Nothing,
    Cmd(super::AppCmd),
    /// A pedal switch bit to flip.
    Switch(u8),
}

impl ControllersState {
    /// The state of `c` now.
    pub fn of(c: &Controllers) -> ControllersState {
        let down = c.down();
        let sw = c.switches();
        ControllersState {
            pedals: (0..PEDALS)
                .map(|i| {
                    let p = c.pedal(i);
                    PedalState { cc: p.cc, function: p.function, control_type: p.control_type, reverse: p.reverse, range: p.range, down: down >> i & 1 != 0 }
                })
                .collect(),
            learning: c.learning().map(|i| i as u8),
            parts: (0..crate::parts::COUNT)
                .map(|p| {
                    let t = c.part_targets(p);
                    PartControllers { sustain: t.sustain, pitch_bend: t.pitch_bend, modulation: t.modulation, bend_range: c.bend_range(p) }
                })
                .collect(),
            sustain: sw & SUSTAIN != 0,
            sostenuto: sw & SOSTENUTO != 0,
            soft: sw & SOFT != 0,
        }
    }
}

/// Controllers.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllersState {
    /// The pedals (always 3).
    pub pedals: Vec<PedalState>,
    /// The pedal learning its CC (0-2), if any.
    pub learning: Option<u8>,
    /// Right 1, Right 2, Right 3, Left (always 4): what reaches each.
    pub parts: Vec<PartControllers>,
    /// The pedal switches in effect now.
    pub sustain: bool,
    pub sostenuto: bool,
    pub soft: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PedalState {
    /// The CC it listens for, or null.
    pub cc: Option<u8>,
    pub function: Function,
    pub control_type: ControlType,
    pub reverse: bool,
    pub range: Range,
    /// Held down now.
    pub down: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartControllers {
    /// The pedal switches (sustain, sostenuto, soft) reach it.
    pub sustain: bool,
    pub pitch_bend: bool,
    pub modulation: bool,
    /// Pitch Bend Range in semitones, 0-12.
    pub bend_range: u8,
}

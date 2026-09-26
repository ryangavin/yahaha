//! Knob Assign pages for the Launchkey's 8 encoders (#197): the Genos LIVE CONTROL knobs
//! (OM p.62-63, RM p.145-148). A page gives each knob a function; KNOB ASSIGN (the
//! encoder page buttons) steps through the pages. The knobs are relative, as the Genos's
//! are: a turn moves the value from where it is now (OM p.63), whoever set it last.
//!
//! This is the pure model: the session hands it the values in effect (`Now`) and runs the
//! command a turn gives back, the same command the app's control for it sends.

use crate::api::{AppCmd, DynamicsCmd, KnobState, KnobsState, HarmonyArpCmd, PartSend, MetronomeCmd, MixerCmd, PartsCmd, StyleSettingsCmd, TrackMuteOrder, TransportCmd};
use crate::engine::RETRIGGER_RATES;
use serde::{Deserialize, Serialize};

/// A Knob Assign page.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum KnobPage {
    /// The Style's live functions: Dynamics, Retrigger, Track Mute A/B, tempo.
    #[default]
    Style,
    /// Levels: the keyboard parts' volumes, Harmony, the metronome, tempo.
    Parts,
    /// The keyboard parts' pan, tempo.
    Pan,
    /// The keyboard parts' Reverb and Chorus sends (Genos Mixer > Effect).
    Effects,
}

impl KnobPage {
    pub const ALL: [KnobPage; 4] = [KnobPage::Style, KnobPage::Parts, KnobPage::Pan, KnobPage::Effects];

    pub fn name(self) -> &'static str {
        match self {
            KnobPage::Style => "Style",
            KnobPage::Parts => "Parts",
            KnobPage::Pan => "Pan",
            KnobPage::Effects => "Effects",
        }
    }

    pub fn index(self) -> usize {
        self as usize
    }

    /// The page `d` steps away, stopping at the first and last.
    pub fn step(self, d: i8) -> KnobPage {
        KnobPage::ALL[(self as i16 + d as i16).clamp(0, KnobPage::ALL.len() as i16 - 1) as usize]
    }

    /// Knobs 1-8. Tempo is knob 8 on every page but Effects, whose eight knobs are the
    /// four parts' two sends.
    pub fn functions(self) -> [KnobFn; 8] {
        use KnobFn::*;
        match self {
            KnobPage::Style => [Dynamics, RetriggerRate, RetriggerOnOff, TrackMuteA, TrackMuteB, None, None, Tempo],
            KnobPage::Parts => [PartVolume(0), PartVolume(1), PartVolume(2), PartVolume(3), HarmonyVolume, MetronomeVolume, None, Tempo],
            KnobPage::Pan => [PartPan(0), PartPan(1), PartPan(2), PartPan(3), None, None, None, Tempo],
            KnobPage::Effects => [PartReverb(0), PartReverb(1), PartReverb(2), PartReverb(3), PartChorus(0), PartChorus(1), PartChorus(2), PartChorus(3)],
        }
    }
}

/// What a knob does (RM p.146-148 names in the comments).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KnobFn {
    /// No Assign (`---`).
    None,
    /// DynCtrl: the Style Dynamics level, 0-127.
    Dynamics,
    /// RtgRate: the Retrigger length; turning right makes it shorter.
    RetriggerRate,
    /// RtgOnOff: right turns Retrigger on, left off.
    RetriggerOnOff,
    /// StyMuteA / StyMuteB: fully left one Style part plays; turning right adds the others.
    TrackMuteA,
    TrackMuteB,
    /// Tempo (Master Tempo), 1 BPM a step.
    Tempo,
    /// Mixer Volume of a keyboard part (0-3: Right 1-3, Left): its CC7.
    PartVolume(u8),
    /// HarmVol: the Keyboard Harmony volume.
    HarmonyVolume,
    /// The metronome's volume.
    MetronomeVolume,
    /// Mixer Pan of a keyboard part (0-3): its CC10, 64 = centre.
    PartPan(u8),
    /// Mixer Reverb / Chorus depth of a keyboard part (0-3): its CC91 / CC93.
    PartReverb(u8),
    PartChorus(u8),
}

/// Knob steps per Retrigger length, and per Retrigger on/off switch.
const RTG_STEPS: i16 = 3;
/// A Track Mute knob's position moves this much per step (0-127: about four steps per part).
const MUTE_STEP: i16 = 4;
/// Levels (0-127) move this much per step.
const LEVEL_STEP: i16 = 2;
/// The tempo range a knob turns through (the engine's).
const MIN_BPM: i32 = 5;
const MAX_BPM: i32 = 500;

impl KnobFn {
    /// The wire name of the function.
    pub fn id(self) -> &'static str {
        match self {
            KnobFn::None => "none",
            KnobFn::Dynamics => "dynamics",
            KnobFn::RetriggerRate => "retriggerRate",
            KnobFn::RetriggerOnOff => "retriggerOnOff",
            KnobFn::TrackMuteA => "trackMuteA",
            KnobFn::TrackMuteB => "trackMuteB",
            KnobFn::Tempo => "tempo",
            KnobFn::PartVolume(_) => "partVolume",
            KnobFn::HarmonyVolume => "harmonyVolume",
            KnobFn::MetronomeVolume => "metronomeVolume",
            KnobFn::PartPan(_) => "partPan",
            KnobFn::PartReverb(_) => "partReverb",
            KnobFn::PartChorus(_) => "partChorus",
        }
    }

    /// A short name, up to 8 characters (the Launchkey display's eight-name page).
    pub fn short(self) -> &'static str {
        match self {
            KnobFn::None => "---",
            KnobFn::Dynamics => "DynCtrl",
            KnobFn::RetriggerRate => "RtgRate",
            KnobFn::RetriggerOnOff => "RtgOnOff",
            KnobFn::TrackMuteA => "StyMuteA",
            KnobFn::TrackMuteB => "StyMuteB",
            KnobFn::Tempo => "Tempo",
            KnobFn::PartVolume(p) => ["Right1", "Right2", "Right3", "Left"][(p & 3) as usize],
            KnobFn::HarmonyVolume => "HarmVol",
            KnobFn::MetronomeVolume => "MetroVol",
            KnobFn::PartPan(p) => ["PanR1", "PanR2", "PanR3", "PanL"][(p & 3) as usize],
            KnobFn::PartReverb(p) => ["RevR1", "RevR2", "RevR3", "RevL"][(p & 3) as usize],
            KnobFn::PartChorus(p) => ["ChoR1", "ChoR2", "ChoR3", "ChoL"][(p & 3) as usize],
        }
    }

    /// The full name.
    pub fn name(self) -> &'static str {
        match self {
            KnobFn::None => "No Assign",
            KnobFn::Dynamics => "Dynamics Control",
            KnobFn::RetriggerRate => "Retrigger Rate",
            KnobFn::RetriggerOnOff => "Retrigger On/Off",
            KnobFn::TrackMuteA => "Style Track Mute A",
            KnobFn::TrackMuteB => "Style Track Mute B",
            KnobFn::Tempo => "Tempo",
            KnobFn::PartVolume(p) => ["Right 1 Volume", "Right 2 Volume", "Right 3 Volume", "Left Volume"][(p & 3) as usize],
            KnobFn::HarmonyVolume => "Harmony Volume",
            KnobFn::MetronomeVolume => "Metronome Volume",
            KnobFn::PartPan(p) => ["Right 1 Pan", "Right 2 Pan", "Right 3 Pan", "Left Pan"][(p & 3) as usize],
            KnobFn::PartReverb(p) => ["Right 1 Reverb", "Right 2 Reverb", "Right 3 Reverb", "Left Reverb"][(p & 3) as usize],
            KnobFn::PartChorus(p) => ["Right 1 Chorus", "Right 2 Chorus", "Right 3 Chorus", "Left Chorus"][(p & 3) as usize],
        }
    }
}

/// The values in effect, as the control side knows them.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Now {
    pub dynamics: u8,
    pub retrigger: bool,
    pub retrigger_rate: u8,
    pub bpm: f64,
    pub part_volume: [u8; 4],
    pub harmony_volume: u8,
    pub metronome_volume: u8,
    /// Each keyboard part's pan, reverb and chorus (`parts::Parts::fx`).
    pub part_fx: [[u8; 3]; 4],
}

/// A knob as it reads now: its value as text, and where it is (0-127) if it has a
/// position (the Genos LED ring; Tempo has none).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Reading {
    pub value: String,
    pub level: Option<u8>,
}

/// The knobs: the page, and what a turn carries over.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Knobs {
    pub page: KnobPage,
    /// Steps turned toward the next switch of a stepped function (Retrigger), per knob.
    acc: [i16; 8],
    /// The Track Mute A and B knob positions (0-127). They set the Style parts' switches
    /// and keep no value of their own, so the knob is where it was last turned to.
    mute: [u8; 2],
}

impl Default for Knobs {
    fn default() -> Knobs {
        Knobs { page: KnobPage::default(), acc: [0; 8], mute: [127; 2] }
    }
}

impl Knobs {
    pub fn set_page(&mut self, page: KnobPage) {
        if page != self.page {
            self.page = page;
            self.acc = [0; 8];
        }
    }

    /// The function of knob `knob` (0-7) on the page.
    pub fn function(&self, knob: u8) -> KnobFn {
        self.page.functions().get(knob as usize).copied().unwrap_or(KnobFn::None)
    }

    /// Knob `knob` (0-7) turned `delta` steps (positive: clockwise): the command that
    /// makes the change, or None when nothing changes.
    pub fn turn(&mut self, knob: u8, delta: i8, now: &Now) -> Option<AppCmd> {
        let k = knob as usize;
        let f = self.function(knob);
        let d = delta as i16;
        let level = |v: u8| (v as i16 + d * LEVEL_STEP).clamp(0, 127) as u8;
        let cmd: AppCmd = match f {
            KnobFn::None => return None,
            KnobFn::Dynamics => DynamicsCmd::SetDynamics { level: level(now.dynamics) }.into(),
            KnobFn::RetriggerRate => {
                let steps = self.stepped(k, d)?;
                StyleSettingsCmd::StepRetriggerRate { delta: steps }.into()
            }
            KnobFn::RetriggerOnOff => {
                let steps = self.stepped(k, d)?;
                if (steps > 0) == now.retrigger {
                    return None;
                }
                TransportCmd::ToggleRetrigger.into()
            }
            KnobFn::TrackMuteA | KnobFn::TrackMuteB => {
                let (i, order) = if f == KnobFn::TrackMuteA { (0, TrackMuteOrder::A) } else { (1, TrackMuteOrder::B) };
                let v = (self.mute[i] as i16 + d * MUTE_STEP).clamp(0, 127) as u8;
                if v == self.mute[i] {
                    return None;
                }
                self.mute[i] = v;
                MixerCmd::StyleTrackMute { order, value: v }.into()
            }
            KnobFn::Tempo => {
                let bpm = (now.bpm.round() as i32 + d as i32).clamp(MIN_BPM, MAX_BPM);
                TransportCmd::SetTempo { bpm: bpm as u16 }.into()
            }
            KnobFn::PartVolume(p) => PartsCmd::SetPartVolume { part: p, volume: level(now.part_volume[(p & 3) as usize]) }.into(),
            KnobFn::HarmonyVolume => HarmonyArpCmd::SetHarmonyVolume { volume: level(now.harmony_volume) }.into(),
            KnobFn::MetronomeVolume => MetronomeCmd::SetMetronomeVolume { volume: level(now.metronome_volume) }.into(),
            KnobFn::PartPan(p) => PartsCmd::SetPartPan { part: p, pan: level(now.part_fx[(p & 3) as usize][0]) }.into(),
            KnobFn::PartReverb(p) => {
                PartsCmd::SetPartSend { part: p, send: PartSend::Reverb, value: level(now.part_fx[(p & 3) as usize][1]) }.into()
            }
            KnobFn::PartChorus(p) => {
                PartsCmd::SetPartSend { part: p, send: PartSend::Chorus, value: level(now.part_fx[(p & 3) as usize][2]) }.into()
            }
        };
        Some(cmd)
    }

    /// A stepped function's knob turned `d`: the whole switches (±1) it has turned
    /// through, if any. Turning back starts the count again.
    fn stepped(&mut self, k: usize, d: i16) -> Option<i8> {
        let a = &mut self.acc[k];
        if (*a > 0 && d < 0) || (*a < 0 && d > 0) {
            *a = 0;
        }
        *a += d;
        let n = *a / RTG_STEPS;
        *a -= n * RTG_STEPS;
        (n != 0).then_some(n.clamp(-6, 6) as i8)
    }

    /// The page and its knobs as the state shows them.
    pub fn state(&self, now: &Now) -> KnobsState {
        let knobs = (0..8u8)
            .map(|k| {
                let (f, r) = (self.function(k), self.reading(k, now));
                KnobState { function: f.id().into(), name: f.name().into(), short: f.short().into(), value: r.value, level: r.level }
            })
            .collect();
        let page = self.page;
        KnobsState { page, page_name: page.name().into(), page_number: page.index() as u8 + 1, page_count: KnobPage::ALL.len() as u8, knobs }
    }

    /// Knob `knob` as it reads now.
    pub fn reading(&self, knob: u8, now: &Now) -> Reading {
        let r = |value: String, level: Option<u8>| Reading { value, level };
        match self.function(knob) {
            KnobFn::None => r(String::new(), None),
            KnobFn::Dynamics => r(now.dynamics.to_string(), Some(now.dynamics)),
            KnobFn::RetriggerRate => {
                let i = RETRIGGER_RATES.iter().position(|&x| x == now.retrigger_rate).unwrap_or(0);
                r(format!("1/{}", now.retrigger_rate), Some((i * 127 / (RETRIGGER_RATES.len() - 1)) as u8))
            }
            KnobFn::RetriggerOnOff => r(if now.retrigger { "On" } else { "Off" }.into(), Some(if now.retrigger { 127 } else { 0 })),
            f @ (KnobFn::TrackMuteA | KnobFn::TrackMuteB) => {
                let v = self.mute[(f == KnobFn::TrackMuteB) as usize];
                let n = TrackMuteOrder::A.mask(v).count_ones();
                r(if n == 8 { "All".into() } else { format!("{n} of 8") }, Some(v))
            }
            KnobFn::Tempo => r(format!("{} BPM", now.bpm.round() as i32), None),
            KnobFn::PartVolume(p) => {
                let v = now.part_volume[(p & 3) as usize];
                r(v.to_string(), Some(v))
            }
            KnobFn::HarmonyVolume => r(now.harmony_volume.to_string(), Some(now.harmony_volume)),
            KnobFn::MetronomeVolume => r(now.metronome_volume.to_string(), Some(now.metronome_volume)),
            KnobFn::PartPan(p) => {
                let v = now.part_fx[(p & 3) as usize][0];
                r(pan_text(v), Some(v))
            }
            KnobFn::PartReverb(p) | KnobFn::PartChorus(p) => {
                let v = now.part_fx[(p & 3) as usize][if matches!(self.function(knob), KnobFn::PartReverb(_)) { 1 } else { 2 }];
                r(v.to_string(), Some(v))
            }
        }
    }
}

/// A pan as the Genos shows it: L63 … C … R63.
pub fn pan_text(v: u8) -> String {
    match v.min(127) as i16 - 64 {
        0 => "C".into(),
        d if d < 0 => format!("L{}", -d),
        d => format!("R{d}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> Now {
        Now {
            dynamics: 64,
            retrigger: false,
            retrigger_rate: 8,
            bpm: 120.0,
            part_volume: [100, 90, 80, 70],
            harmony_volume: 100,
            metronome_volume: 64,
            part_fx: [[64, 40, 0], [30, 50, 10], [100, 0, 0], [64, 127, 5]],
        }
    }

    #[test]
    fn pages_step_and_stop_at_the_ends() {
        assert_eq!(KnobPage::Style.step(-1), KnobPage::Style);
        assert_eq!(KnobPage::Style.step(1), KnobPage::Parts);
        assert_eq!(KnobPage::Effects.step(1), KnobPage::Effects);
        for p in KnobPage::ALL {
            assert!(p == KnobPage::Effects || p.functions()[7] == KnobFn::Tempo, "tempo is knob 8 on {p:?}");
            assert!(p.functions().iter().all(|f| f.short().len() <= 8));
        }
    }

    /// The knobs move the value from where it is now (OM p.63).
    #[test]
    fn levels_move_from_the_value_in_effect() {
        let mut k = Knobs::default();
        let n = Now { dynamics: 100, ..now() };
        assert_eq!(k.turn(0, 3, &n), Some(DynamicsCmd::SetDynamics { level: 106 }.into()));
        assert_eq!(k.turn(0, -64, &n), Some(DynamicsCmd::SetDynamics { level: 0 }.into()));
        assert_eq!(k.turn(7, -2, &now()), Some(TransportCmd::SetTempo { bpm: 118 }.into()));
        assert_eq!(k.turn(7, 1, &Now { bpm: 500.0, ..now() }), Some(TransportCmd::SetTempo { bpm: 500 }.into()));
        assert_eq!(k.turn(5, 1, &now()), None, "knob 6 is unassigned on the Style page");
        k.set_page(KnobPage::Parts);
        assert_eq!(k.turn(1, -1, &now()), Some(PartsCmd::SetPartVolume { part: 1, volume: 88 }.into()));
        assert_eq!(k.turn(4, 1, &now()), Some(HarmonyArpCmd::SetHarmonyVolume { volume: 102 }.into()));
        assert_eq!(k.turn(5, 1, &now()), Some(MetronomeCmd::SetMetronomeVolume { volume: 66 }.into()));
    }

    /// Retrigger steps once per few knob steps; turning back starts the count again.
    #[test]
    fn retrigger_knobs_step() {
        let mut k = Knobs::default();
        assert_eq!(k.turn(1, 1, &now()), None);
        assert_eq!(k.turn(1, 1, &now()), None);
        assert_eq!(k.turn(1, 1, &now()), Some(StyleSettingsCmd::StepRetriggerRate { delta: 1 }.into()), "right: shorter");
        assert_eq!(k.turn(1, 2, &now()), None);
        assert_eq!(k.turn(1, -1, &now()), None, "turning back starts again");
        assert_eq!(k.turn(1, -2, &now()), Some(StyleSettingsCmd::StepRetriggerRate { delta: -1 }.into()));
        assert_eq!(k.turn(1, 7, &now()), Some(StyleSettingsCmd::StepRetriggerRate { delta: 2 }.into()));
        // On/Off: right turns it on, and further right leaves it on.
        assert_eq!(k.turn(2, 3, &now()), Some(TransportCmd::ToggleRetrigger.into()));
        assert_eq!(k.turn(2, 3, &Now { retrigger: true, ..now() }), None);
        assert_eq!(k.turn(2, -3, &Now { retrigger: true, ..now() }), Some(TransportCmd::ToggleRetrigger.into()));
    }

    /// Track Mute A/B start fully right (every part on) and turn parts off going left.
    #[test]
    fn track_mute_knobs() {
        let mut k = Knobs::default();
        assert_eq!(k.reading(3, &now()).value, "All");
        assert_eq!(k.turn(3, 1, &now()), None, "already fully right");
        assert_eq!(k.turn(3, -4, &now()), Some(MixerCmd::StyleTrackMute { order: TrackMuteOrder::A, value: 111 }.into()));
        assert_eq!(k.turn(4, -40, &now()), Some(MixerCmd::StyleTrackMute { order: TrackMuteOrder::B, value: 0 }.into()));
        assert_eq!(k.reading(4, &now()), Reading { value: "1 of 8".into(), level: Some(0) });
        assert_eq!(k.reading(3, &now()).level, Some(111));
    }

    /// Pan and the effect sends, on the Pan and Effects pages (#198's per-part controls).
    #[test]
    fn pan_and_effect_knobs() {
        let mut k = Knobs::default();
        k.set_page(KnobPage::Pan);
        assert_eq!(k.turn(1, -3, &now()), Some(PartsCmd::SetPartPan { part: 1, pan: 24 }.into()));
        assert_eq!(k.reading(0, &now()), Reading { value: "C".into(), level: Some(64) });
        assert_eq!(k.reading(1, &now()).value, "L34");
        assert_eq!(k.reading(2, &now()).value, "R36");
        assert_eq!(k.function(7), KnobFn::Tempo);
        k.set_page(KnobPage::Effects);
        assert_eq!(k.turn(3, 1, &now()), Some(PartsCmd::SetPartSend { part: 3, send: PartSend::Reverb, value: 127 }.into()));
        assert_eq!(k.turn(5, 2, &now()), Some(PartsCmd::SetPartSend { part: 1, send: PartSend::Chorus, value: 14 }.into()));
        assert_eq!(k.reading(0, &now()), Reading { value: "40".into(), level: Some(40) });
        assert_eq!(k.reading(7, &now()), Reading { value: "5".into(), level: Some(5) });
        assert_eq!(pan_text(0), "L64");
        assert_eq!(pan_text(127), "R63");
    }

    #[test]
    fn readings() {
        let k = Knobs::default();
        assert_eq!(k.reading(0, &now()), Reading { value: "64".into(), level: Some(64) });
        assert_eq!(k.reading(1, &now()), Reading { value: "1/8".into(), level: Some(76) });
        assert_eq!(k.reading(2, &now()).value, "Off");
        assert_eq!(k.reading(5, &now()), Reading { value: String::new(), level: None });
        assert_eq!(k.reading(7, &Now { bpm: 97.6, ..now() }).value, "98 BPM");
    }
}

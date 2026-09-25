//! The Launchkey display (#213): whatever control the player touches or moves (a pad, a
//! button, a fader, a fader button, a knob), the display names what it did and its value,
//! then the Launchkey goes back to its normal screen after its own display timeout.
//!
//! The input thread only records which control it was (`Shared::touched`). The words come
//! from the state the app shows, so every control has them with no table of its own: the
//! pads' labels (`pads`), the buttons' and faders' (`surface`: the one label table for the
//! controls beyond the pads, which features that add a function to a button or fader fill
//! in), and the knobs' (`knobs`). The value is read from the state after the control acted.

use super::Control;
use crate::api::{
    AppCmd, AppState, ChordCmd, HarmonyArpCmd, LibraryCmd, MixerCmd, MultiPadCmd, OtsCmd, PadLamp, PadsCmd, PartsCmd,
    PlaylistCmd, RegistrationCmd, StyleSettingsCmd, TransportCmd,
};
use crate::launchkey::{self, Level, Touch};
use crate::parts::FaderPage;
use std::sync::atomic::Ordering::Relaxed;

/// How long after a touch the display follows the value (an engine command shows in the
/// state a moment later; a fader keeps moving).
const FOLLOW_NS: u64 = 600_000_000;

/// What the display shows: title, name, value.
pub(super) type Text = (String, String, String);

/// The control side's display state (a `Control` field).
#[derive(Default)]
pub(super) struct Display {
    /// `Shared::touched` as last read.
    seen: u32,
    /// The control being followed, and until when.
    touch: Option<(Touch, u64)>,
    /// What was last sent.
    pub(super) shown: Option<Text>,
}

impl Control {
    /// After each state build: a new touch starts following that control; while it is
    /// followed, a changed text goes to the display.
    pub(super) fn pump_display(&mut self, st: &AppState, now: u64) {
        let v = self.shared.touched.load(Relaxed);
        if v != self.display.seen {
            self.display.seen = v;
            if let Some(t) = Touch::unpack(v) {
                self.display.touch = Some((t, now + FOLLOW_NS));
                self.display.shown = None;
            }
        }
        let Some((t, until)) = self.display.touch else { return };
        if now > until {
            self.display.touch = None;
            return;
        }
        let Some(text) = display_text(t, st) else { return };
        if self.display.shown.as_ref() == Some(&text) {
            return;
        }
        if let Some(leds) = self.leds.as_mut() {
            for m in launchkey::display_msgs(&text.0, &text.1, &text.2) {
                leds.out.push(&m);
            }
            leds.out.flush();
        }
        self.display.shown = Some(text);
    }
}

/// What the display says for control `t`, from the state `st` (None: a control that does
/// nothing now).
pub(super) fn display_text(t: Touch, st: &AppState) -> Option<Text> {
    let s = |x: &str| x.to_string();
    match t {
        Touch::Pad(note) => {
            let pad = st.pads.pads.iter().find(|p| p.note == note)?;
            let value = value_of(pad.action.as_ref()?, pad.level, st);
            Some((format!("Pads: {}", st.pads.page_name), pad.label.clone(), value))
        }
        Touch::Knob(k) => {
            let knob = st.knobs.knobs.get(k as usize)?;
            let value = if knob.function == "none" { s("-") } else { knob.value.clone() };
            Some((format!("Knobs: {}", st.knobs.page_name), knob.name.clone(), value))
        }
        Touch::Fader(i) => {
            let f = st.surface.faders.get(i as usize)?;
            let title = match (i, st.mixer.fader_page) {
                (8, _) => "Master",
                (_, FaderPage::Panel) => "Faders: Panel",
                (_, FaderPage::Style) => "Faders: Style",
            };
            let (name, value) = match f.value {
                None => (format!("Fader {}", i + 1), s("-")),
                // Soft takeover still catching the fader: the level, and where the fader is.
                Some(v) if f.waiting => (f.label.clone(), f.position.map_or(v.to_string(), |p| format!("{v} > {p}"))),
                Some(v) => (f.label.clone(), v.to_string()),
            };
            Some((s(title), name, value))
        }
        Touch::Button { cc, shift } => {
            // The encoder page buttons are the knobs' KNOB ASSIGN.
            if cc == launchkey::KNOB_UP_CC || cc == launchkey::KNOB_DOWN_CC {
                return Some((s("Knobs"), s("KNOB ASSIGN"), st.knobs.page_name.clone()));
            }
            let c = st.surface.controls.iter().find(|c| c.cc == cc)?;
            button_text("Buttons", c, shift, st)
        }
        Touch::FaderButton { index, shift } => {
            let cc = launchkey::FADER_BTN_CC.start() + index;
            let c = st.surface.controls.iter().find(|c| c.cc == cc)?;
            let title = if index == 8 { "Fader page" } else { "Fader buttons" };
            button_text(title, c, shift, st)
        }
    }
}

fn button_text(title: &str, c: &crate::api::SurfaceControl, shift: bool, st: &AppState) -> Option<Text> {
    let (label, action) = if shift { (&c.shift_label, &c.shift_action) } else { (&c.label, &c.action) };
    Some((title.to_string(), label.clone(), value_of(action.as_ref()?, c.level, st)))
}

/// The value a command leaves, in a few words. `level` is its pad or button light after
/// the press, for switches with nothing better to say.
fn value_of(cmd: &AppCmd, level: Level, st: &AppState) -> String {
    let on = |b: bool| if b { "On" } else { "Off" }.to_string();
    let t = &st.transport;
    let section = || {
        t.queued
            .clone()
            .or_else(|| t.section.clone())
            .or_else(|| t.pending_intro.map(|i| format!("Intro {} armed", i + 1)))
            .unwrap_or_else(|| "Stopped".into())
    };
    let bpm = || format!("{} BPM", t.tempo.round() as i32);
    let selected = || st.keyboard_parts.iter().find(|p| p.selected);
    match cmd {
        AppCmd::Transport(c) => match c {
            TransportCmd::StartStop | TransportCmd::Stop | TransportCmd::SectionReset => {
                if t.running { section() } else { "Stopped".into() }
            }
            TransportCmd::TapTempo | TransportCmd::TempoUp | TransportCmd::TempoDown | TransportCmd::SetTempo { .. } => bpm(),
            TransportCmd::ToggleSyncStart => on(t.sync_start),
            TransportCmd::ToggleSyncStop => on(t.sync_stop),
            TransportCmd::ToggleAutoFill => on(t.auto_fill),
            TransportCmd::ToggleStopAcmp | TransportCmd::SetStopAcmp { .. } => on(t.stop_acmp),
            TransportCmd::ToggleRetrigger => on(t.retrigger),
            TransportCmd::ToggleHalfBarFill | TransportCmd::SetHalfBarFill { .. } => on(t.half_bar_fill),
            TransportCmd::ToggleFade => format!("{:?}", t.fade),
            _ => section(),
        },
        AppCmd::StyleSettings(StyleSettingsCmd::StepRetriggerRate { .. } | StyleSettingsCmd::SetRetriggerRate { .. }) => {
            format!("1/{}", st.style_settings.retrigger_rate)
        }
        AppCmd::Parts(PartsCmd::TogglePart { part } | PartsCmd::SetPartOn { part, .. }) => {
            st.keyboard_parts.get(*part as usize).map_or_else(String::new, |p| on(p.on))
        }
        AppCmd::Parts(PartsCmd::SelectPart { .. } | PartsCmd::StepVoice { .. }) => selected().map_or_else(String::new, |p| p.voice_name.clone()),
        AppCmd::Mixer(MixerCmd::ToggleStylePart { part }) => st.mixer.style_parts.get(*part as usize).map_or_else(String::new, |p| on(p.on)),
        AppCmd::Mixer(MixerCmd::ToggleFaderPage | MixerCmd::SetFaderPage { .. }) => format!("{:?}", st.mixer.fader_page),
        AppCmd::Pads(PadsCmd::SetPadPage { .. } | PadsCmd::CyclePadPage { .. }) => st.pads.page_name.clone(),
        AppCmd::Library(LibraryCmd::StepStyle { .. }) => st.style.name.clone(),
        AppCmd::Playlist(PlaylistCmd::StepPlaylist { .. }) => st.playlist.current.and_then(|i| st.playlist.records.get(i)).map_or_else(|| st.style.name.clone(), |r| r.record.name.clone()),
        AppCmd::Chord(c) => match c {
            ChordCmd::SetFingering { .. } | ChordCmd::NextFingering => st.chord.fingering_name.clone(),
            ChordCmd::ToggleUpper | ChordCmd::SetUpper { .. } => if st.chord.upper { "Upper" } else { "Lower" }.into(),
            ChordCmd::ToggleManualBass | ChordCmd::SetManualBass { .. } => on(st.chord.manual_bass),
            ChordCmd::MoveSplit { .. } => st.chord.split_name.clone(),
            ChordCmd::StepTranspose { .. } | ChordCmd::ResetTranspose => {
                format!("Kbd {:+} Mst {:+}", st.chord.transpose_keyboard, st.chord.transpose_master)
            }
            _ => level_text(level),
        },
        AppCmd::Ots(OtsCmd::RecallOts { index }) => format!("OTS {}", index + 1),
        AppCmd::Ots(OtsCmd::ToggleOtsLink) => on(st.ots.link),
        AppCmd::Registration(c) => match c {
            RegistrationCmd::PressRegist { index } if st.registration.memory => format!("Memorize {}", index + 1),
            RegistrationCmd::PressRegist { .. } | RegistrationCmd::StepRegistSequence { .. } => {
                st.registration.selected.map_or_else(|| "-".into(), |i| format!("Regist {}", i + 1))
            }
            RegistrationCmd::ToggleRegistMemory => on(st.registration.memory),
            RegistrationCmd::ToggleFreeze => on(st.registration.freeze),
            RegistrationCmd::StepRegistBank { .. } => st.registration.bank.name.clone(),
            _ => level_text(level),
        },
        AppCmd::HarmonyArp(HarmonyArpCmd::ToggleHarmonyArp) => {
            if st.harmony_arp.on { st.harmony_arp.type_name.clone() } else { "Off".into() }
        }
        AppCmd::MultiPad(MultiPadCmd::TriggerMultiPad { pad } | MultiPadCmd::StopMultiPad { pad } | MultiPadCmd::ArmMultiPad { pad }) => {
            st.multi_pad.pads.get(*pad as usize).map_or_else(String::new, |p| {
                match p.lamp {
                    PadLamp::Empty => "Empty",
                    PadLamp::Ready => "Stopped",
                    PadLamp::Armed => "Synchro Start",
                    PadLamp::Queued => "Next bar",
                    PadLamp::Playing => "Playing",
                }
                .into()
            })
        }
        _ => level_text(level),
    }
}

/// A switch's state from its light.
fn level_text(level: Level) -> String {
    match level {
        Level::Bright => "On",
        Level::Dim => "Off",
        Level::Off => "-",
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{Options, Port, Session};

    fn session() -> Option<Session> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
        p.exists().then(|| Session::offline(Options { paths: vec![p], ..Options::default() }).unwrap())
    }

    fn shown(s: &Session) -> Option<Text> {
        s.display_shown()
    }

    /// Every kind of control names what it did and its value (#213): a headless session
    /// fed Launchkey messages.
    #[test]
    fn touched_controls_show_what_they_did() {
        let Some(s) = session() else { return };
        let text = |a: &str, b: &str, c: &str| Some((a.to_string(), b.to_string(), c.to_string()));
        // A tempo button.
        s.midi_in(Port::Pads, &[0xB0, launchkey::SCENE_CC, 127]);
        let bpm = s.state().transport.tempo.round() as i32;
        assert_eq!(shown(&s), text("Buttons", "TEMPO +", &format!("{bpm} BPM")));
        // A section pad: Start/Stop, then a Main.
        s.midi_in(Port::Pads, &[0x90, 119, 100]);
        assert_eq!(shown(&s), text("Pads: Sections", "START", "Main A"));
        // A knob.
        s.midi_in(Port::Pads, &[0xBF, 21, 66]);
        assert_eq!(shown(&s), text("Knobs: Style", "Dynamics Control", "68"));
        // A Panel fader: Right 2's level.
        s.midi_in(Port::Pads, &[0xB0, 6, 90]);
        let v = s.state().keyboard_parts[1].volume;
        let t = shown(&s).unwrap();
        assert_eq!((t.0.as_str(), t.1.as_str()), ("Faders: Panel", "RIGHT 2"));
        assert!(t.2.starts_with(&v.to_string()), "{t:?}");
        // A fader button: Right 3 on.
        s.midi_in(Port::Pads, &[0xB0, 39, 127]);
        assert_eq!(shown(&s), text("Fader buttons", "RIGHT 3", "On"));
        // The encoder page button.
        s.midi_in(Port::Pads, &[0xB0, launchkey::KNOB_DOWN_CC, 127]);
        assert_eq!(shown(&s), text("Knobs", "KNOB ASSIGN", "Parts"));
        // A pad page button, then a Chord/Setup pad.
        s.midi_in(Port::Pads, &[0xB0, launchkey::PAD_DOWN_CC, 127]);
        assert_eq!(shown(&s), text("Buttons", "PAGE ▼", "Chord/Setup"));
        s.midi_in(Port::Pads, &[0x90, 103, 100]);
        assert_eq!(shown(&s), text("Pads: Chord/Setup", "UPPER", "Upper"));
    }

    #[test]
    fn a_fader_still_catching_shows_where_it_is() {
        let st = {
            let mut st = AppState::default();
            st.surface.faders = vec![crate::api::SurfaceFader { label: "LEFT".into(), value: Some(96), waiting: true, position: Some(80), set: None }];
            st
        };
        assert_eq!(display_text(Touch::Fader(0), &st), Some(("Faders: Panel".into(), "LEFT".into(), "96 > 80".into())));
    }
}

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
    AppCmd, AppState, ChordCmd, HarmonyArpCmd, LibraryCmd, LooperCmd, LooperMode, MixerCmd, MultiPadCmd, OtsCmd, PadLamp, PadsCmd, PartsCmd,
    QuickRackCmd, StyleSettingsCmd, TransportCmd,
};
use crate::launchkey::{self, Layer, Level, Touch};
use crate::parts::FaderPage;
use std::sync::atomic::Ordering::Relaxed;

/// How long after a touch the display follows the value (an engine command shows in the
/// state a moment later; a fader keeps moving).
const FOLLOW_NS: u64 = 600_000_000;

/// The Panel faders whose level is a scale in percent (100 = as written), not a CC7: the
/// Style volume (#199) and the Multi Pad volume (#196).
const PERCENT_FADERS: [usize; 2] = [crate::parts::STYLE_LEVEL, crate::parts::PAD_LEVEL];

/// The keyboard parts' short names in swap mode's display line ("R1: 23 Rhodes Soft").
pub(super) const SWAP_LABELS: [&str; 4] = ["R1", "R2", "R3", "L"];

/// What the display shows: title, name, value.
pub(super) type Text = (String, String, String);

/// The control side's display state (a `Control` field).
#[derive(Default)]
pub(super) struct Display {
    /// `Shared::touched` as last read.
    seen: u32,
    /// The control being followed, and until when.
    touch: Option<(Touch, u64)>,
    /// The followed knob was turned in swap mode on this keyboard part (0-3).
    swap: Option<u8>,
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
                self.display.swap = None;
                self.display.shown = None;
            }
        }
        let Some((t, until)) = self.display.touch else { return };
        if now > until {
            self.display.touch = None;
            return;
        }
        let text = match (t, self.display.swap, st.surface.layer) {
            // A knob in swap mode (the input thread sets the layer as it records the
            // touch, so a state may see the touch first).
            (Touch::Knob(k), None, Layer::Swap { part }) => {
                self.display.swap = Some(part);
                swap_text(part, k, st)
            }
            (Touch::Knob(k), Some(part), layer) => {
                // Released: the sound it landed on still shows as it arrives; a mix knob's
                // last value stays on the display rather than turn into the page's knob.
                if k != 0 && layer != (Layer::Swap { part }) {
                    self.display.touch = None;
                    return;
                }
                swap_text(part, k, st)
            }
            _ => display_text(t, st),
        };
        let Some(text) = text else { return };
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
            // A level that scales a group of parts (100 = as written) reads as a percentage.
            let pct = st.mixer.fader_page == FaderPage::Panel && PERCENT_FADERS.contains(&(i as usize));
            let unit = if pct { "%" } else { "" };
            let (name, value) = match f.value {
                None => (format!("Fader {}", i + 1), s("-")),
                // Soft takeover still catching the fader: the level, and where the fader is.
                Some(v) if f.waiting => (f.label.clone(), f.position.map_or(format!("{v}{unit}"), |p| format!("{v}{unit} > {p}{unit}"))),
                Some(v) => (f.label.clone(), format!("{v}{unit}")),
            };
            Some((s(title), name, value))
        }
        Touch::Button { cc, shift } => {
            // Shift + encoder page ▼: [ACMP].
            if cc == launchkey::KNOB_DOWN_CC && shift {
                return Some((s("Buttons"), s("ACMP"), if st.transport.acmp { s("On") } else { s("Off") }));
            }
            // Shift + encoder page ▲: the rotary speaker's speed.
            if cc == launchkey::KNOB_UP_CC && shift {
                return Some((s("Buttons"), s("ROTARY"), if st.effects.rotary_fast { s("Fast") } else { s("Slow") }));
            }
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

/// What the display says for knob `knob` turned in swap mode on keyboard part `part`, from
/// the state `st`. Knob 1: the part and its sound's number on top, the sound's name below
/// ("R1: 23" / "Rhodes Soft": the line `R1: 23 Rhodes Soft` split so a long name keeps
/// its 16 characters); a part playing no numbered sound shows "-" and what it plays.
/// Knobs 2-8: the part's mix knob and its value, as the state's knobs show them in swap
/// mode.
pub(super) fn swap_text(part: u8, knob: u8, st: &AppState) -> Option<Text> {
    let label = SWAP_LABELS.get(part as usize)?;
    if knob == 0 {
        let kp = st.keyboard_parts.get(part as usize)?;
        let sound = kp.patch.as_ref().and_then(|id| st.sound_library.patches.iter().find(|p| &p.patch.id == id));
        let (number, name) = match sound {
            Some(s) => (s.number.to_string(), s.patch.name.clone()),
            None => ("-".into(), kp.voice_name.clone()),
        };
        return Some((format!("{label}: {number}"), name, "Swap".into()));
    }
    let k = st.knobs.knobs.get(knob as usize)?;
    let value = if k.function == "none" { "-".into() } else { k.value.clone() };
    Some((format!("Swap {label}"), k.name.clone(), value))
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
            TransportCmd::TapTempo | TransportCmd::TempoUp | TransportCmd::TempoDown | TransportCmd::ResetTempo | TransportCmd::SetTempo { .. } => bpm(),
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
        AppCmd::Ots(OtsCmd::RecallOts { index }) => match st.ots.racks.get(*index as usize).filter(|r| r.rack.is_some() && !r.missing) {
            // A style rack: the rack it loaded.
            Some(r) => format!("OTS {} {}", index + 1, r.name),
            None => format!("OTS {}", index + 1),
        },
        AppCmd::Ots(OtsCmd::ToggleOtsLink) => on(st.ots.link),
        AppCmd::QuickRacks(c) => match c {
            QuickRackCmd::PressQuickRack { .. } | QuickRackCmd::StepQuickRack { .. } => st.live_rack.name.clone(),
            QuickRackCmd::StepQuickRackBank { .. } => format!("Bank {}", crate::racks::quick::bank_letter(st.quick_racks.bank as usize)),
            QuickRackCmd::ToggleQuickRackStore => on(st.quick_racks.store),
            QuickRackCmd::StoreRack { .. } => st.live_rack.name.clone(),
            QuickRackCmd::ClearQuickRack { .. } => "Cleared".into(),
        },
        AppCmd::Looper(LooperCmd::LooperOnOff | LooperCmd::LooperRec) => match st.looper.mode {
            LooperMode::Off if st.looper.has_data => "Off",
            LooperMode::Off => "Empty",
            LooperMode::RecArmed => "Rec at bar",
            LooperMode::Recording => "Recording",
            LooperMode::LoopArmed => "Loop at bar",
            LooperMode::Looping => "Looping",
        }
        .into(),
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
    use crate::session::testing::session;
    use crate::session::{Port, Session};

    fn shown(s: &Session) -> Option<Text> {
        s.display_shown()
    }

    /// Every kind of control names what it did and its value (#213): a headless session
    /// fed Launchkey messages.
    #[test]
    fn touched_controls_show_what_they_did() {
        let s = session();
        let text = |a: &str, b: &str, c: &str| Some((a.to_string(), b.to_string(), c.to_string()));
        // A tempo button.
        s.midi_in(Port::Pads, &[0xB0, launchkey::SCENE_CC, 127]);
        let bpm = s.state().transport.tempo.round() as i32;
        assert_eq!(shown(&s), text("Buttons", "TEMPO +", &format!("{bpm} BPM")));
        // A section pad: Start/Stop, then a Main.
        s.midi_in(Port::Pads, &[0x90, 119, 100]);
        assert_eq!(shown(&s), text("Pads: Sections", "START", "Main A"));
        // A knob.
        s.midi_in(Port::Pads, &[0xBF, 21, 62]);
        assert_eq!(shown(&s), text("Knobs: Style", "Dynamics Control", "123"));
        // A Panel fader: Right 2's level.
        s.midi_in(Port::Pads, &[0xB0, 6, 90]);
        let v = s.state().keyboard_parts[1].volume;
        let t = shown(&s).unwrap();
        assert_eq!((t.0.as_str(), t.1.as_str()), ("Faders: Panel", "RIGHT 2"));
        assert!(t.2.starts_with(&v.to_string()), "{t:?}");
        // A fader button: Right 3 on (a tap: it acts on the release).
        s.midi_in(Port::Pads, &[0xB0, 39, 127, 0xB0, 39, 0]);
        assert_eq!(shown(&s), text("Fader buttons", "RIGHT 3", "On"));
        // The encoder page button.
        s.midi_in(Port::Pads, &[0xB0, launchkey::KNOB_DOWN_CC, 127]);
        assert_eq!(shown(&s), text("Knobs", "KNOB ASSIGN", "Rack"));
        // Shift + ▲: the rotary speed, not the knob page.
        s.midi_in(Port::Pads, &[0xB0, launchkey::SHIFT_CC, 127, 0xB0, launchkey::KNOB_UP_CC, 127, 0xB0, launchkey::SHIFT_CC, 0]);
        assert_eq!(shown(&s), text("Buttons", "ROTARY", "Fast"));
        assert_eq!(s.state().knobs.page_name, "Rack", "the page stays");
        // A pad page button, then a Setup pad (the last page).
        s.midi_in(Port::Pads, &[0xB0, launchkey::PAD_DOWN_CC, 127]);
        assert_eq!(shown(&s), text("Buttons", "PAGE ▼", "Racks"));
        s.midi_in(Port::Pads, &[0xB0, launchkey::PAD_UP_CC, 127]);
        s.send(crate::api::PadsCmd::SetPadPage { page: launchkey::Page::Setup }).unwrap();
        s.midi_in(Port::Pads, &[0x90, 103, 100]);
        assert_eq!(shown(&s), text("Pads: Setup", "UPPER", "Upper"));
        // Fader button 8 on the Panel page: the Chord Looper; Shift: REC/STOP.
        s.midi_in(Port::Pads, &[0xB0, 44, 127]);
        assert_eq!(shown(&s).map(|t| t.1), Some("LOOPER".into()));
        s.midi_in(Port::Pads, &[0xB0, launchkey::SHIFT_CC, 127, 0xB0, 44, 127, 0xB0, launchkey::SHIFT_CC, 0]);
        let t = shown(&s).unwrap();
        assert_eq!((t.1.as_str(), t.2.as_str()), ("LOOP REC", "Rec at bar"));
    }

    /// Swap mode: knob 1 shows the part, the sound's number and its name on each step,
    /// and still does after the release; a mix knob shows the part's own setting.
    #[test]
    fn swap_mode_shows_the_part_number_and_sound() {
        use crate::session::part_sound::tests::{hold, turn, with_sounds};
        let s = with_sounds(&["Grand", "Rhodes Soft", "Strings"], &[]);
        let name = |n: u32| s.state().sound_library.patches.iter().find(|p| p.number == n).unwrap().patch.name.clone();
        let text = |a: &str, b: &str, c: &str| Some((a.to_string(), b.to_string(), c.to_string()));
        hold(&s, 0, true);
        turn(&s, 0, 1);
        assert_eq!(shown(&s), text("R1: 1", &name(1), "Swap"));
        turn(&s, 0, 1);
        assert_eq!(shown(&s), text("R1: 2", &name(2), "Swap"));
        // A mix knob: the part's pan (no command reaches it from the Launchkey yet, so it
        // shows where it is).
        turn(&s, 2, 1);
        let pan = crate::knobs::pan_text(s.state().keyboard_parts[0].pan);
        assert_eq!(shown(&s), text("Swap R1", "Right 1 Pan", &pan));
        turn(&s, 0, 1);
        hold(&s, 0, false);
        assert_eq!(shown(&s), text("R1: 3", &name(3), "Swap"));
        // Left, with no numbered sound yet: its voice.
        let st = s.state();
        let voice = st.keyboard_parts[3].voice_name.clone();
        assert_eq!(swap_text(3, 0, &st), text("L: -", &voice, "Swap"));
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

    /// Panel fader 5, the Style volume (#199), reads in percent; the Style page's fader 5 is
    /// a part's CC7.
    #[test]
    fn the_style_volume_fader_shows_percent() {
        let mut st = AppState::default();
        let f = |label: &str, v, waiting| crate::api::SurfaceFader { label: label.into(), value: Some(v), waiting, position: Some(60), set: None };
        st.surface.faders = vec![f("", 0, false); 4];
        st.surface.faders.push(f("STYLE", 85, false));
        assert_eq!(display_text(Touch::Fader(4), &st), Some(("Faders: Panel".into(), "STYLE".into(), "85%".into())));
        st.surface.faders[4] = f("STYLE", 100, true);
        assert_eq!(display_text(Touch::Fader(4), &st).unwrap().2, "100% > 60%");
        // Panel fader 6, the Multi Pad volume (#196), too.
        st.surface.faders.push(f("M.PAD", 70, false));
        assert_eq!(display_text(Touch::Fader(5), &st).unwrap().2, "70%");
        st.mixer.fader_page = FaderPage::Style;
        st.surface.faders[4] = f("CHORD 2", 85, false);
        assert_eq!(display_text(Touch::Fader(4), &st).unwrap().2, "85");
    }
}

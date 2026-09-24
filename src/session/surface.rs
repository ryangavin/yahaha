//! The Launchkey beyond the pads (Shift, buttons, faders, Track neighbours, the beat
//! clock), as `live::Input` runs it and `Leds` lights it.

use super::Control;
use crate::api::{ns_to_ms, AppCmd, ClockState, MixerCmd, Neighbour, PadsCmd, PartsCmd, SurfaceControl, SurfaceFader, SurfaceState, STYLE_PART_NAMES};
use crate::launchkey::{self, Action, Panel};
use crate::library::Library;
use crate::parts::{self, FaderPage};
use std::sync::atomic::Ordering::Relaxed;

impl Control {
    /// The Launchkey beyond the pads, as `live::Input` runs it and `Leds` lights it.
    pub(super) fn surface(&self, pnl: &Panel, manual_bass_active: bool, now: u64) -> SurfaceState {
        use launchkey::{cc_control, Control as C};
        let shared = &self.shared;
        let kp = &shared.parts;
        let page = pnl.page;
        let styles = self.published.count() > 1;
        let fader_page = kp.fader_page();
        let style_on = launchkey::style_lit(self.snap.parts, manual_bass_active);
        let colours = launchkey::button_colours(page, styles, fader_page, pnl.parts_on, style_on);
        let act = |cc: u8, shift: bool| -> Option<AppCmd> {
            match cc_control(cc, shift)? {
                C::Page(d) => {
                    let to = page.step(d);
                    (to != page).then_some(AppCmd::Pads(PadsCmd::SetPadPage { page: to }))
                }
                C::Act(Action::Style(_)) if !styles => None,
                C::Act(a) => Some(a.into()),
            }
        };
        let mut controls = Vec::new();
        let mut push = |id: String, cc: u8, label: &str, action: Option<AppCmd>, shift: Option<(&str, Option<AppCmd>)>| {
            let label = if action.is_some() { label.to_string() } else { String::new() };
            let (shift_label, shift_action) = match shift {
                Some((l, a)) => (if a.is_some() { l.to_string() } else { String::new() }, a),
                None => (label.clone(), action.clone()),
            };
            let colour = colours.iter().find(|c| c.0 == cc).map(|c| c.1);
            let (rgb, level) = colour.map_or(((0, 0, 0), launchkey::Level::Off), launchkey::palette_colour);
            controls.push(SurfaceControl {
                id,
                cc,
                label,
                action,
                shift_label,
                shift_action,
                rgb: [rgb.0, rgb.1, rgb.2],
                level,
                anim: launchkey::Anim::Solid,
                colour,
            });
        };
        for (id, cc, label, shift_label) in [
            ("padBankUp", launchkey::PAD_UP_CC, "PAGE ▲", "LEFT"),
            ("padBankDown", launchkey::PAD_DOWN_CC, "PAGE ▼", "OTS LINK"),
            ("trackPrev", launchkey::TRACK_LEFT_CC, "◀ STYLE", ""),
            ("trackNext", launchkey::TRACK_RIGHT_CC, "STYLE ▶", ""),
            ("play", launchkey::PLAY_CC, "PLAY", "RESET"),
            ("stop", launchkey::STOP_CC, "STOP", "FADE"),
            ("scene", launchkey::SCENE_CC, "TEMPO +", "RTG SHORT"),
            ("function", launchkey::FUNCTION_CC, "TEMPO -", "RTG LONG"),
        ] {
            let (a, sa) = (act(cc, false), act(cc, true));
            let shift = (sa != a).then_some((shift_label, sa));
            push(id.to_string(), cc, label, a, shift);
        }
        // The buttons under faders 1-8: Panel = Right 1-3 and Left on/off (Shift: select),
        // Style = the Style parts' mute.
        for i in 0..8u8 {
            let cc = launchkey::FADER_BTN_CC.start() + i;
            let id = format!("faderButton{}", i + 1);
            match fader_page {
                FaderPage::Panel if (i as usize) < parts::COUNT => {
                    let p = i as usize;
                    let shift = (launchkey::SELECT_LABELS[p], Some(AppCmd::Parts(PartsCmd::SelectPart { part: i })));
                    push(id, cc, launchkey::PART_LABELS[p], Some(AppCmd::Parts(PartsCmd::TogglePart { part: i })), Some(shift));
                }
                FaderPage::Panel => push(id, cc, "", None, None),
                FaderPage::Style => {
                    let name = STYLE_PART_NAMES[i as usize].to_uppercase();
                    push(id, cc, &name, Some(AppCmd::Mixer(MixerCmd::ToggleStylePart { part: i })), None);
                }
            }
        }
        let master = match fader_page {
            FaderPage::Panel => "PANEL",
            FaderPage::Style => "STYLE",
        };
        push("masterButton".into(), *launchkey::FADER_BTN_CC.end(), master, Some(AppCmd::Mixer(MixerCmd::ToggleFaderPage)), None);

        // The faders: the parts they control on this page, and where they physically are.
        let s = &self.snap;
        let mut faders: Vec<SurfaceFader> = (0..8u8)
            .map(|i| {
                let p = i as usize;
                let position = known(kp.fader_hw[p].load(Relaxed));
                match fader_page {
                    FaderPage::Panel if p < parts::COUNT => SurfaceFader {
                        label: launchkey::PART_LABELS[p].to_string(),
                        value: Some(kp.volume(p)),
                        waiting: kp.waiting(p),
                        position,
                        set: Some(AppCmd::Parts(PartsCmd::SetPartVolume { part: i, volume: 0 })),
                    },
                    FaderPage::Panel => SurfaceFader { position, ..SurfaceFader::default() },
                    FaderPage::Style => SurfaceFader {
                        label: STYLE_PART_NAMES[p].to_uppercase(),
                        value: Some(s.volumes[p]),
                        waiting: s.pickup & (1 << p) != 0,
                        position,
                        set: Some(AppCmd::Mixer(MixerCmd::SetStylePartVolume { part: i, volume: 0 })),
                    },
                }
            })
            .collect();
        let master_pos = known(shared.master_hw.load(Relaxed));
        faders.push(match &self.synth {
            Some(sy) => SurfaceFader {
                label: "MASTER".into(),
                value: Some(sy.control.master.load(Relaxed)),
                waiting: sy.control.master_waiting.load(Relaxed),
                position: master_pos,
                set: Some(AppCmd::Mixer(MixerCmd::SetMasterVolume { volume: 0 })),
            },
            None => SurfaceFader { position: master_pos, ..SurfaceFader::default() },
        });

        SurfaceState {
            shift: shared.shift.load(Relaxed),
            controls,
            faders,
            track_prev: neighbour(&self.published, self.cur, -1),
            track_next: neighbour(&self.published, self.cur, 1),
            clock: ClockState {
                at_ms: 0.0,
                running: s.running,
                tempo: s.bpm,
                beats_per_bar: self.info.quarters_per_bar,
                bar: 1,
                beat: 1,
                phase: 0.0,
                section_anchor_ms: ns_to_ms(s.anchor_ns),
                section_anchor_beats: s.anchor_beats,
                led_anchor_ms: ns_to_ms(self.led_ns),
                led_anchor_beats: self.led_beats,
            }
            .at(ns_to_ms(now)),
        }
    }
}

/// A fader position as last reported (`HW_UNKNOWN` = never).
pub(super) fn known(hw: u8) -> Option<u8> {
    (hw != crate::engine::HW_UNKNOWN).then_some(hw)
}

/// The style `StepStyle { delta }` would load, if it goes anywhere.
fn neighbour(lib: &Library, cur: usize, delta: i8) -> Option<Neighbour> {
    if cur >= lib.len() {
        return None;
    }
    let id = lib.step(cur, delta);
    (id != cur).then(|| {
        let e = lib.entry(id);
        Neighbour { id, name: e.name().to_string(), path: e.path.display().to_string() }
    })
}

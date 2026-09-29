//! The Launchkey beyond the pads (Shift, buttons, faders, Track neighbours, the beat
//! clock), as `live::Input` runs it and `Leds` lights it.

use super::Control;
use crate::api::{ns_to_ms, AppCmd, ChordCmd, ClockState, HarmonyArpCmd, LooperCmd, MixerCmd, Neighbour, PadsCmd, PartSend, PartsCmd, PluginCmd, RackCmd, SurfaceControl, SurfaceFader, SurfaceState, STYLE_PART_NAMES};
use crate::launchkey::{self, Action, Panel};
use crate::library::Library;
use crate::parts::{self, FaderPage, FaderRoute};
use std::sync::atomic::Ordering::Relaxed;

impl Control {
    /// The Launchkey beyond the pads, as `live::Input` runs it and `Leds` lights it.
    pub(super) fn surface(&self, pnl: &Panel, manual_bass_active: bool, now: u64) -> SurfaceState {
        use launchkey::{cc_control, Control as C};
        let shared = &self.shared;
        let kp = &shared.parts;
        let page = pnl.page;
        let styles = self.published.count() > 1;
        let quick_racks = self.quick_panel().stored != 0;
        let fader_page = kp.fader_page();
        let style_on = launchkey::style_lit(self.snap.parts, manual_bass_active);
        let colours = launchkey::button_colours(page, styles, fader_page, kp.fader_layer(), pnl.parts_on, style_on, pnl.lamps());
        let act = |cc: u8, shift: bool| -> Option<AppCmd> {
            match cc_control(cc, shift)? {
                C::Page(d) => {
                    let to = page.step(d);
                    (to != page).then_some(AppCmd::Pads(PadsCmd::SetPadPage { page: to }))
                }
                C::Act(Action::Style(_)) if !styles => None,
                // Shift + Track: dark while the Quick Racks bank on view has no rack.
                C::Act(Action::QuickRackStep(_)) if !quick_racks => None,
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
            ("trackPrev", launchkey::TRACK_LEFT_CC, "◀ STYLE", "◀ RACK"),
            ("trackNext", launchkey::TRACK_RIGHT_CC, "STYLE ▶", "RACK ▶"),
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
        // then HARMONY/ARPEGGIO; Style = the Style parts' mute.
        for i in 0..8u8 {
            let cc = launchkey::FADER_BTN_CC.start() + i;
            let id = format!("faderButton{}", i + 1);
            match fader_page {
                FaderPage::Panel if (i as usize) < parts::COUNT => {
                    let p = i as usize;
                    let shift = (launchkey::SELECT_LABELS[p], Some(AppCmd::Parts(PartsCmd::SelectPart { part: i })));
                    push(id, cc, launchkey::PART_LABELS[p], Some(AppCmd::Parts(PartsCmd::TogglePart { part: i })), Some(shift));
                }
                FaderPage::Panel if i == launchkey::HARM_ARP_FADER_BTN => {
                    push(id, cc, "HARM/ARP", Some(AppCmd::HarmonyArp(HarmonyArpCmd::ToggleHarmonyArp)), None)
                }
                FaderPage::Panel if i == launchkey::PLUGIN_FADER_BTN => {
                    push(id, cc, "PLUGIN", Some(AppCmd::Plugins(PluginCmd::ReloadPartPlugin { part: None })), None)
                }
                FaderPage::Panel if i == launchkey::LEFT_HOLD_FADER_BTN => push(id, cc, "L HOLD", Some(AppCmd::Chord(ChordCmd::ToggleLeftHold)), None),
                FaderPage::Panel if i == launchkey::LOOPER_FADER_BTN => {
                    push(id, cc, "LOOPER", Some(AppCmd::Looper(LooperCmd::LooperOnOff)), Some(("LOOP REC", Some(AppCmd::Looper(LooperCmd::LooperRec)))))
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
        // Shift: the next fader layer (VOL, PAN, REV, CHO, DLY).
        let layer = kp.fader_layer();
        let master = if layer == crate::parts::FaderLayer::Volume { master.to_string() } else { format!("{master} {}", layer.short()) };
        push("masterButton".into(), *launchkey::FADER_BTN_CC.end(), &master, Some(AppCmd::Mixer(MixerCmd::ToggleFaderPage)), Some(("LAYER", Some(AppCmd::Mixer(MixerCmd::StepFaderLayer { delta: 1 })))));

        // The faders: the parts they control on this page, and where they physically are.
        // Panel faders 1-4 in the Volume layer follow the live rack's controller map. In a
        // send layer the faders show and set what the hardware faders move there (as a
        // Genos slider's LED meter shows its Slider Assign Type's value, OM p.62-63):
        // Panel faders 1-4 the part's pan or send, the Style faders the Style part's own
        // send (nothing in PAN); Panel faders 5-6 stay levels.
        let s = &self.snap;
        let (knobs_now, strip_now) = (self.knobs_now(), self.strip_now());
        let mut faders: Vec<SurfaceFader> = (0..8u8)
            .map(|i| {
                let p = i as usize;
                let position = known(kp.fader_hw[p].load(Relaxed));
                let volume_layer = layer == crate::parts::FaderLayer::Volume;
                match fader_page {
                    FaderPage::Panel if p < parts::COUNT && !volume_layer => {
                        let fx = layer.fx_index().unwrap_or(parts::PAN);
                        let set = match layer_send(layer) {
                            Some(send) => PartsCmd::SetPartSend { part: i, send, value: 0 },
                            None => PartsCmd::SetPartPan { part: i, pan: 0 },
                        };
                        SurfaceFader {
                            label: launchkey::PART_LABELS[p].to_string(),
                            value: Some(kp.fx(p)[fx]),
                            waiting: kp.send_waiting.load(Relaxed) & (1 << p) != 0,
                            position,
                            set: Some(AppCmd::Parts(set)),
                        }
                    }
                    FaderPage::Style if !volume_layer => match layer_send(layer) {
                        Some(send) => SurfaceFader {
                            label: STYLE_PART_NAMES[p].to_uppercase(),
                            value: Some(s.style_sends[p][send.index() - parts::REVERB]),
                            waiting: s.send_pickup & (1 << p) != 0,
                            position,
                            set: Some(AppCmd::Mixer(MixerCmd::SetStylePartSend { part: i, send, value: 0 })),
                        },
                        // The Style parts have no pan control: the fader does nothing.
                        None => SurfaceFader { label: STYLE_PART_NAMES[p].to_uppercase(), position, ..SurfaceFader::default() },
                    },
                    FaderPage::Panel if p < parts::COUNT && volume_layer && kp.rack_fader(p) != FaderRoute::Own => {
                        if kp.rack_fader(p) == FaderRoute::Off {
                            return SurfaceFader { position, ..SurfaceFader::default() };
                        }
                        let f = crate::knobs::rack_function(&self.rack_controls.faders[p]);
                        SurfaceFader {
                            label: f.short().to_uppercase(),
                            value: self.knobs.read_at(f, &knobs_now, &strip_now).level,
                            waiting: false,
                            position,
                            set: Some(AppCmd::Rack(RackCmd::MoveRackFader { fader: i, volume: 0 })),
                        }
                    }
                    FaderPage::Panel if p < parts::COUNT => SurfaceFader {
                        label: launchkey::PART_LABELS[p].to_string(),
                        value: Some(kp.volume(p)),
                        waiting: kp.waiting(p),
                        position,
                        set: Some(AppCmd::Parts(PartsCmd::SetPartVolume { part: i, volume: 0 })),
                    },
                    FaderPage::Panel if p == parts::STYLE_LEVEL => SurfaceFader {
                        label: "STYLE".to_string(),
                        value: Some(kp.volume(p)),
                        waiting: kp.waiting(p),
                        position,
                        set: Some(AppCmd::Mixer(MixerCmd::SetStyleVolume { volume: 0 })),
                    },
                    FaderPage::Panel if p == parts::PAD_LEVEL => SurfaceFader {
                        label: "M.PAD".to_string(),
                        value: Some(kp.volume(p)),
                        waiting: kp.waiting(p),
                        position,
                        set: Some(AppCmd::Mixer(MixerCmd::SetMultiPadVolume { volume: 0 })),
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

/// The effect send a fader layer moves; None for Volume and Pan.
fn layer_send(layer: crate::parts::FaderLayer) -> Option<PartSend> {
    use crate::parts::FaderLayer as L;
    match layer {
        L::Reverb => Some(PartSend::Reverb),
        L::Chorus => Some(PartSend::Chorus),
        L::Delay => Some(PartSend::Variation),
        L::Volume | L::Pan => None,
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

#[cfg(test)]
mod tests {
    use crate::api::{AppCmd, MixerCmd, PartSend, PartsCmd};
    use crate::parts::{FaderLayer, FaderPage};
    use crate::session::{Options, Port, Session};

    /// Shift + encoder page ▲ on the Launchkey flips Organ Rotary Slow/Fast, as
    /// `toggleRotaryFast` does, and the ▲ light (the panel the LEDs are drawn from)
    /// follows the state, whichever way it was switched. ▲ alone still steps the knob page.
    #[test]
    fn shift_encoder_page_up_is_rotary_fast() {
        use crate::api::{FxCmd, KnobsCmd};
        use crate::launchkey::{KNOB_UP_CC, SHIFT_CC};
        let s = crate::session::testing::session();
        let fast = |s: &Session| s.state().effects.rotary_fast;
        let lit = |s: &Session| s.inner.lock().panel().rotary_fast;
        let shift_up = |s: &Session| {
            s.midi_in(Port::Pads, &[0xB0, SHIFT_CC, 127]);
            s.midi_in(Port::Pads, &[0xB0, KNOB_UP_CC, 127]);
            s.midi_in(Port::Pads, &[0xB0, KNOB_UP_CC, 0]);
            s.midi_in(Port::Pads, &[0xB0, SHIFT_CC, 0]);
            s.advance(1_000_000);
        };
        assert!(!fast(&s) && !lit(&s));
        shift_up(&s);
        assert!(fast(&s), "Shift + ▲: fast");
        assert!(lit(&s), "▲ lit");
        shift_up(&s);
        assert!(!fast(&s) && !lit(&s), "again: slow, dark");
        s.send(FxCmd::ToggleRotaryFast).unwrap();
        assert!(lit(&s), "lit when the app switches it");
        s.send(KnobsCmd::SetKnobPage { page: crate::knobs::KnobPage::Pan }).unwrap();
        s.midi_in(Port::Pads, &[0xB0, KNOB_UP_CC, 127]);
        s.advance(1_000_000);
        assert_eq!(s.state().knobs.page, crate::knobs::KnobPage::Rack, "▲ alone: the knob page before");
        assert!(fast(&s), "and the rotary stays");
    }

    /// #409: in a send layer the surface faders show and set the layer's value, the same
    /// one the hardware faders move (with soft takeover); the parts' volumes stay put.
    #[test]
    fn send_layers_turn_the_faders_into_pan_and_sends() {
        let path = std::env::temp_dir().join(format!("yahaha-fader-layers-{}.sty", std::process::id()));
        std::fs::write(&path, crate::sff::test_style::synthetic_style_bytes()).unwrap();
        let s = Session::offline(Options { paths: vec![path.clone()], ..Options::default() }).unwrap();
        s.finish_indexing();
        let _ = std::fs::remove_file(&path);
        let fader = |i: u8| *crate::launchkey::FADER_CC.start() + i;
        let set = |i: usize| s.state().surface.faders[i].set.clone();
        let vol = s.state().keyboard_parts[0].volume;

        // REV on the Panel page: fader 1 is Right 1's reverb send.
        s.send(MixerCmd::SetFaderLayer { layer: FaderLayer::Reverb }).unwrap();
        let st = s.state();
        let rev = st.keyboard_parts[0].reverb;
        let f = &st.surface.faders[0];
        assert_eq!((f.label.as_str(), f.value), ("RIGHT 1", Some(rev)));
        assert_eq!(f.set, Some(AppCmd::Parts(PartsCmd::SetPartSend { part: 0, send: PartSend::Reverb, value: 0 })));
        // The hardware fader, far from the send, waits; once it gets there it moves it.
        let far = if rev > 64 { 0 } else { 127 };
        s.midi_in(Port::Pads, &[0xB0, fader(0), far]);
        let f = s.state().surface.faders[0].clone();
        assert_eq!((f.value, f.waiting, f.position), (Some(rev), true, Some(far)));
        s.midi_in(Port::Pads, &[0xB0, fader(0), rev]);
        s.midi_in(Port::Pads, &[0xB0, fader(0), 90]);
        let st = s.state();
        assert_eq!((st.keyboard_parts[0].reverb, st.keyboard_parts[0].volume), (90, vol), "the send moves, not the volume");
        let f = &st.surface.faders[0];
        assert_eq!((f.value, f.waiting), (Some(90), false));
        // The app's fader (the command it sends) does the same.
        s.send(PartsCmd::SetPartSend { part: 0, send: PartSend::Reverb, value: 30 }).unwrap();
        let st = s.state();
        assert_eq!((st.surface.faders[0].value, st.keyboard_parts[0].volume), (Some(30), vol));
        // Faders 5-6 stay levels in every layer.
        assert_eq!(set(4), Some(AppCmd::Mixer(MixerCmd::SetStyleVolume { volume: 0 })));

        // PAN and DLY.
        s.send(MixerCmd::SetFaderLayer { layer: FaderLayer::Pan }).unwrap();
        assert_eq!(set(3), Some(AppCmd::Parts(PartsCmd::SetPartPan { part: 3, pan: 0 })));
        assert_eq!(s.state().surface.faders[3].value, Some(s.state().keyboard_parts[3].pan));
        s.send(MixerCmd::SetFaderLayer { layer: FaderLayer::Delay }).unwrap();
        assert_eq!(set(1), Some(AppCmd::Parts(PartsCmd::SetPartSend { part: 1, send: PartSend::Variation, value: 0 })));

        // The Style page: the Style part's own send; PAN does nothing there.
        s.send(MixerCmd::SetFaderPage { page: FaderPage::Style }).unwrap();
        s.send(MixerCmd::SetFaderLayer { layer: FaderLayer::Chorus }).unwrap();
        let st = s.state();
        let f = &st.surface.faders[2];
        assert_eq!(f.value, Some(st.mixer.style_parts[2].chorus));
        assert_eq!(f.set, Some(AppCmd::Mixer(MixerCmd::SetStylePartSend { part: 2, send: PartSend::Chorus, value: 0 })));
        s.send(MixerCmd::SetStylePartSend { part: 2, send: PartSend::Chorus, value: 55 }).unwrap();
        s.advance(1_000_000);
        let st = s.state();
        assert_eq!((st.surface.faders[2].value, st.mixer.style_parts[2].chorus), (Some(55), 55));
        s.send(MixerCmd::SetFaderLayer { layer: FaderLayer::Pan }).unwrap();
        let f = s.state().surface.faders[2].clone();
        assert_eq!((f.set, f.value), (None, None));

        // Back to VOL: the volumes again.
        s.send(MixerCmd::SetFaderLayer { layer: FaderLayer::Volume }).unwrap();
        s.send(MixerCmd::SetFaderPage { page: FaderPage::Panel }).unwrap();
        assert_eq!(set(0), Some(AppCmd::Parts(PartsCmd::SetPartVolume { part: 0, volume: 0 })));
    }
}

//! The live rack's controller map through offline sessions on the synthetic style (no
//! corpus): the Rack knob page and Panel faders 1-4 follow it, `setRackControl` edits it,
//! and it is saved and loaded with the rack.

use crate::api::*;
use crate::knobs::KnobPage;
use crate::session::testing::{data_dir, session, session_in};
use crate::session::{Port, Session};
use std::collections::BTreeMap;

fn knob(s: &Session, k: usize) -> KnobState {
    s.state().knobs.knobs[k].clone()
}

fn set(s: &Session, control: RackControl, index: u8, target: ControlTarget) -> Result<(), CmdError> {
    s.send(RackCmd::SetRackControl { control, index, target })
}

/// Panel fader `f` (0-3) moved to `v` on the Launchkey.
fn fader(s: &Session, f: u8, v: u8) {
    s.midi_in(Port::Pads, &[0xB0, crate::launchkey::FADER_CC.start() + f, v]);
    s.advance(1_000_000);
}

/// Panel fader `f` swept across its whole travel (so soft takeover picks its level up),
/// then to `v`.
fn sweep(s: &Session, f: u8, v: u8) {
    for x in [0, 127, v] {
        fader(s, f, x);
    }
}

/// With the default map the Rack page is the Parts page it replaced, and faders 1-4 are
/// the parts' levels on the input thread, as before.
#[test]
fn the_default_map_is_the_parts_page() {
    let s = session();
    s.send(KnobsCmd::SetKnobPage { page: KnobPage::Rack }).unwrap();
    let st = s.state();
    assert_eq!(st.knobs.page_name, "Rack");
    let shorts: Vec<_> = st.knobs.knobs.iter().map(|k| k.short.as_str()).collect();
    assert_eq!(shorts, ["Right1", "Right2", "Right3", "Left", "HarmVol", "MetroVol", "---", "Tempo"]);
    let (vol, harm, bpm) = (st.keyboard_parts[1].volume, st.harmony_arp.volume, st.transport.tempo.round() as u16);
    s.send(KnobsCmd::TurnKnob { knob: 1, delta: -3 }).unwrap();
    s.send(KnobsCmd::TurnKnob { knob: 4, delta: -2 }).unwrap();
    s.send(KnobsCmd::TurnKnob { knob: 7, delta: 2 }).unwrap();
    let st = s.state();
    assert_eq!(st.keyboard_parts[1].volume, vol - 6);
    assert_eq!(st.harmony_arp.volume, harm - 4);
    assert_eq!(st.transport.tempo.round() as u16, bpm + 2);
    assert_eq!(st.live_rack.controls, ControlMap::default(), "turning knobs leaves the map");
    // Faders 1-4: the parts' levels, labelled as before (soft takeover: picked up on the
    // way past the level).
    assert_eq!(st.surface.faders[0].label, "RIGHT 1");
    sweep(&s, 2, 90);
    assert_eq!(s.state().keyboard_parts[2].volume, 90);
}

/// Editing a knob's target changes what a turn does, the knob's label and the live rack
/// (modified); a target this build doesn't know, a fader on the tempo and a controller
/// that doesn't exist are refused.
#[test]
fn editing_a_knob_changes_what_it_does() {
    let s = session();
    s.send(KnobsCmd::SetKnobPage { page: KnobPage::Rack }).unwrap();
    set(&s, RackControl::Knob, 0, ControlTarget::PartPan { part: 3 }).unwrap();
    set(&s, RackControl::Knob, 5, ControlTarget::SplitPoint).unwrap();
    let st = s.state();
    assert!(st.live_rack.modified);
    assert_eq!(st.live_rack.controls.knobs[0], ControlTarget::PartPan { part: 3 });
    assert_eq!((knob(&s, 0).short.as_str(), knob(&s, 0).name.as_str()), ("PanL", "Left Pan"));
    assert_eq!((knob(&s, 5).short.as_str(), knob(&s, 5).value.clone()), ("Split", st.chord.split_name.clone()));
    let (vol, pan, split) = (st.keyboard_parts[0].volume, st.keyboard_parts[3].pan, st.chord.split);
    s.send(KnobsCmd::TurnKnob { knob: 0, delta: -4 }).unwrap();
    s.send(KnobsCmd::TurnKnob { knob: 5, delta: 3 }).unwrap();
    let st = s.state();
    assert_eq!(st.keyboard_parts[0].volume, vol, "knob 1 no longer sets Right 1's level");
    assert_eq!(st.keyboard_parts[3].pan, pan - 8);
    assert_eq!(st.chord.split, split + 3);

    assert!(set(&s, RackControl::Knob, 8, ControlTarget::Tempo).is_err());
    assert!(set(&s, RackControl::Fader, 0, ControlTarget::Tempo).is_err());
    assert!(set(&s, RackControl::Knob, 1, ControlTarget::Unknown(serde_json::json!({ "kind": "pluginMacro" }))).is_err());
    assert!(set(&s, RackControl::Knob, 1, ControlTarget::PartLevel { part: 4 }).is_err());
    assert_eq!(s.state().live_rack.controls.knobs[1], ControlTarget::PartLevel { part: 1 }, "unchanged");
}

/// A fader the map gives another target runs it from the Launchkey (through the input
/// thread's actions) and from the app's mirror (`moveRackFader`), labelled with it; one
/// set to none does nothing.
#[test]
fn a_remapped_fader_runs_its_target() {
    let s = session();
    set(&s, RackControl::Fader, 0, ControlTarget::PartReverb { part: 2 }).unwrap();
    set(&s, RackControl::Fader, 1, ControlTarget::None).unwrap();
    set(&s, RackControl::Fader, 3, ControlTarget::HarmonyArp).unwrap();
    let vol = s.state().keyboard_parts[0].volume;
    fader(&s, 0, 70);
    fader(&s, 1, 5);
    let st = s.state();
    assert_eq!(st.keyboard_parts[2].reverb, 70);
    assert_eq!((st.keyboard_parts[0].volume, st.keyboard_parts[1].volume == 5), (vol, false), "no part levels moved");
    let f = &st.surface.faders;
    assert_eq!((f[0].label.as_str(), f[0].value), ("REVR3", Some(70)));
    assert_eq!(f[0].set, Some(AppCmd::Rack(RackCmd::MoveRackFader { fader: 0, volume: 0 })));
    assert_eq!((f[1].label.as_str(), f[1].set.as_ref()), ("", None));
    assert!(!st.harmony_arp.on);
    fader(&s, 3, 100);
    assert!(s.state().harmony_arp.on, "Harmony/Arp on from 64");
    s.send(RackCmd::MoveRackFader { fader: 3, volume: 10 }).unwrap();
    assert!(!s.state().harmony_arp.on);
    s.send(RackCmd::MoveRackFader { fader: 0, volume: 12 }).unwrap();
    assert_eq!(s.state().keyboard_parts[2].reverb, 12);
    // Back to its own level: the input thread's level fader again.
    set(&s, RackControl::Fader, 0, ControlTarget::PartLevel { part: 0 }).unwrap();
    sweep(&s, 0, 45);
    assert_eq!(s.state().keyboard_parts[0].volume, 45);
    assert_eq!(s.state().surface.faders[0].label, "RIGHT 1");
}

/// The map is saved with the rack and comes back when it is loaded.
#[test]
fn the_map_is_saved_and_loaded_with_the_rack() {
    let d = data_dir("rack-controls-save");
    let s = session_in(&d);
    set(&s, RackControl::Knob, 2, ControlTarget::HarmonyArp).unwrap();
    set(&s, RackControl::Fader, 1, ControlTarget::PartChorus { part: 0 }).unwrap();
    let map = s.state().live_rack.controls.clone();
    s.send(RackCmd::SaveRackAs { name: "Mapped".into(), sound_names: BTreeMap::new() }).unwrap();
    let id = s.state().live_rack.id.clone().unwrap();
    let file = crate::racks::Rack::load(&crate::racks::path_for(&crate::racks::dir(&d), "Mapped")).unwrap();
    assert_eq!(file.controls, map);
    s.send(RackCmd::NewRack { discard: true }).unwrap();
    assert_eq!(s.state().live_rack.controls, ControlMap::default());
    s.send(KnobsCmd::SetKnobPage { page: KnobPage::Rack }).unwrap();
    assert_eq!(knob(&s, 2).short, "Right3");
    s.send(RackCmd::LoadRack { id, discard: false }).unwrap();
    let st = s.state();
    assert_eq!(st.live_rack.controls, map);
    assert!(!st.live_rack.modified);
    assert_eq!(knob(&s, 2).short, "HarmArp", "the knobs follow the loaded rack's map");
    assert_eq!(st.surface.faders[1].label, "CHOR1");
    let _ = std::fs::remove_dir_all(&d);
}

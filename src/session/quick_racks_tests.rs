//! Quick Racks through offline sessions on the synthetic style (no corpus): press, Store
//! (with and without a save first), banks, previous/next, the hardware's Recovered rack,
//! the file, and pad page 4.

use crate::api::*;
use crate::controllers::Function;
use crate::launchkey::{self, Action, Page};
use crate::racks::quick::{self, QuickRacks};
use crate::session::live_rack::FILE;
use crate::session::rack_cmds::RECOVERED;
use crate::session::testing::{data_dir, write_style};
use crate::session::{Options, Session};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn dir(test: &str) -> PathBuf {
    data_dir(&format!("quick-racks-{test}"))
}

fn session(d: &Path) -> Session {
    let live_rack = Some(d.join("app").join(FILE));
    Session::offline(Options { paths: vec![write_style(d)], data_dir: Some(d.to_path_buf()), live_rack, ..Options::default() }).unwrap()
}

fn quick_state(s: &Session) -> QuickRacksState {
    s.state().quick_racks.clone()
}

fn volume(s: &Session, part: usize) -> u8 {
    s.state().keyboard_parts[part].volume
}

fn rack_id(s: &Session, name: &str) -> String {
    s.state().racks.iter().find(|r| r.name == name).map(|r| r.id.clone()).unwrap_or_else(|| panic!("no rack {name}"))
}

/// Save the live rack as `name` with Right 1 at `vol`, and store it on `slot` of the bank on view.
fn rack_on(s: &Session, name: &str, vol: u8, slot: u8) -> String {
    s.send(PartsCmd::SetPartVolume { part: 0, volume: vol }).unwrap();
    s.send(RackCmd::SaveRackAs { name: name.into(), sound_names: BTreeMap::new() }).unwrap();
    s.send(QuickRackCmd::ToggleQuickRackStore).unwrap();
    s.send(QuickRackCmd::PressQuickRack { slot, discard: false }).unwrap();
    rack_id(s, name)
}

fn press(s: &Session, slot: u8) -> Result<(), CmdError> {
    s.send(QuickRackCmd::PressQuickRack { slot, discard: false })
}

#[test]
fn store_then_press_loads_the_rack() {
    let d = dir("press");
    let s = session(&d);
    let ballad = rack_on(&s, "Ballad", 60, 0);
    let q = quick_state(&s);
    assert!(!q.store, "storing disarms Store");
    assert_eq!(q.buttons.len(), 8);
    assert_eq!(q.buttons[0], QuickRackButton { rack: Some(ballad.clone()), name: "Ballad".into(), missing: false, loaded: true });
    assert_eq!(q.buttons[1], QuickRackButton::default());
    let loud = rack_on(&s, "Loud", 20, 1);
    assert_eq!((quick_state(&s).buttons[0].loaded, quick_state(&s).buttons[1].loaded), (false, true), "lit: the loaded rack's button");

    press(&s, 0).unwrap();
    assert_eq!(volume(&s, 0), 60);
    assert_eq!(s.state().live_rack.id.as_deref(), Some(ballad.as_str()));
    assert!(quick_state(&s).buttons[0].loaded);
    press(&s, 1).unwrap();
    assert_eq!((volume(&s, 0), s.state().live_rack.id.clone()), (20, Some(loud)));
    assert!(press(&s, 2).is_err(), "an empty button says so");
    assert!(s.state().message.as_ref().unwrap().text.contains("A3 is empty"));
    assert!(press(&s, 10).is_err());

    // The switching guard, as loadRack.
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 33 }).unwrap();
    assert_eq!(press(&s, 0), Err(CmdError::UnsavedChanges));
    assert_eq!(volume(&s, 0), 33, "nothing changed");
    assert!(matches!(s.state().live_rack.prompt, Some(RackPrompt::UnsavedChanges { then: RackSwitch::Load { .. } })));
    s.send(QuickRackCmd::PressQuickRack { slot: 0, discard: true }).unwrap();
    assert_eq!(volume(&s, 0), 60);
    drop(s);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn store_an_unsaved_rack_waits_for_the_save() {
    let d = dir("store-save");
    let s = session(&d);
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 70 }).unwrap();
    s.send(QuickRackCmd::StepQuickRackBank { delta: 1 }).unwrap();
    s.send(QuickRackCmd::ToggleQuickRackStore).unwrap();
    assert!(quick_state(&s).store);
    press(&s, 3).unwrap();
    let q = quick_state(&s);
    assert_eq!((q.store_waiting, q.buttons[3].rack.clone()), (Some(3), None), "a rack never saved waits for its save");
    s.send(RackCmd::SaveRackAs { name: "Gospel".into(), sound_names: BTreeMap::new() }).unwrap();
    let gospel = rack_id(&s, "Gospel");
    let q = quick_state(&s);
    assert_eq!((q.store, q.store_waiting, q.buttons[3].rack.clone(), q.buttons[3].name.as_str()), (false, None, Some(gospel.clone()), "Gospel"));
    let file = QuickRacks::load(&quick::path(&d)).unwrap().unwrap();
    assert_eq!(file.get(1, 3), Some(gospel.as_str()), "stored on B4 in the file");

    // A modified saved rack waits too; Cancel (Store off) lets it go.
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 71 }).unwrap();
    s.send(QuickRackCmd::ToggleQuickRackStore).unwrap();
    press(&s, 4).unwrap();
    assert_eq!(quick_state(&s).store_waiting, Some(4));
    s.send(QuickRackCmd::ToggleQuickRackStore).unwrap();
    assert_eq!((quick_state(&s).store, quick_state(&s).store_waiting), (false, None));
    s.send(RackCmd::SaveRack { sound_names: BTreeMap::new() }).unwrap();
    assert_eq!(quick_state(&s).buttons[4].rack, None, "a cancelled Store stores nothing");
    drop(s);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn banks_step_from_a_to_h_and_keep_their_buttons() {
    let d = dir("banks");
    let s = session(&d);
    assert_eq!(quick_state(&s).bank, 0);
    s.send(QuickRackCmd::StepQuickRackBank { delta: -1 }).unwrap();
    assert_eq!(quick_state(&s).bank, 0, "stops at A");
    let a1 = rack_on(&s, "One", 50, 0);
    for _ in 0..10 {
        s.send(QuickRackCmd::StepQuickRackBank { delta: 1 }).unwrap();
    }
    assert_eq!(quick_state(&s).bank, 7, "stops at H");
    assert_eq!(quick_state(&s).buttons[0].rack, None, "bank H is its own");
    let h8 = rack_on(&s, "Eight", 40, 7);
    s.send(QuickRackCmd::StepQuickRackBank { delta: -1 }).unwrap();
    assert_eq!(quick_state(&s).buttons[7].rack, None);
    for _ in 0..7 {
        s.send(QuickRackCmd::StepQuickRackBank { delta: -1 }).unwrap();
    }
    assert_eq!(quick_state(&s).buttons[0].rack.as_deref(), Some(a1.as_str()));
    // Slots 8 and 9 run on into the next bank (Regist 9-10).
    s.send(QuickRackCmd::StepQuickRackBank { delta: 1 }).unwrap();
    s.send(QuickRackCmd::StepQuickRackBank { delta: 1 }).unwrap();
    s.send(QuickRackCmd::StepQuickRackBank { delta: 1 }).unwrap();
    s.send(QuickRackCmd::StepQuickRackBank { delta: 1 }).unwrap();
    s.send(QuickRackCmd::StepQuickRackBank { delta: 1 }).unwrap();
    s.send(QuickRackCmd::StepQuickRackBank { delta: 1 }).unwrap();
    assert_eq!(quick_state(&s).bank, 6);
    press(&s, 0).unwrap_err();
    s.send(ControllersCmd::TriggerFunction { function: Function::Regist1 }).unwrap_err();
    press(&s, 9).unwrap_err(); // H2: empty
    s.send(QuickRackCmd::ClearQuickRack { bank: 7, slot: 7 }).unwrap();
    assert_eq!(QuickRacks::load(&quick::path(&d)).unwrap().unwrap().get(7, 7), None, "Clear empties H8 in the file");
    let _ = h8;
    drop(s);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn previous_and_next_rack_step_through_the_bank() {
    let d = dir("step");
    let s = session(&d);
    assert!(s.send(QuickRackCmd::StepQuickRack { delta: 1, discard: false }).is_err(), "an empty bank has none");
    let one = rack_on(&s, "One", 11, 1);
    let two = rack_on(&s, "Two", 22, 4);
    let three = rack_on(&s, "Three", 33, 6);
    let live = |s: &Session| s.state().live_rack.id.clone().unwrap();
    assert_eq!(live(&s), three);
    s.send(QuickRackCmd::StepQuickRack { delta: 1, discard: false }).unwrap();
    assert_eq!(live(&s), three, "it stops at the last");
    s.send(QuickRackCmd::StepQuickRack { delta: -1, discard: false }).unwrap();
    assert_eq!((live(&s), volume(&s, 0)), (two.clone(), 22));
    s.send(QuickRackCmd::StepQuickRack { delta: -1, discard: false }).unwrap();
    assert_eq!(live(&s), one);
    s.send(QuickRackCmd::StepQuickRack { delta: -1, discard: false }).unwrap();
    assert_eq!(live(&s), one, "it stops at the first");
    // From a rack on no button: + is the first, − the last.
    s.send(RackCmd::NewRack { discard: false }).unwrap();
    s.send(QuickRackCmd::StepQuickRack { delta: 1, discard: false }).unwrap();
    assert_eq!(live(&s), one);
    s.send(RackCmd::NewRack { discard: false }).unwrap();
    s.send(ControllersCmd::TriggerFunction { function: Function::RegistPrev }).unwrap();
    assert_eq!(live(&s), three, "Regist − is the previous Quick Rack");
    drop(s);
    let _ = std::fs::remove_dir_all(&d);
}

/// Shift + Track on the Launchkey is what the state says: with no rack in the bank on
/// view it has no action and does nothing (no "has no racks" message); with racks, it
/// steps through them.
#[test]
fn shift_track_does_nothing_on_an_empty_bank() {
    use crate::session::Port;
    let d = dir("shift-track");
    let s = session(&d);
    let shift_track = |s: &Session, cc: u8| {
        s.midi_in(Port::Pads, &[0xB0, launchkey::SHIFT_CC, 127]);
        s.midi_in(Port::Pads, &[0xB0, cc, 127]);
        s.midi_in(Port::Pads, &[0xB0, cc, 0]);
        s.midi_in(Port::Pads, &[0xB0, launchkey::SHIFT_CC, 0]);
    };
    let track = |s: &Session| s.state().surface.controls.iter().find(|c| c.id == "trackPrev").unwrap().shift_action.clone();
    assert_eq!(track(&s), None);
    let before = s.state().message.clone();
    shift_track(&s, launchkey::TRACK_LEFT_CC);
    assert_eq!(s.state().message, before, "an empty bank: nothing");
    let one = rack_on(&s, "One", 11, 1);
    rack_on(&s, "Two", 22, 4);
    assert_eq!(track(&s), Some(QuickRackCmd::StepQuickRack { delta: -1, discard: false }.into()));
    shift_track(&s, launchkey::TRACK_LEFT_CC);
    assert_eq!(s.state().live_rack.id, Some(one));
    drop(s);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_hardware_press_with_unsaved_changes_keeps_a_recovered_rack() {
    let d = dir("hardware");
    let s = session(&d);
    let ballad = rack_on(&s, "Ballad", 60, 0);
    let _loud = rack_on(&s, "Loud", 20, 1);
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 99 }).unwrap();
    // Pad page 4, Quick Rack 1: no dialog, so the switch goes ahead.
    s.hardware(Action::QuickRack(0)).unwrap();
    assert_eq!((volume(&s, 0), s.state().live_rack.id.clone()), (60, Some(ballad.clone())));
    assert!(s.state().live_rack.prompt.is_none());
    let recovered = s.state().racks.iter().find(|r| r.name == format!("{RECOVERED}Loud")).cloned().expect("the unsaved rack is kept");
    s.send(RackCmd::LoadRack { id: recovered.id, discard: false }).unwrap();
    assert_eq!(volume(&s, 0), 99, "as it was when the pad was pressed");

    // A pedal on Regist 2 (Quick Rack 2) does the same.
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 98 }).unwrap();
    s.hardware(Action::Assign(Function::Regist2)).unwrap();
    assert_eq!(volume(&s, 0), 20);
    assert!(s.state().racks.iter().any(|r| r.name == format!("{RECOVERED}{RECOVERED}Loud")));

    // Store from the hardware needs a saved rack: there is no save flow.
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 97 }).unwrap();
    s.hardware(Action::QuickRackStore).unwrap();
    assert!(s.hardware(Action::QuickRack(5)).is_err());
    let q = quick_state(&s);
    assert_eq!((q.store, q.store_waiting, q.buttons[5].rack.clone()), (false, None, None));
    drop(s);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_file_comes_back_and_a_newer_one_is_never_saved_over() {
    let d = dir("file");
    let s = session(&d);
    let ballad = rack_on(&s, "Ballad", 60, 2);
    drop(s);
    let s = session(&d);
    assert_eq!(quick_state(&s).buttons[2].rack.as_deref(), Some(ballad.as_str()), "Quick Racks come back on the next start");
    assert!(!quick_state(&s).read_only);
    // Deleting a rack empties its buttons (the loaded rack can't be deleted: load another).
    s.send(RackCmd::NewRack { discard: true }).unwrap();
    s.send(RackCmd::DeleteRack { id: ballad }).unwrap();
    assert_eq!(quick_state(&s).buttons[2].rack, None);
    assert_eq!(QuickRacks::load(&quick::path(&d)).unwrap().unwrap().get(0, 2), None);
    drop(s);

    let newer = r#"{"format":"yahaha.quick-racks","version":9,"banks":[["x"]]}"#;
    std::fs::write(quick::path(&d), newer).unwrap();
    let s = session(&d);
    assert!(quick_state(&s).read_only);
    assert!(s.state().message.as_ref().is_some_and(|m| m.error && m.text.contains("Quick Racks not loaded")));
    s.send(RackCmd::SaveRackAs { name: "Any".into(), sound_names: BTreeMap::new() }).unwrap();
    s.send(QuickRackCmd::ToggleQuickRackStore).unwrap();
    assert!(press(&s, 0).is_err());
    assert!(s.send(QuickRackCmd::ClearQuickRack { bank: 0, slot: 0 }).is_ok(), "nothing to clear");
    assert_eq!(std::fs::read_to_string(quick::path(&d)).unwrap(), newer, "never saved over");
    drop(s);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn page_4_pads_send_the_quick_racks_commands() {
    let d = dir("pads");
    let s = session(&d);
    s.send(PadsCmd::SetPadPage { page: Page::Racks }).unwrap();
    let st = s.state();
    assert_eq!(st.pads.page_name, "Racks");
    let action = |note: u8| st.pads.pads.iter().find(|p| p.note == note).and_then(|p| p.action.clone());
    for i in 0..8u8 {
        assert_eq!(action(96 + i), Some(QuickRackCmd::PressQuickRack { slot: i, discard: false }.into()));
    }
    for i in 0..4u8 {
        assert_eq!(action(112 + i), Some(crate::api::OtsCmd::RecallOts { index: i }.into()));
    }
    assert_eq!(action(116), Some(QuickRackCmd::StepQuickRackBank { delta: -1 }.into()));
    assert_eq!(action(117), Some(QuickRackCmd::StepQuickRackBank { delta: 1 }.into()));
    assert_eq!(action(118), Some(QuickRackCmd::ToggleQuickRackStore.into()));
    assert_eq!(action(119), None, "pad 119 is spare");
    // The lamps follow the buttons: stored blue, loaded red.
    rack_on(&s, "Ballad", 60, 0);
    rack_on(&s, "Loud", 20, 1);
    let st = s.state();
    let pad = |note: u8| st.pads.pads.iter().find(|p| p.note == note).unwrap().clone();
    let rgb = |c: (u8, u8, u8)| [c.0, c.1, c.2];
    assert_eq!(pad(96).rgb, rgb(launchkey::C_QUICK_STORED));
    assert_eq!(pad(97).rgb, rgb(launchkey::C_QUICK_LOADED));
    assert_eq!(pad(98).level, launchkey::Level::Off);
    // Shift + Track ▶: the next Quick Rack.
    let track = st.surface.controls.iter().find(|c| c.id == "trackNext").unwrap();
    assert_eq!((track.shift_label.as_str(), track.shift_action.clone()), ("RACK ▶", Some(QuickRackCmd::StepQuickRack { delta: 1, discard: false }.into())));
    drop(s);
    let _ = std::fs::remove_dir_all(&d);
}

/// `storeRack` stores the live rack on a button of the bank on view in one command,
/// overwriting what is there, as Store then the button does; a slot past 8 is refused.
#[test]
fn store_rack_overwrites_a_button_in_one_command() {
    let d = dir("store-rack");
    let s = session(&d);
    let ballad = rack_on(&s, "Ballad", 60, 0);
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 20 }).unwrap();
    s.send(RackCmd::SaveRackAs { name: "Loud".into(), sound_names: BTreeMap::new() }).unwrap();
    let loud = rack_id(&s, "Loud");
    s.send(QuickRackCmd::StoreRack { slot: 0 }).unwrap();
    let q = quick_state(&s);
    assert_eq!(q.buttons[0].rack.as_deref(), Some(loud.as_str()), "overwritten");
    assert_ne!(Some(ballad), q.buttons[0].rack);
    assert!(!q.store, "Store is not left armed");
    s.send(QuickRackCmd::StoreRack { slot: 3 }).unwrap();
    assert_eq!(quick_state(&s).buttons[3].rack.as_deref(), Some(loud.as_str()), "an empty button too");
    assert!(s.send(QuickRackCmd::StoreRack { slot: 8 }).is_err());
    drop(s);
    let _ = std::fs::remove_dir_all(&d);
}

/// Hold Sound (Panel fader button 6): the state's layer says so, and the pads show and do
/// what the Racks page does, from any page; let go, they are the page's again.
#[test]
fn holding_sound_shows_the_racks_page() {
    use crate::session::Port;
    let d = dir("sound-hold");
    let s = session(&d);
    let sound = launchkey::FADER_BTN_CC.start() + launchkey::SOUND_FADER_BTN;
    let b = s.state().surface.controls.iter().find(|c| c.cc == sound).cloned().unwrap();
    assert_eq!((b.label.as_str(), b.action), ("SOUND", None), "a hold: no command");
    assert_eq!(s.state().surface.layer, launchkey::Layer::None);
    s.midi_in(Port::Pads, &[0xB0, sound, 127]);
    let st = s.state();
    assert_eq!(st.surface.layer, launchkey::Layer::Sound);
    assert_eq!(st.pads.page, Page::Sections, "the page stays");
    let action = |note: u8| st.pads.pads.iter().find(|p| p.note == note).and_then(|p| p.action.clone());
    assert_eq!(action(96), Some(QuickRackCmd::PressQuickRack { slot: 0, discard: false }.into()));
    assert_eq!(action(112), Some(OtsCmd::RecallOts { index: 0 }.into()));
    s.midi_in(Port::Pads, &[0xB0, sound, 0]);
    let st = s.state();
    assert_eq!(st.surface.layer, launchkey::Layer::None);
    assert_eq!(st.pads.pads[0].label, "INTRO 1");
    drop(s);
    let _ = std::fs::remove_dir_all(&d);
}

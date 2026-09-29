//! The rack commands through offline sessions on the synthetic style (no corpus): each
//! command, the switching guard, the hardware switch's Recovered rack, and saving edited
//! sounds with the rack (the plugin cases need the `plugins` feature and DLSMusicDevice).

use super::RECOVERED;
use crate::api::*;
use crate::racks::{self, Rack, SoundRef};
use crate::session::live_rack::{FILE, NEW_NAME, QUIET_NS};
use crate::session::testing::{data_dir, write_style};
use crate::session::{Options, Session};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn dir(test: &str) -> PathBuf {
    data_dir(&format!("rack-cmd-{test}"))
}

/// An offline session with its data folder at `d` and its live rack in `d/app`.
fn session(d: &Path) -> Session {
    let live_rack = Some(d.join("app").join(FILE));
    Session::offline(Options { paths: vec![write_style(d)], data_dir: Some(d.to_path_buf()), live_rack, ..Options::default() }).unwrap()
}

fn live(s: &Session) -> LiveRackState {
    s.state().live_rack.clone()
}

fn rack_files(d: &Path) -> Vec<String> {
    racks::list(&racks::dir(d)).iter().map(|p| racks::name_of(p)).collect()
}

fn entry(s: &Session, name: &str) -> RackEntry {
    s.state().racks.iter().find(|r| r.name == name).cloned().unwrap_or_else(|| panic!("no rack {name}: {:?}", s.state().racks))
}

fn volume(s: &Session, part: usize) -> u8 {
    s.state().keyboard_parts[part].volume
}

fn save_as(s: &Session, name: &str) -> Result<(), CmdError> {
    s.send(RackCmd::SaveRackAs { name: name.into(), sound_names: BTreeMap::new() })
}

#[test]
fn save_as_save_load_and_the_racks_list() {
    let d = dir("save-load");
    let s = session(&d);
    assert!(s.state().racks.is_empty());
    s.send(PartsCmd::SetPartVoice { part: 0, program: 4 }).unwrap();
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 77 }).unwrap();
    assert!(live(&s).modified);

    save_as(&s, "Ballad").unwrap();
    let ballad = entry(&s, "Ballad");
    assert_eq!(live(&s), LiveRackState { name: "Ballad".into(), id: Some(ballad.id.clone()), modified: false, controls: ControlMap::default(), prompt: None });
    assert_eq!(ballad.parts.len(), 4);
    assert_eq!(ballad.parts[0], gm_name(4), "each part names its sound");
    assert_eq!(ballad.on, vec![true, false, false, false]);
    assert!(!ballad.needs_attention);
    assert!(save_as(&s, "Ballad").is_err(), "Save as… never overwrites another rack");
    assert!(save_as(&s, "  ").is_err(), "a rack needs a name");

    // Save overwrites the rack's own file, keeping its id.
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 60 }).unwrap();
    assert!(live(&s).modified);
    s.send(RackCmd::SaveRack { sound_names: BTreeMap::new() }).unwrap();
    assert_eq!(rack_files(&d), vec!["Ballad"]);
    let file = Rack::load(&racks::path_for(&racks::dir(&d), "Ballad")).unwrap();
    assert_eq!((file.id.as_str(), file.parts[0].volume), (ballad.id.as_str(), 60));
    assert!(!live(&s).modified);

    // Load: the parts as saved, the live rack that rack, unmodified.
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 20 }).unwrap();
    s.send(RackCmd::SaveRackAs { name: "Loud".into(), sound_names: BTreeMap::new() }).unwrap();
    let loud = entry(&s, "Loud").id;
    s.send(RackCmd::LoadRack { id: ballad.id.clone(), discard: false }).unwrap();
    assert_eq!((volume(&s, 0), s.state().keyboard_parts[0].program), (60, 4));
    assert_eq!(live(&s), LiveRackState { name: "Ballad".into(), id: Some(ballad.id.clone()), modified: false, controls: ControlMap::default(), prompt: None });
    s.send(RackCmd::LoadRack { id: loud.clone(), discard: false }).unwrap();
    assert_eq!(volume(&s, 0), 20);
    s.advance(QUIET_NS * 3);
    assert!(!live(&s).modified, "nothing that settles after a load counts as a change");
    assert!(s.send(RackCmd::LoadRack { id: "nope".into(), discard: false }).is_err());

    // Save with no rack of its own (a new rack) saves under its name.
    s.send(RackCmd::NewRack { discard: false }).unwrap();
    assert_eq!(live(&s), LiveRackState { name: NEW_NAME.into(), id: None, modified: false, controls: ControlMap::default(), prompt: None });
    assert_eq!((volume(&s, 0), s.state().keyboard_parts[0].program), (100, 0), "a new rack starts on the defaults");
    s.send(RackCmd::SaveRack { sound_names: BTreeMap::new() }).unwrap();
    assert_eq!(rack_files(&d), vec!["Ballad", "Loud", NEW_NAME]);
    assert_eq!(live(&s).id, Some(entry(&s, NEW_NAME).id));
    drop(s);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn loading_a_rack_autosaves_the_live_rack() {
    let d = dir("autosave");
    let s = session(&d);
    s.send(PartsCmd::SetPartVolume { part: 1, volume: 44 }).unwrap();
    save_as(&s, "Ballad").unwrap();
    let id = entry(&s, "Ballad").id;
    s.send(PartsCmd::SetPartVolume { part: 1, volume: 90 }).unwrap();
    s.send(RackCmd::LoadRack { id: id.clone(), discard: true }).unwrap();
    s.advance(QUIET_NS * 2);
    let path = d.join("app").join(FILE);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let text = loop {
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        if text.contains(&id) || std::time::Instant::now() > deadline {
            break text;
        }
        std::thread::yield_now();
    };
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!((v["rack"]["id"].as_str(), v["rack"]["name"].as_str(), v["modified"].as_bool()), (Some(id.as_str()), Some("Ballad"), Some(false)));
    assert_eq!(v["rack"]["parts"][1]["volume"], 44);
    drop(s);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn switching_with_unsaved_changes_asks_unless_discarding() {
    let d = dir("guard");
    let s = session(&d);
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 30 }).unwrap();
    save_as(&s, "Ballad").unwrap();
    let ballad = entry(&s, "Ballad").id;
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 99 }).unwrap();

    let then = RackSwitch::Load { id: ballad.clone(), name: "Ballad".into() };
    assert_eq!(s.send(RackCmd::LoadRack { id: ballad.clone(), discard: false }), Err(CmdError::UnsavedChanges));
    assert_eq!(volume(&s, 0), 99, "nothing changed");
    assert_eq!(live(&s).prompt, Some(RackPrompt::UnsavedChanges { then }));
    assert!(live(&s).modified);
    // Keep editing.
    s.send(RackCmd::DismissRackPrompt).unwrap();
    assert_eq!(live(&s).prompt, None);
    assert_eq!(s.send(RackCmd::NewRack { discard: false }), Err(CmdError::UnsavedChanges));
    assert_eq!(live(&s).prompt, Some(RackPrompt::UnsavedChanges { then: RackSwitch::New }));
    assert_eq!(volume(&s, 0), 99);

    // Discard and switch.
    s.send(RackCmd::LoadRack { id: ballad.clone(), discard: true }).unwrap();
    assert_eq!(volume(&s, 0), 30);
    assert_eq!(live(&s), LiveRackState { name: "Ballad".into(), id: Some(ballad.clone()), modified: false, controls: ControlMap::default(), prompt: None });

    // Save first from the prompt: the engine makes the held switch once saved.
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 60 }).unwrap();
    assert_eq!(s.send(RackCmd::NewRack { discard: false }), Err(CmdError::UnsavedChanges));
    s.send(RackCmd::SaveRack { sound_names: BTreeMap::new() }).unwrap();
    assert_eq!(live(&s).name, NEW_NAME, "switched after the save");
    s.send(RackCmd::LoadRack { id: ballad.clone(), discard: false }).unwrap();
    assert_eq!(volume(&s, 0), 60, "saved first");

    // A Save first whose save fails drops the switch: a later save doesn't switch.
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 40 }).unwrap();
    assert_eq!(s.send(RackCmd::NewRack { discard: false }), Err(CmdError::UnsavedChanges));
    assert!(save_as(&s, "Ballad").is_err(), "the name is taken");
    s.send(RackCmd::SaveRack { sound_names: BTreeMap::new() }).unwrap();
    assert_eq!(live(&s).name, "Ballad", "no switch");
    s.send(RackCmd::LoadRack { id: ballad.clone(), discard: true }).unwrap();
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 30 }).unwrap();
    s.send(RackCmd::SaveRack { sound_names: BTreeMap::new() }).unwrap();

    // Save first, then switch: no question.
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 50 }).unwrap();
    s.send(RackCmd::SaveRack { sound_names: BTreeMap::new() }).unwrap();
    s.send(RackCmd::NewRack { discard: false }).unwrap();
    assert_eq!(live(&s).name, NEW_NAME);
    assert_eq!(rack_files(&d), vec!["Ballad"], "no rack was kept for a switch that asked");
    drop(s);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_hardware_switch_keeps_the_unsaved_rack_as_recovered() {
    let d = dir("recovered");
    let s = session(&d);
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 30 }).unwrap();
    save_as(&s, "Ballad").unwrap();
    let ballad = entry(&s, "Ballad").id;
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 111 }).unwrap();
    save_as(&s, "Other").unwrap();
    let other = entry(&s, "Other").id;
    s.send(RackCmd::LoadRack { id: ballad.clone(), discard: false }).unwrap();
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 77 }).unwrap();

    s.load_rack_from_hardware(Some(&other)).unwrap();
    assert_eq!(volume(&s, 0), 111, "the switch went ahead");
    assert_eq!(live(&s), LiveRackState { name: "Other".into(), id: Some(other.clone()), modified: false, controls: ControlMap::default(), prompt: None });
    let recovered = format!("{RECOVERED}Ballad");
    let kept = entry(&s, &recovered);
    let file = Rack::load(&racks::path_for(&racks::dir(&d), &recovered)).unwrap();
    assert_eq!((file.id.as_str(), file.parts[0].volume), (kept.id.as_str(), 77), "the unsaved changes are kept");
    assert_ne!(kept.id, ballad);
    let saved = Rack::load(&racks::path_for(&racks::dir(&d), "Ballad")).unwrap();
    assert_eq!(saved.parts[0].volume, 30, "the rack itself is untouched");

    // Unmodified: nothing is kept. A second recovery of the same name gets a number.
    s.load_rack_from_hardware(None).unwrap();
    assert_eq!(live(&s).name, NEW_NAME);
    assert_eq!(rack_files(&d).len(), 3);
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 12 }).unwrap();
    s.load_rack_from_hardware(Some(&ballad)).unwrap();
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 13 }).unwrap();
    s.load_rack_from_hardware(None).unwrap();
    let names: Vec<String> = s.state().racks.iter().map(|r| r.name.clone()).collect();
    assert!(names.contains(&format!("{RECOVERED}{NEW_NAME}")), "{names:?}");
    assert!(names.contains(&format!("{RECOVERED}Ballad 2")), "{names:?}");
    drop(s);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn revert_rename_duplicate_and_delete() {
    let d = dir("manage");
    let s = session(&d);
    assert!(s.send(RackCmd::RevertRack).is_err(), "a new rack has nothing to go back to");
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 30 }).unwrap();
    save_as(&s, "Ballad").unwrap();
    let ballad = entry(&s, "Ballad").id;

    // Revert: the saved rack again, unmodified.
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 90 }).unwrap();
    s.send(RackCmd::RevertRack).unwrap();
    assert_eq!(volume(&s, 0), 30);
    assert!(!live(&s).modified);

    // Duplicate: a new id, "copy".
    s.send(RackCmd::DuplicateRack { id: ballad.clone() }).unwrap();
    let copy = entry(&s, "Ballad copy");
    assert_ne!(copy.id, ballad);
    s.send(RackCmd::DuplicateRack { id: ballad.clone() }).unwrap();
    assert!(rack_files(&d).contains(&"Ballad copy 2".to_string()));

    // Rename: the same id, a new file; the loaded rack's name follows, still unmodified.
    s.send(RackCmd::RenameRack { id: ballad.clone(), name: "Slow".into() }).unwrap();
    assert_eq!(entry(&s, "Slow").id, ballad);
    assert!(!rack_files(&d).contains(&"Ballad".to_string()));
    assert_eq!(live(&s), LiveRackState { name: "Slow".into(), id: Some(ballad.clone()), modified: false, controls: ControlMap::default(), prompt: None });
    assert!(s.send(RackCmd::RenameRack { id: ballad.clone(), name: "Ballad copy".into() }).is_err(), "taken");
    // Case only: a rename of itself, even on a case-insensitive filesystem; one file.
    s.send(RackCmd::RenameRack { id: ballad.clone(), name: "slow".into() }).unwrap();
    assert_eq!(entry(&s, "slow").id, ballad);
    assert!(rack_files(&d).contains(&"slow".to_string()));
    assert!(!rack_files(&d).contains(&"Slow".to_string()));
    assert_eq!(s.state().racks.iter().filter(|r| r.id == ballad).count(), 1);
    s.send(RackCmd::RenameRack { id: ballad.clone(), name: "Slow".into() }).unwrap();
    assert_eq!(entry(&s, "Slow").id, ballad);

    // Delete: never the loaded rack.
    assert!(s.send(RackCmd::DeleteRack { id: ballad.clone() }).is_err());
    assert!(rack_files(&d).contains(&"Slow".to_string()));
    s.send(RackCmd::DeleteRack { id: copy.id.clone() }).unwrap();
    assert!(!rack_files(&d).contains(&"Ballad copy".to_string()));
    assert!(s.state().racks.iter().all(|r| r.id != copy.id));
    assert!(s.send(RackCmd::DeleteRack { id: copy.id }).is_err(), "gone");
    drop(s);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_move_whose_old_file_stays_takes_the_new_file_back() {
    let d = dir("replace-file");
    std::fs::create_dir_all(&d).unwrap();
    let (old, new) = (d.join("gone.rack.json"), d.join("new.rack.json"));
    // `old` can't be removed (it isn't there): the new file goes again.
    assert!(super::replace_file(&old, &new, |p| Ok(std::fs::write(p, "{}")?)).is_err());
    assert!(!new.exists());
    std::fs::write(&old, "{}").unwrap();
    super::replace_file(&old, &new, |p| Ok(std::fs::write(p, "{}")?)).unwrap();
    assert!(new.exists() && !old.exists());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn racks_need_a_data_folder() {
    let s = crate::session::testing::session();
    assert!(save_as(&s, "Ballad").is_err());
    assert!(s.send(RackCmd::NewRack { discard: false }).is_ok(), "a new rack needs no file");
}

/// A rack part on a library sound is listed by the sound's name, and a SoundRef the
/// library lacks by its id.
#[test]
fn the_racks_list_names_library_sounds() {
    let d = dir("names");
    let s = session(&d);
    let source = crate::patches::PatchSource::SoundFont { file: "Other.sf2".into(), bank: 0, program: 33 };
    let patch = PatchFields { name: "My Bass".into(), category: Default::default(), tags: Vec::new(), favourite: false, source };
    s.send(SoundLibraryCmd::CreatePatch { patch }).unwrap();
    let id = s.state().sound_library.last_added.clone().unwrap();
    s.send(SoundLibraryCmd::SetPartPatch { part: 3, id: Some(id.clone()) }).unwrap();
    save_as(&s, "Bass").unwrap();
    let e = entry(&s, "Bass");
    assert_eq!(e.parts[3], "My Bass");
    let mut r = Rack::load(&racks::path_for(&racks::dir(&d), "Bass")).unwrap();
    r.parts[3].sound = SoundRef::Library { id: "gone".into() };
    r.save(&racks::path_for(&racks::dir(&d), "Bass")).unwrap();
    s.advance(3_000_000_000);
    assert_eq!(entry(&s, "Bass").parts[3], "gone");
    drop(s);
    let _ = std::fs::remove_dir_all(&d);
}

/// Saving edited plugin sounds with the rack, on DLSMusicDevice.
#[cfg(feature = "plugins")]
mod plugins {
    use super::*;
    use crate::patches::PatchSource;
    use std::time::{Duration, Instant};

    const DLS: &str = "aumu dls  appl";

    fn wait_playing(s: &Session, part: usize) {
        let t0 = Instant::now();
        while s.state().keyboard_parts[part].plugin.as_ref().is_some_and(|p| p.status == PluginStatus::Loading) {
            assert!(t0.elapsed() < Duration::from_secs(20), "the plugin did not load");
            s.advance(1_000_000);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(s.state().keyboard_parts[part].plugin.as_ref().map(|p| p.status), Some(PluginStatus::Playing));
    }

    /// Pump until no plugin state read is running, then twice more (a save fill lands after).
    fn wait_reads(s: &Session) {
        let t0 = Instant::now();
        while s.inner.lock().plugin_state_reads_pending() && t0.elapsed() < Duration::from_secs(10) {
            s.advance(1_000_000);
            std::thread::sleep(Duration::from_millis(2));
        }
        s.advance(1_000_000);
        s.advance(1_000_000);
    }

    /// Turn DLSMusicDevice's tuning on `part` (a real edit), and read the state so the part
    /// shows as edited.
    fn edit(s: &Session, part: usize, cents: f32) {
        let ch = crate::parts::CHANNEL[part];
        let editor = s.inner.lock().plugins.channels[ch as usize].as_ref().unwrap().editor.clone().unwrap();
        // The first read after the sound loaded is the baseline an edit is compared with.
        s.inner.lock().save_channel_state(ch).unwrap();
        wait_reads(s);
        editor.set_parameter(0, 0, 0, cents).unwrap();
        s.inner.lock().save_channel_state(ch).unwrap();
        wait_reads(s);
        assert!(s.state().keyboard_parts[part].sound_edited, "edited");
    }

    fn records(s: &Session) -> usize {
        s.state().sound_library.patches.len()
    }

    fn plugin_state(s: &Session, id: &str) -> String {
        match s.state().sound_library.patches.iter().find(|p| p.patch.id == id).unwrap().patch.source.clone() {
            PatchSource::Plugin { state, .. } => state,
            other => panic!("not a plugin sound: {other:?}"),
        }
    }

    fn dls_session(d: &Path) -> Session {
        let s = session(d);
        s.offline_audio(None, 48_000).unwrap();
        s
    }

    #[test]
    fn save_rack_updates_an_edited_own_sound_in_place() {
        let d = dir("own-sound");
        let s = dls_session(&d);
        s.send(PluginCmd::SetPartPlugin { part: 0, id: DLS.into(), state: None }).unwrap();
        wait_playing(&s, 0);
        s.send(SoundLibraryCmd::SaveSoundAs { part: 0, name: Some("My Keys".into()) }).unwrap();
        let id = s.state().sound_library.last_added.clone().unwrap();
        wait_reads(&s);
        let before = plugin_state(&s, &id);
        let n = records(&s);
        edit(&s, 0, 30.0);

        s.send(RackCmd::SaveRackAs { name: "Keys".into(), sound_names: BTreeMap::new() }).unwrap();
        wait_reads(&s);
        assert_eq!(records(&s), n, "the own sound is saved over, not added");
        assert_ne!(plugin_state(&s, &id), before, "with the edit");
        assert!(!s.state().keyboard_parts[0].sound_edited);
        let r = Rack::load(&racks::path_for(&racks::dir(&d), "Keys")).unwrap();
        assert_eq!((r.parts[0].sound.clone(), r.parts[0].edited_state.clone()), (SoundRef::Library { id: id.clone() }, None), "the rack names the saved sound");
        assert_eq!(entry(&s, "Keys").parts[0], "My Keys");
        assert!(!live(&s).modified);
        drop(s);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn save_rack_asks_a_name_for_an_edited_preset_and_makes_one_sound() {
        let d = dir("preset-sound");
        let (s, n) = edited_preset(&d);

        // No name: nothing saved; the prompt names the part.
        assert_eq!(s.send(RackCmd::SaveRackAs { name: "Pad".into(), sound_names: BTreeMap::new() }), Err(CmdError::NeedsSoundNames));
        let Some(RackPrompt::SoundNames { parts, save_as }) = live(&s).prompt else { panic!("no prompt: {:?}", live(&s)) };
        assert_eq!(parts.iter().map(|p| p.part).collect::<Vec<_>>(), vec![1]);
        assert!(!parts[0].suggested.is_empty());
        assert_eq!(save_as.as_deref(), Some("Pad"));
        assert!(rack_files(&d).is_empty() && records(&s) == n, "nothing saved");
        assert!(s.state().keyboard_parts[1].sound_edited);

        // With a name: exactly one new sound, which the rack names.
        let names = BTreeMap::from([(1, "Soft Pad".to_string())]);
        s.send(RackCmd::SaveRackAs { name: "Pad".into(), sound_names: names.clone() }).unwrap();
        wait_reads(&s);
        assert_eq!(records(&s), n + 1, "one save, one record");
        let mine = s.state().sound_library.patches.iter().find(|p| p.patch.name == "Soft Pad").unwrap().patch.id.clone();
        let r = Rack::load(&racks::path_for(&racks::dir(&d), "Pad")).unwrap();
        assert_eq!((r.parts[1].sound.clone(), r.parts[1].edited_state.clone()), (SoundRef::Library { id: mine.clone() }, None));
        assert_eq!(live(&s).prompt, None);
        assert!(!s.state().keyboard_parts[1].sound_edited);

        // Saving again: the part now plays the user's own sound, so nothing new.
        s.send(RackCmd::SaveRack { sound_names: BTreeMap::new() }).unwrap();
        wait_reads(&s);
        assert_eq!(records(&s), n + 1);
        drop(s);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// "Save first" from the unsaved-changes prompt, when the save needs a sound name:
    /// the switch waits through the name prompt and goes ahead once the save is done.
    #[test]
    fn save_first_keeps_the_switch_through_the_sound_name_prompt() {
        let d = dir("save-first-names");
        let (s, _) = edited_preset(&d);
        assert_eq!(s.send(RackCmd::NewRack { discard: false }), Err(CmdError::UnsavedChanges));
        assert_eq!(s.send(RackCmd::SaveRackAs { name: "Pad".into(), sound_names: BTreeMap::new() }), Err(CmdError::NeedsSoundNames));
        assert!(matches!(live(&s).prompt, Some(RackPrompt::SoundNames { .. })));
        let names = BTreeMap::from([(1, "Soft Pad".to_string())]);
        s.send(RackCmd::SaveRackAs { name: "Pad".into(), sound_names: names }).unwrap();
        assert!(rack_files(&d).contains(&"Pad".to_string()), "saved first");
        assert_eq!(live(&s), LiveRackState { name: NEW_NAME.into(), id: None, modified: false, controls: ControlMap::default(), prompt: None }, "then switched");
        drop(s);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// A session whose part 1 plays an edited `.aupreset` (no sound of the user's), and
    /// the sound library's record count.
    fn edited_preset(d: &Path) -> (Session, usize) {
        use crate::plugin::{presets, LoadConfig, PluginHost, PluginId};
        // An `.aupreset` of DLSMusicDevice's: a file preset, no sound of the user's.
        let root = d.join("Presets");
        let host = PluginHost::with_preset_roots(Some(d.join("plugins.json")), vec![root.clone()]);
        let info = host.info(&PluginId::DLS).unwrap();
        let state = host.load(&PluginId::DLS, LoadConfig::default()).unwrap().get_state().unwrap();
        let file = presets::write_user_preset(&root, &info, "Warm Strings", &state, false).unwrap();
        host.rescan().unwrap();
        let s = dls_session(&d);
        s.inner.lock().plugins.host = Some(host);
        s.inner.lock().start_plugin_scan(false);
        let t0 = Instant::now();
        while s.state().plugins.scanning || s.state().plugins.list.is_empty() {
            assert!(t0.elapsed() < Duration::from_secs(60), "the plugin scan did not finish");
            s.advance(1_000_000);
            std::thread::sleep(Duration::from_millis(5));
        }
        let preset = format!("au:{DLS}#u:{}", file.path.display());
        s.send(SoundsCmd::AssignSound { part: 1, id: preset.clone() }).unwrap();
        wait_playing(&s, 1);
        wait_reads(&s);
        assert_eq!(s.state().keyboard_parts[1].sound.clone().map(|t| t.id), Some(preset));
        let n = records(&s);
        edit(&s, 1, 20.0);
        (s, n)
    }
}

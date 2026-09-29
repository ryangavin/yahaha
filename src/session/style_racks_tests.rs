//! Style racks through offline sessions on the synthetic style (no corpus; it has four
//! OTS): choosing a rack for an OTS button and the file, recalling it from the app, the
//! Launchkey and OTS Link, "Style's own", a deleted rack, a newer file, and a style change
//! with OTS Link off.

use crate::api::*;
use crate::launchkey::Action;
use crate::racks::style_racks::{self, StyleRacks};
use crate::session::live_rack::FILE;
use crate::session::rack_cmds::RECOVERED;
use crate::session::testing::{data_dir, write_style};
use crate::session::{Options, Session};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const MS: u64 = 1_000_000;

/// The synthetic style's file name: its key in style-racks.json.
const STYLE: &str = "Synthetic.sty";

fn dir(test: &str) -> PathBuf {
    data_dir(&format!("style-racks-{test}"))
}

fn options(d: &Path, paths: Vec<PathBuf>) -> Options {
    Options { paths, data_dir: Some(d.to_path_buf()), live_rack: Some(d.join("app").join(FILE)), ..Options::default() }
}

fn session(d: &Path) -> Session {
    Session::offline(options(d, vec![write_style(d)])).unwrap()
}

fn rack_id(s: &Session, name: &str) -> String {
    s.state().racks.iter().find(|r| r.name == name).map(|r| r.id.clone()).unwrap_or_else(|| panic!("no rack {name}"))
}

/// Save the live rack as `name` with Right 1 on program 80 at level `vol`.
fn save_rack(s: &Session, name: &str, vol: u8) -> String {
    s.send(PartsCmd::SetPartVoice { part: 0, program: 80 }).unwrap();
    s.send(PartsCmd::SetPartVolume { part: 0, volume: vol }).unwrap();
    s.send(RackCmd::SaveRackAs { name: name.into(), sound_names: BTreeMap::new() }).unwrap();
    rack_id(s, name)
}

fn right1(s: &Session) -> (u8, u8) {
    let p = &s.state().keyboard_parts[0];
    (p.program, p.volume)
}

fn file(d: &Path) -> StyleRacks {
    StyleRacks::load(&style_racks::path(d)).unwrap().unwrap_or_default()
}

fn recovered(s: &Session) -> usize {
    s.state().racks.iter().filter(|r| r.name.starts_with(RECOVERED)).count()
}

#[test]
fn choosing_a_rack_for_an_ots_is_kept_per_style_and_read_back() {
    let d = dir("assign");
    let s = session(&d);
    let ballad = save_rack(&s, "Ballad", 60);
    assert_eq!(s.state().ots.racks, vec![OtsRack::default(); 4], "every OTS the style's own");
    assert!(!s.state().ots.racks_read_only);
    s.send(OtsCmd::SetOtsRack { index: 1, id: ballad.clone() }).unwrap();
    let want = OtsRack { rack: Some(ballad.clone()), name: "Ballad".into(), missing: false };
    assert_eq!(s.state().ots.racks[1], want);
    assert_eq!(file(&d).get(STYLE, 1), Some(ballad.as_str()), "kept by style file name");
    assert!(s.send(OtsCmd::SetOtsRack { index: 4, id: ballad.clone() }).is_err(), "no OTS 5");
    assert!(s.send(OtsCmd::SetOtsRack { index: 0, id: "nope".into() }).is_err(), "no such rack");
    let sty = std::fs::read(write_style(&d)).unwrap();
    drop(s);

    let s = session(&d);
    assert_eq!(s.state().ots.racks[1], want, "read back at start");
    s.send(OtsCmd::ClearOtsRack { index: 1 }).unwrap();
    assert_eq!(s.state().ots.racks[1], OtsRack::default());
    assert_eq!(file(&d), StyleRacks::default(), "the style's own again: nothing kept");
    assert_eq!(std::fs::read(d.join("styles").join(STYLE)).unwrap(), sty, "the style file isn't touched");
    drop(s);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn recalling_the_ots_loads_the_rack_through_the_guard() {
    let d = dir("recall");
    let s = session(&d);
    let ballad = save_rack(&s, "Ballad", 60);
    s.send(RackCmd::NewRack { discard: false }).unwrap();
    s.send(OtsCmd::SetOtsRack { index: 1, id: ballad.clone() }).unwrap();

    s.send(OtsCmd::RecallOts { index: 1 }).unwrap();
    let st = s.state();
    assert_eq!((st.live_rack.id.as_deref(), st.live_rack.modified), (Some(ballad.as_str()), false), "OTS 2 loads Ballad");
    assert_eq!(right1(&s), (80, 60));
    assert_eq!(st.ots.applied, 2, "it counts as OTS 2 recalled");
    assert!(st.transport.sync_start, "and turns Sync Start on, as any OTS recall");

    // OTS 1 is still the style's own: its voices (Right 1 plays program 0).
    s.send(OtsCmd::RecallOts { index: 0 }).unwrap();
    assert_eq!((s.state().keyboard_parts[0].program, s.state().ots.applied), (0, 1));

    // Unsaved changes: the guard asks, as loadRack; discarding makes the switch and the recall.
    assert!(s.state().live_rack.modified);
    assert_eq!(s.send(OtsCmd::RecallOts { index: 1 }), Err(CmdError::UnsavedChanges));
    let st = s.state();
    assert_eq!((st.keyboard_parts[0].program, st.ots.applied), (0, 1), "nothing changed yet");
    assert!(matches!(st.live_rack.prompt, Some(RackPrompt::UnsavedChanges { then: RackSwitch::Load { ref id, .. } }) if *id == ballad));
    s.send(RackCmd::LoadRack { id: ballad.clone(), discard: true }).unwrap();
    assert_eq!((right1(&s), s.state().ots.applied), ((80, 60), 2));
    assert_eq!(recovered(&s), 0);
    drop(s);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn launchkey_ots_pad_loads_the_rack_keeping_unsaved_changes() {
    let d = dir("pad");
    let s = session(&d);
    let ballad = save_rack(&s, "Ballad", 60);
    s.send(RackCmd::NewRack { discard: false }).unwrap();
    s.send(OtsCmd::SetOtsRack { index: 2, id: ballad.clone() }).unwrap();
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 33 }).unwrap();
    assert!(s.state().live_rack.modified);
    s.hardware(Action::Ots(2)).unwrap();
    let st = s.state();
    assert_eq!((st.live_rack.id.as_deref(), st.ots.applied), (Some(ballad.as_str()), 3), "pad page 3, OTS 3");
    assert_eq!(right1(&s), (80, 60));
    assert_eq!(recovered(&s), 1, "the unsaved rack is kept as Recovered");
    drop(s);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn ots_link_at_a_main_change_loads_the_rack() {
    let d = dir("link");
    let s = session(&d);
    let ballad = save_rack(&s, "Ballad", 60);
    s.send(RackCmd::NewRack { discard: false }).unwrap();
    s.send(OtsCmd::SetOtsRack { index: 1, id: ballad.clone() }).unwrap();
    assert!(!s.state().ots.link, "OTS Link is off by default");
    s.send(TransportCmd::Main { index: 1 }).unwrap();
    s.advance(10 * MS);
    assert_eq!(s.state().live_rack.id, None, "Link off: Main B leaves the rack alone");

    s.send(TransportCmd::Main { index: 0 }).unwrap();
    s.send(OtsCmd::SetOtsLink { on: true }).unwrap();
    s.advance(10 * MS);
    assert_eq!((s.state().ots.applied, s.state().keyboard_parts[0].program), (1, 0), "Main A: the style's own OTS 1");
    assert!(s.state().live_rack.modified, "the style's OTS changed the new rack");
    s.send(TransportCmd::Main { index: 1 }).unwrap();
    s.advance(10 * MS);
    let st = s.state();
    assert_eq!((st.live_rack.id.as_deref(), st.ots.applied), (Some(ballad.as_str()), 2), "Main B: OTS 2 loads Ballad");
    assert_eq!(right1(&s), (80, 60));
    assert_eq!(recovered(&s), 1, "no dialog: the unsaved rack is kept as Recovered");
    drop(s);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_style_s_own_puts_the_ots_back() {
    let d = dir("own");
    let s = session(&d);
    let ballad = save_rack(&s, "Ballad", 60);
    s.send(OtsCmd::SetOtsRack { index: 1, id: ballad.clone() }).unwrap();
    s.send(OtsCmd::ClearOtsRack { index: 1 }).unwrap();
    s.send(OtsCmd::RecallOts { index: 1 }).unwrap();
    let st = s.state();
    assert_eq!((st.keyboard_parts[0].program, st.keyboard_parts[1].program, st.ots.applied), (1, 49, 2), "the style's OTS 2 voices");
    assert_eq!(st.live_rack.id.as_deref(), Some(ballad.as_str()), "no rack switch");
    assert!(st.live_rack.modified);
    drop(s);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_deleted_rack_falls_back_to_the_style_s_own() {
    let d = dir("deleted");
    let s = session(&d);
    let ballad = save_rack(&s, "Ballad", 60);
    let gospel = save_rack(&s, "Gospel", 70);
    s.send(OtsCmd::SetOtsRack { index: 0, id: ballad.clone() }).unwrap();
    s.send(OtsCmd::SetOtsRack { index: 1, id: ballad.clone() }).unwrap();
    s.send(OtsCmd::SetOtsRack { index: 3, id: gospel.clone() }).unwrap();

    // Deleted in the app: the file forgets it.
    s.send(RackCmd::DeleteRack { id: ballad.clone() }).unwrap();
    let st = s.state();
    assert_eq!((st.ots.racks[0].clone(), st.ots.racks[1].clone()), (OtsRack::default(), OtsRack::default()));
    assert_eq!(st.ots.racks[3].rack.as_deref(), Some(gospel.as_str()));
    assert_eq!((file(&d).get(STYLE, 1), file(&d).get(STYLE, 3)), (None, Some(gospel.as_str())));

    // Removed behind yahaha's back: shown missing, and the style's own plays.
    s.send(RackCmd::NewRack { discard: false }).unwrap();
    let path = d.join("Racks").join("Gospel.rack.json");
    std::fs::remove_file(&path).unwrap();
    s.send(OtsCmd::RecallOts { index: 3 }).unwrap();
    let st = s.state();
    assert_eq!(st.ots.racks[3], OtsRack { rack: Some(gospel), name: String::new(), missing: true });
    assert_eq!((st.keyboard_parts[0].program, st.ots.applied, st.live_rack.id.clone()), (3, 4, None), "the style's OTS 4");
    drop(s);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_newer_style_racks_file_is_refused_and_never_saved_over() {
    let d = dir("newer-file");
    let newer = r#"{"format":"yahaha.style-racks","version":2,"styles":{}}"#;
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(style_racks::path(&d), newer).unwrap();
    let s = session(&d);
    let st = s.state();
    assert!(st.ots.racks_read_only);
    assert!(st.message.as_ref().is_some_and(|m| m.error && m.text.contains("Style racks not loaded")));
    let ballad = save_rack(&s, "Ballad", 60);
    assert!(s.send(OtsCmd::SetOtsRack { index: 0, id: ballad.clone() }).is_err());
    assert_eq!(s.state().ots.racks[0], OtsRack::default());
    s.send(RackCmd::NewRack { discard: false }).unwrap();
    s.send(RackCmd::DeleteRack { id: ballad }).unwrap();
    assert_eq!(std::fs::read_to_string(style_racks::path(&d)).unwrap(), newer);
    drop(s);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_style_change_with_ots_link_off_leaves_the_rack_alone() {
    let d = dir("style-change");
    let first = write_style(&d);
    let other = first.with_file_name("Other.sty");
    std::fs::copy(&first, &other).unwrap();
    let s = Session::offline(options(&d, vec![first.parent().unwrap().to_path_buf()])).unwrap();
    s.finish_indexing();
    let entries = s.library_list().entries;
    let id_of = |name: &str| entries.iter().find(|e| e.path.ends_with(name)).unwrap().id;
    s.send(LibraryCmd::LoadStyle { id: id_of(STYLE) }).unwrap();
    s.advance(10 * MS);
    for (p, prog, vol) in [(0u8, 80u8, 61u8), (1, 5, 62), (3, 40, 63)] {
        s.send(PartsCmd::SetPartVoice { part: p, program: prog }).unwrap();
        s.send(PartsCmd::SetPartVolume { part: p, volume: vol }).unwrap();
        s.send(PartsCmd::SetPartPan { part: p, pan: 20 + p }).unwrap();
        s.send(PartsCmd::SetPartOctave { part: p, octave: 1 }).unwrap();
    }
    s.send(PartsCmd::SetPartOn { part: 2, on: true }).unwrap();
    s.send(RackCmd::SaveRackAs { name: "Gospel".into(), sound_names: BTreeMap::new() }).unwrap();
    let gospel = rack_id(&s, "Gospel");
    s.send(RackCmd::SaveRackAs { name: "Ballad".into(), sound_names: BTreeMap::new() }).unwrap();
    let ballad = rack_id(&s, "Ballad");
    // A rack chosen for the other style's OTS 1 (set on the first visit) doesn't load
    // either while Link is off.
    let parts = |s: &Session| {
        s.state().keyboard_parts.iter().map(|p| (p.on, p.program, p.voice_name.clone(), p.volume, p.pan, p.reverb, p.chorus, p.variation, p.octave)).collect::<Vec<_>>()
    };
    let before = parts(&s);
    assert!(!s.state().live_rack.modified);

    for name in ["Other.sty", STYLE, "Other.sty"] {
        s.send(LibraryCmd::LoadStyle { id: id_of(name) }).unwrap();
        s.advance(50 * MS);
        let st = s.state();
        assert!(st.style.path.ends_with(name));
        assert!(!st.ots.link);
        assert_eq!(parts(&s), before, "loading {name} leaves every part's sound and mix");
        assert_eq!((st.live_rack.id.as_deref(), st.live_rack.modified, st.ots.applied), (Some(ballad.as_str()), false, 0), "and the rack unmodified");
        if name == "Other.sty" && st.ots.racks[0].rack.is_none() {
            s.send(OtsCmd::SetOtsRack { index: 0, id: gospel.clone() }).unwrap();
        }
    }
    drop(s);
    let _ = std::fs::remove_dir_all(&d);
}

//! Registration Memory and the Playlist through an offline session.

use crate::api::*;
use crate::fingering::Fingering;
use crate::launchkey::{Level, Page};
use crate::registration::{Group, Groups, PlaylistSort, Record, RecordTarget, SequenceEnd};
use crate::session::{Options, Port, Session};
use std::path::{Path, PathBuf};

const MS: u64 = 1_000_000;

fn corpus(name: &str) -> Option<PathBuf> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2").join(name);
    if p.exists() {
        Some(p)
    } else {
        eprintln!("corpus missing; skipping");
        None
    }
}

/// A fresh data folder for one test.
fn data_dir(test: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("yahaha-reg-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

/// Two styles (SlowWalker first) and a data folder.
fn session(test: &str) -> Option<(Session, PathBuf)> {
    let a = corpus("SlowWalker.T552.sty")?;
    let b = corpus("BubblyDub.T552.sty")?;
    let dir = data_dir(test);
    let s = Session::offline(Options { paths: vec![a, b], data_dir: Some(dir.clone()), ..Options::default() }).unwrap();
    s.finish_indexing();
    Some((s, dir))
}

fn style_id(s: &Session, name: &str) -> usize {
    s.library_list().entries.iter().find(|e| e.path.contains(name)).unwrap().id
}

/// The panel a registration stores, to compare before and after.
#[derive(Debug, PartialEq)]
struct Panel {
    style: String,
    tempo: f64,
    main: u8,
    parts: Vec<(bool, u8, u8, i8)>,
    transpose: (i8, i8),
    split: u8,
    fingering: Fingering,
    style_on: Vec<bool>,
    style_vol: Vec<u8>,
    ots_link: bool,
}

fn panel(s: &Session) -> Panel {
    let st = s.state();
    Panel {
        style: st.style.path.clone(),
        tempo: st.transport.tempo,
        main: st.transport.main,
        parts: st.keyboard_parts.iter().map(|p| (p.on, p.program, p.volume, p.octave)).collect(),
        transpose: (st.chord.transpose_keyboard, st.chord.transpose_master),
        split: st.chord.split,
        fingering: st.chord.fingering,
        style_on: st.mixer.style_parts.iter().map(|p| p.on).collect(),
        style_vol: st.mixer.style_parts.iter().map(|p| p.volume).collect(),
        ots_link: st.ots.link,
    }
}

/// Set up a distinctive panel on the second style.
fn dress(s: &Session) {
    s.send(LibraryCmd::LoadStyle { id: style_id(s, "BubblyDub") }).unwrap();
    s.advance(MS);
    for _ in 0..3 {
        s.send(TransportCmd::TempoUp).unwrap();
    }
    s.send(TransportCmd::Main { index: 2 }).unwrap();
    s.send(PartsCmd::SetPartVoice { part: 0, program: 40 }).unwrap();
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 90 }).unwrap();
    s.send(PartsCmd::SetPartOctave { part: 0, octave: 1 }).unwrap();
    s.send(PartsCmd::SetPartOn { part: 1, on: true }).unwrap();
    s.send(PartsCmd::SetPartVoice { part: 1, program: 11 }).unwrap();
    s.send(ChordCmd::SetTranspose { keyboard: 2, master: -1 }).unwrap();
    s.send(ChordCmd::SetSplit { note: 60 }).unwrap();
    s.send(ChordCmd::SetFingering { fingering: Fingering::Fingered }).unwrap();
    s.send(MixerCmd::ToggleStylePart { part: 5 }).unwrap();
    s.send(MixerCmd::SetStylePartVolume { part: 3, volume: 64 }).unwrap();
    s.send(OtsCmd::SetOtsLink { on: false }).unwrap();
    s.advance(MS);
}

/// Change everything the registration stores.
fn scramble(s: &Session) {
    s.send(LibraryCmd::LoadStyle { id: style_id(s, "SlowWalker") }).unwrap();
    s.advance(MS);
    s.send(TransportCmd::TempoDown).unwrap();
    s.send(TransportCmd::Main { index: 0 }).unwrap();
    s.send(PartsCmd::SetPartVoice { part: 0, program: 1 }).unwrap();
    s.send(PartsCmd::SetPartVolume { part: 0, volume: 30 }).unwrap();
    s.send(PartsCmd::SetPartOctave { part: 0, octave: -1 }).unwrap();
    s.send(PartsCmd::SetPartOn { part: 1, on: false }).unwrap();
    s.send(ChordCmd::ResetTranspose).unwrap();
    s.send(ChordCmd::SetSplit { note: 50 }).unwrap();
    s.send(ChordCmd::SetFingering { fingering: Fingering::SingleFinger }).unwrap();
    s.send(MixerCmd::SetStylePartVolume { part: 3, volume: 120 }).unwrap();
    s.advance(MS);
}

#[test]
fn memorize_and_recall_restore_the_panel() {
    let Some((s, dir)) = session("recall") else { return };
    dress(&s);
    let want = panel(&s);
    s.send(RegistrationCmd::MemorizeRegist { index: 0 }).unwrap();
    let st = s.state();
    let b = &st.registration.buttons[0];
    assert!(b.stored && b.style.as_deref() == Some(st.style.name.as_str()) && b.tempo == Some(want.tempo), "{b:?}");
    assert_eq!(b.voices[0].name, gm_name(40));
    assert_eq!(st.registration.selected, Some(0));
    assert!(st.registration.bank.dirty, "a new bank waits for a name");

    scramble(&s);
    assert_ne!(panel(&s), want);
    s.send(RegistrationCmd::PressRegist { index: 0 }).unwrap();
    s.advance(MS);
    assert_eq!(panel(&s), want);
    assert!(!s.state().registration.pending);
    // An empty button is refused.
    assert!(s.send(RegistrationCmd::RecallRegist { index: 5 }).is_err());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn memory_button_arms_memorize_for_the_next_press() {
    let Some((s, dir)) = session("memory") else { return };
    s.send(RegistrationCmd::ToggleRegistMemory).unwrap();
    assert!(s.state().registration.memory);
    s.send(RegistrationCmd::PressRegist { index: 3 }).unwrap();
    let st = s.state();
    assert!(!st.registration.memory && st.registration.buttons[3].stored);
    assert_eq!(st.pads.page_count, 4);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn freeze_leaves_frozen_groups_alone() {
    let Some((s, dir)) = session("freeze") else { return };
    dress(&s);
    let want = panel(&s);
    s.send(RegistrationCmd::MemorizeRegist { index: 1 }).unwrap();
    scramble(&s);
    s.send(RegistrationCmd::SetFreezeGroup { group: Group::Tempo, on: true }).unwrap();
    s.send(RegistrationCmd::SetFreezeGroup { group: Group::Voice, on: true }).unwrap();
    // Ticked but Freeze off: everything is recalled...
    s.send(RegistrationCmd::RecallRegist { index: 1 }).unwrap();
    s.advance(MS);
    assert_eq!(panel(&s), want);
    // ...with Freeze on, the tempo and the right-hand voices stay.
    scramble(&s);
    let scrambled = panel(&s);
    s.send(RegistrationCmd::ToggleFreeze).unwrap();
    assert!(s.state().registration.freeze);
    s.send(RegistrationCmd::RecallRegist { index: 1 }).unwrap();
    s.advance(MS);
    let got = panel(&s);
    assert_eq!(got.tempo, scrambled.tempo);
    assert_eq!(&got.parts[..3], &scrambled.parts[..3]);
    assert_eq!((got.style, got.split, got.transpose), (want.style, want.split, want.transpose));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn memorize_groups_limit_what_a_button_stores() {
    let Some((s, dir)) = session("groups") else { return };
    dress(&s);
    s.send(RegistrationCmd::SetMemorizeGroup { group: Group::Voice, on: false }).unwrap();
    s.send(RegistrationCmd::SetMemorizeGroup { group: Group::Style, on: false }).unwrap();
    s.send(RegistrationCmd::MemorizeRegist { index: 0 }).unwrap();
    let b = s.state().registration.buttons[0].clone();
    assert!(!b.groups.has(Group::Voice) && !b.groups.has(Group::Style) && b.groups.has(Group::Tempo));
    assert_eq!(b.style, None);
    scramble(&s);
    let scrambled = panel(&s);
    s.send(RegistrationCmd::RecallRegist { index: 0 }).unwrap();
    s.advance(MS);
    let got = panel(&s);
    assert_eq!((got.style, got.parts), (scrambled.style, scrambled.parts));
    assert_eq!(got.transpose, (2, -1), "Transpose was memorized");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_recall_while_playing_changes_style_at_the_bar_line() {
    let Some((s, dir)) = session("playing") else { return };
    dress(&s);
    let want = panel(&s);
    s.send(RegistrationCmd::MemorizeRegist { index: 0 }).unwrap();
    scramble(&s);
    s.send(TransportCmd::StartStop).unwrap();
    s.advance(10 * MS);
    let playing = s.state().style.path.clone();
    s.send(RegistrationCmd::RecallRegist { index: 0 }).unwrap();
    let st = s.state();
    assert!(st.registration.pending, "waiting for the bar line");
    assert_eq!(st.style.path, playing);
    s.advance(4_000 * MS);
    let st = s.state();
    assert!(!st.registration.pending && st.transport.running);
    let got = panel(&s);
    assert_eq!((&got.style, got.tempo, &got.parts), (&want.style, want.tempo, &want.parts));
    assert_eq!(got.main, want.main);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn ots_link_does_not_override_a_recalled_registration() {
    let Some((s, dir)) = session("otslink") else { return };
    if s.state().ots.settings.len() < 2 {
        eprintln!("style has fewer than 2 OTS; skipping");
        return;
    }
    s.send(TransportCmd::Main { index: 0 }).unwrap();
    s.send(OtsCmd::SetOtsLink { on: true }).unwrap();
    s.advance(MS);
    s.send(PartsCmd::SetPartVoice { part: 0, program: 77 }).unwrap();
    s.send(RegistrationCmd::MemorizeRegist { index: 0 }).unwrap();
    // Main B recalls OTS 2 through the link.
    s.send(TransportCmd::Main { index: 1 }).unwrap();
    s.advance(MS);
    assert_eq!(s.state().ots.applied, 2);
    s.send(RegistrationCmd::RecallRegist { index: 0 }).unwrap();
    for _ in 0..5 {
        s.advance(MS);
    }
    let st = s.state();
    assert_eq!(st.transport.main, 0);
    assert_eq!(st.keyboard_parts[0].program, 77, "the registration's voice, not OTS 1's");
    assert!(st.ots.link);
    // Once it has settled, the link works again.
    s.send(TransportCmd::Main { index: 1 }).unwrap();
    s.advance(MS);
    assert_eq!(s.state().ots.applied, 2);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn banks_save_step_and_the_sequence_runs_into_the_next_bank() {
    let Some((s, dir)) = session("banks") else { return };
    assert!(s.send(RegistrationCmd::StepRegistBank { delta: 1 }).is_err(), "no banks yet");
    // Bank A: buttons 1 and 3; the sequence 3, 1, then the next bank.
    s.send(PartsCmd::SetPartVoice { part: 0, program: 10 }).unwrap();
    s.send(RegistrationCmd::MemorizeRegist { index: 0 }).unwrap();
    s.send(PartsCmd::SetPartVoice { part: 0, program: 30 }).unwrap();
    s.send(RegistrationCmd::MemorizeRegist { index: 2 }).unwrap();
    s.send(RegistrationCmd::SetRegistSequence { steps: vec![2, 0], end: SequenceEnd::Next }).unwrap();
    s.send(RegistrationCmd::SetRegistSequenceOn { on: true }).unwrap();
    s.send(RegistrationCmd::SaveRegistBank { name: Some("A".into()) }).unwrap();
    let st = s.state();
    assert_eq!(st.registration.bank.name, "A");
    assert!(!st.registration.bank.dirty);
    assert!(dir.join("Registration/A.regist.json").is_file());
    // Bank B: button 2.
    s.send(RegistrationCmd::NewRegistBank).unwrap();
    s.send(PartsCmd::SetPartVoice { part: 0, program: 50 }).unwrap();
    s.send(RegistrationCmd::MemorizeRegist { index: 1 }).unwrap();
    s.send(RegistrationCmd::SetRegistSequence { steps: vec![1], end: SequenceEnd::Stop }).unwrap();
    s.send(RegistrationCmd::SaveRegistBank { name: Some("B".into()) }).unwrap();
    // Memorizing into a saved bank writes its file at once.
    s.send(RegistrationCmd::MemorizeRegist { index: 4 }).unwrap();
    let st = s.state();
    assert!(!st.registration.bank.dirty);
    assert_eq!(st.registration.banks.iter().map(|b| b.name.as_str()).collect::<Vec<_>>(), ["A", "B"]);
    assert_eq!(st.registration.bank.position, Some(1));

    s.send(RegistrationCmd::StepRegistBank { delta: -1 }).unwrap();
    let st = s.state();
    assert_eq!((st.registration.bank.name.as_str(), st.registration.selected), ("A", None));
    let stored: Vec<bool> = st.registration.buttons.iter().map(|b| b.stored).collect();
    assert_eq!(&stored[..3], [true, false, true]);
    // Regist +: button 3, then 1, then bank B's first step (button 2).
    s.send(RegistrationCmd::StepRegistSequence { delta: 1 }).unwrap();
    assert_eq!(s.state().keyboard_parts[0].program, 30);
    s.send(RegistrationCmd::StepRegistSequence { delta: 1 }).unwrap();
    assert_eq!(s.state().keyboard_parts[0].program, 10);
    s.send(RegistrationCmd::StepRegistSequence { delta: 1 }).unwrap();
    let st = s.state();
    assert_eq!((st.registration.bank.name.as_str(), st.registration.selected), ("B", Some(1)));
    assert_eq!(st.keyboard_parts[0].program, 50);
    // Bank B's sequence is off: Regist + is refused.
    assert!(s.send(RegistrationCmd::StepRegistSequence { delta: 1 }).is_err());
    // Rename and clear.
    s.send(RegistrationCmd::RenameRegist { index: 1, name: "Verse".into() }).unwrap();
    assert_eq!(s.state().registration.buttons[1].name, "Verse");
    s.send(RegistrationCmd::ClearRegist { index: 1 }).unwrap();
    assert!(!s.state().registration.buttons[1].stored);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn launchkey_page_4_recalls_and_memorizes() {
    let Some((s, dir)) = session("pads") else { return };
    s.send(PadsCmd::SetPadPage { page: Page::Registration }).unwrap();
    let st = s.state();
    assert_eq!(st.pads.pads[0].label, "REGIST 1");
    assert_eq!(st.pads.pads[0].action, Some(AppCmd::Registration(RegistrationCmd::PressRegist { index: 0 })));
    // The Memory pad, then pad 10.
    s.midi_in(Port::Pads, &[0x90, 116, 100]);
    assert!(s.state().registration.memory);
    s.midi_in(Port::Pads, &[0x90, 113, 100]);
    let st = s.state();
    assert!(st.registration.buttons[9].stored);
    assert_eq!(st.pads.pads[9].level, Level::Bright);
    // The Freeze pad.
    s.midi_in(Port::Pads, &[0x90, 117, 100]);
    assert!(s.state().registration.freeze);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn playlist_records_load_banks_buttons_and_styles() {
    let Some((s, dir)) = session("playlist") else { return };
    assert!(s.send(PlaylistCmd::AddCurrentBank).is_err(), "the bank has no file yet");
    s.send(PartsCmd::SetPartVoice { part: 0, program: 22 }).unwrap();
    s.send(RegistrationCmd::MemorizeRegist { index: 6 }).unwrap();
    s.send(RegistrationCmd::SaveRegistBank { name: Some("Gig".into()) }).unwrap();
    s.send(PlaylistCmd::AddCurrentBank).unwrap();
    s.send(LibraryCmd::LoadStyle { id: style_id(&s, "BubblyDub") }).unwrap();
    s.advance(MS);
    s.send(PlaylistCmd::AddCurrentStyle).unwrap();
    let st = s.state();
    let recs = &st.playlist.records;
    assert_eq!(recs.len(), 2);
    let bank = dir.join("Registration/Gig.regist.json").display().to_string();
    assert_eq!(recs[0].record.target, RecordTarget::Bank { path: bank, regist: Some(6) });
    assert_eq!(recs[0].record.name, "Gig [7]");
    assert!(!recs[0].missing);
    s.send(PlaylistCmd::SavePlaylist { name: Some("Friday".into()) }).unwrap();
    let file = dir.join("Playlists/Friday.playlist.json");
    assert!(file.is_file());

    // Elsewhere: a new bank, another voice; then the set list brings the song back.
    s.send(RegistrationCmd::NewRegistBank).unwrap();
    s.send(PartsCmd::SetPartVoice { part: 0, program: 0 }).unwrap();
    s.send(PlaylistCmd::NewPlaylist).unwrap();
    s.send(PlaylistCmd::LoadPlaylist { path: file.display().to_string() }).unwrap();
    s.send(PlaylistCmd::StepPlaylist { delta: 1 }).unwrap();
    s.advance(MS);
    let st = s.state();
    assert_eq!((st.registration.bank.name.as_str(), st.registration.selected, st.playlist.current), ("Gig", Some(6), Some(0)));
    assert_eq!(st.keyboard_parts[0].program, 22);
    s.send(PlaylistCmd::StepPlaylist { delta: 1 }).unwrap();
    s.advance(MS);
    assert!(s.state().style.path.contains("BubblyDub"));
    // Sorted: no moving or deleting; saving keeps the displayed order.
    let r = Record { name: "Aardvark".into(), target: RecordTarget::Style { path: "/nowhere.sty".into() } };
    s.send(PlaylistCmd::AddPlaylistRecord { record: r }).unwrap();
    assert!(s.state().playlist.records[2].missing);
    s.send(PlaylistCmd::SetPlaylistSort { sort: PlaylistSort::AToZ }).unwrap();
    assert_eq!(s.state().playlist.records[0].record.name, "Aardvark");
    assert!(s.send(PlaylistCmd::DeletePlaylistRecord { index: 0 }).is_err());
    s.send(PlaylistCmd::SavePlaylist { name: None }).unwrap();
    let st = s.state();
    assert_eq!(st.playlist.sort, PlaylistSort::Normal);
    assert_eq!((st.playlist.records[0].index, st.playlist.records[0].record.name.as_str()), (0, "Aardvark"));
    s.send(PlaylistCmd::MovePlaylistRecord { index: 0, delta: 1 }).unwrap();
    s.send(PlaylistCmd::DeletePlaylistRecord { index: 1 }).unwrap();
    assert_eq!(s.state().playlist.records.len(), 2);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn without_a_data_dir_banks_cannot_be_saved() {
    let Some(p) = corpus("SlowWalker.T552.sty") else { return };
    let s = Session::offline(Options { paths: vec![p], ..Options::default() }).unwrap();
    let st = s.state();
    assert_eq!((st.registration.folder.clone(), st.playlist.folder.clone()), (None, None));
    assert_eq!(st.registration.buttons.len(), 10);
    assert_eq!(st.registration.memorize_groups, Groups::all());
    s.send(RegistrationCmd::MemorizeRegist { index: 0 }).unwrap();
    assert!(s.send(RegistrationCmd::SaveRegistBank { name: Some("x".into()) }).is_err());
}

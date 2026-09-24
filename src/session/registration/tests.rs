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

/// Play on to the middle of the first bar's second beat: a style change asked for
/// within the first beat comes in at once (Section Change Timing, #94), so a recall that
/// waits for the bar line must be made later.
fn past_first_beat(s: &Session) {
    let beat_ms = 60_000.0 / s.state().transport.tempo;
    s.advance((beat_ms * 1.5) as u64 * MS);
}

fn panel(s: &Session) -> Panel {
    let st = s.state();
    Panel {
        style: st.style.path.clone(),
        // Whole BPM: a recall sets the tempo as the panel does (a style's own may be 82.99997).
        tempo: st.transport.tempo.round(),
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
    past_first_beat(&s);
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
    s.send(RegistrationCmd::SaveRegistBank { name: Some("A".into()), overwrite: false }).unwrap();
    let st = s.state();
    assert_eq!(st.registration.bank.name, "A");
    assert!(!st.registration.bank.dirty);
    assert!(dir.join("Registration/A.regist.json").is_file());
    // Bank B: button 2.
    s.send(RegistrationCmd::NewRegistBank).unwrap();
    s.send(PartsCmd::SetPartVoice { part: 0, program: 50 }).unwrap();
    s.send(RegistrationCmd::MemorizeRegist { index: 1 }).unwrap();
    s.send(RegistrationCmd::SetRegistSequence { steps: vec![1], end: SequenceEnd::Stop }).unwrap();
    s.send(RegistrationCmd::SaveRegistBank { name: Some("B".into()), overwrite: false }).unwrap();
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
    // Sequence On/Off is not a bank setting: it is still on in bank B, whose sequence ends
    // (End = Stop) on its only step.
    assert!(st.registration.sequence.on);
    s.send(RegistrationCmd::StepRegistSequence { delta: 1 }).unwrap();
    assert_eq!(s.state().registration.selected, Some(1));
    s.send(RegistrationCmd::SetRegistSequenceOn { on: false }).unwrap();
    assert!(s.send(RegistrationCmd::StepRegistSequence { delta: 1 }).is_err(), "off: Regist + is refused");
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
    s.send(RegistrationCmd::SaveRegistBank { name: Some("Gig".into()), overwrite: false }).unwrap();
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
    s.send(PlaylistCmd::SavePlaylist { name: Some("Friday".into()), overwrite: false }).unwrap();
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
    s.send(PlaylistCmd::SavePlaylist { name: None, overwrite: false }).unwrap();
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
    assert!(s.send(RegistrationCmd::SaveRegistBank { name: Some("x".into()), overwrite: false }).is_err());
}

fn save(s: &Session, name: &str) {
    s.send(RegistrationCmd::SaveRegistBank { name: Some(name.into()), overwrite: false }).unwrap();
}

fn message(s: &Session) -> (String, bool) {
    let m = s.state().message.clone().expect("a message");
    (m.text, m.error)
}

/// Review B1: a second press before the bar line (a double tap, or another button with
/// the same style) must still recall the tempo and the Style mixer after the style loads.
#[test]
fn double_press_while_playing_keeps_tempo_and_mixer() {
    let Some((s, dir)) = session("double") else { return };
    dress(&s);
    let want = panel(&s);
    s.send(RegistrationCmd::MemorizeRegist { index: 0 }).unwrap();
    // Button 2: the same style, but Style part 4 at another level.
    s.send(MixerCmd::SetStylePartVolume { part: 3, volume: 100 }).unwrap();
    s.advance(MS);
    s.send(RegistrationCmd::MemorizeRegist { index: 1 }).unwrap();
    scramble(&s);
    s.send(TransportCmd::StartStop).unwrap();
    past_first_beat(&s);
    s.send(RegistrationCmd::RecallRegist { index: 0 }).unwrap();
    s.send(RegistrationCmd::RecallRegist { index: 0 }).unwrap(); // double tap
    assert!(s.state().registration.pending, "still waiting for the bar line");
    s.advance(4_000 * MS);
    let got = panel(&s);
    assert_eq!((&got.style, got.tempo, &got.style_vol, &got.style_on), (&want.style, want.tempo, &want.style_vol, &want.style_on));
    assert_eq!(got.style_vol[3], 64);

    // Button 1, then button 2 (same style) before the bar: button 2's mixer wins.
    scramble(&s);
    s.advance(4_000 * MS);
    s.send(RegistrationCmd::RecallRegist { index: 0 }).unwrap();
    s.send(RegistrationCmd::RecallRegist { index: 1 }).unwrap();
    s.advance(4_000 * MS);
    let got = panel(&s);
    assert_eq!((&got.style, got.tempo, got.style_vol[3]), (&want.style, want.tempo, 100));
    let _ = std::fs::remove_dir_all(dir);
}

/// Review B2: what a recall could not do is reported, and the rest still recalls.
#[test]
fn missing_style_is_reported() {
    let Some((s, dir)) = session("missing") else { return };
    s.send(PartsCmd::SetPartVoice { part: 0, program: 61 }).unwrap();
    s.send(RegistrationCmd::MemorizeRegist { index: 0 }).unwrap();
    save(&s, "M");
    let file = dir.join("Registration/M.regist.json");
    let mut v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    v["memories"][0]["sections"]["style"]["path"] = "/gone/Nope.sty".into();
    std::fs::write(&file, v.to_string()).unwrap();
    s.send(RegistrationCmd::SelectRegistBank { path: file.display().to_string() }).unwrap();
    s.send(PartsCmd::SetPartVoice { part: 0, program: 1 }).unwrap();
    s.send(RegistrationCmd::RecallRegist { index: 0 }).unwrap();
    s.advance(MS);
    let (text, error) = message(&s);
    assert!(error && text.contains("Registration 1") && text.contains("style not found: /gone/Nope.sty"), "{text}");
    assert_eq!(s.state().keyboard_parts[0].program, 61, "the rest is recalled");
    // Through a Playlist record too.
    s.send(PlaylistCmd::AddCurrentBank).unwrap();
    s.send(PlaylistCmd::LoadPlaylistRecord { index: 0 }).unwrap();
    let (text, error) = message(&s);
    assert!(error && text.starts_with("Playlist 1") && text.contains("style not found"), "{text}");
    // A recall that works says so, not as an error.
    s.send(RegistrationCmd::MemorizeRegist { index: 1 }).unwrap();
    s.send(RegistrationCmd::RecallRegist { index: 1 }).unwrap();
    assert!(!message(&s).1);
    let _ = std::fs::remove_dir_all(dir);
}

/// Review B3: Sequence On/Off is not a Registration item (Data List, Regist Sequence:
/// On/Off Regist = X): it stays when the bank changes, and it is kept across sessions.
#[test]
fn sequence_on_off_stays_across_banks() {
    let Some((s, dir)) = session("seqon") else { return };
    s.send(RegistrationCmd::MemorizeRegist { index: 0 }).unwrap();
    s.send(RegistrationCmd::SetRegistSequence { steps: vec![0], end: SequenceEnd::Next }).unwrap();
    save(&s, "A");
    s.send(RegistrationCmd::NewRegistBank).unwrap();
    s.send(RegistrationCmd::MemorizeRegist { index: 1 }).unwrap();
    s.send(RegistrationCmd::SetRegistSequence { steps: vec![1], end: SequenceEnd::Stop }).unwrap();
    save(&s, "B");
    s.send(RegistrationCmd::SetRegistSequenceOn { on: true }).unwrap();
    let b_file = std::fs::read_to_string(dir.join("Registration/B.regist.json")).unwrap();
    s.send(RegistrationCmd::StepRegistBank { delta: -1 }).unwrap();
    let st = s.state();
    assert_eq!(st.registration.bank.name, "A");
    assert!(st.registration.sequence.on, "On/Off stays when the bank changes");
    // The bank files don't hold it (turning it on didn't rewrite B).
    assert_eq!(std::fs::read_to_string(dir.join("Registration/B.regist.json")).unwrap(), b_file);
    let bv: serde_json::Value = serde_json::from_str(&b_file).unwrap();
    assert!(bv["sequence"].get("on").is_none(), "{b_file}");
    // End = Next runs on into B, whatever B was saved with.
    s.send(RegistrationCmd::StepRegistSequence { delta: 1 }).unwrap();
    s.send(RegistrationCmd::StepRegistSequence { delta: 1 }).unwrap();
    let st = s.state();
    assert_eq!((st.registration.bank.name.as_str(), st.registration.selected), ("B", Some(1)));
    drop(s);
    // A new session keeps the setting (the Genos's Setup/Backup).
    let s = Session::offline(Options { paths: vec![corpus("SlowWalker.T552.sty").unwrap()], data_dir: Some(dir.clone()), ..Options::default() }).unwrap();
    assert!(s.state().registration.sequence.on);
    // An older bank file's `on` is ignored.
    let old = r#"{"format":"yahaha.registration-bank","version":1,"name":"x","memories":[],"sequence":{"on":false,"steps":[3],"end":"top"}}"#;
    let seq = crate::registration::Bank::from_json(old).unwrap().sequence;
    assert_eq!((seq.steps, seq.end), (vec![3], SequenceEnd::Top));
    let _ = std::fs::remove_dir_all(dir);
}

/// Review B4: Save As with the name of another bank's (or playlist's) file does not
/// replace it unless asked to.
#[test]
fn save_as_existing_name_needs_overwrite() {
    let Some((s, dir)) = session("saveas") else { return };
    s.send(RegistrationCmd::MemorizeRegist { index: 0 }).unwrap();
    s.send(RegistrationCmd::MemorizeRegist { index: 1 }).unwrap();
    save(&s, "Gig");
    let gig = dir.join("Registration/Gig.regist.json");
    s.send(RegistrationCmd::NewRegistBank).unwrap();
    assert!(s.send(RegistrationCmd::SaveRegistBank { name: Some("Gig".into()), overwrite: false }).is_err());
    assert!(message(&s).0.contains("already exists"));
    assert_eq!(crate::registration::Bank::load(&gig).unwrap().stored_mask(), 0b11);
    let st = s.state();
    assert_eq!((st.registration.bank.path.clone(), st.registration.bank.name.as_str()), (None, "New Bank"));
    // Memorizing into the unsaved bank still doesn't touch Gig.
    s.send(RegistrationCmd::MemorizeRegist { index: 5 }).unwrap();
    assert_eq!(crate::registration::Bank::load(&gig).unwrap().stored_mask(), 0b11);
    // Saving a bank under its own name again is fine; overwriting is on request.
    s.send(RegistrationCmd::SaveRegistBank { name: Some("Gig".into()), overwrite: true }).unwrap();
    assert_eq!(crate::registration::Bank::load(&gig).unwrap().stored_mask(), 1 << 5);
    save(&s, "Gig");

    // Playlists alike.
    s.send(PlaylistCmd::AddCurrentBank).unwrap();
    s.send(PlaylistCmd::SavePlaylist { name: Some("Friday".into()), overwrite: false }).unwrap();
    s.send(PlaylistCmd::NewPlaylist).unwrap();
    assert!(s.send(PlaylistCmd::SavePlaylist { name: Some("Friday".into()), overwrite: false }).is_err());
    let file = dir.join("Playlists/Friday.playlist.json");
    assert_eq!(crate::registration::Playlist::load(&file).unwrap().records.len(), 1);
    s.send(PlaylistCmd::SavePlaylist { name: Some("Friday".into()), overwrite: true }).unwrap();
    assert_eq!(crate::registration::Playlist::load(&file).unwrap().records.len(), 0);
    let _ = std::fs::remove_dir_all(dir);
}

/// Review N2: a part whose voice this build can't read (a newer `kind`) doesn't stop the
/// other parts from recalling; the message says which part.
#[test]
fn unknown_voice_kind_blocks_only_that_part() {
    let Some((s, dir)) = session("voicekind") else { return };
    s.send(PartsCmd::SetPartOn { part: 1, on: true }).unwrap();
    s.send(PartsCmd::SetPartVoice { part: 1, program: 55 }).unwrap();
    s.send(RegistrationCmd::MemorizeRegist { index: 0 }).unwrap();
    save(&s, "V");
    let file = dir.join("Registration/V.regist.json");
    let mut v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    v["memories"][0]["sections"]["parts"]["parts"][0]["voice"] = serde_json::json!({ "kind": "plugin", "id": "au.x", "state": "..." });
    std::fs::write(&file, v.to_string()).unwrap();
    s.send(RegistrationCmd::SelectRegistBank { path: file.display().to_string() }).unwrap();
    s.send(PartsCmd::SetPartVoice { part: 1, program: 3 }).unwrap();
    s.send(RegistrationCmd::RecallRegist { index: 0 }).unwrap();
    s.advance(MS);
    assert_eq!(s.state().keyboard_parts[1].program, 55);
    let (text, error) = message(&s);
    assert!(error && text.contains("voice not available"), "{text}");
    // Regist Bank Info still lists the part, and the file keeps the voice.
    assert_eq!(s.state().registration.buttons[0].voices[0].name, "?");
    s.send(RegistrationCmd::RenameRegist { index: 0, name: "Keep".into() }).unwrap();
    assert!(std::fs::read_to_string(&file).unwrap().contains("au.x"));
    let _ = std::fs::remove_dir_all(dir);
}

/// Review r2 N1: on a case-insensitive file system (APFS, the Mac's default) "gig" is the
/// file "Gig". Saving your own bank under the name in another case renames it, and it
/// stays the bank in use; another bank's file in another case still needs Overwrite.
#[test]
fn save_own_bank_with_other_case() {
    let Some((s, dir)) = session("case") else { return };
    s.send(RegistrationCmd::MemorizeRegist { index: 0 }).unwrap();
    save(&s, "Gig");
    let folder = dir.join("Registration");
    let insensitive = folder.join("GIG.regist.json").exists();
    save(&s, "gig");
    let st = s.state();
    let path = PathBuf::from(st.registration.bank.path.clone().unwrap());
    assert_eq!(path.file_name().unwrap(), "gig.regist.json");
    assert_eq!(st.registration.bank.name, "gig");
    assert!(st.registration.bank.position.is_some(), "the saved bank is in the list: {:?}", st.registration.banks);
    let names: Vec<String> = crate::registration::list_banks(&folder).iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
    if insensitive {
        assert_eq!(names, ["gig.regist.json"]);
    }
    // A new bank can't take it in any case without Overwrite; with it, it keeps the name typed.
    s.send(RegistrationCmd::NewRegistBank).unwrap();
    assert!(s.send(RegistrationCmd::SaveRegistBank { name: Some("GIG".into()), overwrite: false }).is_err() || !insensitive);
    if insensitive {
        s.send(RegistrationCmd::SaveRegistBank { name: Some("GIG".into()), overwrite: true }).unwrap();
        let st = s.state();
        assert!(st.registration.bank.path.as_deref().unwrap().ends_with("GIG.regist.json"));
        assert!(st.registration.bank.position.is_some());
    }

    // Playlists alike.
    s.send(PlaylistCmd::SavePlaylist { name: Some("Friday".into()), overwrite: false }).unwrap();
    s.send(PlaylistCmd::SavePlaylist { name: Some("friday".into()), overwrite: false }).unwrap();
    let st = s.state();
    assert!(st.playlist.path.as_deref().unwrap().ends_with("friday.playlist.json"));
    assert_eq!(st.playlist.name, "friday");
    if insensitive {
        assert_eq!(st.playlist.playlists.len(), 1);
        assert!(st.playlist.playlists[0].path.ends_with("friday.playlist.json"));
    }
    let _ = std::fs::remove_dir_all(dir);
}

/// Review r2 B1: a recall that finds the Style mixer already where it stored it leaves the
/// parts to the style: its patterns' CC7 (Intro, Main, Ending levels) still move the
/// untouched parts, as with no recall at all.
#[test]
fn recall_leaves_pattern_levels_to_the_style() {
    let Some(path) = corpus("NightCruiser.S930.STY") else { return };
    let levels = |recall: bool| {
        let dir = data_dir(if recall { "cc7-recall" } else { "cc7-plain" });
        let s = Session::offline(Options { paths: vec![path.clone()], data_dir: Some(dir.clone()), ..Options::default() }).unwrap();
        s.finish_indexing();
        s.advance(MS);
        if recall {
            s.send(RegistrationCmd::MemorizeRegist { index: 0 }).unwrap();
            s.send(RegistrationCmd::RecallRegist { index: 0 }).unwrap();
            for _ in 0..5 {
                s.advance(MS);
            }
            assert!(!s.state().registration.pending);
        }
        let vol = |s: &Session| s.state().mixer.style_parts.iter().map(|p| p.volume).collect::<Vec<u8>>();
        let mut out = vec![vol(&s)];
        s.send(TransportCmd::Intro { index: 0 }).unwrap();
        s.send(TransportCmd::StartStop).unwrap();
        s.advance(500 * MS);
        out.push(vol(&s));
        s.send(TransportCmd::Main { index: 1 }).unwrap();
        s.advance(12_000 * MS);
        out.push(vol(&s));
        s.send(TransportCmd::Ending { index: 0 }).unwrap();
        s.advance(4_000 * MS);
        out.push(vol(&s));
        let _ = std::fs::remove_dir_all(dir);
        out
    };
    let plain = levels(false);
    assert!(plain.iter().any(|v| *v != plain[0]), "the style's patterns set their own levels: {plain:?}");
    assert_eq!(levels(true), plain, "a no-op recall must not freeze the pattern levels");
}

/// Review r3 B1: a registration memorized after the song has played stores only the levels
/// the player set (the Genos's Volume(Style) offset), not the ones the patterns' CC7 left on
/// the mixer (an Ending's). On recall the other parts follow the style's patterns again,
/// even a part the player moved since; the player's part holds its level all song.
#[test]
fn memorize_after_playing_keeps_pattern_levels_the_styles() {
    let Some(path) = corpus("NightCruiser.S930.STY") else { return };
    let vol = |s: &Session| s.state().mixer.style_parts.iter().map(|p| p.volume).collect::<Vec<u8>>();
    let song = |s: &Session| {
        let mut out = Vec::new();
        s.send(TransportCmd::Intro { index: 0 }).unwrap();
        s.send(TransportCmd::StartStop).unwrap();
        s.advance(500 * MS);
        s.send(TransportCmd::Main { index: 1 }).unwrap();
        s.advance(12_000 * MS);
        out.push(vol(s));
        s.send(TransportCmd::Ending { index: 0 }).unwrap();
        s.advance(4_000 * MS);
        out.push(vol(s));
        s.advance(8_000 * MS);
        out
    };
    let open = |name: &str| {
        let dir = data_dir(name);
        let s = Session::offline(Options { paths: vec![path.clone()], data_dir: Some(dir.clone()), ..Options::default() }).unwrap();
        s.finish_indexing();
        s.advance(MS);
        (s, dir)
    };
    let (plain, dir) = open("cc7-plain-song");
    let want = song(&plain);
    let _ = std::fs::remove_dir_all(dir);

    let (s, dir) = open("cc7-after-song");
    s.send(MixerCmd::SetStylePartVolume { part: 3, volume: 64 }).unwrap();
    s.advance(MS);
    let first = song(&s);
    assert!(!s.state().transport.running, "the Ending has stopped the band");
    let after = vol(&s);
    assert!((0..8).any(|p| p != 3 && after[p] != want[0][p]), "the Ending left its own levels: {after:?}");
    s.send(RegistrationCmd::MemorizeRegist { index: 0 }).unwrap();
    // The player moves another part afterwards; the registration didn't store it.
    s.send(MixerCmd::SetStylePartVolume { part: 0, volume: 30 }).unwrap();
    s.advance(MS);
    s.send(RegistrationCmd::RecallRegist { index: 0 }).unwrap();
    for _ in 0..5 {
        s.advance(MS);
    }
    assert_eq!(vol(&s)[3], 64);
    let again = song(&s);
    for (run, got) in [("first", &first), ("after recall", &again)] {
        for (i, (got, want)) in got.iter().zip(&want).enumerate() {
            for p in 0..8 {
                if p == 3 {
                    assert_eq!(got[p], 64, "{run}, point {i}: the player's part holds its level");
                } else {
                    assert_eq!(got[p], want[p], "{run}, point {i}, part {p}: the style's pattern level");
                }
            }
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}

/// Review r3 N1: a recall waiting for its style's bar line is dropped when the player
/// chooses another style before then: that style keeps its own mixer and tempo.
#[test]
fn style_chosen_after_a_waiting_recall_keeps_its_own_panel() {
    let Some((s, dir)) = session("chosen-after") else { return };
    dress(&s);
    s.send(RegistrationCmd::MemorizeRegist { index: 0 }).unwrap();
    s.send(LibraryCmd::LoadStyle { id: style_id(&s, "SlowWalker") }).unwrap();
    s.advance(MS);
    let own = panel(&s);
    assert!(own.style.contains("SlowWalker"));
    s.send(TransportCmd::StartStop).unwrap();
    past_first_beat(&s);
    s.send(RegistrationCmd::RecallRegist { index: 0 }).unwrap();
    assert!(s.state().registration.pending);
    // Before the bar line the player goes back to SlowWalker (a new load of it).
    s.send(LibraryCmd::LoadStyle { id: style_id(&s, "SlowWalker") }).unwrap();
    s.advance(4_000 * MS);
    let st = s.state();
    assert!(st.transport.running && !st.registration.pending);
    let got = panel(&s);
    assert!(got.style.contains("SlowWalker"), "{}", got.style);
    assert_eq!(got.tempo, own.tempo, "SlowWalker's tempo, not the registration's");
    assert_eq!(got.style_vol[3], own.style_vol[3], "SlowWalker's part 4 level, not the registration's 64");
    assert_eq!(got.style_on, own.style_on);
    let _ = std::fs::remove_dir_all(dir);
}

/// Review r3 N2: the Multi Pad bank is a registration item (group Multi Pad; Data List
/// "Multi Pad File"): recalled, cleared, and left alone when frozen.
#[test]
fn multi_pad_bank_is_registered() {
    let Some(style) = corpus("SlowWalker.T552.sty") else { return };
    let dir = data_dir("multipad");
    std::fs::create_dir_all(dir.join("Pads")).unwrap();
    std::fs::copy(&style, dir.join("SlowWalker.sty")).unwrap();
    for name in ["A", "B"] {
        std::fs::write(dir.join(format!("Pads/{name}.pad")), crate::multipad::synthetic::demo_bank()).unwrap();
    }
    let s = Session::offline(Options { paths: vec![dir.clone()], data_dir: Some(dir.join("data")), ..Options::default() })
        .unwrap();
    s.finish_indexing();
    s.advance(MS);
    let bank = |s: &Session| s.state().multi_pad.bank.as_ref().map(|b| b.name.clone());
    let load = |s: &Session, name: &str| {
        let path = dir.join(format!("Pads/{name}.pad")).display().to_string();
        s.send(MultiPadCmd::LoadMultiPadPath { path }).unwrap();
        s.advance(MS);
    };
    let recall = |s: &Session, index: u8| {
        s.send(RegistrationCmd::RecallRegist { index }).unwrap();
        for _ in 0..3 {
            s.advance(MS);
        }
    };
    load(&s, "A");
    assert_eq!(bank(&s).as_deref(), Some("A"));
    s.send(RegistrationCmd::MemorizeRegist { index: 0 }).unwrap();
    s.send(MultiPadCmd::ClearMultiPad).unwrap();
    s.advance(MS);
    s.send(RegistrationCmd::MemorizeRegist { index: 1 }).unwrap();
    assert!(s.state().registration.buttons[1].groups.has(Group::MultiPad));

    load(&s, "B");
    recall(&s, 0);
    assert_eq!(bank(&s).as_deref(), Some("A"));
    recall(&s, 1);
    assert_eq!(bank(&s), None, "a button memorized with no bank clears it");

    // Frozen, the bank stays.
    s.send(RegistrationCmd::SetFreezeGroup { group: Group::MultiPad, on: true }).unwrap();
    s.send(RegistrationCmd::SetFreeze { on: true }).unwrap();
    load(&s, "B");
    recall(&s, 0);
    assert_eq!(bank(&s).as_deref(), Some("B"));
    let _ = std::fs::remove_dir_all(dir);
}

/// Review r3 B1, old files: a bank saved before `styleMixer.set` existed still loads and
/// recalls with the earlier behaviour (every stored level that differs is set), and a new
/// memory stores which parts the player set.
#[test]
fn style_mixer_without_player_set_recalls_as_before() {
    let Some((s, dir)) = session("mixer-old-file") else { return };
    s.advance(MS);
    let own = panel(&s).style_vol;
    s.send(MixerCmd::SetStylePartVolume { part: 3, volume: 64 }).unwrap();
    s.advance(MS);
    s.send(RegistrationCmd::MemorizeRegist { index: 0 }).unwrap();
    save(&s, "Old");
    let file = dir.join("Registration/Old.regist.json");
    let mut v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    let mixer = &mut v["memories"][0]["sections"]["styleMixer"];
    let set: Vec<bool> = serde_json::from_value(mixer["set"].clone()).expect("a new memory stores `set`");
    assert_eq!(set, [false, false, false, true, false, false, false, false]);
    // As an earlier build wrote it: levels and on/off only.
    mixer.as_object_mut().unwrap().remove("set");
    std::fs::write(&file, v.to_string()).unwrap();
    s.send(RegistrationCmd::SelectRegistBank { path: file.display().to_string() }).unwrap();
    s.send(MixerCmd::SetStylePartVolume { part: 3, volume: 120 }).unwrap();
    s.send(MixerCmd::SetStylePartVolume { part: 0, volume: 30 }).unwrap();
    s.advance(MS);
    s.send(RegistrationCmd::RecallRegist { index: 0 }).unwrap();
    for _ in 0..3 {
        s.advance(MS);
    }
    let msg = s.state().message.clone();
    assert!(msg.as_ref().is_none_or(|m| !m.error), "{msg:?}");
    let got = panel(&s).style_vol;
    assert_eq!(got[3], 64);
    assert_eq!(got[0], own[0], "the stored level that differs is set, as before");
    let _ = std::fs::remove_dir_all(dir);
}

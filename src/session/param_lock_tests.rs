//! Parameter Lock through an offline session: locked groups survive Registration, OTS and
//! Playlist recalls; the lock state is a setup setting (kept across sessions, not in banks).

use crate::api::{ChordCmd, LockItem, OtsCmd, ParamLockCmd, ParamLockState, PlaylistCmd, RegistrationCmd};
use crate::fingering::Fingering;
use crate::session::{Options, Session};
use std::path::{Path, PathBuf};

const MS: u64 = 1_000_000;

fn session(test: &str) -> Option<(Session, PathBuf)> {
    let style = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
    if !style.exists() {
        eprintln!("corpus missing; skipping");
        return None;
    }
    let dir = std::env::temp_dir().join(format!("yahaha-plock-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let s = Session::offline(Options { paths: vec![style], data_dir: Some(dir.clone()), ..Options::default() }).unwrap();
    s.finish_indexing();
    Some((s, dir))
}

/// Split, fingering, Upper, and an item outside every lock group (Keyboard transpose).
fn panel(s: &Session) -> (u8, Fingering, bool, i8) {
    let c = s.state().chord.clone();
    (c.split, c.fingering, c.upper, c.transpose_keyboard)
}

fn lock(s: &Session, item: LockItem, on: bool) {
    s.send(ParamLockCmd::SetParamLock { item, on }).unwrap();
}

fn set_panel(s: &Session, split: u8, fingering: Fingering, upper: bool, transpose: i8) {
    s.send(ChordCmd::SetSplit { note: split }).unwrap();
    s.send(ChordCmd::SetFingering { fingering }).unwrap();
    s.send(ChordCmd::SetUpper { on: upper }).unwrap();
    s.send(ChordCmd::SetTranspose { keyboard: transpose, master: 0 }).unwrap();
    s.advance(MS);
}

#[test]
fn locked_groups_survive_registration_ots_and_playlist_recalls() {
    let Some((s, dir)) = session("recalls") else { return };
    // Button 1 stores split 60, Fingered, Upper, transpose +2; it goes in a playlist.
    set_panel(&s, 60, Fingering::Fingered, true, 2);
    s.send(RegistrationCmd::MemorizeRegist { index: 0 }).unwrap();
    s.send(RegistrationCmd::SaveRegistBank { name: Some("Gig".into()), overwrite: false }).unwrap();
    s.send(PlaylistCmd::AddCurrentBank).unwrap();

    // The player's own panel, locked.
    let mine = (50, Fingering::SingleFinger, false, 0);
    set_panel(&s, mine.0, mine.1, mine.2, mine.3);
    lock(&s, LockItem::SplitPoint, true);
    lock(&s, LockItem::FingeringType, true);
    assert_eq!(s.state().param_locks, ParamLockState { split_point: true, fingering_type: true });

    // Registration: the locked groups stay; the rest (transpose) is recalled.
    s.send(RegistrationCmd::RecallRegist { index: 0 }).unwrap();
    s.advance(10 * MS);
    assert_eq!(panel(&s), (50, Fingering::SingleFinger, false, 2));

    // OTS: sets no lock-group item at all.
    s.send(ChordCmd::SetTranspose { keyboard: 0, master: 0 }).unwrap();
    s.send(OtsCmd::RecallOts { index: 1 }).unwrap();
    s.advance(10 * MS);
    assert_eq!(panel(&s), mine);

    // Playlist: its record recalls the same button, through the same check.
    s.send(PlaylistCmd::StepPlaylist { delta: 1 }).unwrap();
    s.advance(10 * MS);
    assert_eq!(s.state().registration.selected, Some(0), "the record recalled the button");
    assert_eq!(panel(&s), (50, Fingering::SingleFinger, false, 2));

    // The panel still changes them.
    s.send(ChordCmd::SetSplit { note: 55 }).unwrap();
    assert_eq!(s.state().chord.split, 55);

    // Unlocking one group lets the next recall set it, and only it.
    lock(&s, LockItem::SplitPoint, false);
    s.send(RegistrationCmd::RecallRegist { index: 0 }).unwrap();
    s.advance(10 * MS);
    assert_eq!(panel(&s), (60, Fingering::SingleFinger, false, 2));
    lock(&s, LockItem::FingeringType, false);
    s.send(RegistrationCmd::RecallRegist { index: 0 }).unwrap();
    s.advance(10 * MS);
    assert_eq!(panel(&s), (60, Fingering::Fingered, true, 2));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn locks_are_a_setup_setting_not_part_of_a_bank() {
    let Some((s, dir)) = session("setup") else { return };
    lock(&s, LockItem::FingeringType, true);
    s.send(RegistrationCmd::MemorizeRegist { index: 0 }).unwrap();
    s.send(RegistrationCmd::SaveRegistBank { name: Some("A".into()), overwrite: false }).unwrap();
    let bank = std::fs::read_to_string(dir.join("Registration/A.regist.json")).unwrap();
    assert!(!bank.contains("paramLock"), "{bank}");
    // A new bank keeps them.
    s.send(RegistrationCmd::NewRegistBank).unwrap();
    assert!(s.state().param_locks.fingering_type);
    drop(s);
    // So does the next session (the Genos's Setup/Backup), with Sequence On/Off beside it.
    let style = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
    let s = Session::offline(Options { paths: vec![style], data_dir: Some(dir.clone()), ..Options::default() }).unwrap();
    assert_eq!(s.state().param_locks, ParamLockState { split_point: false, fingering_type: true });
    s.send(RegistrationCmd::SetRegistSequenceOn { on: true }).unwrap();
    let setup: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("Registration/setup.json")).unwrap()).unwrap();
    assert_eq!(setup["paramLocks"]["fingeringType"], true, "{setup}");
    assert_eq!(setup["sequenceOn"], true, "{setup}");
    let _ = std::fs::remove_dir_all(dir);
}

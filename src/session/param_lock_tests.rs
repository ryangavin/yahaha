//! Parameter Lock through an offline session: locked groups survive rack and OTS recalls;
//! the lock state is a setup setting (kept across sessions, not in racks).

use crate::api::{ChordCmd, LockItem, OtsCmd, ParamLockCmd, ParamLockState};
use crate::fingering::Fingering;
use crate::session::testing;
use crate::session::{Options, Session};
use std::path::{Path, PathBuf};

const MS: u64 = 1_000_000;

fn session_in(dir: &Path) -> Session {
    let style = testing::style_path();
    let s = Session::offline(Options { paths: vec![style], data_dir: Some(dir.to_path_buf()), ..Options::default() }).unwrap();
    s.finish_indexing();
    s
}

fn session(test: &str) -> (Session, PathBuf) {
    let dir = std::env::temp_dir().join(format!("yahaha-plock-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    (session_in(&dir), dir)
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
fn locked_groups_survive_rack_and_ots_recalls() {
    let (s, dir) = session("recalls");
    // The rack has split 60 and keyboard transpose +2.
    set_panel(&s, 60, Fingering::Fingered, true, 2);
    let rack = s.capture_rack("Gig");

    // The player's own panel, locked.
    let mine = (50, Fingering::SingleFinger, false, 0);
    set_panel(&s, mine.0, mine.1, mine.2, mine.3);
    lock(&s, LockItem::SplitPoint, true);
    lock(&s, LockItem::FingeringType, true);
    assert_eq!(s.state().param_locks, ParamLockState { split_point: true, fingering_type: true });

    // A rack: the locked split stays; the rest (transpose) is applied.
    assert_eq!(s.apply_rack(&rack), Vec::<String>::new());
    assert_eq!(panel(&s), (50, Fingering::SingleFinger, false, 2));

    // OTS: sets no lock-group item at all.
    s.send(ChordCmd::SetTranspose { keyboard: 0, master: 0 }).unwrap();
    s.send(OtsCmd::RecallOts { index: 1 }).unwrap();
    s.advance(10 * MS);
    assert_eq!(panel(&s), mine);

    // The panel still changes them.
    s.send(ChordCmd::SetSplit { note: 55 }).unwrap();
    assert_eq!(s.state().chord.split, 55);

    // Unlocking the split lets the next rack set it.
    lock(&s, LockItem::SplitPoint, false);
    s.apply_rack(&rack);
    assert_eq!(panel(&s), (60, Fingering::SingleFinger, false, 2));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn locks_are_a_setup_setting_kept_across_sessions() {
    let (s, dir) = session("setup");
    lock(&s, LockItem::FingeringType, true);
    drop(s);
    let s = session_in(&dir);
    assert_eq!(s.state().param_locks, ParamLockState { split_point: false, fingering_type: true });
    let saved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("param-locks.json")).unwrap()).unwrap();
    assert_eq!(saved["fingeringType"], true, "{saved}");
    assert!(!dir.join("Registration").exists(), "nothing goes in the old Registration folder");
    let _ = std::fs::remove_dir_all(dir);
}

/// Locks set before racks were in the Registration folder's `setup.json`: they still apply,
/// that file is left as it was, and a change is saved in the new file.
#[test]
fn locks_from_the_old_registration_setup_file_still_apply() {
    let dir = std::env::temp_dir().join(format!("yahaha-plock-old-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("Registration")).unwrap();
    let old = r#"{"sequenceOn":true,"paramLocks":{"splitPoint":true}}"#;
    std::fs::write(dir.join("Registration/setup.json"), old).unwrap();
    let s = session_in(&dir);
    assert_eq!(s.state().param_locks, ParamLockState { split_point: true, fingering_type: false });
    lock(&s, LockItem::SplitPoint, false);
    drop(s);
    assert_eq!(std::fs::read_to_string(dir.join("Registration/setup.json")).unwrap(), old, "the old file is untouched");
    let s = session_in(&dir);
    assert_eq!(s.state().param_locks, ParamLockState::default(), "the new file wins");
    let _ = std::fs::remove_dir_all(dir);
}

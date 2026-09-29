//! The channel strips reach the synth (`pump_strips`), and a Style part's insert 1 across a
//! style change.

use crate::api::{CompPreset, LibraryCmd, PartCompParam, PartEq, SendKind, StripCmd};
use crate::fx::{FxControl, InsertKind, InsertSlot, InsertType, PartComp, SendSlot};
use crate::parts::CHANNEL;
use crate::session::Session;
use crate::session::testing::session;
use std::sync::atomic::Ordering::Relaxed;

const MS: u64 = 1_000_000;

/// The synthetic style's session with an offline synth, so the pump has somewhere to go.
fn with_synth() -> Session {
    let s = session();
    s.offline_audio(None, 48_000).unwrap();
    s
}

/// Read the synth's effect control.
fn read<T>(s: &Session, f: impl FnOnce(&FxControl) -> T) -> T {
    let ctl = s.inner.lock();
    f(&ctl.synth.as_ref().unwrap().control.fx)
}

fn send(s: &Session, c: StripCmd) {
    s.send(c).unwrap();
}

/// Load another copy of the synthetic style (it has no insertion effects): a style change.
fn change_style(s: &Session, test: &str) {
    let other = crate::session::testing::write_style(&std::env::temp_dir().join(format!("yahaha-strips-{test}-{}", std::process::id())));
    s.send(LibraryCmd::LoadStylePath { path: other.display().to_string() }).unwrap();
    s.advance(50 * MS);
}

#[test]
fn a_keyboard_strips_compressor_reaches_the_synth() {
    let s = with_synth();
    let ch = CHANNEL[1] as usize;
    assert_eq!(read(&s, |c| c.strips.comp(ch)), PartComp::default().clamped());
    send(&s, StripCmd::SetStripCompressorOn { strip: 1, on: true });
    send(&s, StripCmd::SetStripCompressorPreset { strip: 1, preset: CompPreset::Punchy });
    send(&s, StripCmd::SetStripCompressorParam { strip: 1, param: PartCompParam::Threshold, value: -40 });
    let mut want = PartComp::of(true, CompPreset::Punchy);
    want.threshold = -40;
    // The atomic doesn't carry the type, only the parameters.
    assert_eq!(read(&s, |c| c.strips.comp(ch)), PartComp { preset: CompPreset::Natural, ..want });
    assert!(!read(&s, |c| c.strips.comp(CHANNEL[0] as usize).on), "only its own channel");
}

#[test]
fn insert_2_reaches_the_synth_and_plays_nothing_when_off() {
    let s = with_synth();
    let ch = CHANNEL[2] as usize;
    let plays = |s: &Session| read(s, |c| c.strips.second_settings(ch, 120.0, false).kind);
    send(&s, StripCmd::SetStripInsertKind { strip: 2, slot: 1, kind: InsertType::Tremolo });
    assert_eq!(plays(&s), InsertKind::None, "a new slot is off");
    send(&s, StripCmd::SetStripInsertOn { strip: 2, slot: 1, on: true });
    send(&s, StripCmd::SetStripInsertSetting { strip: 2, slot: 1, setting: 2, value: 100 });
    assert_eq!(plays(&s), InsertKind::Tremolo);
    let mut want = InsertType::Tremolo.defaults();
    want[2] = 100;
    assert_eq!(read(&s, |c| c.strips.second(ch)), InsertSlot { kind: InsertType::Tremolo, on: true, values: want });
    send(&s, StripCmd::SetStripInsertOn { strip: 2, slot: 1, on: false });
    assert_eq!(plays(&s), InsertKind::None, "off");
}

#[test]
fn a_keyboard_insert_1s_later_settings_reach_the_synth() {
    let s = with_synth();
    let ch = CHANNEL[0] as usize;
    send(&s, StripCmd::SetStripInsertKind { strip: 0, slot: 0, kind: InsertType::Compressor });
    send(&s, StripCmd::SetStripInsertOn { strip: 0, slot: 0, on: true });
    send(&s, StripCmd::SetStripInsertSetting { strip: 0, slot: 0, setting: 0, value: 90 });
    send(&s, StripCmd::SetStripInsertSetting { strip: 0, slot: 0, setting: 1, value: 40 });
    send(&s, StripCmd::SetStripInsertSetting { strip: 0, slot: 0, setting: 3, value: 7 });
    let d = InsertType::Compressor.defaults();
    assert_eq!(read(&s, |c| c.strips.first_rest(ch)), [40, d[2], 7]);
    // Its kind, on/off and amount go through the part's own slot, as before.
    let slot = s.inner.lock().shared.parts.insert(0);
    assert_eq!((slot.kind(), slot.on, slot.amount), (InsertKind::Compressor, true, 90));
}

#[test]
fn sends_4_to_6_and_their_levels_reach_the_synth() {
    let s = with_synth();
    let (kb, style) = (CHANNEL[0] as usize, 9);
    let added = |s: &Session| read(s, |c| std::array::from_fn::<_, 3, _>(|i| c.sends[i].slot().map(|x| x.kind)));
    let levels = |s: &Session, ch: usize| read(s, |c| std::array::from_fn::<_, 3, _>(|i| c.strip_send[ch][i].load(Relaxed)));
    assert_eq!(added(&s), [None, None, None]);
    send(&s, StripCmd::AddSend { kind: SendKind::Phaser });
    send(&s, StripCmd::AddSend { kind: SendKind::Room });
    send(&s, StripCmd::SetStripSend { strip: 0, send: 3, level: 50 });
    send(&s, StripCmd::SetStripSend { strip: 5, send: 4, level: 60 });
    send(&s, StripCmd::SetSendParam { send: 3, param: 1, value: 200 });
    send(&s, StripCmd::SetSendReturn { send: 4, level: 90 });
    assert_eq!(added(&s), [Some(SendKind::Phaser), Some(SendKind::Room), None]);
    assert_eq!((levels(&s, kb), levels(&s, style)), ([50, 0, 0], [0, 60, 0]));
    let mut phaser = SendSlot::of(SendKind::Phaser);
    phaser.params[1] = 200;
    assert_eq!(read(&s, |c| c.sends[0].slot()), Some(phaser));
    assert_eq!(read(&s, |c| c.sends[1].slot().map(|x| x.return_level)), Some(90));

    // Removing send 4: send 5 moves down, with the levels to it.
    send(&s, StripCmd::RemoveSend { send: 3 });
    assert_eq!(added(&s), [Some(SendKind::Room), None, None]);
    assert_eq!((levels(&s, kb), levels(&s, style)), ([0, 0, 0], [60, 0, 0]));
    assert_eq!(read(&s, |c| c.sends[0].slot().map(|x| x.return_level)), Some(90));
}

#[test]
fn a_style_strips_eq_reaches_the_synth() {
    let s = with_synth();
    let eq = PartEq { low_gain: -6, low_freq: 200, high_gain: 30, high_freq: 5_000 };
    send(&s, StripCmd::SetStripEq { strip: 7, eq });
    assert_eq!(read(&s, |c| c.style_eq[3].get()), eq.clamped());
    assert_eq!(read(&s, |c| c.style_eq[2].get()), PartEq::FLAT, "only its own part");
}

/// A Style part's insert 1 kind chosen by the player plays on the old atomics and the
/// strip's, until the next style; insert 2 and the compressor stay.
#[test]
fn a_style_parts_chosen_insert_lasts_until_the_next_style() {
    let s = with_synth();
    let (p, ch) = (2, 10);
    let old = |s: &Session| read(s, |c| c.insert[p].load(Relaxed));
    assert_eq!(old(&s), 0, "the synthetic style has no insert");
    send(&s, StripCmd::SetStripInsertKind { strip: 6, slot: 0, kind: InsertType::Distortion });
    send(&s, StripCmd::SetStripInsertSetting { strip: 6, slot: 0, setting: 1, value: 20 });
    send(&s, StripCmd::SetStripInsertKind { strip: 6, slot: 1, kind: InsertType::Phaser });
    send(&s, StripCmd::SetStripInsertOn { strip: 6, slot: 1, on: true });
    send(&s, StripCmd::SetStripCompressorOn { strip: 6, on: true });
    assert_eq!(old(&s), InsertKind::Distortion as u8);
    assert_eq!(read(&s, |c| c.strips.first_rest(ch))[0], 20);
    assert_eq!(s.state().mixer.style_parts[2].strip.inserts[0].kind, InsertType::Distortion);

    change_style(&s, "chosen-insert");
    assert_eq!(old(&s), 0, "the new style's (none)");
    assert_eq!(s.state().mixer.style_parts[2].strip.inserts[0].kind, InsertType::None);
    assert_eq!(read(&s, |c| c.strips.second(ch).kind), InsertType::Phaser, "insert 2 stays");
    assert!(read(&s, |c| c.strips.comp(ch).on), "the compressor stays");
}

/// A style's own insert on a part shows as that strip's insert 1; a new style with none
/// there empties the slot (it doesn't keep the old style's kind or on/off).
#[test]
fn a_new_style_without_an_insert_empties_insert_1() {
    let s = with_synth();
    s.inner.lock().info.inserts.push(crate::fx::xg::StyleInsert { channel: 11, msb: 0x49, lsb: 0, name: "Test Dist".into(), kind: Some((InsertKind::Distortion, 90)) });
    s.send(StripCmd::SetStripInsertOn { strip: 7, slot: 0, on: true }).unwrap();
    let slot1 = |s: &Session| s.state().mixer.style_parts[3].strip.inserts[0].clone();
    assert_eq!((slot1(&s).kind, slot1(&s).on), (InsertType::Distortion, true));
    assert_eq!(read(&s, |c| c.insert[3].load(Relaxed)), InsertKind::Distortion as u8);

    change_style(&s, "style-insert");
    assert_eq!((slot1(&s).kind, slot1(&s).on), (InsertType::None, false));
    assert_eq!(read(&s, |c| c.insert[3].load(Relaxed)), 0);
}

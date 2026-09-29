//! The channel strips reach the synth (`pump_strips`), and the rack send override.

use crate::api::{CompPreset, LibraryCmd, PartCompParam, PartEq, SendKind, StripCmd};
use crate::fx::{InsertKind, InsertType, PartComp, SEND_NONE, SendSlot, StripControl};
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

/// Read the synth's strip control.
fn read<T>(s: &Session, f: impl FnOnce(&StripControl) -> T) -> T {
    let ctl = s.inner.lock();
    f(&ctl.synth.as_ref().unwrap().control.fx.strips)
}

fn kind(s: &Session, ch: usize, slot: usize) -> u8 {
    read(s, |c| c.insert_kind[ch][slot].load(Relaxed))
}

fn values(s: &Session, ch: usize, slot: usize) -> [u16; 4] {
    read(s, |c| std::array::from_fn(|i| c.insert_values[ch][slot][i].load(Relaxed)))
}

fn send(s: &Session, c: StripCmd) {
    s.send(c).unwrap();
}

/// Load another copy of the synthetic style: a style change.
fn change_style(s: &Session, test: &str) {
    let other = crate::session::testing::write_style(&std::env::temp_dir().join(format!("yahaha-strips-{test}-{}", std::process::id())));
    s.send(LibraryCmd::LoadStylePath { path: other.display().to_string() }).unwrap();
    s.advance(50 * MS);
}

#[test]
fn a_keyboard_strips_compressor_reaches_the_synth() {
    let s = with_synth();
    let ch = CHANNEL[1] as usize;
    assert_eq!(read(&s, |c| c.comp[ch].get()), PartComp::default().clamped());
    send(&s, StripCmd::SetStripCompressorOn { strip: 1, on: true });
    send(&s, StripCmd::SetStripCompressorPreset { strip: 1, preset: CompPreset::Punchy });
    send(&s, StripCmd::SetStripCompressorParam { strip: 1, param: PartCompParam::Threshold, value: -40 });
    let mut want = PartComp::of(true, CompPreset::Punchy);
    want.threshold = -40;
    // The atomic doesn't carry the type, only the parameters.
    assert_eq!(read(&s, |c| c.comp[ch].get()), PartComp { preset: CompPreset::Natural, ..want });
    assert!(!read(&s, |c| c.comp[CHANNEL[0] as usize].get().on), "only its own channel");
}

#[test]
fn insert_2_reaches_the_synth_and_plays_nothing_when_off() {
    let s = with_synth();
    let ch = CHANNEL[2] as usize;
    send(&s, StripCmd::SetStripInsertKind { strip: 2, slot: 1, kind: InsertType::Tremolo });
    assert_eq!(kind(&s, ch, 1), 0, "a new slot is off");
    send(&s, StripCmd::SetStripInsertOn { strip: 2, slot: 1, on: true });
    send(&s, StripCmd::SetStripInsertSetting { strip: 2, slot: 1, setting: 2, value: 100 });
    assert_eq!(kind(&s, ch, 1), InsertKind::Tremolo as u8);
    let mut want = InsertType::Tremolo.defaults();
    want[2] = 100;
    assert_eq!(values(&s, ch, 1), want);
    send(&s, StripCmd::SetStripInsertOn { strip: 2, slot: 1, on: false });
    assert_eq!(kind(&s, ch, 1), 0, "off");
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
    assert_eq!(kind(&s, ch, 0), InsertKind::Compressor as u8);
    let d = InsertType::Compressor.defaults();
    assert_eq!(values(&s, ch, 0), [90, 40, d[2], 7]);
}

#[test]
fn sends_4_to_6_and_their_levels_reach_the_synth() {
    let s = with_synth();
    let (kb, style) = (CHANNEL[0] as usize, 9);
    let index = |k: SendKind| SendKind::ALL.iter().position(|x| *x == k).unwrap() as u8;
    let added = |s: &Session| read(s, |c| std::array::from_fn::<_, 3, _>(|i| c.added_kind[i].load(Relaxed)));
    let levels = |s: &Session, ch: usize| read(s, |c| std::array::from_fn::<_, 3, _>(|i| c.send[ch][i].load(Relaxed)));
    assert_eq!(added(&s), [SEND_NONE; 3]);
    send(&s, StripCmd::AddSend { kind: SendKind::Phaser });
    send(&s, StripCmd::AddSend { kind: SendKind::Room });
    send(&s, StripCmd::SetStripSend { strip: 0, send: 3, level: 50 });
    send(&s, StripCmd::SetStripSend { strip: 5, send: 4, level: 60 });
    send(&s, StripCmd::SetSendParam { send: 3, param: 1, value: 200 });
    send(&s, StripCmd::SetSendReturn { send: 4, level: 90 });
    assert_eq!(added(&s), [index(SendKind::Phaser), index(SendKind::Room), SEND_NONE]);
    assert_eq!((levels(&s, kb), levels(&s, style)), ([50, 0, 0], [0, 60, 0]));
    let mut phaser = SendSlot::of(SendKind::Phaser);
    phaser.params[1] = 200;
    let params = read(&s, |c| std::array::from_fn::<_, 6, _>(|i| c.added_params[0][i].load(Relaxed)));
    assert_eq!(params, phaser.params);
    assert_eq!(read(&s, |c| [c.added_return[0].load(Relaxed), c.added_return[1].load(Relaxed)]), [64, 90]);

    // Removing send 4: send 5 moves down, with the levels to it.
    send(&s, StripCmd::RemoveSend { send: 3 });
    assert_eq!(added(&s), [index(SendKind::Room), SEND_NONE, SEND_NONE]);
    assert_eq!((levels(&s, kb), levels(&s, style)), ([0, 0, 0], [60, 0, 0]));
    assert_eq!(read(&s, |c| c.added_return[0].load(Relaxed)), 90);
}

#[test]
fn a_style_strips_eq_reaches_the_synth() {
    let s = with_synth();
    let eq = PartEq { low_gain: -6, low_freq: 200, high_gain: 30, high_freq: 5_000 };
    send(&s, StripCmd::SetStripEq { strip: 7, eq });
    assert_eq!(read(&s, |c| c.eq[3].get()), eq.clamped());
    assert_eq!(read(&s, |c| c.eq[2].get()), PartEq::FLAT, "only its own part");
}

/// A Style part's insert 1 kind chosen by the player plays on the old atomics and the
/// strip's, until the next style; insert 2 and the compressor stay.
#[test]
fn a_style_parts_chosen_insert_lasts_until_the_next_style() {
    let s = with_synth();
    let (p, ch) = (2, 10);
    let old = |s: &Session| s.inner.lock().synth.as_ref().unwrap().control.fx.insert[p].load(Relaxed);
    assert_eq!((old(&s), kind(&s, ch, 0)), (0, 0), "the synthetic style has no insert");
    send(&s, StripCmd::SetStripInsertKind { strip: 6, slot: 0, kind: InsertType::Distortion });
    send(&s, StripCmd::SetStripInsertSetting { strip: 6, slot: 0, setting: 1, value: 20 });
    send(&s, StripCmd::SetStripInsertKind { strip: 6, slot: 1, kind: InsertType::Phaser });
    send(&s, StripCmd::SetStripInsertOn { strip: 6, slot: 1, on: true });
    send(&s, StripCmd::SetStripCompressorOn { strip: 6, on: true });
    let d = InsertKind::Distortion as u8;
    assert_eq!((old(&s), kind(&s, ch, 0)), (d, d));
    assert_eq!(values(&s, ch, 0)[1], 20);
    assert_eq!(s.state().mixer.style_parts[2].strip.inserts[0].kind, InsertType::Distortion);

    change_style(&s, "chosen-insert");
    assert_eq!((old(&s), kind(&s, ch, 0)), (0, 0), "the new style's (none)");
    assert_eq!(s.state().mixer.style_parts[2].strip.inserts[0].kind, InsertType::None);
    assert_eq!(kind(&s, ch, 1), InsertKind::Phaser as u8, "insert 2 stays");
    assert!(read(&s, |c| c.comp[ch].get().on), "the compressor stays");
}

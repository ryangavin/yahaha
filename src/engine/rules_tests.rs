//! Tests for the fill functions, Half Bar Fill In, Stop Accompaniment modes and Change
//! Behavior (fills.rs, change_rules.rs), on corpus styles.

use super::*;
use crate::sff::SectionId::{Fill, Main};

const MS: u64 = 1_000_000;

#[derive(Default)]
struct Rec(Vec<Vec<u8>>);

impl Sink for Rec {
    fn send(&mut self, m: &[u8]) {
        self.0.push(m.to_vec());
    }
}

fn prepared(name: &str) -> Option<Box<Prepared>> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2").join(name);
    if !p.exists() {
        eprintln!("corpus missing; skipping");
        return None;
    }
    Some(Box::new(Prepared::new(&Style::load(&p).unwrap())))
}

/// SlowWalker (4/4, 75 BPM, Mains A-D with their fills), stopped, Sync Start armed.
fn slow_walker() -> Option<Engine> {
    prepared("SlowWalker.T552.sty").map(Engine::new)
}

/// Process from `from` through `to` in 5 ms steps.
fn play(e: &mut Engine, from: u64, to: u64, sink: &mut impl Sink) {
    let mut now = from;
    while now <= to {
        e.process(now, sink);
        now += 5 * MS;
    }
}

fn c() -> Chord {
    Chord::new(0, 0)
}

/// Started on a C chord at 0 with Auto Fill off; the bar length.
fn started(auto_fill: bool) -> Option<(Engine, u64, Rec)> {
    let mut e = slow_walker()?;
    let mut r = Rec::default();
    let bar = e.ns_at_bar(1);
    if !auto_fill {
        e.button(Button::AutoFill, 0, &mut r);
    }
    e.set_chord(c(), 0, &mut r);
    assert!(e.running);
    Some((e, bar, r))
}

/// The section playing at `t` after playing up to it.
fn at(e: &mut Engine, from: u64, t: u64, r: &mut Rec) -> Option<SectionId> {
    play(e, from, t, r);
    e.snapshot(t).cur
}

#[test]
fn fill_up_and_down_move_through_a_fill() {
    let Some((mut e, bar, mut r)) = started(false) else { return };
    let t = bar + bar / 3;
    play(&mut e, 0, t, &mut r);
    e.button(Button::FillUp, t, &mut r);
    let s = e.snapshot(t);
    assert_eq!((s.main, s.queued), (1, Some(Fill(1))), "Fill Up from A: B's fill, then B");
    assert_eq!(at(&mut e, t, 2 * bar - 10 * MS, &mut r), Some(Fill(1)));
    assert_eq!(at(&mut e, 2 * bar - 5 * MS, 2 * bar + 10 * MS, &mut r), Some(Main(1)));
    // Fill Down: back to A through A's fill.
    let t = 2 * bar + bar / 3;
    play(&mut e, 2 * bar + 15 * MS, t, &mut r);
    e.button(Button::FillDown, t, &mut r);
    assert_eq!(e.snapshot(t).queued, Some(Fill(0)));
    assert_eq!(at(&mut e, t, 3 * bar + 10 * MS, &mut r), Some(Main(0)));
}

#[test]
fn fill_down_at_main_a_and_fill_self_play_the_own_fill() {
    let Some((mut e, bar, mut r)) = started(false) else { return };
    let t = bar / 3;
    play(&mut e, 0, t, &mut r);
    e.button(Button::FillDown, t, &mut r);
    let s = e.snapshot(t);
    assert_eq!((s.main, s.queued), (0, Some(Fill(0))), "no Main left of A: A's own fill");
    assert_eq!(at(&mut e, t, bar + 10 * MS, &mut r), Some(Main(0)));
    let t = bar + bar / 3;
    play(&mut e, bar + 15 * MS, t, &mut r);
    e.button(Button::FillSelf, t, &mut r);
    assert_eq!(e.snapshot(t).queued, Some(Fill(0)));
}

#[test]
fn fill_functions_while_stopped_select_the_main() {
    let Some(mut e) = slow_walker() else { return };
    let mut r = Rec::default();
    e.button(Button::FillUp, 0, &mut r);
    e.button(Button::FillUp, 0, &mut r);
    assert_eq!(e.snapshot(0).main, 2);
    e.button(Button::FillDown, 0, &mut r);
    e.button(Button::FillSelf, 0, &mut r);
    let s = e.snapshot(0);
    assert_eq!((s.main, s.running, s.queued), (1, false, None));
    assert!(r.0.is_empty(), "nothing plays");
}

#[test]
fn half_bar_fill_starts_the_fill_mid_bar_on_a_first_beat_change() {
    let Some((mut e, bar, mut r)) = started(false) else { return };
    e.button(Button::SetHalfBarFill(true), 0, &mut r);
    assert!(e.snapshot(0).half_bar_fill);
    // Main B pressed on beat 1 of bar 3, Auto Fill off: B's fill from beat 3, B at bar 4.
    let t = 2 * bar + 20 * MS;
    play(&mut e, 0, t, &mut r);
    e.button(Button::Main(1), t, &mut r);
    assert_eq!(e.snapshot(t).queued, Some(Fill(1)), "a fill, though Auto Fill is off");
    assert_eq!(at(&mut e, t, 2 * bar + bar / 2 - 10 * MS, &mut r), Some(Main(0)), "Main A plays to the half bar");
    assert_eq!(at(&mut e, 2 * bar + bar / 2 - 5 * MS, 2 * bar + bar / 2 + 10 * MS, &mut r), Some(Fill(1)));
    // The fill plays its second half: its position is beat 3 of its bar.
    assert_eq!(e.snapshot(2 * bar + bar / 2 + 10 * MS).beat, 2);
    assert_eq!(at(&mut e, 2 * bar + bar / 2 + 15 * MS, 3 * bar + 10 * MS, &mut r), Some(Main(1)));
    // Pressed on beat 2 it does not apply: Main A at the next bar, no fill (Auto Fill off).
    let t = 3 * bar + bar / 4 + 20 * MS;
    play(&mut e, 3 * bar + 15 * MS, t, &mut r);
    e.button(Button::Main(0), t, &mut r);
    assert_eq!(e.snapshot(t).queued, Some(Main(0)));
    assert_eq!(at(&mut e, t, 4 * bar - 10 * MS, &mut r), Some(Main(1)));
    assert_eq!(at(&mut e, 4 * bar - 5 * MS, 4 * bar + 10 * MS, &mut r), Some(Main(0)));
}

#[test]
fn half_bar_fill_toggle_and_off() {
    let Some((mut e, bar, mut r)) = started(false) else { return };
    e.button(Button::HalfBarFill, 0, &mut r);
    e.button(Button::HalfBarFill, 0, &mut r);
    assert!(!e.snapshot(0).half_bar_fill);
    let t = bar + 20 * MS;
    play(&mut e, 0, t, &mut r);
    e.button(Button::Main(1), t, &mut r);
    assert_eq!(e.snapshot(t).queued, Some(Main(1)), "off: the Main at the next bar");
}

/// Program changes sent on `ch` (0-based).
fn programs(r: &Rec, ch: u8) -> Vec<u8> {
    r.0.iter().filter(|m| m[0] == 0xC0 | ch).map(|m| m[1]).collect()
}

fn notes_on(r: &Rec, ch: u8) -> usize {
    r.0.iter().filter(|m| m[0] == 0x90 | ch && m[2] > 0).count()
}

#[test]
fn stop_acmp_modes() {
    let Some(mut e) = slow_walker() else { return };
    let mut r = Rec::default();
    e.button(Button::SyncStart, 0, &mut r); // off: Stop Accompaniment
    // Off: silent.
    e.set_chord(c(), 0, &mut r);
    assert!(!e.running);
    assert_eq!(notes_on(&r, BASS_CH) + notes_on(&r, PAD_CH_T), 0);
    // The toggle turns Style on first.
    e.button(Button::StopAcmp, 0, &mut r);
    assert_eq!(e.snapshot(0).stop_acmp_mode, StopAcmp::Style);
    e.set_chord(Chord::new(7, 0), 0, &mut r);
    assert!(notes_on(&r, BASS_CH) > 0 && notes_on(&r, PAD_CH_T) > 0);
    assert!(programs(&r, BASS_CH).is_empty(), "the style's own voices");
    // Fixed: the fixed voices go out before the notes.
    r.0.clear();
    e.button(Button::SetStopAcmp(StopAcmp::Fixed), 0, &mut r);
    e.set_chord(Chord::new(5, 0), 0, &mut r);
    assert_eq!((programs(&r, BASS_CH), programs(&r, PAD_CH_T)), (vec![FIXED_BASS_PROGRAM], vec![FIXED_PAD_PROGRAM]));
    let pc = r.0.iter().position(|m| m[0] == 0xC0 | BASS_CH).unwrap();
    let on = r.0.iter().position(|m| m[0] == 0x90 | BASS_CH && m[2] > 0).unwrap();
    assert!(pc < on);
    // A second chord sends no program change again.
    r.0.clear();
    e.set_chord(Chord::new(0, 0), 0, &mut r);
    assert!(programs(&r, BASS_CH).is_empty());
    // Back to Style: the style's voices return.
    r.0.clear();
    e.button(Button::SetStopAcmp(StopAcmp::Style), 0, &mut r);
    let back = programs(&r, BASS_CH);
    assert_eq!(back.len(), 1);
    assert_ne!(back[0], FIXED_BASS_PROGRAM);
    // Toggle: off, then back to Style (the last mode on).
    e.button(Button::StopAcmp, 0, &mut r);
    assert_eq!(e.snapshot(0).stop_acmp_mode, StopAcmp::Off);
    assert!(!e.snapshot(0).stop_acmp);
    e.button(Button::StopAcmp, 0, &mut r);
    assert_eq!(e.snapshot(0).stop_acmp_mode, StopAcmp::Style);
}

const PAD_CH_T: u8 = 13;

#[test]
fn sync_start_on_arms_only_when_stopped() {
    let Some(mut e) = slow_walker() else { return };
    let mut r = Rec::default();
    e.button(Button::SyncStart, 0, &mut r);
    assert!(!e.snapshot(0).sync_armed);
    e.sync_start_on();
    assert!(e.snapshot(0).sync_armed);
    e.set_chord(c(), 0, &mut r);
    assert!(e.running);
    e.sync_start_on();
    assert!(e.running && !e.snapshot(0).sync_armed, "playing: nothing changes");
}

fn with_rules(e: &mut Engine, tempo: ChangeRule, parts: ChangeRule, section: Option<u8>) {
    e.set_change_rules(ChangeRules { tempo, parts, section });
}

#[test]
fn change_behavior_while_stopped() {
    use ChangeRule::*;
    let Some(mut e) = slow_walker() else { return };
    let Some(austin) = prepared("AustinCityBlues.S930.STY") else { return };
    let new_bpm = austin.bpm;
    let mut r = Rec::default();
    assert_eq!(e.change_rules(), ChangeRules::default());
    e.button(Button::TempoUp, 0, &mut r);
    let bpm = e.snapshot(0).bpm;
    e.button(Button::TogglePart(3), 0, &mut r);
    // Lock: the tempo and the part states stay.
    with_rules(&mut e, Lock, Lock, None);
    let old = e.load(austin, 0, &mut r);
    let s = e.snapshot(0);
    assert_eq!((s.bpm, s.parts), (bpm, 0xFF & !(1 << 3)));
    // Hold (stopped): the new style's tempo, every part on; Section Set C.
    with_rules(&mut e, Hold, Hold, Some(2));
    let back = e.load(old, 0, &mut r);
    let s = e.snapshot(0);
    assert_eq!((s.bpm, s.parts, s.main), (75.0, 0xFF, 2));
    // Reset, Section Set Off: the main stays.
    e.button(Button::TogglePart(0), 0, &mut r);
    with_rules(&mut e, Reset, Reset, None);
    e.load(back, 0, &mut r);
    let s = e.snapshot(0);
    assert_eq!((s.bpm, s.parts, s.main), (new_bpm, 0xFF, 2));
}

#[test]
fn change_behavior_while_playing() {
    use ChangeRule::*;
    let Some((mut e, bar, mut r)) = started(true) else { return };
    let (Some(a1), Some(a2)) = (prepared("AustinCityBlues.S930.STY"), prepared("AustinCityBlues.S930.STY")) else { return };
    let new_bpm = a1.bpm;
    e.button(Button::TogglePart(3), 0, &mut r);
    // Hold while playing: tempo and parts stay at the bar line.
    let t = bar / 3;
    play(&mut e, 0, t, &mut r);
    e.change_style(a1, t, &mut r);
    play(&mut e, t, bar + 10 * MS, &mut r);
    assert!(!e.style_pending());
    let s = e.snapshot(bar + 10 * MS);
    assert_eq!((s.bpm, s.parts), (75.0, 0xFF & !(1 << 3)));
    // Reset while playing: the new style's tempo and every part on at the bar line.
    with_rules(&mut e, Reset, Reset, Some(3));
    let t = bar + bar / 3;
    play(&mut e, bar + 15 * MS, t, &mut r);
    e.change_style(a2, t, &mut r);
    let t2 = 2 * bar + 10 * MS;
    play(&mut e, t, t2, &mut r);
    let s = e.snapshot(t2);
    assert_eq!((s.bpm, s.parts), (new_bpm, 0xFF));
    assert_eq!(s.cur, Some(Main(0)), "Section Set applies only to a style chosen while stopped");
    while e.take_retired().is_some() {}
}

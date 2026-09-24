use super::*;
use crate::multipad::file::{Layout, Pad};
use crate::sff::TimedEv;
use crate::theory::CANCEL;

#[derive(Default)]
struct Rec(Vec<Vec<u8>>);
impl Sink for Rec {
    fn send(&mut self, msg: &[u8]) {
        self.0.push(msg.to_vec());
    }
}
impl Rec {
    fn take(&mut self) -> Vec<Vec<u8>> {
        std::mem::take(&mut self.0)
    }
    /// Note-on keys, in order.
    fn ons(&self) -> Vec<u8> {
        self.0.iter().filter(|m| m[0] & 0xF0 == 0x90).map(|m| m[1]).collect()
    }
}

const PPQ: u16 = 480;
const BAR: u64 = 1920;

fn ev(tick: u32, ev: Ev) -> TimedEv {
    TimedEv { tick, ev }
}
fn on(tick: u32, key: u8) -> TimedEv {
    ev(tick, Ev::NoteOn { ch: 0, key, vel: 100 })
}
fn off(tick: u32, key: u8) -> TimedEv {
    ev(tick, Ev::NoteOff { ch: 0, key })
}

fn pad(events: Vec<TimedEv>, len: u32, repeat: bool, chord_match: bool) -> Pad {
    Pad { name: "t".into(), channel: 0, events, len, repeat: Some(repeat), chord_match: Some(chord_match), rule: None, image: None }
}

fn bank(pads: Vec<Pad>) -> PadBank {
    let mut b = PadBank {
        name: String::new(),
        ppq: PPQ,
        tempo_us: 500_000,
        timesig: (4, 4),
        layout: Layout::ChannelsAscending,
        pads: Default::default(),
        other_chunks: vec![],
        texts: vec![],
    };
    for (i, p) in pads.into_iter().enumerate() {
        b.pads[i] = Some(p);
    }
    b
}

fn player(pads: Vec<Pad>) -> MultiPadPlayer {
    MultiPadPlayer::new(&bank(pads), PPQ as u32)
}

/// C E G, one beat each, over one bar.
fn arp() -> Vec<TimedEv> {
    vec![on(0, 60), off(480, 60), on(480, 64), off(960, 64), on(960, 67), off(1440, 67)]
}

#[test]
fn start_tick_waits_for_the_next_measure_only_while_playing() {
    assert_eq!(start_tick(1234, Clock::Stopped), 1234);
    let run = Clock::Running { bar_origin: 100, ticks_per_bar: BAR };
    assert_eq!(start_tick(100, run), 100);
    assert_eq!(start_tick(101, run), 100 + BAR);
    assert_eq!(start_tick(100 + BAR, run), 100 + BAR);
    assert_eq!(start_tick(100 + BAR + 1, run), 100 + 2 * BAR);
    // Before the style's first bar: its first bar.
    assert_eq!(start_tick(10, run), 100);
    assert_eq!(start_tick(10, Clock::Running { bar_origin: 0, ticks_per_bar: 0 }), 10);
}

#[test]
fn sync_triggers_follow_acmp() {
    assert!(sync_fires(SyncTrigger::Key, false));
    assert!(!sync_fires(SyncTrigger::Key, true));
    assert!(sync_fires(SyncTrigger::Chord, true));
    assert!(!sync_fires(SyncTrigger::Chord, false));
    assert!(sync_fires(SyncTrigger::StyleStart, true) && sync_fires(SyncTrigger::StyleStart, false));
}

#[test]
fn one_shot_plays_once_and_ends() {
    let mut p = player(vec![pad(arp(), 1920, false, false)]);
    let mut s = Rec::default();
    assert_eq!(p.state(0), PadState::Ready);
    assert_eq!(p.state(1), PadState::Empty);
    assert_eq!(p.state(9), PadState::Empty);
    assert!(!p.trigger(1, 0));
    assert!(!p.trigger(9, 0));
    assert!(p.trigger(0, 1000));
    assert_eq!(p.state(0), PadState::Queued);
    p.process(0..1000, None, &mut s);
    assert!(s.0.is_empty());
    p.process(1000..1001, None, &mut s);
    assert_eq!(s.take(), vec![vec![0x94, 60, 100]]);
    assert_eq!(p.state(0), PadState::Playing);
    p.process(1001..100_000, None, &mut s);
    assert_eq!(s.ons(), vec![64, 67]);
    assert_eq!(s.0.last().unwrap(), &vec![0x84, 67, 0]);
    assert_eq!(p.state(0), PadState::Ready);
    assert!(!p.is_active());
}

#[test]
fn repeat_loops_on_the_pass_length_until_stopped() {
    let mut p = player(vec![pad(arp(), 1920, true, false)]);
    let mut s = Rec::default();
    p.trigger(0, 0);
    p.process(0..3 * BAR, None, &mut s);
    assert_eq!(s.ons(), vec![60, 64, 67, 60, 64, 67, 60, 64, 67]);
    s.take();
    // Called one tick at a time, the second pass starts exactly on the bar.
    let mut p2 = player(vec![pad(arp(), 1920, true, false)]);
    p2.trigger(0, 0);
    let mut pass2 = None;
    for t in 0..2 * BAR {
        p2.process(t..t + 1, None, &mut s);
        if s.ons().len() == 4 && pass2.is_none() {
            pass2 = Some(t);
        }
    }
    assert_eq!(pass2, Some(BAR));
    p2.stop(0, &mut s);
    assert_eq!(p2.state(0), PadState::Ready);
    s.take();
    p2.process(0..100 * BAR, None, &mut s);
    assert!(s.0.is_empty());
}

#[test]
fn a_late_call_skips_whole_repeat_passes() {
    let mut p = player(vec![pad(arp(), 1920, true, false)]);
    let mut s = Rec::default();
    p.trigger(0, 0);
    p.process(0..1, None, &mut s);
    s.take();
    // A range far in the future plays the first pass's backlog, then the pass it is in.
    p.process(1000 * BAR..1000 * BAR + 1, None, &mut s);
    assert_eq!(s.ons(), vec![64, 67, 60]);
}

#[test]
fn pressing_a_playing_pad_restarts_it_and_cuts_its_notes() {
    let mut p = player(vec![pad(vec![on(0, 60), off(1900, 60)], 1920, false, false)]);
    let mut s = Rec::default();
    p.trigger(0, 0);
    p.process(0..500, None, &mut s);
    p.trigger(0, 960);
    assert_eq!(p.state(0), PadState::Playing);
    p.process(500..961, None, &mut s);
    assert_eq!(s.take(), vec![vec![0x94, 60, 100], vec![0x84, 60, 0], vec![0x94, 60, 100]]);
    p.process(961..961 + 1900, None, &mut s);
    assert_eq!(s.take(), vec![vec![0x84, 60, 0]]);
}

#[test]
fn chord_match_transposes_with_the_default_rule() {
    let mut p = player(vec![pad(arp(), 1920, false, true)]);
    let mut s = Rec::default();
    p.trigger(0, 0);
    // F: root +5 (F is at or below the high key F#, so up).
    p.process(0..BAR, Some(Chord::new(5, 0)), &mut s);
    assert_eq!(s.ons(), vec![65, 69, 72]);
    let offs: Vec<u8> = s.0.iter().filter(|m| m[0] == 0x84).map(|m| m[1]).collect();
    assert_eq!(offs, vec![65, 69, 72]);
    s.take();
    // A minor: the root moves down (A is above the high key), E becomes C (the m3).
    p.trigger(0, BAR);
    p.process(BAR..2 * BAR, Some(Chord::new(9, 8)), &mut s);
    assert_eq!(s.ons(), vec![57, 60, 64]);
}

#[test]
fn chord_match_off_no_chord_or_cancel_play_as_written() {
    for (cm, chord) in [(false, Some(Chord::new(5, 0))), (true, None), (true, Some(Chord::new(5, CANCEL)))] {
        let mut p = player(vec![pad(arp(), 1920, false, cm)]);
        let mut s = Rec::default();
        p.trigger(0, 0);
        p.process(0..BAR, chord, &mut s);
        assert_eq!(s.ons(), vec![60, 64, 67], "{cm} {chord:?}");
    }
}

#[test]
fn a_held_note_keeps_its_pitch_across_a_chord_change() {
    let mut p = player(vec![pad(vec![on(0, 60), off(960, 60)], 1920, false, true)]);
    let mut s = Rec::default();
    p.trigger(0, 0);
    p.process(0..10, Some(Chord::new(2, 0)), &mut s);
    p.process(10..BAR, Some(Chord::new(5, 0)), &mut s);
    assert_eq!(s.take(), vec![vec![0x94, 62, 100], vec![0x84, 62, 0]]);
}

#[test]
fn a_casm_rule_from_the_file_drives_chord_match() {
    // Root Fixed / Chord: C-E-G over F voices to the nearest F-A-C (C3-F3-A3).
    let mut rule = default_rule(0);
    for z in rule.zones.iter_mut() {
        z.ntr = Ntr::RootFixed;
        z.ntt = Ntt::Chord;
    }
    let chord = vec![on(0, 60), on(0, 64), on(0, 67), off(480, 60), off(480, 64), off(480, 67)];
    let mut pd = pad(chord, 480, false, true);
    pd.rule = Some(rule);
    let mut p = player(vec![pd]);
    let mut s = Rec::default();
    p.trigger(0, 0);
    p.process(0..BAR, Some(Chord::new(5, 0)), &mut s);
    let mut ons = s.ons();
    ons.sort();
    assert_eq!(ons, vec![60, 65, 69]);
}

#[test]
fn pads_interleave_in_time_on_their_own_channels() {
    let a = pad(vec![on(0, 60), off(480, 60), on(960, 62), off(1440, 62)], 1920, false, false);
    let b = pad(vec![on(480, 70), off(960, 70)], 1920, false, false);
    let mut p = player(vec![a, b]);
    let mut s = Rec::default();
    p.trigger(0, 0);
    p.trigger(1, 0);
    p.process(0..BAR, None, &mut s);
    let got: Vec<(u8, u8)> = s.0.iter().map(|m| (m[0], m[1])).collect();
    assert_eq!(got, vec![(0x94, 60), (0x84, 60), (0x95, 70), (0x85, 70), (0x94, 62), (0x84, 62)]);
}

#[test]
fn host_ticks_rescale_the_pad() {
    // A 480-ppq pad on a 96-ppq host clock: a bar is 384 host ticks.
    let mut p = MultiPadPlayer::new(&bank(vec![pad(arp(), 1920, true, false)]), 96);
    assert_eq!(p.len_ticks(0), 384);
    let mut s = Rec::default();
    p.trigger(0, 0);
    p.process(0..384, None, &mut s);
    assert_eq!(s.ons(), vec![60, 64, 67]);
    p.process(384..385, None, &mut s);
    assert_eq!(s.ons(), vec![60, 64, 67, 60]);
}

#[test]
fn controllers_are_rechannelled_and_sysex_passes_through() {
    let evs = vec![
        ev(0, Ev::Pc { ch: 0, prog: 7 }),
        ev(0, Ev::Cc { ch: 0, cc: 7, val: 90 }),
        ev(0, Ev::Bend { ch: 0, val: 0x2000 }),
        ev(0, Ev::Sysex(vec![0xF0, 0x7E, 0xF7])),
        on(0, 60),
        off(10, 60),
    ];
    let mut p = player(vec![pad(evs, 480, false, false)]);
    p.set_out_channel(0, 10);
    assert_eq!(p.out_channel(0), Some(10));
    let mut s = Rec::default();
    p.trigger(0, 0);
    p.process(0..BAR, None, &mut s);
    assert_eq!(
        s.0,
        vec![
            vec![0xCA, 7],
            vec![0xBA, 7, 90],
            vec![0xEA, 0x00, 0x40],
            vec![0xF0, 0x7E, 0xF7],
            vec![0x9A, 60, 100],
            vec![0x8A, 60, 0]
        ]
    );
}

#[test]
fn synchro_start_arms_and_fires() {
    let mut p = player(vec![pad(arp(), 1920, false, false), pad(arp(), 1920, false, false)]);
    let mut s = Rec::default();
    assert!(!p.arm(3)); // empty
    assert!(p.arm(0));
    assert!(p.arm(1));
    assert!(!p.arm(1)); // the same gesture again disarms
    assert!(p.arm(1));
    assert_eq!(p.state(0), PadState::Armed);
    assert!(p.any_armed() && p.armed(1));
    p.process(0..BAR, None, &mut s);
    assert!(s.0.is_empty(), "armed pads wait");
    let at = start_tick(BAR + 5, Clock::Running { bar_origin: 0, ticks_per_bar: BAR });
    assert_eq!(p.fire_sync(at), 2);
    assert!(!p.any_armed());
    assert_eq!(p.state(1), PadState::Queued);
    p.process(BAR..2 * BAR + 1, None, &mut s);
    assert_eq!(s.ons(), vec![60, 60]);
    // STOP cancels standby as well.
    p.arm(0);
    p.stop_all(&mut s);
    assert!(!p.any_armed());
    assert_eq!(p.fire_sync(0), 0);
    p.arm(1);
    p.disarm_all();
    assert!(!p.any_armed());
}

#[test]
fn synchro_stop_stops_only_repeating_pads() {
    let mut p = player(vec![pad(arp(), 1920, true, false), pad(arp(), 1920, false, false)]);
    let mut s = Rec::default();
    p.trigger(0, 0);
    p.trigger(1, 0);
    p.process(0..100, None, &mut s);
    s.take();
    p.stop_repeating(&mut s);
    assert_eq!(s.take(), vec![vec![0x84, 60, 0]]);
    assert_eq!((p.state(0), p.state(1)), (PadState::Ready, PadState::Playing));
}

#[test]
fn panel_overrides_of_repeat_and_chord_match() {
    let mut p = player(vec![pad(arp(), 1920, false, false)]);
    p.set_repeat(0, true);
    p.set_chord_match(0, true);
    assert!(p.repeat(0) && p.chord_match(0));
    let mut s = Rec::default();
    p.trigger(0, 0);
    p.process(0..2 * BAR, Some(Chord::new(2, 0)), &mut s);
    assert_eq!(s.ons(), vec![62, 66, 69, 62, 66, 69]);
}

#[test]
fn the_sounding_table_overflows_by_dropping_notes() {
    let mut evs: Vec<TimedEv> = (0..100).map(|k| on(k, k as u8)).collect();
    evs.push(off(1000, 0));
    let mut p = player(vec![pad(evs, 1920, false, false)]);
    let mut s = Rec::default();
    p.trigger(0, 0);
    p.process(0..BAR + 1, None, &mut s);
    assert_eq!(s.ons().len(), MAX_SOUNDING);
    // Every on that went out gets its off (at the pass end).
    let offs = s.0.iter().filter(|m| m[0] == 0x84).count();
    assert_eq!(offs, MAX_SOUNDING);
}

#[test]
fn a_parsed_bank_plays() {
    use crate::multipad::file::{build::*, parse};
    let mut f = header(0, 1, 96);
    f.extend(track(&[(0, vec![0x93, 60, 90]), (48, vec![0x83, 60, 0])]));
    let b = parse(&f).unwrap();
    let mut p = MultiPadPlayer::new(&b, 96);
    let mut s = Rec::default();
    p.trigger(0, 0);
    p.process(0..96, None, &mut s);
    assert_eq!(s.0, vec![vec![0x94, 60, 90], vec![0x84, 60, 0]]);
}

#[test]
fn rerouting_a_sounding_pad_ends_its_notes_on_the_old_channel() {
    let mut p = player(vec![pad(vec![on(0, 60), on(0, 64), off(1900, 60)], 1920, false, false)]);
    let mut s = Rec::default();
    p.trigger(0, 0);
    p.process(0..10, None, &mut s);
    p.set_out_channel(0, 9);
    p.process(10..1901, None, &mut s);
    p.stop(0, &mut s);
    assert_eq!(s.take(), vec![vec![0x94, 60, 100], vec![0x94, 64, 100], vec![0x84, 60, 0], vec![0x84, 64, 0]]);
}

fn cc(tick: u32, cc: u8, val: u8) -> TimedEv {
    ev(tick, Ev::Cc { ch: 0, cc, val })
}

/// A pad stopped with its pedal down and bent (STOP + pad, [STOP], a bank swap) leaves its
/// channel re-centred and the pedal up; the same for a one-shot that ends that way.
#[test]
fn a_stopped_pad_releases_its_pedal_and_bend() {
    let bar = BAR as u32;
    let held = vec![cc(0, 64, 127), ev(0, Ev::Bend { ch: 0, val: 0x3000 }), on(0, 60), off(bar * 2, 60)];
    let mut p = player(vec![pad(vec![on(0, 48), off(100, 48)], bar, true, false), pad(held, bar * 4, false, false)]);
    let mut r = Rec::default();
    p.trigger(1, 0);
    p.process(0..1, None, &mut r);
    r.take();
    p.stop(1, &mut r);
    let ch = DEFAULT_OUT_CH[1];
    let reset = vec![vec![0xE0 | ch, 0x00, 0x40], vec![0xB0 | ch, 1, 0], vec![0xB0 | ch, 64, 0]];
    let mut want = vec![vec![0x80 | ch, 60, 0]];
    want.extend(reset.clone());
    assert_eq!(r.take(), want);
    // Pad 1 never moved them: its stop sends only its note-off.
    p.trigger(0, 0);
    p.process(0..1, None, &mut r);
    r.take();
    p.stop(0, &mut r);
    assert_eq!(r.take(), vec![vec![0x80 | DEFAULT_OUT_CH[0], 48, 0]]);
    // A one-shot that ends with the pedal down: released at its end.
    p.trigger(1, 0);
    p.process(0..BAR * 4 + 1, None, &mut r);
    let m = r.take();
    assert_eq!(&m[m.len() - 3..], &reset[..]);
    // stop_all (a bank swap, [STOP]) too.
    p.trigger(1, 0);
    p.process(0..1, None, &mut r);
    r.take();
    p.stop_all(&mut r);
    assert!(r.take().ends_with(&reset));
}

/// Master transpose moves a pad's notes (after Chord Match); the note-off goes to the key
/// that sounded even if Master changes meanwhile. A drum-kit pad (bank MSB 127) stays put.
#[test]
fn master_transpose_moves_pads_but_not_kits() {
    let bar = BAR as u32;
    let kit = vec![cc(0, 0, 127), on(0, 42), off(100, 42)];
    let mut p = player(vec![pad(vec![on(0, 60), off(bar, 60)], bar, false, true), pad(kit, bar, false, false)]);
    assert!(!p.is_kit(0) && p.is_kit(1));
    let mut r = Rec::default();
    p.set_master(2);
    p.trigger(0, 0);
    p.trigger(1, 0);
    // Chord Match to F first (C -> F), then Master +2: G.
    p.process(0..1, Some(crate::parse_chord("F").unwrap()), &mut r);
    assert_eq!(r.ons(), [67, 42]);
    r.take();
    p.set_master(-3);
    p.process(1..BAR + 1, None, &mut r);
    let offs: Vec<Vec<u8>> = r.take().into_iter().filter(|m| m[0] & 0xF0 == 0x80).collect();
    assert!(offs.contains(&vec![0x80 | DEFAULT_OUT_CH[0], 67, 0]), "{offs:?}");
    assert!(offs.contains(&vec![0x80 | DEFAULT_OUT_CH[1], 42, 0]), "{offs:?}");
}

#[test]
fn retimed_presses_start_at_the_new_tick() {
    let mut p = player(vec![pad(vec![on(0, 60), off(100, 60)], BAR as u32, false, false)]);
    let mut r = Rec::default();
    p.trigger(0, BAR);
    p.retime_pending(10);
    p.process(0..11, None, &mut r);
    assert_eq!(r.ons(), [60]);
    assert_eq!(p.state(0), PadState::Playing);
}

//! The engine thread must not allocate or free: a style preview (its whole run, and its
//! end) and a style change at the next bar line run there. A counting global allocator (in
//! this test binary only) checks `EngineLoop::step` through both.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use yahaha::engine::{Button, Engine, PadCmd, Prepared, StyleControls, Transpose, PAD_PPQ};
use yahaha::live::{self, Audition, Cmd, EngineLoop, FxConfig, FxKey, FxMode, Out, PadBank, Shared};
use yahaha::multipad::{file::parse, synthetic, MultiPadPlayer};
use yahaha::rt::{PacketSink, Target};
use yahaha::sff::Style;

struct Counting;
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static FREES: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    /// Count on this thread: the test's own (the test harness allocates on its threads
    /// while another test runs).
    static COUNT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn counting() -> bool {
    COUNT.try_with(|c| c.get()).unwrap_or(false)
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if counting() {
            ALLOCS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        if counting() {
            FREES.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.dealloc(p, l) }
    }
}

#[global_allocator]
static A: Counting = Counting;

/// The counters are global: one test at a time, each counting on its own thread.
static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn count_here() -> std::sync::MutexGuard<'static, ()> {
    let g = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    COUNT.with(|c| c.set(true));
    g
}

fn prep(name: &str) -> Option<Box<Prepared>> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2").join(name);
    p.exists().then(|| Box::new(Prepared::new(&Style::load(&p).unwrap())))
}

#[test]
fn preview_and_next_bar_style_change_do_not_allocate() {
    let (Some(a), Some(b), Some(c)) = (prep("SlowWalker.T552.sty"), prep("TickingAway.T162.sty"), prep("CoolRevibed.T552.sty")) else {
        eprintln!("corpus missing; skipping");
        return;
    };
    let _one = count_here();
    let bar = (60e9 / a.bpm * (a.tpb as f64 / a.ppq as f64)) as u64;
    let preview_bar = (60e9 / c.bpm * (c.tpb as f64 / c.ppq as f64)) as u64;
    let shared = Arc::new(Shared::new(54));
    let mut ch = live::channels(Out::new(PacketSink::new(Target::Null), None));
    let mut l = EngineLoop::new(Engine::new(a), ch.io, shared.clone());
    let chords = ["C", "Am", "F", "G7"].map(|s| yahaha::parse_chord(s).unwrap());
    let preview = Box::new(Audition { engine: Engine::new(c), id: 7, chords });
    let chord = yahaha::parse_chord("F").unwrap();
    // Warm up: the first step sizes nothing lazily later on.
    l.step(1);

    let (allocs, frees) = (ALLOCS.load(Ordering::Relaxed), FREES.load(Ordering::Relaxed));
    let mut now = 1_000;
    ch.audition_tx.push(preview).ok().unwrap();
    let end = now + 4 * preview_bar + 50_000_000;
    while now < end {
        l.step(now);
        now = l.next_deadline().unwrap_or(now + 5_000_000).max(now + 1);
    }
    // The band: a Sync Start chord, then a style change mid-bar, played past the bar line.
    shared.chord.store(chord.pack(1), Ordering::Release);
    l.step(now);
    let t0 = now;
    while now < t0 + bar / 2 {
        now = l.next_deadline().unwrap_or(now + 5_000_000).max(now + 1);
        l.step(now);
    }
    ch.style_tx.push(b).ok().unwrap();
    // What a Registration recall sends the engine: tempo, Style part levels and mutes.
    ch.ui_tx.push(Cmd::Button(Button::SetTempo(96))).ok().unwrap();
    ch.ui_tx.push(Cmd::StyleVolume(3, 64)).ok().unwrap();
    ch.ui_tx.push(Cmd::Button(Button::TogglePart(5))).ok().unwrap();
    let controls = StyleControls { main: Some(1), intro: None, sync_start: None, sync_stop: Some(true), stop_acmp: Some(true), stop_acmp_mode: None, parts: Some(0b1011_1111), volumes: Some([90, 80, 100, 64, 100, 100, 100, 70]), player_set: Some(0b1000_0001), retrigger: Some(true) };
    // Part 4 (moved above) goes back to the style: the player_set mask leaves it out.
    ch.ui_tx.push(Cmd::StyleControls(controls)).ok().unwrap();
    while now < t0 + 2 * bar {
        now = l.next_deadline().unwrap_or(now + 5_000_000).max(now + 1);
        l.step(now);
    }
    // Controllers: a bend range, a part switched on under a held pedal, Fill Up, KeysOff
    // and Panic (the pedal reset).
    shared.controllers.set_bend_range(0, 9);
    shared.controllers.toggle_switch(yahaha::controllers::SUSTAIN);
    shared.parts.toggle(1);
    l.step(now + 1);
    ch.ui_tx.push(Cmd::Button(Button::Fill(1))).ok().unwrap();
    ch.ui_tx.push(Cmd::KeysOff).ok().unwrap();
    l.step(now + 1);
    ch.ui_tx.push(Cmd::Button(Button::StartStop)).ok().unwrap();
    ch.ui_tx.push(Cmd::Panic).ok().unwrap();
    l.step(now + 1);
    assert_eq!(ALLOCS.load(Ordering::Relaxed) - allocs, 0, "allocations on the engine thread");
    assert_eq!(FREES.load(Ordering::Relaxed) - frees, 0, "frees on the engine thread");
    let snaps: Vec<_> = std::iter::from_fn(|| ch.snap_rx.pop().ok()).collect();
    assert!(snaps.iter().any(|s| s.audition.is_some_and(|a| a.bar == 4)), "the preview played its 4 bars");
    assert!(snaps.iter().any(|s| s.running && s.style_pending), "the style change waited for the bar line");
    assert!(snaps.iter().any(|s| s.running && (s.bpm - 96.0).abs() < 1e-9), "the recalled tempo took");
    // The preview and the old style came back to be freed off it.
    assert!(ch.old_audition_rx.pop().is_ok());
    assert!(ch.old_rx.pop().is_ok());
}

/// Keyboard Harmony's Echo category, the arpeggio and Strum run on the engine thread
/// (`live::KbdFx`): keys, type switches, the band starting, a style change to another
/// resolution and PANIC, all without allocating.
#[test]
fn harmony_and_arpeggio_do_not_allocate() {
    let (Some(a), Some(b)) = (prep("SlowWalker.T552.sty"), prep("TickingAway.T162.sty")) else {
        eprintln!("corpus missing; skipping");
        return;
    };
    let _one = count_here();
    let shared = Arc::new(Shared::new(54));
    for p in 0..3 {
        shared.parts.on[p].store(true, Ordering::Relaxed);
    }
    let (synth, mut heard) = rtrb::RingBuffer::new(1 << 16);
    let mut ch = live::channels(Out::new(PacketSink::new(Target::Null), Some(synth)));
    let mut l = EngineLoop::new(Engine::new(a), ch.io, shared.clone());
    let arp = FxConfig { on: true, mode: FxMode::Arpeggio, hold: true, pattern: 3, ..FxConfig::default() }.pack();
    let mut trill = FxConfig { on: true, ..FxConfig::default() };
    trill.harmony.ty = yahaha::harmony::HarmonyType::Trill;
    let trill = trill.pack();
    let chord = yahaha::parse_chord("C").unwrap();
    l.step(1);

    let (allocs, frees) = (ALLOCS.load(Ordering::Relaxed), FREES.load(Ordering::Relaxed));
    let mut now = 1_000;
    let mut ons = [0usize; 4];
    let mut play = |l: &mut EngineLoop, now: &mut u64, ns: u64| {
        let end = *now + ns;
        while *now < end {
            l.step(*now);
            *now = l.next_deadline().unwrap_or(*now + 5_000_000).clamp(*now + 1, end);
            while let Ok(m) = heard.pop() {
                if m[0] & 0xF0 == 0x90 && m[2] > 0 && m[0] & 0x0F < 4 {
                    ons[(m[0] & 3) as usize] += 1;
                }
            }
        }
    };
    let press = |tx: &mut rtrb::Producer<FxKey>, k: u8, on: bool| {
        let (w, b) = ((k / 64) as usize, 1u64 << (k % 64));
        if on {
            shared.fx_held[w].fetch_or(b, Ordering::Release);
            tx.push(FxKey::On { key: k, vel: 100 }).ok().unwrap();
        } else {
            shared.fx_held[w].fetch_and(!b, Ordering::Release);
            tx.push(FxKey::Off { key: k }).ok().unwrap();
        }
    };
    // The arpeggio with the band stopped, then started, then a style with another PPQ.
    shared.kbd_fx.store(arp, Ordering::Release);
    for k in [60, 64, 67] {
        press(&mut ch.fx_tx, k, true);
    }
    play(&mut l, &mut now, 1_000_000_000);
    shared.chord.store(chord.pack(1), Ordering::Release);
    play(&mut l, &mut now, 1_000_000_000);
    ch.style_tx.push(b).ok().unwrap();
    play(&mut l, &mut now, 4_000_000_000);
    for k in [60, 64, 67] {
        press(&mut ch.fx_tx, k, false);
    }
    play(&mut l, &mut now, 500_000_000);
    // Trill, then Strum notes, then PANIC.
    shared.kbd_fx.store(trill, Ordering::Release);
    press(&mut ch.fx_tx, 72, true);
    press(&mut ch.fx_tx, 76, true);
    play(&mut l, &mut now, 1_000_000_000);
    press(&mut ch.fx_tx, 72, false);
    press(&mut ch.fx_tx, 76, false);
    shared.fx_held[1].fetch_or(1 << (72 - 64), Ordering::Release);
    ch.fx_tx.push(FxKey::Strum { melody: 72, ch: 0, note: 67, vel: 90, delay_ms: 15 }).ok().unwrap();
    play(&mut l, &mut now, 100_000_000);
    ch.fx_tx.push(FxKey::StrumOff { melody: 72 }).ok().unwrap();
    ch.ui_tx.push(Cmd::Panic).ok().unwrap();
    play(&mut l, &mut now, 100_000_000);
    assert_eq!(ALLOCS.load(Ordering::Relaxed) - allocs, 0, "allocations on the engine thread");
    assert_eq!(FREES.load(Ordering::Relaxed) - frees, 0, "frees on the engine thread");
    assert!(ch.old_rx.pop().is_ok(), "the style change happened");
    assert!(ons.iter().sum::<usize>() > 50, "the arpeggio and the trill played: {ons:?}");
}

/// Chord settling (engine/settle.rs) under the chord-settle window: a Sync Start chord,
/// rolled chords, a chord and a transpose in one wake, Stop Accompaniment chords while
/// stopped. Notes held back and started at the settle, all on the engine thread, with
/// Chord Match pads playing (their notes wait for the settle too).
#[test]
fn chord_settling_does_not_allocate() {
    let Some(a) = prep("SlowWalker.T552.sty") else {
        eprintln!("corpus missing; skipping");
        return;
    };
    let _one = count_here();
    let bar = (60e9 / a.bpm * (a.tpb as f64 / a.ppq as f64)) as u64;
    let shared = Arc::new(Shared::new(54));
    let mut ch = live::channels(Out::new(PacketSink::new(Target::Null), None));
    let mut l = EngineLoop::new(Engine::new(a), ch.io, shared.clone());
    let pads = Box::new(MultiPadPlayer::new(&parse(&synthetic::demo_bank()).unwrap(), PAD_PPQ));
    l.step(1);

    let (allocs, frees) = (ALLOCS.load(Ordering::Relaxed), FREES.load(Ordering::Relaxed));
    let mut now = 1_000;
    ch.ui_tx.push(Cmd::ChordSettle(10)).ok().unwrap();
    ch.pad_tx.push(PadBank { player: Some(pads), tag: 1 }).ok().unwrap();
    // Bass Riff (Repeat, Chord Match) and the Shaker Loop play throughout.
    ch.ui_tx.push(Cmd::MultiPad(PadCmd::Trigger(2))).ok().unwrap();
    ch.ui_tx.push(Cmd::MultiPad(PadCmd::Trigger(0))).ok().unwrap();
    l.step(now);
    let chord = |s: &str| yahaha::parse_chord(s).unwrap();
    let mut generation = 1;
    let mut play = |c: &str, at: u64, l: &mut EngineLoop| {
        shared.chord.store(chord(c).pack(generation), Ordering::Release);
        generation += 1;
        l.step(at);
    };
    // Sync Start, then a rolled chord every half bar, 3 ms apart, some with a transpose.
    play("C", now, &mut l);
    let run = |l: &mut EngineLoop, now: &mut u64, to: u64| {
        while *now < to {
            *now = l.next_deadline().unwrap_or(*now + 5_000_000).max(*now + 1).min(to);
            l.step(*now);
        }
    };
    for (i, (c1, c2)) in [("F", "F7"), ("G", "G7"), ("Am", "Am7"), ("D", "Dm")].into_iter().enumerate() {
        run(&mut l, &mut now, (i as u64 + 1) * bar / 2 - 2_000_000);
        play(c1, now, &mut l);
        if i % 2 == 0 {
            ch.ui_tx.push(Cmd::Transpose(Transpose::new(i as i8 - 1, 0))).ok().unwrap();
        }
        let to = now + 3_000_000;
        run(&mut l, &mut now, to);
        play(c2, now, &mut l);
    }
    run(&mut l, &mut now, 3 * bar);
    // Stopped, with Stop Accompaniment: a rolled chord settles too.
    ch.ui_tx.push(Cmd::Button(Button::StartStop)).ok().unwrap();
    ch.ui_tx.push(Cmd::Button(Button::SyncStart)).ok().unwrap();
    ch.ui_tx.push(Cmd::Button(Button::StopAcmp)).ok().unwrap();
    l.step(now + 1);
    now += 1;
    play("E", now, &mut l);
    let to = now + 3_000_000;
        run(&mut l, &mut now, to);
    play("E7", now, &mut l);
    let to = now + 50_000_000;
        run(&mut l, &mut now, to);
    assert_eq!(ALLOCS.load(Ordering::Relaxed) - allocs, 0, "allocations on the engine thread");
    assert_eq!(FREES.load(Ordering::Relaxed) - frees, 0, "frees on the engine thread");
    let snaps: Vec<_> = std::iter::from_fn(|| ch.snap_rx.pop().ok()).collect();
    assert!(snaps.iter().any(|s| s.running && s.played.is_some_and(|c| c.name() == "Dm")), "the band followed the rolls");
}

/// Run the engine loop as its thread does, waking at each deadline, until `until`.
fn run(l: &mut EngineLoop, now: &mut u64, until: u64) {
    while *now < until {
        *now = l.next_deadline().unwrap_or(*now + 5_000_000).max(*now + 1);
        l.step(*now);
    }
}

/// The Chord Looper (record, loop, a memory at the bar line), the metronome's clicks,
/// solos and Style Track Mute run on the engine thread too.
#[test]
fn looper_metronome_and_solo_do_not_allocate() {
    let _one = count_here();
    use yahaha::engine::LoopState;
    use yahaha::looper::{ChordSeq, LoopEvent};
    let Some(a) = prep("SlowWalker.T552.sty") else {
        eprintln!("corpus missing; skipping");
        return;
    };
    let bar = (60e9 / a.bpm * (a.tpb as f64 / a.ppq as f64)) as u64;
    let shared = Arc::new(Shared::new(54));
    let (synth_tx, mut synth_rx) = rtrb::RingBuffer::<[u8; 3]>::new(1 << 16);
    let mut ch = live::channels(Out::new(PacketSink::new(Target::Null), Some(synth_tx)));
    let mut l = EngineLoop::new(Engine::new(a), ch.io, shared.clone());
    let chord = |s: &str| yahaha::parse_chord(s).unwrap();
    let memory = ChordSeq::from_events(
        2,
        &[LoopEvent { bar: 0, at: 0, chord: chord("E") }, LoopEvent { bar: 1, at: 960, chord: chord("A") }],
    );
    l.step(1);

    let (allocs, frees) = (ALLOCS.load(Ordering::Relaxed), FREES.load(Ordering::Relaxed));
    let mut now = 1_000;
    ch.ui_tx.push(Cmd::Metronome { on: true, bell: true }).ok().unwrap();
    ch.ui_tx.push(Cmd::Looper(true)).ok().unwrap();
    l.step(now);
    // Stopped, REC arms Sync Start: this chord starts the band and the recording.
    shared.chord.store(chord("C").pack(1), Ordering::Release);
    l.step(now);
    let t0 = now;
    run(&mut l, &mut now, t0 + bar + bar / 2);
    shared.chord.store(chord("F").pack(2), Ordering::Release);
    run(&mut l, &mut now, t0 + 2 * bar - bar / 4);
    ch.ui_tx.push(Cmd::Looper(false)).ok().unwrap();
    ch.ui_tx.push(Cmd::StyleSolo(Some(2))).ok().unwrap();
    run(&mut l, &mut now, t0 + 4 * bar + bar / 2);
    ch.looper_tx.push(memory).ok().unwrap();
    ch.ui_tx.push(Cmd::StyleSolo(None)).ok().unwrap();
    ch.ui_tx.push(Cmd::StyleParts(0b0000_1010)).ok().unwrap();
    run(&mut l, &mut now, t0 + 7 * bar);
    ch.ui_tx.push(Cmd::Button(Button::StartStop)).ok().unwrap();
    l.step(now + 1);
    run(&mut l, &mut now, t0 + 9 * bar);
    assert_eq!(ALLOCS.load(Ordering::Relaxed) - allocs, 0, "allocations on the engine thread");
    assert_eq!(FREES.load(Ordering::Relaxed) - frees, 0, "frees on the engine thread");

    let snaps: Vec<_> = std::iter::from_fn(|| ch.snap_rx.pop().ok()).collect();
    assert!(snaps.iter().any(|s| s.looper.state == LoopState::Recording));
    assert!(snaps.iter().any(|s| s.looper.state == LoopState::Looping && s.style_solo == Some(2)));
    assert!(snaps.iter().any(|s| s.looper.state == LoopState::Looping && s.played == Some(chord("A"))), "the memory took over");
    let rec = ch.recorded_rx.pop().expect("the recording came back");
    assert_eq!(rec.bars(), 2);
    let clicks = std::iter::from_fn(|| synth_rx.pop().ok()).filter(|m| m[0] == yahaha::click::CLICK).count();
    assert!(clicks >= 9 * 4 - 2, "{clicks} clicks");
}

/// Fill Up / Down / Self, Half Bar Fill In, the Stop Accompaniment modes (Fixed voices and
/// back), Change Behavior rules, an OTS Sync Start and a Reset-tempo style change at the bar
/// line all run on the engine thread without allocating or freeing.
#[test]
fn fills_stop_acmp_and_change_rules_do_not_allocate() {
    use yahaha::engine::{ChangeRule, ChangeRules, StopAcmp};
    let _one = count_here();
    let (Some(a), Some(b)) = (prep("SlowWalker.T552.sty"), prep("TickingAway.T162.sty")) else {
        eprintln!("corpus missing; skipping");
        return;
    };
    let bar = (60e9 / a.bpm * (a.tpb as f64 / a.ppq as f64)) as u64;
    let shared = Arc::new(Shared::new(54));
    let mut ch = live::channels(Out::new(PacketSink::new(Target::Null), None));
    let mut l = EngineLoop::new(Engine::new(a), ch.io, shared.clone());
    l.step(1);
    let (allocs, frees) = (ALLOCS.load(Ordering::Relaxed), FREES.load(Ordering::Relaxed));
    let mut now = 1_000;
    let mut cmd = |c: Cmd, now: &mut u64, l: &mut EngineLoop| {
        ch.ui_tx.push(c).ok().unwrap();
        *now += 1;
        l.step(*now);
    };
    let run_to = |to: u64, now: &mut u64, l: &mut EngineLoop| {
        while *now < to {
            *now = l.next_deadline().unwrap_or(*now + 5_000_000).max(*now + 1);
            l.step(*now);
        }
    };
    let chord = |s: &str, g: u16| yahaha::parse_chord(s).unwrap().pack(g);
    // Stopped, Sync Start off: Stop Accompaniment in Fixed, then Style, then Off.
    cmd(Cmd::Button(Button::SyncStart), &mut now, &mut l);
    cmd(Cmd::Button(Button::SetStopAcmp(StopAcmp::Fixed)), &mut now, &mut l);
    for (i, c) in ["C", "F", "G7"].iter().enumerate() {
        shared.chord.store(chord(c, 1 + i as u16), Ordering::Release);
        now += 1_000_000;
        l.step(now);
    }
    cmd(Cmd::Button(Button::SetStopAcmp(StopAcmp::Style)), &mut now, &mut l);
    cmd(Cmd::Button(Button::StopAcmp), &mut now, &mut l);
    cmd(Cmd::ChangeRules(ChangeRules { tempo: ChangeRule::Reset, parts: ChangeRule::Reset, section: Some(1) }), &mut now, &mut l);
    cmd(Cmd::Button(Button::SetHalfBarFill(true)), &mut now, &mut l);
    // An OTS recall arms Sync Start; the next chord starts the band.
    cmd(Cmd::SyncStartOn, &mut now, &mut l);
    shared.chord.store(chord("C", 9), Ordering::Release);
    now += 1;
    l.step(now);
    let t0 = now;
    // Fill Up on beat 1 (a half-bar fill), Fill Down mid-bar, Fill Self, then a style change.
    run_to(t0 + bar + 10_000_000, &mut now, &mut l);
    cmd(Cmd::Button(Button::FillUp), &mut now, &mut l);
    run_to(t0 + 2 * bar + bar / 3, &mut now, &mut l);
    cmd(Cmd::Button(Button::FillDown), &mut now, &mut l);
    run_to(t0 + 3 * bar + bar / 3, &mut now, &mut l);
    cmd(Cmd::Button(Button::FillSelf), &mut now, &mut l);
    ch.style_tx.push(b).ok().unwrap();
    run_to(t0 + 5 * bar, &mut now, &mut l);
    cmd(Cmd::Button(Button::StartStop), &mut now, &mut l);
    assert_eq!(ALLOCS.load(Ordering::Relaxed) - allocs, 0, "allocations on the engine thread");
    assert_eq!(FREES.load(Ordering::Relaxed) - frees, 0, "frees on the engine thread");
    let snaps: Vec<_> = std::iter::from_fn(|| ch.snap_rx.pop().ok()).collect();
    assert!(snaps.iter().any(|s| s.half_bar_fill), "Half Bar Fill on");
    assert!(snaps.iter().any(|s| s.cur == Some(yahaha::sff::SectionId::Fill(1))), "Fill Up played B's fill");
    assert!(ch.old_rx.pop().is_ok(), "the old style came back");
}

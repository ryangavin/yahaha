//! The engine thread must not allocate or free: a style preview (its whole run, and its
//! end) and a style change at the next bar line run there. A counting global allocator (in
//! this test binary only) checks `EngineLoop::step` through both.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use yahaha::engine::{Button, Engine, Prepared};
use yahaha::live::{self, Audition, Cmd, EngineLoop, Out, Shared};
use yahaha::rt::{PacketSink, Target};
use yahaha::sff::Style;

struct Counting;
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static FREES: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        FREES.fetch_add(1, Ordering::Relaxed);
        unsafe { System.dealloc(p, l) }
    }
}

#[global_allocator]
static A: Counting = Counting;

/// The counters are global: the tests take turns.
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn prep(name: &str) -> Option<Box<Prepared>> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2").join(name);
    p.exists().then(|| Box::new(Prepared::new(&Style::load(&p).unwrap())))
}

#[test]
fn preview_and_next_bar_style_change_do_not_allocate() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (Some(a), Some(b), Some(c)) = (prep("SlowWalker.T552.sty"), prep("TickingAway.T162.sty"), prep("CoolRevibed.T552.sty")) else {
        eprintln!("corpus missing; skipping");
        return;
    };
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
    // The preview and the old style came back to be freed off it.
    assert!(ch.old_audition_rx.pop().is_ok());
    assert!(ch.old_rx.pop().is_ok());
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
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
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

//! The engine thread must not allocate or free: a style preview (its whole run, and its
//! end) and a style change at the next bar line run there. A counting global allocator (in
//! this test binary only) checks `EngineLoop::step` through both.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use yahaha::engine::{Button, Engine, Prepared, StyleControls};
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
    ch.ui_tx.push(Cmd::SetTempo(96.0)).ok().unwrap();
    ch.ui_tx.push(Cmd::StyleVolume(3, 64)).ok().unwrap();
    ch.ui_tx.push(Cmd::Button(Button::TogglePart(5))).ok().unwrap();
    let controls = StyleControls { main: Some(1), intro: None, sync_start: None, sync_stop: Some(true), stop_acmp: Some(true), parts: Some(0b1011_1111) };
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

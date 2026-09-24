//! The chart player runs on the engine thread: a plan arriving (and the one it replaces
//! going back out), chart chords on every beat, section changes with fills, an Intro, the
//! player's override and the Ending must not allocate or free there. A counting global
//! allocator (in this test binary only) checks `EngineLoop::step` through a whole song.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use yahaha::engine::{Button, ChartPlan, ChartSettings, Engine, Prepared};
use yahaha::ireal::{expand, parse_chart};
use yahaha::live::{self, Cmd, EngineLoop, Out, Shared};
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

#[test]
fn a_chart_plays_without_allocating() {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
    if !p.exists() {
        eprintln!("corpus missing; skipping");
        return;
    }
    let style = Box::new(Prepared::new(&Style::load(&p).unwrap()));
    let bar = (60e9 / style.bpm * (style.tpb as f64 / style.ppq as f64)) as u64;
    let shared = Arc::new(Shared::new(54));
    let mut ch = live::channels(Out::new(PacketSink::new(Target::Null), None));
    let mut l = EngineLoop::new(Engine::new(style), ch.io, shared.clone());
    // A synthetic chart: sections A and B (a fill between), chords on beats 1 and 3, N.C., and
    // a 6/8 bar whose eighths fall between the quarter lines.
    let chart = "*A[C^7 |D-7 G7 |E-7 A7 |D-7 G7 ]*B[F^7 |n |T68E-7,A7,D-7,G7,C,F|T44D-7 G7 Z";
    let plan = |tag| Box::new(ChartPlan::from_bars(&expand(&parse_chart(chart), 2), tag, None));
    let (first, second) = (plan(1), plan(2));
    // A new song mid-play (it starts from its first bar at the next bar line).
    let mut third = plan(3);
    third.fresh = true;
    let settings = ChartSettings { on: true, intro: Some(0), ending: Some(0), loop_range: None };
    let override_chord = yahaha::parse_chord("Ab7").unwrap();
    l.step(1);
    // Snapshots are collected as they come (the ring holds 256), into room made up front.
    let mut snaps = Vec::with_capacity(100_000);

    let (allocs, frees) = (ALLOCS.load(Ordering::Relaxed), FREES.load(Ordering::Relaxed));
    let mut now = 1_000;
    ch.chart_tx.push(first).ok().unwrap();
    ch.ui_tx.push(Cmd::Chart(settings)).ok().unwrap();
    ch.ui_tx.push(Cmd::Button(Button::StartStop)).ok().unwrap();
    l.step(now);
    let mut run = |l: &mut EngineLoop, now: &mut u64, until: u64| {
        while *now < until {
            *now = l.next_deadline().unwrap_or(*now + 5_000_000).max(*now + 1);
            l.step(*now);
            while let Ok(s) = ch.snap_rx.pop() {
                snaps.push(s);
            }
        }
    };
    run(&mut l, &mut now, 5 * bar + bar / 2);
    // The player's chord mid-bar, then a new plan (the old one goes back out).
    shared.chord.store(override_chord.pack(1), Ordering::Release);
    l.step(now);
    ch.chart_tx.push(second).ok().unwrap();
    // New settings mid-bar take back the chart's queued change and queue it again.
    run(&mut l, &mut now, 7 * bar + bar / 2);
    ch.ui_tx.push(Cmd::Chart(ChartSettings { loop_range: Some((0, 4)), ..settings })).ok().unwrap();
    l.step(now);
    ch.ui_tx.push(Cmd::Chart(settings)).ok().unwrap();
    ch.chart_tx.push(third).ok().unwrap();
    l.step(now);
    run(&mut l, &mut now, 60 * bar);
    assert_eq!(ALLOCS.load(Ordering::Relaxed) - allocs, 0, "allocations on the engine thread");
    assert_eq!(FREES.load(Ordering::Relaxed) - frees, 0, "frees on the engine thread");

    assert!(snaps.iter().any(|s| s.chart_override), "the player took over");
    assert!(snaps.iter().any(|s| s.chart_tag == 2 && s.chart_bar.is_some()), "the second plan played");
    assert!(snaps.iter().any(|s| s.chart_tag == 3 && s.chart_bar == Some(15)), "the new song played from its top to its last bar");
    assert!(!snaps.last().unwrap().running, "the Ending stopped the band");
    assert!(ch.old_chart_rx.pop().is_ok(), "the first plan came back to be freed");
    assert!(ch.old_chart_rx.pop().is_ok(), "the second plan came back to be freed");
}

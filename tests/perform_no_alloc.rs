//! The engine thread must not allocate or free while it fades, retriggers, resets a
//! section, slows an ending down or times the Synchro Stop Window. A counting global
//! allocator (in this test binary only) checks `EngineLoop::step` through all of them.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use yahaha::engine::{Button, Engine, FadeState, MainTiming, Prepared, StyleSettings};
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
fn fades_retrigger_reset_and_ritardando_do_not_allocate() {
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
    let chords = ["C", "F", "G7", "Am"].map(|s| yahaha::parse_chord(s).unwrap());
    let settings = StyleSettings { main_timing: MainTiming::Immediate, fade_in_ms: 300, fade_out_ms: 300, fade_hold_ms: 100, sync_stop_window_ms: 200, retrigger_rate: 16, ..StyleSettings::default() };
    l.step(1);

    let (allocs, frees) = (ALLOCS.load(Ordering::Relaxed), FREES.load(Ordering::Relaxed));
    let mut now = 1_000;
    // What the snapshots showed (read as they come: the ring holds 256).
    let mut seen = Seen::default();
    let mut run = |l: &mut EngineLoop, snaps: &mut rtrb::Consumer<yahaha::engine::Snapshot>, now: &mut u64, dur: u64| {
        let until = *now + dur;
        while *now < until {
            *now = l.next_deadline().unwrap_or(*now + 5_000_000).clamp(*now + 1, *now + 5_000_000);
            l.step(*now);
            while let Ok(s) = snaps.pop() {
                seen.fade_in |= s.fade == FadeState::FadingIn;
                seen.retrigger |= s.retrigger;
                seen.rit |= s.ritardando;
                seen.hold |= s.fade == FadeState::Holding;
                seen.last = Some((s.running, s.fade));
            }
        }
    };
    let snaps = &mut ch.snap_rx;
    ch.ui_tx.push(Cmd::StyleSettings(settings)).ok().unwrap();
    ch.ui_tx.push(Cmd::Button(Button::Fade)).ok().unwrap();
    ch.ui_tx.push(Cmd::Button(Button::SyncStop)).ok().unwrap();
    ch.ui_tx.push(Cmd::Button(Button::Retrigger)).ok().unwrap();
    l.step(now);
    // A Sync Start chord: the fade in, the Synchro Stop Window timing the hold.
    shared.chord.store(chords[0].pack(1), Ordering::Release);
    l.step(now);
    run(&mut l, snaps, &mut now, bar);
    // Chords retrigger the head; a Section Reset; the Main changes at the next beat.
    for (i, c) in chords.iter().enumerate().skip(1) {
        shared.chord.store(c.pack(1 + i as u16), Ordering::Release);
        l.step(now);
        run(&mut l, snaps, &mut now, bar / 3);
    }
    // The length goes to a whole note, then, 3 beats on, to a 32nd: the head catches up
    // to the loop playing now.
    ch.ui_tx.push(Cmd::StyleSettings(StyleSettings { retrigger_rate: 1, ..settings })).ok().unwrap();
    l.step(now);
    run(&mut l, snaps, &mut now, bar * 3 / 4);
    ch.ui_tx.push(Cmd::StyleSettings(StyleSettings { retrigger_rate: 32, ..settings })).ok().unwrap();
    l.step(now);
    run(&mut l, snaps, &mut now, bar / 4);
    ch.ui_tx.push(Cmd::StyleSettings(settings)).ok().unwrap();
    ch.ui_tx.push(Cmd::Button(Button::SectionReset)).ok().unwrap();
    ch.ui_tx.push(Cmd::Button(Button::Retrigger)).ok().unwrap();
    ch.ui_tx.push(Cmd::Button(Button::Main(1))).ok().unwrap();
    l.step(now);
    run(&mut l, snaps, &mut now, bar);
    // The ending, pressed again: ritardando to the stop.
    ch.ui_tx.push(Cmd::Button(Button::Ending(0))).ok().unwrap();
    l.step(now);
    run(&mut l, snaps, &mut now, bar + bar / 4);
    ch.ui_tx.push(Cmd::Button(Button::Ending(0))).ok().unwrap();
    l.step(now);
    run(&mut l, snaps, &mut now, 6 * bar);
    // Again, then a fade out to the stop and its hold.
    ch.ui_tx.push(Cmd::Button(Button::StartStop)).ok().unwrap();
    l.step(now);
    run(&mut l, snaps, &mut now, bar);
    ch.ui_tx.push(Cmd::Button(Button::Fade)).ok().unwrap();
    l.step(now);
    run(&mut l, snaps, &mut now, 500_000_000);
    assert_eq!(ALLOCS.load(Ordering::Relaxed) - allocs, 0, "allocations on the engine thread");
    assert_eq!(FREES.load(Ordering::Relaxed) - frees, 0, "frees on the engine thread");
    assert!(seen.fade_in, "faded in");
    assert!(seen.retrigger, "retriggered");
    assert!(seen.rit, "slowed down");
    assert!(seen.hold, "faded out and held");
    assert_eq!(seen.last, Some((false, FadeState::Off)));
}

#[derive(Default)]
struct Seen {
    fade_in: bool,
    retrigger: bool,
    rit: bool,
    hold: bool,
    last: Option<(bool, FadeState)>,
}

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

/// The counters are global: the tests take turns, so one never counts the other's allocations.
fn serial() -> std::sync::MutexGuard<'static, ()> {
    static ONE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    ONE.lock().unwrap_or_else(|e| e.into_inner())
}

fn prep(name: &str) -> Option<Box<Prepared>> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2").join(name);
    p.exists().then(|| Box::new(Prepared::new(&Style::load(&p).unwrap())))
}

#[test]
fn preview_and_next_bar_style_change_do_not_allocate() {
    let _one = serial();
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
    ch.ui_tx.push(Cmd::Button(Button::StartStop)).ok().unwrap();
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

/// Fill Up / Down / Self, Half Bar Fill In, the Stop Accompaniment modes (Fixed voices and
/// back), Change Behavior rules, an OTS Sync Start and a Reset-tempo style change at the bar
/// line all run on the engine thread without allocating or freeing.
#[test]
fn fills_stop_acmp_and_change_rules_do_not_allocate() {
    use yahaha::engine::{ChangeRule, ChangeRules, StopAcmp};
    let _one = serial();
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

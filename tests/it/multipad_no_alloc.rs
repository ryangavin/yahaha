//! Multi Pads on the engine thread must not allocate or free: a bank swapped in and out,
//! pads pressed, armed, stopped, played across a style change to another resolution and a
//! tempo change, and Synchro Start / Stop. The crate's counting allocator (`alloc_count`)
//! checks `EngineLoop::step` through all of it.

use crate::alloc_count::{count_here, counts};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use yahaha::engine::{Button, Engine, PadCmd, Prepared, SynchroStop, Transpose, PAD_PPQ};
use yahaha::live::{self, Cmd, EngineLoop, Out, PadBank, Shared};
use yahaha::multipad::{file::parse, synthetic, MultiPadPlayer, PadState};
use yahaha::rt::{PacketSink, Target};
use yahaha::sff::Style;

fn prep(name: &str) -> Option<Box<Prepared>> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2").join(name);
    p.exists().then(|| Box::new(Prepared::new(&Style::load(&p).unwrap())))
}

fn player() -> Box<MultiPadPlayer> {
    Box::new(MultiPadPlayer::new(&parse(&synthetic::demo_bank()).unwrap(), PAD_PPQ))
}

#[test]
fn multi_pads_do_not_allocate_on_the_engine_thread() {
    let (Some(a), Some(b)) = (prep("SlowWalker.T552.sty"), prep("TickingAway.T162.sty")) else {
        eprintln!("corpus missing; skipping");
        return;
    };
    assert_ne!(a.ppq, b.ppq, "the style change must change the resolution");
    let bar = (60e9 / a.bpm * (a.tpb as f64 / a.ppq as f64)) as u64;
    let shared = Arc::new(Shared::new(54));
    let mut ch = live::channels(Out::new(PacketSink::new(Target::Null), None));
    let mut l = EngineLoop::new(Engine::new(a), ch.io, shared.clone());
    let (p1, p2) = (player(), player());
    let chord = yahaha::parse_chord("F").unwrap();
    l.step(1);

    let on = count_here();
    let (allocs, frees) = counts();
    let mut now = 1_000;
    let run = |l: &mut EngineLoop, now: &mut u64, until: u64| {
        while *now < until {
            *now = l.next_deadline().unwrap_or(*now + 5_000_000).max(*now + 1).min(until);
            l.step(*now);
        }
    };
    ch.pad_tx.push(PadBank { player: Some(p1), tag: 1 }).ok().unwrap();
    l.step(now);
    // Stopped: pads start at once; arm two for a Sync Start chord.
    for c in [PadCmd::Trigger(0), PadCmd::Trigger(1), PadCmd::Arm(2), PadCmd::Arm(3)] {
        ch.ui_tx.push(Cmd::MultiPad(c)).ok().unwrap();
    }
    l.step(now + 1);
    run(&mut l, &mut now, bar);
    shared.chord.store(chord.pack(1), Ordering::Release);
    l.step(now + 1);
    run(&mut l, &mut now, 2 * bar);
    // The Multi Pad volume (#196) moves while pads play.
    shared.parts.set_volume(yahaha::parts::PAD_LEVEL, 70);
    run(&mut l, &mut now, 2 * bar + bar / 6);
    shared.parts.set_volume(yahaha::parts::PAD_LEVEL, 120);
    run(&mut l, &mut now, 2 * bar + bar / 3);
    // Playing: a press waits for the bar; a tempo change; Master transpose; a style
    // change to another ppq.
    ch.ui_tx.push(Cmd::MultiPad(PadCmd::Trigger(1))).ok().unwrap();
    ch.ui_tx.push(Cmd::Button(Button::TempoUp)).ok().unwrap();
    ch.ui_tx.push(Cmd::Transpose(Transpose::new(0, 2))).ok().unwrap();
    ch.style_tx.push(b).ok().unwrap();
    run(&mut l, &mut now, 4 * bar);
    // Synchro Stop on an Ending, the band stops, STOP + pad, a bank swap, STOP, Panic.
    ch.ui_tx.push(Cmd::MultiPad(PadCmd::SynchroStop(SynchroStop { style_stop: true, ending: true }))).ok().unwrap();
    ch.ui_tx.push(Cmd::Button(Button::Ending(0))).ok().unwrap();
    run(&mut l, &mut now, 8 * bar);
    ch.ui_tx.push(Cmd::MultiPad(PadCmd::Trigger(0))).ok().unwrap();
    ch.ui_tx.push(Cmd::MultiPad(PadCmd::Stop(0))).ok().unwrap();
    ch.pad_tx.push(PadBank { player: Some(p2), tag: 2 }).ok().unwrap();
    ch.ui_tx.push(Cmd::MultiPad(PadCmd::Trigger(2))).ok().unwrap();
    run(&mut l, &mut now, 9 * bar);
    ch.ui_tx.push(Cmd::MultiPad(PadCmd::StopAll)).ok().unwrap();
    ch.ui_tx.push(Cmd::Panic).ok().unwrap();
    ch.pad_tx.push(PadBank { player: None, tag: 3 }).ok().unwrap();
    l.step(now + 1);
    assert_eq!(counts().0 - allocs, 0, "allocations on the engine thread");
    assert_eq!(counts().1 - frees, 0, "frees on the engine thread");
    drop(on);

    let snaps: Vec<_> = std::iter::from_fn(|| ch.snap_rx.pop().ok()).collect();
    assert!(snaps.iter().any(|s| s.multipad.states[2] == PadState::Armed));
    assert!(snaps.iter().any(|s| s.running && s.multipad.states[2] == PadState::Playing), "the chord fired the armed pads");
    assert!(snaps.iter().any(|s| s.multipad.states[1] == PadState::Queued), "a press while playing waits");
    assert_eq!(snaps.last().map(|s| s.multipad.tag), Some(3));
    // Both players came back to be freed off the engine thread.
    assert_eq!(std::iter::from_fn(|| ch.old_pad_rx.pop().ok()).count(), 2);
}

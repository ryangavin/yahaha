//! The MIDI input thread must not allocate or free: the keyboard-part note path
//! (src/live/pipeline.rs: note in, transpose, processor, part routing and output) and the
//! chord section's recognition run there for every key. A counting global allocator (in
//! this test binary only) checks `Input::packet` through both hands, layered parts, a
//! transpose, retriggers, pedals, wheels, the pedals' assignable functions and aftertouch.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use yahaha::live::{FxConfig, FxMode, Input, Out, Shared};
use yahaha::midi::InputHandler;
use yahaha::rt::{PacketSink, Target};
use yahaha::theory::Recognizer;

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

/// The tests take turns: the allocator counts every thread's allocations.
static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn keyboard_note_path_does_not_allocate() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let shared = Arc::new(Shared::new(54));
    for p in 0..3 {
        shared.parts.on[p].store(true, Ordering::Relaxed);
    }
    let (tx, _rx) = rtrb::RingBuffer::new(256);
    let mut input = Input::new(shared.clone(), Recognizer::new(), tx, Out::new(PacketSink::new(Target::Null), None));
    let (actions, mut actions_rx) = rtrb::RingBuffer::new(64);
    input.set_actions(actions);
    // Pedals: 2 runs Start/Stop (an engine button), OTS + (a control-side function, through
    // the actions ring), or Arpeggio Hold / Kbd Harmony/Arpeggio (control-side switches:
    // Hold A sets on the edges, Toggle runs on the press), by round; 3 is a pitch-bend foot
    // controller.
    use yahaha::controllers::{ControlType, Function, PedalSetup, Range};
    let ctl = &shared.controllers;
    ctl.set_pedal(1, PedalSetup { cc: Some(66), function: Function::StartStop, ..PedalSetup::default() });
    ctl.set_pedal(2, PedalSetup { cc: Some(4), function: Function::PitchBend, range: Range::Full, ..PedalSetup::default() });
    // Warm up: nothing sized lazily later on.
    input.packet(1, 0, &[0x90, 60, 100, 0x80, 60, 0]);
    input.end_of_list();

    let (allocs, frees) = (ALLOCS.load(Ordering::Relaxed), FREES.load(Ordering::Relaxed));
    let mut assigned = 0;
    for round in 0..50u8 {
        let (function, control_type) = match round % 4 {
            0 => (Function::StartStop, ControlType::HoldA),
            1 => (Function::OtsNext, ControlType::HoldA),
            2 => (Function::ArpHold, ControlType::HoldA),
            _ => (Function::KbdHarmonyArp, ControlType::Toggle),
        };
        ctl.set_pedal(1, PedalSetup { cc: Some(66), function, control_type, ..PedalSetup::default() });
        while actions_rx.pop().is_ok() {
            assigned += 1;
        }
        shared.key_shift.store((round % 5) as i8 - 2, Ordering::Relaxed);
        // Some rounds with a keyboard part soloed (Left, Right 2, none).
        shared.parts.set_solo([None, Some(3), Some(1)][round as usize % 3]);
        // Some rounds with the Chord Looper looping: the left hand plays too.
        shared.looping.store(round % 2 == 1, Ordering::Relaxed);
        // Some rounds in AI Full Keyboard, where a re-struck chord is checked for three
        // notes (#107); the dyad after everything is up at the end of the round is one.
        use yahaha::fingering::Fingering;
        let mode = if round % 3 == 2 { Fingering::AiFullKeyboard } else { Fingering::Fingered };
        shared.fingering.store(mode.to_u8(), Ordering::Relaxed);
        // A left-hand chord, a right-hand melody over layered parts, a retrigger, the
        // sustain pedal, poly aftertouch, then everything up (one note-off as a note-on
        // with velocity 0, one through running status).
        input.packet(1, 0, &[0x90, 36, 90, 40, 90, 43, 90]);
        input.packet(1, 0, &[0x90, 72, 100, 0x90, 76, 80, 0x90, 72, 110]);
        input.packet(1, 0, &[0xB0, 64, 127, 0xA0, 76, 40, 0xE0, 0, 80]);
        input.packet(1, 0, &[0x80, 36, 0, 40, 0, 0x90, 43, 0]);
        input.packet(1, 0, &[0x80, 72, 0, 76, 0, 0xB0, 64, 0]);
        // The wheels, the pedals' functions, a part switched under them, a reset.
        input.packet(1, 0, &[0xB0, 1, round, 0xE0, round, 0x50, 0xB0, 66, 127, 0xB0, 4, round]);
        shared.parts.toggle(1);
        input.packet(1, 0, &[0xB0, 66, 0, 0xB0, 64, 127, 0xB0, 121, 0, 0xB0, 64, 0]);
        input.packet(1, 0, &[0x90, 64, 90, 67, 90, 0x80, 64, 0, 67, 0]);
        input.end_of_list();
        ctl.reset(&mut |_| {});
    }
    assert_eq!(ALLOCS.load(Ordering::Relaxed) - allocs, 0, "the input thread allocated");
    assert_eq!(FREES.load(Ordering::Relaxed) - frees, 0, "the input thread freed");
    assert!(assigned > 0, "OTS + went through the actions ring");
}

/// The processor slot: every Harmony type (Strum's late notes and the Echo category go to
/// the engine thread's ring), Multi Assign and the arpeggio, switched while keys are down.
#[test]
fn harmony_and_arpeggio_processor_does_not_allocate() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let shared = Arc::new(Shared::new(54));
    for p in 0..3 {
        shared.parts.on[p].store(true, Ordering::Relaxed);
    }
    let (tx, _rx) = rtrb::RingBuffer::new(256);
    let (fx_tx, mut fx_rx) = rtrb::RingBuffer::new(yahaha::live::FX_RING);
    let mut input = Input::new(shared.clone(), Recognizer::new(), tx, Out::new(PacketSink::new(Target::Null), None));
    input.set_fx(fx_tx);
    let configs: Vec<u64> = yahaha::harmony::ALL_TYPES
        .iter()
        .map(|&ty| {
            let mut c = FxConfig { on: true, ..FxConfig::default() };
            c.harmony.ty = ty;
            c.pack()
        })
        .chain([FxConfig { on: true, mode: FxMode::Arpeggio, ..FxConfig::default() }.pack(), FxConfig::default().pack()])
        .collect();
    input.packet(1, 0, &[0x90, 60, 100, 0x80, 60, 0]);
    input.end_of_list();

    let (allocs, frees) = (ALLOCS.load(Ordering::Relaxed), FREES.load(Ordering::Relaxed));
    for round in 0..4u8 {
        for &w in &configs {
            shared.kbd_fx.store(w, Ordering::Relaxed);
            shared.key_shift.store((round % 3) as i8 - 1, Ordering::Relaxed);
            // A left-hand chord, a right-hand melody with a lower key and a retrigger; the
            // type switches while they are down, then everything goes up.
            input.packet(1, 0, &[0x90, 36, 90, 40, 90, 43, 90]);
            input.packet(1, 0, &[0x90, 72, 100, 0x90, 64, 80, 0x90, 76, 110, 0x90, 72, 100]);
            input.end_of_list();
            while fx_rx.pop().is_ok() {}
        }
        for k in [36u8, 40, 43, 64, 72, 76] {
            input.packet(1, 0, &[0x80, k, 0]);
        }
        input.end_of_list();
        while fx_rx.pop().is_ok() {}
    }
    assert_eq!(ALLOCS.load(Ordering::Relaxed) - allocs, 0, "the input thread allocated");
    assert_eq!(FREES.load(Ordering::Relaxed) - frees, 0, "the input thread freed");
}

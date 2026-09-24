//! The MIDI input thread must not allocate or free: the keyboard-part note path
//! (src/live/pipeline.rs: note in, transpose, processor, part routing and output) and the
//! chord section's recognition run there for every key. A counting global allocator (in
//! this test binary only) checks `Input::packet` through both hands, layered parts, a
//! transpose, retriggers, pedals, wheels, the pedals' assignable functions and aftertouch.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use yahaha::live::{Input, Out, Shared};
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

#[test]
fn keyboard_note_path_does_not_allocate() {
    let shared = Arc::new(Shared::new(54));
    for p in 0..3 {
        shared.parts.on[p].store(true, Ordering::Relaxed);
    }
    let (tx, _rx) = rtrb::RingBuffer::new(256);
    let mut input = Input::new(shared.clone(), Recognizer::new(), tx, Out::new(PacketSink::new(Target::Null), None));
    let (actions, mut actions_rx) = rtrb::RingBuffer::new(64);
    input.set_actions(actions);
    // Pedals: 2 runs Start/Stop (an engine button) or, every other round, OTS + (a
    // control-side function, through the actions ring); 3 is a pitch-bend foot controller.
    use yahaha::controllers::{Function, PedalSetup, Range};
    let ctl = &shared.controllers;
    ctl.set_pedal(1, PedalSetup { cc: Some(66), function: Function::StartStop, ..PedalSetup::default() });
    ctl.set_pedal(2, PedalSetup { cc: Some(4), function: Function::PitchBend, range: Range::Full, ..PedalSetup::default() });
    // Warm up: nothing sized lazily later on.
    input.packet(1, 0, &[0x90, 60, 100, 0x80, 60, 0]);
    input.end_of_list();

    let (allocs, frees) = (ALLOCS.load(Ordering::Relaxed), FREES.load(Ordering::Relaxed));
    let mut assigned = 0;
    for round in 0..50u8 {
        let function = if round % 2 == 0 { Function::StartStop } else { Function::OtsNext };
        ctl.set_pedal(1, PedalSetup { cc: Some(66), function, ..PedalSetup::default() });
        while actions_rx.pop().is_ok() {
            assigned += 1;
        }
        shared.key_shift.store((round % 5) as i8 - 2, Ordering::Relaxed);
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
        input.end_of_list();
        ctl.reset(&mut |_| {});
    }
    assert_eq!(ALLOCS.load(Ordering::Relaxed) - allocs, 0, "the input thread allocated");
    assert_eq!(FREES.load(Ordering::Relaxed) - frees, 0, "the input thread freed");
    assert!(assigned > 0, "OTS + went through the actions ring");
}

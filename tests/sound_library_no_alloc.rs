//! The audio callback with the sound library's program map active (#103) must not
//! allocate or free: band program changes routed to a second SoundFont's synthesizer,
//! keyboard parts on their own patches, a table rewrite under the playing channels, a
//! table bank switch (a style change), an audition start and end, and the port's program
//! mapping on the engine thread. A counting global allocator (in this test binary only)
//! checks every call.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use yahaha::parts::Parts;
use yahaha::patches::port::PortMap;
use yahaha::patches::route::{AUDITION, AUDITION_CHANNEL, ROUTE_BANK};
use yahaha::patches::{Route, Routes};
use yahaha::synth::{self, AudioCore, Rack, SynthControl};

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

fn counted(f: impl FnOnce()) -> (usize, usize) {
    let (a, d) = (ALLOCS.load(Ordering::Relaxed), FREES.load(Ordering::Relaxed));
    f();
    (ALLOCS.load(Ordering::Relaxed) - a, FREES.load(Ordering::Relaxed) - d)
}

#[test]
fn the_audio_callback_with_a_map_does_not_allocate() {
    // A tiny SoundFont built in code, so the test runs whatever SoundFonts the checkout has.
    let font = Arc::new(rustysynth::SoundFont::new(&mut &yahaha::patches::sf2::tiny_gm_sound_font()[..]).unwrap());
    // The same font as two slots: font 0 the main one, font 5 an "extra" SoundFont.
    let rack = Box::new(Rack::with_fonts(&[(0, font.clone()), (5, font)], 48_000).unwrap());
    let routes = Arc::new(Routes::new());
    let mut prog = [None; 128];
    for r in &mut prog[32..40] {
        *r = Some(Route::sound_font(5, 0, 33));
    }
    prog[48] = Some(Route::sound_font(5, 0, 49));
    routes.write_bank(0, &prog, Some(Route::sound_font(5, 128, 0)));
    routes.write_bank(1, &[None; 128], None);
    routes.set_parts([Some(Route::sound_font(5, 0, 4)), None, None, None]);

    let (mut band, rx1) = rtrb::RingBuffer::<synth::Msg>::new(256);
    let (_keys, rx2) = rtrb::RingBuffer::<synth::Msg>::new(16);
    let (mut control, rx3) = rtrb::RingBuffer::<synth::Msg>::new(64);
    let parts = Arc::new(Parts::new());
    let (mut core, _swap, _link) = AudioCore::new(Some(rack), vec![rx1, rx2, rx3], parts.clone(), Arc::new(SynthControl::new(0)), 48_000, 2);
    core.set_routes(routes.clone());
    let mut out = vec![0f32; 128];
    let none = (0, 0);
    let mut run = |core: &mut AudioCore| counted(|| core.process(&mut out));
    // Warm up: the first buffers route everything (the parts, the table).
    for _ in 0..4 {
        run(&mut core);
    }
    for m in [[0xBA, 0, 8], [0xCA, 35, 0], [0xCC, 48, 0], [0xC8, 0, 0], [0x9A, 40, 100], [0x9C, 60, 90], [0x98, 36, 100], [0x90, 60, 100]] {
        band.push(m).unwrap();
    }
    assert_eq!(run(&mut core), none, "routed program changes and notes");
    // The control side rewrites the table under the playing channels.
    prog[48] = None;
    routes.write_bank(0, &prog, None);
    assert_eq!(run(&mut core), none, "a table rewrite: the channels route again");
    parts.set_program(1, 5);
    routes.set_parts([None, Some(Route::sound_font(5, 0, 5)), None, None]);
    assert_eq!(run(&mut core), none, "the parts' patches change");
    // A style change: the engine names the other bank.
    band.push([ROUTE_BANK, 1, 0]).unwrap();
    band.push([0xCA, 33, 0]).unwrap();
    assert_eq!(run(&mut core), none, "a bank switch");
    band.push([ROUTE_BANK, 0, 0]).unwrap();
    assert_eq!(run(&mut core), none, "and back");
    // An audition on its channel, from the control side's ring.
    routes.set_audition(Some(Route::sound_font(5, 0, 10)));
    for m in [[AUDITION, 1, 0], [0x90 | AUDITION_CHANNEL, 60, 100]] {
        control.push(m).unwrap();
    }
    assert_eq!(run(&mut core), none, "an audition starts");
    for m in [[0x80 | AUDITION_CHANNEL, 60, 0], [AUDITION, 0, 0]] {
        control.push(m).unwrap();
    }
    assert_eq!(run(&mut core), none, "and ends: the band's channel set up again");
    for _ in 0..8 {
        assert_eq!(run(&mut core), none);
    }

    // The engine thread's port mapping.
    routes.port_mapped.store(true, Ordering::Relaxed);
    let mut pm = PortMap::new(routes.clone());
    pm.set_bank(0);
    let mut sent = 0usize;
    let got = counted(|| {
        for m in [[0xBA, 0, 0], [0xCA, 33, 0], [0xC9, 0, 0], [0xBA, 7, 100]] {
            pm.send(&m, |_| sent += 1);
        }
    });
    assert_eq!(got, none, "the port's program mapping");
    // Program 33 maps (three messages); the drums have no route since the rewrite.
    assert_eq!(sent, 1 + 3 + 1 + 1);
}

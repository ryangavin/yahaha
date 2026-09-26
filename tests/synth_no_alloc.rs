//! The audio callback (`synth::AudioCore::process`) must not allocate or free: SoundFont
//! notes and controllers, the effect bus (sends, types, returns, legacy effects), the
//! master fader, a SoundFont swap, and (feature `plugins`) a
//! keyboard part going over to an Audio Unit instrument (Apple's DLSMusicDevice), playing
//! it, crossfading to a second instance, and back to the SoundFont. SoundFont swaps while
//! the control side is not taking old racks back must not free one either. A counting
//! global allocator (in this test binary only) checks every `process` call, on the calling
//! thread. What the plugin does inside its own render is its own business and does not go
//! through Rust's allocator.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use yahaha::parts::Parts;
use yahaha::synth::{self, AudioCore, Rack, SynthControl};

struct Counting;
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static FREES: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    /// Count on this thread only, only inside `process`: plugin load threads and the
    /// dispose thread allocate and free on their own time.
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

/// The smallest SoundFont in the checkout's soundfonts/, if any.
fn sound_font() -> Option<std::path::PathBuf> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("soundfonts");
    yahaha::library::sound_font_files(&dir).into_iter().map(|f| dir.join(f)).min_by_key(|p| p.metadata().map(|m| m.len()).unwrap_or(u64::MAX))
}

#[test]
fn the_audio_callback_does_not_allocate() {
    let font = sound_font();
    let rack = font.as_ref().map(|f| Rack::load(f, 48_000).unwrap());
    if rack.is_none() {
        eprintln!("no SoundFont: checking the callback without one");
    }
    let (mut feed, rx) = rtrb::RingBuffer::<synth::Msg>::new(256);
    let ctl = Arc::new(SynthControl::new(0));
    let parts = Arc::new(Parts::new());
    let (mut core, mut swap, _link) = AudioCore::new(rack, vec![rx], parts.clone(), ctl.clone(), 48_000, 2);
    let mut out = vec![0f32; 128];
    let mut run = |core: &mut AudioCore, feed: &mut rtrb::Producer<synth::Msg>, msgs: &[[u8; 3]]| -> (usize, usize) {
        for m in msgs {
            feed.push(*m).unwrap();
        }
        let (a, f) = (ALLOCS.load(Ordering::Relaxed), FREES.load(Ordering::Relaxed));
        COUNT.with(|c| c.set(true));
        core.process(&mut out);
        COUNT.with(|c| c.set(false));
        (ALLOCS.load(Ordering::Relaxed) - a, FREES.load(Ordering::Relaxed) - f)
    };
    let none = (0, 0);
    assert_eq!(run(&mut core, &mut feed, &[[0xC0, 0, 0], [0xB0, 7, 100], [0x90, 60, 100], [0x9A, 40, 100]]), none, "notes");
    for _ in 0..10 {
        assert_eq!(run(&mut core, &mut feed, &[]), none, "steady");
    }
    // The effect bus (#204): sends on, every reverb, chorus and delay type, tempo changes, the returns, the
    // SoundFont's own effects instead (legacy) and back, and tails ringing out.
    assert_eq!(run(&mut core, &mut feed, &[[0xB0, 91, 100], [0xB0, 93, 80], [0xBA, 91, 127], [0xBA, 94, 60], [0x90, 64, 100]]), none, "sends");
    for t in 0..4u8 {
        ctl.fx.reverb_type.store(t, Ordering::Relaxed);
        ctl.fx.chorus_type.store(t % 3, Ordering::Relaxed);
        ctl.fx.reverb_return.store(40 + t * 20, Ordering::Relaxed);
        ctl.fx.variation_type.store(t, Ordering::Relaxed);
        ctl.fx.set_tempo(90.0 + t as f64 * 20.0);
        assert_eq!(run(&mut core, &mut feed, &[]), none, "effect types and returns");
    }
    ctl.fx.legacy.store(true, Ordering::Relaxed);
    assert_eq!(run(&mut core, &mut feed, &[[0x90, 67, 100]]), none, "the SoundFont's own effects");
    ctl.fx.legacy.store(false, Ordering::Relaxed);
    assert_eq!(run(&mut core, &mut feed, &[[0x80, 60, 0], [0x80, 64, 0], [0x80, 67, 0], [0x8A, 40, 0]]), none, "the bus again");
    for _ in 0..20 {
        assert_eq!(run(&mut core, &mut feed, &[]), none, "tails");
    }
    ctl.master.store(90, Ordering::Relaxed);
    parts.set_program(0, 5);
    assert_eq!(run(&mut core, &mut feed, &[[0xB0, 1, 30], [0xE0, 0, 80]]), none, "master, program, controllers");
    // A new SoundFont swapped in: replayed, the old one fades and goes back.
    if let Some(f) = &font {
        swap.tx.push(Rack::load(f, 48_000).unwrap()).ok().unwrap();
        for _ in 0..3 {
            assert_eq!(run(&mut core, &mut feed, &[]), none, "SoundFont swap");
        }
        while swap.old.pop().is_ok() {}

        // The control side stops taking old racks back (busy, or not pumping): the return
        // ring fills. The callback holds further swaps until there is room again, rather
        // than freeing a rack itself.
        let sf = Arc::new(rustysynth::SoundFont::new(&mut std::fs::File::open(f).unwrap()).unwrap());
        let cap = swap.old.buffer().capacity();
        let before = ctl.swaps.load(Ordering::Relaxed);
        let mut sent = 0;
        for _ in 0..(cap + 3) * 2 {
            if swap.tx.push(Box::new(Rack::new(&sf, 48_000).unwrap())).is_ok() {
                sent += 1;
            }
            assert_eq!(run(&mut core, &mut feed, &[[0x90, 62, 90]]), none, "SoundFont swaps with the return ring full");
        }
        let taken = (ctl.swaps.load(Ordering::Relaxed) - before) as usize;
        assert_eq!(taken, cap, "swaps wait once the return ring is full");
        assert_eq!(swap.old.slots(), taken, "every replaced rack came back");
        // The control side drains the ring: the waiting racks go in.
        while swap.old.pop().is_ok() {}
        for _ in 0..8 {
            assert_eq!(run(&mut core, &mut feed, &[]), none, "swaps resume");
            while swap.old.pop().is_ok() {}
        }
        assert_eq!((ctl.swaps.load(Ordering::Relaxed) - before) as usize, sent, "every rack sent was taken");
    }

    #[cfg(feature = "plugins")]
    {
        use yahaha::plugin::{LoadConfig, PluginHost, PluginId, Swap};
        use yahaha::route::Source;
        let mut link = _link.unwrap();
        let dls = || PluginHost::new(None).load(&PluginId::DLS, LoadConfig { max_frames: synth::PLUGIN_MAX_BLOCK as u32, ..Default::default() }).unwrap();
        let (a, b) = (dls(), dls());
        // Right 1 (channel 0) over to the plugin.
        link.assign(0, a, Swap::default()).ok().unwrap();
        ctl.routes.set(0, Source::Plugin);
        assert_eq!(run(&mut core, &mut feed, &[[0x90, 64, 100], [0xB0, 7, 110], [0xB0, 10, 30], [0xB0, 64, 127]]), none, "the part goes over to its plugin");
        assert_eq!(core.plugin_channels(), 1);
        for _ in 0..10 {
            assert_eq!(run(&mut core, &mut feed, &[[0x90, 67, 90], [0x80, 64, 0]]), none, "plugin playing");
        }
        link.assign(0, b, Swap::default()).ok().unwrap();
        for _ in 0..6 {
            assert_eq!(run(&mut core, &mut feed, &[[0xB0, 7, 90]]), none, "crossfade to a second instance");
        }
        link.clear(0, 240);
        ctl.routes.set(0, Source::SoundFont(0));
        for _ in 0..6 {
            assert_eq!(run(&mut core, &mut feed, &[[0x90, 60, 100]]), none, "back to the SoundFont");
        }
        assert_eq!(core.plugin_channels(), 0);
        let _ = link.take_retired();
    }
}

//! The plugin rack's audio-thread calls (`begin_block`, `midi`, `render_add`), including a
//! swap, a crossfade and a clear, must not allocate on the host side (a fault only sets a
//! flag and pushes an event; `plugin::tests` covers its behaviour). A counting
//! global allocator (in this test binary only) checks it. What the plugin itself does in its
//! render is its own business and does not go through Rust's allocator.
#![cfg(feature = "plugins")]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use yahaha::plugin::{LoadConfig, PluginHost, PluginId, PluginInstance, Swap, rack};

struct Counting;
static ALLOCS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}

#[global_allocator]
static A: Counting = Counting;

fn dls() -> PluginInstance {
    PluginHost::new(None).load(&PluginId::DLS, LoadConfig { max_frames: 256, ..Default::default() }).unwrap()
}

#[test]
fn rack_audio_path_does_not_allocate() {
    let (mut rack, mut ctl) = rack(256, 48_000.0);
    let (a, b) = (dls(), dls());
    let (mut l, mut r) = (vec![0f32; 256], vec![0f32; 256]);
    let mut run = |rack: &mut yahaha::plugin::PluginRack, msgs: &[[u8; 3]]| -> usize {
        let before = ALLOCS.load(Ordering::Relaxed);
        rack.begin_block();
        for m in msgs {
            rack.midi(*m, 17);
        }
        l.fill(0.0);
        r.fill(0.0);
        rack.render_add(&mut l, &mut r);
        ALLOCS.load(Ordering::Relaxed) - before
    };
    ctl.assign(0, a, Swap::default()).ok().unwrap();
    assert_eq!(run(&mut rack, &[[0xB0, 7, 110], [0xB0, 1, 40], [0xE0, 0, 70], [0x90, 60, 100], [0x90, 64, 100]]), 0, "assign + notes");
    // RPN 0-2 (bend range, tuning) and an NRPN select: tracked, then replayed on the swap.
    let rpn = [[0xB0, 101, 0], [0xB0, 100, 0], [0xB0, 6, 12], [0xB0, 38, 0], [0xB0, 100, 1], [0xB0, 6, 64], [0xB0, 99, 1], [0xB0, 98, 8]];
    assert_eq!(run(&mut rack, &rpn), 0, "RPN / NRPN tracking");
    for _ in 0..20 {
        assert_eq!(run(&mut rack, &[]), 0, "steady");
    }
    ctl.assign(0, b, Swap::default()).ok().unwrap();
    assert_eq!(run(&mut rack, &[[0x90, 67, 100]]), 0, "swap with controller replay");
    for _ in 0..4 {
        assert_eq!(run(&mut rack, &[[0xB0, 7, 90]]), 0, "crossfade and fader moves");
    }
    ctl.clear(0, 240);
    for _ in 0..4 {
        assert_eq!(run(&mut rack, &[[0x80, 60, 0]]), 0, "clear and fade out");
    }
    // Instances come back to be dropped here, off the audio path.
    assert_eq!(ctl.take_retired().len(), 2);
}

//! `Arp::process` and the note/pedal/settings calls must not allocate: the arp runs on
//! the real-time threads. A counting global allocator (in this test binary only) checks it.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use yahaha::arp::{library, Arp, ArpSink, Settings};

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

/// A sink that only counts.
#[derive(Default)]
struct Count {
    on: usize,
    off: usize,
}

impl ArpSink for Count {
    fn note_on(&mut self, _: u64, _: u8, _: u8) {
        self.on += 1;
    }
    fn note_off(&mut self, _: u64, _: u8) {
        self.off += 1;
    }
}

#[test]
fn process_does_not_allocate() {
    let mut arps: Vec<Arp> = library::PATTERNS.iter().map(|p| Arp::new(1920, p.clone())).collect();
    let mut sink = Count::default();
    let before = ALLOCS.load(Ordering::Relaxed);
    for a in arps.iter_mut() {
        for (i, n) in [48u8, 55, 60, 64, 67, 71].into_iter().enumerate() {
            a.note_on(n, 90, i as u64);
        }
        a.set_settings(Settings { hold: true, unit_multiply: 150, ..Settings::default() }, 10);
        let mut t = 0;
        while t < 8 * 7680 {
            a.process(t..t + 97, &mut sink);
            t += 97;
        }
        a.set_sustain(true, t);
        a.note_off(60, t);
        a.set_hold(false, t);
        a.process(t..t + 7680, &mut sink);
        a.all_off(t + 7680, &mut sink);
    }
    let after = ALLOCS.load(Ordering::Relaxed);
    assert_eq!(after - before, 0, "allocated {} times", after - before);
    assert!(sink.on > 1000);
    assert_eq!(sink.on, sink.off);
}

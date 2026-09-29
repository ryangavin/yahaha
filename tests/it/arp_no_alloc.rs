//! `Arp::process` and the note/pedal/settings calls must not allocate: the arp runs on
//! the real-time threads. The crate's counting allocator (`alloc_count`) checks it.

use crate::alloc_count::{count_here, counts};
use yahaha::arp::{library, Arp, ArpSink, Settings};

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
    let _on = count_here();
    let before = counts().0;
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
        // Clock jumps back (style restart), a gap, and a pattern swap between borrowed
        // library patterns.
        a.note_on(62, 100, 0);
        a.process(0..500, &mut sink);
        a.process(90_000..90_500, &mut sink);
        a.set_pattern(library::PATTERNS[0].clone(), 90_500);
        a.process(90_500..91_000, &mut sink);
        a.note_off(62, 91_000);
        a.all_off(t + 7680, &mut sink);
    }
    let after = counts().0;
    assert_eq!(after - before, 0, "allocated {} times", after - before);
    assert!(sink.on > 1000);
    assert_eq!(sink.on, sink.off);
}

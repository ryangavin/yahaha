//! The performance view's collection side (the view itself, `yahaha::perf`, reads it).
//!
//! **Collection** happens where the work is, on the audio, engine and MIDI threads, into
//! [`PERF`]: plain atomics and fixed histograms, no allocation, no lock, no panic. It only
//! runs while the view is on ([`enable`]); off, each audio callback pays one relaxed load
//! (`Perf::on`) and the synthesizers read no clock.
//!
//! It lives in core so every layer that records into it (fx, the synth, the facade) can
//! reach it; the reading side, which needs the synth, stays in the facade's `perf`.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering::Relaxed};

/// MIDI channels.
pub const CHANNELS: usize = 16;

/// The effect bus's sends, a counter row each: the three buses, Reverb (CC91), Chorus
/// (CC93) and Variation (CC94), then the player's sends 4-6. fx asserts it equals its
/// `SENDS` (its own `BUSES` is the three buses).
pub const BUSES: usize = 6;

/// The audio callback's stages, as [`Perf::stage`] indexes them.
pub const STAGES: [&str; 8] = ["midi", "band", "keys", "extra", "fade", "plugins", "fx", "out"];
/// Insertion effect slots: one per Style part (#269).
pub const INSERTS: usize = 8;

pub const ST_MIDI: usize = 0;
pub const ST_BAND: usize = 1;
pub const ST_KEYS: usize = 2;
pub const ST_EXTRA: usize = 3;
pub const ST_FADE: usize = 4;
pub const ST_PLUGINS: usize = 5;
pub const ST_FX: usize = 6;
pub const ST_OUT: usize = 7;

/// The synth's MIDI rings, as `AudioCore` has them: the engine's, the input thread's and
/// the control side's (auditions).
pub const RINGS: [&str; 3] = ["engine", "input", "control"];

/// A histogram bucket's width, and how many there are (the last catches everything
/// longer): 8 µs up to 8 ms, enough for the largest buffer's deadline (1024 frames at
/// 44.1 kHz is 23 ms: past 8 ms it is late anyway at every size up to 256).
const BUCKET_NS: u64 = 8_000;
const BUCKETS: usize = 1024;

/// Durations over a window: how many, their sum and longest, and a histogram for the
/// percentiles. Written by one thread, read (and reset) by the view.
pub struct Timing {
    count: AtomicU64,
    sum_ns: AtomicU64,
    max_ns: AtomicU64,
    hist: [AtomicU32; BUCKETS],
}

impl Timing {
    const fn new() -> Timing {
        Timing { count: AtomicU64::new(0), sum_ns: AtomicU64::new(0), max_ns: AtomicU64::new(0), hist: [const { AtomicU32::new(0) }; BUCKETS] }
    }

    /// Note one duration. RT-safe.
    #[inline]
    pub fn record(&self, ns: u64) {
        self.count.fetch_add(1, Relaxed);
        self.sum_ns.fetch_add(ns, Relaxed);
        self.max_ns.fetch_max(ns, Relaxed);
        self.hist[((ns / BUCKET_NS) as usize).min(BUCKETS - 1)].fetch_add(1, Relaxed);
    }

    /// The window so far, and start a new one.
    // Facade-internal: only the view (`yahaha::perf::take`) reads windows.
    #[doc(hidden)]
    pub fn take(&self) -> Window {
        let count = self.count.swap(0, Relaxed);
        let sum_ns = self.sum_ns.swap(0, Relaxed);
        let max_ns = self.max_ns.swap(0, Relaxed);
        let mut hist = [0u32; BUCKETS];
        for (h, a) in hist.iter_mut().zip(&self.hist) {
            *h = a.swap(0, Relaxed);
        }
        let p99_ns = percentile(&hist, 0.99);
        Window { count, sum_ns, max_ns, p99_ns }
    }
}

/// The upper bound of the bucket holding quantile `q`, in ns (0 if empty).
fn percentile(hist: &[u32; BUCKETS], q: f64) -> u64 {
    let total: u64 = hist.iter().map(|&x| x as u64).sum();
    if total == 0 {
        return 0;
    }
    let want = ((total as f64) * q).ceil().max(1.0) as u64;
    let mut acc = 0u64;
    for (i, &n) in hist.iter().enumerate() {
        acc += n as u64;
        if acc >= want {
            return (i as u64 + 1) * BUCKET_NS;
        }
    }
    BUCKETS as u64 * BUCKET_NS
}

/// One window of a [`Timing`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Window {
    pub count: u64,
    pub sum_ns: u64,
    pub max_ns: u64,
    /// Upper bound, to a bucket (8 µs).
    pub p99_ns: u64,
}

impl Window {
    pub fn mean_ns(&self) -> u64 {
        self.sum_ns.checked_div(self.count).unwrap_or(0)
    }
}

/// A sum and a maximum over a window (a stage, a channel, an effect block).
pub struct Cost {
    sum_ns: AtomicU64,
    max_ns: AtomicU64,
}

impl Cost {
    const fn new() -> Cost {
        Cost { sum_ns: AtomicU64::new(0), max_ns: AtomicU64::new(0) }
    }

    #[inline]
    pub fn add(&self, ns: u64) {
        self.sum_ns.fetch_add(ns, Relaxed);
        self.max_ns.fetch_max(ns, Relaxed);
    }

    // Facade-internal: only the view (`yahaha::perf::take`) reads windows.
    #[doc(hidden)]
    pub fn take(&self) -> (u64, u64) {
        (self.sum_ns.swap(0, Relaxed), self.max_ns.swap(0, Relaxed))
    }
}

/// Everything the view shows, collected where it happens.
pub struct Perf {
    on: AtomicBool,
    /// Each audio callback, whole (the device's callback, around `AudioCore::process`).
    pub callback: Timing,
    /// The buffer size the last callback rendered, and the device's rate.
    pub frames: AtomicU32,
    pub sample_rate: AtomicU32,
    /// The callback's stages ([`STAGES`]), per callback.
    pub stage: [Cost; STAGES.len()],
    /// Each channel's render time per callback: its SoundFont voices, or its plugin.
    pub channel: [Cost; CHANNELS],
    /// Each channel's notes sounding (last callback; held or kept by the pedal, counted by
    /// the synth's rack) and peak level (f32 bits, since the view's last read).
    pub channel_notes: [AtomicU32; CHANNELS],
    pub channel_peak: [AtomicU32; CHANNELS],
    /// Channels playing a plugin (bit per channel), as of the last callback.
    pub plugin_mask: AtomicU32,
    /// The effect bus's sends (reverb, chorus, variation, sends 4-6): time per callback
    /// and output peak after the return gain, while one runs.
    pub bus: [Cost; BUSES],
    pub bus_peak: [AtomicU32; BUSES],
    /// The Style parts' insertion effects (#269, by part 0-7): time per callback and
    /// output peak, while one runs.
    pub insert: [Cost; INSERTS],
    pub insert_peak: [AtomicU32; INSERTS],
    /// SoundFont notes sounding (last callback; see `channel_notes`), and the most in the
    /// window.
    pub voices: AtomicU32,
    pub voices_peak: AtomicU32,
    /// The deepest each synth ring was when a callback started draining it ([`RINGS`]).
    pub ring_depth: [AtomicU32; RINGS.len()],
    /// The engine thread: its work per wake, how late it woke for its deadline, and the
    /// deepest its command rings were (the input thread's, the control side's).
    pub engine: Timing,
    pub engine_late: Timing,
    pub engine_queue: [AtomicU32; 2],
    /// The MIDI input thread: packets, messages, and packet timestamp -> our callback.
    pub midi_packets: AtomicU64,
    pub midi_in: Timing,
}

/// A stopwatch for the audio callback's stages: [`Lap::lap`] charges the time since the
/// last lap to a stage. Not profiling, it reads no clock.
pub struct Lap {
    on: bool,
    mark: u64,
}

impl Lap {
    #[inline]
    pub fn start(on: bool) -> Lap {
        Lap { on, mark: if on { crate::rt::host_now() } else { 0 } }
    }

    /// Charge the time since the last lap to `stage` ([`STAGES`]).
    #[inline]
    pub fn lap(&mut self, stage: usize) {
        if self.on {
            let t = crate::rt::host_now();
            PERF.stage[stage].add(crate::rt::host_to_ns(t.wrapping_sub(self.mark)));
            self.mark = t;
        }
    }

    /// `lap`, less `ns` of the time that was charged to another stage already.
    #[inline]
    pub fn lap_less(&mut self, stage: usize, ns: u64) {
        if self.on {
            let t = crate::rt::host_now();
            PERF.stage[stage].add(crate::rt::host_to_ns(t.wrapping_sub(self.mark)).saturating_sub(ns));
            self.mark = t;
        }
    }

    /// Start the next lap from now (the time since was charged elsewhere).
    #[inline]
    pub fn skip(&mut self) {
        if self.on {
            self.mark = crate::rt::host_now();
        }
    }
}

/// The one [`Perf`].
pub static PERF: Perf = Perf::new();

impl Default for Perf {
    fn default() -> Perf {
        Perf::new()
    }
}

impl Perf {
    pub const fn new() -> Perf {
        Perf {
            on: AtomicBool::new(false),
            callback: Timing::new(),
            frames: AtomicU32::new(0),
            sample_rate: AtomicU32::new(0),
            stage: [const { Cost::new() }; STAGES.len()],
            channel: [const { Cost::new() }; CHANNELS],
            channel_notes: [const { AtomicU32::new(0) }; CHANNELS],
            channel_peak: [const { AtomicU32::new(0) }; CHANNELS],
            plugin_mask: AtomicU32::new(0),
            bus: [const { Cost::new() }; BUSES],
            bus_peak: [const { AtomicU32::new(0) }; BUSES],
            insert: [const { Cost::new() }; INSERTS],
            insert_peak: [const { AtomicU32::new(0) }; INSERTS],
            voices: AtomicU32::new(0),
            voices_peak: AtomicU32::new(0),
            ring_depth: [const { AtomicU32::new(0) }; RINGS.len()],
            engine: Timing::new(),
            engine_late: Timing::new(),
            engine_queue: [const { AtomicU32::new(0) }; 2],
            midi_packets: AtomicU64::new(0),
            midi_in: Timing::new(),
        }
    }

    /// Collecting (the view is on). One relaxed load: the whole cost when off.
    #[inline]
    pub fn on(&self) -> bool {
        self.on.load(Relaxed)
    }

    /// A peak (linear) into an f32-bits maximum.
    #[inline]
    pub fn peak(slot: &AtomicU32, v: f32) {
        if v > 0.0 {
            slot.fetch_max(v.to_bits(), Relaxed);
        }
    }
}

/// Turn collection on (the view calls it; tests may too).
pub fn enable() {
    PERF.on.store(true, Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_resets_when_read() {
        let t = Timing::new();
        for ns in [10_000, 20_000, 30_000, 1_000_000] {
            t.record(ns);
        }
        let w = t.take();
        assert_eq!((w.count, w.max_ns, w.mean_ns()), (4, 1_000_000, 265_000));
        assert_eq!(w.p99_ns, 1_000_000 + BUCKET_NS, "the bucket's upper bound");
        assert_eq!(t.take(), Window::default());
    }

    #[test]
    fn the_p99_ignores_the_odd_outlier() {
        let t = Timing::new();
        for _ in 0..999 {
            t.record(50_000);
        }
        t.record(5_000_000);
        let w = t.take();
        assert_eq!(w.p99_ns, 56_000);
        assert_eq!(w.max_ns, 5_000_000);
    }
}

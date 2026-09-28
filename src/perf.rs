//! The performance view: a `top` for yahaha (`yahaha play --top`, or `YAHAHA_TOP=1` /
//! `--top` for the desktop app launched from a terminal).
//!
//! **Collection** happens where the work is, on the audio, engine and MIDI threads, into
//! [`PERF`]: plain atomics and fixed histograms, no allocation, no lock, no panic. It only
//! runs while the view is on ([`enable`]); off, each audio callback pays one relaxed load
//! (`Perf::on`) and the synthesizers read no clock.
//!
//! **Reading** happens on the view's own thread ([`top`]): once a refresh it swaps every
//! window counter back to 0 (so each figure covers the last refresh), reads the process's
//! memory, and draws a frame with ANSI escapes.

pub mod top;

use crate::synth::SynthControl;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex};

/// MIDI channels.
pub const CHANNELS: usize = 16;

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
    fn take(&self) -> Window {
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

    fn take(&self) -> (u64, u64) {
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
    /// The effect bus's blocks (reverb, chorus, variation): time per callback and output
    /// peak.
    pub bus: [Cost; crate::fx::BUSES],
    pub bus_peak: [AtomicU32; crate::fx::BUSES],
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

/// The synth whose dropout counters the view shows (`synth::start` registers it).
static SYNTH: Mutex<Option<Arc<SynthControl>>> = Mutex::new(None);

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
            bus: [const { Cost::new() }; crate::fx::BUSES],
            bus_peak: [const { AtomicU32::new(0) }; crate::fx::BUSES],
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

/// Whether this launch asked for the view: a `--top` argument, or `YAHAHA_TOP` set to
/// anything but "0" or "".
pub fn requested(args: &[String]) -> bool {
    args.iter().any(|a| a == "--top") || std::env::var("YAHAHA_TOP").is_ok_and(|v| !v.is_empty() && v != "0")
}

/// The synth the view reports dropouts for (the latest started).
pub fn register_synth(ctl: &Arc<SynthControl>) {
    *SYNTH.lock().unwrap_or_else(|e| e.into_inner()) = Some(ctl.clone());
}

fn synth() -> Option<Arc<SynthControl>> {
    SYNTH.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// One refresh's figures, read (and the windows reset) by the view.
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    /// Seconds the window covers.
    pub secs: f64,
    pub rss_bytes: u64,
    pub callback: Window,
    pub frames: u32,
    pub sample_rate: u32,
    pub stages: [(u64, u64); STAGES.len()],
    pub channels: [ChannelRow; CHANNELS],
    pub buses: [(u64, u64, f32); crate::fx::BUSES],
    /// Each Style part's insertion effect (#269): sum, max, peak (zeros: none ran).
    pub inserts: [(u64, u64, f32); INSERTS],
    pub voices: u32,
    pub voices_peak: u32,
    pub rings: [u32; RINGS.len()],
    pub engine: Window,
    pub engine_late: Window,
    pub engine_queue: [u32; 2],
    pub midi_packets: u64,
    pub midi_in: Window,
    /// The synth's dropouts since start: the device's reports, our late buffers (None:
    /// no synth).
    pub dropouts: Option<(u64, u64)>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ChannelRow {
    pub sum_ns: u64,
    pub max_ns: u64,
    pub voices: u32,
    /// Linear, since the last refresh.
    pub peak: f32,
    pub plugin: bool,
}

impl Snapshot {
    /// The buffer's deadline, in ns (0: no callback yet).
    pub fn deadline_ns(&self) -> u64 {
        if self.sample_rate == 0 {
            return 0;
        }
        self.frames as u64 * 1_000_000_000 / self.sample_rate as u64
    }
}

/// Read every window (resetting it) into a [`Snapshot`] covering `secs`.
pub fn take(secs: f64) -> Snapshot {
    let p = &PERF;
    let mut s = Snapshot {
        secs,
        rss_bytes: rss_bytes(),
        callback: p.callback.take(),
        frames: p.frames.load(Relaxed),
        sample_rate: p.sample_rate.load(Relaxed),
        voices: p.voices.load(Relaxed),
        voices_peak: p.voices_peak.swap(0, Relaxed),
        engine: p.engine.take(),
        engine_late: p.engine_late.take(),
        midi_packets: p.midi_packets.swap(0, Relaxed),
        midi_in: p.midi_in.take(),
        dropouts: synth().map(|c| (c.xruns.load(Relaxed), c.late.load(Relaxed))),
        ..Snapshot::default()
    };
    for (d, c) in s.stages.iter_mut().zip(&p.stage) {
        *d = c.take();
    }
    let plugins = p.plugin_mask.load(Relaxed);
    for (ch, row) in s.channels.iter_mut().enumerate() {
        let (sum_ns, max_ns) = p.channel[ch].take();
        *row = ChannelRow {
            sum_ns,
            max_ns,
            voices: p.channel_notes[ch].load(Relaxed),
            peak: f32::from_bits(p.channel_peak[ch].swap(0, Relaxed)),
            plugin: plugins >> ch & 1 == 1,
        };
    }
    for (b, d) in s.buses.iter_mut().enumerate() {
        let (sum, max) = p.bus[b].take();
        *d = (sum, max, f32::from_bits(p.bus_peak[b].swap(0, Relaxed)));
    }
    for (i, d) in s.inserts.iter_mut().enumerate() {
        let (sum, max) = p.insert[i].take();
        *d = (sum, max, f32::from_bits(p.insert_peak[i].swap(0, Relaxed)));
    }
    for (d, a) in s.rings.iter_mut().zip(&p.ring_depth) {
        *d = a.swap(0, Relaxed);
    }
    for (d, a) in s.engine_queue.iter_mut().zip(&p.engine_queue) {
        *d = a.swap(0, Relaxed);
    }
    s
}

/// The process's resident memory (bytes), from the kernel's task info.
#[cfg(target_vendor = "apple")]
pub fn rss_bytes() -> u64 {
    use mach2::task_info::{mach_task_basic_info, MACH_TASK_BASIC_INFO, MACH_TASK_BASIC_INFO_COUNT};
    let mut info: mach_task_basic_info = unsafe { std::mem::zeroed() };
    let mut count = MACH_TASK_BASIC_INFO_COUNT;
    let kr = unsafe {
        mach2::task::task_info(mach2::traps::mach_task_self(), MACH_TASK_BASIC_INFO, &mut info as *mut _ as *mut i32, &mut count)
    };
    if kr == mach2::kern_return::KERN_SUCCESS { info.resident_size } else { 0 }
}

/// The process's resident memory (bytes), from /proc/self/statm (0 if unreadable).
#[cfg(not(target_vendor = "apple"))]
pub fn rss_bytes() -> u64 {
    let Ok(s) = std::fs::read_to_string("/proc/self/statm") else { return 0 };
    let pages: u64 = s.split_whitespace().nth(1).and_then(|v| v.parse().ok()).unwrap_or(0);
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    pages * if page > 0 { page as u64 } else { 4096 }
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

    #[test]
    fn it_is_asked_for_by_flag_or_environment() {
        assert!(requested(&["play".into(), "--top".into()]));
        assert!(!requested(&["play".into()]) || std::env::var("YAHAHA_TOP").is_ok());
    }

    #[test]
    fn memory_is_read_from_the_kernel() {
        assert!(rss_bytes() > 1 << 20, "a running test uses more than a megabyte");
    }
}

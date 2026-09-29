//! The performance view: a `top` for yahaha (`yahaha play --top`, or `YAHAHA_TOP=1` /
//! `--top` for the desktop app launched from a terminal).
//!
//! **Collection** happens where the work is, on the audio, engine and MIDI threads, into
//! [`PERF`]: plain atomics and fixed histograms, no allocation, no lock, no panic. It only
//! runs while the view is on ([`enable`]); off, each audio callback pays one relaxed load
//! (`Perf::on`) and the synthesizers read no clock. That side lives in core
//! (`yahaha_core::perf`, re-exported here) so fx, below the synth, can record into it.
//!
//! **Reading** happens on the view's own thread ([`top`]): once a refresh it swaps every
//! window counter back to 0 (so each figure covers the last refresh), reads the process's
//! memory, and draws a frame with ANSI escapes.

pub mod top;

use crate::synth::SynthControl;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::{Arc, Mutex};

pub use yahaha_core::perf::{
    CHANNELS, Cost, INSERTS, Lap, PERF, Perf, RINGS, STAGES, ST_BAND, ST_EXTRA, ST_FADE, ST_FX, ST_KEYS, ST_MIDI, ST_OUT, ST_PLUGINS, Timing, Window,
    enable,
};

/// The synth whose dropout counters the view shows (`synth::start` registers it).
static SYNTH: Mutex<Option<Arc<SynthControl>>> = Mutex::new(None);

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
    pub buses: [(u64, u64, f32); yahaha_fx::fx::BUSES],
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

    // The window tests (`a_window_resets_when_read`, `the_p99_ignores_the_odd_outlier`)
    // moved to core with `Timing`.

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

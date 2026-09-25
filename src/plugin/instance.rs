//! A loaded, initialised instrument: MIDI in, stereo out, state as bytes.

use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering::Relaxed};
use std::time::{Duration, Instant};

use super::scan::PluginInfo;
use super::sys::{self, OSStatus, Unit};

/// Why a render produced no audio. The output is zeroed in every case.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderError {
    /// `AudioUnitRender` returned this OSStatus (for an out-of-process unit, -66749 means
    /// its process died).
    Status(OSStatus),
    /// The plugin produced NaN or infinity.
    NonFinite,
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            RenderError::Status(st) => write!(f, "render failed: OSStatus {st} {}", sys::status_name(st)),
            RenderError::NonFinite => write!(f, "render produced NaN / infinity"),
        }
    }
}

/// Render timing for one instance, written by the audio thread (relaxed atomics, no locks)
/// and read by anyone holding the `Arc`.
///
/// An **overrun** is a render that took longer than `budget` of the block's real-time
/// duration (default 50%: one plugin eating half the cycle leaves too little for the rest of
/// the mix). A **deadline miss** is a render longer than the whole block: that alone
/// guarantees an audible dropout.
#[derive(Debug)]
pub struct PluginStats {
    pub blocks: AtomicU64,
    pub frames: AtomicU64,
    /// Sum of render times, ns.
    pub total_ns: AtomicU64,
    pub max_ns: AtomicU64,
    pub last_ns: AtomicU64,
    pub overruns: AtomicU64,
    pub deadline_misses: AtomicU64,
    pub errors: AtomicU64,
    /// Overrun threshold as a fraction of the block, in 1/1000 (500 = 50%).
    pub budget_permille: AtomicU32,
}

impl Default for PluginStats {
    fn default() -> Self {
        PluginStats {
            blocks: AtomicU64::new(0),
            frames: AtomicU64::new(0),
            total_ns: AtomicU64::new(0),
            max_ns: AtomicU64::new(0),
            last_ns: AtomicU64::new(0),
            overruns: AtomicU64::new(0),
            deadline_misses: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            budget_permille: AtomicU32::new(500),
        }
    }
}

/// A copy of [`PluginStats`] at one moment.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StatsSnapshot {
    pub blocks: u64,
    pub frames: u64,
    pub mean_us: f64,
    pub max_us: f64,
    pub last_us: f64,
    pub overruns: u64,
    pub deadline_misses: u64,
    pub errors: u64,
    /// Mean render time as a share of real time (0.1 = the plugin uses 10% of one core).
    pub cpu: f64,
}

impl PluginStats {
    pub fn snapshot(&self, sample_rate: f64) -> StatsSnapshot {
        let blocks = self.blocks.load(Relaxed);
        let frames = self.frames.load(Relaxed);
        let total = self.total_ns.load(Relaxed) as f64;
        StatsSnapshot {
            blocks,
            frames,
            mean_us: if blocks > 0 { total / blocks as f64 / 1000.0 } else { 0.0 },
            max_us: self.max_ns.load(Relaxed) as f64 / 1000.0,
            last_us: self.last_ns.load(Relaxed) as f64 / 1000.0,
            overruns: self.overruns.load(Relaxed),
            deadline_misses: self.deadline_misses.load(Relaxed),
            errors: self.errors.load(Relaxed),
            cpu: if frames > 0 { total / 1e9 / (frames as f64 / sample_rate) } else { 0.0 },
        }
    }

    pub fn reset(&self) {
        for a in [&self.blocks, &self.frames, &self.total_ns, &self.max_ns, &self.last_ns, &self.overruns, &self.deadline_misses, &self.errors] {
            a.store(0, Relaxed);
        }
    }

    #[inline]
    fn record(&self, ns: u64, frames: usize, sample_rate: f64) -> bool {
        self.blocks.fetch_add(1, Relaxed);
        self.frames.fetch_add(frames as u64, Relaxed);
        self.total_ns.fetch_add(ns, Relaxed);
        self.last_ns.store(ns, Relaxed);
        self.max_ns.fetch_max(ns, Relaxed);
        let block_ns = frames as f64 * 1e9 / sample_rate;
        let over = ns as f64 > block_ns * self.budget_permille.load(Relaxed) as f64 / 1000.0;
        if over {
            self.overruns.fetch_add(1, Relaxed);
        }
        if ns as f64 > block_ns {
            self.deadline_misses.fetch_add(1, Relaxed);
        }
        over
    }
}

/// How long each stage of a load took.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LoadTimes {
    pub instantiate: Duration,
    pub initialize: Duration,
    pub restore: Duration,
}

impl LoadTimes {
    pub fn total(&self) -> Duration {
        self.instantiate + self.initialize + self.restore
    }
}

/// A loaded, initialised instrument rendering non-interleaved stereo f32.
///
/// Real-time contract: [`PluginInstance::midi`] and [`PluginInstance::render`] take `&mut
/// self`, never allocate, lock or message Objective-C on the host side, so the audio
/// callback may call them. Everything else (state, reconfigure, drop) is for a control
/// thread while the instance is not playing. What the plugin does inside its own render is
/// the plugin's business: [`PluginStats`] measures it.
pub struct PluginInstance {
    unit: Arc<Unit>,
    info: PluginInfo,
    sample_rate: f64,
    max_frames: u32,
    sample_time: f64,
    out_of_process: bool,
    stats: Arc<PluginStats>,
    load_times: LoadTimes,
}

impl PluginInstance {
    pub(crate) fn new(unit: Unit, info: PluginInfo, sample_rate: f64, max_frames: u32, out_of_process: bool, load_times: LoadTimes) -> Self {
        PluginInstance {
            unit: Arc::new(unit),
            info,
            sample_rate,
            max_frames,
            sample_time: 0.0,
            out_of_process,
            stats: Arc::new(PluginStats::default()),
            load_times,
        }
    }

    /// Tests: make the host believe the unit takes longer slices than it was configured for,
    /// so its render fails like a misbehaving plugin's.
    #[cfg(test)]
    pub(crate) fn force_max_frames(&mut self, n: u32) {
        self.max_frames = n;
    }

    pub(crate) fn with_load_times(mut self, t: LoadTimes) -> Self {
        self.load_times = t;
        self
    }

    pub fn info(&self) -> &PluginInfo {
        &self.info
    }
    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }
    pub fn max_frames(&self) -> u32 {
        self.max_frames
    }
    /// Loaded in the AUHostingService (its own process) rather than in yahaha's.
    pub fn out_of_process(&self) -> bool {
        self.out_of_process
    }
    pub fn load_times(&self) -> LoadTimes {
        self.load_times
    }
    /// Render timing, shared with whoever wants to watch it.
    pub fn stats(&self) -> Arc<PluginStats> {
        self.stats.clone()
    }

    /// Queue one short MIDI message `offset` frames into the next render (which must be
    /// longer than `offset`). RT-safe.
    #[inline]
    pub fn midi(&mut self, m: [u8; 3], offset: u32) -> Result<(), RenderError> {
        match self.unit.midi(m, offset) {
            0 => Ok(()),
            st => Err(RenderError::Status(st)),
        }
    }

    /// Render `left.len()` frames (split into `max_frames` slices if longer) into the two
    /// buffers, replacing their contents. RT-safe on the host side. Timing goes to
    /// [`PluginStats`]; a failed or non-finite render zeroes the output and returns the error.
    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) -> Result<(), RenderError> {
        let frames = left.len().min(right.len());
        let t0 = Instant::now();
        let mut done = 0;
        let mut result = Ok(());
        while done < frames {
            let n = (frames - done).min(self.max_frames as usize);
            let st = self.unit.render(self.sample_time, &mut left[done..], &mut right[done..], n);
            self.sample_time += n as f64;
            if st != 0 {
                result = Err(RenderError::Status(st));
                break;
            }
            done += n;
        }
        if result.is_ok() {
            // Cheap next to any synth's render, and a NaN reaching the soft clipper would
            // silence the whole mix.
            let sum: f32 = left[..frames].iter().chain(&right[..frames]).fold(0.0, |a, x| a + x * 0.0);
            if !sum.is_finite() {
                result = Err(RenderError::NonFinite);
            }
        }
        if result.is_err() {
            left[..frames].fill(0.0);
            right[..frames].fill(0.0);
            self.stats.errors.fetch_add(1, Relaxed);
        }
        self.stats.record(t0.elapsed().as_nanos() as u64, frames, self.sample_rate);
        result
    }

    /// Render and report whether this block was an overrun (for the rack's events).
    #[inline]
    pub(crate) fn render_timed(&mut self, left: &mut [f32], right: &mut [f32]) -> (Result<(), RenderError>, bool) {
        let before = self.stats.overruns.load(Relaxed);
        let r = self.render(left, right);
        (r, self.stats.overruns.load(Relaxed) != before)
    }

    /// Render a little silence off the audio thread, so whatever a plugin does lazily on its
    /// first render (allocating voices, paging in a wavetable) happens here and not in the
    /// first block after a swap. The load does this before handing the instance over.
    pub fn prime(&mut self) {
        let n = (self.max_frames as usize).min(512);
        let (mut l, mut r) = (vec![0f32; n], vec![0f32; n]);
        for _ in 0..2 {
            let _ = self.render(&mut l, &mut r);
        }
        self.stats.reset();
    }

    /// The plugin's reported processing latency (lookahead), in seconds.
    pub fn latency_seconds(&self) -> f64 {
        self.unit.latency_seconds()
    }

    /// The complete plugin state (the current preset and every parameter) as bytes: a
    /// binary property list, what a Registration Memory slot stores for this part. Not
    /// RT-safe.
    ///
    /// Call the state functions from a control thread, **not the main thread**, while the
    /// main thread runs its loop: Kontakt's restore waits for work it queues on the main
    /// thread and deadlocks if called there.
    pub fn get_state(&self) -> Result<Vec<u8>> {
        sys::guard("reading the state", || self.unit.class_info())
    }

    /// Restore a state from [`PluginInstance::get_state`]. Not RT-safe: call it off the audio
    /// thread on an instance that is not playing (a preloaded one), then swap it in.
    pub fn set_state(&mut self, bytes: &[u8]) -> Result<()> {
        sys::guard("restoring the state", || self.unit.set_class_info(bytes))
    }

    /// Change the sample rate or maximum block size (a device change). Not RT-safe: the
    /// instance must not be playing.
    pub fn reconfigure(&mut self, sample_rate: f64, max_frames: u32) -> Result<()> {
        let unit = Arc::get_mut(&mut self.unit).ok_or_else(|| anyhow::anyhow!("close the plugin's editor before changing its sample rate"))?;
        unit.uninitialize();
        unit.configure_and_initialize(sample_rate, max_frames)?;
        self.sample_rate = sample_rate;
        self.max_frames = max_frames;
        Ok(())
    }

    /// What the editor window needs: a second handle on the unit and a title. `Send`, so a
    /// control thread can pass it to the main thread.
    pub fn editor_target(&self) -> EditorTarget {
        EditorTarget { unit: self.unit.clone(), title: self.info.full_name(), out_of_process: self.out_of_process }
    }
}

/// A handle for opening a plugin's editor on the main thread (see [`super::editor`]). It
/// keeps the Audio Unit alive while the window is open, even if the instance is retired.
#[derive(Clone)]
pub struct EditorTarget {
    pub(crate) unit: Arc<Unit>,
    pub title: String,
    pub out_of_process: bool,
}

impl EditorTarget {
    /// The plugin's current state, as [`PluginInstance::get_state`], read through this
    /// handle while the instance plays (as a DAW saves a project during playback). Control
    /// thread, not the main thread (see `get_state`).
    pub fn state(&self) -> Result<Vec<u8>> {
        sys::guard("reading the state", || self.unit.class_info())
    }

    /// A reference to this instance that does not keep it alive: to recognise it later
    /// ([`InstanceRef::is`]) without being the one that disposes of it.
    pub fn instance(&self) -> InstanceRef {
        InstanceRef(Arc::downgrade(&self.unit))
    }
}

/// Which instance an [`EditorTarget`] is, without keeping its Audio Unit alive.
#[derive(Clone)]
pub struct InstanceRef(std::sync::Weak<Unit>);

impl InstanceRef {
    /// Whether `target` is this instance (not just the same plugin: a part that loaded the
    /// plugin again has a new instance).
    pub fn is(&self, target: &EditorTarget) -> bool {
        std::sync::Weak::ptr_eq(&self.0, &Arc::downgrade(&target.unit))
    }
}

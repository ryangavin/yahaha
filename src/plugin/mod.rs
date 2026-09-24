//! Plugin hosting (#35): Audio Unit instruments for the keyboard parts (and later the Style
//! parts), behind the `plugins` cargo feature, off by default.
//!
//! Phase 1 is the library layer; nothing in the engine, session or built-in synth uses it
//! yet (docs/plugin-hosting.md has the phase 2 wiring plan). `yahaha plugin-test` exercises
//! it end to end.
//!
//! - [`PluginHost`]: [`PluginHost::scan`] the installed instruments (cached on disk) and
//!   [`PluginHost::load_async`] one off the audio thread, with progress, a timeout and
//!   AUv2 / AUv3 (out-of-process) support.
//! - [`PluginInstance`]: MIDI with sample offsets in, stereo out at a given rate and block
//!   size, [`PluginInstance::get_state`] / [`PluginInstance::set_state`] as bytes, render
//!   timing in [`PluginStats`].
//! - [`PluginRack`] / [`RackControl`]: preloaded instances assigned to MIDI channels on the
//!   audio thread, the part's CC7/CC11 applied host-side ([`PartGain`]), crossfaded swaps,
//!   faulted plugins muted and reported.
//! - [`editor`]: the plugin's window, on the main thread.
//! - `sys`: the thin unsafe core over the objc2 AudioToolbox bindings.

pub mod cli;
pub mod editor;
mod host;
mod instance;
mod rack;
mod scan;
mod sys;

pub use host::{LoadConfig, LoadHandle, LoadMode, LoadProgress, LoadTimedOut, PluginHost};
pub use sys::{StatusError, status_of};
pub use instance::{EditorTarget, LoadTimes, PluginInstance, PluginStats, RenderError, StatsSnapshot};
pub use rack::{DEFAULT_FADE_FRAMES, PluginRack, RackControl, RackEvent, SLOTS, Swap, balance, dispose_later, rack};
pub use scan::{LoadRecord, PluginFormat, PluginId, PluginInfo, default_cache_path};

/// Whether a failed **out-of-process** load may be retried in process: only when the
/// system refused to host the unit out of process at all, i.e. `AudioComponentInstantiate`
/// failed with `kAudioComponentErr_NotPermitted` (-66748) or `kAudioComponentErr_UnsupportedType`
/// (-66751). Never after a timeout, a crash or invalidation of the hosting process
/// (-66749 and the rest), or a failure later in the load (initialising, restoring the
/// state): the plugin itself failed there, and loading it in process would only bring that
/// failure (or crash) into yahaha. Typed: it reads [`StatusError`] / [`LoadTimedOut`], never
/// the message text.
pub fn may_retry_in_process(e: &anyhow::Error) -> bool {
    if e.chain().any(|c| c.is::<LoadTimedOut>()) {
        return false;
    }
    matches!(status_of(e), Some(StatusError { status: -66748 | -66751, what: "AudioComponentInstantiate" }))
}

/// The mixer rule for a part played by a plugin: its CC7 (and CC11) stays the only
/// per-part volume, applied by the host on the plugin's output with exactly the curve the
/// built-in synth uses (rustysynth: `gain = ((vol/16383)·(expr/16383))²` over the 14-bit
/// controller values, MSB = CC7 / CC11, LSB = CC39 / CC43, power-on 100 and 127). The
/// messages are not forwarded, because plugins disagree about CC7 (some ignore it, some map
/// it to a macro, some apply their own curve); doing it here makes a part equally loud
/// whichever engine plays it.
///
/// The gain ramps over one block so fader moves don't click. RT-safe: no allocation.
#[derive(Clone, Debug)]
pub struct PartGain {
    volume: u16,
    expression: u16,
    current: f32,
}

/// A linear ramp over one block, from [`PartGain::ramp`].
#[derive(Clone, Copy, Debug)]
pub struct Ramp {
    start: f32,
    step: f32,
}

impl Ramp {
    /// The gain for frame `i` of the block (reaches the target on the last frame).
    #[inline]
    pub fn at(&self, i: usize) -> f32 {
        self.start + self.step * (i + 1) as f32
    }
}

impl Default for PartGain {
    fn default() -> Self {
        Self::new()
    }
}

impl PartGain {
    /// GM power-on: CC7 = 100, CC11 = 127 (as rustysynth's channel reset).
    pub const fn new() -> Self {
        PartGain { volume: 100 << 7, expression: 127 << 7, current: -1.0 }
    }

    /// The gain for 7-bit CC7 / CC11 values (LSBs 0), the GM curve.
    #[inline]
    pub fn curve(cc7: u8, cc11: u8) -> f32 {
        Self::curve14((cc7.min(127) as u16) << 7, (cc11.min(127) as u16) << 7)
    }

    #[inline]
    fn curve14(volume: u16, expression: u16) -> f32 {
        let g = (volume as f32 / 16383.0) * (expression as f32 / 16383.0);
        g * g
    }

    /// Take a channel message meant for the plugin. Returns true when it was a volume
    /// message the host consumed (so it must not be sent to the plugin). Reset All
    /// Controllers (CC121) restores CC11 only, as GM specifies (and rustysynth does), and is
    /// still forwarded.
    #[inline]
    pub fn take(&mut self, m: [u8; 3]) -> bool {
        if m[0] & 0xF0 != 0xB0 {
            return false;
        }
        let v = (m[2] & 0x7F) as u16;
        match m[1] {
            7 => self.volume = (self.volume & 0x7F) | (v << 7),
            39 => self.volume = (self.volume & !0x7F) | v,
            11 => self.expression = (self.expression & 0x7F) | (v << 7),
            43 => self.expression = (self.expression & !0x7F) | v,
            121 => {
                self.expression = 127 << 7;
                return false;
            }
            _ => return false,
        }
        true
    }

    /// The gain the controllers ask for now.
    pub fn target(&self) -> f32 {
        Self::curve14(self.volume, self.expression)
    }

    /// The ramp for the next `n` frames, from the last block's gain to the target.
    #[inline]
    pub fn ramp(&mut self, n: usize) -> Ramp {
        let target = self.target();
        // First block: start at the target (no fade-in from silence).
        let start = if self.current < 0.0 { target } else { self.current };
        self.current = target;
        Ramp { start, step: if n == 0 { 0.0 } else { (target - start) / n as f32 } }
    }

    /// Scale a rendered stereo buffer by the ramp.
    #[inline]
    pub fn apply(&mut self, left: &mut [f32], right: &mut [f32]) {
        let n = left.len().min(right.len());
        let ramp = self.ramp(n);
        for i in 0..n {
            let g = ramp.at(i);
            left[i] *= g;
            right[i] *= g;
        }
    }
}

#[cfg(test)]
mod tests;

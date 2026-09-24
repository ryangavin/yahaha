//! Plugin hosting spike (#35): Audio Unit instruments for the keyboard parts.
//!
//! Behind the `plugins` cargo feature, off by default. Nothing in the engine, session or
//! built-in synth depends on it yet; `yahaha plugin-test` exercises it end to end. The
//! options report and the integration plan are in docs/plugin-hosting.md.
//!
//! - [`au`]: scan, load, MIDI in, render out, state save/restore (AudioToolbox C API).
//! - [`gui`]: the instrument's editor window (Cocoa view or CoreAudioKit generic view).
//! - [`PartGain`]: the host side of the mixer rule for plugin parts.
//! - [`cli`]: the `plugin-test` command.

pub mod au;
pub mod cli;
mod ffi;
pub mod gui;

pub use au::{scan, ComponentInfo, Instrument};

/// The mixer rule for a part played by a plugin: its CC7 (and CC11) stays the only
/// per-part volume, applied by the host on the plugin's output with the same GM curve the
/// built-in synth uses (`gain = ((CC7/127)·(CC11/127))²`). The messages are not forwarded,
/// because plugins disagree about CC7 (some ignore it, some map it to a macro, some apply
/// their own curve); doing it here makes a part equally loud whichever engine plays it.
///
/// The gain ramps over one buffer so fader moves don't click. RT-safe: no allocation.
#[derive(Clone, Debug)]
pub struct PartGain {
    cc7: u8,
    cc11: u8,
    current: f32,
}

impl Default for PartGain {
    fn default() -> Self {
        // GM power-on defaults: CC7 = 100, CC11 = 127.
        PartGain { cc7: 100, cc11: 127, current: Self::curve(100, 127) }
    }
}

impl PartGain {
    #[inline]
    pub fn curve(cc7: u8, cc11: u8) -> f32 {
        let g = (cc7.min(127) as f32 / 127.0) * (cc11.min(127) as f32 / 127.0);
        g * g
    }

    /// Take a channel message meant for the plugin. Returns true when it was a volume
    /// message the host consumed (so it must not be sent to the plugin). Reset All
    /// Controllers (CC121) restores CC11 only, as GM specifies, and is still forwarded.
    #[inline]
    pub fn take(&mut self, m: [u8; 3]) -> bool {
        if m[0] & 0xF0 != 0xB0 {
            return false;
        }
        match m[1] {
            7 => { self.cc7 = m[2]; true }
            11 => { self.cc11 = m[2]; true }
            121 => { self.cc11 = 127; false }
            _ => false,
        }
    }

    pub fn target(&self) -> f32 {
        Self::curve(self.cc7, self.cc11)
    }

    /// Scale a rendered stereo buffer, ramping linearly from the last gain to the target.
    #[inline]
    pub fn apply(&mut self, left: &mut [f32], right: &mut [f32]) {
        let target = self.target();
        let n = left.len().min(right.len());
        if n == 0 {
            return;
        }
        let step = (target - self.current) / n as f32;
        let mut g = self.current;
        for i in 0..n {
            g += step;
            left[i] *= g;
            right[i] *= g;
        }
        self.current = target;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn part_gain_follows_the_gm_curve_and_swallows_volume() {
        let mut g = PartGain::default();
        assert!(g.take([0xB0, 7, 127]));
        assert!(g.take([0xB0, 11, 127]));
        assert_eq!(g.target(), 1.0);
        assert!(g.take([0xB3, 7, 64]));
        assert!((g.target() - (64.0f32 / 127.0).powi(2)).abs() < 1e-6);
        assert!(!g.take([0xB0, 1, 20]), "mod wheel goes to the plugin");
        assert!(!g.take([0x90, 7, 100]), "a note-on is not CC7");
        let (mut l, mut r) = (vec![1.0f32; 64], vec![1.0f32; 64]);
        g.take([0xB0, 7, 0]);
        g.apply(&mut l, &mut r);
        assert!(l[0] > 0.0 && l[63].abs() < 1e-6, "ramps down within the buffer");
    }
}

//! The metronome's click voice: a short, bright blip the built-in synth mixes into its
//! output (a Genos metronome sounds on the instrument only). It is not a MIDI part: it
//! never reaches the MIDI port, takes no channel from the style or the keyboard parts,
//! and has its own volume (`SynthControl::click_volume`).
//!
//! The engine asks for a click with `Sink::click`; `live::Out` sends it to the synth's
//! ring only, as [`CLICK`] (a status byte no channel message uses). Real-time safe: no
//! allocation, plain arithmetic.

/// The synth-ring message that sounds a click: `[CLICK, accent, 0]` (accent 1 = the bell
/// on the first beat of a bar). 0xF9 is an undefined MIDI System Real-Time byte.
pub const CLICK: u8 = 0xF9;

/// Default metronome volume (0-127).
pub const DEFAULT_VOLUME: u8 = 90;

/// One click at a time; a new one restarts it.
#[derive(Clone, Copy, Debug)]
pub struct Click {
    sample_rate: f32,
    phase: f32,
    step: f32,
    env: f32,
    decay: f32,
    gain: f32,
}

impl Click {
    pub fn new(sample_rate: u32) -> Click {
        Click { sample_rate: sample_rate.max(1) as f32, phase: 0.0, step: 0.0, env: 0.0, decay: 0.0, gain: 0.0 }
    }

    /// The gain of a click at metronome volume `v` (0-127): a squared curve, as a CC7,
    /// peaking at -6 dBFS.
    pub fn gain_of(v: u8) -> f32 {
        let x = v.min(127) as f32 / 127.0;
        0.5 * x * x
    }

    /// Start a click: the bell (a higher, longer tone) on the first beat, else the tick.
    pub fn trigger(&mut self, accent: bool, volume: u8) {
        let (freq, tau) = if accent { (1760.0, 0.035) } else { (1320.0, 0.018) };
        self.phase = 0.0;
        self.step = std::f32::consts::TAU * freq / self.sample_rate;
        self.env = 1.0;
        self.decay = (-1.0 / (tau * self.sample_rate)).exp();
        self.gain = Click::gain_of(volume);
    }

    /// Sounding: not yet died away.
    pub fn active(&self) -> bool {
        self.env * self.gain > 1e-5
    }

    /// Add the click to a stereo buffer (centre), scaled by `level` (the master level).
    pub fn render_add(&mut self, left: &mut [f32], right: &mut [f32], level: f32) {
        if !self.active() {
            return;
        }
        let g = self.gain * level;
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let s = self.phase.sin() * self.env * g;
            *l += s;
            *r += s;
            self.phase += self.step;
            if self.phase > std::f32::consts::TAU {
                self.phase -= std::f32::consts::TAU;
            }
            self.env *= self.decay;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peak(c: &mut Click, frames: usize) -> f32 {
        let (mut l, mut r) = (vec![0.0; frames], vec![0.0; frames]);
        c.render_add(&mut l, &mut r, 1.0);
        l.iter().fold(0f32, |m, x| m.max(x.abs()))
    }

    #[test]
    fn a_click_sounds_and_dies_away() {
        let mut c = Click::new(48_000);
        assert_eq!(peak(&mut c, 64), 0.0);
        c.trigger(false, 127);
        let p = peak(&mut c, 480);
        assert!(p > 0.3 && p <= 0.5, "{p}");
        peak(&mut c, 48_000);
        assert!(!c.active());
    }

    #[test]
    fn volume_scales_it() {
        let mut loud = Click::new(48_000);
        let mut soft = Click::new(48_000);
        loud.trigger(true, 127);
        soft.trigger(true, 40);
        assert!(peak(&mut soft, 480) < peak(&mut loud, 480) / 4.0);
        let mut off = Click::new(48_000);
        off.trigger(true, 0);
        assert_eq!(peak(&mut off, 480), 0.0);
    }
}

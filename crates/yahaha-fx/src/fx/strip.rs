//! The channel strip's chain on the SoundFont side (the mixer rework): every MIDI
//! channel's compressor, insert 1 and insert 2, run in that order on the part's stem by the
//! synth's rack, after the part's EQ and before its meters, sends and the mix.
//!
//! [`StripControl`] (in `FxControl::strips`) carries each channel's compressor and insert
//! slots from the control side: atomics only.

use super::insert::{Insert, InsertSettings};

/// Each channel's strip settings from the control side, read once per buffer by the audio
/// thread: atomics only.
pub struct StripControl {}

impl StripControl {
    pub fn new() -> StripControl {
        StripControl {}
    }
}

impl Default for StripControl {
    fn default() -> StripControl {
        StripControl::new()
    }
}

/// Every MIDI channel's insert on the SoundFont side, each run on its part's stem by the
/// synth's rack (audio thread; allocated in `new`): the Style parts' (channels 9-16) and
/// the keyboard parts' (channels 1-4).
///
/// Each channel has two slots (`super::INSERT_SLOTS`, the mixer rework's strip: insert 1
/// then insert 2), both allocated here. Slot 2 is taken (`set_second`) but not run yet:
/// it passes the stem through until the chaining lands (the mixer rework's lane A).
pub struct ChannelInserts {
    slots: [Insert; 16],
    settings: [InsertSettings; 16],
    /// Slot 2 of each channel: allocated, not run yet.
    #[allow(dead_code)]
    second: [Insert; 16],
    second_settings: [InsertSettings; 16],
}

impl ChannelInserts {
    pub fn new(rate: f32) -> ChannelInserts {
        ChannelInserts {
            slots: std::array::from_fn(|_| Insert::new(rate)),
            settings: [InsertSettings::NONE; 16],
            second: std::array::from_fn(|_| Insert::new(rate)),
            second_settings: [InsertSettings::NONE; 16],
        }
    }

    /// Take the settings for this buffer (`InsertSettings::channels`).
    pub fn set(&mut self, settings: &[InsertSettings; 16]) {
        self.settings = *settings;
    }

    /// Take slot 2's settings for this buffer. Stub: slot 2 passes through for now.
    pub fn set_second(&mut self, settings: &[InsertSettings; 16]) {
        self.second_settings = *settings;
    }

    /// Slot 2's settings as last taken.
    pub fn second(&self) -> &[InsertSettings; 16] {
        &self.second_settings
    }

    /// The channels (bit = channel) whose stem runs through `process` before the mix.
    pub fn mask(&self) -> u16 {
        let mut m = 0;
        for (ch, (slot, s)) in self.slots.iter().zip(&self.settings).enumerate() {
            if slot.active(s.kind) {
                m |= 1 << ch;
            }
        }
        m
    }

    /// Run channel `channel`'s stem (`left`/`right`) through its effect in place. `level`
    /// is the part's gain in the mix (volume x expression, squared, x the master volume).
    pub fn process(&mut self, channel: usize, left: &mut [f32], right: &mut [f32], level: f32) {
        let Some(slot) = self.slots.get_mut(channel) else { return };
        // The top view (#296) shows the Style parts' inserts.
        let band = channel.checked_sub(super::BAND_CHANNELS.start).filter(|&p| p < 8);
        let perf = &yahaha_core::perf::PERF;
        let t0 = band.is_some_and(|_| perf.on()).then(yahaha_core::rt::host_now);
        slot.process(left, right, level, &self.settings[channel]);
        // Its time and output peak, atomics only.
        if let (Some(t0), Some(p)) = (t0, band) {
            perf.insert[p].add(yahaha_core::rt::host_to_ns(yahaha_core::rt::host_now().wrapping_sub(t0)));
            let peak = left.iter().chain(right.iter()).fold(0f32, |m, x| m.max(x.abs()));
            yahaha_core::perf::Perf::peak(&perf.insert_peak[p], peak);
        }
    }
}

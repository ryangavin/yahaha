//! The audio device's output stream around an [`AudioCore`], and changing its buffer size
//! (`SetAudioBuffer`, #104) without losing the core.
//!
//! The core is not moved into the cpal callback. It lives in a [`CoreSlot`]: the callback
//! takes it with one atomic swap, renders, and puts it back. To change the buffer size the
//! control side takes the core out of the slot (the callback then plays silence), drops
//! the old stream, and opens a new one around the same core. The core keeps everything:
//! the SoundFont voices, the MIDI rings (messages sent meanwhile wait in them, so no
//! note-off is lost and nothing sticks), the plugin rack and its instances. Plugins are
//! loaded for `PLUGIN_MAX_BLOCK` frames, more than any buffer offered here, and the
//! sample rate does not change, so the instances need no reload.

use super::AudioCore;
use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, StreamTrait};
use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, Ordering::{Acquire, Release}};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The buffer sizes `SetAudioBuffer` offers, in frames. 64 is the default: 1.3 ms at
/// 48 kHz. Heavy plugins may need 128 or 256.
pub const BUFFER_CHOICES: [u32; 3] = [64, 128, 256];

/// The buffer size a stream asks for when nothing else is chosen.
pub const DEFAULT_BUFFER: u32 = 64;

// The callback used to own the core outright; the slot hides that from the compiler, so
// keep the core `Send` on purpose.
const _: fn() = || {
    fn send<T: Send>() {}
    send::<AudioCore>();
};

/// Owns the [`AudioCore`] whenever the audio callback is not rendering with it.
struct CoreSlot(AtomicPtr<AudioCore>);

impl CoreSlot {
    fn new(core: Box<AudioCore>) -> Arc<CoreSlot> {
        Arc::new(CoreSlot(AtomicPtr::new(Box::into_raw(core))))
    }

    /// Take the core (None: the callback has it, or it is gone). No allocation.
    fn take(&self) -> Option<Box<AudioCore>> {
        let p = self.0.swap(null_mut(), Acquire);
        // SAFETY: a non-null pointer in the slot came from `Box::into_raw` and the swap
        // gave this caller the only copy of it.
        (!p.is_null()).then(|| unsafe { Box::from_raw(p) })
    }

    /// Put the core back (the slot is empty: whoever calls this took it). No allocation.
    fn put(&self, core: Box<AudioCore>) {
        let old = self.0.swap(Box::into_raw(core), Release);
        debug_assert!(old.is_null());
    }
}

impl Drop for CoreSlot {
    fn drop(&mut self) {
        drop(self.take());
    }
}

/// An open output stream playing an [`AudioCore`].
pub(super) struct Output {
    device: cpal::Device,
    channels: u16,
    sample_rate: u32,
    /// The device's buffer size range, in frames (None: it doesn't say).
    range: Option<(u32, u32)>,
    /// Before `slot`: the stream (and its callback) goes first.
    stream: cpal::Stream,
    slot: Arc<CoreSlot>,
    /// The buffer size asked for (None: the device default).
    pub(super) buffer: Option<u32>,
}

/// The buffer size to ask a device for: `want`, within the device's range.
pub(super) fn fit(range: Option<(u32, u32)>, want: u32) -> Option<u32> {
    range.map(|(min, max)| want.clamp(min, max.max(min)))
}

impl Output {
    /// Open `device` at `sample_rate` with `channels` outputs and play `core`, asking for
    /// `want` frames per buffer.
    pub(super) fn open(device: cpal::Device, channels: u16, sample_rate: u32, range: Option<(u32, u32)>, core: AudioCore, want: u32) -> Result<Output> {
        let slot = CoreSlot::new(Box::new(core));
        let buffer = fit(range, want);
        let stream = play(&device, channels, sample_rate, buffer, slot.clone())?;
        Ok(Output { device, channels, sample_rate, range, stream, slot, buffer })
    }

    /// Reopen the stream with `frames` per buffer (within the device's range); the core
    /// carries over. Returns the buffer size now asked for. If the device refuses it, the
    /// stream is reopened as it was and the error returned.
    pub(super) fn set_buffer(&mut self, frames: u32) -> Result<Option<u32>> {
        let want = fit(self.range, frames);
        if want == self.buffer {
            return Ok(want);
        }
        // Take the core from the callback: it renders at most one more buffer with it.
        let deadline = Instant::now() + Duration::from_secs(1);
        let core = loop {
            if let Some(c) = self.slot.take() {
                break c;
            }
            if Instant::now() > deadline {
                return Err(anyhow!("the audio callback did not hand the synth back"));
            }
            std::thread::sleep(Duration::from_millis(1));
        };
        // The old stream plays silence from here and is closed; a fresh slot for the new
        // one, so the old callback can never see the core again.
        let _ = self.stream.pause();
        let slot = CoreSlot::new(core);
        let old = std::mem::replace(&mut self.slot, slot.clone());
        let (stream, buffer, err) = match play(&self.device, self.channels, self.sample_rate, want, slot.clone()) {
            Ok(s) => (Some(s), want, None),
            Err(e) => (None, self.buffer, Some(e)),
        };
        let stream = match stream {
            Some(s) => s,
            // Back to the size that worked.
            None => play(&self.device, self.channels, self.sample_rate, self.buffer, slot)?,
        };
        drop(std::mem::replace(&mut self.stream, stream));
        drop(old);
        self.buffer = buffer;
        match err {
            Some(e) => Err(e),
            None => Ok(buffer),
        }
    }
}

/// Build and start a stream whose callback renders the core in `slot`.
fn play(device: &cpal::Device, channels: u16, sample_rate: u32, buffer: Option<u32>, slot: Arc<CoreSlot>) -> Result<cpal::Stream> {
    let cfg = cpal::StreamConfig {
        channels,
        sample_rate,
        buffer_size: buffer.map(cpal::BufferSize::Fixed).unwrap_or(cpal::BufferSize::Default),
    };
    let callback = move |out: &mut [f32], _: &cpal::OutputCallbackInfo| match slot.take() {
        Some(mut core) => {
            core.process(out);
            slot.put(core);
        }
        None => out.fill(0.0),
    };
    let stream = device.build_output_stream(cfg, callback, |e| eprintln!("audio error: {e}"), None)?;
    stream.play()?;
    Ok(stream)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_fits_the_device_range() {
        assert_eq!(fit(Some((14, 4096)), 128), Some(128));
        assert_eq!(fit(Some((128, 4096)), 64), Some(128));
        assert_eq!(fit(Some((14, 96)), 256), Some(96));
        assert_eq!(fit(None, 64), None);
    }
}

//! Synchro Stop Window (RM p.12): with SYNC STOP on and a window set, a chord held longer
//! than the window turns Sync Stop off, so letting go no longer stops the band. A quicker
//! release stops it, as Sync Stop always does.
//!
//! The hold is timed from the first chord after the chord section was last let go (a
//! change of chord without letting go keeps timing).

use super::*;

/// Engine-side Synchro Stop Window state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct SyncWindow {
    /// When the chord section's keys went down (None: all up).
    held_since: Option<u64>,
}

impl Engine {
    /// A chord was played at `now`.
    #[inline]
    pub(super) fn sync_window_chord(&mut self, now: u64) {
        let w = &mut self.features.sync_window;
        if w.held_since.is_none() {
            w.held_since = Some(now);
        }
    }

    /// Every chord-section key was let go.
    #[inline]
    pub(super) fn sync_window_released(&mut self) {
        self.features.sync_window.held_since = None;
    }

    /// When the window runs out, while it can cancel Sync Stop.
    pub(super) fn sync_window_deadline(&self) -> Option<u64> {
        let ms = self.features.settings.sync_stop_window_ms;
        let since = self.features.sync_window.held_since?;
        (ms > 0 && self.sync_stop && self.running).then(|| since + ms as u64 * 1_000_000)
    }

    /// The window has run out with the chord still held: Sync Stop cancels itself.
    pub(super) fn sync_window_wake(&mut self, now: u64) {
        if self.sync_window_deadline().is_some_and(|d| now >= d) {
            self.sync_stop = false;
        }
    }
}

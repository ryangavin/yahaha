//! The warm pool (#104, docs/plugin-hosting.md "4. Registration Memory"): the plugins a
//! Registration bank's buttons play, preloaded with their stored states when the bank is
//! selected, so a button press hands a ready instance to the rack instead of loading one.
//!
//! - The registration side says what it wants (`warm_plugins`): each plugin voice (id and
//!   state) its buttons use, as many of one voice as a single button plays at once, in
//!   button order, at most [`MAX_WARM`].
//! - Each is loaded as a part loads it (`start_load`: on a `plugin-load` thread, never on
//!   the control, engine, MIDI or audio threads) and waits here, not in the rack: it costs
//!   memory (and, out of process, a helper process), no audio-thread time.
//! - `assign_channel_plugin` takes a matching one (`take_warm`); the pump then assigns it as
//!   it does a finished load, at its next tick. The registration side asks again after a
//!   recall, so the pool refills what a press used.
//! - What the bank no longer wants goes: an instance is disposed of on `plugin-dispose`
//!   (never on the control thread); a load still running is abandoned (its thread disposes
//!   of what it made). A load that fails is not retried until the bank asks again.

use super::PluginVoice;
use crate::plugin::{dispose_later, LoadHandle, LoadMode, LoadProgress, PluginInfo};
use crate::session::Control;

/// The most instances kept warm (Decision: a bank is ten buttons; the first eight plugin
/// voices in button order are preloaded, the rest load when pressed, showing Loading).
pub(crate) const MAX_WARM: usize = 8;

/// One preloaded plugin voice.
pub(crate) struct Warm {
    pub(crate) voice: PluginVoice,
    /// The sample rate it loads at (a device change makes it useless).
    rate: f64,
    pub(crate) info: PluginInfo,
    pub(crate) mode: LoadMode,
    pub(crate) load: LoadHandle,
}

#[derive(Default)]
pub(crate) struct WarmPool {
    pub(crate) entries: Vec<Warm>,
    /// What the bank wants warm.
    want: Vec<PluginVoice>,
    /// Voices whose preload failed: not retried until the bank asks again.
    failed: Vec<PluginVoice>,
    /// Preloaded instances handed to a part so far.
    pub(crate) used: u64,
}

/// Let a warm entry go: a finished instance is disposed of on `plugin-dispose`; a load
/// still running is abandoned (dropping its handle), and its thread disposes of it.
fn evict(w: Warm) {
    let mut load = w.load;
    if let Some(Ok(inst)) = load.take() {
        dispose_later(Box::new(inst));
    }
}

impl Control {
    /// The plugin voices the Registration bank wants warm (at most [`MAX_WARM`] are kept).
    pub(crate) fn warm_plugins(&mut self, mut want: Vec<PluginVoice>) {
        want.truncate(MAX_WARM);
        if want != self.plugins.warm.want {
            self.plugins.warm.failed.clear();
        }
        self.plugins.warm.want = want;
        self.sync_warm();
    }

    /// Bring the pool to what the bank wants: keep what matches, let the rest go, start
    /// loading what is missing.
    fn sync_warm(&mut self) {
        let rate = self.plugin_rate();
        let ready = self.plugin_ready().is_ok();
        let want = if ready { self.plugins.warm.want.clone() } else { Vec::new() };
        let mut keep: Vec<Warm> = Vec::new();
        let mut missing = Vec::new();
        let mut old = std::mem::take(&mut self.plugins.warm.entries);
        for v in want {
            match old.iter().position(|w| w.voice == v && w.rate == rate) {
                Some(i) => keep.push(old.swap_remove(i)),
                None => missing.push(v),
            }
        }
        old.into_iter().for_each(evict);
        self.plugins.warm.entries = keep;
        for v in missing {
            if self.plugins.warm.failed.contains(&v) {
                continue;
            }
            match self.start_load(&v) {
                Ok((info, mode, load)) => self.plugins.warm.entries.push(Warm { voice: v, rate, info, mode, load }),
                Err(_) => self.plugins.warm.failed.push(v),
            }
        }
    }

    /// A preloaded instance of `voice` (loaded, or still loading), for a part to play.
    pub(crate) fn take_warm(&mut self, voice: &PluginVoice) -> Option<Warm> {
        let rate = self.plugin_rate();
        let i = self.plugins.warm.entries.iter().position(|w| &w.voice == voice && w.rate == rate)?;
        self.plugins.warm.used += 1;
        Some(self.plugins.warm.entries.swap_remove(i))
    }

    /// Every pump: preloads that failed or timed out leave the pool (and are not retried
    /// until the bank asks again).
    pub(crate) fn pump_warm(&mut self) {
        let pool = &mut self.plugins.warm;
        let mut i = 0;
        while i < pool.entries.len() {
            if matches!(pool.entries[i].load.progress(), LoadProgress::Failed(_) | LoadProgress::TimedOut(_)) {
                let w = pool.entries.swap_remove(i);
                pool.failed.push(w.voice.clone());
                evict(w);
            } else {
                i += 1;
            }
        }
    }

    /// How many preloaded instances are ready (tests, the Plugins tab).
    #[allow(dead_code)]
    pub(crate) fn warm_ready(&self) -> usize {
        self.plugins.warm.entries.iter().filter(|w| matches!(w.load.progress(), LoadProgress::Ready)).count()
    }
}

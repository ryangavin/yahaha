//! Transport: the panel buttons, start and stop, Sync Start/Stop, tempo and the clock.

use super::*;

impl Engine {
    // ----- time -----

    pub(super) fn set_bpm_internal(&mut self, bpm: f64, now: u64) {
        let t = if self.ns_per_tick > 0.0 { self.tick_at(now) } else { 0.0 };
        self.bpm = bpm.clamp(MIN_BPM, MAX_BPM);
        self.anchor_ns = now;
        self.anchor_tick = t;
        self.ns_per_tick = 60e9 / (self.bpm * self.style.ppq as f64);
    }

    pub(super) fn tick_at(&self, now: u64) -> f64 {
        self.anchor_tick + (now as f64 - self.anchor_ns as f64) / self.ns_per_tick
    }

    pub(super) fn ns_at(&self, tick: f64) -> u64 {
        let v = self.anchor_ns as f64 + (tick - self.anchor_tick) * self.ns_per_tick;
        if v < 0.0 {
            0
        } else {
            v.ceil() as u64
        }
    }

    /// The fingering type allows Sync Stop or not; disallowing turns it off.
    pub fn allow_sync_stop(&mut self, on: bool) {
        self.sync_stop_allowed = on;
        self.sync_stop &= on;
    }

    /// Chord-zone keys all released (for Sync Stop).
    pub fn chord_released(&mut self, now: u64, sink: &mut impl Sink) {
        // The Chord Looper plays the chords: the keyboard's releases don't count either.
        if self.sync_stop && self.running && !self.looper_owns_chords() {
            self.stop(sink);
            self.sync_armed = true;
        }
        let _ = now;
    }

    pub fn button(&mut self, b: Button, now: u64, sink: &mut impl Sink) {
        let s = &self.style;
        match b {
            Button::StartStop => {
                if self.running {
                    self.stop(sink);
                } else {
                    self.start(now, sink);
                }
            }
            Button::Stop => {
                if self.running {
                    self.stop(sink);
                }
            }
            Button::SyncStart => {
                // The player's own choice now: cancelling REC leaves it alone.
                self.features.looper.forget_sync();
                if self.running {
                    self.stop(sink);
                    self.sync_armed = true;
                } else {
                    self.sync_armed = !self.sync_armed;
                }
            }
            Button::SyncStop => self.sync_stop = !self.sync_stop && self.sync_stop_allowed,
            Button::AutoFill => self.auto_fill = !self.auto_fill,
            Button::TogglePart(p) => {
                self.parts ^= 1 << (p & 7);
                self.silence_inaudible(sink);
            }
            Button::StopAcmp => {
                self.stop_acmp = !self.stop_acmp;
                if !self.stop_acmp {
                    self.off_where(sink, |n| n.src == STOP_ACMP_SRC);
                }
            }
            Button::TempoUp => self.set_bpm_internal(self.bpm + 2.0, now),
            Button::TempoDown => self.set_bpm_internal(self.bpm - 2.0, now),
            Button::SetTempo(bpm) => self.set_bpm_internal(bpm as f64, now),
            Button::TapTempo => self.tap(now),
            Button::Intro(i) => {
                if !self.running {
                    self.pending_intro = if self.pending_intro == Some(i) { None } else { Some(i) };
                } else if let Some(slot) = s.resolve(i as usize) {
                    self.queue_at_bar(slot, now);
                }
            }
            Button::Main(i) => {
                let prev = self.main;
                self.main = i;
                if !self.running {
                    return;
                }
                let cur_id = id_of(self.cur);
                match cur_id {
                    SectionId::Main(m) => {
                        let fill = if i == m || self.auto_fill { s.resolve(8 + i as usize) } else { None };
                        let fill = if i == m { s.resolve(8 + m as usize) } else { fill };
                        match fill {
                            Some(f) => self.queue_fill(f, now),
                            None if i != m => {
                                if let Some(slot) = s.resolve(4 + i as usize) {
                                    self.queue_at_bar(slot, now)
                                }
                            }
                            None => {}
                        }
                    }
                    SectionId::Ending(_) => {
                        if let Some(slot) = s.resolve(4 + i as usize) {
                            self.queue_at_bar(slot, now)
                        }
                    }
                    // Intro / fill / break: they flow into self.main when done.
                    _ => {
                        let _ = prev;
                    }
                }
            }
            Button::Fill(d) => {
                // The Main to the left or right (none past A or D: the fill of the Main at
                // the end), always with its fill, as if Auto Fill were on. Stopped, it
                // selects that Main.
                let target = (self.main as i8 + d.signum()).clamp(0, 3) as u8;
                let auto = std::mem::replace(&mut self.auto_fill, true);
                self.button(Button::Main(target), now, sink);
                self.auto_fill = auto;
            }
            Button::Break => {
                if self.running {
                    if let Some(slot) = s.resolve(12) {
                        self.queue_fill(slot, now);
                    }
                }
            }
            Button::Ending(i) => {
                if self.running {
                    match s.resolve(13 + i as usize) {
                        Some(slot) if slot != self.cur => self.queue_at_bar(slot, now),
                        Some(_) => {}
                        None => self.queue_stop_at_bar(now),
                    }
                }
            }
        }
    }

    /// Tap Tempo: the tempo from the last taps (up to four), down to `MIN_BPM`. Taps
    /// further apart than a beat at `MIN_BPM` (12 s) plus half a second of slack (12.5 s)
    /// start again; a tap whose interval is
    /// far from the one before (a change of mind) averages only with the tap before it.
    pub(super) fn tap(&mut self, now: u64) {
        const FORGET_NS: u64 = (60e9 / MIN_BPM) as u64 + 500_000_000;
        if self.tap_n > 0 {
            let last = self.taps[(self.tap_n - 1) % 4];
            let iv = now.saturating_sub(last);
            if iv > FORGET_NS {
                self.tap_n = 0;
            } else if self.tap_n >= 2 {
                let prev = last.saturating_sub(self.taps[(self.tap_n - 2) % 4]) as f64;
                let ratio = iv as f64 / prev.max(1.0);
                if !(1.0 / 1.5..=1.5).contains(&ratio) {
                    self.taps[0] = last;
                    self.tap_n = 1;
                }
            }
        }
        self.taps[self.tap_n % 4] = now;
        self.tap_n += 1;
        if self.tap_n >= 2 {
            let n = self.tap_n.min(4);
            let newest = self.taps[(self.tap_n - 1) % 4];
            let oldest = self.taps[(self.tap_n - n) % 4];
            let iv = (newest - oldest) as f64 / (n - 1) as f64;
            if iv > 0.0 {
                self.set_bpm_internal(60e9 / iv, now);
            }
        }
    }

    // ----- transport -----

    pub(super) fn start(&mut self, now: u64, sink: &mut impl Sink) {
        self.all_off(sink);
        self.running = true;
        self.sync_armed = false;
        self.set_bpm_internal(self.bpm, now);
        self.anchor_tick = 0.0;
        self.anchor_ns = now;
        let slot = self
            .pending_intro
            .take()
            .and_then(|i| self.style.resolve(i as usize))
            .or_else(|| self.style.resolve(4 + self.main as usize));
        let Some(slot) = slot else {
            self.running = false;
            return;
        };
        // A start plays the style's channel setup (SInt) again: parts the player has not
        // moved go back to the style's own level, so an Intro/Ending pattern's CC7 from the
        // last run does not stick. Faders the player moved keep their value.
        self.cur = slot;
        self.restore_untouched_levels();
        // The setup as the first section routes it (#64).
        self.send_init(sink);
        self.sec_start = 0.0;
        self.entry = 0.0;
        self.ev_idx = 0;
        self.queued = None;
        self.lines_from(0.0);
        self.on_start(now, sink);
        // Not `process`: a Sync Start chord settles at the wake's own `process`, after the
        // other inputs of the wake (settle.rs); until then its chord parts wait.
        self.play_due(now, sink);
    }

    /// Stop (if running) and wait for the next chord to start.
    pub fn arm(&mut self, sink: &mut impl Sink) {
        self.stop(sink);
        self.sync_armed = true;
    }

    pub fn stop(&mut self, sink: &mut impl Sink) {
        // A chord change still settling takes effect with nothing left to re-voice.
        self.settle_silently(sink);
        let was_running = self.running;
        self.running = false;
        self.queued = None;
        self.all_off(sink);
        // A style change waiting for the bar line takes over now.
        if let Some(p) = self.pending.take() {
            let old = self.load(p.style, self.anchor_ns, sink);
            self.retire(old);
        }
        if was_running {
            self.on_stop(sink);
        }
    }
}

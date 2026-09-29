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

    // ----- the clock, for real-time code beside the engine (live/kbdfx.rs) -----

    /// The style clock at `now`, in ticks of the style playing (`ppq` per quarter). It
    /// restarts at 0 on every start and re-times on a style change; stopped, it runs on
    /// from its last anchor at the current tempo.
    pub fn style_tick(&self, now: u64) -> f64 {
        self.tick_at(now)
    }

    /// When tick `tick` of the style clock comes.
    pub fn ns_at_tick(&self, tick: f64) -> u64 {
        self.ns_at(tick)
    }

    /// The tempo, in quarter notes per minute.
    pub fn bpm(&self) -> f64 {
        self.bpm
    }

    /// Ticks per quarter note of the style playing.
    pub fn ppq(&self) -> u32 {
        self.style.ppq.max(1)
    }

    /// The fingering type allows Sync Stop or not; disallowing turns it off.
    pub fn allow_sync_stop(&mut self, on: bool) {
        self.sync_stop_allowed = on;
        self.sync_stop &= on;
    }

    /// Chord-zone keys all released (for Sync Stop).
    pub fn chord_released(&mut self, now: u64, sink: &mut impl Sink) {
        // ACMP off: there is no chord section to let go of.
        if !self.acmp() {
            return;
        }
        self.sync_window_released();
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
                // Stopped: STOP also calls off a count-in from TAP TEMPO.
                self.tap_start = None;
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
            Button::StopAcmp => self.toggle_stop_acmp(sink),
            Button::SetStopAcmp(m) => self.set_stop_acmp(m, sink),
            Button::FillUp => self.fill_to(self.neighbour_main(true), now),
            Button::FillDown => self.fill_to(self.neighbour_main(false), now),
            Button::FillSelf => self.fill_to(self.main, now),
            Button::HalfBarFill => self.features.fills.half_bar = !self.features.fills.half_bar,
            Button::SetHalfBarFill(on) => self.features.fills.half_bar = on,
            Button::TempoUp => self.tempo_step(1, now),
            Button::TempoDown => self.tempo_step(-1, now),
            Button::TempoReset => self.reset_tempo(now),
            // Playing, with Style Section Reset on: rewind the section (OM p.46).
            Button::TapTempo if self.running && self.features.settings.section_reset => self.reset_section(now, sink),
            // A tempo set outright during a ritardando becomes the tempo it slows from.
            Button::SetTempo(bpm) => {
                self.set_bpm_internal(bpm as f64, now);
                self.section_tempo_retempo();
                self.rit_retempo(now);
            }
            Button::TapTempo => self.tap(now),
            Button::SectionReset => self.reset_section(now, sink),
            Button::Fade => self.fade_button(now, sink),
            Button::Retrigger => self.toggle_retrigger(),
            Button::Acmp => self.set_acmp(!self.acmp(), sink),
            Button::SetAcmp(on) => self.set_acmp(on, sink),
            Button::Unison => self.set_unison_latched(!self.unison_latched(), sink),
            Button::SetUnison(on) => self.set_unison_latched(on, sink),
            Button::UnisonHeld(on) => self.set_unison_held(on, sink),
            Button::SetUnisonType(ty) => self.set_unison_type(ty),
            Button::Intro(i) => {
                if !self.running {
                    self.pending_intro = if self.pending_intro == Some(i) { None } else { Some(i) };
                } else if let Some(slot) = s.resolve(i as usize) {
                    self.queue_at_bar(slot, now);
                }
            }
            Button::Main(i) => self.press_main(i, false, now),
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
                        // The Ending playing, pressed again: ritardando.
                        Some(_) => self.start_rit(now),
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
    ///
    /// Stopped, a bar's worth of steady taps (four in 4/4) starts the style one beat after
    /// the last tap (OM p.46): a count-in. Each further tap moves the start to a beat after
    /// it; a tap that breaks the count calls it off.
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
                self.rit_retempo(now);
            }
        }
        let beats = (self.style.tpb / self.style.ppq.max(1)).max(1) as usize;
        self.tap_start = (!self.running && self.tap_n >= beats).then(|| now + self.beat_ns());
    }

    /// A count-in from TAP TEMPO whose time has come: start, from its time. Like START
    /// with no chord, the rhythm parts play until the first chord.
    pub(super) fn tap_start_due(&mut self, now: u64, sink: &mut impl Sink) {
        if let Some(t) = self.tap_start.filter(|&t| t <= now) {
            self.tap_start = None;
            if !self.running {
                self.start(t, sink);
            }
        }
    }

    // ----- transport -----

    pub(super) fn start(&mut self, now: u64, sink: &mut impl Sink) {
        self.all_off(sink);
        self.running = true;
        self.tap_start = None;
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
        // A style change waiting for the bar line takes over now, at its own tempo: a
        // ritardando's tempo must not come back over it.
        if let Some(p) = self.pending.take() {
            self.rit_drop();
            let old = self.load(p.style, self.anchor_ns, sink);
            self.retire(old);
        }
        if was_running {
            self.on_stop(sink);
        }
    }
}

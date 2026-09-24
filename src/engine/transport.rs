//! Transport: the panel buttons, start and stop, Sync Start/Stop, tempo and the clock.

use super::*;

/// Style control states for `Engine::set_style_controls`; None leaves one as it is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StyleControls {
    /// Main A-D (0-3).
    pub main: Option<u8>,
    /// The Intro armed to start with (only while stopped).
    pub intro: Option<Option<u8>>,
    /// Sync Start armed (only while stopped).
    pub sync_start: Option<bool>,
    pub sync_stop: Option<bool>,
    pub stop_acmp: Option<bool>,
    /// The Style parts that play (bit 0 = Rhythm 1).
    pub parts: Option<u8>,
    /// The Style part levels (CC7, Rhythm 1 .. Phrase 2). Only a part whose level differs
    /// is set (as a fader move from software); a part already at its level stays the
    /// style's, so its patterns' own CC7 (Intro, Main, Ending levels) still move it.
    pub volumes: Option<[u8; 8]>,
    /// With `volumes`: the parts whose level the player had set (the Genos's Volume(Style)
    /// offset). Those are set to their level and hold it against the patterns' CC7; every
    /// other part goes back to the style (its level, then its patterns' CC7), whatever it
    /// was. None: every part in `volumes` is set where its level differs (a bank written
    /// before this was stored).
    pub player_set: Option<u8>,
}

impl Engine {
    // ----- time -----

    pub(super) fn set_bpm_internal(&mut self, bpm: f64, now: u64) {
        let t = if self.ns_per_tick > 0.0 { self.tick_at(now) } else { 0.0 };
        self.bpm = bpm.clamp(MIN_BPM, MAX_BPM);
        self.anchor_ns = now;
        self.anchor_tick = t;
        self.ns_per_tick = 60e9 / (self.bpm * self.style.ppq as f64);
    }

    /// Set the Style controls to given states (a Registration recall): each is compared
    /// with the engine's own state here, so a recall never depends on a snapshot the
    /// control side may not have seen yet (two recalls in one snapshot period would flip a
    /// toggle twice). Changes go through the same paths as the panel buttons.
    pub fn set_style_controls(&mut self, c: StyleControls, now: u64, sink: &mut impl Sink) {
        if let Some(m) = c.main.map(|m| m.min(3))
            && m != self.main
        {
            // Playing: the section changes at the next bar line, as a Main press does.
            self.button(Button::Main(m), now, sink);
        }
        if !self.running {
            if let Some(i) = c.intro {
                self.pending_intro = i.map(|i| i.min(2));
            }
            // Sync Start while playing would stop the band: only set when stopped.
            if let Some(on) = c.sync_start {
                self.sync_armed = on;
            }
        }
        if let Some(on) = c.sync_stop
            && on != self.sync_stop
        {
            self.button(Button::SyncStop, now, sink);
        }
        if let Some(on) = c.stop_acmp
            && on != self.stop_acmp
        {
            self.button(Button::StopAcmp, now, sink);
        }
        if let Some(parts) = c.parts {
            for p in 0..8u8 {
                if (self.parts ^ parts) & (1 << p) != 0 {
                    self.button(Button::TogglePart(p), now, sink);
                }
            }
        }
        if let Some(volumes) = c.volumes {
            let set = c.player_set.unwrap_or(0xFF);
            for (p, &v) in volumes.iter().enumerate() {
                let bit = 1u8 << p;
                if set & bit != 0 {
                    let v = v.min(127);
                    if self.mixer[p] != v {
                        self.set_volume_from_software(p as u8, v, sink);
                    } else if c.player_set.is_some() {
                        // Already there: it is still the player's level, not the pattern's.
                        self.user_set |= bit;
                    }
                } else if self.user_set & bit != 0 {
                    // The registration left this part to the style: forget the player's
                    // level. The style's own level now (as the section routes it, #64), its
                    // patterns' CC7 from here on.
                    self.user_set &= !bit;
                    let v = self.style.setup(self.cur).mix[p];
                    if self.mixer[p] != v {
                        self.set_mixer(p, v);
                        self.mirror.send(sink, &[0xB0 | (8 + p as u8), 7, v]);
                    }
                }
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    struct Nop;
    impl Sink for Nop {
        fn send(&mut self, _: &[u8]) {}
    }

    /// Two recalls before the control side sees a new snapshot send the same states twice:
    /// the engine compares with its own state, so nothing flips back (review N3).
    #[test]
    fn style_controls_are_states_not_toggles() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
        if !p.exists() {
            eprintln!("corpus missing; skipping");
            return;
        }
        let mut e = Engine::new(Box::new(Prepared::new(&Style::load(&p).unwrap())));
        let set = StyleControls {
            main: Some(2),
            intro: Some(Some(1)),
            sync_start: Some(true),
            sync_stop: Some(true),
            stop_acmp: Some(true),
            parts: Some(0b1101_0111),
            volumes: None,
            player_set: None,
        };
        for _ in 0..2 {
            e.set_style_controls(set, 1, &mut Nop);
            let s = e.snapshot(1);
            assert_eq!((s.main, s.pending_intro, s.sync_armed, s.sync_stop, s.stop_acmp, s.parts), (2, Some(1), true, true, true, 0b1101_0111));
        }
        // None leaves a control as it is.
        e.set_style_controls(StyleControls { parts: Some(0xff), ..StyleControls::default() }, 1, &mut Nop);
        let s = e.snapshot(1);
        assert_eq!((s.main, s.sync_stop, s.parts), (2, true, 0xff));
    }

    /// Review r2 B1: recalled levels are states too. A part already at its level is left
    /// to the style (no fader move, so its patterns' CC7 still apply); a part at another
    /// level is set, as a fader move.
    #[test]
    fn style_volumes_set_only_the_parts_that_differ() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
        if !p.exists() {
            eprintln!("corpus missing; skipping");
            return;
        }
        let mut e = Engine::new(Box::new(Prepared::new(&Style::load(&p).unwrap())));
        let mut volumes = e.mixer;
        volumes[3] = if volumes[3] == 64 { 65 } else { 64 };
        e.set_style_controls(StyleControls { volumes: Some(volumes), ..StyleControls::default() }, 1, &mut Nop);
        assert_eq!(e.mixer, volumes);
        assert_eq!(e.user_set, 1 << 3, "only the part that moved counts as the player's");
        e.set_style_controls(StyleControls { volumes: Some(volumes), ..StyleControls::default() }, 1, &mut Nop);
        assert_eq!(e.user_set, 1 << 3);
    }

    /// Review r3 B1: with `player_set`, only the player's parts are set (and hold, even at
    /// the level they already have); a part the registration left to the style goes back
    /// to the style's level and follows its patterns again, whatever the player did since.
    #[test]
    fn style_volumes_follow_the_player_set_mask() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
        if !p.exists() {
            eprintln!("corpus missing; skipping");
            return;
        }
        let mut e = Engine::new(Box::new(Prepared::new(&Style::load(&p).unwrap())));
        let own = e.style.setup(e.cur).mix;
        // The player moved parts 0 and 5.
        e.set_volume(0, 11, &mut Nop);
        e.set_volume(5, 22, &mut Nop);
        // A registration where the player had set parts 2 (at the level it has now) and 5;
        // its other levels (pattern CC7 at the time) are not the player's.
        let mut volumes = [1u8; 8];
        volumes[2] = e.mixer[2];
        volumes[5] = 33;
        let set = StyleControls { volumes: Some(volumes), player_set: Some(0b0010_0100), ..StyleControls::default() };
        e.set_style_controls(set, 1, &mut Nop);
        assert_eq!(e.user_set, 0b0010_0100, "the stored player parts hold; part 0 is the style's again");
        assert_eq!(e.mixer[0], own[0], "part 0 back at the style's level");
        assert_eq!(e.mixer[5], 33);
        for p in [1, 3, 4, 6, 7] {
            assert_eq!(e.mixer[p], own[p], "part {p}: the stored pattern level is not applied");
        }
        // Pattern CC7 moves the style's parts only.
        e.pattern_volume(8, 70, &mut Nop);
        e.pattern_volume(8 + 5, 70, &mut Nop);
        assert_eq!((e.mixer[0], e.mixer[5]), (70, 33));
    }
}

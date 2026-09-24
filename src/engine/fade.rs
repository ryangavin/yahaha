//! Fade In/Out (OM p.67, RM p.142): armed while stopped, START fades the style in; pressed
//! while it plays, the style fades out and stops, and the volume stays at 0 for the Fade
//! Out Hold Time before it comes back.
//!
//! The fade is the Style's volume (RM p.142: "the Style/Song volume"), not the
//! instrument's: your own playing (Right 1-3, Left, the Multi Pads) never fades. It is
//! made of the Style parts' CC7, the only part volume there is (the mixer principle): while
//! a fade runs, each Style part's CC7 goes out as its fader value scaled by the fade
//! position, to the port and the built-in synth alike, so what the wire shows is what
//! sounds. The faders themselves (`Engine::mixer`, the app's mixer, soft takeover) never
//! move, and when the fade ends the fader values go out again unchanged. A fader moved, or
//! a pattern CC7, during a fade goes out scaled too.
//!
//! The level follows a fader-like curve: the position moves linearly with time, and CC7
//! is a squared gain on a GM receiver. The engine sends new levels every `STEP_NS` while it
//! moves, on its own clock (`hook_wake_ns`), band running or not.

use super::*;

/// How often the level moves during a fade.
pub(super) const STEP_NS: u64 = 10_000_000;

/// What the fade is doing, as the state shows it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FadeState {
    #[default]
    Off,
    /// Stopped, armed: START fades in.
    Armed,
    FadingIn,
    FadingOut,
    /// Faded out and stopped: the volume stays at 0 for the hold time.
    Holding,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Phase {
    #[default]
    Idle,
    /// Fading in from position `from` at `start_ns`.
    In { start_ns: u64, from: f32 },
    /// Fading out from position `from` at `start_ns`.
    Out { start_ns: u64, from: f32 },
    /// At 0 until `until_ns`.
    Hold { until_ns: u64 },
}

/// Engine-side fade state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Fade {
    phase: Phase,
    /// START fades in.
    armed: bool,
    /// The fade position (the Style parts' CC7 scale), 1 = full.
    pos: f32,
    /// When the level moves next (during a fade).
    next_ns: u64,
}

impl Default for Fade {
    fn default() -> Fade {
        Fade { phase: Phase::Idle, armed: false, pos: 1.0, next_ns: 0 }
    }
}

impl Engine {
    /// FADE IN/OUT pressed (`Button::Fade`): stopped, arm or disarm the fade in; playing,
    /// fade out (from where a fade in has got to). A fade out already running carries on.
    pub(super) fn fade_button(&mut self, now: u64, sink: &mut impl Sink) {
        let f = &mut self.features.fade;
        if !self.running {
            f.armed = !f.armed;
            return;
        }
        if matches!(f.phase, Phase::Out { .. }) {
            return;
        }
        f.phase = Phase::Out { start_ns: now, from: f.pos };
        self.fade_wake(now, sink);
    }

    /// What the fade is doing.
    pub fn fade_state(&self) -> FadeState {
        let f = &self.features.fade;
        match f.phase {
            _ if f.armed && !self.running => FadeState::Armed,
            Phase::In { .. } => FadeState::FadingIn,
            Phase::Out { .. } => FadeState::FadingOut,
            Phase::Hold { .. } => FadeState::Holding,
            Phase::Idle => FadeState::Off,
        }
    }

    /// The band started: armed, it fades in from silence; a hold ends.
    pub(super) fn fade_on_start(&mut self, now: u64, sink: &mut impl Sink) {
        let f = &mut self.features.fade;
        if f.armed {
            f.armed = false;
            f.phase = Phase::In { start_ns: now, from: 0.0 };
            self.fade_wake(now, sink);
        } else if f.phase != Phase::Idle {
            f.phase = Phase::Idle;
            self.fade_set(1.0, sink);
        }
    }

    /// The band stopped: a fade in or out it was making ends at full volume (the fade out
    /// that stops it has already moved on to its hold).
    pub(super) fn fade_on_stop(&mut self, sink: &mut impl Sink) {
        if matches!(self.features.fade.phase, Phase::In { .. } | Phase::Out { .. }) {
            self.features.fade.phase = Phase::Idle;
            self.fade_set(1.0, sink);
        }
    }

    /// Move the level to where it is at `now`; the fade out's end stops the band.
    pub(super) fn fade_wake(&mut self, now: u64, sink: &mut impl Sink) {
        let s = self.features.settings;
        let f = self.features.fade;
        let progress = |start: u64, ms: u16| {
            let d = ms as u64 * 1_000_000;
            if d == 0 { 1.0 } else { (now.saturating_sub(start) as f64 / d as f64).min(1.0) as f32 }
        };
        match f.phase {
            Phase::Idle => {}
            Phase::In { start_ns, from } => {
                let p = progress(start_ns, s.fade_in_ms);
                self.fade_set(from + (1.0 - from) * p, sink);
                if p >= 1.0 {
                    self.features.fade.phase = Phase::Idle;
                }
            }
            Phase::Out { start_ns, from } => {
                let p = progress(start_ns, s.fade_out_ms);
                self.fade_set(from * (1.0 - p), sink);
                if p >= 1.0 {
                    let end = start_ns + s.fade_out_ms as u64 * 1_000_000;
                    self.features.fade.phase = Phase::Hold { until_ns: end + s.fade_hold_ms as u64 * 1_000_000 };
                    if self.running {
                        self.stop(sink);
                    }
                    self.fade_wake(now, sink);
                }
            }
            Phase::Hold { until_ns } => {
                if now >= until_ns {
                    self.features.fade.phase = Phase::Idle;
                    self.fade_set(1.0, sink);
                }
            }
        }
        if matches!(self.features.fade.phase, Phase::In { .. } | Phase::Out { .. }) {
            self.features.fade.next_ns = now + STEP_NS;
        }
    }

    /// Panic: any fade (or its hold) ends at once, at full volume, and the arming goes.
    pub fn fade_cancel(&mut self, sink: &mut impl Sink) {
        self.features.fade.armed = false;
        self.features.fade.phase = Phase::Idle;
        self.fade_set(1.0, sink);
    }

    /// A Style part's level as it goes out: fader value `v` scaled by the fade.
    #[inline]
    pub(super) fn faded(&self, v: u8) -> u8 {
        let pos = self.features.fade.pos;
        if pos >= 1.0 {
            v
        } else {
            (v as f32 * pos).round() as u8
        }
    }

    /// When the fade needs the engine next (ns), if it is doing anything.
    pub(super) fn fade_deadline(&self) -> Option<u64> {
        match self.features.fade.phase {
            Phase::Idle => None,
            Phase::In { .. } | Phase::Out { .. } => Some(self.features.fade.next_ns),
            Phase::Hold { until_ns } => Some(until_ns),
        }
    }

    /// Set the fade position and send each Style part's level (its fader value scaled)
    /// where it changed.
    fn fade_set(&mut self, pos: f32, sink: &mut impl Sink) {
        let pos = pos.clamp(0.0, 1.0);
        if pos == self.features.fade.pos {
            return;
        }
        self.features.fade.pos = pos;
        for p in 0..8u8 {
            let v = self.faded(self.mixer[p as usize]);
            let ch = 8 + p;
            if self.mirror.cc[ch as usize][7] != v {
                self.mirror.send(sink, &[0xB0 | ch, 7, v]);
            }
        }
    }
}

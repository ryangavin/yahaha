//! Fade In/Out (OM p.67, RM p.142): armed while stopped, START fades the style in; pressed
//! while it plays, the style fades out and stops, and the volume stays at 0 for the Fade
//! Out Hold Time before it comes back.
//!
//! The fade is the instrument's volume, not a part's: yahaha sends it as the MIDI
//! Universal Master Volume message (`F0 7F 7F 04 01 ll mm F7`), to the port and the
//! built-in synth alike (`live::Out` hands it to the synth, which scales its output by
//! it). The part faders (CC7) never move. Your playing fades with the band, as on the
//! Genos, where the hold silences the whole instrument.
//!
//! The level follows a fader-like curve: the gain is the square of the fade position,
//! which moves linearly with time. The engine sends a new level every `STEP_NS` while it
//! moves, on its own clock (`hook_wake_ns`), band running or not.

use super::*;

/// How often the level moves during a fade.
pub(super) const STEP_NS: u64 = 10_000_000;
/// Full volume, as Master Volume's 14-bit value.
pub const FULL: u16 = 0x3FFF;

/// The MIDI Universal Real Time SysEx Master Volume message for a 14-bit level.
pub fn master_volume_msg(level: u16) -> [u8; 8] {
    let v = level.min(FULL);
    [0xF0, 0x7F, 0x7F, 0x04, 0x01, (v & 0x7F) as u8, (v >> 7) as u8, 0xF7]
}

/// The level a Master Volume message sets, if `msg` is one.
pub fn master_volume_of(msg: &[u8]) -> Option<u16> {
    match *msg {
        [0xF0, 0x7F, _, 0x04, 0x01, lsb, msb, 0xF7] => Some((msb as u16 & 0x7F) << 7 | (lsb as u16 & 0x7F)),
        _ => None,
    }
}

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
    /// The fade position (the gain is its square), 1 = full.
    pos: f32,
    /// The level last sent.
    sent: u16,
    /// When the level moves next (during a fade).
    next_ns: u64,
}

impl Default for Fade {
    fn default() -> Fade {
        Fade { phase: Phase::Idle, armed: false, pos: 1.0, sent: FULL, next_ns: 0 }
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
        self.features.fade.next_ns = now + STEP_NS;
    }

    /// When the fade needs the engine next (ns), if it is doing anything.
    pub(super) fn fade_deadline(&self) -> Option<u64> {
        match self.features.fade.phase {
            Phase::Idle => None,
            Phase::In { .. } | Phase::Out { .. } => Some(self.features.fade.next_ns),
            Phase::Hold { until_ns } => Some(until_ns),
        }
    }

    /// Set the fade position and send its level if that changed.
    fn fade_set(&mut self, pos: f32, sink: &mut impl Sink) {
        let pos = pos.clamp(0.0, 1.0);
        let f = &mut self.features.fade;
        f.pos = pos;
        let level = (pos * pos * FULL as f32).round() as u16;
        if level != f.sent {
            f.sent = level;
            sink.send(&master_volume_msg(level));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn master_volume_round_trips() {
        for v in [0, 1, 127, 128, 8191, FULL] {
            assert_eq!(master_volume_of(&master_volume_msg(v)), Some(v));
        }
        assert_eq!(master_volume_msg(FULL), [0xF0, 0x7F, 0x7F, 0x04, 0x01, 0x7F, 0x7F, 0xF7]);
        assert_eq!(master_volume_of(&[0xF0, 0x43, 0x10, 0x4C, 0, 0, 0x7E, 0xF7]), None);
    }
}

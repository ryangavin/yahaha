//! The Style settings the engine plays by: Genos Section Change Timing (RM p.12), the
//! Synchro Stop Window (RM p.12), Fade In/Out times (RM p.142), Tap Tempo's Style Section
//! Reset (RM p.39) and the Style Retrigger length (RM p.147). The session keeps them and
//! sends them in whole (`live::Cmd::StyleSettings`); the engine reads them from
//! `Features::settings`. docs/section-timing.md has the behaviour and the guesses.

use super::*;
use serde::{Deserialize, Serialize};

/// Section Change Timing, To Main [A]-[D] (also a style change while playing).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MainTiming {
    /// At the next beat; the new section carries on from that beat of the bar. Auto Fill
    /// In on makes it Next Bar.
    Immediate,
    /// At once when pressed within the bar's first beat, else at the next bar line.
    #[default]
    NextBar,
}

/// Section Change Timing, Inside Intro/Ending: changing to another Intro or Ending while
/// one plays. Intro to Intro is always Next Bar; into Ending I, the usual next bar line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IntroEndingTiming {
    /// At once when pressed within the bar's first beat, else at the next bar line.
    #[default]
    NextBar,
    /// When the Intro or Ending playing has finished.
    EndOfSection,
}

/// The Style Retrigger lengths (RtgRate): a whole note, a half, ... a 32nd.
pub const RETRIGGER_RATES: [u8; 6] = [1, 2, 4, 8, 16, 32];
/// The longest Fade In / Fade Out time (20.0 s) and Fade Out Hold time (5.0 s), in ms.
pub const MAX_FADE_MS: u16 = 20_000;
pub const MAX_FADE_HOLD_MS: u16 = 5_000;
/// The longest Synchro Stop Window yahaha offers, in ms (the manuals list no values).
pub const MAX_SYNC_STOP_WINDOW_MS: u16 = 5_000;

/// Everything the engine needs to know about the player's Style settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StyleSettings {
    pub main_timing: MainTiming,
    pub intro_ending_timing: IntroEndingTiming,
    /// Synchro Stop Window in ms; 0 = Off.
    pub sync_stop_window_ms: u16,
    pub fade_in_ms: u16,
    pub fade_out_ms: u16,
    /// How long the volume stays at 0 after a fade out.
    pub fade_hold_ms: u16,
    /// TAP TEMPO while the style plays rewinds the section (on, the Genos default) or sets
    /// the tempo (off, yahaha's default: #128).
    pub section_reset: bool,
    /// Style Retrigger length: 1, 2, 4, 8, 16 or 32 (a whole note .. a 32nd).
    pub retrigger_rate: u8,
}

impl Default for StyleSettings {
    fn default() -> StyleSettings {
        StyleSettings {
            main_timing: MainTiming::NextBar,
            intro_ending_timing: IntroEndingTiming::NextBar,
            sync_stop_window_ms: 0,
            fade_in_ms: 5_000,
            fade_out_ms: 5_000,
            fade_hold_ms: 2_000,
            section_reset: false,
            retrigger_rate: 8,
        }
    }
}

impl StyleSettings {
    /// The same settings with every value in its range (a rate snaps to the nearest one
    /// there is, rounding down).
    pub fn clamped(self) -> StyleSettings {
        let rate = RETRIGGER_RATES.iter().rev().copied().find(|&r| r <= self.retrigger_rate.max(1)).unwrap_or(1);
        StyleSettings {
            sync_stop_window_ms: self.sync_stop_window_ms.min(MAX_SYNC_STOP_WINDOW_MS),
            fade_in_ms: self.fade_in_ms.min(MAX_FADE_MS),
            fade_out_ms: self.fade_out_ms.min(MAX_FADE_MS),
            fade_hold_ms: self.fade_hold_ms.min(MAX_FADE_HOLD_MS),
            retrigger_rate: rate,
            ..self
        }
    }

    /// The retrigger rate `d` steps along `RETRIGGER_RATES` (positive = shorter), stopping
    /// at the ends.
    pub fn step_rate(rate: u8, d: i8) -> u8 {
        let i = RETRIGGER_RATES.iter().position(|&r| r == rate).unwrap_or(3) as i32;
        RETRIGGER_RATES[(i + d as i32).clamp(0, RETRIGGER_RATES.len() as i32 - 1) as usize]
    }
}

impl Engine {
    /// New Style settings (`live::Cmd::StyleSettings`). They apply to what is asked for
    /// from now on; a change already queued keeps its time.
    pub fn set_style_settings(&mut self, s: StyleSettings) {
        self.features.settings = s.clamped();
    }

    /// The Style settings in use.
    pub fn style_settings(&self) -> StyleSettings {
        self.features.settings
    }

    /// Next Bar: at once (`now`, counted from its bar) when `now` is within the first beat
    /// of a bar, else at the next bar line.
    pub(super) fn bar_or_now(&self, now: u64) -> (f64, f64) {
        let t = self.tick_at(now);
        let (tpb, ppq) = (self.style.tpb.max(1) as f64, self.style.ppq.max(1) as f64);
        let pos = t - self.sec_start;
        let bar_start = self.sec_start + (pos / tpb + 1e-9).floor() * tpb;
        if t - bar_start < ppq - 1e-6 {
            (t, bar_start)
        } else {
            let at = self.next_bar(now);
            (at, at)
        }
    }

    /// The next beat line after `now`, and the start of its bar (a Fill, an Immediate
    /// change): the new section plays from the same beat of its bar.
    pub(super) fn next_beat(&self, now: u64) -> (f64, f64) {
        let t = self.tick_at(now);
        let ppq = self.style.ppq as f64;
        let tpb = self.style.tpb as f64;
        let pos = t - self.sec_start;
        let beat = (pos / ppq).ceil() * ppq;
        let at = self.sec_start + beat;
        let bar_start = self.sec_start + (beat / tpb).floor() * tpb;
        (at, bar_start)
    }
}

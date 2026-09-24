//! The Launchkey beyond the pads: Shift, the buttons, the faders, Track neighbours, and
//! the beat clocks.

use crate::launchkey::{Anim, Level};
use serde::{Deserialize, Serialize};

use super::AppCmd;

/// The Launchkey beyond the pads.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceState {
    /// The Launchkey's Shift button is held: show the controls' Shift layer.
    pub shift: bool,
    /// Pad Bank ▲/▼, Track ◀/▶, Play, Stop, the two buttons right of the pads (Scene,
    /// Function), the 8 fader buttons and the master fader button, in that order.
    pub controls: Vec<SurfaceControl>,
    /// Faders 1-8 and the master fader, for the active fader page.
    pub faders: Vec<SurfaceFader>,
    /// The styles Track ◀ / ▶ (and `StepStyle`) load: the previous and next in library
    /// order, skipping files known not to load. None: nowhere to go.
    pub track_prev: Option<Neighbour>,
    pub track_next: Option<Neighbour>,
    /// The beat clocks, to animate in step with the band and the pads.
    pub clock: ClockState,
}

/// A Launchkey button (not a pad): what it does, with and without Shift, and its light.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceControl {
    /// "padBankUp", "padBankDown", "trackPrev", "trackNext", "play", "stop", "scene",
    /// "function", "faderButton1".."faderButton8", "masterButton".
    pub id: String,
    /// Its CC on the DAW port (channel 1).
    pub cc: u8,
    /// What it does now, e.g. "PAGE ▼", "RIGHT 2", "PAD"; empty (and no action) when it
    /// does nothing.
    pub label: String,
    pub action: Option<AppCmd>,
    /// With Shift held. The same as `label`/`action` where Shift changes nothing.
    pub shift_label: String,
    pub shift_action: Option<AppCmd>,
    /// Its light, as a pad's: full colour 0-127 per channel, level, animation (always
    /// `solid`: buttons don't flash).
    pub rgb: [u8; 3],
    pub level: Level,
    pub anim: Anim,
    /// The palette index yahaha sends it (buttons have no RGB mode; `rgb` is its look).
    /// None: yahaha doesn't drive this LED (Play, Stop, Scene, Function): it shows the
    /// Launchkey's own default, reported as `off`.
    pub colour: Option<u8>,
}

/// A Launchkey fader on the active fader page (and the master fader).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceFader {
    /// What it controls, e.g. "RIGHT 1", "BASS", "MASTER"; empty when unused.
    pub label: String,
    /// The level it controls (0-127); None when unused.
    pub value: Option<u8>,
    /// The level is waiting for the hardware fader (soft takeover).
    pub waiting: bool,
    /// Where the hardware fader physically is (0-127), as last reported; None until it
    /// moves.
    pub position: Option<u8>,
    /// What moving it sends: this command with `volume` filled in (`setPartVolume`,
    /// `setStylePartVolume`, `setMasterVolume`; `volume` is 0 here). None: unused.
    pub set: Option<AppCmd>,
}

/// A library entry next to the loaded style.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Neighbour {
    pub id: usize,
    pub name: String,
    pub path: String,
}

/// The beat clocks. Times are the session's monotonic clock, in ms (`atMs` is when this
/// state was read: when it last changed for `Session::state`, now for
/// `Session::state_now`). Beats are quarter notes. Both clocks are anchors: a value at a
/// time, moving on at `tempo` until the next state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClockState {
    /// The session clock (ms) this state was read at.
    pub at_ms: f64,
    pub running: bool,
    /// BPM (quarter notes per minute).
    pub tempo: f64,
    /// Quarter notes per bar (4 in 4/4, 3 in 6/8).
    pub beats_per_bar: f64,
    /// The playing position at `atMs`: bar and beat 1-based in the section, and how far
    /// into the beat (0..1). 1, 1, 0 when stopped.
    pub bar: u32,
    pub beat: u32,
    pub phase: f64,
    /// Position anchor: at `sectionAnchorMs` the section had played `sectionAnchorBeats`.
    pub section_anchor_ms: f64,
    pub section_anchor_beats: f64,
    /// The free-running clock the Launchkey pads flash and pulse on (`Pad::anim`): at
    /// `ledAnchorMs` it read `ledAnchorBeats`.
    pub led_anchor_ms: f64,
    pub led_anchor_beats: f64,
}

impl ClockState {
    /// Quarter notes into the section at `t` (ms on the session clock). Never below 0: a
    /// time before the anchor (a client clock a hair behind the session's) reads as the
    /// section's start.
    pub fn position(&self, t: f64) -> f64 {
        if !self.running {
            return 0.0;
        }
        (self.section_anchor_beats + (t - self.section_anchor_ms) * self.tempo / 60e3).max(0.0)
    }

    /// The pad flash/pulse clock at `t` (ms).
    pub fn led_beats(&self, t: f64) -> f64 {
        self.led_anchor_beats + (t - self.led_anchor_ms) * self.tempo / 60e3
    }

    /// The same clock, read at `t` (ms): `atMs`, `bar`, `beat` and `phase` move on.
    pub fn at(&self, t: f64) -> ClockState {
        let b = self.position(t).max(0.0);
        let bpb = if self.beats_per_bar > 0.0 { self.beats_per_bar } else { 4.0 };
        ClockState {
            at_ms: t,
            bar: (b / bpb).floor() as u32 + 1,
            beat: (b % bpb).floor() as u32 + 1,
            phase: b.fract(),
            ..self.clone()
        }
    }
}

/// Session clock ns as the clock's ms.
pub fn ns_to_ms(ns: u64) -> f64 {
    ns as f64 / 1e6
}

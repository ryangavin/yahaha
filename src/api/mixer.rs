//! The mixer: the Style parts' mute and levels, the fader page, the synth's master level
//! and mute, and the output meters.

use crate::engine::Button;
use crate::parts::FaderPage;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum MixerCmd {
    /// Mute/unmute a Style part (`part` 0-7).
    ToggleStylePart { part: u8 },
    /// Set a Style part's volume (its CC7, 0-127). The Launchkey fader picks it up.
    SetStylePartVolume { part: u8, volume: u8 },
    /// What the Launchkey faders control: the keyboard parts (Panel) or the Style parts.
    SetFaderPage { page: FaderPage },
    ToggleFaderPage,
    /// The built-in synth's master volume (0-127; 100 = unity). The master fader picks it up.
    SetMasterVolume { volume: u8 },
    /// Mute/unmute the built-in synth's audio.
    SetSynthMuted { on: bool },
    ToggleSynthMute,
}

impl MixerCmd {
    /// The engine button this command is, if it is one.
    pub fn button(&self) -> Option<Button> {
        match *self {
            MixerCmd::ToggleStylePart { part } => Some(Button::TogglePart(part & 7)),
            _ => None,
        }
    }
}

/// The mixer: the Style parts, the fader page, the master volume.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MixerState {
    /// What the Launchkey faders 1-8 control.
    pub fader_page: FaderPage,
    /// The 8 Style parts.
    pub style_parts: Vec<StylePart>,
    /// The built-in synth's master volume (0-127, 100 = unity). None without the synth.
    pub master: Option<u8>,
    /// The Launchkey master fader has moved but not yet reached `master`.
    pub master_waiting: bool,
}

/// One of the 8 accompaniment parts.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StylePart {
    /// "Rhythm 1", "Rhythm 2", "Bass", "Chord 1", "Chord 2", "Pad", "Phrase 1", "Phrase 2".
    pub name: String,
    /// MIDI channel, 1-based (9-16).
    pub channel: u8,
    /// Not muted (and not muted by Manual Bass).
    pub on: bool,
    /// The Bass part, muted because Manual Bass is in effect.
    pub muted_by_manual_bass: bool,
    /// Volume (its CC7), 0-127.
    pub volume: u8,
    /// The Launchkey fader has moved but not yet reached `volume`.
    pub waiting: bool,
    /// Where its Launchkey fader (Style page) physically is; None if it hasn't moved.
    pub fader: Option<u8>,
    /// The voice the style was written for.
    pub voice: Option<Voice>,
}

/// A Yamaha voice as the style names it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Voice {
    pub bank_msb: u8,
    pub bank_lsb: u8,
    /// Program, 0-based.
    pub program: u8,
    /// A drum or SFX kit.
    pub kit: bool,
    /// What the built-in synth plays for it, e.g. "Finger Bass (GM 34)",
    /// "≈ Strings  [Yamaha 104/0/49]", "drum kit 127/0/1".
    pub label: String,
}

/// Output levels (`Session::meters`), read on the audio thread. Peaks are linear
/// amplitude (1.0 = full scale), the highest since the previous read: the client applies
/// its own decay and peak hold. Meant for one reader (the app's meter bridge).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meters {
    /// The session clock (ms) at the read.
    pub at_ms: f64,
    /// Keyboard parts (ch 1-4) and Style parts (ch 9-16), after the master level, with
    /// their reverb and chorus, before the soft clipper.
    pub channels: Vec<ChannelMeter>,
    /// Left and right after the soft clipper.
    pub master: [f32; 2],
    /// Audio buffers in which the soft clipper was working (above -1 dBFS), since start.
    pub clips: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelMeter {
    /// MIDI channel, 1-based.
    pub channel: u8,
    pub peak: f32,
}

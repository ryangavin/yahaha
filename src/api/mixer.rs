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
    /// The Style volume (Genos Balance: Style), 0-127, 100 = the parts' CC7 as written: a
    /// scale on every Style part's CC7 as it goes out, like Fade In/Out; the part faders do
    /// not move. Panel fader 5 picks it up.
    SetStyleVolume { volume: u8 },
    /// What the Launchkey faders control: the keyboard parts (Panel) or the Style parts.
    SetFaderPage { page: FaderPage },
    ToggleFaderPage,
    /// The built-in synth's master volume (0-127; 100 = unity). The master fader picks it up.
    SetMasterVolume { volume: u8 },
    /// Mute/unmute the built-in synth's audio.
    SetSynthMuted { on: bool },
    ToggleSynthMute,
    /// Solo a Style part (`part` 0-7): only it plays, even if switched off. `null` ends the
    /// solo. The parts' on/off switches are unchanged.
    SetStyleSolo { part: Option<u8> },
    /// Solo a keyboard part (`part` 0-3: Right 1, Right 2, Right 3, Left): only it sounds
    /// from the keys, even if switched off. `null` ends the solo.
    SetPartSolo { part: Option<u8> },
    /// Style Track Mute (a Genos Live Control knob, RM p.148): `value` 0-127 is the knob.
    /// Fully left leaves one Style part on; turning up adds parts in the `order`'s sequence
    /// until, fully right, all eight are on. Sets the parts' on/off switches.
    StyleTrackMute { order: TrackMuteOrder, value: u8 },
}

/// The order Style Track Mute brings the Style parts in (RM p.148).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TrackMuteOrder {
    /// Rhythm 2, then Rhythm 1, Bass, Chord 1, Chord 2, Pad, Phrase 1, Phrase 2.
    A,
    /// Chord 1, then Chord 2, Pad, Bass, Phrase 1, Phrase 2, Rhythm 1, Rhythm 2.
    B,
}

impl TrackMuteOrder {
    /// Style parts (0-7) in the order the knob turns them on.
    pub fn order(self) -> [u8; 8] {
        match self {
            TrackMuteOrder::A => [1, 0, 2, 3, 4, 5, 6, 7],
            TrackMuteOrder::B => [3, 4, 5, 2, 6, 7, 0, 1],
        }
    }

    /// The Style parts on (bit = part) with the knob at `value` (0-127): 1 part at 0, all
    /// 8 at 127, in even steps between.
    pub fn mask(self, value: u8) -> u8 {
        let n = 1 + (value.min(127) as usize * 7 + 63) / 127;
        self.order()[..n].iter().fold(0, |m, &p| m | 1 << p)
    }
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
    /// The Style volume (0-127, 100 = the Style parts' CC7 as written; `setStyleVolume`).
    pub style_volume: u8,
    /// Panel fader 5 has moved but not yet reached `style_volume`.
    pub style_volume_waiting: bool,
    /// The Style part soloed (0-7), if any: only it plays.
    pub style_solo: Option<u8>,
    /// The keyboard part soloed (0-3), if any: only it sounds from the keys.
    pub part_solo: Option<u8>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_mute_steps() {
        assert_eq!(TrackMuteOrder::A.mask(0), 0b10);
        assert_eq!(TrackMuteOrder::A.mask(127), 0xFF);
        assert_eq!(TrackMuteOrder::B.mask(0), 0b1000);
        // Each step adds the next part in the order.
        let masks: Vec<_> = (0..=127).map(|v| TrackMuteOrder::B.mask(v)).collect();
        let counts: Vec<_> = masks.iter().map(|m| m.count_ones()).collect();
        assert!(counts.windows(2).all(|w| w[1] == w[0] || w[1] == w[0] + 1));
        assert_eq!(TrackMuteOrder::B.mask(64) & 0b11, 0, "the rhythm parts come last in B");
        assert!((1..=8).all(|n| counts.iter().filter(|&&c| c == n).count() >= 9));
    }
}

//! The keyboard parts' plain data (#337): part ids and indices, their channels and default
//! voices, the Panel fader levels, the fader page and layer, the pan and send controllers,
//! and the XG multi part defaults.
//!
//! No dependencies inside the crate, so modules below the engine can use them; they go to
//! the core crate in the workspace split. The `Parts` state (atomics, soft takeover, OTS
//! recall) needs the engine and sff and lives in `engine`. `parts` re-exports both, so the
//! old `crate::parts::...` paths keep working.

pub const RIGHT1: usize = 0;
pub const RIGHT2: usize = 1;
pub const RIGHT3: usize = 2;
pub const LEFT: usize = 3;
pub const COUNT: usize = 4;
pub const NAMES: [&str; COUNT] = ["Right 1", "Right 2", "Right 3", "Left"];
/// Each part's MIDI channel (0-based) on the port and in the synth.
pub const CHANNEL: [u8; COUNT] = [0, 2, 3, 1];
/// Default voices (GM programs): Grand Piano, Strings, Brass Section; Left Strings.
pub const DEFAULT_PROGRAMS: [u8; COUNT] = [0, 48, 61, 48];

/// Panel fader 5: the Style volume (#199), a scale on the Style parts' CC7 (100 = as
/// written), kept with the keyboard parts' levels (`Parts::volume(STYLE_LEVEL)`), with the
/// same soft takeover.
pub const STYLE_LEVEL: usize = 4;
/// Panel fader 6: the Multi Pad volume (#196), the same kind of scale on the pads' CC7
/// (channels 5-8).
pub const PAD_LEVEL: usize = 5;
/// The levels the Panel page's faders control: the four keyboard parts, then the Style
/// and the Multi Pads.
pub const PANEL_FADERS: usize = 6;

/// The part on MIDI channel `ch`, if it is a keyboard part's.
pub fn part_of_channel(ch: u8) -> Option<usize> {
    CHANNEL.iter().position(|&c| c == ch)
}

/// What the Launchkey faders 1-8 control (the button under the master fader toggles it).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FaderPage {
    /// Faders 1-4: Right 1, Right 2, Right 3, Left; 5: the Style volume; 6: the Multi Pad
    /// volume. 7-8 unused.
    #[default]
    Panel,
    /// Faders 1-8: the Style parts.
    Style,
}

/// What the Launchkey faders control across the parts (the mixer's VOL / PAN / REV / CHO /
/// DLY buttons, as Ableton's sends view): each part's CC7, or its pan or effect send (CC10,
/// CC91, CC93, CC94), the same controls `setPartPan` / `setPartSend` set. Shift + the
/// master fader's button steps through them. The master fader is always the master.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FaderLayer {
    #[default]
    Volume,
    Pan,
    Reverb,
    Chorus,
    /// The Variation block, the tempo delay (CC94).
    Delay,
}

impl FaderLayer {
    pub const ALL: [FaderLayer; 5] = [FaderLayer::Volume, FaderLayer::Pan, FaderLayer::Reverb, FaderLayer::Chorus, FaderLayer::Delay];

    /// Its index in `Parts::fx` (`PAN` .. `VARIATION`); None for Volume (the CC7).
    pub fn fx_index(self) -> Option<usize> {
        match self {
            FaderLayer::Volume => None,
            FaderLayer::Pan => Some(PAN),
            FaderLayer::Reverb => Some(REVERB),
            FaderLayer::Chorus => Some(CHORUS),
            FaderLayer::Delay => Some(VARIATION),
        }
    }

    /// The layer `d` steps on, wrapping (VOL -> PAN -> REV -> CHO -> DLY -> VOL).
    pub fn step(self, d: i8) -> FaderLayer {
        let n = Self::ALL.len() as i16;
        Self::ALL[((self as i16 + d as i16).rem_euclid(n)) as usize]
    }

    pub(crate) fn from_u8(v: u8) -> FaderLayer {
        Self::ALL.get(v as usize).copied().unwrap_or_default()
    }

    /// The short name the mixer's buttons show.
    pub fn short(self) -> &'static str {
        ["VOL", "PAN", "REV", "CHO", "DLY"][self as usize]
    }
}

/// What a Panel fader 1-4 does in the Volume layer, from the live rack's controller map
/// (docs/racks.md): the input thread reads it from `Parts::rack_fader`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[repr(u8)]
pub enum FaderRoute {
    /// Its own part's level, with soft takeover on the input thread (the default map).
    #[default]
    Own,
    /// Nothing.
    Off,
    /// Something else: the control side runs it (`moveRackFader`).
    Control,
}

impl FaderRoute {
    pub(crate) fn from_u8(v: u8) -> FaderRoute {
        match v {
            1 => FaderRoute::Off,
            2 => FaderRoute::Control,
            _ => FaderRoute::Own,
        }
    }
}

/// The XG default of multi part parameter (hh, nn) (Data List, MIDI Parameter Change table
/// (MULTI PART)), for the parameters a voice change puts back; None for the others (bank,
/// program, channel, part mode, levels and sends: not voice settings).
pub fn xg_default(hh: u8, nn: u8) -> Option<u8> {
    let v = match (hh, nn) {
        (0x08, 0x05) => 0x01,
        (0x08, 0x0C | 0x0D) => 0x40,
        (0x08, 0x15..=0x1F) => 0x40,
        (0x08, 0x20) => 0x0A,
        (0x08, 0x21 | 0x22) => 0x00,
        (0x08, 0x23) => 0x42,
        (0x08, 0x24 | 0x25) => 0x40,
        (0x08, 0x26..=0x28) => 0x00,
        (0x08, 0x4D..=0x4F) => 0x40,
        (0x08, 0x50..=0x52) => 0x00,
        (0x08, 0x53..=0x55) => 0x40,
        (0x08, 0x56..=0x58) => 0x00,
        (0x08, 0x5A..=0x5C) => 0x40,
        (0x08, 0x5D..=0x5F) => 0x00,
        (0x08, 0x61..=0x63) => 0x40,
        (0x08, 0x64..=0x68) => 0x00,
        (0x08, 0x69..=0x6C) => 0x40,
        (0x08, 0x72 | 0x73) => 0x40,
        (0x08, 0x76) => 0x0C,
        (0x08, 0x77) => 0x36,
        (0x0A, 0x00..=0x03) => 0x00,
        (0x0A, 0x40..=0x45) => 0x40,
        _ => return None,
    };
    Some(v)
}

/// The controllers `Parts::fx` holds: pan, reverb send, chorus send, variation send
/// (the effect bus's tempo delay, #204).
pub const FX_CC: [u8; FX] = [10, 91, 93, 94];
/// How many controllers `Parts::fx` holds.
pub const FX: usize = 4;
/// `Parts::fx` indices.
pub const PAN: usize = 0;
pub const REVERB: usize = 1;
pub const CHORUS: usize = 2;
pub const VARIATION: usize = 3;
/// What each part's pan and sends are before anything sets them: pan centre and every
/// send dry (reverb, chorus and delay 0). The keyboard parts only get a send from the
/// player, or from data that stores one (an OTS, a Registration, a library patch's own
/// defaults); GM's power-on reverb of 40 is not data, so it never applies. The engine
/// thread sends them at start and again after anything that may have reset them
/// (`resend_fx`), so a receiver's own power-on sends never linger.
pub const FX_DEFAULT: [[u8; FX]; COUNT] = [[64, 0, 0, 0]; COUNT];

/// `Parts::solo`: no part soloed.
pub const NO_SOLO: u8 = 255;

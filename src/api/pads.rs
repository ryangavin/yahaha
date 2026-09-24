//! The Launchkey pads.

use crate::launchkey::{Anim, Level, Page};
use serde::{Deserialize, Serialize};

use super::AppCmd;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PadsCmd {
    /// The Launchkey pad page.
    SetPadPage { page: Page },
    /// Step the pad page by `delta`, wrapping (the terminal's Tab / Shift+Tab).
    CyclePadPage { delta: i8 },
}

/// The Launchkey pads.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PadsState {
    pub page: Page,
    /// "Sections", "Chord/Setup", "OTS/Parts".
    pub page_name: String,
    /// 1-based page number, and how many pages there are.
    pub page_number: u8,
    pub page_count: u8,
    /// The 16 pads on this page: the top row (notes 96-103) then the bottom row (112-119).
    pub pads: Vec<Pad>,
    /// A Launchkey is connected (DAW port).
    pub connected: bool,
    /// The LEDs use the Novation palette (`--palette-leds`): the pads show `Pad::palette`,
    /// not `rgb`/`level`/`anim`.
    pub palette_leds: bool,
}

/// A pad in palette-LED mode: what was sent to it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaletteLed {
    /// `solid`, `flash` (between `colour` and `flashColour`) or `pulse` (`colour`). The
    /// Launchkey times flash and pulse itself, not on the beat clock.
    pub mode: Anim,
    pub colour: u8,
    pub rgb: [u8; 3],
    pub level: Level,
    pub flash_colour: Option<u8>,
    pub flash_rgb: Option<[u8; 3]>,
    pub flash_level: Option<Level>,
}

/// One pad: what it does and how it is lit.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pad {
    /// The pad's MIDI note on the Launchkey DAW port.
    pub note: u8,
    /// e.g. "MAIN A", "FINGERED", "OTS 1"; empty for an unused pad.
    pub label: String,
    /// The terminal UI's keyboard shortcut, e.g. "1", "spc", "F10".
    pub key: String,
    /// Full-brightness colour, 0-127 per channel.
    pub rgb: [u8; 3],
    pub level: Level,
    pub anim: Anim,
    /// What pressing it sends (None: an unused pad).
    pub action: Option<AppCmd>,
    /// Palette-LED mode only (`PadsState::paletteLeds`): what the hardware pad shows.
    pub palette: Option<PaletteLed>,
}

//! The Sound hold (docs/eyes-free.md, "Decisions"): while Panel fader button 6
//! (`launchkey::SOUND_FADER_BTN`) is held, on either fader page, the pads act and light as
//! the Racks page from any page (`Layer::Sound`). Capture (hold Sound + tap a Quick Rack
//! pad stores or overwrites that rack with the live rack) comes here too. Lane C (Racks
//! page, Sound hold, capture) fills it.
//!
//! Runs on the MIDI input thread: no allocation, locks or panics.

use crate::launchkey::{self, Action, Layer, Page};

/// The Sound button went down: the layer while it is held.
pub fn press(now: Layer) -> Layer {
    let _ = now;
    Layer::Sound
}

/// The Sound button went up: the Sound layer ends; any other layer (a swap started during
/// the hold) stays.
pub fn release(now: Layer) -> Layer {
    if now == Layer::Sound { Layer::None } else { now }
}

/// Pad `note` pressed while Sound is held, with pad page `page` on view.
pub fn pad(page: Page, note: u8) -> Option<Action> {
    // TODO(lane C): capture on tap (a lit Quick Rack pad overwrites it with the live rack,
    // an empty one stores a new rack), confirmed on the display.
    launchkey::pad_action(page, Layer::Sound, note)
}

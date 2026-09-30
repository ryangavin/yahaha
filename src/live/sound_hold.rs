//! The Sound hold (docs/eyes-free.md, "Decisions"): while Panel fader button 6
//! (`launchkey::SOUND_FADER_BTN`) is held, on either fader page, the pads act and light as
//! the Racks page from any page (`Layer::Sound`); release brings back the page on view.
//!
//! Capture (hold Sound + tap the lit Quick Rack pad overwrites that rack with the live rack;
//! + tap an empty pad stores it as a new rack) is decided on the control side: only it knows
//! which button is lit or empty. The input thread sends the Racks page's press
//! (`Action::QuickRack`) and keeps `Shared::layer` at Sound until release; the control side
//! reads the hold when it runs the press (session/quick_racks.rs, `sound_tap_captures`)
//! and turns it into `storeRack`, which the app's pads send too.
//!
//! Runs on the MIDI input thread: no allocation, locks or panics.

use crate::launchkey::{self, Action, Layer, Page};

/// The Sound button went down: the layer while it is held. A swap in progress (a part's
/// fader button held with a knob turned) gives way: the pads are the Racks page.
pub fn press(now: Layer) -> Layer {
    let _ = now;
    Layer::Sound
}

/// The Sound button went up: the Sound layer ends; any other layer (a swap started during
/// the hold) stays.
pub fn release(now: Layer) -> Layer {
    if now == Layer::Sound { Layer::None } else { now }
}

/// Pad `note` pressed while Sound is held, with pad page `page` on view: the Racks page's
/// pad, whatever the page.
pub fn pad(page: Page, note: u8) -> Option<Action> {
    launchkey::pad_action(page, Layer::Sound, note)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hold_sets_and_clears_the_sound_layer() {
        for now in [Layer::None, Layer::Sound, Layer::Swap { part: 2 }] {
            assert_eq!(press(now), Layer::Sound);
        }
        assert_eq!(release(Layer::Sound), Layer::None);
        assert_eq!(release(Layer::None), Layer::None);
        assert_eq!(release(Layer::Swap { part: 1 }), Layer::Swap { part: 1 }, "a swap started during the hold stays");
    }

    /// From every page, the pads are the Racks page's: Quick Racks 1-8, OTS 1-4, bank -/+,
    /// Store, and the spare pad does nothing.
    #[test]
    fn the_pads_are_the_racks_page_from_every_page() {
        for page in [Page::Sections, Page::Racks, Page::Chord, Page::MultiPads, Page::Setup] {
            for i in 0..8 {
                assert_eq!(pad(page, 96 + i), Some(Action::QuickRack(i)), "{page:?}");
            }
            for i in 0..4 {
                assert_eq!(pad(page, 112 + i), Some(Action::Ots(i)), "{page:?}");
            }
            assert_eq!(pad(page, 116), Some(Action::QuickRackBank(-1)));
            assert_eq!(pad(page, 117), Some(Action::QuickRackBank(1)));
            assert_eq!(pad(page, 118), Some(Action::QuickRackStore));
            assert_eq!(pad(page, 119), None);
            assert_eq!(pad(page, 60), None, "not a pad");
        }
    }
}

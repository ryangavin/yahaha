//! Swap mode (docs/eyes-free.md, "Decisions"): hold a Panel fader button 1-4 (a keyboard
//! part) and turn a knob. Knob 1 steps the part's sound by number, live; knobs 2-8 are its
//! mix; releasing the button commits. The input thread (`Input::fader_button`) keeps the
//! hold and sets `Layer::Swap { part }` on the first knob turn; this module says what the
//! knobs and the release do. Lane B (swap mode) fills it.
//!
//! Runs on the MIDI input thread: no allocation, locks or panics.

use crate::launchkey::Action;

/// Knob `knob` (0-7) turned `delta` steps while keyboard part `part`'s (0-3) fader button
/// is held: what it does instead of its Knob Assign function.
pub fn knob(part: u8, knob: u8, delta: i8) -> Option<Action> {
    match knob {
        0 => Some(Action::SwapSound { part, step: delta }),
        // TODO(lane B): knobs 2-8 become the part's mix (volume, pan, sends, ...).
        _ => None,
    }
}

/// Keyboard part `part`'s fader button went up after a knob turned during the hold: the
/// swap ends and commits.
pub fn commit(part: u8) -> Option<Action> {
    // TODO(lane B): commit the swap (the sound stepped to stays; the display confirms).
    let _ = part;
    None
}

//! Swap mode (docs/eyes-free.md, "Decisions"): hold a Panel fader button 1-4 (a keyboard
//! part) and turn a knob. Knob 1 steps the part's sound by number, live; knobs 2-8 are its
//! mix; releasing the button commits. The input thread (`Input::fader_button`) keeps the
//! hold and sets `Layer::Swap { part }` on the first knob turn; this module says what the
//! knobs and the release do: knob 1 sends `Action::SwapSound`, knobs 2-8 send
//! `Action::SwapKnob`. The sound numbers, the sound change and what each mix knob sets are
//! the control side's (`Control::swap_sound`, `Control::swap_knob`), as a knob's function is.
//!
//! Runs on the MIDI input thread: no allocation, locks or panics.

use crate::launchkey::Action;

/// Knob `knob` (0-7) turned `delta` steps while keyboard part `part`'s (0-3) fader button
/// is held: what it does instead of its Knob Assign function. Knob 1 steps the sound one
/// number per encoder step. Knobs 2-8 are the part's mix: the turn goes to the control side
/// as it came (`Control::swap_knob`). No turn (`delta` 0) or no such knob does nothing.
pub fn knob(part: u8, knob: u8, delta: i8) -> Option<Action> {
    match knob {
        _ if delta == 0 => None,
        0 => Some(Action::SwapSound { part, step: delta }),
        1..=7 => Some(Action::SwapKnob { part, knob, delta }),
        _ => None,
    }
}

/// Keyboard part `part`'s fader button went up after a knob turned during the hold: the
/// swap ends. Nothing more happens: every step already sounded, there is no revert
/// (dialling back is the cancel), and the input thread has cleared the layer.
pub fn commit(part: u8) -> Option<Action> {
    let _ = part;
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knob_1_steps_the_sound_knobs_2_8_are_the_mix_and_the_release_adds_nothing() {
        assert_eq!(knob(2, 0, 1), Some(Action::SwapSound { part: 2, step: 1 }));
        assert_eq!(knob(0, 0, -3), Some(Action::SwapSound { part: 0, step: -3 }));
        assert_eq!(knob(0, 0, 0), None);
        for k in 1..8 {
            assert_eq!(knob(1, k, 1), Some(Action::SwapKnob { part: 1, knob: k, delta: 1 }));
            assert_eq!(knob(3, k, -2), Some(Action::SwapKnob { part: 3, knob: k, delta: -2 }));
            assert_eq!(knob(1, k, 0), None, "no turn");
        }
        assert_eq!(knob(1, 8, 1), None, "no such knob");
        assert_eq!(commit(3), None);
    }
}

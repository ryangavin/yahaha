//! The keyboard strip: the keys held (the input thread's `Shared::keys`) and the chord.

use super::{Control, View};
use crate::api::{HeldNote, KeyboardState, Zone};
use crate::live;
use crate::parts;
use crate::theory;
use std::sync::atomic::Ordering::Relaxed;

impl Control {
    pub(super) fn keyboard_state(&self, view: &View) -> KeyboardState {
        let s = &self.snap;
        let shared = &self.shared;
        let (upper, split, fingering) = (view.upper, view.split, view.fingering);
        KeyboardState {
            held: (0..128u8)
                .filter_map(|k| {
                    let v = shared.keys[k as usize].load(Relaxed);
                    (v & live::KEY_HELD != 0).then(|| HeldNote {
                        note: k,
                        zone: if v & live::KEY_RIGHT != 0 { Zone::Right } else { Zone::Left },
                        parts: (0..parts::COUNT as u8).filter(|p| v & (1 << p) != 0).collect(),
                    })
                })
                .collect(),
            left_split: split,
            chord_tones: s
                .played
                .filter(|c| c.ty != theory::CANCEL)
                .map(|c| theory::chord_tones(c.ty).iter().map(|t| (c.root + t) % 12).collect())
                .unwrap_or_default(),
            chord_bass: s.played.filter(|c| c.ty != theory::CANCEL).map(|c| c.bass.unwrap_or(c.root)),
            detection: if !upper && fingering.full_keyboard() {
                [0, 127]
            } else if upper {
                [split.saturating_add(1).min(127), 127]
            } else {
                [0, split]
            },
        }
    }
}

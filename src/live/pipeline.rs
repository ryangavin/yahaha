//! The keyboard-part note path on the MIDI input thread, as an explicit sequence of stages.
//!
//! A note from a keyboard source (`Input::key_msg_from`) goes through, in this order:
//!
//! 1. **note in** ([`Input::key_down`] / [`Input::key_up`]): the key, its velocity, and the
//!    side of the split it went to (remembered in `Input::route` until it is let go).
//! 2. **transpose** ([`Input::transpose`]): the Keyboard transpose in semitones. It is
//!    carried in [`Note::shift`] and applied at routing together with each part's octave,
//!    in one fold (`engine::shift_key`), so a note pushed past the MIDI range folds exactly
//!    as it always has. Chords are recognized from the keys as fingered (the engine
//!    transposes the chord), so recognition never sees this stage.
//! 3. **processor** ([`Processor`]): one slot. Nothing today ([`Processor::Off`] passes the
//!    note on unchanged); later Keyboard Harmony (`src/harmony.rs`) or the Arpeggiator
//!    (`src/arp/`). The two are mutually exclusive, as on the Genos (one Harmony/Arpeggio
//!    button, one type), so they are variants of one enum rather than two stages.
//! 4. **part routing, held-note bookkeeping, output** ([`Input::sound`]): the Right 1-3 /
//!    Left parts that sound the note (`sounds`), `Keys` (where each held key sounded, so
//!    its note-off, poly aftertouch and retrigger go to the same place), the key strip and
//!    per-source bookkeeping (`track_key`), and the messages to the port and the synth
//!    (`Out`). Then the chord section is re-read if the key belongs to it.
//!
//! Note-offs take the same path without the transpose: they stop what the note-on sounded
//! (`Keys`), whatever the transpose, split or parts are now.
//!
//! # Rules for this thread
//!
//! This is the CoreMIDI receive thread: nothing here allocates, frees, locks or blocks.
//! Every stage works on `Copy` values and fixed arrays; settings come from `Shared`'s
//! atomics; the engine hears about things through its rings (`Cmd`) and `Wakeup`.
//! `tests/input_no_alloc.rs` checks the whole key path with a counting allocator.
//!
//! # Plugging in a processor
//!
//! Add a variant to [`Processor`] holding the feature's state (fixed-size; box it if it is
//! big, but build and drop the box on the control side), and a match arm in
//! [`Processor::note`]. The arm gets every note-on (after transpose) and note-off, and
//! returns what continues to part routing:
//!
//! - `Some(note)`: the note sounds on the keyboard parts as usual (Harmony keeps the
//!   melody note).
//! - `None`: it doesn't (the Arpeggiator swallows the keys it holds; its pattern plays
//!   them). The key still counts as held for the chord section, the key strip and Sync
//!   Stop, and its note-off still reaches `Keys`.
//!
//! Anything extra the processor sounds it sends itself through the `Out` it is given (the
//! same port and synth, flushed at the end of the packet list), with its own bookkeeping
//! of what to stop on the key's note-off (`harmony::HarmonyTracker` for Harmony). Messages
//! it sends go out before the note's own, as they are sent first.
//!
//! Swapping the processor (the Harmony/Arpeggio type changing) is a message to this thread
//! like the others (a ring from the control side, as `Input::set_release`): the new value
//! comes in built, the old one is sent back to be dropped there, and whatever the old one
//! was sounding is stopped first (`HarmonyTracker::release_all`, `Arp::all_off`).
//!
//! ## Time domains
//!
//! This thread only has "now" (`rt::now_ns`): it sees a note when it arrives. What a
//! processor plays later is timed elsewhere:
//!
//! - **Harmony** chords (Duet, Trio, Block, ...) sound with the melody note, here.
//!   The Echo category (Echo, Tremolo, Trill) repeats notes in time: `harmony::EchoGen`
//!   runs on engine nanoseconds (`now` on the engine's clock, `next_due`, `next_events`).
//!   Its note-on/note-off calls come from this stage; the repeats are pulled by the
//!   engine thread (an engine hook plus `hook_deadline`-style wake), so the processor
//!   hands the EchoGen its key events through a ring, and never plays repeats itself.
//! - **Arpeggio** runs in style ticks (`arp::Arp`, `ppq` ticks per quarter, tempo-synced
//!   and quantized to the style's grid), which only the engine thread has. The processor
//!   swallows the keys and forwards key-down/key-up to the engine (a `Cmd`, or a held-key
//!   mask in `Shared`); the arp itself is an engine feature (`engine::hooks::Features`,
//!   driven from `process` with its events on the style clock), and it sends on the
//!   Right parts' channels through the engine's `Sink`.

use super::*;

/// A keyboard note as it moves down the pipeline. `Copy`, a few bytes: stages pass it by
/// value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Note {
    /// The note to sound, before transpose and octaves: the key pressed, unless a processor
    /// moved it.
    pub key: u8,
    /// Note-on velocity (1..=127); 0 for a note-off.
    pub vel: u8,
    /// Left of the split when the key went down.
    pub left: bool,
    /// Keyboard transpose in semitones (the transpose stage). Applied at routing with each
    /// part's octave in one fold.
    pub shift: i8,
}

/// The processor slot: at most one of Harmony and Arpeggio, as on the Genos. See the module
/// docs for how a processor plugs in.
#[derive(Debug, Default)]
pub enum Processor {
    /// No processor: every note goes on to the keyboard parts unchanged.
    #[default]
    Off,
}

impl Processor {
    /// Stage 3: `note` (a note-on after transpose, or a note-off) in, what continues to
    /// part routing out. Extra notes go straight to `_out`.
    #[inline(always)]
    pub fn note(&mut self, note: Note, _out: &mut Out) -> Option<Note> {
        match self {
            Processor::Off => Some(note),
        }
    }
}

impl Input {
    /// Stage 1, note in: key `k` went down with velocity `vel` (> 0) on keyboard source
    /// `slot`, with the split at `split`.
    #[inline]
    pub(super) fn key_down(&mut self, slot: usize, k: u8, vel: u8, split: u8) {
        // Lower: the chord section is the left hand. Upper: it is the right hand (above
        // Split Point (Left)), and the left hand plays the Left part (or the Right parts,
        // with Left off).
        let left = k <= split;
        let r = if left { R_LH } else { R_RH };
        let chord = r == self.chord_side();
        self.route[k as usize] = r;
        let full = !self.shared.upper.load(Relaxed) && Fingering::from_u8(self.shared.fingering.load(Relaxed)).full_keyboard();
        let note = self.transpose(Note { key: k, vel, left, shift: 0 });
        let note = self.processor.note(note, &mut self.out);
        let now = self.sound(k, note, full);
        self.track_key(slot, k, r, now);
        // The Full Keyboard types (Lower only) read both hands.
        if chord || full {
            self.recompute();
        }
    }

    /// Stage 1, note in: key `k` let go (a note-off, or a note-on with velocity 0) on
    /// keyboard source `slot`.
    #[inline]
    pub(super) fn key_up(&mut self, slot: usize, k: u8) {
        let note = Note { key: k, vel: 0, left: self.route[k as usize] == R_LH, shift: 0 };
        let _ = self.processor.note(note, &mut self.out);
        // Stage 4: stop what the key sounded, where it sounded.
        for (ch, note) in self.keys.release(k).iter() {
            self.out.push(&[0x80 | ch, note, 0]);
        }
        let r = std::mem::take(&mut self.route[k as usize]);
        self.track_key(slot, k, 0, Sounded::default());
        if r != 0 {
            // Sync Stop: the last key of the current chord section went up.
            let side = self.chord_side();
            if r & side != 0 && !self.route.iter().any(|&x| x & side != 0) && self.cmd.push(Cmd::ChordReleased).is_ok() {
                self.signal = true;
            }
        }
    }

    /// Stage 2, transpose: the Keyboard transpose, for routing to apply.
    #[inline(always)]
    fn transpose(&self, note: Note) -> Note {
        Note { shift: self.shared.key_shift.load(Relaxed), ..note }
    }

    /// Stage 4, part routing, held-note bookkeeping and output, for key `k` going down as
    /// `note` (None: the processor took it, so nothing sounds). `full`: a Full Keyboard
    /// fingering type is reading both hands. Returns where the key sounds.
    #[inline]
    fn sound(&mut self, k: u8, note: Option<Note>, full: bool) -> Sounded {
        let (now, vel) = match note {
            Some(n) => {
                // With Left off, the left hand plays the Right parts unless the left
                // section is the chord section alone (Lower, not Full Keyboard).
                let chord_only = !self.shared.upper.load(Relaxed) && !full;
                (sounds(&self.shared.parts, n.left, chord_only, n.key, n.shift), n.vel)
            }
            None => (Sounded::default(), 0),
        };
        // A retrigger (possibly after the split, transpose or parts changed): release
        // where it sounded.
        for (pch, pnote) in self.keys.press(k, now).iter() {
            self.out.push(&[0x80 | pch, pnote, 0]);
        }
        for (ch, note) in now.iter() {
            self.out.push(&[0x90 | ch, note, vel]);
        }
        now
    }
}

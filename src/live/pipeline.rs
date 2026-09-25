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
//! 3. **processor** ([`Input::process`], mode [`Processor`]): one slot, Keyboard Harmony
//!    (`src/harmony.rs`) or the Arpeggiator (`src/arp/`). The two are mutually exclusive,
//!    as on the Genos (one HARMONY/ARPEGGIO button, one type), so they are variants of one
//!    enum rather than two stages. Only right-hand keys are processed.
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
//! # The processor
//!
//! The mode ([`Processor`]) comes from the Harmony/Arpeggio settings word
//! (`Shared::kbd_fx`, read on each key; `kbdfx::FxConfig`). What each mode does with a
//! right-hand key going down:
//!
//! - **Off**: nothing; the note goes on.
//! - **Harmony** (Duet .. Strum): the melody key, if it is the highest right-hand key
//!   held, gets its harmony (`harmony::harmonize`, the chord from `harmony::harmony_chord`
//!   and the chord section's current chord). The harmony notes go out here, before the
//!   melody's own, on the Right parts Assign picks, and are counted in `Keys` together
//!   with the keys' own notes (per channel and pitch), so a harmony note and a key on the
//!   same pitch never cut each other short. The melody continues, narrowed to its parts
//!   ([`Note::parts`]). Strum's later notes go to the engine thread (`FxKey::Strum`).
//!
//!   The counts are per thread: this thread's `Keys` and the engine thread's `KbdFx`
//!   voices do not see each other. So a note one thread sounds can be cut short by a
//!   note-off from the other on the same channel and pitch: Strum's later notes against a
//!   plain key (hold C5 with Strum over a C chord, then play and release G4: G4's off ends
//!   the strummed G4), and Echo/Arpeggio notes against left-hand keys sounding on the Right
//!   parts (Upper, or Full Keyboard with Left off). Nothing sticks: each thread still
//!   sends one off for each pitch it counted. Known limit (#100 review N1).
//! - **Multi Assign**: the key continues on one Right part (`harmony::MultiAssign`).
//! - **Echo** (Echo, Tremolo, Trill) and **Arpeggio**: the key is swallowed (`None`) and
//!   sent to the engine thread (`FxKey::On`), which plays it (`kbdfx::KbdFx`). The key still
//!   counts as held for the chord section, the key strip and Sync Stop, and it is marked
//!   in `Shared::fx_held` until it goes up.
//!
//! How each key went through is remembered (`Input::fx_path`), so its note-off takes the
//! same way whatever the settings are by then ([`Input::process_off`]): its harmony notes
//! stop, its Multi Assign part frees, or its key-up goes to the engine thread. Switching
//! the type or the switch never cuts a key that is down on this thread; the engine side
//! stops its own generator (`Arp::all_off`, `EchoGen::all_off`).
//!
//! ## Time domains
//!
//! This thread only has "now": it sees a note when it arrives. What a processor plays
//! later is timed on the engine thread (`kbdfx.rs`): Echo/Tremolo/Trill on engine
//! nanoseconds (`harmony::EchoGen`), the arpeggio on style ticks (`arp::Arp`; its own clock
//! while the band is stopped), and Strum's later notes at their delay. They reach it
//! through an SPSC ring (`FxKey`); the engine wakes for them through
//! `EngineLoop::next_deadline`.

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
    /// The Right parts it may sound on (bit 0-2 = Right 1-3; `harmony::PartMask`): all of
    /// them unless a processor narrows it (Multi Assign, Harmony Assign = Multi).
    pub parts: u8,
}

/// Every Right part (`Note::parts`).
pub const ALL_RIGHT: u8 = 0b111;

/// The processor slot: at most one of Harmony and Arpeggio, as on the Genos. The mode comes
/// from the Harmony/Arpeggio settings word (`kbdfx::FxConfig::processor`); the state it
/// needs lives in [`Input`] (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Processor {
    /// No processor: every note goes on to the keyboard parts unchanged.
    #[default]
    Off,
    /// A Harmony-category type (Duet, Trio, Block, 4-Way, 1+5, Octave, Strum): the melody
    /// key's harmony notes sound here, with it.
    Harmony(HarmonySettings),
    /// Multi Assign: each right-hand key sounds on one Right part, in the order pressed.
    MultiAssign,
    /// Echo, Tremolo, Trill: the right-hand keys go to the engine thread's `EchoGen`.
    Echo,
    /// Arpeggio: the right-hand keys go to the engine thread's `Arp`.
    Arp,
}

/// How a held key went through the processor, so its note-off takes the same way whatever
/// the settings are by then (`Input::fx_path`).
pub(super) const PATH_PLAIN: u8 = 0;
pub(super) const PATH_HARMONY: u8 = 1;
pub(super) const PATH_MULTI: u8 = 2;
pub(super) const PATH_FORWARD: u8 = 3;

/// The harmony notes one melody key sounds (on this thread): (channel, pitch), up to
/// `harmony::MAX_NOTES` notes on each of the three Right parts. `strum`: it also has
/// notes on the engine thread (Strum's later notes).
#[derive(Clone, Copy, Default)]
pub(super) struct Harmonized {
    n: u8,
    notes: [(u8, u8); 3 * harmony::MAX_NOTES],
    strum: bool,
}

impl Harmonized {
    fn push(&mut self, ch: u8, note: u8) {
        if (self.n as usize) < self.notes.len() {
            self.notes[self.n as usize] = (ch, note);
            self.n += 1;
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
        let note = self.transpose(Note { key: k, vel, left, shift: 0, parts: ALL_RIGHT });
        let note = self.process(k, note);
        let now = self.sound(k, note, full);
        self.track_key(slot, k, r, now);
        // Dynamics Touch / Accent: the engine hears each strike in the chord section (not
        // while the Chord Looper loops: then there is no chord section).
        if chord
            && self.shared.strikes.load(Relaxed)
            && !self.shared.looping.load(Relaxed)
            && self.cmd.push(Cmd::Strike(vel)).is_ok()
        {
            self.signal = true;
        }
        // The Full Keyboard types (Lower only) read both hands.
        if chord || full {
            self.recompute();
        }
    }

    /// Stage 1, note in: key `k` let go (a note-off, or a note-on with velocity 0) on
    /// keyboard source `slot`.
    #[inline]
    pub(super) fn key_up(&mut self, slot: usize, k: u8) {
        self.process_off(k);
        // Stage 4: stop what the key sounded, where it sounded.
        for (ch, note) in self.keys.release(k).iter() {
            self.out.push(&[0x80 | ch, note, 0]);
        }
        let r = std::mem::take(&mut self.route[k as usize]);
        self.track_key(slot, k, 0, Sounded::default());
        if r != 0 {
            // Every key the chord is read from is up: the next chord is struck anew.
            let read = self.chord_read_side();
            if r & read != 0 && !self.route.iter().any(|&x| x & read != 0) {
                self.let_go = true;
            }
            // Sync Stop: the last key of the current chord section went up.
            let side = self.chord_side();
            if r & side != 0 && !self.route.iter().any(|&x| x & side != 0) && self.cmd.push(Cmd::ChordReleased).is_ok() {
                self.signal = true;
            }
        }
    }

    /// Stage 3, processor: `note` (a key going down, after transpose) in, what continues
    /// to part routing out (None: the processor took it). Only right-hand keys are
    /// processed (spec §6: the Right parts' section; the chord section and the Left part
    /// never are).
    #[inline]
    fn process(&mut self, k: u8, note: Note) -> Option<Note> {
        let w = self.shared.kbd_fx.load(Relaxed);
        if w != self.fx_word {
            self.fx_word = w;
            self.processor = FxConfig::unpack(w).processor();
        }
        // A retrigger: the key's last way through ends first.
        if self.fx_path[k as usize] != PATH_PLAIN {
            self.process_off(k);
        }
        if note.left {
            return Some(note);
        }
        match self.processor {
            Processor::Off => Some(note),
            Processor::Harmony(s) => {
                self.fx_path[k as usize] = PATH_HARMONY;
                Some(self.harmonize(k, note, s))
            }
            Processor::MultiAssign => {
                self.fx_path[k as usize] = PATH_MULTI;
                let mask = self.multi.press(k, kbdfx::right_parts(&self.shared.parts));
                Some(Note { parts: mask, ..note })
            }
            Processor::Echo | Processor::Arp => {
                if self.fx_tx.is_none() {
                    return Some(note);
                }
                self.fx_path[k as usize] = PATH_FORWARD;
                // Held first, then the message: the engine never sees the message for a
                // key its mask says is up.
                self.set_fx_held(k, true);
                self.fx_send(FxKey::On { key: k, vel: note.vel });
                None
            }
        }
    }

    /// Stage 3 for a key going up: whatever the processor did with it at note-on ends.
    #[inline]
    fn process_off(&mut self, k: u8) {
        match std::mem::replace(&mut self.fx_path[k as usize], PATH_PLAIN) {
            PATH_HARMONY => {
                let h = std::mem::take(&mut self.harmonized[k as usize]);
                for &(ch, n) in &h.notes[..h.n as usize] {
                    if self.keys.unhold(ch, n) {
                        self.out.push(&[0x80 | ch, n, 0]);
                    }
                }
                if h.strum {
                    self.set_fx_held(k, false);
                    self.fx_send(FxKey::StrumOff { melody: k });
                }
            }
            PATH_MULTI => {
                self.multi.release(k);
            }
            PATH_FORWARD => {
                self.set_fx_held(k, false);
                self.fx_send(FxKey::Off { key: k });
            }
            _ => {}
        }
    }

    /// Keyboard Harmony, Harmony category: melody key `k` (the highest right-hand key
    /// held) gets its harmony notes, which sound now on the Right parts Assign picks,
    /// counted with the keys' own notes in `Keys` so neither cuts the other short. Strum's
    /// later notes go to the engine thread. Returns the melody note, narrowed to its parts.
    fn harmonize(&mut self, k: u8, note: Note, s: HarmonySettings) -> Note {
        // Only the top note of the right hand is harmonised (docs/harmony.md).
        if self.route[k as usize + 1..].contains(&R_RH) {
            return note;
        }
        let parts = self.shared.parts.clone();
        // While the Chord Looper plays, the keyboard gives no chord and the style follows
        // the loop's: the harmony follows that (`Shared::loop_chord`).
        let current = if self.shared.looping.load(Relaxed) {
            Chord::unpack(self.shared.loop_chord.load(Relaxed)).map(|(c, _)| c)
        } else {
            self.current
        };
        let chord = harmony::harmony_chord(s.ty, kbdfx::ACMP, parts.is_on(parts::LEFT), current, current);
        let h = harmony::harmonize(k, note.vel, chord, &s, kbdfx::right_parts(&parts));
        let mut done = Harmonized::default();
        for hn in h.notes() {
            for p in [parts::RIGHT1, parts::RIGHT2, parts::RIGHT3] {
                if hn.parts & (1 << p) == 0 || !parts.audible(p) {
                    continue;
                }
                let ch = parts::CHANNEL[p];
                let pitch = shift_key(hn.key, note.shift + 12 * parts.octave_of(p));
                if hn.delay_ms == 0 {
                    self.keys.hold(ch, pitch);
                    self.out.push(&[0x90 | ch, pitch, hn.vel]);
                    done.push(ch, pitch);
                } else if self.fx_tx.is_some() {
                    if !done.strum {
                        done.strum = true;
                        self.set_fx_held(k, true);
                    }
                    self.fx_send(FxKey::Strum { melody: k, ch, note: pitch, vel: hn.vel, delay_ms: hn.delay_ms });
                }
            }
        }
        self.harmonized[k as usize] = done;
        Note { parts: h.melody_parts, ..note }
    }

    fn set_fx_held(&self, k: u8, on: bool) {
        let (w, b) = ((k >> 6) as usize, 1u64 << (k & 63));
        if on {
            self.shared.fx_held[w].fetch_or(b, Release);
        } else {
            self.shared.fx_held[w].fetch_and(!b, Release);
        }
    }

    fn fx_send(&mut self, m: FxKey) {
        if let Some(tx) = self.fx_tx.as_mut()
            && tx.push(m).is_ok()
        {
            self.signal = true;
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
                // section is the chord section alone (Lower, not Full Keyboard). While
                // the Chord Looper loops there is no chord section: the whole keyboard
                // is for performance (RM p.15, p.19).
                let chord_only = !self.shared.upper.load(Relaxed) && !full && !self.shared.looping.load(Relaxed);
                (sounds_on(&self.shared.parts, n.left, chord_only, n.key, n.shift, n.parts), n.vel)
            }
            None => (Sounded::default(), 0),
        };
        // Left Hold: a key on the Left part is the next chord, so what was held lets go
        // first (a re-pedal before this key's note-on, OM p.49).
        if self.shared.controllers.left_hold() && now.iter().any(|(ch, _)| ch == parts::CHANNEL[parts::LEFT]) {
            self.shared.controllers.release_left_hold();
            self.sync_controllers();
        }
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

//! Live runtime: the MIDI input handler (runs on CoreMIDI's thread), the engine thread,
//! and the lock-free plumbing between them and the session's control side
//! (`session.rs`, which every client goes through).
//!
//!   CoreMIDI thread ──chord (AtomicU32) + commands (SPSC) + semaphore──▶ engine thread
//!   CoreMIDI thread ──Launchkey actions (SPSC) + semaphore─────────────▶ control
//!   control ──────────styles/commands (SPSC) + semaphore───────────────▶ engine thread
//!   engine thread ────snapshots (SPSC), old styles (SPSC) + semaphore──▶ control
//!
//! Neither real-time side ever waits on the other: producers use try-push and a
//! non-blocking semaphore signal; the engine only sleeps on the semaphore with a timeout
//! equal to its next deadline.

use crate::engine::{shift_key, AuditionPos, Button, ChartPlan, ChartSettings, Engine, Prepared, Snapshot, Transpose};
use crate::fingering::{self, Fingering};
use crate::launchkey::{self, Action, Control, Page};
use crate::midi::{for_each_message, InputHandler};
use crate::parts::{self, FaderPage, Parts};
use crate::rt::{self, Histogram, PacketSink, Wakeup};
use crate::theory::{Chord, Recognizer, CANCEL, ONE_PLUS_EIGHT, ONE_PLUS_FIVE};
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::atomic::{AtomicBool, AtomicI8, AtomicU32, AtomicU64, AtomicU8, Ordering::*};
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub enum Cmd {
    Button(Button),
    ChordReleased,
    /// A Launchkey part fader (0..8) moved to a value (soft takeover in the engine).
    PartVolume(u8, u8),
    /// A Style part's volume set from software (0..8, 0-127): the fader picks it up.
    StyleVolume(u8, u8),
    /// Manual Bass in effect: mute the Style's Bass part.
    ManualBass(bool),
    Transpose(Transpose),
    Arm,
    Panic,
    /// End the style preview now (`AppCmd::StopAudition`).
    StopAudition,
    /// All Notes Off on the keyboard parts' channels: a MIDI source with keys held was
    /// disconnected (its note-offs will never come).
    KeysOff,
    /// Chart player settings (chart mode, Intro, Ending, loop; engine/chart.rs). The
    /// chart itself comes in on its own ring (`EngineIo::charts`).
    Chart(ChartSettings),
}

/// How many bars a style preview plays.
pub const AUDITION_BARS: u8 = 4;

/// A style preview for the engine thread (`AppCmd::AuditionStyle`): its own engine, built
/// on the control side, playing the style's Main A over `chords`, one a bar, for
/// `AUDITION_BARS` bars, on the band's channels while the band is stopped.
pub struct Audition {
    pub engine: Engine,
    /// The library id it previews (for the state).
    pub id: u32,
    pub chords: [Chord; 4],
}

/// State the real-time threads read without waiting. Owned by the session: clients never
/// touch it (they send `AppCmd`s), only the session's control side and the RT threads.
pub struct Shared {
    pub chord: AtomicU32,
    pub chord_ns: AtomicU64,
    /// Wakes the engine thread.
    pub wake: Wakeup,
    /// Wakes the session's control thread (Launchkey actions, new snapshots).
    pub ctl_wake: Wakeup,
    /// Software moved the synth master level: the master fader has to pick it up again.
    pub master_moved: AtomicBool,
    /// The Launchkey's Shift button is held (for the screen: the Shift layer).
    pub shift: AtomicBool,
    /// Where the Launchkey master fader physically is (`HW_UNKNOWN` until it moves).
    pub master_hw: AtomicU8,
    pub quit: AtomicBool,
    pub split: AtomicU8,
    /// Chord fingering type (`Fingering::to_u8`), read by the input thread on each chord
    /// and by the engine thread on each wake (Sync Stop is refused in the Full types).
    pub fingering: AtomicU8,
    /// Chord Detection Area = Upper: the chord comes from the keys above the split
    /// (Fingered*), and the left hand plays the Left part.
    pub upper: AtomicBool,
    /// The Manual Bass setting. It only takes effect in Upper mode; see `manual_bass()`.
    pub manual_bass: AtomicBool,
    /// Keyboard + Master transpose: the shift applied to played notes. The engine gets
    /// the individual values through `Cmd::Transpose`.
    pub key_shift: AtomicI8,
    /// Engine wake lateness vs. its deadline.
    pub lateness: Histogram,
    /// CoreMIDI packet timestamp -> our callback.
    pub input_lat: Histogram,
    /// Chord published by the input thread -> applied by the engine.
    pub chord_lat: Histogram,
    pub engine_rt: AtomicBool,
    /// Last message from the Launchkey DAW port, packed 0x00SSDDVV (for the on-screen readout).
    pub last_daw: AtomicU32,
    /// Last Launchkey DAW-port note or CC that nothing is mapped to, packed 0x01SSDDVV
    /// (0 = none yet), so a wrong CC number shows on screen.
    pub last_unmapped: AtomicU32,
    /// Launchkey pad page (`launchkey::Page::to_u8`): set by the Pad Bank buttons on the
    /// input thread and Tab on the UI thread, read by both.
    pub page: AtomicU8,
    /// Time spent in engine.process / in the CoreMIDI send, per wake.
    pub flush_lat: Histogram,
    pub work_lat: Histogram,
    pub spin_ns: AtomicU64,
    /// The keyboard parts (Right 1-3, Left) and the fader page.
    pub parts: Arc<Parts>,
    /// Each key as the input thread last saw it, for the app's key strip: `KEY_HELD`, the
    /// side of the split it went to (`KEY_RIGHT`), and the keyboard parts sounding it
    /// (bits 0-3 = Right 1, Right 2, Right 3, Left).
    pub keys: [AtomicU8; 128],
    /// How many keys each keyboard source slot holds (`key_tag`).
    pub src_held: [AtomicU8; MAX_KEY_SOURCES],
}

impl Shared {
    pub fn new(split: u8) -> Shared {
        Shared {
            chord: AtomicU32::new(0),
            chord_ns: AtomicU64::new(0),
            wake: Wakeup::new(),
            ctl_wake: Wakeup::new(),
            master_moved: AtomicBool::new(false),
            shift: AtomicBool::new(false),
            master_hw: AtomicU8::new(crate::engine::HW_UNKNOWN),
            quit: AtomicBool::new(false),
            split: AtomicU8::new(split),
            fingering: AtomicU8::new(Fingering::FingeredOnBass.to_u8()),
            upper: AtomicBool::new(false),
            manual_bass: AtomicBool::new(true),
            key_shift: AtomicI8::new(0),
            lateness: Histogram::new(),
            input_lat: Histogram::new(),
            chord_lat: Histogram::new(),
            engine_rt: AtomicBool::new(false),
            last_daw: AtomicU32::new(0),
            last_unmapped: AtomicU32::new(0),
            page: AtomicU8::new(0),
            flush_lat: Histogram::new(),
            work_lat: Histogram::new(),
            spin_ns: AtomicU64::new(150_000),
            parts: Arc::new(Parts::new()),
            keys: std::array::from_fn(|_| AtomicU8::new(0)),
            src_held: std::array::from_fn(|_| AtomicU8::new(0)),
        }
    }

    /// The keys held: (all of them, the ones on the right of the split), as bit masks
    /// (bit k of word k / 64).
    pub fn held_keys(&self) -> ([u64; 2], [u64; 2]) {
        let (mut held, mut right) = ([0u64; 2], [0u64; 2]);
        for (k, a) in self.keys.iter().enumerate() {
            let v = a.load(Relaxed);
            if v & KEY_HELD != 0 {
                held[k / 64] |= 1 << (k % 64);
                if v & KEY_RIGHT != 0 {
                    right[k / 64] |= 1 << (k % 64);
                }
            }
        }
        (held, right)
    }

    /// Sync Stop is available: always in Upper (Fingered*), else unless the fingering type
    /// is a Full Keyboard one.
    pub fn sync_stop_allowed(&self) -> bool {
        self.upper.load(Relaxed) || Fingering::from_u8(self.fingering.load(Relaxed)).allows_sync_stop()
    }

    /// Move the Launchkey pad page. Pad Bank ▲/▼ (input thread) and Tab (UI thread) both
    /// do this; a compare-and-swap keeps either from losing the other's change.
    pub fn step_page(&self, f: impl Fn(Page) -> Page) {
        let _ = self.page.fetch_update(Relaxed, Relaxed, |p| Some(f(Page::from_u8(p)).to_u8()));
    }

    /// Manual Bass in effect: Upper detection mode with the Manual Bass setting on.
    pub fn manual_bass(&self) -> bool {
        self.upper.load(Relaxed) && self.manual_bass.load(Relaxed)
    }
}

/// Output fan-out: the virtual MIDI port, plus the built-in synth when it's running.
pub struct Out {
    pub midi: PacketSink,
    pub synth: Option<Producer<[u8; 3]>>,
}

impl Out {
    pub fn new(midi: PacketSink, synth: Option<Producer<[u8; 3]>>) -> Out {
        Out { midi, synth }
    }

    #[inline]
    pub fn push(&mut self, msg: &[u8]) {
        self.midi.push(msg);
        // The built-in synth takes channel messages only; SysEx (the style's XG effect
        // setup) is for the port.
        if msg.first() == Some(&0xF0) {
            return;
        }
        if let Some(s) = self.synth.as_mut() {
            let mut m = [0u8; 3];
            let n = msg.len().min(3);
            m[..n].copy_from_slice(&msg[..n]);
            let _ = s.push(m);
        }
    }

    #[inline]
    pub fn flush(&mut self) {
        self.midi.flush();
    }
}

impl crate::engine::Sink for Out {
    #[inline]
    fn send(&mut self, msg: &[u8]) {
        self.push(msg);
    }
}

/// Input port tags: a keyboard (slot 0; the offline session's keys), the Launchkey DAW
/// port, and keyboard sources by slot from `KEY_TAG_BASE` on (`key_tag`), so the keys a
/// source holds are known when it is disconnected.
pub const TAG_KEYS: usize = 1;
pub const TAG_PADS: usize = 2;

/// `Shared::keys`: the key is held, and on the right of the split; the low 4 bits are the
/// keyboard parts sounding it.
pub const KEY_HELD: u8 = 0x80;
pub const KEY_RIGHT: u8 = 0x40;
pub const KEY_PARTS: u8 = 0x0F;

pub const KEY_TAG_BASE: usize = 16;
/// Keyboard source slots (slot 0 is `TAG_KEYS`).
pub const MAX_KEY_SOURCES: usize = 16;

/// The input port tag for keyboard source slot `slot` (1..MAX_KEY_SOURCES).
pub fn key_tag(slot: usize) -> usize {
    KEY_TAG_BASE + slot.min(MAX_KEY_SOURCES - 1)
}

/// The keyboard source slot a tag stands for (None: the pads).
#[inline]
fn key_slot(tag: usize) -> Option<usize> {
    match tag {
        TAG_PADS => None,
        t if t >= KEY_TAG_BASE => Some((t - KEY_TAG_BASE).min(MAX_KEY_SOURCES - 1)),
        _ => Some(0),
    }
}

#[cfg(test)]
pub const RH_CH: u8 = parts::CHANNEL[parts::RIGHT1];
#[cfg(test)]
pub const LH_CH: u8 = parts::CHANNEL[parts::LEFT];

/// Which side of the split a held key went to at note-on, so it keeps counting for that
/// side even if the split or the detection area changed while it was held (where its
/// note-off goes is tracked by `Keys`). Whether a key is a chord key is not stored: it is
/// decided by its side and the area at the time.
const R_LH: u8 = 1;
const R_RH: u8 = 2;

/// Fingered*, the only fingering in Upper detection mode: Fingered without 1+5, 1+8 or
/// Chord Cancel, so melody fragments in the right hand (single notes, octaves, fifths,
/// chromatic runs) don't change the chord. 1+8 and 1+5 are the only Fingered shapes
/// with fewer than three pitch classes (Data List p.44), so Fingered* needs three.
/// As in Fingered, the bass is the root.
pub fn fingered_star(mask: u16, c: Chord) -> Option<Chord> {
    if mask.count_ones() < 3 || matches!(c.ty, CANCEL | ONE_PLUS_EIGHT | ONE_PLUS_FIVE) {
        return None;
    }
    Some(Chord { bass: None, ..c })
}

// ---------------------------------------------------------------------------
// Input (CoreMIDI receive thread)
// ---------------------------------------------------------------------------

mod pipeline;
pub use pipeline::{Note, Processor};

/// The notes one key sounds: a (channel, note) for each keyboard part that played it (up
/// to the three Right parts layered).
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Sounded {
    n: u8,
    notes: [(u8, u8); 3],
}

impl Sounded {
    pub fn push(&mut self, ch: u8, note: u8) {
        if (self.n as usize) < self.notes.len() {
            self.notes[self.n as usize] = (ch, note);
            self.n += 1;
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (u8, u8)> + '_ {
        self.notes[..self.n as usize].iter().copied()
    }
}

/// What a key sounds as, on each part's channel, moved by the transpose `shift` and the
/// part's octave. Left of the split (`left`): the Left part when it sounds. With Left off,
/// the Right parts play over the entire keyboard (OM p.48), except where the left section
/// is the chord section alone (`chord_only`: Lower detection outside the Full Keyboard
/// types, OM p.56). Right of the split: the Right parts that are on, layered.
pub fn sounds(parts: &Parts, left: bool, chord_only: bool, key: u8, shift: i8) -> Sounded {
    let mut s = Sounded::default();
    let side: &[usize] = match (left, parts.left_sounds()) {
        (true, true) => &[parts::LEFT],
        (true, false) if chord_only => &[],
        _ => &[parts::RIGHT1, parts::RIGHT2, parts::RIGHT3],
    };
    for &p in side {
        if p == parts::LEFT || parts.is_on(p) {
            let oct = parts.octave_of(p);
            s.push(parts::CHANNEL[p], shift_key(key, shift + 12 * oct));
        }
    }
    s
}

/// Where each held key sounded (channels and notes), so its note-offs go to the same place
/// even if the split, transpose, part on/off or octave changed while it was down.
///
/// Two held keys can sound the same note on the same channel: an octave shift folds notes
/// beyond the MIDI range back by octaves, and a transpose or octave change between two
/// presses moves one onto the other. Each (channel, note) counts the keys holding it, and
/// its note-off goes out only when the last one lets go, so no held key is cut short.
pub struct Keys {
    sounding: [Sounded; 128],
    held: [[u8; 128]; 16],
}

impl Default for Keys {
    fn default() -> Keys {
        Keys::new()
    }
}

impl Keys {
    pub fn new() -> Keys {
        Keys { sounding: [Sounded::default(); 128], held: [[0; 128]; 16] }
    }

    /// Key down, sounding as `now`: remembered for the release. Returns the notes to stop
    /// first: those of a retrigger of a key still sounding, that no other key holds.
    pub fn press(&mut self, key: u8, now: Sounded) -> Sounded {
        let old = std::mem::replace(&mut self.sounding[key as usize & 127], now);
        let off = self.let_go(old);
        for (ch, note) in now.iter() {
            let n = &mut self.held[ch as usize & 15][note as usize & 127];
            *n = n.saturating_add(1);
        }
        off
    }

    /// Key up: the notes it sounded that no other held key still sounds, to stop.
    pub fn release(&mut self, key: u8) -> Sounded {
        let old = std::mem::take(&mut self.sounding[key as usize & 127]);
        self.let_go(old)
    }

    /// What a held key sounds as (channels and notes).
    pub fn sounding(&self, key: u8) -> Sounded {
        self.sounding[key as usize & 127]
    }

    fn let_go(&mut self, notes: Sounded) -> Sounded {
        let mut off = Sounded::default();
        for (ch, note) in notes.iter() {
            let n = &mut self.held[ch as usize & 15][note as usize & 127];
            *n = n.saturating_sub(1);
            if *n == 0 {
                off.push(ch, note);
            }
        }
        off
    }
}

pub struct Input {
    shared: Arc<Shared>,
    rec: Recognizer,
    /// Which side of the split each held key went to (`R_LH` / `R_RH`, 0 = not held).
    route: [u8; 128],
    keys: Keys,
    current: Option<Chord>,
    generation: u16,
    cmd: Producer<Cmd>,
    out: Out,
    /// Running status per port: [unused, unused, pads, key slot 0, key slot 1, ...].
    running_status: [u8; 3 + MAX_KEY_SOURCES],
    /// The keys each keyboard source slot holds (bit k of word k / 64).
    src_keys: [[u64; 2]; MAX_KEY_SOURCES],
    /// Source slots the control side disconnected: release what they hold.
    release: Option<Consumer<u8>>,
    signal: bool,
    /// An action went to the control side: wake it at the end of the packet list.
    ctl_signal: bool,
    synth: Option<Arc<crate::synth::SynthControl>>,
    /// Launchkey pad and button actions for the session's control side (anything that
    /// isn't an engine button: settings, OTS, style change), run as `AppCmd`s.
    actions: Option<Producer<Action>>,
    /// The Launchkey's Shift button is held.
    shift: bool,
    /// Soft takeover of the Launchkey master fader (the synth master level).
    master_takeover: crate::engine::Takeover,
    /// The note pipeline's processor slot (Harmony or Arpeggio; none yet).
    processor: Processor,
}

impl Input {
    pub fn new(shared: Arc<Shared>, rec: Recognizer, cmd: Producer<Cmd>, out: Out) -> Input {
        Input {
            shared,
            rec,
            route: [0; 128],
            keys: Keys::new(),
            current: None,
            generation: 0,
            cmd,
            out,
            running_status: [0; 3 + MAX_KEY_SOURCES],
            src_keys: [[0; 2]; MAX_KEY_SOURCES],
            release: None,
            signal: false,
            ctl_signal: false,
            synth: None,
            actions: None,
            shift: false,
            master_takeover: crate::engine::Takeover::NEW,
            processor: Processor::Off,
        }
    }

    /// Where the keyboard parts' notes go besides the port: the built-in synth's ring.
    pub fn set_out_synth(&mut self, feed: Option<Producer<[u8; 3]>>) {
        self.out.synth = feed;
    }

    pub fn set_synth(&mut self, ctl: Option<Arc<crate::synth::SynthControl>>) {
        self.synth = ctl;
    }

    pub fn set_actions(&mut self, tx: Producer<Action>) {
        self.actions = Some(tx);
    }

    /// Where the control side says which source slots it disconnected.
    pub fn set_release(&mut self, rx: Consumer<u8>) {
        self.release = Some(rx);
    }

    /// Release every key source slot `slot` holds that no other source holds too, as if
    /// its note-offs had come: the notes stop and the chord section lets go.
    pub fn release_source(&mut self, slot: usize) {
        let slot = slot.min(MAX_KEY_SOURCES - 1);
        for k in 0..128u8 {
            let (w, b) = ((k >> 6) as usize, 1u64 << (k & 63));
            if self.src_keys[slot][w] & b == 0 {
                continue;
            }
            let elsewhere = (0..MAX_KEY_SOURCES).any(|o| o != slot && self.src_keys[o][w] & b != 0);
            if elsewhere {
                self.src_keys[slot][w] &= !b;
                self.shared.src_held[slot].fetch_sub(1, Relaxed);
            } else {
                self.key_msg_from(slot, &[0x80, k, 0]);
            }
        }
    }

    /// Apply the releases the control side queued.
    fn drain_releases(&mut self) {
        let Some(mut rx) = self.release.take() else { return };
        while let Ok(slot) = rx.pop() {
            self.release_source(slot as usize);
        }
        self.release = Some(rx);
    }

    /// Note key `k` held (`r` = its side, `sounded` where it sounds) or let go (`r` = 0)
    /// for the key strip, and by source slot.
    fn track_key(&mut self, slot: usize, k: u8, r: u8, sounded: Sounded) {
        let (w, b) = ((k >> 6) as usize, 1u64 << (k & 63));
        let sh = &self.shared;
        let v = if r == 0 {
            0
        } else {
            let parts = sounded.iter().filter_map(|(ch, _)| parts::part_of_channel(ch)).fold(0u8, |m, p| m | 1 << p);
            KEY_HELD | if r == R_RH { KEY_RIGHT } else { 0 } | (parts & KEY_PARTS)
        };
        sh.keys[k as usize & 127].store(v, Relaxed);
        let slot = slot.min(MAX_KEY_SOURCES - 1);
        if r != 0 {
            if self.src_keys[slot][w] & b == 0 {
                self.src_keys[slot][w] |= b;
                sh.src_held[slot].fetch_add(1, Relaxed);
            }
        } else {
            // A key-up releases the key whichever source pressed it (one key, one note).
            for (o, keys) in self.src_keys.iter_mut().enumerate() {
                if keys[w] & b != 0 {
                    keys[w] &= !b;
                    sh.src_held[o].fetch_sub(1, Relaxed);
                }
            }
        }
        self.ctl_signal = true;
    }

    /// The route bit of the chord section in the current area: the left hand in Lower,
    /// the right hand in Upper.
    #[inline]
    fn chord_side(&self) -> u8 {
        if self.shared.upper.load(Relaxed) { R_RH } else { R_LH }
    }

    fn recompute(&mut self) {
        // The keys the fingering type reads: the chord section of the current area, or
        // the whole keyboard for the Full Keyboard types (Lower only).
        let upper = self.shared.upper.load(Relaxed);
        let mode = Fingering::from_u8(self.shared.fingering.load(Relaxed));
        let side = if !upper && mode.full_keyboard() { R_LH | R_RH } else { self.chord_side() };
        let mut held = [false; 128];
        let mut mask = 0u16;
        for (k, (h, &r)) in held.iter_mut().zip(&self.route).enumerate() {
            if r & side != 0 {
                *h = true;
                mask |= 1 << (k % 12);
            }
        }
        let split = self.shared.split.load(Relaxed);
        // Upper: Fingered* whatever type is selected (OM p.51), i.e. Fingered without
        // 1+5, 1+8 or Chord Cancel.
        let c = if upper {
            fingering::detect(&self.rec, Fingering::Fingered, &held, split, self.current).and_then(|c| fingered_star(mask, c))
        } else {
            fingering::detect(&self.rec, mode, &held, split, self.current)
        };
        if let Some(c) = c {
            if Some(c) != self.current {
                self.current = Some(c);
                self.generation = self.generation.wrapping_add(1);
                self.shared.chord_ns.store(rt::now_ns(), Relaxed);
                self.shared.chord.store(c.pack(self.generation), Release);
                self.signal = true;
            }
        }
    }

    #[cfg(test)]
    fn key_msg(&mut self, m: &[u8]) {
        self.key_msg_from(0, m)
    }

    /// A message from the keyboard source in slot `slot`.
    fn key_msg_from(&mut self, slot: usize, m: &[u8]) {
        let split = self.shared.split.load(Relaxed);
        let st = m[0] & 0xF0;
        match (st, m.len()) {
            // The keyboard-part note path: pipeline.rs.
            (0x90, 3) if m[2] > 0 => self.key_down(slot, m[1] & 0x7F, m[2], split),
            (0x80, 3) | (0x90, 3) => self.key_up(slot, m[1] & 0x7F),
            // Volume and voice belong to the parts (their CC7 and program): the keyboard's
            // own are dropped, so the port, the synth and the screen never disagree.
            (0xB0, 3) if matches!(m[1], 0 | 7 | 32) => {}
            (0xC0, _) => {}
            // Polyphonic aftertouch names a key: it goes to the notes that key sounds, on
            // their channels, after the transpose and each part's octave.
            (0xA0, 3) => {
                for (ch, note) in self.keys.sounding(m[1] & 0x7F).iter() {
                    self.out.push(&[0xA0 | ch, note, m[2]]);
                }
            }
            // Wheels, pedals, pressure: to every keyboard part, on or off, so a pedal
            // release always reaches a note that is still ringing. Sustain affects all
            // notes played on the keyboard (RM, Assignable functions).
            (0xB0 | 0xE0 | 0xD0, _) => {
                let mut msg = [0u8; 3];
                msg[..m.len()].copy_from_slice(m);
                for ch in parts::CHANNEL {
                    msg[0] = st | ch;
                    self.out.push(&msg[..m.len()]);
                }
            }
            _ => {}
        }
    }

    fn pad_msg(&mut self, m: &[u8]) {
        let st = m[0] & 0xF0;
        if m.len() == 3 {
            self.shared.last_daw.store(u32::from_be_bytes([0, m[0], m[1], m[2]]), Relaxed);
        }
        if st == 0xB0 && m.len() == 3 {
            let (cc, v) = (m[1], m[2]);
            if cc == launchkey::SHIFT_CC {
                self.set_shift(v > 0);
                return;
            }
            if m[0] == launchkey::FEATURE_CH_STATUS {
                // Channel 7 carries mode reports and feature-control replies, whose CC
                // numbers overlap the buttons and faders: never act on them. A pad mode
                // report means the firmware's Shift menu was used (or DAW mode came
                // back), which can swallow the Shift release.
                if cc == launchkey::PAD_MODE_CC {
                    self.set_shift(false);
                }
                return;
            }
            if launchkey::FADER_CC.contains(&cc) {
                if cc == launchkey::MASTER_FADER_CC {
                    self.shared.master_hw.store(v, Relaxed);
                    self.ctl_signal = true;
                    // Soft takeover, as for the part faders (in the engine).
                    if let Some(s) = &self.synth {
                        if self.shared.master_moved.swap(false, Relaxed) {
                            self.master_takeover.software_moved(s.master.load(Relaxed));
                        }
                        if self.master_takeover.hardware(s.master.load(Relaxed), v) {
                            s.master.store(v, Relaxed);
                        }
                        s.master_waiting.store(self.master_takeover.waiting(), Relaxed);
                    }
                } else {
                    let f = (cc - launchkey::FADER_CC.start()) as usize;
                    let parts = &self.shared.parts;
                    let prev = parts.fader_hw[f].swap(v, Relaxed);
                    match parts.fader_page() {
                        // The engine thread sends the new volume as the part's CC7.
                        FaderPage::Panel => {
                            if f < parts::COUNT && parts.hw_fader(f, prev, v) {
                                self.signal = true;
                            }
                        }
                        // The Style faders' takeover state lives in the engine and only
                        // changes when a move arrives; a move lost to a full ring leaves it
                        // consistent.
                        FaderPage::Style => {
                            if self.cmd.push(Cmd::PartVolume(f as u8, v)).is_ok() {
                                self.signal = true;
                            }
                        }
                    }
                }
                return;
            }
            if launchkey::FADER_BTN_CC.contains(&cc) {
                if v > 0 {
                    self.fader_button(cc - launchkey::FADER_BTN_CC.start());
                }
                return;
            }
            match launchkey::cc_control(cc, self.shift) {
                Some(Control::Page(d)) if v > 0 => {
                    // Here, not on the control side: the next pad press must already
                    // read the new page.
                    self.shared.step_page(|p| p.step(d));
                    self.ctl_signal = true;
                }
                Some(Control::Act(a)) if v > 0 => self.act(a),
                Some(_) => {}
                None if v > 0 => self.unmapped(m),
                None => {}
            }
            return;
        }
        if st == 0x90 && m.len() == 3 && m[2] > 0 {
            if m[0] & 0x0F == 0 && launchkey::is_pad(m[1]) {
                // The firmware keeps Shift + pad for itself, so a pad note means Shift
                // is up, whatever release we missed.
                self.set_shift(false);
                let page = Page::from_u8(self.shared.page.load(Relaxed));
                if let Some(a) = launchkey::pad_action(page, m[1]) {
                    self.act(a);
                }
            } else {
                self.unmapped(m);
            }
        }
    }

    /// A button under fader `i` (0..8), or under the master fader (8): the fader page
    /// toggle. On the Panel page the buttons turn Right 1-3 and Left on/off (Shift: select
    /// the part for the voice keys); on the Style page they mute the Style parts.
    fn fader_button(&mut self, i: u8) {
        let parts = &self.shared.parts;
        if i == 8 {
            // Here rather than on the control side: the next fader move must already go
            // to the new page. The engine rebinds the Style faders on its next wake
            // (`Parts::take_rebind`).
            parts.toggle_fader_page();
            self.signal = true;
            self.ctl_signal = true;
            return;
        }
        match parts.fader_page() {
            FaderPage::Panel if (i as usize) < parts::COUNT && self.shift => self.act(Action::SelectPart(i)),
            // Left is refused under Manual Bass; its LED stays lit, as the bass sounds.
            FaderPage::Panel if (i as usize) < parts::COUNT => self.act(Action::PartOnOff(i)),
            FaderPage::Panel => {}
            FaderPage::Style => self.act(Action::Button(Button::TogglePart(i))),
        }
    }

    /// Engine buttons go straight to the engine; the rest to the session's control side,
    /// which runs them as the `AppCmd`s their keyboard shortcuts send.
    fn act(&mut self, a: Action) {
        match a {
            Action::Button(b) => {
                if self.cmd.push(Cmd::Button(b)).is_ok() {
                    self.signal = true;
                }
            }
            _ => {
                if let Some(tx) = self.actions.as_mut()
                    && tx.push(a).is_ok()
                {
                    self.ctl_signal = true;
                }
            }
        }
    }

    /// Shift pressed or released: mirrored for the screen when it changes.
    fn set_shift(&mut self, on: bool) {
        if self.shift != on {
            self.shift = on;
            self.shared.shift.store(on, Relaxed);
            self.ctl_signal = true;
        }
    }

    fn unmapped(&self, m: &[u8]) {
        self.shared.last_unmapped.store(u32::from_be_bytes([1, m[0], m[1], m[2]]), Relaxed);
    }
}

impl InputHandler for Input {
    fn packet(&mut self, tag: usize, host_time: u64, data: &[u8]) {
        if host_time != 0 {
            let now = rt::now_ns();
            let t = rt::host_to_ns(host_time);
            if now >= t {
                self.shared.input_lat.record(now - t);
            }
        }
        self.drain_releases();
        let slot = key_slot(tag);
        let i = slot.map_or(2, |s| 3 + s);
        let mut rs = self.running_status[i];
        for_each_message(data, &mut rs, |m| match slot {
            None => self.pad_msg(m),
            Some(s) => self.key_msg_from(s, m),
        });
        self.running_status[i] = rs;
    }

    fn end_of_list(&mut self) {
        self.out.flush();
        if self.signal {
            self.signal = false;
            self.shared.wake.signal();
        }
        if self.ctl_signal {
            self.ctl_signal = false;
            self.shared.ctl_wake.signal();
        }
    }
}

// ---------------------------------------------------------------------------
// Engine thread
// ---------------------------------------------------------------------------

pub struct EngineIo {
    pub input: Consumer<Cmd>,
    pub ui: Consumer<Cmd>,
    pub styles: Consumer<Box<Prepared>>,
    pub old: Producer<Box<Prepared>>,
    /// Style previews to play, and finished ones back to the control side to free.
    pub auditions: Consumer<Box<Audition>>,
    pub old_auditions: Producer<Box<Audition>>,
    /// Chart plans to play (engine/chart.rs), and replaced ones back to free.
    pub charts: Consumer<Box<ChartPlan>>,
    pub old_charts: Producer<Box<ChartPlan>>,
    pub snaps: Producer<Snapshot>,
    pub out: Out,
}

/// Send each keyboard part's volume as CC7 on its channel when it changed, to the port and
/// the built-in synth alike.
fn sync_part_volumes(out: &mut Out, parts: &Parts, last: &mut [u8; parts::COUNT]) {
    for (p, sent) in last.iter_mut().enumerate() {
        let v = parts.volume(p);
        if *sent != v {
            *sent = v;
            out.push(&[0xB0 | parts::CHANNEL[p], 7, v]);
        }
    }
}

/// The engine thread's work, one wake at a time. `run_engine` drives it with the wall
/// clock; an offline session steps it on a virtual one.
pub struct EngineLoop {
    pub engine: Engine,
    pub io: EngineIo,
    shared: Arc<Shared>,
    last_packed: u32,
    last_part_vol: [u8; parts::COUNT],
    last_snap: Option<Snapshot>,
    last_snap_ns: u64,
    /// The style preview playing, and how many of its chords have been played.
    audition: Option<(Box<Audition>, u8)>,
}

/// A command that starts or stops the band, or panics: a style preview ends first, so the
/// two never play over each other.
fn ends_audition(cmd: Cmd) -> bool {
    matches!(cmd, Cmd::Button(Button::StartStop) | Cmd::Arm | Cmd::Panic | Cmd::StopAudition)
}

impl EngineLoop {
    /// Sends the style's setup to the output.
    pub fn new(mut engine: Engine, mut io: EngineIo, shared: Arc<Shared>) -> EngineLoop {
        engine.send_init(&mut io.out);
        io.out.flush();
        // Out of range, so the first wake sends them all.
        EngineLoop {
            engine,
            io,
            shared,
            last_packed: 0,
            last_part_vol: [255u8; parts::COUNT],
            last_snap: None,
            last_snap_ns: 0,
            audition: None,
        }
    }

    /// When the next wake is due: the band's next event, or the preview's.
    pub fn next_deadline(&self) -> Option<u64> {
        let band = self.engine.next_deadline();
        let Some((a, n)) = &self.audition else { return band };
        let chord = if *n < AUDITION_BARS { a.engine.ns_at_bar(*n as u32) } else { a.engine.ns_at_bar(AUDITION_BARS as u32) };
        [band, a.engine.next_deadline(), Some(chord)].into_iter().flatten().min()
    }

    /// Start a preview: its first chord starts its engine (Sync Start is armed on a new
    /// engine), at `now`.
    fn start_audition(&mut self, mut a: Box<Audition>, now: u64) {
        a.engine.set_chord(a.chords[0], now, &mut self.io.out);
        if a.engine.is_running() {
            self.audition = Some((a, 1));
        } else {
            let _ = self.io.old_auditions.push(a);
        }
    }

    /// End the preview (if any): its notes off, the band's setup back on the channels.
    fn end_audition(&mut self) {
        if let Some((mut a, _)) = self.audition.take() {
            a.engine.stop(&mut self.io.out);
            if !self.engine.is_running() {
                self.engine.resync(&mut self.io.out);
            }
            let _ = self.io.old_auditions.push(a);
        }
    }

    /// The preview's chords at their bar lines, its notes, and its end after
    /// `AUDITION_BARS` bars.
    fn play_audition(&mut self, now: u64) {
        let Some((a, n)) = self.audition.as_mut() else { return };
        while *n < AUDITION_BARS && now >= a.engine.ns_at_bar(*n as u32) {
            let c = a.chords[*n as usize % a.chords.len()];
            a.engine.set_chord(c, now, &mut self.io.out);
            *n += 1;
        }
        a.engine.process(now, &mut self.io.out);
        if now >= a.engine.ns_at_bar(AUDITION_BARS as u32) || !a.engine.is_running() {
            self.end_audition();
        }
    }

    /// Styles the engine is done with go back to the control side to be freed there.
    fn retire_styles(&mut self) {
        while let Some(s) = self.engine.take_retired() {
            let _ = self.io.old.push(s);
        }
    }

    /// Input is waiting: a new chord or a command.
    #[inline]
    fn input_pending(&self) -> bool {
        self.shared.chord.load(Relaxed) != self.last_packed || self.io.input.slots() > 0 || self.io.ui.slots() > 0
    }

    /// One wake at `now`: take new styles, the chord and commands, play what is due,
    /// flush, publish a snapshot.
    #[inline]
    pub fn step(&mut self, now: u64) {
        let shared = self.shared.clone();
        // A style change or a new preview ends the preview playing.
        while let Ok(style) = self.io.styles.pop() {
            self.end_audition();
            self.engine.change_style(style, now, &mut self.io.out);
            self.retire_styles();
        }
        while let Ok(plan) = self.io.charts.pop() {
            if let Some(old) = self.engine.set_chart(plan, now) {
                let _ = self.io.old_charts.push(old);
            }
        }
        while let Ok(a) = self.io.auditions.pop() {
            self.end_audition();
            if self.engine.is_running() {
                // Refused: the band plays (the control side checks too).
                let _ = self.io.old_auditions.push(a);
            } else {
                self.start_audition(a, now);
            }
        }
        let packed = shared.chord.load(Acquire);
        if packed != self.last_packed {
            self.last_packed = packed;
            if let Some((c, _)) = Chord::unpack(packed) {
                shared.chord_lat.record(now.saturating_sub(shared.chord_ns.load(Relaxed)));
                // A Sync Start chord starts the band: the preview makes way first.
                if self.audition.is_some() && self.engine.starts_on_chord() && c.ty != CANCEL {
                    self.end_audition();
                }
                self.engine.set_chord(c, now, &mut self.io.out);
            }
        }
        // Read the fingering type and area here rather than taking a command for them, so
        // a full ring can never leave Sync Stop on in a Full Keyboard type.
        self.engine.allow_sync_stop(shared.sync_stop_allowed());
        rebind_faders(&mut self.engine, &shared.parts);
        while let Ok(cmd) = self.io.input.pop() {
            if self.audition.is_some() && ends_audition(cmd) {
                self.end_audition();
            }
            apply(&mut self.engine, &shared.parts, cmd, now, &mut self.io.out);
        }
        while let Ok(cmd) = self.io.ui.pop() {
            if self.audition.is_some() && ends_audition(cmd) {
                self.end_audition();
            }
            apply(&mut self.engine, &shared.parts, cmd, now, &mut self.io.out);
        }
        self.engine.process(now, &mut self.io.out);
        self.retire_styles();
        self.play_audition(now);
        let (engine, io) = (&mut self.engine, &mut self.io);
        sync_part_volumes(&mut io.out, &shared.parts, &mut self.last_part_vol);
        let t1 = rt::now_ns();
        io.out.flush();
        let t2 = rt::now_ns();
        shared.work_lat.record(t1.saturating_sub(now));
        shared.flush_lat.record(t2.saturating_sub(t1));

        let mut snap = engine.snapshot(now);
        snap.audition = self.audition.as_ref().map(|(a, n)| AuditionPos {
            id: a.id,
            bar: *n,
            bars: AUDITION_BARS,
            chord: n.saturating_sub(1),
        });
        let changed = self.last_snap != Some(snap);
        if (changed || now.saturating_sub(self.last_snap_ns) > 50_000_000) && io.snaps.push(snap).is_ok() {
            self.last_snap = Some(snap);
            self.last_snap_ns = now;
            if changed {
                // A non-blocking semaphore signal, after the flush.
                shared.ctl_wake.signal();
            }
        }
    }

    /// Everything off, flushed.
    pub fn stop(&mut self) {
        if let Some((mut a, _)) = self.audition.take() {
            a.engine.stop(&mut self.io.out);
            let _ = self.io.old_auditions.push(a);
        }
        self.engine.stop(&mut self.io.out);
        self.io.out.flush();
    }
}

pub fn run_engine(engine: Engine, io: EngineIo, shared: Arc<Shared>) {
    let rt_ok = std::env::var("YAHAHA_NO_RT").is_err() && rt::make_realtime(1_000_000, 300_000, 1_000_000);
    shared.engine_rt.store(rt_ok, Relaxed);
    let mut l = EngineLoop::new(engine, io, shared.clone());
    loop {
        if shared.quit.load(Relaxed) {
            l.stop();
            return;
        }
        let spin = shared.spin_ns.load(Relaxed);
        let now = rt::now_ns();
        let deadline = l.next_deadline();
        let mut timed_out = false;
        match deadline {
            Some(d) if d > now + spin => timed_out = !shared.wake.wait((d - now - spin).min(20_000_000)),
            Some(_) => timed_out = true,
            None => {
                shared.wake.wait(20_000_000);
            }
        }
        if let (true, Some(d)) = (timed_out, deadline) {
            let mut t = rt::now_ns();
            if t + spin < d {
                // The wait hit its cap well before the deadline: go around and wait again.
                continue;
            }
            // Finish the last stretch by spinning, for microsecond accuracy. Input can
            // still interrupt: check the chord word and command ring while spinning.
            while t < d {
                if l.input_pending() {
                    break;
                }
                std::hint::spin_loop();
                t = rt::now_ns();
            }
            if t >= d {
                shared.lateness.record(t - d);
            }
        }
        l.step(rt::now_ns());
    }
}

/// The faders went to the Style page: the Style parts' takeover starts from where the
/// faders physically were.
fn rebind_faders(engine: &mut Engine, parts: &Parts) {
    if let Some(hw) = parts.take_rebind() {
        engine.faders_at(hw);
    }
}

fn apply(engine: &mut Engine, parts: &Parts, cmd: Cmd, now: u64, out: &mut Out) {
    match cmd {
        Cmd::Button(b) => engine.button(b, now, out),
        Cmd::ChordReleased => engine.chord_released(now, out),
        Cmd::Arm => engine.arm(out),
        Cmd::PartVolume(p, v) => {
            // A page switch made before this move is seen with it (the ring push releases it).
            rebind_faders(engine, parts);
            engine.hw_fader(p, v, out)
        }
        Cmd::StyleVolume(p, v) => engine.set_volume_from_software(p, v, out),
        Cmd::ManualBass(on) => engine.set_manual_bass(on, out),
        Cmd::Transpose(t) => engine.set_transpose(t, now, out),
        Cmd::StopAudition => {}
        Cmd::Chart(s) => engine.set_chart_settings(s),
        Cmd::KeysOff => {
            // The source's pedal, wheels and pressure went to every keyboard part too, and
            // its releases will never come: with the pedal left down, All Notes Off would
            // only move the notes to the pedal (they ring on, and so does everything
            // played after).
            for ch in parts::CHANNEL {
                out.push(&[0xB0 | ch, 64, 0]);
                out.push(&[0xB0 | ch, 1, 0]);
                out.push(&[0xE0 | ch, 0x00, 0x40]);
                out.push(&[0xD0 | ch, 0]);
                out.push(&[0xB0 | ch, 123, 0]);
            }
        }
        Cmd::Panic => {
            engine.stop(out);
            for ch in 0..16u8 {
                out.push(&[0xB0 | ch, 123, 0]);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Wiring
// ---------------------------------------------------------------------------

pub struct Channels {
    pub input_tx: Producer<Cmd>,
    pub ui_tx: Producer<Cmd>,
    pub style_tx: Producer<Box<Prepared>>,
    pub old_rx: Consumer<Box<Prepared>>,
    pub snap_rx: Consumer<Snapshot>,
    pub audition_tx: Producer<Box<Audition>>,
    pub old_audition_rx: Consumer<Box<Audition>>,
    pub chart_tx: Producer<Box<ChartPlan>>,
    pub old_chart_rx: Consumer<Box<ChartPlan>>,
    pub io: EngineIo,
}

pub fn channels(out: Out) -> Channels {
    let (input_tx, input) = RingBuffer::new(256);
    let (ui_tx, ui) = RingBuffer::new(256);
    let (style_tx, styles) = RingBuffer::new(4);
    let (old, old_rx) = RingBuffer::new(8);
    let (snaps, snap_rx) = RingBuffer::new(256);
    let (audition_tx, auditions) = RingBuffer::new(4);
    let (old_auditions, old_audition_rx) = RingBuffer::new(8);
    let (chart_tx, charts) = RingBuffer::new(4);
    let (old_charts, old_chart_rx) = RingBuffer::new(8);
    Channels {
        input_tx,
        ui_tx,
        style_tx,
        old_rx,
        snap_rx,
        audition_tx,
        old_audition_rx,
        chart_tx,
        old_chart_rx,
        io: EngineIo { input, ui, styles, old, snaps, auditions, old_auditions, charts, old_charts, out },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An Input on the default split whose MIDI output is never flushed, so it needs no
    /// CoreMIDI endpoint.
    fn input(mode: Fingering) -> (Input, Arc<Shared>) {
        let shared = Arc::new(Shared::new(54));
        shared.fingering.store(mode.to_u8(), Relaxed);
        let (tx, _rx) = RingBuffer::new(16);
        let out = Out::new(PacketSink::new(rt::Target::Virtual(0)), None);
        (Input::new(shared.clone(), Recognizer::new(), tx, out), shared)
    }

    fn chord(shared: &Shared) -> Option<String> {
        Chord::unpack(shared.chord.load(Relaxed)).map(|(c, _)| c.name())
    }

    /// Malformed input on the keyboard port (a status byte in a data position, a data byte
    /// of 0x80 or more reaching `key_msg`) must never index past the 128 keys. A guard for
    /// the whole input path; `midi::data_bytes_are_always_below_0x80` is the #50 fix's test.
    #[test]
    fn malformed_key_bytes_never_panic() {
        let (mut inp, shared) = input(Fingering::FullKeyboard);
        use crate::midi::InputHandler;
        inp.packet(TAG_KEYS, 0, &[0x90, 0xFF, 100, 0x80, 0xC8, 0, 0x90, 60, 0xF0, 0xF7]);
        inp.packet(TAG_KEYS, 0, &[0x3C, 0x40, 0x90, 0x3C]);
        // Straight into the handler, past the stream splitter.
        inp.key_msg(&[0x90, 0xBC, 100]);
        inp.key_msg(&[0x80, 0xBC, 0]);
        inp.key_msg(&[0x90, 0xFF, 0x80]);
        assert_eq!(inp.route[0x3C], 0, "0xBC is key 60 (masked), and it was released");
        assert!(inp.route[0x7F] != 0, "0xFF is key 127");
        for k in [36, 40, 43] {
            inp.key_msg(&[0x90, k, 100]);
        }
        assert!(chord(&shared).is_some());
    }

    #[test]
    fn right_hand_ignored_outside_full_keyboard_types() {
        for mode in [Fingering::FingeredOnBass, Fingering::Fingered, Fingering::AiFingered, Fingering::MultiFinger] {
            let (mut inp, shared) = input(mode);
            for k in [72, 76, 79] {
                inp.key_msg(&[0x90, k, 100]);
            }
            assert_eq!(chord(&shared), None, "{mode:?}");
            inp.key_msg(&[0x90, 70, 100]); // RH Bb: would make C7 if it counted
            for k in [36, 40, 43] {
                inp.key_msg(&[0x90, k, 100]);
            }
            assert_eq!(chord(&shared).as_deref(), Some("C"), "{mode:?}");
        }
    }

    #[test]
    fn full_keyboard_reads_both_hands_and_tracks_releases() {
        let (mut inp, shared) = input(Fingering::FullKeyboard);
        for k in [72, 76, 79] {
            inp.key_msg(&[0x90, k, 100]);
        }
        assert_eq!(chord(&shared).as_deref(), Some("C")); // RH chord alone
        inp.key_msg(&[0x90, 40, 100]);
        assert_eq!(chord(&shared).as_deref(), Some("C/E")); // LH bass + RH chord
        inp.key_msg(&[0x90, 76, 0]); // note-off as velocity 0
        inp.key_msg(&[0x80, 79, 64]);
        assert!(inp.route[76] == 0 && inp.route[79] == 0 && inp.route[72] != 0 && inp.route[40] != 0);
        inp.key_msg(&[0x80, 40, 0]);
        assert_eq!(inp.route[40], 0);
        // RH A minor over the released keys: the old E and G are gone.
        for k in [69, 76] {
            inp.key_msg(&[0x90, k, 100]);
        }
        assert_eq!(chord(&shared).as_deref(), Some("Am"));
    }

    fn snd(notes: &[(u8, u8)]) -> Sounded {
        let mut s = Sounded::default();
        for &(c, n) in notes {
            s.push(c, n);
        }
        s
    }

    /// A key is released at the pitches and channels it sounded at, whatever changed meanwhile.
    #[test]
    fn keys_release_where_they_sounded() {
        let parts = Parts::new();
        parts.toggle(parts::LEFT);
        let mut k = Keys::new();
        let press = |k: &mut Keys, key: u8, left: bool, shift: i8| {
            let now = sounds(&parts, left, true, key, shift);
            (now, k.press(key, now))
        };
        assert_eq!(press(&mut k, 60, false, 2), (snd(&[(RH_CH, 62)]), snd(&[])));
        assert_eq!(press(&mut k, 48, true, -12), (snd(&[(LH_CH, 36)]), snd(&[])));
        // Transpose changes while both are held: the new press uses it, the releases do not.
        assert_eq!(press(&mut k, 64, false, 5), (snd(&[(RH_CH, 69)]), snd(&[])));
        assert_eq!(k.release(60), snd(&[(RH_CH, 62)]));
        assert_eq!(k.release(48), snd(&[(LH_CH, 36)]));
        assert_eq!(k.release(48), snd(&[]));
        // Retrigger without a release returns the old note to stop first.
        assert_eq!(press(&mut k, 64, false, 0), (snd(&[(RH_CH, 64)]), snd(&[(RH_CH, 69)])));
        assert_eq!(k.release(64), snd(&[(RH_CH, 64)]));
        // Out of MIDI range folds by octaves.
        assert_eq!(press(&mut k, 127, false, 12).0, snd(&[(RH_CH, 127)]));
        assert_eq!(press(&mut k, 2, false, -12).0, snd(&[(RH_CH, 2)]));
        assert_eq!(press(&mut k, 125, false, 5).0, snd(&[(RH_CH, 118)]));
    }

    /// The Right parts that are on sound together, each on its channel and octave. Left of
    /// the split: Left when it sounds (on, or Manual Bass); with Left off, the Right parts
    /// (OM p.48) unless that section only gives the chord (OM p.56).
    #[test]
    fn right_parts_layer_and_left_follows_its_switch() {
        let parts = Parts::new();
        assert_eq!(sounds(&parts, false, true, 60, 0), snd(&[(0, 60)]), "Right 1 alone at start");
        assert_eq!(sounds(&parts, true, true, 40, 0), snd(&[]), "Left off, chord section: chords only");
        assert_eq!(sounds(&parts, true, false, 40, 0), snd(&[(0, 40)]), "Left off: Right 1 over the whole keyboard");
        parts.toggle(parts::RIGHT2);
        parts.toggle(parts::RIGHT3);
        parts.octave[parts::RIGHT2].store(-1, Relaxed);
        parts.octave[parts::RIGHT3].store(2, Relaxed);
        assert_eq!(sounds(&parts, false, true, 60, 1), snd(&[(0, 61), (2, 49), (3, 85)]));
        assert_eq!(sounds(&parts, true, false, 40, 0), snd(&[(0, 40), (2, 28), (3, 64)]), "layered left of the split too");
        parts.toggle(parts::RIGHT1);
        assert_eq!(sounds(&parts, false, true, 60, 0), snd(&[(2, 48), (3, 84)]), "Right 1 off: its channel is silent");
        parts.set_manual_bass(true);
        parts.octave[parts::LEFT].store(-1, Relaxed);
        assert_eq!(sounds(&parts, true, true, 40, 0), snd(&[(1, 40)]), "Manual Bass sounds the left hand, at the pitch played");
        parts.set_manual_bass(false);
        parts.toggle(parts::LEFT);
        assert_eq!(sounds(&parts, true, true, 40, 0), snd(&[(1, 28)]));
        assert_eq!(sounds(&parts, true, false, 40, 0), snd(&[(1, 28)]), "Left on: the split holds in every mode");
    }

    /// Through `Input`, with Left off: keys left of the split play the Right parts in Upper
    /// detection and in the Full Keyboard types (OM p.48, p.51), and only give the chord in
    /// Lower detection otherwise (OM p.56). The chord is read as before in every case.
    #[test]
    fn left_off_right_parts_cover_the_keyboard_unless_chord_section() {
        let rig = |mode: Fingering, upper: bool| {
            let shared = Arc::new(Shared::new(54));
            shared.fingering.store(mode.to_u8(), Relaxed);
            shared.upper.store(upper, Relaxed);
            shared.manual_bass.store(false, Relaxed);
            let (cmd, _cmd_rx) = RingBuffer::new(16);
            let (synth, heard) = RingBuffer::new(64);
            (Input::new(shared.clone(), Recognizer::new(), cmd, Out::new(PacketSink::new(rt::Target::Virtual(0)), Some(synth))), shared, heard)
        };
        let drain = |h: &mut Consumer<[u8; 3]>| std::iter::from_fn(|| h.pop().ok()).collect::<Vec<_>>();
        for (mode, upper, sounds) in [
            (Fingering::Fingered, true, true),
            (Fingering::FullKeyboard, false, true),
            (Fingering::AiFullKeyboard, false, true),
            (Fingering::FingeredOnBass, false, false),
            (Fingering::Fingered, false, false),
        ] {
            let (mut inp, shared, mut heard) = rig(mode, upper);
            shared.parts.toggle(parts::RIGHT2);
            inp.key_msg(&[0x90, 40, 90]);
            let want = if sounds { vec![[0x90, 40, 90], [0x92, 40, 90]] } else { vec![] };
            assert_eq!(drain(&mut heard), want, "{mode:?} upper={upper}");
            inp.key_msg(&[0x80, 40, 0]);
            let want = if sounds { vec![[0x80, 40, 0], [0x82, 40, 0]] } else { vec![] };
            assert_eq!(drain(&mut heard), want, "{mode:?} upper={upper}: released where it sounded");
            if !upper {
                // The chord still comes from the left hand.
                for k in [36, 40, 43] {
                    inp.key_msg(&[0x90, k, 90]);
                }
                assert_eq!(chord(&shared).as_deref(), Some("C"), "{mode:?}");
            }
        }
    }

    /// The keyboard's own CC7, bank select and program changes never reach a part: its
    /// volume and voice are the part's own (the CC7 principle). Other controllers go on.
    #[test]
    fn keyboard_volume_and_voice_messages_are_dropped() {
        let shared = Arc::new(Shared::new(54));
        let (cmd, _cmd_rx) = RingBuffer::new(16);
        let (synth, mut heard) = RingBuffer::new(64);
        let mut input = Input::new(shared.clone(), Recognizer::new(), cmd, Out::new(PacketSink::new(rt::Target::Virtual(0)), Some(synth)));
        for m in [[0xB0, 7, 20], [0xB0, 0, 1], [0xB0, 32, 2]] {
            input.key_msg(&m);
        }
        input.key_msg(&[0xC0, 40]);
        assert!(heard.pop().is_err());
        assert_eq!(shared.parts.volume(parts::RIGHT1), 100);
        input.key_msg(&[0xB0, 1, 64]);
        assert_eq!(std::iter::from_fn(|| heard.pop().ok()).count(), parts::COUNT, "mod wheel to every part");
    }

    /// Through `Input`: a held layered note is released on every part it started on, even
    /// after the parts changed; pedals reach every part.
    #[test]
    fn layered_notes_release_where_they_started() {
        let shared = Arc::new(Shared::new(54));
        let (cmd, _cmd_rx) = RingBuffer::new(16);
        let (synth, mut heard) = RingBuffer::new(64);
        let mut input = Input::new(shared.clone(), Recognizer::new(), cmd, Out::new(PacketSink::new(rt::Target::Virtual(0)), Some(synth)));
        let mut sent = || std::iter::from_fn(|| heard.pop().ok()).collect::<Vec<_>>();
        let parts = &shared.parts;
        parts.toggle(parts::RIGHT2);
        input.key_msg(&[0x90, 72, 90]);
        assert_eq!(sent(), vec![[0x90, 72, 90], [0x92, 72, 90]]);
        parts.toggle(parts::RIGHT1);
        parts.toggle(parts::RIGHT3);
        parts.octave[parts::RIGHT2].store(1, Relaxed);
        input.key_msg(&[0x80, 72, 0]);
        assert_eq!(sent(), vec![[0x80, 72, 0], [0x82, 72, 0]]);
        input.key_msg(&[0xB0, 64, 127]);
        assert_eq!(sent(), vec![[0xB0, 64, 127], [0xB2, 64, 127], [0xB3, 64, 127], [0xB1, 64, 127]]);
        // Lower detection, Left off: the left hand only gives the chord.
        input.key_msg(&[0x90, 40, 90]);
        assert!(sent().is_empty());
        input.key_msg(&[0x80, 40, 0]);
        assert!(sent().is_empty());
    }

    /// Two held keys that land on the same note (an octave shift folding past the MIDI
    /// range, or a transpose change between presses): the note stops only when the last
    /// key holding it lets go.
    #[test]
    fn shared_note_stops_with_its_last_key() {
        let shared = Arc::new(Shared::new(54));
        let (cmd, _cmd_rx) = RingBuffer::new(16);
        let (synth, mut heard) = RingBuffer::new(64);
        let mut input = Input::new(shared.clone(), Recognizer::new(), cmd, Out::new(PacketSink::new(rt::Target::Virtual(0)), Some(synth)));
        let mut sent = || std::iter::from_fn(|| heard.pop().ok()).collect::<Vec<_>>();
        shared.parts.octave[parts::RIGHT1].store(2, Relaxed);
        input.key_msg(&[0x90, 96, 90]);
        input.key_msg(&[0x90, 108, 90]);
        assert_eq!(sent(), vec![[0x90, 120, 90], [0x90, 120, 90]], "108 + 24 folds onto 120");
        input.key_msg(&[0x80, 108, 0]);
        assert!(sent().is_empty(), "key 96 still holds 120");
        input.key_msg(&[0x80, 96, 0]);
        assert_eq!(sent(), vec![[0x80, 120, 0]]);
        // A transpose change between two presses lands them on one note too.
        shared.parts.octave[parts::RIGHT1].store(0, Relaxed);
        input.key_msg(&[0x90, 60, 90]);
        shared.key_shift.store(-2, Relaxed);
        input.key_msg(&[0x90, 62, 90]);
        input.key_msg(&[0x80, 60, 0]);
        assert_eq!(sent(), vec![[0x90, 60, 90], [0x90, 60, 90]]);
        // A retrigger of 62 while it holds 60 with key 60 gone: stop, then start again.
        input.key_msg(&[0x90, 62, 80]);
        assert_eq!(sent(), vec![[0x80, 60, 0], [0x90, 60, 80]]);
        input.key_msg(&[0x80, 62, 0]);
        assert_eq!(sent(), vec![[0x80, 60, 0]]);
    }

    /// Polyphonic aftertouch reaches the notes its key sounds, after the transpose and
    /// each part's octave, on those parts' channels only.
    #[test]
    fn poly_aftertouch_follows_the_sounding_notes() {
        let shared = Arc::new(Shared::new(54));
        let (cmd, _cmd_rx) = RingBuffer::new(16);
        let (synth, mut heard) = RingBuffer::new(64);
        let mut input = Input::new(shared.clone(), Recognizer::new(), cmd, Out::new(PacketSink::new(rt::Target::Virtual(0)), Some(synth)));
        let mut sent = || std::iter::from_fn(|| heard.pop().ok()).collect::<Vec<_>>();
        shared.parts.toggle(parts::RIGHT2);
        shared.parts.octave[parts::RIGHT1].store(1, Relaxed);
        shared.key_shift.store(2, Relaxed);
        input.key_msg(&[0x90, 60, 90]);
        assert_eq!(sent(), vec![[0x90, 74, 90], [0x92, 62, 90]]);
        input.key_msg(&[0xA0, 60, 40]);
        assert_eq!(sent(), vec![[0xA0, 74, 40], [0xA2, 62, 40]]);
        input.key_msg(&[0xA0, 61, 40]);
        assert!(sent().is_empty(), "a key that is not sounding: nothing");
        input.key_msg(&[0x80, 60, 0]);
        sent();
        input.key_msg(&[0xA0, 60, 40]);
        assert!(sent().is_empty(), "released: nothing");
    }

    /// Panel page: faders 1-4 set the keyboard parts' volumes (soft takeover), their
    /// buttons turn the parts on/off and Shift + button selects (as commands, on the control
    /// side, like the pads). The master button switches to
    /// the Style page, whose faders and buttons go to the engine; the engine hears where
    /// the faders physically are.
    #[test]
    fn fader_pages_route_faders_and_buttons() {
        use crate::engine::Button;
        let (mut input, shared, mut cmds, mut acts) = pads_rig();
        let parts = &shared.parts;
        assert_eq!(parts.fader_page(), FaderPage::Panel);
        input.pad_msg(&[0xB0, 6, 100]); // fader 2 = Right 2, at its level already
        input.pad_msg(&[0xB0, 6, 64]);
        assert_eq!(parts.volume(parts::RIGHT2), 64);
        input.pad_msg(&[0xB0, 12, 30]); // fader 8: unused on Panel
        assert!(cmds.pop().is_err(), "nothing reaches the Style parts");
        input.pad_msg(&[0xB0, 40, 127]); // button 4: Left on/off
        assert_eq!(acts.pop(), Ok(Action::PartOnOff(3)));
        input.pad_msg(&[0xB0, launchkey::SHIFT_CC, 127]);
        input.pad_msg(&[0xB0, 39, 127]); // Shift + button 3: select Right 3
        input.pad_msg(&[0xB0, launchkey::SHIFT_CC, 0]);
        assert_eq!(acts.pop(), Ok(Action::SelectPart(2)));
        assert!(acts.pop().is_err());

        input.pad_msg(&[0xB0, 45, 127]); // master button: Style page
        assert_eq!(parts.fader_page(), FaderPage::Style);
        let mut hw = [crate::engine::HW_UNKNOWN; 8];
        hw[1] = 64;
        hw[7] = 30;
        assert_eq!(parts.take_rebind(), Some(hw), "the engine hears where the faders are");
        input.pad_msg(&[0xB0, 6, 10]);
        assert!(matches!(cmds.pop(), Ok(Cmd::PartVolume(1, 10))));
        assert_eq!(parts.volume(parts::RIGHT2), 64, "Right 2 keeps its level");
        input.pad_msg(&[0xB0, 38, 127]);
        assert!(matches!(cmds.pop(), Ok(Cmd::Button(Button::TogglePart(1)))));
        // Back on Panel, fader 2 (now at 10) must pick Right 2 up at 64 first.
        input.pad_msg(&[0xB0, 45, 127]);
        assert_eq!(parts.fader_page(), FaderPage::Panel);
        assert!(parts.waiting(parts::RIGHT2));
        input.pad_msg(&[0xB0, 6, 12]);
        assert_eq!(parts.volume(parts::RIGHT2), 64, "no jump");
        input.pad_msg(&[0xB0, 6, 63]);
        assert_eq!(parts.volume(parts::RIGHT2), 63);
        assert!(cmds.pop().is_err());
    }

    /// The Style-page rebind cannot be lost: with the command ring full, the engine still
    /// rebinds before the first Style fader move it applies, so no part jumps.
    #[test]
    fn style_rebind_survives_a_full_ring() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/TickingAway.T162.sty");
        if !p.exists() {
            eprintln!("corpus missing; skipping");
            return;
        }
        let mut engine = Engine::new(Box::new(Prepared::new(&crate::sff::Style::load(&p).unwrap())));
        let mut out = Out::new(PacketSink::new(rt::Target::Virtual(0)), None);
        let (mut input, shared, mut cmds, _acts) = pads_rig();
        let parts = &shared.parts;
        // Style page: fader 1 picks its part up and moves it to 90.
        parts.set_fader_page(FaderPage::Style);
        let v0 = engine.snapshot(0).volumes[0];
        input.pad_msg(&[0xB0, 5, v0]);
        input.pad_msg(&[0xB0, 5, 90]);
        while let Ok(c) = cmds.pop() {
            apply(&mut engine, parts, c, 0, &mut out);
        }
        rebind_faders(&mut engine, parts);
        assert_eq!(engine.snapshot(0).volumes[0], 90);
        // Panel page, fader 1 down to 10 (Right 1). Fill the ring, then back to Style.
        input.pad_msg(&[0xB0, 45, 127]);
        input.pad_msg(&[0xB0, 5, 10]);
        while input.cmd.push(Cmd::Arm).is_ok() {}
        input.pad_msg(&[0xB0, 45, 127]);
        assert_eq!(parts.fader_page(), FaderPage::Style);
        while let Ok(c) = cmds.pop() {
            apply(&mut engine, parts, c, 0, &mut out);
        }
        // The next Style move arrives with the rebind still pending: no jump from 90 to 12.
        input.pad_msg(&[0xB0, 5, 12]);
        while let Ok(c) = cmds.pop() {
            apply(&mut engine, parts, c, 0, &mut out);
        }
        assert_eq!(engine.snapshot(0).volumes[0], 90, "no jump");
        assert_eq!(engine.snapshot(0).pickup & 1, 1, "fader 1 waits to reach its part");
    }

    /// The style's SysEx goes to the port only: the synth's ring carries channel messages.
    #[test]
    fn sysex_skips_the_synth() {
        let (synth, mut heard) = RingBuffer::new(8);
        let mut out = Out::new(PacketSink::new(rt::Target::Virtual(0)), Some(synth));
        out.push(&[0xF0, 0x43, 0x10, 0x4C, 0x02, 0x01, 0x00, 0x01, 0x10, 0xF7]);
        out.push(&[0xCC, 5]);
        assert_eq!(heard.pop().ok(), Some([0xCC, 5, 0]));
        assert!(heard.pop().is_err());
    }

    /// Through `Input`: notes sound transposed on their split channel, the chord is recognized
    /// from the keys as fingered, and releases go where the notes sounded even after the split
    /// and transpose move.
    #[test]
    fn input_transposes_notes_not_recognition() {
        let shared = Arc::new(Shared::new(59));
        let (cmd, mut cmd_rx) = RingBuffer::new(16);
        let (synth, mut heard) = RingBuffer::new(64);
        let out = Out::new(PacketSink::new(rt::Target::Virtual(0)), Some(synth));
        let mut input = Input::new(shared.clone(), Recognizer::new(), cmd, out);
        let mut sent = || std::iter::from_fn(|| heard.pop().ok()).collect::<Vec<_>>();
        shared.parts.toggle(parts::LEFT);

        shared.key_shift.store(2, Relaxed);
        for k in [48, 52, 55] {
            input.key_msg(&[0x90, k, 100]);
        }
        input.key_msg(&[0x90, 72, 90]);
        assert_eq!(sent(), vec![[0x91, 50, 100], [0x91, 54, 100], [0x91, 57, 100], [0x90, 74, 90]]);
        let (c, _) = Chord::unpack(shared.chord.load(Relaxed)).expect("C E G recognized");
        assert_eq!((c.root, c.ty), (0, 0), "fingered C major, not D");

        // Split and transpose change while keys are down.
        shared.key_shift.store(-3, Relaxed);
        shared.split.store(80, Relaxed);
        input.key_msg(&[0x80, 72, 0]);
        for k in [48, 52] {
            input.key_msg(&[0x90, k, 0]);
        }
        assert_eq!(sent(), vec![[0x80, 74, 0], [0x81, 50, 0], [0x81, 54, 0]]);
        assert!(cmd_rx.pop().is_err(), "a chord key is still held");
        input.key_msg(&[0x80, 55, 0]);
        assert_eq!(sent(), vec![[0x81, 57, 0]]);
        assert!(matches!(cmd_rx.pop(), Ok(Cmd::ChordReleased)));

        // Same key again, now in the left zone with the new shift; a retrigger stops it first.
        input.key_msg(&[0x90, 72, 80]);
        input.key_msg(&[0x90, 72, 70]);
        input.key_msg(&[0x80, 72, 0]);
        assert_eq!(sent(), vec![[0x91, 69, 80], [0x81, 69, 0], [0x91, 69, 70], [0x81, 69, 0]]);
    }

    /// Through `Input`: Pad Bank ▼/▲ switch pages, pads follow the page, engine buttons go
    /// to the engine and the rest to the UI ring, Shift turns ▲/▼ into the old toggles, and
    /// anything unmapped is recorded for the screen.
    #[test]
    fn launchkey_pages_route_pads_and_buttons() {
        use crate::engine::Button;
        let shared = Arc::new(Shared::new(54));
        let (cmd, mut cmds) = RingBuffer::new(16);
        let (act, mut acts) = RingBuffer::new(16);
        let mut input = Input::new(shared.clone(), Recognizer::new(), cmd, Out::new(PacketSink::new(rt::Target::Virtual(0)), None));
        input.set_actions(act);
        let page = || Page::from_u8(shared.page.load(Relaxed));

        input.pad_msg(&[0x90, 96, 100]);
        assert!(matches!(cmds.pop(), Ok(Cmd::Button(Button::Intro(0)))));
        input.pad_msg(&[0xB0, launchkey::PAD_DOWN_CC, 127]);
        input.pad_msg(&[0xB0, launchkey::PAD_DOWN_CC, 0]); // release does nothing
        assert_eq!(page(), Page::ChordSetup);
        input.pad_msg(&[0x90, 97, 100]);
        assert_eq!(acts.pop(), Ok(Action::Fingering(Fingering::Fingered)));
        input.pad_msg(&[0x90, 113, 100]);
        assert!(matches!(cmds.pop(), Ok(Cmd::Button(Button::StopAcmp))));
        input.pad_msg(&[0x90, 113, 0]); // pad release
        assert!(cmds.pop().is_err() && acts.pop().is_err());

        input.pad_msg(&[0xB0, launchkey::PAD_DOWN_CC, 127]);
        input.pad_msg(&[0xB0, launchkey::PAD_DOWN_CC, 127]); // stops at the last page
        assert_eq!(page(), Page::OtsParts);
        input.pad_msg(&[0x90, 114, 100]);
        assert_eq!(acts.pop(), Ok(Action::PartOnOff(2)));
        input.pad_msg(&[0x90, 99, 100]);
        assert_eq!(acts.pop(), Ok(Action::Ots(3)));

        // Shift + ▲ toggles the Left part and leaves the page alone.
        input.pad_msg(&[0xB0, launchkey::SHIFT_CC, 127]);
        input.pad_msg(&[0xB0, launchkey::PAD_UP_CC, 127]);
        input.pad_msg(&[0xB0, launchkey::SHIFT_CC, 0]);
        assert_eq!(acts.pop(), Ok(Action::PartOnOff(3)));
        assert_eq!(page(), Page::OtsParts);
        input.pad_msg(&[0xB0, launchkey::PAD_UP_CC, 127]);
        assert_eq!(page(), Page::ChordSetup);

        // Track buttons change style on any page.
        input.pad_msg(&[0xB0, launchkey::TRACK_RIGHT_CC, 127]);
        input.pad_msg(&[0xB0, launchkey::TRACK_LEFT_CC, 127]);
        assert_eq!((acts.pop(), acts.pop()), (Ok(Action::Style(1)), Ok(Action::Style(-1))));

        assert_eq!(shared.last_unmapped.load(Relaxed), 0);
        input.pad_msg(&[0xB0, 51, 127]);
        assert_eq!(shared.last_unmapped.load(Relaxed), 0x01_B0_33_7F);
        input.pad_msg(&[0x99, 36, 90]); // a Drum-mode pad
        assert_eq!(shared.last_unmapped.load(Relaxed), 0x01_99_24_5A);
        input.pad_msg(&[0x90, 119, 100]); // blank pad on page 2: a known pad, not unmapped
        assert_eq!(shared.last_unmapped.load(Relaxed), 0x01_99_24_5A);
        assert!(cmds.pop().is_err() && acts.pop().is_err());
    }

    fn pads_rig() -> (Input, Arc<Shared>, Consumer<Cmd>, Consumer<Action>) {
        let shared = Arc::new(Shared::new(54));
        let (cmd, cmds) = RingBuffer::new(16);
        let (act, acts) = RingBuffer::new(16);
        let mut input = Input::new(shared.clone(), Recognizer::new(), cmd, Out::new(PacketSink::new(rt::Target::Virtual(0)), None));
        input.set_actions(act);
        (input, shared, cmds, acts)
    }

    /// Channel 7 carries feature-control replies whose CC numbers overlap the buttons
    /// (e.g. 6Bh = 107 is Arp velocity) and faders: they must never act.
    #[test]
    fn channel_7_replies_are_not_button_presses() {
        let (mut input, shared, mut cmds, mut acts) = pads_rig();
        for cc in [102, 103, 104, 105, 106, 107, 115, 116, 5, 6, 7, 37, 45] {
            input.pad_msg(&[0xB6, cc, 127]);
        }
        assert_eq!(Page::from_u8(shared.page.load(Relaxed)), Page::Sections);
        assert!(cmds.pop().is_err() && acts.pop().is_err());
        assert_eq!(shared.last_unmapped.load(Relaxed), 0, "not reported as unmapped either");
        // The same numbers on channel 1 are the buttons.
        input.pad_msg(&[0xB0, launchkey::PAD_DOWN_CC, 127]);
        assert_eq!(Page::from_u8(shared.page.load(Relaxed)), Page::ChordSetup);
    }

    /// A lost Shift release doesn't stick: a pad note (the firmware keeps Shift + pad for
    /// itself) or a pad mode report (Shift menu used, or DAW mode re-entered) clears it.
    #[test]
    fn shift_does_not_stick() {
        let (mut input, shared, _cmds, mut acts) = pads_rig();
        let page = || Page::from_u8(shared.page.load(Relaxed));

        input.pad_msg(&[0xB0, launchkey::SHIFT_CC, 127]); // release never arrives
        input.pad_msg(&[0xB0, launchkey::PAD_DOWN_CC, 127]);
        assert_eq!(acts.pop(), Ok(Action::ToggleOtsLink));
        input.pad_msg(&[0x90, 96, 100]);
        input.pad_msg(&[0xB0, launchkey::PAD_DOWN_CC, 127]);
        assert_eq!(page(), Page::ChordSetup, "a pad press cleared Shift");

        input.pad_msg(&[0xB6, launchkey::SHIFT_CC, 127]); // Shift reported on channel 7 counts too
        input.pad_msg(&[0xB0, launchkey::PAD_UP_CC, 127]);
        assert_eq!(acts.pop(), Ok(Action::PartOnOff(3)));
        input.pad_msg(&[0xB6, launchkey::PAD_MODE_CC, 2]); // back in DAW pad mode
        input.pad_msg(&[0xB0, launchkey::PAD_UP_CC, 127]);
        assert_eq!(page(), Page::Sections, "the pad mode report cleared Shift");
        assert!(acts.pop().is_err());
    }

    /// Page moves from both threads go through one compare-and-swap.
    #[test]
    fn page_steps_from_either_side() {
        let shared = Shared::new(54);
        shared.step_page(|p| p.step(1));
        shared.step_page(|p| p.cycle(1));
        assert_eq!(Page::from_u8(shared.page.load(Relaxed)), Page::OtsParts);
        shared.step_page(|p| p.step(1));
        assert_eq!(Page::from_u8(shared.page.load(Relaxed)), Page::OtsParts);
        shared.step_page(|p| p.cycle(1));
        assert_eq!(Page::from_u8(shared.page.load(Relaxed)), Page::Sections);
    }
}

#[cfg(test)]
mod detection_area {
    use super::*;
    use crate::rt::Target;

    struct Rig {
        input: Input,
        played: Consumer<[u8; 3]>,
        cmds: Consumer<Cmd>,
        shared: Arc<Shared>,
    }

    /// An input handler whose output is captured through the synth ring (nothing is flushed
    /// to CoreMIDI). Split at F#2 (54); Left on, so both hands sound.
    fn rig(upper: bool) -> Rig {
        let shared = Arc::new(Shared::new(54));
        shared.upper.store(upper, Relaxed);
        shared.parts.toggle(parts::LEFT);
        let (tx, played) = RingBuffer::new(1024);
        let (cmd, cmds) = RingBuffer::new(64);
        let input = Input::new(shared.clone(), Recognizer::new(), cmd, Out::new(PacketSink::new(Target::Virtual(0)), Some(tx)));
        Rig { input, played, cmds, shared }
    }

    impl Rig {
        fn on(&mut self, keys: &[u8]) {
            for &k in keys {
                self.input.key_msg(&[0x90, k, 100]);
            }
        }
        fn off(&mut self, keys: &[u8]) {
            for &k in keys {
                self.input.key_msg(&[0x80, k, 0]);
            }
        }
        fn chord(&self) -> Option<Chord> {
            Chord::unpack(self.shared.chord.load(Relaxed)).map(|(c, _)| c)
        }
        fn played(&mut self) -> Vec<[u8; 3]> {
            std::iter::from_fn(|| self.played.pop().ok()).collect()
        }
        fn released(&mut self) -> bool {
            std::iter::from_fn(|| self.cmds.pop().ok()).any(|c| matches!(c, Cmd::ChordReleased))
        }
    }

    #[test]
    fn fingered_star_drops_1_5_1_8_cancel_and_the_bass() {
        let rec = Recognizer::new();
        let pcs = |ps: &[u8]| ps.iter().fold(0u16, |m, &p| m | 1 << (p % 12));
        let fs = |ps: &[u8]| rec.recognize(pcs(ps), ps[0] % 12).and_then(|c| fingered_star(pcs(ps), c));
        assert_eq!(fs(&[0]), None); // single note
        assert_eq!(fs(&[0, 12]), None); // 1+8
        assert_eq!(fs(&[0, 7]), None); // 1+5
        assert_eq!(fs(&[7, 12]), None); // 1+5, inverted
        assert_eq!(fs(&[0, 4]), None); // a melody third is not a chord in Fingered*
        assert_eq!(fs(&[0, 1, 2]), None); // Cancel
        assert_eq!(fs(&[0, 4, 7]), Some(Chord::new(0, 0)));
        assert_eq!(fs(&[9, 12, 16]), Some(Chord::new(9, 8))); // Am
        // An inversion is still the chord, with the root as the bass.
        let c = fs(&[4, 7, 12]).unwrap();
        assert_eq!((c.root, c.ty, c.bass), (0, 0, None));
    }

    /// Lower (default): the left hand is the chord section and sounds on the LH channel.
    #[test]
    fn lower_detects_the_left_hand() {
        let mut r = rig(false);
        r.on(&[36, 40, 43]);
        assert_eq!(r.chord(), Some(Chord::new(0, 0)));
        r.on(&[72, 76, 79, 81]); // right-hand melody never changes the chord
        assert_eq!(r.chord(), Some(Chord::new(0, 0)));
        let p = r.played();
        assert!(p[..3].iter().all(|m| m[0] == 0x90 | LH_CH));
        assert!(p[3..].iter().all(|m| m[0] == 0x90 | RH_CH));
    }

    /// Upper: the chord comes from the right hand; the left hand plays the Left part and
    /// never changes the chord.
    #[test]
    fn upper_detects_the_right_hand() {
        let mut r = rig(true);
        r.on(&[36, 40, 43]); // a C triad in the left hand: just bass notes now
        assert_eq!(r.chord(), None);
        assert!(r.played().iter().all(|m| m[0] == 0x90 | LH_CH));
        r.on(&[69, 72, 76]); // Am in the right hand
        assert_eq!(r.chord(), Some(Chord::new(9, 8)));
        assert!(r.played().iter().all(|m| m[0] == 0x90 | RH_CH));
        r.off(&[69, 72, 76]);
        assert!(r.released(), "releasing the right-hand chord is what Sync Stop sees");
        // Melody alone: single notes, fifths, octaves don't touch the chord.
        r.on(&[79]);
        r.on(&[84]);
        r.on(&[91]);
        assert_eq!(r.chord(), Some(Chord::new(9, 8)));
        r.off(&[79, 84, 91]);
        assert!(r.released(), "all right-hand keys up");
        // A new chord, played as an inversion over a left-hand note: root position, no slash.
        r.on(&[38]);
        r.on(&[67, 71, 74, 77]); // G7 in the right hand
        let c = r.chord().unwrap();
        assert_eq!((c.root, c.ty, c.bass), (7, 19, None));
        // Releasing the left hand does not count as releasing the chord.
        r.played();
        r.off(&[36, 40, 43, 38]);
        assert!(!r.released());
        assert!(r.played().iter().all(|m| m[0] == 0x80 | LH_CH));
    }

    /// Switching the area while keys are held: note-offs follow where each note went.
    #[test]
    fn note_offs_follow_the_note_ons_across_a_switch() {
        let mut r = rig(false);
        r.on(&[36, 40, 43]);
        r.shared.upper.store(true, Relaxed);
        r.on(&[72, 76, 79]); // lower-mode chord keys still held, so C + C = C
        r.played();
        r.off(&[36, 40, 43]);
        assert!(r.played().iter().all(|m| m[0] == 0x80 | LH_CH));
        assert!(!r.released(), "the right hand still holds the chord");
        r.off(&[72, 76, 79]);
        assert!(r.played().iter().all(|m| m[0] == 0x80 | RH_CH));
        assert!(r.released());
    }

    /// A key's chord membership follows the current area, not the one it was pressed in.
    #[test]
    fn held_keys_join_the_chord_of_the_current_area() {
        // Upper -> Lower with a right-hand melody note held: it is no longer a chord key.
        let mut r = rig(true);
        r.on(&[78]);
        r.shared.upper.store(false, Relaxed);
        r.on(&[45, 48, 52]);
        assert_eq!(r.chord(), Some(Chord::new(9, 8)), "Am, not Am6 from the held F#");
        r.off(&[45, 48, 52]);
        assert!(r.released(), "the held right-hand key does not keep the chord section down");
        r.off(&[78]);
        assert!(!r.released());

        // Upper -> Lower with left-hand keys held: they are chord keys straight away.
        let mut r = rig(true);
        r.on(&[36, 40]);
        assert_eq!(r.chord(), None);
        r.shared.upper.store(false, Relaxed);
        r.on(&[43]);
        assert_eq!(r.chord(), Some(Chord::new(0, 0)), "C from all three held keys");
        r.off(&[36, 40]);
        assert!(!r.released());
        r.off(&[43]);
        assert!(r.released());
    }

    /// Upper overrides the selected fingering type with Fingered*: a Full Keyboard type no
    /// longer reads the left hand, Single Finger shapes are melody, and Sync Stop is
    /// available. Back in Lower the selected type applies again.
    #[test]
    fn upper_overrides_the_fingering_type() {
        let mut r = rig(true);
        r.shared.fingering.store(Fingering::FullKeyboard.to_u8(), Relaxed);
        assert!(r.shared.sync_stop_allowed());
        r.on(&[40]); // left-hand E: would make C/E in Full Keyboard
        r.on(&[72, 76, 79]);
        assert_eq!(r.chord(), Some(Chord::new(0, 0)));
        r.off(&[40, 72, 76, 79]);
        r.shared.fingering.store(Fingering::SingleFinger.to_u8(), Relaxed);
        r.on(&[79, 82]); // Single Finger Gm7 shape: two notes, not a Fingered* chord
        assert_eq!(r.chord(), Some(Chord::new(0, 0)));
        r.off(&[79, 82]);
        r.shared.upper.store(false, Relaxed);
        r.shared.fingering.store(Fingering::FullKeyboard.to_u8(), Relaxed);
        assert!(!r.shared.sync_stop_allowed());
        r.on(&[40]);
        r.on(&[72, 76, 79]);
        assert_eq!(r.chord().map(|c| c.name()).as_deref(), Some("C/E"));
    }

    /// Moving the split while a key is held: a repeated note-on moves the note across
    /// (off where it sounded, on where it now belongs) and its note-off follows it.
    #[test]
    fn repeated_note_on_after_a_split_change_moves_the_note() {
        let mut r = rig(false);
        r.on(&[38, 41, 45]); // Dm below the split (54)
        assert_eq!(r.chord(), Some(Chord::new(2, 8)));
        r.played();
        r.shared.split.store(44, Relaxed);
        r.on(&[45]); // now above the split
        assert_eq!(r.played(), vec![[0x80 | LH_CH, 45, 0], [0x90 | RH_CH, 45, 100]]);
        r.off(&[45]);
        assert_eq!(r.played(), vec![[0x80 | RH_CH, 45, 0]]);
        assert!(!r.released(), "38 and 41 are still held in the chord section");
        r.off(&[38, 41]);
        assert_eq!(r.played(), vec![[0x80 | LH_CH, 38, 0], [0x80 | LH_CH, 41, 0]]);
        assert!(r.released());
    }
}

#[cfg(test)]
mod source_tests {
    use super::*;
    use crate::midi::InputHandler;

    type Rig = (Input, Arc<Shared>, Consumer<[u8; 3]>, Consumer<Cmd>, Producer<u8>);

    /// An input with its synth feed, command ring and release ring.
    fn rig() -> Rig {
        let shared = Arc::new(Shared::new(54));
        let (cmd, cmd_rx) = RingBuffer::new(64);
        let (synth, heard) = RingBuffer::new(256);
        let mut input = Input::new(shared.clone(), Recognizer::new(), cmd, Out::new(PacketSink::new(rt::Target::Virtual(0)), Some(synth)));
        let (rel_tx, rel_rx) = RingBuffer::new(MAX_KEY_SOURCES);
        input.set_release(rel_rx);
        (input, shared, heard, cmd_rx, rel_tx)
    }

    fn drain<T>(c: &mut Consumer<T>) -> Vec<T> {
        std::iter::from_fn(|| c.pop().ok()).collect()
    }

    /// A source disconnected with keys down: the keys it alone held are released (their
    /// notes stop, the chord section lets go, Sync Stop hears it); a key another source
    /// also holds keeps sounding. Nothing happens to keys of other sources.
    #[test]
    fn a_dropped_source_releases_its_keys() {
        let (mut inp, shared, mut heard, mut cmds, mut rel) = rig();
        let (a, b) = (key_tag(1), key_tag(2));
        inp.packet(a, 0, &[0x90, 36, 90, 0x90, 40, 90, 0x90, 43, 90, 0x90, 60, 90, 0x90, 72, 90]);
        inp.packet(b, 0, &[0x90, 76, 90, 0x90, 72, 80]);
        inp.end_of_list();
        assert_eq!(shared.src_held[1].load(Relaxed), 5);
        assert_eq!(shared.src_held[2].load(Relaxed), 2);
        let (held, rh) = shared.held_keys();
        assert_eq!((held[0].count_ones() + held[1].count_ones(), rh[1] & (1 << (72 - 64)) != 0), (6, true));
        drain(&mut heard);
        drain(&mut cmds);
        // Source 1 goes; the input thread hears of it with the next packet from anywhere.
        rel.push(1).unwrap();
        inp.packet(b, 0, &[0xFE]);
        inp.end_of_list();
        let offs: Vec<u8> = drain(&mut heard).iter().filter(|m| m[0] == 0x80).map(|m| m[1]).collect();
        // (The left hand only gives the chord here: Lower, Left off.)
        assert_eq!(offs, vec![60], "72 is still held on source 2");
        assert!(drain(&mut cmds).iter().any(|c| matches!(c, Cmd::ChordReleased)), "the chord section let go");
        assert_eq!(shared.src_held[1].load(Relaxed), 0);
        let (held, _) = shared.held_keys();
        assert_eq!(held, [0, (1 << (72 - 64)) | (1 << (76 - 64))]);
        // Source 2's own note-offs still work.
        inp.packet(b, 0, &[0x80, 72, 0, 0x80, 76, 0]);
        inp.end_of_list();
        assert_eq!(drain(&mut heard).iter().filter(|m| m[0] == 0x80).count(), 2);
        assert_eq!(shared.held_keys().0, [0, 0]);
    }

    /// A source dropped with the sustain pedal down: its pedal-up never comes, so the
    /// keyboard parts' pedal is released before All Notes Off (which a held pedal would
    /// only turn into sustained notes), and its wheels are centred.
    #[test]
    fn keys_off_lifts_the_dropped_sources_pedal() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/TickingAway.T162.sty");
        if !p.exists() {
            eprintln!("corpus missing; skipping");
            return;
        }
        let mut engine = Engine::new(Box::new(Prepared::new(&crate::sff::Style::load(&p).unwrap())));
        let (tx, mut heard) = RingBuffer::new(256);
        let mut out = Out::new(PacketSink::new(rt::Target::Virtual(0)), Some(tx));
        let parts = Parts::new();
        apply(&mut engine, &parts, Cmd::KeysOff, 0, &mut out);
        let sent = drain(&mut heard);
        for ch in parts::CHANNEL {
            let at = |m: [u8; 3]| sent.iter().position(|x| *x == m);
            let (pedal, off) = (at([0xB0 | ch, 64, 0]), at([0xB0 | ch, 123, 0]));
            assert!(pedal.is_some() && off.is_some() && pedal < off, "ch {}: pedal up, then all notes off", ch + 1);
            assert!(at([0xE0 | ch, 0, 0x40]).is_some(), "ch {}: bend centred", ch + 1);
        }
    }

    /// Running status is kept per source: two keyboards interleaving can't mix theirs.
    #[test]
    fn running_status_is_per_source() {
        let (mut inp, shared, _heard, _cmds, _rel) = rig();
        inp.packet(key_tag(1), 0, &[0x90, 60, 90]);
        inp.packet(key_tag(2), 0, &[0x80, 64, 0]);
        inp.packet(key_tag(1), 0, &[62, 90]); // running status note-on from source 1
        inp.end_of_list();
        let (held, _) = shared.held_keys();
        assert_eq!(held[0], (1 << 60) | (1 << 62));
        assert_eq!(key_slot(TAG_PADS), None);
        assert_eq!(key_slot(TAG_KEYS), Some(0));
        assert_eq!(key_slot(key_tag(3)), Some(3));
    }
}

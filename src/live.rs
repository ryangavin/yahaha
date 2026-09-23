//! Live runtime: the MIDI input handler (runs on CoreMIDI's thread), the engine thread,
//! and the lock-free plumbing between them and the UI.
//!
//!   CoreMIDI thread ──chord (AtomicU32) + commands (SPSC) + semaphore──▶ engine thread
//!   UI thread ────────styles/commands (SPSC) + semaphore───────────────▶ engine thread
//!   engine thread ────snapshots (SPSC), old styles (SPSC)──────────────▶ UI thread
//!
//! Neither real-time side ever waits on the other: producers use try-push and a
//! non-blocking semaphore signal; the engine only sleeps on the semaphore with a timeout
//! equal to its next deadline.

use crate::engine::{Button, Engine, Prepared, Snapshot};
use crate::fingering::{self, Fingering};
use crate::launchkey;
use crate::midi::{for_each_message, InputHandler};
use crate::rt::{self, Histogram, PacketSink, Wakeup};
use crate::theory::{Chord, Recognizer, CANCEL, ONE_PLUS_EIGHT, ONE_PLUS_FIVE};
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, Ordering::*};
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub enum Cmd {
    Button(Button),
    ChordReleased,
    PartVolume(u8, u8),
    /// Manual Bass in effect: mute the Style's Bass part.
    ManualBass(bool),
    Arm,
    Panic,
}

pub struct Shared {
    pub chord: AtomicU32,
    pub chord_ns: AtomicU64,
    pub wake: Wakeup,
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
    /// Engine wake lateness vs. its deadline.
    pub lateness: Histogram,
    /// CoreMIDI packet timestamp -> our callback.
    pub input_lat: Histogram,
    /// Chord published by the input thread -> applied by the engine.
    pub chord_lat: Histogram,
    pub engine_rt: AtomicBool,
    /// Last message from the Launchkey DAW port, packed 0x00SSDDVV (for the on-screen readout).
    pub last_daw: AtomicU32,
    /// Time spent in engine.process / in the CoreMIDI send, per wake.
    pub flush_lat: Histogram,
    pub work_lat: Histogram,
    pub spin_ns: AtomicU64,
}

impl Shared {
    pub fn new(split: u8) -> Shared {
        Shared {
            chord: AtomicU32::new(0),
            chord_ns: AtomicU64::new(0),
            wake: Wakeup::new(),
            quit: AtomicBool::new(false),
            split: AtomicU8::new(split),
            fingering: AtomicU8::new(Fingering::FingeredOnBass.to_u8()),
            upper: AtomicBool::new(false),
            manual_bass: AtomicBool::new(true),
            lateness: Histogram::new(),
            input_lat: Histogram::new(),
            chord_lat: Histogram::new(),
            engine_rt: AtomicBool::new(false),
            last_daw: AtomicU32::new(0),
            flush_lat: Histogram::new(),
            work_lat: Histogram::new(),
            spin_ns: AtomicU64::new(150_000),
        }
    }

    /// Sync Stop is available: always in Upper (Fingered*), else unless the fingering type
    /// is a Full Keyboard one.
    pub fn sync_stop_allowed(&self) -> bool {
        self.upper.load(Relaxed) || Fingering::from_u8(self.fingering.load(Relaxed)).allows_sync_stop()
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

pub const TAG_KEYS: usize = 1;
pub const TAG_PADS: usize = 2;

pub const RH_CH: u8 = 0;
pub const LH_CH: u8 = 1;

/// Which side of the split a held key went to at note-on, so its note-off follows it
/// even if the split or the detection area changed while it was held. Whether a key is
/// a chord key is not stored: it is decided by its side and the area at the time.
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

pub struct Input {
    shared: Arc<Shared>,
    rec: Recognizer,
    /// Which side of the split each held key went to (`R_LH` / `R_RH`, 0 = not held).
    route: [u8; 128],
    current: Option<Chord>,
    generation: u16,
    cmd: Producer<Cmd>,
    out: Out,
    running_status: [u8; 3],
    signal: bool,
    synth: Option<Arc<crate::synth::SynthControl>>,
}

impl Input {
    pub fn new(shared: Arc<Shared>, rec: Recognizer, cmd: Producer<Cmd>, out: Out) -> Input {
        Input {
            shared,
            rec,
            route: [0; 128],
            current: None,
            generation: 0,
            cmd,
            out,
            running_status: [0; 3],
            signal: false,
            synth: None,
        }
    }

    pub fn set_synth(&mut self, ctl: Option<Arc<crate::synth::SynthControl>>) {
        self.synth = ctl;
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

    fn key_msg(&mut self, m: &[u8]) {
        let split = self.shared.split.load(Relaxed);
        let st = m[0] & 0xF0;
        match (st, m.len()) {
            (0x90, 3) if m[2] > 0 => {
                let k = m[1];
                // Lower: the chord section is the left hand. Upper: it is the right hand
                // (above Split Point (Left)), and the left hand only plays the Left part.
                let left = k <= split;
                let r = if left { R_LH } else { R_RH };
                let chord = r == self.chord_side();
                let prev = self.route[k as usize];
                // A repeated note-on after the split moved: release where it sounded before.
                if prev & R_LH != 0 && !left {
                    self.out.push(&[0x80 | LH_CH, k, 0]);
                } else if prev & R_RH != 0 && left {
                    self.out.push(&[0x80 | RH_CH, k, 0]);
                }
                self.route[k as usize] = r;
                self.out.push(&[0x90 | if left { LH_CH } else { RH_CH }, k, m[2]]);
                // The Full Keyboard types (Lower only) read both hands.
                if chord
                    || (!self.shared.upper.load(Relaxed)
                        && Fingering::from_u8(self.shared.fingering.load(Relaxed)).full_keyboard())
                {
                    self.recompute();
                }
            }
            (0x80, 3) | (0x90, 3) => {
                let k = m[1];
                let r = std::mem::take(&mut self.route[k as usize]);
                if r != 0 {
                    if r & R_LH != 0 {
                        self.out.push(&[0x80 | LH_CH, k, 0]);
                    }
                    if r & R_RH != 0 {
                        self.out.push(&[0x80 | RH_CH, k, 0]);
                    }
                    // Sync Stop: the last key of the current chord section went up.
                    let side = self.chord_side();
                    if r & side != 0
                        && !self.route.iter().any(|&x| x & side != 0)
                        && self.cmd.push(Cmd::ChordReleased).is_ok()
                    {
                        self.signal = true;
                    }
                } else {
                    self.out.push(&[0x80 | RH_CH, k, 0]);
                    // Also release on the LH channel in case the split moved while held.
                    if k <= split {
                        self.out.push(&[0x80 | LH_CH, k, 0]);
                    }
                }
            }
            // Wheels, pedals, pressure: to the right-hand channel.
            (0xB0 | 0xE0 | 0xD0 | 0xA0 | 0xC0, _) => {
                let mut msg = [0u8; 3];
                msg[..m.len()].copy_from_slice(m);
                msg[0] = st | RH_CH;
                self.out.push(&msg[..m.len()]);
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
            if launchkey::FADER_CC.contains(&cc) {
                if cc == 13 {
                    if let Some(s) = &self.synth {
                        s.master.store(v, Relaxed);
                    }
                } else if self.cmd.push(Cmd::PartVolume(cc - 5, v)).is_ok() {
                    self.signal = true;
                }
                return;
            }
            if cc == launchkey::LEFT_BTN_CC || cc == launchkey::OTS_LINK_BTN_CC {
                if v > 0 {
                    if let Some(s) = &self.synth {
                        let flag = if cc == launchkey::LEFT_BTN_CC { &s.lh_sound } else { &s.ots_link };
                        flag.store(!flag.load(Relaxed), Relaxed);
                    }
                }
                return;
            }
            if launchkey::FADER_BTN_CC.contains(&cc) {
                if v > 0 {
                    if let Some(s) = &self.synth {
                        if cc == 45 {
                            let l = !s.layer_mode.load(Relaxed);
                            s.layer_mode.store(l, Relaxed);
                        } else {
                            s.press_slot(cc - 37);
                        }
                    }
                }
                return;
            }
        }
        let b = match (st, m.len()) {
            (0x90, 3) if m[2] > 0 && m[0] & 0x0F == 0 => launchkey::pad_button(m[1]),
            (0xB0, 3) if m[2] > 0 => launchkey::cc_button(m[1]),
            _ => None,
        };
        if let Some(b) = b {
            if self.cmd.push(Cmd::Button(b)).is_ok() {
                self.signal = true;
            }
        }
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
        let mut rs = self.running_status[tag.min(2)];
        for_each_message(data, &mut rs, |m| match tag {
            TAG_PADS => self.pad_msg(m),
            _ => self.key_msg(m),
        });
        self.running_status[tag.min(2)] = rs;
    }

    fn end_of_list(&mut self) {
        self.out.flush();
        if self.signal {
            self.signal = false;
            self.shared.wake.signal();
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
    pub snaps: Producer<Snapshot>,
    pub out: Out,
}

pub fn run_engine(mut engine: Engine, mut io: EngineIo, shared: Arc<Shared>) {
    let rt_ok = std::env::var("YAHAHA_NO_RT").is_err() && rt::make_realtime(1_000_000, 300_000, 1_000_000);
    shared.engine_rt.store(rt_ok, Relaxed);
    let mut last_packed = 0u32;
    let mut last_snap: Option<Snapshot> = None;
    let mut last_snap_ns = 0u64;
    engine.send_init(&mut io.out);
    io.out.flush();
    loop {
        if shared.quit.load(Relaxed) {
            engine.stop(&mut io.out);
            io.out.flush();
            return;
        }
        let spin = shared.spin_ns.load(Relaxed);
        let now = rt::now_ns();
        let deadline = engine.next_deadline();
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
                if shared.chord.load(Relaxed) != last_packed || io.input.slots() > 0 || io.ui.slots() > 0 {
                    break;
                }
                std::hint::spin_loop();
                t = rt::now_ns();
            }
            if t >= d {
                shared.lateness.record(t - d);
            }
        }
        let now = rt::now_ns();

        while let Ok(style) = io.styles.pop() {
            let old = engine.load(style, now, &mut io.out);
            let _ = io.old.push(old);
        }
        let packed = shared.chord.load(Acquire);
        if packed != last_packed {
            last_packed = packed;
            if let Some((c, _)) = Chord::unpack(packed) {
                shared.chord_lat.record(now.saturating_sub(shared.chord_ns.load(Relaxed)));
                engine.set_chord(c, now, &mut io.out);
            }
        }
        // Read the fingering type and area here rather than taking a command for them, so
        // a full ring can never leave Sync Stop on in a Full Keyboard type.
        engine.allow_sync_stop(shared.sync_stop_allowed());
        while let Ok(cmd) = io.input.pop() {
            apply(&mut engine, cmd, now, &mut io.out);
        }
        while let Ok(cmd) = io.ui.pop() {
            apply(&mut engine, cmd, now, &mut io.out);
        }
        engine.process(now, &mut io.out);
        let t1 = rt::now_ns();
        io.out.flush();
        let t2 = rt::now_ns();
        shared.work_lat.record(t1 - now);
        shared.flush_lat.record(t2 - t1);

        let snap = engine.snapshot(now);
        if last_snap != Some(snap) || now - last_snap_ns > 50_000_000 {
            if io.snaps.push(snap).is_ok() {
                last_snap = Some(snap);
                last_snap_ns = now;
            }
        }
    }
}

fn apply(engine: &mut Engine, cmd: Cmd, now: u64, out: &mut Out) {
    match cmd {
        Cmd::Button(b) => engine.button(b, now, out),
        Cmd::ChordReleased => engine.chord_released(now, out),
        Cmd::Arm => engine.arm(out),
        Cmd::PartVolume(p, v) => engine.set_gain(p, v, out),
        Cmd::ManualBass(on) => engine.set_manual_bass(on, out),
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
    pub io: EngineIo,
}

pub fn channels(out: Out) -> Channels {
    let (input_tx, input) = RingBuffer::new(256);
    let (ui_tx, ui) = RingBuffer::new(256);
    let (style_tx, styles) = RingBuffer::new(4);
    let (old, old_rx) = RingBuffer::new(8);
    let (snaps, snap_rx) = RingBuffer::new(256);
    Channels { input_tx, ui_tx, style_tx, old_rx, snap_rx, io: EngineIo { input, ui, styles, old, snaps, out } }
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
    /// to CoreMIDI). Split at F#2 (54).
    fn rig(upper: bool) -> Rig {
        let shared = Arc::new(Shared::new(54));
        shared.upper.store(upper, Relaxed);
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

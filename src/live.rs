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
use crate::launchkey;
use crate::midi::{for_each_message, InputHandler};
use crate::rt::{self, Histogram, PacketSink, Wakeup};
use crate::theory::{Chord, Recognizer};
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, Ordering::*};
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub enum Cmd {
    Button(Button),
    ChordReleased,
    PartVolume(u8, u8),
    Arm,
    Panic,
}

pub struct Shared {
    pub chord: AtomicU32,
    pub chord_ns: AtomicU64,
    pub wake: Wakeup,
    pub quit: AtomicBool,
    pub split: AtomicU8,
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

// ---------------------------------------------------------------------------
// Input (CoreMIDI receive thread)
// ---------------------------------------------------------------------------

pub struct Input {
    shared: Arc<Shared>,
    rec: Recognizer,
    held: [bool; 128],
    zone_held: u32,
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
            held: [false; 128],
            zone_held: 0,
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

    fn recompute(&mut self) {
        let mut mask = 0u16;
        let mut low = None;
        for k in 0..128 {
            if self.held[k] {
                mask |= 1 << (k % 12);
                low.get_or_insert(k as u8);
            }
        }
        let Some(low) = low else { return };
        if let Some(c) = self.rec.recognize(mask, low % 12) {
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
                if k <= split {
                    if !self.held[k as usize] {
                        self.held[k as usize] = true;
                        self.zone_held += 1;
                    }
                    self.out.push(&[0x90 | LH_CH, k, m[2]]);
                    self.recompute();
                } else {
                    self.out.push(&[0x90 | RH_CH, k, m[2]]);
                }
            }
            (0x80, 3) | (0x90, 3) => {
                let k = m[1];
                if self.held[k as usize] {
                    self.held[k as usize] = false;
                    self.zone_held = self.zone_held.saturating_sub(1);
                    self.out.push(&[0x80 | LH_CH, k, 0]);
                    if self.zone_held == 0 && self.cmd.push(Cmd::ChordReleased).is_ok() {
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

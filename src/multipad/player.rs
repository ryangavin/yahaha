//! Multi Pad player core: four pads, Repeat, Chord Match and Synchro Start, with no notion
//! of threads or wall time.
//!
//! Time is in *host ticks*: any monotonic tick count the caller picks, at `host_ppq` ticks
//! per quarter note (the engine can use its own tick clock). Pads are rescaled to it when
//! the player is built. [`MultiPadPlayer::process`] emits every action due before the end of
//! the range it is given, earliest first across pads, and never allocates or panics.
//!
//! Behaviour (docs/genos-features.md §7, RM p.64–65):
//!
//! * A pad plays its phrase once, or loops with Repeat on. Pressing a playing pad restarts
//!   it from the top. Up to four pads play at once.
//! * While a Style (or Song) plays, a press starts at the top of the next measure; when
//!   everything is stopped it starts at once. [`start_tick`] answers "when"; the caller
//!   passes the answer to [`MultiPadPlayer::trigger`].
//! * Chord Match transposes a pad's notes to the current chord with the pad's CASM rule, or
//!   with [`default_rule`] (a CM7 source, Melody table, Root Trans) when it has none.
//!   Without a chord (or under Chord Cancel) the phrase plays as written. A note keeps its
//!   pitch until it ends; notes that start after a chord change follow the new chord.
//! * Synchro Start is data: [`MultiPadPlayer::arm`] toggles a pad's standby,
//!   [`sync_fires`] says whether an event releases it, and [`MultiPadPlayer::fire_sync`]
//!   starts every armed pad.

use super::file::{PadBank, PADS};
use crate::engine::Sink;
use crate::sff::{ChannelRule, Ev, Ntr, Ntt, Rtr, Zone};
use crate::theory::{transpose_group, Chord, NUM_TYPES};
use std::ops::Range;

/// Output channels (0-based) of pads 1..4: MIDI channels 5..8, where the Genos puts the
/// Multi Pad parts (RM p.76).
pub const DEFAULT_OUT_CH: [u8; PADS] = [4, 5, 6, 7];

/// Notes the player can hold at once, across all pads. A note-on past this is dropped.
const MAX_SOUNDING: usize = 64;
/// Largest group of simultaneous note-ons transposed together (as `transpose_group`).
const MAX_GROUP: usize = 6;

/// The rhythm clock a pad press is quantised to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clock {
    /// No Style or Song is playing: pads start at once.
    Stopped,
    /// A Style or Song is playing; bars start at `bar_origin + n * ticks_per_bar`.
    Running { bar_origin: u64, ticks_per_bar: u64 },
}

/// When a pad pressed at `now` starts: at once when stopped, otherwise at the top of the
/// next measure (at once if `now` is exactly on a barline).
pub fn start_tick(now: u64, clock: Clock) -> u64 {
    match clock {
        Clock::Stopped => now,
        Clock::Running { bar_origin, ticks_per_bar } => {
            if now <= bar_origin || ticks_per_bar == 0 {
                return bar_origin.max(now);
            }
            let into = (now - bar_origin) % ticks_per_bar;
            if into == 0 {
                now
            } else {
                now.saturating_add(ticks_per_bar - into)
            }
        }
    }
}

/// What can release pads in Synchro Start standby.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncTrigger {
    /// Any key played.
    Key,
    /// A chord played in the chord section.
    Chord,
    /// The Style starts.
    StyleStart,
}

/// Does `trigger` release armed pads? A key press counts with ACMP off, a chord in the
/// chord section with ACMP on, and a Style start always (§7 Synchro Start).
pub fn sync_fires(trigger: SyncTrigger, acmp_on: bool) -> bool {
    match trigger {
        SyncTrigger::Key => !acmp_on,
        SyncTrigger::Chord => acmp_on,
        SyncTrigger::StyleStart => true,
    }
}

/// The Chord Match rule for a pad with no CASM data: phrases are recorded over CM7 with
/// C, E, G, A, B (RM p.65), like a Style's melodic source pattern, so they follow the
/// chord root and the Melody table. `dest_ch` must not be a rhythm channel (9/10).
pub fn default_rule(dest_ch: u8) -> ChannelRule {
    let z = Zone { ntr: Ntr::RootTrans, ntt: Ntt::Melody, high_key: 6, lo: 0, hi: 127, rtr: Rtr::PitchShift, bass_on: false };
    ChannelRule {
        src_ch: dest_ch,
        name: String::new(),
        dest_ch,
        editable: true,
        note_mute: 0x0FFF,
        chord_mute: (1u64 << NUM_TYPES) - 1,
        autostart: false,
        src_root: 0,
        src_type: 2,
        mid_lo: 0,
        mid_hi: 127,
        zones: [z; 3],
        sff2: true,
    }
}

/// A pad's lamp (§7 Lamps).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadState {
    /// No data: lamp off.
    Empty,
    /// Has data, idle: blue.
    Ready,
    /// Synchro Start standby: red, flashing.
    Armed,
    /// Pressed, waiting for its start tick (the next measure).
    Queued,
    /// Playing: red.
    Playing,
}

#[derive(Debug, Clone, Copy)]
enum Kind {
    On { key: u8, vel: u8 },
    Off { key: u8 },
    /// Any other channel message; the status's channel is replaced on output.
    Short { msg: [u8; 3], len: u8 },
    /// Index into `MultiPadPlayer::sysex`.
    Sysex(usize),
}

#[derive(Debug, Clone, Copy)]
struct PEv {
    tick: u64,
    kind: Kind,
}

#[derive(Debug, Clone)]
struct Prepared {
    events: Vec<PEv>,
    /// Pass length in host ticks, at least 1.
    len: u64,
    repeat: bool,
    chord_match: bool,
    rule: ChannelRule,
    out_ch: u8,
    /// The pad's voice is a drum or SFX kit (its bank MSB, the last before its first note,
    /// is 126/127): Master transpose leaves it alone, as it does the style's kit parts.
    kit: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct Pass {
    start: u64,
    idx: usize,
}

#[derive(Debug, Clone, Copy, Default)]
struct Voice {
    pass: Option<Pass>,
    /// A press waiting for its start tick (restarts the pad if it is playing).
    pending: Option<u64>,
    armed: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct Sounding {
    active: bool,
    pad: u8,
    /// The channel the note-on went out on: its off goes there even if the pad has been
    /// rerouted since.
    ch: u8,
    src_key: u8,
    out_key: u8,
}

/// What a pad does next, in the order same-tick actions of different pads run: offs and
/// controllers first, then pass ends and starts, then note-ons (so one pad's note ending
/// never cuts another pad's note starting on the same tick and key).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Step {
    /// The pass's next event, when it is not a note-on.
    Event,
    /// The pass ends: loop (Repeat) or finish.
    End,
    /// A queued press starts (or restarts) the pad.
    Start,
    /// The pass's next event, a note-on.
    NoteOn,
}

pub struct MultiPadPlayer {
    pads: [Option<Prepared>; PADS],
    voices: [Voice; PADS],
    sounding: [Sounding; MAX_SOUNDING],
    sysex: Vec<Vec<u8>>,
    /// Per pad, the output channels (bit n: channel n) it has bent, modulated or held the
    /// sustain pedal on since its controllers were last reset.
    touched: [u16; PADS],
    /// Master transpose in semitones, applied to every note-on but a kit pad's.
    master: i8,
    /// The chord is settling (engine/settle.rs): Chord Match pads' next note-ons wait
    /// (`set_hold`).
    hold: bool,
}

impl MultiPadPlayer {
    /// Build a player for `bank`, on [`DEFAULT_OUT_CH`], at `host_ppq` ticks per quarter.
    /// Repeat and Chord Match start from the file (or their defaults, see `file`).
    pub fn new(bank: &PadBank, host_ppq: u32) -> MultiPadPlayer {
        let host_ppq = host_ppq.max(1) as u64;
        let file_ppq = bank.ppq.max(1) as u64;
        let scale = |t: u32| (t as u64 * host_ppq + file_ppq / 2) / file_ppq;
        let mut sysex = Vec::new();
        let mut pads: [Option<Prepared>; PADS] = Default::default();
        for (i, pad) in bank.pads.iter().enumerate() {
            let Some(pad) = pad else { continue };
            let out_ch = DEFAULT_OUT_CH[i];
            let mut events = Vec::with_capacity(pad.events.len());
            let mut last = 0;
            for e in &pad.events {
                let tick = scale(e.tick);
                last = last.max(tick);
                let kind = match &e.ev {
                    Ev::NoteOn { key, vel, .. } => Kind::On { key: *key, vel: *vel },
                    Ev::NoteOff { key, .. } => Kind::Off { key: *key },
                    Ev::PolyAt { key, val, .. } => Kind::Short { msg: [0xA0, *key, *val], len: 3 },
                    Ev::Cc { cc, val, .. } => Kind::Short { msg: [0xB0, *cc, *val], len: 3 },
                    Ev::Pc { prog, .. } => Kind::Short { msg: [0xC0, *prog, 0], len: 2 },
                    Ev::ChanAt { val, .. } => Kind::Short { msg: [0xD0, *val, 0], len: 2 },
                    Ev::Bend { val, .. } => {
                        Kind::Short { msg: [0xE0, (*val & 0x7F) as u8, ((*val >> 7) & 0x7F) as u8], len: 3 }
                    }
                    Ev::Sysex(d) => {
                        sysex.push(d.clone());
                        Kind::Sysex(sysex.len() - 1)
                    }
                    Ev::Meta { .. } => continue,
                };
                events.push(PEv { tick, kind });
            }
            // Stable: same-tick events keep file order.
            events.sort_by_key(|e| e.tick);
            let len = scale(pad.len).max(last).max(1);
            // The bank MSB the first note sounds with (else the first one set at all).
            let bank = |before_note: bool| {
                let mut msb = None;
                for e in &events {
                    match e.kind {
                        Kind::On { .. } if before_note => break,
                        Kind::Short { msg: [0xB0, 0, v], .. } => {
                            msb = Some(v);
                            if !before_note {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                msb
            };
            let kit = bank(true).or_else(|| bank(false)).is_some_and(|m| m >= 126);
            let mut rule = pad.rule.clone().unwrap_or_else(|| default_rule(out_ch));
            // The rule's destination decides drum handling in `transpose`: a pad is never
            // a rhythm part, so it takes the pad's own output channel.
            rule.dest_ch = out_ch;
            pads[i] = Some(Prepared {
                events,
                len,
                repeat: pad.repeat_or_default(),
                chord_match: pad.chord_match_or_default(),
                rule,
                out_ch,
                kit,
            });
        }
        MultiPadPlayer {
            pads,
            voices: [Voice::default(); PADS],
            sounding: [Sounding::default(); MAX_SOUNDING],
            sysex,
            touched: [0; PADS],
            master: 0,
            hold: false,
        }
    }

    /// Master transpose (semitones), from the next note-on. Notes already sounding end on
    /// the key they started on.
    pub fn set_master(&mut self, semitones: i8) {
        self.master = semitones;
    }

    /// The chord Chord Match follows is settling (engine/settle.rs): while `on`, a Chord
    /// Match pad's next note-on, and everything after it on that pad, waits, so it starts
    /// with the settled chord, not the one it replaces. Pads without Chord Match play on
    /// time. Once off, what waited plays at the next `process` (late, by the wait).
    pub fn set_hold(&mut self, on: bool) {
        self.hold = on;
    }

    /// The pad's voice is a drum or SFX kit (Master transpose leaves it alone).
    pub fn is_kit(&self, pad: usize) -> bool {
        self.prepared(pad).is_some_and(|p| p.kit)
    }

    fn prepared(&self, pad: usize) -> Option<&Prepared> {
        self.pads.get(pad).and_then(|p| p.as_ref())
    }

    pub fn has_data(&self, pad: usize) -> bool {
        self.prepared(pad).is_some()
    }

    /// Pass length in host ticks (0 for an empty pad).
    pub fn len_ticks(&self, pad: usize) -> u64 {
        self.prepared(pad).map_or(0, |p| p.len)
    }

    pub fn repeat(&self, pad: usize) -> bool {
        self.prepared(pad).is_some_and(|p| p.repeat)
    }

    pub fn chord_match(&self, pad: usize) -> bool {
        self.prepared(pad).is_some_and(|p| p.chord_match)
    }

    /// Panel override of the pad's Repeat flag; takes effect at the end of the current pass.
    pub fn set_repeat(&mut self, pad: usize, on: bool) {
        if let Some(p) = self.pads.get_mut(pad).and_then(|p| p.as_mut()) {
            p.repeat = on;
        }
    }

    /// Panel override of the pad's Chord Match flag; takes effect from the next note.
    pub fn set_chord_match(&mut self, pad: usize, on: bool) {
        if let Some(p) = self.pads.get_mut(pad).and_then(|p| p.as_mut()) {
            p.chord_match = on;
        }
    }

    /// Route a pad to another output channel (0-based). Safe while the pad plays: notes
    /// already sounding end on the channel they started on.
    pub fn set_out_channel(&mut self, pad: usize, ch: u8) {
        if let Some(p) = self.pads.get_mut(pad).and_then(|p| p.as_mut()) {
            p.out_ch = ch & 0x0F;
            p.rule.dest_ch = ch & 0x0F;
        }
    }

    pub fn out_channel(&self, pad: usize) -> Option<u8> {
        self.prepared(pad).map(|p| p.out_ch)
    }

    pub fn state(&self, pad: usize) -> PadState {
        let Some(v) = self.voices.get(pad) else { return PadState::Empty };
        if !self.has_data(pad) {
            PadState::Empty
        } else if v.pass.is_some() {
            PadState::Playing
        } else if v.pending.is_some() {
            PadState::Queued
        } else if v.armed {
            PadState::Armed
        } else {
            PadState::Ready
        }
    }

    /// Any pad playing or queued.
    pub fn is_active(&self) -> bool {
        self.voices.iter().any(|v| v.pass.is_some() || v.pending.is_some())
    }

    /// Press a pad: it starts (or restarts from the top) at `start`, which the caller takes
    /// from [`start_tick`]. A press also takes the pad out of Synchro Start standby.
    /// Returns false for an empty pad.
    pub fn trigger(&mut self, pad: usize, start: u64) -> bool {
        if !self.has_data(pad) {
            return false;
        }
        let v = &mut self.voices[pad];
        v.pending = Some(start);
        v.armed = false;
        true
    }

    /// Stop one pad now (STOP + pad): its notes end, its queue and standby clear.
    pub fn stop(&mut self, pad: usize, sink: &mut impl Sink) {
        if pad >= PADS {
            return;
        }
        self.voices[pad] = Voice::default();
        self.notes_off(pad, sink);
        self.reset_controllers(pad, sink);
    }

    /// [STOP]: every pad stops, and Synchro Start standby is cancelled.
    pub fn stop_all(&mut self, sink: &mut impl Sink) {
        for pad in 0..PADS {
            self.stop(pad, sink);
        }
    }

    /// Stop the pads that repeat (playing or queued), for Multi Pad Synchro Stop when the
    /// Style stops or an Ending starts, whichever the caller's settings say. One-shot pads
    /// play out.
    pub fn stop_repeating(&mut self, sink: &mut impl Sink) {
        for pad in 0..PADS {
            if self.repeat(pad) && (self.voices[pad].pass.is_some() || self.voices[pad].pending.is_some()) {
                self.stop(pad, sink);
            }
        }
    }

    /// Toggle a pad's Synchro Start standby (SELECT + pad). Returns the new state; an empty
    /// pad cannot be armed.
    pub fn arm(&mut self, pad: usize) -> bool {
        if !self.has_data(pad) {
            return false;
        }
        let v = &mut self.voices[pad];
        v.armed = !v.armed;
        v.armed
    }

    pub fn armed(&self, pad: usize) -> bool {
        self.voices.get(pad).is_some_and(|v| v.armed)
    }

    pub fn any_armed(&self) -> bool {
        self.voices.iter().any(|v| v.armed)
    }

    pub fn disarm_all(&mut self) {
        for v in &mut self.voices {
            v.armed = false;
        }
    }

    /// A Synchro Start trigger (see [`sync_fires`]) arrived: every armed pad starts at
    /// `start` (from [`start_tick`]). Returns how many pads it started.
    pub fn fire_sync(&mut self, start: u64) -> usize {
        let mut n = 0;
        for pad in 0..PADS {
            if self.voices[pad].armed {
                self.trigger(pad, start);
                n += 1;
            }
        }
        n
    }

    /// Play everything due before `range.end`, earliest first across pads. Anything due
    /// before `range.start` (a late call) plays at once, except whole Repeat passes that
    /// ended before it, which are skipped. `chord` is the chord Chord Match follows (None:
    /// no chord yet, or ACMP and LEFT off).
    pub fn process(&mut self, range: Range<u64>, chord: Option<Chord>, sink: &mut impl Sink) {
        // Each step consumes an event, ends a pass or starts one, so this terminates; the
        // bound only guards against a logic error spinning the real-time thread.
        for _ in 0..100_000 {
            let Some((pad, at, step)) = self.next_step() else { return };
            if at >= range.end {
                return;
            }
            match step {
                Step::Event | Step::NoteOn => self.emit(pad, chord, sink),
                Step::End => {
                    let Some(p) = self.pads[pad].as_ref() else { return };
                    let (len, repeat) = (p.len, p.repeat);
                    let v = &mut self.voices[pad];
                    if repeat {
                        let mut start = at;
                        // Skip whole passes that ended before the range.
                        if start.saturating_add(len) <= range.start {
                            start += (range.start - start) / len * len;
                        }
                        v.pass = Some(Pass { start, idx: 0 });
                    } else {
                        v.pass = None;
                    }
                    // Notes the phrase leaves hanging end with the pass; a one-shot that
                    // ends bent or with the pedal down leaves its channel as it found it.
                    self.notes_off(pad, sink);
                    if !repeat {
                        self.reset_controllers(pad, sink);
                    }
                }
                Step::Start => {
                    let v = &mut self.voices[pad];
                    v.pending = None;
                    v.pass = Some(Pass { start: at, idx: 0 });
                    self.notes_off(pad, sink);
                    self.reset_controllers(pad, sink);
                }
            }
        }
    }

    /// The tick of the next thing any pad does (an event, a pass end, a queued start), for
    /// the caller's wake-up deadline. None when nothing plays or waits.
    pub fn next_due(&self) -> Option<u64> {
        self.next_step().map(|(_, at, _)| at)
    }

    /// The earliest action of any pad: (pad, tick, step).
    fn next_step(&self) -> Option<(usize, u64, Step)> {
        let mut best: Option<(u64, Step, usize)> = None;
        for pad in 0..PADS {
            let Some(p) = self.pads[pad].as_ref() else { continue };
            let v = &self.voices[pad];
            let mut consider = |at: u64, step: Step| {
                if best.is_none_or(|b| (at, step) < (b.0, b.1)) {
                    best = Some((at, step, pad));
                }
            };
            if let Some(pass) = v.pass {
                match p.events.get(pass.idx) {
                    Some(e) => {
                        let on = matches!(e.kind, Kind::On { .. });
                        // A Chord Match note-on waits for the chord to settle (`set_hold`).
                        if !(on && self.hold && p.chord_match) {
                            consider(pass.start.saturating_add(e.tick), if on { Step::NoteOn } else { Step::Event });
                        }
                    }
                    None => consider(pass.start.saturating_add(p.len), Step::End),
                }
            }
            if let Some(at) = v.pending {
                consider(at, Step::Start);
            }
        }
        best.map(|(at, step, pad)| (pad, at, step))
    }

    /// Emit the next event of `pad`'s pass (a run of same-tick note-ons goes as a group).
    fn emit(&mut self, pad: usize, chord: Option<Chord>, sink: &mut impl Sink) {
        let (Some(p), Some(pass)) = (self.pads[pad].as_ref(), self.voices[pad].pass) else { return };
        let Some(e) = p.events.get(pass.idx) else { return };
        let ch = p.out_ch;
        let mut next = pass.idx + 1;
        match e.kind {
            Kind::On { .. } => {
                let mut keys = [0u8; MAX_GROUP];
                let mut vels = [0u8; MAX_GROUP];
                let mut n = 0;
                for g in &p.events[pass.idx..] {
                    match g.kind {
                        Kind::On { key, vel } if g.tick == e.tick && n < MAX_GROUP => {
                            keys[n] = key;
                            vels[n] = vel;
                            n += 1;
                        }
                        _ => break,
                    }
                }
                next = pass.idx + n;
                let mut out = [None; MAX_GROUP];
                let follow = chord.map(Chord::casm).filter(|c| p.chord_match && (c.ty as usize) < NUM_TYPES);
                match follow {
                    Some(c) => transpose_group(&keys[..n], &p.rule, c, &mut out[..n]),
                    None => {
                        for i in 0..n {
                            out[i] = Some(keys[i]);
                        }
                    }
                }
                let master = if p.kit { 0 } else { self.master };
                for i in 0..n {
                    // A note the rule silences (none should, but a CASM rule may) stays off.
                    if let Some(k) = out[i] {
                        // Master transpose moves the whole instrument (RM p.41), after Chord
                        // Match; the note's off goes to the key it sounds on.
                        let k = crate::engine::shift_key(k, master);
                        self.note_on(pad, ch, keys[i], k, vels[i], sink);
                    }
                }
            }
            Kind::Off { key } => self.note_off(pad, key, sink),
            Kind::Short { msg, len } => {
                let m = [(msg[0] & 0xF0) | ch, msg[1], msg[2]];
                let moved = match msg {
                    [0xE0, lsb, msb] => (lsb, msb) != (0x00, 0x40),
                    [0xB0, 1 | 64, v] => v != 0,
                    _ => false,
                };
                if moved {
                    self.touched[pad] |= 1 << ch;
                }
                sink.send(&m[..len as usize]);
            }
            Kind::Sysex(i) => {
                if let Some(d) = self.sysex.get(i) {
                    sink.send(d);
                }
            }
        }
        if let Some(pass) = self.voices[pad].pass.as_mut() {
            pass.idx = next;
        }
    }

    fn note_on(&mut self, pad: usize, ch: u8, src_key: u8, out_key: u8, vel: u8, sink: &mut impl Sink) {
        // The same pitch again on this pad: end the old one so ons and offs stay balanced.
        for s in self.sounding.iter_mut() {
            if s.active && s.pad as usize == pad && s.out_key == out_key {
                sink.send(&[0x80 | s.ch, out_key, 0]);
                s.active = false;
            }
        }
        if let Some(free) = self.sounding.iter_mut().find(|s| !s.active) {
            *free = Sounding { active: true, pad: pad as u8, ch, src_key, out_key };
            sink.send(&[0x90 | ch, out_key, vel]);
        }
    }

    fn note_off(&mut self, pad: usize, src_key: u8, sink: &mut impl Sink) {
        if let Some(s) = self.sounding.iter_mut().find(|s| s.active && s.pad as usize == pad && s.src_key == src_key) {
            sink.send(&[0x80 | s.ch, s.out_key, 0]);
            s.active = false;
        }
    }

    /// Re-centre the bend and release the modulation wheel and sustain pedal on every
    /// channel `pad` moved them on, as the style parts do when they stop
    /// (`engine::playback::notes_off`): a pad stopped mid-bend or with its pedal down must
    /// not leave its channel bent or sustained for the next pad or bank.
    fn reset_controllers(&mut self, pad: usize, sink: &mut impl Sink) {
        let mask = std::mem::take(&mut self.touched[pad]);
        for ch in 0..16u8 {
            if mask & (1 << ch) != 0 {
                sink.send(&[0xE0 | ch, 0x00, 0x40]);
                sink.send(&[0xB0 | ch, 1, 0]);
                sink.send(&[0xB0 | ch, 64, 0]);
            }
        }
    }

    /// Pads pressed while the band played wait for a bar line of that band. When it stops
    /// (or restarts), those presses start at `start` instead.
    pub fn retime_pending(&mut self, start: u64) {
        for v in &mut self.voices {
            if v.pending.is_some() {
                v.pending = Some(start);
            }
        }
    }

    fn notes_off(&mut self, pad: usize, sink: &mut impl Sink) {
        for s in self.sounding.iter_mut() {
            if s.active && s.pad as usize == pad {
                sink.send(&[0x80 | s.ch, s.out_key, 0]);
                s.active = false;
            }
        }
    }
}

#[cfg(test)]
#[path = "player_tests.rs"]
mod tests;

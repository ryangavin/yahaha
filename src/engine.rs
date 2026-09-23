//! The arranger engine: section state machine, pattern playback, chord following.
//!
//! The engine is deterministic and has no notion of threads or wall time: callers pass
//! the current time in nanoseconds. It never allocates after construction, so the
//! real-time output thread can drive it directly.

use crate::sff::{ChannelRule, Ev, Rtr, SectionId, Style};
use crate::theory::{is_drum_part, plays, transpose_group, Chord, CANCEL};

pub trait Sink {
    fn send(&mut self, msg: &[u8]);
}

// ---------------------------------------------------------------------------
// Prepared (playback-ready) style
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
enum PKind {
    On { key: u8, vel: u8 },
    Off { key: u8 },
    Cc { cc: u8, val: u8 },
    Pc { prog: u8 },
    Bend { lo: u8, hi: u8 },
}

#[derive(Clone, Copy, Debug)]
struct PEvent {
    tick: u32,
    src: u8,
    kind: PKind,
}

pub struct PSection {
    pub id: SectionId,
    pub len: u32,
    events: Vec<PEvent>,
    rules: [Option<ChannelRule>; 16],
}

impl PSection {
    pub fn first_note_tick(&self) -> Option<u32> {
        self.events.iter().find(|e| matches!(e.kind, PKind::On { .. })).map(|e| e.tick)
    }
}

pub const NUM_SLOTS: usize = 17;

pub fn slot_of(id: SectionId) -> usize {
    match id {
        SectionId::Intro(i) => i as usize,
        SectionId::Main(i) => 4 + i as usize,
        SectionId::Fill(i) => 8 + i as usize,
        SectionId::Break => 12,
        SectionId::Ending(i) => 13 + i as usize,
    }
}

pub fn id_of(slot: usize) -> SectionId {
    match slot {
        0..=3 => SectionId::Intro(slot as u8),
        4..=7 => SectionId::Main(slot as u8 - 4),
        8..=11 => SectionId::Fill(slot as u8 - 8),
        12 => SectionId::Break,
        _ => SectionId::Ending(slot as u8 - 13),
    }
}

pub struct Prepared {
    pub name: String,
    pub ppq: u32,
    pub bpm: f64,
    pub tpb: u32,
    pub sections: Vec<Option<PSection>>,
    /// Channel setup messages (already remapped to destination channels).
    pub init: Vec<[u8; 3]>,
    pub init_len: Vec<u8>,
    /// Voice (bank MSB, LSB, program) per destination channel 8..16, for display.
    pub voices: [Option<(u8, u8, u8)>; 16],
}

impl Prepared {
    pub fn new(style: &Style) -> Prepared {
        let mut sections: Vec<Option<PSection>> = (0..NUM_SLOTS).map(|_| None).collect();
        let casm = style.has_casm();
        for (id, sec) in &style.sections {
            let mut rules: [Option<ChannelRule>; 16] = Default::default();
            let from_casm = style.rules_for(*id);
            for ch in 0..16u8 {
                rules[ch as usize] = match from_casm.get(&ch) {
                    Some(r) => Some(r.clone()),
                    None if (8..16).contains(&ch) && (!casm || !from_casm.values().any(|r| r.dest_ch == ch)) => {
                        Some(ChannelRule::default_for(ch))
                    }
                    None => None,
                };
            }
            let mut events: Vec<PEvent> = sec
                .events
                .iter()
                .filter_map(|e| {
                    let (src, kind) = match e.ev {
                        Ev::NoteOn { ch, key, vel } => (ch, PKind::On { key, vel }),
                        Ev::NoteOff { ch, key } => (ch, PKind::Off { key }),
                        Ev::Cc { ch, cc, val } => (ch, PKind::Cc { cc, val }),
                        Ev::Pc { ch, prog } => (ch, PKind::Pc { prog }),
                        Ev::Bend { ch, val } => (ch, PKind::Bend { lo: (val & 0x7F) as u8, hi: (val >> 7) as u8 }),
                        _ => return None,
                    };
                    Some(PEvent { tick: e.tick, src, kind })
                })
                .collect();
            // Stable: offs before ons at the same tick so repeated notes retrigger cleanly.
            events.sort_by_key(|e| (e.tick, !matches!(e.kind, PKind::Off { .. })));
            sections[slot_of(*id)] = Some(PSection { id: *id, len: sec.len, events, rules });
        }

        // Route init (SInt) messages through the Main A rules (or any section's).
        let route = sections
            .iter()
            .flatten()
            .find(|s| s.id == SectionId::Main(0))
            .or_else(|| sections.iter().flatten().next())
            .map(|s| s.rules.clone())
            .unwrap_or_default();
        let mut init = Vec::new();
        let mut init_len = Vec::new();
        let mut voices: [Option<(u8, u8, u8)>; 16] = [None; 16];
        let mut bank = [(0u8, 0u8); 16];
        for ev in &style.init {
            let Some(ch) = ev.channel() else { continue };
            let Some(rule) = &route[ch as usize] else { continue };
            let d = rule.dest_ch;
            let (msg, len) = match *ev {
                Ev::Cc { cc, val, .. } => {
                    if cc == 0 {
                        bank[d as usize].0 = val;
                    }
                    if cc == 32 {
                        bank[d as usize].1 = val;
                    }
                    ([0xB0 | d, cc, val], 3)
                }
                Ev::Pc { prog, .. } => {
                    let (m, l) = bank[d as usize];
                    voices[d as usize] = Some((m, l, prog));
                    ([0xC0 | d, prog, 0], 2)
                }
                Ev::Bend { val, .. } => ([0xE0 | d, (val & 0x7F) as u8, (val >> 7) as u8], 3),
                _ => continue,
            };
            init.push(msg);
            init_len.push(len);
        }
        Prepared {
            name: style.name.clone(),
            ppq: style.ppq as u32,
            bpm: style.bpm(),
            tpb: style.ticks_per_bar(),
            sections,
            init,
            init_len,
            voices,
        }
    }

    fn has(&self, slot: usize) -> bool {
        self.sections[slot].is_some()
    }

    /// Nearest existing section of the same kind (e.g. Main D -> Main C).
    fn resolve(&self, slot: usize) -> Option<usize> {
        let (base, n) = match slot {
            0..=3 => (0, 4),
            4..=7 => (4, 4),
            8..=11 => (8, 4),
            12 => (12, 1),
            _ => (13, 4),
        };
        let i = slot - base;
        (0..n).flat_map(|d| [i.checked_sub(d), Some(i + d)]).flatten().filter(|&j| j < n).map(|j| base + j).find(|&s| self.has(s))
    }
}

// ---------------------------------------------------------------------------
// Commands and state
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Intro(u8),
    Main(u8),
    Break,
    Ending(u8),
    StartStop,
    Stop,
    SyncStart,
    SyncStop,
    AutoFill,
    TapTempo,
    TempoUp,
    TempoDown,
    TogglePart(u8),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Snapshot {
    pub running: bool,
    pub sync_armed: bool,
    pub sync_stop: bool,
    pub auto_fill: bool,
    pub cur: Option<SectionId>,
    pub queued: Option<SectionId>,
    pub pending_intro: Option<u8>,
    pub main: u8,
    pub bar: u32,
    pub beat: u32,
    pub chord: Option<Chord>,
    pub bpm: f64,
    pub parts: u8,
}

#[derive(Clone, Copy)]
struct Sounding {
    active: bool,
    src: u8,
    src_key: u8,
    dest: u8,
    out: u8,
    vel: u8,
    slot: u8,
    started_ns: u64,
}

const EMPTY: Sounding = Sounding { active: false, src: 0, src_key: 0, dest: 0, out: 0, vel: 0, slot: 0, started_ns: 0 };
const MAX_SOUNDING: usize = 256;
/// Notes that started this recently when the chord changes are corrected outright:
/// the player's chord landed just after the beat.
const LATE_CHORD_NS: u64 = 40_000_000;

#[derive(Clone, Copy)]
struct Queued {
    slot: usize,
    at: f64,
    sec_start: f64,
}

pub struct Engine {
    pub style: Box<Prepared>,
    running: bool,
    sync_armed: bool,
    sync_stop: bool,
    auto_fill: bool,
    main: u8,
    pending_intro: Option<u8>,
    cur: usize,
    sec_start: f64,
    ev_idx: usize,
    queued: Option<Queued>,
    chord: Option<Chord>,
    bpm: f64,
    anchor_ns: u64,
    anchor_tick: f64,
    ns_per_tick: f64,
    parts: u8,
    taps: [u64; 4],
    tap_n: usize,
    sounding: [Sounding; MAX_SOUNDING],
}

impl Engine {
    pub fn new(style: Box<Prepared>) -> Engine {
        let bpm = style.bpm;
        let mut e = Engine {
            style,
            running: false,
            sync_armed: true,
            sync_stop: false,
            auto_fill: true,
            main: 0,
            pending_intro: None,
            cur: 4,
            sec_start: 0.0,
            ev_idx: 0,
            queued: None,
            chord: None,
            bpm,
            anchor_ns: 0,
            anchor_tick: 0.0,
            ns_per_tick: 0.0,
            parts: 0xFF,
            taps: [0; 4],
            tap_n: 0,
            sounding: [EMPTY; MAX_SOUNDING],
        };
        e.set_bpm_internal(bpm, 0);
        e
    }

    /// Swap in a new style; returns the old one so the caller can free it off the RT thread.
    pub fn load(&mut self, style: Box<Prepared>, now: u64, sink: &mut impl Sink) -> Box<Prepared> {
        self.all_off(sink);
        let old = std::mem::replace(&mut self.style, style);
        self.queued = None;
        if !self.running {
            let bpm = self.style.bpm;
            self.set_bpm_internal(bpm, now);
        }
        self.send_init(sink);
        if self.running {
            // Continue from the next bar of the equivalent section.
            let slot = self.style.resolve(slot_of(SectionId::Main(self.main))).unwrap_or(4);
            let t = self.tick_at(now);
            self.cur = slot;
            self.sec_start = t;
            self.seek(0.0);
        }
        old
    }

    pub fn send_init(&self, sink: &mut impl Sink) {
        for (m, &l) in self.style.init.iter().zip(&self.style.init_len) {
            sink.send(&m[..l as usize]);
        }
    }

    // ----- time -----

    fn set_bpm_internal(&mut self, bpm: f64, now: u64) {
        let t = if self.ns_per_tick > 0.0 { self.tick_at(now) } else { 0.0 };
        self.bpm = bpm.clamp(30.0, 300.0);
        self.anchor_ns = now;
        self.anchor_tick = t;
        self.ns_per_tick = 60e9 / (self.bpm * self.style.ppq as f64);
    }

    fn tick_at(&self, now: u64) -> f64 {
        self.anchor_tick + (now as f64 - self.anchor_ns as f64) / self.ns_per_tick
    }

    fn ns_at(&self, tick: f64) -> u64 {
        let v = self.anchor_ns as f64 + (tick - self.anchor_tick) * self.ns_per_tick;
        if v < 0.0 {
            0
        } else {
            v.ceil() as u64
        }
    }

    // ----- queries -----

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn snapshot(&self, now: u64) -> Snapshot {
        let (bar, beat) = if self.running {
            let pos = (self.tick_at(now) - self.sec_start).max(0.0);
            let tpb = self.style.tpb as f64;
            ((pos / tpb) as u32, ((pos % tpb) / self.style.ppq as f64) as u32)
        } else {
            (0, 0)
        };
        Snapshot {
            running: self.running,
            sync_armed: self.sync_armed,
            sync_stop: self.sync_stop,
            auto_fill: self.auto_fill,
            cur: if self.running { Some(id_of(self.cur)) } else { None },
            queued: self.queued.map(|q| id_of(q.slot)),
            pending_intro: self.pending_intro,
            main: self.main,
            bar,
            beat,
            chord: self.chord,
            bpm: self.bpm,
            parts: self.parts,
        }
    }

    /// Time of the next thing the engine needs to do, if running.
    pub fn next_deadline(&self) -> Option<u64> {
        if !self.running {
            return None;
        }
        let sec = self.style.sections[self.cur].as_ref()?;
        let mut t = self.sec_start + sec.len as f64;
        if let Some(q) = self.queued {
            t = t.min(q.at);
        }
        if let Some(e) = sec.events.get(self.ev_idx) {
            t = t.min(self.sec_start + e.tick as f64);
        }
        Some(self.ns_at(t))
    }

    // ----- input -----

    /// Returns true if this chord should start playback (sync start).
    pub fn set_chord(&mut self, chord: Chord, now: u64, sink: &mut impl Sink) {
        let first = self.chord.is_none();
        self.chord = Some(chord);
        if self.sync_armed && !self.running && chord.ty != CANCEL {
            self.start(now, sink);
            return;
        }
        if self.running && !first {
            self.revoice(chord, now, sink);
        }
    }

    /// Chord-zone keys all released (for Sync Stop).
    pub fn chord_released(&mut self, now: u64, sink: &mut impl Sink) {
        if self.sync_stop && self.running {
            self.stop(sink);
            self.sync_armed = true;
        }
        let _ = now;
    }

    pub fn button(&mut self, b: Button, now: u64, sink: &mut impl Sink) {
        let s = &self.style;
        match b {
            Button::StartStop => {
                if self.running {
                    self.stop(sink);
                } else {
                    self.start(now, sink);
                }
            }
            Button::Stop => {
                if self.running {
                    self.stop(sink);
                }
            }
            Button::SyncStart => {
                if self.running {
                    self.stop(sink);
                    self.sync_armed = true;
                } else {
                    self.sync_armed = !self.sync_armed;
                }
            }
            Button::SyncStop => self.sync_stop = !self.sync_stop,
            Button::AutoFill => self.auto_fill = !self.auto_fill,
            Button::TogglePart(p) => {
                self.parts ^= 1 << (p & 7);
                if self.parts & (1 << (p & 7)) == 0 {
                    self.off_where(sink, |n| n.dest == 8 + (p & 7));
                }
            }
            Button::TempoUp => self.set_bpm_internal(self.bpm + 2.0, now),
            Button::TempoDown => self.set_bpm_internal(self.bpm - 2.0, now),
            Button::TapTempo => self.tap(now),
            Button::Intro(i) => {
                if !self.running {
                    self.pending_intro = if self.pending_intro == Some(i) { None } else { Some(i) };
                } else if let Some(slot) = s.resolve(i as usize) {
                    self.queue_at_bar(slot, now);
                }
            }
            Button::Main(i) => {
                let prev = self.main;
                self.main = i;
                if !self.running {
                    return;
                }
                let cur_id = id_of(self.cur);
                match cur_id {
                    SectionId::Main(m) => {
                        let fill = if i == m || self.auto_fill { s.resolve(8 + i as usize) } else { None };
                        let fill = if i == m { s.resolve(8 + m as usize) } else { fill };
                        match fill {
                            Some(f) => self.queue_fill(f, now),
                            None if i != m => {
                                if let Some(slot) = s.resolve(4 + i as usize) {
                                    self.queue_at_bar(slot, now)
                                }
                            }
                            None => {}
                        }
                    }
                    SectionId::Ending(_) => {
                        if let Some(slot) = s.resolve(4 + i as usize) {
                            self.queue_at_bar(slot, now)
                        }
                    }
                    // Intro / fill / break: they flow into self.main when done.
                    _ => {
                        let _ = prev;
                    }
                }
            }
            Button::Break => {
                if self.running {
                    if let Some(slot) = s.resolve(12) {
                        self.queue_fill(slot, now);
                    }
                }
            }
            Button::Ending(i) => {
                if self.running {
                    match s.resolve(13 + i as usize) {
                        Some(slot) if slot != self.cur => self.queue_at_bar(slot, now),
                        Some(_) => {}
                        None => self.queue_stop_at_bar(now),
                    }
                }
            }
        }
    }

    fn tap(&mut self, now: u64) {
        if self.tap_n > 0 && now.saturating_sub(self.taps[(self.tap_n - 1) % 4]) > 2_000_000_000 {
            self.tap_n = 0;
        }
        self.taps[self.tap_n % 4] = now;
        self.tap_n += 1;
        if self.tap_n >= 2 {
            let n = self.tap_n.min(4);
            let newest = self.taps[(self.tap_n - 1) % 4];
            let oldest = self.taps[(self.tap_n - n) % 4];
            let iv = (newest - oldest) as f64 / (n - 1) as f64;
            if iv > 0.0 {
                self.set_bpm_internal(60e9 / iv, now);
            }
        }
    }

    // ----- transport -----

    fn start(&mut self, now: u64, sink: &mut impl Sink) {
        self.running = true;
        self.sync_armed = false;
        self.set_bpm_internal(self.bpm, now);
        self.anchor_tick = 0.0;
        self.anchor_ns = now;
        let slot = self
            .pending_intro
            .take()
            .and_then(|i| self.style.resolve(i as usize))
            .or_else(|| self.style.resolve(4 + self.main as usize));
        let Some(slot) = slot else {
            self.running = false;
            return;
        };
        self.send_init(sink);
        self.cur = slot;
        self.sec_start = 0.0;
        self.ev_idx = 0;
        self.queued = None;
        self.process(now, sink);
    }

    /// Stop (if running) and wait for the next chord to start.
    pub fn arm(&mut self, sink: &mut impl Sink) {
        self.stop(sink);
        self.sync_armed = true;
    }

    pub fn stop(&mut self, sink: &mut impl Sink) {
        self.running = false;
        self.queued = None;
        self.all_off(sink);
    }

    fn next_bar(&self, now: u64) -> f64 {
        let t = self.tick_at(now);
        let tpb = self.style.tpb as f64;
        let pos = t - self.sec_start;
        self.sec_start + ((pos / tpb).floor() + 1.0) * tpb
    }

    fn queue_at_bar(&mut self, slot: usize, now: u64) {
        let at = self.next_bar(now);
        self.queued = Some(Queued { slot, at, sec_start: at });
    }

    fn queue_stop_at_bar(&mut self, now: u64) {
        let at = self.next_bar(now);
        self.queued = Some(Queued { slot: usize::MAX, at, sec_start: at });
    }

    /// Fills and breaks start at the next beat and play the rest of the bar, aligned so the
    /// fill's beat matches the bar position.
    fn queue_fill(&mut self, slot: usize, now: u64) {
        let t = self.tick_at(now);
        let ppq = self.style.ppq as f64;
        let tpb = self.style.tpb as f64;
        let pos = t - self.sec_start;
        let beat = (pos / ppq).ceil() * ppq;
        let at = self.sec_start + beat;
        let bar_start = self.sec_start + (beat / tpb).floor() * tpb;
        self.queued = Some(Queued { slot, at, sec_start: bar_start });
    }

    fn seek(&mut self, pos: f64) {
        let sec = self.style.sections[self.cur].as_ref().unwrap();
        self.ev_idx = sec.events.partition_point(|e| (e.tick as f64) < pos);
    }

    // ----- playback -----

    /// Emit everything due up to `now`.
    pub fn process(&mut self, now: u64, sink: &mut impl Sink) {
        if !self.running {
            return;
        }
        let target = self.tick_at(now) + 1e-6;
        loop {
            let Some(sec) = self.style.sections[self.cur].as_ref() else {
                self.stop(sink);
                return;
            };
            let sec_end = self.sec_start + sec.len as f64;
            let (boundary, inclusive) = match self.queued {
                Some(q) if q.at < sec_end => (q.at, false),
                _ => (sec_end, true),
            };
            if let Some(e) = sec.events.get(self.ev_idx) {
                let t = self.sec_start + e.tick as f64;
                let before = if inclusive { t <= boundary + 1e-6 } else { t < boundary - 1e-6 };
                if before && t <= target {
                    self.emit_at_index(now, sink);
                    continue;
                }
            }
            if boundary <= target {
                self.transition(boundary, now, sink);
                if !self.running {
                    return;
                }
                continue;
            }
            break;
        }
    }

    fn transition(&mut self, at: f64, now: u64, sink: &mut impl Sink) {
        self.all_off(sink);
        let sec_end = self.sec_start + self.style.sections[self.cur].as_ref().map_or(0, |s| s.len) as f64;
        let (next, start) = match self.queued.take() {
            Some(q) if q.at <= sec_end + 1e-6 => (q.slot, q.sec_start),
            q => {
                self.queued = q;
                let main_slot = self.style.resolve(4 + self.main as usize).unwrap_or(4);
                match id_of(self.cur) {
                    SectionId::Main(_) => (main_slot, at),
                    SectionId::Ending(_) => (usize::MAX, at),
                    _ => (main_slot, at),
                }
            }
        };
        if next == usize::MAX {
            self.stop(sink);
            self.sync_armed = true;
            return;
        }
        self.cur = next;
        self.sec_start = start;
        self.seek(at - start);
        let _ = now;
    }

    fn chord_for(&self, rule: &ChannelRule) -> Option<Chord> {
        match self.chord {
            Some(c) => Some(c),
            None if is_drum_part(rule.dest_ch) || rule.autostart => Some(Chord::new(rule.src_root, rule.src_type)),
            None => None,
        }
    }

    fn emit_at_index(&mut self, now: u64, sink: &mut impl Sink) {
        let sec = self.style.sections[self.cur].as_ref().unwrap();
        let e = sec.events[self.ev_idx];
        let Some(rule) = sec.rules[e.src as usize].as_ref() else {
            self.ev_idx += 1;
            return;
        };
        let dest = rule.dest_ch;
        match e.kind {
            PKind::On { .. } => {
                // Group simultaneous note-ons on this source channel for voice leading.
                let mut keys = [0u8; 8];
                let mut vels = [0u8; 8];
                let mut n = 0;
                let mut j = self.ev_idx;
                while let Some(g) = sec.events.get(j) {
                    if g.tick != e.tick || n == 8 {
                        break;
                    }
                    if g.src == e.src {
                        if let PKind::On { key, vel } = g.kind {
                            keys[n] = key;
                            vels[n] = vel;
                            n += 1;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                    j += 1;
                }
                self.ev_idx = j;
                let part_on = self.parts & (1 << (dest.saturating_sub(8) & 7)) != 0;
                let Some(chord) = self.chord_for(rule) else { return };
                if !part_on || !plays(rule, chord) {
                    return;
                }
                let mut outs = [None; 8];
                transpose_group(&keys[..n], rule, chord, &mut outs[..n]);
                let slot = self.cur as u8;
                for i in 0..n {
                    if let Some(out) = outs[i] {
                        self.note_on(e.src, keys[i], dest, out, vels[i], slot, now, sink);
                    }
                }
            }
            PKind::Off { key } => {
                self.ev_idx += 1;
                self.off_where(sink, |s| s.src == e.src && s.src_key == key);
            }
            PKind::Cc { cc, val } => {
                self.ev_idx += 1;
                sink.send(&[0xB0 | dest, cc, val]);
            }
            PKind::Pc { prog } => {
                self.ev_idx += 1;
                sink.send(&[0xC0 | dest, prog]);
            }
            PKind::Bend { lo, hi } => {
                self.ev_idx += 1;
                sink.send(&[0xE0 | dest, lo, hi]);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn note_on(&mut self, src: u8, src_key: u8, dest: u8, out: u8, vel: u8, slot: u8, now: u64, sink: &mut impl Sink) {
        // Steal an identical sounding note on the same channel so offs stay balanced.
        self.off_where(sink, |s| s.dest == dest && s.out == out);
        if let Some(free) = self.sounding.iter_mut().find(|s| !s.active) {
            *free = Sounding { active: true, src, src_key, dest, out, vel, slot, started_ns: now };
            sink.send(&[0x90 | dest, out, vel]);
        }
    }

    fn off_where(&mut self, sink: &mut impl Sink, f: impl Fn(&Sounding) -> bool) {
        for s in self.sounding.iter_mut() {
            if s.active && f(s) {
                s.active = false;
                sink.send(&[0x80 | s.dest, s.out, 0]);
            }
        }
    }

    fn all_off(&mut self, sink: &mut impl Sink) {
        self.off_where(sink, |_| true);
    }

    /// Re-pitch sounding notes after a chord change according to each part's retrigger rule.
    fn revoice(&mut self, chord: Chord, now: u64, sink: &mut impl Sink) {
        for i in 0..MAX_SOUNDING {
            let s = self.sounding[i];
            if !s.active || is_drum_part(s.dest) {
                continue;
            }
            let Some(sec) = self.style.sections[s.slot as usize].as_ref() else { continue };
            let Some(rule) = sec.rules[s.src as usize].as_ref() else { continue };
            if !plays(rule, chord) {
                self.sounding[i].active = false;
                sink.send(&[0x80 | s.dest, s.out, 0]);
                continue;
            }
            // Group notes that started together on this channel so Root Fixed voicings move as a unit.
            let mut idx = [0usize; 8];
            let mut keys = [0u8; 8];
            let mut n = 0;
            for j in i..MAX_SOUNDING {
                let o = self.sounding[j];
                if o.active && o.src == s.src && o.slot == s.slot && o.started_ns == s.started_ns && n < 8 {
                    idx[n] = j;
                    keys[n] = o.src_key;
                    n += 1;
                }
            }
            let mut outs = [None; 8];
            transpose_group(&keys[..n], rule, chord, &mut outs[..n]);
            for k in 0..n {
                let j = idx[k];
                let o = self.sounding[j];
                if o.started_ns == u64::MAX {
                    continue; // already handled this pass
                }
                let late = now.saturating_sub(o.started_ns) < LATE_CHORD_NS;
                let zone = rule.zone_for(o.src_key);
                let target = match (late, zone.rtr) {
                    (true, _) => outs[k],
                    (false, Rtr::Stop) => None,
                    (false, Rtr::PitchShiftToRoot | Rtr::RetriggerToRoot) => {
                        let pc = chord.bass.unwrap_or(chord.root) as i32;
                        let cur = o.out as i32;
                        let mut d = (pc - cur).rem_euclid(12);
                        if d > 6 {
                            d -= 12;
                        }
                        Some((cur + d).clamp(0, 127) as u8)
                    }
                    (false, _) => outs[k],
                };
                if target == Some(o.out) {
                    self.sounding[j].started_ns = u64::MAX;
                    continue;
                }
                sink.send(&[0x80 | o.dest, o.out, 0]);
                match target {
                    Some(t) => {
                        sink.send(&[0x90 | o.dest, t, o.vel]);
                        self.sounding[j].out = t;
                        self.sounding[j].started_ns = u64::MAX;
                    }
                    None => self.sounding[j].active = false,
                }
            }
            // Restore start times for grouping on the next chord change.
            for k in 0..n {
                if self.sounding[idx[k]].started_ns == u64::MAX {
                    self.sounding[idx[k]].started_ns = s.started_ns;
                }
            }
        }
    }
}

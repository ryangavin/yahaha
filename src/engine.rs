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

    /// Does part `dest_ch` play its notes exactly as written in this section, whatever the
    /// chord? True for the drum parts, and for parts whose every source channel is Root Fixed
    /// (or Guitar) + Bypass for every key: the same test `theory::transpose` passes through on.
    pub fn plays_as_written(&self, dest_ch: u8) -> bool {
        use crate::sff::{Ntr, Ntt};
        crate::theory::is_drum_part(dest_ch)
            || self.rules.iter().flatten().filter(|r| r.dest_ch == dest_ch).all(|r| {
                (0..=127).all(|k| {
                    let z = r.zone_for(k);
                    z.ntt == Ntt::Bypass && z.ntr != Ntr::RootTrans
                })
            })
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
    /// The style's channel setup (SInt), remapped to destination channels, without the
    /// parts' CC7 (the mixer sends those).
    pub init: Msgs,
    /// How many of `init`'s messages set up the parts (voices, controllers, XG part
    /// parameters). The rest is the effect and drum setup SysEx.
    pub init_parts: usize,
    /// Voice (bank MSB, LSB, program) per destination channel 8..16, for display.
    pub voices: [Option<(u8, u8, u8)>; 16],
    /// Destination channels that Master transpose leaves alone: the drum parts and any
    /// part whose voice is a drum or SFX kit (bank MSB 126/127).
    pub kit: [bool; 16],
    /// The style's own part levels: the last init (SInt) CC7 routed to each accompaniment
    /// part 1-8 (MIDI ch 9-16), or the GM default 100. Loading the style sets the mixer
    /// faders to these.
    pub mix: [u8; 8],
}

/// MIDI messages of any length (SysEx too), stored back to back so sending them from the
/// engine thread allocates nothing.
#[derive(Default)]
pub struct Msgs {
    bytes: Vec<u8>,
    ends: Vec<u32>,
}

impl Msgs {
    fn push(&mut self, m: &[u8]) {
        self.bytes.extend_from_slice(m);
        self.ends.push(self.bytes.len() as u32);
    }

    pub fn len(&self) -> usize {
        self.ends.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &[u8]> + '_ {
        let mut start = 0;
        self.ends.iter().map(move |&end| {
            let m = &self.bytes[start..end as usize];
            start = end as usize;
            m
        })
    }
}

/// XG Multi Part parameter address of a part's volume. The part fader owns the volume.
const XG_PART_VOLUME: u8 = 0x0B;

/// GM default channel volume (CC7) for a part the style never sets.
pub const GM_VOLUME: u8 = 100;
/// Soft takeover: a hardware fader within this distance of the software value picks it up.
pub const PICKUP_RANGE: u8 = 2;

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
        // Per part: bank, program, then the controllers, then its XG part parameters (after
        // the program change, which resets them on an XG receiver). The effect and drum
        // setup SysEx go last, once every part's voice is in place.
        let sint = style.sint();
        let mut init = Msgs::default();
        let mut voices: [Option<(u8, u8, u8)>; 16] = [None; 16];
        let mut mix = [GM_VOLUME; 8];
        for (src, c) in sint.channels.iter().enumerate() {
            let Some(rule) = &route[src] else { continue };
            let d = rule.dest_ch;
            let part = (8..16).contains(&d);
            for (cc, v) in [(0, c.bank_msb), (32, c.bank_lsb)] {
                if let Some(v) = v {
                    init.push(&[0xB0 | d, cc, v]);
                }
            }
            if let Some(prog) = c.program {
                init.push(&[0xC0 | d, prog]);
                voices[d as usize] = Some((c.bank_msb.unwrap_or(0), c.bank_lsb.unwrap_or(0), prog));
            }
            for (cc, v) in [(0, c.pending_msb), (32, c.pending_lsb)] {
                if let Some(v) = v {
                    init.push(&[0xB0 | d, cc, v]);
                }
            }
            match c.volume {
                Some(v) if part => mix[d as usize - 8] = v,
                Some(v) => init.push(&[0xB0 | d, 7, v]),
                None => {}
            }
            for (cc, v) in [(10, c.pan), (91, c.reverb), (93, c.chorus)] {
                if let Some(v) = v {
                    init.push(&[0xB0 | d, cc, v]);
                }
            }
            for ev in &c.other {
                match *ev {
                    Ev::Cc { cc, val, .. } => init.push(&[0xB0 | d, cc, val]),
                    Ev::Bend { val, .. } => init.push(&[0xE0 | d, (val & 0x7F) as u8, (val >> 7) as u8]),
                    _ => {}
                }
            }
            for &(addr, v) in &c.xg_part {
                if !(part && addr == XG_PART_VOLUME) {
                    init.push(&[0xF0, 0x43, 0x10, 0x4C, 0x08, d, addr, v, 0xF7]);
                }
            }
        }
        let init_parts = init.len();
        let mut buf = Vec::new();
        for v in &sint.sysex {
            buf.clone_from(v);
            // An insertion or variation effect assigned to a part follows that part to its
            // destination channel; a part the style never routes gets none (7F = off).
            if let Some((i, part)) = crate::sff::xg_effect_part(v).and_then(|i| Some((i, route.get(v[i] as usize)?))) {
                buf[i] = part.as_ref().map_or(0x7F, |r| r.dest_ch);
            }
            init.push(&buf);
        }
        let mut kit = [false; 16];
        for (d, k) in kit.iter_mut().enumerate() {
            *k = is_drum_part(d as u8) || voices[d].is_some_and(|(msb, _, _)| msb >= 126);
        }
        Prepared {
            name: style.name.clone(),
            ppq: style.ppq as u32,
            bpm: style.bpm(),
            tpb: style.ticks_per_bar(),
            sections,
            init,
            init_parts,
            voices,
            kit,
            mix,
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
    StopAcmp,
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
    /// Mixer fader per part (0..=127): the part's volume, sent as its CC7 unchanged.
    pub volumes: [u8; 8],
    /// Parts whose hardware fader is waiting to pick up the software value (soft takeover).
    pub pickup: u8,
    pub stop_acmp: bool,
    /// Keyboard and Master transpose in semitones (-12..=12 each).
    pub transpose: Transpose,
    /// The chord as fingered, before Keyboard transpose (`chord` is what the style follows).
    pub played: Option<Chord>,
}

/// Genos TRANSPOSE targets that matter for live play (RM p.42). Keyboard shifts the keys
/// and the chord root sent to the Style; Master shifts everything that sounds, the Style
/// output included, except drum and SFX kits. Song transpose has nothing to act on here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Transpose {
    pub keyboard: i8,
    pub master: i8,
}

impl Transpose {
    pub const RANGE: i8 = 12;

    pub fn new(keyboard: i8, master: i8) -> Transpose {
        Transpose { keyboard: keyboard.clamp(-Self::RANGE, Self::RANGE), master: master.clamp(-Self::RANGE, Self::RANGE) }
    }

    /// Total shift applied to the notes the player plays.
    pub fn keys(self) -> i8 {
        self.keyboard + self.master
    }
}

/// Shift a key by `d` semitones, folding by octaves to stay inside the MIDI range.
#[inline]
pub fn shift_key(key: u8, d: i8) -> u8 {
    let mut k = key as i32 + d as i32;
    while k > 127 {
        k -= 12;
    }
    while k < 0 {
        k += 12;
    }
    k as u8
}

/// A chord moved by `d` semitones (root and on-bass note).
pub fn shift_chord(c: Chord, d: i8) -> Chord {
    let pc = |p: u8| (p as i32 + d as i32).rem_euclid(12) as u8;
    Chord { root: pc(c.root), bass: c.bass.map(pc), ..c }
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

/// Soft takeover for an absolute, non-motorised hardware fader controlling a value that
/// software can also move (the Launchkey part and master faders). After software moves
/// the value, the fader is ignored until it comes within `PICKUP_RANGE` of it or crosses
/// it; then it follows again. A fader that has never reported must also pick up first.
#[derive(Clone, Copy, Debug)]
pub struct Takeover {
    /// Last position the fader reported (`HW_UNKNOWN` until it moves).
    hw: u8,
    /// The fader tracks the value.
    picked: bool,
}

/// `Takeover::hw`: the fader has not reported a position yet.
const HW_UNKNOWN: u8 = 255;

impl Takeover {
    pub const NEW: Takeover = Takeover { hw: HW_UNKNOWN, picked: false };

    /// Software set the value to `v`: the fader keeps control only if it is already there.
    pub fn software_moved(&mut self, v: u8) {
        self.picked = self.hw != HW_UNKNOWN && self.hw.abs_diff(v) <= PICKUP_RANGE;
    }

    /// The fader reported `v` while the value is `cur`. True: the fader controls the value
    /// and `v` applies. Crossing is judged from the last report, so a move lost on the way
    /// (a full command ring) still counts as a crossing on the next one.
    pub fn hardware(&mut self, cur: u8, v: u8) -> bool {
        let prev = std::mem::replace(&mut self.hw, v);
        if !self.picked {
            let (c, a, b) = (cur as i16, prev as i16, v as i16);
            let near = v.abs_diff(cur) <= PICKUP_RANGE;
            let crossed = prev != HW_UNKNOWN && (a - c).signum() != (b - c).signum();
            self.picked = near || crossed;
        }
        self.picked
    }

    /// The fader has reported a position but does not control the value yet.
    pub fn waiting(&self) -> bool {
        self.hw != HW_UNKNOWN && !self.picked
    }
}

const EMPTY: Sounding = Sounding { active: false, src: 0, src_key: 0, dest: 0, out: 0, vel: 0, slot: 0, started_ns: 0 };
const MAX_SOUNDING: usize = 256;
/// Pseudo source channel for Stop Accompaniment notes.
const STOP_ACMP_SRC: u8 = 255;
/// The Style's Bass part (MIDI channel 11).
const BASS_CH: u8 = 10;
/// Notes that started this recently when the chord changes are corrected outright:
/// the player's chord landed just after the beat.
const LATE_CHORD_NS: u64 = 40_000_000;

/// The chord a channel follows. With no chord yet, or after Chord Cancel ("a state in
/// which no chord is input", OM p.46), only rhythm parts and channels whose CASM
/// autostart bit is set play, as recorded (their source chord); everything else rests.
fn effective_chord(chord: Option<Chord>, rule: &ChannelRule) -> Option<Chord> {
    match chord {
        Some(c) if c.ty != CANCEL => Some(c),
        _ if is_drum_part(rule.dest_ch) || rule.autostart => Some(Chord::new(rule.src_root, rule.src_type)),
        _ => None,
    }
}

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
    /// Sync Stop is unavailable with the Full Keyboard fingering types.
    sync_stop_allowed: bool,
    auto_fill: bool,
    main: u8,
    pending_intro: Option<u8>,
    cur: usize,
    sec_start: f64,
    /// Section-relative tick this pass of the section started playing from (a Fill or
    /// Break enters mid-bar; nothing before it has sounded).
    entry: f64,
    ev_idx: usize,
    queued: Option<Queued>,
    /// The chord the style follows: `played` moved by Keyboard transpose.
    chord: Option<Chord>,
    played: Option<Chord>,
    transpose: Transpose,
    bpm: f64,
    anchor_ns: u64,
    anchor_tick: f64,
    ns_per_tick: f64,
    parts: u8,
    /// Mixer faders: each part's channel volume (CC7), sent as is. A style load sets them
    /// from the style's own levels.
    mixer: [u8; 8],
    /// Parts whose fader the player has moved since the style loaded: pattern CC7 no
    /// longer overrides them.
    user_set: u8,
    /// Soft takeover state of each part's hardware fader.
    takeover: [Takeover; 8],
    stop_acmp: bool,
    /// Manual Bass (Upper detection mode): the Style's Bass part is muted; the player's
    /// left hand plays the bass instead.
    manual_bass: bool,
    taps: [u64; 4],
    tap_n: usize,
    sounding: [Sounding; MAX_SOUNDING],
}

impl Engine {
    pub fn new(style: Box<Prepared>) -> Engine {
        let bpm = style.bpm;
        let mixer = style.mix;
        let mut e = Engine {
            style,
            running: false,
            sync_armed: true,
            sync_stop: false,
            sync_stop_allowed: true,
            auto_fill: true,
            main: 0,
            pending_intro: None,
            cur: 4,
            sec_start: 0.0,
            entry: 0.0,
            ev_idx: 0,
            queued: None,
            chord: None,
            played: None,
            transpose: Transpose::default(),
            bpm,
            anchor_ns: 0,
            anchor_tick: 0.0,
            ns_per_tick: 0.0,
            parts: 0xFF,
            mixer,
            user_set: 0,
            takeover: [Takeover::NEW; 8],
            stop_acmp: false,
            manual_bass: false,
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
        // A new style brings its own part levels (no parameter lock): the faders move to
        // them and the player's earlier moves are forgotten.
        self.user_set = 0;
        for p in 0..8 {
            let v = self.style.mix[p];
            self.set_mixer(p, v);
        }
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

    /// The style's channel setup, with the mixer's volume on each part in place of the
    /// style's own CC7 so a restart never undoes a fader the player moved.
    pub fn send_init(&mut self, sink: &mut impl Sink) {
        for m in self.style.init.iter() {
            sink.send(m);
        }
        for p in 0..8u8 {
            sink.send(&[0xB0 | (8 + p), 7, self.mixer[p as usize]]);
        }
    }

    /// Parts the player has not moved go back to the style's own level (the SInt CC7). A
    /// level already there is left alone, so its hardware fader keeps control.
    fn restore_untouched_levels(&mut self) {
        for p in 0..8 {
            if self.user_set & (1 << p) == 0 && self.mixer[p] != self.style.mix[p] {
                let v = self.style.mix[p];
                self.set_mixer(p, v);
            }
        }
    }

    /// A section change plays the style's part setup (SInt) again, as newer instruments
    /// do, so a voice or controller a pattern changed does not carry into the next section.
    /// Its CC7 is a part's fader value, so like a pattern CC7 it moves only the faders the
    /// player has not moved; the others keep their level and are not re-sent. The effect
    /// and drum setup SysEx is not re-sent: no pattern changes it, and an XG receiver
    /// would cut the reverb and delay tails.
    fn reapply_init(&mut self, sink: &mut impl Sink) {
        self.restore_untouched_levels();
        for m in self.style.init.iter().take(self.style.init_parts) {
            sink.send(m);
        }
        for p in 0..8u8 {
            if self.user_set & (1 << p) == 0 {
                sink.send(&[0xB0 | (8 + p), 7, self.mixer[p as usize]]);
            }
        }
    }

    /// Set a part's fader from software (style load, pattern CC7). A hardware fader that
    /// is not already there has to pick the new value up before it takes control again.
    fn set_mixer(&mut self, p: usize, v: u8) {
        self.mixer[p] = v;
        self.takeover[p].software_moved(v);
    }

    /// A CC7 from the style's pattern on `ch`. It is the part's fader value, so it moves the
    /// fader, unless the player has moved that fader since the style loaded.
    fn pattern_volume(&mut self, ch: u8, val: u8, sink: &mut impl Sink) {
        if !(8..16).contains(&ch) {
            sink.send(&[0xB0 | ch, 7, val]);
            return;
        }
        let p = (ch - 8) as usize;
        if self.user_set & (1 << p) != 0 {
            return;
        }
        self.set_mixer(p, val);
        sink.send(&[0xB0 | ch, 7, val]);
    }

    /// A part fader (0..8) moved to `value`: sent as that part's CC7, unchanged.
    pub fn set_volume(&mut self, part: u8, value: u8, sink: &mut impl Sink) {
        let p = (part & 7) as usize;
        let v = value.min(127);
        self.mixer[p] = v;
        self.user_set |= 1 << p;
        sink.send(&[0xB0 | (8 + p as u8), 7, v]);
    }

    /// A hardware fader (absolute, not motorised) reported `value` for part 0..8. Soft
    /// takeover: after the software value moved on its own, the fader is ignored until it
    /// comes within `PICKUP_RANGE` of that value or crosses it; then it follows again.
    pub fn hw_fader(&mut self, part: u8, value: u8, sink: &mut impl Sink) {
        let p = (part & 7) as usize;
        let v = value.min(127);
        if self.takeover[p].hardware(self.mixer[p], v) {
            self.set_volume(part, v, sink);
        }
    }

    /// Parts whose hardware fader has reported a position but not yet picked up.
    fn pickup_waiting(&self) -> u8 {
        let mut m = 0;
        for (p, t) in self.takeover.iter().enumerate() {
            if t.waiting() {
                m |= 1 << p;
            }
        }
        m
    }

    /// Manual Bass on/off: mutes the Style's Bass part (and its Stop Accompaniment note).
    pub fn set_manual_bass(&mut self, on: bool, sink: &mut impl Sink) {
        self.manual_bass = on;
        if on {
            self.off_where(sink, |n| n.dest == BASS_CH);
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

    #[allow(dead_code)]
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
            volumes: self.mixer,
            pickup: self.pickup_waiting(),
            stop_acmp: self.stop_acmp,
            transpose: self.transpose,
            played: self.played,
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

    /// A chord as fingered (before Keyboard transpose). Starts playback when sync start is armed.
    pub fn set_chord(&mut self, played: Chord, now: u64, sink: &mut impl Sink) {
        self.played = Some(played);
        let chord = shift_chord(played, self.transpose.keyboard);
        let prev = self.chord;
        self.chord = Some(chord);
        if self.sync_armed && !self.running && chord.ty != CANCEL {
            self.start(now, sink);
            return;
        }
        if self.running {
            if prev.is_some() {
                self.revoice(chord, now, sink);
            }
            self.catch_up(prev, chord, now, sink);
        }
        if !self.running && self.stop_acmp {
            self.sound_stop_acmp(chord, now, sink);
        }
    }

    /// New transpose settings. They apply to notes started from now on; sounding notes keep
    /// their pitch until they end or the next chord change revoices them. A Keyboard change
    /// moves the held chord at once, so the band follows as if the same keys had been played
    /// in the new key. Stop Accompaniment notes move only if they are still sounding.
    pub fn set_transpose(&mut self, t: Transpose, now: u64, sink: &mut impl Sink) {
        let old = self.transpose;
        self.transpose = Transpose::new(t.keyboard, t.master);
        if self.transpose.keyboard == old.keyboard {
            return;
        }
        let Some(played) = self.played else { return };
        let chord = shift_chord(played, self.transpose.keyboard);
        let prev = self.chord;
        self.chord = Some(chord);
        if self.running {
            self.revoice(chord, now, sink);
            self.catch_up(prev, chord, now, sink);
        } else if self.stop_acmp && self.sounding.iter().any(|n| n.active && n.src == STOP_ACMP_SRC) {
            self.sound_stop_acmp(chord, now, sink);
        }
    }

    /// Master transpose for a note on `dest` (never on drum/SFX kits).
    #[inline]
    fn master(&self, dest: u8, key: u8) -> u8 {
        if self.style.kit[dest as usize & 15] {
            key
        } else {
            shift_key(key, self.transpose.master)
        }
    }

    /// Stop Accompaniment: with the band stopped, the held chord sounds on the style's
    /// Bass (root / on-bass note) and Pad (chord tones) voices.
    fn sound_stop_acmp(&mut self, chord: Chord, now: u64, sink: &mut impl Sink) {
        self.off_where(sink, |n| n.src == STOP_ACMP_SRC);
        if chord.ty == CANCEL {
            return;
        }
        let bass = 36 + chord.bass.unwrap_or(chord.root);
        self.note_on(STOP_ACMP_SRC, 0, 10, bass, 90, 4, now, sink);
        for (i, &t) in crate::theory::chord_tones(chord.ty).iter().enumerate() {
            let pc = (chord.root + t) % 12;
            let key = 55 + (pc + 12 - 7) % 12; // G3..F#4
            self.note_on(STOP_ACMP_SRC, 1 + i as u8, 13, key, 70, 4, now, sink);
        }
    }

    /// The fingering type allows Sync Stop or not; disallowing turns it off.
    pub fn allow_sync_stop(&mut self, on: bool) {
        self.sync_stop_allowed = on;
        self.sync_stop &= on;
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
            Button::SyncStop => self.sync_stop = !self.sync_stop && self.sync_stop_allowed,
            Button::AutoFill => self.auto_fill = !self.auto_fill,
            Button::TogglePart(p) => {
                self.parts ^= 1 << (p & 7);
                if self.parts & (1 << (p & 7)) == 0 {
                    self.off_where(sink, |n| n.dest == 8 + (p & 7));
                }
            }
            Button::StopAcmp => {
                self.stop_acmp = !self.stop_acmp;
                if !self.stop_acmp {
                    self.off_where(sink, |n| n.src == STOP_ACMP_SRC);
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
        self.all_off(sink);
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
        // A start plays the style's channel setup (SInt) again: parts the player has not
        // moved go back to the style's own level, so an Intro/Ending pattern's CC7 from the
        // last run does not stick. Faders the player moved keep their value.
        self.restore_untouched_levels();
        self.send_init(sink);
        self.cur = slot;
        self.sec_start = 0.0;
        self.entry = 0.0;
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
        self.entry = pos;
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
        // A section repeating itself is not a change: its own pattern carries on.
        if next != self.cur {
            self.reapply_init(sink);
        }
        self.cur = next;
        self.sec_start = start;
        self.seek(at - start);
        self.chase(sink);
        let _ = now;
    }

    /// A section entered mid-bar (a Fill or Break at the next beat) skips its events before
    /// the entry point. Its notes stay skipped, but its voice and controller changes are
    /// sent, in order, so the part plays the section's voice from the entry on.
    fn chase(&mut self, sink: &mut impl Sink) {
        for i in 0..self.ev_idx {
            let sec = self.style.sections[self.cur].as_ref().unwrap();
            let e = sec.events[i];
            let Some(dest) = sec.rules[e.src as usize].as_ref().map(|r| r.dest_ch) else { continue };
            self.emit_control(dest, e.kind, sink);
        }
    }

    /// A pattern's controller, program change or pitch bend on `dest`; notes are not
    /// controls and send nothing here.
    fn emit_control(&mut self, dest: u8, kind: PKind, sink: &mut impl Sink) {
        match kind {
            PKind::Cc { cc: 7, val } => self.pattern_volume(dest, val, sink),
            PKind::Cc { cc, val } => sink.send(&[0xB0 | dest, cc, val]),
            PKind::Pc { prog } => sink.send(&[0xC0 | dest, prog]),
            PKind::Bend { lo, hi } => sink.send(&[0xE0 | dest, lo, hi]),
            PKind::On { .. } | PKind::Off { .. } => {}
        }
    }

    fn chord_for(&self, rule: &ChannelRule) -> Option<Chord> {
        effective_chord(self.chord, rule)
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
            PKind::Cc { .. } | PKind::Pc { .. } | PKind::Bend { .. } => {
                self.ev_idx += 1;
                self.emit_control(dest, e.kind, sink);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn note_on(&mut self, src: u8, src_key: u8, dest: u8, out: u8, vel: u8, slot: u8, now: u64, sink: &mut impl Sink) {
        if self.manual_bass && dest == BASS_CH {
            return;
        }
        let out = self.master(dest, out);
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
        // Patterns bend (bass slides etc.); leaving a section or style mid-bend would
        // leave the part detuned. Re-centre bends and clear wheels/pedal on every part.
        for ch in 8..16u8 {
            sink.send(&[0xE0 | ch, 0x00, 0x40]);
            sink.send(&[0xB0 | ch, 1, 0]);
            sink.send(&[0xB0 | ch, 64, 0]);
        }
    }

    /// A chord that lands just after the beat (within `LATE_CHORD_NS`) also brings in
    /// the parts the previous chord kept silent (no chord yet, Chord Cancel, or CASM
    /// chord-mute routing). Their notes from that window were skipped, so `revoice` has
    /// nothing to correct; start the ones the pattern still holds now.
    fn catch_up(&mut self, prev: Option<Chord>, chord: Chord, now: u64, sink: &mut impl Sink) {
        let Some(sec) = self.style.sections[self.cur].as_ref() else { return };
        // Never reach back past where this section came in: those notes never played.
        let lo = (self.tick_at(now.saturating_sub(LATE_CHORD_NS)) - self.sec_start).max(self.entry);
        let end = self.ev_idx.min(sec.events.len());
        let mut i = end;
        while i > 0 && sec.events[i - 1].tick as f64 >= lo {
            i -= 1;
        }
        // (src, src key, dest, out, vel): room for 8 parts x 8 notes; beyond that the
        // rest are dropped rather than allocating.
        let mut buf = [(0u8, 0u8, 0u8, 0u8, 0u8); 64];
        let mut n_buf = 0;
        while i < end {
            let e = sec.events[i];
            // Group simultaneous note-ons on this source channel, as `emit_at_index` does.
            let mut keys = [0u8; 8];
            let mut vels = [0u8; 8];
            let mut n = 0;
            let mut j = i;
            while let Some(g) = sec.events[..end].get(j) {
                match g.kind {
                    PKind::On { key, vel } if g.tick == e.tick && g.src == e.src && n < 8 => {
                        keys[n] = key;
                        vels[n] = vel;
                        n += 1;
                        j += 1;
                    }
                    _ => break,
                }
            }
            i = j.max(i + 1);
            let Some(rule) = sec.rules[e.src as usize].as_ref() else { continue };
            let part_on = self.parts & (1 << (rule.dest_ch.saturating_sub(8) & 7)) != 0;
            let was = effective_chord(prev, rule).filter(|&c| plays(rule, c));
            let Some(now_chord) = effective_chord(Some(chord), rule).filter(|&c| plays(rule, c)) else { continue };
            if n == 0 || !part_on || was.is_some() {
                continue;
            }
            let mut outs = [None; 8];
            transpose_group(&keys[..n], rule, now_chord, &mut outs[..n]);
            for k in 0..n {
                let released = sec.events[j..end]
                    .iter()
                    .any(|o| o.src == e.src && matches!(o.kind, PKind::Off { key } if key == keys[k]));
                if let (Some(out), false, true) = (outs[k], released, n_buf < buf.len()) {
                    buf[n_buf] = (e.src, keys[k], rule.dest_ch, out, vels[k]);
                    n_buf += 1;
                }
            }
        }
        let slot = self.cur as u8;
        for &(src, key, dest, out, vel) in &buf[..n_buf] {
            self.note_on(src, key, dest, out, vel, slot, now, sink);
        }
    }

    /// Re-pitch sounding notes after a chord change according to each part's retrigger rule.
    fn revoice(&mut self, chord: Chord, now: u64, sink: &mut impl Sink) {
        for i in 0..MAX_SOUNDING {
            let s = self.sounding[i];
            if !s.active || is_drum_part(s.dest) || s.src >= 16 {
                continue;
            }
            let Some(sec) = self.style.sections[s.slot as usize].as_ref() else { continue };
            let Some(rule) = sec.rules[s.src as usize].as_ref() else { continue };
            let Some(chord) = effective_chord(Some(chord), rule).filter(|&c| plays(rule, c)) else {
                self.sounding[i].active = false;
                sink.send(&[0x80 | s.dest, s.out, 0]);
                continue;
            };
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
                    (true, _) => outs[k].map(|t| self.master(o.dest, t)),
                    (false, Rtr::Stop) => None,
                    (false, Rtr::PitchShiftToRoot | Rtr::RetriggerToRoot) => {
                        let pc = self.master(o.dest, chord.bass.unwrap_or(chord.root)) as i32;
                        let cur = o.out as i32;
                        let mut d = (pc - cur).rem_euclid(12);
                        if d > 6 {
                            d -= 12;
                        }
                        Some((cur + d).clamp(0, 127) as u8)
                    }
                    (false, _) => outs[k].map(|t| self.master(o.dest, t)),
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

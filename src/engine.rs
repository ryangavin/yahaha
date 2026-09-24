//! The arranger engine: section state machine, pattern playback, chord following.
//!
//! The engine is deterministic and has no notion of threads or wall time: callers pass
//! the current time in nanoseconds. It never allocates after construction, so the
//! real-time output thread can drive it directly.

use crate::sff::{ChannelRule, Ev, Ntr, Ntt, Rtr, SectionId, Style};
use crate::theory::{is_drum_part, plays, transpose_group, Chord, CANCEL, GUITAR_NOISE};

pub trait Sink {
    fn send(&mut self, msg: &[u8]);

    /// Retrigger Rule pitch shift: every note on `ch` now sounds `semis` above the key it
    /// was sent with (the engine has already sent the pitch bend). MIDI sinks ignore it; the
    /// sim listing uses it to show the pitch that sounds.
    fn retune(&mut self, _ch: u8, _semis: i8) {}
}

/// Pitch bend range (RPN 0) a part has before the style sets one: the GM/XG default.
pub const GM_BEND_RANGE: u8 = 2;
/// The pitch shift the engine can bend a part that follows chords by: up to an octave
/// either way, which covers every to-Root move (at most 6) and nearly every Pitch Shift.
/// It is also the smallest bend range such a part gets on the output.
pub const RTR_BEND_RANGE: u8 = 12;
/// The widest pitch bend range a Genos part takes (DL p.98: RPN 0 Pitch Bend Sensitivity
/// 00H-18H, received by the Style parts; MIDI Implementation Chart: 0-24 semi).
pub const MAX_BEND_RANGE: u8 = 24;

/// Parts whose sounding notes a chord change can re-pitch: every accompaniment part but
/// the two rhythm parts.
#[inline]
fn follows_chords(ch: u8) -> bool {
    (8..16).contains(&ch) && !is_drum_part(ch)
}

/// The pitch bend range a part gets on the output: the style's own, raised so that a
/// pitch shift of `RTR_BEND_RANGE` fits on top of the widest bend its patterns make
/// (`pat_max` semitones), but no wider than `MAX_BEND_RANGE`, and never below
/// `RTR_BEND_RANGE`. The pattern's own bends are rescaled to it (see `Engine::send_bend`).
#[inline]
fn out_bend_range(style_range: u8, pat_max: u8) -> u8 {
    style_range.max(RTR_BEND_RANGE).max(pat_max.saturating_add(RTR_BEND_RANGE).min(MAX_BEND_RANGE))
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
    /// How many of `init`'s messages a section change sends again: the parts' setup
    /// (voices, controllers, XG part parameters) and the drum setup SysEx, which the
    /// parts' program changes reset. The rest is the effect SysEx.
    pub init_resend: usize,
    /// Voice (bank MSB, LSB, program) per destination channel 8..16, for display.
    pub voices: [Option<(u8, u8, u8)>; 16],
    /// Destination channels that Master transpose leaves alone: the drum parts and any
    /// part whose voice is a drum or SFX kit (bank MSB 126/127).
    pub kit: [bool; 16],
    /// The style's own part levels: the last init (SInt) CC7 routed to each accompaniment
    /// part 1-8 (MIDI ch 9-16), or the GM default 100. Loading the style sets the mixer
    /// faders to these.
    pub mix: [u8; 8],
    /// Each part's pitch bend range as the style's channel setup leaves it (RPN 0, in
    /// semitones). `init` sends `out_bend_range` of it instead.
    pub bend_range: [u8; 16],
    /// The widest pitch bend each part's patterns make, in semitones either way: the
    /// headroom a Retrigger Rule pitch shift must leave them.
    pub pat_bend_max: [u8; 16],
    /// The widest Retrigger Rule pitch shift each part can take, in semitones either way:
    /// what the narrowest output range it can have (its patterns may set a narrower bend
    /// range than the channel setup) leaves over `pat_bend_max`.
    pub shift_room: [u8; 16],
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

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn iter(&self) -> impl Iterator<Item = &[u8]> + '_ {
        let mut start = 0;
        self.ends.iter().map(move |&end| {
            let m = &self.bytes[start..end as usize];
            start = end as usize;
            m
        })
    }

    /// Message `i`.
    pub fn get(&self, i: usize) -> &[u8] {
        let start = if i == 0 { 0 } else { self.ends[i - 1] as usize };
        &self.bytes[start..self.ends[i] as usize]
    }
}

/// A controller value `Mirror` has not seen sent.
const UNSENT: u8 = 0xFF;
/// (N)RPN values `Mirror` remembers per channel.
const MIRROR_PARAMS: usize = 8;
/// A free `Mirror::params` slot.
const NO_PARAM: u16 = 0xFFFF;
/// `Mirror` parameter numbers: an NRPN has this bit set, an RPN has not.
const NRPN_BIT: u16 = 0x4000;

/// What the engine has sent on each channel: controllers, voice, (N)RPN data entries and
/// pitch bend. A section change compares the style's channel setup with it and sends only
/// what differs, so a section change that changes nothing sends nothing: a receiver never
/// reloads a voice it already plays or has its drum setup rewritten (see
/// `Engine::reapply_init`).
struct Mirror {
    cc: [[u8; 128]; 16],
    /// (bank MSB, LSB, program) of the last program change, with the bank selects in
    /// effect when it was sent.
    voice: [Option<(u8, u8, u8)>; 16],
    rpn: [u16; 16],
    nrpn: [u16; 16],
    /// Channels where an NRPN was selected last (else an RPN).
    nrpn_on: u16,
    /// (parameter, data entry MSB, LSB) per channel; parameter as `selected` returns it.
    params: [[(u16, u8, u8); MIRROR_PARAMS]; 16],
    bend: [Option<u16>; 16],
}

impl Mirror {
    const NEW: Mirror = Mirror {
        cc: [[UNSENT; 128]; 16],
        voice: [None; 16],
        rpn: [RPN_NULL; 16],
        nrpn: [RPN_NULL; 16],
        nrpn_on: 0,
        params: [[(NO_PARAM, UNSENT, UNSENT); MIRROR_PARAMS]; 16],
        bend: [None; 16],
    };

    /// The (N)RPN a data entry on `ch` sets, if any (the null RPN/NRPN sets nothing).
    fn selected(&self, ch: usize) -> Option<u16> {
        if self.nrpn_on & (1 << ch) != 0 {
            (self.nrpn[ch] != RPN_NULL).then_some(NRPN_BIT | self.nrpn[ch])
        } else {
            (self.rpn[ch] != RPN_NULL).then_some(self.rpn[ch])
        }
    }

    /// The data entry sent for parameter `p` on `ch` as (MSB, LSB), if known.
    fn param(&self, ch: usize, p: u16) -> Option<(u8, u8)> {
        self.params[ch].iter().find(|e| e.0 == p).map(|e| (e.1, e.2))
    }

    fn set_param(&mut self, ch: usize, p: u16, msb: Option<u8>, lsb: Option<u8>) {
        let slots = &mut self.params[ch];
        let i = slots.iter().position(|e| e.0 == p).or_else(|| slots.iter().position(|e| e.0 == NO_PARAM));
        // Full: forget the oldest; an unknown value is simply sent again.
        let i = i.unwrap_or_else(|| {
            slots.rotate_left(1);
            slots[MIRROR_PARAMS - 1] = (NO_PARAM, UNSENT, UNSENT);
            MIRROR_PARAMS - 1
        });
        let e = &mut slots[i];
        if e.0 != p {
            *e = (p, UNSENT, UNSENT);
        }
        if let Some(v) = msb {
            e.1 = v;
        }
        if let Some(v) = lsb {
            e.2 = v;
        }
    }

    /// Note a message sent.
    fn track(&mut self, m: &[u8]) {
        if m.len() < 2 || m[0] >= 0xF0 {
            return;
        }
        let ch = (m[0] & 0x0F) as usize;
        match (m[0] & 0xF0, m.len()) {
            (0xB0, 3) => {
                let (cc, v) = (m[1] & 0x7F, m[2]);
                self.cc[ch][cc as usize] = v;
                let bit = 1 << ch;
                match cc {
                    101 | 100 => {
                        self.rpn[ch] = select_rpn(self.rpn[ch], cc, v);
                        self.nrpn_on &= !bit;
                    }
                    99 => {
                        self.nrpn[ch] = (self.nrpn[ch] & 0x7F) | (v as u16) << 7;
                        self.nrpn_on |= bit;
                    }
                    98 => {
                        self.nrpn[ch] = (self.nrpn[ch] & !0x7F) | v as u16;
                        self.nrpn_on |= bit;
                    }
                    6 | 38 => {
                        if let Some(p) = self.selected(ch) {
                            let (msb, lsb) = if cc == 6 { (Some(v), None) } else { (None, Some(v)) };
                            self.set_param(ch, p, msb, lsb);
                        }
                    }
                    _ => {}
                }
            }
            (0xC0, _) => self.voice[ch] = Some((self.cc[ch][0], self.cc[ch][32], m[1])),
            (0xE0, 3) => self.bend[ch] = Some((m[2] as u16) << 7 | m[1] as u16),
            _ => {}
        }
    }

    /// Send `m` and note it.
    #[inline]
    fn send(&mut self, sink: &mut impl Sink, m: &[u8]) {
        self.track(m);
        sink.send(m);
    }

    /// Select parameter `p` (as `selected` returns it; `None` = the null RPN) on `ch`.
    fn select(&mut self, sink: &mut impl Sink, ch: u8, p: Option<u16>) {
        let st = 0xB0 | ch;
        match p {
            Some(p) if p & NRPN_BIT != 0 => {
                let n = p & !NRPN_BIT;
                self.send(sink, &[st, 99, (n >> 7) as u8]);
                self.send(sink, &[st, 98, (n & 0x7F) as u8]);
            }
            p => {
                let r = p.unwrap_or(RPN_NULL);
                self.send(sink, &[st, 101, (r >> 7) as u8]);
                self.send(sink, &[st, 100, (r & 0x7F) as u8]);
            }
        }
    }
}

/// XG Multi Part parameter address of a part's volume. The part fader owns the volume.
const XG_PART_VOLUME: u8 = 0x0B;

/// GM default channel volume (CC7) for a part the style never sets.
pub const GM_VOLUME: u8 = 100;
/// No RPN selected (MSB and LSB 127).
const RPN_NULL: u16 = 0x3FFF;
/// Pitch bend centre.
const BEND_CENTRE: u16 = 0x2000;
/// Soft takeover: a hardware fader within this distance of the software value picks it up.
pub const PICKUP_RANGE: u8 = 2;

/// The RPN selected on a channel (MSB << 7 | LSB) after controller `cc` = `val`. Selecting
/// an NRPN (CC 98/99) deselects the RPN, so data entry no longer sets it.
#[inline]
fn select_rpn(rpn: u16, cc: u8, val: u8) -> u16 {
    match cc {
        101 => (rpn & 0x7F) | (val as u16) << 7,
        100 => (rpn & !0x7F) | val as u16,
        98 | 99 => RPN_NULL,
        _ => rpn,
    }
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
        // Pitch bend range: note what the style's channel setup sets on each part.
        let sint = style.sint();
        let routed = || sint.channels.iter().enumerate().filter_map(|(src, c)| Some((route[src].as_ref()?.dest_ch as usize & 15, c)));
        let mut bend_range = [GM_BEND_RANGE; 16];
        let mut rpn = [RPN_NULL; 16];
        let mut depth = [0u16; 16];
        for (d, c) in routed() {
            for ev in &c.other {
                match *ev {
                    Ev::Cc { cc, val, .. } => {
                        if cc == 6 && rpn[d] == 0 {
                            bend_range[d] = val;
                        }
                        rpn[d] = select_rpn(rpn[d], cc, val);
                    }
                    Ev::Bend { val, .. } => depth[d] = depth[d].max(val.abs_diff(BEND_CENTRE)),
                    _ => {}
                }
            }
        }
        // The widest bend each part makes (the channel setup's or a pattern's), in
        // semitones of the widest range the style gives it, and the narrowest range.
        let mut widest = bend_range;
        let mut narrowest = bend_range;
        for sec in sections.iter().flatten() {
            let mut rpn = [RPN_NULL; 16];
            for e in &sec.events {
                let Some(d) = sec.rules[e.src as usize & 15].as_ref().map(|r| r.dest_ch as usize & 15) else { continue };
                match e.kind {
                    PKind::Cc { cc, val } => {
                        let r = &mut rpn[e.src as usize & 15];
                        if cc == 6 && *r == 0 {
                            widest[d] = widest[d].max(val);
                            narrowest[d] = narrowest[d].min(val);
                        }
                        *r = select_rpn(*r, cc, val);
                    }
                    PKind::Bend { lo, hi } => depth[d] = depth[d].max(((hi as u16) << 7 | lo as u16).abs_diff(BEND_CENTRE)),
                    _ => {}
                }
            }
        }
        let mut pat_bend_max = [0u8; 16];
        for c in 8..16 {
            pat_bend_max[c] = (depth[c] as f32 / BEND_CENTRE as f32 * widest[c] as f32).ceil() as u8;
        }
        let shift_room: [u8; 16] = std::array::from_fn(|c| out_bend_range(narrowest[c], pat_bend_max[c]) - pat_bend_max[c]);
        // Per part: bank, program, then the controllers, then its XG part parameters (after
        // the program change, which resets them on an XG receiver). Then the drum setup
        // SysEx, once every part's voice and part mode is in place (a program change on a
        // drum setup part resets its drum setup), and the effect SysEx last. A part that
        // follows chords gets its output bend range in place of the style's (see
        // `out_bend_range`), then has it set (a style that sets none leaves the GM
        // default), and the RPN deselected, with the part setup, so a section change sets
        // it again.
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
            let mut rpn = RPN_NULL;
            for ev in &c.other {
                match *ev {
                    Ev::Cc { cc: 6, val, .. } if rpn == 0 && follows_chords(d) => {
                        init.push(&[0xB0 | d, 6, out_bend_range(val, pat_bend_max[d as usize & 15])]);
                    }
                    Ev::Cc { cc, val, .. } => {
                        rpn = select_rpn(rpn, cc, val);
                        init.push(&[0xB0 | d, cc, val]);
                    }
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
        for ch in (8..16u8).filter(|&ch| follows_chords(ch)) {
            let range = out_bend_range(bend_range[ch as usize], pat_bend_max[ch as usize]);
            for (cc, val) in [(101, 0), (100, 0), (6, range), (38, 0), (101, 127), (100, 127)] {
                init.push(&[0xB0 | ch, cc, val]);
            }
        }
        for v in sint.sysex.iter().filter(|v| crate::sff::is_drum_setup(v)) {
            init.push(v);
        }
        let init_resend = init.len();
        let mut buf = Vec::new();
        for v in sint.sysex.iter().filter(|v| !crate::sff::is_drum_setup(v)) {
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
            init_resend,
            voices,
            kit,
            mix,
            bend_range,
            pat_bend_max,
            shift_room,
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
    /// When the note last got an attack: its start, or a retrigger on a chord change.
    attack_ns: u64,
    /// A voice that shares its key with another voice sounding on the part (two voices a
    /// chord folds together): it sends nothing, but keeps its place in the pattern, so a
    /// later chord can part the two again, and it sounds on in the other's place if that
    /// one ends first. There is always an unmuted voice on the same part and key.
    muted: bool,
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
pub const HW_UNKNOWN: u8 = 255;

impl Takeover {
    pub const NEW: Takeover = Takeover { hw: HW_UNKNOWN, picked: false };

    /// The physical fader, last reported at `hw` (`HW_UNKNOWN` if never), is handed the
    /// value `cur` (a fader page switch): it controls it only if it is already there.
    pub fn at(hw: u8, cur: u8) -> Takeover {
        let mut t = Takeover { hw, picked: false };
        t.software_moved(cur);
        t
    }

    /// State kept elsewhere (atomics shared between threads), rebuilt for one report.
    pub fn resume(hw: u8, picked: bool) -> Takeover {
        Takeover { hw, picked }
    }

    pub fn picked(&self) -> bool {
        self.picked
    }

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

const EMPTY: Sounding =
    Sounding { active: false, src: 0, src_key: 0, dest: 0, out: 0, vel: 0, slot: 0, started_ns: 0, attack_ns: 0, muted: false };
const MAX_SOUNDING: usize = 256;
/// Pseudo source channel for Stop Accompaniment notes.
const STOP_ACMP_SRC: u8 = 255;
/// The Style's Bass part (MIDI channel 11).
const BASS_CH: u8 = 10;
/// Notes that started this recently when the chord changes are corrected outright:
/// the player's chord landed just after the beat.
const LATE_CHORD_NS: u64 = 40_000_000;
/// Notes that end this soon after the chord changes are left to end as they are: the
/// player's chord landed just before the beat, and a new attack would be a blip.
pub(crate) const EARLY_CHORD_NS: u64 = 40_000_000;

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
    /// Retrigger Rule pitch shift per channel: the semitones every note on the channel is
    /// bent by. A note sent while it is set goes out that much lower so it sounds true
    /// (`Sounding::out` is the key sent). It returns to 0 when the channel falls silent.
    rtr_bend: [i8; 16],
    /// The pattern's own pitch bend per channel (14-bit, in the style's bend range).
    pat_bend: [u16; 16],
    /// The style's pitch bend range per channel (RPN 0, semitones).
    bend_range: [u8; 16],
    /// The RPN a pattern has selected per channel (MSB << 7 | LSB).
    rpn: [u16; 16],
    /// What has been sent on each channel (controllers, voices, (N)RPNs, bends).
    mirror: Box<Mirror>,
    /// Channels where a pattern sent a program change since the part setup last went out
    /// there: the receiver has reset that part's XG parameters and drum setup, even when
    /// the pattern has since gone back to the setup's voice.
    pattern_pc: u16,
    /// Pitch bends that did not fit the output range and were clamped.
    #[cfg(test)]
    pub(crate) bend_clamps: std::cell::Cell<u32>,
    /// Notes a chord change retriggered, as (time, channel, key sent).
    #[cfg(test)]
    pub(crate) retriggered: Vec<(u64, u8, u8)>,
}

/// What a chord change does to one sounding note (`Engine::revoice_part`). Pitches are
/// what sounds (key sent plus the channel's `rtr_bend`).
#[derive(Clone, Copy, PartialEq)]
enum Revoice {
    /// Untouched: its pattern note-off is due right now.
    Leave,
    /// Keeps sounding at its pitch.
    Hold,
    Cut,
    /// Bend to this pitch, no new attack (Pitch Shift, Pitch Shift to Root).
    Shift(u8),
    /// New attack at this pitch.
    Retrigger(u8),
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
            rtr_bend: [0; 16],
            pat_bend: [BEND_CENTRE; 16],
            bend_range: [GM_BEND_RANGE; 16],
            rpn: [RPN_NULL; 16],
            mirror: Box::new(Mirror::NEW),
            pattern_pc: 0,
            #[cfg(test)]
            bend_clamps: Default::default(),
            #[cfg(test)]
            retriggered: Vec::new(),
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
        self.bend_range = self.style.bend_range;
        self.rpn = [RPN_NULL; 16];
        for i in 0..self.style.init.len() {
            let m = self.style.init.get(i);
            if m[0] == 0xF0 || m.len() > 3 {
                sink.send(m);
                continue;
            }
            let mut b = [0u8; 3];
            b[..m.len()].copy_from_slice(m);
            let m = &b[..m.len()];
            let ch = m[0] & 0x0F;
            if m[0] & 0xF0 == 0xE0 && m.len() == 3 && follows_chords(ch) {
                self.pat_bend[ch as usize] = (m[2] as u16) << 7 | m[1] as u16;
                self.send_bend(ch, sink);
            } else {
                self.mirror.send(sink, m);
            }
        }
        for p in 0..8u8 {
            self.mirror.send(sink, &[0xB0 | (8 + p), 7, self.mixer[p as usize]]);
        }
        self.pattern_pc = 0;
        self.sync_rpn();
    }

    /// The RPN each channel has selected, as far as a pattern's data entry is concerned.
    fn sync_rpn(&mut self) {
        for ch in 0..16 {
            self.rpn[ch] = if self.mirror.nrpn_on & (1 << ch) != 0 { RPN_NULL } else { self.mirror.rpn[ch] };
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

    /// A section change puts the style's part setup (SInt) back, so a voice or controller
    /// a pattern changed does not carry into the next section. It sends only what differs
    /// from what the channel has (`Mirror`): a section change after sections that changed
    /// nothing sends nothing, as on a Genos, where changing section never reloads a voice
    /// or resets a part. Resending it all cost every boundary 150-200 messages (up to
    /// 700 bytes, with a program change per part and the drum setup SysEx) queued ahead of
    /// the new section's first notes.
    ///
    /// A program change goes out only for a voice that differs, and not on the channels in
    /// `own_voice`, whose new section sets its own voice by its entry point. The part's XG
    /// parameters follow a program change sent (it resets them on an XG receiver), and the
    /// drum setup SysEx goes again only after one (a program change resets it), or when a
    /// pattern's own program change reset them since (`pattern_pc`), even if that pattern
    /// has since gone back to the setup's voice and no program change is needed. The effect
    /// SysEx is never re-sent: no pattern changes it, and an XG receiver would cut the
    /// reverb and delay tails. The part levels (CC7) are the faders: like a pattern CC7,
    /// the SInt's moves only the faders the player has not moved.
    fn reapply_init(&mut self, own_voice: u16, sink: &mut impl Sink) {
        self.restore_untouched_levels();
        self.bend_range = self.style.bend_range;
        let mut voice_sent = 0u16;
        // A pattern's program change reset these parts' XG parameters and drum setup on the
        // receiver, even if it went back to the setup's voice (no program change here). Not
        // on the parts whose new section sets its own voice: the setup's parameters belong
        // to the setup's voice. They are put back at the first section change that doesn't.
        let reset = self.pattern_pc & !own_voice;
        let kits = (0..16).filter(|&c| self.style.kit[c]).fold(0u16, |m, c| m | 1 << c);
        // The (N)RPN the setup selects on each channel as it goes, and the channels where it
        // selects one.
        let (mut rpn, mut nrpn, mut nrpn_on) = ([RPN_NULL; 16], [RPN_NULL; 16], 0u16);
        let mut selects = 0u16;
        for i in 0..self.style.init_resend {
            let m = self.style.init.get(i);
            if m[0] == 0xF0 {
                // XG Multi Part parameter (08 pp): after that part's program change only.
                let send = match *m {
                    [0xF0, 0x43, d, 0x4C, 0x08, part, ..] if d & 0xF0 == 0x10 => (voice_sent | reset) & (1 << (part & 15)) != 0,
                    _ if crate::sff::is_drum_setup(m) => voice_sent != 0 || reset & kits != 0,
                    _ => false,
                };
                if send {
                    sink.send(m);
                }
                continue;
            }
            if m.len() > 3 {
                continue;
            }
            let mut b = [0u8; 3];
            b[..m.len()].copy_from_slice(m);
            let m = &b[..m.len()];
            let ch = (m[0] & 0x0F) as usize;
            let bit = 1u16 << ch;
            match (m[0] & 0xF0, m.len()) {
                (0xB0, 3) => match m[1] {
                    101 | 100 => {
                        rpn[ch] = select_rpn(rpn[ch], m[1], m[2]);
                        nrpn_on &= !bit;
                        selects |= bit;
                    }
                    99 | 98 => {
                        nrpn[ch] = if m[1] == 99 { (nrpn[ch] & 0x7F) | (m[2] as u16) << 7 } else { (nrpn[ch] & !0x7F) | m[2] as u16 };
                        nrpn_on |= bit;
                        selects |= bit;
                    }
                    6 | 38 => {
                        let sel = if nrpn_on & bit != 0 {
                            (nrpn[ch] != RPN_NULL).then_some(NRPN_BIT | nrpn[ch])
                        } else {
                            (rpn[ch] != RPN_NULL).then_some(rpn[ch])
                        };
                        let Some(p) = sel else { continue };
                        let have = self.mirror.param(ch, p);
                        let same = have.is_some_and(|(msb, lsb)| if m[1] == 6 { msb == m[2] } else { lsb == m[2] });
                        if !same {
                            if self.mirror.selected(ch) != Some(p) {
                                self.mirror.select(sink, ch as u8, Some(p));
                            }
                            self.mirror.send(sink, m);
                        }
                    }
                    cc => {
                        if self.mirror.cc[ch][cc as usize] != m[2] {
                            self.mirror.send(sink, m);
                        }
                    }
                },
                (0xC0, _) => {
                    let want = (self.mirror.cc[ch][0], self.mirror.cc[ch][32], m[1]);
                    if own_voice & bit == 0 && self.mirror.voice[ch] != Some(want) {
                        self.mirror.send(sink, m);
                        voice_sent |= bit;
                    }
                }
                (0xE0, 3) => {
                    let v = (m[2] as u16) << 7 | m[1] as u16;
                    if follows_chords(ch as u8) {
                        if self.pat_bend[ch] != v {
                            self.pat_bend[ch] = v;
                            self.send_bend(ch as u8, sink);
                        }
                    } else if self.mirror.bend[ch] != Some(v) {
                        self.mirror.send(sink, m);
                    }
                }
                _ => {}
            }
        }
        // Leave each channel with the parameter the setup leaves selected (the null RPN).
        for ch in 0..16 {
            if selects & (1 << ch) == 0 {
                continue;
            }
            let want = if nrpn_on & (1 << ch) != 0 {
                (nrpn[ch] != RPN_NULL).then_some(NRPN_BIT | nrpn[ch])
            } else {
                (rpn[ch] != RPN_NULL).then_some(rpn[ch])
            };
            if self.mirror.selected(ch) != want {
                self.mirror.select(sink, ch as u8, want);
            }
        }
        self.pattern_pc &= own_voice;
        self.sync_rpn();
        for p in 0..8u8 {
            let v = self.mixer[p as usize];
            if self.user_set & (1 << p) == 0 && self.mirror.cc[8 + p as usize][7] != v {
                self.mirror.send(sink, &[0xB0 | (8 + p), 7, v]);
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
            self.mirror.send(sink, &[0xB0 | ch, 7, val]);
            return;
        }
        let p = (ch - 8) as usize;
        if self.user_set & (1 << p) != 0 {
            return;
        }
        self.set_mixer(p, val);
        self.mirror.send(sink, &[0xB0 | ch, 7, val]);
    }

    /// A part fader (0..8) moved to `value`: sent as that part's CC7, unchanged.
    pub fn set_volume(&mut self, part: u8, value: u8, sink: &mut impl Sink) {
        let p = (part & 7) as usize;
        let v = value.min(127);
        self.mixer[p] = v;
        self.user_set |= 1 << p;
        self.mirror.send(sink, &[0xB0 | (8 + p as u8), 7, v]);
    }

    /// A part's volume set from software (the app's mixer): as a fader move, but the
    /// hardware fader has to pick the new value up before it takes control again.
    pub fn set_volume_from_software(&mut self, part: u8, value: u8, sink: &mut impl Sink) {
        let p = (part & 7) as usize;
        self.set_volume(part, value, sink);
        self.takeover[p].software_moved(self.mixer[p]);
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

    /// The Launchkey faders now control the Style parts again (fader page switch), from
    /// their physical positions `hw` (`HW_UNKNOWN` = never moved): each picks its part up
    /// only once it reaches the part's level, as after any software move.
    pub fn faders_at(&mut self, hw: [u8; 8]) {
        for ((t, &h), &v) in self.takeover.iter_mut().zip(&hw).zip(&self.mixer) {
            *t = Takeover::at(h, v);
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
        self.notes_off(true, sink);
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
        let change = next != self.cur;
        self.cur = next;
        self.sec_start = start;
        self.seek(at - start);
        // A section repeating itself is not a change: its own pattern carries on.
        if change {
            let own_voice = self.own_voice(at - start);
            self.reapply_init(own_voice, sink);
        }
        self.chase(sink);
        let _ = now;
    }

    /// Channels whose current section sends a program change up to section tick `entry`
    /// (inclusive): at the section change they take that voice, not the SInt's.
    fn own_voice(&self, entry: f64) -> u16 {
        let sec = self.style.sections[self.cur].as_ref().unwrap();
        let mut m = 0u16;
        for e in sec.events.iter().take_while(|e| e.tick as f64 <= entry + 1e-6) {
            if let (PKind::Pc { .. }, Some(r)) = (e.kind, sec.rules[e.src as usize & 15].as_ref()) {
                m |= 1 << (r.dest_ch & 15);
            }
        }
        m
    }

    /// A section entered mid-bar (a Fill or Break at the next beat) skips its events before
    /// the entry point. Its notes stay skipped, but its voice and controllers are brought to
    /// where the pattern has them at the entry, so the part plays the section's voice from
    /// there on. Bank selects, program changes and (N)RPN messages go in order; for the
    /// other controllers and the pitch bend only the value in effect at the entry matters,
    /// so each goes once, and only if it differs from what the channel has: an expression
    /// swell or a bend before the entry is not replayed as a burst.
    fn chase(&mut self, sink: &mut impl Sink) {
        if self.ev_idx == 0 {
            return;
        }
        let mut last_cc = [[UNSENT; 128]; 16];
        let mut last_bend: [Option<(u8, u8)>; 16] = [None; 16];
        for i in 0..self.ev_idx {
            let sec = self.style.sections[self.cur].as_ref().unwrap();
            let e = sec.events[i];
            let Some(dest) = sec.rules[e.src as usize].as_ref().map(|r| r.dest_ch) else { continue };
            let d = dest as usize & 15;
            match e.kind {
                PKind::Cc { cc: cc @ (0 | 32), val } => {
                    if self.mirror.cc[d][cc as usize] != val {
                        self.mirror.send(sink, &[0xB0 | dest, cc, val]);
                    }
                }
                PKind::Cc { cc: 6 | 38 | 96..=101, .. } => self.emit_control(dest, e.kind, sink),
                PKind::Cc { cc, val } => last_cc[d][cc as usize & 127] = val,
                PKind::Pc { prog } => {
                    let want = (self.mirror.cc[d][0], self.mirror.cc[d][32], prog);
                    if self.mirror.voice[d] != Some(want) {
                        self.mirror.send(sink, &[0xC0 | dest, prog]);
                        self.pattern_pc |= 1 << d;
                    }
                }
                PKind::Bend { lo, hi } => last_bend[d] = Some((lo, hi)),
                PKind::On { .. } | PKind::Off { .. } => {}
            }
        }
        for d in 0..16u8 {
            for cc in 0..128u8 {
                let val = last_cc[d as usize][cc as usize];
                if val == UNSENT {
                    continue;
                }
                if cc == 7 && (8..16).contains(&d) {
                    let p = d as usize - 8;
                    if self.user_set & (1 << p) == 0 && (self.mixer[p] != val || self.mirror.cc[d as usize][7] != val) {
                        self.pattern_volume(d, val, sink);
                    }
                } else if self.mirror.cc[d as usize][cc as usize] != val {
                    self.mirror.send(sink, &[0xB0 | d, cc, val]);
                }
            }
            if let Some((lo, hi)) = last_bend[d as usize] {
                let v = (hi as u16) << 7 | lo as u16;
                if follows_chords(d) {
                    if self.pat_bend[d as usize] != v {
                        self.pat_bend[d as usize] = v;
                        self.send_bend(d, sink);
                    }
                } else if self.mirror.bend[d as usize] != Some(v) {
                    self.mirror.send(sink, &[0xE0 | d, lo, hi]);
                }
            }
        }
    }

    /// A pattern's controller, program change or pitch bend on `dest`; notes are not
    /// controls and send nothing here. A program change for the voice the part already
    /// has (a pattern restating its voice on its first beat) is not sent: a receiver would
    /// reload the voice for nothing.
    fn emit_control(&mut self, dest: u8, kind: PKind, sink: &mut impl Sink) {
        match kind {
            PKind::Cc { cc: 7, val } => self.pattern_volume(dest, val, sink),
            PKind::Cc { cc: cc @ (6 | 98..=101), val } if follows_chords(dest) => self.pattern_rpn(dest, cc, val, sink),
            PKind::Cc { cc, val } => self.mirror.send(sink, &[0xB0 | dest, cc, val]),
            PKind::Pc { prog } => {
                let d = dest as usize & 15;
                if self.mirror.voice[d] != Some((self.mirror.cc[d][0], self.mirror.cc[d][32], prog)) {
                    self.mirror.send(sink, &[0xC0 | dest, prog]);
                    self.pattern_pc |= 1 << d;
                }
            }
            PKind::Bend { lo, hi } if follows_chords(dest) => {
                self.pat_bend[dest as usize] = (hi as u16) << 7 | lo as u16;
                self.send_bend(dest, sink);
            }
            PKind::Bend { lo, hi } => self.mirror.send(sink, &[0xE0 | dest, lo, hi]),
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
        let pitch = self.master(dest, out);
        // A part a pitch shift left bent is straightened once it has fallen silent.
        if self.rtr_bend[dest as usize & 15] != 0 && !self.sounding.iter().any(|s| s.active && s.dest == dest) {
            self.set_rtr_bend(dest, 0, sink);
        }
        let guitar = self.is_guitar(slot, src, src_key);
        // On a part still bent, the key goes out that much lower so it sounds at `pitch`.
        // A guitar noise key (MegaVoice) is not a pitch: moving it would pick another
        // noise, so it goes out as it is (and sounds with the part's bend).
        let out = if guitar && src_key >= GUITAR_NOISE { pitch } else { shift_key(pitch, -self.rtr_bend[dest as usize & 15]) };
        // Two voices landing on one key at the same instant (a chord that folds voices
        // together, such as 1+8, or a pattern note on the key a chord change just
        // retriggered) sound once: stealing would leave a zero-length note. The later
        // voice is kept muted beside the sounding one, and the key sounds until both
        // have ended. (Rhythm parts play as written: a doubled hit stays doubled.)
        // A guitar strum spreads its strings over a few ticks, so for a Guitar note the
        // "instant" is the strum: a string on a key another string struck within a 32nd
        // note joins it, muted, instead of cutting it short and striking the pitch again.
        // A later strum strikes it again, as a guitarist would.
        let strum = (self.ns_per_tick * self.style.ppq as f64 / 8.0) as u64;
        let same_instant = |s: &Sounding| {
            s.active && s.dest == dest && s.out == out && (s.attack_ns == now || guitar && now.saturating_sub(s.attack_ns) < strum)
        };
        let muted = follows_chords(dest) && self.sounding.iter().any(same_instant);
        if !muted {
            // Steal an identical sounding note on the same channel so offs stay balanced.
            self.off_where(sink, |s| s.dest == dest && s.out == out);
        }
        if let Some(free) = self.sounding.iter_mut().find(|s| !s.active) {
            *free = Sounding { active: true, src, src_key, dest, out, vel, slot, started_ns: now, attack_ns: now, muted };
            if !muted {
                sink.send(&[0x90 | dest, out, vel]);
            }
        }
    }

    /// Send a part's pitch bend: the pattern's own bend, rescaled from the style's bend
    /// range to the part's output range, plus the Retrigger Rule pitch shift.
    fn send_bend(&mut self, ch: u8, sink: &mut impl Sink) {
        let c = ch as usize & 15;
        let (style, out) = (self.bend_range[c] as f32, self.out_range(ch) as f32);
        let centre = BEND_CENTRE as f32;
        let pat = (self.pat_bend[c] as f32 - centre) * style / out;
        let rtr = self.rtr_bend[c] as f32 * centre / out;
        let v = (centre + pat + rtr).round();
        // (A full bend up, centre + 8192, is sent as 16383, 0.15 cents short at most.)
        #[cfg(test)]
        if !(0.0..=16384.0).contains(&v) {
            self.bend_clamps.set(self.bend_clamps.get() + 1);
        }
        let v = v.clamp(0.0, 16383.0) as u16;
        self.mirror.send(sink, &[0xE0 | ch, (v & 0x7F) as u8, (v >> 7) as u8]);
    }

    /// The pitch bend range part `ch` has on the output.
    #[inline]
    fn out_range(&self, ch: u8) -> u8 {
        out_bend_range(self.bend_range[ch as usize & 15], self.style.pat_bend_max[ch as usize & 15])
    }

    /// Bend every note on `ch` by `semis` (the Retrigger Rule pitch shift).
    fn set_rtr_bend(&mut self, ch: u8, semis: i8, sink: &mut impl Sink) {
        let c = ch as usize & 15;
        if self.rtr_bend[c] != semis {
            self.rtr_bend[c] = semis;
            self.send_bend(ch, sink);
            sink.retune(ch, semis);
        }
    }

    /// (N)RPN messages from a pattern on a part that follows chords. A pitch bend range
    /// (RPN 0) becomes the style's range the part's bends are rescaled from; the part
    /// itself gets `out_range` of it.
    fn pattern_rpn(&mut self, ch: u8, cc: u8, val: u8, sink: &mut impl Sink) {
        let c = ch as usize & 15;
        if cc == 6 && self.rpn[c] == 0 {
            self.bend_range[c] = val;
            let range = self.out_range(ch);
            self.mirror.send(sink, &[0xB0 | ch, 6, range]);
            self.send_bend(ch, sink);
            return;
        }
        self.rpn[c] = select_rpn(self.rpn[c], cc, val);
        self.mirror.send(sink, &[0xB0 | ch, cc, val]);
    }

    /// Pattern events due at `now`, or within `EARLY_CHORD_NS` of it, that `process` has
    /// not played yet: they run from `ev_idx` to the index returned. None when the section
    /// itself ends by then: the all-off at its boundary cuts every note.
    fn due_now(&self, now: u64) -> Option<usize> {
        self.due_within(now, EARLY_CHORD_NS)
    }

    /// `due_now` with a window of `window` ns.
    fn due_within(&self, now: u64, window: u64) -> Option<usize> {
        let sec = self.style.sections[self.cur].as_ref()?;
        let target = self.tick_at(now + window) + 1e-6;
        let sec_end = self.sec_start + sec.len as f64;
        let (boundary, inclusive) = match self.queued {
            Some(q) if q.at < sec_end => (q.at, false),
            _ => (sec_end, true),
        };
        if boundary <= target {
            return None;
        }
        let mut k = self.ev_idx;
        while let Some(e) = sec.events.get(k) {
            let t = self.sec_start + e.tick as f64;
            let before = if inclusive { t <= boundary + 1e-6 } else { t < boundary - 1e-6 };
            if !before || t > target {
                break;
            }
            k += 1;
        }
        Some(k)
    }

    /// The pattern releases source key `key` on `src` among the events due now (`due`
    /// from `due_now`): a note started or retriggered for it now would be a blip.
    fn ends_now(&self, src: u8, key: u8, due: usize) -> bool {
        let Some(sec) = self.style.sections[self.cur].as_ref() else { return true };
        let due = due.min(sec.events.len());
        sec.events[self.ev_idx.min(due)..due]
            .iter()
            .any(|e| e.src == src && matches!(e.kind, PKind::Off { key: k } if k == key))
    }

    /// The pattern strikes `pitch` on part `dest` under `chord` among the events due now
    /// (`due` from `due_now`): a note started or retriggered at that pitch now would be cut
    /// short by that attack.
    fn struck_now(&self, dest: u8, pitch: u8, chord: Chord, due: usize) -> bool {
        let Some(sec) = self.style.sections[self.cur].as_ref() else { return false };
        if self.parts & (1 << (dest.saturating_sub(8) & 7)) == 0 {
            return false;
        }
        let due = due.min(sec.events.len());
        let mut i = self.ev_idx.min(due);
        while i < due {
            let e = sec.events[i];
            // Group simultaneous note-ons on this source channel, as `emit_at_index` does.
            let mut keys = [0u8; 8];
            let mut n = 0;
            let mut j = i;
            while let Some(g) = sec.events.get(j) {
                match g.kind {
                    PKind::On { key, .. } if g.tick == e.tick && g.src == e.src && n < 8 => {
                        keys[n] = key;
                        n += 1;
                        j += 1;
                    }
                    _ => break,
                }
            }
            i = j.max(i + 1);
            if n == 0 {
                continue;
            }
            let Some(rule) = sec.rules[e.src as usize].as_ref().filter(|r| r.dest_ch == dest) else { continue };
            let Some(c) = effective_chord(Some(chord), rule).filter(|&c| plays(rule, c)) else { continue };
            let mut outs = [None; 8];
            transpose_group(&keys[..n], rule, c, &mut outs[..n]);
            if outs[..n].iter().flatten().any(|&o| self.master(dest, o) == pitch) {
                return true;
            }
        }
        false
    }

    /// End the voices `f` picks. A muted voice ends silently; a sounding one hands its key
    /// to a muted voice that shares it and sounds on, if there is one.
    /// Does this source note follow the chord as a guitar string (NTR Guitar)?
    fn is_guitar(&self, slot: u8, src: u8, src_key: u8) -> bool {
        let rule = self.style.sections.get(slot as usize).and_then(|s| s.as_ref()).and_then(|s| s.rules.get(src as usize));
        rule.and_then(|r| r.as_ref()).is_some_and(|r| r.zone_for(src_key).ntr == Ntr::Guitar)
    }

    /// Tests: pairs of sounding (not muted) guitar strings of one part on the same key.
    #[cfg(test)]
    pub fn guitar_unisons(&self) -> usize {
        let s = &self.sounding;
        let live = |i: usize| s[i].active && !s[i].muted && self.is_guitar(s[i].slot, s[i].src, s[i].src_key);
        (0..MAX_SOUNDING)
            .filter(|&i| live(i))
            .map(|i| (i + 1..MAX_SOUNDING).filter(|&j| live(j) && (s[j].dest, s[j].out) == (s[i].dest, s[i].out)).count())
            .sum()
    }

    /// Tests: the guitar strings of one part struck at or after `since`, muted twins
    /// included, as (source key, sounding pitch).
    #[cfg(test)]
    pub fn guitar_strings(&self, dest: u8, since: u64) -> Vec<(u8, u8)> {
        let bend = self.rtr_bend[dest as usize & 15];
        let mut v: Vec<_> = self
            .sounding
            .iter()
            .filter(|s| s.active && s.dest == dest && s.started_ns >= since && self.is_guitar(s.slot, s.src, s.src_key))
            .map(|s| (s.src_key, if s.src_key >= GUITAR_NOISE { s.out } else { shift_key(s.out, bend) }))
            .collect();
        v.sort();
        v
    }

    fn off_where(&mut self, sink: &mut impl Sink, f: impl Fn(&Sounding) -> bool) {
        for i in 0..MAX_SOUNDING {
            let s = self.sounding[i];
            if !s.active || !f(&s) {
                continue;
            }
            self.sounding[i].active = false;
            if s.muted {
                continue;
            }
            let twin = |o: &Sounding| o.active && o.muted && o.dest == s.dest && o.out == s.out && !f(o);
            match self.sounding.iter_mut().find(|o| twin(o)) {
                Some(o) => o.muted = false,
                None => sink.send(&[0x80 | s.dest, s.out, 0]),
            }
        }
    }

    fn all_off(&mut self, sink: &mut impl Sink) {
        self.notes_off(false, sink);
    }

    /// End every note. Patterns bend (bass slides etc.); leaving a section or style
    /// mid-bend would leave the part detuned, so bends are re-centred and wheels and pedal
    /// cleared on every part. At a section change (`changed_only`) only on the parts where
    /// they are not already there, so the new section's first notes are not queued behind
    /// 24 messages that change nothing.
    fn notes_off(&mut self, changed_only: bool, sink: &mut impl Sink) {
        self.off_where(sink, |_| true);
        for ch in 8..16u8 {
            let c = ch as usize;
            if !changed_only || self.mirror.bend[c] != Some(BEND_CENTRE) {
                self.mirror.send(sink, &[0xE0 | ch, 0x00, 0x40]);
            }
            for cc in [1u8, 64] {
                if !changed_only || self.mirror.cc[c][cc as usize] != 0 {
                    self.mirror.send(sink, &[0xB0 | ch, cc, 0]);
                }
            }
        }
        self.pat_bend = [BEND_CENTRE; 16];
        for ch in 0..16u8 {
            if self.rtr_bend[ch as usize] != 0 {
                self.rtr_bend[ch as usize] = 0;
                sink.retune(ch, 0);
            }
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
            // A part that was playing has its notes re-voiced by `revoice`, except guitar
            // strings the previous chord left out (muted by Stroke, or voiced nowhere):
            // those have no voice to re-pitch, so they come in here.
            let guitar = (0..n).any(|k| rule.zone_for(keys[k]).ntr == Ntr::Guitar);
            if n == 0 || !part_on || was.is_some() && !guitar {
                continue;
            }
            let mut outs = [None; 8];
            transpose_group(&keys[..n], rule, now_chord, &mut outs[..n]);
            if was.is_some() {
                let slot = self.cur as u8;
                for k in 0..n {
                    let voiced = rule.zone_for(keys[k]).ntr == Ntr::Guitar
                        && self.sounding.iter().any(|s| s.active && s.src == e.src && s.slot == slot && s.src_key == keys[k]);
                    if voiced || rule.zone_for(keys[k]).ntr != Ntr::Guitar {
                        outs[k] = None;
                    }
                }
            }
            // A note is started only if no more of it is lost than is still to come: not if
            // the section boundary, its own note-off or a new attack on its key ends it
            // sooner than it should have started ago.
            let missed = now.saturating_sub(self.ns_at(self.sec_start + e.tick as f64));
            let Some(due) = self.due_within(now, missed) else { continue };
            for k in 0..n {
                let released = sec.events[j..end]
                    .iter()
                    .any(|o| o.src == e.src && matches!(o.kind, PKind::Off { key } if key == keys[k]))
                    || self.ends_now(e.src, keys[k], due)
                    || outs[k].is_some_and(|o| self.struck_now(rule.dest_ch, self.master(rule.dest_ch, o), chord, due));
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
        // At a section boundary the all-off cuts every note now: nothing to re-pitch.
        let Some(due) = self.due_now(now) else { return };
        for dest in 8..16u8 {
            if follows_chords(dest) {
                self.revoice_part(dest, chord, now, due, sink);
            }
        }
    }

    /// Revoice the notes sounding on part `dest` (RM p.31, RTR). Pitch Shift bends a note
    /// with no new attack, but pitch bend is per channel, so every note on the part moves
    /// by the same bend: the one that serves the most of the part's continuing notes (a
    /// note that keeps its pitch counts for no change). A Pitch Shift note that needs a
    /// different shift is retriggered at its new pitch instead, and so is a held note the
    /// bend would detune. Two voices that land on one key sound once (a second note-on for
    /// a sounding key would be cut by the first note-off); the second is kept muted, so the
    /// next chord can part them again. A note whose pattern note-off is
    /// due now or within `EARLY_CHORD_NS` is never attacked again: where it would be
    /// retriggered it plays out as it is, or stops if the part's bend moves.
    fn revoice_part(&mut self, dest: u8, chord: Chord, now: u64, due: usize, sink: &mut impl Sink) {
        let bend = self.rtr_bend[dest as usize & 15] as i16;
        // The part's sounding notes (indices into `sounding`) and what happens to each.
        let mut notes = [0u8; MAX_SOUNDING];
        let mut plan = [Revoice::Leave; MAX_SOUNDING];
        // Its pattern note-off is due now (`due_now`).
        let mut ends = [false; MAX_SOUNDING];
        let mut m = 0;
        for (i, s) in self.sounding.iter().enumerate() {
            if s.active && s.dest == dest {
                notes[m] = i as u8;
                plan[m] = Revoice::Hold;
                m += 1;
            }
        }
        let mut done = [false; MAX_SOUNDING];
        for a in 0..m {
            let s = self.sounding[notes[a] as usize];
            if done[a] || s.src >= 16 {
                continue;
            }
            let Some(sec) = self.style.sections[s.slot as usize].as_ref() else { continue };
            let Some(rule) = sec.rules[s.src as usize].as_ref() else { continue };
            // Group notes that started together on this channel so Root Fixed voicings move as a unit.
            let mut grp = [0usize; 8];
            let mut keys = [0u8; 8];
            let mut n = 0;
            for b in a..m {
                let o = self.sounding[notes[b] as usize];
                if !done[b] && o.src == s.src && o.slot == s.slot && o.started_ns == s.started_ns && n < 8 {
                    done[b] = true;
                    grp[n] = b;
                    keys[n] = o.src_key;
                    n += 1;
                }
            }
            let chord = effective_chord(Some(chord), rule).filter(|&c| plays(rule, c));
            let mut outs = [None; 8];
            if let Some(c) = chord {
                transpose_group(&keys[..n], rule, c, &mut outs[..n]);
            }
            // The player's chord landed just after these notes started: correct them outright.
            let late = now.saturating_sub(s.started_ns) < LATE_CHORD_NS;
            for k in 0..n {
                let b = grp[k];
                let o = self.sounding[notes[b] as usize];
                ends[b] = o.slot as usize == self.cur && self.ends_now(o.src, o.src_key, due);
                let Some(c) = chord else {
                    plan[b] = Revoice::Cut;
                    continue;
                };
                let cur = o.out as i16 + bend;
                let to = |t: Option<u8>| t.map(|t| self.master(dest, t));
                let zone = rule.zone_for(o.src_key);
                // A guitar noise key is not a pitch: no chord moves it, and it has no say
                // in the part's bend.
                if zone.ntr == Ntr::Guitar && o.src_key >= GUITAR_NOISE {
                    plan[b] = Revoice::Leave;
                    continue;
                }
                // Nearest note, up or down, with the pitch class of the new root (the slash
                // bass on a Bass On channel, as `theory::transpose` has it), in the same
                // octave or the next.
                let to_root = || {
                    let bass_on = zone.bass_on || zone.ntt == Ntt::Bass;
                    let pc = self.master(dest, if bass_on { c.bass.unwrap_or(c.root) } else { c.root }) as i16;
                    let mut d = (pc - cur).rem_euclid(12);
                    if d > 6 {
                        d -= 12;
                    }
                    (cur + d).clamp(0, 127) as u8
                };
                let target = match (late, zone.rtr) {
                    (true, _) => to(outs[k]).map(Revoice::Retrigger),
                    (false, Rtr::Stop) => None,
                    (false, Rtr::PitchShift) => to(outs[k]).map(Revoice::Shift),
                    (false, Rtr::PitchShiftToRoot) => Some(Revoice::Shift(to_root())),
                    (false, Rtr::Retrigger | Rtr::NoteGenerator) => to(outs[k]).map(Revoice::Retrigger),
                    (false, Rtr::RetriggerToRoot) => Some(Revoice::Retrigger(to_root())),
                };
                plan[b] = match target {
                    None => Revoice::Cut,
                    Some(Revoice::Shift(p) | Revoice::Retrigger(p)) if p as i16 == cur => Revoice::Hold,
                    Some(t) => t,
                };
            }
        }

        // The shift the bend makes: the one most continuing notes need, within what the
        // output range leaves over the pattern's own widest bend. (A note about to end has
        // no say, and a muted voice none beside its sounding twin wanting the same shift:
        // one key, one vote.)
        let room = self.style.shift_room[dest as usize & 15] as i16;
        let want = |a: usize, plan: &[Revoice]| match plan[a] {
            _ if ends[a] => None,
            Revoice::Hold => Some(0),
            Revoice::Shift(p) => Some(p as i16 - (self.sounding[notes[a] as usize].out as i16 + bend)),
            _ => None,
        };
        let shift_of = |a: usize, plan: &[Revoice]| {
            let d = want(a, plan)?;
            let s = self.sounding[notes[a] as usize];
            let twin = |b: usize| {
                let o = self.sounding[notes[b] as usize];
                !o.muted && o.out == s.out && want(b, plan) == Some(d)
            };
            (!s.muted || !(0..m).any(twin)).then_some(d)
        };
        let mut continuing = false;
        let mut best: Option<(usize, i16)> = None;
        for a in 0..m {
            let Some(d) = shift_of(a, &plan) else { continue };
            continuing = true;
            if (bend + d).abs() > room {
                continue;
            }
            let votes = (0..m).filter(|&b| shift_of(b, &plan) == Some(d)).count();
            // Ties: the smaller shift, then upwards.
            let better = best.is_none_or(|(v, bd)| (votes, -d.abs(), d) > (v, -bd.abs(), bd));
            if better {
                best = Some((votes, d));
            }
        }
        // With nothing sounding on through the change, the bend stays until the part falls
        // silent (see `note_on`), so release tails keep their pitch.
        let new_bend = bend + best.filter(|_| continuing).map_or(0, |b| b.1);

        // Note-offs first, so a retriggered note never lands on a key that is still held.
        for a in 0..m {
            let i = notes[a] as usize;
            let cur = self.sounding[i].out as i16 + bend;
            plan[a] = match plan[a] {
                Revoice::Hold if new_bend != bend => Revoice::Retrigger(cur.clamp(0, 127) as u8),
                Revoice::Shift(p) if p as i16 - cur != new_bend - bend => Revoice::Retrigger(p),
                p => p,
            };
            // A note about to end is not attacked again for a moment: it plays out as it is,
            // or, rather than bent out of tune for its last moment, it stops here (unless it
            // started just now and would last no time).
            if ends[a] && matches!(plan[a], Revoice::Retrigger(_)) {
                plan[a] = if new_bend != bend && self.sounding[i].attack_ns != now { Revoice::Cut } else { Revoice::Leave };
            }
        }
        // A sounding voice that stops or moves hands its key to a muted voice that stays on it.
        let moves = |p: Revoice| matches!(p, Revoice::Cut | Revoice::Retrigger(_));
        for a in 0..m {
            let s = self.sounding[notes[a] as usize];
            if s.muted || !moves(plan[a]) {
                continue;
            }
            let twin = (0..m).find(|&b| {
                let o = self.sounding[notes[b] as usize];
                o.muted && o.out == s.out && !moves(plan[b])
            });
            if let Some(b) = twin {
                self.sounding[notes[a] as usize].muted = true;
                self.sounding[notes[b] as usize].muted = false;
            }
        }
        for a in 0..m {
            if moves(plan[a]) {
                let s = &mut self.sounding[notes[a] as usize];
                s.active = false;
                if !s.muted {
                    sink.send(&[0x80 | s.dest, s.out, 0]);
                }
            }
        }
        self.set_rtr_bend(dest, new_bend as i8, sink);
        for a in 0..m {
            let Revoice::Retrigger(p) = plan[a] else { continue };
            if self.struck_now(dest, p, chord, due) {
                continue; // the pattern strikes that pitch right now: the note ends here
            }
            let i = notes[a] as usize;
            let out = shift_key(p, -(new_bend as i8));
            let on_key = |b: usize| {
                let o = self.sounding[notes[b] as usize];
                o.active && o.out == out
            };
            // The key already sounds on and on: this voice joins it, muted.
            let muted = (0..m).any(|b| on_key(b) && plan[b] != Revoice::Leave);
            if !muted {
                // A note about to end on that key makes way.
                for &n in &notes[..m] {
                    let o = &mut self.sounding[n as usize];
                    if o.active && o.out == out {
                        o.active = false;
                        if !o.muted {
                            sink.send(&[0x80 | o.dest, o.out, 0]);
                        }
                    }
                }
            }
            let s = &mut self.sounding[i];
            s.active = true;
            s.out = out;
            s.muted = muted;
            if muted {
                continue;
            }
            s.attack_ns = now;
            sink.send(&[0x90 | s.dest, out, s.vel]);
            #[cfg(test)]
            self.retriggered.push((now, dest, out));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Data entry after an NRPN select (CC 99/98) is not a pitch bend range; selecting
    /// RPN 0 again makes it one.
    #[test]
    fn nrpn_deselects_the_rpn() {
        let rpn0 = |r: u16| select_rpn(select_rpn(r, 101, 0), 100, 0);
        let r = rpn0(RPN_NULL);
        assert_eq!(r, 0);
        let r = select_rpn(select_rpn(r, 99, 1), 98, 8);
        assert_ne!(r, 0);
        assert_eq!(select_rpn(r, 6, 24), r);
        assert_eq!(rpn0(r), 0);
    }

    /// The output range fits a full octave of pitch shift over the pattern's own widest
    /// bend, up to 24.
    #[test]
    fn output_bend_range_leaves_room_for_pattern_bends() {
        assert_eq!(out_bend_range(GM_BEND_RANGE, 0), 12);
        assert_eq!(out_bend_range(12, 1), 13);
        assert_eq!(out_bend_range(12, 12), 24);
        assert_eq!(out_bend_range(24, 24), 24);
        assert_eq!(out_bend_range(2, 20), 24);
    }
}

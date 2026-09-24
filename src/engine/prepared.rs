//! A style made ready to play: its sections as flat event lists with their channel
//! rules, its channel setup, voices and levels. Built on the control side; the engine
//! only reads it.

use super::*;
use crate::sff::Ev;

#[derive(Clone, Copy, Debug)]
pub(super) enum PKind {
    On { key: u8, vel: u8 },
    Off { key: u8 },
    Cc { cc: u8, val: u8 },
    Pc { prog: u8 },
    Bend { lo: u8, hi: u8 },
}

#[derive(Clone, Copy, Debug)]
pub(super) struct PEvent {
    pub(super) tick: u32,
    pub(super) src: u8,
    pub(super) kind: PKind,
}

pub struct PSection {
    pub id: SectionId,
    pub len: u32,
    pub(super) events: Vec<PEvent>,
    pub(super) rules: [Option<ChannelRule>; 16],
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
    /// Set by whoever hands the style to the engine (the session numbers each load), so
    /// it can tell from a snapshot (`Snapshot::style_tag`) when the engine switched to it.
    pub tag: u64,
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

    pub fn is_empty(&self) -> bool {
        self.ends.is_empty()
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
            tag: 0,
        }
    }

    pub(super) fn has(&self, slot: usize) -> bool {
        self.sections[slot].is_some()
    }

    /// Nearest existing section of the same kind (e.g. Main D -> Main C).
    pub(super) fn resolve(&self, slot: usize) -> Option<usize> {
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

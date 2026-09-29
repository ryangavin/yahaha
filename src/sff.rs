//! Yamaha style file (SFF1/SFF2) parser.
//!
//! A style file is a type-0 SMF followed by optional chunks (CASM, OTSc, FNRc, MHhd...).
//! Reference: Wierzba & Bedesem, "Style Files – Introduction and Details" v2.1.

use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;

// The shared style types live below sff (see `style_types`); these keep the old paths.
pub use crate::style_types::{ChannelRule, Ev, Ntr, Ntt, Rtr, SectionId, Zone};

// ---------------------------------------------------------------------------
// MIDI events
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct TimedEv {
    pub tick: u32,
    pub ev: Ev,
}

// ---------------------------------------------------------------------------
// CASM
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Cseg {
    pub sections: Vec<String>,
    pub rules: Vec<ChannelRule>,
}

// ---------------------------------------------------------------------------
// Style
// ---------------------------------------------------------------------------

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Section {
    pub id: SectionId,
    pub start: u32,
    pub len: u32,
    /// Events with ticks relative to section start.
    pub events: Vec<TimedEv>,
}

/// A section under a marker yahaha doesn't know. Ticks as in `Section`.
#[derive(Debug, Clone)]
pub struct OpaqueSection {
    pub name: String,
    pub start: u32,
    pub len: u32,
    pub events: Vec<TimedEv>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Timing {
    /// Microseconds per quarter note.
    Tempo(u32),
    /// Numerator, denominator.
    TimeSig(u8, u8),
}

/// A tempo or time-signature change inside a section: its marker name and the tick
/// relative to the section start.
#[derive(Debug, Clone, PartialEq)]
pub struct TimingChange {
    pub section: String,
    pub tick: u32,
    pub change: Timing,
}

/// One part of a One Touch Setting: Right 1, Right 2, Right 3 or Left.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct OtsPart {
    pub on: bool,
    /// Yamaha bank MSB, LSB, program.
    pub voice: Option<(u8, u8, u8)>,
    pub volume: u8,
    /// Octave shift (-2..=2).
    pub octave: i8,
    /// Pan, reverb, chorus and variation sends (CC10, CC91, CC93, CC94) as the track
    /// leaves them; None where it doesn't set one.
    pub fx: [Option<u8>; 4],
    /// The voice's filter, EG, vibrato and portamento, by `tone::TONE_CC`: cutoff,
    /// resonance, attack, decay, release, vibrato rate, depth and delay (relative, 64 = the
    /// voice's own), portamento switch and time. None where the track sets none.
    pub tone: [Option<u8>; crate::tone::TONE],
    /// Pitch bend range in semitones (RPN 0 coarse).
    pub bend_range: Option<u8>,
    /// The XG multi part parameters the track sets for this part (mono/poly, velocity
    /// sense, controller depths, EQ...).
    pub xg: XgParams,
    /// The XG insertion effect type (MSB, LSB) the track gives this part: its Insertion
    /// Effect block (`F0 43 1n 4C 03 nn 00 mm ll F7`), block nn being the part (0-3) unless
    /// the track assigns the block to another (`03 nn 0C pp`). None: the track sets none.
    pub insert: Option<(u8, u8)>,
}

/// XG multi part parameters, `F0 43 1n 4C hh pp nn vv F7` (hh = 08 or 0A, pp = the part):
/// (hh, nn, vv), one per parameter with the last value set, in the order first set.
/// Fixed size so an `Ots` stays `Copy` and a recall never allocates.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct XgParams {
    n: u8,
    items: [(u8, u8, u8); XG_MAX],
}

/// The most XG parameters `XgParams` keeps per part (the corpus OTS set 22).
pub const XG_MAX: usize = 32;

impl XgParams {
    /// Set a parameter (replacing its earlier value); false when it is new and there's no
    /// room left.
    pub fn set(&mut self, hh: u8, nn: u8, vv: u8) -> bool {
        let n = self.n as usize;
        if let Some(i) = self.items[..n].iter_mut().find(|i| (i.0, i.1) == (hh, nn)) {
            i.2 = vv;
            return true;
        }
        if n == XG_MAX {
            return false;
        }
        self.items[n] = (hh, nn, vv);
        self.n += 1;
        true
    }

    pub fn iter(&self) -> impl Iterator<Item = (u8, u8, u8)> + '_ {
        self.items[..self.n as usize].iter().copied()
    }

    pub fn len(&self) -> usize {
        self.n as usize
    }

    pub fn is_empty(&self) -> bool {
        self.n == 0
    }
}

/// An XG multi part parameter change: (hh, part, nn, vv) for `F0 43 1n 4C hh pp nn vv F7`
/// with hh = 08 (multi part) or 0A (its extension).
pub fn xg_multi_part(v: &[u8]) -> Option<(u8, u8, u8, u8)> {
    (v.len() == 9 && v[0] == 0xF0 && v[1] == 0x43 && v[2] & 0xF0 == 0x10 && v[3] == 0x4C && matches!(v[4], 0x08 | 0x0A) && v[8] == 0xF7)
        .then(|| (v[4], v[5], v[6], v[7]))
}

/// The Insertion Effect blocks (Genos: 1-28, numbered 0-27 in the SysEx).
const INSERT_BLOCKS: usize = 28;

/// An XG Insertion Effect block parameter change: (block, address, data) for
/// `F0 43 1n 4C 03 nn aa dd.. F7`.
fn xg_insert_block(v: &[u8]) -> Option<(u8, u8, &[u8])> {
    match v {
        [0xF0, 0x43, d, 0x4C, 0x03, nn, aa, data @ .., 0xF7] if d & 0xF0 == 0x10 && (*nn as usize) < INSERT_BLOCKS && !data.is_empty() => {
            Some((*nn, *aa, data))
        }
        _ => None,
    }
}

/// XG NRPN (MSB 1) to its `tone::TONE_CC` index: the OTS tracks write decay, release and
/// vibrato this way (and cutoff, resonance, attack could be).
fn tone_nrpn(lsb: u8) -> Option<usize> {
    let i = match lsb {
        0x20 => 0,
        0x21 => 1,
        0x63 => 2,
        0x64 => 3,
        0x66 => 4,
        0x08 => 5,
        0x09 => 6,
        0x0A => 7,
        _ => return None,
    };
    Some(i)
}

/// A One Touch Setting: the panel voices a style suggests for your hands.
/// Parts: 0-2 = Right 1-3 (MIDI ch 1-3 in the OTS track), 3 = Left (ch 4).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Ots {
    pub parts: [OtsPart; 4],
}

/// Parse an OTSc chunk: a sequence of MTrk tracks, one per setting. Voices are plain bank
/// select + program change on channels 1-4; part on/off and octave are Yamaha SysEx
/// `F0 43 73 01 50 08 <part> <param> <value> F7` (param 00 = on/off, 03 = octave, 0x40 centre).
/// Pan and the reverb/chorus/variation sends are plain CC10/91/93/94 on the part's channel (the last one
/// wins). So are the filter, EG, vibrato and portamento (`OtsPart::tone`), except that the
/// tracks write some of them as XG NRPNs (MSB 1) instead: both are read, the last wins, as a
/// receiver would take them. Pitch bend range is RPN 0; the XG part SysEx is kept per part,
/// and so is the XG Insertion Effect type (`OtsPart::insert`; the corpus tracks set blocks
/// 0-3, one per part, and assign none elsewhere).
/// Not read: fine tune (RPN 1, always centre in the corpus) and the Genos-own part SysEx
/// (`43 73 01 5x`) beyond on/off and octave.
pub fn parse_ots(data: &[u8]) -> Vec<Ots> {
    let mut out = Vec::new();
    let mut p = 0;
    while p + 8 <= data.len() && &data[p..p + 4] == b"MTrk" {
        let len = be32(&data[p + 4..p + 8]);
        let end = (p + 8 + len).min(data.len());
        let mut ots = Ots::default();
        let mut bank = [(0u8, 0u8); 4];
        // Per channel: the RPN (101, 100) and NRPN (99, 98) selected, and whether an NRPN
        // was selected last (data entry goes to it).
        let mut rpn = [(0x7Fu8, 0x7Fu8); 4];
        let mut nrpn = [(0x7Fu8, 0x7Fu8, false); 4];
        // The Insertion Effect blocks the track sets: type MSB, LSB and the part assigned.
        let mut blocks = [(None::<u8>, None::<u8>, None::<u8>); INSERT_BLOCKS];
        for part in ots.parts.iter_mut() {
            part.volume = 100;
        }
        if let Ok(evs) = parse_track(&data[p + 8..end]) {
            for e in evs {
                match e.ev {
                    Ev::Cc { ch, cc, val } if ch < 4 && crate::tone::TONE_CC.contains(&cc) => {
                        let i = crate::tone::TONE_CC.iter().position(|&c| c == cc).unwrap_or(0);
                        ots.parts[ch as usize].tone[i] = Some(val);
                    }
                    Ev::Cc { ch, cc, val } if ch < 4 && matches!(cc, 98..=101 | 6) => {
                        let c = ch as usize;
                        match cc {
                            101 => (rpn[c].0, nrpn[c].2) = (val, false),
                            100 => (rpn[c].1, nrpn[c].2) = (val, false),
                            99 => (nrpn[c].0, nrpn[c].2) = (val, true),
                            98 => (nrpn[c].1, nrpn[c].2) = (val, true),
                            _ if nrpn[c].2 => {
                                if let Some(i) = tone_nrpn(nrpn[c].1).filter(|_| nrpn[c].0 == 1) {
                                    ots.parts[c].tone[i] = Some(val);
                                }
                            }
                            _ if rpn[c] == (0, 0) => ots.parts[c].bend_range = Some(val),
                            _ => {}
                        }
                    }
                    Ev::Cc { ch, cc, val } if ch < 4 => match cc {
                        0 => bank[ch as usize].0 = val,
                        32 => bank[ch as usize].1 = val,
                        7 => ots.parts[ch as usize].volume = val,
                        10 => ots.parts[ch as usize].fx[0] = Some(val),
                        91 => ots.parts[ch as usize].fx[1] = Some(val),
                        93 => ots.parts[ch as usize].fx[2] = Some(val),
                        94 => ots.parts[ch as usize].fx[3] = Some(val),
                        _ => {}
                    },
                    Ev::Pc { ch, prog } if ch < 4 => {
                        let (m, l) = bank[ch as usize];
                        ots.parts[ch as usize].voice = Some((m, l, prog));
                    }
                    Ev::Sysex(ref v) if xg_multi_part(v).is_some_and(|x| x.1 < 4) => {
                        let (hh, pp, nn, vv) = xg_multi_part(v).unwrap_or_default();
                        ots.parts[pp as usize].xg.set(hh, nn, vv);
                    }
                    Ev::Sysex(ref v) if xg_insert_block(v).is_some() => {
                        let (nn, aa, data) = xg_insert_block(v).unwrap_or_default();
                        let b = &mut blocks[nn as usize];
                        for (k, &d) in data.iter().enumerate() {
                            match aa as usize + k {
                                0x00 => b.0 = Some(d),
                                0x01 => b.1 = Some(d),
                                0x0C => b.2 = Some(d),
                                _ => {}
                            }
                        }
                    }
                    Ev::Sysex(ref v) if v.len() == 10 && v[1..6] == [0x43, 0x73, 0x01, 0x50, 0x08] && v[6] < 4 => {
                        let part = &mut ots.parts[v[6] as usize];
                        match v[7] {
                            0x00 => part.on = v[8] >= 0x40,
                            0x03 => part.octave = (v[8] as i16 - 0x40).clamp(-2, 2) as i8,
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
        // Each part's insertion type: the first block on it (by block number) wins.
        for (nn, &(msb, lsb, assigned)) in blocks.iter().enumerate() {
            let part = assigned.map_or(nn, usize::from);
            if let (Some(msb), Some(p)) = (msb, ots.parts.get_mut(part))
                && p.insert.is_none()
            {
                p.insert = Some((msb, lsb.unwrap_or(0)));
            }
        }
        out.push(ots);
        p = end;
    }
    out
}

/// One MIDI channel's setup in a style's SInt. The named fields keep the last value the
/// file sets; `None` where it sets none.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChannelInit {
    /// The bank select in effect when the (last) program change arrives: the voice's bank.
    /// Without a program change, the last bank select.
    pub bank_msb: Option<u8>,
    pub bank_lsb: Option<u8>,
    pub program: Option<u8>,
    /// A bank select after the last program change. It selects no voice; it waits for the
    /// next program change (a pattern's), so it goes out after the program change.
    pub pending_msb: Option<u8>,
    pub pending_lsb: Option<u8>,
    /// CC7.
    pub volume: Option<u8>,
    /// CC10.
    pub pan: Option<u8>,
    /// CC91.
    pub reverb: Option<u8>,
    /// CC93.
    pub chorus: Option<u8>,
    /// XG Multi Part parameters for this channel's part (`F0 43 1n 4C 08 pp aa vv F7`,
    /// part pp = this channel): (address, value) in file order.
    pub xg_part: Vec<(u8, u8)>,
    /// Every other controller (expression, variation send, filter, RPN/NRPN sequences) and
    /// pitch bend, in file order: their order can matter.
    pub other: Vec<Ev>,
}

/// A style's channel setup (the SInt part): per-channel init, keyed by the channel the file
/// addresses (the source channel), plus the SysEx that isn't a part's.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SInt {
    pub channels: [ChannelInit; 16],
    /// XG effect, insertion effect and drum setup SysEx and any other SysEx, in file order,
    /// without the system resets (`is_reset`).
    pub sysex: Vec<Vec<u8>>,
}

/// GM/GM2 System On/Off, XG System On, XG All Parameter Reset or GS Reset: SysEx that
/// resets every channel of the receiver, not just the style's parts.
pub fn is_reset(v: &[u8]) -> bool {
    match v {
        [0xF0, 0x7E, _, 0x09, 0x01..=0x03, ..] => true,
        [0xF0, 0x43, d, 0x4C, 0x00, 0x00, 0x7E | 0x7F, 0x00, ..] => d & 0xF0 == 0x10,
        [0xF0, 0x41, _, 0x42, 0x12, 0x40, 0x00, 0x7F, 0x00, ..] => true,
        _ => false,
    }
}

/// An XG Multi Part parameter change with one data byte: (part, address, value).
pub fn xg_part_param(v: &[u8]) -> Option<(u8, u8, u8)> {
    match *v {
        [0xF0, 0x43, d, 0x4C, 0x08, part, addr, val, 0xF7] if d & 0xF0 == 0x10 && part < 16 => Some((part, addr, val)),
        _ => None,
    }
}

/// XG Drum Setup SysEx: a Drum Setup n parameter (`F0 43 1n 4C 3n rr pp vv F7`) or a
/// Drum Setup Reset (`F0 43 1n 4C 00 00 7D nn F7`). A program change on a part that uses
/// Drum Setup n initializes it (Data List, Drum Setup note).
pub fn is_drum_setup(v: &[u8]) -> bool {
    match *v {
        [0xF0, 0x43, d, 0x4C, 0x30 | 0x31, ..] | [0xF0, 0x43, d, 0x4C, 0x00, 0x00, 0x7D, ..] => d & 0xF0 == 0x10,
        _ => false,
    }
}

/// Where an XG effect block's part assignment keeps its part number, if `v` is one: the
/// Insertion Effect Part (`F0 43 1n 4C 03 nn 0C pp F7`) or the Variation Part
/// (`F0 43 1n 4C 02 01 5B pp F7`). The part is a source channel, so it needs routing.
pub fn xg_effect_part(v: &[u8]) -> Option<usize> {
    match *v {
        [0xF0, 0x43, d, 0x4C, 0x03, _, 0x0C, _, 0xF7] | [0xF0, 0x43, d, 0x4C, 0x02, 0x01, 0x5B, _, 0xF7]
            if d & 0xF0 == 0x10 =>
        {
            Some(7)
        }
        _ => None,
    }
}

impl SInt {
    pub fn parse(init: &[Ev]) -> SInt {
        let mut s = SInt::default();
        for ev in init {
            match *ev {
                Ev::Cc { ch, cc, val } => {
                    let c = &mut s.channels[ch as usize & 15];
                    let voiced = c.program.is_some();
                    match cc {
                        0 if voiced => c.pending_msb = Some(val),
                        0 => c.bank_msb = Some(val),
                        32 if voiced => c.pending_lsb = Some(val),
                        32 => c.bank_lsb = Some(val),
                        7 => c.volume = Some(val),
                        10 => c.pan = Some(val),
                        91 => c.reverb = Some(val),
                        93 => c.chorus = Some(val),
                        _ => c.other.push(ev.clone()),
                    }
                }
                Ev::Pc { ch, prog } => {
                    let c = &mut s.channels[ch as usize & 15];
                    if let Some(v) = c.pending_msb.take() {
                        c.bank_msb = Some(v);
                    }
                    if let Some(v) = c.pending_lsb.take() {
                        c.bank_lsb = Some(v);
                    }
                    c.program = Some(prog);
                }
                Ev::Bend { ch, .. } => s.channels[ch as usize & 15].other.push(ev.clone()),
                Ev::Sysex(ref v) if is_reset(v) => {}
                Ev::Sysex(ref v) => match xg_part_param(v) {
                    Some((part, addr, val)) => s.channels[part as usize].xg_part.push((addr, val)),
                    None => s.sysex.push(v.clone()),
                },
                _ => {}
            }
        }
        s
    }
}

#[derive(Debug, Clone)]
pub struct Style {
    pub name: String,
    pub format: String,
    pub ppq: u16,
    /// Microseconds per quarter note.
    pub tempo_us: u32,
    pub timesig: (u8, u8),
    /// Channel setup events from the SInt part (and anything before the first section).
    pub init: Vec<Ev>,
    pub sections: BTreeMap<SectionId, Section>,
    /// Sections under markers the Genos has no button for ("Fill In AB", "Fill In EE"): cut
    /// out so they don't run on at the end of the section before them, and never played.
    pub opaque_sections: Vec<OpaqueSection>,
    /// Tempo and time-signature changes after bar 1 (mostly Ending ritardandos). Kept, not
    /// played: the style's tempo and meter are those of bar 1 (`tempo_us`, `timesig`).
    pub timing_changes: Vec<TimingChange>,
    pub casm: Vec<Cseg>,
    /// One Touch Settings (up to 4).
    pub ots: Vec<Ots>,
    /// Raw trailing chunks we don't interpret (id, bytes).
    pub other_chunks: Vec<(String, Vec<u8>)>,
}

impl Style {
    pub fn bpm(&self) -> f64 {
        60_000_000.0 / self.tempo_us as f64
    }

    /// The channel setup, structured.
    pub fn sint(&self) -> SInt {
        SInt::parse(&self.init)
    }

    pub fn ticks_per_bar(&self) -> u32 {
        let (n, d) = self.timesig;
        ((self.ppq as u32 * 4 * n as u32) / d.max(1) as u32).max(1)
    }

    /// Channel rules for a section, keyed by source channel. Falls back to defaults
    /// for channels without a CASM entry.
    pub fn rules_for(&self, id: SectionId) -> BTreeMap<u8, ChannelRule> {
        let name = id.name();
        let mut out = BTreeMap::new();
        if let Some(seg) = self.casm.iter().find(|c| c.sections.iter().any(|s| *s == name)) {
            for r in &seg.rules {
                out.insert(r.src_ch, r.clone());
            }
        }
        out
    }

    pub fn has_casm(&self) -> bool {
        !self.casm.is_empty()
    }

    pub fn load(path: &std::path::Path) -> Result<Style> {
        let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        parse(&bytes).with_context(|| format!("parsing {}", path.display()))
    }
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

fn be32(b: &[u8]) -> usize {
    u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize
}

struct Reader<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> Reader<'a> {
    fn u8(&mut self) -> Result<u8> {
        let v = *self.b.get(self.p).context("unexpected end of track")?;
        self.p += 1;
        Ok(v)
    }
    fn vlq(&mut self) -> Result<u32> {
        let mut v: u32 = 0;
        for _ in 0..4 {
            let c = self.u8()?;
            v = (v << 7) | (c & 0x7F) as u32;
            if c & 0x80 == 0 {
                return Ok(v);
            }
        }
        bail!("bad variable-length quantity")
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        if self.p + n > self.b.len() {
            bail!("unexpected end of track");
        }
        let s = &self.b[self.p..self.p + n];
        self.p += n;
        Ok(s)
    }
}

pub(crate) fn parse_track(data: &[u8]) -> Result<Vec<TimedEv>> {
    let mut r = Reader { b: data, p: 0 };
    let mut out = Vec::new();
    let mut tick: u32 = 0;
    let mut running: u8 = 0;
    while r.p < data.len() {
        tick = tick.saturating_add(r.vlq()?);
        let mut status = r.u8()?;
        if status < 0x80 {
            if running == 0 {
                bail!("running status without prior status at byte {}", r.p);
            }
            r.p -= 1;
            status = running;
        }
        let ev = match status {
            0xF0 | 0xF7 => {
                running = 0;
                let len = r.vlq()? as usize;
                let body = r.take(len)?;
                let mut v = Vec::with_capacity(len + 1);
                if status == 0xF0 {
                    v.push(0xF0);
                }
                v.extend_from_slice(body);
                Ev::Sysex(v)
            }
            0xFF => {
                running = 0;
                let ty = r.u8()?;
                let len = r.vlq()? as usize;
                let d = r.take(len)?.to_vec();
                if ty == 0x2F {
                    out.push(TimedEv { tick, ev: Ev::Meta { ty, data: d } });
                    break;
                }
                Ev::Meta { ty, data: d }
            }
            0x80..=0xEF => {
                running = status;
                let ch = status & 0x0F;
                match status & 0xF0 {
                    0x80 => {
                        let key = r.u8()? & 0x7F;
                        let _ = r.u8()?;
                        Ev::NoteOff { ch, key }
                    }
                    0x90 => {
                        let key = r.u8()? & 0x7F;
                        let vel = r.u8()? & 0x7F;
                        if vel == 0 {
                            Ev::NoteOff { ch, key }
                        } else {
                            Ev::NoteOn { ch, key, vel }
                        }
                    }
                    0xA0 => Ev::PolyAt { ch, key: r.u8()? & 0x7F, val: r.u8()? & 0x7F },
                    0xB0 => Ev::Cc { ch, cc: r.u8()? & 0x7F, val: r.u8()? & 0x7F },
                    0xC0 => Ev::Pc { ch, prog: r.u8()? & 0x7F },
                    0xD0 => Ev::ChanAt { ch, val: r.u8()? & 0x7F },
                    _ => {
                        let lo = (r.u8()? & 0x7F) as u16;
                        let hi = (r.u8()? & 0x7F) as u16;
                        Ev::Bend { ch, val: (hi << 7) | lo }
                    }
                }
            }
            _ => bail!("unsupported status byte {status:#04x}"),
        };
        out.push(TimedEv { tick, ev });
    }
    Ok(out)
}

fn decode_ntt_new(v: u8, ntr: Ntr) -> Ntt {
    let t = v & 0x7F;
    if ntr == Ntr::Guitar {
        return match t {
            1 => Ntt::GuitarStroke,
            2 => Ntt::GuitarArpeggio,
            _ => Ntt::GuitarAllPurpose,
        };
    }
    match t {
        0 => Ntt::Bypass,
        1 => Ntt::Melody,
        2 => Ntt::Chord,
        3 => Ntt::MelodicMinor,
        4 => Ntt::MelodicMinor5,
        5 => Ntt::HarmonicMinor,
        6 => Ntt::HarmonicMinor5,
        7 => Ntt::NaturalMinor,
        8 => Ntt::NaturalMinor5,
        9 => Ntt::Dorian,
        10 => Ntt::Dorian5,
        _ => Ntt::Melody,
    }
}

/// SFF1 Ctab NTT byte -> (table, Bass On). Wierzba/Bedesem document 00H..05H only (Bypass,
/// Melody, Chord, Bass, Melodic Minor, Harmonic Minor; no corpus file uses anything else).
/// 06H..0AH have no Ctab meaning, so they take the only meaning those numbers have anywhere:
/// the Cntt/Ctb2 tables (Harmonic Minor 5th Var. .. Dorian 5th Var.), which never collide
/// with a Ctab code. Nothing documents a Bass On bit in a Ctab byte (Bass On is the "Bass"
/// code), so every other value, 80H..FFH included, is undefined and plays as Melody without
/// Bass On, like `decode_ntt_new`'s fallback.
fn decode_ntt_old(v: u8) -> (Ntt, bool) {
    let ntt = match v {
        0 => Ntt::Bypass,
        1 => Ntt::Melody,
        2 => Ntt::Chord,
        3 => Ntt::Bass,
        4 => Ntt::MelodicMinor,
        5 => Ntt::HarmonicMinor,
        t @ 6..=10 => decode_ntt_new(t, Ntr::RootTrans),
        _ => Ntt::Melody,
    };
    (ntt, ntt == Ntt::Bass)
}

fn decode_ntr(v: u8) -> Ntr {
    match v {
        1 => Ntr::RootFixed,
        2 => Ntr::Guitar,
        _ => Ntr::RootTrans,
    }
}

fn decode_rtr(v: u8) -> Rtr {
    match v {
        0 => Rtr::Stop,
        1 => Rtr::PitchShift,
        2 => Rtr::PitchShiftToRoot,
        3 => Rtr::Retrigger,
        4 => Rtr::RetriggerToRoot,
        _ => Rtr::NoteGenerator,
    }
}

fn parse_zone(b: &[u8]) -> Zone {
    let ntr = decode_ntr(b[0]);
    Zone {
        ntr,
        ntt: decode_ntt_new(b[1], ntr),
        high_key: b[2] % 12,
        lo: b[3] & 0x7F,
        hi: b[4] & 0x7F,
        rtr: decode_rtr(b[5]),
        bass_on: b[1] & 0x80 != 0,
    }
}

/// Source chord type ids run 0 (Maj) ..= 33 (sus2). Anything else (including 0x22 Cancel,
/// which is not a recordable source chord) falls back to the Style Creator default, Maj7,
/// so the transposer never sees a type it has no chord tones for.
fn source_chord_type(v: u8) -> u8 {
    if (v as usize) < crate::theory::NUM_TYPES {
        v
    } else {
        2
    }
}

fn parse_ctab(d: &[u8], sff2: bool) -> Result<ChannelRule> {
    if d.len() < 26 {
        bail!("Ctab record too short ({} bytes)", d.len());
    }
    let name = String::from_utf8_lossy(&d[1..9]).trim_end().to_string();
    let note_mute = u16::from_be_bytes([d[11], d[12]]) & 0x0FFF;
    let mut cm: u64 = 0;
    for &x in &d[13..18] {
        cm = (cm << 8) | x as u64;
    }
    let src_ch = d[0] & 0x0F;
    // Accompaniment parts live on channels 9-16 (0-based 8-15); 1-8 belong to the keyboard
    // and pad voices, so a malformed destination falls back to the source channel's part.
    let dest_ch = match d[9] & 0x0F {
        ch @ 8..=15 => ch,
        _ => src_ch | 0x08,
    };
    let mut rule = ChannelRule {
        src_ch,
        name,
        dest_ch,
        editable: d[10] == 0,
        note_mute,
        chord_mute: cm & ((1u64 << 34) - 1),
        autostart: cm & (1u64 << 34) != 0,
        src_root: d[18] % 12,
        src_type: source_chord_type(d[19]),
        mid_lo: 0,
        mid_hi: 127,
        zones: [Zone {
            ntr: Ntr::RootTrans,
            ntt: Ntt::Bypass,
            high_key: 0,
            lo: 0,
            hi: 127,
            rtr: Rtr::PitchShift,
            bass_on: false,
        }; 3],
        sff2,
    };
    if sff2 {
        if d.len() < 40 {
            bail!("Ctb2 record too short ({} bytes)", d.len());
        }
        rule.mid_lo = d[20];
        rule.mid_hi = d[21];
        rule.zones = [parse_zone(&d[22..28]), parse_zone(&d[28..34]), parse_zone(&d[34..40])];
    } else {
        let ntr = decode_ntr(d[20]);
        let (ntt, bass_on) = decode_ntt_old(d[21]);
        let z = Zone {
            ntr,
            ntt,
            high_key: d[22] % 12,
            lo: d[23] & 0x7F,
            hi: d[24] & 0x7F,
            rtr: decode_rtr(d[25]),
            bass_on,
        };
        rule.zones = [z; 3];
    }
    Ok(rule)
}

/// Iterate `id(4) len(4) data` records in a buffer.
fn records(buf: &[u8]) -> Result<Vec<(&[u8], &[u8])>> {
    let mut out = Vec::new();
    let mut p = 0;
    while p + 8 <= buf.len() {
        let id = &buf[p..p + 4];
        let len = be32(&buf[p + 4..p + 8]);
        let end = p + 8 + len;
        if end > buf.len() {
            bail!("record {} overruns its container", String::from_utf8_lossy(id));
        }
        out.push((id, &buf[p + 8..end]));
        p = end;
    }
    if p != buf.len() {
        bail!("{} trailing bytes after last record", buf.len() - p);
    }
    Ok(out)
}

pub(crate) fn parse_casm(data: &[u8]) -> Result<Vec<Cseg>> {
    let mut segs = Vec::new();
    for (id, cseg) in records(data)? {
        if id != b"CSEG" {
            bail!("expected CSEG, found {}", String::from_utf8_lossy(id));
        }
        let mut seg = Cseg { sections: vec![], rules: vec![] };
        for (rid, d) in records(cseg)? {
            match rid {
                b"Sdec" => {
                    seg.sections =
                        String::from_utf8_lossy(d).split(',').map(|s| s.trim().to_string()).collect()
                }
                b"Ctab" => seg.rules.push(parse_ctab(d, false)?),
                b"Ctb2" => seg.rules.push(parse_ctab(d, true)?),
                // Cntt refines an SFF1 Ctab's table with the ones Ctab cannot encode (5th Var.,
                // Natural Minor, Dorian); Wierzba/Bedesem: it overrides the Ctab NTT. It never
                // touches a Ctb2, which already holds per-zone NTT and Bass On (and no corpus
                // file mixes the two). Bass On is OR'd, not replaced: every corpus Cntt for a
                // Ctab "Bass" channel is 01H (Melody, bit 7 clear), so there the Ctab code, not
                // the Cntt bit, is what carries Bass On. (Read literally, the spec's Cntt bit 7
                // "Bass on/off" would switch those Bass parts off; we don't, see genos-features.)
                b"Cntt" => {
                    if d.len() >= 2 {
                        let ch = d[0] & 0x0F;
                        if let Some(r) = seg.rules.iter_mut().find(|r| r.src_ch == ch && !r.sff2) {
                            let ntt = decode_ntt_new(d[1], r.zones[1].ntr);
                            for z in r.zones.iter_mut() {
                                z.ntt = ntt;
                                // OR, not replace: the Ctab "Bass" code keeps Bass On (#14).
                                z.bass_on |= d[1] & 0x80 != 0;
                            }
                        }
                    }
                }
                other => bail!("unknown CSEG record {}", String::from_utf8_lossy(other)),
            }
        }
        segs.push(seg);
    }
    Ok(segs)
}

pub fn parse(bytes: &[u8]) -> Result<Style> {
    let (ppq, events, mut p) = parse_header_track(bytes)?;

    let mut casm = Vec::new();
    let mut other_chunks = Vec::new();
    while p + 8 <= bytes.len() {
        let id = String::from_utf8_lossy(&bytes[p..p + 4]).to_string();
        let len = be32(&bytes[p + 4..p + 8]);
        let end = (p + 8 + len).min(bytes.len());
        let data = &bytes[p + 8..end];
        if id == "CASM" {
            casm = parse_casm(data)?;
        } else {
            other_chunks.push((id, data.to_vec()));
        }
        p = end;
    }

    build_style(ppq, events, casm, other_chunks)
}

/// The MThd header and the style track: (ppq, events, offset just past the track).
fn parse_header_track(bytes: &[u8]) -> Result<(u16, Vec<TimedEv>, usize)> {
    if bytes.len() < 22 || &bytes[0..4] != b"MThd" {
        bail!("not a MIDI/style file");
    }
    let hlen = be32(&bytes[4..8]);
    if hlen < 6 || 8 + hlen + 8 > bytes.len() {
        bail!("bad MThd length {hlen}");
    }
    let h = &bytes[8..8 + hlen];
    let format = u16::from_be_bytes([h[0], h[1]]);
    let ntracks = u16::from_be_bytes([h[2], h[3]]);
    let ppq = u16::from_be_bytes([h[4], h[5]]);
    // Some styles carry a garbage track count; instruments only read the first MTrk.
    let _ = ntracks;
    if format != 0 {
        bail!("style MIDI must be type 0 (got type {format})");
    }
    if ppq & 0x8000 != 0 {
        bail!("SMPTE time division not supported");
    }
    if ppq == 0 {
        bail!("zero ticks per quarter note");
    }
    let p = 8 + hlen;
    if &bytes[p..p + 4] != b"MTrk" {
        bail!("missing MTrk");
    }
    let tlen = be32(&bytes[p + 4..p + 8]);
    let track = &bytes[p + 8..(p + 8 + tlen).min(bytes.len())];
    let events = parse_track(track)?;
    Ok((ppq, events, p + 8 + tlen))
}

/// Header facts of a style track: what `Style` and `Summary` take from the meta events.
struct Meta {
    name: String,
    format: String,
    tempo_us: u32,
    timesig: (u8, u8),
    /// Section markers in track order: every marker from the first known section on, so an
    /// unknown one ends the section before it. `Err` holds an unknown marker's name.
    marks: Vec<(u32, Result<SectionId, String>)>,
    /// Tempo and time-signature changes after the first section: (tick, index into `marks`
    /// of the section they fall in, change).
    timing: Vec<(u32, usize, Timing)>,
    end_tick: u32,
}

impl Meta {
    /// The known sections, in track order.
    fn known(&self) -> impl Iterator<Item = SectionId> + '_ {
        self.marks.iter().filter_map(|(_, id)| id.as_ref().ok().copied())
    }
}

fn scan_meta(events: &[TimedEv]) -> Meta {
    let mut m = Meta {
        name: String::new(),
        format: String::new(),
        tempo_us: 500_000,
        timesig: (4, 4),
        marks: Vec::new(),
        timing: Vec::new(),
        end_tick: 0,
    };
    for e in events {
        m.end_tick = m.end_tick.max(e.tick);
        if let Ev::Meta { ty, data } = &e.ev {
            // Tempo and time signature set the style only before the first section; later
            // ones are kept per section (see `Style::timing_changes`).
            let before_sections = m.marks.is_empty();
            let later = |m: &mut Meta, t: Timing| m.timing.push((e.tick, m.marks.len() - 1, t));
            match *ty {
                0x06 => {
                    let t = String::from_utf8_lossy(data).to_string();
                    if t == "SFF1" || t == "SFF2" {
                        m.format = t;
                    } else if let Some(id) = SectionId::parse(&t) {
                        m.marks.push((e.tick, Ok(id)));
                    } else if !before_sections && t != "SInt" {
                        m.marks.push((e.tick, Err(t.trim().to_string())));
                    }
                }
                // Names are often NUL-padded to a fixed width; the name ends at the first NUL.
                0x03 if m.name.is_empty() => {
                    let text = data.split(|&b| b == 0).next().unwrap_or_default();
                    m.name = String::from_utf8_lossy(text).trim().to_string()
                }
                0x51 if data.len() == 3 && data[..] != [0, 0, 0] => {
                    let us = ((data[0] as u32) << 16) | ((data[1] as u32) << 8) | data[2] as u32;
                    if before_sections {
                        m.tempo_us = us
                    } else {
                        later(&mut m, Timing::Tempo(us))
                    }
                }
                // Ignore impossible signatures (n/0, 2^8+) so bar length stays non-zero.
                0x58 if data.len() >= 2 && data[0] > 0 && data[1] < 8 => {
                    let sig = (data[0], 1u8 << data[1]);
                    if before_sections {
                        m.timesig = sig
                    } else {
                        later(&mut m, Timing::TimeSig(sig.0, sig.1))
                    }
                }
                _ => {}
            }
        }
    }
    m
}

/// What the style browser lists: the header facts, without building sections or reading
/// the chunks after the track (CASM, OTS).
#[derive(Debug, Clone, PartialEq)]
pub struct Summary {
    /// The SFF name marker; empty if the style has none.
    pub name: String,
    pub bpm: f64,
    pub timesig: (u8, u8),
    /// Sections present, sorted.
    pub sections: Vec<SectionId>,
    /// "SFF1" or "SFF2", from the file's header (empty if it names neither).
    pub format: String,
}

impl Summary {
    /// Reads only the MThd header and the style track, never the whole file.
    pub fn load(path: &std::path::Path) -> Result<Summary> {
        use std::io::Read;
        // No path in the errors: the browser shows it next to them.
        let mut f = std::fs::File::open(path)?;
        let mut bytes = Vec::new();
        (&mut f).take(8).read_to_end(&mut bytes)?;
        if bytes.len() < 8 || &bytes[0..4] != b"MThd" {
            bail!("not a MIDI/style file");
        }
        // The header body and the MTrk chunk header, then the track itself.
        let hlen = be32(&bytes[4..8]).min(1 << 16);
        (&mut f).take(hlen as u64 + 8).read_to_end(&mut bytes)?;
        if bytes.len() == 8 + hlen + 8 {
            let tlen = be32(&bytes[8 + hlen + 4..]);
            (&mut f).take(tlen as u64).read_to_end(&mut bytes)?;
        }
        summarize(&bytes)
    }
}

/// `Summary` of a style file's bytes; anything after the style track is ignored.
pub fn summarize(bytes: &[u8]) -> Result<Summary> {
    let (_, events, _) = parse_header_track(bytes)?;
    let m = scan_meta(&events);
    if m.known().next().is_none() {
        bail!("no section markers found");
    }
    let mut sections: Vec<SectionId> = m.known().collect();
    sections.sort();
    sections.dedup();
    Ok(Summary { name: m.name, bpm: 60_000_000.0 / m.tempo_us as f64, timesig: m.timesig, sections, format: m.format })
}

fn build_style(
    ppq: u16,
    events: Vec<TimedEv>,
    casm: Vec<Cseg>,
    other_chunks: Vec<(String, Vec<u8>)>,
) -> Result<Style> {
    let Meta { name, format: fmt, tempo_us, timesig, marks, timing, end_tick } = scan_meta(&events);
    if marks.is_empty() {
        bail!("no section markers found");
    }
    let first = marks[0].0;

    let init: Vec<Ev> = events
        .iter()
        .filter(|e| e.tick < first || (e.tick == first && !matches!(e.ev, Ev::NoteOn { .. })))
        .filter(|e| e.ev.channel().is_some() || matches!(e.ev, Ev::Sysex(_)))
        .map(|e| e.ev.clone())
        .collect();

    let mut sections = BTreeMap::new();
    let mut opaque_sections = Vec::new();
    for (i, &(start, ref id)) in marks.iter().enumerate() {
        let end = marks.get(i + 1).map(|m| m.0).unwrap_or(end_tick);
        let evs: Vec<TimedEv> = events
            .iter()
            .filter(|e| {
                e.tick >= start
                    && (e.tick < end || (e.tick == end && matches!(e.ev, Ev::NoteOff { .. })))
            })
            .filter(|e| !matches!(e.ev, Ev::Meta { .. }))
            .map(|e| TimedEv { tick: e.tick - start, ev: e.ev.clone() })
            .collect();
        // Round section length to whole bars.
        let tpb = ((ppq as u32 * 4 * timesig.0 as u32) / timesig.1.max(1) as u32).max(1);
        // Saturating: tick accumulation saturates, so a malformed file can put `end` at u32::MAX.
        let bars = ((end - start).saturating_add(tpb / 2) / tpb).clamp(1, u32::MAX / tpb);
        let len = bars * tpb;
        match id {
            Ok(id) => {
                sections.insert(*id, Section { id: *id, start, len, events: evs });
            }
            Err(name) => opaque_sections.push(OpaqueSection { name: name.clone(), start, len, events: evs }),
        }
    }
    let timing_changes = timing
        .into_iter()
        .map(|(tick, i, change)| {
            let (start, ref id) = marks[i];
            let section = id.as_ref().map_or_else(|n| n.clone(), |id| id.name());
            TimingChange { section, tick: tick - start, change }
        })
        .collect();

    let ots = other_chunks.iter().find(|(id, _)| id == "OTSc").map(|(_, d)| parse_ots(d)).unwrap_or_default();
    Ok(Style { name, format: fmt, ppq, tempo_us, timesig, init, sections, opaque_sections, timing_changes, casm, ots, other_chunks })
}


#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "slow-tests")]
    use crate::theory::{self, Chord, NUM_TYPES};

    /// A Ctb2 record: src ch 12 -> dest 12, all roots/types on, source C + `src_type`,
    /// with every zone byte set to `zone`.
    fn ctb2(src_type: u8, zone: u8) -> Vec<u8> {
        let mut d = vec![11];
        d.extend_from_slice(b"Chord1  ");
        d.extend_from_slice(&[11, 0, 0x0F, 0xFF, 0x03, 0xFF, 0xFF, 0xFF, 0xFF, 0, src_type, 0, 127]);
        d.extend_from_slice(&[zone; 18]);
        assert_eq!(d.len(), 40);
        d
    }

    pub(super) fn chunk(id: &[u8], body: &[u8]) -> Vec<u8> {
        let mut v = id.to_vec();
        v.extend_from_slice(&(body.len() as u32).to_be_bytes());
        v.extend_from_slice(body);
        v
    }

    /// Smallest style the parser accepts: one Main A bar with a note on ch 12 plus a CASM
    /// segment holding `rec`.
    #[cfg(feature = "slow-tests")]
    fn style_bytes(rec_id: &[u8], rec: &[u8]) -> Vec<u8> {
        style_bytes_recs(&[(rec_id, rec)])
    }

    /// As `style_bytes`, with several CSEG records after the Sdec.
    fn style_bytes_recs(recs: &[(&[u8], &[u8])]) -> Vec<u8> {
        let mut trk = vec![0x00, 0xFF, 0x06, 4];
        trk.extend_from_slice(b"SFF2");
        trk.extend_from_slice(&[0x00, 0xFF, 0x06, 6]);
        trk.extend_from_slice(b"Main A");
        trk.extend_from_slice(&[0x00, 0x9B, 60, 100, 0x83, 0x00, 0x8B, 60, 0, 0x00, 0xFF, 0x2F, 0]);
        let mut cseg = chunk(b"Sdec", b"Main A");
        for (id, rec) in recs {
            cseg.extend(chunk(id, rec));
        }
        let mut out = chunk(b"MThd", &[0, 0, 0, 1, 0, 96]);
        out.extend(chunk(b"MTrk", &trk));
        out.extend(chunk(b"CASM", &chunk(b"CSEG", &cseg)));
        out
    }

    /// Drive every key through every chord for each rule, the way the engine would. `plays`,
    /// `transpose` and `transpose_group` are pure functions of the rule, chord and keys, so a
    /// rule equal to one already in `seen` would repeat exactly the same calls and is not
    /// driven again (most corrupted files parse back to rules the test has already seen).
    #[cfg(feature = "slow-tests")]
    fn exercise(style: &Style, seen: &mut Vec<ChannelRule>) {
        for seg in &style.casm {
            for r in &seg.rules {
                assert!((r.src_type as usize) < NUM_TYPES, "src_type {} survived parsing", r.src_type);
                assert!((8..16).contains(&r.dest_ch), "dest_ch {} survived parsing", r.dest_ch);
                if seen.contains(r) {
                    continue;
                }
                seen.push(r.clone());
                // Every chord type that exists: CASM types, Cancel and the display-only ids.
                for ty in 0..theory::TYPE_NAMES.len() as u8 {
                    let display_only = matches!(ty, theory::M7B5 | theory::FLAT5 | theory::MM7B5);
                    for root in 0..12 {
                        let c = Chord::new(root, ty);
                        let plays = theory::plays(r, c);
                        if ty == theory::CANCEL {
                            assert_eq!(plays, theory::is_drum_part(r.dest_ch));
                        }
                        if display_only {
                            assert_eq!(plays, theory::plays(r, c.casm()));
                        }
                        let mut out = [None; 3];
                        for k in 0..=127u8 {
                            let n = theory::transpose(k, r, c);
                            assert!(n.is_none_or(|n| n <= 127));
                            if display_only {
                                assert_eq!(n, theory::transpose(k, r, c.casm()));
                            }
                        }
                        theory::transpose_group(&[48, 52, 55], r, c, &mut out);
                    }
                }
            }
        }
    }

    #[test]
    fn source_chord_type_is_clamped() {
        for (raw, want) in [(0u8, 0u8), (2, 2), (33, 33), (34, 2), (35, 2), (36, 2), (37, 2), (38, 2), (63, 2), (0x7F, 2), (0xFF, 2)] {
            let r = parse_ctab(&ctb2(raw, 0), true).unwrap();
            assert_eq!(r.src_type, want, "Ctb2 src_type {raw}");
            let r = parse_ctab(&ctb2(raw, 0)[..26], false).unwrap();
            assert_eq!(r.src_type, want, "Ctab src_type {raw}");
        }
    }

    #[cfg(feature = "slow-tests")]
    #[test]
    fn malformed_ctab_transposes_without_panicking() {
        // Out-of-range source types with garbage zone bytes (NTR/NTT/RTR enums, High Key and
        // inverted note limits), as a whole style file through the real parser.
        let seen = &mut Vec::new();
        for src_type in [34u8, 35, 36, 37, 38, 63, 64, 0x80, 0xFF] {
            for zone in [0x00u8, 0x07, 0x7F, 0x80, 0xFF] {
                let s = parse(&style_bytes(b"Ctb2", &ctb2(src_type, zone))).unwrap();
                exercise(&s, seen);
                let s = parse(&style_bytes(b"Ctab", &ctb2(src_type, zone)[..27])).unwrap();
                exercise(&s, seen);
            }
        }
    }

    /// A 27-byte SFF1 Ctab record for src ch 12 (see `ctb2`) with the given NTR and NTT bytes.
    pub(super) fn ctab(ntr: u8, ntt: u8) -> Vec<u8> {
        let mut d = ctb2(2, 0)[..27].to_vec();
        d[20] = ntr;
        d[21] = ntt;
        d[24] = 127;
        d
    }

    #[test]
    fn every_ctab_ntt_code_decodes() {
        let want = [
            Ntt::Bypass,
            Ntt::Melody,
            Ntt::Chord,
            Ntt::Bass,
            Ntt::MelodicMinor,
            Ntt::HarmonicMinor,
            Ntt::HarmonicMinor5,
            Ntt::NaturalMinor,
            Ntt::NaturalMinor5,
            Ntt::Dorian,
            Ntt::Dorian5,
        ];
        for v in 0..=255u8 {
            let r = parse_ctab(&ctab(0, v), false).unwrap();
            let ntt = want.get(v as usize).copied().unwrap_or(Ntt::Melody);
            assert!(r.zones.iter().all(|z| z.ntt == ntt), "Ctab NTT {v:#04x}: {:?}", r.zones[1].ntt);
            assert!(r.zones.iter().all(|z| z.bass_on == (v == 3)), "Ctab NTT {v:#04x} Bass On");
        }
    }

    #[test]
    fn cntt_overrides_ctab_table_and_keeps_bass_on() {
        let cseg = |ctab_ntt: u8, cntt: u8| {
            let s = parse(&style_bytes_recs(&[(b"Ctab", &ctab(0, ctab_ntt)), (b"Cntt", &[11, cntt])])).unwrap();
            let r = s.casm[0].rules[0].clone();
            assert!(r.zones.iter().all(|z| z.ntt == r.zones[1].ntt && z.bass_on == r.zones[1].bass_on));
            (r.zones[1].ntt, r.zones[1].bass_on)
        };
        // What the corpus writes: Harmonic Minor refined to its 5th Var., Bass kept as Melody
        // with Bass On even though the Cntt bit is clear.
        assert_eq!(cseg(5, 0x06), (Ntt::HarmonicMinor5, false));
        assert_eq!(cseg(3, 0x01), (Ntt::Melody, true));
        assert_eq!(cseg(2, 0x02), (Ntt::Chord, false));
        // Every Cntt table, with and without its Bass bit.
        assert_eq!(cseg(1, 0x09), (Ntt::Dorian, false));
        assert_eq!(cseg(1, 0x8A), (Ntt::Dorian5, true));
        assert_eq!(cseg(2, 0x87), (Ntt::NaturalMinor, true));
        // A Cntt for a channel with no Ctab changes nothing.
        let s = parse(&style_bytes_recs(&[(b"Ctab", &ctab(0, 2)), (b"Cntt", &[3, 0x8A])])).unwrap();
        let r = &s.casm[0].rules[0];
        assert_eq!(r.zones.map(|z| (z.ntt, z.bass_on)), [(Ntt::Chord, false); 3]);
    }

    #[test]
    fn cntt_never_overrides_ctb2() {
        let mut d = ctb2(2, 0);
        for z in [22, 28, 34] {
            d[z] = 0; // Root Trans
            d[z + 1] = 0x02; // Chord, Bass Off
            d[z + 4] = 127;
        }
        let s = parse(&style_bytes_recs(&[(b"Ctb2", &d), (b"Cntt", &[11, 0x8A])])).unwrap();
        let r = &s.casm[0].rules[0];
        assert!(r.zones.iter().all(|z| z.ntt == Ntt::Chord && !z.bass_on));
    }

    #[cfg(feature = "slow-tests")]
    #[test]
    fn corpus_cntt_bass_parts_follow_slash_chords() {
        // Every corpus Cntt style writes its Ctab "Bass" channel's Cntt as plain Melody. The Bass
        // part must still carry Bass On, as 193 of 194 SFF2 corpus styles give their Bass part.
        let (mut found, mut bass_parts) = (0, 0);
        for (p, s) in crate::library::corpus_loaded() {
            // The raw bytes pick the Cntt styles; the parse comes from the shared cache.
            let Ok(bytes) = std::fs::read(p) else { continue };
            if !bytes.windows(4).any(|w| w == b"Cntt") {
                continue;
            }
            found += 1;
            for r in s.casm.iter().flat_map(|seg| &seg.rules) {
                if r.dest_ch == 10 && r.zones[1].ntt != Ntt::Bypass {
                    assert!(r.zones.iter().all(|z| z.bass_on), "{}: Bass part lost Bass On", p.display());
                    assert_eq!(r.zones[1].ntt, Ntt::Melody, "{}", p.display());
                    // The audible check: the part's source root plays E under C/E, C under C.
                    let key = 36 + r.src_root % 12;
                    let c = crate::theory::Chord::new(0, 0);
                    let c_over_e = crate::theory::Chord { bass: Some(4), ..c };
                    let pc = |ch| crate::theory::transpose(key, r, ch).map(|n| n % 12);
                    assert_eq!(pc(c), Some(0), "{}: root under C", p.display());
                    assert_eq!(pc(c_over_e), Some(4), "{}: root under C/E", p.display());
                    bass_parts += 1;
                }
            }
        }
        if found == 0 {
            eprintln!("no corpus Cntt styles; skipping");
        } else {
            assert!(bass_parts > 0, "Cntt styles found but no Bass part checked");
        }
    }

    #[test]
    fn dest_channel_stays_on_accompaniment_parts() {
        for (dest, want) in [(8u8, 8u8), (15, 15), (0x1F, 15), (0, 11), (7, 11), (0xF3, 11)] {
            let mut d = ctb2(2, 0);
            d[9] = dest;
            assert_eq!(parse_ctab(&d, true).unwrap().dest_ch, want, "dest byte {dest:#x}");
        }
        // Source channel below 9 too: stay on the matching accompaniment part.
        let mut d = ctb2(2, 0);
        d[0] = 2;
        d[9] = 2;
        assert_eq!(parse_ctab(&d, true).unwrap().dest_ch, 10);
    }

    #[test]
    fn huge_deltas_do_not_overflow_section_length() {
        // Main A marker, then 20 events each 0x0FFFFFFF ticks apart: the end tick saturates
        // at u32::MAX and the bar rounding must not overflow.
        let mut trk = vec![0x00, 0xFF, 0x06, 4];
        trk.extend_from_slice(b"SFF2");
        trk.extend_from_slice(&[0x00, 0xFF, 0x06, 6]);
        trk.extend_from_slice(b"Main A");
        trk.extend_from_slice(&[0x00, 0x9B, 60, 100]);
        for _ in 0..20 {
            trk.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0x7F, 0x8B, 60, 0]);
        }
        trk.extend_from_slice(&[0x00, 0xFF, 0x2F, 0]);
        let mut bytes = chunk(b"MThd", &[0, 0, 0, 1, 0, 96]);
        bytes.extend(chunk(b"MTrk", &trk));
        let s = parse(&bytes).unwrap();
        let sec = &s.sections[&SectionId::Main(0)];
        let tpb = 96 * 4;
        assert!(sec.len >= tpb && sec.len % tpb == 0, "len {}", sec.len);
        assert!(sec.len > u32::MAX - tpb, "len {} should cover the whole saturated span", sec.len);
    }

    #[cfg(feature = "slow-tests")]
    #[test]
    fn truncated_and_corrupted_files_do_not_panic() {
        let good = style_bytes(b"Ctb2", &ctb2(0xFF, 0xFF));
        assert!(parse(&good).is_ok());
        let seen = &mut Vec::new();
        for n in 0..good.len() {
            if let Ok(s) = parse(&good[..n]) {
                exercise(&s, seen);
            }
        }
        // Deterministic byte flips over every position.
        let mut seed = 0x2545_F491_4F6C_DD1Du64;
        for i in 0..good.len() {
            for _ in 0..4 {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                let mut b = good.clone();
                b[i] = seed as u8;
                if let Ok(s) = parse(&b) {
                    exercise(&s, seen);
                }
            }
        }
    }

    #[test]
    fn ots_parse() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/FunkyFinger.S930.STY");
        if !p.exists() {
            return;
        }
        let s = Style::load(&p).unwrap();
        assert_eq!(s.ots.len(), 4);
        let o = &s.ots[0];
        assert_eq!(o.parts[0].voice, Some((0, 116, 4)));
        assert!(o.parts[0].on);
        assert!(!o.parts[1].on && !o.parts[2].on && !o.parts[3].on);
        assert_eq!(o.parts[3].voice, Some((0, 117, 95)));
        assert_eq!(o.parts[3].volume, 75);
        // SlowWalker OTS 1: Right 1 + Right 2 layered, Left on.
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
        let s = Style::load(&p).unwrap();
        let o = &s.ots[0];
        assert!(o.parts[0].on && o.parts[1].on && !o.parts[2].on && o.parts[3].on);
        // Its Right 1 voice settings (#238): cutoff CC74 70; release CC72 0x60, then XG
        // NRPN 1/66 0x40 (the last wins); decay and vibrato as NRPN; portamento on, time 8;
        // bend range 2; mono (XG 08 05 = 0).
        let r1 = &o.parts[0];
        assert_eq!(r1.tone, [70, 64, 64, 64, 64, 74, 70, 63, 127, 8].map(Some));
        assert_eq!(r1.bend_range, Some(2));
        assert_eq!(r1.xg.len(), 22);
        assert!(r1.xg.iter().any(|x| x == (0x08, 0x05, 0)), "mono");
        assert!(o.parts[1].xg.iter().any(|x| x == (0x08, 0x05, 1)), "Right 2 poly");
    }

    /// Hand-made OTS track: CC and NRPN writes of the same setting, the last wins; an RPN
    /// other than 0 and an NRPN outside MSB 1 set nothing; XG SysEx for another part.
    #[test]
    fn ots_voice_settings_parse() {
        let cc = |cc: u8, v: u8| [0x00, 0xB1, cc, v];
        let mut trk = Vec::new();
        for m in [cc(74, 90), cc(99, 1), cc(98, 0x20), cc(6, 30), cc(72, 20), cc(99, 2), cc(98, 0x66), cc(6, 99)] {
            trk.extend_from_slice(&m);
        }
        for m in [cc(101, 0), cc(100, 1), cc(6, 70), cc(101, 0), cc(100, 0), cc(6, 12), cc(65, 127)] {
            trk.extend_from_slice(&m);
        }
        trk.extend_from_slice(&[0x00, 0xF0, 8, 0x43, 0x10, 0x4C, 0x08, 0x01, 0x05, 0x00, 0xF7]);
        trk.extend_from_slice(&[0x00, 0xF0, 8, 0x43, 0x10, 0x4C, 0x0A, 0x01, 0x40, 0x50, 0xF7]);
        trk.extend_from_slice(&[0x00, 0xF0, 8, 0x43, 0x10, 0x4C, 0x08, 0x07, 0x05, 0x00, 0xF7]);
        trk.extend_from_slice(&[0x00, 0xFF, 0x2F, 0]);
        let o = parse_ots(&chunk(b"MTrk", &trk));
        let r2 = &o[0].parts[1];
        let mut want = [None; crate::tone::TONE];
        want[crate::tone::CUTOFF] = Some(30);
        want[crate::tone::RELEASE] = Some(20);
        want[crate::tone::PORTAMENTO] = Some(127);
        assert_eq!(r2.tone, want);
        assert_eq!(r2.bend_range, Some(12));
        assert_eq!(r2.xg.iter().collect::<Vec<_>>(), vec![(0x08, 0x05, 0), (0x0A, 0x40, 0x50)]);
        assert!(o[0].parts[0].xg.is_empty() && o[0].parts[0].tone == [None; crate::tone::TONE]);
        assert!(o[0].parts.iter().all(|p| p.insert.is_none()), "no insertion SysEx");
    }

    /// An OTS track's Insertion Effect blocks: block n is part n (type MSB and LSB in one
    /// message, or in two), unless the track assigns it to another part; the first block
    /// on a part wins; a block past the parts sets nothing.
    #[test]
    fn ots_insertion_types_parse() {
        let ins = |nn: u8, aa: u8, data: &[u8]| {
            let mut m = vec![0x00, 0xF0, 7 + data.len() as u8, 0x43, 0x10, 0x4C, 0x03, nn, aa];
            m.extend_from_slice(data);
            m.push(0xF7);
            m
        };
        let mut trk = Vec::new();
        for m in [ins(0, 0x00, &[0x60, 0x10]), ins(1, 0x00, &[0x45]), ins(1, 0x01, &[0x11]), ins(2, 0x00, &[0x46, 0x00]), ins(2, 0x0C, &[3])] {
            trk.extend_from_slice(&m);
        }
        trk.extend_from_slice(&ins(3, 0x00, &[0x4C, 0x00]));
        trk.extend_from_slice(&ins(9, 0x00, &[0x49, 0x00]));
        trk.extend_from_slice(&[0x00, 0xFF, 0x2F, 0]);
        let o = parse_ots(&chunk(b"MTrk", &trk));
        let got: Vec<_> = o[0].parts.iter().map(|p| p.insert).collect();
        // Block 2 is assigned to Left (part 3) and comes before block 3: it wins there.
        assert_eq!(got, vec![Some((0x60, 0x10)), Some((0x45, 0x11)), None, Some((0x46, 0x00))]);
    }

    /// Corpus counts (#238): how many OTS parts set each voice setting.
    #[cfg(feature = "slow-tests")]
    #[test]
    fn corpus_ots_voice_settings() {
        let (mut parts, mut tone, mut bend, mut xg, mut xg_max, mut inserts) = (0, [0; crate::tone::TONE], 0, 0, 0, 0);
        for (_, s) in crate::library::corpus_loaded() {
            for q in s.ots.iter().flat_map(|o| &o.parts) {
                parts += 1;
                for (n, v) in tone.iter_mut().zip(q.tone) {
                    *n += v.is_some() as usize;
                }
                bend += q.bend_range.is_some() as usize;
                xg += q.xg.len();
                xg_max = xg_max.max(q.xg.len());
                inserts += q.insert.is_some() as usize;
            }
        }
        eprintln!("{parts} OTS parts: tone {tone:?} (by TONE_CC), bend range {bend}, {xg} XG part parameters (max {xg_max} per part), {inserts} insertion types");
        if parts == 0 {
            return;
        }
        assert!(xg_max < XG_MAX, "room to spare for XG parameters: {xg_max}");
        assert_eq!((tone, bend, inserts), ([parts; crate::tone::TONE], parts, parts), "every corpus OTS part sets them all");
    }

    #[test]
    fn sint_parses_per_channel() {
        let xg_part = |part, addr, v| Ev::Sysex(vec![0xF0, 0x43, 0x10, 0x4C, 0x08, part, addr, v, 0xF7]);
        let reverb = vec![0xF0, 0x43, 0x10, 0x4C, 0x02, 0x01, 0x00, 0x01, 0x10, 0xF7];
        let init = vec![
            Ev::Sysex(vec![0xF0, 0x7E, 0x7F, 0x09, 0x01, 0xF7]),
            Ev::Sysex(vec![0xF0, 0x43, 0x10, 0x4C, 0x00, 0x00, 0x7E, 0x00, 0xF7]),
            Ev::Sysex(reverb.clone()),
            Ev::Cc { ch: 9, cc: 0, val: 127 },
            Ev::Cc { ch: 9, cc: 32, val: 0 },
            Ev::Pc { ch: 9, prog: 25 },
            Ev::Cc { ch: 9, cc: 7, val: 100 },
            Ev::Cc { ch: 9, cc: 7, val: 88 },
            Ev::Cc { ch: 9, cc: 10, val: 64 },
            Ev::Cc { ch: 9, cc: 91, val: 30 },
            Ev::Cc { ch: 9, cc: 93, val: 5 },
            Ev::Cc { ch: 9, cc: 101, val: 0 },
            Ev::Cc { ch: 9, cc: 100, val: 0 },
            Ev::Cc { ch: 9, cc: 6, val: 2 },
            Ev::Bend { ch: 9, val: 0x2000 },
            xg_part(9, 0x07, 2),
            Ev::Cc { ch: 11, cc: 11, val: 120 },
        ];
        let s = SInt::parse(&init);
        let c = &s.channels[9];
        assert_eq!((c.bank_msb, c.bank_lsb, c.program), (Some(127), Some(0), Some(25)));
        assert_eq!((c.volume, c.pan, c.reverb, c.chorus), (Some(88), Some(64), Some(30), Some(5)));
        assert_eq!(c.other, vec![
            Ev::Cc { ch: 9, cc: 101, val: 0 },
            Ev::Cc { ch: 9, cc: 100, val: 0 },
            Ev::Cc { ch: 9, cc: 6, val: 2 },
            Ev::Bend { ch: 9, val: 0x2000 },
        ]);
        assert_eq!(c.xg_part, vec![(0x07, 2)]);
        assert_eq!(s.channels[11].other, vec![Ev::Cc { ch: 11, cc: 11, val: 120 }]);
        assert_eq!(s.channels[11].volume, None);
        assert_eq!(s.sysex, vec![reverb], "resets left out");
    }

    /// A receiver's bank registers and selected voice after `msgs` (bank MSB/LSB CCs and
    /// program changes on one channel): ((MSB, LSB), voice).
    type Voiced = ((Option<u8>, Option<u8>), Option<(Option<u8>, Option<u8>, u8)>);
    fn receive<'a>(msgs: impl IntoIterator<Item = &'a Ev>) -> Voiced {
        let (mut bank, mut voice) = ((None, None), None);
        for ev in msgs {
            match *ev {
                Ev::Cc { cc: 0, val, .. } => bank.0 = Some(val),
                Ev::Cc { cc: 32, val, .. } => bank.1 = Some(val),
                Ev::Pc { prog, .. } => voice = Some((bank.0, bank.1, prog)),
                _ => {}
            }
        }
        (bank, voice)
    }

    /// What the structure sends for a channel's voice: bank, program, then the pending bank.
    fn voice_msgs(ch: u8, c: &ChannelInit) -> Vec<Ev> {
        let cc = |cc, v: Option<u8>| v.map(|val| Ev::Cc { ch, cc, val });
        [cc(0, c.bank_msb), cc(32, c.bank_lsb), c.program.map(|prog| Ev::Pc { ch, prog }), cc(0, c.pending_msb), cc(32, c.pending_lsb)]
            .into_iter()
            .flatten()
            .collect()
    }

    /// A bank select after the program change selects no voice: the voice keeps the bank in
    /// effect at the program change, and the late bank select stays pending.
    #[test]
    fn sint_bank_after_program_is_pending() {
        // ChartPop1's ch 15: CC0 8, CC32 2, PC 3, CC0 104 selects 8/2/3.
        let init = vec![
            Ev::Cc { ch: 14, cc: 0, val: 8 },
            Ev::Cc { ch: 14, cc: 32, val: 2 },
            Ev::Pc { ch: 14, prog: 3 },
            Ev::Cc { ch: 14, cc: 0, val: 104 },
        ];
        let c = &SInt::parse(&init).channels[14];
        assert_eq!((c.bank_msb, c.bank_lsb, c.program), (Some(8), Some(2), Some(3)));
        assert_eq!((c.pending_msb, c.pending_lsb), (Some(104), None));
        assert_eq!(receive(&voice_msgs(14, c)), receive(&init));
        assert_eq!(receive(&init).1, Some((Some(8), Some(2), 3)));
        // A bank select before a later program change is that program's bank.
        let init = [&init[..], &[Ev::Pc { ch: 14, prog: 7 }]].concat();
        let c = &SInt::parse(&init).channels[14];
        assert_eq!((c.bank_msb, c.bank_lsb, c.program, c.pending_msb), (Some(104), Some(2), Some(7), None));
    }

    /// Every corpus style's SInt survives structuring: each channel's last value of every
    /// controller is where the structure says, its voice and bank registers end up as the
    /// file leaves them, and every SysEx but the resets is kept.
    #[cfg(feature = "slow-tests")]
    #[test]
    fn sint_structures_every_corpus_style() {
        let (mut styles, mut resets, mut pending) = (0, 0, 0);
        for (p, style) in crate::library::corpus_loaded() {
            styles += 1;
            let s = style.sint();
            let mut last_cc = BTreeMap::new();
            let mut last_pc = [None; 16];
            let mut kept = 0;
            for ev in &style.init {
                match *ev {
                    Ev::Cc { ch, cc, val } => {
                        last_cc.insert((ch, cc), val);
                    }
                    Ev::Pc { ch, prog } => last_pc[ch as usize] = Some(prog),
                    Ev::Sysex(ref v) if is_reset(v) => resets += 1,
                    Ev::Sysex(_) => kept += 1,
                    _ => {}
                }
            }
            for (&(ch, cc), &val) in &last_cc {
                let c = &s.channels[ch as usize];
                let got = match cc {
                    0 | 32 => continue,
                    7 => c.volume,
                    10 => c.pan,
                    91 => c.reverb,
                    93 => c.chorus,
                    _ => c.other.iter().rev().find_map(|e| match *e {
                        Ev::Cc { cc: x, val, .. } if x == cc => Some(val),
                        _ => None,
                    }),
                };
                assert_eq!(got, Some(val), "{p:?} ch {ch} cc {cc}");
            }
            for ch in 0..16u8 {
                let c = &s.channels[ch as usize];
                assert_eq!(c.program, last_pc[ch as usize], "{p:?} ch {ch}");
                let file = style.init.iter().filter(|e| e.channel() == Some(ch));
                assert_eq!(receive(&voice_msgs(ch, c)), receive(file), "{p:?} ch {ch}");
                pending += (c.pending_msb.is_some() || c.pending_lsb.is_some()) as usize;
            }
            let xg: usize = s.channels.iter().map(|c| c.xg_part.len()).sum();
            assert_eq!(xg + s.sysex.len(), kept, "{p:?}");
        }
        if styles > 0 {
            assert!(styles >= 100 && resets > 0 && pending > 0, "{styles} styles, {resets} resets, {pending} pending");
        }
    }

    /// SFF2 Bass On is read per zone: this piano's left-hand zone (below Mid Low 50)
    /// follows slash chords, its chord zone does not.
    #[test]
    fn bass_on_per_zone() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/SX900Style for Genos/6-8ChartBallad.T547.prs");
        if !p.exists() {
            return;
        }
        let s = Style::load(&p).unwrap();
        let r = s.casm[0].rules.iter().find(|r| r.src_ch == 12).unwrap();
        assert_eq!(r.mid_lo, 50);
        assert!(r.zones[0].bass_on);
        assert!(!r.zones[1].bass_on && !r.zones[2].bass_on);
        // Through transpose, over C/E: the left-hand C moves to the slash bass E, the
        // chord-zone C stays exactly where it plays over plain C.
        use crate::theory::{transpose, Chord};
        let (c, c_over_e) = (Chord::new(0, 0), Chord { root: 0, ty: 0, bass: Some(4) });
        let low = transpose(36, r, c_over_e).unwrap();
        assert_eq!(low % 12, 4, "low zone C1 over C/E -> {low}");
        assert_ne!(Some(low), transpose(36, r, c));
        assert_eq!(transpose(60, r, c_over_e), transpose(60, r, c));
        assert_eq!(transpose(60, r, c), Some(60));
    }

    /// A style track built from (delta, raw event bytes) pairs, ppq 96 (a 4/4 bar is 384).
    pub(super) fn track_style(evs: &[(u32, Vec<u8>)]) -> Vec<u8> {
        let mut trk = Vec::new();
        for (delta, ev) in evs {
            let mut d = *delta;
            let mut v = vec![(d & 0x7F) as u8];
            d >>= 7;
            while d > 0 {
                v.insert(0, (d & 0x7F) as u8 | 0x80);
                d >>= 7;
            }
            trk.extend(v);
            trk.extend_from_slice(ev);
        }
        trk.extend_from_slice(&[0x00, 0xFF, 0x2F, 0]);
        let mut out = chunk(b"MThd", &[0, 0, 0, 1, 0, 96]);
        out.extend(chunk(b"MTrk", &trk));
        out
    }

    pub(super) fn marker(t: &str) -> Vec<u8> {
        let mut v = vec![0xFF, 0x06, t.len() as u8];
        v.extend_from_slice(t.as_bytes());
        v
    }

    pub(super) fn tempo(us: u32) -> Vec<u8> {
        vec![0xFF, 0x51, 3, (us >> 16) as u8, (us >> 8) as u8, us as u8]
    }

    /// Main A, Fill In AA, an unknown "Fill In AB", then Ending A; one bar each, with a
    /// tempo change inside Fill In AB and a tempo and time-signature change inside Ending A.
    fn sections_style() -> Vec<u8> {
        let on = |k: u8| vec![0x9B, k, 100];
        let off = |k: u8| vec![0x8B, k, 0];
        track_style(&[
            (0, marker("SFF2")),
            (0, tempo(500_000)),
            (0, vec![0xFF, 0x58, 4, 4, 2, 24, 8]),
            (0, marker("Main A")),
            (0, on(60)),
            (384, off(60)),
            (0, marker("Fill In AA")),
            (0, on(62)),
            (384, off(62)),
            (0, marker("Fill In AB")),
            (0, on(64)),
            (192, tempo(600_000)),
            (192, off(64)),
            (0, marker("Ending A")),
            (0, on(65)),
            (192, tempo(700_000)),
            (0, vec![0xFF, 0x58, 4, 3, 2, 24, 8]),
            (192, off(65)),
        ])
    }

    fn note_ons(evs: &[TimedEv]) -> Vec<(u32, u8)> {
        evs.iter()
            .filter_map(|e| match e.ev {
                Ev::NoteOn { key, .. } => Some((e.tick, key)),
                _ => None,
            })
            .collect()
    }

    /// An unknown marker opens its own (opaque, unplayed) section: its events no longer
    /// leak into the section before it.
    #[test]
    fn unknown_marker_is_its_own_section() {
        let s = parse(&sections_style()).unwrap();
        let fill = &s.sections[&SectionId::Fill(0)];
        assert_eq!(fill.len, 384);
        assert_eq!(note_ons(&fill.events), vec![(0, 62)]);
        let ids: Vec<_> = s.sections.keys().copied().collect();
        assert_eq!(ids, vec![SectionId::Main(0), SectionId::Fill(0), SectionId::Ending(0)]);
        assert_eq!(s.opaque_sections.len(), 1);
        let ab = &s.opaque_sections[0];
        assert_eq!((ab.name.as_str(), ab.start, ab.len), ("Fill In AB", 768, 384));
        assert_eq!(note_ons(&ab.events), vec![(0, 64)]);
        let sum = summarize(&sections_style()).unwrap();
        assert_eq!(sum.sections, ids);
    }

    /// Tempo and time-signature changes after the first section are kept per section, but the
    /// style's tempo and meter stay those of bar 1.
    #[test]
    fn later_tempo_and_timesig_are_kept_not_applied() {
        let s = parse(&sections_style()).unwrap();
        assert_eq!((s.tempo_us, s.timesig), (500_000, (4, 4)));
        let got: Vec<_> = s.timing_changes.iter().map(|c| (c.section.as_str(), c.tick, c.change)).collect();
        assert_eq!(
            got,
            vec![
                ("Fill In AB", 192, Timing::Tempo(600_000)),
                ("Ending A", 192, Timing::Tempo(700_000)),
                ("Ending A", 192, Timing::TimeSig(3, 4)),
            ]
        );
        let sum = summarize(&sections_style()).unwrap();
        assert_eq!((sum.bpm, sum.timesig), (120.0, (4, 4)));
    }

    /// Corpus counts: 3 unknown markers (2 Fill In AB, 1 Fill In EE) in 2 styles, and tempo
    /// changes after bar 1 (Intro/Ending ritardandos) in 71 styles; no later meter changes.
    #[cfg(feature = "slow-tests")]
    #[test]
    fn corpus_unknown_markers_and_later_timing() {
        let (mut styles, mut opaque, mut tempo_styles, mut tempos, mut sigs) = (0, Vec::new(), 0, 0, 0);
        for (p, s) in crate::library::corpus_loaded() {
            styles += 1;
            opaque.extend(s.opaque_sections.iter().map(|o| o.name.clone()));
            let t = s.timing_changes.iter().filter(|c| matches!(c.change, Timing::Tempo(_))).count();
            tempos += t;
            tempo_styles += (t > 0) as usize;
            sigs += s.timing_changes.len() - t;
            // Each opaque section ends where the next section starts: nothing leaks.
            for o in &s.opaque_sections {
                let next = s.sections.values().map(|x| x.start).filter(|&x| x > o.start).min();
                assert_eq!(next, Some(o.start + o.len), "{p:?} {}", o.name);
            }
        }
        if styles == 0 {
            eprintln!("no corpus; skipping");
            return;
        }
        opaque.sort();
        eprintln!("{styles} styles: opaque {opaque:?}, {tempos} tempo changes in {tempo_styles} styles, {sigs} meter changes");
        assert_eq!(opaque, vec!["Fill In AB", "Fill In AB", "Fill In EE"]);
        assert_eq!((tempo_styles, tempos, sigs), (71, 1193, 0));
    }
}

/// A synthetic style for tests that must run without the git-ignored corpus (CI has none):
/// original notes, no Yamaha data. It has what the Launchkey's pads reach: Intro A-C, Main
/// A-D, Fill In AA-DD, Break and Ending A-C; an SFF1 CASM (rhythm Bypass with Autostart,
/// Bass, Chord and Melody parts); a channel setup with voices and XG effect types; and four
/// One Touch Settings that differ in voices, part on/off, volume, octave and sends.
#[cfg(test)]
pub(crate) mod test_style {
    use super::tests::{chunk, ctab, marker, tempo, track_style};

    /// Ticks per quarter note (`track_style`'s) and per 4/4 bar.
    const PPQ: u32 = 96;
    const BAR: u32 = PPQ * 4;
    /// 100 BPM.
    pub(crate) const TEMPO_US: u32 = 600_000;
    pub(crate) const NAME: &str = "Synthetic Test";

    /// The sections in track order, with their length in bars.
    pub(crate) const SECTIONS: [(&str, u32); 15] = [
        ("Main A", 2),
        ("Main B", 2),
        ("Main C", 2),
        ("Main D", 2),
        ("Fill In AA", 1),
        ("Fill In BB", 1),
        ("Fill In CC", 1),
        ("Fill In DD", 1),
        ("Fill In BA", 1),
        ("Intro A", 1),
        ("Ending A", 1),
        ("Intro B", 2),
        ("Ending B", 2),
        ("Intro C", 1),
        ("Ending C", 1),
    ];

    /// A style part: its channel (0-based, 8-15), Ctab name, voice (bank MSB, LSB,
    /// program), volume, pan, Ctab NTR, NTT, RTR, Autostart, high key and note limit low,
    /// and its notes in each bar: (tick in the bar, length, key). Every note ends inside
    /// its bar, so no note-off lands on the next section's first tick.
    struct Part {
        ch: u8,
        name: &'static [u8; 8],
        voice: (u8, u8, u8),
        volume: u8,
        pan: u8,
        ntr: u8,
        ntt: u8,
        rtr: u8,
        autostart: bool,
        high_key: u8,
        lo: u8,
        notes: &'static [(u32, u32, u8)],
    }

    /// NTR: 0 Root Trans, 1 Root Fixed. NTT (Ctab): 0 Bypass, 1 Melody, 2 Chord, 3 Bass.
    /// RTR: 1 Pitch Shift, 3 Retrigger. Source chord: C Maj.
    const PARTS: [Part; 8] = [
        Part { ch: 8, name: b"Rhythm1 ", voice: (127, 0, 0), volume: 100, pan: 64, ntr: 1, ntt: 0, rtr: 1, autostart: true, high_key: 6, lo: 0, notes: &[(0, 48, 36), (96, 48, 38), (192, 48, 36), (288, 48, 38)] },
        Part { ch: 9, name: b"Rhythm2 ", voice: (127, 0, 0), volume: 90, pan: 64, ntr: 1, ntt: 0, rtr: 1, autostart: true, high_key: 6, lo: 0, notes: &[(0, 40, 42), (48, 40, 42), (96, 40, 42), (144, 40, 42), (192, 40, 42), (240, 40, 42), (288, 40, 42), (336, 40, 46)] },
        Part { ch: 10, name: b"Bass    ", voice: (0, 0, 33), volume: 100, pan: 64, ntr: 0, ntt: 3, rtr: 1, autostart: false, high_key: 7, lo: 28, notes: &[(0, 180, 36), (192, 90, 43), (288, 90, 40)] },
        Part { ch: 11, name: b"Chord1  ", voice: (0, 0, 0), volume: 80, pan: 44, ntr: 1, ntt: 2, rtr: 1, autostart: false, high_key: 6, lo: 40, notes: &[(0, 90, 60), (0, 90, 64), (0, 90, 67), (192, 90, 60), (192, 90, 64), (192, 90, 67)] },
        Part { ch: 12, name: b"Chord2  ", voice: (0, 0, 25), volume: 75, pan: 84, ntr: 1, ntt: 2, rtr: 3, autostart: false, high_key: 6, lo: 40, notes: &[(96, 80, 55), (96, 80, 60), (96, 80, 64), (288, 80, 55), (288, 80, 60), (288, 80, 64)] },
        Part { ch: 13, name: b"Pad     ", voice: (0, 0, 48), volume: 70, pan: 64, ntr: 0, ntt: 2, rtr: 1, autostart: false, high_key: 7, lo: 40, notes: &[(0, 376, 48), (0, 376, 55), (0, 376, 64)] },
        Part { ch: 14, name: b"Phrase1 ", voice: (0, 0, 61), volume: 85, pan: 54, ntr: 0, ntt: 1, rtr: 1, autostart: false, high_key: 7, lo: 40, notes: &[(0, 90, 72), (144, 40, 76), (288, 90, 79)] },
        Part { ch: 15, name: b"Phrase2 ", voice: (0, 0, 73), volume: 80, pan: 74, ntr: 0, ntt: 1, rtr: 3, autostart: false, high_key: 7, lo: 40, notes: &[(192, 180, 84)] },
    ];

    /// Which parts play in a section: the Mains build up, the fills are rhythm, bass and
    /// Chord 1 (plus toms), the Break is kick and bass.
    fn plays(section: &str, ch: u8) -> bool {
        match section {
            "Main A" => matches!(ch, 8..=11 | 13),
            "Main B" => ch != 15,
            "Fill In BA" => matches!(ch, 8 | 10),
            s if s.starts_with("Fill In") => ch <= 11,
            s if s.starts_with("Intro") => matches!(ch, 8 | 9 | 11 | 13 | 14),
            _ => true,
        }
    }

    /// Toms on Rhythm 1 in the second half of a fill's bar.
    const TOMS: [(u32, u32, u8); 4] = [(192, 40, 45), (240, 40, 47), (288, 40, 48), (336, 40, 50)];

    /// Four One Touch Settings: (Right 1, Right 2, Right 3, Left) each (on, GM program,
    /// volume, octave).
    const OTS: [[(bool, u8, u8, i8); 4]; 4] = [
        [(true, 0, 100, 0), (true, 48, 80, -1), (false, 25, 90, 0), (true, 32, 90, 0)],
        [(true, 4, 110, 0), (false, 49, 70, 0), (false, 26, 90, 0), (true, 33, 85, -1)],
        [(true, 16, 95, 1), (false, 50, 90, 0), (true, 61, 75, 1), (false, 34, 90, 0)],
        [(true, 24, 105, 0), (true, 52, 85, 0), (true, 73, 80, 0), (true, 35, 100, 0)],
    ];

    fn ots_bytes() -> Vec<u8> {
        let mut out = Vec::new();
        for (n, setting) in OTS.iter().enumerate() {
            let n = n as u8;
            let mut trk = Vec::new();
            let mut ev = |bytes: &[u8]| {
                trk.push(0);
                trk.extend_from_slice(bytes);
            };
            for (part, &(on, prog, volume, octave)) in setting.iter().enumerate() {
                let (p, ch) = (part as u8, 0xB0 | part as u8);
                for (cc, v) in [(0, 0), (32, 0), (7, volume), (10, 34 + 20 * p), (91, 30 + 10 * n), (93, 10 * n)] {
                    ev(&[ch, cc, v]);
                }
                ev(&[0xC0 | p, prog]);
                // Genos part on/off and octave (`F0 43 73 01 50 08 pp 00|03 vv F7`).
                ev(&[0xF0, 9, 0x43, 0x73, 0x01, 0x50, 0x08, p, 0x00, if on { 0x7F } else { 0x00 }, 0xF7]);
                ev(&[0xF0, 9, 0x43, 0x73, 0x01, 0x50, 0x08, p, 0x03, (0x40 + octave) as u8, 0xF7]);
                if part == 0 {
                    // Right 1: bend range (RPN 0), cutoff, and mono in OTS 1 (XG 08 pp 05 00).
                    for (cc, v) in [(101, 0), (100, 0), (6, 2 + n), (74, 64 + 4 * n)] {
                        ev(&[ch, cc, v]);
                    }
                    if n == 0 {
                        ev(&[0xF0, 8, 0x43, 0x10, 0x4C, 0x08, 0x00, 0x05, 0x00, 0xF7]);
                    }
                }
            }
            trk.extend_from_slice(&[0, 0xFF, 0x2F, 0]);
            out.extend(chunk(b"MTrk", &trk));
        }
        out
    }

    /// The part's SFF1 Ctab record (27 bytes).
    fn part_ctab(p: &Part) -> Vec<u8> {
        let mut d = ctab(p.ntr, p.ntt);
        d[0] = p.ch;
        d[1..9].copy_from_slice(p.name);
        d[9] = p.ch;
        if p.autostart {
            d[13] |= 0x04; // chord mute bit 34
        }
        d[18] = 0; // source root C
        d[19] = 0; // source chord Maj
        d[22] = p.high_key;
        d[23] = p.lo;
        d[25] = p.rtr;
        d
    }

    /// The style file's bytes.
    pub(crate) fn synthetic_style_bytes() -> Vec<u8> {
        let mut name = vec![0xFF, 0x03, NAME.len() as u8];
        name.extend_from_slice(NAME.as_bytes());
        // (absolute tick, event), then sorted (stably) by tick.
        let mut evs: Vec<(u32, Vec<u8>)> = vec![
            (0, name),
            (0, marker("SFF1")),
            (0, tempo(TEMPO_US)),
            (0, vec![0xFF, 0x58, 4, 4, 2, 24, 8]),
            (0, marker("SInt")),
            // XG System On, Reverb type Hall 1, Chorus type Chorus 1.
            (0, vec![0xF0, 8, 0x43, 0x10, 0x4C, 0x00, 0x00, 0x7E, 0x00, 0xF7]),
            (0, vec![0xF0, 9, 0x43, 0x10, 0x4C, 0x02, 0x01, 0x00, 0x01, 0x00, 0xF7]),
            (0, vec![0xF0, 9, 0x43, 0x10, 0x4C, 0x02, 0x01, 0x20, 0x41, 0x00, 0xF7]),
        ];
        for p in &PARTS {
            let cc = 0xB0 | p.ch;
            for (c, v) in [(0, p.voice.0), (32, p.voice.1)] {
                evs.push((0, vec![cc, c, v]));
            }
            evs.push((0, vec![0xC0 | p.ch, p.voice.2]));
            for (c, v) in [(7, p.volume), (10, p.pan), (91, 40), (93, 10)] {
                evs.push((0, vec![cc, c, v]));
            }
        }
        // The sections start a bar after the setup.
        let mut start = BAR;
        for (i, &(section, bars)) in SECTIONS.iter().enumerate() {
            let vel = 70 + 4 * i as u8;
            evs.push((start, marker(section)));
            if let Some(m) = section.strip_prefix("Main ") {
                // Pad expression, per Main.
                let m = m.as_bytes()[0] - b'A';
                evs.push((start, vec![0xB0 | 13, 11, 127 - 10 * m]));
            }
            for bar in (0..bars).map(|b| start + b * BAR) {
                let fill = section.starts_with("Fill In") && section != "Fill In BA";
                for p in PARTS.iter().filter(|p| plays(section, p.ch)) {
                    let toms: &[(u32, u32, u8)] = if fill && p.ch == 8 { &TOMS } else { &[] };
                    for &(t, len, key) in p.notes.iter().chain(toms) {
                        evs.push((bar + t, vec![0x90 | p.ch, key, vel]));
                        evs.push((bar + t + len, vec![0x80 | p.ch, key, 0]));
                    }
                }
            }
            start += bars * BAR;
        }
        // An empty text event at the end, so the last section's length is exact.
        evs.push((start, vec![0xFF, 0x01, 0]));
        evs.sort_by_key(|e| e.0);
        let mut now = 0;
        let deltas: Vec<(u32, Vec<u8>)> = evs
            .into_iter()
            .map(|(t, e)| {
                let d = t - now;
                now = t;
                (d, e)
            })
            .collect();
        let mut out = track_style(&deltas);
        let names: Vec<&str> = SECTIONS.iter().map(|s| s.0).collect();
        let mut cseg = chunk(b"Sdec", names.join(",").as_bytes());
        for p in &PARTS {
            cseg.extend(chunk(b"Ctab", &part_ctab(p)));
        }
        out.extend(chunk(b"CASM", &chunk(b"CSEG", &cseg)));
        out.extend(chunk(b"OTSc", &ots_bytes()));
        out
    }

    #[test]
    fn synthetic_style_parses_with_what_the_launchkey_reaches() {
        use super::{Ntt, SectionId};
        let s = super::parse(&synthetic_style_bytes()).unwrap();
        assert_eq!((s.name.as_str(), s.format.as_str(), s.ppq, s.timesig), (NAME, "SFF1", PPQ as u16, (4, 4)));
        assert_eq!(s.tempo_us, TEMPO_US);
        assert!(s.opaque_sections.is_empty() && s.timing_changes.is_empty());
        for (name, bars) in SECTIONS {
            let id = SectionId::parse(name).unwrap();
            let sec = &s.sections[&id];
            assert_eq!(sec.len, bars * BAR, "{name}");
            assert!(sec.events.iter().any(|e| matches!(e.ev, super::Ev::NoteOn { .. })), "{name} plays");
        }
        for i in 0..3 {
            assert!(s.sections.contains_key(&SectionId::Intro(i)) && s.sections.contains_key(&SectionId::Ending(i)));
        }
        for i in 0..4 {
            assert!(s.sections.contains_key(&SectionId::Main(i)) && s.sections.contains_key(&SectionId::Fill(i)));
        }
        assert!(s.sections.contains_key(&SectionId::Break));
        // One CASM segment for every section, a rule per part.
        assert_eq!(s.casm.len(), 1);
        assert_eq!(s.casm[0].sections.len(), SECTIONS.len());
        let rules = s.rules_for(SectionId::Main(0));
        assert_eq!(rules.keys().copied().collect::<Vec<_>>(), (8..16).collect::<Vec<u8>>());
        assert!(rules[&8].autostart && rules[&9].autostart && !rules[&10].autostart);
        assert_eq!((rules[&10].zones[0].ntt, rules[&10].zones[0].bass_on), (Ntt::Bass, true));
        assert_eq!(rules[&11].zones[0].ntt, Ntt::Chord);
        assert_eq!((rules[&11].src_root, rules[&11].src_type), (0, 0));
        // The channel setup gives every part a voice.
        let sint = s.sint();
        for p in &PARTS {
            let c = &sint.channels[p.ch as usize];
            assert_eq!((c.bank_msb, c.bank_lsb, c.program), (Some(p.voice.0), Some(p.voice.1), Some(p.voice.2)));
        }
        // Four OTS, each different, with voices, on/off and octave as written.
        assert_eq!(s.ots.len(), 4);
        for (o, want) in s.ots.iter().zip(OTS) {
            for (q, (on, prog, volume, octave)) in o.parts.iter().zip(want) {
                assert_eq!((q.on, q.voice, q.volume, q.octave), (on, Some((0, 0, prog)), volume, octave));
            }
        }
        assert_eq!(s.ots[0].parts[0].bend_range, Some(2));
        assert!(s.ots[0].parts[0].xg.iter().any(|x| x == (0x08, 0x05, 0)), "Right 1 mono in OTS 1");
    }
}

//! Yamaha style file (SFF1/SFF2) parser.
//!
//! A style file is a type-0 SMF followed by optional chunks (CASM, OTSc, FNRc, MHhd...).
//! Reference: Wierzba & Bedesem, "Style Files – Introduction and Details" v2.1.

use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// MIDI events
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum Ev {
    NoteOn { ch: u8, key: u8, vel: u8 },
    NoteOff { ch: u8, key: u8 },
    PolyAt { ch: u8, key: u8, val: u8 },
    Cc { ch: u8, cc: u8, val: u8 },
    Pc { ch: u8, prog: u8 },
    ChanAt { ch: u8, val: u8 },
    Bend { ch: u8, val: u16 },
    /// Full sysex message including the leading F0.
    Sysex(Vec<u8>),
    Meta { ty: u8, data: Vec<u8> },
}

impl Ev {
    pub fn channel(&self) -> Option<u8> {
        match *self {
            Ev::NoteOn { ch, .. }
            | Ev::NoteOff { ch, .. }
            | Ev::PolyAt { ch, .. }
            | Ev::Cc { ch, .. }
            | Ev::Pc { ch, .. }
            | Ev::ChanAt { ch, .. }
            | Ev::Bend { ch, .. } => Some(ch),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimedEv {
    pub tick: u32,
    pub ev: Ev,
}

// ---------------------------------------------------------------------------
// CASM
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ntr {
    RootTrans,
    RootFixed,
    Guitar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ntt {
    Bypass,
    Melody,
    Chord,
    /// Old SFF1 "Bass" table; equivalent to Melody with Bass On.
    Bass,
    MelodicMinor,
    MelodicMinor5,
    HarmonicMinor,
    HarmonicMinor5,
    NaturalMinor,
    NaturalMinor5,
    Dorian,
    Dorian5,
    GuitarAllPurpose,
    GuitarStroke,
    GuitarArpeggio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rtr {
    Stop,
    PitchShift,
    PitchShiftToRoot,
    Retrigger,
    RetriggerToRoot,
    NoteGenerator,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Zone {
    pub ntr: Ntr,
    pub ntt: Ntt,
    pub high_key: u8,
    pub lo: u8,
    pub hi: u8,
    pub rtr: Rtr,
    /// NTT Bass On: this zone follows slash chords. SFF2 stores it per zone (bit 7 of
    /// each zone's NTT byte); SFF1 and Cntt set every zone alike.
    pub bass_on: bool,
}

/// Channel rule for one source channel within one or more sections (a Ctab/Ctb2 record).
#[allow(dead_code)] // editable/sff2 kept for dump/debugging
#[derive(Debug, Clone)]
pub struct ChannelRule {
    pub src_ch: u8,
    pub name: String,
    /// Destination accompaniment channel, 8..=15 (MIDI channels 9..16).
    pub dest_ch: u8,
    pub editable: bool,
    /// Bit n set = chord root n (C=0) plays.
    pub note_mute: u16,
    /// Bit n set = chord type n (see `theory::ChordType` ids) plays.
    pub chord_mute: u64,
    pub autostart: bool,
    pub src_root: u8,
    pub src_type: u8,
    /// Notes below `mid_lo` use zones[0], above `mid_hi` use zones[2].
    pub mid_lo: u8,
    pub mid_hi: u8,
    pub zones: [Zone; 3],
    pub sff2: bool,
}

impl ChannelRule {
    pub fn zone_for(&self, key: u8) -> &Zone {
        if key < self.mid_lo {
            &self.zones[0]
        } else if key > self.mid_hi {
            &self.zones[2]
        } else {
            &self.zones[1]
        }
    }

    /// Rule used when a style has no CASM data for a channel: channels 9..16 play
    /// through with the conventional table for their part, source chord CMaj7.
    pub fn default_for(ch: u8) -> ChannelRule {
        let (ntr, ntt, bass) = match ch {
            8 | 9 => (Ntr::RootFixed, Ntt::Bypass, false),
            10 => (Ntr::RootTrans, Ntt::Melody, true),
            11 | 12 | 13 => (Ntr::RootFixed, Ntt::Chord, false),
            _ => (Ntr::RootTrans, Ntt::Melody, false),
        };
        let z = Zone { ntr, ntt, high_key: 6, lo: 0, hi: 127, rtr: Rtr::PitchShift, bass_on: bass };
        ChannelRule {
            src_ch: ch,
            name: String::new(),
            dest_ch: ch,
            editable: true,
            note_mute: 0x0FFF,
            chord_mute: (1u64 << 34) - 1,
            autostart: false,
            src_root: 0,
            src_type: 2,
            mid_lo: 0,
            mid_hi: 127,
            zones: [z; 3],
            sff2: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Cseg {
    pub sections: Vec<String>,
    pub rules: Vec<ChannelRule>,
}

// ---------------------------------------------------------------------------
// Style
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SectionId {
    Intro(u8),  // 0..=3 (A..D)
    Main(u8),   // 0..=3
    Fill(u8),   // 0..=3 (AA..DD)
    Break,      // "Fill In BA"
    Ending(u8), // 0..=3
}

impl SectionId {
    pub fn parse(s: &str) -> Option<SectionId> {
        let letter = |c: &str| -> Option<u8> {
            match c {
                "A" => Some(0),
                "B" => Some(1),
                "C" => Some(2),
                "D" => Some(3),
                _ => None,
            }
        };
        let s = s.trim();
        if let Some(r) = s.strip_prefix("Intro ") {
            return letter(r).map(SectionId::Intro);
        }
        if let Some(r) = s.strip_prefix("Main ") {
            return letter(r).map(SectionId::Main);
        }
        if let Some(r) = s.strip_prefix("Ending ") {
            return letter(r).map(SectionId::Ending);
        }
        if let Some(r) = s.strip_prefix("Fill In ") {
            return match r {
                "AA" => Some(SectionId::Fill(0)),
                "BB" => Some(SectionId::Fill(1)),
                "CC" => Some(SectionId::Fill(2)),
                "DD" => Some(SectionId::Fill(3)),
                "BA" => Some(SectionId::Break),
                _ => None,
            };
        }
        None
    }

    pub fn name(&self) -> String {
        const L: [char; 4] = ['A', 'B', 'C', 'D'];
        match *self {
            SectionId::Intro(i) => format!("Intro {}", L[i as usize]),
            SectionId::Main(i) => format!("Main {}", L[i as usize]),
            SectionId::Fill(i) => format!("Fill In {0}{0}", L[i as usize]),
            SectionId::Break => "Fill In BA".into(),
            SectionId::Ending(i) => format!("Ending {}", L[i as usize]),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Section {
    pub id: SectionId,
    pub start: u32,
    pub len: u32,
    /// Events with ticks relative to section start.
    pub events: Vec<TimedEv>,
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
pub fn parse_ots(data: &[u8]) -> Vec<Ots> {
    let mut out = Vec::new();
    let mut p = 0;
    while p + 8 <= data.len() && &data[p..p + 4] == b"MTrk" {
        let len = be32(&data[p + 4..p + 8]);
        let end = (p + 8 + len).min(data.len());
        let mut ots = Ots::default();
        let mut bank = [(0u8, 0u8); 4];
        for part in ots.parts.iter_mut() {
            part.volume = 100;
        }
        if let Ok(evs) = parse_track(&data[p + 8..end]) {
            for e in evs {
                match e.ev {
                    Ev::Cc { ch, cc, val } if ch < 4 => match cc {
                        0 => bank[ch as usize].0 = val,
                        32 => bank[ch as usize].1 = val,
                        7 => ots.parts[ch as usize].volume = val,
                        _ => {}
                    },
                    Ev::Pc { ch, prog } if ch < 4 => {
                        let (m, l) = bank[ch as usize];
                        ots.parts[ch as usize].voice = Some((m, l, prog));
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
    /// Section markers in track order.
    marks: Vec<(u32, SectionId)>,
    end_tick: u32,
}

fn scan_meta(events: &[TimedEv]) -> Meta {
    let mut m = Meta { name: String::new(), format: String::new(), tempo_us: 500_000, timesig: (4, 4), marks: Vec::new(), end_tick: 0 };
    for e in events {
        m.end_tick = m.end_tick.max(e.tick);
        if let Ev::Meta { ty, data } = &e.ev {
            // Tempo and time signature count only before the first section.
            let before_sections = m.marks.is_empty();
            match *ty {
                0x06 => {
                    let t = String::from_utf8_lossy(data).to_string();
                    if t == "SFF1" || t == "SFF2" {
                        m.format = t;
                    } else if let Some(id) = SectionId::parse(&t) {
                        m.marks.push((e.tick, id));
                    }
                }
                // Names are often NUL-padded to a fixed width; the name ends at the first NUL.
                0x03 if m.name.is_empty() => {
                    let text = data.split(|&b| b == 0).next().unwrap_or_default();
                    m.name = String::from_utf8_lossy(text).trim().to_string()
                }
                0x51 if data.len() == 3 && data[..] != [0, 0, 0] && before_sections => {
                    m.tempo_us = ((data[0] as u32) << 16) | ((data[1] as u32) << 8) | data[2] as u32
                }
                // Ignore impossible signatures (n/0, 2^8+) so bar length stays non-zero.
                0x58 if data.len() >= 2 && data[0] > 0 && data[1] < 8 && before_sections => {
                    m.timesig = (data[0], 1u8 << data[1])
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
    if m.marks.is_empty() {
        bail!("no section markers found");
    }
    let mut sections: Vec<SectionId> = m.marks.iter().map(|&(_, id)| id).collect();
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
    let Meta { name, format: fmt, tempo_us, timesig, marks, end_tick } = scan_meta(&events);
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
    for (i, &(start, id)) in marks.iter().enumerate() {
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
        sections.insert(id, Section { id, start, len, events: evs });
    }

    let ots = other_chunks.iter().find(|(id, _)| id == "OTSc").map(|(_, d)| parse_ots(d)).unwrap_or_default();
    Ok(Style { name, format: fmt, ppq, tempo_us, timesig, init, sections, casm, ots, other_chunks })
}


#[cfg(test)]
mod tests {
    use super::*;
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

    fn chunk(id: &[u8], body: &[u8]) -> Vec<u8> {
        let mut v = id.to_vec();
        v.extend_from_slice(&(body.len() as u32).to_be_bytes());
        v.extend_from_slice(body);
        v
    }

    /// Smallest style the parser accepts: one Main A bar with a note on ch 12 plus a CASM
    /// segment holding `rec`.
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

    /// Drive every key through every chord for each rule, the way the engine would.
    fn exercise(style: &Style) {
        for seg in &style.casm {
            for r in &seg.rules {
                assert!((r.src_type as usize) < NUM_TYPES, "src_type {} survived parsing", r.src_type);
                assert!((8..16).contains(&r.dest_ch), "dest_ch {} survived parsing", r.dest_ch);
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

    #[test]
    fn malformed_ctab_transposes_without_panicking() {
        // Out-of-range source types with garbage zone bytes (NTR/NTT/RTR enums, High Key and
        // inverted note limits), as a whole style file through the real parser.
        for src_type in [34u8, 35, 36, 37, 38, 63, 64, 0x80, 0xFF] {
            for zone in [0x00u8, 0x07, 0x7F, 0x80, 0xFF] {
                let s = parse(&style_bytes(b"Ctb2", &ctb2(src_type, zone))).unwrap();
                exercise(&s);
                let s = parse(&style_bytes(b"Ctab", &ctb2(src_type, zone)[..27])).unwrap();
                exercise(&s);
            }
        }
    }

    /// A 27-byte SFF1 Ctab record for src ch 12 (see `ctb2`) with the given NTR and NTT bytes.
    fn ctab(ntr: u8, ntt: u8) -> Vec<u8> {
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

    #[test]
    fn corpus_cntt_bass_parts_follow_slash_chords() {
        // Every corpus Cntt style writes its Ctab "Bass" channel's Cntt as plain Melody. The Bass
        // part must still carry Bass On, as 193 of 194 SFF2 corpus styles give their Bass part.
        let (mut found, mut bass_parts) = (0, 0);
        for p in crate::library::corpus_styles() {
            let Ok(bytes) = std::fs::read(&p) else { continue };
            if !bytes.windows(4).any(|w| w == b"Cntt") {
                continue;
            }
            let s = parse(&bytes).unwrap();
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

    #[test]
    fn truncated_and_corrupted_files_do_not_panic() {
        let good = style_bytes(b"Ctb2", &ctb2(0xFF, 0xFF));
        assert!(parse(&good).is_ok());
        for n in 0..good.len() {
            if let Ok(s) = parse(&good[..n]) {
                exercise(&s);
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
                    exercise(&s);
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
    #[test]
    fn sint_structures_every_corpus_style() {
        let (mut styles, mut resets, mut pending) = (0, 0, 0);
        for p in crate::library::corpus_styles() {
            let style = Style::load(&p).unwrap_or_else(|e| panic!("{}: {e:#}", p.display()));
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
}

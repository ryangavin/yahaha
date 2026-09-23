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
    pub bass_on: bool,
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
        let z = Zone { ntr, ntt, high_key: 6, lo: 0, hi: 127, rtr: Rtr::PitchShift };
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
            bass_on: bass,
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
                            0x03 => part.octave = (v[8] as i8 - 0x40).clamp(-2, 2),
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

    pub fn ticks_per_bar(&self) -> u32 {
        let (n, d) = self.timesig;
        (self.ppq as u32 * 4 * n as u32) / d.max(1) as u32
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

fn parse_track(data: &[u8]) -> Result<Vec<TimedEv>> {
    let mut r = Reader { b: data, p: 0 };
    let mut out = Vec::new();
    let mut tick: u32 = 0;
    let mut running: u8 = 0;
    while r.p < data.len() {
        tick += r.vlq()?;
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
                        let key = r.u8()?;
                        let _ = r.u8()?;
                        Ev::NoteOff { ch, key }
                    }
                    0x90 => {
                        let key = r.u8()?;
                        let vel = r.u8()?;
                        if vel == 0 {
                            Ev::NoteOff { ch, key }
                        } else {
                            Ev::NoteOn { ch, key, vel }
                        }
                    }
                    0xA0 => Ev::PolyAt { ch, key: r.u8()?, val: r.u8()? },
                    0xB0 => Ev::Cc { ch, cc: r.u8()?, val: r.u8()? },
                    0xC0 => Ev::Pc { ch, prog: r.u8()? },
                    0xD0 => Ev::ChanAt { ch, val: r.u8()? },
                    _ => {
                        let lo = r.u8()? as u16;
                        let hi = r.u8()? as u16;
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

fn decode_ntt_old(v: u8) -> Ntt {
    match v {
        0 => Ntt::Bypass,
        1 => Ntt::Melody,
        2 => Ntt::Chord,
        3 => Ntt::Bass,
        4 => Ntt::MelodicMinor,
        5 => Ntt::HarmonicMinor,
        _ => Ntt::Melody,
    }
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

fn parse_zone(b: &[u8]) -> (Zone, bool) {
    let ntr = decode_ntr(b[0]);
    let z = Zone {
        ntr,
        ntt: decode_ntt_new(b[1], ntr),
        high_key: b[2] % 12,
        lo: b[3] & 0x7F,
        hi: b[4] & 0x7F,
        rtr: decode_rtr(b[5]),
    };
    (z, b[1] & 0x80 != 0)
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
    let mut rule = ChannelRule {
        src_ch: d[0] & 0x0F,
        name,
        dest_ch: d[9] & 0x0F,
        editable: d[10] == 0,
        note_mute,
        chord_mute: cm & ((1u64 << 34) - 1),
        autostart: cm & (1u64 << 34) != 0,
        src_root: d[18] % 12,
        src_type: d[19],
        mid_lo: 0,
        mid_hi: 127,
        zones: [Zone {
            ntr: Ntr::RootTrans,
            ntt: Ntt::Bypass,
            high_key: 0,
            lo: 0,
            hi: 127,
            rtr: Rtr::PitchShift,
        }; 3],
        bass_on: false,
        sff2,
    };
    if sff2 {
        if d.len() < 40 {
            bail!("Ctb2 record too short ({} bytes)", d.len());
        }
        rule.mid_lo = d[20];
        rule.mid_hi = d[21];
        let (lz, lb) = parse_zone(&d[22..28]);
        let (mz, mb) = parse_zone(&d[28..34]);
        let (hz, hb) = parse_zone(&d[34..40]);
        rule.zones = [lz, mz, hz];
        rule.bass_on = mb || lb || hb;
    } else {
        let ntr = decode_ntr(d[20]);
        let ntt = decode_ntt_old(d[21]);
        let z = Zone {
            ntr,
            ntt,
            high_key: d[22] % 12,
            lo: d[23] & 0x7F,
            hi: d[24] & 0x7F,
            rtr: decode_rtr(d[25]),
        };
        rule.zones = [z; 3];
        rule.bass_on = ntt == Ntt::Bass;
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

fn parse_casm(data: &[u8]) -> Result<Vec<Cseg>> {
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
                b"Cntt" => {
                    if d.len() >= 2 {
                        let ch = d[0] & 0x0F;
                        if let Some(r) = seg.rules.iter_mut().find(|r| r.src_ch == ch) {
                            let ntr = r.zones[1].ntr;
                            let ntt = decode_ntt_new(d[1], ntr);
                            for z in r.zones.iter_mut() {
                                z.ntt = ntt;
                            }
                            r.bass_on = d[1] & 0x80 != 0;
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
    if bytes.len() < 22 || &bytes[0..4] != b"MThd" {
        bail!("not a MIDI/style file");
    }
    let hlen = be32(&bytes[4..8]);
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
    let mut p = 8 + hlen;
    if &bytes[p..p + 4] != b"MTrk" {
        bail!("missing MTrk");
    }
    let tlen = be32(&bytes[p + 4..p + 8]);
    let track = &bytes[p + 8..(p + 8 + tlen).min(bytes.len())];
    let events = parse_track(track)?;
    p += 8 + tlen;

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

fn build_style(
    ppq: u16,
    events: Vec<TimedEv>,
    casm: Vec<Cseg>,
    other_chunks: Vec<(String, Vec<u8>)>,
) -> Result<Style> {
    let mut name = String::new();
    let mut fmt = String::new();
    let mut tempo_us = 500_000;
    let mut timesig = (4u8, 4u8);
    let mut end_tick = 0;

    // Find section markers.
    let mut marks: Vec<(u32, SectionId)> = Vec::new();
    let mut first_section_tick = None;
    for e in &events {
        end_tick = end_tick.max(e.tick);
        if let Ev::Meta { ty, data } = &e.ev {
            match *ty {
                0x06 => {
                    let t = String::from_utf8_lossy(data).to_string();
                    if t == "SFF1" || t == "SFF2" {
                        fmt = t;
                    } else if let Some(id) = SectionId::parse(&t) {
                        marks.push((e.tick, id));
                        first_section_tick.get_or_insert(e.tick);
                    }
                }
                0x03 if name.is_empty() => name = String::from_utf8_lossy(data).trim().to_string(),
                0x51 if data.len() == 3 && first_section_tick.is_none() => {
                    tempo_us = ((data[0] as u32) << 16) | ((data[1] as u32) << 8) | data[2] as u32
                }
                0x58 if data.len() >= 2 && first_section_tick.is_none() => {
                    timesig = (data[0], 1u8 << data[1])
                }
                _ => {}
            }
        }
    }
    if marks.is_empty() {
        bail!("no section markers found");
    }
    let first = first_section_tick.unwrap();

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
        let tpb = (ppq as u32 * 4 * timesig.0 as u32) / timesig.1.max(1) as u32;
        let len = (((end - start) + tpb / 2) / tpb).max(1) * tpb;
        sections.insert(id, Section { id, start, len, events: evs });
    }

    let ots = other_chunks.iter().find(|(id, _)| id == "OTSc").map(|(_, d)| parse_ots(d)).unwrap_or_default();
    Ok(Style { name, format: fmt, ppq, tempo_us, timesig, init, sections, casm, ots, other_chunks })
}


#[cfg(test)]
mod tests {
    use super::*;

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
}

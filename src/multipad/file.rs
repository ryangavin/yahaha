//! Multi Pad bank (.pad) parser.
//!
//! # The format
//!
//! Yamaha does not publish the .pad format (docs/genos-features.md §E, §G.6). The one
//! public description is Jørgen Sørensen's "Multi Pad Format" article
//! (<http://www.jososoft.dk/yamaha/articles/multipad.htm>, The Unofficial YAMAHA Keyboard
//! Resource Site). It describes the Tyros layout and says other models differ slightly:
//!
//! * A **type 1 SMF with 5 tracks**, 1920 ppq on Tyros (96 on older PSRs such as the
//!   PSR-740).
//! * **Track 0** holds 10 text events (meta 01H), all at tick 0:
//!   1. `CMxxxx`: **Chord Match** of pads 1..4, one `0`/`1` character per pad.
//!   2. `RPxxxx`: **Repeat** of pads 1..4, the same way. This is where Repeat lives.
//!   3. to 6. `N<y><name>...`, 52 bytes padded with blanks: pad `y`'s name. The article's
//!      example is `N2HipHop1 2 ....`: the pad number appears again after the name, so
//!      the parser drops the name from the last blank-separated token starting with `y`.
//!   7. to 10. `I<y>...` (e.g. `I1S375`): the pad's panel image reference.
//! * **Tracks 1..4** are the pads, one measure each, voice setup plus notes. Track `n`
//!   plays on MIDI channel `n` (pad 1 on channel 1, and so on).
//!
//! The article mentions no CASM chunk: Tyros pads have no per-pad chord rule, only the
//! Chord Match switch. The parser still reads a CASM chunk if one is present (the
//! style-file analogy; it gives names and rules), but the CM/RP text events win. Nothing
//! here has been checked against a real bank yet (Genos may differ from Tyros): every
//! unknown chunk is kept raw in [`PadBank::other_chunks`] and every track-0 text event in
//! [`PadBank::texts`], and `yahaha pad <file>` prints both, so the first real bank shows
//! where this is wrong.
//!
//! What the parser accepts:
//!
//! * **Type 1 SMF** (the documented layout): with a conductor track 0 (no channel
//!   events), track `n` is pad `n`; otherwise track `n` is pad `n + 1`. An empty track
//!   leaves its pad empty and does not shift the pads after it. The pad name comes from the
//!   `N` text event, else the CASM record, else the track name (meta 03H).
//! * **Type 0 SMF** (a fallback, not described anywhere): each pad is one MIDI channel.
//!   With a CASM chunk, its Ctab/Ctb2 records assign pads in record order; without one,
//!   the channels in use, lowest first, are pads 1..4. CM/RP/N events apply by pad number.
//!
//! Per-pad flags:
//!
//! * **Chord Match**: the `CM` event; else inferred from a CASM rule (on unless every zone
//!   is NTT Bypass with NTR Root Fixed); else unknown.
//! * **Repeat**: the `RP` event; else unknown.
//!
//! Unknown values stay `None` here; [`Pad::repeat_or_default`] and
//! [`Pad::chord_match_or_default`] give the player's defaults.

use crate::sff::{self, ChannelRule, Ev, Ntr, Ntt, TimedEv};
use anyhow::{bail, Context, Result};

/// Pads per bank.
pub const PADS: usize = 4;

/// How the pads were found in the file (reported by the dump).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// Type 0: one pad per channel, assigned by the CASM records.
    ChannelsByCasm,
    /// Type 0: one pad per channel, lowest channel first (no CASM).
    ChannelsAscending,
    /// Type 1: one pad per track.
    Tracks,
}

#[derive(Debug, Clone)]
pub struct Pad {
    pub name: String,
    /// The source MIDI channel (0-based) the phrase is recorded on.
    pub channel: u8,
    /// The phrase: this pad's channel events and sysex, ticks from the top of the pad.
    pub events: Vec<TimedEv>,
    /// Phrase length in file ticks: past the last event, rounded up to a whole beat so a
    /// repeating pad stays on the beat (RM p.65). At least one beat.
    pub len: u32,
    /// Repeat flag, if the file says (see the module docs).
    pub repeat: Option<bool>,
    /// Chord Match flag, if the file says (see the module docs).
    pub chord_match: Option<bool>,
    /// The CASM rule for this pad's channel, if the file has one.
    pub rule: Option<ChannelRule>,
    /// The panel image reference from the `I<y>` text event (e.g. `S375`), if any.
    pub image: Option<String>,
}

impl Pad {
    pub fn repeat_or_default(&self) -> bool {
        self.repeat.unwrap_or(false)
    }

    /// Chord Match, off when the file does not say (a phrase as written is the safe default).
    pub fn chord_match_or_default(&self) -> bool {
        self.chord_match.unwrap_or(false)
    }

    /// Number of note-ons in the phrase.
    pub fn notes(&self) -> usize {
        self.events.iter().filter(|e| matches!(e.ev, Ev::NoteOn { .. })).count()
    }
}

#[derive(Debug, Clone)]
pub struct PadBank {
    /// The first track name (meta 03H) in the file; empty if none.
    pub name: String,
    /// Ticks per quarter note.
    pub ppq: u16,
    /// Microseconds per quarter note (500000 if the file sets none).
    pub tempo_us: u32,
    pub timesig: (u8, u8),
    pub layout: Layout,
    pub pads: [Option<Pad>; PADS],
    /// Chunks after the MIDI data other than CASM, raw (id, bytes).
    pub other_chunks: Vec<(String, Vec<u8>)>,
    /// Every text event (meta 01H) of the first track, as written, for the dump.
    pub texts: Vec<String>,
}

impl PadBank {
    pub fn load(path: &std::path::Path) -> Result<PadBank> {
        let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        parse(&bytes).with_context(|| format!("parsing {}", path.display()))
    }

    pub fn bpm(&self) -> f64 {
        60_000_000.0 / self.tempo_us.max(1) as f64
    }
}

/// Chord Match as a pad's CASM rule implies it: off only for the rule that plays every
/// note as written in every zone.
pub fn rule_matches_chords(rule: &ChannelRule) -> bool {
    !rule.zones.iter().all(|z| z.ntt == Ntt::Bypass && z.ntr == Ntr::RootFixed)
}

fn be32(b: &[u8]) -> usize {
    u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize
}

/// Parse a Multi Pad bank.
pub fn parse(bytes: &[u8]) -> Result<PadBank> {
    if bytes.len() < 14 || &bytes[0..4] != b"MThd" {
        bail!("not a MIDI/Multi Pad file");
    }
    let hlen = be32(&bytes[4..8]);
    if hlen < 6 || 8 + hlen > bytes.len() {
        bail!("bad MThd length {hlen}");
    }
    let h = &bytes[8..8 + hlen];
    let format = u16::from_be_bytes([h[0], h[1]]);
    let ntracks = u16::from_be_bytes([h[2], h[3]]) as usize;
    let ppq = u16::from_be_bytes([h[4], h[5]]);
    if format > 1 {
        bail!("Multi Pad MIDI must be type 0 or 1 (got type {format})");
    }
    if ppq & 0x8000 != 0 {
        bail!("SMPTE time division not supported");
    }
    if ppq == 0 {
        bail!("zero ticks per quarter note");
    }

    // MTrk chunks first (type 0 reads only the first, as instruments do), then the rest.
    let mut p = 8 + hlen;
    let mut tracks: Vec<Vec<TimedEv>> = Vec::new();
    let mut casm = Vec::new();
    let mut other_chunks = Vec::new();
    while p + 8 <= bytes.len() {
        let id = &bytes[p..p + 4];
        let len = be32(&bytes[p + 4..p + 8]);
        let end = p.saturating_add(8).saturating_add(len).min(bytes.len());
        let data = &bytes[p + 8..end];
        let want_track = if format == 0 { tracks.is_empty() } else { tracks.len() < ntracks.max(1) };
        match id {
            b"MTrk" if want_track => {
                tracks.push(sff::parse_track(data).with_context(|| format!("track {}", tracks.len() + 1))?)
            }
            b"CASM" => casm = sff::parse_casm(data).context("CASM")?,
            _ => other_chunks.push((String::from_utf8_lossy(id).into_owned(), data.to_vec())),
        }
        p = end;
    }
    if tracks.is_empty() {
        bail!("missing MTrk");
    }

    // Header facts: the name from the first track, tempo and time signature from any.
    let name = tracks[0]
        .iter()
        .find_map(|e| match &e.ev {
            Ev::Meta { ty: 0x03, data } => Some(meta_text(data)),
            _ => None,
        })
        .unwrap_or_default();
    let mut tempo_us = 500_000;
    let mut timesig = (4, 4);
    for e in tracks.iter().flatten() {
        if let Ev::Meta { ty, data } = &e.ev {
            match *ty {
                0x51 if data.len() == 3 && data[..] != [0, 0, 0] => {
                    tempo_us = ((data[0] as u32) << 16) | ((data[1] as u32) << 8) | data[2] as u32
                }
                0x58 if data.len() >= 2 && data[0] > 0 && data[1] < 8 => timesig = (data[0], 1u8 << data[1]),
                _ => {}
            }
        }
    }

    // The Yamaha header text events (module docs): CM, RP, N<y>, I<y>.
    let texts: Vec<String> = tracks[0]
        .iter()
        .filter_map(|e| match &e.ev {
            Ev::Meta { ty: 0x01, data } => Some(String::from_utf8_lossy(data).into_owned()),
            _ => None,
        })
        .collect();
    let header = Header::from_texts(&texts);

    // All the rules in the CASM, first record per channel wins, in record order.
    let mut rules: Vec<ChannelRule> = Vec::new();
    for r in casm.iter().flat_map(|seg| seg.rules.iter()) {
        if !rules.iter().any(|x| x.src_ch == r.src_ch) {
            rules.push(r.clone());
        }
    }
    let rule_for = |ch: u8| rules.iter().find(|r| r.src_ch == ch).cloned();

    let beat = ppq as u32;
    let mut pads: [Option<Pad>; PADS] = Default::default();
    let layout;
    if format == 1 && tracks.len() > 1 {
        layout = Layout::Tracks;
        // A conductor track 0 (no channel events) is not a pad: then track n is pad n.
        let first = usize::from(tracks[0].iter().all(|e| e.ev.channel().is_none()));
        for (slot, t) in tracks.iter().skip(first).take(PADS).enumerate() {
            let Some(ch) = t.iter().find_map(|e| e.ev.channel()) else { continue };
            let track_name = t.iter().find_map(|e| match &e.ev {
                Ev::Meta { ty: 0x03, data } => Some(meta_text(data)),
                _ => None,
            });
            let events: Vec<TimedEv> =
                t.iter().filter(|e| e.ev.channel().is_some() || matches!(e.ev, Ev::Sysex(_))).cloned().collect();
            let end = t.iter().map(|e| e.tick).max().unwrap_or(0);
            let rule = rule_for(ch);
            let name = rule.as_ref().map(|r| r.name.clone()).filter(|n| !n.is_empty()).or(track_name);
            pads[slot] = Some(make_pad(name, slot, ch, events, end, beat, rule));
        }
    } else {
        let track = &tracks[0];
        let mut used: Vec<u8> = track.iter().filter_map(|e| e.ev.channel()).collect();
        used.sort_unstable();
        used.dedup();
        let channels: Vec<u8> = if rules.is_empty() {
            layout = Layout::ChannelsAscending;
            used.iter().copied().take(PADS).collect()
        } else {
            layout = Layout::ChannelsByCasm;
            rules.iter().map(|r| r.src_ch).take(PADS).collect()
        };
        // Sysex has no channel: it goes to the first pad only, so it is sent once.
        let first_ch = channels.first().copied();
        for (slot, &ch) in channels.iter().enumerate() {
            let events: Vec<TimedEv> = track
                .iter()
                .filter(|e| match e.ev {
                    Ev::Sysex(_) => Some(ch) == first_ch,
                    _ => e.ev.channel() == Some(ch),
                })
                .cloned()
                .collect();
            let rule = rule_for(ch);
            if events.is_empty() && rule.is_none() {
                continue;
            }
            let end = events.iter().map(|e| e.tick).max().unwrap_or(0);
            let name = rule.as_ref().map(|r| r.name.clone()).filter(|n| !n.is_empty());
            pads[slot] = Some(make_pad(name, slot, ch, events, end, beat, rule));
        }
    }

    for (slot, pad) in pads.iter_mut().enumerate() {
        let Some(pad) = pad else { continue };
        if let Some(cm) = header.chord_match[slot] {
            pad.chord_match = Some(cm);
        }
        pad.repeat = header.repeat[slot];
        if let Some(n) = header.names[slot].clone() {
            pad.name = n;
        }
        pad.image = header.images[slot].clone();
    }

    Ok(PadBank { name, ppq, tempo_us, timesig, layout, pads, other_chunks, texts })
}

fn make_pad(
    name: Option<String>,
    slot: usize,
    channel: u8,
    events: Vec<TimedEv>,
    last_tick: u32,
    beat: u32,
    rule: Option<ChannelRule>,
) -> Pad {
    // The last event rounded up to a whole beat; an event on the end tick still belongs to
    // the pass (an off, a controller), but a note-on there needs one more beat to sound.
    let last_on = events.iter().filter(|e| matches!(e.ev, Ev::NoteOn { .. })).map(|e| e.tick).max();
    let mut len = last_tick.div_ceil(beat).max(1).saturating_mul(beat);
    if last_on.is_some_and(|t| t >= len) {
        len = len.saturating_add(beat);
    }
    Pad {
        name: name.unwrap_or_else(|| format!("Pad {}", slot + 1)),
        channel,
        events,
        len,
        repeat: None,
        chord_match: rule.as_ref().map(rule_matches_chords),
        rule,
        image: None,
    }
}

/// What the track-0 text events say (module docs).
#[derive(Debug, Default)]
struct Header {
    chord_match: [Option<bool>; PADS],
    repeat: [Option<bool>; PADS],
    names: [Option<String>; PADS],
    images: [Option<String>; PADS],
}

impl Header {
    fn from_texts(texts: &[String]) -> Header {
        let mut h = Header::default();
        let flags = |rest: &str| {
            let mut f = [None; PADS];
            for (slot, c) in rest.chars().take(PADS).enumerate() {
                f[slot] = match c {
                    '0' => Some(false),
                    '1' => Some(true),
                    _ => None,
                };
            }
            f
        };
        for t in texts {
            let t = t.trim_end_matches(['\0', ' ']);
            let mut chars = t.chars();
            let (tag, num) = (chars.next(), chars.next());
            // `N<y>` / `I<y>`: y is the pad number 1..4, then the payload.
            let pad = num.and_then(|c| c.to_digit(10)).map(|d| d as usize).filter(|d| (1..=PADS).contains(d));
            let payload = chars.as_str();
            if let Some(rest) = t.strip_prefix("CM") {
                h.chord_match = flags(rest);
            } else if let Some(rest) = t.strip_prefix("RP") {
                h.repeat = flags(rest);
            } else if let (Some('N'), Some(d)) = (tag, pad) {
                let name = pad_name(payload, d);
                if !name.is_empty() {
                    h.names[d - 1] = Some(name);
                }
            } else if let (Some('I'), Some(d)) = (tag, pad) {
                h.images[d - 1] = Some(payload.trim().to_string());
            }
        }
        h
    }
}

/// The name in an `N<y>` event, from the text after `N<y>`. The article's example
/// `HipHop1 2 ....` repeats the pad number after the name, so the name ends before the last
/// blank-separated token (not the first) that starts with that number.
fn pad_name(payload: &str, pad: usize) -> String {
    let payload = payload.trim();
    let num = char::from_digit(pad as u32, 10).unwrap_or('?');
    let mut cut = payload.len();
    let mut pos = 0;
    for (i, tok) in payload.split(' ').enumerate() {
        if i > 0 && tok.starts_with(num) {
            cut = pos;
        }
        pos += tok.len() + 1;
    }
    payload[..cut].trim().to_string()
}

fn meta_text(data: &[u8]) -> String {
    let text = data.split(|&b| b == 0).next().unwrap_or_default();
    String::from_utf8_lossy(text).trim().to_string()
}

// ---------------------------------------------------------------------------
// Synthetic files (tests only: no Yamaha data is ever committed)
// ---------------------------------------------------------------------------

#[cfg(test)]
pub(crate) mod build {
    //! A tiny SMF writer for synthetic Multi Pad banks.

    fn vlq(mut v: u32, out: &mut Vec<u8>) {
        let mut buf = [0u8; 4];
        let mut n = 0;
        loop {
            buf[n] = (v & 0x7F) as u8;
            n += 1;
            v >>= 7;
            if v == 0 {
                break;
            }
        }
        for i in (0..n).rev() {
            out.push(buf[i] | if i > 0 { 0x80 } else { 0 });
        }
    }

    /// An MTrk chunk from (absolute tick, raw event bytes incl. status). Meta events are
    /// written as `[0xFF, ty, data...]` and get their length inserted.
    pub fn track(events: &[(u32, Vec<u8>)]) -> Vec<u8> {
        let mut body = Vec::new();
        let mut last = 0;
        for (t, ev) in events {
            vlq(t - last, &mut body);
            last = *t;
            if ev[0] == 0xFF {
                body.extend_from_slice(&ev[..2]);
                vlq(ev.len() as u32 - 2, &mut body);
                body.extend_from_slice(&ev[2..]);
            } else if ev[0] == 0xF0 {
                body.push(0xF0);
                vlq(ev.len() as u32 - 1, &mut body);
                body.extend_from_slice(&ev[1..]);
            } else {
                body.extend_from_slice(ev);
            }
        }
        body.extend_from_slice(&[0, 0xFF, 0x2F, 0]);
        chunk(b"MTrk", &body)
    }

    pub fn chunk(id: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut v = id.to_vec();
        v.extend_from_slice(&(data.len() as u32).to_be_bytes());
        v.extend_from_slice(data);
        v
    }

    pub fn header(format: u16, ntracks: u16, ppq: u16) -> Vec<u8> {
        let mut h = Vec::new();
        h.extend_from_slice(&format.to_be_bytes());
        h.extend_from_slice(&ntracks.to_be_bytes());
        h.extend_from_slice(&ppq.to_be_bytes());
        chunk(b"MThd", &h)
    }

    pub fn meta(ty: u8, data: &[u8]) -> Vec<u8> {
        let mut v = vec![0xFF, ty];
        v.extend_from_slice(data);
        v
    }

    /// A Ctb2 record (SFF2 layout, see sff::parse_ctab): one zone setup for all three zones.
    pub fn ctb2(src_ch: u8, name: &str, ntr: u8, ntt: u8, high_key: u8) -> Vec<u8> {
        let mut d = vec![src_ch];
        let mut n = name.as_bytes().to_vec();
        n.resize(8, b' ');
        d.extend_from_slice(&n);
        d.push(src_ch); // destination
        d.push(0); // editable
        d.extend_from_slice(&[0x0F, 0xFF]); // note mute: all roots
        d.extend_from_slice(&[0x03, 0xFF, 0xFF, 0xFF, 0xFF]); // chord mute: all types
        d.push(0); // source root C
        d.push(2); // source type Maj7
        d.push(0); // mid lo
        d.push(127); // mid hi
        for _ in 0..3 {
            d.extend_from_slice(&[ntr, ntt, high_key, 0, 127, 1]);
        }
        chunk(b"Ctb2", &d)
    }

    pub fn casm(records: &[Vec<u8>]) -> Vec<u8> {
        let mut seg = chunk(b"Sdec", b"Multi Pad");
        for r in records {
            seg.extend_from_slice(r);
        }
        chunk(b"CASM", &chunk(b"CSEG", &seg))
    }
}

#[cfg(test)]
mod tests {
    use super::build::*;
    use super::*;

    fn on(ch: u8, k: u8) -> Vec<u8> {
        vec![0x90 | ch, k, 100]
    }
    fn off(ch: u8, k: u8) -> Vec<u8> {
        vec![0x80 | ch, k, 0]
    }

    /// Two pads on channels 1 and 3 (0 and 2), one bar each at 480 ppq.
    fn type0(with_casm: bool) -> Vec<u8> {
        let mut f = header(0, 1, 480);
        f.extend(track(&[
            (0, meta(0x03, b"My Bank\0\0")),
            (0, meta(0x51, &[0x07, 0xA1, 0x20])), // 120 bpm
            (0, meta(0x58, &[3, 2, 24, 8])),
            (0, vec![0xF0, 0x43, 0x10, 0x4C, 0, 0, 0x7E, 0, 0xF7]),
            (0, vec![0xC2, 5]),
            (0, on(0, 60)),
            (0, on(2, 64)),
            (240, off(0, 60)),
            (480, on(0, 67)),
            (700, off(0, 67)),
            (1000, off(2, 64)),
        ]));
        if with_casm {
            // Pad 1 = channel 3 (converts), pad 2 = channel 1 (Bypass/Root Fixed).
            f.extend(casm(&[ctb2(2, "Strum", 0, 1, 6), ctb2(0, "Hit", 1, 0, 6)]));
        }
        f.extend(chunk(b"XXXX", &[1, 2, 3]));
        f
    }

    #[test]
    fn type0_without_casm_assigns_channels_lowest_first() {
        let b = parse(&type0(false)).unwrap();
        assert_eq!(b.layout, Layout::ChannelsAscending);
        assert_eq!(b.name, "My Bank");
        assert_eq!(b.ppq, 480);
        assert_eq!(b.timesig, (3, 4));
        assert!((b.bpm() - 120.0).abs() < 1e-9);
        let p1 = b.pads[0].as_ref().unwrap();
        let p2 = b.pads[1].as_ref().unwrap();
        assert!(b.pads[2].is_none() && b.pads[3].is_none());
        assert_eq!((p1.channel, p1.name.as_str(), p1.notes()), (0, "Pad 1", 2));
        assert_eq!((p2.channel, p2.name.as_str(), p2.notes()), (2, "Pad 2", 1));
        // The sysex goes to the first pad only; the program change to its own channel.
        assert!(p1.events.iter().any(|e| matches!(e.ev, Ev::Sysex(_))));
        assert!(!p2.events.iter().any(|e| matches!(e.ev, Ev::Sysex(_))));
        assert!(p2.events.iter().any(|e| matches!(e.ev, Ev::Pc { ch: 2, prog: 5 })));
        // Lengths: past the last event, whole beats (700 -> 960, 1000 -> 1440).
        assert_eq!((p1.len, p2.len), (960, 1440));
        assert_eq!((p1.repeat, p1.chord_match), (None, None));
        assert_eq!(b.other_chunks.len(), 1);
        assert_eq!(b.other_chunks[0].0, "XXXX");
    }

    #[test]
    fn casm_orders_pads_and_names_them() {
        let b = parse(&type0(true)).unwrap();
        assert_eq!(b.layout, Layout::ChannelsByCasm);
        let p1 = b.pads[0].as_ref().unwrap();
        let p2 = b.pads[1].as_ref().unwrap();
        assert_eq!((p1.channel, p1.name.as_str(), p1.chord_match), (2, "Strum", Some(true)));
        assert_eq!((p2.channel, p2.name.as_str(), p2.chord_match), (0, "Hit", Some(false)));
        assert_eq!(p1.rule.as_ref().unwrap().src_type, 2);
        // Sysex follows the first pad, which is now channel 3.
        assert!(p1.events.iter().any(|e| matches!(e.ev, Ev::Sysex(_))));
    }

    #[test]
    fn type1_takes_one_pad_per_track() {
        let mut f = header(1, 3, 96);
        f.extend(track(&[(0, meta(0x51, &[0x07, 0xA1, 0x20]))])); // conductor
        f.extend(track(&[(0, meta(0x03, b"Riff")), (0, on(4, 60)), (96, off(4, 60))]));
        f.extend(track(&[(0, meta(0x03, b"Swell")), (0, on(5, 62)), (10, off(5, 62))]));
        let b = parse(&f).unwrap();
        assert_eq!(b.layout, Layout::Tracks);
        let p1 = b.pads[0].as_ref().unwrap();
        let p2 = b.pads[1].as_ref().unwrap();
        assert_eq!((p1.name.as_str(), p1.channel, p1.len), ("Riff", 4, 96));
        assert_eq!((p2.name.as_str(), p2.channel, p2.len), ("Swell", 5, 96));
    }

    /// The Tyros layout from the jososoft article (module docs): type 1, 5 tracks at 1920
    /// ppq, a conductor with CM/RP/N/I text events, pads on tracks 1..4 and channels 1..4.
    fn tyros(pad_tracks: [bool; 4]) -> Vec<u8> {
        let text = |s: &str| meta(0x01, s.as_bytes());
        let n = |pad: u8, name: &str| {
            let mut t = format!("N{pad}{name} {pad}");
            while t.len() < 52 {
                t.push(' ');
            }
            text(&t)
        };
        let mut f = header(1, 5, 1920);
        f.extend(track(&[
            (0, text("CM1010")),
            (0, text("RP0110")),
            (0, n(1, "HipHop1")),
            (0, n(2, "Hip Hop 2")),
            (0, n(3, "Brass")),
            (0, n(4, "Sweep")),
            (0, text("I1S375")),
            (0, text("I2S376")),
            (0, text("I3S377")),
            (0, text("I4S378")),
        ]));
        for (i, &has) in pad_tracks.iter().enumerate() {
            let ch = i as u8;
            if has {
                f.extend(track(&[(0, vec![0xC0 | ch, 10]), (0, on(ch, 60)), (1920, off(ch, 60))]));
            } else {
                f.extend(track(&[]));
            }
        }
        f
    }

    #[test]
    fn tyros_layout_reads_chord_match_repeat_names_and_images_from_track_0() {
        let b = parse(&tyros([true; 4])).unwrap();
        assert_eq!(b.layout, Layout::Tracks);
        assert_eq!(b.ppq, 1920);
        assert_eq!(b.texts.len(), 10);
        let got: Vec<_> = b
            .pads
            .iter()
            .map(|p| {
                let p = p.as_ref().unwrap();
                (p.name.as_str(), p.channel, p.chord_match, p.repeat, p.image.as_deref())
            })
            .collect();
        assert_eq!(
            got,
            [
                ("HipHop1", 0, Some(true), Some(false), Some("S375")),
                ("Hip Hop 2", 1, Some(false), Some(true), Some("S376")),
                ("Brass", 2, Some(true), Some(true), Some("S377")),
                ("Sweep", 3, Some(false), Some(false), Some("S378")),
            ]
        );
        assert_eq!(b.pads[0].as_ref().unwrap().len, 1920);
    }

    #[test]
    fn an_empty_pad_track_keeps_the_later_pads_in_place() {
        let b = parse(&tyros([true, false, true, true])).unwrap();
        assert!(b.pads[1].is_none());
        let p3 = b.pads[2].as_ref().unwrap();
        assert_eq!((p3.name.as_str(), p3.channel, p3.repeat), ("Brass", 2, Some(true)));
    }

    #[test]
    fn header_flags_beat_the_casm_inference() {
        // CASM says pad 1 (channel 3) converts; CM says it does not.
        let f = {
            let mut g = header(0, 1, 480);
            g.extend(track(&[
                (0, meta(0x01, b"CM0100")),
                (0, meta(0x01, b"RP1000")),
                (0, on(0, 60)),
                (0, on(2, 64)),
                (240, off(0, 60)),
                (480, off(2, 64)),
            ]));
            g.extend(casm(&[ctb2(2, "Strum", 0, 1, 6), ctb2(0, "Hit", 1, 0, 6)]));
            g
        };
        let b = parse(&f).unwrap();
        let (p1, p2) = (b.pads[0].as_ref().unwrap(), b.pads[1].as_ref().unwrap());
        assert_eq!((p1.name.as_str(), p1.chord_match, p1.repeat), ("Strum", Some(false), Some(true)));
        assert_eq!((p2.name.as_str(), p2.chord_match, p2.repeat), ("Hit", Some(true), Some(false)));
    }

    #[test]
    fn pad_names_drop_the_repeated_pad_number() {
        assert_eq!(pad_name("HipHop1 2 ....", 2), "HipHop1");
        assert_eq!(pad_name("Hip Hop 2", 2), "Hip Hop");
        assert_eq!(pad_name("Hip Hop 1", 2), "Hip Hop 1");
        assert_eq!(pad_name("  ", 1), "");
        assert_eq!(pad_name("\u{fffd}\u{fffd} 3", 3), "\u{fffd}\u{fffd}");
    }

    #[test]
    fn rejects_garbage_without_panicking() {
        assert!(parse(b"").is_err());
        assert!(parse(b"RIFF0000WAVEfmt ").is_err());
        let mut f = header(2, 1, 480);
        f.extend(track(&[]));
        assert!(parse(&f).is_err());
        assert!(parse(&header(0, 1, 480)).is_err()); // no track
        // Truncated anywhere: an error or a bank, never a panic.
        let full = type0(true);
        for n in 0..full.len() {
            let _ = parse(&full[..n]);
        }
        let full = tyros([true; 4]);
        for n in 0..full.len() {
            let _ = parse(&full[..n]);
        }
        // Text events that are not UTF-8, or too short, never panic.
        let mut f = header(1, 2, 96);
        f.extend(track(&[(0, meta(0x01, &[b'N', 0xC3])), (0, meta(0x01, b"N")), (0, meta(0x01, &[b'I', b'1', 0xFF])), (0, meta(0x01, b"CM"))]));
        f.extend(track(&[(0, on(0, 60)), (96, off(0, 60))]));
        assert!(parse(&f).is_ok());
    }
}

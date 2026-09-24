//! Multi Pad bank (.pad) parser.
//!
//! # Status of the format
//!
//! Yamaha does not document the .pad format (docs/genos-features.md §E, §G.6) and no .pad
//! file was available when this was written, so the layout below is **provisional**. It
//! follows the style-file analogy: an SMF followed by Yamaha chunks, with CASM carrying the
//! chord-conversion rules. Every chunk the parser does not understand is kept raw in
//! [`PadBank::other_chunks`], and `yahaha pad <file>` dumps them, so the first real bank
//! shows where the parser is wrong.
//!
//! What the parser accepts:
//!
//! * **Type 0 SMF** (one MTrk): each pad is one MIDI channel (the Multi Pad Creator records
//!   one channel per pad, RM p.64). With a CASM chunk, its Ctab/Ctb2 records assign pads in
//!   record order (pad 1 = first record's source channel) and give each pad its name and
//!   conversion rule. Without CASM, the channels in use, lowest first, are pads 1..4.
//! * **Type 1 SMF**: each MTrk that has channel events is a pad, in track order; its track
//!   name (meta 03H) is the pad name. CASM, if present, is matched by channel.
//!
//! Per-pad flags:
//!
//! * **Chord Match** comes from the pad's CASM rule: on unless every zone is NTT Bypass with
//!   NTR Root Fixed (the rule that leaves notes as written). With no rule it is unknown.
//! * **Repeat** has no known location. It is `None` until a real bank shows where it lives.
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
        let mut slot = 0;
        for t in &tracks {
            let Some(ch) = t.iter().find_map(|e| e.ev.channel()) else { continue };
            if slot == PADS {
                break;
            }
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
            slot += 1;
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

    Ok(PadBank { name, ppq, tempo_us, timesig, layout, pads, other_chunks })
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
    }
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
    }
}

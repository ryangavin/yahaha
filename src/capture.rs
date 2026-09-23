//! Reference capture kit (#9): a Genos or PSR-SX owner plays our chord script into the
//! instrument's ACMP and records what the style parts send over MIDI. We then compare that
//! recording with what our engine plays for the same script.
//!
//! - `yahaha capture-kit <dir>` writes the script (docs/capture-kit/capture.script) as one
//!   MIDI file per kit style, at that style's tempo, plus the owner instructions. The files
//!   hold only our own chord script, so they can be shared freely.
//! - `yahaha capture-import <recording.mid> <style>` lines the recording up with our
//!   engine's take and reports what differs, per bar and part, and any chord the instrument
//!   read differently from the one we meant.
//! - With `--golden <dir>`, a verified recording becomes a reference digest
//!   (tests/reference): hashes only, like the golden snapshots, because a listing of the
//!   notes a style plays would copy the style.
//!
//! Timing: the kit starts with a lead-in, then the owner's instrument starts the style on the
//! first chord (Sync Start), exactly as `sim` starts it. A recording's clock is its own, so
//! the importer finds bar 1 by lining up the parts that play as written (drums), which are
//! the same on both sides whatever the chord, and fits the tempo from all matched notes.
//! Notes that agree within the tolerance take our timing, so MIDI jitter never shows up as
//! a difference or changes a digest.

use crate::engine::Button;
use crate::fingering::{self, Fingering};
use crate::sff::{self, Ev, Style};
use crate::sim::{self, PlayedNote, ScriptStep, Step, Take, PART_NAMES};
use crate::theory::{self, Chord, Recognizer};
use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The chord script every capture plays.
pub const SCRIPT: &str = include_str!("../docs/capture-kit/capture.script");
/// Owner instructions; `capture-kit` adds a table of the files it wrote.
const INSTRUCTIONS: &str = include_str!("../docs/capture-kit/README.md");

/// Bars of silence before bar 1, so the owner can start the recorder and the player first.
pub const LEAD_IN_BARS: u32 = 2;
/// Chord keys go out on MIDI channel 1 (the owner sets it to receive as Keyboard).
pub const KEYS_CHANNEL: u8 = 0;
const VELOCITY: u8 = 100;
/// The Genos keyboard runs E0-G6 (Yamaha octaves), and the default split point is F#2.
const LOWEST_KEY: i32 = 28;
const SPLIT: i32 = 54;

/// A style the kit asks owners to record: the file name in our corpus, where the owner
/// gets the identical style, and what it covers.
pub struct KitStyle {
    pub file: &'static str,
    pub title: &'static str,
    pub source: &'static str,
    pub covers: &'static str,
}

const MOX: &str = "MOX performance styles V2 (free, sandsoftwaresound.net)";
const FACTORY: &str = "Genos / Genos2 factory style (compared with our Tyros5 copy)";

/// The golden-snapshot styles first: they come from a free pack, so the owner loads the very
/// file we have. The factory styles follow; their Genos data may differ from our Tyros5
/// copies, and the drum check in the report shows whether it does.
pub const KIT: [KitStyle; 9] = [
    KitStyle { file: "OrganCruise.S930.STY", title: "Organ Cruise", source: MOX, covers: "chord-mute routing, fills with parts played as written" },
    KitStyle { file: "JackDoesItAgain.S930.STY", title: "Jack Does It Again", source: MOX, covers: "Guitar NTR, MegaVoice bass and guitar" },
    KitStyle { file: "SmoothItOver.S930.STY", title: "Smooth It Over", source: MOX, covers: "minor-5th NTT tables, several source chords" },
    KitStyle { file: "BluesOrganTrio.S930.STY", title: "Blues Organ Trio", source: MOX, covers: "fast swing (172 bpm), walking bass" },
    KitStyle { file: "AustinCityBlues.S930.STY", title: "Austin City Blues", source: MOX, covers: "slow (82 bpm), Guitar NTR" },
    KitStyle { file: "TickingAway.T162.sty", title: "Ticking Away", source: MOX, covers: "a style without CASM (default channel rules)" },
    KitStyle { file: "60s8Beat.T160.prs", title: "60s8Beat", source: FACTORY, covers: "SFF GE guitar parts" },
    KitStyle { file: "SoulShuffle.T161.prs", title: "SoulShuffle", source: FACTORY, covers: "SFF GE, shuffle" },
    KitStyle { file: "RockShuffle.T162.prs", title: "RockShuffle", source: FACTORY, covers: "SFF GE, shuffle" },
];

// ---------------------------------------------------------------------------
// The kit: chord keys and the MIDI file
// ---------------------------------------------------------------------------

/// The keys the kit holds for a chord: close root position with the on-bass note (if any)
/// lowest, all between E0 and the default split point F#2. Cancel is root + b2 + 2, 1+8 is
/// the root and its octave.
pub fn voicing(c: Chord) -> Vec<u8> {
    let tones: &[u8] = match c.ty {
        theory::CANCEL => &[0, 1, 2],
        theory::ONE_PLUS_EIGHT => &[0, 12],
        t => theory::chord_tones(t),
    };
    let mut keys: Vec<i32> = Vec::new();
    let mut root = c.root as i32;
    if let Some(b) = c.bass {
        keys.push(b as i32);
        root = b as i32 + (c.root as i32 - b as i32).rem_euclid(12);
    }
    keys.extend(tones.iter().map(|&t| root + t as i32));
    keys.sort_unstable();
    keys.dedup();
    let (lo, hi) = (keys[0], keys[keys.len() - 1]);
    // Lowest key in C1..B1, or an octave lower if the top would pass the split point.
    let mut shift = 36;
    while hi + shift > SPLIT && lo + shift - 12 >= LOWEST_KEY {
        shift -= 12;
    }
    keys.iter().map(|k| (k + shift) as u8).collect()
}

/// The chord the instrument should read from `keys` (Fingered On Bass, default split).
pub fn read_back(rec: &Recognizer, keys: &[u8]) -> Option<Chord> {
    let mut held = [false; 128];
    for &k in keys {
        held[k as usize] = true;
    }
    fingering::detect(rec, Fingering::FingeredOnBass, &held, SPLIT as u8, None)
}

/// The Section Control switch number (DL p.111) for a section button.
pub fn section_code(b: Button) -> Option<u8> {
    Some(match b {
        Button::Intro(i) => i,
        Button::Main(i) => 0x08 + i,
        Button::Break => 0x18,
        Button::Ending(i) => 0x20 + i,
        _ => return None,
    })
}

fn section_control(code: u8, on: bool) -> Vec<u8> {
    vec![0xF0, 0x43, 0x7E, 0x00, code, if on { 0x7F } else { 0x00 }, 0xF7]
}

fn vlq(mut v: u32, out: &mut Vec<u8>) {
    let mut buf = [0u8; 5];
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

/// A type 0 Standard MIDI File from (tick, message) pairs, stable-sorted by tick. Messages
/// are channel messages, complete sysex (F0 ... F7) or meta events (FF type data).
pub fn write_smf(ppq: u16, mut events: Vec<(u32, Vec<u8>)>) -> Vec<u8> {
    events.sort_by_key(|e| e.0);
    let mut trk = Vec::new();
    let mut now = 0;
    for (t, m) in &events {
        vlq(t - now, &mut trk);
        now = *t;
        match m[0] {
            0xF0 => {
                trk.push(0xF0);
                vlq(m.len() as u32 - 1, &mut trk);
                trk.extend_from_slice(&m[1..]);
            }
            0xFF => {
                trk.extend_from_slice(&m[..2]);
                vlq(m.len() as u32 - 2, &mut trk);
                trk.extend_from_slice(&m[2..]);
            }
            _ => trk.extend_from_slice(m),
        }
    }
    trk.extend_from_slice(&[0, 0xFF, 0x2F, 0]);
    let mut out = b"MThd".to_vec();
    out.extend_from_slice(&6u32.to_be_bytes());
    out.extend_from_slice(&[0, 0, 0, 1]);
    out.extend_from_slice(&ppq.to_be_bytes());
    out.extend_from_slice(b"MTrk");
    out.extend_from_slice(&(trk.len() as u32).to_be_bytes());
    out.extend_from_slice(&trk);
    out
}

fn meta(ty: u8, data: &[u8]) -> Vec<u8> {
    let mut m = vec![0xFF, ty];
    m.extend_from_slice(data);
    m
}

/// The capture script as a MIDI file for one style: its tempo, time signature and ppq, a
/// lead-in of `LEAD_IN_BARS`, then the chord keys on channel 1 and a Section Control
/// message for every button (pressed one tick ahead, as `sim` presses it). A marker names
/// each bar. Main A is selected during the lead-in, as it is when `sim` starts.
pub fn kit_midi(style: &Style, script: &str) -> Result<Vec<u8>> {
    let (ppq, tpb) = (style.ppq as u32, style.ticks_per_bar());
    let (steps, bars) = sim::parse_script(script, tpb)?;
    let t0 = LEAD_IN_BARS * tpb;
    let rec = Recognizer::new();
    let us = (60e6 / style.bpm()).round() as u32;
    let (num, den) = style.timesig;
    let mut ev: Vec<(u32, Vec<u8>)> = vec![
        (0, meta(0x03, format!("yahaha capture: {}", style.name.trim_end_matches(['\0', ' '])).as_bytes())),
        (0, meta(0x51, &us.to_be_bytes()[1..])),
        (0, meta(0x58, &[num, den.max(1).trailing_zeros() as u8, 24, 8])),
        (0, meta(0x06, b"lead-in: ACMP on, SYNC START armed")),
    ];
    // Main A lit, so the intro leads into it.
    ev.push((ppq, section_control(0x08, true)));
    ev.push((ppq + 1, section_control(0x08, false)));
    for bar in 0..bars {
        let labels: Vec<&str> = steps.iter().filter(|s| s.tick / tpb == bar).map(|s| s.label.as_str()).collect();
        ev.push((t0 + bar * tpb, meta(0x06, format!("bar {} {}", bar + 1, labels.join(" ")).trim_end().as_bytes())));
    }
    let off = |held: &mut Vec<u8>, t: u32, ev: &mut Vec<(u32, Vec<u8>)>| {
        for k in held.drain(..) {
            ev.push((t, vec![0x80 | KEYS_CHANNEL, k, 0]));
        }
    };
    let mut held: Vec<u8> = Vec::new();
    for s in &steps {
        let t = match s.step {
            Step::Button(_) => t0 + s.tick - 1,
            _ => t0 + s.tick,
        };
        match s.step {
            Step::Chord(c) => {
                off(&mut held, t, &mut ev);
                held = voicing(c);
                // The instrument must read the keys as the chord our engine is given.
                if read_back(&rec, &held) != Some(c) {
                    bail!("{}: the kit's keys would not read back as that chord", s.label);
                }
                for &k in &held {
                    ev.push((t, vec![0x90 | KEYS_CHANNEL, k, VELOCITY]));
                }
            }
            Step::Release => off(&mut held, t, &mut ev),
            Step::Button(b) => {
                let code = section_code(b).with_context(|| format!("{} has no MIDI Section Control message", s.label))?;
                ev.push((t, section_control(code, true)));
                ev.push((t + 1, section_control(code, false)));
            }
            _ => bail!("{} cannot be sent over MIDI", s.label),
        }
    }
    off(&mut held, t0 + bars * tpb, &mut ev);
    ev.push((t0 + (bars + 1) * tpb, meta(0x06, b"end: stop recording")));
    Ok(write_smf(style.ppq, ev))
}

/// Find a style by file name under `root` (recursively).
pub fn find_file(root: &Path, name: &str) -> Option<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.file_name().is_some_and(|f| f == name) {
                return Some(p);
            }
        }
    }
    None
}

/// `yahaha capture-kit <out-dir> [style]...`: one MIDI file per style (the kit styles found
/// in ./corpus when none are given) and the owner instructions, README.md.
pub fn write_kit(out: &Path, styles: &[PathBuf]) -> Result<()> {
    let mut paths: Vec<(PathBuf, Option<&KitStyle>)> = Vec::new();
    if styles.is_empty() {
        for k in &KIT {
            match find_file(Path::new("corpus"), k.file) {
                Some(p) => paths.push((p, Some(k))),
                None => eprintln!("capture-kit: {} is not in ./corpus; skipping it", k.file),
            }
        }
    } else {
        for p in styles {
            let name = p.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
            paths.push((p.clone(), KIT.iter().find(|k| k.file == name)));
        }
    }
    if paths.is_empty() {
        bail!("no styles: give style files, or run from the checkout with ./corpus");
    }
    std::fs::create_dir_all(out)?;
    let mut table = String::from(
        "\n## Files in this kit\n\n| MIDI file | Style to load | Where to get it | Tempo | Length | Covers |\n|---|---|---|---|---|---|\n",
    );
    for (path, kit) in &paths {
        let style = Style::load(path)?;
        let (_, bars) = sim::parse_script(SCRIPT, style.ticks_per_bar())?;
        let stem = path.file_name().unwrap().to_string_lossy().split('.').next().unwrap_or("style").to_string();
        let file = format!("{stem}.capture.mid");
        std::fs::write(out.join(&file), kit_midi(&style, SCRIPT)?)?;
        let secs = (bars + LEAD_IN_BARS) as f64 * style.ticks_per_bar() as f64 / style.ppq as f64 * 60.0 / style.bpm();
        table += &format!(
            "| `{file}` | {} (`{}`) | {} | {:.0} bpm | {bars} bars + {LEAD_IN_BARS} lead-in, {}:{:02} | {} |\n",
            kit.map_or(stem.as_str(), |k| k.title),
            path.file_name().unwrap().to_string_lossy(),
            kit.map_or("-", |k| k.source),
            style.bpm(),
            secs as u32 / 60,
            secs as u32 % 60,
            kit.map_or("-", |k| k.covers),
        );
        println!("wrote {}", out.join(&file).display());
    }
    std::fs::write(out.join("README.md"), format!("{INSTRUCTIONS}{table}"))?;
    std::fs::write(out.join("capture.script"), SCRIPT)?;
    println!("wrote {}", out.join("README.md").display());
    Ok(())
}

// ---------------------------------------------------------------------------
// Reading a recording
// ---------------------------------------------------------------------------

/// Every event of a Standard MIDI File (type 0 or 1), in seconds, all tracks merged.
pub fn read_smf(bytes: &[u8]) -> Result<Vec<(f64, Ev)>> {
    if bytes.len() < 14 || &bytes[0..4] != b"MThd" {
        bail!("not a MIDI file");
    }
    let hlen = u32::from_be_bytes(bytes[4..8].try_into()?) as usize;
    let division = u16::from_be_bytes([bytes[12], bytes[13]]);
    if division & 0x8000 != 0 || division == 0 {
        bail!("SMPTE or zero time division is not supported; save the recording with ticks per quarter note");
    }
    let mut events: Vec<(u32, Ev)> = Vec::new();
    let mut p = 8 + hlen;
    while p + 8 <= bytes.len() {
        let len = u32::from_be_bytes(bytes[p + 4..p + 8].try_into()?) as usize;
        let end = (p + 8 + len).min(bytes.len());
        if &bytes[p..p + 4] == b"MTrk" {
            events.extend(sff::parse_track(&bytes[p + 8..end])?.into_iter().map(|e| (e.tick, e.ev)));
        }
        p = end;
    }
    // A stable sort keeps each track's order within a tick.
    events.sort_by_key(|e| e.0);
    let (mut last_tick, mut sec, mut us_per_q) = (0u32, 0f64, 500_000f64);
    let mut out = Vec::with_capacity(events.len());
    for (t, e) in events {
        sec += (t - last_tick) as f64 * us_per_q / 1e6 / division as f64;
        last_tick = t;
        if let Ev::Meta { ty: 0x51, data } = &e
            && data.len() == 3
        {
            us_per_q = u32::from_be_bytes([0, data[0], data[1], data[2]]) as f64;
        }
        out.push((sec, e));
    }
    Ok(out)
}

/// A chord from the Chord Control bytes (DL p.111; see docs/genos-features.md §C.10).
pub fn chord_from_bytes(cr: u8, ct: u8, bn: u8) -> Option<Chord> {
    let pc = |b: u8| -> Option<u8> {
        let n = b & 0x0F;
        let f = (b >> 4) & 0x07;
        if !(1..=7).contains(&n) || f > 6 {
            return None;
        }
        let natural = [0i32, 2, 4, 5, 7, 9, 11][n as usize - 1];
        Some((natural + f as i32 - 3).rem_euclid(12) as u8)
    };
    let root = pc(cr)?;
    if ct as usize > theory::NUM_TYPES {
        return None;
    }
    let bass = if bn == 127 { None } else { pc(bn) };
    Some(Chord { root, ty: ct, bass: bass.filter(|&b| b != root) })
}

/// The chord in a Chord Control SysEx (F0 43 7E 02 ...) or Song chord meta event
/// (FF 7F 43 7B 01 ...).
fn chord_event(e: &Ev) -> Option<Chord> {
    match e {
        Ev::Sysex(d) if d.len() >= 8 && d[..4] == [0xF0, 0x43, 0x7E, 0x02] => chord_from_bytes(d[4], d[5], d[6]),
        Ev::Meta { ty: 0x7F, data } if data.len() >= 6 && data[..3] == [0x43, 0x7B, 0x01] => chord_from_bytes(data[3], data[4], data[5]),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Aligning and comparing
// ---------------------------------------------------------------------------

/// A recorded style note, in the style's ticks at the style's tempo from the start of the
/// file (before alignment).
#[derive(Clone, Copy)]
struct RecNote {
    x: f64,
    ch: u8,
    key: u8,
    end: Option<f64>,
}

pub struct ImportOptions {
    /// Notes this close (ms) count as the same note.
    pub tolerance_ms: f64,
    /// Where bar 1 starts in the recording (ms), when the automatic alignment fails.
    pub offset_ms: Option<f64>,
}

impl Default for ImportOptions {
    fn default() -> Self {
        ImportOptions { tolerance_ms: 25.0, offset_ms: None }
    }
}

pub struct Import {
    /// What the instrument played, as a take of our script (sections and steps are ours).
    pub hardware: Take,
    /// What our engine plays for the same script.
    pub ours: Take,
    pub report: String,
    /// Whether the recording can serve as reference data (see `verify`).
    pub verified: bool,
    /// Bars where any part differs.
    #[cfg_attr(not(test), allow(dead_code))]
    pub differing_bars: usize,
}

/// Matches recorded notes to our notes with the same channel and key within `tol` ticks,
/// with our tick = a * x + b. Returns, per recorded note, the index of its match.
fn match_notes(rec: &[RecNote], ours: &[PlayedNote], a: f64, b: f64, tol: f64) -> Vec<Option<usize>> {
    let mut by_key: HashMap<(u8, u8), Vec<usize>> = HashMap::new();
    for (i, n) in ours.iter().enumerate() {
        by_key.entry((n.ch, n.key)).or_default().push(i);
    }
    let mut used = vec![false; ours.len()];
    rec.iter()
        .map(|r| {
            let t = a * r.x + b;
            let end = r.end.map(|e| a * e + b);
            // Among our notes that start close enough, the one whose start and end are both
            // nearest: a key retriggered a few ticks later must not swap with the first.
            let best = by_key
                .get(&(r.ch, r.key))?
                .iter()
                .filter(|&&i| !used[i])
                .map(|&i| (i, (ours[i].tick as f64 - t).abs()))
                .filter(|&(_, d)| d <= tol)
                .map(|(i, d)| {
                    let our_end = ours[i].len.map(|l| (ours[i].tick + l) as f64);
                    (i, d + our_end.zip(end).map_or(0.0, |(o, e)| (o - e).abs()))
                })
                .min_by(|p, q| p.1.total_cmp(&q.1))?
                .0;
            used[best] = true;
            Some(best)
        })
        .collect()
}

/// Least-squares fit ours = a * x + b over matched pairs.
fn fit(rec: &[RecNote], ours: &[PlayedNote], m: &[Option<usize>]) -> Option<(f64, f64)> {
    let pairs: Vec<(f64, f64)> = rec.iter().zip(m).filter_map(|(r, m)| m.map(|i| (r.x, ours[i].tick as f64))).collect();
    if pairs.len() < 8 {
        return None;
    }
    let n = pairs.len() as f64;
    let (mx, my) = (pairs.iter().map(|p| p.0).sum::<f64>() / n, pairs.iter().map(|p| p.1).sum::<f64>() / n);
    let var: f64 = pairs.iter().map(|p| (p.0 - mx).powi(2)).sum();
    let cov: f64 = pairs.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
    if var < 1.0 {
        return Some((1.0, my - mx));
    }
    let a = cov / var;
    Some((a, my - a * mx))
}

/// Items in `a` that are not in `b`, as multisets.
fn minus(a: &[String], b: &[String]) -> Vec<String> {
    let mut rest: Vec<&String> = b.iter().collect();
    a.iter()
        .filter(|w| match rest.iter().position(|x| x == w) {
            Some(i) => {
                rest.remove(i);
                false
            }
            None => true,
        })
        .cloned()
        .collect()
}

/// Line a recording up with our take of `script` on `style` and compare them.
pub fn import(recording: &[u8], style: &Style, script: &str, opts: &ImportOptions) -> Result<Import> {
    let ours = sim::perform(style, script)?;
    let events = read_smf(recording)?;
    let per_sec = ours.bpm / 60.0 * ours.ppq as f64;
    let tol = opts.tolerance_ms / 1000.0 * per_sec;

    // Style notes (channels 9-16); each note-off closes the oldest open note of its key.
    let mut rec: Vec<RecNote> = Vec::new();
    let mut first_key: Option<f64> = None;
    for (s, e) in &events {
        let x = s * per_sec;
        match *e {
            Ev::NoteOn { ch, key, .. } if ch >= 8 => rec.push(RecNote { x, ch, key, end: None }),
            Ev::NoteOn { .. } => {
                first_key.get_or_insert(x);
            }
            Ev::NoteOff { ch, key } if ch >= 8 => {
                if let Some(n) = rec.iter_mut().find(|n| n.ch == ch && n.key == key && n.end.is_none()) {
                    n.end = Some(x);
                }
            }
            _ => {}
        }
    }
    if rec.is_empty() {
        bail!("the recording has no notes on channels 9-16 (the style parts)");
    }
    let chords: Vec<(f64, Chord)> = events.iter().filter_map(|(s, e)| chord_event(e).map(|c| (s * per_sec, c))).collect();

    // Bar 1: try lining up each part's first note with ours, and the first chord key, at the
    // style's tempo and at the tempo the drums' overall span suggests, and keep the pair that
    // matches the most notes. Then fit tempo and offset to every match.
    let span = |ch: Option<u8>| {
        let on = |c: u8| ch.is_none_or(|x| x == c);
        let r = rec.iter().filter(|n| on(n.ch)).map(|n| n.x);
        let o = ours.notes.iter().filter(|n| on(n.ch)).map(|n| n.tick as f64);
        Some(((r.clone().reduce(f64::min)?, r.reduce(f64::max)?), (o.clone().reduce(f64::min)?, o.reduce(f64::max)?)))
    };
    let (mut a, mut b) = if let Some(ms) = opts.offset_ms {
        (1.0, -ms / 1000.0 * per_sec)
    } else {
        let mut tempos = vec![1.0];
        if let Some(((r0, r1), (o0, o1))) = span(Some(9)).or(span(Some(8)))
            && r1 > r0
        {
            tempos.push(((o1 - o0) / (r1 - r0)).clamp(0.8, 1.25));
        }
        let mut cands: Vec<(f64, f64)> = Vec::new();
        for &a in &tempos {
            for ch in (8..16).map(Some).chain([None]) {
                if let Some(((r0, _), (o0, _))) = span(ch) {
                    cands.push((a, o0 - a * r0));
                }
            }
            cands.extend(first_key.map(|x| (a, -a * x)));
        }
        let score = |&(a, b): &(f64, f64)| match_notes(&rec, &ours.notes, a, b, tol).iter().filter(|m| m.is_some()).count();
        cands.into_iter().max_by_key(score).unwrap_or((1.0, 0.0))
    };
    let mut matched = match_notes(&rec, &ours.notes, a, b, tol);
    for _ in 0..3 {
        if let Some((fa, fb)) = fit(&rec, &ours.notes, &matched) {
            (a, b) = (fa, fb);
        }
        matched = match_notes(&rec, &ours.notes, a, b, tol);
    }

    // The instrument's take: our script's steps and sections, its notes. A note that matches
    // ours takes our start and, when the ends agree too, our length.
    let end = (ours.bars * ours.tpb) as f64;
    let mut hardware = ours.clone();
    hardware.notes.clear();
    let mut before = 0;
    // Notes starting together keep our order (the order they were sent in says nothing),
    // so a take that matches ours renders the same listing.
    let mut placed: Vec<(usize, PlayedNote)> = Vec::new();
    for (r, &m) in rec.iter().zip(&matched) {
        let t = a * r.x + b;
        if t < -tol {
            before += 1;
            continue;
        }
        // A note with no match on its key most likely differs in pitch only: it takes the
        // timing of our nearest note on the same channel, so it lands in the same bar and
        // beat instead of wherever the jitter put it.
        let m = m.or_else(|| {
            let near = ours.notes.iter().enumerate().filter(|(_, o)| o.ch == r.ch).map(|(i, o)| (i, (o.tick as f64 - t).abs()));
            near.filter(|&(_, d)| d <= tol).min_by(|p, q| p.1.total_cmp(&q.1)).map(|(i, _)| i)
        });
        let our = m.map(|i| ours.notes[i]);
        let tick = our.map_or(t.round().max(0.0) as u32, |o| o.tick);
        let len = r.end.map(|e| a * e + b).filter(|&e| e <= end + tol).map(|e| {
            let l = (e - tick as f64).round().max(0.0) as u32;
            match our.and_then(|o| o.len) {
                Some(ol) if (ol as f64 - l as f64).abs() <= tol => ol,
                _ => l,
            }
        });
        placed.push((m.unwrap_or(usize::MAX), PlayedNote { tick, ch: r.ch, key: r.key, len }));
    }
    placed.sort_by_key(|(i, n)| (n.tick, *i, n.ch, n.key));
    hardware.notes = placed.into_iter().map(|(_, n)| n).collect();

    let mut out = String::new();
    let matched_n = matched.iter().filter(|m| m.is_some()).count();
    let secs = |t: f64| (t - b) / a / per_sec;
    out += &format!(
        "alignment: bar 1 at {:.3} s in the recording; tempo {:.2} bpm (style {:.2}, ratio {:.5}); {matched_n} of {} recorded notes within {} ms of ours\n",
        secs(0.0),
        ours.bpm * a,
        ours.bpm,
        a,
        rec.len(),
        opts.tolerance_ms
    );
    if before > 0 {
        out += &format!("           {before} recorded notes before bar 1 ignored\n");
    }

    // Verification: the parts that play as written must agree, the tempo must be the style's,
    // and the recording must cover the whole script.
    let mut problems = Vec::new();
    let written: Vec<usize> = (0..ours.notes.len()).filter(|&i| ours.as_written(ours.notes[i].tick, ours.notes[i].ch)).collect();
    let hit: Vec<bool> = {
        let mut v = vec![false; ours.notes.len()];
        for i in matched.iter().flatten() {
            v[*i] = true;
        }
        v
    };
    let written_hit = written.iter().filter(|&&i| hit[i]).count();
    if written.is_empty() {
        problems.push("no part plays as written, so the alignment cannot be checked".to_string());
    } else if (written_hit as f64) < 0.98 * written.len() as f64 {
        problems.push(format!(
            "only {written_hit} of {} notes of the parts that play as written (drums) match: wrong style, section buttons not received, or a bad alignment (try --offset-ms)",
            written.len()
        ));
    }
    if (a - 1.0).abs() > 0.002 {
        problems.push(format!("the instrument played at {:.2} bpm, not the style's {:.2}: leave the tempo alone and record again", ours.bpm * a, ours.bpm));
    }
    // The style stops after the ending, so "the whole script" means up to our last note (a
    // bar of slack: an ending one bar shorter is a difference, not a short recording).
    let last = rec.iter().map(|r| a * r.x + b).fold(f64::MIN, f64::max);
    let our_last = ours.notes.iter().map(|n| n.tick).max().unwrap_or(0) as f64;
    if last < our_last - ours.tpb as f64 {
        problems.push(format!(
            "the recording stops in bar {}, but the script plays on to bar {}",
            (last.max(0.0) as u32) / ours.tpb + 1,
            our_last as u32 / ours.tpb + 1
        ));
    }

    // Chords the instrument read, against the ones the script meant.
    let steps: Vec<&ScriptStep> = ours.steps.iter().filter(|s| matches!(s.step, Step::Chord(_))).collect();
    if chords.is_empty() {
        out += "chords: the recording has no Chord Control messages (turn on Chord System Exclusive Message Transmit to check them)\n";
    } else {
        let (mut same, mut silent, mut wrong) = (0, 0, Vec::new());
        for (k, s) in steps.iter().enumerate() {
            let Step::Chord(want) = s.step else { continue };
            let lo = s.tick as f64 - tol;
            let hi = steps.get(k + 1).map_or(end, |n| n.tick as f64 - tol).min(s.tick as f64 + ours.ppq as f64);
            let got = chords.iter().rev().map(|(x, c)| (a * x + b, *c)).find(|(t, _)| *t >= lo && *t < hi).map(|(_, c)| c);
            match got {
                Some(c) if c == want.casm() => same += 1,
                Some(c) => wrong.push(format!(
                    "  bar {} beat {}: played {}, read as {}\n",
                    s.tick / ours.tpb + 1,
                    ours.pos(s.tick),
                    s.label,
                    c.name()
                )),
                None => silent += 1,
            }
        }
        out += &format!("chords: {same} of {} chord changes read as meant, {} read differently, {silent} with no chord message\n", steps.len(), wrong.len());
        for w in &wrong {
            out += w;
        }
    }
    let verified = problems.is_empty();
    if verified {
        out += "verified: yes\n";
    } else {
        out += "verified: no\n";
        for p in &problems {
            out += &format!("  - {p}\n");
        }
    }

    // Per bar and part.
    let mut per_part = [(0usize, 0usize); 8];
    let mut diffs = String::new();
    let mut differing_bars = 0;
    for bar in 0..ours.bars {
        let mut lines = Vec::new();
        for ch in 8..16u8 {
            let (hi, hw) = hardware.part_items(bar, ch);
            let (oi, ow) = ours.part_items(bar, ch);
            if hi.is_empty() && hw == 0 && oi.is_empty() && ow == 0 {
                continue;
            }
            let p = &mut per_part[ch as usize - 8];
            p.1 += 1;
            let (only_hw, only_ours) = (minus(&hi, &oi), minus(&oi, &hi));
            let mut what = Vec::new();
            if !only_hw.is_empty() || !only_ours.is_empty() {
                let list = |v: &[String]| if v.is_empty() { "nothing".to_string() } else { v.join(", ") };
                what.push(format!("instrument only: {} | yahaha only: {}", list(&only_hw), list(&only_ours)));
            }
            if hw != ow {
                what.push(format!("as written: instrument {hw}, yahaha {ow}"));
            }
            if !what.is_empty() {
                p.0 += 1;
                lines.push(format!("  ch{:<2} {:<8} {}", ch + 1, PART_NAMES[ch as usize - 8], what.join("; ")));
            }
        }
        if !lines.is_empty() {
            differing_bars += 1;
            diffs += &format!("{}\n{}\n", ours.bar_header(bar), lines.join("\n"));
        }
    }
    out += &format!("\nbars that differ: {differing_bars} of {}\n", ours.bars);
    for (i, (d, n)) in per_part.iter().enumerate().filter(|(_, p)| p.1 > 0) {
        out += &format!("  ch{:<2} {:<8} {d} of {n} bars\n", i + 9, PART_NAMES[i]);
    }
    if !diffs.is_empty() {
        out += "\ndifferences (instrument = the recording, yahaha = our engine):\n";
        out += &diffs;
    }
    Ok(Import { hardware, ours, report: out, verified, differing_bars })
}

/// The reference digest of a recording, as `tests/reference/<style file>.digest`.
pub fn reference_digest(imp: &Import) -> String {
    sim::digest(&imp.hardware.render())
}

/// `yahaha capture-import <recording.mid> <style> [options]`.
pub fn import_cmd(args: &[String]) -> Result<()> {
    let usage = "usage: yahaha capture-import <recording.mid> <style> [--tolerance-ms N] [--offset-ms N] [--listing FILE] [--golden DIR [--force]]";
    let (Some(recording), Some(style_path)) = (args.first(), args.get(1)) else {
        bail!("{usage}");
    };
    let mut opts = ImportOptions::default();
    let (mut listing, mut golden, mut force) = (None, None, false);
    let mut it = args[2..].iter();
    while let Some(flag) = it.next() {
        let mut value = || it.next().cloned().with_context(|| format!("{flag} wants a value\n{usage}"));
        match flag.as_str() {
            "--tolerance-ms" => opts.tolerance_ms = value()?.parse()?,
            "--offset-ms" => opts.offset_ms = Some(value()?.parse()?),
            "--listing" => listing = Some(PathBuf::from(value()?)),
            "--golden" => golden = Some(PathBuf::from(value()?)),
            "--force" => force = true,
            other => bail!("unknown option {other}\n{usage}"),
        }
    }
    let style_path = Path::new(style_path);
    let style = Style::load(style_path)?;
    let bytes = std::fs::read(recording).with_context(|| format!("reading {recording}"))?;
    let imp = import(&bytes, &style, SCRIPT, &opts)?;
    println!("capture-import: {recording} against yahaha on {}\n{}", style_path.display(), imp.report);
    if let Some(path) = listing {
        // Readable notes: for this machine only, never for the repo (see tests/golden/README.md).
        std::fs::write(&path, imp.hardware.render())?;
        println!("wrote the recording's listing to {} (keep it local: it transcribes the style)", path.display());
    }
    if let Some(dir) = golden {
        if !imp.verified && !force {
            bail!("the recording is not verified (see above); not writing a reference digest (--force overrides)");
        }
        std::fs::create_dir_all(&dir)?;
        let name = style_path.file_name().unwrap().to_string_lossy().to_string();
        let file = dir.join(format!("{name}.digest"));
        let reference = reference_digest(&imp);
        std::fs::write(&file, &reference)?;
        println!("wrote {}", file.display());
        // Where we differ from the hardware today: the reference test allows these until
        // they are fixed (and says so once they are).
        let known = dir.join(format!("{name}.known"));
        let diffs = digest_differences(&reference, &sim::digest(&imp.ours.render()));
        let mut text = format!("# Where yahaha differed from {recording} when it was imported. One `bar N chX` per line.\n");
        for d in &diffs {
            text += d;
            text.push('\n');
        }
        std::fs::write(&known, text)?;
        println!("wrote {} ({} known differences)", known.display(), diffs.len());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Reference digests (tests/reference)
// ---------------------------------------------------------------------------

/// The part lines of a digest by (bar header index, part key), and the bar headers.
fn digest_parts(d: &str) -> (Vec<&str>, HashMap<(usize, String), &str>) {
    let (mut heads, mut parts) = (Vec::new(), HashMap::new());
    for line in d.lines() {
        if line.starts_with("bar ") {
            heads.push(line);
        } else if line.starts_with("  ch") && !heads.is_empty() {
            parts.insert((heads.len(), sim::part_key(line)), line);
        }
    }
    (heads, parts)
}

/// Where two digests differ, as `bar N chX` keys (and `bar N header` for the bar line).
pub fn digest_differences(want: &str, got: &str) -> Vec<String> {
    let ((wh, wp), (gh, gp)) = (digest_parts(want), digest_parts(got));
    let mut out = Vec::new();
    for i in 0..wh.len().max(gh.len()) {
        if wh.get(i) != gh.get(i) {
            out.push(format!("bar {} header", i + 1));
        }
    }
    let mut keys: Vec<&(usize, String)> = wp.keys().chain(gp.keys()).collect();
    keys.sort_by_key(|(b, k)| (*b, k.get(2..).and_then(|k| k.split(' ').next()).and_then(|c| c.parse::<u8>().ok())));
    keys.dedup();
    for k in keys {
        if wp.get(k) != gp.get(k) {
            out.push(format!("bar {} {}", k.0, k.1.split(' ').next().unwrap_or("")));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus_style(name: &str) -> Option<Style> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus");
        find_file(&root, name).map(|p| Style::load(&p).unwrap())
    }

    /// Every chord in the script fits the chord section and reads back as itself, every
    /// button has a Section Control message, and the script covers what the kit promises.
    #[test]
    fn script_reads_back_and_covers_the_kit() {
        let rec = Recognizer::new();
        let (steps, bars) = sim::parse_script(SCRIPT, 1920).unwrap();
        let mut types = [false; 38];
        let (mut slash, mut busiest) = (0, 0);
        for s in &steps {
            match s.step {
                Step::Chord(c) => {
                    let keys = voicing(c);
                    assert!(keys.iter().all(|&k| (LOWEST_KEY..=SPLIT).contains(&(k as i32))), "{}: keys {keys:?} outside E0..F#2", s.label);
                    assert_eq!(read_back(&rec, &keys), Some(c), "{} ({keys:?}) reads as {:?}", s.label, read_back(&rec, &keys).map(|c| c.name()));
                    types[c.ty as usize] = true;
                    slash += c.bass.is_some() as usize;
                }
                Step::Button(b) => assert!(section_code(b).is_some(), "{} has no Section Control message", s.label),
                Step::Release => {}
                _ => panic!("{} cannot be sent over MIDI", s.label),
            }
        }
        for bar in 0..bars {
            busiest = busiest.max(steps.iter().filter(|s| s.tick / 1920 == bar && matches!(s.step, Step::Chord(_))).count());
        }
        let missing: Vec<&str> = (0..38).filter(|&t| !types[t]).map(|t| theory::TYPE_NAMES[t]).collect();
        assert!(missing.is_empty(), "chord types missing from the script: {missing:?}");
        assert!(slash >= 12, "only {slash} slash chords");
        assert_eq!(busiest, 8, "the script should change chord on every eighth somewhere");
        for b in ["[IntroA]", "[MainA]", "[MainB]", "[MainC]", "[MainD]", "[Break]", "[EndingA]", "^"] {
            assert!(steps.iter().any(|s| s.label == b), "script lacks {b}");
        }
    }

    #[test]
    fn voicings() {
        let c = |s: &str| crate::parse_chord(s).unwrap();
        assert_eq!(voicing(c("C")), [36, 40, 43]);
        // E1 bass, then C2 E2 G2 would pass F#2, so everything drops an octave: E0 C1 E1 G1.
        assert_eq!(voicing(c("C/E")), [28, 36, 40, 43]);
        assert_eq!(voicing(c("B")), [47, 51, 54]);
        assert_eq!(voicing(c("C1+8")), [36, 48]);
        assert_eq!(voicing(c("Ccancel")), [36, 37, 38]);
    }

    #[test]
    fn chord_bytes() {
        assert_eq!(chord_from_bytes(0x31, 0, 127), Some(Chord::new(0, 0)));
        assert_eq!(chord_from_bytes(0x41, 8, 127), Some(Chord::new(1, 8)), "C#m");
        assert_eq!(chord_from_bytes(0x27, 19, 0x44), Some(Chord { root: 10, ty: 19, bass: Some(6) }), "Bb7/F#");
        assert_eq!(chord_from_bytes(0x31, 34, 127).map(|c| c.ty), Some(theory::CANCEL));
        assert_eq!(chord_from_bytes(0x38, 0, 127), None, "bad note");
        assert_eq!(chord_from_bytes(0x31, 40, 127), None, "bad type");
    }

    /// The kit file plays the script's keys on channel 1 after the lead-in, with a Section
    /// Control press for every button one tick ahead of its slot.
    #[test]
    fn kit_midi_plays_the_script() {
        let Some(style) = corpus_style(KIT[0].file) else {
            return;
        };
        let bytes = kit_midi(&style, "[IntroA] | C | Am - - [MainB] - |").unwrap();
        let ev = read_smf(&bytes).unwrap();
        let ppq_sec = 60.0 / style.bpm() / style.ppq as f64;
        let tick = |s: f64| (s / ppq_sec).round() as u32;
        let t0 = LEAD_IN_BARS * style.ticks_per_bar();
        let ons: Vec<(u32, u8)> = ev.iter().filter_map(|(s, e)| match e {
            Ev::NoteOn { ch: 0, key, .. } => Some((tick(*s), *key)),
            _ => None,
        }).collect();
        assert_eq!(ons, [(t0, 36), (t0, 40), (t0, 43), (t0 + style.ticks_per_bar(), 45), (t0 + style.ticks_per_bar(), 48), (t0 + style.ticks_per_bar(), 52)]);
        let sections: Vec<(u32, Vec<u8>)> = ev.iter().filter_map(|(s, e)| match e {
            Ev::Sysex(d) if d[5] == 0x7F => Some((tick(*s), d.clone())),
            _ => None,
        }).collect();
        let mb = t0 + style.ticks_per_bar() + 3 * style.ppq as u32 - 1;
        assert_eq!(sections, [(style.ppq as u32, section_control(0x08, true)), (t0 - 1, section_control(0, true)), (mb, section_control(0x09, true))]);
        let offs = ev.iter().filter(|(_, e)| matches!(e, Ev::NoteOff { ch: 0, .. })).count();
        assert_eq!(offs, 6, "every key is let go");
        assert!(kit_midi(&style, "| C [SyncStop] |").is_err());
    }

    /// A fake recording of our own take: other ppq and clock, bar 1 somewhere in the file,
    /// a little MIDI jitter, chord messages, and the instrument's keyboard echo on channel 4.
    fn fake_recording(take: &Take, jitter_ms: f64, speed: f64, tweak: impl Fn(&mut PlayedNote)) -> Vec<u8> {
        let ppq = 480u16;
        let bpm = 120.0;
        let to_rec = |t: u32| {
            let sec = 3.217 + t as f64 / (take.bpm / 60.0 * take.ppq as f64) / speed;
            (sec * bpm / 60.0 * ppq as f64).round() as u32
        };
        let mut ev: Vec<(u32, Vec<u8>)> = vec![(0, meta(0x51, &500_000u32.to_be_bytes()[1..]))];
        let mut seed = 12345u32;
        let mut jit = || {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
            let j = ((seed >> 16) % 1000) as f64 / 1000.0 * 2.0 - 1.0;
            (j * jitter_ms / 1000.0 * bpm / 60.0 * ppq as f64).round() as i64
        };
        let mut notes: Vec<(u32, Vec<u8>)> = Vec::new();
        for n in &take.notes {
            let mut n = *n;
            tweak(&mut n);
            notes.push((to_rec(n.tick), vec![0x90 | n.ch, n.key, 90]));
            // A note still sounding at the end of the script is let go a bar later.
            notes.push((to_rec(n.tick + n.len.unwrap_or(take.bars * take.tpb + take.tpb - n.tick)), vec![0x80 | n.ch, n.key, 0]));
        }
        // Jitter moves the time stamps but, like a real MIDI cable, never the order.
        notes.sort_by_key(|e| e.0);
        let mut last = 0;
        for (t, m) in notes {
            last = (t as i64 + jit()).max(last);
            ev.push((last as u32, m));
        }
        for s in &take.steps {
            if let Step::Chord(c) = s.step {
                let t = to_rec(s.tick) + 2;
                ev.push((t, vec![0x93, 60, 80]));
                // C# = 0x41: natural letters 1..7 with the accidental in the high bits.
                let (letter, acc) = [(1, 3), (1, 4), (2, 3), (3, 2), (3, 3), (4, 3), (4, 4), (5, 3), (6, 2), (6, 3), (7, 2), (7, 3)][c.root as usize];
                let bn = c.bass.map_or(127, |b| {
                    let (l, a) = [(1, 3), (1, 4), (2, 3), (3, 2), (3, 3), (4, 3), (4, 4), (5, 3), (6, 2), (6, 3), (7, 2), (7, 3)][b as usize];
                    (a << 4) | l
                });
                ev.push((t + 1, vec![0xF0, 0x43, 0x7E, 0x02, (acc << 4) | letter, c.casm().ty, bn, 127, 0xF7]));
            }
        }
        write_smf(ppq, ev)
    }

    /// A recording of exactly what we play (jittered, shifted, another clock) imports with no
    /// differences, verifies, and its reference digest is our own snapshot's digest. Every
    /// kit style in the corpus.
    #[test]
    fn import_round_trip() {
        for k in &KIT {
            let Some(style) = corpus_style(k.file) else {
                continue;
            };
            let ours = sim::perform(&style, SCRIPT).unwrap();
            let rec = fake_recording(&ours, 4.0, 1.0, |_| {});
            let imp = import(&rec, &style, SCRIPT, &ImportOptions::default()).unwrap();
            let what = format!("{}:\n{}", k.file, imp.report);
            assert!(imp.verified, "{what}");
            assert_eq!(imp.differing_bars, 0, "{what}");
            assert!(imp.report.contains("bar 1 at 3.21"), "{what}");
            assert!(imp.report.contains("read as meant, 0 read differently, 0 with no chord message"), "{what}");
            assert_eq!(reference_digest(&imp), sim::digest(&sim::snapshot(&style, SCRIPT).unwrap()), "{what}");
        }
    }

    /// A changed bass note shows up in its bar and part only; a wrong tempo fails verification.
    #[test]
    fn import_reports_differences() {
        let Some(style) = corpus_style(KIT[0].file) else {
            return;
        };
        let ours = sim::perform(&style, SCRIPT).unwrap();
        let tpb = ours.tpb;
        let target = *ours.notes.iter().find(|n| n.ch == 10 && n.tick / tpb == 19 && !ours.as_written(n.tick, 10)).expect("a bass note in bar 20");
        let rec = fake_recording(&ours, 0.0, 1.0, |n| {
            if *n == target {
                n.key += 2;
            }
        });
        let imp = import(&rec, &style, SCRIPT, &ImportOptions::default()).unwrap();
        assert!(imp.verified, "{}", imp.report);
        assert_eq!(imp.differing_bars, 1, "{}", imp.report);
        let diffs = imp.report.split("differences").nth(1).unwrap();
        assert!(diffs.starts_with(" (instrument = the recording, yahaha = our engine):\nbar 20 "), "{}", imp.report);
        assert!(diffs.contains("  ch11 Bass     instrument only: "), "{}", imp.report);
        let got = reference_digest(&imp);
        let want = sim::digest(&sim::snapshot(&style, SCRIPT).unwrap());
        assert_eq!(digest_differences(&got, &want), ["bar 20 ch11"]);

        let fast = fake_recording(&ours, 3.0, 1.01, |_| {});
        let imp = import(&fast, &style, SCRIPT, &ImportOptions::default()).unwrap();
        assert!(!imp.verified, "{}", imp.report);
        assert!(imp.report.contains("leave the tempo alone"), "{}", imp.report);
    }

    /// Every reference digest in tests/reference matches what we play now, except the
    /// differences its `.known` file lists (`bar N chX`, one per line). A listed difference
    /// that is gone is reported so the list can shrink.
    #[test]
    fn reference_captures() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/reference");
        let mut failures = Vec::new();
        for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = e.path();
            let Some(name) = path.file_name().and_then(|f| f.to_str()).and_then(|f| f.strip_suffix(".digest")) else {
                continue;
            };
            let Some(style) = corpus_style(name) else {
                eprintln!("reference: {name} not in corpus; skipping");
                continue;
            };
            let want = std::fs::read_to_string(&path).unwrap();
            let got = sim::digest(&sim::snapshot(&style, SCRIPT).unwrap());
            let known: Vec<String> = std::fs::read_to_string(dir.join(format!("{name}.known")))
                .unwrap_or_default()
                .lines()
                .map(|l| l.split('#').next().unwrap().trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();
            let diffs = digest_differences(&want, &got);
            let new: Vec<&String> = diffs.iter().filter(|d| !known.contains(d)).collect();
            let fixed: Vec<&String> = known.iter().filter(|k| !diffs.contains(k)).collect();
            if !fixed.is_empty() {
                eprintln!("reference: {name}: now matches the hardware in {fixed:?}; remove them from {name}.known");
            }
            if !new.is_empty() {
                failures.push(format!("{name}: differs from the recorded hardware in {new:?}"));
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    /// Committed reference digests hold hashes only, like the golden ones.
    #[test]
    fn committed_references_hold_no_notes() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/reference");
        for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = e.path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            assert!(
                name.ends_with(".digest") || name.ends_with(".known") || name == "README.md",
                "tests/reference/{name}: only digests, .known lists and the README belong here"
            );
            if !name.ends_with(".digest") {
                continue;
            }
            for line in std::fs::read_to_string(&path).unwrap().lines() {
                assert!(!line.contains('~'), "{name}: note listed: {line}");
                if line.starts_with("  ") {
                    let w: Vec<&str> = line.split_whitespace().collect();
                    assert!(w.len() == 3 && w[0].starts_with("ch") && w[2].len() == 16 && w[2].chars().all(|c| c.is_ascii_hexdigit()), "{name}: not a digest line: {line}");
                }
            }
        }
    }

    #[test]
    fn digest_differences_name_bars_and_parts() {
        let want = "bars 2\n\nbar 1  Main A  C@1.0000\n  ch11 Bass         aaaaaaaaaaaaaaaa\n\nbar 2  Main A  G7@1.0000\n  ch12 Chord1       bbbbbbbbbbbbbbbb\n";
        let got = "bars 2\n\nbar 1  Main A  C@1.0000\n  ch11 Bass         aaaaaaaaaaaaaaaa\n\nbar 2  Main B  G7@1.0000\n  ch12 Chord1       cccccccccccccccc\n  ch13 Chord2       dddddddddddddddd\n";
        assert_eq!(digest_differences(want, got), ["bar 2 header", "bar 2 ch12", "bar 2 ch13"]);
        assert!(digest_differences(want, want).is_empty());
    }
}

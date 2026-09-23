//! Reference capture kit (#9): a Genos or Genos2 owner plays our chord script into the
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
//! first chord (Sync Start), exactly as `sim` starts it. Every later step goes out a little
//! ahead of its slot, never on a beat line (`plan`), and our engine plays it at the same
//! tick. A recording's clock is its own, so the importer finds bar 1 by lining up the parts
//! that play as written (drums), which are the same on both sides whatever the chord, and
//! fits the tempo from all matched notes. Notes that agree within the tolerance take our
//! timing, so MIDI jitter never shows up as a difference or changes a digest.

use crate::engine::Button;
use crate::fingering::{self, Fingering};
use crate::sff::{self, Ev, SectionId, Style};
use crate::sim::{self, PlayedNote, ScriptStep, Step, Take, PART_NAMES};
use crate::theory::{self, Chord, Recognizer};
use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The chord script every capture plays.
pub const SCRIPT: &str = include_str!("../docs/capture-kit/capture.script");
/// Owner instructions; `capture-kit` adds a table of the files it wrote.
const INSTRUCTIONS: &str = include_str!("../docs/capture-kit/README.md");

/// A chord that comes this soon after a note started revoices that note outright rather than
/// by its Retrigger Rule: our engine's window (`engine::LATE_CHORD_NS`) when the kit was
/// made, and a guess at the instrument's. Part of the kit's timing (see `plan`), so it stays
/// fixed even if the engine's changes.
const LATE_CHORD_MS: f64 = 40.0;
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
    KitStyle { file: "90sDisco.T161.prs", title: "90sDisco", source: FACTORY, covers: "SFF GE, several source channels per part" },
];

// ---------------------------------------------------------------------------
// The kit: chord keys and the MIDI file
// ---------------------------------------------------------------------------

/// When the kit sends each step of a script on a style, and when our engine plays it for the
/// comparison. Nothing goes out on a beat or bar line, where USB jitter and the drift between
/// the computer's clock and the instrument's (bounded by `import`) would decide which side of
/// the line the instrument sees it: this beat's fill or the next one's, the old chord or the
/// new one on the downbeat.
///
/// - A section button presses half a beat ahead of its slot. The Genos changes sections on a
///   beat or a bar line (RM p.12, Section Change Timing), and half a beat is as far from both
///   as a press can be.
/// - A chord (or release) goes out a little ahead of its slot. A chord a few ms either side
///   of a note start or end would decide whether that note starts on the old chord, is cut
///   or is retriggered, and one that comes `LATE_CHORD_MS` after a note started decides how
///   it is revoiced. So for each position in the beat the script puts chords on, the lead is
///   the shortest, between a 24th and a quarter of a beat, that keeps furthest from all of
///   those points in the sections the script calls up, taken round the beat. It depends
///   only on the style's patterns (the rhythm channels 9 and 10 left out: they play as
///   written whatever the chord), never on what our engine does with them, so a kit file
///   stays valid while the engine changes. The first chord starts the style (Sync Start)
///   and is bar 1 itself.
pub struct Plan {
    pub steps: Vec<ScriptStep>,
    pub bars: u32,
    /// The tick each step acts at, from the start of bar 1 (by index).
    pub acts: Vec<u32>,
    /// The lead (ticks) of the chords on a beat.
    pub chord_lead: u32,
    /// How far (ticks) any chord may land from its tick before it reaches one of those points
    /// or its slot, in any of the sections. (`chord_rooms` narrows it to the sections that
    /// play around each chord.)
    #[cfg_attr(not(test), allow(dead_code))]
    pub margin: u32,
}

/// The positions in the beat (ticks) where a chord landing would decide something in the
/// sections `used`: every note start and end, and every point `LATE_CHORD_MS` after a start.
/// The rhythm channels 9 and 10 are left out: they play as written whatever the chord.
fn edges(style: &Style, used: &[SectionId]) -> Vec<bool> {
    let ppq = style.ppq as u32;
    let late = (LATE_CHORD_MS / 1000.0 * style.bpm() / 60.0 * ppq as f64).round() as u32;
    let mut edges = vec![false; ppq as usize];
    for e in style.sections.values().filter(|s| used.contains(&s.id)).flat_map(|s| &s.events) {
        match e.ev {
            Ev::NoteOn { ch, vel, .. } if ch != 8 && ch != 9 && vel > 0 => {
                edges[(e.tick % ppq) as usize] = true;
                edges[((e.tick + late) % ppq) as usize] = true;
            }
            Ev::NoteOn { ch, .. } | Ev::NoteOff { ch, .. } if ch != 8 && ch != 9 => edges[(e.tick % ppq) as usize] = true,
            _ => {}
        }
    }
    edges
}

/// Distance (ticks) from a position in the beat to the nearest of `edges`, round the beat.
fn room(edges: &[bool], p: u32) -> u32 {
    let ppq = edges.len() as u32;
    (0..ppq).find(|&d| edges[((p + d) % ppq) as usize] || edges[((p + ppq - d) % ppq) as usize]).unwrap_or(ppq)
}

/// The tick a section button presses at: half a beat ahead of its slot.
pub fn press_tick(slot: u32, ppq: u32) -> u32 {
    slot.saturating_sub(ppq / 2)
}

/// Whether a press at tick `t` falls in the first beat of a bar. There the Genos changes
/// section at once rather than at the next bar (RM p.12, Section Change Timing = Next Bar),
/// the one place where our model and its rule differ, so the kit never presses there.
pub fn in_first_beat(t: u32, tpb: u32, ppq: u32) -> bool {
    t % tpb < ppq
}

pub fn plan(style: &Style, script: &str) -> Result<Plan> {
    let ppq = style.ppq as u32;
    let (steps, bars) = sim::parse_script(script, style.ticks_per_bar())?;
    // The sections the script's buttons call up (Main A, and its fill, when it starts).
    let mut used = vec![SectionId::Main(0), SectionId::Fill(0)];
    for s in &steps {
        match s.step {
            Step::Button(Button::Intro(i)) => used.push(SectionId::Intro(i)),
            Step::Button(Button::Main(i)) => used.extend([SectionId::Main(i), SectionId::Fill(i)]),
            Step::Button(Button::Break) => used.push(SectionId::Break),
            Step::Button(Button::Ending(i)) => used.push(SectionId::Ending(i)),
            _ => {}
        }
    }
    let edges = edges(style, &used);
    // The best lead for chords on each position in the beat the script uses.
    let mut leads: Vec<(u32, u32, u32)> = Vec::new();
    for s in steps.iter().filter(|s| s.tick > 0 && !matches!(s.step, Step::Button(_))) {
        let phase = s.tick % ppq;
        if leads.iter().any(|l| l.0 == phase) {
            continue;
        }
        let best = (ppq / 24..=ppq / 4).map(|lead| (phase, lead, room(&edges, (phase + ppq - lead) % ppq).min(lead))).fold((phase, ppq / 24, 0), |b, c| if c.2 > b.2 { c } else { b });
        leads.push(best);
    }
    let lead_at = |t: u32| leads.iter().find(|l| l.0 == t % ppq).map_or(0, |l| l.1);
    let acts = steps
        .iter()
        .map(|s| match s.step {
            _ if s.tick == 0 => 0,
            Step::Button(_) => press_tick(s.tick, ppq),
            _ => s.tick - lead_at(s.tick),
        })
        .collect();
    let margin = leads.iter().map(|l| l.2).min().unwrap_or(0);
    let chord_lead = leads.iter().find(|l| l.0 == 0).map_or(0, |l| l.1);
    Ok(Plan { steps, bars, acts, chord_lead, margin })
}

/// How far (ticks) each chord of a take (by step index; the first, which starts the style,
/// left out) may land from where it acts before it meets a note start or end, or the point
/// `LATE_CHORD_MS` after a start, in the sections playing around it, or reaches its slot.
fn chord_rooms(style: &Style, take: &Take) -> Vec<(usize, u32)> {
    let ppq = style.ppq as u32;
    (0..take.steps.len())
        .filter(|&i| take.steps[i].tick > 0 && matches!(take.steps[i].step, Step::Chord(_)))
        .map(|i| {
            let (slot, act) = (take.steps[i].tick, take.acts[i]);
            let lead = slot - act;
            let used: Vec<SectionId> = [act.saturating_sub(lead), act, slot].iter().filter_map(|&t| take.section_at(t)).collect();
            (i, room(&edges(style, &used), act % ppq).min(lead))
        })
        .collect()
}

/// The clock difference (ppm) between the computer and the instrument that `rooms` take:
/// the chord that goes out at `act` slips by ppm * act from where the first chord put the
/// style, and must stay within its room. (The first chord starts the style at tick 0.)
fn clock_budget(take: &Take, rooms: &[(usize, u32)]) -> f64 {
    rooms.iter().map(|&(i, r)| r as f64 / take.acts[i].max(1) as f64 * 1e6).fold(f64::INFINITY, f64::min)
}

/// The clock difference (ppm) the kit tolerates on `style`: see `import`. It depends on the
/// style's patterns and on our section timeline, so the importer works it out again for
/// each recording.
pub fn clock_budget_ppm(style: &Style, script: &str) -> Result<f64> {
    let take = perform(style, script)?;
    Ok(clock_budget(&take, &chord_rooms(style, &take)))
}

/// Our engine's take of `script` on `style`, with the steps timed as the kit sends them.
pub fn perform(style: &Style, script: &str) -> Result<Take> {
    let p = plan(style, script)?;
    Ok(sim::perform_steps(style, p.steps, p.bars, p.acts))
}

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
/// message for every button, each sent at its `plan` tick. A marker names each bar. Main A
/// is selected during the lead-in, as it is when `sim` starts.
///
/// `clock_ppm` is how fast the owner's instrument clock runs against their computer's, as
/// `capture-import` measured it from an earlier recording (0 for a new owner): the file runs
/// that much faster, so the chords keep to the instrument's beat.
pub fn kit_midi(style: &Style, script: &str, clock_ppm: f64) -> Result<Vec<u8>> {
    let (ppq, tpb) = (style.ppq as u32, style.ticks_per_bar());
    let Plan { steps, bars, acts, .. } = plan(style, script)?;
    let t0 = LEAD_IN_BARS * tpb;
    let rec = Recognizer::new();
    let us = (60e6 / (style.bpm() * (1.0 + clock_ppm * 1e-6))).round() as u32;
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
    for (s, &act) in steps.iter().zip(&acts) {
        // A button before the first chord (an Intro) goes out in the lead-in.
        let t = if s.tick == 0 && matches!(s.step, Step::Button(_)) { t0 - ppq / 2 } else { t0 + act };
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
                if s.tick > 0 && in_first_beat(act, tpb, ppq) {
                    bail!("{} in bar {} presses in the first beat of a bar, where the Genos changes section at once", s.label, act / tpb + 1);
                }
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
/// in ./corpus when none are given) and the owner instructions, README.md. `clock_ppm`: see
/// `kit_midi`.
pub fn write_kit(out: &Path, styles: &[PathBuf], clock_ppm: f64) -> Result<()> {
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
        "\n## Files in this kit\n\n| MIDI file | Style to load | Where to get it | Tempo | Length | Covers | Clock tolerance |\n|---|---|---|---|---|---|---|\n",
    );
    if clock_ppm != 0.0 {
        table = format!("\nThese files run {clock_ppm:+.0} ppm fast, to match the clock of the instrument they were made for.\n{table}");
    }
    for (path, kit) in &paths {
        let style = Style::load(path)?;
        let (_, bars) = sim::parse_script(SCRIPT, style.ticks_per_bar())?;
        let stem = path.file_name().unwrap().to_string_lossy().split('.').next().unwrap_or("style").to_string();
        let file = format!("{stem}.capture.mid");
        std::fs::write(out.join(&file), kit_midi(&style, SCRIPT, clock_ppm)?)?;
        let secs = (bars + LEAD_IN_BARS) as f64 * style.ticks_per_bar() as f64 / style.ppq as f64 * 60.0 / style.bpm();
        table += &format!(
            "| `{file}` | {} (`{}`) | {} | {:.0} bpm | {bars} bars + {LEAD_IN_BARS} lead-in, {}:{:02} | {} | {:.0} ppm |\n",
            kit.map_or(stem.as_str(), |k| k.title),
            path.file_name().unwrap().to_string_lossy(),
            kit.map_or("-", |k| k.source),
            style.bpm(),
            secs as u32 / 60,
            secs as u32 % 60,
            kit.map_or("-", |k| k.covers),
            clock_budget_ppm(&style, SCRIPT)?,
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
    /// The `--clock-ppm` the kit file was made with.
    pub clock_ppm: f64,
}

impl Default for ImportOptions {
    fn default() -> Self {
        ImportOptions { tolerance_ms: 8.0, offset_ms: None, clock_ppm: 0.0 }
    }
}

pub struct Import {
    /// What the instrument played, as a take of our script. Its steps are the script's; its
    /// sections are ours (the instrument does not send its own), so only the report and
    /// `--listing` use them, never the reference digest (`Take::render_reference`).
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
    let Plan { steps, bars, acts, chord_lead, .. } = plan(style, script)?;
    let ours = sim::perform_steps(style, steps, bars, acts);
    let rooms = chord_rooms(style, &ours);
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
            // Its start is ours, but its end can come a burst of MIDI traffic late.
            match our.and_then(|o| o.len) {
                Some(ol) if (ol as f64 - l as f64).abs() <= 2.0 * tol => ol,
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
    let ms = |ticks: f64| ticks / per_sec * 1000.0;
    // How far matched notes sit from ours after the fit: the MIDI jitter. If it comes near the
    // tolerance, the tolerance is too tight for this setup (or too loose, if it is far below).
    let mut spread: Vec<f64> = rec.iter().zip(&matched).filter_map(|(r, m)| m.map(|i| (a * r.x + b - ours.notes[i].tick as f64).abs())).collect();
    spread.sort_by(f64::total_cmp);
    let pct = |p: f64| spread.get(((spread.len() as f64 * p) as usize).min(spread.len().saturating_sub(1))).map_or(0.0, |&t| ms(t));
    out += &format!(
        "alignment: bar 1 at {:.3} s in the recording; tempo {:.3} bpm (style {:.3}, ratio {:.6}); {matched_n} of {} recorded notes within {} ms of ours (median {:.1} ms off, 99% within {:.1} ms)\n",
        secs(0.0),
        ours.bpm * a,
        ours.bpm,
        a,
        rec.len(),
        opts.tolerance_ms,
        pct(0.5),
        pct(0.99)
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

    // Chords the instrument read, against the ones the script meant, and when it read them.
    // The kit sends each chord on the computer's clock; the style plays on the instrument's.
    // A chord that slipped by its room (`chord_rooms`) may have met a note on the other side
    // than in our take, so the room is the limit. The slip is measured from the first chord,
    // which started the style, so the instrument's own delay in reading a chord cancels out:
    // from the chord messages, or without them from the drift since the first chord.
    let chord_steps: Vec<usize> = (0..ours.steps.len()).filter(|&i| matches!(ours.steps[i].step, Step::Chord(_))).collect();
    let act = |i: usize| ours.acts[i] as f64;
    let reach = ours.ppq as f64 / 4.0;
    // (slip, room, step) of the chord that came closest to its room, as a share of it.
    let mut worst: Option<(f64, f64, usize)> = None;
    let mut slipped = |slip: f64, i: usize| {
        let Some(&(_, r)) = rooms.iter().find(|r| r.0 == i) else { return };
        let r = r as f64;
        if worst.is_none_or(|(s, wr, _)| slip * wr > s * r) {
            worst = Some((slip, r, i));
        }
    };
    // The instrument's clock against the computer's (the recorder's), and against the kit's.
    let clock_ppm = (a - 1.0) * 1e6;
    let kit_ratio = a / (1.0 + opts.clock_ppm * 1e-6);
    if chords.is_empty() {
        out += "chords: the recording has no Chord Control messages (turn on Chord System Exclusive Message Transmit to check them)\n";
        for &i in &chord_steps {
            slipped((kit_ratio - 1.0).abs() * act(i), i);
        }
    } else {
        let (mut same, mut silent, mut wrong) = (0, 0, Vec::new());
        let mut delay = None;
        for (k, &i) in chord_steps.iter().enumerate() {
            let (s, Step::Chord(want)) = (&ours.steps[i], ours.steps[i].step) else { continue };
            let lo = act(i) - reach;
            let hi = chord_steps.get(k + 1).map_or(end, |&n| act(n) - reach).min(act(i) + ours.ppq as f64);
            let got = chords.iter().rev().map(|(x, c)| (a * x + b, *c)).find(|(t, _)| *t >= lo && *t < hi);
            if let Some((t, _)) = got {
                let d = *delay.get_or_insert(t - act(i));
                slipped((t - act(i) - d).abs(), i);
            }
            match got.map(|(_, c)| c) {
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
        out += &format!("chords: {same} of {} chord changes read as meant, {} read differently, {silent} with no chord message\n", chord_steps.len(), wrong.len());
        for w in &wrong {
            out += w;
        }
    }
    let budget = clock_budget(&ours, &rooms);
    if let Some((slip, room, i)) = worst {
        let s = &ours.steps[i];
        let at = format!("{} in bar {}", s.label, s.tick / ours.tpb + 1);
        out += &format!(
            "chord timing: sent {:.1} ms ahead of the beat; closest call {at}, {:.1} ms from where yahaha plays it (room {:.1} ms)\n",
            ms(chord_lead as f64),
            ms(slip),
            ms(room)
        );
        let kit = if opts.clock_ppm != 0.0 { format!(", the kit ran {:+.0} ppm fast", opts.clock_ppm) } else { String::new() };
        out += &format!("clocks: the instrument's runs {clock_ppm:+.1} ppm against the computer's{kit}; this style takes up to {budget:.0} ppm between the kit and the instrument\n");
        if slip >= room {
            // The drift between a computer and an instrument stays put, so recording again
            // cannot help; a kit that runs at the instrument's speed does.
            problems.push(format!(
                "{at} reached the instrument {:.1} ms from where yahaha plays it, past the {:.1} ms before a note would follow another chord than in our take. The instrument's clock runs {clock_ppm:+.0} ppm against the computer's, and this style takes {budget:.0} ppm. Recording again on the same computer and instrument will not help: make the owner a kit with `yahaha capture-kit <dir> --clock-ppm {clock_ppm:.0}`, which keeps to the instrument's clock, and import that recording with the same --clock-ppm",
                ms(slip),
                ms(room),
            ));
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
    // A note-on and note-off on one tick sounds nothing, and the instrument may not send one.
    let zero = ours.notes.iter().filter(|n| n.len == Some(0)).count();
    if zero > 0 {
        let unmatched = (0..ours.notes.len()).filter(|&i| ours.notes[i].len == Some(0) && !hit[i]).count();
        out += &format!("  yahaha plays {zero} zero-length notes (on and off on one tick); {unmatched} of them are yahaha only\n");
    }
    // Where each fill starts is our reading of the Genos (RM p.12 does not say): the drums
    // of every fill bar, on a line of their own, show whether the instrument agrees.
    let fills: Vec<(u32, SectionId)> = ours.sections.iter().filter_map(|&(t, id)| id.filter(|id| matches!(id, SectionId::Fill(_))).map(|id| (t, id))).collect();
    if !fills.is_empty() {
        out += "\nfill drums (the fill as yahaha starts it; drums that differ mean the instrument started it elsewhere):\n";
        for (t, id) in fills {
            let bar = t / ours.tpb;
            let (mut only_hw, mut only_ours) = (0, 0);
            for ch in [8, 9] {
                let ((hi, _), (oi, _)) = (hardware.part_items(bar, ch), ours.part_items(bar, ch));
                only_hw += minus(&hi, &oi).len();
                only_ours += minus(&oi, &hi).len();
            }
            let what = if only_hw + only_ours == 0 { "same".to_string() } else { format!("differ: {only_hw} instrument only, {only_ours} yahaha only") };
            out += &format!("  bar {} {} from beat {}: drums {what}\n", bar + 1, id.name(), ours.pos(t));
        }
    }
    if !diffs.is_empty() {
        out += "\ndifferences (instrument = the recording, yahaha = our engine):\n";
        out += &diffs;
    }
    Ok(Import { hardware, ours, report: out, verified, differing_bars })
}

/// The reference digest of a recording, as `tests/reference/<style file>.digest`.
pub fn reference_digest(imp: &Import) -> String {
    sim::digest(&imp.hardware.render_reference())
}

/// Write `<style file>.digest` and `<style file>.known` for a recording into `dir`: the
/// recording's reference digest, and the `bar N chX` keys where yahaha differs from it today
/// (the reference test allows these until they are fixed, and says so once they are).
/// Refuses an unverified recording unless `force`. Returns the number of known differences.
pub fn write_reference(imp: &Import, dir: &Path, style_file: &str, recording: &Path, force: bool) -> Result<usize> {
    if !imp.verified && !force {
        bail!("the recording is not verified (see above); not writing a reference digest (--force overrides)");
    }
    std::fs::create_dir_all(dir)?;
    let reference = reference_digest(imp);
    std::fs::write(dir.join(format!("{style_file}.digest")), &reference)?;
    let diffs = digest_differences(&reference, &sim::digest(&imp.ours.render_reference()));
    // The file name only: the full path would put the owner's home directory in the repo.
    let from = recording.file_name().map_or("the recording".into(), |f| f.to_string_lossy());
    let mut text = format!("# Where yahaha differed from {from} when it was imported. One `bar N chX` per line.\n");
    for d in &diffs {
        text += d;
        text.push('\n');
    }
    std::fs::write(dir.join(format!("{style_file}.known")), text)?;
    Ok(diffs.len())
}

/// `yahaha capture-import <recording.mid> <style> [options]`.
pub fn import_cmd(args: &[String]) -> Result<()> {
    let usage = "usage: yahaha capture-import <recording.mid> <style> [--tolerance-ms N] [--offset-ms N] [--clock-ppm N] [--listing FILE] [--golden DIR [--force]]";
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
            "--clock-ppm" => opts.clock_ppm = value()?.parse()?,
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
        let name = style_path.file_name().unwrap().to_string_lossy().to_string();
        let known = write_reference(&imp, &dir, &name, Path::new(recording), force)?;
        println!("wrote {} and {} ({known} known differences)", dir.join(format!("{name}.digest")).display(), dir.join(format!("{name}.known")).display());
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

    /// What yahaha plays for the capture script on `style`, in the reference digest's form.
    fn our_reference_digest(style: &Style) -> Result<String> {
        Ok(sim::digest(&perform(style, SCRIPT)?.render_reference()))
    }

    /// Every chord in the script fits the chord section and reads back as itself, no chord is
    /// struck again while it is held, every button has a Section Control message and presses
    /// outside a bar's first beat, and the script covers what the kit promises.
    #[test]
    fn script_reads_back_and_covers_the_kit() {
        let rec = Recognizer::new();
        let (steps, bars) = sim::parse_script(SCRIPT, 1920).unwrap();
        let mut types = [false; 38];
        let (mut slash, mut busiest) = (0, 0);
        let mut held: Option<Chord> = None;
        for s in &steps {
            // A key let go and pressed again on the same tick is no legato player's chord
            // change, and the instrument may not even send a chord message for it.
            if let Step::Chord(c) = s.step {
                assert_ne!(held, Some(c), "{} at tick {} strikes the held chord again; write \"-\"", s.label, s.tick);
            }
            held = match s.step {
                Step::Chord(c) => Some(c),
                Step::Release => None,
                _ => held,
            };
            match s.step {
                Step::Chord(c) => {
                    let keys = voicing(c);
                    assert!(keys.iter().all(|&k| (LOWEST_KEY..=SPLIT).contains(&(k as i32))), "{}: keys {keys:?} outside E0..F#2", s.label);
                    assert_eq!(read_back(&rec, &keys), Some(c), "{} ({keys:?}) reads as {:?}", s.label, read_back(&rec, &keys).map(|c| c.name()));
                    types[c.ty as usize] = true;
                    slash += c.bass.is_some() as usize;
                }
                Step::Button(b) => {
                    assert!(section_code(b).is_some(), "{} has no Section Control message", s.label);
                    assert!(s.tick == 0 || !in_first_beat(press_tick(s.tick, 480), 1920, 480), "{} at tick {} presses in the first beat of a bar", s.label, s.tick);
                }
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

    /// The kit file plays the script's keys on channel 1 after the lead-in, a 64th note ahead
    /// of their slot except the first, with a Section Control press for every button half a
    /// beat ahead of its slot.
    #[test]
    fn kit_midi_plays_the_script() {
        let Some(style) = corpus_style(KIT[0].file) else {
            return;
        };
        let bytes = kit_midi(&style, "[IntroA] | C | Am - - [MainB] - |", 0.0).unwrap();
        let ev = read_smf(&bytes).unwrap();
        let ppq_sec = 60.0 / style.bpm() / style.ppq as f64;
        let tick = |s: f64| (s / ppq_sec).round() as u32;
        let (t0, tpb, ppq) = (LEAD_IN_BARS * style.ticks_per_bar(), style.ticks_per_bar(), style.ppq as u32);
        let ons: Vec<(u32, u8)> = ev.iter().filter_map(|(s, e)| match e {
            Ev::NoteOn { ch: 0, key, .. } => Some((tick(*s), *key)),
            _ => None,
        }).collect();
        let am = t0 + plan(&style, "[IntroA] | C | Am - - [MainB] - |").unwrap().acts[2];
        assert_eq!(ons, [(t0, 36), (t0, 40), (t0, 43), (am, 45), (am, 48), (am, 52)]);
        let sections: Vec<(u32, Vec<u8>)> = ev.iter().filter_map(|(s, e)| match e {
            Ev::Sysex(d) if d[5] == 0x7F => Some((tick(*s), d.clone())),
            _ => None,
        }).collect();
        let mb = t0 + tpb + 3 * ppq - ppq / 2;
        assert_eq!(sections, [(ppq, section_control(0x08, true)), (t0 - ppq / 2, section_control(0, true)), (mb, section_control(0x09, true))]);
        // No step goes out within 5% of a beat of a beat line (the first chord starts the style).
        for (s, e) in &ev {
            let t = tick(*s);
            if t > t0 && matches!(e, Ev::NoteOn { ch: 0, .. } | Ev::Sysex(_)) {
                let off = (t % ppq).min(ppq - t % ppq);
                assert!(off >= ppq / 20, "an event {off} ticks from a beat line at tick {t}");
            }
        }
        let offs = ev.iter().filter(|(_, e)| matches!(e, Ev::NoteOff { ch: 0, .. })).count();
        assert_eq!(offs, 6, "every key is let go");
        assert!(kit_midi(&style, "| C [SyncStop] |", 0.0).is_err());
    }

    /// A button presses half a beat ahead of its slot, so one on beat 2 presses in beat 1
    /// (refused) and one on beat 3 presses in beat 2 (fine), and one on a downbeat presses in
    /// the bar before (fine).
    #[test]
    fn first_beat_presses_are_refused() {
        let (tpb, ppq) = (1920, 480);
        let pressed = |slot: u32| in_first_beat(press_tick(slot, ppq), tpb, ppq);
        assert!(pressed(2400), "slot beat 2 of bar 2: press at 2160, in beat 1");
        assert!(!pressed(2880), "slot beat 3 of bar 2: press at 2640, in beat 2");
        assert!(!pressed(3840), "slot on bar 3's downbeat: press at 3600, in bar 2's beat 4");
        let Some(style) = corpus_style(KIT[0].file) else {
            return;
        };
        assert_eq!(style.ticks_per_bar(), 4 * style.ppq as u32, "a 4/4 style");
        let err = kit_midi(&style, "| C | C [MainB] - - - |", 0.0).unwrap_err().to_string();
        assert!(err.contains("[MainB] in bar 2 presses in the first beat"), "{err}");
        assert!(kit_midi(&style, "| C | C - [MainB] - - |", 0.0).is_ok());
        assert!(kit_midi(&style, "| C | C - - - | [MainB] G |", 0.0).is_ok());
    }

    /// How a fake recording differs from a perfect one.
    struct Fake {
        /// Random MIDI jitter on the style notes, up to this many ms either way.
        jitter_ms: f64,
        /// The instrument's clock against the computer's: the style runs this much faster,
        /// while the chords and the recorder keep the computer's time.
        speed: f64,
        /// The recorder's tempo (its ticks mean nothing to the instrument).
        rec_bpm: f64,
        /// The kit file runs this much faster than the style (`capture-kit --clock-ppm`).
        kit_speed: f64,
        /// Style notes played before bar 1 (the owner trying the style first).
        noodling: bool,
        /// Type 1, with the tempo on a track of its own.
        type1: bool,
    }

    const FAKE: Fake = Fake { jitter_ms: 4.0, speed: 1.0, rec_bpm: 120.0, kit_speed: 1.0, noodling: false, type1: false };

    /// A fake recording of a take: other ppq and clock, bar 1 somewhere in the file, MIDI
    /// jitter, chord messages, and the instrument's keyboard echo on channel 4.
    fn fake(take: &Take, f: &Fake, tweak: impl Fn(&mut PlayedNote)) -> Vec<u8> {
        let ppq = 480u16;
        let bar1 = 3.217;
        let per_sec = take.bpm / 60.0 * take.ppq as f64;
        let rec_tick = |sec: f64| (sec * f.rec_bpm / 60.0 * ppq as f64).round() as u32;
        // A style note plays on the instrument's clock; a chord arrives on the computer's.
        let style_at = |t: u32| rec_tick(bar1 + t as f64 / per_sec / f.speed);
        let daw_at = |t: u32| rec_tick(bar1 + t as f64 / per_sec / f.kit_speed);
        let tempo = meta(0x51, &((60e6 / f.rec_bpm).round() as u32).to_be_bytes()[1..]);
        let mut ev: Vec<(u32, Vec<u8>)> = Vec::new();
        let mut seed = 12345u32;
        let mut jit = || {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
            let j = ((seed >> 16) % 1000) as f64 / 1000.0 * 2.0 - 1.0;
            (j * f.jitter_ms / 1000.0 * f.rec_bpm / 60.0 * ppq as f64).round() as i64
        };
        let mut notes: Vec<(u32, Vec<u8>)> = Vec::new();
        for n in &take.notes {
            let mut n = *n;
            tweak(&mut n);
            notes.push((style_at(n.tick), vec![0x90 | n.ch, n.key, 90]));
            // A note still sounding at the end of the script is let go a bar later.
            notes.push((style_at(n.tick + n.len.unwrap_or(take.bars * take.tpb + take.tpb - n.tick)), vec![0x80 | n.ch, n.key, 0]));
        }
        if f.noodling {
            for (i, key) in [60u8, 64, 67].into_iter().enumerate() {
                let t = rec_tick(1.0 + i as f64 * 0.2);
                notes.push((t, vec![0x9B, key, 90]));
                notes.push((t + 50, vec![0x8B, key, 0]));
            }
        }
        // Jitter moves the time stamps but, like a real MIDI cable, never the order.
        notes.sort_by_key(|e| e.0);
        let mut last = 0;
        for (t, m) in notes {
            last = (t as i64 + jit()).max(last);
            ev.push((last as u32, m));
        }
        for (s, &act) in take.steps.iter().zip(&take.acts) {
            if let Step::Chord(c) = s.step {
                let t = daw_at(act) + 2;
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
        if !f.type1 {
            ev.push((0, tempo));
            return write_smf(ppq, ev);
        }
        // Type 1: the tempo track, then the performance; each is a type 0 file's track chunk.
        let chunk = |events: Vec<(u32, Vec<u8>)>| write_smf(ppq, events)[14..].to_vec();
        let mut out = b"MThd".to_vec();
        out.extend_from_slice(&6u32.to_be_bytes());
        out.extend_from_slice(&[0, 1, 0, 2]);
        out.extend_from_slice(&ppq.to_be_bytes());
        out.extend(chunk(vec![(0, tempo)]));
        out.extend(chunk(ev));
        out
    }

    fn fake_recording(take: &Take, jitter_ms: f64, speed: f64, tweak: impl Fn(&mut PlayedNote)) -> Vec<u8> {
        fake(take, &Fake { jitter_ms, speed, ..FAKE }, tweak)
    }

    /// A recording of exactly what we play (jittered, shifted, another clock) imports with no
    /// differences, verifies, and its reference digest is our own. Every kit style in the
    /// corpus.
    #[test]
    fn import_round_trip() {
        for k in &KIT {
            let Some(style) = corpus_style(k.file) else {
                continue;
            };
            let ours = perform(&style, SCRIPT).unwrap();
            let rec = fake_recording(&ours, 4.0, 1.0, |_| {});
            let imp = import(&rec, &style, SCRIPT, &ImportOptions::default()).unwrap();
            let what = format!("{}:\n{}", k.file, imp.report);
            assert!(imp.verified, "{what}");
            assert_eq!(imp.differing_bars, 0, "{what}");
            assert!(imp.report.contains("bar 1 at 3.21"), "{what}");
            assert!(imp.report.contains("read as meant, 0 read differently, 0 with no chord message"), "{what}");
            // Every fill's drums on their own line, all the same; zero-length notes counted.
            assert!(imp.report.contains("\nfill drums ") && imp.report.contains(": drums same\n") && !imp.report.contains("drums differ"), "{what}");
            let zero = ours.notes.iter().filter(|n| n.len == Some(0)).count();
            assert!(zero == 0 || imp.report.contains(&format!("yahaha plays {zero} zero-length notes (on and off on one tick); 0 of them are yahaha only")), "{what}");
            assert_eq!(reference_digest(&imp), our_reference_digest(&style).unwrap(), "{what}");
        }
    }

    /// A type 1 file with the tempo on its own track, at another tempo, with notes played
    /// before bar 1: those are ignored and said so, and the rest imports as before.
    #[test]
    fn import_type1_with_notes_before_bar_1() {
        let Some(style) = corpus_style(KIT[1].file) else {
            return;
        };
        let ours = perform(&style, SCRIPT).unwrap();
        let rec = fake(&ours, &Fake { rec_bpm: 97.0, noodling: true, type1: true, ..FAKE }, |_| {});
        assert_eq!(&rec[8..12], &[0, 1, 0, 2], "a type 1 file with two tracks");
        let imp = import(&rec, &style, SCRIPT, &ImportOptions::default()).unwrap();
        assert!(imp.verified, "{}", imp.report);
        assert!(imp.report.contains("bar 1 at 3.21"), "{}", imp.report);
        assert!(imp.report.contains("3 recorded notes before bar 1 ignored"), "{}", imp.report);
        assert_eq!(imp.differing_bars, 0, "{}", imp.report);
        assert_eq!(reference_digest(&imp), our_reference_digest(&style).unwrap());
    }

    /// A changed bass note shows up in its bar and part only; a wrong tempo fails verification.
    #[test]
    fn import_reports_differences() {
        let Some(style) = corpus_style(KIT[0].file) else {
            return;
        };
        let ours = perform(&style, SCRIPT).unwrap();
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
        assert_eq!(digest_differences(&reference_digest(&imp), &our_reference_digest(&style).unwrap()), ["bar 20 ch11"]);

        let fast = fake_recording(&ours, 3.0, 1.01, |_| {});
        let imp = import(&fast, &style, SCRIPT, &ImportOptions::default()).unwrap();
        assert!(!imp.verified, "{}", imp.report);
        assert!(imp.report.contains("leave the tempo alone"), "{}", imp.report);
    }

    /// A fake recording with its Chord Control messages taken out.
    fn without_chord_messages(rec: &[u8]) -> Vec<u8> {
        let per_q = 60.0 / FAKE.rec_bpm / 480.0;
        let ev: Vec<(u32, Vec<u8>)> = read_smf(rec)
            .unwrap()
            .into_iter()
            .filter_map(|(s, e)| {
                let t = (s / per_q).round() as u32;
                match e {
                    Ev::NoteOn { ch, key, vel } => Some((t, vec![0x90 | ch, key, vel])),
                    Ev::NoteOff { ch, key } => Some((t, vec![0x80 | ch, key, 0])),
                    _ => None,
                }
            })
            .collect();
        write_smf(480, ev)
    }

    /// B2: the computer's and the instrument's clocks, over every kit style. Off by half the
    /// style's clock budget, a recording verifies. Off by twice the budget it does not, with
    /// or without chord messages, and the report says what helps: a kit made with
    /// `--clock-ppm` at the measured drift, which verifies again. Recording again does not.
    #[test]
    fn import_clock_drift_over_every_style() {
        let mut ran = false;
        for k in &KIT {
            let Some(style) = corpus_style(k.file) else {
                continue;
            };
            ran = true;
            let ours = perform(&style, SCRIPT).unwrap();
            let budget = clock_budget_ppm(&style, SCRIPT).unwrap();
            eprintln!("{}: clock budget {budget:.0} ppm", k.file);
            let opts = ImportOptions::default();
            let fine = fake(&ours, &Fake { jitter_ms: 2.0, speed: 1.0 + 0.5 * budget * 1e-6, ..FAKE }, |_| {});
            let imp = import(&fine, &style, SCRIPT, &opts).unwrap();
            assert!(imp.verified, "{} at {:.0} ppm:\n{}", k.file, 0.5 * budget, imp.report);

            // Twice the budget, and never past the tempo check: the chord timing catches it.
            let ppm = (2.0 * budget).min(1900.0);
            assert!(ppm > budget * 1.2, "{}: a {budget:.0} ppm budget is past the tempo check", k.file);
            let speed = 1.0 + ppm * 1e-6;
            let drifting = fake(&ours, &Fake { jitter_ms: 2.0, speed, ..FAKE }, |_| {});
            let mut measured = Vec::new();
            for rec in [drifting.clone(), without_chord_messages(&drifting)] {
                let imp = import(&rec, &style, SCRIPT, &opts).unwrap();
                let what = format!("{} at {ppm:.0} ppm:\n{}", k.file, imp.report);
                assert!(!imp.verified, "{what}");
                assert!(!imp.report.contains("leave the tempo alone"), "{what}");
                assert!(!imp.report.contains("record again"), "{what}");
                let advice = imp.report.split("will not help: make the owner a kit with `yahaha capture-kit <dir> --clock-ppm ").nth(1).expect(&what);
                let p: f64 = advice.split('`').next().unwrap().parse().unwrap();
                assert!((p - ppm).abs() <= 3.0, "{what}");
                measured.push(p);
            }
            // The kit made for that instrument, at the drift the report measured, keeps to its
            // clock.
            let kit_speed = 1.0 + measured[0] * 1e-6;
            let corrected = fake(&ours, &Fake { jitter_ms: 2.0, speed, kit_speed, ..FAKE }, |_| {});
            let opts = ImportOptions { clock_ppm: measured[0], ..ImportOptions::default() };
            for rec in [corrected.clone(), without_chord_messages(&corrected)] {
                let imp = import(&rec, &style, SCRIPT, &opts).unwrap();
                assert!(imp.verified, "{} at {ppm:.0} ppm with a corrected kit:\n{}", k.file, imp.report);
                assert_eq!(reference_digest(&imp), our_reference_digest(&style).unwrap(), "{}", k.file);
            }
        }
        if !ran {
            eprintln!("no corpus; skipping");
        }
    }

    /// `--clock-ppm` makes the kit file's tempo that much faster, to the microsecond the
    /// tempo meta event carries, and changes nothing else.
    #[test]
    fn kit_clock_correction_sets_the_tempo() {
        let Some(style) = corpus_style(KIT[0].file) else {
            return;
        };
        let tempo = |bytes: &[u8]| {
            read_smf(bytes).unwrap().into_iter().find_map(|(_, e)| match e {
                Ev::Meta { ty: 0x51, data } => Some(u32::from_be_bytes([0, data[0], data[1], data[2]])),
                _ => None,
            })
        };
        let (plain, fast) = (kit_midi(&style, SCRIPT, 0.0).unwrap(), kit_midi(&style, SCRIPT, 120.0).unwrap());
        let (p, f) = (tempo(&plain).unwrap() as f64, tempo(&fast).unwrap() as f64);
        assert!((p / f - 1.000120).abs() < 1.5 / p, "{p} us against {f} us");
        assert_eq!(plain.len(), fast.len());
    }

    /// A section button the instrument did not take changes the drums: not verified.
    #[test]
    fn import_rejects_a_missed_section_change() {
        let Some(style) = corpus_style(KIT[0].file) else {
            return;
        };
        let script = SCRIPT.replacen("[MainB] G7", "G7", 1);
        assert_ne!(script, SCRIPT);
        let missed = perform(&style, &script).unwrap();
        let imp = import(&fake_recording(&missed, 2.0, 1.0, |_| {}), &style, SCRIPT, &ImportOptions::default()).unwrap();
        assert!(!imp.verified, "{}", imp.report);
        assert!(imp.report.contains("notes of the parts that play as written (drums) match"), "{}", imp.report);
    }

    /// `--golden` refuses an unverified recording unless forced, and writes the digest and a
    /// `.known` list naming the recording by file name only.
    #[test]
    fn write_reference_files() {
        let Some(style) = corpus_style(KIT[0].file) else {
            return;
        };
        let dir = std::env::temp_dir().join(format!("yahaha-reference-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let recording = Path::new("/Users/someone/Music/OrganCruise.recording.mid");
        let ours = perform(&style, SCRIPT).unwrap();

        let fast = import(&fake_recording(&ours, 3.0, 1.01, |_| {}), &style, SCRIPT, &ImportOptions::default()).unwrap();
        assert!(write_reference(&fast, &dir, KIT[0].file, recording, false).is_err());
        assert!(!dir.join(format!("{}.digest", KIT[0].file)).exists());
        assert!(write_reference(&fast, &dir, KIT[0].file, recording, true).is_ok(), "--force writes it anyway");

        let tpb = ours.tpb;
        let target = *ours.notes.iter().find(|n| n.ch == 10 && n.tick / tpb == 19 && !ours.as_written(n.tick, 10)).unwrap();
        let rec = fake_recording(&ours, 0.0, 1.0, |n| {
            if *n == target {
                n.key += 2;
            }
        });
        let imp = import(&rec, &style, SCRIPT, &ImportOptions::default()).unwrap();
        assert_eq!(write_reference(&imp, &dir, KIT[0].file, recording, false).unwrap(), 1);
        let digest = std::fs::read_to_string(dir.join(format!("{}.digest", KIT[0].file))).unwrap();
        assert_eq!(digest, reference_digest(&imp));
        let known = std::fs::read_to_string(dir.join(format!("{}.known", KIT[0].file))).unwrap();
        assert_eq!(known, "# Where yahaha differed from OrganCruise.recording.mid when it was imported. One `bar N chX` per line.\nbar 20 ch11\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The reference digest depends on the notes and the script only: moving one of our
    /// section changes (as an M2 fix might) leaves it alone, while the golden listing changes.
    #[test]
    fn reference_digest_ignores_our_sections() {
        let Some(style) = corpus_style(KIT[0].file) else {
            return;
        };
        let ours = perform(&style, SCRIPT).unwrap();
        let mut moved = ours.clone();
        let k = moved.sections.iter().position(|(t, _)| *t > 0).unwrap();
        moved.sections[k].0 += moved.ppq;
        assert_ne!(moved.render(), ours.render());
        assert_eq!(moved.render_reference(), ours.render_reference());
    }

    /// B1: whether a step reaches the instrument a few ms early or late does not change what
    /// it plays, because the kit sends nothing on a beat line. Steps sent on the line (as the
    /// golden snapshots time them) do change it when they arrive late.
    #[test]
    fn kit_timing_survives_jitter() {
        // The same sections, and the same notes give or take the shift: a note the chord
        // cuts or retriggers moves with it.
        let same = |a: &Take, b: &Take, d: u32| -> Result<(), String> {
            if a.sections != b.sections {
                return Err("the sections change".into());
            }
            let rec: Vec<RecNote> = b.notes.iter().map(|n| RecNote { x: n.tick as f64, ch: n.ch, key: n.key, end: n.len.map(|l| (n.tick + l) as f64) }).collect();
            let m = match_notes(&rec, &a.notes, 1.0, 0.0, 2.0 * d as f64);
            for (r, m) in rec.iter().zip(&m) {
                let Some(i) = *m else {
                    return Err(format!("only the shifted take plays key {} on ch{} at tick {}", r.key, r.ch + 1, r.x));
                };
                let ends = (a.notes[i].len.map(|l| (a.notes[i].tick + l) as f64), r.end);
                if !matches!(ends, (None, None)) && !matches!(ends, (Some(p), Some(q)) if (p - q).abs() <= 2.0 * d as f64) {
                    return Err(format!("key {} on ch{} at tick {}: ends {ends:?}", r.key, r.ch + 1, r.x));
                }
            }
            if m.iter().flatten().count() != a.notes.len() {
                return Err(format!("{} notes against {}", a.notes.len(), b.notes.len()));
            }
            Ok(())
        };
        let shifted = |take: &Take, by: i64| -> Vec<u32> {
            take.steps.iter().zip(&take.acts).map(|(s, &t)| if s.tick == 0 { t } else { (t as i64 + by) as u32 }).collect()
        };
        let mut on_line_differs = false;
        for k in &KIT {
            let Some(style) = corpus_style(k.file) else {
                continue;
            };
            // 3 ms of USB jitter and clock drift, in ticks.
            let d = (0.003 * style.bpm() / 60.0 * style.ppq as f64).ceil() as i64;
            let p = plan(&style, SCRIPT).unwrap();
            eprintln!("{}: chords {} ticks ahead, {} ticks of room ({:.1} ms)", k.file, p.chord_lead, p.margin, p.margin as f64 / style.ppq as f64 * 60_000.0 / style.bpm());
            assert!(p.margin as i64 > d, "{}: only {} ticks of room", k.file, p.margin);
            let take = perform(&style, SCRIPT).unwrap();
            for by in [-d, d] {
                let other = sim::perform_steps(&style, take.steps.clone(), take.bars, shifted(&take, by));
                if let Err(e) = same(&take, &other, d as u32) {
                    panic!("{}: steps {by} ticks late: {e}", k.file);
                }
            }
            let on_line = sim::perform(&style, SCRIPT).unwrap();
            let late = sim::perform_steps(&style, on_line.steps.clone(), on_line.bars, shifted(&on_line, d));
            on_line_differs |= same(&on_line, &late, d as u32).is_err();
        }
        assert!(on_line_differs || corpus_style(KIT[0].file).is_none(), "steps on the line should be sensitive to jitter");
    }

    /// Each factory kit style is a Genos preset of exactly that name (the Data List's style
    /// list), so an owner finds it on the Preset tab. The kit's old RockShuffle is not one.
    /// Skipped without the (git-ignored) manuals.
    #[test]
    fn factory_kit_styles_are_genos_presets() {
        let Ok(list) = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/manuals/Genos_data_list.txt")) else {
            eprintln!("factory_kit_styles_are_genos_presets: no docs/manuals/Genos_data_list.txt; skipping");
            return;
        };
        // The style list is in columns two or more spaces apart ("US RockShuffle" is one name).
        let names: std::collections::HashSet<&str> = list.lines().flat_map(|l| l.split("  ")).map(str::trim).filter(|w| !w.is_empty()).collect();
        assert!(names.contains("US RockShuffle") && !names.contains("RockShuffle"));
        for k in KIT.iter().filter(|k| k.source == FACTORY) {
            assert!(names.contains(k.title), "{} is not a Genos preset style", k.title);
        }
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
            let got = our_reference_digest(&style).unwrap();
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

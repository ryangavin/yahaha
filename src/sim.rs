//! Offline driver: runs the engine with simulated time and records what it sends.

use crate::engine::{slot_of, Button, Engine, Prepared, Sink};
use crate::sff::SectionId;
use crate::sff::Style;
use crate::theory::{Chord, NOTE_NAMES};
use anyhow::{anyhow, bail, Result};

#[derive(Default)]
pub struct Recorder {
    pub now: u64,
    pub out: Vec<(u64, Vec<u8>)>,
}

impl Sink for Recorder {
    fn send(&mut self, msg: &[u8]) {
        self.out.push((self.now, msg.to_vec()));
    }
}

#[derive(Clone, Copy)]
pub enum Step {
    Chord(Chord),
    /// Every chord-section key let go (Sync Stop listens for this).
    Release,
    Button(Button),
}

/// Run a script of (time_ns, step) against the engine until `end_ns`.
#[cfg_attr(not(test), allow(dead_code))]
pub fn run(style: Box<Prepared>, script: &[(u64, Step)], end_ns: u64) -> (Engine, Recorder) {
    run_observed(style, script, end_ns, |_, _| {})
}

/// `run`, calling `observe` after every engine wake-up (section changes land on one).
pub fn run_observed(
    style: Box<Prepared>,
    script: &[(u64, Step)],
    end_ns: u64,
    mut observe: impl FnMut(&Engine, u64),
) -> (Engine, Recorder) {
    let mut e = Engine::new(style);
    let mut rec = Recorder::default();
    let mut i = 0;
    let mut now = 0u64;
    loop {
        let next_script = script.get(i).map(|s| s.0);
        let next_engine = e.next_deadline();
        let t = [next_script, next_engine, Some(end_ns)].into_iter().flatten().min().unwrap();
        now = now.max(t);
        rec.now = now;
        // Script steps at this instant happen before the engine plays what is due.
        while let Some((ts, step)) = script.get(i) {
            if *ts > now {
                break;
            }
            match step {
                Step::Chord(c) => e.set_chord(*c, now, &mut rec),
                Step::Release => e.chord_released(now, &mut rec),
                Step::Button(b) => e.button(*b, now, &mut rec),
            }
            i += 1;
        }
        e.process(now, &mut rec);
        observe(&e, now);
        if now >= end_ns {
            break;
        }
    }
    (e, rec)
}

// ---------------------------------------------------------------------------
// Scripts and snapshots (`yahaha sim`, tests/golden)
// ---------------------------------------------------------------------------

/// One script step, at a tick counted from the start of bar 1, with the text shown for it.
pub struct ScriptStep {
    pub tick: u32,
    pub step: Step,
    pub label: String,
}

/// Parse a performance script (grammar in tests/golden/README.md). `|` separates bars; the
/// chords, `-` holds and `^` releases in a bar share it equally; `[Button]` tokens take no
/// time and press at the next slot. Without any `|`, every chord is its own bar ("C Am F G7").
/// Returns the steps and the number of bars.
pub fn parse_script(text: &str, tpb: u32) -> Result<(Vec<ScriptStep>, u32)> {
    // A token starting with '#' comments out the rest of the line ("F#m7" is a chord).
    let mut tokens: Vec<&str> = Vec::new();
    for (i, l) in text.lines().enumerate() {
        let line: Vec<&str> = l.split_whitespace().take_while(|t| !t.starts_with('#')).collect();
        // "| |" on one line would silently drop a bar and shift every later one. (A line
        // ending in '|' followed by one starting with '|' is the normal layout.)
        if line.windows(2).any(|w| w == ["|", "|"]) {
            bail!("line {}: empty bar \"| |\"; write \"| - |\" to hold the chord for a bar", i + 1);
        }
        tokens.extend(line);
    }
    if !tokens.contains(&"|") {
        tokens = tokens.into_iter().flat_map(|t| if t.starts_with('[') { vec![t] } else { vec![t, "|"] }).collect();
    }
    let mut steps = Vec::new();
    let mut bars = 0u32;
    let mut carried: Vec<&str> = Vec::new();
    for bar in tokens.split(|t| *t == "|") {
        let slots = bar.iter().filter(|t| !t.starts_with('[')).count() as u32;
        if slots == 0 {
            // Buttons between bars ("| [MainB] |") press at the start of the next bar; a bar
            // needs at least one chord or "-".
            carried.extend_from_slice(bar);
            continue;
        }
        let start = bars * tpb;
        let mut slot = 0;
        for &t in carried.drain(..).chain(bar.iter().copied()).collect::<Vec<_>>().iter() {
            // A button after the last slot presses at the start of the next bar.
            let tick = start + slot * tpb / slots;
            if let Some(name) = t.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
                let b = parse_button(name).ok_or_else(|| anyhow!("unknown button [{name}]"))?;
                steps.push(ScriptStep { tick, step: Step::Button(b), label: t.to_string() });
                continue;
            }
            if t == "^" {
                steps.push(ScriptStep { tick, step: Step::Release, label: t.to_string() });
            } else if t != "-" {
                let c = crate::parse_chord(t)?;
                steps.push(ScriptStep { tick, step: Step::Chord(c), label: t.to_string() });
            }
            slot += 1;
        }
        bars += 1;
    }
    if let Some(t) = carried.first() {
        bail!("{t} after the last bar never plays");
    }
    Ok((steps, bars))
}

fn parse_button(name: &str) -> Option<Button> {
    let letter = |s: &str| match s {
        "A" => Some(0),
        "B" => Some(1),
        "C" => Some(2),
        "D" => Some(3),
        _ => None,
    };
    Some(match name {
        "Break" => Button::Break,
        "AutoFill" => Button::AutoFill,
        "StartStop" => Button::StartStop,
        "Stop" => Button::Stop,
        "SyncStart" => Button::SyncStart,
        "SyncStop" => Button::SyncStop,
        "StopAcmp" => Button::StopAcmp,
        _ => {
            if let Some(l) = name.strip_prefix("Intro") {
                Button::Intro(letter(l)?)
            } else if let Some(l) = name.strip_prefix("Main") {
                Button::Main(letter(l)?)
            } else if let Some(l) = name.strip_prefix("Ending") {
                Button::Ending(letter(l)?)
            } else {
                return None;
            }
        }
    })
}

const PART_NAMES: [&str; 8] = ["Rhythm1", "Rhythm2", "Bass", "Chord1", "Chord2", "Pad", "Phrase1", "Phrase2"];

/// Play a script on a style and list, bar by bar, the section playing, the script steps and
/// every note each part starts, as `beat.tick Note~length` (Yamaha octaves, C3 = 60; `~…` =
/// still sounding when the script ends). Positions and lengths are in the style's ticks.
/// Parts that play as written whatever the chord (drums, Root Fixed + Bypass) only get a
/// per-bar note count: listing them would copy the style's own pattern and says nothing
/// about chord following. The listing depends only on the style and the script, so it can
/// be diffed.
pub fn snapshot(style: &Style, script: &str) -> Result<String> {
    let prep = Box::new(Prepared::new(style));
    // as_written[slot][part]: see PSection::plays_as_written.
    let as_written: Vec<[bool; 8]> = prep
        .sections
        .iter()
        .map(|s| s.as_ref().map_or([false; 8], |s| std::array::from_fn(|p| s.plays_as_written(8 + p as u8))))
        .collect();
    let (ppq, tpb) = (prep.ppq, prep.tpb);
    let ns_per_tick = 60e9 / (prep.bpm * ppq as f64);
    let ns = |tick: u32| (tick as f64 * ns_per_tick).ceil() as u64;
    let tick = |ns: u64| (ns as f64 / ns_per_tick).round() as u32;
    let (steps, bars) = parse_script(script, tpb)?;
    // Buttons press one tick ahead of their slot, as a player would: a press exactly on a
    // beat or bar line would otherwise depend on float rounding (this beat or the next?).
    // The listing shows them at that tick, which may be in the previous bar.
    let at = |s: &ScriptStep| match s.step {
        Step::Button(_) => s.tick.saturating_sub(1),
        _ => s.tick,
    };
    let mut timed: Vec<(u64, Step)> = steps.iter().map(|s| (ns(at(s)), s.step)).collect();
    timed.sort_by_key(|s| s.0);
    let (num, den) = style.timesig;
    let header = format!("style  {}  {:.0} bpm  {num}/{den}  ppq {ppq}\nbars   {bars}\n", style.name.trim_end_matches(|c: char| c.is_whitespace() || c == '\0'), prep.bpm);

    let mut sections: Vec<(u32, Option<SectionId>)> = Vec::new();
    let mut last = None;
    let (_, rec) = run_observed(prep, &timed, ns(bars * tpb), |e, now| {
        let cur = e.snapshot(now).cur;
        if cur != last {
            sections.push((tick(now), cur));
            last = cur;
        }
    });
    let section_at = |t: u32| sections.iter().rev().find(|(s, _)| *s <= t).and_then(|(_, id)| *id);
    let name = |id: Option<SectionId>| id.map_or("stopped".into(), |c| c.name());

    // (start, channel, key, length): each note-off closes the oldest open note of its key.
    let mut notes: Vec<(u32, u8, u8, Option<u32>)> = Vec::new();
    for (t, m) in &rec.out {
        let (kind, ch) = (m[0] & 0xF0, m[0] & 0x0F);
        if kind == 0x90 && m[2] > 0 {
            notes.push((tick(*t), ch, m[1], None));
        } else if (kind == 0x80 || kind == 0x90)
            && let Some(n) = notes.iter_mut().find(|n| n.1 == ch && n.2 == m[1] && n.3.is_none())
        {
            n.3 = Some(tick(*t) - n.0);
        }
    }

    let width = (ppq - 1).to_string().len();
    let pos = |t: u32| format!("{}.{:0width$}", (t % tpb) / ppq + 1, t % ppq);
    let mut out = header;
    for bar in 0..bars {
        let (lo, hi) = (bar * tpb, (bar + 1) * tpb);
        out += &format!("\nbar {}  {}", bar + 1, name(section_at(lo)));
        for (t, s) in sections.iter().filter(|(t, _)| *t > lo && *t < hi) {
            out += &format!(" > {}@{}", name(*s), pos(*t));
        }
        for s in steps.iter().filter(|s| at(s) >= lo && at(s) < hi) {
            out += &format!("  {}@{}", s.label, pos(at(s)));
        }
        out.push('\n');
        for ch in 8..16u8 {
            let (mut items, mut written) = (Vec::new(), 0);
            for &(t, _, key, len) in notes.iter().filter(|n| n.1 == ch && n.0 >= lo && n.0 < hi) {
                if section_at(t).is_some_and(|id| as_written[slot_of(id)][ch as usize - 8]) {
                    written += 1;
                    continue;
                }
                let len = len.map_or("…".to_string(), |l| l.to_string());
                items.push(format!("{} {}{}~{len}", pos(t), NOTE_NAMES[key as usize % 12], key as i32 / 12 - 2));
            }
            if written > 0 {
                items.push(format!("{written} as written"));
            }
            if !items.is_empty() {
                out += &format!("  ch{} {:<8}{}\n", ch + 1, PART_NAMES[ch as usize - 8], items.join("  "));
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sff::Style;

    fn corpus() -> Vec<std::path::PathBuf> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus");
        let mut v = Vec::new();
        let mut stack = vec![dir];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().map_or(false, |x| {
                    matches!(x.to_ascii_lowercase().to_str(), Some("sty" | "prs" | "sst" | "bcs" | "pcs" | "fps"))
                }) {
                    v.push(p);
                }
            }
        }
        v.sort();
        v
    }

    /// Faders scale the style's own part volume instead of replacing it.
    #[test]
    fn fader_scales_style_volume() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/FunkyFinger.S930.STY");
        if !path.exists() {
            return;
        }
        let prep = Box::new(Prepared::new(&Style::load(&path).unwrap()));
        let mut e = Engine::new(prep);
        let mut rec = Recorder::default();
        e.send_init(&mut rec);
        let style_bass = rec.out.iter().rev().find(|(_, m)| m[0] == 0xBA && m[1] == 7).map(|(_, m)| m[2]).unwrap_or(100);
        rec.out.clear();
        e.set_gain(2, 64, &mut rec); // part 3 = Bass (ch 11)
        assert_eq!(rec.out.last().unwrap().1, vec![0xBA, 7, (style_bass as u32 * 64 / 127) as u8]);
        rec.out.clear();
        e.send_init(&mut rec); // a later style volume message is still scaled
        let v = rec.out.iter().rev().find(|(_, m)| m[0] == 0xBA && m[1] == 7).unwrap().1[2];
        assert_eq!(v, (style_bass as u32 * 64 / 127) as u8);
    }

    /// Stop Accompaniment: chords sound on bass + pad while stopped, and clear on start.
    #[test]
    fn stop_accompaniment() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/FunkyFinger.S930.STY");
        if !path.exists() {
            return;
        }
        let mut e = Engine::new(Box::new(Prepared::new(&Style::load(&path).unwrap())));
        let mut rec = Recorder::default();
        e.button(Button::SyncStart, 0, &mut rec); // disarm sync start
        e.button(Button::StopAcmp, 0, &mut rec);
        e.set_chord(Chord::new(9, 8), 1, &mut rec); // Am
        let ons: Vec<(u8, u8)> = rec.out.iter().filter(|(_, m)| m[0] & 0xF0 == 0x90).map(|(_, m)| (m[0] & 0xF, m[1] % 12)).collect();
        assert!(ons.contains(&(10, 9)), "bass A: {ons:?}");
        for pc in [9, 0, 4] {
            assert!(ons.contains(&(13, pc)), "pad {pc}: {ons:?}");
        }
        assert!(!e.is_running());
        rec.out.clear();
        e.set_chord(Chord::new(5, 0), 2, &mut rec); // F: old notes off, new on
        assert!(rec.out.iter().any(|(_, m)| m[0] == 0x8A && m[1] % 12 == 9));
        assert!(rec.out.iter().any(|(_, m)| m[0] == 0x9A && m[1] % 12 == 5));
        rec.out.clear();
        e.button(Button::StartStop, 3, &mut rec);
        assert!(rec.out.iter().any(|(_, m)| m[0] == 0x8A && m[1] % 12 == 5), "released on start");
    }

    /// Every style under every chord type, including 1+8, Cancel and the display-only
    /// Data List types, which must play exactly like the CASM type they map to.
    #[test]
    fn corpus_every_chord_type() {
        use crate::theory::{casm_type, CANCEL, TYPE_NAMES};
        for f in corpus() {
            let style = Style::load(&f).unwrap();
            let prep = Prepared::new(&style);
            let bar = (60e9 / prep.bpm * (prep.tpb as f64 / prep.ppq as f64)) as u64;
            let play = |ty: u8| {
                let script = [(0, Step::Chord(Chord { root: 2, ty, bass: Some(9) }))];
                run(Box::new(Prepared::new(&style)), &script, bar * 2).1.out
            };
            for ty in 0..TYPE_NAMES.len() as u8 {
                let out = play(ty);
                // Cancel does not sync-start the style.
                assert!(ty == CANCEL || out.iter().any(|(_, m)| m[0] & 0xF0 == 0x90), "{}: silent under {ty}", f.display());
                if casm_type(ty) != ty {
                    assert_eq!(out, play(casm_type(ty)), "{}: type {}", f.display(), TYPE_NAMES[ty as usize]);
                }
            }
        }
    }

    /// Every style: play through intro, mains, fills, break, chord changes and ending.
    /// Afterwards the engine must be stopped with no sounding notes, and every note-on
    /// must have a matching note-off.
    #[test]
    fn corpus_full_performance_no_stuck_notes() {
        let files = corpus();
        if files.is_empty() {
            eprintln!("no corpus; skipping");
            return;
        }
        let chords = [Chord::new(0, 0), Chord::new(9, 10), Chord::new(5, 2), Chord::new(7, 19),
                      Chord::new(2, 8), Chord { root: 0, ty: 0, bass: Some(4) }, Chord::new(10, 22), Chord::new(6, 11)];
        for f in files {
            let style = Style::load(&f).unwrap();
            let prep = Box::new(Prepared::new(&style));
            let bar = (60e9 / prep.bpm * (prep.tpb as f64 / prep.ppq as f64)) as u64;
            let mut script = vec![(0, Step::Button(Button::Intro(0))), (1_000, Step::Chord(chords[0]))];
            let mut t = bar / 3;
            for (i, b) in [Button::Main(1), Button::Main(1), Button::Main(2), Button::Break, Button::Main(3),
                           Button::Main(0), Button::AutoFill, Button::Main(2), Button::Intro(1)].iter().enumerate() {
                script.push((t, Step::Button(*b)));
                script.push((t + bar / 7, Step::Chord(chords[(i + 1) % chords.len()])));
                t += bar + bar / 5;
            }
            script.push((t, Step::Button(Button::Ending(0))));
            let (e, rec) = run(prep, &script, t + bar * 12);
            let name = f.file_name().unwrap().to_string_lossy().to_string();
            assert!(!e.is_running(), "{name}: still running after ending");
            let mut on = std::collections::HashMap::<(u8, u8), i32>::new();
            let mut notes = 0;
            for (_, m) in &rec.out {
                match m[0] & 0xF0 {
                    0x90 => { *on.entry((m[0] & 0xF, m[1])).or_default() += 1; notes += 1; }
                    0x80 => { *on.entry((m[0] & 0xF, m[1])).or_default() -= 1; }
                    _ => {}
                }
            }
            assert!(notes > 20, "{name}: only {notes} notes");
            for ((ch, key), n) in on {
                assert_eq!(n, 0, "{name}: ch{} key {key} unbalanced by {n}", ch + 1);
            }
            // No part may be left bent after it stops.
            for ch in 8..16u8 {
                if let Some((_, m)) = rec.out.iter().rev().find(|(_, m)| m[0] == 0xE0 | ch) {
                    assert_eq!((m[1], m[2]), (0, 0x40), "{name}: ch{} left bent", ch + 1);
                }
            }
        }
    }
}

#[cfg(test)]
mod style_switch {
    use super::*;
    use crate::sff::Style;

    /// Switching style mid-bend must re-centre the bend (TenorToTheMAX bends Chord 2 constantly).
    #[test]
    fn style_change_recentres_bends() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2");
        let (a, b) = (dir.join("TenorToTheMAX.S930.STY"), dir.join("FunkyFinger.S930.STY"));
        if !a.exists() || !b.exists() {
            return;
        }
        let pa = Box::new(Prepared::new(&Style::load(&a).unwrap()));
        let pb = Box::new(Prepared::new(&Style::load(&b).unwrap()));
        let bar = (60e9 / pa.bpm * (pa.tpb as f64 / pa.ppq as f64)) as u64;
        // Stop right after the first bend away from centre.
        let (_, full) = run(pa, &[(0, Step::Chord(Chord::new(0, 0)))], bar * 8);
        let (t_bend, status) = full
            .out
            .iter()
            .find(|(_, m)| m[0] & 0xF0 == 0xE0 && (m[1], m[2]) != (0, 0x40))
            .map(|(t, m)| (*t, m[0]))
            .expect("expected the style to bend a part");
        let pa = Box::new(Prepared::new(&Style::load(&a).unwrap()));
        let (mut e, mut rec) = run(pa, &[(0, Step::Chord(Chord::new(0, 0)))], t_bend);
        rec.out.clear();
        rec.now = t_bend + 1;
        let _old = e.load(pb, t_bend + 1, &mut rec);
        let last = rec.out.iter().rev().find(|(_, m)| m[0] == status).map(|(_, m)| (m[1], m[2]));
        assert_eq!(last, Some((0, 0x40)));
    }
}

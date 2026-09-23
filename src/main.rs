mod bench;
mod engine;
mod launchkey;
mod live;
mod midi;
mod rt;
mod sff;
mod sim;
mod synth;
mod theory;
mod ui;

use anyhow::Result;
use std::path::PathBuf;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("dump") => {
            for p in &args[2..] {
                dump(&PathBuf::from(p))?;
            }
        }
        Some("sim") => sim_cmd(&args[2..])?,
        Some("play") | None if args.len() > 2 || args.get(1).map_or(false, |a| a == "play") => play_cmd(&args[2..])?,
        Some("drive") => bench::drive()?,
        Some("screen") => ui::screen_html(std::path::Path::new(&args[2]), std::path::Path::new(&args[3]))?,
        Some("bench") => bench::run(std::path::Path::new(&args[2]), args.get(3).and_then(|s| s.parse().ok()))?,
        _ => eprintln!(
            "usage:\n  yahaha play <style or folder>... [--split F#2] [--input <name>] [--all-inputs] [--no-pads] [--sf2 file | --no-synth] [--palette-leds]\n  yahaha bench <style> [spin_us]\n  yahaha sim <style> \"C Am F G7\"\n  yahaha dump <style>..."
        ),
    }
    Ok(())
}

fn dump(path: &std::path::Path) -> Result<()> {
    let s = sff::Style::load(path)?;
    println!("{}  [{}]  {:.0} bpm  {}/{}  ppq {}", s.name, s.format, s.bpm(), s.timesig.0, s.timesig.1, s.ppq);
    for (id, sec) in &s.sections {
        let notes = sec.events.iter().filter(|e| matches!(e.ev, sff::Ev::NoteOn { .. })).count();
        let chans: std::collections::BTreeSet<u8> = sec.events.iter().filter_map(|e| e.ev.channel()).collect();
        println!("  {:<11} {:>2} bars  {:>4} notes  ch {:?}", id.name(), sec.len / s.ticks_per_bar(), notes,
            chans.iter().map(|c| c + 1).collect::<Vec<_>>());
    }
    for seg in &s.casm {
        println!("  CSEG {:?}", seg.sections);
        for r in &seg.rules {
            let z = &r.zones[1];
            println!("    src {:>2} -> {:>2} {:<8} root {} type {:>2} mid {}..{} {:?}/{:?} hk {} lim {}..{} {:?} bass {} nm {:03x} cm {:09x}",
                r.src_ch + 1, r.dest_ch + 1, r.name, r.src_root, r.src_type, r.mid_lo, r.mid_hi,
                z.ntr, z.ntt, z.high_key, z.lo, z.hi, z.rtr, r.bass_on, r.note_mute, r.chord_mute);
        }
    }
    for (id, d) in &s.other_chunks {
        println!("  chunk {id} ({} bytes)", d.len());
    }
    Ok(())
}

/// `yahaha sim style.sty "C Am F G7"`: one chord per bar on Main A, printed per part.
fn sim_cmd(args: &[String]) -> Result<()> {
    use theory::{Chord, Recognizer};
    let style = sff::Style::load(std::path::Path::new(&args[0]))?;
    let prep = Box::new(engine::Prepared::new(&style));
    let rec = Recognizer::new();
    let chords: Vec<Chord> = args[1].split_whitespace().map(|c| parse_chord(&rec, c)).collect::<Result<_>>()?;
    let bar_ns = (60e9 / prep.bpm * (prep.tpb as f64 / prep.ppq as f64)) as u64;
    let script: Vec<(u64, sim::Step)> =
        chords.iter().enumerate().map(|(i, c)| (i as u64 * bar_ns, sim::Step::Chord(*c))).collect();
    let (_, r) = sim::run(prep, &script, chords.len() as u64 * bar_ns);
    for (bar, c) in chords.iter().enumerate() {
        println!("bar {} {}", bar + 1, c.name());
        for ch in 8..16u8 {
            let notes: Vec<String> = r
                .out
                .iter()
                .filter(|(t, m)| *t >= bar as u64 * bar_ns && *t < (bar as u64 + 1) * bar_ns && m[0] == 0x90 | ch)
                .map(|(_, m)| format!("{}{}", theory::NOTE_NAMES[m[1] as usize % 12], m[1] as i32 / 12 - 2))
                .collect();
            if !notes.is_empty() {
                println!("  ch{:>2}: {}", ch + 1, notes.join(" "));
            }
        }
    }
    Ok(())
}

/// Parse a chord symbol by building its notes and running the recognizer.
fn parse_chord(rec: &theory::Recognizer, s: &str) -> Result<theory::Chord> {
    let (body, bass) = match s.split_once('/') {
        Some((b, bass)) => (b, Some(bass)),
        None => (s, None),
    };
    let root_len = if body.len() > 1 && (body.as_bytes()[1] == b'#' || body.as_bytes()[1] == b'b') { 2 } else { 1 };
    let pc = |n: &str| theory::NOTE_NAMES.iter().position(|x| *x == n).or_else(|| {
        ["C", "Db", "D", "D#", "E", "F", "Gb", "G", "G#", "A", "A#", "B"].iter().position(|x| *x == n)
    });
    let root = pc(&body[..root_len]).ok_or_else(|| anyhow::anyhow!("bad chord {s}"))? as u8;
    let suffix = &body[root_len..];
    let ty = theory::TYPE_NAMES.iter().position(|t| *t == suffix).ok_or_else(|| anyhow::anyhow!("bad chord type {suffix}"))? as u8;
    let bass = bass.map(|b| pc(b).map(|p| p as u8).ok_or_else(|| anyhow::anyhow!("bad bass {b}"))).transpose()?;
    let _ = rec;
    Ok(theory::Chord { root, ty, bass: bass.filter(|&b| b != root) })
}

fn play_cmd(args: &[String]) -> Result<()> {
    let mut paths = Vec::new();
    let mut split = 54; // F#2 in Yamaha octave numbering (C3 = 60), the Genos default
    let mut all_inputs = false;
    let mut no_pads = false;
    let mut no_synth = false;
    let mut palette_leds = false;
    let mut sf2: Option<PathBuf> = None;
    let mut inputs = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--split" => {
                i += 1;
                split = parse_note(args.get(i).map(|s| s.as_str()).unwrap_or(""))
                    .ok_or_else(|| anyhow::anyhow!("--split wants a note like F#2 or a MIDI number"))?;
            }
            "--all-inputs" => all_inputs = true,
            "--no-pads" => no_pads = true,
            "--no-synth" => no_synth = true,
            "--palette-leds" => palette_leds = true,
            "--sf2" => {
                i += 1;
                sf2 = args.get(i).map(PathBuf::from);
            }
            "--input" => {
                i += 1;
                inputs.push(args.get(i).cloned().unwrap_or_default());
            }
            p => paths.push(PathBuf::from(p)),
        }
        i += 1;
    }
    if paths.is_empty() {
        paths.push(PathBuf::from("corpus"));
    }
    // Default SoundFont: the first .sf2 in ./soundfonts.
    if sf2.is_none() && !no_synth {
        sf2 = std::fs::read_dir("soundfonts").into_iter().flatten().flatten().map(|e| e.path())
            .find(|p| p.extension().map_or(false, |x| x.eq_ignore_ascii_case("sf2")));
    }
    if no_synth {
        sf2 = None;
    }
    ui::play(ui::Options { paths, split, all_inputs, inputs, no_pads, sf2, palette_leds })
}

/// "F#2" (Yamaha numbering, C3 = 60) or a raw MIDI number.
fn parse_note(s: &str) -> Option<u8> {
    if let Ok(n) = s.parse::<u8>() {
        return Some(n);
    }
    let (name, oct) = s.split_at(s.find(|c: char| c.is_ascii_digit() || c == '-')?);
    let pc = theory::NOTE_NAMES.iter().position(|n| n.eq_ignore_ascii_case(name))
        .or_else(|| ["C", "Db", "D", "D#", "E", "F", "Gb", "G", "G#", "A", "A#", "B"].iter().position(|n| n.eq_ignore_ascii_case(name)))?;
    let oct: i32 = oct.parse().ok()?;
    u8::try_from((oct + 2) * 12 + pc as i32).ok()
}

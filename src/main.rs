//! The `yahaha` command line: the terminal front panel (`play`) and the developer tools.
//! Everything but the terminal UI lives in the `yahaha` library.

mod ui;

use yahaha::{bench, capture, engine, fingering, oracle, sff, sim};
use anyhow::{Context, Result};
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
        Some("capture-kit") => capture_kit_cmd(&args[2..])?,
        Some("capture-import") => capture::import_cmd(&args[2..])?,
        Some("oracle") => oracle_cmd(&args[2..])?,
        Some("play") | None if args.len() > 2 || args.get(1).map_or(false, |a| a == "play") => play_cmd(&args[2..])?,
        Some("drive") => bench::drive()?,
        Some("screen") => ui::screen_html(std::path::Path::new(&args[2]), std::path::Path::new(&args[3]))?,
        Some("bench") => bench::run(std::path::Path::new(&args[2]), args.get(3).and_then(|s| s.parse().ok()))?,
        Some("state-json") => state_json(&args[2..])?,
        Some("pad") => {
            for p in &args[2..] {
                pad_dump(&PathBuf::from(p))?;
            }
        }
        Some("ireal") => ireal_cmd(&args[2..])?,
        #[cfg(feature = "plugins")]
        Some("plugin-test") => yahaha::plugin::cli::run(&args[2..])?,
        _ => eprintln!(
            "usage:\n  yahaha play <style or folder>... [--split F#2] [--input <name>] [--all-inputs] [--no-pads] [--sf2 file | --no-synth] [--palette-leds] [--audio-out 11]\n      [--fingering single|multi|fingered|on-bass|ai|full|ai-full] [--upper [--no-manual-bass]] [--transpose N] [--master-transpose N]\n  yahaha bench <style> [spin_us]\n  yahaha sim <style> <\"C Am F G7\" | script file>\n  yahaha capture-kit <out-dir> [--clock-ppm N] [style]...\n  yahaha capture-import <recording.mid> <style> [--tolerance-ms N] [--offset-ms N] [--clock-ppm N] [--listing FILE] [--golden DIR [--force]]\n  yahaha oracle <style or folder>... [--pairs | --scores | --diff scores.txt]\n  yahaha dump <style>...\n  yahaha pad <multi pad bank.pad>...\n  yahaha state-json <style or folder> [\"C Am\"] [--library]\n  yahaha plugin-test [name] [--list | --rescan] [--bench] [--swap-to name] [--oop] [--gui] [--channel N] [--sf2 file | --no-sf2]   (needs --features plugins)\n  yahaha ireal <file or irealb:// link> [--choruses N]"
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
    let sint = s.sint();
    for (ch, c) in sint.channels.iter().enumerate() {
        if *c != sff::ChannelInit::default() {
            let pending = match (c.pending_msb, c.pending_lsb) {
                (None, None) => String::new(),
                (m, l) => format!(" (then bank {m:?}/{l:?})"),
            };
            println!("  SInt ch {:>2} bank {:?}/{:?} prog {:?}{pending} vol {:?} pan {:?} rev {:?} cho {:?}  +{} ctl  +{} xg",
                ch + 1, c.bank_msb, c.bank_lsb, c.program, c.volume, c.pan, c.reverb, c.chorus, c.other.len(), c.xg_part.len());
        }
    }
    if !sint.sysex.is_empty() {
        println!("  SInt sysex {}", sint.sysex.len());
    }
    for seg in &s.casm {
        println!("  CSEG {:?}", seg.sections);
        for r in &seg.rules {
            let z = &r.zones[1];
            // Bass On per zone, low/mid/high: "1--" is the low zone only.
            let b = |i: usize| if r.zones[i].bass_on { '1' } else { '-' };
            println!("    src {:>2} -> {:>2} {:<8} root {} type {:>2} mid {}..{} {:?}/{:?} hk {} lim {}..{} {:?} bass {}{}{} nm {:03x} cm {:09x}",
                r.src_ch + 1, r.dest_ch + 1, r.name, r.src_root, r.src_type, r.mid_lo, r.mid_hi,
                z.ntr, z.ntt, z.high_key, z.lo, z.hi, z.rtr, b(0), b(1), b(2), r.note_mute, r.chord_mute);
        }
    }
    for (id, d) in &s.other_chunks {
        println!("  chunk {id} ({} bytes)", d.len());
    }
    Ok(())
}

/// `yahaha pad <bank.pad>`: what the Multi Pad parser makes of a bank (src/multipad/file.rs;
/// the .pad layout is provisional, so unknown chunks are shown raw).
fn pad_dump(path: &std::path::Path) -> Result<()> {
    use yahaha::multipad::PadBank;
    let b = PadBank::load(path)?;
    println!("{}  {:?}  {:.0} bpm  {}/{}  ppq {}", if b.name.is_empty() { "(no name)" } else { &b.name },
        b.layout, b.bpm(), b.timesig.0, b.timesig.1, b.ppq);
    let flag = |f: Option<bool>| match f { Some(true) => "on", Some(false) => "off", None => "?" };
    for (i, pad) in b.pads.iter().enumerate() {
        let Some(p) = pad else {
            println!("  pad {}  (empty)", i + 1);
            continue;
        };
        println!("  pad {}  {:<10} ch {:>2}  {:>4} notes  {:>6} ticks ({:.2} beats)  repeat {}  chord match {}",
            i + 1, p.name, p.channel + 1, p.notes(), p.len, p.len as f64 / b.ppq as f64, flag(p.repeat), flag(p.chord_match));
        if let Some(r) = &p.rule {
            let z = &r.zones[1];
            println!("         rule: src {} type {} {:?}/{:?} hk {} lim {}..{} {:?}",
                yahaha::theory::NOTE_NAMES[r.src_root as usize % 12], r.src_type, z.ntr, z.ntt, z.high_key, z.lo, z.hi, z.rtr);
        }
    }
    for t in &b.texts {
        println!("  text {t:?}");
    }
    for (id, d) in &b.other_chunks {
        let head: Vec<String> = d.iter().take(16).map(|x| format!("{x:02x}")).collect();
        println!("  chunk {id} ({} bytes) {}", d.len(), head.join(" "));
    }
    Ok(())
}

/// `yahaha state-json <style or folder> ["C Am F"] [--library]`: an offline session's
/// `AppState` as JSON (mock data for the app; docs/app-api.md), after playing the chords
/// one bar each (in the left hand, Sync Start). `--library` prints the library instead.
fn state_json(args: &[String]) -> Result<()> {
    use yahaha::session::{Options, Port, Session};
    let usage = "usage: yahaha state-json <style or folder> [\"C Am F\"] [--library]";
    let library = args.iter().any(|a| a == "--library");
    let mut rest = args.iter().filter(|a| *a != "--library");
    let path = rest.next().ok_or_else(|| anyhow::anyhow!(usage))?;
    let s = Session::offline(Options { paths: vec![PathBuf::from(path)], ..Options::default() })?;
    s.finish_indexing();
    if library {
        println!("{}", serde_json::to_string_pretty(&s.library_list())?);
        return Ok(());
    }
    let chords = rest.next().map(|c| c.split_whitespace().map(yahaha::parse_chord).collect::<Result<Vec<_>>>()).transpose()?;
    let st = s.state();
    let bar = (60e9 / st.style.tempo * st.transport.beats_per_bar as f64) as u64;
    let mut held: Vec<u8> = Vec::new();
    for c in chords.unwrap_or_default() {
        for k in held.drain(..) {
            s.midi_in(Port::Keys, &[0x80, k, 0]);
        }
        // Root position in the left hand, below the default split (F#2).
        held = yahaha::theory::chord_tones(c.ty).iter().map(|t| 24 + c.root + t).collect();
        for &k in &held {
            s.midi_in(Port::Keys, &[0x90, k, 100]);
        }
        s.advance(bar);
    }
    println!("{}", serde_json::to_string_pretty(&s.state_now())?);
    Ok(())
}

/// `yahaha ireal <file or link> [--choruses N]`: every song in an iReal Pro link (or an
/// HTML / text file holding links) with its form expanded into bars (src/ireal).
fn ireal_cmd(args: &[String]) -> Result<()> {
    use yahaha::ireal;
    let usage = "usage: yahaha ireal <file or irealb:// link> [--choruses N]";
    let (mut src, mut choruses) = (None, None);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--choruses" {
            choruses = Some(it.next().and_then(|n| n.parse::<u32>().ok()).with_context(|| format!("--choruses wants a number\n{usage}"))?);
        } else {
            src = Some(a);
        }
    }
    let src = src.ok_or_else(|| anyhow::anyhow!(usage))?;
    let text = if src.starts_with("irealb") { src.clone() } else { std::fs::read_to_string(src).with_context(|| format!("reading {src}"))? };
    for list in ireal::parse(&text)? {
        if let Some(name) = &list.name {
            println!("playlist: {name}");
        }
        for song in &list.songs {
            let n = choruses.unwrap_or(song.repeats);
            println!("\n{} ({}): {}, key {}, {} bpm, {} chorus(es)", song.title, song.composer, song.style, song.key, song.tempo, n);
            for (i, bar) in song.bars(n).iter().enumerate() {
                println!("{:>4}  {bar}", i + 1);
            }
        }
    }
    Ok(())
}

/// `yahaha sim style.sty "C Am F G7"` (one chord per bar) or `yahaha sim style.sty script.txt`:
/// plays a script (grammar in tests/golden/README.md) and lists what each part plays.
fn sim_cmd(args: &[String]) -> Result<()> {
    let (Some(style), Some(script)) = (args.first(), args.get(1)) else {
        anyhow::bail!("usage: yahaha sim <style> <\"C Am F G7\" | script file>");
    };
    let style = sff::Style::load(std::path::Path::new(style))?;
    let path = std::path::Path::new(script);
    let script = if path.is_file() { std::fs::read_to_string(path)? } else { script.clone() };
    print!("{}", sim::snapshot(&style, &script)?);
    Ok(())
}

/// `yahaha capture-kit <out-dir> [--clock-ppm N] [style]...`: the Genos-owner capture kit
/// (src/capture.rs).
fn capture_kit_cmd(args: &[String]) -> Result<()> {
    let usage = "usage: yahaha capture-kit <out-dir> [--clock-ppm N] [style]...";
    let Some(out) = args.first() else {
        anyhow::bail!("{usage}");
    };
    let (mut styles, mut clock_ppm) = (Vec::new(), 0.0);
    let mut it = args[1..].iter();
    while let Some(a) = it.next() {
        if a == "--clock-ppm" {
            clock_ppm = it.next().with_context(|| format!("--clock-ppm wants a value\n{usage}"))?.parse()?;
        } else {
            styles.push(PathBuf::from(a));
        }
    }
    capture::write_kit(std::path::Path::new(out), &styles, clock_ppm)
}

/// `yahaha oracle corpus/ [--pairs | --scores | --diff scores.txt]`: score our chord
/// conversion against the authors' own chord-muted alternatives (see docs/oracle.md).
/// Prints counts only, never notes. `--scores` prints the pinned form
/// (tests/oracle/scores.txt), `--diff` what changed against such a file, over the styles
/// that file lists (only their own lines when the run covers some of them). The three flags are exclusive.
fn oracle_cmd(args: &[String]) -> Result<()> {
    let usage = "usage: yahaha oracle <style or folder>... [--pairs | --scores | --diff scores.txt]";
    let mut paths = Vec::new();
    let mut against = None;
    let (mut pairs, mut scores) = (false, false);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--diff" => against = Some(it.next().ok_or_else(|| anyhow::anyhow!(usage))?),
            "--pairs" => pairs = true,
            "--scores" => scores = true,
            _ => paths.push(PathBuf::from(a)),
        }
    }
    if paths.is_empty() || pairs as u8 + scores as u8 + against.is_some() as u8 > 1 {
        anyhow::bail!(usage);
    }
    let mut rep = oracle::run(&paths);
    if let Some(f) = against {
        let want = std::fs::read_to_string(f)?;
        let d = oracle::diff_against(&mut rep, &want);
        print!("{d}");
        if !d.lines().any(|l| l.starts_with("  ")) {
            println!("no change");
        }
    } else if scores {
        print!("{}", rep.pinned());
    } else {
        print!("{}", rep.text(pairs));
    }
    Ok(())
}

fn play_cmd(args: &[String]) -> Result<()> {
    let mut paths = Vec::new();
    let mut split = 54; // F#2 in Yamaha octave numbering (C3 = 60), the Genos default
    let mut all_inputs = false;
    let mut no_pads = false;
    let mut no_synth = false;
    let mut palette_leds = false;
    let mut audio_out: Option<u8> = None;
    let mut upper = false;
    let mut manual_bass = true;
    let mut sf2: Option<PathBuf> = None;
    let mut fingering = fingering::Fingering::FingeredOnBass;
    let mut transpose = engine::Transpose::default();
    let mut inputs = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--split" => {
                i += 1;
                split = yahaha::parse_note(args.get(i).map(|s| s.as_str()).unwrap_or(""))
                    .ok_or_else(|| anyhow::anyhow!("--split wants a note like F#2 or a MIDI number"))?;
            }
            "--all-inputs" => all_inputs = true,
            "--no-pads" => no_pads = true,
            "--no-synth" => no_synth = true,
            "--palette-leds" => palette_leds = true,
            "--upper" => upper = true,
            "--no-manual-bass" => manual_bass = false,
            "--audio-out" => {
                i += 1;
                audio_out = args.get(i).and_then(|s| s.split('/').next()?.parse().ok());
            }
            "--sf2" => {
                i += 1;
                sf2 = args.get(i).map(PathBuf::from);
            }
            "--fingering" => {
                i += 1;
                fingering = args.get(i).and_then(|s| fingering::Fingering::parse(s)).ok_or_else(|| {
                    anyhow::anyhow!("--fingering wants single, multi, fingered, on-bass, ai, full or ai-full")
                })?;
            }
            "--transpose" | "--master-transpose" => {
                let flag = args[i].clone();
                i += 1;
                let n = args.get(i).and_then(|s| s.trim_start_matches('+').parse::<i8>().ok())
                    .filter(|n| (-engine::Transpose::RANGE..=engine::Transpose::RANGE).contains(n))
                    .ok_or_else(|| anyhow::anyhow!("{flag} wants semitones from -12 to 12"))?;
                if flag == "--transpose" {
                    transpose.keyboard = n;
                } else {
                    transpose.master = n;
                }
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
    ui::play(yahaha::Options { paths, split, all_inputs, inputs, no_pads, sf2, palette_leds, audio_out, fingering, upper, manual_bass, transpose })
}


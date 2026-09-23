//! `yahaha play`: device setup, terminal front panel, Launchkey LEDs. Runs on the main
//! thread at normal priority; talks to the engine only through lock-free rings.

use crate::engine::{id_of, slot_of, Button, Engine, Prepared, Snapshot, NUM_SLOTS};
use crate::launchkey::{self, Led};
use crate::live::{self, Cmd, Input, Shared, TAG_KEYS, TAG_PADS};
use crate::midi::{self, Client};
use crate::rt::{PacketSink, Target};
use crate::sff::{SectionId, Style};
use crate::theory::{Recognizer, NOTE_NAMES};
use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style as St};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;
use std::time::Duration;

pub struct Options {
    pub paths: Vec<PathBuf>,
    pub split: u8,
    pub all_inputs: bool,
    /// Only connect sources whose name contains one of these.
    pub inputs: Vec<String>,
    pub no_pads: bool,
}

fn collect_styles(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let is_style = |p: &Path| {
        p.extension().map_or(false, |x| {
            matches!(x.to_ascii_lowercase().to_str(), Some("sty" | "prs" | "sst" | "bcs" | "pcs" | "pst" | "fps"))
        })
    };
    for p in paths {
        if p.is_dir() {
            let mut stack = vec![p.clone()];
            while let Some(d) = stack.pop() {
                for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
                    let q = e.path();
                    if q.is_dir() {
                        stack.push(q);
                    } else if is_style(&q) {
                        out.push(q);
                    }
                }
            }
        } else {
            out.push(p.clone());
        }
    }
    out.sort_by_key(|p| p.file_name().map(|n| n.to_ascii_lowercase()));
    out
}

pub fn note_name(n: u8) -> String {
    // Yamaha octave numbering (C3 = MIDI 60), as on the Genos.
    format!("{}{}", NOTE_NAMES[n as usize % 12], n as i32 / 12 - 2)
}

struct Loaded {
    name: String,
    format: String,
    has: [bool; NUM_SLOTS],
    voices: [Option<(u8, u8, u8)>; 16],
}

fn load(path: &Path) -> Result<(Box<Prepared>, Loaded)> {
    let style = Style::load(path)?;
    let prep = Box::new(Prepared::new(&style));
    let mut has = [false; NUM_SLOTS];
    for (i, s) in prep.sections.iter().enumerate() {
        has[i] = s.is_some();
    }
    let name = if style.name.is_empty() {
        path.file_stem().unwrap_or_default().to_string_lossy().to_string()
    } else {
        style.name.clone()
    };
    let info = Loaded { name, format: style.format.clone(), has, voices: prep.voices };
    Ok((prep, info))
}

pub fn play(opts: Options) -> Result<()> {
    let styles = collect_styles(&opts.paths);
    anyhow::ensure!(!styles.is_empty(), "no style files found");
    let mut idx = 0;
    let (prep, mut info) = load(&styles[0]).with_context(|| format!("loading {}", styles[0].display()))?;

    // --- MIDI setup ---
    let client = Client::new("yahaha")?;
    let out_src = client.virtual_source("yahaha")?;
    let shared = Arc::new(Shared::new(opts.split));
    let mut ch = live::channels(PacketSink::new(Target::Virtual(out_src)));
    let input = Input::new(shared.clone(), Recognizer::new(), ch.input_tx, PacketSink::new(Target::Virtual(out_src)));
    let port = client.input_port("yahaha in", input)?;

    let sources = midi::sources();
    let lk_keys: Vec<_> = sources.iter().filter(|(_, n)| n.contains("Launchkey") && !n.contains("DAW")).collect();
    let lk_daw = sources.iter().find(|(_, n)| n.contains("Launchkey") && n.contains("DAW"));
    let mut connected = Vec::new();
    if !opts.inputs.is_empty() {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while connected.is_empty() {
            for (e, n) in midi::sources() {
                if opts.inputs.iter().any(|want| n.contains(want.as_str())) {
                    port.connect(e, TAG_KEYS)?;
                    connected.push(n.clone());
                }
            }
            anyhow::ensure!(!connected.is_empty() || std::time::Instant::now() < deadline, "no MIDI source matching {:?}", opts.inputs);
            if connected.is_empty() {
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    } else if lk_keys.is_empty() || opts.all_inputs {
        for (e, n) in &sources {
            if n.starts_with("yahaha") || n.contains("DAW") {
                continue;
            }
            port.connect(*e, TAG_KEYS)?;
            connected.push(n.clone());
        }
    } else {
        for (e, n) in &lk_keys {
            port.connect(*e, TAG_KEYS)?;
            connected.push(n.clone());
        }
    }
    let mut leds: Option<PacketSink> = None;
    if let Some((e, n)) = lk_daw.filter(|_| !opts.no_pads) {
        port.connect(*e, TAG_PADS)?;
        connected.push(format!("{n} (pads)"));
        if let Some((d, _)) = midi::destinations().into_iter().find(|(_, n)| n.contains("Launchkey") && n.contains("DAW")) {
            let out_port = client.output_port("yahaha leds")?;
            let mut s = PacketSink::new(Target::Port(out_port, d));
            s.push(&launchkey::ENTER_DAW);
            s.flush();
            leds = Some(s);
        }
    }

    // --- engine thread ---
    let engine = Engine::new(prep);
    let sh = shared.clone();
    let io = ch.io;
    let engine_thread =
        std::thread::Builder::new().name("yahaha-engine".into()).spawn(move || live::run_engine(engine, io, sh))?;

    // --- terminal ---
    let mut term = ratatui::init();
    let mut snap: Option<Snapshot> = None;
    let mut last_leds: [(u8, Option<Led>); 16] = [(0, None); 16];
    let mut led_buf = Vec::new();
    let mut message = String::new();
    let result: Result<()> = (|| loop {
        while let Ok(s) = ch.snap_rx.pop() {
            snap = Some(s);
        }
        while ch.old_rx.pop().is_ok() {} // drop old styles here, off the RT thread

        if let (Some(s), Some(out)) = (&snap, leds.as_mut()) {
            for (i, (note, led)) in launchkey::pad_leds(s, &info.has).into_iter().enumerate() {
                if last_leds[i] != (note, Some(led)) {
                    led_buf.clear();
                    launchkey::led_msgs(note, led, &mut led_buf);
                    for m in &led_buf {
                        out.push(m);
                    }
                    last_leds[i] = (note, Some(led));
                }
            }
            out.flush();
        }

        term.draw(|f| draw(f, &info, snap.as_ref(), &shared, &connected, idx, styles.len(), &message))?;

        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(k) = event::read()? {
                if k.kind != KeyEventKind::Press {
                    continue;
                }
                let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
                let b = match k.code {
                    KeyCode::Esc => return Ok(()),
                    KeyCode::Char('c') if ctrl => return Ok(()),
                    KeyCode::Char(' ') => Some(Button::StartStop),
                    KeyCode::Char('1') => Some(Button::Main(0)),
                    KeyCode::Char('2') => Some(Button::Main(1)),
                    KeyCode::Char('3') => Some(Button::Main(2)),
                    KeyCode::Char('4') => Some(Button::Main(3)),
                    KeyCode::Char('q') => Some(Button::Intro(0)),
                    KeyCode::Char('w') => Some(Button::Intro(1)),
                    KeyCode::Char('e') => Some(Button::Intro(2)),
                    KeyCode::Char('i') => Some(Button::Ending(0)),
                    KeyCode::Char('o') => Some(Button::Ending(1)),
                    KeyCode::Char('p') => Some(Button::Ending(2)),
                    KeyCode::Char('g') => Some(Button::Break),
                    KeyCode::Char('y') => Some(Button::SyncStart),
                    KeyCode::Char('u') => Some(Button::AutoFill),
                    KeyCode::Char('j') => Some(Button::SyncStop),
                    KeyCode::Char('t') => Some(Button::TapTempo),
                    KeyCode::Char('=') | KeyCode::Char('+') => Some(Button::TempoUp),
                    KeyCode::Char('-') => Some(Button::TempoDown),
                    KeyCode::Char(c) if "zxcvbnm,".contains(c) => {
                        Some(Button::TogglePart("zxcvbnm,".find(c).unwrap() as u8))
                    }
                    KeyCode::Char('[') => {
                        let s = shared.split.load(Relaxed);
                        shared.split.store(s.saturating_sub(1).max(24), Relaxed);
                        None
                    }
                    KeyCode::Char(']') => {
                        let s = shared.split.load(Relaxed);
                        shared.split.store((s + 1).min(96), Relaxed);
                        None
                    }
                    KeyCode::Char('!') => {
                        let _ = ch.ui_tx.push(Cmd::Panic);
                        shared.wake.signal();
                        None
                    }
                    KeyCode::Left | KeyCode::Right => {
                        let n = styles.len();
                        let next = if k.code == KeyCode::Right { (idx + 1) % n } else { (idx + n - 1) % n };
                        match load(&styles[next]) {
                            Ok((p, i)) => {
                                if ch.style_tx.push(p).is_ok() {
                                    idx = next;
                                    info = i;
                                    message.clear();
                                    shared.wake.signal();
                                }
                            }
                            Err(e) => message = format!("{}: {e:#}", styles[next].display()),
                        }
                        None
                    }
                    _ => None,
                };
                if let Some(b) = b {
                    let _ = ch.ui_tx.push(Cmd::Button(b));
                    shared.wake.signal();
                }
            }
        }
    })();

    ratatui::restore();
    shared.quit.store(true, Relaxed);
    shared.wake.signal();
    let _ = engine_thread.join();
    if let Some(out) = leds.as_mut() {
        for n in (96..104).chain(112..120) {
            out.push(&[0x90, n, 0]);
        }
        out.push(&launchkey::EXIT_DAW);
        out.flush();
    }
    drop(port);
    result
}

fn gm_name(prog: u8) -> &'static str {
    const GM: [&str; 128] = [
        "Grand Piano", "Bright Piano", "E.Grand", "Honky-tonk", "E.Piano 1", "E.Piano 2", "Harpsichord", "Clavinet",
        "Celesta", "Glockenspiel", "Music Box", "Vibraphone", "Marimba", "Xylophone", "Tubular Bells", "Dulcimer",
        "Drawbar Organ", "Perc. Organ", "Rock Organ", "Church Organ", "Reed Organ", "Accordion", "Harmonica", "Bandoneon",
        "Nylon Gtr", "Steel Gtr", "Jazz Gtr", "Clean Gtr", "Muted Gtr", "Overdrive Gtr", "Distortion Gtr", "Gtr Harmonics",
        "Acoustic Bass", "Finger Bass", "Pick Bass", "Fretless Bass", "Slap Bass 1", "Slap Bass 2", "Synth Bass 1", "Synth Bass 2",
        "Violin", "Viola", "Cello", "Contrabass", "Tremolo Str", "Pizzicato Str", "Harp", "Timpani",
        "Strings", "Slow Strings", "Synth Str 1", "Synth Str 2", "Choir Aahs", "Voice Oohs", "Synth Voice", "Orch. Hit",
        "Trumpet", "Trombone", "Tuba", "Muted Trumpet", "French Horn", "Brass Section", "Synth Brass 1", "Synth Brass 2",
        "Soprano Sax", "Alto Sax", "Tenor Sax", "Baritone Sax", "Oboe", "English Horn", "Bassoon", "Clarinet",
        "Piccolo", "Flute", "Recorder", "Pan Flute", "Blown Bottle", "Shakuhachi", "Whistle", "Ocarina",
        "Square Lead", "Saw Lead", "Calliope", "Chiff Lead", "Charang", "Voice Lead", "Fifths Lead", "Bass+Lead",
        "New Age Pad", "Warm Pad", "Polysynth", "Choir Pad", "Bowed Pad", "Metallic Pad", "Halo Pad", "Sweep Pad",
        "Rain", "Soundtrack", "Crystal", "Atmosphere", "Brightness", "Goblins", "Echoes", "Sci-fi",
        "Sitar", "Banjo", "Shamisen", "Koto", "Kalimba", "Bagpipe", "Fiddle", "Shanai",
        "Tinkle Bell", "Agogo", "Steel Drums", "Woodblock", "Taiko", "Melodic Tom", "Synth Drum", "Reverse Cymbal",
        "Fret Noise", "Breath Noise", "Seashore", "Bird", "Telephone", "Helicopter", "Applause", "Gunshot",
    ];
    GM[prog as usize & 127]
}

fn voice_label(dest: u8, v: Option<(u8, u8, u8)>) -> String {
    match v {
        None => "—".into(),
        Some((msb, lsb, pc)) if msb >= 126 || dest == 8 || dest == 9 => format!("drum kit {msb}/{lsb}/{}", pc + 1),
        Some((0, 0, pc)) => format!("{} (GM {})", gm_name(pc), pc + 1),
        Some((msb, lsb, pc)) => format!("≈ {}  [{msb}/{lsb}/{}]", gm_name(pc), pc + 1),
    }
}

const PART_NAMES: [&str; 8] = ["Rhythm 1", "Rhythm 2", "Bass", "Chord 1", "Chord 2", "Pad", "Phrase 1", "Phrase 2"];

#[allow(clippy::too_many_arguments)]
fn draw(
    f: &mut ratatui::Frame,
    info: &Loaded,
    snap: Option<&Snapshot>,
    shared: &Shared,
    connected: &[String],
    idx: usize,
    total: usize,
    message: &str,
) {
    let area = f.area();
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(5),
        Constraint::Length(4),
        Constraint::Length(10),
        Constraint::Length(3),
        Constraint::Min(4),
    ])
    .split(area);

    let bold = St::default().add_modifier(Modifier::BOLD);
    let dim = St::default().fg(Color::DarkGray);
    let s = snap.copied();
    let bpm = s.map(|s| s.bpm).unwrap_or(0.0);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" yahaha ", St::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(format!("  {}  ", info.name)),
            Span::styled(format!("[{}]", info.format), dim),
            Span::raw(format!("   {:.0} bpm   style {}/{}   ←/→ change style", bpm, idx + 1, total)),
        ])),
        rows[0],
    );

    // Transport + chord.
    let (state, pos) = match s {
        Some(s) if s.running => (
            format!("▶ {}", s.cur.map(|c| c.name().to_uppercase()).unwrap_or_default()),
            format!("bar {}  beat {}", s.bar + 1, s.beat + 1),
        ),
        Some(s) if s.sync_armed => ("◆ SYNC START — play a chord".to_string(), String::new()),
        _ => ("■ stopped".to_string(), String::new()),
    };
    let next = s.and_then(|s| s.queued).map(|q| format!("next: {}", q.name())).unwrap_or_default();
    let chord = s.and_then(|s| s.chord).map(|c| c.name()).unwrap_or_else(|| "—".into());
    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![Span::styled(state, bold), Span::raw(format!("   {pos}   ")), Span::styled(next, St::default().fg(Color::Yellow))]),
            Line::raw(""),
            Line::from(vec![Span::raw("  chord  "), Span::styled(chord, bold.fg(Color::Cyan))]),
        ])
        .block(Block::default().borders(Borders::ALL)),
        rows[1],
    );

    // Sections.
    let sec_span = |id: SectionId| -> Span {
        let name = match id {
            SectionId::Intro(i) => format!("Intro {}", ["I", "II", "III", "IV"][i as usize]),
            SectionId::Ending(i) => format!("End {}", ["I", "II", "III", "IV"][i as usize]),
            SectionId::Break => "Break".into(),
            other => other.name(),
        };
        let st = if !info.has[slot_of(id)] {
            dim
        } else if s.and_then(|s| s.cur) == Some(id) {
            St::default().fg(Color::Black).bg(Color::Green)
        } else if s.and_then(|s| s.queued) == Some(id) {
            St::default().fg(Color::Black).bg(Color::Yellow)
        } else {
            St::default()
        };
        Span::styled(format!(" {name} "), st)
    };
    let mut l1 = vec![];
    for i in 0..3 {
        l1.push(sec_span(SectionId::Intro(i)));
    }
    l1.push(Span::raw("   "));
    for i in 0..4 {
        l1.push(sec_span(SectionId::Main(i)));
    }
    let mut l2 = vec![];
    for i in 0..4 {
        l2.push(sec_span(SectionId::Fill(i)));
    }
    l2.push(sec_span(SectionId::Break));
    l2.push(Span::raw("   "));
    for i in 0..3 {
        l2.push(sec_span(SectionId::Ending(i)));
    }
    f.render_widget(
        Paragraph::new(vec![Line::from(l1), Line::from(l2)]).block(Block::default().borders(Borders::ALL).title(" sections ")),
        rows[2],
    );

    // Parts.
    let parts = s.map(|s| s.parts).unwrap_or(0xFF);
    let mut lines = vec![];
    for p in 0..8u8 {
        let on = parts & (1 << p) != 0;
        let key = "zxcvbnm,".chars().nth(p as usize).unwrap();
        lines.push(Line::from(vec![
            Span::styled(format!(" [{key}] ch {:>2} ", 9 + p), dim),
            Span::styled(format!("{:<9}", PART_NAMES[p as usize]), if on { bold } else { dim }),
            Span::styled(format!(" {}", voice_label(8 + p, info.voices[8 + p as usize])), if on { St::default() } else { dim }),
        ]));
    }
    f.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" parts → virtual port \"yahaha\" (you: RH ch 1, LH ch 2) ")),
        rows[3],
    );

    // Status.
    let flag = |on: bool, name: &str| Span::styled(format!(" {name} "), if on { St::default().fg(Color::Black).bg(Color::Cyan) } else { dim });
    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                flag(s.map_or(false, |s| s.sync_armed), "SYNC START [y]"),
                flag(s.map_or(false, |s| s.auto_fill), "AUTO FILL [u]"),
                flag(s.map_or(false, |s| s.sync_stop), "SYNC STOP [j]"),
                Span::raw(format!("  split {} [ / ]", note_name(shared.split.load(Relaxed)))),
            ]),
            Line::from(Span::styled(
                format!(
                    " engine: RT {}  wake err p99 <{}µs  chord→engine p99 <{}µs  midi-in p99 <{}µs",
                    if shared.engine_rt.load(Relaxed) { "on" } else { "OFF" },
                    shared.lateness.percentile_us(0.99),
                    shared.chord_lat.percentile_us(0.99),
                    shared.input_lat.percentile_us(0.99),
                ),
                dim,
            )),
        ]),
        rows[4],
    );

    let mut help = vec![
        Line::from(Span::styled(
            " space start/stop · 1-4 Main A-D (again = fill) · q w e intro · i o p ending · g break · t tap · -/= tempo · ! panic · esc quit",
            dim,
        )),
        Line::from(Span::styled(format!(" inputs: {}", connected.join(", ")), dim)),
    ];
    if !message.is_empty() {
        help.push(Line::from(Span::styled(format!(" {message}"), St::default().fg(Color::Red))));
    }
    let _ = id_of;
    f.render_widget(Paragraph::new(help), rows[5]);
}

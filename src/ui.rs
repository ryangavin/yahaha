//! `yahaha play`: device setup, terminal front panel, Launchkey LEDs. Runs on the main
//! thread at normal priority; talks to the engine only through lock-free rings.

use crate::engine::{id_of, Button, Engine, Prepared, Snapshot, Transpose, NUM_SLOTS};
use crate::fingering::Fingering;
use crate::launchkey::{self, Led};
use crate::live::{self, Cmd, Input, Shared, TAG_KEYS, TAG_PADS};
use crate::midi::{self, Client};
use crate::rt::{PacketSink, Target};
use crate::synth;
use crate::sff::Style;
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
    /// SoundFont for the built-in synth; None = no synth.
    pub sf2: Option<PathBuf>,
    /// Use Novation palette colours (and hardware flashing) instead of RGB SysEx.
    pub palette_leds: bool,
    /// 1-based left output channel for the synth (None = auto).
    pub audio_out: Option<u8>,
    /// Chord fingering type at startup.
    pub fingering: Fingering,
    /// Chord Detection Area = Upper.
    pub upper: bool,
    /// The Manual Bass setting (takes effect in Upper mode only).
    pub manual_bass: bool,
    /// Initial Keyboard / Master transpose.
    pub transpose: Transpose,
}

/// Push the effective Manual Bass state (Upper mode and the setting both on) to the
/// engine, which mutes the Style's Bass part, and to the synth, which gives the Left part
/// the Style's Bass voice.
fn sync_manual_bass(shared: &Shared, ui_tx: &mut rtrb::Producer<Cmd>, synth: Option<&synth::Synth>) {
    let on = shared.manual_bass();
    if ui_tx.push(Cmd::ManualBass(on)).is_ok() {
        shared.wake.signal();
    }
    if let Some(sy) = synth {
        sy.control.set_manual_bass(on);
    }
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
    ots: Vec<crate::sff::Ots>,
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
    let info = Loaded { name, format: style.format.clone(), has, voices: prep.voices, ots: style.ots.clone() };
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
    shared.fingering.store(opts.fingering.to_u8(), Relaxed);
    shared.upper.store(opts.upper, Relaxed);
    shared.manual_bass.store(opts.manual_bass, Relaxed);

    // --- built-in synth (optional) ---
    let mut feeds = synth::feeds();
    let mut synth_err = String::new();
    let synth = match &opts.sf2 {
        Some(p) => match synth::start(p, std::mem::take(&mut feeds.consumers), opts.audio_out) {
            Ok(s) => Some(s),
            Err(e) => {
                synth_err = format!("synth off: {e:#}");
                None
            }
        },
        None => None,
    };
    if synth.is_none() {
        feeds.engine = None;
        feeds.input = None;
    }

    let mut ch = live::channels(live::Out::new(PacketSink::new(Target::Virtual(out_src)), feeds.engine));
    let mut input = Input::new(
        shared.clone(),
        Recognizer::new(),
        ch.input_tx,
        live::Out::new(PacketSink::new(Target::Virtual(out_src)), feeds.input),
    );
    input.set_synth(synth.as_ref().map(|s| s.control.clone()));
    let port = client.input_port("yahaha in", input)?;

    let sources = midi::sources();
    let lk_keys: Vec<_> = sources.iter().filter(|(_, n)| n.contains("Launchkey") && !n.contains("DAW")).collect();
    let lk_daw = sources.iter().find(|(_, n)| n.contains("Launchkey") && n.contains("DAW"));
    let mut connected = Vec::new();
    if !opts.inputs.is_empty() {
        for (e, n) in midi::sources() {
            if opts.inputs.iter().any(|want| n.contains(want.as_str())) {
                port.connect(e, TAG_KEYS)?;
                connected.push(n.clone());
            }
        }
        if connected.is_empty() {
            connected.push(format!("(nothing matched {:?})", opts.inputs));
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

    // --- transpose ---
    let mut transpose = opts.transpose;
    // Played notes and the engine must agree, so the key shift only changes once the
    // engine has the command. Returns false (nothing changed) if the ring is full.
    let set_transpose = |t: Transpose, tx: &mut rtrb::Producer<Cmd>| -> bool {
        if tx.push(Cmd::Transpose(t)).is_err() {
            return false;
        }
        shared.key_shift.store(t.keys(), Relaxed);
        shared.wake.signal();
        true
    };
    if !set_transpose(transpose, &mut ch.ui_tx) {
        transpose = Transpose::default();
    }

    // --- engine thread ---
    let engine = Engine::new(prep);
    let sh = shared.clone();
    let io = ch.io;
    let engine_thread =
        std::thread::Builder::new().name("yahaha-engine".into()).spawn(move || live::run_engine(engine, io, sh))?;
    if let Some(sy) = &synth {
        sy.control.set_bass_program(synth::style_bass_program(info.voices[10]));
    }
    sync_manual_bass(&shared, &mut ch.ui_tx, synth.as_ref());

    // --- terminal ---
    let mut term = ratatui::init();
    let mut snap: Option<Snapshot> = None;
    let mut last_leds: [(u8, Option<Led>); 16] = [(0, None); 16];
    let mut last_rgb: [Option<(u8, u8, u8)>; 16] = [None; 16];
    let mut last_fader_btns: Option<(u8, bool)> = None;
    let mut last_side: Option<(bool, bool)> = None;
    let mut last_ots_key: Option<(usize, u8)> = None;
    let mut last_link = false;
    let mut led_buf = Vec::new();
    let mut message = synth_err;
    let clock = std::time::Instant::now();
    let mut beats = 0.0f64;
    let mut last_tick = 0.0f64;
    let result: Result<()> = (|| loop {
        while let Ok(s) = ch.snap_rx.pop() {
            snap = Some(s);
        }
        // OTS Link: Main A-D recall One Touch Settings 1-4 (also on style change).
        if let (Some(sy), Some(s)) = (&synth, &snap) {
            let key = (idx, s.main);
            let link = sy.control.ots_link.load(Relaxed);
            if link && (last_ots_key != Some(key) || !last_link) {
                if let Some(o) = info.ots.get(s.main as usize) {
                    sy.control.apply_ots(o, s.main + 1);
                }
            }
            last_ots_key = Some(key);
            last_link = link;
        }
        while ch.old_rx.pop().is_ok() {} // drop old styles here, off the RT thread

        // Free-running beat clock for flashing/pulsing, following the current tempo.
        let t = clock.elapsed().as_secs_f64();
        beats += (t - last_tick) * snap.map_or(120.0, |s| s.bpm) / 60.0;
        last_tick = t;
        if let (Some(s), Some(out)) = (&snap, leds.as_mut()) {
            if opts.palette_leds {
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
            } else {
                for (i, (pad, look)) in launchkey::looks(s, &info.has).iter().enumerate() {
                    let rgb = launchkey::rgb_at(look, beats);
                    if last_rgb[i] != Some(rgb) {
                        out.push(&launchkey::rgb_sysex(*pad, rgb));
                        last_rgb[i] = Some(rgb);
                    }
                }
            }
            if let Some(sy) = &synth {
                let fb = (sy.control.active.load(Relaxed), sy.control.layer_mode.load(Relaxed));
                if last_fader_btns != Some(fb) {
                    led_buf.clear();
                    launchkey::fader_button_msgs(fb.0, fb.1, &mut led_buf);
                    for m in &led_buf {
                        out.push(m);
                    }
                    last_fader_btns = Some(fb);
                }
                let side = (sy.control.lh_sound.load(Relaxed), sy.control.ots_link.load(Relaxed));
                if last_side != Some(side) {
                    led_buf.clear();
                    launchkey::side_button_msgs(side.0, side.1, &mut led_buf);
                    for m in &led_buf {
                        out.push(m);
                    }
                    last_side = Some(side);
                }
            }
            out.flush();
        }

        term.draw(|f| draw(f, &info, snap.as_ref(), &shared, &connected, idx, styles.len(), &message, beats, synth.as_ref().map(|s| (&s.info, &*s.control))))?;

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
                    KeyCode::Char('9') | KeyCode::Char('0') => {
                        if let Some(sy) = &synth {
                            sy.control.step_focus_program(if k.code == KeyCode::Char('0') { 1 } else { -1 });
                        }
                        None
                    }
                    KeyCode::F(n) if (1..=8).contains(&n) => {
                        if let Some(sy) = &synth {
                            sy.control.press_slot(n - 1);
                        }
                        None
                    }
                    KeyCode::F(9) => {
                        if let Some(sy) = &synth {
                            let l = !sy.control.layer_mode.load(Relaxed);
                            sy.control.layer_mode.store(l, Relaxed);
                        }
                        None
                    }
                    KeyCode::Char('l') => {
                        if let Some(sy) = &synth {
                            let v = !sy.control.lh_sound.load(Relaxed);
                            sy.control.lh_sound.store(v, Relaxed);
                        }
                        None
                    }
                    KeyCode::Char('a') => {
                        // Next stereo output pair: 1/2 -> 3/4 -> ... -> back to 1/2.
                        if let Some(sy) = &synth {
                            let n = sy.info.channels.max(2) as u8;
                            let c = sy.control.out_ch.load(Relaxed);
                            sy.control.out_ch.store(if c + 4 <= n { c + 2 } else { 0 }, Relaxed);
                        }
                        None
                    }
                    KeyCode::Char('k') => {
                        if let Some(sy) = &synth {
                            let v = !sy.control.muted.load(Relaxed);
                            sy.control.muted.store(v, Relaxed);
                        }
                        None
                    }
                    KeyCode::Char(c) if "!@#$".contains(c) => {
                        let n = "!@#$".find(c).unwrap();
                        if let (Some(sy), Some(o)) = (&synth, info.ots.get(n)) {
                            sy.control.apply_ots(o, n as u8 + 1);
                        }
                        None
                    }
                    KeyCode::F(10) => {
                        if let Some(sy) = &synth {
                            let v = !sy.control.ots_link.load(Relaxed);
                            sy.control.ots_link.store(v, Relaxed);
                        }
                        None
                    }
                    KeyCode::Char('(') | KeyCode::Char(')') => {
                        if let Some(sy) = &synth {
                            sy.control.step_left_program(if k.code == KeyCode::Char(')') { 1 } else { -1 });
                        }
                        None
                    }
                    // TRANSPOSE -/+: ; ' for Keyboard, : " (shifted) for Master, / resets both.
                    KeyCode::Char(c) if ";':\"/".contains(c) => {
                        let (kb, m) = (transpose.keyboard, transpose.master);
                        let t = match c {
                            ';' => Transpose::new(kb - 1, m),
                            '\'' => Transpose::new(kb + 1, m),
                            ':' => Transpose::new(kb, m - 1),
                            '"' => Transpose::new(kb, m + 1),
                            _ => Transpose::default(),
                        };
                        if set_transpose(t, &mut ch.ui_tx) {
                            transpose = t;
                        }
                        None
                    }
                    KeyCode::Char('h') => Some(Button::StopAcmp),
                    KeyCode::Char('f') => {
                        let f = Fingering::from_u8(shared.fingering.load(Relaxed)).next();
                        shared.fingering.store(f.to_u8(), Relaxed);
                        shared.wake.signal();
                        None
                    }
                    KeyCode::Char('d') => {
                        let v = !shared.upper.load(Relaxed);
                        shared.upper.store(v, Relaxed);
                        // Selecting Upper turns Manual Bass on, its default there.
                        if v {
                            shared.manual_bass.store(true, Relaxed);
                        }
                        sync_manual_bass(&shared, &mut ch.ui_tx, synth.as_ref());
                        None
                    }
                    KeyCode::Char('D') => {
                        // Manual Bass is only available in Upper mode.
                        if shared.upper.load(Relaxed) {
                            let v = !shared.manual_bass.load(Relaxed);
                            shared.manual_bass.store(v, Relaxed);
                            sync_manual_bass(&shared, &mut ch.ui_tx, synth.as_ref());
                        }
                        None
                    }
                    KeyCode::Char('\\') => {
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
                                    if let Some(sy) = &synth {
                                        sy.control.set_bass_program(synth::style_bass_program(info.voices[10]));
                                    }
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

pub fn gm_name(prog: u8) -> &'static str {
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
        Some((msb, lsb, pc)) => {
            let gm = crate::synth::gm_fallback(dest, msb, pc);
            format!("≈ {}  [Yamaha {msb}/{lsb}/{}]", gm_name(gm), pc + 1)
        }
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
    beats: f64,
    synth: Option<(&synth::SynthInfo, &synth::SynthControl)>,
) {
    let area = f.area();
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(5),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(6),
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
    let tr = s.map(|s| s.transpose).unwrap_or_default();
    let fingered = match s.and_then(|s| s.played) {
        Some(p) if tr.keyboard != 0 => format!("  (fingered {})", p.name()),
        _ => String::new(),
    };
    let tr_st = if tr == Transpose::default() { dim } else { St::default().fg(Color::Yellow) };
    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![Span::styled(state, bold), Span::raw(format!("   {pos}   ")), Span::styled(next, St::default().fg(Color::Yellow))]),
            Line::raw(""),
            Line::from(vec![
                Span::raw("  chord  "),
                Span::styled(chord, bold.fg(Color::Cyan)),
                Span::styled(fingered, dim),
                Span::styled(format!("   transpose kbd {:+} master {:+}", tr.keyboard, tr.master), tr_st),
            ]),
        ])
        .block(Block::default().borders(Borders::ALL)),
        rows[1],
    );

    // Pad map: mirrors the Launchkey pads, same colours and animation.
    let default_snap = Snapshot {
        running: false, sync_armed: true, sync_stop: false, auto_fill: true, cur: None, queued: None,
        pending_intro: None, main: 0, bar: 0, beat: 0, chord: None, bpm: 120.0, parts: 0xFF, gains: [127; 8], stop_acmp: false,
        transpose: Transpose::default(), played: None,
    };
    let looks = launchkey::looks(s.as_ref().unwrap_or(&default_snap), &info.has);
    let pad_lines = |row: &[(u8, launchkey::Look)]| -> [Line<'static>; 3] {
        let mut top = vec![Span::raw(" ")];
        let mut mid = vec![Span::raw(" ")];
        let mut bot = vec![Span::raw(" ")];
        for (_, look) in row {
            let (r, g, b) = launchkey::rgb_at(look, beats);
            // Lift low levels a little on screen; terminals render dark colours darker than LEDs.
            let lift = |c: u8| ((c as f32 / 127.0).powf(0.8) * 255.0) as u8;
            let bg = Color::Rgb(lift(r), lift(g), lift(b));
            let lum = r as u32 * 3 + g as u32 * 6 + b as u32;
            let fg = if lum > 500 { Color::Black } else { Color::Gray };
            let st = St::default().bg(bg).fg(fg);
            top.push(Span::styled(format!("{:^10}", ""), st));
            mid.push(Span::styled(format!("{:^10}", look.label), st.add_modifier(Modifier::BOLD)));
            bot.push(Span::styled(format!("{:^10}", format!("[{}]", look.key)), st));
            for v in [&mut top, &mut mid, &mut bot] {
                v.push(Span::raw(" "));
            }
        }
        [Line::from(top), Line::from(mid), Line::from(bot)]
    };
    let mut pad_rows: Vec<Line> = Vec::new();
    pad_rows.extend(pad_lines(&looks[..8]));
    pad_rows.push(Line::raw(""));
    pad_rows.extend(pad_lines(&looks[8..]));
    pad_rows.push(Line::from(Span::styled(
        " dim = available · bright = playing / on · flashing = queued (next bar; fills next beat) · pulsing = armed, waiting for you · dark = style lacks it",
        dim,
    )));
    f.render_widget(
        Paragraph::new(pad_rows).block(Block::default().borders(Borders::ALL).title(" Launchkey pads (same colours as the hardware) ")),
        rows[2],
    );

    // Parts.
    let parts = s.map(|s| s.parts).unwrap_or(0xFF);
    let mut lines = vec![];
    for p in 0..8u8 {
        // Manual Bass mutes the Style's Bass part in the engine (its voice moves to the left hand).
        let manual_bass = p == 2 && shared.manual_bass();
        let on = parts & (1 << p) != 0 && !manual_bass;
        let key = "zxcvbnm,".chars().nth(p as usize).unwrap();
        let g = s.map_or(127, |s| s.gains[p as usize]);
        let bar = "█".repeat((g as usize * 8).div_ceil(127)) + &"·".repeat(8 - (g as usize * 8).div_ceil(127));
        lines.push(Line::from(vec![
            Span::styled(format!(" [{key}] ch {:>2} ", 9 + p), dim),
            Span::styled(format!("{bar} "), if on { St::default().fg(Color::Green) } else { dim }),
            Span::styled(format!("{:<9}", PART_NAMES[p as usize]), if on { bold } else { dim }),
            Span::styled(format!(" {}", voice_label(8 + p, info.voices[8 + p as usize])), if on { St::default() } else { dim }),
            Span::styled(if manual_bass { "  (muted: Manual Bass)" } else { "" }, St::default().fg(Color::Yellow)),
        ]));
    }
    f.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" parts / faders 1-8 → virtual port \"yahaha\" (you: RH ch 1, LH ch 2) ")),
        rows[3],
    );

    // Status.
    let flag = |on: bool, name: &str| Span::styled(format!(" {name} "), if on { St::default().fg(Color::Black).bg(Color::Cyan) } else { dim });
    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                flag(s.map_or(false, |s| s.sync_armed), "SYNC START [y]"),
                flag(s.map_or(false, |s| s.auto_fill), "AUTO FILL [u]"),
                if shared.sync_stop_allowed() {
                    flag(s.map_or(false, |s| s.sync_stop), "SYNC STOP [j]")
                } else {
                    Span::styled(" SYNC STOP n/a ", dim)
                },
                flag(s.map_or(false, |s| s.stop_acmp), "STOP ACMP [h]"),
                flag(synth.map_or(false, |(_, c)| c.ots_link.load(Relaxed)), "OTS LINK [F10]"),
                Span::styled(
                    match (info.ots.len(), synth.map(|(_, c)| c.ots_applied.load(Relaxed)).unwrap_or(0)) {
                        (0, _) => " no One Touch Settings".to_string(),
                        (n, 0) => format!(" {n} OTS [shift 1-{n}]"),
                        (n, a) => format!(" OTS {a}/{n} loaded [shift 1-{n}]"),
                    },
                    dim,
                ),
                Span::raw(format!("  split {} [ / ]", note_name(shared.split.load(Relaxed)))),
                Span::raw(format!(
                    "  fingering {}{} [f]",
                    Fingering::from_u8(shared.fingering.load(Relaxed)).name(),
                    // Upper overrides the selected type; it applies again back in Lower.
                    if shared.upper.load(Relaxed) { " (Upper: Fingered*)" } else { "" }
                )),
            ]),
            Line::from({
                let upper = shared.upper.load(Relaxed);
                let split = note_name(shared.split.load(Relaxed));
                let mut v = vec![Span::raw(" chord detection "), flag(upper, if upper { "UPPER · Fingered* [d]" } else { "LOWER [d]" })];
                if upper {
                    v.push(flag(shared.manual_bass.load(Relaxed), "MANUAL BASS [D]"));
                    let lh = if shared.manual_bass() { "bass (style Bass part muted)" } else { "Left voice" };
                    v.push(Span::styled(format!(" chord: keys above {split} · left hand: {lh}"), dim));
                } else {
                    v.push(Span::styled(format!(" chord: keys up to {split}"), dim));
                }
                v
            }),
            Line::from(match synth {
                Some((_, c)) => {
                    let active = c.active.load(Relaxed);
                    let focus = c.focus.load(Relaxed);
                    let mut v = vec![Span::raw(" voices ")];
                    for i in 0..crate::synth::SLOTS {
                        let on = active & (1 << i) != 0;
                        let name = gm_name(c.slots[i].load(Relaxed));
                        let short: String = name.chars().take(8).collect();
                        let st = if on { St::default().fg(Color::Black).bg(Color::Rgb(40, 110, 255)) } else { dim };
                        let st = if i as u8 == focus { st.add_modifier(Modifier::UNDERLINED) } else { st };
                        v.push(Span::styled(format!("F{} {short} ", i + 1), st));
                    }
                    v.push(Span::styled(
                        " LAYER [F9] ",
                        if c.layer_mode.load(Relaxed) { St::default().fg(Color::Black).bg(Color::Rgb(255, 140, 0)) } else { dim },
                    ));
                    v.push(Span::raw(" "));
                    let mb = c.manual_bass.load(Relaxed);
                    let left_on = c.lh_sound.load(Relaxed) || mb;
                    let left = if mb { &c.bass_program } else { &c.left_program };
                    let lname: String = gm_name(left.load(Relaxed)).chars().take(10).collect();
                    v.push(Span::styled(
                        format!(" LEFT {}{lname} [l ( )] ", if mb { "bass: " } else { "" }),
                        if left_on { St::default().fg(Color::Black).bg(Color::Rgb(40, 200, 90)) } else { dim },
                    ));
                    Line::from(v)
                }
                None => Line::from(Span::styled(" synth: off (MIDI out only; use --sf2 <file>)", dim)),
            }),
            Line::from(match synth {
                Some((info_s, c)) => {
                    Span::styled(
                        format!(
                            " synth: {} → {} out {}/{} [a] · {} Hz · {} · master {}% · re-voice slot [9/0] · {}[k]",
                            info_s.name,
                            info_s.device,
                            c.out_ch.load(Relaxed) + 1,
                            c.out_ch.load(Relaxed) + 2,
                            info_s.sample_rate,
                            info_s.buffer.map(|b| format!("{b} frames ({:.1} ms)", b as f64 * 1000.0 / info_s.sample_rate as f64)).unwrap_or("default buffer".into()),
                            c.master.load(Relaxed) as u32 * 100 / 127,
                            if c.muted.load(Relaxed) { "MUTED " } else { "" },
                        ),
                        dim,
                    )
                }
                None => Span::raw(""),
            }),
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
            " space start/stop · 1-4 Main A-D (again = fill) · q w e intro · i o p ending · g break · t tap · -/= tempo · F1-F8 voice · F9 layer · ; ' kbd transpose · : \" master · / reset · \\ panic · esc quit",
            dim,
        )),
        Line::from(Span::styled(
            format!(" inputs: {}   ·   last Launchkey control msg: {:06X}", connected.join(", "), shared.last_daw.load(Relaxed)),
            dim,
        )),
    ];
    if !message.is_empty() {
        help.push(Line::from(Span::styled(format!(" {message}"), St::default().fg(Color::Red))));
    }
    let _ = id_of;
    f.render_widget(Paragraph::new(help), rows[5]);
}

/// Debug: render one frame (with a sample playing state) to HTML so the layout can be
/// checked without a terminal. `yahaha screen <style> out.html`
pub fn screen_html(style: &Path, out: &Path) -> Result<()> {
    use ratatui::backend::TestBackend;
    let (_, info) = load(style)?;
    let shared = Shared::new(54);
    let snap = Snapshot {
        running: true,
        sync_armed: false,
        sync_stop: false,
        auto_fill: true,
        cur: Some(crate::sff::SectionId::Main(0)),
        queued: Some(crate::sff::SectionId::Fill(1)),
        pending_intro: None,
        main: 1,
        bar: 1,
        beat: 2,
        chord: Some(crate::theory::Chord { root: 9, ty: 10, bass: Some(7) }),
        bpm: 110.0,
        parts: 0xFF & !(1 << 5),
        gains: [127, 110, 96, 127, 80, 64, 127, 100],
        stop_acmp: false,
        transpose: Transpose::new(2, 0),
        played: Some(crate::theory::Chord { root: 7, ty: 10, bass: Some(5) }),
    };
    let mut term = ratatui::Terminal::new(TestBackend::new(150, 44))?;
    let si = synth::SynthInfo { name: "GeneralUser-GS".into(), sample_rate: 48000, buffer: Some(64), device: "Model 16".into(), channels: 14 };
    let sc = synth::SynthControl::new(10);
    if let Some(o) = info.ots.first() {
        sc.apply_ots(o, 1);
    }
    sc.ots_link.store(true, Relaxed);
    sc.master.store(110, Relaxed);
    term.draw(|f| draw(f, &info, Some(&snap), &shared, &["Launchkey MK4 61 MIDI Out".into()], 0, 35, "", 0.25, Some((&si, &sc))))?;
    let buf = term.backend().buffer().clone();
    let col = |c: Color, dflt: &str| -> String {
        match c {
            Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
            Color::Black => "#000".into(),
            Color::Gray => "#bbb".into(),
            Color::DarkGray => "#666".into(),
            Color::Yellow => "#e5c000".into(),
            Color::Green => "#3c3".into(),
            Color::Cyan => "#3cc".into(),
            Color::Red => "#e33".into(),
            _ => dflt.into(),
        }
    };
    let mut html = String::from("<html><body style='background:#111;margin:0'><pre style='font:14px Menlo,monospace;line-height:1.15;color:#ddd;margin:8px'>");
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            let c = &buf[(x, y)];
            let sym = c.symbol().replace('&', "&amp;").replace('<', "&lt;");
            let bold = if c.modifier.contains(Modifier::BOLD) { "font-weight:bold;" } else { "" };
            html.push_str(&format!("<span style='color:{};background:{};{bold}'>{sym}</span>", col(c.fg, "#ddd"), col(c.bg, "transparent")));
        }
        html.push('\n');
    }
    html.push_str("</pre></body></html>");
    std::fs::write(out, html)?;
    Ok(())
}

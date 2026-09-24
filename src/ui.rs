//! `yahaha play`: the terminal front panel. The first client of the Session API: it
//! draws `AppState` and sends `AppCmd`s; the session runs everything else (MIDI, the
//! engine, the synth, the Launchkey). Runs on the main thread at normal priority.

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style as St};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use std::cell::Cell;
use std::path::Path;
use std::time::Duration;
use yahaha::api::{
    AppCmd, AppState, LibraryCmd, MixerCmd, MultiPadCmd, MultiPadState, OtsCmd, Pad, PadLamp, PadsCmd, PartsCmd, SettingsCmd, SystemCmd,
};
use yahaha::engine::Button;
use yahaha::launchkey::{self, Action};
use yahaha::library::{self, Info, Library};
use yahaha::parts::{self, FaderPage};
use yahaha::session::{Options, Session};

/// Keyboard shortcuts for the controls the Launchkey also reaches, so a key and its pad
/// or button send the same command.
fn key_action(code: KeyCode) -> Option<Action> {
    let b = |b| Some(Action::Button(b));
    match code {
        KeyCode::Char(' ') => b(Button::StartStop),
        KeyCode::Char('1') => b(Button::Main(0)),
        KeyCode::Char('2') => b(Button::Main(1)),
        KeyCode::Char('3') => b(Button::Main(2)),
        KeyCode::Char('4') => b(Button::Main(3)),
        KeyCode::Char('q') => b(Button::Intro(0)),
        KeyCode::Char('w') => b(Button::Intro(1)),
        KeyCode::Char('e') => b(Button::Intro(2)),
        KeyCode::Char('i') => b(Button::Ending(0)),
        KeyCode::Char('o') => b(Button::Ending(1)),
        KeyCode::Char('p') => b(Button::Ending(2)),
        KeyCode::Char('g') => b(Button::Break),
        KeyCode::Char('y') => b(Button::SyncStart),
        KeyCode::Char('u') => b(Button::AutoFill),
        KeyCode::Char('j') => b(Button::SyncStop),
        KeyCode::Char('t') => b(Button::TapTempo),
        KeyCode::Char('=') | KeyCode::Char('+') => b(Button::TempoUp),
        KeyCode::Char('-') => b(Button::TempoDown),
        KeyCode::Char('h') => b(Button::StopAcmp),
        KeyCode::Char(c) if "zxcvbnm,".contains(c) => b(Button::TogglePart("zxcvbnm,".find(c).unwrap() as u8)),
        KeyCode::Char('[') => Some(Action::Split(-1)),
        KeyCode::Char(']') => Some(Action::Split(1)),
        KeyCode::Char('f') => Some(Action::NextFingering),
        KeyCode::Char('d') => Some(Action::ToggleUpper),
        KeyCode::Char('D') => Some(Action::ToggleManualBass),
        // TRANSPOSE -/+: ; ' for Keyboard, : " (shifted) for Master, / resets both.
        KeyCode::Char(';') => Some(Action::Transpose { keyboard: -1, master: 0 }),
        KeyCode::Char('\'') => Some(Action::Transpose { keyboard: 1, master: 0 }),
        KeyCode::Char(':') => Some(Action::Transpose { keyboard: 0, master: -1 }),
        KeyCode::Char('"') => Some(Action::Transpose { keyboard: 0, master: 1 }),
        KeyCode::Char('/') => Some(Action::TransposeReset),
        KeyCode::Char(c) if "!@#$".contains(c) => Some(Action::Ots("!@#$".find(c).unwrap() as u8)),
        KeyCode::F(10) => Some(Action::ToggleOtsLink),
        KeyCode::F(9) => Some(Action::ToggleFaderPage),
        KeyCode::F(n) if (1..=4).contains(&n) => Some(Action::SelectPart(n - 1)),
        KeyCode::Char(c) if "5678".contains(c) => Some(Action::PartOnOff("5678".find(c).unwrap() as u8)),
        KeyCode::Char('l') => Some(Action::PartOnOff(parts::LEFT as u8)),
        KeyCode::Char('9') => Some(Action::PartVoice(-1)),
        KeyCode::Char('0') => Some(Action::PartVoice(1)),
        KeyCode::Left => Some(Action::Style(-1)),
        KeyCode::Right => Some(Action::Style(1)),
        _ => None,
    }
}

/// The keys that are commands but not Launchkey actions.
fn key_cmd(code: KeyCode) -> Option<AppCmd> {
    match code {
        KeyCode::Tab => Some(AppCmd::Pads(PadsCmd::CyclePadPage { delta: 1 })),
        KeyCode::BackTab => Some(AppCmd::Pads(PadsCmd::CyclePadPage { delta: -1 })),
        // Next stereo output pair: 1/2 -> 3/4 -> ... -> back to 1/2.
        KeyCode::Char('a') => Some(AppCmd::Settings(SettingsCmd::NextAudioOutput)),
        KeyCode::Char('k') => Some(AppCmd::Mixer(MixerCmd::ToggleSynthMute)),
        KeyCode::Char('\\') => Some(AppCmd::System(SystemCmd::Panic)),
        // Multi Pads 1-4 (Shift+z x c v, above the Style part keys) and their STOP (Shift+b).
        KeyCode::Char(c) if "ZXCV".contains(c) => {
            Some(AppCmd::MultiPad(MultiPadCmd::TriggerMultiPad { pad: "ZXCV".find(c).unwrap() as u8 }))
        }
        KeyCode::Char('B') => Some(AppCmd::MultiPad(MultiPadCmd::StopAllMultiPads)),
        code => key_action(code).map(AppCmd::from),
    }
}

/// The Multi Pads on the status line: the bank and each pad's lamp (· ready, > playing,
/// ~ waiting for the bar, * armed, blank empty).
fn multi_pad_line(mp: &MultiPadState) -> String {
    let Some(bank) = &mp.bank else { return "multi pad: none [Z X C V · B stop]".into() };
    let lamps: String = mp
        .pads
        .iter()
        .map(|p| match p.lamp {
            PadLamp::Empty => ' ',
            PadLamp::Ready => '·',
            PadLamp::Armed => '*',
            PadLamp::Queued => '~',
            PadLamp::Playing => '>',
        })
        .collect();
    format!("multi pad: {} [{lamps}] [Z X C V · B stop]", bank.name)
}

/// `Esc` quits only when pressed twice within `WINDOW`. `Esc` also closes the browser, and
/// `Enter` closes it by loading, so one habitual `Esc` too many mustn't stop the band.
#[derive(Default)]
struct QuitGuard {
    armed: Option<Duration>,
}

impl QuitGuard {
    const WINDOW: Duration = Duration::from_millis(1500);
    const MSG: &str = "press esc again to quit";

    /// `Esc` at `now` (time since start). True means quit.
    fn esc(&mut self, now: Duration) -> bool {
        let quit = self.armed.is_some_and(|t| now.saturating_sub(t) < Self::WINDOW);
        self.armed = (!quit).then_some(now);
        quit
    }

    /// Whether a second `Esc` at `now` would quit (the prompt shows while it would).
    fn is_armed(&mut self, now: Duration) -> bool {
        if self.armed.is_some_and(|t| now.saturating_sub(t) >= Self::WINDOW) {
            self.armed = None;
        }
        self.armed.is_some()
    }
}

/// The style browser overlay (`Enter` opens it). While it is open every typed key goes
/// here, never to the performance shortcuts; MIDI and the Launchkey are separate paths.
struct Browser {
    query: String,
    /// Entry id under the cursor. If the filter hides it, the first match is the cursor.
    cursor: usize,
    /// Rows the list showed last frame, for PgUp/PgDn.
    page: Cell<usize>,
}

enum BrowseKey {
    Stay,
    Close,
    Load(usize),
}

impl Browser {
    fn open(current: usize) -> Browser {
        Browser { query: String::new(), cursor: current, page: Cell::new(10) }
    }

    /// Matching entry ids and the cursor's position among them.
    fn visible(&self, lib: &Library) -> (Vec<usize>, usize) {
        let v = lib.filter(&self.query);
        let pos = v.iter().position(|&i| i == self.cursor).unwrap_or(0);
        (v, pos)
    }

    fn key(&mut self, code: KeyCode, mods: KeyModifiers, lib: &Library) -> BrowseKey {
        let (v, pos) = self.visible(lib);
        let page = self.page.get().max(1);
        let last = v.len().saturating_sub(1);
        let to = match code {
            KeyCode::Esc => return BrowseKey::Close,
            KeyCode::Enter => return v.get(pos).map_or(BrowseKey::Stay, |&id| BrowseKey::Load(id)),
            KeyCode::Up => pos.saturating_sub(1),
            KeyCode::Down => (pos + 1).min(last),
            KeyCode::PageUp => pos.saturating_sub(page),
            KeyCode::PageDown => (pos + page).min(last),
            KeyCode::Home => 0,
            KeyCode::End => last,
            KeyCode::Backspace => {
                self.query.pop();
                return BrowseKey::Stay;
            }
            // Ctrl/Alt chords are commands, not text (Ctrl+C quits before this).
            KeyCode::Char(_) if mods.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => return BrowseKey::Stay,
            KeyCode::Char(c) => {
                self.query.push(c);
                // Keep the cursor if it still matches, else take the first match.
                let (v, pos) = self.visible(lib);
                if let Some(&id) = v.get(pos) {
                    self.cursor = id;
                }
                return BrowseKey::Stay;
            }
            _ => return BrowseKey::Stay,
        };
        if let Some(&id) = v.get(to) {
            self.cursor = id;
        }
        BrowseKey::Stay
    }
}

/// `s` cut or padded to exactly `w` characters.
fn fit(s: &str, w: usize) -> String {
    let n = s.chars().count();
    if n > w {
        s.chars().take(w.saturating_sub(1)).chain(['…']).collect()
    } else {
        format!("{s}{}", " ".repeat(w - n))
    }
}

pub fn play(opts: Options) -> Result<()> {
    let session = Session::start(opts)?;
    let mut browser: Option<Browser> = None;
    let mut term = ratatui::init();
    let clock = std::time::Instant::now();
    let mut quit_guard = QuitGuard::default();
    // The quit prompt shows in the message line; it hides the session's message it
    // replaced for good, and a newer message replaces it.
    let mut quit_prompt = false;
    let mut hidden_msg: Option<u64> = None;
    let result: Result<()> = (|| loop {
        let st = session.state();
        let lib = session.library();
        let msg = st.message.as_ref().filter(|m| Some(m.seq) != hidden_msg);
        if quit_prompt && (msg.is_some() || !quit_guard.is_armed(clock.elapsed())) {
            quit_prompt = false;
        }
        let message = if quit_prompt { QuitGuard::MSG.to_string() } else { msg.map(|m| m.text.clone()).unwrap_or_default() };
        let beats = session.beats();
        term.draw(|f| {
            draw(f, &st, &message, beats);
            if let Some(b) = &browser {
                draw_browser(f, b, &lib, st.style.id, &message);
            }
        })?;

        if !event::poll(Duration::from_millis(16))? {
            continue;
        }
        let Event::Key(k) = event::read()? else { continue };
        if k.kind != KeyEventKind::Press {
            continue;
        }
        if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c') {
            return Ok(());
        }
        // Only two presses of Esc in a row quit.
        if k.code != KeyCode::Esc || browser.is_some() {
            quit_guard.armed = None;
        }
        // The browser takes every key while it's open (typing filters, never plays). Its
        // pick loads through the same command as ←/→; the browser closes once it loads.
        if let Some(b) = browser.as_mut() {
            match b.key(k.code, k.modifiers, &lib) {
                BrowseKey::Stay => {}
                BrowseKey::Close => browser = None,
                BrowseKey::Load(id) if id == st.style.id => browser = None,
                BrowseKey::Load(id) => {
                    if session.send(LibraryCmd::LoadStyle { id }).is_ok() {
                        browser = None;
                    }
                }
            }
            continue;
        }
        match k.code {
            KeyCode::Esc => {
                if quit_guard.esc(clock.elapsed()) {
                    return Ok(());
                }
                quit_prompt = true;
                hidden_msg = st.message.as_ref().map(|m| m.seq);
            }
            KeyCode::Enter => browser = Some(Browser::open(st.style.id)),
            code => {
                // Errors show in the message line (the session sets it).
                if let Some(c) = key_cmd(code) {
                    let _ = session.send(c);
                }
            }
        }
    })();
    ratatui::restore();
    session.stop();
    result
}

#[allow(clippy::too_many_lines)]
fn draw(f: &mut ratatui::Frame, st: &AppState, message: &str, beats: f64) {
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
    let t = &st.transport;
    let ch = &st.chord;

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" yahaha ", St::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(format!("  {}  ", st.style.name)),
            Span::styled(format!("[{}]", st.style.format), dim),
            Span::raw(format!(
                "   {:.0} bpm   style {}/{}   ←/→ or Track ◀/▶ change style · enter browse",
                t.tempo,
                st.library.position + 1,
                st.library.count
            )),
        ])),
        rows[0],
    );

    // Transport + chord.
    let (state, pos) = if t.running {
        (format!("▶ {}", t.section.as_deref().unwrap_or_default().to_uppercase()), format!("bar {}  beat {}", t.bar, t.beat))
    } else if t.sync_start {
        ("◆ SYNC START — play a chord".to_string(), String::new())
    } else {
        ("■ stopped".to_string(), String::new())
    };
    let next = t.queued.as_ref().map(|q| format!("next: {q}")).unwrap_or_default();
    let chord = ch.name.clone().unwrap_or_else(|| "—".into());
    let fingered = match &ch.fingered {
        Some(p) if ch.transpose_keyboard != 0 => format!("  (fingered {p})"),
        _ => String::new(),
    };
    let tr_st = if ch.transpose_keyboard == 0 && ch.transpose_master == 0 { dim } else { St::default().fg(Color::Yellow) };
    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![Span::styled(state, bold), Span::raw(format!("   {pos}   ")), Span::styled(next, St::default().fg(Color::Yellow))]),
            Line::raw(""),
            Line::from(vec![
                Span::raw("  chord  "),
                Span::styled(chord, bold.fg(Color::Cyan)),
                Span::styled(fingered, dim),
                Span::styled(format!("   transpose kbd {:+} master {:+}", ch.transpose_keyboard, ch.transpose_master), tr_st),
            ]),
        ])
        .block(Block::default().borders(Borders::ALL)),
        rows[1],
    );

    // Pad map: mirrors the Launchkey pads, same colours and animation.
    let pad_lines = |row: &[Pad]| -> [Line<'static>; 3] {
        let mut top = vec![Span::raw(" ")];
        let mut mid = vec![Span::raw(" ")];
        let mut bot = vec![Span::raw(" ")];
        for pad in row {
            let (r, g, b) = launchkey::lit((pad.rgb[0], pad.rgb[1], pad.rgb[2]), pad.level, pad.anim, beats);
            // Lift low levels a little on screen; terminals render dark colours darker than LEDs.
            let lift = |c: u8| ((c as f32 / 127.0).powf(0.8) * 255.0) as u8;
            let bg = Color::Rgb(lift(r), lift(g), lift(b));
            let lum = r as u32 * 3 + g as u32 * 6 + b as u32;
            let fg = if lum > 500 { Color::Black } else { Color::Gray };
            let st = St::default().bg(bg).fg(fg);
            top.push(Span::styled(format!("{:^10}", ""), st));
            mid.push(Span::styled(format!("{:^10}", pad.label), st.add_modifier(Modifier::BOLD)));
            bot.push(Span::styled(format!("{:^10}", format!("[{}]", pad.key)), st));
            for v in [&mut top, &mut mid, &mut bot] {
                v.push(Span::raw(" "));
            }
        }
        [Line::from(top), Line::from(mid), Line::from(bot)]
    };
    let pads = &st.pads.pads;
    let mut pad_rows: Vec<Line> = Vec::new();
    pad_rows.extend(pad_lines(&pads[..8.min(pads.len())]));
    pad_rows.push(Line::raw(""));
    pad_rows.extend(pad_lines(&pads[8.min(pads.len())..]));
    pad_rows.push(Line::from(Span::styled(
        " dim = available · bright = playing / on · flashing = queued (next bar; fills next beat) · pulsing = armed, waiting for you · dark = style lacks it",
        dim,
    )));
    f.render_widget(
        Paragraph::new(pad_rows).block(Block::default().borders(Borders::ALL).title(format!(
            " Launchkey pads · page {}/{} {} · Pad Bank ▲/▼ or Tab to switch (same colours as the hardware) ",
            st.pads.page_number, st.pads.page_count, st.pads.page_name,
        ))),
        rows[2],
    );

    // Mixer: the keyboard parts (Panel) and the Style parts, like the Genos Mixer tabs.
    // Each level is the part's CC7 (0-127); "↕" = the Launchkey fader must reach it first.
    let level = |g: u8, on: bool, waiting: bool| -> [Span<'static>; 2] {
        let n = (g as usize * 8).div_ceil(127);
        [
            Span::styled(format!("{}{} {g:>3}", "█".repeat(n), "·".repeat(8 - n)), if on { St::default().fg(Color::Green) } else { dim }),
            Span::styled(if waiting { "↕ " } else { "  " }, St::default().fg(Color::Yellow)),
        ]
    };
    let page = st.mixer.fader_page;
    let mut lines = vec![];
    for (p, kp) in st.keyboard_parts.iter().enumerate() {
        let on = kp.sounding;
        let sel = kp.selected;
        // The octave the part plays at: not applied to the bass under Manual Bass.
        let oct = if kp.plays_bass { 0 } else { kp.octave };
        let mut v = vec![Span::styled(format!(" {}[F{}] ch {} ", if sel { "▶" } else { " " }, p + 1, kp.channel), if sel { bold } else { dim })];
        v.extend(level(kp.volume, on, kp.waiting));
        v.push(Span::styled(format!("{:<8}", kp.name), if on { bold } else { dim }));
        v.push(Span::styled(format!(" {}{}", if kp.plays_bass { "bass: " } else { "" }, kp.voice_name), if on { St::default() } else { dim }));
        v.push(Span::styled(if oct != 0 { format!("  oct {oct:+}") } else { String::new() }, dim));
        // The sustain pedal holds this part's notes.
        let sus = st.controllers.sustain && on && st.controllers.parts.get(p).is_some_and(|c| c.sustain);
        v.push(Span::styled(if sus { "  sus" } else { "" }, St::default().fg(Color::Yellow)));
        lines.push(Line::from(v));
    }
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(" F1-F4 edit · 9/0 voice · 5 6 7 8 (l) on/off", dim)));
    lines.push(Line::from(Span::styled(
        match st.io.synth {
            Some(_) => " → port + synth, one channel per part",
            None => " → port only (synth off: --sf2 <file>)",
        },
        dim,
    )));
    let active = St::default().fg(Color::Yellow);
    let title = |name: &str, faders: &str, on: bool| {
        Line::from(Span::styled(
            format!(" {name}{} ", if on { format!(" · faders {faders} [F9]") } else { String::new() }),
            if on { active.add_modifier(Modifier::BOLD) } else { St::default() },
        ))
    };
    let cols = Layout::horizontal([Constraint::Length(58), Constraint::Min(20)]).split(rows[3]);
    let border = |on: bool| if on { active } else { dim };
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default().borders(Borders::ALL).border_style(border(page == FaderPage::Panel)).title(title("Panel", "1-4", page == FaderPage::Panel)),
        ),
        cols[0],
    );

    let mut lines = vec![];
    for (p, sp) in st.mixer.style_parts.iter().enumerate() {
        let key = "zxcvbnm,".chars().nth(p).unwrap_or(' ');
        let mut v = vec![Span::styled(format!(" [{key}] ch {:>2} ", sp.channel), dim)];
        v.extend(level(sp.volume, sp.on, sp.waiting));
        v.push(Span::styled(format!("{:<9}", sp.name), if sp.on { bold } else { dim }));
        let voice = sp.voice.as_ref().map_or("—", |v| v.label.as_str());
        v.push(Span::styled(format!(" {voice}"), if sp.on { St::default() } else { dim }));
        v.push(Span::styled(if sp.muted_by_manual_bass { "  (muted: Manual Bass)" } else { "" }, St::default().fg(Color::Yellow)));
        lines.push(Line::from(v));
    }
    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border(page == FaderPage::Style))
                .title(title("Style", "1-8", page == FaderPage::Style))
                .title_bottom(Line::from(Span::styled(" CC7 → virtual port \"yahaha\" · ↕ move fader to pick up ", dim))),
        ),
        cols[1],
    );

    // Status.
    let flag = |on: bool, name: &str| Span::styled(format!(" {name} "), if on { St::default().fg(Color::Black).bg(Color::Cyan) } else { dim });
    let ots = &st.ots;
    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                flag(t.sync_start, "SYNC START [y]"),
                flag(t.auto_fill, "AUTO FILL [u]"),
                if t.sync_stop_available { flag(t.sync_stop, "SYNC STOP [j]") } else { Span::styled(" SYNC STOP n/a ", dim) },
                flag(t.stop_acmp, "STOP ACMP [h]"),
                flag(ots.link, "OTS LINK [F10]"),
                Span::styled(
                    match (ots.settings.len(), ots.applied) {
                        (0, _) => " no One Touch Settings".to_string(),
                        (n, 0) => format!(" {n} OTS [shift 1-{n}]"),
                        (n, a) => format!(" OTS {a}/{n} loaded [shift 1-{n}]"),
                    },
                    dim,
                ),
                Span::raw(format!("  split {} [ / ]", ch.split_name)),
                Span::raw(format!(
                    "  fingering {}{} [f]",
                    ch.fingering_name,
                    // Upper overrides the selected type; it applies again back in Lower.
                    if ch.upper { " (Upper: Fingered*)" } else { "" }
                )),
            ]),
            Line::from({
                let mut v = vec![Span::raw(" chord detection "), flag(ch.upper, if ch.upper { "UPPER · Fingered* [d]" } else { "LOWER [d]" })];
                if ch.upper {
                    v.push(flag(ch.manual_bass, "MANUAL BASS [D]"));
                    let lh = if ch.manual_bass_active { "bass (style Bass part muted)" } else { "Left voice" };
                    v.push(Span::styled(format!(" chord: keys above {} · left hand: {lh}", ch.split_name), dim));
                } else {
                    v.push(Span::styled(format!(" chord: keys up to {}", ch.split_name), dim));
                }
                v.push(Span::raw(format!("   {}", multi_pad_line(&st.multi_pad))));
                v
            }),
            Line::from(match &st.io.synth {
                Some(sy) => Span::styled(
                    format!(
                        " synth: {} → {} out {}/{} [a] · {} Hz · {} · master {}{} · {}[k]",
                        sy.sound_font,
                        sy.device,
                        sy.output_pair[0],
                        sy.output_pair[1],
                        sy.sample_rate,
                        sy.buffer_frames
                            .map(|b| format!("{b} frames ({:.1} ms)", b as f64 * 1000.0 / sy.sample_rate as f64))
                            .unwrap_or("default buffer".into()),
                        st.mixer.master.unwrap_or(0),
                        if st.mixer.master_waiting { " ↕" } else { "" },
                        if sy.muted { "MUTED " } else { "" },
                    ),
                    dim,
                ),
                None => Span::styled(" synth: off (MIDI out only; use --sf2 <file>)", dim),
            }),
            Line::from(Span::styled(
                format!(
                    " engine: RT {}  wake err p99 <{}µs  chord→engine p99 <{}µs  midi-in p99 <{}µs",
                    if st.io.engine.realtime { "on" } else { "OFF" },
                    st.io.engine.wake_p99_us,
                    st.io.engine.chord_p99_us,
                    st.io.engine.midi_in_p99_us,
                ),
                dim,
            )),
        ]),
        rows[4],
    );

    let mut help = vec![
        Line::from(Span::styled(
            " space start/stop · 1-4 Main A-D (again = fill) · q w e intro · i o p ending · g break · t tap · -/= tempo · F1-F4 part · 9/0 voice · 5-8 part on/off · F9 faders Panel/Style · ; ' kbd transpose · : \" master · / reset · Z X C V multi pads · B pad stop · tab pad page · enter browse styles · \\ panic · esc twice quit",
            dim,
        )),
        Line::from(Span::styled(
            format!(" inputs: {}   ·   last Launchkey control msg: {:06X}   {}", st.io.inputs.join(", "), st.io.last_control, st.io.unmapped),
            dim,
        )),
    ];
    if !message.is_empty() {
        help.push(Line::from(Span::styled(format!(" {message}"), St::default().fg(Color::Red))));
    }
    f.render_widget(Paragraph::new(help), rows[5]);
}

/// The style browser, drawn over the front panel.
fn draw_browser(f: &mut ratatui::Frame, b: &Browser, lib: &Library, current: usize, message: &str) {
    let full = f.area();
    let area = ratatui::layout::Rect {
        x: full.x + 2.min(full.width / 10),
        y: full.y + 1.min(full.height / 10),
        width: full.width.saturating_sub(2 * 2.min(full.width / 10)),
        height: full.height.saturating_sub(2 * 1.min(full.height / 10)),
    };
    let dim = St::default().fg(Color::DarkGray);
    let (v, pos) = b.visible(lib);
    let pending = lib.pending();
    let title = if pending > 0 {
        format!(" Styles · {} of {} · indexing, {pending} to go ", v.len(), lib.count())
    } else {
        format!(" Styles · {} of {} ", v.len(), lib.count())
    };
    let block = Block::default().borders(Borders::ALL).title(title).border_style(St::default().fg(Color::Yellow));
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);
    let rows = Layout::vertical([Constraint::Length(2), Constraint::Min(1), Constraint::Length(3)]).split(inner);

    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::raw(" filter: "),
                Span::styled(format!("{}▏", b.query), St::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(if b.query.is_empty() { "  type to filter by name or folder" } else { "" }, dim),
            ]),
            Line::from(Span::styled(
                format!("   {}{}{:>8}  {:<5} sections", fit("name", 34), fit("folder", 28), "tempo", "time"),
                dim.add_modifier(Modifier::UNDERLINED),
            )),
        ]),
        rows[0],
    );

    // A window of the list with the cursor kept near the middle.
    let h = rows[1].height as usize;
    b.page.set(h.saturating_sub(1).max(1));
    let top = pos.saturating_sub(h / 2).min(v.len().saturating_sub(h));
    let mut lines = Vec::new();
    for (i, &id) in v.iter().enumerate().skip(top).take(h) {
        let e = lib.entry(id);
        let mut st = if id == current { St::default().fg(Color::Yellow).add_modifier(Modifier::BOLD) } else { St::default() };
        if i == pos {
            st = st.bg(Color::Rgb(40, 60, 110));
        }
        let mark = if id == current { " ▶ " } else { "   " };
        let (right, right_st) = match &e.info {
            Info::Pending => ("…".to_string(), dim),
            Info::Ok(s) => (
                format!("{:>4.0} bpm  {:<5} {}", s.bpm, format!("{}/{}", s.timesig.0, s.timesig.1), library::sections_text(&s.sections)),
                St::default(),
            ),
            Info::Err(err) => (format!("✗ {err}"), St::default().fg(Color::Red)),
        };
        let right_st = if i == pos { right_st.bg(Color::Rgb(40, 60, 110)) } else { right_st };
        lines.push(Line::from(vec![
            Span::styled(format!("{mark}{}{}", fit(e.name(), 34), fit(&e.folder, 28)), st),
            Span::styled(right, right_st),
        ]));
    }
    if v.is_empty() {
        lines.push(Line::from(Span::styled("   no style matches", dim)));
    }
    f.render_widget(Paragraph::new(lines), rows[1]);

    let detail = v.get(pos).map(|&id| lib.entry(id).path.display().to_string()).unwrap_or_default();
    let mut foot = vec![
        Line::from(Span::styled(format!(" {detail}"), dim)),
        Line::from(Span::styled(
            " type to filter · backspace edit · ↑/↓ PgUp/PgDn Home/End move · enter load (the band keeps playing) · esc close",
            dim,
        )),
    ];
    if !message.is_empty() {
        foot.push(Line::from(Span::styled(format!(" {message}"), St::default().fg(Color::Red))));
    }
    f.render_widget(Paragraph::new(foot), rows[2]);
}

/// Debug: render one frame (with a sample playing state) to HTML so the layout can be
/// checked without a terminal. `yahaha screen <style> out.html`; give a folder instead of a
/// style to see the browser open over it. The state comes from an offline session.
pub fn screen_html(style: &Path, out: &Path) -> Result<()> {
    use ratatui::backend::TestBackend;
    use yahaha::api::SynthState;
    use yahaha::engine::{Snapshot, Transpose};
    let session = Session::offline(Options { paths: vec![style.to_path_buf()], ..Options::default() })?;
    session.finish_indexing();
    let lib = session.library();
    let current = lib.order()[lib.len() / 3];
    let _ = session.send(LibraryCmd::LoadStyle { id: current });
    let browser = style.is_dir().then(|| Browser::open(current));
    let _ = session.send(OtsCmd::RecallOts { index: 0 });
    let _ = session.send(OtsCmd::SetOtsLink { on: true });
    let _ = session.send(PartsCmd::SelectPart { part: parts::RIGHT2 as u8 });
    session.show_snapshot(Snapshot {
        running: true,
        sync_armed: false,
        sync_stop: false,
        auto_fill: true,
        cur: Some(yahaha::sff::SectionId::Main(0)),
        queued: Some(yahaha::sff::SectionId::Fill(1)),
        pending_intro: None,
        main: 1,
        bar: 1,
        beat: 2,
        chord: Some(yahaha::theory::Chord { root: 9, ty: 10, bass: Some(7) }),
        bpm: 110.0,
        parts: 0xFF & !(1 << 5),
        volumes: [127, 110, 96, 127, 80, 64, 127, 100],
        pickup: 1 << 4,
        stop_acmp: false,
        transpose: Transpose::new(2, 0),
        played: Some(yahaha::theory::Chord { root: 7, ty: 10, bass: Some(5) }),
        anchor_ns: 0,
        anchor_beats: 6.0,
        style_tag: 0,
        style_pending: false,
        section_bars: 4,
        audition: None,
        multipad: Default::default(),
    });
    // What a live session with the synth and a Launchkey would add.
    let mut st = (*session.state()).clone();
    st.io.synth = Some(SynthState {
        sound_font: "GeneralUser-GS".into(),
        device: "Model 16".into(),
        sample_rate: 48000,
        buffer_frames: Some(64),
        channels: 14,
        output_pair: [11, 12],
        muted: false,
    });
    st.mixer.master = Some(110);
    st.io.inputs = vec!["Launchkey MK4 61 MIDI Out".into()];
    st.message = None;
    let mut term = ratatui::Terminal::new(TestBackend::new(150, 44))?;
    term.draw(|f| {
        draw(f, &st, "", 0.25);
        if let Some(b) = &browser {
            draw_browser(f, b, &lib, current, "");
        }
    })?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use yahaha::api::{ChordCmd, TransportCmd};
    use yahaha::launchkey::Page;

    /// Every Launchkey control on pages 2 and 3, and the Track buttons, is a keyboard
    /// shortcut's action (the fingering pads select directly what `f` steps through).
    #[test]
    fn launchkey_actions_are_keyboard_actions() {
        let keys: Vec<Action> = (0u8..128)
            .map(|c| KeyCode::Char(c as char))
            .chain((1..=12).map(KeyCode::F))
            .chain([KeyCode::Left, KeyCode::Right])
            .filter_map(key_action)
            .collect();
        let pads = [96u8, 97, 98, 99, 100, 101, 102, 103, 112, 113, 114, 115, 116, 117, 118, 119];
        for page in [Page::ChordSetup, Page::OtsParts] {
            for a in pads.iter().filter_map(|&n| launchkey::pad_action(page, n)) {
                if !matches!(a, Action::Fingering(_)) {
                    assert!(keys.contains(&a), "{page:?}: {a:?} has no key");
                }
            }
        }
        for cc in [launchkey::TRACK_LEFT_CC, launchkey::TRACK_RIGHT_CC] {
            let Some(launchkey::Control::Act(a)) = launchkey::cc_control(cc, false) else { panic!("track button") };
            assert!(keys.contains(&a));
        }
        assert_eq!(key_action(KeyCode::Right), Some(Action::Style(1)));
        assert_eq!(key_action(KeyCode::Char('f')), Some(Action::NextFingering));
    }

    /// Every key is a command: the Launchkey's, and the terminal's own.
    #[test]
    fn keys_send_commands() {
        assert_eq!(key_cmd(KeyCode::Char('2')), Some(AppCmd::Transport(TransportCmd::Main { index: 1 })));
        assert_eq!(key_cmd(KeyCode::Char('[')), Some(AppCmd::Chord(ChordCmd::MoveSplit { delta: -1 })));
        assert_eq!(key_cmd(KeyCode::Char('!')), Some(AppCmd::Ots(OtsCmd::RecallOts { index: 0 })));
        assert_eq!(key_cmd(KeyCode::BackTab), Some(AppCmd::Pads(PadsCmd::CyclePadPage { delta: -1 })));
        assert_eq!(key_cmd(KeyCode::Char('\\')), Some(AppCmd::System(SystemCmd::Panic)));
        assert_eq!(key_cmd(KeyCode::Char('k')), Some(AppCmd::Mixer(MixerCmd::ToggleSynthMute)));
        assert_eq!(key_cmd(KeyCode::Char('Z')), Some(AppCmd::MultiPad(MultiPadCmd::TriggerMultiPad { pad: 0 })));
        assert_eq!(key_cmd(KeyCode::Char('V')), Some(AppCmd::MultiPad(MultiPadCmd::TriggerMultiPad { pad: 3 })));
        assert_eq!(key_cmd(KeyCode::Char('B')), Some(AppCmd::MultiPad(MultiPadCmd::StopAllMultiPads)));
        assert_eq!(key_cmd(KeyCode::Char('K')), None);
    }

    /// Letters typed into the browser filter; they never reach the performance shortcuts
    /// (the caller hands the browser every key while it's open).
    #[test]
    fn browser_keys_filter_move_and_load() {
        let lib = Library::scan(&[PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("corpus")]);
        if lib.len() < 3 {
            return;
        }
        let first = lib.order()[0];
        let mut b = Browser::open(lib.order()[1]);
        assert!(matches!(b.key(KeyCode::Up, KeyModifiers::NONE, &lib), BrowseKey::Stay));
        assert_eq!(b.cursor, first);
        assert!(matches!(b.key(KeyCode::Up, KeyModifiers::NONE, &lib), BrowseKey::Stay));
        assert_eq!(b.cursor, first, "stops at the top");
        for c in "FUNK".chars() {
            assert!(matches!(b.key(KeyCode::Char(c), KeyModifiers::NONE, &lib), BrowseKey::Stay));
        }
        assert_eq!(b.query, "FUNK");
        let (v, pos) = b.visible(&lib);
        assert!(!v.is_empty() && v.iter().all(|&i| lib.entry(i).name().to_lowercase().contains("funk")));
        assert_eq!(v[pos], b.cursor, "the cursor moves onto a match");
        b.key(KeyCode::End, KeyModifiers::NONE, &lib);
        assert_eq!(b.cursor, *v.last().unwrap());
        assert!(matches!(b.key(KeyCode::Enter, KeyModifiers::NONE, &lib), BrowseKey::Load(id) if id == *v.last().unwrap()));
        b.key(KeyCode::Backspace, KeyModifiers::NONE, &lib);
        assert_eq!(b.query, "FUN");
        assert!(matches!(b.key(KeyCode::Esc, KeyModifiers::NONE, &lib), BrowseKey::Close));
        // Nothing matches: Enter does nothing.
        b.key(KeyCode::Char('#'), KeyModifiers::NONE, &lib);
        b.key(KeyCode::Char('#'), KeyModifiers::NONE, &lib);
        assert!(matches!(b.key(KeyCode::Enter, KeyModifiers::NONE, &lib), BrowseKey::Stay));
    }

    /// Ctrl/Alt + letter is not text: it leaves the filter alone. Shift is just a capital.
    #[test]
    fn browser_ignores_ctrl_and_alt_letters() {
        let lib = Library::scan(&[]);
        let mut b = Browser::open(0);
        b.key(KeyCode::Char('u'), KeyModifiers::CONTROL, &lib);
        b.key(KeyCode::Char('x'), KeyModifiers::ALT, &lib);
        assert_eq!(b.query, "");
        b.key(KeyCode::Char('S'), KeyModifiers::SHIFT, &lib);
        assert_eq!(b.query, "S");
    }

    #[test]
    fn esc_quits_only_when_pressed_twice_quickly() {
        let ms = Duration::from_millis;
        let mut g = QuitGuard::default();
        assert!(!g.esc(ms(1000)), "one esc only arms");
        assert!(g.is_armed(ms(1100)));
        assert!(g.esc(ms(1200)), "a second esc quits");
        // Too slow: the second press arms again instead.
        let mut g = QuitGuard::default();
        assert!(!g.esc(ms(0)));
        assert!(!g.is_armed(ms(1600)), "the prompt runs out");
        assert!(!g.esc(ms(1700)));
        assert!(g.esc(ms(2000)));
        // Another key in between disarms (the play loop clears `armed`).
        let mut g = QuitGuard::default();
        assert!(!g.esc(ms(0)));
        g.armed = None;
        assert!(!g.esc(ms(100)));
    }

    #[test]
    fn unmapped_readout() {
        use yahaha::api::unmapped_text;
        assert_eq!(unmapped_text(0), "");
        assert_eq!(unmapped_text(0x01_B0_67_7F), "unmapped CC 103 = 127");
        assert_eq!(unmapped_text(0x01_BF_55_41), "unmapped CC 85 = 65 (ch 16)");
        assert_eq!(unmapped_text(0x01_99_24_5A), "unmapped note 36 = 90 (ch 10)");
    }
}

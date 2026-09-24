//! A mock session for the app shell, for running without MIDI or styles (`YAHAHA_MOCK=1`,
//! or when the engine can't start). It builds the engine's own `AppState` types
//! (yahaha::api) and behaves like `app/src/lib/api/mock.ts` (the browser-only
//! dev mock), from the same fixture: the band advances bar by bar, queued sections take
//! over at the bar (fills at the beat), chords change, faders wait for pickup. No audio,
//! no MIDI.

use std::time::Instant;

use yahaha::api::*;
use yahaha::fingering::Fingering;
use yahaha::launchkey::{self as lk, Action, Anim, Control, Level, Page};
use yahaha::parts::{self, FaderPage};

const FIXTURE: &str = include_str!("../../src/lib/api/mock-fixture.json");
const ROOT: &str = "/Users/me/Styles";
/// The MIDI sources the mock rig has: (name, the Launchkey DAW port).
const MOCK_SOURCES: [(&str, bool); 4] = [
    ("Launchkey 49 MK4 LKMK4 MIDI Out", false),
    ("Launchkey 49 MK4 LKMK4 DAW Out", true),
    ("TASCAM Model 16", false),
    ("IAC Driver Bus 1", false),
];
const MOCK_SOUND_FONTS: [&str; 3] = ["GeneralUser-GS.sf2", "FluidR3_GM.sf2", "MuseScore_General.sf2"];

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureStyle {
    id: usize,
    name: String,
    folder: String,
    file: String,
    tempo: f64,
    time_signature: [u8; 2],
    sections: Vec<String>,
    ots: usize,
    error: Option<String>,
}

#[derive(serde::Deserialize)]
struct Fixture {
    gm: Vec<String>,
    styles: Vec<FixtureStyle>,
}

const INTROS: [&str; 3] = ["Intro A", "Intro B", "Intro C"];
const MAINS: [&str; 4] = ["Main A", "Main B", "Main C", "Main D"];
const FILLS: [&str; 4] = ["Fill In AA", "Fill In BB", "Fill In CC", "Fill In DD"];
const BREAK: &str = "Fill In BA";
const ENDINGS: [&str; 3] = ["Ending A", "Ending B", "Ending C"];
const PROGRESSION: [&str; 12] = ["C", "Am7", "Fmaj7", "G7", "Em7", "A7", "Dm7", "G7sus4", "C/E", "F", "Fm6", "C"];
const NOTE_NAMES: [&str; 12] = ["C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"];
const STYLE_PARTS: [(&str, u8, u8, u8, bool, &str, u8); 8] = [
    ("Rhythm 1", 127, 0, 0, true, "drum kit 127/0/1", 100),
    ("Rhythm 2", 127, 0, 25, true, "drum kit 127/0/26", 100),
    ("Bass", 0, 0, 33, false, "Finger Bass (GM 34)", 96),
    ("Chord 1", 0, 112, 27, false, "≈ Clean Gtr  [Yamaha 0/112/28]", 80),
    ("Chord 2", 0, 112, 4, false, "≈ E.Piano 1  [Yamaha 0/112/5]", 76),
    ("Pad", 104, 0, 48, false, "≈ Strings  [Yamaha 104/0/49]", 70),
    ("Phrase 1", 0, 0, 61, false, "Brass Section (GM 62)", 88),
    ("Phrase 2", 0, 0, 73, false, "Flute (GM 74)", 84),
];
/// (program, on, volume, octave) for Right 1, Right 2, Right 3, Left, per OTS.
const OTS: [[(u8, bool, u8, i8); 4]; 4] = [
    [(0, true, 100, 0), (48, true, 70, 0), (61, false, 90, 0), (48, false, 80, 0)],
    [(4, true, 100, 0), (89, true, 60, 1), (61, false, 90, 0), (33, false, 90, -1)],
    [(16, true, 96, 0), (61, false, 90, 0), (56, false, 90, 0), (48, false, 80, 0)],
    [(65, true, 104, 0), (61, true, 80, 0), (56, false, 90, 0), (48, true, 70, 0)],
];

fn transpose_chord(name: &str, d: i8) -> String {
    let shift = |root: &str| match NOTE_NAMES.iter().position(|n| *n == root) {
        Some(i) => NOTE_NAMES[(i as i32 + d as i32).rem_euclid(12) as usize].to_string(),
        None => root.to_string(),
    };
    let root_len = |s: &str| if s.len() > 1 && matches!(&s[1..2], "#" | "b") { 2 } else { 1 };
    let (chord, bass) = match name.split_once('/') {
        Some((c, b)) => (c, Some(b)),
        None => (name, None),
    };
    let n = root_len(chord);
    let mut out = shift(&chord[..n]) + &chord[n..];
    if let Some(b) = bass {
        out += "/";
        out += &shift(b);
    }
    out
}

fn beats_per_bar([n, d]: [u8; 2]) -> u8 {
    if d == 8 && n % 3 == 0 { n / 3 } else { n }
}

/// Quarter notes per bar, as the engine's `quarters_per_bar`: 4 in 4/4, 3 in 3/4 and 6/8.
/// The mock's bars are this long, and `surface.clock` counts in quarter notes.
fn quarters_per_bar([n, d]: [u8; 2]) -> f64 {
    n as f64 * 4.0 / d.max(1) as f64
}

/// Where the hardware faders start (1-8, master), as app/src/lib/api/mock.ts has them.
const HW_FADERS: [u8; 9] = [100, 72, 100, 100, 0, 0, 0, 0, 100];

/// What moves the section anchor (docs/app-api.md `surface.clock`): the tempo, the bar
/// length, the section and where it started, running or not.
type SectionKey = (u64, u64, Option<String>, u32, bool);

fn sections_text(sections: &[String]) -> String {
    let letters = |names: &[&str]| -> String {
        names.iter().filter(|n| sections.iter().any(|s| s == *n)).map(|n| n.chars().last().unwrap()).collect()
    };
    let mut parts: Vec<String> = [("Main", letters(&MAINS)), ("Intro", letters(&INTROS)), ("Ending", letters(&ENDINGS)), ("Fill", letters(&FILLS))]
        .into_iter()
        .filter(|(_, l)| !l.is_empty())
        .map(|(k, l)| format!("{k} {l}"))
        .collect();
    if sections.iter().any(|s| s == BREAK) {
        parts.push("Break".into());
    }
    parts.join(" · ")
}

pub struct MockSession {
    pub state: AppState,
    gm: Vec<String>,
    styles: Vec<FixtureStyle>,
    library: LibraryList,
    clock: f64,
    section_start: u32,
    taps: Vec<f64>,
    now: f64,
    progression: usize,
    message_seq: u64,
    /// The hardware faders 1-8 and master, where they physically are (they never move:
    /// the mock has no Launchkey).
    hw_faders: [u8; 9],
    /// `surface.clock`: the section anchor (ms, quarter notes) and what it was taken for.
    section_anchor: (f64, f64),
    section_key: Option<SectionKey>,
    /// The free-running LED clock: at ms it read beats, moving on at tempo.
    led_anchor: (f64, f64, f64),
    /// The wall clock at the last `catch_up`.
    wall: Option<Instant>,
}

impl Default for MockSession {
    fn default() -> Self {
        Self::new()
    }
}

impl MockSession {
    /// A session mid-song (Main B, bar 12), like the frontend's demo mock.
    pub fn new() -> MockSession {
        let f: Fixture = serde_json::from_str(FIXTURE).expect("mock fixture");
        let library = LibraryList {
            revision: 1,
            entries: f
                .styles
                .iter()
                .map(|s| LibraryEntry {
                    id: s.id,
                    name: s.name.clone(),
                    folder: s.folder.clone(),
                    path: format!("{ROOT}/{}/{}", s.folder, s.file),
                    status: if s.error.is_some() { "error" } else { "ok" }.into(),
                    error: s.error.clone(),
                    tempo: s.error.is_none().then_some(s.tempo),
                    time_signature: s.error.is_none().then_some(s.time_signature),
                    sections: if s.error.is_some() { String::new() } else { sections_text(&s.sections) },
                    format: s.error.is_none().then(|| if s.file.to_lowercase().ends_with(".sty") { "SFF1" } else { "SFF2" }.to_string()),
                })
                .collect(),
            voices: voice_options(),
        };
        let gm = f.gm;
        let part = |i: usize, program: u8, on: bool| KeyboardPart {
            name: ["Right 1", "Right 2", "Right 3", "Left"][i].into(),
            channel: [1, 3, 4, 2][i],
            on,
            sounding: on,
            selected: i == 0,
            volume: 100,
            waiting: false,
            program,
            voice_name: gm[program as usize].clone(),
            plays_bass: false,
            octave: 0,
            fader: None,
        };
        let s0 = &f.styles[0];
        let state = AppState {
            version: 1,
            style: StyleState::default(),
            transport: TransportState {
                running: true,
                sync_start: false,
                sync_stop: false,
                sync_stop_available: true,
                auto_fill: false,
                stop_acmp: false,
                section: Some("Main B".into()),
                queued: None,
                pending_intro: None,
                main: 1,
                bar: 12,
                beat: 1,
                beats_per_bar: beats_per_bar(s0.time_signature),
                section_bars: Some(4),
                tempo: s0.tempo,
                lamps: vec![],
            },
            chord: ChordState {
                name: Some("Am7".into()),
                fingered: Some("Am7".into()),
                fingering: Fingering::FingeredOnBass,
                fingering_name: String::new(),
                upper: false,
                manual_bass: true,
                manual_bass_active: false,
                split: 54,
                split_name: String::new(),
                transpose_keyboard: 0,
                transpose_master: 0,
            },
            keyboard_parts: vec![part(0, 0, true), part(1, 48, true), part(2, 61, false), part(3, 48, false)],
            mixer: MixerState {
                fader_page: FaderPage::Panel,
                style_parts: STYLE_PARTS
                    .iter()
                    .enumerate()
                    .map(|(i, (name, msb, lsb, program, kit, label, volume))| StylePart {
                        name: name.to_string(),
                        channel: 9 + i as u8,
                        on: true,
                        muted_by_manual_bass: false,
                        volume: *volume,
                        waiting: false,
                        fader: None,
                        voice: Some(Voice { bank_msb: *msb, bank_lsb: *lsb, program: *program, kit: *kit, label: label.to_string() }),
                    })
                    .collect(),
                master: Some(100),
                master_waiting: false,
            },
            pads: PadsState { page: Page::Sections, page_name: String::new(), page_number: 1, page_count: 3, pads: vec![], connected: true, palette_leds: false },
            ots: OtsState { settings: vec![], applied: 0, link: false },
            library: LibraryStatus {
                revision: 1,
                count: library.entries.len(),
                position: 0,
                pending: 0,
                roots: vec![ROOT.to_string()],
                scanning: false,
            },
            surface: SurfaceState::default(),
            io: IoState {
                output_port: "yahaha".into(),
                inputs: vec!["Launchkey 49 MK4 LKMK4 MIDI Out".into(), "Launchkey 49 MK4 LKMK4 DAW Out (pads)".into()],
                synth: Some(SynthState {
                    sound_font: "GeneralUser-GS".into(),
                    device: "MacBook Pro Speakers".into(),
                    sample_rate: 48000,
                    buffer_frames: Some(64),
                    channels: 2,
                    output_pair: [1, 2],
                    muted: false,
                }),
                engine: EngineStats { realtime: true, wake_p99_us: 3, chord_p99_us: 15, midi_in_p99_us: 120 },
                last_control: 0,
                unmapped: String::new(),
                offline: false,
                sources: MOCK_SOURCES
                    .iter()
                    .map(|&(name, pads)| MidiSource { name: name.into(), listening: true, pads })
                    .collect(),
                all_inputs: true,
                sound_fonts: MOCK_SOUND_FONTS.iter().map(|f| f.to_string()).collect(),
                sound_font_file: Some(MOCK_SOUND_FONTS[0].into()),
                sound_font_loading: false,
            },
            preview: PreviewState::default(),
            // Mid-song: the left hand holds the Am7 it fingered.
            keyboard: KeyboardState {
                held: [45, 48, 52, 55].map(|note| HeldNote { note, zone: Zone::Left, parts: vec![] }).to_vec(),
                left_split: 54,
                chord_tones: vec![9, 0, 4, 7],
                chord_bass: Some(9),
                detection: [0, 54],
            },
            message: None,
        };
        let mut m = MockSession {
            state,
            gm,
            styles: f.styles,
            library,
            clock: 0.0,
            section_start: 0,
            taps: vec![],
            now: 0.0,
            progression: 0,
            message_seq: 0,
            hw_faders: HW_FADERS,
            section_anchor: (0.0, 0.0),
            section_key: None,
            led_anchor: (0.0, 0.0, 0.0), // anchored by the first `derive`
            wall: None,
        };
        m.set_style(0);
        m.state.ots.applied = 2;
        m.state.mixer.style_parts[5].volume = 58;
        m.state.mixer.style_parts[5].waiting = true;
        m.clock = 11.0 * m.bar_quarters();
        m.position();
        m.derive();
        m
    }

    /// The mock standing in for an engine that didn't start: it says so (an offline
    /// session, no Launchkey, no synth, and the reason in the status line) instead of
    /// passing for a connected rig.
    pub fn fallback(reason: impl Into<String>) -> MockSession {
        let mut m = MockSession::new();
        m.state.io.offline = true;
        m.state.io.inputs.clear();
        m.state.io.synth = None;
        m.state.pads.connected = false;
        // No synth, no master volume (as the engine without one).
        m.state.mixer.master = None;
        m.derive();
        m.message(reason, true);
        m
    }

    pub fn library(&self) -> &LibraryList {
        &self.library
    }

    fn has(&self, s: &str) -> bool {
        self.state.style.sections.iter().any(|x| x == s)
    }

    fn message(&mut self, text: impl Into<String>, error: bool) {
        self.message_seq += 1;
        self.state.message = Some(Message { seq: self.message_seq, text: text.into(), error });
    }

    /// Quarter notes per bar of the loaded style (`clock` counts quarter notes).
    fn bar_quarters(&self) -> f64 {
        quarters_per_bar(self.state.style.time_signature)
    }

    fn position(&mut self) {
        let qpb = self.bar_quarters();
        let t = &mut self.state.transport;
        let bpb = t.beats_per_bar.max(1);
        let bar = (self.clock / qpb).floor() as u32;
        t.bar = bar.saturating_sub(self.section_start) + 1;
        // Beats as `beats_per_bar` counts them (dotted quarters in 6/8).
        t.beat = ((self.clock.rem_euclid(qpb) / (qpb / bpb as f64)).floor() as u32).min(bpb as u32 - 1) + 1;
    }

    /// Move the mock's clock on to the wall clock (the first call only starts it); true if
    /// anything changed. The app's 60 Hz tick and the `state` command both call it, so the
    /// clock has been read whenever the state is.
    pub fn catch_up(&mut self) -> bool {
        let now = Instant::now();
        let ms = self.wall.map_or(0.0, |w| now.duration_since(w).as_secs_f64() * 1000.0);
        self.wall = Some(now);
        self.advance(ms)
    }

    /// The state with its clock read now (`surface.clock.atMs`), as the engine's
    /// `Session::state_now`.
    pub fn state_now(&self) -> AppState {
        let mut st = self.state.clone();
        st.surface.clock = st.surface.clock.at(self.now);
        st
    }

    /// Move the clock on by `ms` milliseconds; true if anything changed.
    pub fn advance(&mut self, ms: f64) -> bool {
        let before = self.state.clone();
        let mut left = ms;
        while left > 0.0 {
            self.step(left.min(20.0));
            left -= 20.0;
        }
        self.bump(&before)
    }

    /// Run a command; true if anything changed.
    pub fn send(&mut self, cmd: impl Into<AppCmd>) -> bool {
        let before = self.state.clone();
        self.cmd(cmd.into());
        self.bump(&before)
    }

    fn bump(&mut self, before: &AppState) -> bool {
        self.derive();
        // The clock as read when the state last changed: time passing alone changes nothing.
        let clock = &mut self.state.surface.clock;
        *clock = clock.at(before.surface.clock.at_ms);
        let changed = self.state != *before;
        if changed {
            self.state.version = before.version + 1;
            let clock = &mut self.state.surface.clock;
            *clock = clock.at(self.now);
        }
        changed
    }

    fn step(&mut self, ms: f64) {
        self.now += ms;
        if !self.state.transport.running {
            return;
        }
        let bpb = self.bar_quarters();
        let before = self.clock;
        self.clock += ms / 60000.0 * self.state.transport.tempo;
        if self.clock.floor() != before.floor() {
            self.on_beat();
        }
        if (self.clock / bpb).floor() != (before / bpb).floor() {
            self.on_bar((self.clock / bpb).floor() as u32);
        }
        if self.state.transport.running {
            self.position();
        }
    }

    fn on_beat(&mut self) {
        let qpb = self.bar_quarters();
        let t = &mut self.state.transport;
        if let Some(q) = t.queued.clone().filter(|q| FILLS.contains(&q.as_str())) {
            t.section = Some(q);
            t.queued = None;
            self.section_start = (self.clock / qpb).floor() as u32;
        }
    }

    fn on_bar(&mut self, bar: u32) {
        let t = &self.state.transport;
        let main = MAINS[t.main as usize];
        let section = t.section.clone();
        let queued = t.queued.clone();
        let played = bar.saturating_sub(self.section_start);
        if section.as_deref().is_some_and(|s| FILLS.contains(&s)) {
            let next = queued.filter(|q| MAINS.contains(&q.as_str())).unwrap_or(main.into());
            self.state.transport.queued = None;
            self.enter(&next, bar);
        } else if let Some(q) = queued {
            self.state.transport.queued = None;
            self.enter(&q, bar);
        } else if let Some(s) = section.filter(|s| !MAINS.contains(&s.as_str())) {
            let len = if INTROS.contains(&s.as_str()) || ENDINGS.contains(&s.as_str()) { 2 } else { 1 };
            if played >= len {
                if ENDINGS.contains(&s.as_str()) {
                    self.stop_band();
                    return;
                }
                self.enter(main, bar);
            }
        }
        // Demo: every 8 bars queue the next Main; a pattern volume change needs pickup.
        if bar % 8 == 6 && self.state.transport.queued.is_none() {
            let index = (self.state.transport.main + 1) % 4;
            self.cmd(AppCmd::Transport(TransportCmd::Main { index }));
        }
        if bar % 8 == 0 {
            let p = &mut self.state.mixer.style_parts[(bar / 8 % 8) as usize];
            p.volume = if bar % 16 == 0 { p.volume.saturating_sub(14).max(40) } else { (p.volume + 14).min(127) };
            p.waiting = true;
        }
        if bar % 2 == 0 {
            let chord = PROGRESSION[self.progression % PROGRESSION.len()];
            self.progression += 1;
            self.chord_arrives(chord);
        }
    }

    fn enter(&mut self, s: &str, bar: u32) {
        self.state.transport.section = Some(s.into());
        self.section_start = bar;
        if let Some(m) = MAINS.iter().position(|x| *x == s) {
            self.state.transport.main = m as u8;
            if self.state.ots.link && m < self.state.ots.settings.len() {
                self.recall_ots(m);
            }
        }
    }

    fn chord_arrives(&mut self, chord: &str) {
        let k = self.state.chord.transpose_keyboard;
        self.state.chord.name = Some(transpose_chord(chord, k));
        self.state.chord.fingered = Some(chord.into());
        if !self.state.transport.running && self.state.transport.sync_start {
            self.start_band();
        }
    }

    fn start_band(&mut self) {
        let intro = self.state.transport.pending_intro.map(|i| INTROS[i as usize]).filter(|s| self.has(s));
        let t = &mut self.state.transport;
        t.running = true;
        t.sync_start = false;
        t.section = Some(intro.unwrap_or(MAINS[t.main as usize]).into());
        t.pending_intro = None;
        t.queued = None;
        self.clock = 0.0;
        self.section_start = 0;
        self.position();
    }

    fn stop_band(&mut self) {
        let t = &mut self.state.transport;
        t.running = false;
        t.section = None;
        t.queued = None;
        t.bar = 1;
        t.beat = 1;
    }

    fn recall_ots(&mut self, n: usize) {
        let panel = self.state.mixer.fader_page == FaderPage::Panel;
        let setting = self.state.ots.settings[n].clone();
        for (p, o) in self.state.keyboard_parts.iter_mut().zip(&setting.parts) {
            if let Some(prog) = o.program {
                p.program = prog;
            }
            p.on = o.on;
            p.octave = o.octave;
            if p.volume != o.volume {
                p.waiting = panel;
            }
            p.volume = o.volume;
        }
        self.state.ots.applied = n as u8 + 1;
    }

    fn set_style(&mut self, id: usize) {
        let s = &self.styles[id];
        self.state.style = StyleState {
            id: s.id,
            path: format!("{ROOT}/{}/{}", s.folder, s.file),
            name: s.name.clone(),
            format: if s.file.ends_with(".sty") { "SFF1" } else { "SFF2" }.into(),
            tempo: s.tempo,
            time_signature: s.time_signature,
            sections: s.sections.clone(),
        };
        self.state.transport.tempo = s.tempo;
        self.state.transport.beats_per_bar = beats_per_bar(s.time_signature);
        self.state.ots.settings = OTS[..s.ots.min(4)]
            .iter()
            .enumerate()
            .map(|(i, parts)| OtsSetting {
                name: format!("OTS {}", i + 1),
                parts: parts
                    .iter()
                    .map(|(program, on, volume, octave)| OtsPart {
                        on: *on,
                        program: Some(*program),
                        voice_name: self.gm[*program as usize].clone(),
                        volume: *volume,
                        octave: *octave,
                    })
                    .collect(),
            })
            .collect();
        self.state.ots.applied = 0;
        self.state.library.position = self.library.entries.iter().position(|e| e.id == id).unwrap_or(0);
    }

    fn load_style(&mut self, id: usize) {
        let Some(s) = self.styles.get(id) else { return };
        if let Some(e) = &s.error {
            let text = format!("{}/{}: {e}", s.folder, s.file);
            self.message(text, true);
            return;
        }
        self.set_style(id);
        let style_page = self.state.mixer.fader_page == FaderPage::Style;
        for p in &mut self.state.mixer.style_parts {
            p.volume = 100;
            p.waiting = style_page;
        }
        let main = self.state.transport.main as usize;
        if self.state.ots.link && main < self.state.ots.settings.len() {
            self.recall_ots(main);
        }
        if self.state.transport.section.as_deref().is_some_and(|s| !self.has(s)) {
            self.state.transport.section = MAINS.iter().find(|m| self.has(m)).map(|m| m.to_string());
        }
        self.state.message = None;
    }

    /// The fields the engine computes from the others: names, flags, pads and lamps.
    fn derive(&mut self) {
        let st = &mut self.state;
        let c = &mut st.chord;
        c.fingering_name = if c.upper { "Fingered*".into() } else { c.fingering.name().into() };
        c.manual_bass_active = c.upper && c.manual_bass;
        c.split_name = note_name(c.split);
        let full = matches!(c.fingering, Fingering::FullKeyboard | Fingering::AiFullKeyboard);
        st.transport.sync_stop_available = c.upper || !full;
        // The keyboard strip: the split and where chord detection listens (no keys held).
        st.keyboard.left_split = c.split;
        st.keyboard.detection = if c.upper {
            [c.split.saturating_add(1).min(127), 127]
        } else if full {
            [0, 127]
        } else {
            [0, c.split]
        };
        // The mock's patterns: a Main is 4 bars, an Intro or Ending 2, a fill 1.
        st.transport.section_bars = st.transport.section.as_deref().map(|s| if s.starts_with("Main") { 4 } else if s.starts_with("Intro") || s.starts_with("Ending") { 2 } else { 1 });
        let mb = c.manual_bass_active;
        for (i, p) in st.keyboard_parts.iter_mut().enumerate() {
            p.plays_bass = i == 3 && mb;
            p.sounding = p.on || p.plays_bass;
            p.voice_name = if p.plays_bass { "Finger Bass".into() } else { self.gm[p.program as usize].clone() };
        }
        for (i, p) in st.mixer.style_parts.iter_mut().enumerate() {
            p.muted_by_manual_bass = i == 2 && mb;
        }
        // Where each part's hardware fader physically is (Panel faders 1-4, Style 1-8).
        for (p, hw) in st.keyboard_parts.iter_mut().zip(self.hw_faders) {
            p.fader = Some(hw);
        }
        for (p, hw) in st.mixer.style_parts.iter_mut().zip(self.hw_faders) {
            p.fader = Some(hw);
        }
        st.pads.page_name = st.pads.page.name().into();
        st.pads.page_number = st.pads.page as u8 + 1;
        st.transport.lamps = pads_for(st, Page::Sections);
        st.pads.pads = pads_for(st, st.pads.page);
        self.anchor_clocks();
        self.state.surface = self.surface();
    }

    /// Re-anchor `surface.clock` as the engine does: the section anchor when the tempo,
    /// the section or where it started changes (or the band starts or stops), the
    /// free-running LED clock when the tempo changes, carrying on from where it was.
    fn anchor_clocks(&mut self) {
        let t = &self.state.transport;
        let qpb = self.bar_quarters();
        let key = (t.tempo.to_bits(), qpb.to_bits(), t.section.clone(), self.section_start, t.running);
        if self.section_key.as_ref() != Some(&key) {
            let beats = if t.running { self.clock - self.section_start as f64 * qpb } else { 0.0 };
            self.section_anchor = (self.now, beats);
            self.section_key = Some(key);
        }
        let (ms, beats, tempo) = self.led_anchor;
        if tempo != t.tempo {
            self.led_anchor = (self.now, beats + (self.now - ms) * tempo / 60e3, t.tempo);
        }
    }

    /// The Launchkey beyond the pads, as the engine's `Session::surface` (src/session.rs)
    /// builds it, from the mock's state. No Shift: the mock has no hardware.
    fn surface(&self) -> SurfaceState {
        let st = &self.state;
        let page = st.pads.page;
        let styles = self.library.entries.len() > 1;
        let fader_page = st.mixer.fader_page;
        let mask = |bits: Vec<bool>| bits.iter().enumerate().fold(0u8, |m, (i, on)| m | (*on as u8) << i);
        let parts_on = mask(st.keyboard_parts.iter().map(|p| p.sounding).collect());
        let style_on = lk::style_lit(mask(st.mixer.style_parts.iter().map(|p| p.on).collect()), st.chord.manual_bass_active);
        let colours = lk::button_colours(page, styles, fader_page, parts_on, style_on);
        let act = |cc: u8, shift: bool| -> Option<AppCmd> {
            match lk::cc_control(cc, shift)? {
                Control::Page(d) => {
                    let to = page.step(d);
                    (to != page).then_some(AppCmd::Pads(PadsCmd::SetPadPage { page: to }))
                }
                Control::Act(Action::Style(_)) if !styles => None,
                Control::Act(a) => Some(a.into()),
            }
        };
        let mut controls = Vec::new();
        let mut push = |id: String, cc: u8, label: &str, action: Option<AppCmd>, shift: Option<(&str, Option<AppCmd>)>| {
            let label = if action.is_some() { label.to_string() } else { String::new() };
            let (shift_label, shift_action) = match shift {
                Some((l, a)) => (if a.is_some() { l.to_string() } else { String::new() }, a),
                None => (label.clone(), action.clone()),
            };
            let colour = colours.iter().find(|c| c.0 == cc).map(|c| c.1);
            let (rgb, level) = colour.map_or(((0, 0, 0), Level::Off), lk::palette_colour);
            controls.push(SurfaceControl {
                id,
                cc,
                label,
                action,
                shift_label,
                shift_action,
                rgb: [rgb.0, rgb.1, rgb.2],
                level,
                anim: Anim::Solid,
                colour,
            });
        };
        for (id, cc, label, shift_label) in [
            ("padBankUp", lk::PAD_UP_CC, "PAGE ▲", "LEFT"),
            ("padBankDown", lk::PAD_DOWN_CC, "PAGE ▼", "OTS LINK"),
            ("trackPrev", lk::TRACK_LEFT_CC, "◀ STYLE", ""),
            ("trackNext", lk::TRACK_RIGHT_CC, "STYLE ▶", ""),
            ("play", lk::PLAY_CC, "PLAY", ""),
            ("stop", lk::STOP_CC, "STOP", ""),
            ("scene", lk::SCENE_CC, "TEMPO +", ""),
            ("function", lk::FUNCTION_CC, "TEMPO -", ""),
        ] {
            let (a, sa) = (act(cc, false), act(cc, true));
            let shift = (sa != a).then_some((shift_label, sa));
            push(id.to_string(), cc, label, a, shift);
        }
        // The buttons under faders 1-8: Panel = Right 1-3 and Left on/off (Shift: select),
        // Style = the Style parts' mute.
        for i in 0..8u8 {
            let cc = lk::FADER_BTN_CC.start() + i;
            let id = format!("faderButton{}", i + 1);
            match fader_page {
                FaderPage::Panel if (i as usize) < parts::COUNT => {
                    let p = i as usize;
                    let shift = (lk::SELECT_LABELS[p], Some(AppCmd::Parts(PartsCmd::SelectPart { part: i })));
                    push(id, cc, lk::PART_LABELS[p], Some(AppCmd::Parts(PartsCmd::TogglePart { part: i })), Some(shift));
                }
                FaderPage::Panel => push(id, cc, "", None, None),
                FaderPage::Style => {
                    let name = STYLE_PART_NAMES[i as usize].to_uppercase();
                    push(id, cc, &name, Some(AppCmd::Mixer(MixerCmd::ToggleStylePart { part: i })), None);
                }
            }
        }
        let master = match fader_page {
            FaderPage::Panel => "PANEL",
            FaderPage::Style => "STYLE",
        };
        push("masterButton".into(), *lk::FADER_BTN_CC.end(), master, Some(AppCmd::Mixer(MixerCmd::ToggleFaderPage)), None);

        // The faders: the parts they control on this page, and where they physically are.
        let mut faders: Vec<SurfaceFader> = (0..8u8)
            .map(|i| {
                let p = i as usize;
                let position = Some(self.hw_faders[p]);
                match fader_page {
                    FaderPage::Panel if p < parts::COUNT => SurfaceFader {
                        label: lk::PART_LABELS[p].to_string(),
                        value: Some(st.keyboard_parts[p].volume),
                        waiting: st.keyboard_parts[p].waiting,
                        position,
                        set: Some(AppCmd::Parts(PartsCmd::SetPartVolume { part: i, volume: 0 })),
                    },
                    FaderPage::Panel => SurfaceFader { position, ..SurfaceFader::default() },
                    FaderPage::Style => SurfaceFader {
                        label: STYLE_PART_NAMES[p].to_uppercase(),
                        value: Some(st.mixer.style_parts[p].volume),
                        waiting: st.mixer.style_parts[p].waiting,
                        position,
                        set: Some(AppCmd::Mixer(MixerCmd::SetStylePartVolume { part: i, volume: 0 })),
                    },
                }
            })
            .collect();
        let master_pos = Some(self.hw_faders[8]);
        faders.push(match st.mixer.master {
            Some(v) => SurfaceFader {
                label: "MASTER".into(),
                value: Some(v),
                waiting: st.mixer.master_waiting,
                position: master_pos,
                set: Some(AppCmd::Mixer(MixerCmd::SetMasterVolume { volume: 0 })),
            },
            None => SurfaceFader { position: master_pos, ..SurfaceFader::default() },
        });

        let t = &st.transport;
        SurfaceState {
            shift: false,
            controls,
            faders,
            track_prev: self.neighbour(-1),
            track_next: self.neighbour(1),
            clock: ClockState {
                at_ms: 0.0,
                running: t.running,
                tempo: t.tempo,
                beats_per_bar: self.bar_quarters(),
                bar: 1,
                beat: 1,
                phase: 0.0,
                section_anchor_ms: self.section_anchor.0,
                section_anchor_beats: self.section_anchor.1,
                led_anchor_ms: self.led_anchor.0,
                led_anchor_beats: self.led_anchor.1,
            }
            .at(self.now),
        }
    }

    /// The style `StepStyle { delta }` would load, if it goes anywhere: the engine's
    /// `neighbour` and `Library::step` (src/session.rs, src/library.rs, private there)
    /// over the mock's library order, skipping entries that don't load, wrapping.
    fn neighbour(&self, delta: i8) -> Option<Neighbour> {
        let lib = &self.library.entries;
        let n = lib.len();
        let cur = self.state.library.position;
        if cur >= n {
            return None;
        }
        let mut pos = cur;
        for _ in 1..n {
            pos = if delta > 0 { (pos + 1) % n } else { (pos + n - 1) % n };
            let e = &lib[pos];
            if e.status != "error" {
                return Some(Neighbour { id: e.id, name: e.name.clone(), path: e.path.clone() });
            }
        }
        None
    }

    fn cmd(&mut self, cmd: AppCmd) {
        let running = self.state.transport.running;
        let vol = |v: u8| v.min(127);
        match cmd {
            AppCmd::Transport(TransportCmd::StartStop) => {
                if running {
                    self.stop_band()
                } else {
                    self.start_band()
                }
            }
            AppCmd::Transport(TransportCmd::Stop) => {
                if running {
                    self.stop_band()
                }
            }
            AppCmd::Transport(TransportCmd::Intro { index }) => {
                let id = INTROS[index.min(2) as usize];
                if self.has(id) {
                    let t = &mut self.state.transport;
                    if running {
                        t.queued = Some(id.into());
                    } else {
                        t.pending_intro = if t.pending_intro == Some(index) { None } else { Some(index) };
                    }
                }
            }
            AppCmd::Transport(TransportCmd::Main { index }) => {
                let i = index.min(3);
                let m = MAINS[i as usize];
                if !self.has(m) {
                    return;
                }
                let t = &mut self.state.transport;
                if !running {
                    t.main = i;
                } else if t.section.as_deref() == Some(m) {
                    t.queued = Some(FILLS[i as usize].into());
                } else if t.auto_fill && t.main != i {
                    t.queued = Some(FILLS[t.main as usize].into());
                    t.main = i;
                } else {
                    t.queued = Some(m.into());
                }
            }
            AppCmd::Transport(TransportCmd::Break) => {
                if running && self.has(BREAK) {
                    self.state.transport.queued = Some(BREAK.into());
                }
            }
            AppCmd::Transport(TransportCmd::Ending { index }) => {
                let id = ENDINGS[index.min(2) as usize];
                if running && self.has(id) {
                    self.state.transport.queued = Some(id.into());
                }
            }
            AppCmd::Transport(TransportCmd::ToggleSyncStart) => {
                if running {
                    self.stop_band();
                }
                self.state.transport.sync_start = !self.state.transport.sync_start;
            }
            AppCmd::Transport(TransportCmd::ToggleSyncStop) => {
                if self.state.transport.sync_stop_available {
                    self.state.transport.sync_stop = !self.state.transport.sync_stop;
                } else {
                    self.message("Sync Stop is not available with the Full Keyboard fingering types", true);
                }
            }
            AppCmd::Transport(TransportCmd::ToggleAutoFill) => self.state.transport.auto_fill = !self.state.transport.auto_fill,
            AppCmd::Transport(TransportCmd::ToggleStopAcmp) => self.state.transport.stop_acmp = !self.state.transport.stop_acmp,
            AppCmd::Transport(TransportCmd::TapTempo) => {
                let now = self.now;
                self.taps.retain(|x| now - x < 2000.0);
                self.taps.push(now);
                if self.taps.len() > 4 {
                    self.taps.remove(0);
                }
                if self.taps.len() >= 2 {
                    let avg = (self.taps[self.taps.len() - 1] - self.taps[0]) / (self.taps.len() - 1) as f64;
                    if avg > 0.0 {
                        self.state.transport.tempo = (60000.0 / avg).round().clamp(30.0, 300.0);
                    }
                }
            }
            AppCmd::Transport(TransportCmd::TempoUp) => self.state.transport.tempo = (self.state.transport.tempo + 1.0).min(300.0),
            AppCmd::Transport(TransportCmd::TempoDown) => self.state.transport.tempo = (self.state.transport.tempo - 1.0).max(30.0),
            AppCmd::Mixer(MixerCmd::ToggleStylePart { part }) => {
                if let Some(p) = self.state.mixer.style_parts.get_mut(part as usize) {
                    p.on = !p.on;
                }
            }
            AppCmd::Mixer(MixerCmd::SetStylePartVolume { part, volume }) => {
                if let Some(p) = self.state.mixer.style_parts.get_mut(part as usize) {
                    p.volume = vol(volume);
                    p.waiting = false;
                }
            }
            AppCmd::Chord(ChordCmd::SetFingering { fingering }) => self.state.chord.fingering = fingering,
            AppCmd::Chord(ChordCmd::NextFingering) => {
                let i = Fingering::ALL.iter().position(|f| *f == self.state.chord.fingering).unwrap_or(0);
                self.state.chord.fingering = Fingering::ALL[(i + 1) % Fingering::ALL.len()];
            }
            AppCmd::Chord(ChordCmd::SetUpper { on }) => self.set_upper(on),
            AppCmd::Chord(ChordCmd::ToggleUpper) => self.set_upper(!self.state.chord.upper),
            AppCmd::Chord(ChordCmd::SetManualBass { on }) => self.set_manual_bass(on),
            AppCmd::Chord(ChordCmd::ToggleManualBass) => self.set_manual_bass(!self.state.chord.manual_bass),
            AppCmd::Chord(ChordCmd::SetSplit { note }) => self.state.chord.split = note.clamp(24, 96),
            AppCmd::Chord(ChordCmd::MoveSplit { delta }) => {
                self.state.chord.split = (self.state.chord.split as i16 + delta as i16).clamp(24, 96) as u8;
            }
            AppCmd::Chord(ChordCmd::SetTranspose { keyboard, master }) => {
                self.state.chord.transpose_keyboard = keyboard.clamp(-12, 12);
                self.state.chord.transpose_master = master.clamp(-12, 12);
            }
            AppCmd::Chord(ChordCmd::StepTranspose { keyboard, master }) => {
                let c = &mut self.state.chord;
                c.transpose_keyboard = (c.transpose_keyboard + keyboard).clamp(-12, 12);
                c.transpose_master = (c.transpose_master + master).clamp(-12, 12);
            }
            AppCmd::Chord(ChordCmd::ResetTranspose) => {
                self.state.chord.transpose_keyboard = 0;
                self.state.chord.transpose_master = 0;
            }
            AppCmd::Parts(PartsCmd::SetPartOn { part, on }) => self.set_part_on(part, on),
            AppCmd::Parts(PartsCmd::TogglePart { part }) => {
                let on = self.state.keyboard_parts.get(part as usize).is_some_and(|p| !p.on);
                self.set_part_on(part, on);
            }
            AppCmd::Parts(PartsCmd::SelectPart { part }) => {
                for (i, p) in self.state.keyboard_parts.iter_mut().enumerate() {
                    p.selected = i == part as usize;
                }
            }
            AppCmd::Parts(PartsCmd::SetPartVoice { part, program }) => {
                if let Some(p) = self.state.keyboard_parts.get_mut(part as usize) {
                    p.program = program & 127;
                }
            }
            AppCmd::Parts(PartsCmd::StepVoice { delta }) => {
                if let Some(p) = self.state.keyboard_parts.iter_mut().find(|p| p.selected) {
                    p.program = (p.program as i16 + delta as i16).rem_euclid(128) as u8;
                }
            }
            AppCmd::Parts(PartsCmd::SetPartVolume { part, volume }) => {
                if let Some(p) = self.state.keyboard_parts.get_mut(part as usize) {
                    p.volume = vol(volume);
                    p.waiting = false;
                }
            }
            AppCmd::Parts(PartsCmd::SetPartOctave { part, octave }) => {
                if let Some(p) = self.state.keyboard_parts.get_mut(part as usize) {
                    p.octave = octave.clamp(-2, 2);
                }
            }
            AppCmd::Mixer(MixerCmd::SetFaderPage { page }) => self.set_fader_page(page),
            AppCmd::Mixer(MixerCmd::ToggleFaderPage) => {
                let page = if self.state.mixer.fader_page == FaderPage::Panel { FaderPage::Style } else { FaderPage::Panel };
                self.set_fader_page(page);
            }
            AppCmd::Pads(PadsCmd::SetPadPage { page }) => self.state.pads.page = page,
            AppCmd::Pads(PadsCmd::CyclePadPage { delta }) => {
                let i = self.state.pads.page as i8;
                self.state.pads.page = Page::ALL[(i + delta).rem_euclid(3) as usize];
            }
            AppCmd::Mixer(MixerCmd::SetMasterVolume { volume }) => {
                if self.state.io.synth.is_some() {
                    self.state.mixer.master = Some(vol(volume));
                    self.state.mixer.master_waiting = false;
                }
            }
            AppCmd::Ots(OtsCmd::RecallOts { index }) => {
                if (index as usize) < self.state.ots.settings.len() {
                    self.recall_ots(index as usize);
                }
            }
            AppCmd::Ots(OtsCmd::SetOtsLink { on }) => self.state.ots.link = on,
            AppCmd::Ots(OtsCmd::ToggleOtsLink) => self.state.ots.link = !self.state.ots.link,
            AppCmd::Library(LibraryCmd::LoadStyle { id }) => self.load_style(id),
            AppCmd::Library(LibraryCmd::LoadStylePath { path }) => match self.library.entries.iter().find(|e| e.path == path).map(|e| e.id) {
                Some(id) => self.load_style(id),
                None => self.message(format!("{path}: not found"), true),
            },
            AppCmd::Library(LibraryCmd::StepStyle { delta }) => {
                let n = self.library.entries.len() as i64;
                let mut i = self.state.library.position as i64;
                for _ in 0..n {
                    i = (i + delta as i64).rem_euclid(n);
                    if self.library.entries[i as usize].status == "ok" {
                        break;
                    }
                }
                let id = self.library.entries[i as usize].id;
                self.load_style(id);
            }
            AppCmd::Mixer(MixerCmd::SetSynthMuted { on }) => {
                if let Some(s) = &mut self.state.io.synth {
                    s.muted = on;
                }
            }
            AppCmd::Mixer(MixerCmd::ToggleSynthMute) => {
                if let Some(s) = &mut self.state.io.synth {
                    s.muted = !s.muted;
                }
            }
            AppCmd::Settings(SettingsCmd::SetAudioOutput { first }) => {
                if let Some(s) = &mut self.state.io.synth {
                    if (first as u32) + 1 < s.channels {
                        s.output_pair = [first + 1, first + 2];
                    }
                }
            }
            AppCmd::Settings(SettingsCmd::NextAudioOutput) => {
                if let Some(s) = &mut self.state.io.synth {
                    let next = s.output_pair[1] + 1;
                    s.output_pair = if (next as u32) < s.channels { [next, next + 1] } else { [1, 2] };
                }
            }
            AppCmd::System(SystemCmd::Panic) => {
                self.stop_band();
                self.message("All notes off", false);
            }
            AppCmd::System(SystemCmd::ClearMessage) => self.state.message = None,
            // Without a clock of its own for the bar line, the mock loads at once.
            AppCmd::Library(LibraryCmd::QueueStyle { id }) => self.load_style(id),
            AppCmd::Preview(PreviewCmd::AuditionStyle { id }) => {
                if self.state.transport.running {
                    self.message("Stop the band to preview a style", true);
                } else if id < self.styles.len() {
                    self.state.preview.audition = Some(AuditionState { id, bar: 1, bars: 4, chord: Some("C".into()) });
                }
            }
            AppCmd::Preview(PreviewCmd::StopAudition) => self.state.preview.audition = None,
            AppCmd::Library(LibraryCmd::RescanLibrary) => self.message("Style folders rescanned", false),
            AppCmd::Settings(SettingsCmd::SetSoundFont { file }) => {
                if self.state.io.sound_fonts.contains(&file) {
                    if let Some(s) = self.state.io.synth.as_mut() {
                        s.sound_font = file.trim_end_matches(".sf2").to_string();
                    }
                    self.state.io.sound_font_file = Some(file);
                } else {
                    self.message(format!("no SoundFont {file} in the SoundFont folder"), true);
                }
            }
            AppCmd::Settings(SettingsCmd::SetMidiInputs { all, names }) => {
                let io = &mut self.state.io;
                io.all_inputs = all;
                for src in &mut io.sources {
                    src.listening = src.pads || all || names.iter().any(|n| !n.is_empty() && src.name.contains(n.as_str()));
                }
                io.inputs = io.sources.iter().filter(|s| s.listening).map(|s| if s.pads { format!("{} (pads)", s.name) } else { s.name.clone() }).collect();
            }
            AppCmd::Settings(SettingsCmd::SetPaletteLeds { on }) => self.state.pads.palette_leds = on,
        }
    }

    fn set_upper(&mut self, on: bool) {
        self.state.chord.upper = on;
        if on {
            self.state.chord.manual_bass = true;
        }
    }

    fn set_manual_bass(&mut self, on: bool) {
        if self.state.chord.upper {
            self.state.chord.manual_bass = on;
        } else {
            self.message("Manual Bass is only available with chord detection Upper", true);
        }
    }

    fn set_part_on(&mut self, part: u8, on: bool) {
        let c = &self.state.chord;
        if part == 3 && !on && c.upper && c.manual_bass {
            self.message("Left plays the bass while Manual Bass is on", true);
        } else if let Some(p) = self.state.keyboard_parts.get_mut(part as usize) {
            p.on = on;
        }
    }

    fn set_fader_page(&mut self, page: FaderPage) {
        if page == self.state.mixer.fader_page {
            return;
        }
        self.state.mixer.fader_page = page;
        // The hardware faders are wherever they were: every level on the new page waits.
        match page {
            FaderPage::Panel => self.state.keyboard_parts.iter_mut().for_each(|p| p.waiting = true),
            FaderPage::Style => self.state.mixer.style_parts.iter_mut().for_each(|p| p.waiting = true),
        }
    }
}

// --- Pad lights: a port of `looks()` in src/launchkey.rs --------------------------------

const C_INTRO: [u8; 3] = [127, 95, 0];
const C_MAIN: [u8; 3] = [0, 127, 16];
const C_ENDING: [u8; 3] = [127, 0, 0];
const C_BREAK: [u8; 3] = [90, 0, 127];
const C_SYNC: [u8; 3] = [127, 45, 0];
const C_FILL: [u8; 3] = [0, 45, 127];
const C_TAP: [u8; 3] = [100, 100, 100];
const C_STOPSYNC: [u8; 3] = [0, 110, 110];
const C_RUN: [u8; 3] = [0, 127, 0];
const C_IDLE: [u8; 3] = [127, 0, 0];
const C_CHORD: [u8; 3] = [0, 100, 127];
const C_OTS: [u8; 3] = [127, 0, 70];

fn pad(note: u8, label: &str, key: &str, action: Option<AppCmd>, (rgb, level, anim): ([u8; 3], Level, Anim)) -> Pad {
    Pad { note, label: label.into(), key: key.into(), rgb, level, anim, action, palette: None }
}

fn toggle(on: bool, rgb: [u8; 3]) -> ([u8; 3], Level, Anim) {
    (rgb, if on { Level::Bright } else { Level::Dim }, Anim::Solid)
}

fn pads_for(s: &AppState, page: Page) -> Vec<Pad> {
    let t = &s.transport;
    let has = |id: &str| s.style.sections.iter().any(|x| x == id);
    let is = |o: &Option<String>, id: &str| o.as_deref() == Some(id);
    let sec = |id: &str, rgb| {
        if !has(id) {
            (rgb, Level::Off, Anim::Solid)
        } else if is(&t.queued, id) {
            (rgb, Level::Bright, Anim::Flash)
        } else if is(&t.section, id) {
            (rgb, Level::Bright, Anim::Solid)
        } else {
            (rgb, Level::Dim, Anim::Solid)
        }
    };
    let page_pad = |note, label: &str, key: &str, action, available: bool, on: bool| {
        let rgb = if page == Page::ChordSetup { C_CHORD } else { C_OTS };
        let level = if !available { Level::Off } else if on { Level::Bright } else { Level::Dim };
        pad(note, label, key, action, (rgb, level, Anim::Solid))
    };
    match page {
        Page::Sections => {
            let main = |i: usize| {
                let (id, fill) = (MAINS[i], FILLS[i]);
                if !has(id) {
                    (C_MAIN, Level::Off, Anim::Solid)
                } else if is(&t.queued, id) || is(&t.queued, fill) || is(&t.section, fill) {
                    (C_MAIN, Level::Bright, Anim::Flash)
                } else if is(&t.section, id) || (t.main as usize == i && !t.section.as_deref().is_some_and(|x| MAINS.contains(&x))) {
                    (C_MAIN, Level::Bright, Anim::Solid)
                } else {
                    (C_MAIN, Level::Dim, Anim::Solid)
                }
            };
            let intro = |i: usize| {
                if t.pending_intro == Some(i as u8) && has(INTROS[i]) {
                    (C_INTRO, Level::Bright, Anim::Pulse)
                } else {
                    sec(INTROS[i], C_INTRO)
                }
            };
            vec![
                pad(96, "INTRO 1", "q", Some(AppCmd::Transport(TransportCmd::Intro { index: 0 })), intro(0)),
                pad(97, "INTRO 2", "w", Some(AppCmd::Transport(TransportCmd::Intro { index: 1 })), intro(1)),
                pad(98, "INTRO 3", "e", Some(AppCmd::Transport(TransportCmd::Intro { index: 2 })), intro(2)),
                pad(99, "SYNC ST", "y", Some(AppCmd::Transport(TransportCmd::ToggleSyncStart)), if t.sync_start { (C_SYNC, Level::Bright, Anim::Pulse) } else { toggle(false, C_SYNC) }),
                pad(100, "ENDING 1", "i", Some(AppCmd::Transport(TransportCmd::Ending { index: 0 })), sec(ENDINGS[0], C_ENDING)),
                pad(101, "ENDING 2", "o", Some(AppCmd::Transport(TransportCmd::Ending { index: 1 })), sec(ENDINGS[1], C_ENDING)),
                pad(102, "ENDING 3", "p", Some(AppCmd::Transport(TransportCmd::Ending { index: 2 })), sec(ENDINGS[2], C_ENDING)),
                pad(103, "AUTOFILL", "u", Some(AppCmd::Transport(TransportCmd::ToggleAutoFill)), toggle(t.auto_fill, C_FILL)),
                pad(112, "MAIN A", "1", Some(AppCmd::Transport(TransportCmd::Main { index: 0 })), main(0)),
                pad(113, "MAIN B", "2", Some(AppCmd::Transport(TransportCmd::Main { index: 1 })), main(1)),
                pad(114, "MAIN C", "3", Some(AppCmd::Transport(TransportCmd::Main { index: 2 })), main(2)),
                pad(115, "MAIN D", "4", Some(AppCmd::Transport(TransportCmd::Main { index: 3 })), main(3)),
                pad(116, "BREAK", "g", Some(AppCmd::Transport(TransportCmd::Break)), sec(BREAK, C_BREAK)),
                pad(117, "TAP", "t", Some(AppCmd::Transport(TransportCmd::TapTempo)), toggle(t.running && t.beat == 1, C_TAP)),
                pad(118, "SYNC STP", "j", Some(AppCmd::Transport(TransportCmd::ToggleSyncStop)), toggle(t.sync_stop, C_STOPSYNC)),
                pad(119, if t.running { "START" } else { "STOP" }, "spc", Some(AppCmd::Transport(TransportCmd::StartStop)), (if t.running { C_RUN } else { C_IDLE }, Level::Bright, Anim::Solid)),
            ]
        }
        Page::ChordSetup => {
            const LABELS: [&str; 7] = ["SINGLE", "FINGERED", "ON BASS", "MULTI", "AI FING", "FULL KBD", "AI FULL"];
            let c = &s.chord;
            let mut v: Vec<Pad> = Fingering::ALL
                .iter()
                .enumerate()
                .map(|(i, f)| page_pad(96 + i as u8, LABELS[i], "pad", Some(AppCmd::Chord(ChordCmd::SetFingering { fingering: *f })), true, c.fingering == *f))
                .collect();
            v.extend([
                page_pad(103, "UPPER", "d", Some(AppCmd::Chord(ChordCmd::ToggleUpper)), true, c.upper),
                page_pad(112, "MAN BASS", "D", Some(AppCmd::Chord(ChordCmd::ToggleManualBass)), c.upper, c.manual_bass),
                page_pad(113, "STOP ACMP", "h", Some(AppCmd::Transport(TransportCmd::ToggleStopAcmp)), true, t.stop_acmp),
                page_pad(114, "SPLIT -", "[", Some(AppCmd::Chord(ChordCmd::MoveSplit { delta: -1 })), true, false),
                page_pad(115, "SPLIT +", "]", Some(AppCmd::Chord(ChordCmd::MoveSplit { delta: 1 })), true, false),
                page_pad(116, "KBD TR -", ";", Some(AppCmd::Chord(ChordCmd::StepTranspose { keyboard: -1, master: 0 })), true, c.transpose_keyboard < 0),
                page_pad(117, "KBD TR +", "'", Some(AppCmd::Chord(ChordCmd::StepTranspose { keyboard: 1, master: 0 })), true, c.transpose_keyboard > 0),
                page_pad(118, "TR RESET", "/", Some(AppCmd::Chord(ChordCmd::ResetTranspose)), true, c.transpose_keyboard != 0 || c.transpose_master != 0),
                page_pad(119, "", "", None, false, false),
            ]);
            v
        }
        Page::OtsParts => {
            let n = s.ots.settings.len();
            let mut v: Vec<Pad> = (0..4)
                .map(|i| page_pad(96 + i as u8, &format!("OTS {}", i + 1), &format!("⇧{}", i + 1), Some(AppCmd::Ots(OtsCmd::RecallOts { index: i as u8 })), i < n, s.ots.applied as usize == i + 1))
                .collect();
            v.extend([
                page_pad(100, "OTS LINK", "F10", Some(AppCmd::Ots(OtsCmd::ToggleOtsLink)), true, s.ots.link),
                page_pad(101, "", "", None, false, false),
                page_pad(102, "VOICE -", "9", Some(AppCmd::Parts(PartsCmd::StepVoice { delta: -1 })), true, false),
                page_pad(103, "VOICE +", "0", Some(AppCmd::Parts(PartsCmd::StepVoice { delta: 1 })), true, false),
            ]);
            for (i, (label, key)) in [("RIGHT 1", "5"), ("RIGHT 2", "6"), ("RIGHT 3", "7"), ("LEFT", "8/l")].iter().enumerate() {
                v.push(page_pad(112 + i as u8, label, key, Some(AppCmd::Parts(PartsCmd::TogglePart { part: i as u8 })), true, s.keyboard_parts[i].on));
            }
            for (i, label) in ["EDIT R1", "EDIT R2", "EDIT R3", "EDIT L"].iter().enumerate() {
                v.push(page_pad(116 + i as u8, label, &format!("F{}", i + 1), Some(AppCmd::Parts(PartsCmd::SelectPart { part: i as u8 })), true, s.keyboard_parts[i].selected));
            }
            v
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar_ms(m: &MockSession) -> f64 {
        60000.0 / m.state.transport.tempo * m.state.transport.beats_per_bar as f64
    }

    #[test]
    fn a_queued_main_takes_over_at_the_next_bar() {
        let mut m = MockSession::new();
        m.advance(bar_ms(&m) * 0.1);
        m.send(TransportCmd::Main { index: 2 });
        assert_eq!(m.state.transport.queued.as_deref(), Some("Main C"));
        m.advance(bar_ms(&m));
        assert_eq!(m.state.transport.section.as_deref(), Some("Main C"));
    }

    #[test]
    fn pressing_the_playing_main_queues_its_fill_and_the_lamp_flashes() {
        let mut m = MockSession::new();
        m.send(TransportCmd::Main { index: 1 });
        assert_eq!(m.state.transport.queued.as_deref(), Some("Fill In BB"));
        let lamp = m.state.transport.lamps.iter().find(|p| p.note == 113).unwrap();
        assert_eq!((lamp.level, lamp.anim), (Level::Bright, Anim::Flash));
    }

    #[test]
    fn versions_change_only_with_the_state() {
        let mut m = MockSession::new();
        let v = m.state.version;
        assert!(!m.send(OtsCmd::SetOtsLink { on: false }));
        assert_eq!(m.state.version, v);
        assert!(m.send(OtsCmd::ToggleOtsLink));
        assert_eq!(m.state.version, v + 1);
    }

    #[test]
    fn state_has_the_documented_json() {
        let v = serde_json::to_value(&MockSession::new().state).unwrap();
        assert_eq!(v["transport"]["section"], "Main B");
        assert_eq!(v["chord"]["fingering"], "fingeredOnBass");
        assert_eq!(v["chord"]["splitName"], "F#2");
        assert_eq!(v["pads"]["page"], "sections");
        assert_eq!(v["mixer"]["faderPage"], "panel");
        assert_eq!(v["transport"]["lamps"][8]["action"]["type"], "main");
        assert_eq!(v["keyboardParts"].as_array().unwrap().len(), 4);
    }

    /// Key paths and value kinds, as app/src/lib/api/shape.ts computes them.
    fn shape(v: &serde_json::Value, path: &str, out: &mut std::collections::HashMap<String, &'static str>) {
        use serde_json::Value;
        match v {
            Value::Array(a) => {
                if let Some(x) = a.first() {
                    shape(x, &format!("{path}[]"), out)
                }
            }
            Value::Object(o) => {
                for (k, x) in o {
                    let p = if path.is_empty() { k.clone() } else { format!("{path}.{k}") };
                    let kind = match x {
                        Value::Null => "null",
                        Value::Array(_) => "array",
                        Value::Bool(_) => "boolean",
                        Value::Number(_) => "number",
                        Value::String(_) => "string",
                        Value::Object(_) => "object",
                    };
                    out.insert(p.clone(), kind);
                    shape(x, &p, out);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn state_and_library_match_the_recorded_engine_shape() {
        let recorded: serde_json::Value = serde_json::from_str(include_str!("../../src/lib/api/engine-shape.json")).unwrap();
        let mut m = MockSession::new();
        m.send(TransportCmd::Intro { index: 0 });
        for (key, value) in [("state", serde_json::to_value(&m.state).unwrap()), ("library", serde_json::to_value(m.library()).unwrap())] {
            let mut have = std::collections::HashMap::new();
            shape(&value, "", &mut have);
            for line in recorded[key].as_array().unwrap() {
                let (path, kind) = line.as_str().unwrap().split_once(": ").unwrap();
                let got = have.get(path).unwrap_or_else(|| panic!("{key}: missing {path}"));
                assert!(kind == "null" || *got == "null" || *got == kind, "{key}: {path} engine {kind}, mock {got}");
            }
        }
    }

    #[test]
    fn the_surface_has_every_control_and_fader_on_both_fader_pages() {
        let mut m = MockSession::new();
        let ids = |m: &MockSession| m.state.surface.controls.iter().map(|c| c.id.clone()).collect::<Vec<_>>();
        let labels = |m: &MockSession| m.state.surface.controls.iter().map(|c| c.label.clone()).collect::<Vec<_>>();
        let faders = |m: &MockSession| m.state.surface.faders.iter().map(|f| f.label.clone()).collect::<Vec<_>>();
        let mut want: Vec<String> = ["padBankUp", "padBankDown", "trackPrev", "trackNext", "play", "stop", "scene", "function"].map(String::from).to_vec();
        want.extend((1..=8).map(|i| format!("faderButton{i}")));
        want.push("masterButton".into());
        assert_eq!(ids(&m), want);
        assert_eq!(m.state.surface.faders.len(), 9);
        assert!(!m.state.surface.shift);

        // Panel page, pad page 1: no Pad Bank ▲.
        let s = &m.state.surface;
        assert_eq!(
            labels(&m),
            ["", "PAGE ▼", "◀ STYLE", "STYLE ▶", "PLAY", "STOP", "TEMPO +", "TEMPO -", "RIGHT 1", "RIGHT 2", "RIGHT 3", "LEFT", "", "", "", "", "PANEL"]
        );
        assert_eq!((s.controls[0].shift_label.as_str(), s.controls[1].shift_label.as_str()), ("LEFT", "OTS LINK"));
        assert_eq!(s.controls[0].action, None);
        assert_eq!(s.controls[0].shift_action, Some(AppCmd::Parts(PartsCmd::TogglePart { part: 3 })));
        assert_eq!(s.controls[8].shift_label, "EDIT R1");
        assert_eq!(s.controls[8].shift_action, Some(AppCmd::Parts(PartsCmd::SelectPart { part: 0 })));
        assert_eq!(s.controls[3].action, Some(AppCmd::Library(LibraryCmd::StepStyle { delta: 1 })));
        assert!(s.controls[4..8].iter().all(|c| c.colour.is_none() && c.level == Level::Off));
        assert!(s.controls.iter().all(|c| c.anim == Anim::Solid));
        assert_eq!(s.controls[16].level, Level::Bright);
        assert_eq!(faders(&m), ["RIGHT 1", "RIGHT 2", "RIGHT 3", "LEFT", "", "", "", "", "MASTER"]);
        assert_eq!(s.faders.iter().map(|f| f.position).collect::<Vec<_>>(), HW_FADERS.map(Some));
        assert_eq!(s.faders[4].set, None);
        assert_eq!(m.state.keyboard_parts[1].fader, Some(72));
        assert_eq!(m.state.mixer.style_parts[7].fader, Some(0));

        // Style page.
        m.send(MixerCmd::ToggleFaderPage);
        m.send(PadsCmd::SetPadPage { page: Page::OtsParts });
        let s = &m.state.surface;
        assert_eq!(
            labels(&m),
            ["PAGE ▲", "", "◀ STYLE", "STYLE ▶", "PLAY", "STOP", "TEMPO +", "TEMPO -", "RHYTHM 1", "RHYTHM 2", "BASS", "CHORD 1", "CHORD 2", "PAD", "PHRASE 1", "PHRASE 2", "STYLE"]
        );
        assert_eq!(s.controls[8].shift_label, "RHYTHM 1");
        assert_eq!(s.controls[13].action, Some(AppCmd::Mixer(MixerCmd::ToggleStylePart { part: 5 })));
        assert_eq!(faders(&m), ["RHYTHM 1", "RHYTHM 2", "BASS", "CHORD 1", "CHORD 2", "PAD", "PHRASE 1", "PHRASE 2", "MASTER"]);
        assert_eq!(s.faders[5].set, Some(AppCmd::Mixer(MixerCmd::SetStylePartVolume { part: 5, volume: 0 })));
    }

    #[test]
    fn track_neighbours_skip_styles_that_do_not_load_and_wrap() {
        let m = MockSession::new();
        let lib = &m.library().entries;
        let prev = m.state.surface.track_prev.clone().unwrap();
        let next = m.state.surface.track_next.clone().unwrap();
        assert_eq!(prev.id, lib.iter().rev().find(|e| e.status == "ok").unwrap().id, "wraps to the last that loads");
        assert_eq!(next.id, lib.iter().skip(1).find(|e| e.status == "ok").unwrap().id);
        assert_eq!(next.path, lib[next.id].path);
    }

    #[test]
    fn the_clock_follows_the_band_and_is_read_when_the_state_changes() {
        let mut m = MockSession::new();
        let c = m.state.surface.clock.clone();
        assert_eq!((c.bar, c.beat, c.running), (12, 1, true));
        assert_eq!(c.beats_per_bar, quarters_per_bar(m.state.style.time_signature));
        // Time passing alone changes nothing: the anchors carry the position.
        let v = m.state.version;
        m.advance(10.0);
        assert_eq!(m.state.version, v);
        assert_eq!(m.state.surface.clock, c);
        let now = m.state_now().surface.clock;
        assert_eq!(now.at_ms, 10.0);
        assert!((now.phase - 10.0 * c.tempo / 60e3).abs() < 1e-9);
        // A tempo change re-anchors both clocks where they were.
        let led = c.led_beats(10.0);
        m.send(TransportCmd::TempoUp);
        let c2 = m.state.surface.clock.clone();
        assert_eq!((c2.at_ms, c2.section_anchor_ms, c2.led_anchor_ms), (10.0, 10.0, 10.0));
        assert!((c2.led_anchor_beats - led).abs() < 1e-9);
        assert!((c2.position(10.0) - c.position(10.0)).abs() < 1e-9);
        // The transport and the clock agree on the bar.
        m.advance(bar_ms(&m) * 1.5);
        let st = m.state_now();
        assert_eq!(st.surface.clock.bar, st.transport.bar);
        // Stopped: 1, 1, 0.
        m.send(TransportCmd::Stop);
        let c3 = m.state_now().surface.clock;
        assert_eq!((c3.running, c3.bar, c3.beat, c3.phase), (false, 1, 1, 0.0));
    }

    #[test]
    fn chords_transpose() {
        assert_eq!(transpose_chord("Am7/G", 2), "Bm7/A");
        assert_eq!(transpose_chord("C#m", -1), "Cm");
    }
}

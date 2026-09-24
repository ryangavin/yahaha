//! A mock session for the app shell until #16's engine `Session` is wired in. It speaks
//! the same API (`api.rs`) and behaves like `app/src/lib/api/mock.ts` (the browser-only
//! dev mock), from the same fixture: the band advances bar by bar, queued sections take
//! over at the bar (fills at the beat), chords change, faders wait for pickup. No audio,
//! no MIDI.

use crate::api::*;

const FIXTURE: &str = include_str!("../../src/lib/api/mock-fixture.json");
const ROOT: &str = "/Users/me/Styles";

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
                })
                .collect(),
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
                        voice: Some(Voice { bank_msb: *msb, bank_lsb: *lsb, program: *program, kit: *kit, label: label.to_string() }),
                    })
                    .collect(),
                master: Some(100),
                master_waiting: false,
            },
            pads: PadsState { page: Page::Sections, page_name: String::new(), page_number: 1, page_count: 3, pads: vec![], connected: true },
            ots: OtsState { settings: vec![], applied: 0, link: false },
            library: LibraryStatus { revision: 1, count: library.entries.len(), position: 0, pending: 0 },
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
        };
        m.set_style(0);
        m.state.ots.applied = 2;
        m.state.mixer.style_parts[5].volume = 58;
        m.state.mixer.style_parts[5].waiting = true;
        m.clock = 11.0 * m.state.transport.beats_per_bar as f64;
        m.position();
        m.derive();
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

    fn position(&mut self) {
        let t = &mut self.state.transport;
        let bpb = t.beats_per_bar as f64;
        let bar = (self.clock / bpb).floor() as u32;
        t.bar = bar.saturating_sub(self.section_start) + 1;
        t.beat = (self.clock.floor() as u32) % t.beats_per_bar as u32 + 1;
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
    pub fn send(&mut self, cmd: AppCmd) -> bool {
        let before = self.state.clone();
        self.cmd(cmd);
        self.bump(&before)
    }

    fn bump(&mut self, before: &AppState) -> bool {
        self.derive();
        let changed = self.state != *before;
        if changed {
            self.state.version = before.version + 1;
        }
        changed
    }

    fn step(&mut self, ms: f64) {
        self.now += ms;
        if !self.state.transport.running {
            return;
        }
        let bpb = self.state.transport.beats_per_bar as f64;
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
        let t = &mut self.state.transport;
        if let Some(q) = t.queued.clone().filter(|q| FILLS.contains(&q.as_str())) {
            t.section = Some(q);
            t.queued = None;
            self.section_start = (self.clock / t.beats_per_bar as f64).floor() as u32;
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
            self.cmd(AppCmd::Main { index });
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
        st.transport.sync_stop_available = c.upper || !matches!(c.fingering, Fingering::FullKeyboard | Fingering::AiFullKeyboard);
        let mb = c.manual_bass_active;
        for (i, p) in st.keyboard_parts.iter_mut().enumerate() {
            p.plays_bass = i == 3 && mb;
            p.sounding = p.on || p.plays_bass;
            p.voice_name = if p.plays_bass { "Finger Bass".into() } else { self.gm[p.program as usize].clone() };
        }
        for (i, p) in st.mixer.style_parts.iter_mut().enumerate() {
            p.muted_by_manual_bass = i == 2 && mb;
        }
        st.pads.page_name = st.pads.page.name().into();
        st.pads.page_number = st.pads.page as u8 + 1;
        st.transport.lamps = pads_for(st, Page::Sections);
        st.pads.pads = pads_for(st, st.pads.page);
    }

    fn cmd(&mut self, cmd: AppCmd) {
        let running = self.state.transport.running;
        let vol = |v: u8| v.min(127);
        match cmd {
            AppCmd::StartStop => {
                if running {
                    self.stop_band()
                } else {
                    self.start_band()
                }
            }
            AppCmd::Stop => {
                if running {
                    self.stop_band()
                }
            }
            AppCmd::Intro { index } => {
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
            AppCmd::Main { index } => {
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
            AppCmd::Break => {
                if running && self.has(BREAK) {
                    self.state.transport.queued = Some(BREAK.into());
                }
            }
            AppCmd::Ending { index } => {
                let id = ENDINGS[index.min(2) as usize];
                if running && self.has(id) {
                    self.state.transport.queued = Some(id.into());
                }
            }
            AppCmd::ToggleSyncStart => {
                if running {
                    self.stop_band();
                }
                self.state.transport.sync_start = !self.state.transport.sync_start;
            }
            AppCmd::ToggleSyncStop => {
                if self.state.transport.sync_stop_available {
                    self.state.transport.sync_stop = !self.state.transport.sync_stop;
                } else {
                    self.message("Sync Stop is not available with the Full Keyboard fingering types", true);
                }
            }
            AppCmd::ToggleAutoFill => self.state.transport.auto_fill = !self.state.transport.auto_fill,
            AppCmd::ToggleStopAcmp => self.state.transport.stop_acmp = !self.state.transport.stop_acmp,
            AppCmd::TapTempo => {
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
            AppCmd::TempoUp => self.state.transport.tempo = (self.state.transport.tempo + 1.0).min(300.0),
            AppCmd::TempoDown => self.state.transport.tempo = (self.state.transport.tempo - 1.0).max(30.0),
            AppCmd::ToggleStylePart { part } => {
                if let Some(p) = self.state.mixer.style_parts.get_mut(part as usize) {
                    p.on = !p.on;
                }
            }
            AppCmd::SetStylePartVolume { part, volume } => {
                if let Some(p) = self.state.mixer.style_parts.get_mut(part as usize) {
                    p.volume = vol(volume);
                    p.waiting = false;
                }
            }
            AppCmd::SetFingering { fingering } => self.state.chord.fingering = fingering,
            AppCmd::NextFingering => {
                let i = Fingering::ALL.iter().position(|f| *f == self.state.chord.fingering).unwrap_or(0);
                self.state.chord.fingering = Fingering::ALL[(i + 1) % Fingering::ALL.len()];
            }
            AppCmd::SetUpper { on } => self.set_upper(on),
            AppCmd::ToggleUpper => self.set_upper(!self.state.chord.upper),
            AppCmd::SetManualBass { on } => self.set_manual_bass(on),
            AppCmd::ToggleManualBass => self.set_manual_bass(!self.state.chord.manual_bass),
            AppCmd::SetSplit { note } => self.state.chord.split = note.clamp(24, 96),
            AppCmd::MoveSplit { delta } => {
                self.state.chord.split = (self.state.chord.split as i16 + delta as i16).clamp(24, 96) as u8;
            }
            AppCmd::SetTranspose { keyboard, master } => {
                self.state.chord.transpose_keyboard = keyboard.clamp(-12, 12);
                self.state.chord.transpose_master = master.clamp(-12, 12);
            }
            AppCmd::StepTranspose { keyboard, master } => {
                let c = &mut self.state.chord;
                c.transpose_keyboard = (c.transpose_keyboard + keyboard).clamp(-12, 12);
                c.transpose_master = (c.transpose_master + master).clamp(-12, 12);
            }
            AppCmd::ResetTranspose => {
                self.state.chord.transpose_keyboard = 0;
                self.state.chord.transpose_master = 0;
            }
            AppCmd::SetPartOn { part, on } => self.set_part_on(part, on),
            AppCmd::TogglePart { part } => {
                let on = self.state.keyboard_parts.get(part as usize).is_some_and(|p| !p.on);
                self.set_part_on(part, on);
            }
            AppCmd::SelectPart { part } => {
                for (i, p) in self.state.keyboard_parts.iter_mut().enumerate() {
                    p.selected = i == part as usize;
                }
            }
            AppCmd::SetPartVoice { part, program } => {
                if let Some(p) = self.state.keyboard_parts.get_mut(part as usize) {
                    p.program = program & 127;
                }
            }
            AppCmd::StepVoice { delta } => {
                if let Some(p) = self.state.keyboard_parts.iter_mut().find(|p| p.selected) {
                    p.program = (p.program as i16 + delta as i16).rem_euclid(128) as u8;
                }
            }
            AppCmd::SetPartVolume { part, volume } => {
                if let Some(p) = self.state.keyboard_parts.get_mut(part as usize) {
                    p.volume = vol(volume);
                    p.waiting = false;
                }
            }
            AppCmd::SetPartOctave { part, octave } => {
                if let Some(p) = self.state.keyboard_parts.get_mut(part as usize) {
                    p.octave = octave.clamp(-2, 2);
                }
            }
            AppCmd::SetFaderPage { page } => self.set_fader_page(page),
            AppCmd::ToggleFaderPage => {
                let page = if self.state.mixer.fader_page == FaderPage::Panel { FaderPage::Style } else { FaderPage::Panel };
                self.set_fader_page(page);
            }
            AppCmd::SetPadPage { page } => self.state.pads.page = page,
            AppCmd::CyclePadPage { delta } => {
                let i = self.state.pads.page as i8;
                self.state.pads.page = Page::ALL[(i + delta).rem_euclid(3) as usize];
            }
            AppCmd::SetMasterVolume { volume } => {
                self.state.mixer.master = Some(vol(volume));
                self.state.mixer.master_waiting = false;
            }
            AppCmd::RecallOts { index } => {
                if (index as usize) < self.state.ots.settings.len() {
                    self.recall_ots(index as usize);
                }
            }
            AppCmd::SetOtsLink { on } => self.state.ots.link = on,
            AppCmd::ToggleOtsLink => self.state.ots.link = !self.state.ots.link,
            AppCmd::LoadStyle { id } => self.load_style(id),
            AppCmd::LoadStylePath { path } => match self.library.entries.iter().find(|e| e.path == path).map(|e| e.id) {
                Some(id) => self.load_style(id),
                None => self.message(format!("{path}: not found"), true),
            },
            AppCmd::StepStyle { delta } => {
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
            AppCmd::SetSynthMuted { on } => {
                if let Some(s) = &mut self.state.io.synth {
                    s.muted = on;
                }
            }
            AppCmd::ToggleSynthMute => {
                if let Some(s) = &mut self.state.io.synth {
                    s.muted = !s.muted;
                }
            }
            AppCmd::SetAudioOutput { first } => {
                if let Some(s) = &mut self.state.io.synth {
                    if (first as u32) + 1 < s.channels {
                        s.output_pair = [first + 1, first + 2];
                    }
                }
            }
            AppCmd::NextAudioOutput => {
                if let Some(s) = &mut self.state.io.synth {
                    let next = s.output_pair[1] + 1;
                    s.output_pair = if (next as u32) < s.channels { [next, next + 1] } else { [1, 2] };
                }
            }
            AppCmd::Panic => {
                self.stop_band();
                self.message("All notes off", false);
            }
            AppCmd::ClearMessage => self.state.message = None,
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
    Pad { note, label: label.into(), key: key.into(), rgb, level, anim, action }
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
                pad(96, "INTRO 1", "q", Some(AppCmd::Intro { index: 0 }), intro(0)),
                pad(97, "INTRO 2", "w", Some(AppCmd::Intro { index: 1 }), intro(1)),
                pad(98, "INTRO 3", "e", Some(AppCmd::Intro { index: 2 }), intro(2)),
                pad(99, "SYNC ST", "y", Some(AppCmd::ToggleSyncStart), if t.sync_start { (C_SYNC, Level::Bright, Anim::Pulse) } else { toggle(false, C_SYNC) }),
                pad(100, "ENDING 1", "i", Some(AppCmd::Ending { index: 0 }), sec(ENDINGS[0], C_ENDING)),
                pad(101, "ENDING 2", "o", Some(AppCmd::Ending { index: 1 }), sec(ENDINGS[1], C_ENDING)),
                pad(102, "ENDING 3", "p", Some(AppCmd::Ending { index: 2 }), sec(ENDINGS[2], C_ENDING)),
                pad(103, "AUTOFILL", "u", Some(AppCmd::ToggleAutoFill), toggle(t.auto_fill, C_FILL)),
                pad(112, "MAIN A", "1", Some(AppCmd::Main { index: 0 }), main(0)),
                pad(113, "MAIN B", "2", Some(AppCmd::Main { index: 1 }), main(1)),
                pad(114, "MAIN C", "3", Some(AppCmd::Main { index: 2 }), main(2)),
                pad(115, "MAIN D", "4", Some(AppCmd::Main { index: 3 }), main(3)),
                pad(116, "BREAK", "g", Some(AppCmd::Break), sec(BREAK, C_BREAK)),
                pad(117, "TAP", "t", Some(AppCmd::TapTempo), toggle(t.running && t.beat == 1, C_TAP)),
                pad(118, "SYNC STP", "j", Some(AppCmd::ToggleSyncStop), toggle(t.sync_stop, C_STOPSYNC)),
                pad(119, if t.running { "START" } else { "STOP" }, "spc", Some(AppCmd::StartStop), (if t.running { C_RUN } else { C_IDLE }, Level::Bright, Anim::Solid)),
            ]
        }
        Page::ChordSetup => {
            const LABELS: [&str; 7] = ["SINGLE", "FINGERED", "ON BASS", "MULTI", "AI FING", "FULL KBD", "AI FULL"];
            let c = &s.chord;
            let mut v: Vec<Pad> = Fingering::ALL
                .iter()
                .enumerate()
                .map(|(i, f)| page_pad(96 + i as u8, LABELS[i], "pad", Some(AppCmd::SetFingering { fingering: *f }), true, c.fingering == *f))
                .collect();
            v.extend([
                page_pad(103, "UPPER", "d", Some(AppCmd::ToggleUpper), true, c.upper),
                page_pad(112, "MAN BASS", "D", Some(AppCmd::ToggleManualBass), c.upper, c.manual_bass),
                page_pad(113, "STOP ACMP", "h", Some(AppCmd::ToggleStopAcmp), true, t.stop_acmp),
                page_pad(114, "SPLIT -", "[", Some(AppCmd::MoveSplit { delta: -1 }), true, false),
                page_pad(115, "SPLIT +", "]", Some(AppCmd::MoveSplit { delta: 1 }), true, false),
                page_pad(116, "KBD TR -", ";", Some(AppCmd::StepTranspose { keyboard: -1, master: 0 }), true, c.transpose_keyboard < 0),
                page_pad(117, "KBD TR +", "'", Some(AppCmd::StepTranspose { keyboard: 1, master: 0 }), true, c.transpose_keyboard > 0),
                page_pad(118, "TR RESET", "/", Some(AppCmd::ResetTranspose), true, c.transpose_keyboard != 0 || c.transpose_master != 0),
                page_pad(119, "", "", None, false, false),
            ]);
            v
        }
        Page::OtsParts => {
            let n = s.ots.settings.len();
            let mut v: Vec<Pad> = (0..4)
                .map(|i| page_pad(96 + i as u8, &format!("OTS {}", i + 1), &format!("⇧{}", i + 1), Some(AppCmd::RecallOts { index: i as u8 }), i < n, s.ots.applied as usize == i + 1))
                .collect();
            v.extend([
                page_pad(100, "OTS LINK", "F10", Some(AppCmd::ToggleOtsLink), true, s.ots.link),
                page_pad(101, "", "", None, false, false),
                page_pad(102, "VOICE -", "9", Some(AppCmd::StepVoice { delta: -1 }), true, false),
                page_pad(103, "VOICE +", "0", Some(AppCmd::StepVoice { delta: 1 }), true, false),
            ]);
            for (i, (label, key)) in [("RIGHT 1", "5"), ("RIGHT 2", "6"), ("RIGHT 3", "7"), ("LEFT", "8/l")].iter().enumerate() {
                v.push(page_pad(112 + i as u8, label, key, Some(AppCmd::TogglePart { part: i as u8 }), true, s.keyboard_parts[i].on));
            }
            for (i, label) in ["EDIT R1", "EDIT R2", "EDIT R3", "EDIT L"].iter().enumerate() {
                v.push(page_pad(116 + i as u8, label, &format!("F{}", i + 1), Some(AppCmd::SelectPart { part: i as u8 }), true, s.keyboard_parts[i].selected));
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
        m.send(AppCmd::Main { index: 2 });
        assert_eq!(m.state.transport.queued.as_deref(), Some("Main C"));
        m.advance(bar_ms(&m));
        assert_eq!(m.state.transport.section.as_deref(), Some("Main C"));
    }

    #[test]
    fn pressing_the_playing_main_queues_its_fill_and_the_lamp_flashes() {
        let mut m = MockSession::new();
        m.send(AppCmd::Main { index: 1 });
        assert_eq!(m.state.transport.queued.as_deref(), Some("Fill In BB"));
        let lamp = m.state.transport.lamps.iter().find(|p| p.note == 113).unwrap();
        assert_eq!((lamp.level, lamp.anim), (Level::Bright, Anim::Flash));
    }

    #[test]
    fn versions_change_only_with_the_state() {
        let mut m = MockSession::new();
        let v = m.state.version;
        assert!(!m.send(AppCmd::SetOtsLink { on: false }));
        assert_eq!(m.state.version, v);
        assert!(m.send(AppCmd::ToggleOtsLink));
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
        m.send(AppCmd::Intro { index: 0 });
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
    fn chords_transpose() {
        assert_eq!(transpose_chord("Am7/G", 2), "Bm7/A");
        assert_eq!(transpose_chord("C#m", -1), "Cm");
    }
}

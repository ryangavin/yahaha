//! A mock session for the app shell, for running without MIDI or styles (`YAHAHA_MOCK=1`,
//! or when the engine can't start). It builds the engine's own `AppState` types
//! (yahaha::api) and behaves like `app/src/lib/api/mock.ts` (the browser-only
//! dev mock), from the same fixture: the band advances bar by bar, queued sections take
//! over at the bar (fills at the beat), chords change, faders wait for pickup. No audio,
//! no MIDI.

use std::time::Instant;

#[path = "mock_multipad.rs"]
mod multipad;
#[path = "mock_sound.rs"]
mod sound;
#[path = "mock_sounds.rs"]
mod sounds;

use yahaha::api::*;
use yahaha::engine::{FadeState, StyleSettings};
use yahaha::controllers::{Controllers, PedalSetup, PEDALS};
use yahaha::fingering::Fingering;
use yahaha::launchkey::{self as lk, Action, Anim, Control, Level, Page};
use yahaha::parts::{self, FaderPage};

use crate::mock_regist::{Effect, MockRegist};
use crate::mock_looper::{self, MockLooper};

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
    /// Steady taps in a row (the engine's count), and when a bar of them starts the band.
    tap_run: usize,
    tap_start: Option<f64>,
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
    /// The imported iReal Pro playlists (#89), parsed by the engine's own `ireal` module.
    chart_lists: Vec<yahaha::ireal::Playlist>,
    /// The chart's last bar has played and it has no Ending: stop at the next bar line.
    chart_end: bool,
    /// The Style settings (`StyleSettingsCmd`), and how long the fade phase playing has
    /// left (ms).
    settings: StyleSettings,
    fade_left: f64,
    /// Registration Memory and the Playlist (in memory).
    regist: MockRegist,
    /// The Chord Looper, as the engine runs it (mock_looper.rs).
    looper: MockLooper,
    /// Multi Pads (mock_multipad.rs).
    pads: multipad::MockPads,
    /// Pedals and wheels: the engine's own model (no keyboard, so nothing moves them but
    /// commands).
    controllers: Controllers,
    /// The sound library (mock_sound.rs).
    sound: sound::MockSound,
    /// The sound catalog (mock_sounds.rs).
    sounds: sounds::MockSounds,
    /// Knob Assign pages (#197): the engine's own model.
    knobs: yahaha::knobs::Knobs,
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
            harmony_types: harmony_type_options(),
            arp_patterns: arp_pattern_options(),
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
            pan: 64,
            reverb: 40,
            chorus: 0,
            fader: None,
            plugin: None,
            patch: None,
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
                half_bar_fill: false,
                stop_acmp_mode: StopAcmpMode::Off,
                fade: FadeState::Off,
                retrigger: false,
                ritardando: false,
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
                settle_ms: yahaha::engine::CHORD_SETTLE_DEFAULT_MS,
                left_hold: false,
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
                style_solo: None,
                part_solo: None,
            },
            pads: PadsState { page: Page::Sections, page_name: String::new(), page_number: 1, page_count: Page::ALL.len() as u8, pads: vec![], connected: true, palette_leds: false },
            ots: OtsState { settings: vec![], applied: 0, link: false, link_timing: OtsLinkTiming::MainChange },
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
                default_sound_set: None,
                auto_sound_set: Some(MOCK_SOUND_FONTS[0].into()),
            },
            preview: PreviewState::default(),
            chart: ChartState::default(),
            multi_pad: multipad::initial(),
            harmony_arp: harmony_arp_default(),
            // Mid-song: the left hand holds the Am7 it fingered.
            keyboard: KeyboardState {
                held: [45, 48, 52, 55].map(|note| HeldNote { note, zone: Zone::Left, parts: vec![] }).to_vec(),
                left_split: 54,
                chord_tones: vec![9, 0, 4, 7],
                chord_bass: Some(9),
                detection: [0, 54],
            },
            style_settings: StyleSettingsState::default(),
            controllers: ControllersState::of(&Controllers::new()),
            message: None,
            style_change: StyleChangeState::default(),
            registration: RegistrationState::default(),
            playlist: PlaylistState::default(),
            looper: mock_looper::empty(),
            metronome: MetronomeState { on: false, volume: 90, bell: true, audible: true },
            plugins: mock_plugins(),
            sound_library: SoundLibraryState::default(),
            param_locks: ParamLockState::default(),
            sounds: SoundsState::default(),
            dynamics: DynamicsState::default(),
            knobs: KnobsState::default(),
        };
        let songs: Vec<(String, String)> = library.entries.iter().filter(|e| e.status == "ok").map(|e| (e.path.clone(), e.name.clone())).collect();
        let mut m = MockSession {
            state,
            gm,
            styles: f.styles,
            library,
            clock: 0.0,
            section_start: 0,
            taps: vec![],
            tap_run: 0,
            tap_start: None,
            now: 0.0,
            progression: 0,
            message_seq: 0,
            hw_faders: HW_FADERS,
            section_anchor: (0.0, 0.0),
            section_key: None,
            led_anchor: (0.0, 0.0, 0.0), // anchored by the first `derive`
            wall: None,
            chart_lists: Vec::new(),
            chart_end: false,
            settings: StyleSettings::default(),
            fade_left: 0.0,
            regist: MockRegist::new(&songs),
            looper: MockLooper::default(),
            pads: multipad::MockPads::default(),
            controllers: Controllers::new(),
            sound: sound::MockSound::default(),
            sounds: sounds::MockSounds::default(),
            knobs: Default::default(),
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

    /// The sound catalog (#117).
    pub fn sounds(&self) -> SoundCatalog {
        self.sounds.catalog(&self.state)
    }

    fn sounds_cmd(&mut self, c: SoundsCmd) {
        match self.sounds.cmd(&self.state, c) {
            Err(e) => self.message(e, true),
            Ok(sounds::Then::Nothing) => {}
            Ok(sounds::Then::Run(cmds)) => {
                // A preset from the synth's own font is the part's GM voice (SetPartVoice):
                // it ends a plugin picked for the part, as a SoundFont patch does.
                for c in cmds {
                    self.cmd(c);
                }
            }
            Ok(sounds::Then::AddThenAssign(add, part)) => {
                self.cmd(add);
                self.derive();
                let id = self.state.sound_library.last_added.clone();
                self.cmd(SoundLibraryCmd::SetPartPatch { part, id }.into());
            }
        }
    }

    fn has(&self, s: &str) -> bool {
        self.state.style.sections.iter().any(|x| x == s)
    }

    /// Instrument plugins, as the TS mock (mock-plugins.ts) plays them, minus the load
    /// time: a part's plugin plays at once.
    fn plugin_cmd(&mut self, c: PluginCmd) {
        match c {
            PluginCmd::SetPartPlugin { part, id, .. } => {
                self.sound.part_plugin(part as usize, true);
                self.set_part_plugin(part as usize, id);
            }
            PluginCmd::ClearPartPlugin { part } => {
                self.sound.part_plugin(part as usize, false);
                self.state.keyboard_parts[(part & 3) as usize].plugin = None;
            }
            PluginCmd::SavePartPluginState { .. } | PluginCmd::RescanPlugins => {}
            PluginCmd::ReloadPartPlugin { part } => {
                let part = match part {
                    Some(p) => (p & 3) as usize,
                    None => self.state.keyboard_parts.iter().position(|p| p.selected).unwrap_or(0),
                };
                let name = self.state.keyboard_parts[part].name.clone();
                match self.state.keyboard_parts[part].plugin.as_ref().map(|p| (p.status, p.id.clone(), p.name.clone())) {
                    None => self.message(format!("{name} plays its SoundFont voice; there is no plugin to reload"), true),
                    Some((PluginStatus::Muted | PluginStatus::Failed, id, _)) => self.set_part_plugin(part, id),
                    Some((PluginStatus::Playing, _, plugin)) => self.message(format!("{name}'s {plugin} is playing; nothing to reload"), true),
                    Some((PluginStatus::Loading, _, plugin)) => self.message(format!("{name}'s {plugin} is still loading"), true),
                }
            }
            PluginCmd::SetPluginInProcess { id, in_process } => {
                let Some(e) = self.state.plugins.list.iter_mut().find(|p| p.id == id) else {
                    return self.message(format!("no instrument Audio Unit {id} is installed"), true);
                };
                if in_process && !e.can_run_in_process {
                    let text = format!("{}: {} is an AUv3 that only runs out of process", e.manufacturer, e.name);
                    return self.message(text, true);
                }
                e.in_process = in_process;
                let name = e.name.clone();
                let playing = self.state.keyboard_parts.iter().any(|k| k.plugin.as_ref().is_some_and(|p| p.id == id && p.status == PluginStatus::Playing));
                if playing {
                    let r#where = if in_process { "inside yahaha" } else { "in its own process" };
                    self.message(format!("{name} runs {where} from its next load (the next start, or pick it again)"), false);
                }
            }
        }
    }

    fn set_part_plugin(&mut self, part: usize, id: String) {
        let Some(e) = self.state.plugins.list.iter().find(|p| p.id == id).cloned() else {
            return self.message(format!("no instrument Audio Unit {id} is installed"), true);
        };
        let failed = e.last_error.clone();
        let fallback = failed.is_none() && e.id == MOCK_FALLBACK_ID && !e.in_process;
        // AUSampler plays the heavy plugin: a high CPU share and a few slow renders.
        let heavy = failed.is_none() && e.id == MOCK_HEAVY_ID;
        self.state.keyboard_parts[part & 3].plugin = Some(PartPlugin {
            id: e.id,
            name: e.name.clone(),
            manufacturer: e.manufacturer.clone(),
            status: if failed.is_some() { PluginStatus::Failed } else { PluginStatus::Playing },
            stage: None,
            error: failed.clone(),
            out_of_process: e.manufacturer != "Apple" && !e.in_process && !fallback,
            in_process_fallback: fallback,
            cpu: if failed.is_some() { 0.0 } else if heavy { 0.31 } else { 0.012 },
            overruns: if heavy { 4 } else { 0 },
            recent_overruns: if heavy { 4 } else { 0 },
            editor: failed.is_none(),
        });
        if let Some(err) = failed {
            self.message(format!("{} didn't load: {err}", e.name), true);
        } else if fallback {
            self.message(format!("{} can't run in its own process; loading it inside yahaha instead (if it crashes, yahaha goes with it)", e.name), false);
        }
    }

    /// The default sound set (#117): a font in the folder, or None for Auto.
    fn set_default_sound_set(&mut self, file: Option<String>) {
        if let Some(f) = &file
            && !self.state.io.sound_fonts.contains(f)
        {
            return self.message(format!("no SoundFont {f} in the SoundFont folder"), true);
        }
        let io = &mut self.state.io;
        io.default_sound_set = file.clone();
        let Some(play) = file.or_else(|| io.auto_sound_set.clone()) else { return };
        if let Some(s) = io.synth.as_mut() {
            s.sound_font = play.trim_end_matches(".sf2").to_string();
        }
        io.sound_font_file = Some(play);
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
        self.sync_part_plugins();
        self.bump(&before)
    }

    /// The values the knobs turn from (the session's `knobs_now`).
    fn knobs_now(&self) -> yahaha::knobs::Now {
        let s = &self.state;
        yahaha::knobs::Now {
            dynamics: s.dynamics.level,
            retrigger: s.transport.retrigger,
            retrigger_rate: s.style_settings.retrigger_rate,
            bpm: s.transport.tempo,
            part_volume: [0, 1, 2, 3].map(|p| s.keyboard_parts[p].volume),
            harmony_volume: s.harmony_arp.volume,
            metronome_volume: s.metronome.volume,
        }
    }

    /// A keyboard part's own plugin patch plays its plugin (the session's
    /// `sync_part_plugins`).
    fn sync_part_plugins(&mut self) {
        for (p, voice) in self.sound.part_plugins() {
            match voice {
                Some((id, _state)) => self.set_part_plugin(p, id),
                None => self.state.keyboard_parts[p].plugin = None,
            }
        }
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
        // A bar of taps while stopped: the band starts a beat after the last (OM p.46).
        if let Some(t) = self.tap_start
            && self.now >= t
        {
            self.tap_start = None;
            if !self.state.transport.running {
                self.start_band();
            }
        }
        self.step_fade(ms);
        self.pads.beats(&mut self.state.multi_pad, ms / 60000.0 * self.state.transport.tempo);
        self.sound.advance(ms, self.state.transport.running);
        self.sounds.advance(ms, self.state.transport.running);
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
        if self.chart_playing() {
            // The quarter this is, as a place in the bar (a chord goes in at beat/beats of
            // its chart bar, as the engine places it).
            let pos = self.clock.rem_euclid(qpb).floor() / qpb;
            let c = &self.state.chart;
            if let (Some(song), Some(i)) = (&c.song, c.bar) {
                let name = chord_at(song, i as usize, pos);
                self.chart_chord(name);
            }
        }
        let t = &mut self.state.transport;
        if let Some(q) = t.queued.clone().filter(|q| FILLS.contains(&q.as_str())) {
            t.section = Some(q);
            t.queued = None;
            self.section_start = (self.clock / qpb).floor() as u32;
        }
    }

    fn on_bar(&mut self, bar: u32) {
        if self.chart_end {
            self.stop_band();
            return;
        }
        self.pads.bar(&mut self.state.multi_pad);
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
        if self.chart_playing() {
            self.chart_bar();
            self.looper_bar(bar);
            return;
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
            self.keyboard_chord(chord);
        }
        self.looper_bar(bar);
    }

    /// A chord from the (imaginary) left hand: the Chord Looper ignores it while it loops,
    /// records it while it records.
    fn keyboard_chord(&mut self, chord: &str) {
        let bpb = self.bar_quarters();
        let (bar, beat) = ((self.clock / bpb).floor() as u32, (self.clock % bpb).floor() + 1.0);
        if self.looper.keyboard_chord(&self.state.looper, chord, bar, beat) {
            self.chord_arrives(chord);
        }
    }

    /// A bar line for the Chord Looper: it may start recording, or play the loop's chord.
    fn looper_bar(&mut self, bar: u32) {
        let played = self.state.chord.fingered.clone();
        if let Some(c) = self.looper.on_bar(&mut self.state.looper, bar, played.as_deref()) {
            if played.as_deref() != Some(c.as_str()) {
                self.chord_arrives(&c);
            }
        }
    }

    fn enter(&mut self, s: &str, bar: u32) {
        let from_ending = self.state.transport.section.as_deref().is_some_and(|c| ENDINGS.contains(&c));
        if ENDINGS.contains(&s) && !from_ending {
            self.pads.ending_started(&mut self.state.multi_pad);
        }
        self.state.transport.section = Some(s.into());
        self.section_start = bar;
        if let Some(m) = MAINS.iter().position(|x| *x == s) {
            self.state.transport.main = m as u8;
            // OTS Link Timing "At Main Section Change": as the Main starts playing.
            let ots = &self.state.ots;
            if ots.link && ots.link_timing == OtsLinkTiming::MainChange && m < ots.settings.len() && ots.applied as usize != m + 1 {
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
        self.pads.chord(&mut self.state.multi_pad, self.state.transport.running);
    }

    /// Fade In/Out: a fade in runs out, a fade out stops the band and holds, a hold ends.
    fn step_fade(&mut self, ms: f64) {
        if matches!(self.state.transport.fade, FadeState::Off | FadeState::Armed) {
            return;
        }
        self.fade_left -= ms;
        if self.fade_left > 0.0 {
            return;
        }
        match self.state.transport.fade {
            FadeState::FadingOut => {
                self.stop_band();
                self.state.transport.fade = FadeState::Holding;
                self.fade_left += self.settings.fade_hold_ms as f64;
            }
            _ => self.state.transport.fade = FadeState::Off,
        }
    }

    /// Style Section Reset: the section starts again from its top, now.
    fn reset_section(&mut self) {
        if self.state.transport.running {
            let qpb = self.bar_quarters();
            self.section_start = (self.clock / qpb).floor() as u32;
            self.clock = self.section_start as f64 * qpb;
            self.position();
        }
    }

    fn start_band(&mut self) {
        self.tap_start = None;
        match self.state.transport.fade {
            FadeState::Armed => {
                self.state.transport.fade = FadeState::FadingIn;
                self.fade_left = self.settings.fade_in_ms as f64;
            }
            FadeState::Holding => self.state.transport.fade = FadeState::Off,
            _ => {}
        }
        self.chart_end = false;
        if let (true, Some(song)) = (self.state.chart.on, self.state.chart.song.as_ref()) {
            // The chart's Intro (unless one is armed), its first Main and first chord.
            let first = song.bars.first().map_or(0, |b| b.main);
            let name = chord_at(song, 0, 0.0);
            let t = &mut self.state.transport;
            if t.pending_intro.is_none() {
                t.pending_intro = self.state.chart.intro;
            }
            t.main = first;
            self.state.chart.bar = None;
            self.chart_chord(name);
        }
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
        if self.chart_playing() && self.state.transport.section.as_deref().is_some_and(|s| MAINS.contains(&s)) {
            self.chart_bar();
        }
        self.looper_bar(0);
        self.pads.band_started(&mut self.state.multi_pad);
    }

    fn stop_band(&mut self) {
        self.state.chart.bar = None;
        self.chart_end = false;
        if self.state.transport.running {
            self.pads.band_stopped(&mut self.state.multi_pad);
        }
        let t = &mut self.state.transport;
        if matches!(t.fade, FadeState::FadingIn | FadeState::FadingOut) {
            t.fade = FadeState::Off;
        }
        t.ritardando = false;
        t.running = false;
        t.section = None;
        t.queued = None;
        t.bar = 1;
        t.beat = 1;
        self.looper.on_stop(&mut self.state.looper);
    }

    /// Part `part` was given a GM voice (SetPartVoice, Voice −/+, #179): a plugin picked
    /// for it ends, and its own library patch goes (with that patch's plugin).
    fn gm_voice(&mut self, part: usize) {
        if self.sound.own_plugin(part)
            && let Some(p) = self.state.keyboard_parts.get_mut(part & 3)
        {
            p.plugin = None;
        }
        self.sound.part_voice(part);
    }

    fn recall_ots(&mut self, n: usize) {
        let panel = self.state.mixer.fader_page == FaderPage::Panel;
        let setting = self.state.ots.settings[n].clone();
        for (i, (p, o)) in self.state.keyboard_parts.iter_mut().zip(&setting.parts).enumerate() {
            if let Some(prog) = o.program {
                p.program = prog;
                // A GM voice ends a plugin picked for the part (#179).
                if self.sound.own_plugin(i) {
                    p.plugin = None;
                }
                self.sound.part_voice(i);
            }
            p.on = o.on;
            p.octave = o.octave;
            if p.volume != o.volume {
                p.waiting = panel;
            }
            p.volume = o.volume;
        }
        self.state.ots.applied = n as u8 + 1;
        // OTS turns Sync Start on (ACMP is always on): the next chord starts a stopped band.
        if !self.state.transport.running {
            self.state.transport.sync_start = true;
        }
    }

    /// Fill Up / Down / Self: a fill, then Main `target`. Stopped: selects it.
    fn fill_to(&mut self, target: u8) {
        let running = self.state.transport.running;
        let fill = FILLS[target as usize];
        let has_fill = self.has(fill);
        let t = &mut self.state.transport;
        t.main = target;
        if running {
            t.queued = Some(if has_fill { fill } else { MAINS[target as usize] }.into());
            let ots = &self.state.ots;
            if ots.link && ots.link_timing == OtsLinkTiming::Immediate && (target as usize) < ots.settings.len() {
                self.recall_ots(target as usize);
            }
        }
    }

    /// The Main next to `from` the style has, `up` or down; `from` at the end of the row.
    fn neighbour_main(&self, from: u8, up: bool) -> u8 {
        let mut j = from as i32;
        loop {
            j += if up { 1 } else { -1 };
            if !(0..4).contains(&j) {
                return from;
            }
            if self.has(MAINS[j as usize]) {
                return j as u8;
            }
        }
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
        let tempo = self.state.transport.tempo;
        self.set_style(id);
        // Change Behavior: Lock keeps, Hold keeps while playing, Reset takes the new style's.
        let running = self.state.transport.running;
        let rules = self.state.style_change;
        let resets = |r: ChangeRuleMode| r == ChangeRuleMode::Reset || (r == ChangeRuleMode::Hold && !running);
        if !resets(rules.tempo) {
            self.state.transport.tempo = tempo;
        }
        if resets(rules.parts) {
            for p in &mut self.state.mixer.style_parts {
                p.on = true;
            }
        }
        if let (false, Some(m)) = (running, rules.section_set) {
            let near = (0..4i32).flat_map(|d| [m as i32 - d, m as i32 + d]).find(|&j| (0..4).contains(&j) && self.has(MAINS[j as usize]));
            self.state.transport.main = near.map_or(m, |j| j as u8);
        }
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
        self.looper.publish(&mut self.state.looper);
        self.state.knobs = self.knobs.state(&self.knobs_now());
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
            // A keyboard solo: only that part sounds, even if it is off.
            p.sounding = match st.mixer.part_solo {
                Some(s) => s as usize == i,
                None => p.on || p.plays_bass,
            };
            p.voice_name = if p.plays_bass { "Finger Bass".into() } else { self.gm[p.program as usize].clone() };
        }
        self.sound.derive(st, &self.gm);
        self.sounds.derive(st);
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
        self.regist.fill(st);
        st.pads.pads = if st.pads.page == Page::Registration { self.regist.pads() } else { pads_for(st, st.pads.page) };
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
        let songs = self.regist.has_songs();
        let fader_page = st.mixer.fader_page;
        let mask = |bits: Vec<bool>| bits.iter().enumerate().fold(0u8, |m, (i, on)| m | (*on as u8) << i);
        let parts_on = mask(st.keyboard_parts.iter().map(|p| p.sounding).collect());
        let style_on = lk::style_lit(mask(st.mixer.style_parts.iter().map(|p| p.on).collect()), st.chord.manual_bass_active);
        let fault = st.keyboard_parts.iter().find(|p| p.selected).and_then(|p| p.plugin.as_ref()).is_some_and(|p| matches!(p.status, PluginStatus::Muted | PluginStatus::Failed));
        let looper = match st.looper.mode {
            LooperMode::Off if st.looper.has_data => lk::LooperLamp::Ready,
            LooperMode::Off => lk::LooperLamp::Empty,
            LooperMode::RecArmed => lk::LooperLamp::RecArmed,
            LooperMode::Recording => lk::LooperLamp::Recording,
            LooperMode::LoopArmed => lk::LooperLamp::LoopArmed,
            LooperMode::Looping => lk::LooperLamp::Looping,
        };
        let colours = lk::button_colours(page, styles, fader_page, parts_on, style_on, lk::PanelLamps { harmony_arp: st.harmony_arp.on, plugin_fault: fault, left_hold: st.chord.left_hold, looper });
        let act = |cc: u8, shift: bool| -> Option<AppCmd> {
            match lk::cc_control(cc, shift)? {
                Control::Page(d) => {
                    let to = page.step(d);
                    (to != page).then_some(AppCmd::Pads(PadsCmd::SetPadPage { page: to }))
                }
                Control::Act(Action::Style(_)) if !styles => None,
                Control::Act(Action::Playlist(_)) if !songs => None,
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
            ("trackPrev", lk::TRACK_LEFT_CC, "◀ STYLE", "◀ SONG"),
            ("trackNext", lk::TRACK_RIGHT_CC, "STYLE ▶", "SONG ▶"),
            ("play", lk::PLAY_CC, "PLAY", "RESET"),
            ("stop", lk::STOP_CC, "STOP", "FADE"),
            ("scene", lk::SCENE_CC, "TEMPO +", "RTG SHORT"),
            ("function", lk::FUNCTION_CC, "TEMPO -", "RTG LONG"),
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
                FaderPage::Panel if i == lk::HARM_ARP_FADER_BTN => {
                    push(id, cc, "HARM/ARP", Some(AppCmd::HarmonyArp(HarmonyArpCmd::ToggleHarmonyArp)), None)
                }
                FaderPage::Panel if i == lk::PLUGIN_FADER_BTN => {
                    push(id, cc, "PLUGIN", Some(AppCmd::Plugins(PluginCmd::ReloadPartPlugin { part: None })), None)
                }
                FaderPage::Panel if i == lk::LEFT_HOLD_FADER_BTN => push(id, cc, "L HOLD", Some(AppCmd::Chord(ChordCmd::ToggleLeftHold)), None),
                FaderPage::Panel if i == lk::LOOPER_FADER_BTN => {
                    push(id, cc, "LOOPER", Some(AppCmd::Looper(LooperCmd::LooperOnOff)), Some(("LOOP REC", Some(AppCmd::Looper(LooperCmd::LooperRec)))))
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
                self.tap_start = None;
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
                let ots = &self.state.ots;
                if running && ots.link && ots.link_timing == OtsLinkTiming::Immediate && (i as usize) < ots.settings.len() {
                    self.recall_ots(i as usize);
                }
            }
            AppCmd::Transport(TransportCmd::FillUp) => self.fill_to(self.neighbour_main(self.state.transport.main, true)),
            AppCmd::Transport(TransportCmd::FillDown) => self.fill_to(self.neighbour_main(self.state.transport.main, false)),
            AppCmd::Transport(TransportCmd::FillSelf) => {
                if running {
                    self.fill_to(self.state.transport.main)
                }
            }
            AppCmd::Transport(TransportCmd::FillBreak) => {
                if running && self.has(BREAK) {
                    self.state.transport.queued = Some(BREAK.into());
                }
            }
            AppCmd::Transport(TransportCmd::ToggleHalfBarFill) => self.state.transport.half_bar_fill = !self.state.transport.half_bar_fill,
            AppCmd::Transport(TransportCmd::SetHalfBarFill { on }) => self.state.transport.half_bar_fill = on,
            AppCmd::Transport(TransportCmd::SetStopAcmp { mode }) => {
                self.state.transport.stop_acmp_mode = mode;
                self.state.transport.stop_acmp = mode != StopAcmpMode::Off;
            }
            // The same as Fill Down / Self / Up.
            AppCmd::Transport(TransportCmd::Fill { delta }) => self.cmd(AppCmd::Transport(match delta.signum() {
                -1 => TransportCmd::FillDown,
                1 => TransportCmd::FillUp,
                _ => TransportCmd::FillSelf,
            })),
            AppCmd::Plugins(c) => self.plugin_cmd(c),
            AppCmd::Sounds(c) => self.sounds_cmd(c),
            AppCmd::Controllers(c) => {
                let before = match c {
                    ControllersCmd::SetPedal { pedal, .. } => Some((pedal as usize % PEDALS, self.controllers.pedal(pedal as usize % PEDALS))),
                    _ => None,
                };
                match c.apply_setting(&self.controllers) {
                    Ok(true) => {
                        // Kbd Harmony/Arpeggio and Arpeggio Hold on a Hold pedal (no pedal is
                        // ever down here): as the session does.
                        if let Some((i, old)) = before {
                            let sets = yahaha::controllers::control_switch_sets(old, self.controllers.pedal(i), false, false);
                            for (f, on) in sets.into_iter().flatten() {
                                if let Some(cmd) = yahaha::api::function_set(f, on) {
                                    self.cmd(cmd);
                                }
                            }
                        }
                        // No keyboard: a pedal learning "hears" the Launchkey's sustain jack.
                        if let ControllersCmd::LearnPedal { pedal: Some(p) } = c {
                            let s = self.controllers.pedal(p as usize % PEDALS);
                            self.controllers.set_pedal(p as usize % PEDALS, PedalSetup { cc: Some(64), ..s });
                            self.controllers.learn(None);
                        }
                    }
                    Ok(false) => {
                        if let ControllersCmd::TriggerFunction { function } = c {
                            let ots = self.state.ots.settings.len() as u8;
                            match function_run(function, self.state.chord.fingering, ots, self.state.ots.applied) {
                                Ok(FunctionRun::Cmd(c)) => self.cmd(c),
                                Ok(FunctionRun::Switch(b)) => self.controllers.toggle_switch(b),
                                Ok(FunctionRun::Nothing) => {}
                                Err(e) => self.message(e, true),
                            }
                        }
                    }
                    Err(e) => self.message(e, true),
                }
                self.state.controllers = ControllersState::of(&self.controllers);
            }
            AppCmd::Transport(TransportCmd::Break) => {
                if running && self.has(BREAK) {
                    self.state.transport.queued = Some(BREAK.into());
                }
            }
            AppCmd::Transport(TransportCmd::Ending { index }) => {
                let id = ENDINGS[index.min(2) as usize];
                if running && self.state.transport.section.as_deref() == Some(id) {
                    // The Ending playing, pressed again: ritardando.
                    self.state.transport.ritardando = true;
                } else if running && self.has(id) {
                    self.state.transport.queued = Some(id.into());
                }
            }
            AppCmd::Transport(TransportCmd::ToggleFade) => {
                let t = &mut self.state.transport;
                if !running {
                    t.fade = if t.fade == FadeState::Armed { FadeState::Off } else { FadeState::Armed };
                } else if t.fade != FadeState::FadingOut {
                    t.fade = FadeState::FadingOut;
                    self.fade_left = self.settings.fade_out_ms as f64;
                }
            }
            AppCmd::Transport(TransportCmd::SectionReset) => self.reset_section(),
            AppCmd::Transport(TransportCmd::ToggleRetrigger) => self.state.transport.retrigger = !self.state.transport.retrigger,
            AppCmd::Transport(TransportCmd::TapTempo) if running && self.settings.section_reset => self.reset_section(),
            AppCmd::StyleSettings(c) => {
                self.settings = c.apply(self.settings);
                self.state.style_settings = self.settings.into();
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
            AppCmd::Transport(TransportCmd::ToggleStopAcmp) => {
                let t = &mut self.state.transport;
                t.stop_acmp_mode = if t.stop_acmp_mode == StopAcmpMode::Off { StopAcmpMode::Style } else { StopAcmpMode::Off };
                t.stop_acmp = t.stop_acmp_mode != StopAcmpMode::Off;
            }
            AppCmd::Transport(TransportCmd::TapTempo) => {
                let now = self.now;
                // As the engine: taps up to 12.5 s apart count (down to 5 BPM); a jump in
                // the interval by more than half starts a fresh average from the tap before.
                if let Some(&last) = self.taps.last() {
                    if now - last > 12_500.0 {
                        self.taps.clear();
                        self.tap_run = 0;
                    } else if self.taps.len() >= 2 {
                        let r = (now - last) / (last - self.taps[self.taps.len() - 2]).max(1.0);
                        if !(1.0 / 1.5..=1.5).contains(&r) {
                            self.taps = vec![last];
                            self.tap_run = 1;
                        }
                    }
                }
                self.taps.push(now);
                self.tap_run += 1;
                if self.taps.len() > 4 {
                    self.taps.remove(0);
                }
                if self.taps.len() >= 2 {
                    let avg = (self.taps[self.taps.len() - 1] - self.taps[0]) / (self.taps.len() - 1) as f64;
                    if avg > 0.0 {
                        self.state.transport.tempo = (60000.0 / avg).round().clamp(5.0, 500.0);
                    }
                }
                // Stopped, a bar of steady taps (its quarters) starts the band a beat later.
                let bar = self.bar_quarters().floor().max(1.0) as usize;
                self.tap_start = (!running && self.tap_run >= bar).then(|| now + 60000.0 / self.state.transport.tempo);
            }
            AppCmd::Transport(TransportCmd::TempoUp) => self.state.transport.tempo = (self.state.transport.tempo + 1.0).min(500.0),
            AppCmd::Transport(TransportCmd::TempoDown) => self.state.transport.tempo = (self.state.transport.tempo - 1.0).max(5.0),
            AppCmd::Transport(TransportCmd::SetTempo { bpm }) => self.state.transport.tempo = (bpm as f64).clamp(5.0, 500.0),
            AppCmd::Mixer(MixerCmd::SetStyleSolo { part }) => self.state.mixer.style_solo = part.map(|p| p & 7),
            AppCmd::Mixer(MixerCmd::SetPartSolo { part }) => self.state.mixer.part_solo = part.map(|p| p & 3),
            AppCmd::Mixer(MixerCmd::StyleTrackMute { order, value }) => {
                let mask = order.mask(value);
                for (i, p) in self.state.mixer.style_parts.iter_mut().enumerate() {
                    p.on = mask & (1 << i) != 0;
                }
            }
            AppCmd::Looper(LooperCmd::LooperRec) => {
                if self.looper.rec(&mut self.state.looper, running) {
                    self.state.transport.sync_start = true;
                }
            }
            AppCmd::Looper(LooperCmd::LooperOnOff) => {
                // A loop about to arm turns chart mode off first (session/looper.rs).
                let l = &self.state.looper;
                if self.state.chart.on && (l.mode == LooperMode::Recording || l.mode == LooperMode::Off && l.has_data) {
                    self.set_chart_mode(false);
                }
                self.looper.on_off(&mut self.state.looper)
            }
            AppCmd::Looper(LooperCmd::SelectLooperMemory { index }) => {
                if let Err(e) = self.looper.select(&mut self.state.looper, index as usize % 8) {
                    self.message(e, true);
                }
            }
            AppCmd::Looper(LooperCmd::StoreLooperMemory { index }) => {
                if let Err(e) = self.looper.store(&mut self.state.looper, index as usize % 8) {
                    self.message(e, true);
                }
            }
            AppCmd::Looper(LooperCmd::ClearLooperMemory { index }) => self.looper.clear(&mut self.state.looper, index as usize % 8),
            AppCmd::Looper(LooperCmd::NewLooperBank) => self.looper.new_bank(&mut self.state.looper),
            AppCmd::Metronome(MetronomeCmd::ToggleMetronome) => self.state.metronome.on = !self.state.metronome.on,
            AppCmd::Metronome(MetronomeCmd::SetMetronome { on }) => self.state.metronome.on = on,
            AppCmd::Metronome(MetronomeCmd::SetMetronomeVolume { volume }) => self.state.metronome.volume = vol(volume),
            AppCmd::Metronome(MetronomeCmd::SetMetronomeBell { on }) => self.state.metronome.bell = on,
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
            AppCmd::Chord(ChordCmd::SetChordSettle { ms }) => self.state.chord.settle_ms = ms.min(yahaha::engine::CHORD_SETTLE_MAX_MS),
            AppCmd::Chord(ChordCmd::SetLeftHold { on }) => self.state.chord.left_hold = on,
            AppCmd::Chord(ChordCmd::ToggleLeftHold) => self.state.chord.left_hold = !self.state.chord.left_hold,
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
                self.gm_voice(part as usize);
            }
            AppCmd::Parts(PartsCmd::StepVoice { delta }) => {
                if let Some(i) = self.state.keyboard_parts.iter().position(|p| p.selected) {
                    let p = &mut self.state.keyboard_parts[i];
                    p.program = (p.program as i16 + delta as i16).rem_euclid(128) as u8;
                    self.gm_voice(i);
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
            AppCmd::Parts(PartsCmd::SetPartPan { part, pan }) => {
                if let Some(p) = self.state.keyboard_parts.get_mut(part as usize) {
                    p.pan = vol(pan);
                }
            }
            AppCmd::Parts(PartsCmd::SetPartSend { part, send, value }) => {
                if let Some(p) = self.state.keyboard_parts.get_mut(part as usize) {
                    match send {
                        PartSend::Reverb => p.reverb = vol(value),
                        PartSend::Chorus => p.chorus = vol(value),
                    }
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
                self.state.pads.page = Page::ALL[(i as i16 + delta as i16).rem_euclid(Page::ALL.len() as i16) as usize];
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
            AppCmd::Ots(OtsCmd::SetOtsLinkTiming { timing }) => self.state.ots.link_timing = timing,
            AppCmd::StyleChange(c) => {
                let sc = &mut self.state.style_change;
                let flip = |cur: ChangeRuleMode, to: ChangeRuleMode| if cur == ChangeRuleMode::Reset { to } else { ChangeRuleMode::Reset };
                match c {
                    StyleChangeCmd::SetTempoChange { rule } => sc.tempo = rule,
                    StyleChangeCmd::SetPartsChange { rule } => sc.parts = rule,
                    StyleChangeCmd::SetSectionSet { section } => sc.section_set = section.map(|m| m.min(3)),
                    StyleChangeCmd::ToggleStyleTempoLock => sc.tempo = flip(sc.tempo, ChangeRuleMode::Lock),
                    StyleChangeCmd::ToggleStyleTempoHold => sc.tempo = flip(sc.tempo, ChangeRuleMode::Hold),
                }
            }
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
            AppCmd::Settings(SettingsCmd::SetAudioBuffer { frames }) => match &mut self.state.io.synth {
                Some(s) if matches!(frames, 64 | 128 | 256) => s.buffer_frames = Some(frames),
                Some(_) => self.message(format!("the audio buffer is 64, 128 or 256 frames, not {frames}"), true),
                None => self.message("the synth is off", true),
            },
            AppCmd::Settings(SettingsCmd::NextAudioOutput) => {
                if let Some(s) = &mut self.state.io.synth {
                    let next = s.output_pair[1] + 1;
                    s.output_pair = if (next as u32) < s.channels { [next, next + 1] } else { [1, 2] };
                }
            }
            AppCmd::System(SystemCmd::Panic) => {
                self.stop_band();
                self.pads.panic(&mut self.state.multi_pad);
                self.controllers.reset(&mut |_| {});
                // As the session: the control-side switches a Hold pedal was keeping on go
                // off (`Control::pump_pedal_releases`).
                if let Some(down) = self.controllers.take_reset_releases() {
                    for i in 0..PEDALS {
                        let f = yahaha::controllers::reset_release(self.controllers.pedal(i), down >> i & 1 != 0);
                        if let Some(cmd) = f.and_then(|f| yahaha::api::function_set(f, false)) {
                            self.cmd(cmd);
                        }
                    }
                }
                self.state.controllers = ControllersState::of(&self.controllers);
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
            AppCmd::Settings(SettingsCmd::SetSoundFont { file }) => self.set_default_sound_set(Some(file)),
            AppCmd::Settings(SettingsCmd::SetDefaultSoundSet { file }) => self.set_default_sound_set(file),
            AppCmd::Settings(SettingsCmd::SetMidiInputs { all, names }) => {
                let io = &mut self.state.io;
                io.all_inputs = all;
                for src in &mut io.sources {
                    src.listening = src.pads || all || names.iter().any(|n| !n.is_empty() && src.name.contains(n.as_str()));
                }
                io.inputs = io.sources.iter().filter(|s| s.listening).map(|s| if s.pads { format!("{} (pads)", s.name) } else { s.name.clone() }).collect();
            }
            AppCmd::Settings(SettingsCmd::SetPaletteLeds { on }) => self.state.pads.palette_leds = on,
            AppCmd::Chart(c) => self.chart_cmd(c),
            AppCmd::ParamLock(ParamLockCmd::SetParamLock { item, on }) => self.state.param_locks.set(item, on),
            // As the session: a turn runs its function's command from the value in effect.
            AppCmd::Knobs(c) => match c {
                KnobsCmd::SetKnobPage { page } => self.knobs.set_page(page),
                KnobsCmd::StepKnobPage { delta } => self.knobs.set_page(self.knobs.page.step(delta)),
                KnobsCmd::TurnKnob { knob, delta } => {
                    let now = self.knobs_now();
                    if let Some(cmd) = self.knobs.turn(knob, delta, &now) {
                        self.cmd(cmd);
                    }
                }
            },
            AppCmd::Dynamics(c) => {
                // As the session: the command applies to the settings in effect.
                let d = &self.state.dynamics;
                let now = yahaha::engine::DynamicsSettings {
                    control: d.control,
                    level: d.level,
                    touch: d.touch,
                    accent: d.accent,
                    accent_min: d.accent_threshold,
                };
                self.state.dynamics = c.apply(now).into();
            }
            AppCmd::Registration(c) => {
                let fx = self.regist.registration_cmd(c, &self.state);
                self.run_regist(fx);
            }
            AppCmd::Playlist(c) => {
                let fx = self.regist.playlist_cmd(c, &self.state);
                self.run_regist(fx);
            }
            AppCmd::MultiPad(c) => {
                let running = self.state.transport.running;
                if let Some(e) = self.pads.cmd(&mut self.state.multi_pad, c, running) {
                    self.message(e, true);
                }
            }
            AppCmd::HarmonyArp(c) => {
                if let Err(e) = harmony_arp_cmd(&mut self.state.harmony_arp, c) {
                    self.message(&e, true);
                }
            }
            AppCmd::SoundLibrary(c) => {
                // A rule may name a catalog entry (#117): it gets that sound's library patch.
                let c = match c {
                    SoundLibraryCmd::SetFamilyRule { family, patch, style } => match self.rule_patch(patch) {
                        Ok(patch) => SoundLibraryCmd::SetFamilyRule { family, patch, style },
                        Err(e) => return self.message(e, true),
                    },
                    SoundLibraryCmd::SetProgramOverride { program, patch, style } => match self.rule_patch(patch) {
                        Ok(patch) => SoundLibraryCmd::SetProgramOverride { program, patch, style },
                        Err(e) => return self.message(e, true),
                    },
                    SoundLibraryCmd::SetDrumRule { patch, style } => match self.rule_patch(patch) {
                        Ok(patch) => SoundLibraryCmd::SetDrumRule { patch, style },
                        Err(e) => return self.message(e, true),
                    },
                    c => c,
                };
                let export = matches!(c, SoundLibraryCmd::ExportSoundLibrary { .. });
                // A SoundFont patch picked over a Plugins-tab plugin ends that plugin.
                if let SoundLibraryCmd::SetPartPatch { part, id: Some(id) } = &c
                    && self.state.sound_library.patches.iter().any(|p| &p.patch.id == id && matches!(p.patch.source, PatchSource::SoundFont { .. }))
                    && self.sound.own_plugin(*part as usize)
                {
                    self.state.keyboard_parts[(*part & 3) as usize].plugin = None;
                }
                match self.sound.cmd(&mut self.state, c) {
                    Some(e) => self.message(e, true),
                    None if export => self.message("Sound library exported to /Users/me/Documents/yahaha/sound-library-export.json", false),
                    None => {}
                }
            }
        }
    }

    /// A rule's patch: a catalog id becomes its library patch, added once (#117).
    fn rule_patch(&mut self, patch: Option<String>) -> Result<Option<String>, String> {
        let Some(id) = patch else { return Ok(None) };
        match self.sounds.patch_for(&self.state, &id)? {
            Ok(patch) => Ok(Some(patch)),
            Err(add) => {
                self.cmd(add);
                self.derive();
                Ok(self.state.sound_library.last_added.clone())
            }
        }
    }

    /// A recall's style load and Main press (OTS Link held off: the registration's voices
    /// win), then the rest of it.
    fn run_regist(&mut self, fx: Vec<Effect>) {
        let tempo = self.state.transport.tempo;
        for e in fx {
            match e {
                Effect::LoadStyle(path) => self.cmd(AppCmd::Library(LibraryCmd::LoadStylePath { path })),
                Effect::Main(index) => {
                    let link = self.state.ots.link;
                    self.state.ots.link = false;
                    self.cmd(AppCmd::Transport(TransportCmd::Main { index }));
                    self.state.ots.link = link;
                }
                Effect::Message(text, error) => self.message(text, error),
            }
        }
        self.regist.apply_pending(&mut self.state, tempo);
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
                page_pad(119, "RETRIG", "R", Some(AppCmd::Transport(TransportCmd::ToggleRetrigger)), true, t.retrigger),
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
                page_pad(101, "FADE", "F", Some(AppCmd::Transport(TransportCmd::ToggleFade)), true, t.fade != FadeState::Off),
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
        // Page 4 comes from the Registration mock (`MockRegist::pads`).
        Page::Registration | Page::MultiPads => vec![],
    }
}

/// Harmony/Arpeggio off, Standard Duet 1: the engine's defaults.
fn harmony_arp_default() -> HarmonyArpState {
    let mut h = HarmonyArpState {
        volume: 100,
        touch_limit: 1,
        arp: ArpSettings { fixed_velocity: 100, ..ArpSettings::default() },
        ..HarmonyArpState::default()
    };
    name_harmony_arp(&mut h);
    h
}

fn name_harmony_arp(h: &mut HarmonyArpState) {
    let t = match h.mode {
        HarmonyArpMode::Harmony => harmony_type_options().swap_remove(h.harmony_type as usize),
        HarmonyArpMode::Arpeggio => arp_pattern_options().swap_remove(h.arp_pattern as usize),
    };
    h.type_name = t.name;
    h.category = t.category;
}

/// The Harmony/Arpeggio commands, as src/session/harmony_arp.rs runs them (the mock plays
/// no notes).
fn harmony_arp_cmd(h: &mut HarmonyArpState, c: HarmonyArpCmd) -> Result<(), String> {
    let (types, patterns) = (harmony_type_options().len(), arp_pattern_options().len());
    match c {
        HarmonyArpCmd::ToggleHarmonyArp => h.on = !h.on,
        HarmonyArpCmd::SetHarmonyArpOn { on } => h.on = on,
        HarmonyArpCmd::SetHarmonyType { index } => {
            if index as usize >= types {
                return Err(format!("no Harmony type {index} (0-{})", types - 1));
            }
            h.mode = HarmonyArpMode::Harmony;
            h.harmony_type = index;
        }
        HarmonyArpCmd::SetArpPattern { index } => {
            if index as usize >= patterns {
                return Err(format!("no arpeggio pattern {index} (0-{})", patterns - 1));
            }
            h.mode = HarmonyArpMode::Arpeggio;
            h.arp_pattern = index;
        }
        HarmonyArpCmd::StepHarmonyArpType { delta } => {
            let n = (types + patterns) as i32;
            let cur = match h.mode {
                HarmonyArpMode::Harmony => h.harmony_type as i32,
                HarmonyArpMode::Arpeggio => (types + h.arp_pattern as usize) as i32,
            };
            let next = (cur + delta as i32).rem_euclid(n) as usize;
            if next < types {
                h.mode = HarmonyArpMode::Harmony;
                h.harmony_type = next as u8;
            } else {
                h.mode = HarmonyArpMode::Arpeggio;
                h.arp_pattern = (next - types) as u8;
            }
        }
        HarmonyArpCmd::SetHarmonyVolume { volume } => h.volume = volume.min(127),
        HarmonyArpCmd::SetHarmonySpeed { speed } => h.speed = speed,
        HarmonyArpCmd::SetHarmonyAssign { assign } => h.assign = assign,
        HarmonyArpCmd::SetChordNoteOnly { on } => h.chord_note_only = on,
        HarmonyArpCmd::SetTouchLimit { velocity } => h.touch_limit = velocity.clamp(1, 127),
        HarmonyArpCmd::SetArpQuantize { quantize } => h.arp.quantize = quantize,
        HarmonyArpCmd::SetArpHold { on } => h.arp.hold = on,
        HarmonyArpCmd::ToggleArpHold => h.arp.hold = !h.arp.hold,
        HarmonyArpCmd::SetArpPedalHold { on } => h.arp.pedal_hold = on,
        HarmonyArpCmd::ToggleArpPedalHold => h.arp.pedal_hold = !h.arp.pedal_hold,
        HarmonyArpCmd::SetArpVelocity { mode, velocity } => {
            h.arp.velocity = mode;
            // As the engine: only Fixed keeps a velocity of its own.
            h.arp.fixed_velocity = if mode == ArpVelocityMode::Fixed { velocity.clamp(1, 127) } else { 100 };
        }
        HarmonyArpCmd::SetArpKeepKeyOn { on } => h.arp.keep_key_on = on,
    }
    name_harmony_arp(h);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar_ms(m: &MockSession) -> f64 {
        60000.0 / m.state.transport.tempo * m.state.transport.beats_per_bar as f64
    }

    #[test]
    fn fade_out_stops_the_band_then_holds_and_settings_apply() {
        let mut m = MockSession::new();
        m.send(StyleSettingsCmd::SetFadeOutTime { ms: 1000 });
        m.send(StyleSettingsCmd::SetFadeHoldTime { ms: 500 });
        assert_eq!((m.state.style_settings.fade_out_ms, m.state.style_settings.fade_hold_ms), (1000, 500));
        m.send(TransportCmd::ToggleFade);
        assert_eq!(m.state.transport.fade, FadeState::FadingOut);
        m.advance(1010.0);
        assert!(!m.state.transport.running);
        assert_eq!(m.state.transport.fade, FadeState::Holding);
        m.advance(500.0);
        assert_eq!(m.state.transport.fade, FadeState::Off);
        m.send(TransportCmd::ToggleFade);
        assert_eq!(m.state.transport.fade, FadeState::Armed);
        m.send(TransportCmd::StartStop);
        assert_eq!(m.state.transport.fade, FadeState::FadingIn);
        m.send(TransportCmd::ToggleRetrigger);
        assert!(m.state.transport.retrigger);
        m.send(StyleSettingsCmd::StepRetriggerRate { delta: 1 });
        assert_eq!(m.state.style_settings.retrigger_rate, 16);
    }

    /// Stopped, a bar of steady taps starts the band a beat after the last tap, as the
    /// engine does (#195, OM p.46); STOP during the count-in calls it off.
    #[test]
    fn a_bar_of_taps_while_stopped_starts_the_band() {
        let mut m = MockSession::new();
        let beats = quarters_per_bar(m.state.style.time_signature).floor() as usize;
        let tap_bar = |m: &mut MockSession| {
            for i in 0..beats {
                if i > 0 {
                    m.advance(500.0);
                }
                m.send(TransportCmd::TapTempo);
            }
        };
        m.send(TransportCmd::Stop);
        assert!(!m.state.transport.running, "stopped");
        tap_bar(&mut m);
        assert_eq!(m.state.transport.tempo, 120.0, "{beats} taps");
        m.advance(480.0);
        assert!(!m.state.transport.running);
        m.advance(40.0);
        assert!(m.state.transport.running);
        m.send(TransportCmd::StartStop);
        m.advance(20000.0);
        tap_bar(&mut m);
        m.send(TransportCmd::Stop);
        m.advance(2000.0);
        assert!(!m.state.transport.running);
    }

    /// `setPluginInProcess` sets the list's override; the next load runs in process (#104).
    #[test]
    fn the_in_process_override_applies_from_the_next_load() {
        let mut m = MockSession::new();
        let entry = |m: &MockSession, id: &str| m.state.plugins.list.iter().find(|p| p.id == id).cloned().unwrap();
        m.send(PluginCmd::SetPluginInProcess { id: "aumu Mock Demo".into(), in_process: true });
        assert!(!entry(&m, "aumu Mock Demo").in_process, "an AUv3 that only runs out of process");
        assert!(m.state.message.as_ref().is_some_and(|x| x.error));
        m.send(PluginCmd::SetPluginInProcess { id: "aumu dls  appl".into(), in_process: true });
        assert!(entry(&m, "aumu dls  appl").in_process);
        m.send(PluginCmd::SetPluginInProcess { id: "aumu dls  appl".into(), in_process: false });
        assert!(!entry(&m, "aumu dls  appl").in_process);
    }

    /// A plugin the system won't host out of process loads in process and says so (#104).
    #[test]
    fn a_plugin_that_falls_back_in_process_says_so() {
        let mut m = MockSession::new();
        m.send(PluginCmd::SetPartPlugin { part: 1, id: MOCK_FALLBACK_ID.into(), state: None });
        let p = m.state.keyboard_parts[1].plugin.clone().unwrap();
        assert_eq!((p.status, p.out_of_process, p.in_process_fallback), (PluginStatus::Playing, false, true));
        assert!(m.state.message.as_ref().is_some_and(|x| x.text.contains("can't run in its own process") && !x.error));
        m.send(PluginCmd::SetPartPlugin { part: 1, id: "aumu dls  appl".into(), state: None });
        assert!(!m.state.keyboard_parts[1].plugin.as_ref().unwrap().in_process_fallback);
    }

    /// AUSampler plays the heavy plugin: the CPU and overrun readout has something to show.
    #[test]
    fn the_heavy_mock_plugin_reports_slow_renders() {
        let mut m = MockSession::new();
        m.send(PluginCmd::SetPartPlugin { part: 0, id: MOCK_HEAVY_ID.into(), state: None });
        let p = m.state.keyboard_parts[0].plugin.clone().unwrap();
        assert_eq!((p.recent_overruns, p.overruns), (4, 4));
        assert!(p.cpu > 0.3);
        m.send(PluginCmd::SetPartPlugin { part: 0, id: "aumu dls  appl".into(), state: None });
        assert_eq!(m.state.keyboard_parts[0].plugin.as_ref().unwrap().recent_overruns, 0);
    }

    /// `reloadPartPlugin` retries the selected part's failed plugin; Panel fader button 6
    /// is red while it needs that.
    #[test]
    fn reload_part_plugin_and_its_launchkey_button() {
        let mut m = MockSession::new();
        let b6 = |m: &MockSession| m.surface().controls.into_iter().find(|c| c.id == "faderButton6").unwrap();
        assert_eq!((b6(&m).label.as_str(), b6(&m).level), ("PLUGIN", Level::Off));
        m.send(PluginCmd::SetPartPlugin { part: 0, id: "aumu Mock Demo".into(), state: None });
        assert_eq!((b6(&m).level, b6(&m).action), (Level::Bright, Some(AppCmd::Plugins(PluginCmd::ReloadPartPlugin { part: None }))));
        m.send(PluginCmd::ReloadPartPlugin { part: None });
        assert_eq!(m.state.keyboard_parts[0].plugin.as_ref().unwrap().status, PluginStatus::Failed, "Broken Synth fails again");
        m.send(PluginCmd::ReloadPartPlugin { part: Some(1) });
        assert!(m.state.message.as_ref().is_some_and(|x| x.error && x.text.contains("SoundFont")));
    }

    /// A plugin patch on a keyboard part plays its plugin, as the session does (#109).
    #[test]
    fn a_plugin_patch_on_a_part_plays_its_plugin() {
        let mut m = MockSession::new();
        let r1 = |m: &MockSession| (m.state.keyboard_parts[0].patch.clone(), m.state.keyboard_parts[0].plugin.as_ref().map(|p| p.id.clone()));
        let dls = Some("aumu dls  appl".to_string());
        m.send(SoundLibraryCmd::SetPartPatch { part: 0, id: Some("keys-au".into()) });
        assert_eq!(r1(&m), (Some("keys-au".into()), dls.clone()));
        m.send(SoundLibraryCmd::SetPartPatch { part: 0, id: Some("stage-grand".into()) });
        assert_eq!(r1(&m), (Some("stage-grand".into()), None));
        m.send(SoundLibraryCmd::SetPartPatch { part: 0, id: Some("keys-au".into()) });
        m.send(PluginCmd::SetPartPlugin { part: 0, id: "aumu dls  appl".into(), state: None });
        assert_eq!(r1(&m), (None, dls.clone()), "a Plugins-tab plugin ends the patch");
        m.send(SoundLibraryCmd::SetPartPatch { part: 0, id: Some("stage-grand".into()) });
        assert_eq!(r1(&m), (Some("stage-grand".into()), None), "a SoundFont patch ends a Plugins-tab plugin");
        m.send(SoundLibraryCmd::SetPartPatch { part: 0, id: Some("keys-au".into()) });
        m.send(PartsCmd::SetPartVoice { part: 0, program: 0 });
        assert_eq!(r1(&m), (None, None), "a GM voice ends the plugin patch");
    }

    /// A SoundFont sound from the Sound Browser ends a plugin picked for the part: a
    /// preset from the synth's own font, another font's and a saved one (#171 review).
    #[test]
    fn a_soundfont_sound_from_the_browser_ends_a_picked_plugin() {
        let mut m = MockSession::new();
        let plugin = |m: &MockSession, p: usize| m.state.keyboard_parts[p].plugin.as_ref().map(|x| x.id.clone());
        for (part, id) in [(0u8, "sf:GeneralUser-GS.sf2:0:0"), (1, "sf:FluidR3_GM.sf2:0:48"), (2, "saved:stage-grand")] {
            m.send(SoundsCmd::AssignSound { part, id: "au:aumu dls  appl".into() });
            assert_eq!(plugin(&m, part as usize).as_deref(), Some("aumu dls  appl"));
            m.send(SoundsCmd::AssignSound { part, id: id.into() });
            assert_eq!(plugin(&m, part as usize), None, "{id} ends the plugin");
        }
        assert_eq!(m.state.keyboard_parts[0].patch, None, "the synth's own preset is the GM voice");
    }

    /// #179: a GM voice selection ends a plugin picked for the part: Voice −/+,
    /// SetPartVoice and a One Touch Setting's voice.
    #[test]
    fn a_gm_voice_selection_ends_a_picked_plugin() {
        let mut m = MockSession::new();
        let pick = |m: &mut MockSession, part: u8| {
            m.send(PluginCmd::SetPartPlugin { part, id: "aumu dls  appl".into(), state: None });
            assert!(m.state.keyboard_parts[part as usize].plugin.is_some());
        };
        pick(&mut m, 1);
        m.send(PartsCmd::SelectPart { part: 1 });
        m.send(PartsCmd::StepVoice { delta: 1 });
        assert!(m.state.keyboard_parts[1].plugin.is_none(), "Voice +");
        pick(&mut m, 2);
        m.send(PartsCmd::SetPartVoice { part: 2, program: 40 });
        assert!(m.state.keyboard_parts[2].plugin.is_none(), "SetPartVoice");
        let i = m.state.ots.settings.iter().position(|o| o.parts[0].program.is_some()).expect("an OTS with a voice");
        pick(&mut m, 0);
        m.send(OtsCmd::RecallOts { index: i as u8 });
        assert!(m.state.keyboard_parts[0].plugin.is_none(), "OTS");
    }

    /// The sound catalog (#117): every preset, plugin and saved sound; assigning routes
    /// by source, as the session does.
    #[test]
    fn the_sound_catalog_assigns_by_source() {
        let mut m = MockSession::new();
        let cat = m.sounds();
        assert_eq!(cat.entries.len() as u32, m.state.sounds.count);
        assert_eq!(cat.revision, m.state.sounds.revision);
        assert!(cat.entries.iter().any(|e| e.id == "au:aumu dls  appl" && e.plugin.is_some()));
        assert!(cat.entries.iter().any(|e| e.id == "saved:stage-grand" && e.favourite));
        m.send(SoundsCmd::AssignSound { part: 1, id: "sf:GeneralUser-GS.sf2:0:33".into() });
        assert_eq!((m.state.keyboard_parts[1].program, m.state.keyboard_parts[1].patch.clone()), (33, None));
        let n = m.state.sound_library.patches.len();
        m.send(SoundsCmd::AssignSound { part: 0, id: "sf:FluidR3_GM.sf2:0:48".into() });
        m.send(SoundsCmd::AssignSound { part: 2, id: "sf:FluidR3_GM.sf2:0:48".into() });
        assert_eq!(m.state.sound_library.patches.len(), n + 1, "added once");
        assert_eq!(m.state.keyboard_parts[0].patch, m.state.keyboard_parts[2].patch);
        m.send(SoundsCmd::AssignSound { part: 3, id: "au:aumu dls  appl".into() });
        assert_eq!(m.state.keyboard_parts[3].plugin.as_ref().map(|p| p.id.as_str()), Some("aumu dls  appl"));
        let rev = m.state.sounds.revision;
        m.send(SoundsCmd::SetSoundFavourite { id: "au:aumu dls  appl".into(), on: true });
        assert!(m.state.sounds.revision > rev);
        let cat = m.sounds();
        assert_eq!(cat.recents[0], "au:aumu dls  appl");
        assert!(cat.entries.iter().any(|e| e.id == "au:aumu dls  appl" && e.favourite && e.recent));
        m.send(TransportCmd::Stop);
        m.advance(10.0);
        assert!(!m.state.transport.running);
        m.send(SoundsCmd::AuditionSound { id: "sf:GeneralUser-GS.sf2:128:0".into() });
        assert_eq!(m.state.sounds.auditioning.as_deref(), Some("sf:GeneralUser-GS.sf2:128:0"));
        m.advance(3100.0);
        assert_eq!(m.state.sounds.auditioning, None);
    }

    /// Program map rules take catalog ids (#117): a preset or plugin becomes a patch once.
    #[test]
    fn map_rules_take_catalog_ids() {
        let mut m = MockSession::new();
        let n = m.state.sound_library.patches.len();
        m.send(SoundLibraryCmd::SetFamilyRule { family: 2, patch: Some("au:aumu samp appl".into()), style: false });
        m.send(SoundLibraryCmd::SetDrumRule { patch: Some("au:aumu samp appl".into()), style: true });
        assert_eq!(m.state.sound_library.patches.len(), n + 1);
        let id = m.state.sound_library.patches[n].patch.id.clone();
        assert_eq!(m.state.sound_library.map.families[2].as_deref(), Some(id.as_str()));
        assert_eq!(m.state.sound_library.style_map.drums.as_deref(), Some(id.as_str()));
        m.send(SoundLibraryCmd::SetProgramOverride { program: 5, patch: Some("saved:stage-grand".into()), style: false });
        assert!(m.state.sound_library.map.overrides.iter().any(|o| o.program == 5 && o.patch == "stage-grand"));
        m.send(SoundLibraryCmd::SetDrumRule { patch: Some("sf:Nope.sf2:0:0".into()), style: false });
        assert!(m.state.message.as_ref().is_some_and(|x| x.error));
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
    fn pedals_and_their_functions() {
        use yahaha::controllers::Function;
        let mut m = MockSession::new();
        m.send(ControllersCmd::SetPedal { pedal: 1, cc: Some(66), function: Function::FillUp, control_type: Default::default(), reverse: false, range: Default::default() });
        assert_eq!(m.state.controllers.pedals[1].function, Function::FillUp);
        // Playing Main B: Fill Up plays Main C's fill, then Main C.
        m.send(ControllersCmd::TriggerFunction { function: Function::FillUp });
        assert_eq!(m.state.transport.queued.as_deref(), Some("Fill In CC"));
        assert_eq!(m.state.transport.main, 2);
        m.send(ControllersCmd::TriggerFunction { function: Function::Sustain });
        assert!(m.state.controllers.sustain);
        m.send(SystemCmd::Panic);
        assert!(!m.state.controllers.sustain);
        // Registration Bank +: the REGIST BANK [+] button loads the next demo bank.
        let before = m.state.registration.bank.path.clone();
        m.send(ControllersCmd::TriggerFunction { function: Function::RegistBankNext });
        assert!(m.state.registration.bank.path.is_some());
        assert_ne!(m.state.registration.bank.path, before);
        // Regist + (#200): the demo bank's first stored button, then the next one.
        m.send(RegistrationCmd::SetRegistSequenceOn { on: false });
        m.send(ControllersCmd::TriggerFunction { function: Function::RegistNext });
        let first = m.state.registration.selected;
        assert!(first.is_some());
        m.send(ControllersCmd::TriggerFunction { function: Function::Regist1 });
        assert_eq!(m.state.registration.selected, Some(0));
    }

    #[test]
    fn harmony_arpeggio_settings_follow_the_commands() {
        let mut m = MockSession::new();
        assert_eq!(m.state.harmony_arp.type_name, "Standard Duet 1");
        assert_eq!(m.library().harmony_types.len(), 23);
        m.send(HarmonyArpCmd::ToggleHarmonyArp);
        m.send(HarmonyArpCmd::StepHarmonyArpType { delta: -1 });
        let h = &m.state.harmony_arp;
        assert!(h.on);
        assert_eq!((h.mode, h.type_name.as_str()), (HarmonyArpMode::Arpeggio, "Pluck Line"), "wraps into the arpeggios");
        m.send(HarmonyArpCmd::SetArpVelocity { mode: ArpVelocityMode::Fixed, velocity: 200 });
        assert_eq!(m.state.harmony_arp.arp.fixed_velocity, 127);
        m.send(HarmonyArpCmd::SetHarmonyType { index: 99 });
        assert!(m.state.message.as_ref().is_some_and(|x| x.error));
        // The HARMONY/ARPEGGIO switch is the button under fader 5 on the Panel fader page.
        let b5 = m.surface().controls.into_iter().find(|c| c.id == "faderButton5").unwrap();
        assert_eq!((b5.label.as_str(), b5.action, b5.level), ("HARM/ARP", Some(AppCmd::HarmonyArp(HarmonyArpCmd::ToggleHarmonyArp)), Level::Bright));
        // Kbd Harmony/Arpeggio and Arpeggio Hold are pedal functions: Try switches them, and a
        // Hold B pedal (up) holds the arpeggio at once.
        use yahaha::controllers::{ControlType, Function};
        m.send(ControllersCmd::TriggerFunction { function: Function::KbdHarmonyArp });
        assert!(!m.state.harmony_arp.on);
        assert!(!m.state.harmony_arp.arp.hold);
        m.send(ControllersCmd::SetPedal { pedal: 1, cc: Some(66), function: Function::ArpHold, control_type: ControlType::HoldB, reverse: false, range: Default::default() });
        assert!(m.state.harmony_arp.arp.pedal_hold, "the pedal function, not the setting");
        assert!(!m.state.harmony_arp.arp.hold);
        m.send(ControllersCmd::TriggerFunction { function: Function::ArpHold });
        assert!(!m.state.harmony_arp.arp.pedal_hold, "Try switches the pedal function");
        m.send(ControllersCmd::TriggerFunction { function: Function::ArpHold });
        // PANIC lets go of what a Hold pedal keeps on (Hold B, up); the setting stays.
        m.send(HarmonyArpCmd::SetArpHold { on: true });
        m.send(SystemCmd::Panic);
        assert!(!m.state.harmony_arp.arp.pedal_hold);
        assert!(m.state.harmony_arp.arp.hold);
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
            ["", "PAGE ▼", "◀ STYLE", "STYLE ▶", "PLAY", "STOP", "TEMPO +", "TEMPO -", "RIGHT 1", "RIGHT 2", "RIGHT 3", "LEFT", "HARM/ARP", "PLUGIN", "L HOLD", "LOOPER", "PANEL"]
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
        m.send(PadsCmd::SetPadPage { page: Page::MultiPads });
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
    fn registration_recalls_lights_page_4_and_the_playlist_steps() {
        let mut m = MockSession::new();
        assert_eq!(m.state.registration.bank.name, "Friday Gig");
        assert_eq!(m.state.registration.buttons.len(), 10);
        m.send(RegistrationCmd::RecallRegist { index: 3 });
        assert_eq!(m.state.registration.selected, Some(3));
        assert_eq!(m.state.transport.tempo, 132.0);
        assert_eq!(m.state.keyboard_parts[0].program, 26);
        m.send(PadsCmd::SetPadPage { page: Page::Registration });
        assert_eq!(m.state.pads.pads.len(), 16);
        assert_eq!((m.state.pads.pads[3].rgb, m.state.pads.pads[0].rgb), ([127, 0, 0], [0, 40, 127]));
        assert_eq!(m.state.pads.pads[9].level, Level::Off);
        // Memory, then button 10.
        m.send(RegistrationCmd::ToggleRegistMemory);
        assert!(m.state.pads.pads.iter().take(10).all(|p| p.anim == Anim::Flash));
        m.send(RegistrationCmd::PressRegist { index: 9 });
        assert!(m.state.registration.buttons[9].stored);
        // Shift + Track steps the playlist: its first record recalls Friday Gig [1].
        let tl = m.state.surface.controls.iter().find(|c| c.id == "trackNext").unwrap().clone();
        assert_eq!((tl.shift_label.as_str(), tl.shift_action.clone()), ("SONG ▶", Some(AppCmd::Playlist(PlaylistCmd::StepPlaylist { delta: 1 }))));
        m.send(PlaylistCmd::StepPlaylist { delta: 1 });
        assert_eq!((m.state.playlist.current, m.state.registration.selected), (Some(0), Some(0)));
        assert_eq!(m.state.transport.tempo, 72.0);
    }

    /// As the session's `harmonyArp` registrable: Memorize stores Keyboard Harmony/Arpeggio,
    /// a recall puts it back (not the pedal's Arpeggio Hold), Freeze leaves it.
    #[test]
    fn registration_stores_harmony_arpeggio() {
        let mut m = MockSession::new();
        m.send(HarmonyArpCmd::SetArpPattern { index: 4 });
        m.send(HarmonyArpCmd::SetHarmonyArpOn { on: true });
        m.send(HarmonyArpCmd::SetHarmonyVolume { volume: 60 });
        let want = m.state.harmony_arp.clone();
        m.send(RegistrationCmd::MemorizeRegist { index: 5 });
        let scramble = |m: &mut MockSession| {
            m.send(HarmonyArpCmd::SetHarmonyType { index: 1 });
            m.send(HarmonyArpCmd::SetHarmonyArpOn { on: false });
            m.send(HarmonyArpCmd::SetHarmonyVolume { volume: 100 });
        };
        scramble(&mut m);
        m.send(HarmonyArpCmd::SetArpPedalHold { on: true });
        m.send(RegistrationCmd::RecallRegist { index: 5 });
        let mut got = m.state.harmony_arp.clone();
        assert!(got.arp.pedal_hold, "the pedal's, not recalled");
        got.arp.pedal_hold = false;
        assert_eq!(got, want);
        scramble(&mut m);
        let scrambled = m.state.harmony_arp.clone();
        m.send(RegistrationCmd::SetFreezeGroup { group: yahaha::registration::Group::HarmonyArp, on: true });
        m.send(RegistrationCmd::SetFreeze { on: true });
        m.send(RegistrationCmd::RecallRegist { index: 5 });
        assert_eq!(m.state.harmony_arp, scrambled, "frozen");
    }

    /// Knob Assign pages (#197): a turn runs its function's command, as the session's.
    #[test]
    fn knobs_turn_their_functions_as_the_session() {
        let mut m = MockSession::new();
        assert_eq!((m.state.knobs.page_name.as_str(), m.state.knobs.knobs.len()), ("Style", 8));
        m.send(KnobsCmd::TurnKnob { knob: 0, delta: 4 });
        assert_eq!(m.state.dynamics.level, 72);
        assert_eq!(m.state.knobs.knobs[0].value, "72");
        let bpm = m.state.transport.tempo.round();
        m.send(KnobsCmd::TurnKnob { knob: 7, delta: -3 });
        assert_eq!(m.state.transport.tempo, bpm - 3.0);
        m.send(KnobsCmd::StepKnobPage { delta: 1 });
        let v = m.state.keyboard_parts[3].volume;
        m.send(KnobsCmd::TurnKnob { knob: 3, delta: -1 });
        assert_eq!(m.state.keyboard_parts[3].volume, v.saturating_sub(2));
        assert_eq!(m.state.knobs.page_number, 2);
    }

    /// Style Dynamics (#180): commands apply to the settings in effect and clamp, as the
    /// session's.
    #[test]
    fn dynamics_commands_apply_and_clamp_as_the_session() {
        let mut m = MockSession::new();
        assert_eq!(m.state.dynamics, DynamicsState::default());
        m.send(DynamicsCmd::SetDynamics { level: 120 });
        m.send(DynamicsCmd::StepDynamics { delta: 20 });
        m.send(DynamicsCmd::ToggleAccent);
        m.send(DynamicsCmd::SetAccentThreshold { velocity: 0 });
        m.send(DynamicsCmd::SetDynamicsTouch { on: true });
        let d = &m.state.dynamics;
        assert_eq!((d.level, d.accent, d.accent_threshold, d.touch, d.control), (127, true, 1, true, true));
    }

    /// As the session's Parameter Lock: a locked group keeps the player's setting through
    /// a recall; the other groups are recalled.
    #[test]
    fn param_lock_keeps_locked_groups_through_a_recall() {
        let mut m = MockSession::new();
        m.send(ChordCmd::SetSplit { note: 60 });
        m.send(ChordCmd::SetFingering { fingering: yahaha::fingering::Fingering::Fingered });
        m.send(RegistrationCmd::MemorizeRegist { index: 4 });
        m.send(ChordCmd::SetSplit { note: 50 });
        m.send(ChordCmd::SetFingering { fingering: yahaha::fingering::Fingering::SingleFinger });
        m.send(ParamLockCmd::SetParamLock { item: LockItem::SplitPoint, on: true });
        assert!(m.state.param_locks.split_point && !m.state.param_locks.fingering_type);
        m.send(RegistrationCmd::RecallRegist { index: 4 });
        assert_eq!(m.state.chord.split, 50, "locked");
        assert_eq!(m.state.chord.fingering, yahaha::fingering::Fingering::Fingered, "not locked");
    }

    /// Save As names files as the session does ("A:B" is "A_B") and, like the Mac's file
    /// system, ignores case: your own bank in another case is renamed, not refused.
    #[test]
    fn save_as_uses_the_session_file_names() {
        let mut m = MockSession::new();
        let save = |m: &mut MockSession, name: &str, overwrite: bool| m.send(RegistrationCmd::SaveRegistBank { name: Some(name.into()), overwrite });
        m.send(RegistrationCmd::NewRegistBank);
        save(&mut m, "A_B", false);
        m.send(RegistrationCmd::NewRegistBank);
        save(&mut m, "A:B", false);
        assert!(m.state.message.as_ref().is_some_and(|x| x.error && x.text.contains("already exists")));
        assert_eq!(m.state.registration.bank.path, None);
        m.send(RegistrationCmd::NewRegistBank);
        save(&mut m, "Mine", false);
        save(&mut m, "MINE", false);
        let r = &m.state.registration;
        assert!(r.bank.path.as_deref().is_some_and(|p| p.ends_with("/MINE.regist.json")));
        assert_eq!(r.banks.iter().filter(|b| b.name.eq_ignore_ascii_case("mine")).count(), 1);
        assert!(r.bank.position.is_some());
    }

    #[test]
    fn chords_transpose() {
        assert_eq!(transpose_chord("Am7/G", 2), "Bm7/A");
        assert_eq!(transpose_chord("C#m", -1), "Cm");
    }
}

/// The chord in effect at `pos` (a fraction) of bar `i` (held from earlier bars when it has
/// none). A chord on beat `b` of an `n`-beat bar is at `b/n` of it.
fn chord_at(song: &ChartSong, i: usize, pos: f64) -> Option<String> {
    (0..=i.min(song.bars.len().saturating_sub(1))).rev().find_map(|b| {
        let beats = song.bars[b].time[0].max(1) as f64;
        song.bars[b].chords.iter().rev().find(|c| b < i || c.beat as f64 / beats <= pos + 1e-6).map(|c| c.name.clone())
    })
}

/// The iReal chart player (#89), as engine/chart.rs plays it, bar by bar. Imports use the
/// engine's own parser; the chart state is the engine's (`yahaha::session::chart_song`).
impl MockSession {
    fn chart_playing(&self) -> bool {
        self.state.chart.on && self.state.chart.song.is_some() && self.state.transport.running
    }

    fn chart_next(&self, i: u32) -> Option<u32> {
        let c = &self.state.chart;
        let n = c.song.as_ref().map_or(0, |s| s.bars.len()) as u32;
        if let Some([a, b]) = c.loop_range.filter(|&[a, b]| a < b && b <= n) {
            if i + 1 >= b {
                return Some(a);
            }
        }
        (i + 1 < n).then_some(i + 1)
    }

    fn chart_chord(&mut self, name: Option<String>) {
        let Some(name) = name else { return };
        self.state.chord.name = Some(transpose_chord(&name, self.state.chord.transpose_keyboard));
        self.state.chord.fingered = Some(name);
    }

    /// A bar line: the chart moves on a bar (not in an Intro or Ending) and queues its
    /// next section (the mock plays fills from the next beat, so they lead in from there).
    fn chart_bar(&mut self) {
        let t = &self.state.transport;
        let Some(sec) = t.section.clone() else { return };
        if INTROS.contains(&sec.as_str()) || ENDINGS.contains(&sec.as_str()) {
            return;
        }
        let i = match self.state.chart.bar {
            None => 0,
            Some(b) => match self.chart_next(b) {
                Some(n) => n,
                None => return,
            },
        };
        self.state.chart.bar = Some(i);
        let name = self.state.chart.song.as_ref().and_then(|s| chord_at(s, i as usize, 0.0));
        self.chart_chord(name);
        let Some(n1) = self.chart_next(i) else {
            match self.state.chart.ending.map(|e| ENDINGS[e as usize % 3]).filter(|e| self.has(e)) {
                Some(e) => self.state.transport.queued = Some(e.into()),
                None => self.chart_end = true,
            }
            return;
        };
        let next = self.state.chart.song.as_ref().map(|s| (s.bars[n1 as usize].section_start, s.bars[n1 as usize].main));
        if let Some((true, main)) = next {
            let fill = FILLS[main as usize % 4];
            let t = &mut self.state.transport;
            t.main = main;
            let queued = if t.auto_fill && self.has(fill) { fill } else { MAINS[main as usize % 4] };
            self.state.transport.queued = Some(queued.into());
        }
    }

    /// Choose a song: its chart, suggested style (loaded with Auto style) and, stopped,
    /// its tempo.
    fn select_chart(&mut self, playlist: usize, song: usize, fresh: bool) {
        let Some(s) = self.chart_lists.get(playlist).and_then(|l| l.songs.get(song)).cloned() else {
            self.message(format!("no song {song} in playlist {playlist}"), true);
            return;
        };
        let c = &mut self.state.chart;
        c.selected = Some([playlist, song]);
        let view = yahaha::session::chart_song(&s, c.choruses);
        c.bar = c.bar.map(|b| b.min(view.bars.len().saturating_sub(1) as u32));
        c.song = Some(view);
        if !fresh {
            return;
        }
        c.loop_range = None;
        let words = yahaha::ireal::style_words(&s.style, &s.groove);
        let hit = self
            .library
            .entries
            .iter()
            .find(|e| e.status == "ok" && words.iter().any(|w| format!("{} {}", e.name, e.folder).to_lowercase().contains(w)))
            .map(|e| e.id);
        self.state.chart.suggested_style = hit;
        if let (true, Some(id)) = (self.state.chart.auto_style, hit) {
            if id != self.state.style.id {
                self.load_style(id);
            }
        }
        if !self.state.transport.running && s.tempo > 0 {
            self.state.transport.tempo = s.tempo as f64;
        }
    }

    fn import_charts(&mut self, text: &str) {
        let lists = match yahaha::ireal::parse(text) {
            Ok(l) => l,
            Err(e) => return self.message(format!("iReal import: {e:#}"), true),
        };
        let first = self.chart_lists.len();
        let songs: usize = lists.iter().map(|l| l.songs.len()).sum();
        for (i, l) in lists.into_iter().enumerate() {
            let name = l.name.clone().filter(|n| !n.trim().is_empty()).unwrap_or_else(|| match l.songs.as_slice() {
                [one] => one.title.clone(),
                _ => format!("Playlist {}", first + i + 1),
            });
            self.state.chart.playlists.push(ChartPlaylist {
                name,
                songs: l
                    .songs
                    .iter()
                    .map(|s| ChartSongInfo {
                        title: s.title.clone(),
                        composer: s.composer.clone(),
                        style: s.style.clone(),
                        key: s.key.clone(),
                        tempo: (s.tempo > 0).then_some(s.tempo),
                    })
                    .collect(),
            });
            self.chart_lists.push(l);
        }
        if self.state.chart.selected.is_none() && self.chart_lists.get(first).is_some_and(|l| !l.songs.is_empty()) {
            self.select_chart(first, 0, true);
        }
        self.message(format!("Imported {songs} song{}", if songs == 1 { "" } else { "s" }), false);
    }

    fn chart_cmd(&mut self, cmd: ChartCmd) {
        match cmd {
            ChartCmd::ImportCharts { text } => self.import_charts(&text),
            ChartCmd::ImportChartFile { path } => match std::fs::read_to_string(&path) {
                Ok(text) => self.import_charts(&text),
                Err(e) => self.message(format!("{path}: {e}"), true),
            },
            ChartCmd::SelectChart { playlist, song } => self.select_chart(playlist, song, true),
            ChartCmd::StepChart { delta } => {
                if let Some([p, s]) = self.state.chart.selected {
                    let n = self.state.chart.playlists[p].songs.len() as i64;
                    let to = (s as i64 + delta as i64).clamp(0, n - 1) as usize;
                    if to != s {
                        self.select_chart(p, to, true);
                    }
                }
            }
            ChartCmd::RemoveChartPlaylist { playlist } => {
                if playlist < self.chart_lists.len() {
                    self.chart_lists.remove(playlist);
                    let c = &mut self.state.chart;
                    c.playlists.remove(playlist);
                    match c.selected {
                        Some([p, _]) if p == playlist => {
                            *c = ChartState { playlists: std::mem::take(&mut c.playlists), choruses: c.choruses, ..ChartState::default() }
                        }
                        Some([p, s]) if p > playlist => c.selected = Some([p - 1, s]),
                        _ => {}
                    }
                }
            }
            ChartCmd::SetChartMode { on } => self.set_chart_mode(on),
            ChartCmd::ToggleChartMode => self.set_chart_mode(!self.state.chart.on),
            ChartCmd::SetChartChoruses { choruses } => {
                self.state.chart.choruses = choruses.clamp(1, 99);
                if let Some([p, s]) = self.state.chart.selected {
                    self.select_chart(p, s, false);
                }
                // Fewer choruses: a loop past the new end goes.
                let n = self.state.chart.song.as_ref().map_or(0, |s| s.bars.len()) as u32;
                if self.state.chart.loop_range.is_some_and(|[_, b]| b > n) {
                    self.state.chart.loop_range = None;
                }
            }
            ChartCmd::SetChartLoop { range } => {
                let n = self.state.chart.song.as_ref().map_or(0, |s| s.bars.len()) as u32;
                match range {
                    Some([a, b]) if !(a < b && b <= n) => self.message(format!("no bars {}-{b} in the chart", a + 1), true),
                    r => self.state.chart.loop_range = r,
                }
            }
            ChartCmd::SetChartIntro { index } => self.state.chart.intro = index.map(|i| i.min(2)),
            ChartCmd::SetChartEnding { index } => self.state.chart.ending = index.map(|i| i.min(2)),
            ChartCmd::SetChartAutoStyle { on } => self.state.chart.auto_style = on,
        }
    }

    fn set_chart_mode(&mut self, on: bool) {
        if on && self.state.chart.song.is_none() {
            return self.message("Import an iReal Pro chart first", true);
        }
        // Only one of the chart and the Chord Looper gives the chords: chart mode on stops
        // a loop (engine/chart.rs).
        if on && matches!(self.state.looper.mode, LooperMode::Looping | LooperMode::LoopArmed) {
            self.looper.on_off(&mut self.state.looper);
        }
        self.state.chart.on = on;
        if !on {
            self.state.chart.bar = None;
        }
    }
}

/// The mock plugin the system won't host out of process: it loads in process instead (as
/// app/src/lib/api/mock-plugins.ts).
const MOCK_FALLBACK_ID: &str = "aumu Tiny Demo";

/// The mock plugin that plays heavy: a high CPU share and a few slow renders (as
/// app/src/lib/api/mock-plugins.ts).
const MOCK_HEAVY_ID: &str = "aumu samp appl";

/// The mock's installed plugins: Apple's built-in instruments, one made-up synth that
/// always fails to load, and one that falls back to loading in process (as
/// app/src/lib/api/mock-plugins.ts).
fn mock_plugins() -> PluginsState {
    let e = |id: &str, name: &str, manufacturer: &str, format: &str, last_error: Option<&str>| PluginEntry {
        id: id.into(),
        name: name.into(),
        manufacturer: manufacturer.into(),
        version: if manufacturer == "Apple" { "1.0.0" } else { "0.9.0" }.into(),
        format: format.into(),
        last_error: last_error.map(Into::into),
        in_process: false,
        can_run_in_process: format == "AUv2",
    };
    PluginsState {
        available: true,
        scanning: false,
        list: vec![
            e("aumu dls  appl", "DLSMusicDevice", "Apple", "AUv2", None),
            e("aumu samp appl", "AUSampler", "Apple", "AUv2", None),
            e("aumu Mock Demo", "Broken Synth", "Example Audio", "AUv3", Some("timed out after 20.0 s")),
            e(MOCK_FALLBACK_ID, "Tiny Synth", "Example Audio", "AUv2", None),
        ],
    }
}

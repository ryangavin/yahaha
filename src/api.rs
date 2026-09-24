//! The app API: what a client (the terminal UI, the desktop app) sends to a
//! [`Session`](crate::Session) and what it reads back. docs/app-api.md is the contract.
//!
//! - [`AppCmd`]: every user action, from the screen, the computer keyboard or the
//!   Launchkey. `Session::send` runs it.
//! - [`AppState`]: everything a front panel shows, as one plain snapshot. `Session::state`
//!   returns the latest; its `version` changes whenever anything in it does.
//! - [`Event`]: a cheap "something changed" notification (`Session::subscribe`).
//!
//! All three serialize with serde (JSON: camelCase fields, `type`-tagged commands).

use crate::engine::Button;
use crate::fingering::Fingering;
use crate::launchkey::{Action, Anim, Level, Page};
use crate::parts::FaderPage;
use crate::theory::NOTE_NAMES;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Every user action. Indices are 0-based. Keyboard parts: 0 = Right 1, 1 = Right 2,
/// 2 = Right 3, 3 = Left. Style parts: 0-7 = Rhythm 1, Rhythm 2, Bass, Chord 1, Chord 2,
/// Pad, Phrase 1, Phrase 2 (MIDI channels 9-16).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum AppCmd {
    // --- Sections and transport (the Genos panel buttons; engine state) ---
    /// Intro 1-3 (`index` 0-2). Stopped: the intro plays when the style starts.
    /// Playing: queued for the next bar.
    Intro { index: u8 },
    /// Main A-D (`index` 0-3). Pressing the Main that is playing plays its fill.
    Main { index: u8 },
    /// Break (Fill In BA).
    Break,
    /// Ending 1-3 (`index` 0-2): ends the style after the ending.
    Ending { index: u8 },
    /// START/STOP.
    StartStop,
    /// Stop (the Launchkey Stop button): stops if playing, else nothing.
    Stop,
    /// SYNC START on/off.
    ToggleSyncStart,
    /// SYNC STOP on/off. The engine ignores it while `transport.syncStopAvailable` is false.
    ToggleSyncStop,
    /// AUTO FILL IN on/off.
    ToggleAutoFill,
    /// STOP ACMP on/off.
    ToggleStopAcmp,
    /// TAP TEMPO: taps set the tempo; stopped, four taps start the style.
    TapTempo,
    /// Tempo up/down one step.
    TempoUp,
    TempoDown,
    /// Mute/unmute a Style part (`part` 0-7).
    ToggleStylePart { part: u8 },
    /// Set a Style part's volume (its CC7, 0-127). The Launchkey fader picks it up.
    SetStylePartVolume { part: u8, volume: u8 },

    // --- Chord detection, split, transpose ---
    /// Select a fingering type.
    SetFingering { fingering: Fingering },
    /// Step to the next fingering type (display order, wrapping).
    NextFingering,
    /// Chord Detection Area: Upper (true) or Lower. Selecting Upper turns Manual Bass on.
    SetUpper { on: bool },
    ToggleUpper,
    /// The Manual Bass setting. Only changes in Upper (ignored in Lower).
    SetManualBass { on: bool },
    ToggleManualBass,
    /// Split point, a MIDI note (clamped to 24-96). Keys at or below it are the left hand.
    SetSplit { note: u8 },
    /// Move the split point by `delta` keys.
    MoveSplit { delta: i8 },
    /// Keyboard and Master transpose in semitones (each clamped to -12..=12).
    SetTranspose { keyboard: i8, master: i8 },
    /// Add to the Keyboard and Master transpose.
    StepTranspose { keyboard: i8, master: i8 },
    /// Keyboard and Master transpose back to 0.
    ResetTranspose,

    // --- Keyboard parts (Right 1-3, Left) ---
    /// Turn a part on/off. Left is refused under Manual Bass (it plays the bass then).
    SetPartOn { part: u8, on: bool },
    TogglePart { part: u8 },
    /// The part the voice commands (`StepVoice`) and the Launchkey voice pads edit.
    SelectPart { part: u8 },
    /// Set a part's voice (GM program 0-127).
    SetPartVoice { part: u8, program: u8 },
    /// Previous/next voice for the selected part.
    StepVoice { delta: i8 },
    /// A part's volume (its CC7, 0-127). The Launchkey fader picks it up.
    SetPartVolume { part: u8, volume: u8 },
    /// A part's octave shift (-2..=2).
    SetPartOctave { part: u8, octave: i8 },

    // --- Mixer and Launchkey pages ---
    /// What the Launchkey faders control: the keyboard parts (Panel) or the Style parts.
    SetFaderPage { page: FaderPage },
    ToggleFaderPage,
    /// The Launchkey pad page.
    SetPadPage { page: Page },
    /// Step the pad page by `delta`, wrapping (the terminal's Tab / Shift+Tab).
    CyclePadPage { delta: i8 },
    /// The built-in synth's master volume (0-127; 100 = unity). The master fader picks it up.
    SetMasterVolume { volume: u8 },

    // --- One Touch Settings ---
    /// Recall One Touch Setting 1-4 (`index` 0-3) into the keyboard parts.
    RecallOts { index: u8 },
    /// OTS Link: Main A-D recall OTS 1-4.
    SetOtsLink { on: bool },
    ToggleOtsLink,

    // --- Styles ---
    /// Load a style from the library by entry id (`LibraryEntry::id`). Playing or stopped.
    LoadStyle { id: usize },
    /// Load a style file by path (added to the library if it isn't in it).
    LoadStylePath { path: String },
    /// Previous/next style in library order, skipping files that don't load.
    StepStyle { delta: i8 },

    // --- Output ---
    /// Mute/unmute the built-in synth's audio.
    SetSynthMuted { on: bool },
    ToggleSynthMute,
    /// The synth's stereo output pair, by its left channel (0-based; 0 = outputs 1/2).
    SetAudioOutput { first: u8 },
    /// Next stereo output pair, wrapping: 1/2 -> 3/4 -> ... -> 1/2.
    NextAudioOutput,
    /// All notes off, the style stops.
    Panic,
    /// Clear `AppState::message`.
    ClearMessage,
}

impl From<Button> for AppCmd {
    fn from(b: Button) -> AppCmd {
        match b {
            Button::Intro(i) => AppCmd::Intro { index: i },
            Button::Main(i) => AppCmd::Main { index: i },
            Button::Break => AppCmd::Break,
            Button::Ending(i) => AppCmd::Ending { index: i },
            Button::StartStop => AppCmd::StartStop,
            Button::Stop => AppCmd::Stop,
            Button::SyncStart => AppCmd::ToggleSyncStart,
            Button::SyncStop => AppCmd::ToggleSyncStop,
            Button::AutoFill => AppCmd::ToggleAutoFill,
            Button::TapTempo => AppCmd::TapTempo,
            Button::TempoUp => AppCmd::TempoUp,
            Button::TempoDown => AppCmd::TempoDown,
            Button::TogglePart(p) => AppCmd::ToggleStylePart { part: p },
            Button::StopAcmp => AppCmd::ToggleStopAcmp,
        }
    }
}

impl AppCmd {
    /// The engine button this command is, if it is one.
    pub fn button(&self) -> Option<Button> {
        Some(match *self {
            AppCmd::Intro { index } => Button::Intro(index.min(3)),
            AppCmd::Main { index } => Button::Main(index.min(3)),
            AppCmd::Break => Button::Break,
            AppCmd::Ending { index } => Button::Ending(index.min(3)),
            AppCmd::StartStop => Button::StartStop,
            AppCmd::Stop => Button::Stop,
            AppCmd::ToggleSyncStart => Button::SyncStart,
            AppCmd::ToggleSyncStop => Button::SyncStop,
            AppCmd::ToggleAutoFill => Button::AutoFill,
            AppCmd::TapTempo => Button::TapTempo,
            AppCmd::TempoUp => Button::TempoUp,
            AppCmd::TempoDown => Button::TempoDown,
            AppCmd::ToggleStylePart { part } => Button::TogglePart(part & 7),
            AppCmd::ToggleStopAcmp => Button::StopAcmp,
            _ => return None,
        })
    }
}

/// A Launchkey pad or button action is the command its keyboard shortcut sends.
impl From<Action> for AppCmd {
    fn from(a: Action) -> AppCmd {
        match a {
            Action::Button(b) => b.into(),
            Action::Fingering(f) => AppCmd::SetFingering { fingering: f },
            Action::NextFingering => AppCmd::NextFingering,
            Action::ToggleUpper => AppCmd::ToggleUpper,
            Action::ToggleManualBass => AppCmd::ToggleManualBass,
            Action::Split(d) => AppCmd::MoveSplit { delta: d },
            Action::Transpose { keyboard, master } => AppCmd::StepTranspose { keyboard, master },
            Action::TransposeReset => AppCmd::ResetTranspose,
            Action::Ots(n) => AppCmd::RecallOts { index: n },
            Action::ToggleOtsLink => AppCmd::ToggleOtsLink,
            Action::PartOnOff(p) => AppCmd::TogglePart { part: p },
            Action::SelectPart(p) => AppCmd::SelectPart { part: p },
            Action::PartVoice(d) => AppCmd::StepVoice { delta: d },
            Action::ToggleFaderPage => AppCmd::ToggleFaderPage,
            Action::Style(d) => AppCmd::StepStyle { delta: d },
        }
    }
}

/// Why `Session::send` did not run a command.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "message", rename_all = "camelCase")]
pub enum CmdError {
    /// The engine's queue is full for a moment: nothing changed, try again.
    Busy,
    /// The command was refused or failed; the text says why (it is also `AppState::message`).
    Failed(String),
}

impl std::fmt::Display for CmdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CmdError::Busy => write!(f, "busy, try again"),
            CmdError::Failed(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for CmdError {}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Notifications from `Session::subscribe`. They carry no data: read `Session::state()`
/// (or `Session::library()`) when one arrives. Several changes may share one event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Event {
    /// `AppState` changed; `version` is the new `AppState::version`.
    StateChanged { version: u64 },
    /// The style library changed (indexing progress, a file that failed to load, a style
    /// added by path); `revision` is the new `LibraryStatus::revision`.
    LibraryChanged { revision: u64 },
    /// The session stopped.
    Stopped,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// Everything a front panel shows. Plain data: clone it, serialize it, compare it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    /// Changes whenever anything else in the state does (never goes back).
    pub version: u64,
    pub style: StyleState,
    pub transport: TransportState,
    pub chord: ChordState,
    /// Right 1, Right 2, Right 3, Left (always 4).
    pub keyboard_parts: Vec<KeyboardPart>,
    pub mixer: MixerState,
    pub pads: PadsState,
    pub ots: OtsState,
    pub library: LibraryStatus,
    pub io: IoState,
    /// The last notice or error, until the next one or `ClearMessage`.
    pub message: Option<Message>,
}

/// The loaded style.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleState {
    /// Its library entry id (`LibraryEntry::id`).
    pub id: usize,
    pub path: String,
    /// The style's name (its SFF name, else the file name).
    pub name: String,
    /// "SFF1" or "SFF2".
    pub format: String,
    /// The style's own tempo, in BPM (the current tempo is `transport.tempo`).
    pub tempo: f64,
    /// Time signature, e.g. [4, 4].
    pub time_signature: [u8; 2],
    /// Names of the sections the style has, e.g. "Intro A", "Main B", "Fill In AA",
    /// "Fill In BA" (Break), "Ending C".
    pub sections: Vec<String>,
}

/// Playback state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransportState {
    pub running: bool,
    /// Sync Start is armed: the first chord starts the style.
    pub sync_start: bool,
    pub sync_stop: bool,
    /// Sync Stop can be turned on (not in the Full Keyboard fingering types in Lower).
    pub sync_stop_available: bool,
    pub auto_fill: bool,
    pub stop_acmp: bool,
    /// The section playing (None when stopped), e.g. "Main A", "Fill In AA".
    pub section: Option<String>,
    /// The section queued to play next (at the next bar; a fill at the next beat).
    pub queued: Option<String>,
    /// The Intro (0-2) armed to play when the style starts.
    pub pending_intro: Option<u8>,
    /// The Main section (0-3 = A-D) the style is on, or returns to after a fill.
    pub main: u8,
    /// Position in the section playing: bar and beat, both 1-based (1, 1 when stopped).
    pub bar: u32,
    pub beat: u32,
    /// Beats per bar (the time signature's numerator).
    pub beats_per_bar: u8,
    /// Current tempo in BPM.
    pub tempo: f64,
    /// Page 1 of the Launchkey pads (sections, Sync Start/Stop, Auto Fill, Tap, Start/Stop),
    /// whatever page the hardware is on: the section lamps exactly as the pads show them.
    pub lamps: Vec<Pad>,
}

/// Chord detection.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChordState {
    /// The chord the style follows (after Keyboard transpose), e.g. "Am7/G".
    pub name: Option<String>,
    /// The chord as fingered, before Keyboard transpose.
    pub fingered: Option<String>,
    pub fingering: Fingering,
    /// Display name, e.g. "Fingered On Bass".
    pub fingering_name: String,
    /// Chord Detection Area = Upper (the chord comes from the right hand, as Fingered*).
    pub upper: bool,
    /// The Manual Bass setting.
    pub manual_bass: bool,
    /// Manual Bass in effect (Upper and the setting on): the left hand plays the Style's
    /// Bass voice and the Style's Bass part is muted.
    pub manual_bass_active: bool,
    /// Split point, a MIDI note: keys at or below it are the left hand.
    pub split: u8,
    /// The split point in Yamaha octave numbering (C3 = 60), e.g. "F#2".
    pub split_name: String,
    /// Keyboard transpose (the keys and the chord), semitones -12..=12.
    pub transpose_keyboard: i8,
    /// Master transpose (everything that sounds but drum kits), semitones -12..=12.
    pub transpose_master: i8,
}

/// A keyboard part: Right 1, Right 2, Right 3 or Left.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardPart {
    /// "Right 1", "Right 2", "Right 3", "Left".
    pub name: String,
    /// MIDI channel, 1-based (Right 1 = 1, Left = 2, Right 2 = 3, Right 3 = 4).
    pub channel: u8,
    /// The part's on/off switch.
    pub on: bool,
    /// It sounds: on, or Left playing the bass under Manual Bass.
    pub sounding: bool,
    /// The part the voice commands edit.
    pub selected: bool,
    /// Volume (its CC7), 0-127.
    pub volume: u8,
    /// The Launchkey fader has moved but not yet reached `volume` (soft takeover).
    pub waiting: bool,
    /// The part's own voice, a GM program 0-127.
    pub program: u8,
    /// What its channel plays: its voice, or the Style's Bass voice under Manual Bass.
    pub voice_name: String,
    /// Left playing the Style's Bass voice (Manual Bass).
    pub plays_bass: bool,
    /// The octave setting, -2..=2 (not applied while `plays_bass`).
    pub octave: i8,
}

/// The mixer: the Style parts, the fader page, the master volume.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MixerState {
    /// What the Launchkey faders 1-8 control.
    pub fader_page: FaderPage,
    /// The 8 Style parts.
    pub style_parts: Vec<StylePart>,
    /// The built-in synth's master volume (0-127, 100 = unity). None without the synth.
    pub master: Option<u8>,
    /// The Launchkey master fader has moved but not yet reached `master`.
    pub master_waiting: bool,
}

/// One of the 8 accompaniment parts.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StylePart {
    /// "Rhythm 1", "Rhythm 2", "Bass", "Chord 1", "Chord 2", "Pad", "Phrase 1", "Phrase 2".
    pub name: String,
    /// MIDI channel, 1-based (9-16).
    pub channel: u8,
    /// Not muted (and not muted by Manual Bass).
    pub on: bool,
    /// The Bass part, muted because Manual Bass is in effect.
    pub muted_by_manual_bass: bool,
    /// Volume (its CC7), 0-127.
    pub volume: u8,
    /// The Launchkey fader has moved but not yet reached `volume`.
    pub waiting: bool,
    /// The voice the style was written for.
    pub voice: Option<Voice>,
}

/// A Yamaha voice as the style names it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Voice {
    pub bank_msb: u8,
    pub bank_lsb: u8,
    /// Program, 0-based.
    pub program: u8,
    /// A drum or SFX kit.
    pub kit: bool,
    /// What the built-in synth plays for it, e.g. "Finger Bass (GM 34)",
    /// "≈ Strings  [Yamaha 104/0/49]", "drum kit 127/0/1".
    pub label: String,
}

/// The Launchkey pads.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PadsState {
    pub page: Page,
    /// "Sections", "Chord/Setup", "OTS/Parts".
    pub page_name: String,
    /// 1-based page number, and how many pages there are.
    pub page_number: u8,
    pub page_count: u8,
    /// The 16 pads on this page: the top row (notes 96-103) then the bottom row (112-119).
    pub pads: Vec<Pad>,
    /// A Launchkey is connected (DAW port).
    pub connected: bool,
}

/// One pad: what it does and how it is lit.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pad {
    /// The pad's MIDI note on the Launchkey DAW port.
    pub note: u8,
    /// e.g. "MAIN A", "FINGERED", "OTS 1"; empty for an unused pad.
    pub label: String,
    /// The terminal UI's keyboard shortcut, e.g. "1", "spc", "F10".
    pub key: String,
    /// Full-brightness colour, 0-127 per channel.
    pub rgb: [u8; 3],
    pub level: Level,
    pub anim: Anim,
    /// What pressing it sends (None: an unused pad).
    pub action: Option<AppCmd>,
}

/// One Touch Settings.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtsState {
    /// The style's One Touch Settings (0-4).
    pub settings: Vec<OtsSetting>,
    /// The last one recalled, 1-based (0 = none since the style loaded).
    pub applied: u8,
    pub link: bool,
}

/// One One Touch Setting (the style has no names for them: "OTS 1".."OTS 4").
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtsSetting {
    /// "OTS 1".."OTS 4".
    pub name: String,
    /// Right 1, Right 2, Right 3, Left as it sets them.
    pub parts: Vec<OtsPart>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtsPart {
    pub on: bool,
    /// GM program, or None for a drum kit voice (the part keeps its own).
    pub program: Option<u8>,
    pub voice_name: String,
    pub volume: u8,
    pub octave: i8,
}

/// The style library. The entries themselves come from `Session::library()`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStatus {
    /// Changes whenever the library does (see `Event::LibraryChanged`).
    pub revision: u64,
    pub count: usize,
    /// The loaded style's position in library order (0-based).
    pub position: usize,
    /// Entries still waiting to be indexed.
    pub pending: usize,
}

/// A library entry (`Session::library_list`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryEntry {
    /// Stable for the session: `AppCmd::LoadStyle { id }`.
    pub id: usize,
    pub name: String,
    /// Folder relative to the scanned root, `/`-separated (the category).
    pub folder: String,
    pub path: String,
    /// "pending" (not indexed yet), "ok", or "error".
    pub status: String,
    /// Why it doesn't load (status "error").
    pub error: Option<String>,
    pub tempo: Option<f64>,
    pub time_signature: Option<[u8; 2]>,
    /// Short section list, e.g. "Main ABCD · Intro ABC · Ending ABC · Fill ABCD · Break".
    pub sections: String,
}

/// The library in display order (folder, then name).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryList {
    pub revision: u64,
    pub entries: Vec<LibraryEntry>,
}

/// MIDI and audio.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IoState {
    /// The virtual MIDI output the band and your playing go out on ("yahaha").
    pub output_port: String,
    /// The MIDI sources connected as inputs (a Launchkey DAW port shows "(pads)").
    pub inputs: Vec<String>,
    /// The built-in synth, when it runs.
    pub synth: Option<SynthState>,
    pub engine: EngineStats,
    /// The last message from the Launchkey DAW port, packed 0x00SSDDVV (0 = none).
    pub last_control: u32,
    /// The last Launchkey note or CC that nothing is mapped to, e.g. "unmapped CC 103 = 127".
    pub unmapped: String,
    /// An offline session (no MIDI, no audio; tests and the app's dev mode).
    pub offline: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SynthState {
    /// The SoundFont's name.
    pub sound_font: String,
    /// The audio device.
    pub device: String,
    pub sample_rate: u32,
    /// Buffer size in frames (None: the device default).
    pub buffer_frames: Option<u32>,
    /// Output channels the device has.
    pub channels: u32,
    /// The stereo pair it plays on, 1-based, e.g. [1, 2].
    pub output_pair: [u8; 2],
    pub muted: bool,
}

/// Real-time health.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineStats {
    /// The engine thread got the real-time scheduling policy.
    pub realtime: bool,
    /// 99th percentiles, in µs (upper bounds): engine wake vs. its deadline, chord
    /// published -> applied by the engine, MIDI packet timestamp -> our input callback.
    pub wake_p99_us: u32,
    pub chord_p99_us: u32,
    pub midi_in_p99_us: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    /// Increases with every new message (so the same text twice is two messages).
    pub seq: u64,
    pub text: String,
    pub error: bool,
}

// ---------------------------------------------------------------------------
// Names
// ---------------------------------------------------------------------------

/// A MIDI note in Yamaha octave numbering (C3 = MIDI 60), as on the Genos: "F#2".
pub fn note_name(n: u8) -> String {
    format!("{}{}", NOTE_NAMES[n as usize % 12], n as i32 / 12 - 2)
}

pub const STYLE_PART_NAMES: [&str; 8] = ["Rhythm 1", "Rhythm 2", "Bass", "Chord 1", "Chord 2", "Pad", "Phrase 1", "Phrase 2"];

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

/// A style voice on MIDI channel `dest` (0-based), as the built-in synth plays it.
pub fn voice_label(dest: u8, v: Option<(u8, u8, u8)>) -> String {
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

/// "unmapped CC 103 = 127" for a packed 0x01SSDDVV (empty for 0).
pub fn unmapped_text(packed: u32) -> String {
    let [valid, st, d1, d2] = packed.to_be_bytes();
    if valid == 0 {
        return String::new();
    }
    let kind = if st & 0xF0 == 0xB0 { "CC" } else { "note" };
    let ch = if st & 0x0F != 0 { format!(" (ch {})", (st & 0x0F) + 1) } else { String::new() };
    format!("unmapped {kind} {d1} = {d2}{ch}")
}

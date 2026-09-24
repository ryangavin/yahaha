//! The app API as the shell serves it: `AppCmd`, `AppState`, `LibraryList`, `Event`.
//!
//! A stand-in copy of #16's `src/api.rs` (draft PR #71, docs/app-api.md), same JSON
//! field for field: camelCase fields, `type`-tagged commands and events. When #16 lands,
//! delete this file and `use yahaha::{AppCmd, AppState, ...}` instead; `lib.rs`'s commands
//! keep their signatures.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Fingering {
    SingleFinger,
    MultiFinger,
    Fingered,
    FingeredOnBass,
    AiFingered,
    FullKeyboard,
    AiFullKeyboard,
}

impl Fingering {
    /// Display order (and pad order on page 2).
    pub const ALL: [Fingering; 7] = [
        Fingering::SingleFinger,
        Fingering::Fingered,
        Fingering::FingeredOnBass,
        Fingering::MultiFinger,
        Fingering::AiFingered,
        Fingering::FullKeyboard,
        Fingering::AiFullKeyboard,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Fingering::SingleFinger => "Single Finger",
            Fingering::MultiFinger => "Multi Finger",
            Fingering::Fingered => "Fingered",
            Fingering::FingeredOnBass => "Fingered On Bass",
            Fingering::AiFingered => "AI Fingered",
            Fingering::FullKeyboard => "Full Keyboard",
            Fingering::AiFullKeyboard => "AI Full Keyboard",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Page {
    Sections,
    ChordSetup,
    OtsParts,
}

impl Page {
    pub const ALL: [Page; 3] = [Page::Sections, Page::ChordSetup, Page::OtsParts];
    pub fn name(self) -> &'static str {
        ["Sections", "Chord/Setup", "OTS/Parts"][self as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FaderPage {
    Panel,
    Style,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Level {
    Off,
    Dim,
    Bright,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Anim {
    Solid,
    Flash,
    Pulse,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum AppCmd {
    Intro { index: u8 },
    Main { index: u8 },
    Break,
    Ending { index: u8 },
    StartStop,
    Stop,
    ToggleSyncStart,
    ToggleSyncStop,
    ToggleAutoFill,
    ToggleStopAcmp,
    TapTempo,
    TempoUp,
    TempoDown,
    ToggleStylePart { part: u8 },
    SetStylePartVolume { part: u8, volume: u8 },
    SetFingering { fingering: Fingering },
    NextFingering,
    SetUpper { on: bool },
    ToggleUpper,
    SetManualBass { on: bool },
    ToggleManualBass,
    SetSplit { note: u8 },
    MoveSplit { delta: i8 },
    SetTranspose { keyboard: i8, master: i8 },
    StepTranspose { keyboard: i8, master: i8 },
    ResetTranspose,
    SetPartOn { part: u8, on: bool },
    TogglePart { part: u8 },
    SelectPart { part: u8 },
    SetPartVoice { part: u8, program: u8 },
    StepVoice { delta: i8 },
    SetPartVolume { part: u8, volume: u8 },
    SetPartOctave { part: u8, octave: i8 },
    SetFaderPage { page: FaderPage },
    ToggleFaderPage,
    SetPadPage { page: Page },
    CyclePadPage { delta: i8 },
    SetMasterVolume { volume: u8 },
    RecallOts { index: u8 },
    SetOtsLink { on: bool },
    ToggleOtsLink,
    LoadStyle { id: usize },
    LoadStylePath { path: String },
    StepStyle { delta: i8 },
    SetSynthMuted { on: bool },
    ToggleSynthMute,
    SetAudioOutput { first: u8 },
    NextAudioOutput,
    Panic,
    ClearMessage,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "message", rename_all = "camelCase")]
pub enum CmdError {
    Busy,
    Failed(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Event {
    StateChanged { version: u64 },
    LibraryChanged { revision: u64 },
    Stopped,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub version: u64,
    pub style: StyleState,
    pub transport: TransportState,
    pub chord: ChordState,
    pub keyboard_parts: Vec<KeyboardPart>,
    pub mixer: MixerState,
    pub pads: PadsState,
    pub ots: OtsState,
    pub library: LibraryStatus,
    pub io: IoState,
    pub message: Option<Message>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleState {
    pub id: usize,
    pub path: String,
    pub name: String,
    pub format: String,
    pub tempo: f64,
    pub time_signature: [u8; 2],
    pub sections: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransportState {
    pub running: bool,
    pub sync_start: bool,
    pub sync_stop: bool,
    pub sync_stop_available: bool,
    pub auto_fill: bool,
    pub stop_acmp: bool,
    pub section: Option<String>,
    pub queued: Option<String>,
    pub pending_intro: Option<u8>,
    pub main: u8,
    pub bar: u32,
    pub beat: u32,
    pub beats_per_bar: u8,
    pub tempo: f64,
    pub lamps: Vec<Pad>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChordState {
    pub name: Option<String>,
    pub fingered: Option<String>,
    pub fingering: Fingering,
    pub fingering_name: String,
    pub upper: bool,
    pub manual_bass: bool,
    pub manual_bass_active: bool,
    pub split: u8,
    pub split_name: String,
    pub transpose_keyboard: i8,
    pub transpose_master: i8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardPart {
    pub name: String,
    pub channel: u8,
    pub on: bool,
    pub sounding: bool,
    pub selected: bool,
    pub volume: u8,
    pub waiting: bool,
    pub program: u8,
    pub voice_name: String,
    pub plays_bass: bool,
    pub octave: i8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MixerState {
    pub fader_page: FaderPage,
    pub style_parts: Vec<StylePart>,
    pub master: Option<u8>,
    pub master_waiting: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StylePart {
    pub name: String,
    pub channel: u8,
    pub on: bool,
    pub muted_by_manual_bass: bool,
    pub volume: u8,
    pub waiting: bool,
    pub voice: Option<Voice>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Voice {
    pub bank_msb: u8,
    pub bank_lsb: u8,
    pub program: u8,
    pub kit: bool,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PadsState {
    pub page: Page,
    pub page_name: String,
    pub page_number: u8,
    pub page_count: u8,
    pub pads: Vec<Pad>,
    pub connected: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pad {
    pub note: u8,
    pub label: String,
    pub key: String,
    pub rgb: [u8; 3],
    pub level: Level,
    pub anim: Anim,
    pub action: Option<AppCmd>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtsState {
    pub settings: Vec<OtsSetting>,
    pub applied: u8,
    pub link: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtsSetting {
    pub name: String,
    pub parts: Vec<OtsPart>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtsPart {
    pub on: bool,
    pub program: Option<u8>,
    pub voice_name: String,
    pub volume: u8,
    pub octave: i8,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStatus {
    pub revision: u64,
    pub count: usize,
    pub position: usize,
    pub pending: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryEntry {
    pub id: usize,
    pub name: String,
    pub folder: String,
    pub path: String,
    pub status: String,
    pub error: Option<String>,
    pub tempo: Option<f64>,
    pub time_signature: Option<[u8; 2]>,
    pub sections: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryList {
    pub revision: u64,
    pub entries: Vec<LibraryEntry>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IoState {
    pub output_port: String,
    pub inputs: Vec<String>,
    pub synth: Option<SynthState>,
    pub engine: EngineStats,
    pub last_control: u32,
    pub unmapped: String,
    pub offline: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SynthState {
    pub sound_font: String,
    pub device: String,
    pub sample_rate: u32,
    pub buffer_frames: Option<u32>,
    pub channels: u32,
    pub output_pair: [u8; 2],
    pub muted: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineStats {
    pub realtime: bool,
    pub wake_p99_us: u32,
    pub chord_p99_us: u32,
    pub midi_in_p99_us: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub seq: u64,
    pub text: String,
    pub error: bool,
}

/// A MIDI note in Yamaha octave numbering (C3 = MIDI 60): "F#2".
pub fn note_name(n: u8) -> String {
    const NAMES: [&str; 12] = ["C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"];
    format!("{}{}", NAMES[n as usize % 12], n as i32 / 12 - 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_have_the_documented_json() {
        let c: AppCmd = serde_json::from_str(r#"{"type":"main","index":1}"#).unwrap();
        assert_eq!(c, AppCmd::Main { index: 1 });
        let c: AppCmd = serde_json::from_str(r#"{"type":"setFingering","fingering":"aiFullKeyboard"}"#).unwrap();
        assert_eq!(c, AppCmd::SetFingering { fingering: Fingering::AiFullKeyboard });
        let c: AppCmd = serde_json::from_str(r#"{"type":"setPadPage","page":"otsParts"}"#).unwrap();
        assert_eq!(c, AppCmd::SetPadPage { page: Page::OtsParts });
        let c: AppCmd = serde_json::from_str(r#"{"type":"startStop"}"#).unwrap();
        assert_eq!(c, AppCmd::StartStop);
        assert_eq!(
            serde_json::to_string(&Event::StateChanged { version: 3 }).unwrap(),
            r#"{"type":"stateChanged","version":3}"#
        );
    }
}

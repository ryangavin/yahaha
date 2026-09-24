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
//!
//! Each feature has a module here: its commands (one enum, e.g. [`TransportCmd`]) and
//! its part of the state (e.g. [`TransportState`]). `AppCmd` has one variant per group
//! and `AppState` one field per feature; both keep the flat JSON of docs/app-api.md
//! (`{"type":"main","index":1}`, `state.transport`), which tests/api_wire.rs pins. A new
//! feature adds a module, one line in `app_cmd!` below and/or one field in `AppState`
//! (docs/architecture.md, "Adding a feature").

mod chord;
mod controllers;
mod harmony_arp;
mod keyboard;
mod library;
mod looper;
mod metronome;
mod mixer;
mod multipad;
mod ots;
mod pads;
mod plugins;
mod parts;
mod playlist;
mod preview;
mod registration;
mod settings;
mod style_change;
mod style_settings;
mod surface;
mod system;
mod transport;

pub use chord::*;
pub use controllers::*;
pub use harmony_arp::*;
pub use keyboard::*;
pub use library::*;
pub use looper::*;
pub use metronome::*;
pub use mixer::*;
pub use multipad::*;
pub use ots::*;
pub use pads::*;
pub use plugins::*;
pub use parts::*;
pub use playlist::*;
pub use preview::*;
pub use registration::*;
pub use settings::*;
pub use style_change::*;
pub use style_settings::*;
pub use surface::*;
pub use system::*;
pub use transport::*;

use crate::engine::Button;
use crate::launchkey::Action;
use crate::theory::NOTE_NAMES;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Builds `AppCmd` from its groups: the enum, `From<GroupCmd> for AppCmd`, and the
/// deserializer. On the wire a command is its group's JSON (`{"type":"main",...}`): the
/// group level is Rust-only.
macro_rules! app_cmd {
    ($($(#[$doc:meta])* $group:ident($ty:ty),)*) => {
        /// Every user action, by feature. Indices are 0-based. Keyboard parts: 0 = Right 1,
        /// 1 = Right 2, 2 = Right 3, 3 = Left. Style parts: 0-7 = Rhythm 1, Rhythm 2, Bass,
        /// Chord 1, Chord 2, Pad, Phrase 1, Phrase 2 (MIDI channels 9-16).
        ///
        /// `Session::send` takes a group's command directly
        /// (`send(TransportCmd::Main { index: 1 })`).
        #[derive(Clone, Debug, PartialEq, Serialize)]
        #[serde(untagged)]
        pub enum AppCmd {
            $($(#[$doc])* $group($ty),)*
        }

        $(impl From<$ty> for AppCmd {
            fn from(c: $ty) -> AppCmd {
                AppCmd::$group(c)
            }
        })*

        /// The group whose `type` it is parses it. (A plain `untagged` would say only
        /// "data did not match any variant" for a mistyped field.)
        impl<'de> Deserialize<'de> for AppCmd {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<AppCmd, D::Error> {
                let v = serde_json::Value::deserialize(d)?;
                let Some(tag) = v.get("type").and_then(|t| t.as_str()) else {
                    return Err(D::Error::missing_field("type"));
                };
                $(match <$ty>::deserialize(&v) {
                    Ok(c) => return Ok(AppCmd::$group(c)),
                    Err(e) if !e.to_string().starts_with("unknown variant") => return Err(D::Error::custom(e)),
                    Err(_) => {}
                })*
                Err(D::Error::custom(format_args!("unknown variant `{tag}`, expected an AppCmd type")))
            }
        }
    };
}

app_cmd! {
    /// Sections and transport (the Genos panel buttons; engine state).
    Transport(TransportCmd),
    /// Style part mute and levels, fader page, synth master and mute.
    Mixer(MixerCmd),
    /// Chord detection, split, transpose.
    Chord(ChordCmd),
    /// Keyboard parts (Right 1-3, Left).
    Parts(PartsCmd),
    /// Launchkey pad pages.
    Pads(PadsCmd),
    /// One Touch Settings.
    Ots(OtsCmd),
    /// Loading styles, the library.
    Library(LibraryCmd),
    /// Style preview.
    Preview(PreviewCmd),
    /// Audio output, SoundFont, MIDI inputs, LEDs.
    Settings(SettingsCmd),
    /// Panic, the message line.
    System(SystemCmd),
    /// Style Setting > Change Behavior: tempo, part on/off, Section Set.
    StyleChange(StyleChangeCmd),
    /// Section Change Timing, Synchro Stop Window, fade times, Section Reset, Retrigger length.
    StyleSettings(StyleSettingsCmd),
    /// Registration Memory: buttons, banks, Memorize, Freeze, Registration Sequence.
    Registration(RegistrationCmd),
    /// The Playlist.
    Playlist(PlaylistCmd),
    /// Chord Looper: record, loop, memories.
    Looper(LooperCmd),
    /// Metronome on/off, volume, bell.
    Metronome(MetronomeCmd),
    /// Multi Pads: the bank, the pads, Synchro Stop.
    MultiPad(MultiPadCmd),
    /// Pedals, wheels and assignable functions.
    Controllers(ControllersCmd),
    /// Instrument plugins (Audio Units) for the keyboard parts.
    Plugins(PluginCmd),
    /// Keyboard Harmony / Arpeggio.
    HarmonyArp(HarmonyArpCmd),
}

impl From<Button> for AppCmd {
    fn from(b: Button) -> AppCmd {
        match b {
            Button::Intro(i) => TransportCmd::Intro { index: i }.into(),
            Button::Main(i) => TransportCmd::Main { index: i }.into(),
            Button::Break => TransportCmd::Break.into(),
            Button::Fill(d) => TransportCmd::Fill { delta: d }.into(),
            Button::Ending(i) => TransportCmd::Ending { index: i }.into(),
            Button::StartStop => TransportCmd::StartStop.into(),
            Button::Stop => TransportCmd::Stop.into(),
            Button::SyncStart => TransportCmd::ToggleSyncStart.into(),
            Button::SyncStop => TransportCmd::ToggleSyncStop.into(),
            Button::AutoFill => TransportCmd::ToggleAutoFill.into(),
            Button::TapTempo => TransportCmd::TapTempo.into(),
            Button::TempoUp => TransportCmd::TempoUp.into(),
            Button::TempoDown => TransportCmd::TempoDown.into(),
            Button::SetTempo(bpm) => TransportCmd::SetTempo { bpm }.into(),
            Button::TogglePart(p) => MixerCmd::ToggleStylePart { part: p }.into(),
            Button::StopAcmp => TransportCmd::ToggleStopAcmp.into(),
            Button::SetStopAcmp(m) => TransportCmd::SetStopAcmp { mode: m.into() }.into(),
            Button::FillUp => TransportCmd::FillUp.into(),
            Button::FillDown => TransportCmd::FillDown.into(),
            Button::FillSelf => TransportCmd::FillSelf.into(),
            Button::HalfBarFill => TransportCmd::ToggleHalfBarFill.into(),
            Button::SetHalfBarFill(on) => TransportCmd::SetHalfBarFill { on }.into(),
            Button::Fade => TransportCmd::ToggleFade.into(),
            Button::SectionReset => TransportCmd::SectionReset.into(),
            Button::Retrigger => TransportCmd::ToggleRetrigger.into(),
        }
    }
}

impl AppCmd {
    /// The engine button this command is, if it is one.
    pub fn button(&self) -> Option<Button> {
        match self {
            AppCmd::Transport(c) => Some(c.button()),
            AppCmd::Mixer(c) => c.button(),
            _ => None,
        }
    }
}

/// A Launchkey pad or button action is the command its keyboard shortcut sends.
impl From<Action> for AppCmd {
    fn from(a: Action) -> AppCmd {
        match a {
            Action::Button(b) => b.into(),
            Action::Fingering(f) => ChordCmd::SetFingering { fingering: f }.into(),
            Action::NextFingering => ChordCmd::NextFingering.into(),
            Action::ToggleUpper => ChordCmd::ToggleUpper.into(),
            Action::ToggleManualBass => ChordCmd::ToggleManualBass.into(),
            Action::Split(d) => ChordCmd::MoveSplit { delta: d }.into(),
            Action::Transpose { keyboard, master } => ChordCmd::StepTranspose { keyboard, master }.into(),
            Action::TransposeReset => ChordCmd::ResetTranspose.into(),
            Action::Ots(n) => OtsCmd::RecallOts { index: n }.into(),
            Action::ToggleOtsLink => OtsCmd::ToggleOtsLink.into(),
            Action::PartOnOff(p) => PartsCmd::TogglePart { part: p }.into(),
            Action::SelectPart(p) => PartsCmd::SelectPart { part: p }.into(),
            Action::PartVoice(d) => PartsCmd::StepVoice { delta: d }.into(),
            Action::ToggleFaderPage => MixerCmd::ToggleFaderPage.into(),
            Action::Style(d) => LibraryCmd::StepStyle { delta: d }.into(),
            Action::RetriggerRate(d) => StyleSettingsCmd::StepRetriggerRate { delta: d }.into(),
            Action::Regist(i) => RegistrationCmd::PressRegist { index: i }.into(),
            Action::RegistMemory => RegistrationCmd::ToggleRegistMemory.into(),
            Action::RegistFreeze => RegistrationCmd::ToggleFreeze.into(),
            Action::RegistBank(d) => RegistrationCmd::StepRegistBank { delta: d }.into(),
            Action::RegistSeq(d) => RegistrationCmd::StepRegistSequence { delta: d }.into(),
            Action::Playlist(d) => PlaylistCmd::StepPlaylist { delta: d }.into(),
            Action::Assign(f) => ControllersCmd::TriggerFunction { function: f }.into(),
            Action::AssignSet(f, on) => function_set(f, on).unwrap_or(ControllersCmd::TriggerFunction { function: f }.into()),
            Action::ToggleHarmonyArp => HarmonyArpCmd::ToggleHarmonyArp.into(),
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

/// Everything a front panel shows. Plain data: clone it, serialize it, compare it. Each
/// field is one feature's state, defined in that feature's module; the field order is the
/// JSON order.
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
    /// The Launchkey beyond the pads: Shift, every button, the faders, Track neighbours,
    /// the beat clock. With `pads`, a 1:1 mirror of the hardware.
    pub surface: SurfaceState,
    pub io: IoState,
    /// The style preview and the style waiting for the bar line.
    pub preview: PreviewState,
    /// The keys held and the chord, for the app's keyboard strip.
    pub keyboard: KeyboardState,
    /// Style Setting > Change Behavior.
    pub style_change: StyleChangeState,
    /// Section Change Timing, Synchro Stop Window, fade times, Section Reset, Retrigger length.
    pub style_settings: StyleSettingsState,
    /// Registration Memory: the bank, its ten buttons, Freeze, the Registration Sequence.
    pub registration: RegistrationState,
    /// The Playlist.
    pub playlist: PlaylistState,
    /// Multi Pads: the bank, the four pads, Synchro Stop, the bank files.
    pub multi_pad: MultiPadState,
    /// Pedals, wheels, their parts and the pedals' assignable functions.
    pub controllers: ControllersState,
    /// Keyboard Harmony / Arpeggio: the switch, the type, the settings.
    #[serde(default)]
    pub harmony_arp: HarmonyArpState,
    /// The last notice or error, until the next one or `ClearMessage`.
    pub message: Option<Message>,
    /// The Chord Looper.
    pub looper: LooperState,
    /// The metronome.
    pub metronome: MetronomeState,
    /// The instrument plugin host: the installed plugins (keyboard parts' plugins are in
    /// `keyboard_parts[i].plugin`).
    #[serde(default)]
    pub plugins: PluginsState,
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

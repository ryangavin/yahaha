//! Settings: audio output, SoundFont, MIDI inputs, Launchkey LEDs; and the MIDI and audio
//! state they show.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum SettingsCmd {
    /// The synth's stereo output pair, by its left channel (0-based; 0 = outputs 1/2).
    SetAudioOutput { first: u8 },
    /// Next stereo output pair, wrapping: 1/2 -> 3/4 -> ... -> 1/2.
    NextAudioOutput,
    /// Which MIDI sources play the keyboard: every one (`all`), or those named in `names`
    /// (a name matches a source whose name contains it). `all` false with no names: the
    /// default, a Launchkey's keys when there is one, else every source. The Launchkey's
    /// DAW port is always the pads. Keys held on a source that is dropped are released.
    SetMidiInputs { all: bool, names: Vec<String> },
    /// Launchkey LEDs in Novation palette colours (and hardware flashing) instead of RGB.
    SetPaletteLeds { on: bool },
    /// The synth's audio buffer: 64, 128, 256, 512 or 1024 frames
    /// (`io.synth.bufferFrames`). Larger buffers give heavy plugins, and a busy machine,
    /// more time per block, at the cost of latency. The output
    /// reopens with a moment of silence; voices, plugins and held notes carry over. A live
    /// session remembers it.
    SetAudioBuffer { frames: u32 },
}

/// The settings kept in `<data>/settings.json` and restored at start. Besides these, it
/// keeps the Setup pad page's switches, which the state shows where they act: the
/// fingering type and Chord Detection Area (`chord.fingering`, `chord.upper`), OTS Link
/// (`ots.link`) and Stop ACMP (`transport.stopAcmpMode`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsState {
    /// The order of pad pages 2-5 (`setPadPageOrder`); Sections is always page 1.
    pub pad_pages: Vec<crate::launchkey::Page>,
}

impl Default for SettingsState {
    fn default() -> SettingsState {
        SettingsState { pad_pages: crate::launchkey::PageOrder::DEFAULT.movable().collect() }
    }
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
    /// Every MIDI source there is (the keyboard sources `SetMidiInputs` chooses from, and
    /// the Launchkey DAW port), and whether yahaha listens to it.
    pub sources: Vec<MidiSource>,
    /// Every source is a keyboard (`SetMidiInputs { all: true }`, `--all-inputs`).
    pub all_inputs: bool,
    /// The SoundFonts (`.sf2` file names) in the SoundFont folder.
    pub sound_fonts: Vec<String>,
    /// The synth's main font: the most GM-complete in the folder, which plays a channel no
    /// route covers (None without the synth). Not a setting: the GM map decides what every
    /// program plays (`soundLibrary.gmMap`).
    pub sound_font_file: Option<String>,
    /// A rack of SoundFonts is loading (a new main font, or fonts the map needs).
    pub sound_font_loading: bool,
}

/// A MIDI source.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MidiSource {
    /// Its name, as `SetMidiInputs` matches it.
    pub name: String,
    /// yahaha listens to it (as a keyboard, or as the pads).
    pub listening: bool,
    /// The Launchkey DAW port: the pads, buttons and faders.
    pub pads: bool,
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
    /// Audio dropouts since the synth started: the device reported an overload (an IO
    /// cycle missed its deadline), or the callback took longer than its buffer lasts. The
    /// app suggests a larger buffer when they keep coming.
    #[serde(default)]
    pub dropouts: u64,
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

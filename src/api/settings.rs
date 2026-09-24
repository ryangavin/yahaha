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
    /// Reload the synth from another SoundFont in its folder (`io.soundFonts`, by file
    /// name). It loads in the background (`io.soundFontLoading`) and swaps in between two
    /// audio buffers; the voices and controllers in use carry over, notes sounding stop.
    SetSoundFont { file: String },
    /// Which MIDI sources play the keyboard: every one (`all`), or those named in `names`
    /// (a name matches a source whose name contains it). `all` false with no names: the
    /// default, a Launchkey's keys when there is one, else every source. The Launchkey's
    /// DAW port is always the pads. Keys held on a source that is dropped are released.
    SetMidiInputs { all: bool, names: Vec<String> },
    /// Launchkey LEDs in Novation palette colours (and hardware flashing) instead of RGB.
    SetPaletteLeds { on: bool },
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
    /// The SoundFonts (`.sf2` file names) in the synth's folder, for `SetSoundFont`.
    pub sound_fonts: Vec<String>,
    /// The file the synth plays (None without the synth).
    pub sound_font_file: Option<String>,
    /// A `SetSoundFont` is loading.
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

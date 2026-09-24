//! The sound library (#103): a small, user-built list of patches (about 20) and the
//! program map that sends every style part to one of them.
//!
//! - [`Patch`]: a name, a Genos voice [`Category`], tags, a favourite flag, a [`PatchSource`]
//!   (a SoundFont preset, or a plugin with its saved state) and [`PatchDefaults`].
//! - [`ProgramMap`] (map.rs): GM family rules, per-program overrides and a drum rule. One
//!   global map, and optionally one per style (stored here, keyed by the style's file
//!   name, never in the style file). [`resolve`] decides which patch a program plays.
//! - [`SoundLibrary`] (store.rs): the patches and maps as one versioned JSON file in the
//!   data folder, with import and export.
//! - route.rs: the resolved map as the real-time threads read it (atomics, no allocation).
//! - sf2.rs: the presets of a `.sf2` file (its PHDR chunk only), for "Add from SoundFont".
//!
//! Everything here but route.rs runs on the control side. docs/sound-library.md has the
//! behaviour and the decisions.

pub mod map;
pub mod port;
pub mod route;
pub mod sf2;
pub mod store;
#[cfg(test)]
mod tests;

pub use map::*;
pub use route::{Route, Routes, Source};
pub use store::*;

use serde::{Deserialize, Serialize};

/// The Genos voice categories (the Voice Selection display's tabs), which the library
/// groups its patches by.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Category {
    #[default]
    Piano,
    #[serde(rename = "ePiano")]
    EPiano,
    Organ,
    Guitar,
    Bass,
    Strings,
    Brass,
    SaxWoodwind,
    SynthLead,
    Pad,
    Choir,
    DrumsPerc,
    Sfx,
}

impl Category {
    pub const ALL: [Category; 13] = [
        Category::Piano,
        Category::EPiano,
        Category::Organ,
        Category::Guitar,
        Category::Bass,
        Category::Strings,
        Category::Brass,
        Category::SaxWoodwind,
        Category::SynthLead,
        Category::Pad,
        Category::Choir,
        Category::DrumsPerc,
        Category::Sfx,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Category::Piano => "Piano",
            Category::EPiano => "E.Piano",
            Category::Organ => "Organ",
            Category::Guitar => "Guitar",
            Category::Bass => "Bass",
            Category::Strings => "Strings",
            Category::Brass => "Brass",
            Category::SaxWoodwind => "Sax/Woodwind",
            Category::SynthLead => "Synth Lead",
            Category::Pad => "Pad",
            Category::Choir => "Choir",
            Category::DrumsPerc => "Drums/Perc",
            Category::Sfx => "SFX",
        }
    }

    /// The category a GM program (or a SoundFont preset: bank 128 is drums) most likely
    /// belongs to: the starting guess when a preset or a part's sound becomes a patch.
    pub fn guess(bank: u16, program: u8) -> Category {
        if bank >= 128 {
            return Category::DrumsPerc;
        }
        match program & 127 {
            4 | 5 => Category::EPiano,
            0..=7 => Category::Piano,
            8..=15 => Category::DrumsPerc,
            16..=23 => Category::Organ,
            24..=31 => Category::Guitar,
            32..=39 => Category::Bass,
            40..=51 => Category::Strings,
            52..=54 => Category::Choir,
            55 => Category::Sfx,
            56..=63 => Category::Brass,
            64..=79 => Category::SaxWoodwind,
            80..=87 => Category::SynthLead,
            88..=95 => Category::Pad,
            96..=103 => Category::Pad,
            104..=107 => Category::Guitar,
            108 => Category::DrumsPerc,
            109..=111 => Category::SaxWoodwind,
            112..=119 => Category::DrumsPerc,
            _ => Category::Sfx,
        }
    }
}

/// Where a patch's sound comes from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PatchSource {
    /// A preset of a SoundFont in the SoundFont folder: its file name, SoundFont bank
    /// (128 = drum kits) and program.
    SoundFont { file: String, bank: u16, program: u8 },
    /// An instrument plugin (an Audio Unit today): its component id ("aumu:abcd:manu")
    /// and saved state (its ClassInfo bytes, base64, as #91 stores it). Plugin patches play
    /// through #91's per-channel plugin rack when the build has it; otherwise the
    /// SoundFont fallback.
    Plugin { component_id: String, #[serde(default)] state: String },
}

/// What a patch brings with it when it is picked: plain MIDI settings, sent as CCs so the
/// mixer shows them (the mixer principle: no hidden gain).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchDefaults {
    /// CC7. A keyboard part picking the patch takes it as its volume; a Style part only
    /// when the style sets no CC7 of its own for it.
    pub volume: Option<u8>,
    /// CC10 (64 = centre). Keyboard parts only.
    pub pan: Option<u8>,
    /// CC91. Keyboard parts only.
    pub reverb: Option<u8>,
    /// CC93. Keyboard parts only.
    pub chorus: Option<u8>,
    /// Octave shift (-2..=2). Keyboard parts only.
    pub octave: i8,
}

impl Default for PatchDefaults {
    fn default() -> PatchDefaults {
        PatchDefaults { volume: None, pan: None, reverb: None, chorus: None, octave: 0 }
    }
}

impl PatchDefaults {
    pub fn clamped(self) -> PatchDefaults {
        PatchDefaults {
            volume: self.volume.map(|v| v.min(127)),
            pan: self.pan.map(|v| v.min(127)),
            reverb: self.reverb.map(|v| v.min(127)),
            chorus: self.chorus.map(|v| v.min(127)),
            octave: self.octave.clamp(-2, 2),
        }
    }
}

/// One sound in the library.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Patch {
    /// Stable id: the maps, the parts and exports refer to it.
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub category: Category,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub favourite: bool,
    pub source: PatchSource,
    #[serde(default)]
    pub defaults: PatchDefaults,
}

/// Why a patch plays its SoundFont fallback instead of itself, if it does.
pub fn unavailable_reason(p: &Patch, sound_fonts: &[String]) -> Option<String> {
    match &p.source {
        PatchSource::Plugin { .. } if !cfg!(feature = "plugins") => Some("needs plugin hosting (#91)".into()),
        PatchSource::Plugin { .. } => None,
        PatchSource::SoundFont { file, .. } if !sound_fonts.iter().any(|f| f == file) => {
            Some(format!("{file} is not in the SoundFont folder"))
        }
        PatchSource::SoundFont { .. } => None,
    }
}

/// A new id for a patch named `name`, unique in `taken`: its name in lower case with
/// dashes ("my-bass"), then "-2", "-3", ... if that is taken.
pub fn new_id<'a>(name: &str, taken: impl Iterator<Item = &'a str> + Clone) -> String {
    let mut base: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    base.truncate(32);
    if base.is_empty() {
        base = "patch".into();
    }
    let free = |id: &str| !taken.clone().any(|t| t == id);
    if free(&base) {
        return base;
    }
    (2..).map(|n| format!("{base}-{n}")).find(|id| free(id)).unwrap()
}

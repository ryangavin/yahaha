//! The unified Sound model (docs/sound-browser.md): what a part plays, and how every
//! other record (plugin-parts.json, Registration, OTS) names it.
//!
//! A **Sound** is one of two things:
//!
//! - a **font preset**: a preset of a SoundFont in a scanned folder ([`FontPreset`]: file,
//!   bank, program). It needs no library entry; its id is `sf:<file>:<bank>:<program>`.
//! - a **plugin sound**: a library [`Patch`](super::Patch) whose source is
//!   [`PatchSource::Plugin`](super::PatchSource::Plugin): an instrument plus its captured
//!   state, name, category, CC defaults and favourite flag. Its id is `saved:<patch id>`.
//!
//! There is exactly one kind of plugin sound (D1). Where it came from is its
//! [`PluginOrigin`]: made in yahaha (`user`), a factory preset (`factory`, by number), or an
//! imported `.aupreset` file (`file`, remembered by path). A factory preset's state is not
//! known until an instance plays it, so its sound starts with an empty state and the state
//! is captured the first time it plays ([`SoundLibrary::capture_state`](super::SoundLibrary::capture_state)).
//!
//! A library patch whose source is a SoundFont preset is a font preset the user added to
//! their sounds (with a name, category and defaults of its own); it resolves to that
//! preset, so its [`FontPreset`] provenance is known (D6).

use serde::{Deserialize, Serialize};
use std::fmt;

/// A SoundFont preset: which file (by file name, as the scanned folder has it), which
/// SoundFont bank (128 = drum kits) and program. Every resolution that ends in a font
/// records one, so a `.sf2` writer can be added later without changing the data (D6).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FontPreset {
    pub file: String,
    pub bank: u16,
    pub program: u8,
}

impl FontPreset {
    pub fn new(file: impl Into<String>, bank: u16, program: u8) -> FontPreset {
        FontPreset { file: file.into(), bank, program: program & 127 }
    }

    /// A drum kit (SoundFont bank 128).
    pub fn is_kit(&self) -> bool {
        self.bank >= 128
    }
}

/// Where a plugin sound came from. It never changes what plays (the state does); it lets
/// a scan find the sound again for a factory preset or an `.aupreset` file, instead of
/// adding a duplicate.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PluginOrigin {
    /// Made in yahaha (Save as…, New sound from a plugin), or a library from before the
    /// origin was recorded.
    #[default]
    User,
    /// The plugin's factory preset `number` (`kAudioUnitProperty_PresentPreset`).
    Factory { number: i32 },
    /// An `.aupreset` file, imported at a scan and remembered by its path.
    File { path: String },
}

impl PluginOrigin {
    pub fn is_user(&self) -> bool {
        matches!(self, PluginOrigin::User)
    }

    /// The origin a Sound Browser preset key names: `f:<number>` or `u:<path>`.
    pub fn from_preset_key(key: &str) -> Option<PluginOrigin> {
        if let Some(n) = key.strip_prefix("f:") {
            return n.parse().ok().map(|number| PluginOrigin::Factory { number });
        }
        key.strip_prefix("u:").filter(|p| !p.is_empty()).map(|p| PluginOrigin::File { path: p.to_string() })
    }
}

/// A Sound's id, as parts, recall records, map rules and the browser name it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SoundId {
    /// `sf:<file>:<bank>:<program>`: a font preset.
    Font(FontPreset),
    /// `saved:<patch id>`: a library patch (every plugin sound, and font presets added to
    /// My Sounds).
    Library(String),
}

impl SoundId {
    /// Parse `sf:…` or `saved:…`. A bare string is a library patch id (what map rules and
    /// part patches have always stored).
    pub fn parse(s: &str) -> Option<SoundId> {
        if let Some(rest) = s.strip_prefix("sf:") {
            let (rest, program) = rest.rsplit_once(':')?;
            let (file, bank) = rest.rsplit_once(':')?;
            let program: u8 = program.parse().ok().filter(|&p: &u8| p < 128)?;
            let bank: u16 = bank.parse().ok()?;
            return (!file.is_empty()).then(|| SoundId::Font(FontPreset::new(file, bank, program)));
        }
        let id = s.strip_prefix("saved:").unwrap_or(s);
        (!id.is_empty() && !id.contains(':')).then(|| SoundId::Library(id.to_string()))
    }
}

impl fmt::Display for SoundId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SoundId::Font(p) => write!(f, "sf:{}:{}:{}", p.file, p.bank, p.program),
            SoundId::Library(id) => write!(f, "saved:{id}"),
        }
    }
}

/// What a record keeps to say which Sound it plays: the id (a [`SoundId`] string) and
/// its name when it was stored, so recall shows the name even if the sound is gone.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundTag {
    pub id: String,
    pub name: String,
}

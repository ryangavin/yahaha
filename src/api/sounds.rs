//! The sound catalog (#117): every sound the Sound Browser offers, from every source, in
//! one list. SoundFont presets (every `.sf2` in the SoundFont folder), instrument plugins
//! and saved sounds (the sound library's patches).
//!
//! The list itself is fetched like the style library (`Session::sound_catalog`, Tauri
//! `sounds()`), since a 2,000-preset list does not belong in `AppState`. `AppState::sounds`
//! is a small summary whose `revision` says when to fetch again.
//!
//! Entry ids: `sf:<file>:<bank>:<program>`, `au:<component id>`, `saved:<patch id>`.

use super::PatchCategory;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum SoundsCmd {
    /// Mark or unmark a favourite (a saved sound's is its patch's `favourite`).
    SetSoundFavourite { id: String, on: bool },
    /// Play a sound on its own for a moment (the band must be stopped), as `auditionPatch`
    /// does. A plugin auditions on a part: assign it.
    AuditionSound { id: String },
    StopSoundAudition,
    /// Keyboard part `part` (0-3) plays the sound: a preset of the default sound set as its
    /// voice (`setPartVoice`), a preset of another font as a saved sound (`setPartPatch`,
    /// adding the preset to the library once), a plugin (`setPartPlugin`), a saved sound
    /// (`setPartPatch`). It goes to the top of the Recents.
    AssignSound { part: u8, id: String },
    /// A plugin's (or a saved sound's) category. A preset's comes from its GM family.
    SetSoundCategory { id: String, category: PatchCategory },
}

/// Where a sound comes from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SoundSource {
    #[default]
    SoundFont,
    Plugin,
    Saved,
}

/// A plugin's details, for the browser's badges.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundPluginInfo {
    /// "AUv2" or "AUv3".
    pub format: String,
    /// The last load's error, so the browser can warn.
    pub last_error: Option<String>,
}

/// One sound in the catalog.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundEntry {
    /// `sf:<file>:<bank>:<program>`, `au:<component id>` or `saved:<patch id>`.
    pub id: String,
    pub name: String,
    pub category: PatchCategory,
    pub source: SoundSource,
    /// The SoundFont file, the plugin's maker, or what a saved sound plays.
    pub detail: String,
    pub favourite: bool,
    /// In the Recents (`SoundCatalog::recents` has their order).
    pub recent: bool,
    /// Plugins only.
    pub plugin: Option<SoundPluginInfo>,
}

/// The whole catalog: presets by file then bank and program, plugins by maker then name,
/// saved sounds in the library's order.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundCatalog {
    /// `AppState::sounds.revision` when it was built.
    pub revision: u64,
    pub entries: Vec<SoundEntry>,
    /// The ids last assigned, most recent first (at most 20).
    pub recents: Vec<String>,
}

/// The catalog's summary in `AppState`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundsState {
    /// Changes whenever the catalog does: fetch it again (`Event::SoundsChanged`).
    pub revision: u64,
    /// The entries in the catalog.
    pub count: u32,
    /// Plugins are being scanned: more may come.
    pub scanning: bool,
    /// The sound being auditioned (`auditionSound`).
    pub auditioning: Option<String>,
}

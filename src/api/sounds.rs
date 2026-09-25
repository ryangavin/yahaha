//! The sound catalog (#117): every sound the Sound Browser offers, from every source, in
//! one list. SoundFont presets (every `.sf2` in the SoundFont folder), instrument plugins
//! and saved sounds (the sound library's patches).
//!
//! The list itself is fetched like the style library (`Session::sound_catalog`, Tauri
//! `sounds()`), since a 2,000-preset list does not belong in `AppState`. `AppState::sounds`
//! is a small summary whose `revision` says when to fetch again.
//!
//! Entry ids: `sf:<file>:<bank>:<program>`, `au:<component id>`, `saved:<patch id>`.

use super::{PatchCategory, PluginEntry};
use crate::patches::sf2::Preset;
use crate::patches::{Category, Patch, PatchSource};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum SoundsCmd {
    /// Mark or unmark a favourite (a saved sound's is its patch's `favourite`).
    SetSoundFavourite { id: String, on: bool },
    /// Play a sound on its own for a moment (the band must be stopped), as `auditionPatch`
    /// does (a plugin with its default preset).
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

/// How many Recents are kept.
pub const MAX_RECENTS: usize = 20;

/// What the user set in the browser, saved in `sound-settings.json` (the session's and the
/// mocks' catalogs are built from it the same way).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundPrefs {
    /// Favourite presets and plugins (a saved sound's is its patch's).
    #[serde(default)]
    pub favourites: BTreeSet<String>,
    /// Most recent first.
    #[serde(default)]
    pub recents: Vec<String>,
    /// Plugin categories the user set, by entry id.
    #[serde(default)]
    pub sound_categories: BTreeMap<String, Category>,
}

impl SoundPrefs {
    /// `id` to the top of the Recents.
    pub fn push_recent(&mut self, id: String) {
        self.recents.retain(|r| *r != id);
        self.recents.insert(0, id);
        self.recents.truncate(MAX_RECENTS);
    }

    /// A plugin's category: the user's, else the guess from its name and maker.
    pub fn plugin_category(&self, id: &str, name: &str, maker: &str) -> Category {
        self.sound_categories.get(id).copied().unwrap_or_else(|| plugin_category(name, maker))
    }

    /// The catalog's entries: each font's presets, the plugins, the saved sounds.
    pub fn entries(&self, fonts: &[(&str, &[Preset])], plugins: &[PluginEntry], patches: &[Patch]) -> Vec<SoundEntry> {
        let recent = |id: &str| self.recents.iter().any(|r| r == id);
        let mut entries = Vec::new();
        for (f, presets) in fonts {
            for p in presets.iter() {
                let id = preset_id(f, p.bank, p.program);
                entries.push(SoundEntry {
                    name: if p.name.trim().is_empty() { format!("{f} {}:{}", p.bank, p.program) } else { p.name.trim().to_string() },
                    category: Category::guess(p.bank, p.program),
                    source: SoundSource::SoundFont,
                    detail: f.to_string(),
                    favourite: self.favourites.contains(&id),
                    recent: recent(&id),
                    plugin: None,
                    id,
                });
            }
        }
        for p in plugins {
            let id = format!("au:{}", p.id);
            entries.push(SoundEntry {
                category: self.plugin_category(&id, &p.name, &p.manufacturer),
                source: SoundSource::Plugin,
                detail: p.manufacturer.clone(),
                favourite: self.favourites.contains(&id),
                recent: recent(&id),
                plugin: Some(SoundPluginInfo { format: p.format.clone(), last_error: p.last_error.clone() }),
                name: p.name.clone(),
                id,
            });
        }
        for p in patches {
            let id = format!("saved:{}", p.id);
            let detail = match &p.source {
                PatchSource::SoundFont { file, .. } => file.clone(),
                PatchSource::Plugin { component_id, .. } => component_id.clone(),
            };
            entries.push(SoundEntry {
                name: p.name.clone(),
                category: p.category,
                source: SoundSource::Saved,
                detail,
                favourite: p.favourite,
                recent: recent(&id),
                plugin: None,
                id,
            });
        }
        entries
    }
}

/// A preset's catalog id.
pub fn preset_id(file: &str, bank: u16, program: u8) -> String {
    format!("sf:{file}:{bank}:{program}")
}

/// A preset id's file, bank and program. The file may hold ':' itself: bank and program
/// are the last two fields.
pub fn parse_preset_id(id: &str) -> Option<(&str, u16, u8)> {
    let rest = id.strip_prefix("sf:")?;
    let (rest, program) = rest.rsplit_once(':')?;
    let (file, bank) = rest.rsplit_once(':')?;
    Some((file, bank.parse().ok()?, program.parse().ok().filter(|&p: &u8| p < 128)?))
}

/// A plugin's likely category, from its name and maker: most instrument plugins are
/// synths, so anything else is Synth Lead.
pub fn plugin_category(name: &str, maker: &str) -> Category {
    let n = format!("{name} {maker}").to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| n.contains(w));
    if has(&["rhodes", "wurli", "e.piano", "epiano", "electric piano", "e-piano", "ep-", "clav"]) {
        Category::EPiano
    } else if has(&["piano", "grand", "keyscape", "pianoteq"]) {
        Category::Piano
    } else if has(&["organ", "b-3", "b3", "hammond", "vox continental", "farfisa"]) {
        Category::Organ
    } else if has(&["drum", "perc", "kit", "beat", "battery", "808", "909"]) {
        Category::DrumsPerc
    } else if has(&["bass"]) {
        Category::Bass
    } else if has(&["guitar", "strum"]) {
        Category::Guitar
    } else if has(&["string", "violin", "cello", "orchestra", "symphon"]) {
        Category::Strings
    } else if has(&["brass", "trumpet", "horn", "trombone"]) {
        Category::Brass
    } else if has(&["sax", "flute", "clarinet", "oboe", "wind"]) {
        Category::SaxWoodwind
    } else if has(&["choir", "vocal", "voice", "vox"]) {
        Category::Choir
    } else if has(&["pad", "ambient", "atmos"]) {
        Category::Pad
    } else {
        Category::SynthLead
    }
}


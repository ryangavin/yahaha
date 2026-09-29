//! The sound catalog (#117): every sound the Sound Browser offers, from every source, in
//! one list. SoundFont presets (every `.sf2` in the SoundFont folder), instrument plugins
//! and saved sounds (the sound library's patches).
//!
//! The list itself is fetched like the style library (`Session::sound_catalog`, Tauri
//! `sounds()`), since a 2,000-preset list does not belong in `AppState`. `AppState::sounds`
//! is a small summary whose `revision` says when to fetch again.
//!
//! Entry ids: `sf:<file>:<bank>:<program>`, `au:<component id>`, `saved:<patch id>`, and a
//! plugin's preset `au:<component id>#<preset key>` (a key is `f:<number>` for a factory
//! preset, `u:<path>` for an `.aupreset` file).

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
    /// Keyboard part `part` (0-3) plays the sound: a preset of the synth's main font as its
    /// voice (`setPartVoice`), a preset of another font as a saved sound (`setPartPatch`,
    /// adding the preset to the library once), a plugin (`setPartPlugin`), a saved sound
    /// (`setPartPatch`). It goes to the top of the Recents.
    AssignSound { part: u8, id: String },
    /// A plugin's, a plugin preset's (or a saved sound's) category. A SoundFont preset's
    /// comes from its GM family.
    SetSoundCategory { id: String, category: PatchCategory },
    /// List plugin `id`'s (`au:<component id>`) presets: the browser expanded it. Its
    /// `.aupreset` files are listed at every scan; its factory presets need an instance,
    /// so a plugin never loaded yet is loaded once in the background (then cached). The
    /// catalog moves when they are in.
    ListPluginPresets { id: String },
    /// Add catalog entry `id` (a font preset, a plugin, or one of a plugin's presets) to
    /// My Sounds: the library's patch for it, added once (the Instruments tab's "Add to my
    /// sounds"). A saved sound is in already.
    AddToMySounds { id: String },
    /// Save what keyboard part `part`'s plugin plays now (as its editor left it) as a user
    /// preset: a standard `.aupreset` named `name` in
    /// `~/Library/Audio/Presets/<Manufacturer>/<Plugin>/` (Logic and MainStage read it
    /// too), filed under `category` in the browser. The part then plays that preset. A
    /// preset of that name that exists already (the file is shared with Logic) is refused
    /// unless `overwrite` is set.
    SavePartAsPluginPreset {
        part: u8,
        name: String,
        category: PatchCategory,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        overwrite: bool,
    },
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
    /// How many presets it has in the catalog (entries whose `parent` is it), once its
    /// factory presets were read; None until then (unknown: `listPluginPresets` reads them),
    /// even when its `.aupreset` files are listed already.
    #[serde(default)]
    pub presets: Option<u32>,
    /// Why listing its factory presets failed (an error, a timeout, no list): the browser
    /// stops waiting and says so. It is not tried again until the next plugin scan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presets_error: Option<String>,
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
    /// Plugins (and plugin presets) only.
    pub plugin: Option<SoundPluginInfo>,
    /// A plugin preset's plugin (`au:<component id>`): the browser lists it under it.
    #[serde(default)]
    pub parent: Option<String>,
}

/// One preset of a plugin, for the catalog.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginPresetEntry {
    /// `f:<number>` or `u:<path>`.
    pub key: String,
    pub name: String,
    /// The sub-folder of an `.aupreset` ("Pianos").
    pub folder: Option<String>,
}

/// A plugin's presets, for the catalog.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginPresetList {
    /// The component id ("aumu Nik2 -NI-").
    pub plugin: String,
    /// Its factory presets were read (else only its `.aupreset` files are here).
    pub listed: bool,
    pub presets: Vec<PluginPresetEntry>,
    /// Listing its factory presets failed: why (`SoundPluginInfo::presets_error`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
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
    /// Each SoundFont in the folder, for the Instruments tab (the order of `entries`).
    #[serde(default)]
    pub fonts: Vec<FontSummary>,
}

/// A SoundFont file as the Instruments tab shows it: how many presets and kits it has and
/// how GM-complete it is (`patches::gm::gm_completeness`, what auto-fill ranks fonts by).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FontSummary {
    pub file: String,
    /// Melodic presets (banks below 128).
    pub presets: u32,
    /// Drum kits (bank 128).
    pub kits: u32,
    /// GM programs it has on bank 0, of 128.
    pub gm_programs: u8,
    /// It has a drum kit.
    pub gm_kit: bool,
}

/// Each font's summary.
pub fn font_summaries(fonts: &[(&str, &[Preset])]) -> Vec<FontSummary> {
    fonts
        .iter()
        .map(|(file, presets)| {
            let (gm_programs, gm_kit) = crate::patches::gm::gm_completeness(presets);
            let kits = presets.iter().filter(|p| p.bank >= 128).count() as u32;
            FontSummary { file: file.to_string(), presets: presets.len() as u32 - kits, kits, gm_programs, gm_kit }
        })
        .collect()
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
    /// Plugins (`au:<component id>`) whose presets are being listed (`listPluginPresets`).
    /// Left out of the JSON while none is.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub listing_presets: Vec<String>,
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

    /// A plugin preset's category: the user's, else a guess from its name and folder, else
    /// its plugin's.
    pub fn preset_category(&self, id: &str, name: &str, folder: Option<&str>, plugin: Category) -> Category {
        self.sound_categories.get(id).copied().or_else(|| guess_category(&format!("{name} {}", folder.unwrap_or("")))).unwrap_or(plugin)
    }

    /// The catalog's entries: each font's presets, the plugins (each followed by its
    /// presets), the saved sounds.
    pub fn entries(&self, fonts: &[(&str, &[Preset])], plugins: &[PluginEntry], presets: &[PluginPresetList], patches: &[Patch]) -> Vec<SoundEntry> {
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
                    parent: None,
                    id,
                });
            }
        }
        for p in plugins {
            let id = format!("au:{}", p.id);
            let list = presets.iter().find(|l| l.plugin == p.id);
            let category = self.plugin_category(&id, &p.name, &p.manufacturer);
            let info = |presets, presets_error| Some(SoundPluginInfo { format: p.format.clone(), last_error: p.last_error.clone(), presets, presets_error });
            entries.push(SoundEntry {
                category,
                source: SoundSource::Plugin,
                detail: p.manufacturer.clone(),
                favourite: self.favourites.contains(&id),
                recent: recent(&id),
                // Its `.aupreset` files alone are not its count: unknown until listed.
                plugin: info(list.filter(|l| l.listed).map(|l| l.presets.len() as u32), list.and_then(|l| l.error.clone())),
                name: p.name.clone(),
                parent: None,
                id: id.clone(),
            });
            for q in list.map_or(&[][..], |l| l.presets.as_slice()) {
                let pid = plugin_preset_id(&p.id, &q.key);
                entries.push(SoundEntry {
                    category: self.preset_category(&pid, &q.name, q.folder.as_deref(), category),
                    source: SoundSource::Plugin,
                    detail: match &q.folder {
                        Some(f) => format!("{} · {f}", p.name),
                        None => p.name.clone(),
                    },
                    favourite: self.favourites.contains(&pid),
                    recent: recent(&pid),
                    plugin: info(None, None),
                    name: q.name.clone(),
                    parent: Some(id.clone()),
                    id: pid,
                });
            }
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
                parent: None,
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

/// A plugin preset's catalog id: `au:<component id>#<key>`.
pub fn plugin_preset_id(component: &str, key: &str) -> String {
    format!("au:{component}#{key}")
}

/// A plugin entry id's component id and preset key (`au:<component>` or
/// `au:<component>#<key>`). None if it is not a plugin's.
pub fn parse_plugin_id(id: &str) -> Option<(&str, Option<&str>)> {
    let rest = id.strip_prefix("au:")?;
    Some(match rest.split_once('#') {
        Some((c, k)) => (c, Some(k)),
        None => (rest, None),
    })
}

/// A plugin's likely category, from its name and maker: most instrument plugins are
/// synths, so anything else is Synth Lead.
pub fn plugin_category(name: &str, maker: &str) -> Category {
    guess_category(&format!("{name} {maker}")).unwrap_or(Category::SynthLead)
}

/// The category words in `text` suggest ("Upright Piano": Piano), if any.
pub fn guess_category(text: &str) -> Option<Category> {
    let n = text.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| n.contains(w));
    Some(if has(&["rhodes", "wurli", "e.piano", "epiano", "electric piano", "e-piano", "ep-", "clav"]) {
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
    } else if has(&["synth", "lead"]) {
        Category::SynthLead
    } else {
        return None;
    })
}


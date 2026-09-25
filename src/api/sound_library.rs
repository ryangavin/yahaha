//! The sound library (#103): patches, the program map, and what the current style uses.

pub use crate::patches::sf2::Preset;
pub use crate::patches::{Category as PatchCategory, Patch, PatchDefaults, PatchSource, ProgramMap, ProgramOverride, RuleKind};
use serde::{Deserialize, Serialize};

/// A patch's editable fields (all but its id).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchFields {
    pub name: String,
    #[serde(default)]
    pub category: PatchCategory,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub favourite: bool,
    pub source: PatchSource,
    #[serde(default)]
    pub defaults: PatchDefaults,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum SoundLibraryCmd {
    /// Add a patch at the end of the list (its id: `soundLibrary.lastAdded`).
    CreatePatch { patch: PatchFields },
    /// Change a patch: rename, recategorise, tags, favourite, source, defaults.
    UpdatePatch { id: String, patch: PatchFields },
    /// Delete a patch. Rules naming it go; parts playing it go back to their GM voice.
    DeletePatch { id: String },
    /// A copy of a patch, right after it (e.g. the same plugin with another state).
    DuplicatePatch { id: String },
    /// Move a patch to position `to` (0-based) in the list.
    MovePatch { id: String, to: u32 },
    /// Mark or unmark a favourite.
    SetPatchFavourite { id: String, favourite: bool },
    /// Save keyboard part `part`'s sound (its patch, else its GM voice on the synth's
    /// SoundFont, with its volume and octave) as a new patch.
    SavePartAsPatch { part: u8, name: Option<String> },
    /// Add a SoundFont preset (`browseSoundFont`) as a new patch.
    AddPresetAsPatch { file: String, bank: u16, program: u8, name: Option<String> },
    /// Play a patch on its own for a moment (the band must be stopped).
    AuditionPatch { id: String },
    /// Play a SoundFont preset on its own for a moment, before adding it.
    AuditionPreset { file: String, bank: u16, program: u8 },
    StopPatchAudition,
    /// A GM family rule (`family` 0-15: programs 8·family .. 8·family+7), or with `style` the
    /// current style's own rule. `patch` None clears it. In the rule commands `patch` may
    /// also be a sound catalog id (#117): a preset or plugin becomes a library patch once.
    SetFamilyRule {
        family: u8,
        patch: Option<String>,
        #[serde(default)]
        style: bool,
    },
    /// A program override (GM program 0-127), global or the current style's own.
    SetProgramOverride {
        program: u8,
        patch: Option<String>,
        #[serde(default)]
        style: bool,
    },
    /// The drum rule (Rhythm 1/2 and any Yamaha drum kit bank), global or the style's own.
    SetDrumRule {
        patch: Option<String>,
        #[serde(default)]
        style: bool,
    },
    /// Forget the current style's own map (the global map applies again).
    ClearStyleMap,
    /// A keyboard part's own library patch (None: its GM voice, through the map).
    SetPartPatch { part: u8, id: Option<String> },
    /// The `yahaha` port gets the mapped bank/program instead of the style's own.
    SetPortSendsMapped { on: bool },
    /// List a SoundFont's presets (`soundLibrary.browse`); None closes the list.
    BrowseSoundFont { file: Option<String> },
    /// Add the patches of a library file (a full library or a bare patch list); with
    /// `replace`, it replaces the library instead. `maps`: take its program maps too.
    ImportSoundLibrary {
        path: String,
        #[serde(default)]
        replace: bool,
        #[serde(default)]
        maps: bool,
    },
    /// Write the library to a file (None: `sound-library-export.json` in the data folder).
    ExportSoundLibrary { path: Option<String> },
}

/// A patch as the state shows it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchInfo {
    #[serde(flatten)]
    pub patch: Patch,
    /// It plays itself (false: it plays the SoundFont fallback; `note` says why).
    pub available: bool,
    pub note: Option<String>,
}

/// A category, for pickers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryInfo {
    pub id: PatchCategory,
    pub label: String,
}

/// A program the current style sends a Style part, and what it plays.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgramUse {
    /// MIDI channel, 1-based (9-16).
    pub channel: u8,
    /// "Rhythm 1" .. "Phrase 2".
    pub part: String,
    pub msb: u8,
    pub lsb: u8,
    /// The program as sent, 0-127.
    pub program: u8,
    /// The GM program the map looks up (the program, or the synth's GM voice for a
    /// Yamaha-only bank).
    pub gm_program: u8,
    /// The voice as the synth plays it without the library ("Finger Bass (GM 34)").
    pub voice: String,
    /// A drum part (the drum rule applies).
    pub drums: bool,
    /// The patch it resolves to (None: the fallback).
    pub patch: Option<String>,
    pub rule: RuleKind,
    /// The rule is the style's own.
    pub from_style: bool,
    /// What sounds: the patch's name, or the fallback voice.
    pub plays: String,
}

/// A SoundFont's presets, being browsed.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundFontBrowse {
    pub file: String,
    pub presets: Vec<Preset>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundLibraryState {
    /// The patches, in the user's order.
    pub patches: Vec<PatchInfo>,
    /// The Genos voice categories, in display order.
    pub categories: Vec<CategoryInfo>,
    /// The 16 GM family names (family `i` = programs 8i..8i+7).
    pub families: Vec<String>,
    /// The global program map.
    pub map: ProgramMap,
    /// The current style's own map (empty: the global map alone).
    pub style_map: ProgramMap,
    /// What the style's own map is stored under (its file name).
    pub style_key: String,
    /// Every program the current style sends its parts, and what each plays.
    pub usage: Vec<ProgramUse>,
    /// The port gets mapped programs (`setPortSendsMapped`).
    pub port_sends_mapped: bool,
    /// The patch (id) being auditioned, or "preset" for a SoundFont preset.
    pub auditioning: Option<String>,
    /// The SoundFont whose presets are listed.
    pub browse: Option<SoundFontBrowse>,
    /// Where the library is saved (None: nowhere, e.g. an offline session).
    pub file: Option<String>,
    /// The SoundFonts the synth has loaded besides its own for library patches.
    pub extra_sound_fonts: Vec<String>,
    /// The id of the patch last created, duplicated or saved.
    pub last_added: Option<String>,
}

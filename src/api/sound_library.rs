//! The sound library (#103): patches, the program map, and what the current style uses.

pub use crate::patches::sf2::Preset;
pub use crate::patches::{Category as PatchCategory, Patch, PatchSource, PluginOrigin, ProgramMap, ProgramOverride, RuleKind};
use serde::{Deserialize, Serialize};

/// A patch's editable fields (all but its id). A sound is the raw instrument: it has no
/// mix (docs/racks.md). An older client's `defaults` is ignored.
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
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum SoundLibraryCmd {
    /// Add a patch at the end of the list (its id: `soundLibrary.lastAdded`).
    CreatePatch { patch: PatchFields },
    /// Change a patch: rename, recategorise, tags, favourite, source.
    UpdatePatch { id: String, patch: PatchFields },
    /// Delete a patch. Rules naming it go; parts playing it go back to their GM voice.
    DeletePatch { id: String },
    /// A copy of a patch, right after it (e.g. the same plugin with another state).
    DuplicatePatch { id: String },
    /// Move a patch to position `to` (0-based) in the list.
    MovePatch { id: String, to: u32 },
    /// Mark or unmark a favourite.
    SetPatchFavourite { id: String, favourite: bool },
    /// Save: keyboard part `part`'s sound as it plays now (its plugin's state; never the
    /// part's mix) over the Sound it plays. A sound that isn't the user's own (a factory
    /// preset, an `.aupreset` file) or none at all is saved as a new one (`saveSoundAs`).
    SaveSound { part: u8 },
    /// Save as…: keyboard part `part`'s sound as a new Sound (named `name`, else after what
    /// it plays), which the part then plays.
    SaveSoundAs { part: u8, name: Option<String> },
    /// The old "Save as patch": `saveSoundAs` (kept for older clients).
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
    /// Write the library as an export bundle (metadata, maps and every plugin sound's
    /// state; SoundFonts by file name) to a file (None: `sound-library-export.json` in the
    /// data folder).
    ExportSoundLibrary { path: Option<String> },
    /// Export plugin sound `id` as an `.aupreset` named after it, in the plugin's user
    /// preset folder (`~/Library/Audio/Presets/<Manufacturer>/<Plugin>/`, where Logic
    /// reads it). A preset of that name that exists is refused unless `overwrite`.
    ExportSoundPreset {
        id: String,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        overwrite: bool,
    },
}

/// A patch as the state shows it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchInfo {
    #[serde(flatten)]
    pub patch: PatchView,
    /// It plays itself (false: it plays the SoundFont fallback; `note` says why).
    pub available: bool,
    pub note: Option<String>,
}

/// A library [`Patch`] as the state shows it: the same fields, with a plugin source's
/// state (a base64 blob, MBs for a sampler) reduced to whether it has one. Whatever
/// needs the state reads the library's `Patch`, never this.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchView {
    pub id: String,
    pub name: String,
    pub category: PatchCategory,
    pub tags: Vec<String>,
    pub favourite: bool,
    pub source: PatchSourceView,
}

/// A [`PatchSource`] as the state shows it: a plugin's `hasState` in place of its state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PatchSourceView {
    SoundFont {
        file: String,
        bank: u16,
        program: u8,
    },
    /// `has_state`: the stored state is not empty (false on a factory preset: it has not
    /// played yet).
    Plugin {
        component_id: String,
        has_state: bool,
        #[serde(default, skip_serializing_if = "PluginOrigin::is_user")]
        origin: PluginOrigin,
    },
}

impl From<&PatchSource> for PatchSourceView {
    fn from(s: &PatchSource) -> Self {
        match s {
            PatchSource::SoundFont { file, bank, program } => PatchSourceView::SoundFont { file: file.clone(), bank: *bank, program: *program },
            PatchSource::Plugin { component_id, state, origin } => {
                PatchSourceView::Plugin { component_id: component_id.clone(), has_state: !state.is_empty(), origin: origin.clone() }
            }
        }
    }
}

/// `updatePatch`'s rule: a plugin source with no state, for the same plugin as the patch's
/// `old` source, keeps its stored state (the state shows none to send back).
pub fn keep_plugin_state(new: &mut PatchSource, old: &PatchSource) {
    if let (PatchSource::Plugin { component_id, state, .. }, PatchSource::Plugin { component_id: was, state: stored, .. }) = (new, old)
        && state.is_empty()
        && component_id == was
    {
        state.clone_from(stored);
    }
}

impl From<&Patch> for PatchView {
    fn from(p: &Patch) -> Self {
        PatchView {
            id: p.id.clone(),
            name: p.name.clone(),
            category: p.category,
            tags: p.tags.clone(),
            favourite: p.favourite,
            source: (&p.source).into(),
        }
    }
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
    /// The GM map for the style playing (docs/sound-browser.md): 129 rows, the drums then
    /// programs 0-127, each with its rules and what it resolves to (the Sound, the layer
    /// that decided it, and the font preset behind it).
    #[serde(default)]
    pub gm_map: Vec<crate::patches::GmMapRow>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plugin(state: &str) -> Patch {
        Patch {
            id: "keys".into(),
            name: "Keys".into(),
            category: PatchCategory::EPiano,
            tags: vec![],
            favourite: false,
            source: PatchSource::Plugin { component_id: "aumu Smp7 Fake".into(), state: state.into(), origin: PluginOrigin::Factory { number: 2 } },
        }
    }

    #[test]
    fn the_state_shows_whether_a_plugin_sound_has_state_never_the_state() {
        let json = serde_json::to_value(PatchInfo { patch: (&plugin("c2FtcGxlcg==")).into(), available: true, note: None }).unwrap();
        assert_eq!(json["source"], serde_json::json!({ "kind": "plugin", "componentId": "aumu Smp7 Fake", "hasState": true, "origin": { "kind": "factory", "number": 2 } }));
        assert!(!json.to_string().contains("c2FtcGxlcg"));
        let fresh = PatchView::from(&plugin(""));
        assert!(matches!(fresh.source, PatchSourceView::Plugin { has_state: false, .. }), "a factory preset that has not played");
        let back: PatchInfo = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(serde_json::to_value(back).unwrap(), json);
    }

    #[test]
    fn an_update_without_state_keeps_the_stored_one_for_the_same_plugin() {
        let stored = plugin("c2FtcGxlcg==").source;
        let mut same = plugin("").source;
        keep_plugin_state(&mut same, &stored);
        assert_eq!(same, stored);
        let mut other = PatchSource::plugin("aumu dls  appl", "");
        keep_plugin_state(&mut other, &stored);
        assert_eq!(other, PatchSource::plugin("aumu dls  appl", ""), "another plugin's state never carries over");
        let mut given = plugin("bmV3").source;
        keep_plugin_state(&mut given, &stored);
        assert_eq!(given, plugin("bmV3").source, "a state sent replaces it");
    }
}

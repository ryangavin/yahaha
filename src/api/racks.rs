//! Racks (docs/racks.md): the rack commands (new, load, save, save as, revert, rename,
//! duplicate, delete, with the switching guard), the user's racks, and the live rack's
//! name, where it came from and whether it has unsaved changes.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The rack commands. A rack is named by its `id` (stable across renames).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum RackCmd {
    /// Start a new rack (the parts' default sounds and mix). With unsaved changes and no
    /// `discard`, nothing changes: `liveRack.prompt` asks (`unsavedChanges`).
    NewRack {
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        discard: bool,
    },
    /// Load the user's rack `id`. The same guard as `newRack`.
    LoadRack {
        id: String,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        discard: bool,
    },
    /// Save the live rack over its own rack (none yet: as a new one, under its name), with
    /// its edited sounds. `soundNames` names the new sounds that edited factory, file or
    /// SoundFont presets become, by part (0-3).
    SaveRack {
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty", deserialize_with = "part_names")]
        sound_names: BTreeMap<u8, String>,
    },
    /// Save the live rack as a new rack called `name`, with its edited sounds.
    SaveRackAs {
        name: String,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty", deserialize_with = "part_names")]
        sound_names: BTreeMap<u8, String>,
    },
    /// Discard the live rack's changes: load its own rack again.
    RevertRack,
    RenameRack { id: String, name: String },
    DuplicateRack { id: String },
    /// Delete the user's rack `id` (not the loaded one).
    DeleteRack { id: String },
    /// Keep editing: the prompt (`liveRack.prompt`) goes, nothing else changes.
    DismissRackPrompt,
}

/// `soundNames`: names by part, keyed "0"-"3" as JSON object keys are strings (read from a
/// `serde_json::Value` too, which doesn't turn keys into numbers itself).
fn part_names<'de, D: serde::Deserializer<'de>>(d: D) -> Result<BTreeMap<u8, String>, D::Error> {
    BTreeMap::<String, String>::deserialize(d)?
        .into_iter()
        .map(|(k, v)| k.parse::<u8>().map(|p| (p, v)).map_err(|_| serde::de::Error::custom(format!("soundNames: no part {k:?}"))))
        .collect()
}

/// One of the user's racks (`<data>/Racks/*.rack.json`), for Library › Racks.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RackEntry {
    pub id: String,
    pub name: String,
    /// The sound each part plays, by name: Right 1, Right 2, Right 3, Left.
    pub parts: Vec<String>,
    /// Which parts are on.
    pub on: Vec<bool>,
    /// A part's sound is on a plugin that isn't installed (`plugins.needsAttention`).
    pub needs_attention: bool,
}

/// The live rack: what's under the player's hands now, autosaved and restored on boot.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveRackState {
    /// Its name: the saved rack's it came from, "Restored" (the first boot after racks
    /// came in), or "New rack".
    pub name: String,
    /// The id of the saved rack it came from; null when it came from none.
    pub id: Option<String>,
    /// It changed since it was loaded or saved: a sound, the mix, the split, Harmony/Arp,
    /// the transpose, the controller map, or a plugin edit (`soundEdited`).
    pub modified: bool,
    /// A rack command that needs the player's answer first; null when none.
    #[serde(default)]
    pub prompt: Option<RackPrompt>,
}

/// What a refused rack command asks the player.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum RackPrompt {
    /// `loadRack` or `newRack` with unsaved changes: Save first, Discard and switch (the
    /// same command with `discard`), or Keep editing (`dismissRackPrompt`).
    UnsavedChanges { then: RackSwitch },
    /// `saveRack` or `saveRackAs` (`saveAs` names the rack) found edited presets that
    /// become new sounds: each needs a name (`soundNames`).
    SoundNames { parts: Vec<SoundNameAsk>, save_as: Option<String> },
}

/// The switch a prompt holds back.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum RackSwitch {
    Load { id: String, name: String },
    New,
}

/// A part whose edited sound needs a name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundNameAsk {
    pub part: u8,
    /// The preset's own name, to start from.
    pub suggested: String,
}

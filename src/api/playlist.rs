//! The Playlist: set lists of registration banks and styles (docs/registration.md).

use crate::registration::{PlaylistSort, Record};
use serde::{Deserialize, Serialize};

/// Record indices are 0-based positions in the playlist file's order (`PlaylistRow::index`),
/// whatever the display order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PlaylistCmd {
    /// Start a new, empty, unsaved playlist.
    NewPlaylist,
    /// Open a playlist file (a path from `playlist.playlists`, or any file).
    LoadPlaylist { path: String },
    /// Save: to its file, or with `name` as a new file in the folder (Save As). Saves the
    /// displayed order and sets the sort back to Normal. Saving with the name of another
    /// playlist's file is refused unless `overwrite` is set.
    SavePlaylist {
        name: Option<String>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        overwrite: bool,
    },
    /// Add a record at the end.
    AddPlaylistRecord { record: Record },
    /// Add the bank in use as a record, recalling the selected button if one is lit.
    AddCurrentBank,
    /// Add the loaded style as a record.
    AddCurrentStyle,
    /// Append every record of another playlist file (Append Playlist).
    AppendPlaylist { path: String },
    /// Replace a record (Record Edit: name, target, the button it recalls).
    SetPlaylistRecord { index: usize, record: Record },
    /// Move a record up (-1) or down (+1). Refused while sorted.
    MovePlaylistRecord { index: usize, delta: i8 },
    /// Delete a record. Refused while sorted.
    DeletePlaylistRecord { index: usize },
    /// Display order: normal, A to Z, Z to A.
    SetPlaylistSort { sort: PlaylistSort },
    /// Load a record: its bank (and button) or its style.
    LoadPlaylistRecord { index: usize },
    /// Load the previous/next record in display order.
    StepPlaylist { delta: i8 },
}

/// The Playlist.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistState {
    pub name: String,
    /// Its file (None: not saved yet).
    pub path: Option<String>,
    pub dirty: bool,
    pub sort: PlaylistSort,
    /// The records in display order.
    pub records: Vec<PlaylistRow>,
    /// The record last loaded (file-order index).
    pub current: Option<usize>,
    /// The playlist files in the folder.
    pub playlists: Vec<PlaylistFileEntry>,
    /// The folder playlists are saved to (None: saving is off).
    pub folder: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistRow {
    /// Its position in the file (what the commands take).
    pub index: usize,
    pub record: Record,
    /// Its bank or style file is not there.
    pub missing: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistFileEntry {
    pub name: String,
    pub path: String,
}

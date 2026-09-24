//! Styles: the loaded style, the library, and loading styles from it.

use serde::{Deserialize, Serialize};

use super::gm_name;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum LibraryCmd {
    /// Load a style from the library by entry id (`LibraryEntry::id`). Playing or stopped.
    LoadStyle { id: usize },
    /// Load a style file by path (added to the library if it isn't in it).
    LoadStylePath { path: String },
    /// Previous/next style in library order, skipping files that don't load.
    StepStyle { delta: i8 },
    /// Load a style at the next bar line (playing), keeping the section and the bar
    /// position; stopped, the same as `LoadStyle`. A later one before that bar line
    /// replaces it; if the band stops first, it loads then. (`LoadStyle` and `StepStyle`
    /// wait for the bar line too while playing: `preview.queued` shows the style waiting.)
    QueueStyle { id: usize },
    /// Rescan the style folders (`library.roots`) for files added or removed, on a
    /// background thread (`library.scanning`). Ids stay the same for files still there;
    /// files gone leave the list.
    RescanLibrary,
}

/// The loaded style.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleState {
    /// Its library entry id (`LibraryEntry::id`).
    pub id: usize,
    pub path: String,
    /// The style's name (its SFF name, else the file name).
    pub name: String,
    /// "SFF1" or "SFF2".
    pub format: String,
    /// The style's own tempo, in BPM (the current tempo is `transport.tempo`).
    pub tempo: f64,
    /// Time signature, e.g. [4, 4].
    pub time_signature: [u8; 2],
    /// Names of the sections the style has, e.g. "Intro A", "Main B", "Fill In AA",
    /// "Fill In BA" (Break), "Ending C".
    pub sections: Vec<String>,
}

/// The style library. The entries themselves come from `Session::library()`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStatus {
    /// Changes whenever the library does (see `Event::LibraryChanged`).
    pub revision: u64,
    pub count: usize,
    /// The loaded style's position in library order (0-based).
    pub position: usize,
    /// Entries still waiting to be indexed.
    pub pending: usize,
    /// The style folders (and files) the library scans.
    pub roots: Vec<String>,
    /// A rescan (`RescanLibrary`) is walking the folders.
    pub scanning: bool,
}

/// A library entry (`Session::library_list`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryEntry {
    /// Stable for the session: `AppCmd::LoadStyle { id }`.
    pub id: usize,
    pub name: String,
    /// Folder relative to the scanned root, `/`-separated (the category).
    pub folder: String,
    pub path: String,
    /// "pending" (not indexed yet), "ok", or "error".
    pub status: String,
    /// Why it doesn't load (status "error").
    pub error: Option<String>,
    pub tempo: Option<f64>,
    pub time_signature: Option<[u8; 2]>,
    /// Short section list, e.g. "Main ABCD · Intro ABC · Ending ABC · Fill ABCD · Break".
    pub sections: String,
    /// "SFF1" or "SFF2" from the file's header; None while pending or unreadable.
    pub format: Option<String>,
}

/// The library in display order (folder, then name).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryList {
    pub revision: u64,
    pub entries: Vec<LibraryEntry>,
    /// The voices `SetPartVoice` picks from (the same for every revision).
    pub voices: Vec<VoiceOption>,
}

/// A voice a keyboard part can play: a GM program on bank 0 (the built-in synth plays
/// the SoundFont's GM bank; `SetPartVoice` takes the program).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceOption {
    pub program: u8,
    pub bank_msb: u8,
    pub bank_lsb: u8,
    /// e.g. "Grand Piano".
    pub name: String,
}

/// Every voice `SetPartVoice` can use.
pub fn voice_options() -> Vec<VoiceOption> {
    (0..128u8).map(|p| VoiceOption { program: p, bank_msb: 0, bank_lsb: 0, name: gm_name(p).to_string() }).collect()
}

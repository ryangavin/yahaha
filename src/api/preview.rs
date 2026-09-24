//! Style preview (audition) and the style waiting for the bar line.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PreviewCmd {
    /// Preview a style while the band is stopped: its Main A, at its own tempo, with its own
    /// voices and levels, over C Am F G7 (a chord a bar) for 4 bars, then it stops by itself.
    /// The loaded style, OTS, keyboard parts, mixer and transport are untouched. Refused
    /// (`failed`) while the band plays; a new one replaces the one playing. It ends early
    /// on `StopAudition`, a style change, START/STOP or a Sync Start chord.
    AuditionStyle { id: usize },
    /// End the style preview now.
    StopAudition,
}

/// Style preview and queue (`AuditionStyle`, `QueueStyle`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewState {
    /// The preview playing (None: none).
    pub audition: Option<AuditionState>,
    /// The library id of a style waiting for the next bar line to take over.
    pub queued: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditionState {
    /// The library id it previews.
    pub id: usize,
    /// The bar playing, 1-based, of `bars`.
    pub bar: u8,
    pub bars: u8,
    /// The chord playing, e.g. "Am".
    pub chord: Option<String>,
}

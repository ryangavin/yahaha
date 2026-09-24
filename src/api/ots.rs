//! One Touch Settings.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum OtsCmd {
    /// Recall One Touch Setting 1-4 (`index` 0-3) into the keyboard parts.
    RecallOts { index: u8 },
    /// OTS Link: Main A-D recall OTS 1-4.
    SetOtsLink { on: bool },
    ToggleOtsLink,
    /// OTS Link Timing: when Link recalls the OTS of a Main pressed during playback.
    SetOtsLinkTiming { timing: OtsLinkTiming },
}

/// OTS Link Timing (Style Setting, RM p.11). Stopped, a Main recalls its OTS at once
/// either way. A style change recalls the new style's OTS when that style takes over
/// (the bar line, the next beat, or the end of an Ending: `Engine::change_point`), under
/// both.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OtsLinkTiming {
    /// The moment the Main is pressed ("Real Time").
    Immediate,
    /// When that Main starts playing (its change point, or after its fill). The default:
    /// the owner's preference, so the sounds never change under the player while the old
    /// section still plays (the manual gives no default).
    #[default]
    MainChange,
}

/// One Touch Settings.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtsState {
    /// The style's One Touch Settings (0-4).
    pub settings: Vec<OtsSetting>,
    /// The last one recalled, 1-based (0 = none since the style loaded).
    pub applied: u8,
    pub link: bool,
    /// When OTS Link recalls during playback.
    pub link_timing: OtsLinkTiming,
}

/// One One Touch Setting (the style has no names for them: "OTS 1".."OTS 4").
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtsSetting {
    /// "OTS 1".."OTS 4".
    pub name: String,
    /// Right 1, Right 2, Right 3, Left as it sets them.
    pub parts: Vec<OtsPart>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtsPart {
    pub on: bool,
    /// GM program, or None for a drum kit voice (the part keeps its own).
    pub program: Option<u8>,
    pub voice_name: String,
    pub volume: u8,
    pub octave: i8,
}

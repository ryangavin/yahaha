//! What the Home screen shows (the redesign's section grid, docs/design): the style's Main
//! A-D patterns with a preview of each, the fill for each Main, the bar progress, the OTS
//! in use, and the band's effect sends. Read-only: every command that changes any of it is
//! elsewhere (sections, OTS, effects).

use serde::{Deserialize, Serialize};

/// The Home screen's data.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeState {
    /// Main A-D, always four (`present` false where the style has none).
    pub mains: Vec<HomeMain>,
    /// Where the section playing is.
    pub progress: HomeProgress,
    /// The One Touch Setting applied last, if any.
    pub ots: Option<HomeOts>,
    /// The band's effect sends (Reverb, Chorus, Delay).
    pub band_sends: Vec<HomeSend>,
}

/// One Main section and its fill.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeMain {
    /// "Main A".
    pub name: String,
    pub present: bool,
    /// Its pattern's length in bars (0 when absent).
    pub bars: u32,
    /// Grid steps per bar: sixteenth notes (16 in 4/4, 12 in 3/4).
    pub steps_per_bar: u8,
    /// Note-ons per step across the whole pattern (`bars * stepsPerBar` entries, capped at 255).
    pub density: Vec<u8>,
    /// The first bar as four lanes (`stepsPerBar` entries each, the loudest velocity on the
    /// step, 0 = none): kick, snare, hi-hats (the rhythm parts' GM drum keys) and the Bass part.
    pub lanes: HomeLanes,
    /// Its fill ("Fill In AA" for Main A).
    pub fill: HomeFill,
    /// It is the Main the style is on (or returns to).
    pub current: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeLanes {
    pub kick: Vec<u8>,
    pub snare: Vec<u8>,
    pub hats: Vec<u8>,
    pub bass: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeFill {
    pub name: String,
    pub present: bool,
    pub bars: u32,
    /// It is queued or playing now.
    pub active: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeProgress {
    pub running: bool,
    /// Bar and beat in the section playing, 1-based.
    pub bar: u32,
    pub beat: u32,
    /// The section's length in bars (None when stopped).
    pub bars: Option<u32>,
    pub beats_per_bar: u8,
    /// How far through the section, 0.0-1.0, at beat resolution (0 when stopped).
    pub fraction: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeOts {
    /// 0-3.
    pub index: u8,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeSend {
    pub block: super::FxBlock,
    /// "Reverb".
    pub name: String,
    /// Its effect type's name, e.g. "Hall 1".
    pub effect_name: String,
    /// The band send, 0-127 % (`setBandSend`).
    pub level: u8,
}

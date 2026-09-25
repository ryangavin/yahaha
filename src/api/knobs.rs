//! Knob Assign pages for the Launchkey's 8 encoders (#197; `src/knobs.rs`).

use crate::knobs::KnobPage;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum KnobsCmd {
    /// The Knob Assign page.
    SetKnobPage { page: KnobPage },
    /// Step the Knob Assign page by `delta`, stopping at the first and last (the encoder
    /// page buttons ▲/▼).
    StepKnobPage { delta: i8 },
    /// Turn knob `knob` (0-7) by `delta` steps (positive: clockwise). It runs the command
    /// of the knob's function on the page, from the value in effect.
    TurnKnob { knob: u8, delta: i8 },
}

/// The knobs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnobsState {
    pub page: KnobPage,
    /// "Style", "Parts".
    pub page_name: String,
    /// 1-based page number, and how many pages there are.
    pub page_number: u8,
    pub page_count: u8,
    /// Knobs 1-8 on this page.
    pub knobs: Vec<KnobState>,
}

impl Default for KnobsState {
    fn default() -> KnobsState {
        KnobsState { page: KnobPage::default(), page_name: KnobPage::default().name().into(), page_number: 1, page_count: KnobPage::ALL.len() as u8, knobs: Vec::new() }
    }
}

/// One knob on the page.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnobState {
    /// The function: `none`, `dynamics`, `retriggerRate`, `retriggerOnOff`, `trackMuteA`,
    /// `trackMuteB`, `tempo`, `partVolume`, `harmonyVolume`, `metronomeVolume`.
    pub function: String,
    /// "Dynamics Control", and its short name for small displays ("DynCtrl", up to 8
    /// characters; "---" for No Assign).
    pub name: String,
    pub short: String,
    /// The value as text ("64", "1/8", "On", "3 of 8", "120 BPM"); empty for No Assign.
    pub value: String,
    /// Where the knob is, 0-127 (the Genos LED ring); null when it has no position (tempo,
    /// No Assign).
    pub level: Option<u8>,
}

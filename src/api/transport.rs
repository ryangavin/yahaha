//! Sections and transport: the Genos panel buttons the engine runs.

use crate::engine::{Button, FadeState, StopAcmp};
use serde::{Deserialize, Serialize};

use super::Pad;

/// Sections, START/STOP, Sync Start/Stop, Auto Fill, Stop ACMP, tempo.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum TransportCmd {
    /// Intro 1-3 (`index` 0-2). Stopped: the intro plays when the style starts.
    /// Playing: queued for the next bar.
    Intro { index: u8 },
    /// Main A-D (`index` 0-3). Pressing the Main that is playing plays its fill.
    Main { index: u8 },
    /// Break (Fill In BA).
    Break,
    /// Fill Down (`delta` -1), Fill Self (0), Fill Up (1): a fill, then the Main to the
    /// left, the same one, or the one to the right (the Genos assignable functions). The
    /// same as `FillDown`, `FillSelf` and `FillUp`.
    Fill { delta: i8 },
    /// Ending 1-3 (`index` 0-2): ends the style after the ending.
    Ending { index: u8 },
    /// START/STOP.
    StartStop,
    /// Stop (the Launchkey Stop button): stops if playing, else nothing.
    Stop,
    /// SYNC START on/off.
    ToggleSyncStart,
    /// SYNC STOP on/off. The engine ignores it while `transport.syncStopAvailable` is false.
    ToggleSyncStop,
    /// AUTO FILL IN on/off.
    ToggleAutoFill,
    /// STOP ACMP on/off: Off <-> the mode last on (Style at first).
    ToggleStopAcmp,
    /// The Stop Accompaniment mode (Style Setting > Stop ACMP).
    SetStopAcmp { mode: StopAcmpMode },
    /// Fill Up: a fill, then the next Main to the right (RM p.142).
    FillUp,
    /// Fill Down: a fill, then the next Main to the left.
    FillDown,
    /// Fill Self: the Main's own fill.
    FillSelf,
    /// Fill Break: the Break (the same as `Break`).
    FillBreak,
    /// Half Bar Fill In on/off, and set.
    ToggleHalfBarFill,
    SetHalfBarFill { on: bool },
    /// TAP TEMPO: taps set the tempo. While the style plays with Style Section Reset on
    /// (`styleSettings.sectionReset`, the default), a tap rewinds the section instead.
    TapTempo,
    /// Tempo up/down one step (1 BPM).
    TempoUp,
    TempoDown,
    /// TEMPO − and + together: back to the tempo the style came with (OM p.46).
    ResetTempo,
    /// FADE IN/OUT: stopped, arm (or disarm) a fade in for the next start; playing, fade
    /// out and stop (`transport.fade`).
    ToggleFade,
    /// Style Section Reset: the section playing starts again from its top, now.
    SectionReset,
    /// Style Retrigger on/off: while on, each chord played in a Main restarts it and its
    /// first `styleSettings.retriggerRate`-th note loops (`transport.retrigger`).
    ToggleRetrigger,
    /// Set the tempo, in BPM (5-500; clamped).
    SetTempo { bpm: u16 },
}

impl TransportCmd {
    /// The engine button this command is (every transport command is one).
    pub fn button(&self) -> Button {
        match *self {
            TransportCmd::Intro { index } => Button::Intro(index.min(3)),
            TransportCmd::Main { index } => Button::Main(index.min(3)),
            TransportCmd::Break => Button::Break,
            TransportCmd::Fill { delta } => match delta.signum() {
                -1 => Button::FillDown,
                1 => Button::FillUp,
                _ => Button::FillSelf,
            },
            TransportCmd::Ending { index } => Button::Ending(index.min(3)),
            TransportCmd::StartStop => Button::StartStop,
            TransportCmd::Stop => Button::Stop,
            TransportCmd::ToggleSyncStart => Button::SyncStart,
            TransportCmd::ToggleSyncStop => Button::SyncStop,
            TransportCmd::ToggleAutoFill => Button::AutoFill,
            TransportCmd::TapTempo => Button::TapTempo,
            TransportCmd::TempoUp => Button::TempoUp,
            TransportCmd::TempoDown => Button::TempoDown,
            TransportCmd::ResetTempo => Button::TempoReset,
            TransportCmd::SetTempo { bpm } => Button::SetTempo(bpm),
            TransportCmd::ToggleStopAcmp => Button::StopAcmp,
            TransportCmd::SetStopAcmp { mode } => Button::SetStopAcmp(mode.into()),
            TransportCmd::FillUp => Button::FillUp,
            TransportCmd::FillDown => Button::FillDown,
            TransportCmd::FillSelf => Button::FillSelf,
            TransportCmd::FillBreak => Button::Break,
            TransportCmd::ToggleHalfBarFill => Button::HalfBarFill,
            TransportCmd::SetHalfBarFill { on } => Button::SetHalfBarFill(on),
            TransportCmd::ToggleFade => Button::Fade,
            TransportCmd::SectionReset => Button::SectionReset,
            TransportCmd::ToggleRetrigger => Button::Retrigger,
        }
    }
}

/// Playback state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransportState {
    pub running: bool,
    /// Sync Start is armed: the first chord starts the style.
    pub sync_start: bool,
    pub sync_stop: bool,
    /// Sync Stop can be turned on (not in the Full Keyboard fingering types in Lower).
    pub sync_stop_available: bool,
    pub auto_fill: bool,
    /// Stop Accompaniment sounds the chord (`stop_acmp_mode` is not Off).
    pub stop_acmp: bool,
    /// The section playing (None when stopped), e.g. "Main A", "Fill In AA".
    pub section: Option<String>,
    /// The section queued to play next (at the next bar; a fill at the next beat).
    pub queued: Option<String>,
    /// The Intro (0-2) armed to play when the style starts.
    pub pending_intro: Option<u8>,
    /// The Main section (0-3 = A-D) the style is on, or returns to after a fill.
    pub main: u8,
    /// Position in the section playing: bar and beat, both 1-based (1, 1 when stopped).
    pub bar: u32,
    pub beat: u32,
    /// Beats per bar (the time signature's numerator).
    pub beats_per_bar: u8,
    /// How many bars the section playing lasts (a Main's pattern length; it loops). None
    /// when stopped.
    pub section_bars: Option<u32>,
    /// Current tempo in BPM.
    pub tempo: f64,
    /// Page 1 of the Launchkey pads (sections, Sync Start/Stop, Auto Fill, Tap, Start/Stop),
    /// whatever page the hardware is on: the section lamps exactly as the pads show them.
    pub lamps: Vec<Pad>,
    /// Fade In/Out: "off", "armed" (stopped; START fades in), "fadingIn", "fadingOut",
    /// "holding" (faded out and stopped; silent for the hold time).
    pub fade: FadeState,
    /// Style Retrigger is on.
    pub retrigger: bool,
    /// The Ending is slowing down (pressed again while it plays).
    pub ritardando: bool,
    /// Half Bar Fill In.
    pub half_bar_fill: bool,
    /// Stop Accompaniment: what a chord sounds on with the band stopped and Sync Start off.
    pub stop_acmp_mode: StopAcmpMode,
}

/// Stop Accompaniment (Style Setting > Stop ACMP, RM p.11).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StopAcmpMode {
    /// The chord doesn't sound.
    #[default]
    Off,
    /// It sounds on the style's Bass and Pad voices.
    Style,
    /// It sounds on fixed Bass and Pad voices (Finger Bass, Warm Pad), whatever the style.
    Fixed,
}

impl From<StopAcmpMode> for StopAcmp {
    fn from(m: StopAcmpMode) -> StopAcmp {
        match m {
            StopAcmpMode::Off => StopAcmp::Off,
            StopAcmpMode::Style => StopAcmp::Style,
            StopAcmpMode::Fixed => StopAcmp::Fixed,
        }
    }
}

impl From<StopAcmp> for StopAcmpMode {
    fn from(m: StopAcmp) -> StopAcmpMode {
        match m {
            StopAcmp::Off => StopAcmpMode::Off,
            StopAcmp::Style => StopAcmpMode::Style,
            StopAcmp::Fixed => StopAcmpMode::Fixed,
        }
    }
}

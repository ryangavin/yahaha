//! Registration Memory: the ten buttons, banks, Memorize, Freeze and the Registration
//! Sequence (docs/registration.md).

use crate::registration::{Group, Groups, SequenceEnd};
use serde::{Deserialize, Serialize};

/// Button indices are 0-based (0-9 = the panel's [1]-[10]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum RegistrationCmd {
    /// A REGISTRATION MEMORY button as the panel has it: recalls the button, or, while
    /// MEMORY is armed (`toggleRegistMemory`), memorizes the panel into it.
    PressRegist { index: u8 },
    /// Recall a button (refused if it is empty).
    RecallRegist { index: u8 },
    /// Memorize the panel into a button (the `memorizeGroups`), replacing what it held.
    MemorizeRegist { index: u8 },
    /// The MEMORY button: arm (or disarm) Memorize for the next button press.
    ToggleRegistMemory,
    /// Tick or untick a group in the Memory window (what Memorize stores).
    SetMemorizeGroup { group: Group, on: bool },
    /// Empty a button (Regist Bank Edit, Delete).
    ClearRegist { index: u8 },
    /// Rename a button (Regist Bank Edit, Rename).
    RenameRegist { index: u8, name: String },
    /// REGIST BANK -/+: the previous/next bank file in the folder.
    StepRegistBank { delta: i8 },
    /// Load a bank file (its path, as `registration.banks` lists it).
    SelectRegistBank { path: String },
    /// Start a new, empty, unsaved bank.
    NewRegistBank,
    /// Save the bank: to its file, or with `name` as a new file in the folder (Save As).
    /// Saving with the name of another bank's file is refused unless `overwrite` is set.
    SaveRegistBank {
        name: Option<String>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        overwrite: bool,
    },
    /// Registration Freeze on/off (the FREEZE button).
    SetFreeze { on: bool },
    ToggleFreeze,
    /// Tick or untick a group on the Regist Freeze display.
    SetFreezeGroup { group: Group, on: bool },
    /// Program the bank's Registration Sequence: buttons (0-9) in order, and the end action.
    SetRegistSequence { steps: Vec<u8>, end: SequenceEnd },
    /// Registration Sequence on/off.
    SetRegistSequenceOn { on: bool },
    ToggleRegistSequence,
    /// Regist +/- (`delta` 1 / -1): the next/previous step of the sequence.
    StepRegistSequence { delta: i8 },
}

/// Registration Memory.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrationState {
    /// The bank in use.
    pub bank: BankState,
    /// The bank files in the folder, in order (REGIST BANK -/+ step through them).
    pub banks: Vec<BankFile>,
    /// The folder bank files are saved to (None: saving is off, e.g. an offline session).
    pub folder: Option<String>,
    /// Always 10: buttons [1]-[10] (Regist Bank Info).
    pub buttons: Vec<RegistButton>,
    /// The button last recalled or memorized (lit red), 0-based.
    pub selected: Option<u8>,
    /// MEMORY is armed: the next button press memorizes.
    pub memory: bool,
    /// The Memory window's ticked groups.
    pub memorize_groups: Groups,
    /// Registration Freeze is on.
    pub freeze: bool,
    /// The Regist Freeze display's ticked groups (they stay unchanged on recall while
    /// `freeze` is on).
    pub freeze_groups: Groups,
    pub sequence: SequenceState,
    /// A recall is waiting for the style it loads to take over (the bar line when playing);
    /// the rest of the registration follows then.
    pub pending: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BankState {
    pub name: String,
    /// Its file (None: a new bank not saved yet).
    pub path: Option<String>,
    /// Changed since it was loaded or saved.
    pub dirty: bool,
    /// Its place in `banks` (None when unsaved or outside the folder).
    pub position: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BankFile {
    pub name: String,
    pub path: String,
}

/// One Registration Memory button, as Regist Bank Info shows it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistButton {
    /// 0-based.
    pub index: u8,
    /// It holds a registration (lit blue, or red when selected).
    pub stored: bool,
    pub name: String,
    /// The groups it memorized.
    pub groups: Groups,
    /// The style it loads (name), if it stores one.
    pub style: Option<String>,
    pub tempo: Option<f64>,
    /// Right 1, Right 2, Right 3, Left: the voice names it sets, if it stores them.
    pub voices: Vec<RegistVoice>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistVoice {
    pub name: String,
    pub on: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SequenceState {
    pub on: bool,
    /// Buttons (0-based) in order.
    pub steps: Vec<u8>,
    pub end: SequenceEnd,
    /// The step last recalled (0-based into `steps`); None before the first.
    pub position: Option<usize>,
}

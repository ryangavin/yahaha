//! Snapshots (the Genos's Registration Memory): eight per snapshot bank (Bank A, B, ...
//! in a bank file), Store (the Genos's MEMORY), Freeze and the Registration Sequence
//! (docs/registration.md). The wire names keep the Genos words (`pressRegist`,
//! `toggleRegistMemory`, `memory`) so older clients and saved controller maps still work.

use crate::registration::{Group, Groups, SequenceEnd};
use serde::{Deserialize, Serialize};

/// `index` is a snapshot's index in the bank file, 0-based: `bank * 8 + slot` (0 = A1, 9 =
/// B2). `slot` is 0-7 within the snapshot bank on view (`snapshotBank`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum RegistrationCmd {
    /// A snapshot button as the panel has it: recalls the snapshot, or, while Store is
    /// armed (`toggleRegistMemory`), stores the panel into it.
    PressRegist { index: u8 },
    /// Snapshot button `slot` (0-7) of the snapshot bank on view, as `pressRegist` does
    /// (the Launchkey pads and the Regist 1-8 assignable functions send this). Slots past
    /// 7 run on into the next banks (Regist 9-10: the next bank's 1-2).
    PressSnapshot { slot: u8 },
    /// Snapshot bank -/+: view (and so press) the previous/next eight. It stops at Bank A
    /// and at one empty bank past the last stored one (at most Bank H).
    StepSnapshotBank { delta: i8 },
    /// View snapshot bank `bank` (0 = A).
    SelectSnapshotBank { bank: u8 },
    /// Recall a button (refused if it is empty).
    RecallRegist { index: u8 },
    /// Memorize the panel into a button (the `memorizeGroups`), replacing what it held.
    MemorizeRegist { index: u8 },
    /// The STORE button (the Genos's MEMORY): arm (or disarm) Store for the next button
    /// press.
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
    /// Regist +/- from a pedal (`delta` 1 / -1; RM p.114 Pedal Control): the sequence's
    /// next/previous step while it is on and has steps, else the bank's next/previous
    /// stored button (from none: + the first, − the last; it stops at either end).
    StepRegist { delta: i8 },
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
    /// Every snapshot of the bank file, by index (Regist Bank Info): whole snapshot banks
    /// of eight, through the bank on view (`snapshotBank`).
    pub buttons: Vec<RegistButton>,
    /// The snapshot last recalled or stored (lit red): its index.
    pub selected: Option<u8>,
    /// Store is armed: the next button press stores.
    pub memory: bool,
    /// The snapshot bank on view, 0-based (0 = Bank A): the Launchkey pads and
    /// `pressSnapshot` press its eight.
    #[serde(default)]
    pub snapshot_bank: u8,
    /// Snapshot banks the file holds (at least 1); `stepSnapshotBank` goes one past.
    #[serde(default = "one")]
    pub snapshot_banks: u8,
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

fn one() -> u8 {
    1
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

/// One snapshot, as Regist Bank Info shows it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistButton {
    /// Its index in the bank file, 0-based (`bank * 8 + slot`).
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
    /// Snapshot indices in order.
    pub steps: Vec<u8>,
    pub end: SequenceEnd,
    /// The step last recalled (0-based into `steps`); None before the first.
    pub position: Option<usize>,
}

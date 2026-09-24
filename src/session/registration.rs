//! Registration Memory: the ten buttons, banks, Memorize, Freeze, the Registration
//! Sequence (docs/registration.md).
//!
//! A registration is recalled in two phases. The style comes first (`Registrable::early`);
//! when it changes, the rest waits until the engine plays the new style (at once when
//! stopped, at the bar line when playing), because a style load resets the tempo, the
//! Style part mixer and the section. Everything else is then recalled in `REGISTRABLES`
//! order. While a recall settles, OTS Link holds still (`registration_holds_ots`): the
//! registration's own voices win over the OTS of the section it selects.

mod sections;

pub(super) use sections::REGISTRABLES;

use super::Control;
use crate::api::{BankFile, BankState, CmdError, RegistButton, RegistVoice, RegistrationCmd, RegistrationState, SequenceState};
use crate::launchkey::RegistPanel;
use crate::registration::{self as reg, Bank, Groups, Memory, SeqMove, BANK_EXT, BUTTONS};
use std::path::{Path, PathBuf};

/// How long OTS Link waits for a recall to settle at most (a section that never comes,
/// because the player chose another one first).
const OTS_HOLD_NS: u64 = 8_000_000_000;

/// Items Parameter Lock can protect from Registration (and Playlist) recall. The lock
/// state belongs to Parameter Lock (#m5-rules); recall asks `param_locked`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LockItem {
    /// Split points (Data List lock group "Split Point").
    SplitPoint,
    /// Fingering type and Chord Detection Area (lock group "Fingering Type").
    FingeringType,
}

/// The control side's Registration Memory.
pub(super) struct RegState {
    /// Where bank files live (None: no saving, e.g. offline sessions without a data dir).
    pub(super) dir: Option<PathBuf>,
    bank: Bank,
    path: Option<PathBuf>,
    dirty: bool,
    /// Bank files in `dir`, in order (refreshed when banks are stepped, saved or loaded).
    banks: Vec<PathBuf>,
    selected: Option<u8>,
    memory: bool,
    memorize: Groups,
    freeze: bool,
    frozen: Groups,
    seq_pos: Option<usize>,
    /// A recall waiting for its style to play: the memory and the groups it recalls.
    deferred: Option<(Memory, Groups)>,
    /// OTS Link waits for this recall to settle: the Main it selected (None: only a style
    /// change), and when the hold started (0 = not yet stamped).
    ots_hold: Option<(Option<u8>, u64)>,
    /// Regist Bank Info, rebuilt when the bank changes.
    buttons: Vec<RegistButton>,
}

impl RegState {
    pub(super) fn new(dir: Option<PathBuf>) -> RegState {
        let mut r = RegState {
            dir,
            bank: Bank::default(),
            path: None,
            dirty: false,
            banks: Vec::new(),
            selected: None,
            memory: false,
            memorize: Groups::all(),
            freeze: false,
            frozen: Groups::NONE,
            seq_pos: None,
            deferred: None,
            ots_hold: None,
            buttons: Vec::new(),
        };
        r.list_banks();
        r.summarize();
        r
    }

    fn list_banks(&mut self) {
        self.banks = self.dir.as_deref().map(reg::list_banks).unwrap_or_default();
    }

    /// Rebuild Regist Bank Info from the bank.
    fn summarize(&mut self) {
        self.buttons = (0..BUTTONS).map(|i| summary(i as u8, self.bank.memories[i].as_ref())).collect();
    }

    fn position(&self) -> Option<usize> {
        let p = self.path.as_ref()?;
        self.banks.iter().position(|b| b == p)
    }

    /// The groups a recall of `m` changes: what it memorized, less the frozen groups.
    fn recall_groups(&self, m: &Memory) -> Groups {
        if self.freeze { m.groups.minus(self.frozen) } else { m.groups }
    }
}

/// Regist Bank Info for one button.
fn summary(index: u8, m: Option<&Memory>) -> RegistButton {
    let Some(m) = m else { return RegistButton { index, ..RegistButton::default() } };
    let info = sections::info(m);
    RegistButton {
        index,
        stored: true,
        name: m.name.clone(),
        groups: m.groups,
        style: info.style,
        tempo: info.tempo,
        voices: info.voices.into_iter().map(|(name, on)| RegistVoice { name, on }).collect(),
    }
}

impl Control {
    pub(super) fn registration_cmd(&mut self, c: RegistrationCmd) -> Result<(), CmdError> {
        match c {
            RegistrationCmd::PressRegist { index } => {
                if self.reg.memory {
                    return self.memorize(index);
                }
                return self.recall_button(index);
            }
            RegistrationCmd::RecallRegist { index } => return self.recall_button(index),
            RegistrationCmd::MemorizeRegist { index } => return self.memorize(index),
            RegistrationCmd::ToggleRegistMemory => self.reg.memory = !self.reg.memory,
            RegistrationCmd::SetMemorizeGroup { group, on } => self.reg.memorize.set(group, on),
            RegistrationCmd::ClearRegist { index } => {
                let i = self.button(index)?;
                self.reg.bank.memories[i] = None;
                if self.reg.selected == Some(index) {
                    self.reg.selected = None;
                }
                return self.bank_changed();
            }
            RegistrationCmd::RenameRegist { index, name } => {
                let i = self.button(index)?;
                match self.reg.bank.memories[i].as_mut() {
                    Some(m) => m.name = name.trim().to_string(),
                    None => return self.fail(format!("Registration {} is empty", index + 1)),
                }
                return self.bank_changed();
            }
            RegistrationCmd::StepRegistBank { delta } => return self.step_bank(delta, false),
            RegistrationCmd::SelectRegistBank { path } => return self.load_bank(Path::new(&path)),
            RegistrationCmd::NewRegistBank => {
                self.reg.bank = Bank::default();
                self.reg.path = None;
                self.reg.dirty = false;
                self.reg.selected = None;
                self.reg.seq_pos = None;
                self.reg.summarize();
            }
            RegistrationCmd::SaveRegistBank { name } => return self.save_bank(name),
            RegistrationCmd::SetFreeze { on } => self.reg.freeze = on,
            RegistrationCmd::ToggleFreeze => self.reg.freeze = !self.reg.freeze,
            RegistrationCmd::SetFreezeGroup { group, on } => self.reg.frozen.set(group, on),
            RegistrationCmd::SetRegistSequence { steps, end } => {
                let on = self.reg.bank.sequence.on;
                self.reg.bank.sequence = reg::Sequence { on, steps, end }.clean();
                self.reg.seq_pos = None;
                return self.bank_changed();
            }
            RegistrationCmd::SetRegistSequenceOn { on } => {
                self.reg.bank.sequence.on = on;
                return self.bank_changed();
            }
            RegistrationCmd::ToggleRegistSequence => {
                self.reg.bank.sequence.on = !self.reg.bank.sequence.on;
                return self.bank_changed();
            }
            RegistrationCmd::StepRegistSequence { delta } => return self.step_sequence(delta),
        }
        Ok(())
    }

    fn button(&mut self, index: u8) -> Result<usize, CmdError> {
        if (index as usize) < BUTTONS {
            Ok(index as usize)
        } else {
            Err(self.fail(format!("no Registration button {}", index as usize + 1)).unwrap_err())
        }
    }

    /// Memorize the panel into button `index` (the Memory window's groups).
    fn memorize(&mut self, index: u8) -> Result<(), CmdError> {
        let i = self.button(index)?;
        self.reg.memory = false;
        let groups = self.reg.memorize;
        if groups.is_empty() {
            return self.fail("Memorize: no groups ticked");
        }
        let mut m = Memory { name: String::new(), groups, sections: Default::default() };
        for r in REGISTRABLES {
            if let Some(v) = (r.capture)(self, groups) {
                m.sections.insert(r.key.to_string(), v);
            }
        }
        // Named after its style, as Regist Bank Info shows a button (rename to change).
        m.name = sections::info(&m).style.unwrap_or_else(|| format!("Registration {}", i + 1));
        self.reg.bank.memories[i] = Some(m);
        self.reg.selected = Some(index);
        self.say(format!("Memorized to Registration {}", i + 1), false);
        self.bank_changed()
    }

    /// The bank changed: rebuild the info, and save it to its file (a new bank waits for
    /// `SaveRegistBank` with a name).
    fn bank_changed(&mut self) -> Result<(), CmdError> {
        self.reg.summarize();
        self.reg.dirty = true;
        match self.reg.path.clone() {
            Some(p) => self.write_bank(&p),
            None => Ok(()),
        }
    }

    fn write_bank(&mut self, path: &Path) -> Result<(), CmdError> {
        match self.reg.bank.save(path) {
            Ok(()) => {
                self.reg.dirty = false;
                Ok(())
            }
            Err(e) => self.fail(format!("saving the bank: {e:#}")),
        }
    }

    fn save_bank(&mut self, name: Option<String>) -> Result<(), CmdError> {
        let path = match (name, &self.reg.path) {
            (None, Some(p)) => p.clone(),
            (name, _) => {
                let Some(dir) = self.reg.dir.clone() else {
                    return self.fail("no Registration folder to save to");
                };
                let name = name.unwrap_or_else(|| self.reg.bank.name.clone());
                self.reg.bank.name = name.trim().to_string();
                dir.join(reg::file_name(&name, BANK_EXT))
            }
        };
        self.write_bank(&path)?;
        self.reg.path = Some(path);
        self.reg.list_banks();
        self.say(format!("Saved bank {}", self.reg.bank.name), false);
        Ok(())
    }

    /// Load a bank file; its buttons are not recalled (the Genos lights them blue).
    pub(super) fn load_bank(&mut self, path: &Path) -> Result<(), CmdError> {
        match Bank::load(path) {
            Ok(mut b) => {
                b.name = reg::bank_name(path);
                b.sequence = b.sequence.clean();
                self.reg.bank = b;
                self.reg.path = Some(path.to_path_buf());
                self.reg.dirty = false;
                self.reg.selected = None;
                self.reg.memory = false;
                self.reg.seq_pos = None;
                self.reg.summarize();
                self.reg.list_banks();
                Ok(())
            }
            Err(e) => self.fail(format!("{e:#}")),
        }
    }

    /// REGIST BANK -/+ (and the sequence's "Next"): the neighbouring bank file, stopping at
    /// the first and last. From an unsaved bank, + goes to the first file and - to the last.
    fn step_bank(&mut self, delta: i8, quiet: bool) -> Result<(), CmdError> {
        self.reg.list_banks();
        let n = self.reg.banks.len();
        if n == 0 {
            return if quiet { Ok(()) } else { self.fail("no Registration banks saved yet") };
        }
        let next = match self.reg.position() {
            Some(p) => {
                let q = p as i64 + delta.signum() as i64;
                if q < 0 || q >= n as i64 {
                    return Ok(());
                }
                q as usize
            }
            None if delta >= 0 => 0,
            None => n - 1,
        };
        let path = self.reg.banks[next].clone();
        self.load_bank(&path)?;
        self.say(format!("Bank: {}", self.reg.bank.name), false);
        Ok(())
    }

    /// Regist +/-: the Registration Sequence's next/previous step.
    fn step_sequence(&mut self, delta: i8) -> Result<(), CmdError> {
        let seq = &self.reg.bank.sequence;
        if !seq.on {
            return self.fail("Registration Sequence is off");
        }
        match seq.step(self.reg.seq_pos, delta) {
            SeqMove::Stay => Ok(()),
            SeqMove::Step(p) => {
                let b = self.reg.bank.sequence.steps[p];
                self.reg.seq_pos = Some(p);
                self.recall_index(b, false)
            }
            SeqMove::NextBank | SeqMove::PrevBank => {
                let fwd = delta > 0;
                let before = self.reg.path.clone();
                self.step_bank(delta, true)?;
                if self.reg.path == before {
                    return Ok(()); // no bank beyond this one
                }
                let seq = &self.reg.bank.sequence;
                if seq.steps.is_empty() {
                    return Ok(());
                }
                let p = if fwd { 0 } else { seq.steps.len() - 1 };
                let b = seq.steps[p];
                self.reg.seq_pos = Some(p);
                self.recall_index(b, false)
            }
        }
    }

    /// A button pressed (or picked in the app): recall it, and move the sequence cursor to it.
    fn recall_button(&mut self, index: u8) -> Result<(), CmdError> {
        self.recall_index(index, true)
    }

    /// Recall button `index` of the bank in use.
    pub(super) fn recall_index(&mut self, index: u8, follow: bool) -> Result<(), CmdError> {
        let i = self.button(index)?;
        let Some(m) = self.reg.bank.memories[i].clone() else {
            return self.fail(format!("Registration {} is empty", i + 1));
        };
        self.reg.memory = false;
        self.reg.selected = Some(index);
        if follow {
            self.reg.seq_pos = self.reg.bank.sequence.follow(self.reg.seq_pos, index);
        }
        let groups = self.reg.recall_groups(&m);
        let name = if m.name.is_empty() { format!("Registration {}", i + 1) } else { m.name.clone() };
        self.recall(m, groups);
        self.say(format!("Registration {}: {name}", i + 1), false);
        Ok(())
    }

    /// Recall a memory's sections in `groups`: the early ones now; the rest now, or once
    /// the style the recall loads plays.
    fn recall(&mut self, m: Memory, groups: Groups) {
        self.reg.deferred = None;
        let before = self.pending_style.as_ref().map(|p| p.1);
        let mut errors = Vec::new();
        for r in REGISTRABLES.iter().filter(|r| r.early) {
            if let Some(v) = m.sections.get(r.key)
                && let Err(e) = (r.recall)(self, v, groups)
            {
                errors.push(e);
            }
        }
        let waiting = self.pending_style.as_ref().map(|p| p.1) != before && self.pending_style.is_some();
        if waiting {
            self.reg.ots_hold = Some((None, 0));
            self.reg.deferred = Some((m, groups));
        } else {
            errors.extend(self.recall_late(&m, groups));
        }
        if !errors.is_empty() {
            self.say(errors.join("; "), true);
        }
    }

    /// The sections after the style, in order.
    fn recall_late(&mut self, m: &Memory, groups: Groups) -> Vec<String> {
        let mut errors = Vec::new();
        for r in REGISTRABLES.iter().filter(|r| !r.early) {
            if let Some(v) = m.sections.get(r.key)
                && let Err(e) = (r.recall)(self, v, groups)
            {
                errors.push(e);
            }
        }
        errors
    }

    /// A recall's section change: OTS Link holds until the engine is on Main `main`.
    pub(super) fn hold_ots_for(&mut self, main: Option<u8>) {
        let main = main.or(self.reg.ots_hold.and_then(|h| h.0));
        self.reg.ots_hold = Some((main, 0));
    }

    /// OTS Link must not recall now: a registration is still settling (its style or its
    /// section hasn't come yet). `pump_ots_link` asks.
    pub(super) fn registration_holds_ots(&self) -> bool {
        self.reg.ots_hold.is_some()
    }

    /// Every pump: a deferred recall runs once its style plays; the OTS Link hold ends
    /// when the engine shows what the recall asked for (or after `OTS_HOLD_NS`).
    pub(super) fn pump_registration(&mut self, now: u64) {
        if self.reg.deferred.is_some() && self.pending_style.is_none() {
            let (m, groups) = self.reg.deferred.take().unwrap();
            let errors = self.recall_late(&m, groups);
            if !errors.is_empty() {
                self.say(errors.join("; "), true);
            }
        }
        if let Some((main, since)) = self.reg.ots_hold {
            if since == 0 {
                self.reg.ots_hold = Some((main, now.max(1)));
                return;
            }
            let s = &self.snap;
            let settled = self.reg.deferred.is_none()
                && self.pending_style.is_none()
                && !s.style_pending
                && main.is_none_or(|m| s.main == m);
            if settled || now.saturating_sub(since) > OTS_HOLD_NS {
                self.reg.ots_hold = None;
            }
        }
    }

    /// The parameter lock for `item` (Parameter Lock, RM p.163): a locked item only
    /// changes from the panel, never from Registration, OTS or Playlist recall.
    /// Parameter Lock itself is not built yet: nothing is locked.
    pub(super) fn param_locked(&self, item: LockItem) -> bool {
        let _ = item;
        false
    }

    /// The bank's file, if it has one (a Playlist record links to it).
    pub(super) fn reg_bank_path(&self) -> Option<PathBuf> {
        self.reg.path.clone()
    }

    pub(super) fn reg_bank_name(&self) -> String {
        self.reg.bank.name.clone()
    }

    pub(super) fn reg_selected(&self) -> Option<u8> {
        self.reg.selected
    }

    /// Page 4 of the Launchkey.
    pub(super) fn regist_panel(&self) -> RegistPanel {
        let r = &self.reg;
        RegistPanel {
            stored: r.bank.stored_mask(),
            selected: r.selected.map_or(0, |s| s + 1),
            memory: r.memory,
            freeze: r.freeze,
            sequence: r.bank.sequence.on && !r.bank.sequence.steps.is_empty(),
            banks: !r.banks.is_empty(),
        }
    }

    pub(super) fn registration_state(&self) -> RegistrationState {
        let r = &self.reg;
        RegistrationState {
            bank: BankState {
                name: r.bank.name.clone(),
                path: r.path.as_ref().map(|p| p.display().to_string()),
                dirty: r.dirty,
                position: r.position(),
            },
            banks: r.banks.iter().map(|p| BankFile { name: reg::bank_name(p), path: p.display().to_string() }).collect(),
            folder: r.dir.as_ref().map(|d| d.display().to_string()),
            buttons: r.buttons.clone(),
            selected: r.selected,
            memory: r.memory,
            memorize_groups: r.memorize,
            freeze: r.freeze,
            freeze_groups: r.frozen,
            sequence: SequenceState {
                on: r.bank.sequence.on,
                steps: r.bank.sequence.steps.clone(),
                end: r.bank.sequence.end,
                position: r.seq_pos,
            },
            pending: r.deferred.is_some(),
        }
    }
}

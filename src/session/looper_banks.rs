//! Chord Looper bank files (#201): the eight memories saved as yahaha's own JSON
//! (`looper::BankFile`, `<name>.looper.json` in the data folder's `ChordLooper` folder),
//! and kept across sessions: every change to the memories is written at once, to the
//! bank's own file, or to `autosave.json` while the bank has none. `setup.json` remembers
//! which bank was in use, so the next session starts with it (docs/chord-looper.md).

use super::Control;
use crate::api::{BankFile as BankEntry, CmdError};
use crate::engine::LoopState;
use crate::looper::{BankFile, ChordSeq, BANK_EXT};
use crate::registration as reg;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The memories of a bank with no file of its own.
const AUTOSAVE_FILE: &str = "autosave.json";
/// Which bank file was in use (`{"bank": path | null}`).
const SETUP_FILE: &str = "setup.json";
const NEW_BANK: &str = "New Bank";

#[derive(Default, Serialize, Deserialize)]
struct Setup {
    #[serde(default)]
    bank: Option<PathBuf>,
}

/// The Chord Looper's bank: its name, its file, and the folder's other files.
pub(super) struct BankFiles {
    dir: Option<PathBuf>,
    name: String,
    path: Option<PathBuf>,
    banks: Vec<PathBuf>,
}

impl BankFiles {
    pub(super) fn new(dir: Option<PathBuf>) -> BankFiles {
        let mut f = BankFiles { dir, name: NEW_BANK.into(), path: None, banks: Vec::new() };
        f.relist();
        f
    }

    /// The bank the last session left: its file (if it is still there), else the unsaved
    /// bank's autosave. Its name and file become this one's.
    pub(super) fn startup(&mut self) -> Option<BankFile> {
        let dir = self.dir.clone()?;
        let setup: Setup = std::fs::read_to_string(dir.join(SETUP_FILE)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        if let Some(path) = setup.bank {
            if let Some(b) = read(&path) {
                self.name = reg::file_stem(&path, BANK_EXT);
                self.path = Some(path);
                return Some(b);
            }
        }
        let b = read(&dir.join(AUTOSAVE_FILE))?;
        self.name = if b.name.trim().is_empty() { NEW_BANK.into() } else { b.name.clone() };
        Some(b)
    }

    pub(super) fn name(&self) -> &str {
        &self.name
    }

    pub(super) fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub(super) fn list(&self) -> Vec<BankEntry> {
        self.banks.iter().map(|p| BankEntry { name: reg::file_stem(p, BANK_EXT), path: p.to_string_lossy().to_string() }).collect()
    }

    fn relist(&mut self) {
        self.banks = self.dir.as_deref().map(|d| reg::list_files(d, BANK_EXT)).unwrap_or_default();
    }

    /// A new bank: no file, "New Bank".
    pub(super) fn detach(&mut self) {
        self.name = NEW_BANK.into();
        self.set_path(None);
    }

    fn set_path(&mut self, path: Option<PathBuf>) {
        self.path = path;
        if let Some(dir) = &self.dir {
            if let Ok(text) = serde_json::to_string_pretty(&Setup { bank: self.path.clone() }) {
                let _ = reg::write_atomic(&dir.join(SETUP_FILE), &text);
            }
        }
        self.relist();
    }

    /// Write `memories` where this bank keeps them (nothing without a data folder).
    fn write(&self, memories: &[Option<(String, ChordSeq)>]) -> anyhow::Result<()> {
        let Some(dir) = &self.dir else { return Ok(()) };
        let target = self.path.clone().unwrap_or_else(|| dir.join(AUTOSAVE_FILE));
        reg::write_atomic(&target, &BankFile::new(&self.name, memories).to_json())
    }
}

fn read(path: &Path) -> Option<BankFile> {
    BankFile::from_json(&std::fs::read_to_string(path).ok()?).ok()
}

impl Control {
    /// The memories changed: write them (a failure is said, and the memories stay).
    pub(super) fn looper_autosave(&mut self) {
        if let Err(e) = self.looper.files.write(self.looper.memories()) {
            self.say(format!("Chord Looper: saving the memories: {e:#}"), true);
        }
    }

    /// `SaveLooperBank`: to the bank's file, or as `name` (Save As).
    pub(super) fn save_looper_bank(&mut self, name: Option<String>, overwrite: bool) -> Result<(), CmdError> {
        let Some(dir) = self.looper.files.dir.clone() else {
            return self.fail("Chord Looper: no data folder to save banks in");
        };
        if let Some(name) = name {
            let name = name.trim().to_string();
            if name.is_empty() {
                return self.fail("Chord Looper: give the bank a name");
            }
            let own = self.looper.files.path.clone();
            let path = match reg::save_target(&dir, &reg::file_name(&name, BANK_EXT), own.as_deref(), overwrite) {
                Ok(p) => p,
                Err(reg::SaveClash::Exists) => return self.fail(format!("a Chord Looper bank called {name} already exists: save under another name, or overwrite it")),
                Err(reg::SaveClash::Rename(e)) => return self.fail(format!("Chord Looper: renaming the bank file: {e}")),
            };
            self.looper.files.name = reg::file_stem(&path, BANK_EXT);
            self.looper.files.set_path(Some(path));
        } else if self.looper.files.path.is_none() {
            return self.fail("Chord Looper: give the bank a name to save it");
        }
        match self.looper.files.write(self.looper.memories()) {
            Ok(()) => {
                self.looper.files.relist();
                self.say(format!("Saved Chord Looper bank {}", self.looper.files.name), false);
                Ok(())
            }
            Err(e) => self.fail(format!("Chord Looper: saving the bank: {e:#}")),
        }
    }

    /// `LoadLooperBank`: its memories replace the eight (refused while recording).
    pub(super) fn load_looper_bank(&mut self, path: &str) -> Result<(), CmdError> {
        if matches!(self.snap.looper.state, LoopState::Recording | LoopState::RecArmed) {
            return self.fail("Chord Looper: stop recording before loading a bank");
        }
        let path = PathBuf::from(path);
        let b = match std::fs::read_to_string(&path).map_err(anyhow::Error::from).and_then(|t| BankFile::from_json(&t)) {
            Ok(b) => b,
            Err(e) => return self.fail(format!("Chord Looper: reading {}: {e:#}", path.display())),
        };
        self.looper.set_memories(&b);
        self.looper.files.name = reg::file_stem(&path, BANK_EXT);
        self.looper.files.set_path(Some(path));
        self.say(format!("Chord Looper bank: {}", self.looper.files.name), false);
        Ok(())
    }
}

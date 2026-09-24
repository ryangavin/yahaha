//! Multi Pads (docs/multipad.md): the bank list, loading a bank (parsed and built into a
//! player here, then handed to the engine thread through `live::PadBank`), the pad
//! commands, and the state.

use super::Control;
use crate::api::{
    CmdError, MultiPadBank, MultiPadBankEntry, MultiPadCmd, MultiPadPad, MultiPadState, MultiPadSynchroStop, PadLamp,
};
use crate::engine::{PadCmd, SynchroStop, PAD_PPQ};
use crate::live::{Cmd, PadBank};
use crate::multipad::library::{self, BankFile};
use crate::multipad::{MultiPadPlayer, PadBank as PadFile, PadState, PADS};
use std::path::{Path, PathBuf};
use std::sync::mpsc;

/// The control side's Multi Pad state (a `Control` field).
#[derive(Default)]
pub(super) struct Pads {
    /// The bank files, with their ids (stable for the session).
    banks: Vec<(usize, BankFile)>,
    next_id: usize,
    /// The bank the engine plays (by `PadBank::tag`), and one on its way there.
    loaded: Option<Loaded>,
    pending: Option<Loaded>,
    seq: u64,
    /// A rescan of the bank files running on a thread of its own (started with the style
    /// library's `RescanLibrary`).
    scan_rx: Option<mpsc::Receiver<Vec<BankFile>>>,
}

#[derive(Clone)]
struct Loaded {
    tag: u64,
    /// None for "no bank" (`ClearMultiPad`).
    bank: Option<(usize, String, PathBuf, [String; PADS])>,
}

impl Pads {
    /// The bank files under `roots`, as the start of a session finds them.
    pub(super) fn scan(roots: &[PathBuf]) -> Pads {
        let mut p = Pads::default();
        p.merge(library::scan(roots));
        p
    }

    /// A new scan: files still there keep their ids, new ones get new ids.
    fn merge(&mut self, found: Vec<BankFile>) {
        let mut out = Vec::with_capacity(found.len());
        for f in found {
            let id = match self.banks.iter().find(|(_, b)| b.path == f.path) {
                Some((id, _)) => *id,
                None => {
                    self.next_id += 1;
                    self.next_id - 1
                }
            };
            out.push((id, f));
        }
        self.banks = out;
    }

    fn find(&self, path: &Path) -> Option<usize> {
        self.banks.iter().find(|(_, b)| b.path == path).map(|(id, _)| *id)
    }

    fn entry(&self, id: usize) -> Option<&BankFile> {
        self.banks.iter().find(|(i, _)| *i == id).map(|(_, b)| b)
    }
}

fn pad_index(pad: u8) -> Result<u8, String> {
    if (pad as usize) < PADS {
        Ok(pad)
    } else {
        Err(format!("no Multi Pad {pad} (pads are 0-3)"))
    }
}

impl Control {
    pub(super) fn multipad_cmd(&mut self, c: MultiPadCmd) -> Result<(), CmdError> {
        let cmd = match c {
            MultiPadCmd::LoadMultiPad { id } => return self.load_bank(id),
            MultiPadCmd::LoadMultiPadPath { path } => {
                let path = PathBuf::from(path);
                if let Some(id) = self.multipad.find(&path) {
                    return self.load_bank(id);
                }
                // A file outside the library joins the bank list only once it has loaded.
                let root = path.parent().map(Path::to_path_buf).unwrap_or_default();
                let f = library::bank_file(&path, &root);
                let bank = match PadFile::load(&f.path) {
                    Ok(b) => b,
                    Err(e) => return self.fail(format!("{}: {e:#}", f.path.display())),
                };
                let id = self.multipad.next_id;
                self.multipad.next_id += 1;
                self.multipad.banks.push((id, f.clone()));
                return self.send_parsed(id, f, &bank);
            }
            MultiPadCmd::ClearMultiPad => return self.send_bank(None, None),
            MultiPadCmd::TriggerMultiPad { pad } => pad_index(pad).map(PadCmd::Trigger),
            MultiPadCmd::StopMultiPad { pad } => pad_index(pad).map(PadCmd::Stop),
            MultiPadCmd::StopAllMultiPads => Ok(PadCmd::StopAll),
            MultiPadCmd::ArmMultiPad { pad } => pad_index(pad).map(PadCmd::Arm),
            MultiPadCmd::SetMultiPadRepeat { pad, on } => pad_index(pad).map(|p| PadCmd::Repeat(p, on)),
            MultiPadCmd::SetMultiPadChordMatch { pad, on } => pad_index(pad).map(|p| PadCmd::ChordMatch(p, on)),
            MultiPadCmd::SetMultiPadSynchroStop { style_stop, ending } => {
                Ok(PadCmd::SynchroStop(SynchroStop { style_stop, ending }))
            }
        };
        match cmd {
            Ok(c) => self.engine_cmd(Cmd::MultiPad(c)),
            Err(t) => self.fail(t),
        }
    }

    /// Parse bank `id` and hand its player to the engine.
    fn load_bank(&mut self, id: usize) -> Result<(), CmdError> {
        let Some(f) = self.multipad.entry(id).cloned() else {
            return self.fail(format!("no Multi Pad bank {id}"));
        };
        let bank = match PadFile::load(&f.path) {
            Ok(b) => b,
            Err(e) => return self.fail(format!("{}: {e:#}", f.path.display())),
        };
        self.send_parsed(id, f, &bank)
    }

    /// Build bank `id`'s player from its parsed file and hand it to the engine.
    fn send_parsed(&mut self, id: usize, f: BankFile, bank: &PadFile) -> Result<(), CmdError> {
        let names = std::array::from_fn(|i| bank.pads[i].as_ref().map(|p| p.name.clone()).unwrap_or_default());
        let player = Box::new(MultiPadPlayer::new(bank, PAD_PPQ));
        self.send_bank(Some(player), Some((id, f.name, f.path, names)))
    }

    fn send_bank(
        &mut self,
        player: Option<Box<MultiPadPlayer>>,
        bank: Option<(usize, String, PathBuf, [String; PADS])>,
    ) -> Result<(), CmdError> {
        self.multipad.seq += 1;
        let tag = self.multipad.seq;
        if self.pad_tx.push(PadBank { player, tag }).is_err() {
            return Err(CmdError::Busy);
        }
        self.multipad.pending = Some(Loaded { tag, bank });
        self.wake_engine();
        self.message = None;
        Ok(())
    }

    /// Rescan the bank files off the control thread (the style library's rescan calls
    /// this when it starts its own).
    pub(super) fn rescan_pads(&mut self) {
        if self.multipad.scan_rx.is_some() {
            return;
        }
        let roots = self.roots.clone();
        let (tx, rx) = mpsc::channel();
        let scan = move || drop(tx.send(library::scan(&roots)));
        if std::thread::Builder::new().name("yahaha-pad-scan".into()).spawn(scan).is_ok() {
            self.multipad.scan_rx = Some(rx);
        }
    }

    /// Replaced players back from the engine are freed here; the bank the engine now plays
    /// becomes the loaded one; a finished bank rescan is merged in.
    pub(super) fn pump_multipad(&mut self) {
        while self.old_pad_rx.pop().is_ok() {}
        if let Some(p) = &self.multipad.pending
            && p.tag == self.snap.multipad.tag
        {
            self.multipad.loaded = self.multipad.pending.take();
        }
        if let Some(rx) = &self.multipad.scan_rx {
            match rx.try_recv() {
                Ok(found) => {
                    self.multipad.scan_rx = None;
                    self.multipad.merge(found);
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => self.multipad.scan_rx = None,
            }
        }
    }

    pub(super) fn multipad_state(&self) -> MultiPadState {
        let s = &self.snap.multipad;
        let loaded = self.multipad.loaded.as_ref().filter(|l| l.tag == s.tag).and_then(|l| l.bank.as_ref());
        let pads = (0..PADS)
            .map(|i| MultiPadPad {
                index: i as u8,
                name: loaded.map(|b| b.3[i].clone()).unwrap_or_default(),
                lamp: match s.states[i] {
                    PadState::Empty => PadLamp::Empty,
                    PadState::Ready => PadLamp::Ready,
                    PadState::Armed => PadLamp::Armed,
                    PadState::Queued => PadLamp::Queued,
                    PadState::Playing => PadLamp::Playing,
                },
                repeat: s.repeat[i],
                chord_match: s.chord_match[i],
                channel: crate::multipad::player::DEFAULT_OUT_CH[i] + 1,
            })
            .collect();
        MultiPadState {
            bank: loaded.map(|(id, name, path, _)| MultiPadBank { id: *id, name: name.clone(), path: path.display().to_string() }),
            loading: self.multipad.pending.is_some(),
            pads,
            synchro_stop: MultiPadSynchroStop { style_stop: s.synchro.style_stop, ending: s.synchro.ending },
            banks: self
                .multipad
                .banks
                .iter()
                .map(|(id, b)| MultiPadBankEntry {
                    id: *id,
                    name: b.name.clone(),
                    folder: b.folder.clone(),
                    path: b.path.display().to_string(),
                })
                .collect(),
        }
    }
}

#[cfg(test)]
#[path = "multipad_tests.rs"]
mod tests;

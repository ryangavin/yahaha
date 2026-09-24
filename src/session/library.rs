//! Styles: the loaded style, style changes, the library and its index and rescans.

use super::Control;
use crate::api::{CmdError, LibraryCmd, LibraryEntry, LibraryStatus, StyleState};
use crate::engine::{id_of, Prepared, NUM_SLOTS};
use crate::library::{self, Info, Library};
use crate::sff::{Ots, Style};
use crate::synth;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering::Relaxed;
use std::sync::mpsc;

/// The loaded style, as the panel shows it.
pub(super) struct Loaded {
    pub(super) path: PathBuf,
    pub(super) name: String,
    pub(super) format: String,
    pub(super) bpm: f64,
    pub(super) timesig: (u8, u8),
    /// Quarter notes per bar.
    pub(super) quarters_per_bar: f64,
    pub(super) has: [bool; NUM_SLOTS],
    pub(super) voices: [Option<(u8, u8, u8)>; 16],
    pub(super) ots: Vec<Ots>,
}

pub(super) fn load(path: &Path) -> Result<(Box<Prepared>, Loaded)> {
    let style = Style::load(path)?;
    let prep = Box::new(Prepared::new(&style));
    let mut has = [false; NUM_SLOTS];
    for (i, s) in prep.sections.iter().enumerate() {
        has[i] = s.is_some();
    }
    let name = if style.name.is_empty() {
        path.file_stem().unwrap_or_default().to_string_lossy().to_string()
    } else {
        style.name.clone()
    };
    let info = Loaded {
        path: path.to_path_buf(),
        name,
        format: style.format.clone(),
        bpm: prep.bpm,
        timesig: style.timesig,
        quarters_per_bar: prep.tpb as f64 / prep.ppq.max(1) as f64,
        has,
        voices: prep.voices,
        ots: style.ots.clone(),
    };
    Ok((prep, info))
}

/// A scanned library, its index results to come, and the first style that loads (its id).
pub(super) type Opened = (Library, mpsc::Receiver<(usize, Info)>, usize, Box<Prepared>, Loaded);

/// Scan the library, start indexing it, and load the first style that loads.
pub(super) fn open_library(paths: &[PathBuf]) -> Result<Opened> {
    // The folder walk is quick; the index (names, tempos) fills in on a background thread.
    let lib = Library::scan(paths);
    anyhow::ensure!(!lib.is_empty(), "no style files found");
    let index_rx = lib.spawn_indexer();
    for &id in lib.order() {
        match load(&lib.entry(id).path) {
            Ok((prep, info)) => return Ok((lib, index_rx, id, prep, info)),
            Err(e) if lib.len() == 1 => {
                return Err(e).with_context(|| format!("loading {}", lib.entry(id).path.display()));
            }
            Err(_) => {}
        }
    }
    anyhow::bail!("no style file loads")
}

impl Control {
    pub(super) fn library_cmd(&mut self, c: LibraryCmd) -> Result<(), CmdError> {
        match c {
            // With one style there is nowhere to go, and the loaded one isn't reloaded.
            LibraryCmd::LoadStyle { id } | LibraryCmd::QueueStyle { id } => return self.choose_style(id),
            LibraryCmd::RescanLibrary => self.rescan(),
            LibraryCmd::LoadStylePath { path } => {
                let path = PathBuf::from(path);
                let id = match self.lib.find(&path) {
                    Some(id) => id,
                    None => {
                        let id = self.lib.add_file(path);
                        self.lib_rev += 1;
                        self.lib_urgent = true;
                        id
                    }
                };
                return self.choose_style(id);
            }
            // Folder-then-name order, the browser's unfiltered list.
            LibraryCmd::StepStyle { delta } => {
                let next = self.lib.step(self.target_style(), delta);
                return self.choose_style(next);
            }
        }
        Ok(())
    }

    /// Load a style and hand it to the engine, playing or stopped: the one path every
    /// style change takes. A file that fails to load is marked as an error row, so
    /// stepping skips it next time.
    ///
    /// Stopped, the engine switches at once; playing, at the next bar line
    /// (`Engine::change_style`). Either way the state shows the new style once the engine
    /// plays it (`promote_style`); until then `preview.queued` names it.
    fn switch_style(&mut self, id: usize) -> Result<(), CmdError> {
        match self.load_entry(id) {
            Ok((mut p, info)) => {
                self.style_seq += 1;
                p.tag = self.style_seq;
                if self.style_tx.push(p).is_err() {
                    return Err(CmdError::Busy);
                }
                self.pending_style = Some((id, self.style_seq, info));
                self.wake_engine();
                self.message = None;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Load library entry `id`. A file that fails to load is marked as an error row, so
    /// stepping skips it next time.
    pub(super) fn load_entry(&mut self, id: usize) -> Result<(Box<Prepared>, Loaded), CmdError> {
        if id >= self.lib.len() {
            return Err(self.fail(format!("no style {id} in the library")).unwrap_err());
        }
        let path = self.lib.entry(id).path.clone();
        load(&path).map_err(|e| {
            self.lib.set_info(id, Info::Err(format!("{e:#}")));
            self.lib.sort();
            self.lib_rev += 1;
            self.lib_urgent = true;
            self.fail(format!("{}: {e:#}", path.display())).unwrap_err()
        })
    }

    /// The style the player last chose: the one waiting for the bar line, else the one
    /// playing.
    fn target_style(&self) -> usize {
        self.pending_style.as_ref().map_or(self.cur, |p| p.0)
    }

    /// Load `id` unless it is already the style chosen.
    fn choose_style(&mut self, id: usize) -> Result<(), CmdError> {
        if id != self.target_style() {
            return self.switch_style(id);
        }
        Ok(())
    }

    /// The engine plays the style handed to it: it becomes the loaded style.
    pub(super) fn promote_style(&mut self) {
        let Some((_, tag, _)) = &self.pending_style else { return };
        if self.snap.style_tag != *tag {
            return;
        }
        let (id, _, info) = self.pending_style.take().unwrap();
        self.shared.parts.set_bass_program(synth::style_bass_program(info.voices[10]));
        // No OTS of the new style is recalled yet (OTS Link recalls one on the next pass
        // if it's on).
        self.shared.parts.ots_applied.store(0, Relaxed);
        self.cur = id;
        self.info = info;
    }

    /// Start a rescan of the style folders on a thread of its own.
    fn rescan(&mut self) {
        if self.scan_rx.is_some() {
            return;
        }
        let roots = self.roots.clone();
        let (tx, rx) = mpsc::channel();
        if std::thread::Builder::new().name("yahaha-scan".into()).spawn(move || drop(tx.send(Library::scan(&roots)))).is_ok() {
            self.scan_rx = Some(rx);
        }
    }

    /// A finished rescan: merge it, and index what's new.
    pub(super) fn pump_rescan(&mut self) {
        let Some(rx) = &self.scan_rx else { return };
        match rx.try_recv() {
            Ok(scanned) => {
                self.scan_rx = None;
                let added = self.lib.merge(scanned, &self.roots);
                // Every entry still pending (new, or not reached by the first index).
                self.index_rx = Some(self.lib.spawn_pending_indexer());
                self.lib_rev += 1;
                self.lib_urgent = true;
                let n = self.lib.count();
                self.say(format!("Style folders rescanned: {n} styles, {} new", added.len()), false);
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => self.scan_rx = None,
        }
    }

    /// Index results that came in.
    pub(super) fn pump_index(&mut self) {
        if let Some(rx) = &self.index_rx
            && self.lib.apply(rx) > 0
        {
            self.lib_rev += 1;
        }
    }

    pub(super) fn style_state(&self) -> StyleState {
        let info = &self.info;
        StyleState {
            id: self.cur,
            path: info.path.display().to_string(),
            name: info.name.clone(),
            format: info.format.clone(),
            tempo: info.bpm,
            time_signature: [info.timesig.0, info.timesig.1],
            sections: (0..NUM_SLOTS).filter(|&i| info.has[i]).map(|i| id_of(i).name()).collect(),
        }
    }

    pub(super) fn library_status(&self) -> LibraryStatus {
        LibraryStatus {
            // The revision `library()` has (published at most every 250 ms while indexing).
            revision: self.lib_published,
            count: self.published.count(),
            position: self.published.position(self.cur),
            pending: self.published.pending(),
            roots: self.roots.iter().map(|p| p.display().to_string()).collect(),
            scanning: self.scan_rx.is_some(),
        }
    }
}

/// A library entry as plain data.
pub fn library_entry(lib: &Library, id: usize) -> LibraryEntry {
    let e = lib.entry(id);
    let (status, error, tempo, ts, sections) = match &e.info {
        Info::Pending => ("pending", None, None, None, String::new()),
        Info::Ok(s) => ("ok", None, Some(s.bpm), Some([s.timesig.0, s.timesig.1]), library::sections_text(&s.sections)),
        Info::Err(err) => ("error", Some(err.clone()), None, None, String::new()),
    };
    let format = match &e.info {
        Info::Ok(s) if !s.format.is_empty() => Some(s.format.clone()),
        _ => None,
    };
    LibraryEntry {
        id,
        name: e.name().to_string(),
        folder: e.folder.clone(),
        path: e.path.display().to_string(),
        status: status.into(),
        error,
        tempo,
        time_signature: ts,
        sections,
        format,
    }
}

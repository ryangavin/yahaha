//! The Playlist: set lists whose records load a Registration bank (and one of its
//! buttons) or a style (OM p.100-103; docs/registration.md).

use super::Control;
use crate::api::{CmdError, LibraryCmd, PlaylistCmd, PlaylistFileEntry, PlaylistRow, PlaylistState};
use crate::registration::playlist::{self as pl, MAX_RECORDS, PLAYLIST_EXT};
use crate::registration::{self as reg, Playlist, PlaylistSort, Record, RecordTarget};
use std::path::{Path, PathBuf};

/// The control side's Playlist.
pub(super) struct PlaylistCtl {
    /// Where playlist files live (None: no saving).
    dir: Option<PathBuf>,
    list: Playlist,
    path: Option<PathBuf>,
    dirty: bool,
    sort: PlaylistSort,
    current: Option<usize>,
    files: Vec<PathBuf>,
    /// Per record: its file is not there (checked when the list changes, not every frame).
    missing: Vec<bool>,
}

impl PlaylistCtl {
    pub(super) fn new(dir: Option<PathBuf>) -> PlaylistCtl {
        let mut p = PlaylistCtl {
            dir,
            list: Playlist::default(),
            path: None,
            dirty: false,
            sort: PlaylistSort::Normal,
            current: None,
            files: Vec::new(),
            missing: Vec::new(),
        };
        p.list_files();
        p
    }

    fn list_files(&mut self) {
        self.files = self.dir.as_deref().map(pl::list_playlists).unwrap_or_default();
    }

    /// The folder relative record paths are read from: the playlist's own, else the folder.
    fn base(&self) -> Option<PathBuf> {
        self.path.as_ref().and_then(|p| p.parent().map(Path::to_path_buf)).or_else(|| self.dir.clone())
    }

    fn target_path(&self, t: &RecordTarget) -> PathBuf {
        pl::resolve(t.path(), self.base().as_deref())
    }

    fn changed(&mut self) {
        self.dirty = true;
        self.check();
    }

    fn check(&mut self) {
        self.missing = self.list.records.iter().map(|r| !self.target_path(&r.target).is_file()).collect();
    }
}

impl Control {
    pub(super) fn playlist_cmd(&mut self, c: PlaylistCmd) -> Result<(), CmdError> {
        match c {
            PlaylistCmd::NewPlaylist => {
                self.playlist.list = Playlist::default();
                self.playlist.path = None;
                self.playlist.dirty = false;
                self.playlist.sort = PlaylistSort::Normal;
                self.playlist.current = None;
                self.playlist.check();
            }
            PlaylistCmd::LoadPlaylist { path } => return self.load_playlist(Path::new(&path)),
            PlaylistCmd::SavePlaylist { name, overwrite } => return self.save_playlist(name, overwrite),
            PlaylistCmd::AddPlaylistRecord { record } => return self.add_record(record),
            PlaylistCmd::AddCurrentBank => {
                let Some(path) = self.reg_bank_path() else {
                    return self.fail("save the bank first: a Playlist record links to a bank file");
                };
                let name = self.reg_bank_name();
                let regist = self.reg_selected();
                let name = match regist {
                    Some(i) => format!("{name} [{}]", i + 1),
                    None => name,
                };
                return self.add_record(Record { name, target: RecordTarget::Bank { path: path.display().to_string(), regist } });
            }
            PlaylistCmd::AddCurrentStyle => {
                let (name, path) = (self.info.name.clone(), self.info.path.display().to_string());
                return self.add_record(Record { name, target: RecordTarget::Style { path } });
            }
            PlaylistCmd::AppendPlaylist { path } => {
                let other = match Playlist::load(Path::new(&path)) {
                    Ok(p) => p,
                    Err(e) => return self.fail(format!("{e:#}")),
                };
                // Its relative paths are relative to its own folder: store them absolute.
                let base = Path::new(&path).parent().map(Path::to_path_buf);
                for mut r in other.records {
                    let abs = pl::resolve(r.target.path(), base.as_deref()).display().to_string();
                    match &mut r.target {
                        RecordTarget::Bank { path, .. } | RecordTarget::Style { path } => *path = abs,
                    }
                    if !self.playlist.list.push(r) {
                        self.playlist.changed();
                        return self.fail(format!("a playlist holds at most {MAX_RECORDS} records"));
                    }
                }
                self.playlist.changed();
            }
            PlaylistCmd::SetPlaylistRecord { index, record } => {
                let Some(r) = self.playlist.list.records.get_mut(index) else { return self.no_record(index) };
                *r = clean(record);
                self.playlist.changed();
            }
            PlaylistCmd::MovePlaylistRecord { index, delta } => {
                self.unsorted("move records")?;
                if self.playlist.list.move_record(index, delta as i32) {
                    if let Some(c) = self.playlist.current {
                        let j = (index as i64 + delta as i64) as usize;
                        self.playlist.current = Some(if c == index {
                            j
                        } else if c == j {
                            index
                        } else {
                            c
                        });
                    }
                    self.playlist.changed();
                }
            }
            PlaylistCmd::DeletePlaylistRecord { index } => {
                self.unsorted("delete records")?;
                if index >= self.playlist.list.records.len() {
                    return self.no_record(index);
                }
                self.playlist.list.records.remove(index);
                self.playlist.current = match self.playlist.current {
                    Some(c) if c == index => None,
                    Some(c) if c > index => Some(c - 1),
                    c => c,
                };
                self.playlist.changed();
            }
            PlaylistCmd::SetPlaylistSort { sort } => self.playlist.sort = sort,
            PlaylistCmd::LoadPlaylistRecord { index } => return self.load_record(index),
            PlaylistCmd::StepPlaylist { delta } => {
                let order = self.playlist.list.order(self.playlist.sort);
                // An empty playlist: nothing to step to (Shift + Track is dark then).
                if order.is_empty() {
                    return Ok(());
                }
                let pos = self.playlist.current.and_then(|c| order.iter().position(|&i| i == c));
                let next = match pos {
                    None if delta >= 0 => 0,
                    None => order.len() - 1,
                    Some(p) => (p as i64 + delta.signum() as i64).clamp(0, order.len() as i64 - 1) as usize,
                };
                if Some(next) == pos {
                    return Ok(());
                }
                return self.load_record(order[next]);
            }
        }
        Ok(())
    }

    fn no_record(&mut self, index: usize) -> Result<(), CmdError> {
        self.fail(format!("no playlist record {}", index + 1))
    }

    /// Up/Down and Delete are off while the list is sorted (OM p.102).
    fn unsorted(&mut self, what: &str) -> Result<(), CmdError> {
        if self.playlist.sort != PlaylistSort::Normal {
            return self.fail(format!("sort the playlist back to Normal to {what}"));
        }
        Ok(())
    }

    fn add_record(&mut self, r: Record) -> Result<(), CmdError> {
        if !self.playlist.list.push(clean(r)) {
            return self.fail(format!("a playlist holds at most {MAX_RECORDS} records"));
        }
        self.playlist.changed();
        Ok(())
    }

    fn load_playlist(&mut self, path: &Path) -> Result<(), CmdError> {
        match Playlist::load(path) {
            Ok(mut p) => {
                p.name = pl::playlist_name(path);
                let c = &mut self.playlist;
                c.list = p;
                c.path = Some(path.to_path_buf());
                c.dirty = false;
                c.sort = PlaylistSort::Normal;
                c.current = None;
                c.list_files();
                c.check();
                Ok(())
            }
            Err(e) => self.fail(format!("{e:#}")),
        }
    }

    /// Save: the displayed order, then back to Normal (OM p.102). A file of that name that
    /// belongs to another playlist is only replaced with `overwrite`.
    fn save_playlist(&mut self, name: Option<String>, overwrite: bool) -> Result<(), CmdError> {
        let path = match (name, &self.playlist.path) {
            (None, Some(p)) => p.clone(),
            (name, _) => {
                let Some(dir) = self.playlist.dir.clone() else {
                    return self.fail("no Playlist folder to save to");
                };
                let name = name.unwrap_or_else(|| self.playlist.list.name.clone()).trim().to_string();
                let path = match reg::save_target(&dir, &reg::file_name(&name, PLAYLIST_EXT), self.playlist.path.as_deref(), overwrite) {
                    Ok(p) => p,
                    Err(reg::SaveClash::Exists) => return self.fail(format!("a playlist called {name} already exists: save under another name, or overwrite it")),
                    Err(reg::SaveClash::Rename(e)) => return self.fail(format!("renaming the playlist to {name}: {e}")),
                };
                self.playlist.list.name = name;
                path
            }
        };
        let c = &mut self.playlist;
        if c.sort != PlaylistSort::Normal {
            let order = c.list.order(c.sort);
            c.current = c.current.and_then(|cur| order.iter().position(|&i| i == cur));
            c.list.apply_order(c.sort);
            c.sort = PlaylistSort::Normal;
        }
        if let Err(e) = c.list.save(&path) {
            return self.fail(format!("saving the playlist: {e:#}"));
        }
        let c = &mut self.playlist;
        c.path = Some(path);
        c.dirty = false;
        c.list_files();
        c.check();
        let name = c.list.name.clone();
        self.say(format!("Saved playlist {name}"), false);
        Ok(())
    }

    /// Load a record: its bank and then its button, or its style.
    fn load_record(&mut self, index: usize) -> Result<(), CmdError> {
        let Some(r) = self.playlist.list.records.get(index).cloned() else { return self.no_record(index) };
        let path = self.playlist.target_path(&r.target);
        self.playlist.current = Some(index);
        match r.target {
            RecordTarget::Bank { regist, .. } => {
                self.load_regist_bank(&path)?;
                if let Some(i) = regist {
                    // What the button could not recall stays in the message.
                    let (_, errors) = self.recall_quiet(i, true)?;
                    self.say_recalled(format!("Playlist {}: {}", index + 1, r.name), &errors);
                    return Ok(());
                }
            }
            RecordTarget::Style { .. } => {
                self.library_cmd(LibraryCmd::LoadStylePath { path: path.display().to_string() })?;
            }
        }
        self.say(format!("Playlist {}: {}", index + 1, r.name), false);
        Ok(())
    }

    pub(super) fn playlist_is_empty(&self) -> bool {
        self.playlist.list.records.is_empty()
    }

    pub(super) fn playlist_state(&self) -> PlaylistState {
        let c = &self.playlist;
        PlaylistState {
            name: c.list.name.clone(),
            path: c.path.as_ref().map(|p| p.display().to_string()),
            dirty: c.dirty,
            sort: c.sort,
            records: c
                .list
                .order(c.sort)
                .into_iter()
                .map(|i| PlaylistRow { index: i, record: c.list.records[i].clone(), missing: c.missing.get(i).copied().unwrap_or(false) })
                .collect(),
            current: c.current,
            playlists: c.files.iter().map(|p| PlaylistFileEntry { name: pl::playlist_name(p), path: p.display().to_string() }).collect(),
            folder: c.dir.as_ref().map(|d| d.display().to_string()),
        }
    }
}

/// A record as stored: a name (its file's, if none), a button 0-9.
fn clean(mut r: Record) -> Record {
    if let RecordTarget::Bank { regist, .. } = &mut r.target {
        *regist = regist.filter(|&i| (i as usize) < reg::BUTTONS);
    }
    if r.name.trim().is_empty() {
        let p = Path::new(r.target.path());
        r.name = match &r.target {
            RecordTarget::Bank { .. } => reg::bank_name(p),
            RecordTarget::Style { .. } => p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default(),
        };
    }
    r.name = r.name.trim().to_string();
    r
}

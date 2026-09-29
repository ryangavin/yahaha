//! The live rack (docs/racks.md): the rack under the player's hands now, unsaved changes
//! and plugin states included. It autosaves to `live-rack.json` in
//! `~/Library/Application Support/yahaha` and comes back when the session starts, so after
//! a restart the keyboard parts sound and mix as they did, saved or not.
//!
//! - **Changes.** Every pump compares the rack playing now (captured without plugin states,
//!   so nothing big is read or encoded) with the last one it saw. Any difference (a sound,
//!   the mix, the split, Harmony/Arp, the transpose, the controller map, or a plugin edit,
//!   `soundEdited`) sets `modified` and schedules a save. A change to a part's plugin state
//!   (the plugin host's 30-second reads, a load finishing) schedules a save without
//!   setting `modified`. Loading or saving a rack clears `modified` ([`Control::live_rack_clean`]).
//! - **Autosave.** A save waits until nothing has changed for [`QUIET_NS`], or until
//!   [`MAX_WAIT_NS`] after the first unsaved change, whichever comes first. The rack
//!   (with the plugin states the parts hold) is captured on the control thread, and
//!   serialized and written (atomically: a temporary file, then a rename) on a
//!   `live-rack` thread of its own. Stopping reads the playing plugins' states (with the
//!   plugin host's deadline), waits for that thread, and writes the rack one last time.
//! - **Start.** `live-rack.json` is applied as a rack; its plugins load without the
//!   in-process fallback. With no live rack yet but a `plugin-parts.json` beside it (the
//!   parts' plugins before racks), those plugins and today's parts become the live rack,
//!   named "Restored"; the old file is left in place and never read again once the live
//!   rack is saved. A `live-rack.json` that can't be read is moved aside (`.bak`) and the
//!   session starts on its defaults (any read error but a missing file counts; one that
//!   can't be moved either is left alone and not saved over); one a newer yahaha wrote is left alone, and this
//!   session doesn't save over it.
//!
//! The file is `{ "format": "yahaha.liveRack", "version": 1, "modified", "rack" }`, `rack`
//! a `racks::Rack` whose `id` is the saved rack it came from (empty: none).

use super::{Control, Session};
use crate::api::LiveRackState;
use crate::data_files::write_atomic;
use crate::racks::Rack;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

/// The live rack's file name.
pub const FILE: &str = "live-rack.json";
/// Where the parts' plugins were kept before racks, beside the live rack.
pub const OLD_FILE: &str = "plugin-parts.json";
/// The live rack file's `format`.
const FORMAT: &str = "yahaha.liveRack";
/// The live rack file version this build reads and writes.
const VERSION: u32 = 1;
/// A save waits until nothing has changed for this long...
pub(super) const QUIET_NS: u64 = 1_000_000_000;
/// ...or this long after the first unsaved change.
pub(super) const MAX_WAIT_NS: u64 = 10_000_000_000;

/// The name of a live rack that came from nothing saved.
pub const NEW_NAME: &str = "New rack";
/// The name of the live rack made from `plugin-parts.json` and the parts, on the first
/// start after racks came in.
pub const RESTORED_NAME: &str = "Restored";

/// Where a live session keeps its live rack: `~/Library/Application Support/yahaha/live-rack.json`.
pub fn default_live_rack_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support/yahaha").join(FILE))
}

/// The live rack file.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LiveRackFile {
    format: String,
    version: u32,
    #[serde(default)]
    modified: bool,
    rack: Rack,
}

/// What reading the live rack file found.
enum Read {
    Missing,
    Loaded(LiveRackFile),
    /// A newer yahaha wrote it: left alone, and not saved over.
    Newer(String),
    /// It can't be read: moved to this path.
    Corrupt(String, PathBuf),
    /// It can't be read, nor moved aside: left alone, and not saved over.
    Stuck(String),
}

/// Keep an unreadable live rack for the user (it may be recoverable) rather than let the
/// next save replace it: moved to `.json.bak`.
fn move_aside(path: &Path, why: String) -> Read {
    let bak = path.with_extension("json.bak");
    match std::fs::rename(path, &bak) {
        Ok(()) => Read::Corrupt(why, bak),
        Err(e) => Read::Stuck(format!("{why}; and it could not be moved aside: {e}")),
    }
}

fn read(path: &Path) -> Read {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Read::Missing,
        // Permissions, not UTF-8, an I/O error: not a missing file.
        Err(e) => return move_aside(path, e.to_string()),
    };
    let parsed = serde_json::from_str::<Value>(&text).map_err(anyhow::Error::from).and_then(|v| {
        let format = v.get("format").and_then(Value::as_str).unwrap_or_default();
        anyhow::ensure!(format == FORMAT, "not a live rack (format {format:?})");
        let version = v.get("version").and_then(Value::as_u64).unwrap_or(0);
        if version > VERSION as u64 {
            return Ok(Err(format!("made by a newer yahaha (live rack version {version})")));
        }
        Ok(Ok(serde_json::from_value::<LiveRackFile>(v)?))
    });
    match parsed {
        Ok(Ok(f)) => Read::Loaded(f),
        Ok(Err(newer)) => Read::Newer(newer),
        Err(e) => move_aside(path, format!("{e:#}")),
    }
}

fn write(path: &Path, f: &LiveRackFile) -> Result<()> {
    let json = serde_json::to_string_pretty(f).context("encoding the live rack")?;
    write_atomic(path, &json)
}

/// The `live-rack` thread: writes the latest rack it was given (older ones still queued
/// are skipped), and reports failures back.
struct Writer {
    tx: mpsc::Sender<LiveRackFile>,
    errors: mpsc::Receiver<String>,
    /// Closes when the thread ends.
    done: mpsc::Receiver<()>,
    thread: std::thread::JoinHandle<()>,
}

/// How long stopping waits for the `live-rack` thread (as for the plugins' states).
const STOP_DEADLINE: Duration = Duration::from_secs(2);

impl Writer {
    fn spawn(path: PathBuf) -> std::io::Result<Writer> {
        let (tx, rx) = mpsc::channel::<LiveRackFile>();
        let (etx, errors) = mpsc::channel();
        let (done_tx, done) = mpsc::channel::<()>();
        let thread = std::thread::Builder::new().name("live-rack".into()).spawn(move || {
            let _done = done_tx;
            while let Ok(mut f) = rx.recv() {
                while let Ok(newer) = rx.try_recv() {
                    f = newer;
                }
                if let Err(e) = write(&path, &f) {
                    let _ = etx.send(format!("{e:#}"));
                }
            }
        })?;
        Ok(Writer { tx, errors, done, thread })
    }

    /// Every save handed over is written, then the thread ends. False: it didn't end
    /// within `deadline` (a stuck disk); it is left running, so quitting never hangs.
    fn finish(self, deadline: Duration) -> bool {
        drop(self.tx);
        if let Err(mpsc::RecvTimeoutError::Timeout) = self.done.recv_timeout(deadline) {
            return false;
        }
        let _ = self.thread.join();
        true
    }
}

/// The control side's live rack.
#[derive(Default)]
pub(super) struct LiveRack {
    /// Where it autosaves. None: it doesn't (an offline session without one, or a file a
    /// newer yahaha wrote).
    path: Option<PathBuf>,
    pub(super) name: String,
    /// The saved rack it came from.
    pub(super) id: Option<String>,
    pub(super) modified: bool,
    /// A rack command waiting for the player's answer (session/rack_cmds.rs).
    pub(super) prompt: Option<crate::api::RackPrompt>,
    /// A switch waiting for a save the player chose first, while the save asks for sound
    /// names (session/rack_cmds.rs).
    pub(super) held: Option<crate::api::RackSwitch>,
    /// The rack as the last check saw it, without plugin states. None until the first
    /// pump after the start.
    seen: Option<Rack>,
    /// When the first unsaved change was, and the last.
    dirty_since: Option<u64>,
    changed_ns: u64,
    writer: Option<Writer>,
}

impl Control {
    /// The session's start: apply the live rack at `path` (see the module docs). None: no
    /// live rack file (offline sessions by default); it starts as a new rack.
    /// A split or transpose given on this launch (`opts.split_given`, `transpose_given`)
    /// wins over the live rack's, which then shows modified; flags not given never do.
    pub(super) fn restore_live_rack(&mut self, path: Option<PathBuf>, opts: &super::Options) {
        self.live_rack.name = NEW_NAME.into();
        let Some(path) = path else { return };
        match read(&path) {
            Read::Loaded(f) => {
                self.set_plugins_restoring(true);
                let mut problems = self.apply_rack(&f.rack);
                self.set_plugins_restoring(false);
                let mut overridden = false;
                if opts.split_given && opts.split != f.rack.split.min(127) {
                    match self.chord_cmd(crate::api::ChordCmd::SetSplit { note: opts.split }) {
                        Ok(_) => overridden = true,
                        Err(e) => problems.push(e.to_string()),
                    }
                }
                if opts.transpose_given && opts.transpose.keyboard != f.rack.transpose {
                    match self.set_transpose(crate::engine::Transpose::new(opts.transpose.keyboard, self.transpose.master)) {
                        Ok(_) => overridden = true,
                        Err(e) => problems.push(e.to_string()),
                    }
                }
                for p in problems {
                    self.say(p, true);
                }
                self.live_rack.name = f.rack.name.clone();
                self.live_rack.id = Some(f.rack.id.clone()).filter(|id| !id.is_empty());
                self.live_rack.modified = f.modified || overridden;
            }
            Read::Missing => {
                let old = path.with_file_name(OLD_FILE);
                if old.exists() {
                    self.restore_plugin_parts_from(&old);
                    // It is saved nowhere else: switching away asks first.
                    self.live_rack.name = RESTORED_NAME.into();
                    self.live_rack.modified = true;
                    self.live_rack_touched(self.clock_ns);
                }
            }
            Read::Newer(why) => {
                self.say(format!("the live rack is {why}; starting on a new rack, and not saving over it ({})", path.display()), true);
                return;
            }
            Read::Corrupt(e, bak) => self.say(format!("the live rack could not be read ({e}); moved to {}", bak.display()), true),
            Read::Stuck(e) => {
                self.say(format!("the live rack could not be read ({e}); starting on a new rack, and not saving over it ({})", path.display()), true);
                return;
            }
        }
        self.live_rack.path = Some(path);
    }

    /// The first start after racks came in: the parts' plugins from `plugin-parts.json`.
    fn restore_plugin_parts_from(&mut self, old: &Path) {
        #[cfg(feature = "plugins")]
        self.restore_plugin_parts_file(old);
        #[cfg(not(feature = "plugins"))]
        let _ = old;
    }

    fn set_plugins_restoring(&mut self, on: bool) {
        #[cfg(feature = "plugins")]
        {
            self.plugins.restoring = on;
        }
        #[cfg(not(feature = "plugins"))]
        let _ = on;
    }

    /// Something in the live rack changed (a part's plugin state, say): save it soon.
    pub(super) fn live_rack_touched(&mut self, now: u64) {
        let l = &mut self.live_rack;
        l.dirty_since.get_or_insert(now);
        l.changed_ns = now;
    }

    /// Loading or saving a rack: the live rack is that rack, unmodified (and saved soon).
    pub(super) fn live_rack_clean(&mut self, name: &str, id: Option<String>) {
        self.live_rack.name = name.to_string();
        self.live_rack.id = id;
        self.live_rack.modified = false;
        self.live_rack.prompt = None;
        self.live_rack.seen = Some(self.capture_rack_with(false));
        self.live_rack_touched(self.clock_ns);
    }

    /// Each pump: a change sets `modified` (and schedules a save); a save that is due
    /// goes to the `live-rack` thread.
    pub(super) fn pump_live_rack(&mut self, now: u64) {
        let rack = self.capture_rack_with(false);
        match &self.live_rack.seen {
            Some(seen) if *seen == rack => {}
            Some(_) => {
                self.live_rack.seen = Some(rack);
                self.live_rack.modified = true;
                self.live_rack_touched(now);
            }
            None => self.live_rack.seen = Some(rack),
        }
        let errors: Vec<String> = self.live_rack.writer.as_ref().map(|w| w.errors.try_iter().collect()).unwrap_or_default();
        for e in errors {
            self.say(format!("the live rack could not be saved: {e}"), true);
        }
        let l = &self.live_rack;
        let due = l.dirty_since.is_some_and(|since| {
            now.saturating_sub(l.changed_ns) >= QUIET_NS || now.saturating_sub(since) >= MAX_WAIT_NS
        });
        if due && l.path.is_some() {
            self.save_live_rack_soon();
        }
    }

    /// The live rack as the file keeps it, plugin states included.
    fn live_rack_file(&self) -> LiveRackFile {
        let l = &self.live_rack;
        let rack = Rack { name: l.name.clone(), id: l.id.clone().unwrap_or_default(), ..self.capture_rack_with(true) };
        LiveRackFile { format: FORMAT.into(), version: VERSION, modified: l.modified, rack }
    }

    /// Hand the live rack to the `live-rack` thread (started on the first save).
    fn save_live_rack_soon(&mut self) {
        let Some(path) = self.live_rack.path.clone() else { return };
        self.live_rack.dirty_since = None;
        if self.live_rack.writer.is_none() {
            match Writer::spawn(path) {
                Ok(w) => self.live_rack.writer = Some(w),
                Err(e) => return self.say(format!("the live rack could not be saved: {e}"), true),
            }
        }
        let f = self.live_rack_file();
        if let Some(w) = &self.live_rack.writer {
            let _ = w.tx.send(f);
        }
    }

    /// Stopping: the playing plugins' states, then the live rack's last save, written here
    /// once the `live-rack` thread has written what it was given.
    pub(super) fn save_live_rack_on_stop(&mut self) {
        self.read_plugin_states_on_stop();
        if let Some(w) = self.live_rack.writer.take()
            && !w.finish(STOP_DEADLINE)
        {
            // Still writing: a second write now would race it for the same temp file.
            eprintln!("yahaha: the live rack could not be saved: the last autosave is still writing");
            self.live_rack.dirty_since = None;
            return;
        }
        let Some(path) = self.live_rack.path.clone() else { return };
        let f = self.live_rack_file();
        if let Err(e) = write(&path, &f) {
            eprintln!("yahaha: the live rack could not be saved: {e:#}");
        }
        self.live_rack.dirty_since = None;
    }

    pub(super) fn live_rack_state(&self) -> LiveRackState {
        let l = &self.live_rack;
        LiveRackState { name: l.name.clone(), id: l.id.clone(), modified: l.modified, prompt: l.prompt.clone() }
    }
}

impl Session {
    /// Tests: the live rack is `name` (the saved rack `id`), unmodified, as loading or
    /// saving a rack will leave it.
    #[cfg(test)]
    pub(crate) fn live_rack_clean(&self, name: &str, id: Option<String>) {
        self.inner.lock().live_rack_clean(name, id);
        self.settle();
    }
}

#[cfg(test)]
#[path = "live_rack_tests.rs"]
mod tests;

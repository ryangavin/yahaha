//! The yahaha app shell. It serves the frontend the way docs/app-api.md lays out:
//!
//! - command `send(cmd: AppCmd) -> Result<(), CmdError>`
//! - command `state() -> AppState`
//! - command `library() -> LibraryList`
//! - command `meters() -> Meters` (output levels since the last call; poll at display rate)
//! - commands `open_plugin_editor(part)` / `close_plugin_editor(part)`: a keyboard part's
//!   instrument plugin window, opened on the main thread (AppKit); closing it keeps the
//!   plugin's settings with the part (`savePartPluginState`)
//! - event `yahaha` (`Event`): `stateChanged { version }`, `libraryChanged { revision }`,
//!   `stopped`
//!
//! Behind them is either the real engine (`yahaha::Session`: MIDI, the Launchkey, the
//! synth) or `mock::MockSession`, a band that plays itself with no I/O:
//!
//! - `YAHAHA_MOCK=1`: the mock.
//! - `YAHAHA_STYLES=path[:path…]`: the engine, with those style files/folders. Without it,
//!   the repo's `corpus/` folder when it exists (a dev checkout), else the mock.
//! - `YAHAHA_SF2=file.sf2`: the synth's SoundFont; else the first `.sf2` in the repo's
//!   `soundfonts/`, else no synth.
//!
//! If the engine can't start (no CoreMIDI, say), the shell falls back to the mock, which then
//! reports an offline session with no Launchkey and the reason in the status line.
//! On exit the engine is stopped, which puts the Launchkey back in standalone mode.

pub mod mock;
mod mock_regist;
mod mock_looper;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use mock::MockSession;
use serde_json::Value;
use tauri::{Emitter, Manager, State};

enum Backend {
    Live(yahaha::Session),
    Mock(Box<Mutex<MockSession>>),
}

type Shared = Arc<Backend>;

fn failed(msg: impl ToString) -> Value {
    serde_json::json!({ "kind": "failed", "message": msg.to_string() })
}

/// Commands arrive as JSON and are parsed into the engine's `AppCmd`, so the shell speaks
/// exactly the documented shape whichever backend runs.
#[tauri::command]
fn send(cmd: Value, backend: State<'_, Shared>, app: tauri::AppHandle) -> Result<(), Value> {
    match &**backend {
        Backend::Live(s) => {
            let cmd: yahaha::AppCmd = serde_json::from_value(cmd).map_err(failed)?;
            s.send(cmd).map_err(|e| serde_json::to_value(e).unwrap_or_else(failed))
        }
        Backend::Mock(m) => {
            let cmd: yahaha::AppCmd = serde_json::from_value(cmd).map_err(failed)?;
            let mut m = m.lock().map_err(|_| serde_json::json!({ "kind": "busy" }))?;
            if m.send(cmd) {
                let _ = app.emit("yahaha", yahaha::Event::StateChanged { version: m.state.version });
            }
            Ok(())
        }
    }
}

#[tauri::command]
fn state(backend: State<'_, Shared>, app: tauri::AppHandle) -> Value {
    match &**backend {
        // With its clock read now (docs/app-api.md, `surface.clock`).
        Backend::Live(s) => serde_json::to_value(s.state_now()).unwrap_or(Value::Null),
        Backend::Mock(m) => {
            let mut m = m.lock().unwrap();
            // The mock's clock moved on to now first, as the engine's is always now.
            if m.catch_up() {
                let _ = app.emit("yahaha", yahaha::Event::StateChanged { version: m.state.version });
            }
            serde_json::to_value(m.state_now()).unwrap_or(Value::Null)
        }
    }
}

#[tauri::command]
fn library(backend: State<'_, Shared>) -> Value {
    match &**backend {
        Backend::Live(s) => serde_json::to_value(s.library_list()).unwrap_or(Value::Null),
        Backend::Mock(m) => serde_json::to_value(m.lock().unwrap().library()).unwrap_or(Value::Null),
    }
}

/// Output levels: each part's and the master's peak since the last call, and the clip
/// count. The mock has no audio: zero levels, no channels.
#[tauri::command]
fn meters(backend: State<'_, Shared>) -> Value {
    match &**backend {
        Backend::Live(s) => serde_json::to_value(s.meters()).unwrap_or(Value::Null),
        Backend::Mock(_) => serde_json::to_value(yahaha::api::Meters::default()).unwrap_or(Value::Null),
    }
}

thread_local! {
    /// The open plugin editor windows, by keyboard part. Main thread only (AppKit).
    static EDITORS: std::cell::RefCell<std::collections::HashMap<u8, yahaha::plugin::editor::Editor>> = Default::default();
}

/// Open keyboard part `part`'s plugin editor (or bring it to the front). The mock has no
/// plugins: it says so in the message line.
#[tauri::command]
fn open_plugin_editor(part: u8, backend: State<'_, Shared>, app: tauri::AppHandle) -> Result<(), Value> {
    let target = match &**backend {
        Backend::Live(s) => s.plugin_editor(part).ok_or_else(|| failed("the part is not playing a plugin"))?,
        Backend::Mock(_) => return Err(failed("the demo session has no plugins")),
    };
    let part = part & 3;
    app.run_on_main_thread(move || {
        let Ok(mtm) = yahaha::plugin::editor::main_thread() else { return };
        EDITORS.with(|eds| {
            let mut eds = eds.borrow_mut();
            // The same plugin's window still open: focus it. Otherwise (closed by the user,
            // or another plugin now) a new one.
            if let Some(e) = eds.get(&part)
                && e.is_open()
                && e.is_for(&target)
            {
                e.focus();
                return;
            }
            eds.remove(&part);
            match yahaha::plugin::editor::open_editor(mtm, &target) {
                Ok(e) => {
                    eds.insert(part, e);
                }
                Err(e) => eprintln!("plugin editor: {e:#}"),
            }
        });
    })
    .map_err(failed)
}

/// Close keyboard part `part`'s plugin editor and keep the plugin's settings with the part.
#[tauri::command]
fn close_plugin_editor(part: u8, backend: State<'_, Shared>, app: tauri::AppHandle) -> Result<(), Value> {
    let part = part & 3;
    app.run_on_main_thread(move || EDITORS.with(|eds| drop(eds.borrow_mut().remove(&part)))).map_err(failed)?;
    if let Backend::Live(s) = &**backend {
        let _ = s.send(yahaha::api::PluginCmd::SavePartPluginState { part });
    }
    Ok(())
}

/// Forward the engine's events to the webview. The frontend coalesces `stateChanged` to
/// one fetch per animation frame.
fn forward_events(app: tauri::AppHandle, backend: Shared) {
    let Backend::Live(s) = &*backend else { return };
    for e in s.subscribe() {
        let stopped = e == yahaha::Event::Stopped;
        if app.emit("yahaha", e).is_err() || stopped {
            break;
        }
    }
}

/// Tick the mock at ~60 Hz and tell the webview when its state changed.
fn tick_mock(app: tauri::AppHandle, backend: Shared) {
    let Backend::Mock(m) = &*backend else { return };
    let frame = Duration::from_micros(16_667);
    m.lock().unwrap().catch_up(); // start the mock's clock
    loop {
        std::thread::sleep(frame);
        let changed = {
            let mut m = m.lock().unwrap();
            m.catch_up().then_some(m.state.version)
        };
        if let Some(version) = changed {
            if app.emit("yahaha", yahaha::Event::StateChanged { version }).is_err() {
                break;
            }
        }
    }
}

/// The repo root in a dev checkout (app/src-tauri/../..).
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn first_sf2(dir: &Path) -> Option<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("sf2")))
        .collect();
    v.sort();
    v.into_iter().next()
}

fn backend() -> Backend {
    if std::env::var_os("YAHAHA_MOCK").is_some_and(|v| v != "0") {
        return Backend::Mock(Box::new(Mutex::new(MockSession::new())));
    }
    let paths: Vec<PathBuf> = match std::env::var_os("YAHAHA_STYLES") {
        Some(v) => std::env::split_paths(&v).collect(),
        None => {
            let corpus = repo_root().join("corpus");
            if corpus.is_dir() { vec![corpus] } else { vec![] }
        }
    };
    if paths.is_empty() {
        eprintln!("yahaha: no styles (set YAHAHA_STYLES); running the mock session");
        return Backend::Mock(Box::new(Mutex::new(MockSession::fallback(
            "No styles found (set YAHAHA_STYLES): a demo band with no sound or MIDI",
        ))));
    }
    let sf2 = std::env::var_os("YAHAHA_SF2").map(PathBuf::from).or_else(|| first_sf2(&repo_root().join("soundfonts")));
    match yahaha::Session::start(yahaha::Options { paths, sf2, data_dir: yahaha::session::default_data_dir(), ..yahaha::Options::default() }) {
        Ok(s) => Backend::Live(s),
        Err(e) => {
            eprintln!("yahaha: the engine didn't start ({e:#}); running the mock session");
            Backend::Mock(Box::new(Mutex::new(MockSession::fallback(format!(
                "The engine didn't start ({e:#}): a demo band with no sound or MIDI"
            )))))
        }
    }
}

/// Stop the engine: the band stops, the Launchkey's lights go off and it leaves DAW mode
/// (back to standalone), audio and MIDI close. Idempotent. Run on app exit: Tauri ends
/// the process with `exit()`, and the event thread holds a clone of the `Arc`, so
/// `Session`'s `Drop` never runs on its own.
fn shutdown(backend: &Backend) {
    if let Backend::Live(s) = backend {
        s.stop();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let shared: Shared = Arc::new(backend());
    let app = tauri::Builder::default()
        .manage(shared)
        .setup(|app| {
            let handle = app.handle().clone();
            let b = app.state::<Shared>().inner().clone();
            let live = matches!(*b, Backend::Live(_));
            std::thread::Builder::new()
                .name(if live { "session-events" } else { "mock-tick" }.into())
                .spawn(move || if live { forward_events(handle, b) } else { tick_mock(handle, b) })?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![send, state, library, meters, open_plugin_editor, close_plugin_editor])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");
    app.run(|app, event| {
        if let tauri::RunEvent::Exit = event {
            shutdown(&app.state::<Shared>());
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Launchkey goes back to standalone on exit: `shutdown` stops a live session (its
    /// subscribers get `Stopped`, which also ends the event-forwarding thread).
    #[test]
    fn shutdown_stops_a_live_session() {
        let style = repo_root().join("corpus/MOX_v2");
        let Some(path) = std::fs::read_dir(&style).ok().and_then(|d| d.filter_map(|e| e.ok()).map(|e| e.path()).find(|p| p.is_file()))
        else {
            eprintln!("corpus missing; skipping");
            return;
        };
        let s = yahaha::Session::offline(yahaha::Options { paths: vec![path], ..yahaha::Options::default() }).unwrap();
        let events = s.subscribe();
        let backend: Shared = Arc::new(Backend::Live(s));
        let held = backend.clone(); // as the event thread holds it
        shutdown(&backend);
        assert!(events.try_iter().any(|e| e == yahaha::Event::Stopped));
        shutdown(&held); // idempotent
    }

    /// When the engine can't start, the stand-in mock doesn't pass for a working rig.
    #[test]
    fn the_fallback_mock_says_it_is_not_the_engine() {
        let m = MockSession::fallback("The engine didn't start (no CoreMIDI)");
        let v = serde_json::to_value(&m.state).unwrap();
        assert_eq!(v["io"]["offline"], true);
        assert_eq!(v["pads"]["connected"], false);
        assert!(v["io"]["synth"].is_null());
        assert!(v["message"]["text"].as_str().unwrap().contains("didn't start"));
        let normal = serde_json::to_value(&MockSession::new().state).unwrap();
        assert_eq!(normal["pads"]["connected"], true, "YAHAHA_MOCK=1 keeps the demo rig");
    }
}

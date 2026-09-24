//! The yahaha app shell. It serves the frontend the way docs/app-api.md lays out:
//!
//! - command `send(cmd: AppCmd) -> Result<(), CmdError>`
//! - command `state() -> AppState`
//! - command `library() -> LibraryList`
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
//! If the engine can't start (no CoreMIDI, say), the shell falls back to the mock.

pub mod mock;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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
fn state(backend: State<'_, Shared>) -> Value {
    match &**backend {
        Backend::Live(s) => serde_json::to_value(&*s.state()).unwrap_or(Value::Null),
        Backend::Mock(m) => serde_json::to_value(&m.lock().unwrap().state).unwrap_or(Value::Null),
    }
}

#[tauri::command]
fn library(backend: State<'_, Shared>) -> Value {
    match &**backend {
        Backend::Live(s) => serde_json::to_value(s.library_list()).unwrap_or(Value::Null),
        Backend::Mock(m) => serde_json::to_value(m.lock().unwrap().library()).unwrap_or(Value::Null),
    }
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
    let mut last = Instant::now();
    loop {
        std::thread::sleep(frame);
        let now = Instant::now();
        let changed = {
            let mut m = m.lock().unwrap();
            m.advance(now.duration_since(last).as_secs_f64() * 1000.0).then_some(m.state.version)
        };
        last = now;
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
        return Backend::Mock(Box::new(Mutex::new(MockSession::new())));
    }
    let sf2 = std::env::var_os("YAHAHA_SF2").map(PathBuf::from).or_else(|| first_sf2(&repo_root().join("soundfonts")));
    match yahaha::Session::start(yahaha::Options { paths, sf2, ..yahaha::Options::default() }) {
        Ok(s) => Backend::Live(s),
        Err(e) => {
            eprintln!("yahaha: the engine didn't start ({e:#}); running the mock session");
            Backend::Mock(Box::new(Mutex::new(MockSession::new())))
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let shared: Shared = Arc::new(backend());
    tauri::Builder::default()
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
        .invoke_handler(tauri::generate_handler![send, state, library])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

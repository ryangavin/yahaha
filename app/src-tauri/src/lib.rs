//! The yahaha app shell. It serves the frontend the way docs/app-api.md (#16) lays out:
//!
//! - command `send(cmd: AppCmd) -> Result<(), CmdError>`
//! - command `state() -> AppState`
//! - command `library() -> LibraryList`
//! - event `yahaha` (`Event`): `stateChanged { version }`, `libraryChanged { revision }`
//!
//! Today they're backed by `mock::MockSession`, ticked at ~60 Hz. When #16 lands, swap it
//! for `yahaha::Session` (and `api` for `yahaha`'s types): `send` → `session.send`,
//! `state` → `session.state()`, `library` → `session.library_list()`, and forward
//! `session.subscribe()` events instead of the tick loop.

pub mod api;
pub mod mock;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use api::{AppCmd, AppState, CmdError, Event, LibraryList};
use mock::MockSession;
use tauri::{Emitter, Manager, State};

type Session = Arc<Mutex<MockSession>>;

#[tauri::command]
fn send(cmd: AppCmd, session: State<'_, Session>, app: tauri::AppHandle) -> Result<(), CmdError> {
    let mut s = session.lock().map_err(|_| CmdError::Busy)?;
    if s.send(cmd) {
        let _ = app.emit("yahaha", Event::StateChanged { version: s.state.version });
    }
    Ok(())
}

#[tauri::command]
fn state(session: State<'_, Session>) -> AppState {
    session.lock().unwrap().state.clone()
}

#[tauri::command]
fn library(session: State<'_, Session>) -> LibraryList {
    session.lock().unwrap().library().clone()
}

/// Tick the mock at ~60 Hz and tell the webview when its state changed.
fn tick_loop(app: tauri::AppHandle, session: Session) {
    let frame = Duration::from_micros(16_667);
    let mut last = Instant::now();
    loop {
        std::thread::sleep(frame);
        let now = Instant::now();
        let changed = {
            let mut s = session.lock().unwrap();
            s.advance(now.duration_since(last).as_secs_f64() * 1000.0).then_some(s.state.version)
        };
        last = now;
        if let Some(version) = changed {
            if app.emit("yahaha", Event::StateChanged { version }).is_err() {
                break;
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let session: Session = Arc::new(Mutex::new(MockSession::new()));
    tauri::Builder::default()
        .manage(session)
        .setup(|app| {
            let handle = app.handle().clone();
            let s = app.state::<Session>().inner().clone();
            std::thread::Builder::new().name("session-tick".into()).spawn(move || tick_loop(handle, s))?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![send, state, library])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

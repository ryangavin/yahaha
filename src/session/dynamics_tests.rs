//! Style Dynamics Control, Touch and Accent through an offline session (#180).

use crate::api::{DynamicsCmd, DynamicsState};
use crate::session::{Options, Port, Session};
use std::path::Path;

const MS: u64 = 1_000_000;

fn session() -> Option<Session> {
    let style = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
    if !style.exists() {
        eprintln!("corpus missing; skipping");
        return None;
    }
    Some(Session::offline(Options { paths: vec![style], ..Options::default() }).unwrap())
}

fn strike(s: &Session, key: u8, vel: u8) {
    s.midi_in(Port::Keys, &[0x90, key, vel]);
    s.midi_in(Port::Keys, &[0x80, key, 0]);
}

#[test]
fn settings_and_the_level() {
    let Some(s) = session() else { return };
    let d = s.state().dynamics.clone();
    assert_eq!(d, DynamicsState { control: true, level: 127, touch: false, accent: false, accent_threshold: 110, ..Default::default() });
    s.send(DynamicsCmd::SetDynamics { level: 100 }).unwrap();
    s.send(DynamicsCmd::StepDynamics { delta: -10 }).unwrap();
    s.send(DynamicsCmd::SetDynamicsControl { on: false }).unwrap();
    s.send(DynamicsCmd::SetAccentThreshold { velocity: 120 }).unwrap();
    s.send(DynamicsCmd::ToggleAccent).unwrap();
    let d = s.state().dynamics.clone();
    assert_eq!(d, DynamicsState { control: false, level: 90, touch: false, accent: true, accent_threshold: 120, ..Default::default() });
}

/// Touch: the left hand's strikes set the level (Touch off, they do not), and a later
/// setting keeps the level the hand set.
#[test]
fn touch_follows_the_left_hand() {
    let Some(s) = session() else { return };
    strike(&s, 38, 30);
    s.advance(10 * MS);
    assert_eq!(s.state().dynamics.level, 127, "Touch off");
    s.send(DynamicsCmd::SetDynamicsTouch { on: true }).unwrap();
    strike(&s, 36, 100);
    s.advance(10 * MS);
    assert_eq!(s.state().dynamics.level, 127);
    strike(&s, 38, 30);
    s.advance(10 * MS);
    assert_eq!(s.state().dynamics.level, 38);
    // The right hand does not move it.
    strike(&s, 72, 127);
    s.advance(10 * MS);
    assert_eq!(s.state().dynamics.level, 38);
    s.send(DynamicsCmd::SetAccent { on: true }).unwrap();
    assert_eq!(s.state().dynamics.level, 38, "a setting keeps the level Touch set");
    // Touch and Accent off: strikes no longer reach the engine.
    s.send(DynamicsCmd::SetAccent { on: false }).unwrap();
    s.send(DynamicsCmd::SetDynamicsTouch { on: false }).unwrap();
    strike(&s, 36, 127);
    s.advance(10 * MS);
    assert_eq!(s.state().dynamics.level, 38);
}

/// Dynamics starts at its maximum (as written) and goes back there with each style load.
#[test]
fn a_style_load_maxes_dynamics() {
    let Some(s) = session() else { return };
    assert_eq!(s.state().dynamics.level, 127);
    s.send(DynamicsCmd::SetDynamics { level: 40 }).unwrap();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2");
    let other = std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()).map(|e| e.path())
        .find(|p| p.extension().is_some_and(|x| x == "sty") && !p.ends_with("SlowWalker.T552.sty"));
    let Some(other) = other else { return };
    s.send(crate::api::LibraryCmd::LoadStylePath { path: other.display().to_string() }).unwrap();
    s.advance(50 * MS);
    assert_eq!(s.state().dynamics.level, 127);
}

/// Accent Mode Fill: a hard left-hand strike while a Main plays queues the Main's fill.
#[test]
fn a_hard_left_hand_strike_plays_the_fill() {
    let Some(s) = session() else { return };
    s.send(DynamicsCmd::SetAccent { on: true }).unwrap();
    s.send(DynamicsCmd::SetAccentMode { mode: crate::engine::AccentMode::Fill }).unwrap();
    // Sync Start: the chord starts the band.
    for k in [36, 40, 43] {
        s.midi_in(Port::Keys, &[0x90, k, 90]);
    }
    s.advance(700 * MS);
    let st = s.state();
    assert!(st.transport.running);
    assert_eq!(st.transport.queued, None);
    s.midi_in(Port::Keys, &[0x80, 36, 0]);
    s.midi_in(Port::Keys, &[0x90, 36, 120]);
    s.advance(MS);
    assert_eq!(s.state().transport.queued.as_deref(), Some("Fill In AA"));
}

/// Accent Mode and Source reach the state and survive other Dynamics commands.
#[test]
fn accent_mode_and_source() {
    let Some(s) = session() else { return };
    let d = s.state().dynamics.clone();
    assert_eq!((d.accent_mode, d.accent_source), (crate::engine::AccentMode::Hits, crate::engine::AccentSource::Left));
    s.send(DynamicsCmd::SetAccentMode { mode: crate::engine::AccentMode::Fill }).unwrap();
    s.send(DynamicsCmd::SetAccentSource { source: crate::engine::AccentSource::Both }).unwrap();
    s.send(DynamicsCmd::ToggleAccent).unwrap();
    let d = s.state().dynamics.clone();
    assert_eq!((d.accent_mode, d.accent_source, d.accent), (crate::engine::AccentMode::Fill, crate::engine::AccentSource::Both, true));
}

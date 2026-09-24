//! yahaha: a software arranger that plays Yamaha Genos/PSR style files live.
//!
//! The library holds the engine, the real-time runtime and the app API. Clients (the
//! terminal UI in the `yahaha` binary, the desktop app) drive it through
//! [`Session`]: [`AppCmd`] in, [`AppState`] out. See docs/app-api.md.

pub mod api;
pub mod arp;
pub mod bench;
pub mod capture;
pub mod click;
pub mod controllers;
pub mod engine;
pub mod fingering;
pub mod ireal;
#[cfg(test)]
mod golden;
pub mod harmony;
pub mod launchkey;
pub mod library;
pub mod live;
pub mod looper;
pub mod midi;
pub mod multipad;
pub mod oracle;
pub mod parts;
#[cfg(feature = "plugins")]
pub mod plugin;
#[cfg(test)]
mod recognizer_golden;
pub mod rt;
pub mod session;
pub mod sff;
pub mod sim;
pub mod synth;
pub mod theory;

pub use api::{AppCmd, AppState, Event};
pub use session::{Options, Session};

use anyhow::Result;

/// Parse a chord symbol: root, a `TYPE_NAMES` suffix, and an optional `/bass`.
pub fn parse_chord(s: &str) -> Result<theory::Chord> {
    let (body, bass) = match s.split_once('/') {
        Some((b, bass)) => (b, Some(bass)),
        None => (s, None),
    };
    let root_len = if body.len() > 1 && (body.as_bytes()[1] == b'#' || body.as_bytes()[1] == b'b') { 2 } else { 1 };
    let pc = |n: &str| theory::NOTE_NAMES.iter().position(|x| *x == n).or_else(|| {
        ["C", "Db", "D", "D#", "E", "F", "Gb", "G", "G#", "A", "A#", "B"].iter().position(|x| *x == n)
    });
    let root = body.get(..root_len).and_then(pc).ok_or_else(|| anyhow::anyhow!("bad chord {s}"))? as u8;
    let suffix = &body[root_len..];
    let ty = theory::TYPE_NAMES.iter().position(|t| *t == suffix)
        // "6/9" would read as a bass note, so the 6(9) chord is also spelled "6(9)" or "69".
        .or_else(|| matches!(suffix, "6(9)" | "69").then_some(6))
        .ok_or_else(|| anyhow::anyhow!("bad chord type {suffix}"))? as u8;
    let bass = bass.map(|b| pc(b).map(|p| p as u8).ok_or_else(|| anyhow::anyhow!("bad bass {b}"))).transpose()?;
    Ok(theory::Chord { root, ty, bass: bass.filter(|&b| b != root) })
}

/// "F#2" (Yamaha numbering, C3 = 60) or a raw MIDI number.
pub fn parse_note(s: &str) -> Option<u8> {
    if let Ok(n) = s.parse::<u8>() {
        return Some(n);
    }
    let (name, oct) = s.split_at(s.find(|c: char| c.is_ascii_digit() || c == '-')?);
    let pc = theory::NOTE_NAMES.iter().position(|n| n.eq_ignore_ascii_case(name))
        .or_else(|| ["C", "Db", "D", "D#", "E", "F", "Gb", "G", "G#", "A", "A#", "B"].iter().position(|n| n.eq_ignore_ascii_case(name)))?;
    let oct: i32 = oct.parse().ok()?;
    u8::try_from((oct + 2) * 12 + pc as i32).ok()
}

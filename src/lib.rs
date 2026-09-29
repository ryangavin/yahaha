//! yahaha: a software arranger that plays Yamaha Genos/PSR style files live.
//!
//! The library holds the engine, the real-time runtime and the app API. Clients (the
//! terminal UI in the `yahaha` binary, the desktop app) drive it through
//! [`Session`]: [`AppCmd`] in, [`AppState`] out. See docs/app-api.md.

pub mod api;
pub use yahaha_core::arp;
pub mod bench;
pub mod capture;
pub use yahaha_core::click;
pub mod controllers;
pub use yahaha_core::data_files;
pub mod engine;
pub use yahaha_core::fingering;
pub use yahaha_fx::fx;
pub use yahaha_core::ireal;
pub mod knobs;
#[cfg(test)]
mod golden;
pub use yahaha_core::harmony;
pub mod launchkey;
pub mod library;
pub mod live;
pub use yahaha_core::looper;
pub use yahaha_core::megavoice;
pub use yahaha_core::midi;
pub mod multipad;
pub mod oracle;
pub mod parts;
pub use yahaha_core::parts_data;
pub mod patches;
pub mod perf;
#[cfg(feature = "plugins")]
pub mod plugin;
// Plugin hosting is Audio Units: macOS only. Without the feature, session's rack is the
// stub it already uses when plugins are off.
#[cfg(all(feature = "plugins", not(target_os = "macos")))]
compile_error!("the `plugins` feature (Audio Unit hosting) is macOS only; build without it on this platform");
#[cfg(test)]
mod recognizer_golden;
// Tests of core modules that need a higher layer (engine, sff, sim, library).
#[cfg(test)]
mod fingering_tests;
#[cfg(test)]
mod megavoice_tests;
#[cfg(test)]
mod theory_corpus_tests;
// Tests of fx that need a higher layer (sff, engine).
#[cfg(test)]
mod fx_tests;
pub mod racks;
pub use yahaha_core::route;
pub use yahaha_core::rt;
pub mod session;
pub mod sff;
pub mod sim;
pub use yahaha_core::style_types;
pub mod synth;
pub use yahaha_core::theory;
pub use yahaha_core::tone;
pub use yahaha_core::voice_gm;

pub use api::{AppCmd, AppState, Event};
pub use session::{Options, Session};
/// Chord and note-name parsing lives in yahaha-core with the theory it reads.
pub use yahaha_core::{parse_chord, parse_note};

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
pub use yahaha_engine::controllers;
pub use yahaha_core::data_files;
pub use yahaha_engine::engine;
// Tests of engine modules that need a higher layer (synth).
#[cfg(test)]
mod engine_tests;
pub use yahaha_core::fingering;
pub use yahaha_fx::fx;
pub use yahaha_core::ireal;
pub mod knobs;
#[cfg(test)]
mod golden;
pub use yahaha_core::harmony;
pub use yahaha_engine::launchkey;
pub use yahaha_sff::library;
// Tests of sff/library that read the facade's own source.
#[cfg(test)]
mod library_tests;
pub mod live;
pub use yahaha_core::looper;
pub use yahaha_core::megavoice;
pub use yahaha_core::midi;
pub use yahaha_engine::multipad;
pub mod oracle;
pub use yahaha_engine::parts;
pub use yahaha_core::parts_data;
pub use yahaha_synth::patches;
// Tests of patches that need a higher layer (session).
#[cfg(test)]
mod patches_tests;
pub use yahaha_synth::perf;
// Plugin hosting (`plugins` forwards to yahaha-synth's feature, which also holds the
// macOS-only guard).
#[cfg(feature = "plugins")]
pub use yahaha_synth::plugin;
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
pub use yahaha_sff::sff;
pub use yahaha_engine::sim;
pub use yahaha_core::style_types;
pub use yahaha_synth::synth;
pub use yahaha_core::theory;
pub use yahaha_core::tone;
pub use yahaha_core::voice_gm;

pub use api::{AppCmd, AppState, Event};
pub use session::{Options, Session};
/// Chord and note-name parsing lives in yahaha-core with the theory it reads.
pub use yahaha_core::{parse_chord, parse_note};

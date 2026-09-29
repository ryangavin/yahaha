//! yahaha-synth: the built-in SoundFont synth (on rustysynth), the sound library, Audio
//! Unit plugin hosting and the performance view (AGENTS.md, Layering).
//!
//! Depends on `yahaha-core`, `yahaha-sff`, `yahaha-fx` and `yahaha-engine` (the synth
//! reads the keyboard parts, `engine::Parts`, so it sits above the engine). The `yahaha`
//! facade re-exports each module under its old path (`yahaha::synth`, `crate::synth` in
//! the facade), so nothing that uses them changes.

pub mod patches;
pub mod perf;
// Plugin hosting (#35) is behind the `plugins` feature; the facade's `plugins` feature
// forwards here.
#[cfg(feature = "plugins")]
pub mod plugin;
// Plugin hosting is Audio Units: macOS only. Without the feature, the facade's rack is the
// stub it already uses when plugins are off.
#[cfg(all(feature = "plugins", not(target_os = "macos")))]
compile_error!("the `plugins` feature (Audio Unit hosting) is macOS only; build without it on this platform");
pub mod synth;

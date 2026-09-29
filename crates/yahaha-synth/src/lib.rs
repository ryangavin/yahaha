//! yahaha-synth: the built-in SoundFont synth, the sound library and plugin hosting
//! (AGENTS.md, Layering). Empty for now.
//!
//! Modules that will move in from the `yahaha` facade: `synth`, `patches`, `plugin`,
//! `perf`. Plugin hosting is behind the `plugins` feature (macOS only).
//!
//! Depends on `yahaha-core`, `yahaha-sff`, `yahaha-fx` and `yahaha-engine` (the synth
//! reads the keyboard parts, `engine::Parts`, so it sits above the engine).

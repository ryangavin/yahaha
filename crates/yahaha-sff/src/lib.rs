//! yahaha-sff: style files (SFF1/SFF2) and the style library (AGENTS.md, Layering).
//!
//! Depends on `yahaha-core`. The `yahaha` facade re-exports both modules under their old
//! paths (`yahaha::sff`, `crate::sff` in the facade), so nothing that uses them changes.

pub mod library;
pub mod sff;

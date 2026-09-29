//! yahaha-fx: the audio effects (AGENTS.md, Layering): the send bus, the insertion
//! effects, the part EQ and the master chain, all real-time safe.
//!
//! Depends on `yahaha-core` only. The `yahaha` facade re-exports `fx` under its old path
//! (`yahaha::fx`, `crate::fx` in the facade), so nothing that uses it changes.

pub mod fx;

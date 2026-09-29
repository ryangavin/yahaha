//! yahaha-engine: the arranger engine (AGENTS.md, Layering): the engine, the Multi Pads,
//! the keyboard parts, the controllers, the Launchkey mapping and the simulator the tests
//! drive the engine with.
//!
//! Depends on `yahaha-core`, `yahaha-sff` and `yahaha-fx`. The `yahaha` facade re-exports
//! each module under its old path (`yahaha::engine`, `crate::engine` in the facade), so
//! nothing that uses them changes.

pub mod controllers;
pub mod engine;
pub mod launchkey;
pub mod multipad;
pub mod parts;
pub mod sim;

//! Multi Pads (docs/genos-features.md §7): the bank file parser ([`file`]) and a player
//! core ([`player`]) that is independent of the engine thread.

pub mod file;
pub mod player;

pub use file::{Pad, PadBank, PADS};
pub use player::{start_tick, sync_fires, Clock, MultiPadPlayer, PadState, SyncTrigger};

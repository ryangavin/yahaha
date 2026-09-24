//! Session-wide commands (panic, the message line) and the message itself.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum SystemCmd {
    /// All notes off, the style stops.
    Panic,
    /// Clear `AppState::message`.
    ClearMessage,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    /// Increases with every new message (so the same text twice is two messages).
    pub seq: u64,
    pub text: String,
    pub error: bool,
}

//! Racks (docs/racks.md): the live rack's name, where it came from and whether it has
//! unsaved changes, and its controller map.

pub use crate::racks::{ControlMap, ControlTarget};
use serde::{Deserialize, Serialize};

/// The live rack: what's under the player's hands now, autosaved and restored on boot.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveRackState {
    /// Its name: the saved rack's it came from, "Restored" (the first boot after racks
    /// came in), or "New rack".
    pub name: String,
    /// The id of the saved rack it came from; null when it came from none.
    pub id: Option<String>,
    /// It changed since it was loaded or saved: a sound, the mix, the split, Harmony/Arp,
    /// the transpose, the controller map, or a plugin edit (`soundEdited`).
    pub modified: bool,
    /// Its controller map: what Launchkey faders 1-4 and knobs 1-8 do on the Rack knob
    /// page. Shown in the Rack panel.
    #[serde(default)]
    pub controls: ControlMap,
}

//! Parameter Lock (Genos RM p.163, Menu › Utility › Parameter Lock): a locked parameter
//! group changes only from the panel. Registration Memory, One Touch Setting and Playlist
//! recalls leave it as the player set it. docs/registration.md lists what each group covers.

use serde::{Deserialize, Serialize};

/// A Parameter Lock group: the Data List's Parameter Chart "Parameter Lock" column, for the
/// groups whose parameters yahaha has. (The Genos also has Master EQ, Reverb Type, the
/// Reverb/Chorus/Variation Return Levels and Vocal Harmony/Mic Setting.)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LockItem {
    /// "Split Point": the split point.
    SplitPoint,
    /// "Fingering Type": the fingering type and the Chord Detection Area (Upper, and the
    /// Manual Bass that goes with it).
    FingeringType,
}

impl LockItem {
    /// Every group, in the order the Parameter Lock page lists them.
    pub const ALL: [LockItem; 2] = [LockItem::SplitPoint, LockItem::FingeringType];
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ParamLockCmd {
    /// Lock (`on`) or unlock a Parameter Lock group. A setup setting: kept across banks and
    /// sessions, never in a bank.
    SetParamLock { item: LockItem, on: bool },
}

/// Which Parameter Lock groups are locked (all unlocked by default, as on the Genos).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ParamLockState {
    pub split_point: bool,
    pub fingering_type: bool,
}

impl ParamLockState {
    pub fn get(&self, item: LockItem) -> bool {
        match item {
            LockItem::SplitPoint => self.split_point,
            LockItem::FingeringType => self.fingering_type,
        }
    }

    pub fn set(&mut self, item: LockItem, on: bool) {
        match item {
            LockItem::SplitPoint => self.split_point = on,
            LockItem::FingeringType => self.fingering_type = on,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_follows_set_for_every_group() {
        for item in LockItem::ALL {
            let mut s = ParamLockState::default();
            assert!(!s.get(item));
            s.set(item, true);
            assert!(s.get(item));
            assert_eq!(LockItem::ALL.iter().filter(|&&i| s.get(i)).count(), 1, "{item:?} alone");
        }
    }

    #[test]
    fn missing_groups_read_as_unlocked() {
        let s: ParamLockState = serde_json::from_str(r#"{"splitPoint":true}"#).unwrap();
        assert!(s.split_point && !s.fingering_type);
    }
}

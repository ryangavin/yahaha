//! The settings a rack shares with Registration Memory, as both store them: a keyboard
//! part's voice settings ([`ToneReg`]) and Keyboard Harmony/Arpeggio ([`HarmonyArpReg`]).

use crate::api::{ArpQuantize, ArpVelocityMode, HarmonyArpMode, HarmonyAssign, HarmonySpeed};
use crate::parts;
use serde::{Deserialize, Serialize};

/// A part's `parts::TONE_CC` controllers by name, and its XG multi part parameters as
/// `[hh, nn, vv]` (`F0 43 1n 4C hh pp nn vv F7`). None / empty: not set.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToneReg {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cutoff: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resonance: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attack: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decay: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vibrato_rate: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vibrato_depth: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vibrato_delay: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub portamento: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub portamento_time: Option<u8>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub xg: Vec<[u8; 3]>,
}

impl ToneReg {
    pub fn is_empty(&self) -> bool {
        *self == ToneReg::default()
    }

    /// Part `p`'s voice settings as they are now.
    pub fn capture(kp: &parts::Parts, p: usize) -> ToneReg {
        let t = kp.tone(p);
        ToneReg {
            cutoff: t[parts::CUTOFF],
            resonance: t[parts::RESONANCE],
            attack: t[parts::ATTACK],
            decay: t[parts::DECAY],
            release: t[parts::RELEASE],
            vibrato_rate: t[parts::VIBRATO_RATE],
            vibrato_depth: t[parts::VIBRATO_DEPTH],
            vibrato_delay: t[parts::VIBRATO_DELAY],
            portamento: t[parts::PORTAMENTO],
            portamento_time: t[parts::PORTAMENTO_TIME],
            xg: kp.xg(p).into_iter().map(|(hh, nn, vv)| [hh, nn, vv]).collect(),
        }
    }

    /// By `parts::TONE_CC` index.
    pub fn controllers(&self) -> [Option<u8>; parts::TONE] {
        let mut t = [None; parts::TONE];
        t[parts::CUTOFF] = self.cutoff;
        t[parts::RESONANCE] = self.resonance;
        t[parts::ATTACK] = self.attack;
        t[parts::DECAY] = self.decay;
        t[parts::RELEASE] = self.release;
        t[parts::VIBRATO_RATE] = self.vibrato_rate;
        t[parts::VIBRATO_DEPTH] = self.vibrato_depth;
        t[parts::VIBRATO_DELAY] = self.vibrato_delay;
        t[parts::PORTAMENTO] = self.portamento;
        t[parts::PORTAMENTO_TIME] = self.portamento_time;
        t
    }
}

/// Keyboard Harmony/Arpeggio (Data List, Freeze group "Keyboard Harmony/Arpeggio"): the
/// HARMONY/ARPEGGIO switch, the selected type and the detail settings. The type and
/// pattern are stored by name, so a list that grows or reorders still finds them. The
/// Arpeggio Hold pedal function is not stored: it is the pedal's, not a setting.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HarmonyArpReg {
    pub on: bool,
    pub mode: HarmonyArpMode,
    pub harmony_type: String,
    pub arp_pattern: String,
    pub volume: u8,
    pub speed: HarmonySpeed,
    pub assign: HarmonyAssign,
    pub chord_note_only: bool,
    pub touch_limit: u8,
    pub arp_quantize: ArpQuantize,
    pub arp_hold: bool,
    pub arp_velocity: ArpVelocityMode,
    pub arp_fixed_velocity: u8,
    pub arp_keep_key_on: bool,
}

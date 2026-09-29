//! Channel strips and send effects (the mixer rework). Every strip (the 4 keyboard parts,
//! the 8 Style parts) runs EQ → compressor → insert 1 → insert 2 → sends → pan/level.
//! A strip is addressed by `strip`, 0-11: 0-3 the keyboard parts (Right 1, Right 2,
//! Right 3, Left), 4-11 the Style parts (Rhythm 1 ... Phrase 2).
//!
//! The send effects are up to [`SENDS`]: sends 1-3 (`send` 0-2) are always there and are
//! the three buses the style's reverb, chorus and delay sends feed (their kinds set by the
//! style as `FxCmd` sets them); sends 4-6 (`send` 3-5) are added by the player and
//! belong to the rack.
//!
//! What older commands already cover goes through them ([`StripCmd::legacy`]): a
//! keyboard strip's EQ, sends 1-3 and insert 1, a Style strip's sends 1-3 and its style
//! insert's on/off and amount, sends 1-3's kinds, parameters and returns. The rest is
//! kept in [`Strips`] and shown in the state; nothing plays it yet (the mixer rework's
//! lanes).

use crate::fx::{INSERT_SLOTS, InsertSlot, KnobSpec, PartComp, SendSlot};
use serde::{Deserialize, Serialize};

pub use crate::fx::{InsertType, PartCompParam, SENDS, SendKind};

/// Channel strips: the keyboard parts', then the Style parts'.
pub const STRIPS: usize = 12;
/// Strips 0-3 are the keyboard parts'.
pub const KEYBOARD_STRIPS: usize = 4;
/// Sends 1-3 (0-2) are the buses the style feeds.
pub const STYLE_SENDS: usize = 3;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum StripCmd {
    /// A strip's EQ (the low and high shelves, clamped as `setPartEq`). On a keyboard
    /// strip it is `setPartEq`.
    SetStripEq { strip: u8, eq: crate::api::PartEq },
    /// A strip's compressor on or off.
    SetStripCompressorOn { strip: u8, on: bool },
    /// A strip compressor's type: its parameters come with it.
    SetStripCompressorPreset { strip: u8, preset: crate::api::CompPreset },
    /// One strip compressor parameter, clamped to its range: `threshold` -48..0 dB,
    /// `ratio` 10-200 (tenths: 40 = 4:1), `attack` 1-100 ms, `release` 10-1000 ms,
    /// `makeup` 0-24 dB.
    SetStripCompressorParam { strip: u8, param: PartCompParam, value: i16 },
    /// Insert slot `slot` (0-1) plays `kind` ("none" empties it), its settings at the
    /// kind's defaults. On/off is unchanged. An unknown kind is refused.
    SetStripInsertKind { strip: u8, slot: u8, kind: InsertType },
    /// Insert slot `slot` on or off.
    SetStripInsertOn { strip: u8, slot: u8, on: bool },
    /// One of an insert's settings (`setting` 0-3, by `InsertSlotState::settings`),
    /// clamped to its range. A setting its kind hasn't is refused.
    SetStripInsertSetting { strip: u8, slot: u8, setting: u8, value: u16 },
    /// A strip's send level to send effect `send` (0-5), 0-127. Sends 3-5 must be there.
    SetStripSend { strip: u8, send: u8, level: u8 },
    /// Add a send effect (send 4-6, the rack's) playing `kind` at its defaults, returning
    /// at 0 dB. Refused when all six are there.
    AddSend { kind: SendKind },
    /// Remove an added send effect (`send` 3-5); the ones after it move down, with every
    /// strip's level to them.
    RemoveSend { send: u8 },
    /// A send effect's kind; its parameters go back to the kind's defaults. Sends 0-2 take
    /// their own bus's kinds (reverb, chorus, delay) for now.
    SetSendKind { send: u8, kind: SendKind },
    /// One of a send effect's parameters (`param` by `SendState::params`), clamped.
    SetSendParam { send: u8, param: u8, value: u16 },
    /// A send effect's return level, 0-127 (64 = 0 dB).
    SetSendReturn { send: u8, level: u8 },
    /// Whether the live rack overrides send `send` (0-2)'s kind (on: the rack keeps the
    /// kind it has now and brings it back on load, over the style's).
    SetRackSendOverride { send: u8, on: bool },
}

/// A strip compressor as the app shows it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartCompState {
    pub on: bool,
    pub preset: crate::api::CompPreset,
    /// dB, -48..0.
    pub threshold: i8,
    /// Tenths, 10-200.
    pub ratio: u8,
    /// ms, 1-100.
    pub attack: u16,
    /// ms, 10-1000.
    pub release: u16,
    /// dB, 0-24.
    pub makeup: i8,
    /// The parameters differ from the type's.
    pub edited: bool,
}

impl From<PartComp> for PartCompState {
    fn from(c: PartComp) -> PartCompState {
        let c = c.clamped();
        PartCompState { on: c.on, preset: c.preset, threshold: c.threshold, ratio: c.ratio, attack: c.attack, release: c.release, makeup: c.makeup, edited: c.edited() }
    }
}

impl Default for PartCompState {
    fn default() -> PartCompState {
        PartComp::default().into()
    }
}

/// One setting of an insert or parameter of a send effect, as the app shows it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingState {
    /// "Drive".
    pub name: String,
    pub value: u16,
    pub min: u16,
    pub max: u16,
    /// The kind starts it here.
    pub default: u16,
    /// "64", "12 ms", "0.50 Hz".
    pub display: String,
}

impl SettingState {
    fn of(s: &KnobSpec, v: u16) -> SettingState {
        let value = s.clamp(v);
        SettingState { name: s.name.into(), value, min: s.min, max: s.max, default: s.default, display: s.display(value) }
    }
}

/// An insert slot as the app shows it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InsertSlotState {
    pub kind: InsertType,
    /// "Auto Wah"; "None" for an empty slot.
    pub name: String,
    pub on: bool,
    /// Its kind's settings (2-4), in order; none for an empty slot.
    pub settings: Vec<SettingState>,
}

impl From<&InsertSlot> for InsertSlotState {
    fn from(s: &InsertSlot) -> InsertSlotState {
        InsertSlotState { kind: s.kind.clone(), name: s.kind.name().into(), on: s.on, settings: s.kind.settings().iter().zip(s.values).map(|(k, v)| SettingState::of(k, v)).collect() }
    }
}

/// One strip: EQ, compressor, two insert slots, and its level to each send effect.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StripState {
    pub eq: crate::api::PartEq,
    pub comp: PartCompState,
    pub inserts: [InsertSlotState; INSERT_SLOTS],
    /// Its level to sends 1-6, 0-127 (a send that isn't there: 0).
    pub sends: [u8; SENDS],
}

/// One send effect as the app shows it (`EffectsState::sends`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendState {
    /// 0-5.
    pub send: u8,
    pub kind: SendKind,
    /// "Hall".
    pub name: String,
    /// Its kind's parameters, in order.
    pub params: Vec<SettingState>,
    /// 0-127, 64 = 0 dB.
    pub return_level: u8,
    /// Fed by the style's sends (sends 1-3).
    pub from_style: bool,
    /// The rack sets it: an added send (4-6), or send 1-3 with the rack's override on.
    pub set_by_rack: bool,
}

impl SendState {
    fn of(send: usize, s: &SendSlot, from_style: bool, set_by_rack: bool) -> SendState {
        let params = s.kind.params().iter().zip(s.params).map(|(k, v)| SettingState::of(k, v)).collect();
        SendState { send: send as u8, kind: s.kind.clone(), name: s.kind.name().into(), params, return_level: s.return_level, from_style, set_by_rack }
    }
}

/// One strip's settings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StripSettings {
    pub eq: crate::api::PartEq,
    pub comp: PartComp,
    pub inserts: [InsertSlot; INSERT_SLOTS],
    pub sends: [u8; SENDS],
}

/// The strips and the added send effects: what the strip commands keep. The session and
/// the dev mock both hold one and play the commands through it.
#[derive(Clone, Debug, PartialEq)]
pub struct Strips {
    pub strips: [StripSettings; STRIPS],
    /// Sends 4-6, as added (at most 3).
    pub added: Vec<SendSlot>,
    /// Sends 1-3: the rack overrides the style's kind.
    pub overrides: [bool; STYLE_SENDS],
    /// Sends 1-3 as last shown (their kind, parameters and return come from the buses;
    /// `fill` keeps them here so a check needs no state).
    pub style_sends: [SendSlot; STYLE_SENDS],
}

impl Default for Strips {
    fn default() -> Strips {
        Strips {
            strips: Default::default(),
            added: Vec::new(),
            overrides: [false; STYLE_SENDS],
            style_sends: [SendSlot::of(SendKind::Hall), SendSlot::of(SendKind::Chorus), SendSlot::of(SendKind::DottedEighth)],
        }
    }
}

/// The bus a send 0-2 is.
fn block_of(send: usize) -> crate::api::FxBlock {
    crate::api::FxBlock::ALL[send]
}

/// A bus kind as the older API names it.
fn fx_type(kind: &SendKind) -> Option<(crate::api::FxBlock, crate::api::FxType)> {
    let (block, index) = kind.bus()?;
    let b = crate::api::FxBlock::ALL[block];
    Some((b, *b.types().get(index as usize)?))
}

impl StripCmd {
    /// The older commands this one is, when they cover it: the session and the dev mock
    /// send those first (and still keep this one in [`Strips`], for what the older
    /// ones don't carry). Empty: [`Strips::apply`] alone.
    pub fn legacy(&self) -> Vec<crate::api::AppCmd> {
        use crate::api::PartsCmd;
        // Emptying a keyboard strip's insert 1: the slot goes back to `PartInsert::OFF`,
        // which `fill` shows as none (turning it off alone would keep the old kind).
        if let StripCmd::SetStripInsertKind { strip, slot: 0, kind: InsertType::None } = *self {
            if (strip as usize) < KEYBOARD_STRIPS {
                let off = crate::api::PartInsert::OFF;
                return vec![
                    PartsCmd::SetKeyboardInsertEffect { part: strip, effect: off.effect }.into(),
                    PartsCmd::SetKeyboardInsertOn { part: strip, on: off.on }.into(),
                    PartsCmd::SetKeyboardInsertAmount { part: strip, amount: off.amount }.into(),
                ];
            }
        }
        self.legacy_one().map_or_else(Vec::new, |c| vec![c])
    }

    fn legacy_one(&self) -> Option<crate::api::AppCmd> {
        use crate::api::{FxCmd, InsertEffect, MixerCmd, PartSend, PartsCmd};
        let kb = |strip: u8| ((strip as usize) < KEYBOARD_STRIPS).then_some(strip);
        let style = |strip: u8| (strip as usize).checked_sub(KEYBOARD_STRIPS).filter(|&p| p < 8).map(|p| p as u8);
        let part_send = |send: u8| [PartSend::Reverb, PartSend::Chorus, PartSend::Variation].get(send as usize).copied();
        Some(match *self {
            StripCmd::SetStripEq { strip, eq } => PartsCmd::SetPartEq { part: kb(strip)?, eq }.into(),
            StripCmd::SetStripSend { strip, send, level } => match (kb(strip), style(strip), part_send(send)) {
                (Some(part), _, Some(send)) => PartsCmd::SetPartSend { part, send, value: level }.into(),
                (_, Some(part), Some(send)) => MixerCmd::SetStylePartSend { part, send, value: level }.into(),
                _ => return None,
            },
            StripCmd::SetStripInsertKind { strip, slot: 0, ref kind } => {
                let part = kb(strip)?;
                match kind {
                    // `legacy` empties the slot.
                    InsertType::None => return None,
                    InsertType::Unknown(_) => return None,
                    k => PartsCmd::SetKeyboardInsertEffect { part, effect: InsertEffect::from(k.kind()) }.into(),
                }
            }
            StripCmd::SetStripInsertOn { strip, slot: 0, on } => match (kb(strip), style(strip)) {
                (Some(part), _) => PartsCmd::SetKeyboardInsertOn { part, on }.into(),
                (_, Some(part)) => FxCmd::SetPartInsertOn { part, on }.into(),
                _ => return None,
            },
            StripCmd::SetStripInsertSetting { strip, slot: 0, setting: 0, value } => {
                let amount = value.min(127) as u8;
                match (kb(strip), style(strip)) {
                    (Some(part), _) => PartsCmd::SetKeyboardInsertAmount { part, amount }.into(),
                    (_, Some(part)) => FxCmd::SetPartInsertAmount { part, amount }.into(),
                    _ => return None,
                }
            }
            StripCmd::SetSendKind { send, ref kind } if (send as usize) < STYLE_SENDS => {
                let (block, effect) = fx_type(kind).filter(|(b, _)| *b == block_of(send as usize))?;
                FxCmd::SetEffectType { block, effect }.into()
            }
            StripCmd::SetSendParam { send, param, value } if (send as usize) < STYLE_SENDS => {
                let block = block_of(send as usize);
                let param = crate::api::FxParam::of_block(block.index()).nth(param as usize)?;
                FxCmd::SetEffectParam { block, param, value }.into()
            }
            StripCmd::SetSendReturn { send, level } if (send as usize) < STYLE_SENDS => FxCmd::SetEffectReturn { block: block_of(send as usize), level }.into(),
            _ => return None,
        })
    }
}

impl Strips {
    fn strip(&mut self, strip: u8) -> Result<&mut StripSettings, String> {
        self.strips.get_mut(strip as usize).ok_or_else(|| format!("no strip {strip} (0-11)"))
    }

    /// How many send effects there are: 3 and the added ones.
    pub fn sends(&self) -> usize {
        STYLE_SENDS + self.added.len()
    }

    fn slot(&mut self, strip: u8, slot: u8) -> Result<&mut InsertSlot, String> {
        self.strip(strip)?.inserts.get_mut(slot as usize).ok_or_else(|| format!("no insert slot {slot} (0-1)"))
    }

    fn send_slot(&mut self, send: u8) -> Result<&mut SendSlot, String> {
        let s = send as usize;
        if s < STYLE_SENDS {
            return Ok(&mut self.style_sends[s]);
        }
        let n = self.sends();
        self.added.get_mut(s - STYLE_SENDS).ok_or_else(|| format!("no send {} (there are {n})", s + 1))
    }

    /// Play a command here. Err: why it was refused (nothing changed).
    pub fn apply(&mut self, c: &StripCmd) -> Result<(), String> {
        match c {
            StripCmd::SetStripEq { strip, eq } => self.strip(*strip)?.eq = eq.clamped(),
            StripCmd::SetStripCompressorOn { strip, on } => self.strip(*strip)?.comp.on = *on,
            StripCmd::SetStripCompressorPreset { strip, preset } => {
                let s = self.strip(*strip)?;
                s.comp = PartComp::of(s.comp.on, *preset);
            }
            StripCmd::SetStripCompressorParam { strip, param, value } => self.strip(*strip)?.comp.set(*param, *value),
            StripCmd::SetStripInsertKind { strip, slot, kind } => {
                if let InsertType::Unknown(k) = kind {
                    return Err(format!("no insert kind {k:?}"));
                }
                self.slot(*strip, *slot)?.set_kind(kind.clone());
            }
            StripCmd::SetStripInsertOn { strip, slot, on } => self.slot(*strip, *slot)?.on = *on,
            StripCmd::SetStripInsertSetting { strip, slot, setting, value } => self.slot(*strip, *slot)?.set_value(*setting as usize, *value)?,
            StripCmd::SetStripSend { strip, send, level } => {
                let n = self.sends();
                if *send as usize >= n {
                    return Err(format!("no send {} (there are {n})", *send as usize + 1));
                }
                self.strip(*strip)?.sends[*send as usize] = (*level).min(127);
            }
            StripCmd::AddSend { kind } => {
                if let SendKind::Unknown(k) = kind {
                    return Err(format!("no send kind {k:?}"));
                }
                if self.sends() >= SENDS {
                    return Err(format!("there are already {SENDS} sends"));
                }
                self.added.push(SendSlot::of(kind.clone()));
            }
            StripCmd::RemoveSend { send } => {
                let s = *send as usize;
                if s < STYLE_SENDS || s >= self.sends() {
                    return Err(format!("send {} can't be removed (only added ones: 4-{})", s + 1, self.sends()));
                }
                self.added.remove(s - STYLE_SENDS);
                for st in &mut self.strips {
                    st.sends.copy_within(s + 1.., s);
                    st.sends[SENDS - 1] = 0;
                }
            }
            StripCmd::SetSendKind { send, kind } => {
                if let SendKind::Unknown(k) = kind {
                    return Err(format!("no send kind {k:?}"));
                }
                let s = *send as usize;
                if s < STYLE_SENDS && kind.bus().map(|b| b.0) != Some(s) {
                    return Err(format!("send {} plays {} types", s + 1, block_of(s).name()));
                }
                self.send_slot(*send)?.set_kind(kind.clone());
            }
            StripCmd::SetSendParam { send, param, value } => self.send_slot(*send)?.set_param(*param as usize, *value)?,
            StripCmd::SetSendReturn { send, level } => self.send_slot(*send)?.return_level = (*level).min(127),
            StripCmd::SetRackSendOverride { send, on } => {
                let o = self.overrides.get_mut(*send as usize).ok_or_else(|| format!("only sends 1-3 have an override (not {})", *send as usize + 1))?;
                *o = *on;
            }
        }
        Ok(())
    }

    /// Put the strips and the send effects into `st`: every part's `strip` and
    /// `effects.sends`. What older state already has (the fields [`StripCmd::legacy`]
    /// covers) is taken from it, so the two never disagree.
    pub fn fill(&mut self, st: &mut crate::api::AppState) {
        // Sends 1-3 from the buses.
        for (i, b) in st.effects.blocks.iter().take(STYLE_SENDS).enumerate() {
            let s = &mut self.style_sends[i];
            if let Some(kind) = SendKind::of_bus(i, b.block.type_index(b.effect)) {
                if s.kind != kind {
                    s.set_kind(kind);
                }
            }
            for (o, p) in s.params.iter_mut().zip(&b.params) {
                *o = p.value;
            }
            s.return_level = b.return_level;
        }
        st.effects.sends = self.style_sends.iter().enumerate().map(|(i, s)| SendState::of(i, s, true, self.overrides[i])).collect();
        st.effects.sends.extend(self.added.iter().enumerate().map(|(i, s)| SendState::of(STYLE_SENDS + i, s, false, true)));

        let n = self.sends();
        let state = |s: &StripSettings| StripState {
            eq: s.eq,
            comp: s.comp.into(),
            inserts: [(&s.inserts[0]).into(), (&s.inserts[1]).into()],
            sends: std::array::from_fn(|i| if i < n { s.sends[i] } else { 0 }),
        };
        // Insert 1 from an older slot: its kind, on/off and amount; the other settings
        // this one's while the kind is the same.
        let slot1 = |mine: &mut InsertSlot, kind: InsertType, on: bool, amount: u8| {
            if mine.kind != kind {
                mine.set_kind(kind);
            }
            mine.on = on;
            if !mine.kind.settings().is_empty() {
                mine.values[0] = amount as u16;
            }
        };
        for (p, kp) in st.keyboard_parts.iter_mut().enumerate().take(KEYBOARD_STRIPS) {
            let s = &mut self.strips[p];
            s.eq = kp.eq;
            s.sends[..STYLE_SENDS].copy_from_slice(&[kp.reverb, kp.chorus, kp.variation]);
            let old = InsertSlot::from_part_insert(kp.insert);
            slot1(&mut s.inserts[0], old.kind, old.on, kp.insert.amount);
            kp.strip = state(s);
        }
        for (p, sp) in st.mixer.style_parts.iter_mut().enumerate().take(8) {
            let s = &mut self.strips[KEYBOARD_STRIPS + p];
            s.sends[..STYLE_SENDS].copy_from_slice(&[sp.reverb, sp.chorus, sp.variation]);
            // The style's insert, when it has one for this part.
            if let Some(i) = st.effects.inserts.iter().find(|i| i.part as usize == p) {
                let kind = i.effect.map_or(InsertType::None, |e| InsertType::from(e.kind()));
                slot1(&mut s.inserts[0], kind, i.on, i.amount);
            }
            sp.strip = state(s);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{AppCmd, AppState, FxBlock, FxCmd, FxType, PartsCmd};

    fn state() -> AppState {
        let mut st = AppState::default();
        st.keyboard_parts = vec![crate::api::KeyboardPart::default(); 4];
        st.mixer.style_parts = vec![crate::api::StylePart::default(); 8];
        st.effects = crate::api::EffectsState::initial();
        st
    }

    #[test]
    fn strip_commands_round_trip_on_the_wire() {
        let cmds = [
            StripCmd::SetStripInsertKind { strip: 5, slot: 1, kind: InsertType::Phaser },
            StripCmd::AddSend { kind: SendKind::Plate },
            StripCmd::SetStripCompressorParam { strip: 0, param: PartCompParam::Ratio, value: 40 },
        ];
        for c in cmds {
            let j = serde_json::to_string(&c).unwrap();
            assert_eq!(serde_json::from_str::<AppCmd>(&j).unwrap(), AppCmd::Strips(c));
        }
        let j = r#"{"type":"setStripInsertKind","strip":5,"slot":1,"kind":"phaser"}"#;
        assert!(serde_json::from_str::<AppCmd>(j).is_ok());
    }

    #[test]
    fn older_commands_cover_what_they_did() {
        let c = StripCmd::SetStripSend { strip: 1, send: 0, level: 90 };
        assert_eq!(c.legacy(), vec![PartsCmd::SetPartSend { part: 1, send: crate::api::PartSend::Reverb, value: 90 }.into()]);
        let c = StripCmd::SetStripInsertOn { strip: 6, slot: 0, on: false };
        assert_eq!(c.legacy(), vec![FxCmd::SetPartInsertOn { part: 2, on: false }.into()]);
        let c = StripCmd::SetSendKind { send: 0, kind: SendKind::Plate };
        assert_eq!(c.legacy(), vec![FxCmd::SetEffectType { block: FxBlock::Reverb, effect: FxType::Plate }.into()]);
        assert!(StripCmd::SetStripInsertOn { strip: 6, slot: 1, on: true }.legacy().is_empty(), "insert 2 is new");
        assert!(StripCmd::SetStripSend { strip: 1, send: 3, level: 9 }.legacy().is_empty(), "sends 4-6 are new");
    }

    #[test]
    fn sends_are_added_and_removed_with_their_levels() {
        let mut s = Strips::default();
        assert!(s.apply(&StripCmd::SetStripSend { strip: 0, send: 3, level: 10 }).is_err(), "no send 4 yet");
        s.apply(&StripCmd::AddSend { kind: SendKind::Phaser }).unwrap();
        s.apply(&StripCmd::AddSend { kind: SendKind::Room }).unwrap();
        s.apply(&StripCmd::SetStripSend { strip: 0, send: 4, level: 77 }).unwrap();
        s.apply(&StripCmd::RemoveSend { send: 3 }).unwrap();
        assert_eq!(s.added, vec![SendSlot::of(SendKind::Room)]);
        assert_eq!(s.strips[0].sends[3], 77, "the level moved down with its send");
        assert!(s.apply(&StripCmd::RemoveSend { send: 0 }).is_err(), "sends 1-3 stay");
        assert!(s.apply(&StripCmd::AddSend { kind: SendKind::Unknown("x".into()) }).is_err());
        s.apply(&StripCmd::AddSend { kind: SendKind::Hall }).unwrap();
        s.apply(&StripCmd::AddSend { kind: SendKind::Hall }).unwrap();
        assert!(s.apply(&StripCmd::AddSend { kind: SendKind::Hall }).is_err(), "six at most");
    }

    #[test]
    fn the_state_shows_every_strip_and_send() {
        let mut s = Strips::default();
        s.apply(&StripCmd::SetStripInsertKind { strip: 11, slot: 1, kind: InsertType::Tremolo }).unwrap();
        s.apply(&StripCmd::SetStripCompressorOn { strip: 2, on: true }).unwrap();
        s.apply(&StripCmd::AddSend { kind: SendKind::Phaser }).unwrap();
        s.apply(&StripCmd::SetRackSendOverride { send: 1, on: true }).unwrap();
        let mut st = state();
        st.keyboard_parts[0].reverb = 55;
        st.keyboard_parts[0].insert = crate::api::PartInsert { effect: crate::api::InsertEffect::Rotary, on: true, amount: 99 };
        s.fill(&mut st);
        let kp = &st.keyboard_parts[0].strip;
        assert_eq!(kp.sends[0], 55, "the older send");
        assert_eq!((kp.inserts[0].kind.clone(), kp.inserts[0].settings[0].value), (InsertType::Rotary, 99), "the older insert");
        assert!(st.keyboard_parts[2].strip.comp.on);
        assert_eq!(st.mixer.style_parts[7].strip.inserts[1].name, "Tremolo");
        let sends: Vec<_> = st.effects.sends.iter().map(|s| (s.kind.clone(), s.from_style, s.set_by_rack)).collect();
        assert_eq!(sends, vec![(SendKind::Hall, true, false), (SendKind::Chorus, true, true), (SendKind::DottedEighth, true, false), (SendKind::Phaser, false, true)]);
        assert_eq!(st.effects.sends[3].params.len(), 3);
    }
}

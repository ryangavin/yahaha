//! Channel strips and send effects (the mixer rework; `api::StripCmd`). What an older
//! command covers goes through it (`StripCmd::legacy`); everything is also kept in
//! `Control::strips`, which the state shows (`Strips::fill`).
//!
//! `pump_strips` (at the end of every `pump_fx`) stores every strip into
//! `SynthControl::fx` by MIDI channel: its compressor and insert slots (`fx.strips`,
//! insert 1 as it plays: a keyboard part's own slot, a Style part's style insert or the
//! kind the player chose), its levels to sends 4-6 (`fx.strip_send`), sends 4-6
//! themselves (`fx.sends`), and the Style parts' EQ (`fx.style_eq`). The synth must call
//! `fx.set_style_eq_rate` when it starts, for the Style parts' EQ coefficients, and play
//! `fx.style_eq` on channels 9-16.
//! The rack send override (`setRackSendOverride`) keeps a style change off sends 1-3.

use super::Control;
use crate::api::{CmdError, FxBlock, KEYBOARD_STRIPS, STYLE_SENDS, StripCmd, VoiceSettings};
use crate::fx::{InsertSlot, SendSlot};
use std::sync::atomic::Ordering::Relaxed;

impl Control {
    pub(super) fn strips_cmd(&mut self, c: StripCmd) -> Result<(), CmdError> {
        // A command the strips refuse (for example a setting on an empty or setting-less
        // insert) changes nothing: the older commands don't run either.
        if let Err(e) = self.strips.get_mut().check(&c) {
            return self.fail(e);
        }
        // The older commands run first.
        for old in c.legacy() {
            self.apply(old)?;
        }
        if let Err(e) = self.strips.get_mut().apply(&c) {
            return self.fail(e);
        }
        match c {
            // A Style part's insert 1: the player's kind over the style's, its amount the
            // kind's own again.
            StripCmd::SetStripInsertKind { strip, slot: 0, ref kind } if (KEYBOARD_STRIPS..KEYBOARD_STRIPS + 8).contains(&(strip as usize)) => {
                let p = strip as usize - KEYBOARD_STRIPS;
                self.fx.insert_kind[p] = Some(kind.kind());
                self.fx.insert_amount[p] = None;
            }
            StripCmd::SetRackSendOverride { send, on } => self.set_send_override(send as usize, on),
            _ => {}
        }
        // A keyboard strip's voice settings are its part's: the engine sends them.
        let kp = &self.shared.parts;
        if let Some((p, tone, xg)) = c.part_tone(|p, nn| kp.xg(p).iter().any(|&(hh, n, _)| (hh, n) == (0x08, nn))) {
            kp.set_tone(p, tone, xg);
        }
        self.pump_fx();
        Ok(())
    }

    /// Store every strip's settings for the synth, by MIDI channel: its compressor, insert
    /// 1's settings 2-4 and insert 2 (`fx.strips`), its levels to sends 4-6
    /// (`fx.strip_send`); sends 4-6 themselves (`fx.sends`); the Style parts' EQ
    /// (`fx.style_eq`). Insert 1's kind, on/off and amount go as before: a keyboard part's
    /// own slot, a Style part's `fx.insert` (`pump_fx`). Runs on every pump: plain stores.
    pub(super) fn pump_strips(&self) {
        let Some(synth) = self.synth.as_ref() else { return };
        let fx = &synth.control.fx;
        let cell = self.strips.borrow();
        let sends = cell.sends();
        for (i, s) in cell.strips.iter().enumerate() {
            let (ch, slot1) = if i < KEYBOARD_STRIPS {
                (crate::parts::CHANNEL[i] as usize, self.keyboard_insert1(i))
            } else {
                (8 + i - KEYBOARD_STRIPS, self.style_insert1(i - KEYBOARD_STRIPS))
            };
            fx.strips.set_comp(ch, &s.comp);
            fx.strips.set_first(ch, &slot1);
            fx.strips.set_second(ch, &s.inserts[1]);
            for (j, a) in fx.strip_send[ch].iter().enumerate() {
                let send = STYLE_SENDS + j;
                a.store(if send < sends { s.sends[send].min(127) } else { 0 }, Relaxed);
            }
        }
        for (i, c) in fx.sends.iter().enumerate() {
            match cell.added.get(i) {
                Some(s) => c.set(s),
                None => c.clear(),
            }
        }
        for (p, s) in cell.strips[KEYBOARD_STRIPS..].iter().enumerate() {
            fx.set_style_eq(p, s.eq);
        }
    }

    /// Every strip into `st` (`Strips::fill`), the keyboard strips' voice settings read
    /// from their parts first.
    pub(super) fn fill_strips(&self, st: &mut crate::api::AppState) {
        let mut strips = self.strips.borrow_mut();
        let kp = &self.shared.parts;
        for p in 0..KEYBOARD_STRIPS {
            strips.voice(p, VoiceSettings::of(kp.tone(p), &kp.xg(p)));
        }
        strips.fill(st);
    }

    /// Send effect `i` (0-2, a bus) as it plays now: its block's type, parameters and
    /// return level. A block type with no send kind (none today) keeps the kind the strips
    /// last showed, and says so on stderr: never a panic on the control thread.
    pub(super) fn style_send_slot(&self, i: usize) -> SendSlot {
        let block = FxBlock::ALL[i];
        let kind = match crate::fx::SendKind::of_bus(i, block.type_index(self.fx.effect[i])) {
            Some(k) => k,
            None => {
                let shown = self.strips.borrow().style_sends[i].kind.clone();
                eprintln!("send {}: {:?} has no send kind; keeping {}", i + 1, self.fx.effect[i], shown.name());
                shown
            }
        };
        let mut params = [0; crate::fx::SEND_PARAMS];
        for (o, p) in params.iter_mut().zip(crate::fx::Param::of_block(i)) {
            *o = self.fx.params[p.index()];
        }
        SendSlot { kind, params, return_level: self.fx.returns[i] }
    }

    /// Send effect `i` (0-2) plays `s` (the rack's override): its block's type, parameters
    /// and return level, without taking the block off Follow Style. Err: `s` is not one of
    /// that block's kinds (nothing changed).
    pub(super) fn set_style_send(&mut self, i: usize, s: &SendSlot) -> Result<(), String> {
        let Some((b, index)) = s.kind.bus().filter(|(b, _)| *b == i) else {
            return Err(format!("send {} can't play {}", i + 1, s.kind.name()));
        };
        let Some(&effect) = FxBlock::ALL[b].types().get(index as usize) else {
            return Err(format!("send {} can't play {}", i + 1, s.kind.name()));
        };
        self.fx.effect[i] = effect;
        for (p, &v) in crate::fx::Param::of_block(i).zip(&s.params) {
            self.fx.params[p.index()] = p.clamp(v);
        }
        self.fx.returns[i] = s.return_level.min(127);
        Ok(())
    }

    /// The live rack's override of send effect `i` (0-2) on or off, in both places that
    /// keep it. Turned off, the block goes back to the style's type and parameters when it
    /// follows the style (a block the player took off Follow Style keeps what it has).
    pub(super) fn set_send_override(&mut self, i: usize, on: bool) {
        let was = self.fx.rack_override[i];
        self.fx.rack_override[i] = on;
        self.strips.get_mut().overrides[i] = on;
        if was && !on && self.fx.follow[i] {
            let effects = self.info.effects.clone();
            self.fx.apply_style(&effects, Some(FxBlock::ALL[i]));
        }
    }

    /// Keyboard part `p`'s insert 1 as it plays: the part's own slot (kind, on/off,
    /// amount), with the strip's other settings while the strip has the same kind (else
    /// that kind's defaults).
    pub(super) fn keyboard_insert1(&self, p: usize) -> InsertSlot {
        let mut slot = InsertSlot::from_part_insert(self.shared.parts.insert(p));
        let cell = &self.strips.borrow().strips[p].inserts[0];
        if cell.kind == slot.kind {
            slot.values[1..].copy_from_slice(&cell.values[1..]);
        }
        slot
    }
}

#[cfg(test)]
#[path = "strips_tests.rs"]
mod strips_tests;

#[cfg(test)]
mod tests {
    use crate::api::{InsertType, SendKind, StripCmd};
    use crate::session::testing::session;

    #[test]
    fn strip_commands_show_in_the_state() {
        let s = session();
        s.send(StripCmd::SetStripInsertKind { strip: 4, slot: 1, kind: InsertType::Phaser }).unwrap();
        s.send(StripCmd::AddSend { kind: SendKind::Plate }).unwrap();
        s.send(StripCmd::SetStripSend { strip: 0, send: 3, level: 88 }).unwrap();
        s.send(StripCmd::SetStripSend { strip: 1, send: 1, level: 70 }).unwrap();
        let st = s.state();
        assert_eq!(st.mixer.style_parts[0].strip.inserts[1].kind, InsertType::Phaser);
        assert_eq!(st.effects.sends.len(), 4);
        assert_eq!(st.keyboard_parts[0].strip.sends[3], 88);
        assert_eq!(st.keyboard_parts[1].chorus, 70, "send 2 is the part's chorus");
        assert_eq!(st.keyboard_parts[1].strip.sends[1], 70);
        assert!(s.send(StripCmd::RemoveSend { send: 1 }).is_err());
    }

    /// A keyboard strip's voice settings are its part's: the commands send them on the
    /// part's channel, and what an OTS or a rack set shows on the strip.
    #[test]
    fn voice_settings_play_on_the_part() {
        use crate::api::{Portamento, ToneControl};
        let s = session();
        s.take_output();
        s.send(StripCmd::SetStripTone { strip: 1, control: ToneControl::Cutoff, value: 20 }).unwrap();
        s.send(StripCmd::SetStripPortamento { strip: 1, on: true, time: 33 }).unwrap();
        s.send(StripCmd::SetStripMono { strip: 1, on: true }).unwrap();
        s.advance(1_000_000);
        let ch = crate::parts::CHANNEL[1];
        let out = s.take_output();
        assert!(out.contains(&[0xB0 | ch, 74, 20]), "cutoff on Right 2's channel: {out:?}");
        assert!(out.contains(&[0xB0 | ch, 65, 127]) && out.contains(&[0xB0 | ch, 5, 33]), "portamento: {out:?}");
        // Mono is the XG part's Mono/Poly, sent as SysEx (`Parts::send_tone`).
        assert!(s.inner.lock().shared.parts.xg(1).contains(&(0x08, 0x05, 0)));
        let st = s.state().keyboard_parts[1].strip.clone();
        assert_eq!((st.tone.cutoff, st.mono, st.portamento), (20, true, Portamento { on: true, time: 33 }));
        // Set the older way (an OTS, a rack): the strip shows it.
        let mut t = [None; crate::parts::TONE];
        t[crate::parts::RELEASE] = Some(100);
        s.inner.lock().shared.parts.set_tone(2, t, []);
        s.advance(1_000_000);
        assert_eq!(s.state().keyboard_parts[2].strip.tone.release, 100);
        // An OTS that sets only the XG parameter: it is sent last, so it plays and shows.
        s.inner.lock().shared.parts.set_tone(2, [None; crate::parts::TONE], [(0x08, ToneControl::Release.xg(), 30)]);
        s.advance(1_000_000);
        assert_eq!(s.state().keyboard_parts[2].strip.tone.release, 30);
        assert!(s.send(StripCmd::SetStripMono { strip: 6, on: true }).is_err(), "a Style strip has no voice settings");
    }

    #[test]
    fn none_empties_a_keyboard_insert() {
        let s = session();
        s.send(StripCmd::SetStripInsertKind { strip: 0, slot: 0, kind: InsertType::Rotary }).unwrap();
        s.send(StripCmd::SetStripInsertKind { strip: 0, slot: 0, kind: InsertType::None }).unwrap();
        assert_eq!(s.state().keyboard_parts[0].strip.inserts[0].kind, InsertType::None);
    }

    /// Every type a bus plays is a send kind, so `style_send_slot` never needs its
    /// fallback today.
    #[test]
    fn every_bus_type_is_a_send_kind() {
        for (i, b) in crate::api::FxBlock::ALL.iter().enumerate().take(3) {
            for (n, t) in b.types().iter().enumerate() {
                assert!(SendKind::of_bus(i, n as u8).is_some(), "{b:?} {t:?}");
                assert_eq!(b.type_index(*t), n as u8);
            }
        }
    }

    /// A refused setting on an empty insert 1 changes nothing: the older amount command
    /// doesn't run, so the slot doesn't come back as an off distortion.
    #[test]
    fn a_setting_on_an_empty_insert_is_refused() {
        let s = session();
        let before = s.state().keyboard_parts[0].insert;
        assert!(s.send(StripCmd::SetStripInsertSetting { strip: 0, slot: 0, setting: 0, value: 10 }).is_err());
        let st = s.state();
        assert_eq!(st.keyboard_parts[0].strip.inserts[0].kind, InsertType::None);
        assert_eq!(st.keyboard_parts[0].insert, before, "the older slot is untouched");
        // Emptied by the player, too.
        s.send(StripCmd::SetStripInsertKind { strip: 1, slot: 0, kind: InsertType::Tremolo }).unwrap();
        s.send(StripCmd::SetStripInsertKind { strip: 1, slot: 0, kind: InsertType::None }).unwrap();
        assert!(s.send(StripCmd::SetStripInsertSetting { strip: 1, slot: 0, setting: 0, value: 99 }).is_err());
        assert_eq!(s.state().keyboard_parts[1].strip.inserts[0].kind, InsertType::None);
    }
}

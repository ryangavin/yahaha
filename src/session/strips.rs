//! Channel strips and send effects (the mixer rework; `api::StripCmd`). What an older
//! command covers goes through it (`StripCmd::legacy`); everything is also kept in
//! `Control::strips`, which the state shows (`Strips::fill`).
//!
//! Stub: nothing new reaches the synth yet (a strip's compressor, insert 2, insert
//! settings past the first, sends 4-6 and their levels, the rack send override). The
//! mixer rework's session lane pumps them.

use super::Control;
use crate::api::{CmdError, StripCmd, VoiceSettings, KEYBOARD_STRIPS};

impl Control {
    pub(super) fn strips_cmd(&mut self, c: StripCmd) -> Result<(), CmdError> {
        // The older commands run first; a strip error is still returned after them
        // (for example a setting on an empty or setting-less insert).
        for old in c.legacy() {
            self.apply(old)?;
        }
        if let Err(e) = self.strips.get_mut().apply(&c) {
            return self.fail(e);
        }
        // A keyboard strip's voice settings are its part's: the engine sends them.
        let kp = &self.shared.parts;
        if let Some((p, tone, xg)) = c.part_tone(|p, nn| kp.xg(p).iter().any(|&(hh, n, _)| (hh, n) == (0x08, nn))) {
            kp.set_tone(p, tone, xg);
        }
        Ok(())
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
}

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
        assert!(s.send(StripCmd::SetStripMono { strip: 6, on: true }).is_err(), "a Style strip has no voice settings");
    }

    #[test]
    fn none_empties_a_keyboard_insert() {
        let s = session();
        s.send(StripCmd::SetStripInsertKind { strip: 0, slot: 0, kind: InsertType::Rotary }).unwrap();
        s.send(StripCmd::SetStripInsertKind { strip: 0, slot: 0, kind: InsertType::None }).unwrap();
        assert_eq!(s.state().keyboard_parts[0].strip.inserts[0].kind, InsertType::None);
    }

    #[test]
    fn a_setting_on_an_empty_insert_is_refused() {
        let s = session();
        assert!(s.send(StripCmd::SetStripInsertSetting { strip: 0, slot: 0, setting: 0, value: 10 }).is_err());
    }
}

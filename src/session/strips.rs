//! Channel strips and send effects (the mixer rework; `api::StripCmd`). What an older
//! command covers goes through it (`StripCmd::legacy`); everything is also kept in
//! `Control::strips`, which the state shows (`Strips::fill`).
//!
//! Stub: nothing new reaches the synth yet (a strip's compressor, insert 2, insert
//! settings past the first, sends 4-6 and their levels, the rack send override). The
//! mixer rework's session lane pumps them.

use super::Control;
use crate::api::{CmdError, StripCmd};

impl Control {
    pub(super) fn strips_cmd(&mut self, c: StripCmd) -> Result<(), CmdError> {
        // The older commands run first; a strip error is still returned after them
        // (for example a setting on an empty or setting-less insert).
        for old in c.legacy() {
            self.apply(old)?;
        }
        match self.strips.get_mut().apply(&c) {
            Err(e) => self.fail(e),
            Ok(()) => Ok(()),
        }
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

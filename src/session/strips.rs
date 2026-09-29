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
        let legacy = c.legacy();
        let covered = legacy.is_some();
        if let Some(old) = legacy {
            self.apply(old)?;
        }
        match self.strips.get_mut().apply(&c) {
            // What the older command took, it decided.
            Err(_) if covered => Ok(()),
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
}

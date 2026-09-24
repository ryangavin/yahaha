//! Style Setting > Change Behavior: the control side keeps the settings (for the state)
//! and hands the engine a copy, which applies them at the next style change.

use super::Control;
use crate::api::{ChangeRuleMode, CmdError, StyleChangeCmd};
use crate::live::Cmd;

impl Control {
    pub(super) fn style_change_cmd(&mut self, c: StyleChangeCmd) -> Result<(), CmdError> {
        let mut s = self.style_change;
        let flip = |cur: ChangeRuleMode, to: ChangeRuleMode| if cur == ChangeRuleMode::Reset { to } else { ChangeRuleMode::Reset };
        match c {
            StyleChangeCmd::SetTempoChange { rule } => s.tempo = rule,
            StyleChangeCmd::SetPartsChange { rule } => s.parts = rule,
            StyleChangeCmd::SetSectionSet { section } => s.section_set = section.map(|m| m.min(3)),
            StyleChangeCmd::ToggleStyleTempoLock => s.tempo = flip(s.tempo, ChangeRuleMode::Lock),
            StyleChangeCmd::ToggleStyleTempoHold => s.tempo = flip(s.tempo, ChangeRuleMode::Hold),
        }
        self.engine_cmd(Cmd::ChangeRules(s.into()))?;
        self.style_change = s;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::api::*;
    use crate::session::{Options, Port, Session};
    use std::path::{Path, PathBuf};

    const MS: u64 = 1_000_000;

    fn corpus(name: &str) -> Option<PathBuf> {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2").join(name);
        if !p.exists() {
            eprintln!("corpus missing; skipping");
            return None;
        }
        Some(p)
    }

    fn offline(name: &str) -> Option<Session> {
        Some(Session::offline(Options { paths: vec![corpus(name)?], ..Options::default() }).unwrap())
    }

    fn chord_c(s: &Session) {
        for n in [36, 40, 43] {
            s.midi_in(Port::Keys, &[0x90, n, 100]);
        }
    }

    /// Advance in 10 ms steps until `f` holds (at most `max_ms`).
    fn until(s: &Session, max_ms: u64, f: impl Fn(&AppState) -> bool) -> bool {
        for _ in 0..max_ms / 10 {
            if f(&s.state()) {
                return true;
            }
            s.advance(10 * MS);
        }
        false
    }

    #[test]
    fn ots_recall_turns_sync_start_on() {
        let Some(s) = offline("SlowWalker.T552.sty") else { return };
        s.send(TransportCmd::ToggleSyncStart).unwrap();
        assert!(!s.state().transport.sync_start);
        s.send(OtsCmd::RecallOts { index: 1 }).unwrap();
        assert!(s.state().transport.sync_start, "OTS turns Sync Start on");
        chord_c(&s);
        s.advance(50 * MS);
        assert!(s.state().transport.running, "the next chord starts the style");
        s.send(OtsCmd::RecallOts { index: 0 }).unwrap();
        s.advance(10 * MS);
        let st = s.state();
        assert!(st.transport.running && !st.transport.sync_start, "playing: the recall does not stop the band");
    }

    #[test]
    fn ots_link_timing_at_main_section_change() {
        let Some(s) = offline("SlowWalker.T552.sty") else { return };
        assert_eq!(s.state().ots.link_timing, OtsLinkTiming::Immediate);
        s.send(OtsCmd::SetOtsLinkTiming { timing: OtsLinkTiming::MainChange }).unwrap();
        s.send(OtsCmd::SetOtsLink { on: true }).unwrap();
        assert_eq!(s.state().ots.link_timing, OtsLinkTiming::MainChange);
        assert_eq!(s.state().ots.applied, 1, "stopped: Main A's OTS at once");
        chord_c(&s);
        s.advance(500 * MS);
        s.send(TransportCmd::Main { index: 1 }).unwrap();
        s.advance(10 * MS);
        let st = s.state();
        assert_eq!((st.transport.main, st.ots.applied), (1, 1), "pressed: not yet");
        // Auto Fill: B's fill first, still OTS 1; OTS 2 when Main B starts.
        assert!(until(&s, 4000, |st| st.transport.section.as_deref() == Some("Fill In BB")), "the fill");
        assert_eq!(s.state().ots.applied, 1);
        assert!(until(&s, 4000, |st| st.transport.section.as_deref() == Some("Main B")), "Main B");
        assert_eq!(s.state().ots.applied, 2, "at the Main section change");
        // Immediate: as the Main is pressed.
        s.send(OtsCmd::SetOtsLinkTiming { timing: OtsLinkTiming::Immediate }).unwrap();
        s.send(TransportCmd::Main { index: 2 }).unwrap();
        s.advance(10 * MS);
        let st = s.state();
        assert_eq!(st.ots.applied, 3);
        assert_ne!(st.transport.section.as_deref(), Some("Main C"));
    }

    #[test]
    fn change_behavior_commands_and_state() {
        let (Some(s), Some(other)) = (offline("SlowWalker.T552.sty"), corpus("AustinCityBlues.S930.STY")) else { return };
        assert_eq!(
            s.state().style_change,
            StyleChangeState { tempo: ChangeRuleMode::Hold, parts: ChangeRuleMode::Hold, section_set: None }
        );
        s.send(StyleChangeCmd::ToggleStyleTempoLock).unwrap();
        assert_eq!(s.state().style_change.tempo, ChangeRuleMode::Reset, "Hold -> Reset");
        s.send(StyleChangeCmd::ToggleStyleTempoLock).unwrap();
        assert_eq!(s.state().style_change.tempo, ChangeRuleMode::Lock);
        s.send(StyleChangeCmd::ToggleStyleTempoHold).unwrap();
        assert_eq!(s.state().style_change.tempo, ChangeRuleMode::Reset);
        s.send(StyleChangeCmd::ToggleStyleTempoHold).unwrap();
        assert_eq!(s.state().style_change.tempo, ChangeRuleMode::Hold);
        s.send(StyleChangeCmd::SetTempoChange { rule: ChangeRuleMode::Lock }).unwrap();
        s.send(StyleChangeCmd::SetPartsChange { rule: ChangeRuleMode::Lock }).unwrap();
        s.send(StyleChangeCmd::SetSectionSet { section: Some(7) }).unwrap();
        assert_eq!(s.state().style_change.section_set, Some(3), "clamped to Main D");
        // Locked: another style keeps the tempo and the muted part; Section Set picks D.
        s.send(TransportCmd::TempoUp).unwrap();
        s.send(MixerCmd::ToggleStylePart { part: 2 }).unwrap();
        let tempo = s.state().transport.tempo;
        s.send(LibraryCmd::LoadStylePath { path: other.display().to_string() }).unwrap();
        s.advance(10 * MS);
        let st = s.state();
        assert!(st.style.path.contains("Austin"), "{}", st.style.path);
        assert_eq!(st.transport.tempo, tempo);
        assert!(!st.mixer.style_parts[2].on, "Part On/Off Lock keeps the mute");
        assert_eq!(st.transport.main, 3);
    }
}

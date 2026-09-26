//! Knob Assign pages (#197): the session keeps the page and the knobs' own positions
//! (`knobs::Knobs`), and runs a turn as the command of the knob's function.

use super::Control;
use crate::api::{CmdError, KnobsCmd, KnobsState};
use crate::knobs::Now;

impl Control {
    pub(super) fn knobs_cmd(&mut self, c: KnobsCmd) -> Result<(), CmdError> {
        match c {
            KnobsCmd::SetKnobPage { page } => self.knobs.set_page(page),
            KnobsCmd::StepKnobPage { delta } => self.knobs.set_page(self.knobs.page.step(delta)),
            KnobsCmd::TurnKnob { knob, delta } => {
                let now = self.knobs_now();
                if let Some(cmd) = self.knobs.turn(knob, delta, &now) {
                    return self.apply(cmd);
                }
            }
        }
        Ok(())
    }

    /// The values the knobs turn from.
    pub(super) fn knobs_now(&self) -> Now {
        let parts = &self.shared.parts;
        Now {
            dynamics: self.snap.dynamics,
            retrigger: self.snap.retrigger,
            retrigger_rate: self.style_settings.retrigger_rate,
            bpm: self.snap.bpm,
            part_volume: [0, 1, 2, 3].map(|p| parts.volume(p)),
            harmony_volume: self.harmony_arp.harmony.volume,
            metronome_volume: self.metronome.volume,
            part_fx: [0, 1, 2, 3].map(|p| parts.fx(p)),
            fx_return: self.fx.returns,
        }
    }

    pub(super) fn knobs_state(&self) -> KnobsState {
        self.knobs.state(&self.knobs_now())
    }
}

#[cfg(test)]
mod tests {
    use crate::api::*;
    use crate::knobs::KnobPage;
    use crate::session::{Options, Session};

    fn session() -> Option<Session> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
        p.exists().then(|| Session::offline(Options { paths: vec![p], ..Options::default() }).unwrap())
    }

    /// A turn runs its function's command from the value in effect, and the state shows
    /// the page and the new value.
    #[test]
    fn a_knob_turn_changes_its_function_and_shows() {
        let Some(s) = session() else { return };
        let st = s.state();
        assert_eq!(st.knobs.page, KnobPage::Style);
        assert_eq!(st.knobs.knobs.len(), 8);
        assert_eq!((st.knobs.knobs[0].short.as_str(), st.knobs.knobs[0].value.as_str()), ("DynCtrl", "64"));
        s.send(KnobsCmd::TurnKnob { knob: 0, delta: 5 }).unwrap();
        assert_eq!(s.state().dynamics.level, 74);
        assert_eq!(s.state().knobs.knobs[0].level, Some(74));
        // Track Mute A fully left: only Rhythm 2 plays.
        s.send(KnobsCmd::TurnKnob { knob: 3, delta: -40 }).unwrap();
        s.send(KnobsCmd::TurnKnob { knob: 7, delta: 3 }).unwrap();
        let st = s.state();
        assert_eq!(st.knobs.knobs[3].value, "1 of 8");
        assert_eq!(st.knobs.knobs[7].value, format!("{} BPM", st.transport.tempo.round() as i32));
        s.send(KnobsCmd::StepKnobPage { delta: 1 }).unwrap();
        s.send(KnobsCmd::TurnKnob { knob: 0, delta: -10 }).unwrap();
        let st = s.state();
        assert_eq!((st.knobs.page_name.as_str(), st.knobs.page_number, st.knobs.page_count), ("Parts", 2, 4));
        assert_eq!(st.knobs.knobs[0].value, st.keyboard_parts[0].volume.to_string());
        s.send(KnobsCmd::SetKnobPage { page: KnobPage::Style }).unwrap();
        assert_eq!(s.state().knobs.page, KnobPage::Style);
    }
}

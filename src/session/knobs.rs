//! Knob Assign pages (#197): the session keeps the page and the knobs' own positions
//! (`knobs::Knobs`), and runs a turn as the command of the knob's function.

use super::Control;
use crate::api::{CmdError, KnobsCmd, KnobsState};
use crate::fx::InsertSlot;
use crate::knobs::{InsertNow, Now, StripNow};

impl Control {
    pub(super) fn knobs_cmd(&mut self, c: KnobsCmd) -> Result<(), CmdError> {
        match c {
            KnobsCmd::SetKnobPage { page } => self.knobs.set_page(page),
            KnobsCmd::StepKnobPage { delta } => self.knobs.set_page(self.knobs.page.step(delta)),
            KnobsCmd::TurnKnob { knob, delta } => {
                let (now, strips) = (self.knobs_now(), self.strip_now());
                if let Some(cmd) = self.knobs.turn_at(knob, delta, &now, &strips) {
                    return self.apply(cmd);
                }
            }
            KnobsCmd::ResetKnob { knob } => {
                let (now, strips) = (self.knobs_now(), self.strip_now());
                if let Some(cmd) = self.knobs.reset_at(knob, &now, &strips) {
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
            swing: self.style_settings.swing,
            bpm: self.snap.bpm,
            part_volume: [0, 1, 2, 3].map(|p| parts.volume(p)),
            harmony_volume: self.harmony_arp.harmony.volume,
            metronome_volume: self.metronome.volume,
            part_fx: [0, 1, 2, 3].map(|p| parts.fx(p)),
            fx_return: self.fx.returns,
            fx_params: self.fx.params,
            fx_defaults: {
                let mut d = crate::fx::default_params();
                for b in crate::api::FxBlock::ALL {
                    crate::fx::type_defaults(b.index(), b.type_index(self.fx.effect[b.index()]), &mut d);
                }
                d
            },
            harmony_arp: self.harmony_arp.on,
            split: self.shared.split.load(std::sync::atomic::Ordering::Relaxed),
        }
    }

    /// The keyboard parts' strips the knobs and faders turn from: the strips as kept
    /// (`Control::strips`), with insert 1 as the part's older insert says (its kind, on/off
    /// and amount win, as `Strips::fill` makes them), and the rotary speed.
    pub(super) fn strip_now(&self) -> StripNow {
        let strips = self.strips.borrow();
        let parts = &self.shared.parts;
        let insert = |i: &InsertSlot| InsertNow { on: i.on, values: i.values, specs: i.kind.settings() };
        StripNow {
            inserts: [0, 1, 2, 3].map(|p| {
                let s = &strips.strips[p];
                let old = InsertSlot::from_part_insert(parts.insert(p));
                let mut first = s.inserts[0].clone();
                if first.kind != old.kind {
                    first.set_kind(old.kind);
                }
                first.on = old.on;
                if !first.kind.settings().is_empty() {
                    first.values[0] = old.values[0];
                }
                [insert(&first), insert(&s.inserts[1])]
            }),
            sends: [0, 1, 2, 3].map(|p| strips.strips[p].sends),
            send_count: strips.sends() as u8,
            rotary_fast: self.fx.rotary_fast,
        }
    }

    pub(super) fn knobs_state(&self) -> KnobsState {
        self.knobs.state_at(&self.knobs_now(), &self.strip_now())
    }
}

#[cfg(test)]
mod tests {
    use crate::api::*;
    use crate::knobs::KnobPage;
    use crate::session::testing::session;

    /// A turn runs its function's command from the value in effect, and the state shows
    /// the page and the new value.
    #[test]
    fn a_knob_turn_changes_its_function_and_shows() {
        let s = session();
        let st = s.state();
        assert_eq!(st.knobs.page, KnobPage::Style);
        assert_eq!(st.knobs.knobs.len(), 8);
        assert_eq!((st.knobs.knobs[0].short.as_str(), st.knobs.knobs[0].value.as_str()), ("DynCtrl", "127"));
        s.send(KnobsCmd::TurnKnob { knob: 0, delta: -5 }).unwrap();
        assert_eq!(s.state().dynamics.level, 117);
        assert_eq!(s.state().knobs.knobs[0].level, Some(117));
        // Track Mute A fully left: only Rhythm 2 plays.
        s.send(KnobsCmd::TurnKnob { knob: 3, delta: -40 }).unwrap();
        s.send(KnobsCmd::TurnKnob { knob: 7, delta: 3 }).unwrap();
        let st = s.state();
        assert_eq!(st.knobs.knobs[3].value, "1 of 8");
        assert_eq!(st.knobs.knobs[7].value, format!("{} BPM", st.transport.tempo.round() as i32));
        s.send(KnobsCmd::StepKnobPage { delta: 1 }).unwrap();
        s.send(KnobsCmd::TurnKnob { knob: 0, delta: -10 }).unwrap();
        let st = s.state();
        assert_eq!((st.knobs.page_name.as_str(), st.knobs.page_number, st.knobs.page_count), ("Rack", 2, 6));
        assert_eq!(st.knobs.knobs[0].value, st.keyboard_parts[0].volume.to_string());
        s.send(KnobsCmd::SetKnobPage { page: KnobPage::Style }).unwrap();
        assert_eq!(s.state().knobs.page, KnobPage::Style);
    }
}

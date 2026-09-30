//! Knob Assign pages (#197): the session keeps the page and the knobs' own positions
//! (`knobs::Knobs`), and runs a turn as the command of the knob's function.
//!
//! Swap mode (docs/eyes-free.md): while a Panel part button is held with a knob turned
//! (`Layer::Swap`), the knobs are that part's: knob 1 its sound by number (`swapSound`),
//! knobs 2-8 its mix (`swap_map`). The state's knobs show them, and the app's knob turns
//! go to them, until the button is released.

use super::display::SWAP_LABELS;
use super::sound_library::number_of;
use super::Control;
use crate::api::{CmdError, KnobState, KnobsCmd, KnobsState};
use crate::fx::InsertSlot;
use crate::knobs::{InsertNow, KnobPage, Knobs, Now, StripNow};
use crate::launchkey::Layer;
use crate::parts;
use crate::racks::{ControlMap, ControlTarget};

/// Swap mode's knobs for keyboard part `part`, as a controller map (knob 1, the sound, is
/// `swapSound`'s, so none here): the part's own functions from the knob pages, level (the
/// Rack page), pan (the Pan page) and its reverb, chorus and delay sends (the effect
/// pages), then its insert 1's first setting (its amount) and its level to send 4.
fn swap_map(part: u8) -> ControlMap {
    use ControlTarget as T;
    ControlMap {
        faders: Default::default(),
        knobs: [
            T::None,
            T::PartLevel { part },
            T::PartPan { part },
            T::PartReverb { part },
            T::PartChorus { part },
            T::PartDelay { part },
            T::PartInsertSetting { part, slot: 0, setting: 0 },
            T::PartSend { part, send: 3 },
        ],
    }
}

/// Swap mode's knobs for keyboard part `part`, turned and read as the Rack page's are.
/// Kept fresh for each turn: none of its functions carries steps over.
fn swap_knobs(part: u8) -> Knobs {
    let mut k = Knobs::default();
    k.set_page(KnobPage::Rack);
    k.set_rack(&swap_map(part));
    k
}

impl Control {
    pub(super) fn knobs_cmd(&mut self, c: KnobsCmd) -> Result<(), CmdError> {
        match c {
            KnobsCmd::SetKnobPage { page } => self.knobs.set_page(page),
            KnobsCmd::StepKnobPage { delta } => self.knobs.set_page(self.knobs.page.step(delta)),
            KnobsCmd::TurnKnob { knob, delta } => {
                if let Some(part) = self.swap_part() {
                    return self.swap_knob(part, knob, delta);
                }
                let (now, strips) = (self.knobs_now(), self.strip_now());
                if let Some(cmd) = self.knobs.turn_at(knob, delta, &now, &strips) {
                    return self.apply(cmd);
                }
            }
            KnobsCmd::ResetKnob { knob } => {
                let (now, strips) = (self.knobs_now(), self.strip_now());
                let cmd = match self.swap_part() {
                    // The sound has no default to go back to.
                    Some(_) if knob == 0 => None,
                    Some(part) => swap_knobs(part).reset_at(knob, &now, &strips),
                    None => self.knobs.reset_at(knob, &now, &strips),
                };
                if let Some(cmd) = cmd {
                    return self.apply(cmd);
                }
            }
        }
        Ok(())
    }

    /// The keyboard part in swap mode (its Panel fader button held with a knob turned).
    pub(super) fn swap_part(&self) -> Option<u8> {
        match self.shared.layer() {
            Layer::Swap { part } if (part as usize) < parts::COUNT => Some(part),
            _ => None,
        }
    }

    /// Swap mode: knob `knob` (0-7) of keyboard part `part` turned `delta` steps. Knob 1
    /// steps the part's sound by number; knobs 2-8 change its mix (`swap_map`).
    pub(super) fn swap_knob(&mut self, part: u8, knob: u8, delta: i8) -> Result<(), CmdError> {
        if knob == 0 {
            return self.swap_sound(part, delta as i32);
        }
        let (now, strips) = (self.knobs_now(), self.strip_now());
        match swap_knobs(part).turn_at(knob, delta, &now, &strips) {
            Some(cmd) => self.apply(cmd),
            None => Ok(()),
        }
    }

    /// Swap mode's knob 1 as it reads: the part's sound number and name ("23 Rhodes
    /// Soft"), or "-" when it plays no numbered sound (its GM voice, or a plugin of its
    /// own).
    fn swap_sound_text(&self, part: u8) -> String {
        let Some((id, name)) = self.part_patch(part as usize & 3) else { return "-".into() };
        match number_of(&self.sound.lib, &id) {
            Some(n) => format!("{n} {name}"),
            None => "-".into(),
        }
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

    /// The knobs as the state shows them: the Knob Assign page, or in swap mode the part's
    /// knobs (`pageName` "Swap R1", knob 1 `swapSound`), over the page the knobs go back to.
    pub(super) fn knobs_state(&self) -> KnobsState {
        let (now, strips) = (self.knobs_now(), self.strip_now());
        let Some(part) = self.swap_part() else { return self.knobs.state_at(&now, &strips) };
        let mut st = swap_knobs(part).state_at(&now, &strips);
        let page = self.knobs.page;
        st.page = page;
        st.page_number = page.index() as u8 + 1;
        st.page_name = format!("Swap {}", SWAP_LABELS[part as usize]);
        st.knobs[0] = KnobState {
            function: "swapSound".into(),
            name: format!("{} Sound", parts::NAMES[part as usize]),
            short: "Sound".into(),
            value: self.swap_sound_text(part),
            level: None,
        };
        st
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

    /// Swap mode: the state's knobs are the held part's (its sound, then its mix), the
    /// app's knob turns change that part's mix, and the release gives the page back.
    #[test]
    fn swap_mode_knobs_are_the_parts_mix() {
        use crate::session::part_sound::tests::{hold, part_number, turn, with_sounds};
        let s = with_sounds(&["Grand", "Rhodes Soft"], &[]);
        hold(&s, 2, true);
        turn(&s, 0, 1);
        let st = s.state();
        assert_eq!((st.knobs.page, st.knobs.page_name.as_str()), (KnobPage::Style, "Swap R3"));
        let (n, name) = part_number(&s, 2);
        assert_eq!(n, 1);
        let k = &st.knobs.knobs;
        assert_eq!((k[0].function.as_str(), k[0].value.clone()), ("swapSound", format!("1 {name}")));
        let f: Vec<_> = k[1..6].iter().map(|k| k.name.as_str()).collect();
        assert_eq!(f, ["Right 3 Volume", "Right 3 Pan", "Right 3 Reverb", "Right 3 Chorus", "Right 3 Delay"]);
        assert_eq!(k.len(), 8);

        // Knobs 2-6: the part's level, pan and sends; nothing else moves.
        let before = s.state();
        let r3 = &before.keyboard_parts[2];
        s.send(KnobsCmd::TurnKnob { knob: 1, delta: -5 }).unwrap();
        s.send(KnobsCmd::TurnKnob { knob: 2, delta: 3 }).unwrap();
        s.send(KnobsCmd::TurnKnob { knob: 3, delta: 4 }).unwrap();
        s.send(KnobsCmd::TurnKnob { knob: 4, delta: 5 }).unwrap();
        s.send(KnobsCmd::TurnKnob { knob: 5, delta: 6 }).unwrap();
        let st = s.state();
        let now = &st.keyboard_parts[2];
        assert_eq!(now.volume, r3.volume.saturating_sub(10));
        assert_eq!(now.pan, (r3.pan + 6).min(127));
        assert_eq!(now.reverb, (r3.reverb + 8).min(127));
        assert_eq!(now.chorus, (r3.chorus + 10).min(127));
        assert_eq!(now.variation, (r3.variation + 12).min(127));
        for p in [0, 1, 3] {
            assert_eq!(st.keyboard_parts[p].volume, before.keyboard_parts[p].volume);
        }
        assert_eq!(st.dynamics.level, before.dynamics.level, "not the Style page's knob");
        assert_eq!(st.knobs.knobs[1].value, now.volume.to_string());
        // Knob 1 from the app steps the sound too; its reset does nothing.
        s.send(KnobsCmd::TurnKnob { knob: 0, delta: 1 }).unwrap();
        assert_eq!(part_number(&s, 2).0, 2);
        s.send(KnobsCmd::ResetKnob { knob: 0 }).unwrap();
        assert_eq!(part_number(&s, 2).0, 2);
        // A reset puts the part's pan back to the centre.
        s.send(KnobsCmd::ResetKnob { knob: 2 }).unwrap();
        assert_eq!(s.state().keyboard_parts[2].pan, 64);

        hold(&s, 2, false);
        let st = s.state();
        assert_eq!((st.knobs.page_name.as_str(), st.knobs.knobs[0].function.as_str()), ("Style", "dynamics"));
        let vol = st.keyboard_parts[2].volume;
        s.send(KnobsCmd::TurnKnob { knob: 1, delta: 1 }).unwrap();
        assert_eq!(s.state().keyboard_parts[2].volume, vol, "the page's knob again");
    }
}

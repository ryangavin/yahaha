//! The mock's Multi Pads (the twin of `app/src/lib/api/mock-multipad.ts`): the engine's
//! behaviour (docs/multipad.md), closely enough for the Multi Pad panel. No audio.

use yahaha::api::{MultiPadBank, MultiPadBankEntry, MultiPadCmd, MultiPadPad, MultiPadState, MultiPadSynchroStop, PadLamp};

const ROOT: &str = "/Users/me/Styles";

/// (name, folder, pad names, repeat, chord match): the synthetic demo bank and two more.
type Bank = (&'static str, &'static str, [&'static str; 4], [bool; 4], [bool; 4]);
const BANKS: [Bank; 3] = [
    ("Demo", "Pads", ["Shaker Loop", "Rise Arp", "Bass Riff", "Brass Hit"], [true, false, true, false], [false, true, true, true]),
    ("Latin Perc", "Pads/Latin", ["Conga Loop", "Timbale Fill", "Cowbell", ""], [true, false, true, false], [false; 4]),
    ("Strings FX", "Pads", ["Swell", "Pizz Run", "Stab", "Tremolo"], [false, false, false, true], [true; 4]),
];

fn empty_pads() -> Vec<MultiPadPad> {
    (0..4u8).map(|i| MultiPadPad { index: i, channel: 5 + i, ..MultiPadPad::default() }).collect()
}

pub fn initial() -> MultiPadState {
    MultiPadState {
        bank: None,
        loading: false,
        pads: empty_pads(),
        synchro_stop: MultiPadSynchroStop::default(),
        banks: BANKS
            .iter()
            .enumerate()
            .map(|(id, b)| MultiPadBankEntry { id, name: b.0.into(), folder: b.1.into(), path: format!("{ROOT}/{}/{}.pad", b.1, b.0) })
            .collect(),
    }
}

/// Beats left of each playing pad's pass (every mock pad is one 4-beat bar).
#[derive(Default)]
pub struct MockPads {
    left: [f64; 4],
}

impl MockPads {
    fn set(&mut self, st: &mut MultiPadState, i: usize, lamp: PadLamp) {
        st.pads[i].lamp = lamp;
        if lamp == PadLamp::Playing {
            self.left[i] = 4.0;
        }
    }

    fn has(st: &MultiPadState, i: usize) -> bool {
        st.pads.get(i).is_some_and(|p| p.lamp != PadLamp::Empty)
    }

    fn start(&mut self, st: &mut MultiPadState, i: usize, running: bool) {
        if Self::has(st, i) {
            self.set(st, i, if running { PadLamp::Queued } else { PadLamp::Playing });
        }
    }

    fn each(&mut self, st: &mut MultiPadState, from: PadLamp, to: impl Fn(&MultiPadPad) -> Option<PadLamp>) {
        for i in 0..4 {
            if st.pads[i].lamp == from
                && let Some(l) = to(&st.pads[i])
            {
                self.set(st, i, l);
            }
        }
    }

    /// A pad command; an error message for a refused one.
    pub fn cmd(&mut self, st: &mut MultiPadState, c: MultiPadCmd, running: bool) -> Option<String> {
        let pad = match &c {
            MultiPadCmd::TriggerMultiPad { pad }
            | MultiPadCmd::StopMultiPad { pad }
            | MultiPadCmd::ArmMultiPad { pad }
            | MultiPadCmd::SetMultiPadRepeat { pad, .. }
            | MultiPadCmd::SetMultiPadChordMatch { pad, .. } => Some(*pad as usize),
            _ => None,
        };
        if let Some(p) = pad.filter(|&p| p > 3) {
            return Some(format!("no Multi Pad {p} (pads are 0-3)"));
        }
        let i = pad.unwrap_or(0);
        match c {
            MultiPadCmd::LoadMultiPad { id } => return self.load(st, id).err(),
            MultiPadCmd::LoadMultiPadPath { path } => match st.banks.iter().position(|b| b.path == path) {
                Some(id) => return self.load(st, id).err(),
                None => return Some(format!("{path}: not a MIDI/Multi Pad file")),
            },
            MultiPadCmd::ClearMultiPad => {
                st.bank = None;
                st.pads = empty_pads();
            }
            MultiPadCmd::TriggerMultiPad { .. } => {
                for k in 0..4 {
                    if st.pads[k].lamp == PadLamp::Armed {
                        self.start(st, k, running);
                    }
                }
                self.start(st, i, running);
            }
            MultiPadCmd::StopMultiPad { .. } => {
                if Self::has(st, i) {
                    self.set(st, i, PadLamp::Ready);
                }
            }
            MultiPadCmd::StopAllMultiPads => self.panic(st),
            MultiPadCmd::ArmMultiPad { .. } => match st.pads[i].lamp {
                PadLamp::Armed => self.set(st, i, PadLamp::Ready),
                PadLamp::Ready => self.set(st, i, PadLamp::Armed),
                _ => {}
            },
            MultiPadCmd::SetMultiPadRepeat { on, .. } => {
                if Self::has(st, i) {
                    st.pads[i].repeat = on;
                }
            }
            MultiPadCmd::SetMultiPadChordMatch { on, .. } => {
                if Self::has(st, i) {
                    st.pads[i].chord_match = on;
                }
            }
            MultiPadCmd::SetMultiPadSynchroStop { style_stop, ending } => st.synchro_stop = MultiPadSynchroStop { style_stop, ending },
        }
        None
    }

    fn load(&mut self, st: &mut MultiPadState, id: usize) -> Result<(), String> {
        let Some(b) = BANKS.get(id) else { return Err(format!("no Multi Pad bank {id}")) };
        st.bank = Some(MultiPadBank { id, name: b.0.into(), path: st.banks[id].path.clone() });
        st.pads = (0..4)
            .map(|i| {
                let has = !b.2[i].is_empty();
                MultiPadPad {
                    index: i as u8,
                    name: b.2[i].into(),
                    lamp: if has { PadLamp::Ready } else { PadLamp::Empty },
                    repeat: has && b.3[i],
                    chord_match: has && b.4[i],
                    channel: 5 + i as u8,
                }
            })
            .collect();
        Ok(())
    }

    /// The clock moved on `beats` beats.
    pub fn beats(&mut self, st: &mut MultiPadState, beats: f64) {
        for i in 0..4 {
            if st.pads[i].lamp != PadLamp::Playing {
                continue;
            }
            self.left[i] -= beats;
            if self.left[i] <= 0.0 {
                if st.pads[i].repeat {
                    self.left[i] += 4.0;
                } else {
                    st.pads[i].lamp = PadLamp::Ready;
                }
            }
        }
    }

    /// A bar line while the band plays: queued pads start.
    pub fn bar(&mut self, st: &mut MultiPadState) {
        self.each(st, PadLamp::Queued, |_| Some(PadLamp::Playing));
    }

    pub fn band_started(&mut self, st: &mut MultiPadState) {
        self.each(st, PadLamp::Armed, |_| Some(PadLamp::Playing));
    }

    pub fn chord(&mut self, st: &mut MultiPadState, running: bool) {
        let to = if running { PadLamp::Queued } else { PadLamp::Playing };
        self.each(st, PadLamp::Armed, |_| Some(to));
    }

    fn stop_repeating(&mut self, st: &mut MultiPadState) {
        for from in [PadLamp::Playing, PadLamp::Queued] {
            self.each(st, from, |p| p.repeat.then_some(PadLamp::Ready));
        }
    }

    pub fn band_stopped(&mut self, st: &mut MultiPadState) {
        if st.synchro_stop.style_stop {
            self.stop_repeating(st);
        }
    }

    pub fn ending_started(&mut self, st: &mut MultiPadState) {
        if st.synchro_stop.ending {
            self.stop_repeating(st);
        }
    }

    pub fn panic(&mut self, st: &mut MultiPadState) {
        for i in 0..4 {
            if Self::has(st, i) {
                self.set(st, i, PadLamp::Ready);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_press_queue_and_synchro_stop() {
        let (mut st, mut p) = (initial(), MockPads::default());
        assert!(p.cmd(&mut st, MultiPadCmd::LoadMultiPad { id: 0 }, false).is_none());
        assert_eq!(st.bank.as_ref().map(|b| b.name.as_str()), Some("Demo"));
        p.cmd(&mut st, MultiPadCmd::TriggerMultiPad { pad: 2 }, true);
        assert_eq!(st.pads[2].lamp, PadLamp::Queued);
        p.bar(&mut st);
        assert_eq!(st.pads[2].lamp, PadLamp::Playing);
        p.band_stopped(&mut st);
        assert_eq!(st.pads[2].lamp, PadLamp::Ready);
        assert!(p.cmd(&mut st, MultiPadCmd::ArmMultiPad { pad: 9 }, false).is_some());
    }
}

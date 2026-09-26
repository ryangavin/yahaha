//! Chord Looper: the panel buttons go to the engine (engine/looper.rs), which records and
//! loops; the eight memories live here. A finished recording comes back through a ring
//! (`pump_looper`); a memory goes to the engine through another (`LooperCtl::tx`).

use super::Control;
use crate::api::{CmdError, LoopChord, LooperCmd, LooperMemory, LooperMode, LooperState};
use crate::engine::LoopState;
use crate::launchkey::LooperLamp;
use crate::live::Cmd;
use crate::looper::{ChordSeq, LOOP_PPQ};
use rtrb::{Consumer, Producer};

/// Chord Looper memories per bank (RM p.17).
pub const MEMORIES: usize = 8;

/// The control side of the Chord Looper.
pub(super) struct LooperCtl {
    /// Sequences to the engine (a memory selected), and finished recordings back.
    tx: Producer<ChordSeq>,
    rx: Consumer<ChordSeq>,
    /// The engine's sequence, as last handed over either way.
    current: ChordSeq,
    memories: [Option<(String, ChordSeq)>; MEMORIES],
    selected: Option<u8>,
    /// A memory selected while looping, and the engine's `seq_gen` then: it has taken over
    /// once the engine's generation moves on.
    pending: Option<(u8, u32)>,
    /// Memories stored so far, for the "CLD_001" names.
    stored: u32,
    /// The engine's `chart_yields` last seen: ON/OFF turned chart mode off to arm a loop.
    chart_yields: u16,
}

impl LooperCtl {
    pub(super) fn new(tx: Producer<ChordSeq>, rx: Consumer<ChordSeq>) -> LooperCtl {
        LooperCtl { tx, rx, current: ChordSeq::EMPTY, memories: Default::default(), selected: None, pending: None, stored: 0, chart_yields: 0 }
    }

    /// Tests: hand the engine an empty sequence (as a fresh engine has).
    #[cfg(test)]
    pub(super) fn empty_engine_seq(&mut self) {
        self.tx.push(ChordSeq::EMPTY).unwrap();
    }
}

fn chords(seq: &ChordSeq) -> Vec<LoopChord> {
    seq.events()
        .iter()
        .map(|e| LoopChord { bar: e.bar as u32 + 1, beat: 1.0 + e.at as f64 / LOOP_PPQ as f64, chord: e.chord.name() })
        .collect()
}

impl Control {
    pub(super) fn looper_cmd(&mut self, c: LooperCmd) -> Result<(), CmdError> {
        let state = self.snap.looper.state;
        let recording = matches!(state, LoopState::Recording | LoopState::RecArmed);
        match c {
            LooperCmd::LooperRec => self.engine_cmd(Cmd::Looper(true)),
            // Only one of the chart player and the Chord Looper gives the chords: the engine
            // decides whether the loop arms, and turns chart mode off for it on its own state
            // (#110: a snapshot taken before a memory was selected can't say); `pump_looper`
            // follows.
            LooperCmd::LooperOnOff => self.engine_cmd(Cmd::Looper(false)),
            LooperCmd::SelectLooperMemory { index } => {
                let i = index as usize % MEMORIES;
                if recording {
                    return self.fail("Chord Looper: stop recording before choosing a memory");
                }
                let Some((_, seq)) = self.looper.memories[i].clone() else {
                    // An empty memory: selected, nothing to load.
                    self.looper.selected = Some(i as u8);
                    self.looper.pending = None;
                    return Ok(());
                };
                self.looper.tx.push(seq).map_err(|_| CmdError::Busy)?;
                self.wake_engine();
                if state == LoopState::Looping {
                    self.looper.pending = Some((i as u8, self.snap.looper.seq_gen));
                } else {
                    self.looper.selected = Some(i as u8);
                    self.looper.pending = None;
                    self.looper.current = seq;
                }
                Ok(())
            }
            LooperCmd::StoreLooperMemory { index } => {
                let i = index as usize % MEMORIES;
                if self.looper.current.is_empty() || recording {
                    return self.fail("Chord Looper: record a chord sequence first");
                }
                self.looper.stored += 1;
                self.looper.memories[i] = Some((format!("CLD_{:03}", self.looper.stored), self.looper.current));
                self.looper.selected = Some(i as u8);
                Ok(())
            }
            LooperCmd::ClearLooperMemory { index } => {
                let i = index as usize % MEMORIES;
                self.looper.memories[i] = None;
                if self.looper.pending.is_some_and(|(p, _)| p as usize == i) {
                    self.looper.pending = None;
                }
                Ok(())
            }
            LooperCmd::NewLooperBank => {
                self.looper.memories = Default::default();
                self.looper.selected = None;
                self.looper.pending = None;
                Ok(())
            }
        }
    }

    /// A finished recording becomes the current sequence (no memory selected); a memory
    /// chosen while looping becomes the selected one once the engine has switched to it.
    pub(super) fn pump_looper(&mut self) {
        let yields = self.snap.looper.chart_yields;
        if yields != self.looper.chart_yields {
            self.looper.chart_yields = yields;
            if self.chart_mode_off_by_engine() {
                self.say("Chart mode off: the Chord Looper plays", false);
            }
        }
        let l = &mut self.looper;
        while let Ok(seq) = l.rx.pop() {
            l.current = seq;
            l.selected = None;
        }
        if let Some((i, since)) = l.pending {
            let s = self.snap.looper;
            if s.state != LoopState::Looping && !s.pending && s.seq_gen == since {
                // The loop stopped before the bar line: the memory never took over.
                l.pending = None;
            } else if s.seq_gen != since {
                l.pending = None;
                l.selected = Some(i);
                if let Some((_, seq)) = &l.memories[i as usize] {
                    l.current = *seq;
                }
            }
        }
    }

    /// The Chord Looper as the Launchkey's Panel fader button 8 shows it.
    pub(super) fn looper_lamp(&self) -> LooperLamp {
        let s = self.snap.looper;
        match s.state {
            LoopState::Off if s.has_data => LooperLamp::Ready,
            LoopState::Off => LooperLamp::Empty,
            LoopState::RecArmed => LooperLamp::RecArmed,
            LoopState::Recording => LooperLamp::Recording,
            LoopState::LoopArmed => LooperLamp::LoopArmed,
            LoopState::Looping => LooperLamp::Looping,
        }
    }

    pub(super) fn looper_state(&self) -> LooperState {
        let s = self.snap.looper;
        let l = &self.looper;
        let mode = match s.state {
            LoopState::Off => LooperMode::Off,
            LoopState::RecArmed => LooperMode::RecArmed,
            LoopState::Recording => LooperMode::Recording,
            LoopState::LoopArmed => LooperMode::LoopArmed,
            LoopState::Looping => LooperMode::Looping,
        };
        let recording = s.state == LoopState::Recording;
        LooperState {
            mode,
            has_data: s.has_data,
            bar: (recording || s.state == LoopState::Looping).then_some(s.bar as u32 + 1),
            bars: s.bars as u32,
            chords: if recording { Vec::new() } else { chords(&l.current) },
            memory: l.selected,
            pending_memory: l.pending.map(|(i, _)| i),
            memories: l
                .memories
                .iter()
                .map(|m| match m {
                    Some((name, seq)) => LooperMemory { name: Some(name.clone()), bars: seq.bars() as u32, chords: chords(seq) },
                    None => LooperMemory::default(),
                })
                .collect(),
        }
    }
}

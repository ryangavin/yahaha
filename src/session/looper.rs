//! Chord Looper: the panel buttons go to the engine (engine/looper.rs), which records and
//! loops; the eight memories live here. A finished recording comes back through a ring
//! (`pump_looper`); a memory goes to the engine through another (`LooperCtl::tx`).

use super::Control;
use crate::api::{CmdError, LoopChord, LooperCmd, LooperMemory, LooperMode, LooperState};
use crate::engine::LoopState;
use crate::live::Cmd;
use crate::looper::{ChordSeq, SeqFile, LOOP_PPQ};
use crate::registration::{Group, Groups};
use serde::{Deserialize, Serialize};
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

// ----- Registration (group Chord Looper; Data List p.82) -----

/// The Chord Looper in a Registration Memory: the memory selected, ON/OFF, and the
/// sequence it plays, so a recall works in a later session too (#201).
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LooperReg {
    /// The memory selected (0-7), if any.
    #[serde(default)]
    memory: Option<u8>,
    /// ON/OFF: the loop armed or playing.
    #[serde(default)]
    on: bool,
    /// The selected memory's sequence (or the current one with no memory selected).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sequence: Option<SeqFile>,
    /// The memory's name ("CLD_001").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

pub(super) fn looper_capture(c: &Control, g: Groups) -> Option<serde_json::Value> {
    if !g.has(Group::ChordLooper) {
        return None;
    }
    let l = &c.looper;
    let memory = l.selected;
    let (name, seq) = match memory.and_then(|i| l.memories[i as usize].as_ref()) {
        Some((name, seq)) => (Some(name.clone()), *seq),
        None => (None, l.current),
    };
    let on = matches!(c.snap.looper.state, LoopState::LoopArmed | LoopState::Looping);
    let r = LooperReg { memory, on, sequence: (!seq.is_empty()).then(|| SeqFile::of(&seq)), name };
    serde_json::to_value(&r).ok()
}

/// Recall the `chordLooper` section: the memory (its sequence put back if the memory lost
/// it), then ON/OFF: on arms the loop (it starts at the next bar line, or with the style),
/// off stops a loop at once. A recording under way is left alone.
pub(super) fn looper_recall(c: &mut Control, v: &serde_json::Value, g: Groups) -> Result<(), String> {
    if !g.has(Group::ChordLooper) {
        return Ok(());
    }
    let r: LooperReg = serde_json::from_value(v.clone()).map_err(|e| format!("registration chordLooper: {e}"))?;
    let state = c.snap.looper.state;
    if matches!(state, LoopState::RecArmed | LoopState::Recording) {
        return Ok(());
    }
    let e = |e: CmdError| e.to_string();
    let seq = r.sequence.as_ref().map(SeqFile::to_seq).filter(|s| !s.is_empty());
    match (r.memory, seq) {
        (Some(i), seq) => {
            let i = i as usize % MEMORIES;
            if let Some(seq) = seq {
                if c.looper.memories[i].as_ref().is_none_or(|(_, m)| *m != seq) {
                    let name = r.name.clone().unwrap_or_else(|| format!("Registration {}", i + 1));
                    c.looper.memories[i] = Some((name, seq));
                }
            }
            c.looper_cmd(LooperCmd::SelectLooperMemory { index: i as u8 }).map_err(e)?;
        }
        (None, Some(seq)) => {
            c.looper.tx.push(seq).map_err(|_| "Chord Looper busy".to_string())?;
            c.looper.current = seq;
            c.looper.selected = None;
            c.wake_engine();
        }
        (None, None) => {}
    }
    let on = matches!(state, LoopState::LoopArmed | LoopState::Looping);
    if on != r.on {
        c.looper_cmd(LooperCmd::LooperOnOff).map_err(e)?;
    }
    Ok(())
}

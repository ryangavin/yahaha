//! The mock session's Chord Looper, the twin of `app/src/lib/api/mock-looper.ts`: the
//! engine's rules (src/engine/looper.rs) on whole beats. Recording, loop playback and a
//! memory change start at the next bar; stopping the loop is immediate.

use std::collections::BTreeMap;
use yahaha::api::{BankFile, LoopChord, LooperMemory, LooperMode, LooperState};

const MEMORIES: usize = 8;
const BANK_DIR: &str = "/mock/ChordLooper";
const NEW_BANK: &str = "New Bank";

pub fn empty() -> LooperState {
    LooperState { memories: vec![LooperMemory::default(); MEMORIES], bank_name: NEW_BANK.into(), ..LooperState::default() }
}

fn bank_name(path: &str) -> String {
    let n = path.rsplit('/').next().unwrap_or(path);
    n.strip_suffix(".looper.json").unwrap_or(n).to_string()
}

#[derive(Clone, Default)]
struct Seq {
    bars: u32,
    chords: Vec<LoopChord>,
}

#[derive(Default)]
pub struct MockLooper {
    seq: Seq,
    pending: Option<Seq>,
    rec_start: u32,
    rec_bars: u32,
    loop_bar: u32,
    loop_start: u32,
    stored: u32,
    /// Bank files, in memory: path -> memories.
    files: BTreeMap<String, Vec<LooperMemory>>,
}

impl MockLooper {
    /// A chord from the keyboard at `bar` (the band's count) and 1-based `beat`. False: the
    /// loop plays and ignores it.
    pub fn keyboard_chord(&mut self, s: &LooperState, chord: &str, bar: u32, beat: f64) -> bool {
        match s.mode {
            LooperMode::Looping => false,
            LooperMode::Recording => {
                self.record(bar.saturating_sub(self.rec_start) + 1, beat, chord);
                true
            }
            _ => true,
        }
    }

    fn record(&mut self, bar: u32, beat: f64, chord: &str) {
        match self.seq.chords.last_mut() {
            Some(l) if l.bar == bar && l.beat == beat => l.chord = chord.into(),
            Some(l) if l.chord == chord => {}
            _ => self.seq.chords.push(LoopChord { bar, beat, chord: chord.into() }),
        }
    }

    /// REC/STOP. True: Sync Start turns on (stopped).
    pub fn rec(&mut self, s: &mut LooperState, running: bool) -> bool {
        match s.mode {
            LooperMode::RecArmed => s.mode = LooperMode::Off,
            LooperMode::Recording => self.finish(s, LooperMode::Off),
            _ => {
                s.mode = LooperMode::RecArmed;
                self.pending = None;
                s.pending_memory = None;
                return !running;
            }
        }
        false
    }

    /// ON/OFF. A loop stops on its chord: what the keyboard played over it was not chord
    /// input (RM p.15, OM p.68).
    pub fn on_off(&mut self, s: &mut LooperState) {
        match s.mode {
            LooperMode::Recording => self.finish(s, LooperMode::LoopArmed),
            LooperMode::RecArmed | LooperMode::LoopArmed => s.mode = LooperMode::Off,
            LooperMode::Off if !self.seq.chords.is_empty() => s.mode = LooperMode::LoopArmed,
            LooperMode::Off => {}
            LooperMode::Looping => {
                s.mode = LooperMode::Off;
                self.pending = None;
                s.pending_memory = None;
            }
        }
    }

    fn finish(&mut self, s: &mut LooperState, next: LooperMode) {
        self.seq.bars = self.rec_bars;
        let bars = self.rec_bars;
        self.seq.chords.retain(|c| c.bar <= bars);
        s.memory = None;
        s.mode = if self.seq.chords.is_empty() { LooperMode::Off } else { next };
    }

    /// A bar line: the loop's chord to play there, if any.
    pub fn on_bar(&mut self, s: &mut LooperState, bar: u32, played: Option<&str>) -> Option<String> {
        match s.mode {
            LooperMode::RecArmed => {
                s.mode = LooperMode::Recording;
                self.seq = Seq::default();
                self.rec_start = bar;
                self.rec_bars = 1;
                if let Some(p) = played {
                    self.record(1, 1.0, p);
                }
                return None;
            }
            LooperMode::Recording => {
                self.rec_bars = bar.saturating_sub(self.rec_start) + 1;
                return None;
            }
            LooperMode::LoopArmed | LooperMode::Looping if s.mode == LooperMode::LoopArmed || self.pending.is_some() => {
                if let Some(p) = self.pending.take() {
                    self.seq = p;
                    s.memory = s.pending_memory.take();
                }
                s.mode = LooperMode::Looping;
                self.loop_start = bar;
            }
            _ => {}
        }
        if s.mode != LooperMode::Looping || self.seq.bars == 0 {
            return None;
        }
        self.loop_bar = bar.saturating_sub(self.loop_start) % self.seq.bars;
        let at = self.seq.chords.iter().rev().find(|c| c.bar <= self.loop_bar + 1).or(self.seq.chords.last());
        at.map(|c| c.chord.clone())
    }

    /// The band stopped: a recording ends; a loop waits for the next start.
    pub fn on_stop(&mut self, s: &mut LooperState) {
        match s.mode {
            LooperMode::Recording => self.finish(s, LooperMode::Off),
            LooperMode::RecArmed => s.mode = LooperMode::Off,
            LooperMode::Looping => s.mode = LooperMode::LoopArmed,
            _ => {}
        }
    }

    pub fn select(&mut self, s: &mut LooperState, i: usize) -> Result<(), &'static str> {
        if matches!(s.mode, LooperMode::Recording | LooperMode::RecArmed) {
            return Err("Chord Looper: stop recording before choosing a memory");
        }
        let m = &s.memories[i];
        if m.name.is_none() {
            s.memory = Some(i as u8);
            return Ok(());
        }
        let seq = Seq { bars: m.bars, chords: m.chords.clone() };
        if s.mode == LooperMode::Looping {
            self.pending = Some(seq);
            s.pending_memory = Some(i as u8);
        } else {
            self.seq = seq;
            s.memory = Some(i as u8);
        }
        Ok(())
    }

    pub fn store(&mut self, s: &mut LooperState, i: usize) -> Result<(), &'static str> {
        if self.seq.chords.is_empty() || matches!(s.mode, LooperMode::Recording | LooperMode::RecArmed) {
            return Err("Chord Looper: record a chord sequence first");
        }
        self.stored += 1;
        s.memories[i] = LooperMemory { name: Some(format!("CLD_{:03}", self.stored)), bars: self.seq.bars, chords: self.seq.chords.clone() };
        s.memory = Some(i as u8);
        Ok(())
    }

    pub fn clear(&mut self, s: &mut LooperState, i: usize) {
        s.memories[i] = LooperMemory::default();
    }

    pub fn new_bank(&mut self, s: &mut LooperState) {
        s.memories = empty().memories;
        s.memory = None;
        s.pending_memory = None;
        s.bank_name = NEW_BANK.into();
        s.bank_path = None;
        self.pending = None;
    }

    /// Save to the bank's file, or as `name` (as the session's `save_looper_bank`).
    pub fn save_bank(&mut self, s: &mut LooperState, name: Option<String>, overwrite: bool) -> Result<(), String> {
        let mut path = s.bank_path.clone();
        if let Some(name) = name {
            let n = name.trim().to_string();
            if n.is_empty() {
                return Err("Chord Looper: give the bank a name".into());
            }
            let p = format!("{BANK_DIR}/{}.looper.json", n.replace(['/', '\\', ':'], "_"));
            if Some(&p) != s.bank_path.as_ref() && self.files.contains_key(&p) && !overwrite {
                return Err(format!("a Chord Looper bank called {n} already exists: save under another name, or overwrite it"));
            }
            s.bank_name = n;
            path = Some(p);
        }
        let Some(path) = path else { return Err("Chord Looper: give the bank a name to save it".into()) };
        self.files.insert(path.clone(), s.memories.clone());
        s.bank_path = Some(path);
        s.banks = self.files.keys().map(|p| BankFile { name: bank_name(p), path: p.clone() }).collect();
        s.banks.sort_by_key(|b| b.name.to_lowercase());
        Ok(())
    }

    pub fn load_bank(&mut self, s: &mut LooperState, path: &str) -> Result<(), String> {
        if matches!(s.mode, LooperMode::Recording | LooperMode::RecArmed) {
            return Err("Chord Looper: stop recording before loading a bank".into());
        }
        let m = self.files.get(path).ok_or_else(|| format!("Chord Looper: reading {path}: not found"))?;
        s.memories = m.clone();
        s.memory = None;
        s.pending_memory = None;
        self.pending = None;
        s.bank_path = Some(path.to_string());
        s.bank_name = bank_name(path);
        Ok(())
    }

    /// Bring the derived fields up to date.
    pub fn publish(&self, s: &mut LooperState) {
        let recording = s.mode == LooperMode::Recording;
        s.has_data = !self.seq.chords.is_empty() && !recording;
        s.bars = if recording { self.rec_bars } else { self.seq.bars };
        s.bar = if recording {
            Some(self.rec_bars)
        } else if s.mode == LooperMode::Looping {
            Some(self.loop_bar + 1)
        } else {
            None
        };
        s.chords = if recording { vec![] } else { self.seq.chords.clone() };
    }
}

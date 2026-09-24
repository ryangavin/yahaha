//! Chord Looper in the engine (Genos CHORD LOOPER, RM p.14-19): records the chords the
//! player plays, bar by bar, and while it loops feeds them to the style as if they were
//! played. Recording, loop playback and a memory change all start at the next bar line
//! (`on_bar`); stopping the loop is immediate.
//!
//! States (the panel's REC/STOP and ON/OFF lamps):
//!
//! | State | REC/STOP | ON/OFF | |
//! |---|---|---|---|
//! | `Off` | off | off / blue (data) | the keyboard's chords |
//! | `RecArmed` | flashing | | recording starts at the next bar line (stopped: with the first chord, Sync Start on) |
//! | `Recording` | on | | the keyboard's chords, recorded |
//! | `LoopArmed` | | flashing | the loop starts at the next bar line (stopped: when the style starts) |
//! | `Looping` | | on | the loop's chords; the keyboard's are ignored |
//!
//! The sequence is a fixed-size `ChordSeq` in a box built on the control side: nothing
//! here allocates. A finished recording waits in `recorded` for the engine loop to pass
//! it to the control side (`take_recorded`); memories come in with `looper_load`.

use super::*;
use crate::looper::{ChordSeq, LOOP_PPQ, MAX_BARS};

/// Where the Chord Looper is (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LoopState {
    #[default]
    Off,
    RecArmed,
    Recording,
    LoopArmed,
    Looping,
}

/// The Chord Looper in a `Snapshot`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LooperSnap {
    pub state: LoopState,
    /// Recording: the bar being recorded; looping: the loop's bar playing (0-based).
    pub bar: u16,
    /// Recording: bars recorded so far; otherwise the sequence's length.
    pub bars: u16,
    /// There is a sequence to loop.
    pub has_data: bool,
    /// A memory change waits for the next bar line.
    pub pending: bool,
    /// Counts changes of the sequence (a recording, a memory taking over).
    pub seq_gen: u32,
}

#[derive(Default)]
pub(super) struct Looper {
    state: LoopState,
    seq: ChordSeq,
    /// A memory to take over at the next bar line (while looping).
    pending: Option<ChordSeq>,
    /// Bars begun since recording started.
    rec_bars: u16,
    /// The loop's bar playing, and the next chord change in `seq` to play.
    bar: u16,
    idx: usize,
    /// Tick (on the section timeline) of the bar line last passed.
    bar_tick: f64,
    /// A finished recording for `take_recorded`.
    recorded: bool,
    seq_gen: u32,
    /// REC/STOP turned Sync Start on (stopped): cancelling the recording turns it off again.
    armed_sync: bool,
}

impl Looper {
    /// The player pressed SYNC START: it is theirs, not REC/STOP's, from now on.
    pub(super) fn forget_sync(&mut self) {
        self.armed_sync = false;
    }
}

impl Engine {
    fn loop_bar_len(&self) -> u32 {
        (self.style.tpb as u64 * LOOP_PPQ as u64 / self.style.ppq.max(1) as u64) as u32
    }

    /// Chord Looper REC/STOP.
    pub fn looper_rec(&mut self) {
        let l = &mut self.features.looper;
        match l.state {
            LoopState::Off | LoopState::LoopArmed | LoopState::Looping => {
                l.state = LoopState::RecArmed;
                l.pending = None;
                if !self.running && !self.sync_armed {
                    // Recording from stopped: Sync Start, so the first chord starts both.
                    self.sync_armed = true;
                    l.armed_sync = true;
                }
            }
            LoopState::RecArmed => self.cancel_rec(),
            LoopState::Recording => self.finish_recording(LoopState::Off),
        }
    }

    /// Recording armed, cancelled before it began: Sync Start goes off again if REC/STOP
    /// turned it on (and the band has not started).
    fn cancel_rec(&mut self) {
        let l = &mut self.features.looper;
        l.state = LoopState::Off;
        if std::mem::take(&mut l.armed_sync) && !self.running {
            self.sync_armed = false;
        }
    }

    /// Chord Looper ON/OFF.
    pub fn looper_on_off(&mut self) {
        let l = &mut self.features.looper;
        match l.state {
            LoopState::Recording => self.finish_recording(LoopState::LoopArmed),
            LoopState::RecArmed => self.cancel_rec(),
            LoopState::LoopArmed => l.state = LoopState::Off,
            LoopState::Off if !l.seq.is_empty() => l.state = LoopState::LoopArmed,
            LoopState::Off => {}
            // The loop stops at once; the style keeps the loop's chord until the keyboard
            // plays one (chord input was disabled while looping: RM p.15, OM p.68).
            LoopState::Looping => {
                l.state = LoopState::Off;
                l.pending = None;
            }
        }
    }

    /// A sequence from a memory. While looping it takes over at the next bar line; while
    /// recording it is ignored; otherwise it becomes the sequence at once.
    pub fn looper_load(&mut self, seq: &ChordSeq) {
        let l = &mut self.features.looper;
        match l.state {
            LoopState::Recording | LoopState::RecArmed => {}
            LoopState::Looping => l.pending = Some(*seq),
            LoopState::Off | LoopState::LoopArmed => {
                l.seq = *seq;
                l.seq_gen = l.seq_gen.wrapping_add(1);
                if l.seq.is_empty() {
                    l.state = LoopState::Off;
                }
            }
        }
    }

    /// A recording that has finished since the last call.
    pub fn take_recorded(&mut self) -> Option<ChordSeq> {
        let l = &mut self.features.looper;
        std::mem::take(&mut l.recorded).then_some(l.seq)
    }

    pub fn looper_snapshot(&self) -> LooperSnap {
        let l = &self.features.looper;
        let recording = l.state == LoopState::Recording;
        LooperSnap {
            state: l.state,
            bar: if recording { l.rec_bars.saturating_sub(1) } else if l.state == LoopState::Looping { l.bar } else { 0 },
            bars: if recording { l.rec_bars } else { l.seq.bars() },
            has_data: !l.seq.is_empty(),
            pending: l.pending.is_some(),
            seq_gen: l.seq_gen,
        }
    }

    /// The loop plays: the keyboard's chords (and releasing them, for Sync Stop) are ignored.
    pub fn looper_owns_chords(&self) -> bool {
        self.features.looper.state == LoopState::Looping
    }

    /// A chord from the keyboard. False: the loop is playing and ignores it. Recording, it
    /// is recorded where it falls in the bar.
    pub(super) fn looper_keyboard_chord(&mut self, played: Chord, now: u64) -> bool {
        let bar_len = self.loop_bar_len();
        let pos = ((self.tick_at(now) - self.features.looper.bar_tick) * LOOP_PPQ as f64 / self.style.ppq.max(1) as f64).max(0.0);
        let running = self.running;
        let l = &mut self.features.looper;
        match l.state {
            LoopState::Looping => false,
            LoopState::Recording if running => {
                if !l.seq.record(l.rec_bars.saturating_sub(1), pos as u32, bar_len, played) {
                    // Full: recording stops as if REC/STOP had been pressed.
                    self.finish_recording(LoopState::Off);
                }
                true
            }
            _ => true,
        }
    }

    fn finish_recording(&mut self, next: LoopState) {
        let l = &mut self.features.looper;
        l.seq.finish(l.rec_bars);
        l.recorded = true;
        l.seq_gen = l.seq_gen.wrapping_add(1);
        l.state = if l.seq.is_empty() { LoopState::Off } else { next };
    }

    /// Play the loop's chord `c` as if it had been played.
    fn loop_chord(&mut self, c: Chord, now: u64, sink: &mut impl Sink) {
        if self.played != Some(c) {
            self.apply_chord(c, now, sink);
        }
    }

    /// The band stopped (hook): a recording ends; a loop waits for the next start.
    pub(super) fn looper_on_stop(&mut self) {
        let l = &mut self.features.looper;
        match l.state {
            LoopState::Recording => self.finish_recording(LoopState::Off),
            LoopState::RecArmed => {
                l.state = LoopState::Off;
                l.armed_sync = false;
            }
            LoopState::Looping => {
                l.state = LoopState::LoopArmed;
                if let Some(p) = l.pending.take() {
                    l.seq = p;
                    l.seq_gen = l.seq_gen.wrapping_add(1);
                }
            }
            LoopState::Off | LoopState::LoopArmed => {}
        }
    }

    /// A bar line (hook): what was armed starts here; the loop moves to its next bar.
    pub(super) fn looper_on_bar(&mut self, now: u64, sink: &mut impl Sink) {
        let tick = self.lines.next;
        let bar_len = self.loop_bar_len();
        let played = self.played;
        let running = self.running;
        let l = &mut self.features.looper;
        l.bar_tick = tick;
        let enter = match l.state {
            LoopState::RecArmed if running => {
                l.seq.clear();
                l.rec_bars = 1;
                l.state = LoopState::Recording;
                l.armed_sync = false;
                // The chord held as recording starts is its first.
                if let Some(p) = played {
                    l.seq.record(0, 0, bar_len, p);
                }
                false
            }
            LoopState::Recording => {
                l.rec_bars += 1;
                if l.rec_bars > MAX_BARS {
                    l.rec_bars = MAX_BARS;
                    self.finish_recording(LoopState::Off);
                }
                false
            }
            LoopState::LoopArmed if running => {
                if let Some(p) = l.pending.take() {
                    l.seq = p;
                    l.seq_gen = l.seq_gen.wrapping_add(1);
                }
                if l.seq.is_empty() {
                    l.state = LoopState::Off;
                    false
                } else {
                    l.state = LoopState::Looping;
                        true
                }
            }
            LoopState::Looping => {
                if let Some(p) = l.pending.take() {
                    l.seq = p;
                    l.seq_gen = l.seq_gen.wrapping_add(1);
                    true
                } else {
                    l.bar = (l.bar + 1) % l.seq.bars().max(1);
                    l.idx = l.seq.events().partition_point(|e| e.bar < l.bar);
                    false
                }
            }
            _ => false,
        };
        if enter {
            // The loop (or a new memory) starts from its top, with the chord in force there.
            let l = &mut self.features.looper;
            l.bar = 0;
            l.idx = 0;
            if let Some(c) = l.seq.chord_at_bar(0) {
                self.loop_chord(c, now, sink);
            }
        }
        self.looper_play_due(tick, now, sink);
    }

    /// Tick of the loop's next chord change in the bar playing, if any.
    pub(super) fn looper_due(&self) -> Option<f64> {
        let l = &self.features.looper;
        if l.state != LoopState::Looping || !self.running {
            return None;
        }
        let e = l.seq.events().get(l.idx).filter(|e| e.bar == l.bar)?;
        Some(l.bar_tick + e.at as f64 * self.style.ppq as f64 / LOOP_PPQ as f64)
    }

    /// Play the loop's chord changes due by tick `target`.
    pub(super) fn looper_play_due(&mut self, target: f64, now: u64, sink: &mut impl Sink) {
        while let Some(t) = self.looper_due() {
            if t > target + 1e-6 {
                break;
            }
            let l = &mut self.features.looper;
            let c = l.seq.events()[l.idx].chord;
            l.idx += 1;
            self.loop_chord(c, now, sink);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_chord;

    struct Nop;
    impl Sink for Nop {
        fn send(&mut self, _: &[u8]) {}
    }

    fn engine() -> Option<Engine> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
        if !p.exists() {
            eprintln!("corpus missing; skipping");
            return None;
        }
        Some(Engine::new(Box::new(Prepared::new(&Style::load(&p).unwrap()))))
    }

    fn c(n: &str) -> Chord {
        parse_chord(n).unwrap()
    }

    /// Run the engine from `from` to `to` in 5 ms steps, playing `chords` (time, chord) on the
    /// way; returns the chords the style followed, with when.
    fn run(e: &mut Engine, from: u64, to: u64, chords: &[(u64, &str)]) -> Vec<(u64, Chord)> {
        let mut out = Vec::new();
        let mut now = from;
        let mut last = e.played;
        while now < to {
            for &(t, n) in chords {
                if t >= now && t < now + 5_000_000 {
                    e.set_chord(c(n), t, &mut Nop);
                }
            }
            e.process(now, &mut Nop);
            if e.played != last {
                last = e.played;
                out.push((now, last.unwrap()));
            }
            now += 5_000_000;
        }
        out
    }

    /// Record C | F G while playing, loop it: the loop plays the same chords from the next
    /// bar, bar after bar, and the keyboard is ignored; ON/OFF stops it on the loop's chord
    /// and gives the keyboard back.
    #[test]
    fn record_then_loop() {
        let Some(mut e) = engine() else { return };
        let bar = e.ns_at_bar(1);
        let half = bar / 2;
        e.set_chord(c("C"), 0, &mut Nop);
        run(&mut e, 0, bar / 3, &[]);
        e.looper_rec();
        assert_eq!(e.looper_snapshot().state, LoopState::RecArmed);
        // Recording starts at bar 2 with C held; F at bar 3, G half-way through it.
        run(&mut e, bar / 3, 3 * bar - bar / 4, &[(2 * bar, "F"), (2 * bar + half, "G")]);
        assert_eq!(e.looper_snapshot().state, LoopState::Recording);
        e.looper_on_off();
        let s = e.looper_snapshot();
        assert_eq!((s.state, s.bars), (LoopState::LoopArmed, 2));
        let rec = e.take_recorded().unwrap();
        assert_eq!(rec.events().iter().map(|e| e.chord).collect::<Vec<_>>(), [c("C"), c("F"), c("G")]);
        assert!(e.take_recorded().is_none());
        // Bar 4 on: the loop, and a keyboard chord changes nothing.
        let heard = run(&mut e, 3 * bar - bar / 4, 7 * bar + bar / 4, &[(4 * bar + half / 2, "Bb")]);
        assert_eq!(e.looper_snapshot().state, LoopState::Looping);
        let near = |a: u64, b: u64| a.abs_diff(b) <= 6_000_000;
        let want = [(3 * bar, "C"), (4 * bar, "F"), (4 * bar + half, "G"), (5 * bar, "C"), (6 * bar, "F"), (6 * bar + half, "G"), (7 * bar, "C")];
        assert_eq!(heard.len(), want.len(), "{heard:?}");
        for ((t, ch), (wt, wn)) in heard.iter().zip(want) {
            assert!(near(*t, wt) && *ch == c(wn), "{t} {ch:?} vs {wt} {wn}");
        }
        // Off: at once, and the style keeps the loop's chord: the Bb played while looping
        // was not chord input (RM p.15, OM p.68).
        e.looper_on_off();
        assert_eq!(e.played, Some(c("C")));
        // The next chord from the keyboard is followed.
        e.set_chord(c("Bb"), 7 * bar + bar / 2, &mut Nop);
        assert_eq!(e.played, Some(c("Bb")));
        assert_eq!(e.looper_snapshot().state, LoopState::Off);
        assert!(e.looper_snapshot().has_data);
    }

    /// REC/STOP while stopped turns Sync Start on; cancelling the recording before it
    /// begins (REC/STOP or ON/OFF again) turns it off again. Sync Start that was already
    /// on, or that the player pressed since, stays on.
    #[test]
    fn cancelled_rec_disarms_its_sync_start() {
        let Some(mut e) = engine() else { return };
        e.button(Button::SyncStart, 0, &mut Nop); // off
        e.looper_rec();
        assert!(e.starts_on_chord());
        e.looper_rec();
        assert_eq!(e.looper_snapshot().state, LoopState::Off);
        assert!(!e.starts_on_chord(), "REC cancelled: Sync Start off again");
        e.looper_rec();
        e.looper_on_off();
        assert_eq!(e.looper_snapshot().state, LoopState::Off);
        assert!(!e.starts_on_chord(), "cancelled with ON/OFF too");
        // Already on: REC did not turn it on, so cancelling leaves it on.
        e.button(Button::SyncStart, 4, &mut Nop); // on
        e.looper_rec();
        e.looper_rec();
        assert!(e.starts_on_chord());
        // The player pressed SYNC START off and on while REC was armed: theirs.
        e.button(Button::SyncStart, 7, &mut Nop); // off
        e.looper_rec();
        e.button(Button::SyncStart, 9, &mut Nop); // off
        e.button(Button::SyncStart, 10, &mut Nop); // on
        e.looper_rec();
        assert!(e.starts_on_chord());
    }

    /// Recording from stopped: Sync Start, and the first chord starts the style and the
    /// recording together; START/STOP ends both.
    #[test]
    fn record_from_stopped() {
        let Some(mut e) = engine() else { return };
        e.button(Button::SyncStart, 0, &mut Nop); // off
        assert!(!e.starts_on_chord());
        e.looper_rec();
        assert!(e.starts_on_chord());
        let bar = e.ns_at_bar(1);
        run(&mut e, 0, 10 * bar, &[(1_000_000, "D"), (1_000_000 + bar, "A")]);
        assert_eq!(e.looper_snapshot().state, LoopState::Recording);
        e.button(Button::StartStop, 10 * bar, &mut Nop);
        let s = e.looper_snapshot();
        assert_eq!((s.state, s.has_data), (LoopState::Off, true));
        let rec = e.take_recorded().unwrap();
        assert_eq!(rec.events()[0].chord, c("D"));
        assert_eq!((rec.events()[1].bar, rec.events()[1].at), (1, 0));
    }

    /// A memory chosen while looping takes over at the next bar line, from its top.
    #[test]
    fn memory_change_at_the_bar_line() {
        let Some(mut e) = engine() else { return };
        let ev = |ch: &str| crate::looper::LoopEvent { bar: 0, at: 0, chord: c(ch) };
        e.looper_load(&ChordSeq::from_events(1, &[ev("E")]));
        e.looper_on_off();
        assert_eq!(e.looper_snapshot().state, LoopState::LoopArmed);
        let bar = e.ns_at_bar(1);
        e.button(Button::StartStop, 0, &mut Nop);
        assert_eq!(e.played, Some(c("E")));
        run(&mut e, 0, bar + bar / 2, &[]);
        e.looper_load(&ChordSeq::from_events(1, &[ev("Ab")]));
        assert!(e.looper_snapshot().pending);
        run(&mut e, bar + bar / 2, 2 * bar - 10_000_000, &[]);
        assert_eq!(e.played, Some(c("E")));
        run(&mut e, 2 * bar - 10_000_000, 2 * bar + 10_000_000, &[]);
        assert_eq!(e.played, Some(c("Ab")));
        // Stopping the band leaves the loop armed for the next start.
        e.button(Button::StartStop, 3 * bar, &mut Nop);
        assert_eq!(e.looper_snapshot().state, LoopState::LoopArmed);
    }
}

//! Chord Looper sequences (Genos CHORD LOOPER, RM p.14-19, OM p.68-69).
//!
//! A [`ChordSeq`] is a recorded chord progression: whole bars, and the chords played in
//! them, each at a position in its bar. It is a fixed-size `Copy` value (no heap), so the
//! engine thread records into one, plays one, and hands one to the control side through
//! a ring without allocating. The engine glue (recording, looping, the bar-line timing)
//! is `engine/looper.rs`; the memories and the app API are `session/looper.rs`.
//!
//! What the manuals leave open, and what yahaha does (docs/chord-looper.md):
//! - Recorded chord times snap to the nearest 16th note ([`SNAP`]). A chord played just
//!   before a bar line (anticipated) lands on the bar line.
//! - The loop is whole bars: the bars recording ran through, counting the bar in which
//!   recording stopped.
//! - At most [`MAX_EVENTS`] chords and [`MAX_BARS`] bars; recording stops when full.

use crate::theory::Chord;

/// Chord changes a sequence holds.
pub const MAX_EVENTS: usize = 128;
/// Bars a sequence holds.
pub const MAX_BARS: u16 = 64;
/// Positions in a bar are in 1/`LOOP_PPQ` quarter notes.
pub const LOOP_PPQ: u32 = 480;
/// Recorded chord times snap to this grid: a 16th note.
pub const SNAP: u32 = LOOP_PPQ / 4;

/// One chord change: chord `chord` (as fingered, before Keyboard transpose) from `at`
/// (1/`LOOP_PPQ` quarter notes after the start of bar `bar`, 0-based) on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoopEvent {
    pub bar: u16,
    pub at: u16,
    pub chord: Chord,
}

const NONE: LoopEvent = LoopEvent { bar: 0, at: 0, chord: Chord::new(0, 0) };

/// A chord sequence: `bars` whole bars and the chord changes in them, in time order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChordSeq {
    bars: u16,
    len: u16,
    events: [LoopEvent; MAX_EVENTS],
}

impl Default for ChordSeq {
    fn default() -> ChordSeq {
        ChordSeq::EMPTY
    }
}

impl ChordSeq {
    pub const EMPTY: ChordSeq = ChordSeq { bars: 0, len: 0, events: [NONE; MAX_EVENTS] };

    /// A sequence of `bars` bars from events in time order (tests, imports). Events past
    /// the end, out of order, or beyond `MAX_EVENTS` are dropped.
    pub fn from_events(bars: u16, events: &[LoopEvent]) -> ChordSeq {
        let mut s = ChordSeq::EMPTY;
        for &e in events {
            s.push(e);
        }
        s.finish(bars);
        s
    }

    /// Nothing to loop: no bars or no chords.
    pub fn is_empty(&self) -> bool {
        self.bars == 0 || self.len == 0
    }

    pub fn bars(&self) -> u16 {
        self.bars
    }

    pub fn events(&self) -> &[LoopEvent] {
        &self.events[..self.len as usize]
    }

    /// Full: no room for another chord change.
    pub fn is_full(&self) -> bool {
        self.len as usize >= MAX_EVENTS
    }

    pub fn clear(&mut self) {
        self.bars = 0;
        self.len = 0;
    }

    /// Add a chord change at the end. A change at the same position as the last one
    /// replaces it (the chord was re-fingered within one grid step); the same chord as the
    /// last one is no change. False when full or out of order.
    pub fn push(&mut self, e: LoopEvent) -> bool {
        if let Some(last) = self.len.checked_sub(1).map(|i| &mut self.events[i as usize]) {
            if (e.bar, e.at) < (last.bar, last.at) {
                return false;
            }
            if (e.bar, e.at) == (last.bar, last.at) {
                last.chord = e.chord;
                // Re-fingered back to the chord before it: no change at all.
                if self.len >= 2 && self.events[self.len as usize - 2].chord == e.chord {
                    self.len -= 1;
                }
                return true;
            }
            if last.chord == e.chord {
                return true;
            }
        }
        if self.is_full() {
            return false;
        }
        self.events[self.len as usize] = e;
        self.len += 1;
        true
    }

    /// Record `chord`, played `pos` (1/`LOOP_PPQ` quarter notes) into bar `bar` of a
    /// recording whose bars last `bar_len`: snapped to the 16th-note grid, where a chord
    /// that snaps to the bar's end belongs to the next bar's first beat.
    pub fn record(&mut self, bar: u16, pos: u32, bar_len: u32, chord: Chord) -> bool {
        let snapped = (pos + SNAP / 2) / SNAP * SNAP;
        let (bar, at) = if snapped >= bar_len.max(1) { (bar.saturating_add(1), 0) } else { (bar, snapped) };
        self.push(LoopEvent { bar, at: at.min(u16::MAX as u32) as u16, chord })
    }

    /// The recording is `bars` bars long: changes past the end are dropped.
    pub fn finish(&mut self, bars: u16) {
        self.bars = bars.min(MAX_BARS);
        while self.len > 0 && self.events[self.len as usize - 1].bar >= self.bars {
            self.len -= 1;
        }
        if self.len == 0 {
            self.bars = 0;
        }
    }

    /// The chord in force at the start of bar `bar` (the last change at or before it,
    /// wrapping round the loop): what a loop entered there follows.
    pub fn chord_at_bar(&self, bar: u16) -> Option<Chord> {
        let ev = self.events();
        ev.iter().rev().find(|e| e.bar < bar || (e.bar == bar && e.at == 0)).or(ev.last()).map(|e| e.chord)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_chord;

    fn c(name: &str) -> Chord {
        parse_chord(name).unwrap()
    }

    const BAR: u32 = 4 * LOOP_PPQ;

    #[test]
    fn records_on_a_sixteenth_grid() {
        let mut s = ChordSeq::EMPTY;
        s.record(0, 0, BAR, c("C"));
        // Slightly late for beat 3, slightly early for bar 2.
        s.record(0, 2 * LOOP_PPQ + 30, BAR, c("F"));
        s.record(0, BAR - 20, BAR, c("G"));
        s.finish(2);
        let ev: Vec<_> = s.events().iter().map(|e| (e.bar, e.at)).collect();
        assert_eq!(ev, [(0, 0), (0, 2 * LOOP_PPQ as u16), (1, 0)]);
        assert_eq!(s.bars(), 2);
    }

    #[test]
    fn a_refingered_chord_replaces_the_last_one() {
        let mut s = ChordSeq::EMPTY;
        s.record(0, 0, BAR, c("C"));
        s.record(0, 10, BAR, c("Cm"));
        assert_eq!(s.events().len(), 1);
        assert_eq!(s.events()[0].chord, c("Cm"));
        // The same chord again is no change.
        s.record(1, 0, BAR, c("Cm"));
        assert_eq!(s.events().len(), 1);
        // A change and back within one step: nothing left of it.
        s.record(2, 0, BAR, c("F"));
        s.record(2, 5, BAR, c("Cm"));
        assert_eq!(s.events().len(), 1);
    }

    #[test]
    fn finish_drops_changes_past_the_end() {
        let mut s = ChordSeq::EMPTY;
        s.record(0, 0, BAR, c("C"));
        s.record(3, BAR - 10, BAR, c("F")); // anticipates bar 5
        s.finish(4);
        assert_eq!(s.events().len(), 1);
        assert_eq!(s.bars(), 4);
        let mut e = ChordSeq::EMPTY;
        e.finish(3);
        assert!(e.is_empty());
        assert_eq!(e.bars(), 0);
    }

    #[test]
    fn full_and_out_of_order() {
        let mut s = ChordSeq::EMPTY;
        for i in 0..MAX_EVENTS as u16 + 4 {
            let ch = if i % 2 == 0 { c("C") } else { c("F") };
            s.push(LoopEvent { bar: i / 4, at: (i % 4) * 480, chord: ch });
        }
        assert!(s.is_full());
        assert!(!s.push(LoopEvent { bar: 0, at: 0, chord: c("G") }));
    }

    #[test]
    fn chord_at_bar_wraps() {
        let s = ChordSeq::from_events(
            4,
            &[
                LoopEvent { bar: 0, at: 0, chord: c("C") },
                LoopEvent { bar: 1, at: 960, chord: c("F") },
                LoopEvent { bar: 3, at: 0, chord: c("G") },
            ],
        );
        assert_eq!(s.chord_at_bar(0), Some(c("C")));
        assert_eq!(s.chord_at_bar(2), Some(c("F")));
        assert_eq!(s.chord_at_bar(3), Some(c("G")));
    }
}

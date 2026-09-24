//! Chord settling: while the band plays, a chord change (a new chord, or a Keyboard
//! transpose that moves the held one) does not reach the notes at once. It waits until
//! `process` runs, and then until the chord has held still for the chord-settle window
//! (`Engine::set_chord_settle`), and the band follows the chord it settled on, once.
//!
//! Why wait at all:
//! - **One engine wake, one revoice (#47).** A chord and a command handled at the same
//!   instant (the chord word and a Keyboard transpose, Stop, Break, chord release, a part
//!   toggle, all read in one wake) used to each move the notes: the chord restruck a note
//!   and the command moved or cut it at the same instant, a zero-length note. Now every
//!   input of a wake changes state only, and `process` re-voices once, after them all.
//!   This part needs no window: with the window at 0 the chord settles in the same wake.
//! - **Rolled chords (#65).** The recognizer publishes a chord on each key, so a chord
//!   whose keys land a few ms apart (F, then F7 3 ms later) is two chord changes. Each one
//!   re-pitched the notes, so a note attacked on the passing chord was cut and struck
//!   again a moment later. With a window, the band waits for the chord to hold still for
//!   that long (each change in the roll starts the wait again, but never past three
//!   windows after the first), and follows it once.
//!
//! What waits: while a chord is unsettled, the pattern's new notes on the parts that
//! follow chords are held back (the rhythm parts play on time, and so does everything
//! already sounding), and at the settle they start, with the settled chord, if the
//! pattern still holds them (`catch_up`). Notes sounding through the change are re-voiced
//! at the settle (`revoice`). The Retrigger Rules' "late chord" test still counts from
//! when the chord arrived, not from the settle, so a chord that lands just after the beat
//! takes the downbeat as before.
//!
//! The cost is latency, and only there: a chord struck on (or just before) a note of a
//! chord part delays that note by up to the window. A chord struck at least a window
//! ahead of the beat costs nothing. See docs/genos-features.md (Chord settle) for the
//! choice of the default.

use super::*;

/// The chord-settle window a session starts with (`Engine::set_chord_settle`), in ms.
pub const CHORD_SETTLE_DEFAULT_MS: u32 = 10;
/// The widest chord-settle window, in ms.
pub const CHORD_SETTLE_MAX_MS: u32 = 30;
/// A roll keeps the chord unsettled for at most this many windows after its first change.
const SETTLE_CAP: u64 = 3;

/// A chord change the band has not followed yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Unsettled {
    /// When the first change of this burst arrived (the "late chord" tests count from it).
    pub(super) first: u64,
    /// When the latest one did.
    pub(super) last: u64,
}

/// Where the pattern's held-back notes start: section slot `slot` (entered at
/// `sec_start`), event index `idx` on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Hold {
    pub(super) slot: usize,
    pub(super) sec_start: f64,
    pub(super) idx: usize,
}

impl Engine {
    /// The chord-settle window, in ns (0: a chord settles in the wake it arrives in).
    pub fn set_chord_settle(&mut self, ns: u64) {
        self.settle_ns = ns.min(CHORD_SETTLE_MAX_MS as u64 * 1_000_000);
    }

    pub fn chord_settle(&self) -> u64 {
        self.settle_ns
    }

    /// A chord change at `now` for the band to follow once it settles.
    pub(super) fn unsettle(&mut self, now: u64) {
        self.unsettled = Some(match self.unsettled {
            Some(u) => Unsettled { first: u.first, last: now.max(u.last) },
            None => Unsettled { first: now, last: now },
        });
    }

    /// When the chord change waiting settles.
    pub(super) fn settle_at(&self) -> Option<u64> {
        let u = self.unsettled?;
        Some((u.last + self.settle_ns).min(u.first + SETTLE_CAP * self.settle_ns))
    }

    /// The pattern's new notes on the chord parts wait for the chord to settle.
    #[inline]
    pub(super) fn holding(&self) -> bool {
        self.unsettled.is_some()
    }

    /// Note that the pattern's notes from event `idx` of the section playing on are held
    /// back (the first one only: the rest follow it).
    pub(super) fn hold_from(&mut self, idx: usize) {
        let here = |h: &Hold| h.slot == self.cur && h.sec_start == self.sec_start;
        if !self.hold.as_ref().is_some_and(here) {
            self.hold = Some(Hold { slot: self.cur, sec_start: self.sec_start, idx });
        }
    }

    /// The first held-back event of the section playing, if any.
    pub(super) fn held_from(&self) -> Option<usize> {
        self.hold.filter(|h| h.slot == self.cur && h.sec_start == self.sec_start).map(|h| h.idx)
    }

    /// Settle the chord change waiting, if its time has come: the band follows the chord
    /// once (`process` calls this before it plays anything due).
    pub(super) fn settle_due(&mut self, now: u64, sink: &mut impl Sink) {
        if self.settle_at().is_some_and(|t| now >= t) {
            self.settle(now, sink);
        }
    }

    /// The band follows the chord change waiting, now: it re-voices the notes sounding and
    /// starts the notes it held back.
    pub(super) fn settle(&mut self, now: u64, sink: &mut impl Sink) {
        let Some(u) = self.unsettled.take() else { return };
        let Some(played) = self.played else {
            self.hold = None;
            return;
        };
        let chord = shift_chord(played, self.transpose.keyboard);
        let prev = self.chord;
        self.chord = Some(chord);
        if self.running {
            if prev.is_some() {
                self.revoice(chord, u.first, now, sink);
            }
            self.catch_up(prev, chord, u.first, now, sink);
        }
        self.hold = None;
        self.on_chord(prev, now, sink);
    }

    /// The band stops (or leaves the notes) before a waiting chord change settled: the
    /// chord takes effect with nothing to re-voice.
    pub(super) fn settle_silently(&mut self, sink: &mut impl Sink) {
        let Some(u) = self.unsettled.take() else { return };
        self.hold = None;
        let Some(played) = self.played else { return };
        let prev = self.chord;
        self.chord = Some(shift_chord(played, self.transpose.keyboard));
        self.on_chord(prev, u.last, sink);
    }
}

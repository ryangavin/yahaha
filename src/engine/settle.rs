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
//! at the settle (`revoice`). A Chord Match Multi Pad's new notes wait too, band playing
//! or stopped (engine/multipad.rs, `MultiPadPlayer::set_hold`): a pad note is never
//! re-voiced, so one struck on the chord being replaced would keep it. The Retrigger Rules' "late chord" test still counts from
//! when the chord arrived, not from the settle, so a chord that lands just after the beat
//! takes the downbeat as before.
//!
//! What does not wait: an exact, machine-made chord change (the Chord Looper's playback,
//! and a chart's chords, iReal #98) is never rolled, so it goes through
//! `Engine::apply_chord_unsettled`, which settles at once.
//!
//! The cost is latency, and only there: a chord struck on (or just before) a note of a
//! chord part delays that note by up to the window (and `catch_up` drops a held-back note
//! with less of it left than it waited, one shorter than about twice the wait). A chord struck at least a window
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
    /// A chord was played in it (not only a Keyboard transpose): with the band stopped,
    /// Stop Accompaniment sounds it even if nothing sounds yet.
    pub(super) chord: bool,
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

    /// A chord change at `now` (a chord played, or only a Keyboard transpose) for the band
    /// to follow once it settles.
    pub(super) fn unsettle(&mut self, now: u64, chord: bool) {
        self.unsettled = Some(match self.unsettled {
            Some(u) => Unsettled { first: u.first, last: now.max(u.last), chord: u.chord || chord },
            None => Unsettled { first: now, last: now, chord },
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

    /// The style follows chord `played` (as fingered) at once, with no chord-settle window:
    /// the path for exact, machine-made chord changes that are never rolled, such as the
    /// Chord Looper's playback (looper.rs) and a chart's chords (chart.rs, with `held`
    /// false: no keys are held, so no Synchro Stop Window and no Retrigger). A keyboard
    /// chord goes through `set_chord` and waits for the window instead.
    ///
    /// It settles every change waiting (a Keyboard transpose too) before the notes of its
    /// own tick, so none of them are held back. Call it where no other input of the wake
    /// follows (from `process`, or a hook it runs): an input handled after it in the same
    /// wake would move the notes it just struck again (#47).
    pub(super) fn apply_chord_unsettled(&mut self, played: Chord, now: u64, sink: &mut impl Sink) {
        self.apply_chord_unsettled_from(played, true, now, sink);
    }

    /// `apply_chord_unsettled`; `held` as in `apply_chord_from`.
    pub(super) fn apply_chord_unsettled_from(&mut self, played: Chord, held: bool, now: u64, sink: &mut impl Sink) {
        self.apply_chord_from(played, held, now, sink);
        // Playing, or stopped under Stop Accompaniment (`apply_chord` left it unsettled).
        self.settle(now, sink);
    }

    /// Settle the chord change waiting, if its time has come: the band follows the chord
    /// once (`process` calls this before it plays anything due).
    pub(super) fn settle_due(&mut self, now: u64, sink: &mut impl Sink) {
        if self.settle_at().is_some_and(|t| now >= t) {
            self.settle(now, sink);
        }
    }

    /// The band follows the chord change waiting, now: it re-voices the notes sounding and
    /// starts the notes it held back. With the band stopped, Stop Accompaniment sounds the
    /// chord (a transpose alone moves its notes only if they still sound).
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
            // The same chord struck again (after letting go), or a roll or transpose that
            // came back to it, is no chord change: the Retrigger Rules move nothing (a
            // Pitch Shift to Root bass would otherwise jump to the root at every
            // re-strike). The notes held back still start.
            if prev.is_some_and(|p| p != chord) {
                self.revoice(chord, u.first, now, sink);
            }
            self.catch_up(prev, chord, u.first, now, sink);
        } else if self.stop_acmp != StopAcmp::Off && (u.chord || self.sounding.iter().any(|n| n.active && n.src == STOP_ACMP_SRC)) {
            self.sound_stop_acmp(chord, now, sink);
        }
        self.hold = None;
        // (A Sync Start chord was the chord from the start: it only settles here.)
        if prev != Some(chord) {
            self.on_chord(prev, now, sink);
        }
    }

    /// The band stops before a waiting chord change settled: the chord takes effect with
    /// nothing to re-voice. (Stopped, a change Stop Accompaniment waits on is left to
    /// settle.)
    pub(super) fn settle_silently(&mut self, sink: &mut impl Sink) {
        if !self.running {
            return;
        }
        let Some(u) = self.unsettled.take() else { return };
        self.hold = None;
        let Some(played) = self.played else { return };
        let prev = self.chord;
        let chord = shift_chord(played, self.transpose.keyboard);
        self.chord = Some(chord);
        if prev != Some(chord) {
            self.on_chord(prev, u.last, sink);
        }
    }
}

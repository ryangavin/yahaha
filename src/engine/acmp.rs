//! The [ACMP] switch (OM p.44, p.47): Auto Accompaniment on or off.
//!
//! With ACMP off there is no chord section:
//! - the band plays its rhythm parts only (no chord, as before the first chord);
//! - chords from the keyboard are ignored, and so are chord-section releases (no Sync Stop);
//! - Sync Start starts on any key (`any_key`, sent by the input thread);
//! - the keyboard plays the Right parts over its whole range, or Left below the split (the
//!   input thread reads `Shared::acmp`).
//!
//! An OTS recall and Chord Looper REC turn it back on (OM p.66, RM p.15). Turned on, the
//! chord parts wait for the next chord played.

use super::*;

/// Engine-side ACMP state. `off` rather than `on`, so the default (`Features::default`) is
/// ACMP on, as a Genos powers up.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Acmp {
    off: bool,
}

impl Engine {
    /// ACMP is on: chords from the keyboard drive the band.
    #[inline]
    pub fn acmp(&self) -> bool {
        !self.features.acmp.off
    }

    /// ACMP on or off. Off: the chord goes (the chord parts end their notes and wait; the
    /// rhythm plays on), and Sync Stop and Stop Accompaniment's notes with it.
    pub fn set_acmp(&mut self, on: bool, sink: &mut impl Sink) {
        if on == self.acmp() {
            return;
        }
        self.features.acmp.off = !on;
        if on {
            return;
        }
        self.chord = None;
        self.played = None;
        self.unsettled = None;
        self.hold = None;
        self.sync_stop = false;
        self.off_where(sink, |n| follows_chords(n.dest) || n.src == STOP_ACMP_SRC);
    }

    /// A key went down with ACMP off: Sync Start starts the rhythm (OM p.44).
    pub fn any_key(&mut self, now: u64, sink: &mut impl Sink) {
        if !self.acmp() && !self.running && self.sync_armed {
            self.start(now, sink);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: u64 = 1_000_000;

    #[derive(Default)]
    struct Rec(Vec<Vec<u8>>);
    impl Sink for Rec {
        fn send(&mut self, m: &[u8]) {
            self.0.push(m.to_vec());
        }
    }

    fn engine() -> Option<Engine> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/SlowWalker.T552.sty");
        if !p.exists() {
            eprintln!("corpus missing; skipping");
            return None;
        }
        Some(Engine::new(Box::new(Prepared::new(&crate::sff::Style::load(&p).unwrap()))))
    }

    fn play(e: &mut Engine, r: &mut Rec, from: u64, to: u64) {
        let mut now = from;
        while now <= to {
            e.process(now, r);
            now += 5 * MS;
        }
    }

    /// Note-ons on the parts that follow chords, and on the rhythm parts.
    fn ons(r: &Rec) -> (usize, usize) {
        let on = |m: &Vec<u8>| m.len() == 3 && m[0] & 0xF0 == 0x90 && m[2] > 0;
        let chordal = r.0.iter().filter(|m| on(m) && follows_chords(m[0] & 0x0F)).count();
        let rhythm = r.0.iter().filter(|m| on(m) && (8..16).contains(&(m[0] & 0x0F)) && !follows_chords(m[0] & 0x0F)).count();
        (chordal, rhythm)
    }

    #[test]
    fn acmp_is_on_by_default() {
        let Some(e) = engine() else { return };
        assert!(e.acmp());
        assert!(e.snapshot(0).acmp);
    }

    /// ACMP off: Sync Start fires on any key, the rhythm plays alone, chords change nothing.
    #[test]
    fn acmp_off_plays_rhythm_only_and_ignores_chords() {
        let Some(mut e) = engine() else { return };
        let mut r = Rec::default();
        e.button(Button::SetAcmp(false), 0, &mut r);
        assert!(e.starts_on_chord(), "Sync Start is armed");
        e.set_chord(Chord::new(0, 0), 0, &mut r);
        assert!(!e.running, "a chord doesn't start it");
        e.any_key(MS, &mut r);
        assert!(e.running, "any key does");
        e.set_chord(Chord::new(7, 0), 2 * MS, &mut r);
        play(&mut e, &mut r, 2 * MS, 4_000 * MS);
        assert_eq!(e.snapshot(0).chord, None);
        let (chordal, rhythm) = ons(&r);
        assert_eq!(chordal, 0);
        assert!(rhythm > 10);
    }

    /// Turned off while playing: the chord parts stop, the rhythm plays on; back on, they
    /// come in with the next chord.
    #[test]
    fn acmp_off_while_playing_and_back_on() {
        let Some(mut e) = engine() else { return };
        let mut r = Rec::default();
        e.set_chord(Chord::new(0, 0), 0, &mut r);
        play(&mut e, &mut r, 0, 2_000 * MS);
        assert!(ons(&r).0 > 0);
        e.button(Button::Acmp, 2_000 * MS, &mut r);
        assert!(!e.acmp());
        let from = r.0.len();
        play(&mut e, &mut r, 2_000 * MS, 6_000 * MS);
        let after = Rec(r.0[from..].to_vec());
        assert_eq!(ons(&after).0, 0);
        assert!(ons(&after).1 > 0, "the rhythm plays on");
        // Sync Stop can't be on with ACMP off: a release doesn't stop the band.
        e.chord_released(6_000 * MS, &mut r);
        assert!(e.running);
        e.button(Button::Acmp, 6_000 * MS, &mut r);
        e.set_chord(Chord::new(5, 0), 6_000 * MS, &mut r);
        let from = r.0.len();
        play(&mut e, &mut r, 6_000 * MS, 10_000 * MS);
        assert!(ons(&Rec(r.0[from..].to_vec())).0 > 0, "the chord parts are back");
    }

    /// Chord Looper REC turns ACMP on (RM p.15).
    #[test]
    fn looper_rec_turns_acmp_on() {
        let Some(mut e) = engine() else { return };
        e.button(Button::SetAcmp(false), 0, &mut Rec::default());
        e.looper_rec();
        assert!(e.acmp());
    }
}

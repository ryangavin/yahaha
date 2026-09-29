//! Unison (a PSR-SX feature, not a Genos2 one: docs/genos-features.md §C.9): while it is
//! engaged, each right-hand key the player strikes also sounds on the Style's pitched
//! accompaniment parts, in rhythm with the player, and those parts' own patterns rest.
//!
//! - **Bass** plays the chord's root (its on-bass note, if it has one) in the bass range,
//!   or, with Unison Type Melody, the played key folded into the bass range. With no chord
//!   yet it plays the played key, folded.
//! - **Chord 1, Chord 2, Pad** play the chord's tones voiced just below the played key
//!   (the played key alone with no chord).
//! - **Phrase 1, Phrase 2** double the played key.
//! - The drums never play along (Accent is their feature).
//!
//! Note-offs follow the player's: each voice is a `Sounding` with source `UNISON_SRC` and
//! the player's key as its source key, in the engine's preallocated voice array. It works
//! with the band stopped or playing; playing, the parts' pattern notes are cut when Unison
//! engages and rest while it lasts (`Engine::unison_mutes`), then come back at their next
//! notes.
//!
//! Engaged = latched (the app's toggle, a Toggle pedal) or held (a Hold pedal, the
//! assignable function "Unison").

use super::*;

/// Pseudo source channel for Unison notes (`Sounding::src`).
pub(super) const UNISON_SRC: u8 = 254;

/// What the Bass part plays in Unison.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UnisonType {
    /// The chord's root (or on-bass note): the band's bass follows the harmony.
    #[default]
    Root,
    /// The played key, folded into the bass range: the whole band plays the line.
    Melody,
}

/// Engine-side Unison state.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Unison {
    latched: bool,
    held: bool,
    pub(super) ty: UnisonType,
}

impl Unison {
    #[inline]
    fn on(self) -> bool {
        self.latched || self.held
    }
}

/// The lowest key of the bass range Unison's bass plays in (C2..B2).
const BASS_LOW: u8 = 36;
/// The chord tones voiced under the played key: at most this many.
const MAX_TONES: usize = 4;

/// `key` folded into the octave from `low`.
#[inline]
fn fold(key: u8, low: u8) -> u8 {
    low + (key + 12 - low % 12) % 12
}

impl Engine {
    /// Unison is engaged.
    #[inline]
    pub fn unison(&self) -> bool {
        self.features.unison.on()
    }

    /// The latched Unison switch (the app's toggle).
    pub fn unison_latched(&self) -> bool {
        self.features.unison.latched
    }

    pub fn unison_type(&self) -> UnisonType {
        self.features.unison.ty
    }

    pub(super) fn set_unison_type(&mut self, ty: UnisonType) {
        self.features.unison.ty = ty;
    }

    /// Latch Unison on or off.
    pub(super) fn set_unison_latched(&mut self, on: bool, sink: &mut impl Sink) {
        self.set_unison_state(Unison { latched: on, ..self.features.unison }, sink);
    }

    /// A Hold pedal given Unison went down (`on`) or up.
    pub(super) fn set_unison_held(&mut self, on: bool, sink: &mut impl Sink) {
        self.set_unison_state(Unison { held: on, ..self.features.unison }, sink);
    }

    fn set_unison_state(&mut self, u: Unison, sink: &mut impl Sink) {
        let was = self.unison();
        self.features.unison = u;
        match (was, u.on()) {
            // Engaged: the pitched parts' pattern notes stop; they rest until it ends.
            (false, true) => self.off_where(sink, |n| n.src < 16 && follows_chords(n.dest)),
            // Released: the player's line stops on the band; the patterns come back at
            // their next notes.
            (true, false) => self.off_where(sink, |n| n.src == UNISON_SRC),
            _ => {}
        }
    }

    /// A pattern note on `dest` rests: Unison has the part.
    #[inline]
    pub(super) fn unison_mutes(&self, src: u8, dest: u8) -> bool {
        src < 16 && self.unison() && follows_chords(dest)
    }

    /// A right-hand key (as played, before transpose) went down with `vel` (> 0), or up
    /// (`vel` 0). Down, with Unison engaged, it sounds on the pitched Style parts; up, what
    /// it sounded there ends (whether Unison is still engaged or not).
    pub fn unison_key(&mut self, key: u8, vel: u8, now: u64, sink: &mut impl Sink) {
        let key = key & 0x7F;
        // A key struck again ends its earlier strike first.
        self.off_where(sink, |n| n.src == UNISON_SRC && n.src_key == key);
        if vel == 0 || !self.unison() {
            return;
        }
        let played = shift_key(key, self.transpose.keyboard);
        let chord = self.chord.filter(|c| c.ty != CANCEL);
        let audible = self.audible();
        let slot = self.cur as u8;
        for dest in 8..16u8 {
            if !follows_chords(dest) || audible & (1 << (dest - 8)) == 0 {
                continue;
            }
            match dest {
                BASS_CH => {
                    let pc = match (self.features.unison.ty, chord) {
                        (UnisonType::Root, Some(c)) => c.bass.unwrap_or(c.root),
                        _ => played % 12,
                    };
                    let out = self.master(dest, fold(pc, BASS_LOW));
                    self.note_on(UNISON_SRC, key, dest, out, vel, slot, now, sink);
                }
                // Phrase 1 and 2 double the line.
                14 | 15 => {
                    let out = self.master(dest, played);
                    self.note_on(UNISON_SRC, key, dest, out, vel, slot, now, sink);
                }
                // Chord 1, Chord 2, Pad: the chord, voiced just below the played key.
                _ => match chord {
                    Some(c) => {
                        for &t in yahaha_core::theory::chord_tones(c.ty).iter().take(MAX_TONES) {
                            let pc = (c.root + t) % 12;
                            let below = (played % 12 + 12 - pc) % 12;
                            let k = if played >= below { played - below } else { played };
                            let out = self.master(dest, k);
                            self.note_on(UNISON_SRC, key, dest, out, vel, slot, now, sink);
                        }
                    }
                    None => {
                        let out = self.master(dest, played);
                        self.note_on(UNISON_SRC, key, dest, out, vel, slot, now, sink);
                    }
                },
            }
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
        let p = yahaha_sff::library::corpus_dir().join("MOX_v2/SlowWalker.T552.sty");
        if !p.exists() {
            eprintln!("corpus missing; skipping");
            return None;
        }
        Some(Engine::new(Box::new(Prepared::new(&yahaha_sff::sff::Style::load(&p).unwrap()))))
    }

    fn play(e: &mut Engine, r: &mut Rec, from: u64, to: u64) {
        let mut now = from;
        while now <= to {
            e.process(now, r);
            now += 5 * MS;
        }
    }

    fn ons(r: &[Vec<u8>], ch: u8) -> Vec<u8> {
        r.iter().filter(|m| m.len() == 3 && m[0] == 0x90 | ch && m[2] > 0).map(|m| m[1]).collect()
    }

    fn offs(r: &[Vec<u8>], ch: u8) -> Vec<u8> {
        r.iter().filter(|m| m.len() == 3 && (m[0] == 0x80 | ch || (m[0] == 0x90 | ch && m[2] == 0))).map(|m| m[1]).collect()
    }

    fn unison_voices(e: &Engine) -> usize {
        e.sounding.iter().filter(|s| s.active && s.src == UNISON_SRC).count()
    }

    #[test]
    fn fold_keeps_the_pitch_class() {
        for k in 0..128u8 {
            let f = fold(k, BASS_LOW);
            assert!((BASS_LOW..BASS_LOW + 12).contains(&f));
            assert_eq!(f % 12, k % 12);
        }
    }

    /// Stopped: a key sounds the root on the Bass, the chord under the key on the chord
    /// parts, the key on the phrases, nothing on the drums; its release ends them all.
    #[test]
    fn stopped_style_follows_note_on_and_off() {
        let Some(mut e) = engine() else { return };
        let mut r = Rec::default();
        if e.snapshot(0).sync_armed {
            e.button(Button::SyncStart, 0, &mut r);
        }
        e.set_chord(Chord::new(0, 0), 0, &mut r); // C major
        assert!(!e.running);
        r.0.clear();
        e.unison_key(76, 100, MS, &mut r);
        assert!(r.0.is_empty(), "off: nothing");
        e.button(Button::SetUnison(true), MS, &mut r);
        e.unison_key(76, 100, 2 * MS, &mut r); // E5
        assert_eq!(ons(&r.0, BASS_CH), [36], "the root, C2");
        let mut pad = ons(&r.0, 13);
        pad.sort();
        assert_eq!(pad, [67, 72, 76], "C E G voiced under E5");
        assert_eq!(ons(&r.0, 14), [76], "the phrase doubles the line");
        assert!((8..10).all(|ch| ons(&r.0, ch).is_empty()), "no drums");
        let n = unison_voices(&e);
        assert!(n >= 5);
        e.unison_key(76, 0, 3 * MS, &mut r);
        assert_eq!(unison_voices(&e), 0);
        assert_eq!(offs(&r.0, BASS_CH), [36]);
        assert_eq!(offs(&r.0, 13).len(), 3);
    }

    /// Melody type: the Bass plays the line folded into the bass range.
    #[test]
    fn melody_type_puts_the_line_on_the_bass() {
        let Some(mut e) = engine() else { return };
        let mut r = Rec::default();
        e.set_chord(Chord::new(0, 0), 0, &mut r);
        e.button(Button::SetUnisonType(UnisonType::Melody), 0, &mut r);
        e.button(Button::SetUnison(true), 0, &mut r);
        r.0.clear();
        e.unison_key(79, 90, MS, &mut r); // G5
        assert_eq!(ons(&r.0, BASS_CH), [43], "G2");
    }

    /// A Hold pedal: on while held, and the latched toggle keeps it on past the release.
    #[test]
    fn held_and_latched_combine() {
        let Some(mut e) = engine() else { return };
        let mut r = Rec::default();
        e.button(Button::UnisonHeld(true), 0, &mut r);
        assert!(e.unison() && !e.unison_latched());
        e.unison_key(72, 90, MS, &mut r);
        assert!(unison_voices(&e) > 0);
        e.button(Button::UnisonHeld(false), 2 * MS, &mut r);
        assert!(!e.unison());
        assert_eq!(unison_voices(&e), 0, "letting go ends the line");
        e.button(Button::Unison, 3 * MS, &mut r);
        e.button(Button::UnisonHeld(true), 3 * MS, &mut r);
        e.button(Button::UnisonHeld(false), 4 * MS, &mut r);
        assert!(e.unison(), "latched stays on");
        assert!(e.snapshot(0).unison);
    }

    /// Playing: engaging cuts the pitched parts' pattern notes, they rest while it lasts
    /// (the drums play on), and come back when it ends.
    #[test]
    fn mutes_and_resumes_the_patterns() {
        let Some(mut e) = engine() else { return };
        let mut r = Rec::default();
        e.set_chord(Chord::new(0, 0), 0, &mut r);
        play(&mut e, &mut r, 0, 2_000 * MS);
        assert!(ons(&r.0, BASS_CH).len() + (11..16).map(|c| ons(&r.0, c).len()).sum::<usize>() > 0);
        e.button(Button::SetUnison(true), 2_000 * MS, &mut r);
        assert!(!e.sounding.iter().any(|s| s.active && s.src < 16 && follows_chords(s.dest)), "the pattern notes stop");
        let from = r.0.len();
        e.unison_key(72, 100, 2_000 * MS, &mut r);
        play(&mut e, &mut r, 2_005 * MS, 6_000 * MS);
        let during = &r.0[from..];
        assert_eq!(ons(during, BASS_CH), [36], "only the player's note on the bass");
        assert!((8..10).map(|c| ons(during, c).len()).sum::<usize>() > 0, "the drums play on");
        e.unison_key(72, 0, 6_000 * MS, &mut r);
        e.button(Button::SetUnison(false), 6_000 * MS, &mut r);
        let from = r.0.len();
        play(&mut e, &mut r, 6_000 * MS, 12_000 * MS);
        let after = &r.0[from..];
        assert!((10..16).map(|c| ons(after, c).len()).sum::<usize>() > 0, "the patterns are back");
    }

    /// A release after Unison was let go still ends nothing twice (the voices went with it),
    /// and a key held through a disengage sounds nothing on its release.
    #[test]
    fn release_after_disengage_is_quiet() {
        let Some(mut e) = engine() else { return };
        let mut r = Rec::default();
        e.button(Button::SetUnison(true), 0, &mut r);
        e.unison_key(60, 90, MS, &mut r);
        e.button(Button::SetUnison(false), 2 * MS, &mut r);
        let n = r.0.len();
        e.unison_key(60, 0, 3 * MS, &mut r);
        assert_eq!(r.0.len(), n);
    }
}

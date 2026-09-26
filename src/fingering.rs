//! Chord fingering types (RM p.9, OM p.46): a dispatch layer around the Fingered
//! recognizer in `theory`. Runs on the CoreMIDI input thread, so everything here works
//! on the held-key array in place and never allocates.
//!
//! Where the manuals leave a rule open, this module picks one (see each function).
//! Everything below three pitch classes is decided here, not by the recognizer, whose
//! Fingered table has no chords of one or two notes other than 1+5 and 1+8.
//!
//! - **Fingered**: the On Bass reading with the bass on the root; a bass note that is
//!   not a chord tone makes no chord.
//! - **Single Finger**: the highest key is the root. Any other black key below it makes
//!   the chord minor, any white key makes it a 7th, both make it m7. Octave doublings of
//!   the root are ignored.
//! - **Multi Finger**: three or more pitch classes are read as Fingered (not Cancel),
//!   unless the Fingered reading leaves out a chord note and the keys are a close Single
//!   Finger shape; otherwise a Single Finger shape (at most one black and one white
//!   pitch class, within an octave below a root key) is read as Single Finger. Two
//!   pitch classes are Single Finger, except a perfect fifth up from the lowest key,
//!   which is 1+5. One pitch class is the major chord.
//! - **AI Fingered**: the lowest key is the bass (#194). Three or more pitch classes are
//!   read as Fingered On Bass. Two keep the previous chord over the lower note when the
//!   upper one belongs to it (C then B+C is C/B, Am then G+A is Am/G); otherwise they
//!   are inferred from their interval (`infer_dyad`), again over the lower note (G#-D is
//!   E7/G#), and a 2nd keeps the chord as it is. One keeps the previous chord as it is
//!   if it belongs to it, and is otherwise the major chord on it (1+8 if doubled).
//! - **Full Keyboard**: the whole keyboard is read with On Bass rules (the lowest key is
//!   the bass), with melody set aside (`full`). It needs three pitch classes, so a
//!   melody alone never changes the chord, and 1+5 and 1+8 never occur. Sync Stop is
//!   unavailable (the engine refuses it).
//! - **AI Full Keyboard**: Full Keyboard, else AI inference on the lowest two pitch
//!   classes (single notes are melody and never change the chord). 9th, 11th and 13th
//!   chords are reduced to the chord without the tension.
//! - Chord Cancel only exists in Fingered, Fingered On Bass and AI Fingered.
//! - Fingered and Multi Finger always put the bass on the root.

use crate::theory::{chord_tones, Chord, Recognizer, CANCEL};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Fingering {
    SingleFinger,
    MultiFinger,
    Fingered,
    #[default]
    FingeredOnBass,
    AiFingered,
    FullKeyboard,
    AiFullKeyboard,
}

impl Fingering {
    /// Display order, as in the Genos Split & Fingering screen.
    pub const ALL: [Fingering; 7] = [
        Fingering::SingleFinger,
        Fingering::Fingered,
        Fingering::FingeredOnBass,
        Fingering::MultiFinger,
        Fingering::AiFingered,
        Fingering::FullKeyboard,
        Fingering::AiFullKeyboard,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Fingering::SingleFinger => "Single Finger",
            Fingering::MultiFinger => "Multi Finger",
            Fingering::Fingered => "Fingered",
            Fingering::FingeredOnBass => "Fingered On Bass",
            Fingering::AiFingered => "AI Fingered",
            Fingering::FullKeyboard => "Full Keyboard",
            Fingering::AiFullKeyboard => "AI Full Keyboard",
        }
    }

    /// Parse a CLI name: "single", "multi", "fingered", "on-bass", "ai", "full", "ai-full"
    /// (case, spaces, dashes and underscores are ignored; full names work too).
    pub fn parse(s: &str) -> Option<Fingering> {
        let k: String = s.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_ascii_lowercase();
        Some(match k.as_str() {
            "single" | "singlefinger" => Fingering::SingleFinger,
            "multi" | "multifinger" => Fingering::MultiFinger,
            "fingered" => Fingering::Fingered,
            "onbass" | "fingeredonbass" => Fingering::FingeredOnBass,
            "ai" | "aifingered" => Fingering::AiFingered,
            "full" | "fullkeyboard" => Fingering::FullKeyboard,
            "aifull" | "aifullkeyboard" => Fingering::AiFullKeyboard,
            _ => return None,
        })
    }

    pub fn to_u8(self) -> u8 {
        Fingering::ALL.iter().position(|&f| f == self).unwrap() as u8
    }

    pub fn from_u8(v: u8) -> Fingering {
        Fingering::ALL[v as usize % Fingering::ALL.len()]
    }

    /// Next type in display order (the TUI cycles with one key).
    pub fn next(self) -> Fingering {
        Fingering::from_u8(self.to_u8() + 1)
    }

    /// Chords are detected over the whole keyboard, ignoring the split.
    pub fn full_keyboard(self) -> bool {
        matches!(self, Fingering::FullKeyboard | Fingering::AiFullKeyboard)
    }

    /// Sync Stop cannot be turned on with Full or AI Full Keyboard (OM p.66).
    pub fn allows_sync_stop(self) -> bool {
        !self.full_keyboard()
    }

    /// Chord Cancel is available only in Fingered, Fingered On Bass and AI Fingered (OM p.46).
    pub fn allows_cancel(self) -> bool {
        matches!(self, Fingering::Fingered | Fingering::FingeredOnBass | Fingering::AiFingered)
    }
}

/// Held keys summarised: pitch-class mask, lowest and highest key, distinct pitch
/// classes, and the number of keys.
#[derive(Clone, Copy)]
struct Scan {
    mask: u16,
    low: u8,
    high: u8,
    n: u32,
    keys: u32,
}

/// Summarise the held keys in `lo..hi`.
fn scan(held: &[bool; 128], lo: usize, hi: usize) -> Option<Scan> {
    let mut mask = 0u16;
    let mut low = None;
    let mut high = 0u8;
    let mut keys = 0;
    for (k, &h) in held.iter().enumerate().take(hi).skip(lo) {
        if h {
            let k = k as u8;
            mask |= 1 << (k % 12);
            low.get_or_insert(k);
            high = k;
            keys += 1;
        }
    }
    Some(Scan { mask, low: low?, high, n: mask.count_ones(), keys })
}

fn is_black(key: u8) -> bool {
    matches!(key % 12, 1 | 3 | 6 | 8 | 10)
}

fn tones_mask(c: Chord) -> u16 {
    let m = chord_tones(c.ty).iter().fold(0u16, |m, &t| m | 1 << ((c.root + t) % 12));
    m | c.bass.map_or(0, |b| 1 << (b % 12))
}

fn root_bass(c: Chord) -> Chord {
    Chord { bass: None, ..c }
}

/// A slash chord whose bass is not one of the chord's tones (C/F#).
fn foreign_bass(c: Chord) -> bool {
    c.bass.is_some_and(|b| tones_mask(root_bass(c)) & (1 << (b % 12)) == 0)
}

/// The one call into the Fingered On Bass recognizer. The key count makes 1+8.
#[inline]
fn fingered(rec: &Recognizer, s: Scan) -> Option<Chord> {
    rec.recognize_keys(s.mask, s.low % 12, s.keys, true)
}

/// Plain Fingered: the root is the bass, so a bass note outside the chord makes no
/// chord (Fingered On Bass reads F#-C-E-G as C/F#).
fn plain(rec: &Recognizer, s: Scan) -> Option<Chord> {
    rec.recognize_keys(s.mask, s.low % 12, s.keys, false)
}

/// Detect the chord for the held keys. `held` is the chord section, or the whole keyboard
/// for the Full Keyboard types (which also use `split`: keys at or below it are the left
/// hand). `prev` is the chord in force. Returns None to leave the chord unchanged (a Some
/// equal to `prev` also changes nothing).
pub fn detect(rec: &Recognizer, mode: Fingering, held: &[bool; 128], split: u8, prev: Option<Chord>) -> Option<Chord> {
    let s = scan(held, 0, 128)?;
    let c = match mode {
        Fingering::FingeredOnBass => fingered(rec, s),
        Fingering::Fingered => plain(rec, s),
        Fingering::SingleFinger => Some(single(held, s)),
        Fingering::MultiFinger => multi(rec, held, s),
        Fingering::AiFingered => ai(rec, s, prev),
        Fingering::FullKeyboard => full(rec, held, split),
        Fingering::AiFullKeyboard => ai_full(rec, held, split, prev).map(no_tensions),
    };
    c.filter(|c| c.ty != CANCEL || mode.allows_cancel())
}

/// Whether the keys held, with the chord they give the same as the chord before, play
/// that chord again (after every chord key was up). In AI Full Keyboard only three notes
/// or more do: a single note or a dyad that fits the chord is melody (#107), so a
/// two-note figure in the right hand does not restart a Retrigger loop or time a Synchro
/// Stop. Every other type: always.
pub fn restrikes(rec: &Recognizer, mode: Fingering, held: &[bool; 128], split: u8) -> bool {
    mode != Fingering::AiFullKeyboard || full(rec, held, split).is_some()
}

/// Single Finger: root = highest key; black key below = m, white = 7, both = m7.
fn single(held: &[bool; 128], s: Scan) -> Chord {
    let root = s.high % 12;
    let (mut black, mut white) = (false, false);
    for k in s.low..s.high {
        if held[k as usize] && k % 12 != root {
            if is_black(k) {
                black = true;
            } else {
                white = true;
            }
        }
    }
    let ty = match (black, white) {
        (false, false) => MAJOR,
        (true, false) => MINOR,
        (false, true) => SEVENTH,
        (true, true) => MINOR7,
    };
    Chord::new(root, ty)
}

/// A Single Finger shape: besides the root (the highest key's pitch class) at most one
/// black and one white pitch class, each key of them less than `reach` semitones below
/// a held root key. Octave doublings of the root count, so Bb-C-C' is Cm.
fn single_shape(held: &[bool; 128], s: Scan, reach: u8) -> bool {
    let root = s.high % 12;
    let (mut black, mut white) = (0u16, 0u16);
    for k in s.low..s.high {
        if held[k as usize] && k % 12 != root {
            let up = (root + 12 - k % 12) % 12;
            if up >= reach || !held[(k + up) as usize] {
                return false;
            }
            if is_black(k) {
                black |= 1 << (k % 12);
            } else {
                white |= 1 << (k % 12);
            }
        }
    }
    black.count_ones() <= 1 && white.count_ones() <= 1
}

fn multi(rec: &Recognizer, held: &[bool; 128], s: Scan) -> Option<Chord> {
    if s.n >= 3 {
        // Fingered first, so C-Eb-G stays Cm. A reading that leaves out a chord note
        // loses to the close Single Finger shape (a black and a white key within a minor
        // third below the root): Bb-C-C# is C#m7, not Bbm(add9) without its fifth.
        return match plain(rec, s).filter(|c| c.ty != CANCEL) {
            Some(c) if tones_mask(c) & !s.mask == 0 || !single_shape(held, s, 4) => Some(c),
            _ if single_shape(held, s, 12) => Some(single(held, s)),
            _ => None,
        };
    }
    // Two pitch classes: a perfect fifth up from the lowest key is Fingered 1+5 (C-G);
    // anything else is Single Finger (G-C is C7). One pitch class is the major chord
    // (the root key only), octave doublings included.
    if s.n == 2 && s.mask & (1 << ((s.low + 7) % 12)) != 0 {
        return Some(Chord::new(s.low % 12, ONE_FIVE));
    }
    Some(single(held, s))
}

const MAJOR: u8 = 0;
const MAJ7: u8 = 2;
const MINOR: u8 = 8;
const MINOR7: u8 = 10;
const SEVENTH: u8 = 19;
const ONE_EIGHT: u8 = 30;
const ONE_FIVE: u8 = 31;

/// Keep `prev` while every held note is one of its tones (or its bass).
fn keep(s: Scan, prev: Option<Chord>) -> Option<Chord> {
    prev.filter(|&p| p.ty != CANCEL && s.mask & !tones_mask(p) == 0)
}

/// `c` over the pitch class `bass` (no slash when it is the root).
fn over(c: Chord, bass: u8) -> Chord {
    Chord { bass: (bass != c.root).then_some(bass), ..c }
}

/// AI Fingered, with the lowest key as the bass (#194). Three or more pitch classes are
/// Fingered On Bass. Two keep the previous chord over the lower note when the upper
/// note is one of its tones (C then B+C is C/B), and are otherwise inferred by their
/// interval over the lower note; a 2nd is a passing note and changes nothing. A lone
/// pitch class keeps the previous chord unchanged when it belongs to it, and is otherwise
/// the major chord on it (1+8 when held in more than one octave).
fn ai(rec: &Recognizer, s: Scan, prev: Option<Chord>) -> Option<Chord> {
    let low = s.low % 12;
    match s.n {
        3.. => fingered(rec, s),
        2 => {
            let upper = s.mask & !(1 << low);
            match prev.filter(|&p| p.ty != CANCEL && upper & !tones_mask(root_bass(p)) == 0) {
                Some(p) => Some(over(p, low)),
                // infer_dyad gives `prev` back only for a 2nd (anything that fits `prev`
                // was kept above): that passing note leaves the chord as it is.
                None => infer_dyad(s, prev).map(|c| if Some(c) == prev { c } else { over(c, low) }),
            }
        }
        _ => keep(s, prev).or(Some(Chord::new(low, if s.keys >= 2 { ONE_EIGHT } else { MAJOR }))),
    }
}

/// Distance between two roots around the circle of fifths (0..=6).
fn fifths(a: u8, b: u8) -> u8 {
    let d = (b + 12 - a) % 12 * 7 % 12;
    d.min(12 - d)
}

/// Two pitch classes not in the previous chord. `a` is the lower key's pitch class.
fn infer_dyad(s: Scan, prev: Option<Chord>) -> Option<Chord> {
    let a = s.low % 12;
    let b = (0..12u8).find(|&p| p != a && s.mask & (1 << p) != 0)?;
    Some(match (b + 12 - a) % 12 {
        3 => Chord::new(a, MINOR),    // minor 3rd: m
        4 => Chord::new(a, MAJOR),    // major 3rd: major
        8 => Chord::new(b, MAJOR),    // minor 6th = major 1st inversion (E-C = C)
        9 => Chord::new(b, MINOR),    // major 6th = minor 1st inversion (C-A = Am)
        10 => Chord::new(a, SEVENTH), // minor 7th: 7
        11 => Chord::new(a, MAJ7),    // major 7th: maj7
        7 => Chord::new(a, ONE_FIVE), // fifth: 1+5, as Fingered
        5 => Chord::new(b, ONE_FIVE), // fourth: 1+5 inverted (G-C = C1+5)
        6 => {
            // Tritone: the 3rd and 7th of a dominant 7th, which fits two roots a tritone
            // apart (B-F is G7 or Db7). Take the one nearer the previous chord on the
            // circle of fifths (G7 over C); with no preference the lower note is the 3rd.
            let (lo3, hi3) = ((a + 8) % 12, (b + 8) % 12);
            let root = match prev {
                Some(p) if p.ty != CANCEL && fifths(p.root, hi3) < fifths(p.root, lo3) => hi3,
                _ => lo3,
            };
            Chord::new(root, SEVENTH)
        }
        _ => return prev, // a 2nd: a passing note, keep the chord
    })
}

/// A Full Keyboard reading (On Bass rules, no Cancel), and whether its bass is foreign.
fn full_reading(rec: &Recognizer, s: Scan) -> Option<(Chord, bool)> {
    fingered(rec, s).filter(|c| c.ty != CANCEL).map(|c| (c, foreign_bass(c)))
}

/// Full Keyboard: the whole keyboard read with On Bass rules, melody set aside.
///
/// 1. A chord at or below the split (three or more pitch classes, not over a foreign
///    bass) is the chord, and everything above the split is melody.
/// 2. Otherwise the whole keyboard is read, dropping the highest key (the melody) while
///    the rest is no chord, or while the rest is a chord on a different root (C-E-G
///    with a Db on top stays C, whatever the four notes spell). A top key that keeps
///    the root is part of the chord (C-G | E-Bb is C7, so a same-root melody note over
///    an incomplete left hand changes the chord type), and an inversion with the root
///    on top reads as the chord below it (E-G-B-C is Em, not Cmaj7/E).
/// 3. A chord over a bass note foreign to it (C/F#) counts only when no reading is a
///    plain chord or an inversion, unless the top key is needed for it: with F# in the
///    left hand and C-E-G in the right, G is a chord tone, not a melody note leaving
///    C(b5)/F#.
///
/// Needs three pitch classes, so a melody alone never changes the chord.
fn full(rec: &Recognizer, held: &[bool; 128], split: u8) -> Option<Chord> {
    let lh = scan(held, 0, split as usize + 1).filter(|s| s.n >= 3);
    if let Some((c, false)) = lh.and_then(|lh| full_reading(rec, lh)) {
        return Some(c);
    }
    let mut foreign = None;
    let mut s = scan(held, 0, 128)?;
    while s.n >= 3 {
        let below = scan(held, 0, s.high as usize);
        if let Some((c, is_foreign)) = full_reading(rec, s) {
            // Whether the reading without the top key is a plain chord on the same root.
            let same_root = below
                .filter(|b| b.n >= 3)
                .and_then(|b| full_reading(rec, b))
                .filter(|&(_, f)| !f)
                .map(|(d, _)| d.root == c.root);
            match (is_foreign, same_root) {
                (false, Some(false)) => {} // the top key moves the root: melody
                (false, _) | (true, Some(true)) => return Some(c),
                (true, _) => {
                    foreign.get_or_insert(c);
                }
            }
        }
        s = below?;
    }
    foreign
}

/// AI Full Keyboard: Full Keyboard, else the lowest two pitch classes (melody dropped
/// from the top) by AI inference. A lone note is melody and never changes the chord.
fn ai_full(rec: &Recognizer, held: &[bool; 128], split: u8, prev: Option<Chord>) -> Option<Chord> {
    if let Some(c) = full(rec, held, split) {
        return Some(c);
    }
    let mut s = scan(held, 0, 128)?;
    while s.n > 2 {
        s = scan(held, 0, s.high as usize)?;
    }
    if s.n < 2 {
        return None;
    }
    keep(s, prev).or_else(|| infer_dyad(s, prev))
}

/// AI Full Keyboard cannot play 9th, 11th or 13th chords: drop the tension.
fn no_tensions(c: Chord) -> Chord {
    let ty = match c.ty {
        4 => 0,
        6 => 1,
        3 | 5 => 2,
        12 => 8,
        13 | 14 => 10,
        16 => 15,
        22..=27 => 19,
        t => t,
    };
    Chord { ty, ..c }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::NOTE_NAMES;

    fn held(keys: &[u8]) -> [bool; 128] {
        let mut h = [false; 128];
        for &k in keys {
            h[k as usize] = true;
        }
        h
    }

    /// The Genos default split point (F#2): keys at or below it are the left hand.
    const SPLIT: u8 = 54;

    fn get(rec: &Recognizer, mode: Fingering, keys: &[u8], prev: Option<Chord>) -> Option<Chord> {
        detect(rec, mode, &held(keys), SPLIT, prev)
    }

    fn name(rec: &Recognizer, mode: Fingering, keys: &[u8], prev: Option<Chord>) -> String {
        get(rec, mode, keys, prev).map_or("-".into(), |c| c.name())
    }

    #[test]
    fn parse_and_cycle() {
        assert_eq!(Fingering::parse("on-bass"), Some(Fingering::FingeredOnBass));
        assert_eq!(Fingering::parse("AI Full Keyboard"), Some(Fingering::AiFullKeyboard));
        assert_eq!(Fingering::parse("ai_fingered"), Some(Fingering::AiFingered));
        assert_eq!(Fingering::parse("nope"), None);
        let mut f = Fingering::SingleFinger;
        for _ in 0..Fingering::ALL.len() {
            assert_eq!(Fingering::from_u8(f.to_u8()), f);
            f = f.next();
        }
        assert_eq!(f, Fingering::SingleFinger);
        assert!(!Fingering::FullKeyboard.allows_sync_stop() && Fingering::AiFingered.allows_sync_stop());
    }

    #[test]
    fn single_finger() {
        let r = Recognizer::new();
        let sf = Fingering::SingleFinger;
        assert_eq!(name(&r, sf, &[48], None), "C");
        assert_eq!(name(&r, sf, &[46, 48], None), "Cm"); // + black key below
        assert_eq!(name(&r, sf, &[45, 48], None), "C7"); // + white key below
        assert_eq!(name(&r, sf, &[45, 46, 48], None), "Cm7"); // + both
        assert_eq!(name(&r, sf, &[53, 55], None), "G7");
        assert_eq!(name(&r, sf, &[36, 48], None), "C"); // root doubled an octave down
        assert_eq!(name(&r, sf, &[46, 48, 60], None), "Cm");
    }

    #[test]
    fn fingered_vs_on_bass() {
        let r = Recognizer::new();
        let c_e = [40, 48, 52, 55];
        assert_eq!(name(&r, Fingering::FingeredOnBass, &c_e, None), "C/E");
        assert_eq!(name(&r, Fingering::Fingered, &c_e, None), "C");
        // A bass note outside the chord is a slash chord only in On Bass.
        let c_fs = [42, 48, 52, 55];
        assert_eq!(name(&r, Fingering::FingeredOnBass, &c_fs, None), "C/F#");
        assert_eq!(name(&r, Fingering::Fingered, &c_fs, None), "-");
        assert_eq!(get(&r, Fingering::Fingered, &[48, 49, 50], None).unwrap().ty, CANCEL);
    }

    /// Both Fingered types go through the key-counting recognizer: the same key in two
    /// octaves is 1+8, and a lone key or a non-1+5 dyad is no chord (the chord stays).
    #[test]
    fn fingered_one_plus_eight_and_short_inputs() {
        let r = Recognizer::new();
        for m in [Fingering::FingeredOnBass, Fingering::Fingered] {
            assert_eq!(get(&r, m, &[36, 48], None), Some(Chord::new(0, ONE_EIGHT)), "{m:?}");
            assert_eq!(get(&r, m, &[36, 48, 60], None), Some(Chord::new(0, ONE_EIGHT)), "{m:?}");
            assert_eq!(get(&r, m, &[48, 55], None), Some(Chord::new(0, ONE_FIVE)), "{m:?}");
            assert_eq!(get(&r, m, &[48], None), None, "{m:?}");
            assert_eq!(get(&r, m, &[48, 52], None), None, "{m:?}");
        }
    }

    #[test]
    fn multi_finger() {
        let r = Recognizer::new();
        let m = Fingering::MultiFinger;
        assert_eq!(name(&r, m, &[48, 51, 55], None), "Cm"); // Fingered wins over G + Eb + C
        assert_eq!(name(&r, m, &[43, 47, 50, 53], None), "G7");
        assert_eq!(name(&r, m, &[48, 51, 58], None), "Cm7"); // shell voicing, not Bbm7
        assert_eq!(name(&r, m, &[46, 48], None), "Cm");
        assert_eq!(name(&r, m, &[45, 48], None), "C7");
        assert_eq!(name(&r, m, &[45, 46, 48], None), "Cm7");
        assert_eq!(name(&r, m, &[48], None), "C"); // root key only
        assert_eq!(name(&r, m, &[36, 48], None), "C"); // and doubled: Single Finger, not 1+8
        assert_eq!(name(&r, m, &[46, 48, 60], None), "Cm"); // measured from the nearer C
        assert_eq!(name(&r, m, &[36, 52], None), "E7"); // any dyad but a fifth: Single Finger
        assert_eq!(name(&r, m, &[43, 48], None), "C7"); // G below C
        assert_eq!(get(&r, m, &[48, 55], None), Some(Chord::new(0, ONE_FIVE))); // C-G: 1+5
        // So G7 as G + a white key below needs a key other than the C a fifth below.
        assert_eq!(name(&r, m, &[53, 55], None), "G7"); // F-G
        assert_eq!(name(&r, m, &[50, 55], None), "G7"); // D-G
        assert_eq!(name(&r, m, &[52, 55], None), "G7"); // E-G
        // No Cancel in Multi: C-Db-D is the Single Finger m7 shape on D.
        assert_eq!(name(&r, m, &[48, 49, 50], None), "Dm7");
    }

    /// The canonical Single Finger shapes (the nearest black and/or white key to the left
    /// of the root) read as Single Finger in Multi Finger, on every root.
    #[test]
    fn multi_finger_single_shapes_all_roots() {
        let r = Recognizer::new();
        let m = Fingering::MultiFinger;
        for root in 60u8..72 {
            let black = (1..12).map(|d| root - d).find(|&k| is_black(k)).unwrap();
            let white = (1..12).map(|d| root - d).find(|&k| !is_black(k)).unwrap();
            let n = NOTE_NAMES[root as usize % 12];
            assert_eq!(name(&r, m, &[root], None), n);
            assert_eq!(name(&r, m, &[black, root], None), format!("{n}m"));
            assert_eq!(name(&r, m, &[white, root], None), format!("{n}7"));
            let mut keys = [black, white, root];
            keys.sort();
            assert_eq!(name(&r, m, &keys, None), format!("{n}m7"), "{keys:?}");
        }
    }

    #[test]
    fn ai_fingered() {
        let r = Recognizer::new();
        let ai = Fingering::AiFingered;
        let am7 = Some(Chord::new(9, 10));
        let c = Some(Chord::new(0, 0));
        let f = Some(Chord::new(5, 0));
        assert_eq!(name(&r, ai, &[45, 52], am7), "Am7"); // A-E are in Am7: keep it
        assert_eq!(name(&r, ai, &[48, 52], am7), "Am7/C"); // C-E too, over the lower C
        assert_eq!(name(&r, ai, &[57], am7), "Am7"); // so is a lone A
        assert_eq!(name(&r, ai, &[48], am7), "Am7"); // or a lone C: no bass change
        assert_eq!(name(&r, ai, &[50], am7), "D"); // lone note outside it: major on it
        assert_eq!(get(&r, ai, &[38, 50], am7), Some(Chord::new(2, ONE_EIGHT))); // doubled: 1+8
        assert_eq!(name(&r, ai, &[50, 53], am7), "Dm");
        assert_eq!(name(&r, ai, &[50, 54], None), "D");
        assert_eq!(name(&r, ai, &[52, 60], None), "C/E"); // inferred, lowest note is the bass
        assert_eq!(name(&r, ai, &[48, 57], None), "Am/C");
        assert_eq!(name(&r, ai, &[48, 58], None), "C7");
        assert_eq!(name(&r, ai, &[48, 59], None), "Cmaj7");
        assert_eq!(get(&r, ai, &[48, 55], None), Some(Chord::new(0, ONE_FIVE)));
        assert_eq!(get(&r, ai, &[43, 48], None), Some(Chord { bass: Some(7), ..Chord::new(0, ONE_FIVE) })); // G-C: C1+5/G
        // Tritones: the dominant nearer the previous chord on the circle of fifths.
        assert_eq!(name(&r, ai, &[47, 53], None), "G7/B"); // B-F: the lower note is the 3rd
        assert_eq!(name(&r, ai, &[41, 47], c), "G7/F"); // F-B over C
        assert_eq!(name(&r, ai, &[41, 47], None), "C#7/F");
        assert_eq!(name(&r, ai, &[46, 52], f), "C7/Bb"); // Bb-E over F
        assert_eq!(name(&r, ai, &[48, 50], am7), "Am7"); // a 2nd: passing note, chord unchanged
        assert_eq!(name(&r, ai, &[40, 48, 52, 55], None), "C/E"); // 3+ notes: On Bass reading
        assert_eq!(name(&r, ai, &[42, 48, 52, 55], None), "C/F#"); // foreign bass, as On Bass
        assert_eq!(get(&r, ai, &[48, 49, 50], am7).unwrap().ty, CANCEL);
    }

    /// AI Fingered keeps a slash bass (#194): after a chord, a held chord note with a
    /// lower note puts the chord over that note. Plain Fingered keeps the root bass.
    #[test]
    fn ai_fingered_slash_bass() {
        let r = Recognizer::new();
        let ai = Fingering::AiFingered;
        let c = Some(Chord::new(0, MAJOR));
        let am = Some(Chord::new(9, MINOR));
        let e7 = Some(Chord::new(4, SEVENTH));
        // C, then a lone C keeps it, then B below: C/B.
        assert_eq!(name(&r, ai, &[48], c), "C");
        assert_eq!(name(&r, ai, &[47, 48], c), "C/B");
        // A descending line under C: C/Bb, C/A, C/G.
        let c_b = get(&r, ai, &[47, 48], c);
        assert_eq!(name(&r, ai, &[46, 48], c_b), "C/Bb");
        assert_eq!(name(&r, ai, &[45, 48], c), "C/A");
        assert_eq!(name(&r, ai, &[43, 48], c), "C/G");
        // Am, then G+A: Am/G, then F#+A: Am/F#.
        assert_eq!(name(&r, ai, &[43, 45], am), "Am/G");
        let am_g = get(&r, ai, &[43, 45], am);
        assert_eq!(name(&r, ai, &[42, 45], am_g), "Am/F#");
        // Two-note E7/G# (shown with yahaha's flat spelling): G# under E after E7, or the G#-D tritone after Am/G.
        assert_eq!(name(&r, ai, &[44, 52], e7), "E7/Ab");
        assert_eq!(name(&r, ai, &[44, 50], am_g), "E7/Ab");
        // The root lowest is no slash chord.
        assert_eq!(name(&r, ai, &[48, 52], c), "C");
        // Plain Fingered never takes the lowest note as the bass.
        assert_eq!(name(&r, Fingering::Fingered, &[47, 48, 52, 55], c), "Cmaj7");
        assert_eq!(get(&r, Fingering::Fingered, &[47, 48], c), None);
    }

    #[test]
    fn full_keyboard() {
        let r = Recognizer::new();
        let f = Fingering::FullKeyboard;
        assert_eq!(name(&r, f, &[40, 72, 76, 79], None), "C/E"); // LH bass + RH chord
        assert_eq!(name(&r, f, &[38, 72, 76, 79], None), "Cadd9/D"); // bass that is a chord tone
        assert_eq!(name(&r, f, &[42, 72, 76, 79], None), "C/F#"); // foreign bass: last resort
        // LH chord + RH melody: the melody never re-chords the band, chord tone or not.
        for melody in [69, 70, 71, 73, 74, 77, 78, 79, 84] {
            assert_eq!(name(&r, f, &[36, 40, 43, melody], None), "C", "melody {melody}");
        }
        assert_eq!(name(&r, f, &[40, 43, 48, 74], None), "C/E");
        // All in the right hand: a top note that would move the root is melody...
        assert_eq!(name(&r, f, &[60, 64, 67, 73], None), "C");
        assert_eq!(name(&r, f, &[60, 64, 67, 78], None), "C");
        // ...one that keeps it is part of the chord.
        assert_eq!(name(&r, f, &[60, 64, 67, 71], None), "Cmaj7");
        // Rule 2's cost: a first-inversion maj7 with the root on top reads as the minor
        // triad below it, because the top key moves the root (E-G-B-C: Em, A-C-E-F: Am).
        assert_eq!(name(&r, f, &[64, 67, 71, 72], None), "Em");
        assert_eq!(name(&r, f, &[57, 60, 64, 65], None), "Am");
        // A left-hand shell with the upper structure in the right hand is the full chord:
        // C-G | E-Bb is C7, so the top right-hand key cannot be assumed to be melody.
        assert_eq!(name(&r, f, &[36, 43, 64, 70], None), "C7");
        assert_eq!(name(&r, f, &[72], Some(Chord::new(0, 0))), "-"); // melody alone
        assert_eq!(name(&r, f, &[48, 49, 50], None), "-"); // no Cancel
    }

    #[test]
    fn ai_full_keyboard() {
        let r = Recognizer::new();
        let f = Fingering::AiFullKeyboard;
        let c = Some(Chord::new(0, 0));
        assert_eq!(name(&r, f, &[48, 52, 55, 58, 62], None), "C7"); // C9 not available
        assert_eq!(name(&r, f, &[48, 52, 55, 62], None), "C"); // Cadd9 -> C
        assert_eq!(name(&r, f, &[36, 40, 43, 74], None), "C"); // LH chord + RH melody
        assert_eq!(name(&r, f, &[74], c), "-"); // lone melody note
        assert_eq!(name(&r, f, &[50, 53], c), "Dm");
        // LH dyad + RH melody: the dyad is inferred with the melody set aside.
        assert_eq!(name(&r, f, &[45, 48, 74], None), "Am");
        assert_eq!(name(&r, f, &[48, 52, 77], Some(Chord::new(9, 10))), "Am7"); // kept
        // D-F with E on top: D-F-E is no chord, so E is melody and D-F infers Dm.
        assert_eq!(name(&r, f, &[50, 53, 76], c), "Dm");
        // D-F with C on top is a chord (Dm7), so C is part of it.
        assert_eq!(name(&r, f, &[50, 53, 72], c), "Dm7");
    }

    /// Full Keyboard types switch Sync Stop off and keep it off.
    #[test]
    fn full_keyboard_disables_sync_stop() {
        use crate::engine::{Button, Engine, Prepared};
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/MOX_v2/FunkyFinger.S930.STY");
        if !path.exists() {
            return;
        }
        let mut e = Engine::new(Box::new(Prepared::new(&crate::sff::Style::load(&path).unwrap())));
        let mut rec = crate::sim::Recorder::default();
        e.button(Button::SyncStop, 0, &mut rec);
        assert!(e.snapshot(0).sync_stop);
        e.allow_sync_stop(Fingering::AiFullKeyboard.allows_sync_stop());
        assert!(!e.snapshot(0).sync_stop);
        e.button(Button::SyncStop, 0, &mut rec);
        assert!(!e.snapshot(0).sync_stop);
        e.allow_sync_stop(Fingering::AiFingered.allows_sync_stop());
        e.button(Button::SyncStop, 0, &mut rec);
        assert!(e.snapshot(0).sync_stop);
    }
}

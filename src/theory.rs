//! Chord types, chord recognition, and style note transposition (NTR/NTT).
//!
//! Everything here is allocation-free so it can run on the real-time threads.

use crate::sff::{ChannelRule, Ntr, Ntt, Zone};

pub const CANCEL: u8 = 0x22;
/// Chord types a style's CASM data knows about (chord mute bits, source chord).
pub const NUM_TYPES: usize = 34;

/// Data List chords with no MIDI / CASM code. They are recognised and displayed as
/// themselves, and the style follows them as the CASM type from [`casm_type`].
pub const M7B5: u8 = 35;
pub const FLAT5: u8 = 36;
pub const MM7B5: u8 = 37;

/// Chord type ids 0..=34 match the Yamaha source-chord / chord-mute numbering.
pub const TYPE_NAMES: [&str; 38] = [
    "", "6", "maj7", "maj7#11", "add9", "maj9", "6/9", "aug", "m", "m6", "m7", "m7b5", "m(add9)",
    "m9", "m11", "mMaj7", "mMaj9", "dim", "dim7", "7", "7sus4", "7b5", "9", "7#11", "13", "7b9",
    "7b13", "7#9", "maj7#5", "7#5", "1+8", "1+5", "sus4", "sus2", "cancel", "maj7b5", "(b5)", "mMaj7b5",
];

/// Pitch classes of each chord type relative to the root.
const TONES: [&[u8]; 38] = [
    &[0, 4, 7],
    &[0, 4, 7, 9],
    &[0, 4, 7, 11],
    &[0, 4, 6, 7, 11],
    &[0, 2, 4, 7],
    &[0, 2, 4, 7, 11],
    &[0, 2, 4, 7, 9],
    &[0, 4, 8],
    &[0, 3, 7],
    &[0, 3, 7, 9],
    &[0, 3, 7, 10],
    &[0, 3, 6, 10],
    &[0, 2, 3, 7],
    &[0, 2, 3, 7, 10],
    &[0, 3, 5, 7, 10],
    &[0, 3, 7, 11],
    &[0, 2, 3, 7, 11],
    &[0, 3, 6],
    &[0, 3, 6, 9],
    &[0, 4, 7, 10],
    &[0, 5, 7, 10],
    &[0, 4, 6, 10],
    &[0, 2, 4, 7, 10],
    &[0, 4, 6, 7, 10],
    &[0, 4, 7, 9, 10],
    &[0, 1, 4, 7, 10],
    &[0, 4, 7, 8, 10],
    &[0, 3, 4, 7, 10],
    &[0, 4, 8, 11],
    &[0, 4, 8, 10],
    &[0],
    &[0, 7],
    &[0, 5, 7],
    &[0, 2, 7],
    &[],
    &[0, 4, 6, 11],
    &[0, 4, 6],
    &[0, 3, 6, 11],
];

/// The CASM chord type a style follows for `ty`. The three Data List chords without a
/// code map to the nearest CASM type: the smallest type holding every played note, else
/// the largest one using only played notes. The mapped type may add a note (M7b5 -> M7(#11)
/// adds the natural 5th, (b5) -> 7b5 adds the b7) but never drops a played one, except
/// mM7b5, which has no superset and drops its M7.
/// M7b5 -> M7(#11) (b5 = #11), (b5) -> 7b5, mM7b5 -> dim.
pub const fn casm_type(ty: u8) -> u8 {
    match ty {
        M7B5 => 3,
        FLAT5 => 21,
        MM7B5 => 17,
        t => t,
    }
}

pub const NOTE_NAMES: [&str; 12] = ["C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chord {
    pub root: u8,
    pub ty: u8,
    /// On-bass note (pitch class) when it differs from the root.
    pub bass: Option<u8>,
}

impl Chord {
    pub const fn new(root: u8, ty: u8) -> Chord {
        Chord { root, ty, bass: None }
    }

    pub fn name(&self) -> String {
        if self.ty == CANCEL {
            return "N.C.".into();
        }
        let mut s = format!("{}{}", NOTE_NAMES[self.root as usize % 12], TYPE_NAMES[self.ty as usize]);
        if let Some(b) = self.bass {
            s.push('/');
            s.push_str(NOTE_NAMES[b as usize % 12]);
        }
        s
    }

    /// Pack into 32 bits for lock-free publication: [valid:1][bass_present:1][bass:4][root:4][ty:6] + generation in the high 16 bits.
    pub fn pack(&self, generation: u16) -> u32 {
        let b = self.bass.map(|b| 0x10 | b as u32).unwrap_or(0);
        ((generation as u32) << 16) | 0x8000 | (b << 10) | ((self.root as u32 & 0xF) << 6) | (self.ty as u32 & 0x3F)
    }

    pub fn unpack(v: u32) -> Option<(Chord, u16)> {
        if v & 0x8000 == 0 {
            return None;
        }
        let bass = if v & (0x10 << 10) != 0 { Some(((v >> 10) & 0xF) as u8) } else { None };
        Some((Chord { root: ((v >> 6) & 0xF) as u8, ty: (v & 0x3F) as u8, bass }, (v >> 16) as u16))
    }

    /// The chord as the style follows it: display-only types become their CASM type.
    #[inline]
    pub const fn casm(self) -> Chord {
        Chord { ty: casm_type(self.ty), ..self }
    }
}

/// Pitch classes (relative to the root) of a chord type, as played (M7b5 has a b5).
pub fn chord_tones(ty: u8) -> &'static [u8] {
    TONES[(ty as usize).min(TONES.len() - 1)]
}

fn mask_of(ty: u8) -> u16 {
    mask_of_set(TONES[ty as usize])
}

fn mask_of_set(set: &[u8]) -> u16 {
    set.iter().fold(0, |m, &t| m | 1 << t)
}

fn rot(mask: u16, by: u8) -> u16 {
    let by = by % 12;
    ((mask << by) | (mask >> (12 - by))) & 0x0FFF
}

// ---------------------------------------------------------------------------
// Recognition
// ---------------------------------------------------------------------------

/// Data List p.45 "Chord Types Recognized in the Fingered Mode", in table order:
/// (type, required intervals, intervals that may be omitted). 1+8 is not here because
/// it needs the key count, not just pitch classes (see [`Recognizer::recognize_keys`]).
const FINGERED: [(u8, &[u8], &[u8]); 37] = [
    (31, &[0, 7], &[]),                   // 1+5
    (0, &[0, 4, 7], &[]),                 // M
    (1, &[0, 7, 9], &[4]),                // 6
    (2, &[0, 4, 11], &[7]),               // M7
    (M7B5, &[0, 4, 6, 11], &[]),          // M7b5
    (3, &[0, 4, 6, 7, 11], &[2]),         // M7(#11)
    (4, &[0, 2, 4, 7], &[]),              // (9)
    (5, &[0, 2, 4, 11], &[7]),            // M7(9)
    (6, &[0, 2, 4, 9], &[7]),             // 6(9)
    (FLAT5, &[0, 4, 6], &[]),             // (b5)
    (7, &[0, 4, 8], &[]),                 // aug
    (29, &[0, 4, 8, 10], &[]),            // 7aug
    (28, &[0, 8, 11], &[4]),              // M7aug
    (8, &[0, 3, 7], &[]),                 // m
    (9, &[0, 3, 7, 9], &[]),              // m6
    (10, &[0, 3, 10], &[7]),              // m7
    (11, &[0, 3, 6, 10], &[]),            // m7b5
    (12, &[0, 2, 3, 7], &[]),             // m(9)
    (13, &[0, 2, 3, 10], &[7]),           // m7(9)
    (14, &[0, 3, 5, 7], &[2, 10]),        // m7(11)
    (MM7B5, &[0, 3, 6, 11], &[]),         // mM7b5
    (15, &[0, 3, 11], &[7]),              // mM7
    (16, &[0, 2, 3, 11], &[7]),           // mM7(9)
    (17, &[0, 3, 6], &[]),                // dim
    (18, &[0, 3, 6, 9], &[]),             // dim7
    (19, &[0, 4, 10], &[7]),              // 7
    (20, &[0, 5, 7, 10], &[]),            // 7sus4
    (22, &[0, 2, 4, 10], &[7]),           // 7(9)
    (23, &[0, 4, 6, 7, 10], &[2]),        // 7(#11)
    (24, &[0, 4, 9, 10], &[7]),           // 7(13)
    (21, &[0, 4, 6, 10], &[]),            // 7b5
    (25, &[0, 1, 4, 10], &[7]),           // 7(b9)
    (26, &[0, 4, 7, 8, 10], &[]),         // 7(b13)
    (27, &[0, 3, 4, 10], &[7]),           // 7(#9)
    (32, &[0, 5, 7], &[]),                // sus4
    (33, &[0, 2, 7], &[]),                // sus2
    (CANCEL, &[0, 1, 2], &[]),            // cancel
];

/// How well an interval above the root sits in the bass, for inversions whose lowest
/// note is not a root of any reading: root, then 5th, 3rd, altered 5th/4th, 6th/7th, tensions.
fn bass_rank(iv: u8) -> u8 {
    match iv {
        0 => 0,
        7 => 1,
        3 | 4 => 2,
        5 | 6 | 8 => 3,
        9..=11 => 4,
        _ => 5,
    }
}

/// Precomputed "Fingered" / "Fingered On Bass" recognizer: 4096 pitch-class masks × 12 lowest notes.
pub struct Recognizer {
    table: Box<[u16; 4096 * 12]>,
}

const NO_CHORD: u16 = 0xFFFF;
/// Table flag: the bass is a note outside the chord (On Bass only; plain Fingered ignores the shape).
const EXTRA_BASS: u16 = 0x8000;

impl Default for Recognizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Recognizer {
    pub fn new() -> Recognizer {
        let mut table = Box::new([NO_CHORD; 4096 * 12]);
        for mask in 1u16..4096 {
            for low in 0u8..12 {
                if mask & (1 << low) == 0 {
                    continue;
                }
                if let Some((c, extra)) = Self::compute(mask, low) {
                    // [extra_bass:1][bass:4][bass_present:1][root:4][ty:6]
                    let b = c.bass.map(|b| 0x400 | (b as u16) << 11).unwrap_or(0);
                    let e = if extra { EXTRA_BASS } else { 0 };
                    table[mask as usize * 12 + low as usize] = e | b | (c.root as u16) << 6 | c.ty as u16;
                }
            }
        }
        Recognizer { table }
    }

    /// Best Data List reading of exactly this pitch-class set. Ambiguous sets resolve by:
    /// 1. a reading whose root is the lowest note (C E G A = C6, A C E G = Am7; dim7 and
    ///    aug take the lowest note as root);
    /// 2. fewest omitted notes;
    /// 3. the lowest note's role in the chord ([`bass_rank`]: E G A C = Am7/E, G A C E = C6/G);
    /// 4. Data List table order.
    fn table_match(mask: u16, low: u8) -> Option<Chord> {
        let mut best: Option<((bool, u32, u8, usize), Chord)> = None;
        for (i, &(ty, req, opt)) in FINGERED.iter().enumerate() {
            let (req, opt) = (mask_of_set(req), mask_of_set(opt));
            for root in 0..12u8 {
                let (r, o) = (rot(req, root), rot(opt, root));
                if mask & r != r || mask & !(r | o) != 0 {
                    continue;
                }
                if ty == CANCEL {
                    return Some(Chord::new(0, CANCEL));
                }
                let omitted = (o & !mask).count_ones();
                let key = (root != low, omitted, bass_rank((low + 12 - root) % 12), i);
                if best.is_none_or(|(k, _)| key < k) {
                    let bass = if root == low { None } else { Some(low) };
                    best = Some((key, Chord { root, ty, bass }));
                }
            }
        }
        best.map(|(_, c)| c)
    }

    /// On Bass result for a pitch-class set, and whether the bass is an extra non-chord note.
    fn compute(mask: u16, low: u8) -> Option<(Chord, bool)> {
        // One pitch class is 1+8 or nothing, depending on the key count (decided at lookup).
        if mask.count_ones() < 2 {
            return None;
        }
        if let Some(c) = Self::table_match(mask, low) {
            return Some((c, false));
        }
        // On Bass: a table chord over a bass note that is not part of it (C E G over F# = C/F#).
        // Only a complete three- or four-note chord counts, so a tension-laden set does not
        // turn into an unrelated root over the bass (C E G B F is not G13/C).
        let upper = mask & !(1 << low);
        if (3..=4).contains(&upper.count_ones()) {
            let lowest_upper = (1..12u8).map(|i| (low + i) % 12).find(|&p| upper & (1 << p) != 0).unwrap();
            let complete = |c: &Chord| c.ty != CANCEL && rot(mask_of(c.ty), c.root) == upper;
            if let Some(c) = Self::table_match(upper, lowest_upper).filter(complete) {
                return Some((Chord { bass: Some(low), ..c }, true));
            }
        }
        None
    }

    /// Fingered On Bass recognition from a pitch-class mask and the lowest held pitch
    /// class, assuming one key per pitch class (so a lone pitch class is no chord).
    #[allow(dead_code)] // pitch-class-only entry point; live input uses `recognize_keys`
    #[inline]
    pub fn recognize(&self, mask: u16, low: u8) -> Option<Chord> {
        self.recognize_keys(mask, low, (mask & 0x0FFF).count_ones(), true)
    }

    /// Fingered (`on_bass` false: bass = root) or Fingered On Bass recognition. `keys` is
    /// the number of held keys; two or more keys on one pitch class make 1+8. Fewer than
    /// three keys only ever make 1+5 or 1+8. None means no chord: keep the previous one.
    #[inline]
    pub fn recognize_keys(&self, mask: u16, low: u8, keys: u32, on_bass: bool) -> Option<Chord> {
        let mask = mask & 0x0FFF;
        if mask.count_ones() == 1 {
            return (keys >= 2).then(|| Chord::new(mask.trailing_zeros() as u8, 30));
        }
        if mask == 0 {
            return None;
        }
        let v = self.table[mask as usize * 12 + low as usize % 12];
        if v == NO_CHORD || (v & EXTRA_BASS != 0 && !on_bass) {
            return None;
        }
        let bass = if v & 0x400 != 0 && on_bass { Some(((v >> 11) & 0xF) as u8) } else { None };
        Some(Chord { root: ((v >> 6) & 0xF) as u8, ty: (v & 0x3F) as u8, bass })
    }
}

// ---------------------------------------------------------------------------
// Scale / role model used by the transposition tables
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Quality {
    Major,
    Minor,
    Dominant,
    HalfDim,
    Dim,
}

fn quality(ty: u8) -> Quality {
    match ty {
        8..=10 | 12..=16 => Quality::Minor,
        11 => Quality::HalfDim,
        17 | 18 => Quality::Dim,
        19..=27 | 29 => Quality::Dominant,
        _ => Quality::Major,
    }
}

/// Seven-degree scale (values relative to root, may reach 12) describing how a
/// melodic line written over the chord should sound.
fn scale(ty: u8, ntt: Ntt) -> [u8; 7] {
    let minor_base: [u8; 7] = match ntt {
        Ntt::MelodicMinor | Ntt::MelodicMinor5 => [0, 2, 3, 5, 7, 9, 11],
        Ntt::HarmonicMinor | Ntt::HarmonicMinor5 => [0, 2, 3, 5, 7, 8, 11],
        Ntt::Dorian | Ntt::Dorian5 => [0, 2, 3, 5, 7, 9, 10],
        _ => [0, 2, 3, 5, 7, 8, 10],
    };
    let mut s = match quality(ty) {
        Quality::Major => [0, 2, 4, 5, 7, 9, 11],
        Quality::Minor => minor_base,
        Quality::Dominant => [0, 2, 4, 5, 7, 9, 10],
        Quality::HalfDim => [0, 2, 3, 5, 6, 8, 10],
        Quality::Dim => [0, 2, 3, 5, 6, 8, 9],
    };
    // Chord tones override the scale.
    match ty {
        1 | 6 => s[5] = 9,
        3 => s[3] = 6,
        7 => s[4] = 8,
        9 => s[5] = 9,
        10 | 13 | 14 => s[6] = 10,
        15 | 16 => s[6] = 11,
        18 => s[6] = 9,
        20 => s[2] = 5,
        21 => s[4] = 6,
        23 => s[3] = 6,
        24 => s[5] = 9,
        25 => s[1] = 1,
        26 => s[5] = 8,
        27 => s[1] = 3,
        28 => {
            s[3] = 6;
            s[4] = 8
        }
        29 => s[4] = 8,
        // No-3rd chords sound only what is common to major and minor. 1+8: every degree
        // becomes the root. 1+5: the 3rd moves up to the 5th, the 6th down to it, the 7th
        // up to the octave; the 2nd and 4th stay. `scale_map` snaps chromatic notes too.
        30 => s = [0, 0, 0, 0, 0, 0, 12],
        31 => s = [0, 2, 7, 5, 7, 7, 12],
        32 => s[2] = 5,
        33 => s[2] = 2,
        _ => {}
    }
    s
}

/// 1+8 and 1+5: no 3rd, so the accompaniment must fit both major and minor.
#[inline]
fn is_no_third(ty: u8) -> bool {
    matches!(ty, 30 | 31)
}

/// Scale used when `ty` is the *source* chord. The 1+8 / 1+5 target scales fold several
/// degrees together, so a pattern recorded over them is read with the plain major scale
/// (it only contains root, 5th and the quality-neutral 2nd / 4th anyway).
fn src_scale(ty: u8, ntt: Ntt) -> [u8; 7] {
    if is_no_third(ty) {
        scale(0, ntt)
    } else {
        scale(ty, ntt)
    }
}

fn scale_map(d: u8, src: &[u8; 7], tgt: &[u8; 7], tgt_ty: u8) -> i8 {
    let mut idx = 0;
    for i in 0..7 {
        if src[i] <= d {
            idx = i;
        }
    }
    // Chromatic passing notes keep their offset, except over 1+8 / 1+5 where they would
    // bring back a 3rd (Eb over C1+8 would sound C#); snap them to the degree below.
    let off = if is_no_third(tgt_ty) { 0 } else { d as i8 - src[idx] as i8 };
    tgt[idx] as i8 + off
}

/// Chord tones ordered by importance (the notes the Chord table keeps), followed by the rest.
/// Returns (important, others) as semitone lists.
fn importance(ty: u8) -> ([u8; 3], [u8; 2], usize) {
    let t = TONES[ty as usize];
    match t.len() {
        1 => ([0, 0, 12], [0, 0], 0),
        2 => ([0, 7, 12], [0, 0], 0),
        3 => ([t[1], t[2], 12], [0, 0], 0),
        4 => {
            // 3rd, 5th, 7th/6th; root is the least important.
            let (third, fifth, sev) = four_note_roles(t);
            ([third, fifth, sev], [0, 0], 1)
        }
        _ => {
            // 3rd, 7th, tension; root and 5th are dropped first.
            let (third, fifth, sev, ten) = five_note_roles(ty, t);
            ([third, sev, ten], [0, fifth], 2)
        }
    }
}

fn four_note_roles(t: &[u8]) -> (u8, u8, u8) {
    // Tones are sorted; for add9 / m(add9) the "7th" slot is the 9th.
    if t[1] == 2 {
        (t[2], t[3], 14)
    } else {
        (t[1], t[2], t[3])
    }
}

fn five_note_roles(ty: u8, t: &[u8]) -> (u8, u8, u8, u8) {
    let third = *t.iter().find(|&&x| matches!(x, 3 | 4 | 5)).unwrap_or(&4);
    let sev = *t.iter().rev().find(|&&x| matches!(x, 9 | 10 | 11)).unwrap_or(&10);
    let fifth = if t.contains(&7) { 7 } else { *t.iter().find(|&&x| x == 6 || x == 8).unwrap_or(&7) };
    let ten = match ty {
        3 | 23 => 6,
        14 => 5,
        24 => 9,
        25 => 13,
        26 => 8,
        27 => 15,
        _ => 14,
    };
    (third, fifth, sev, ten)
}

fn chord_map(d: u8, src_ty: u8, tgt_ty: u8, src_scale: &[u8; 7], tgt_scale: &[u8; 7]) -> i8 {
    let v = chord_map_raw(d, src_ty, tgt_ty, src_scale, tgt_scale);
    if tgt_ty == 31 {
        // Chord parts over 1+5 keep only root and 5th: the 2nd falls to the root, the
        // 4th rises to the 5th (the Melody scale keeps both as passing tones).
        return match v.rem_euclid(12) {
            2 => v - 2,
            5 => v + 2,
            _ => v,
        };
    }
    v
}

fn chord_map_raw(d: u8, src_ty: u8, tgt_ty: u8, src_scale: &[u8; 7], tgt_scale: &[u8; 7]) -> i8 {
    let (si, so, _) = importance(src_ty);
    let (ti, to, _) = importance(tgt_ty);
    let tgt_fifth = if to[1] != 0 { to[1] } else { ti[1] };
    if is_no_third(src_ty) {
        // Recorded over 1+8 / 1+5: root stays, the 5th follows the target's 5th.
        return match d {
            0 => 0,
            7 => tgt_fifth as i8,
            _ => scale_map(d, src_scale, tgt_scale, tgt_ty),
        };
    }
    for i in 0..3 {
        if si[i] % 12 == d {
            return ti[i] as i8 - if si[i] >= 12 { 12 } else { 0 };
        }
    }
    if d == 0 {
        return 0;
    }
    if so[1] != 0 && so[1] % 12 == d {
        return tgt_fifth as i8;
    }
    scale_map(d, src_scale, tgt_scale, tgt_ty)
}

// ---------------------------------------------------------------------------
// Transposition
// ---------------------------------------------------------------------------

#[inline]
fn fold_into(mut n: i32, lo: u8, hi: u8) -> i32 {
    let (lo, hi) = (lo as i32, hi as i32);
    if hi - lo >= 11 {
        while n < lo {
            n += 12;
        }
        while n > hi {
            n -= 12;
        }
    }
    n.clamp(0, 127)
}

pub fn is_drum_part(dest_ch: u8) -> bool {
    dest_ch == 8 || dest_ch == 9
}

/// Does this source channel play at all under the chord?
///
/// Chord Cancel is resolved by the engine (it is the no-chord state, see
/// `engine::effective_chord`) before it gets here; the guard keeps the CASM autostart
/// bit (bit 34) from being read as a chord-mute bit.
#[inline]
pub fn plays(rule: &ChannelRule, chord: Chord) -> bool {
    let chord = chord.casm();
    if chord.ty == CANCEL {
        return is_drum_part(rule.dest_ch);
    }
    rule.chord_mute & (1u64 << chord.ty) != 0 && rule.note_mute & (1 << chord.root) != 0
}

/// Transpose one source note for the target chord. Returns None if the note should not sound.
pub fn transpose(key: u8, rule: &ChannelRule, chord: Chord) -> Option<u8> {
    let chord = chord.casm();
    if is_drum_part(rule.dest_ch) {
        return Some(key);
    }
    if chord.ty as usize >= NUM_TYPES {
        return None; // Cancel has no pitches to follow
    }
    let z = rule.zone_for(key);
    if z.ntt == Ntt::Bypass && z.ntr != Ntr::RootTrans {
        return Some(key);
    }
    if z.ntr == Ntr::Guitar {
        return guitar(key, rule, z, chord);
    }
    let sr = rule.src_root;
    let d = (key as i32 - sr as i32).rem_euclid(12) as u8;
    let mut val: i32 = match z.ntt {
        Ntt::Bypass => d as i32,
        Ntt::Chord => {
            let s = src_scale(rule.src_type, Ntt::Melody);
            let t = scale(chord.ty, Ntt::Melody);
            let v = chord_map(d, rule.src_type, chord.ty, &s, &t) as i32;
            // Keep the movement small.
            let mut delta = v - d as i32;
            while delta > 6 {
                delta -= 12;
            }
            while delta < -6 {
                delta += 12;
            }
            d as i32 + delta
        }
        ntt => {
            let s = src_scale(rule.src_type, ntt);
            let t = scale(chord.ty, ntt);
            scale_map(d, &s, &t, chord.ty) as i32
        }
    };
    // On-bass: parts with Bass On replace the root with the bass note.
    if let (true, Some(b)) = (rule.bass_on || z.ntt == Ntt::Bass, chord.bass) {
        if val.rem_euclid(12) == 0 {
            let mut off = (b as i32 - chord.root as i32).rem_euclid(12);
            if off > 6 {
                off -= 12;
            }
            val += off;
        }
    }
    let out = match z.ntr {
        Ntr::RootFixed => {
            let pc = (chord.root as i32 + val).rem_euclid(12);
            // Nearest octave to the original note.
            let base = key as i32 - (key as i32).rem_euclid(12) + pc;
            [base - 12, base, base + 12].into_iter().min_by_key(|n| (n - key as i32).abs()).unwrap()
        }
        _ => {
            let mut shift = (chord.root as i32 - sr as i32).rem_euclid(12);
            if chord.root > z.high_key {
                shift -= 12;
            }
            key as i32 + shift + (val - d as i32)
        }
    };
    Some(fold_into(out, z.lo, z.hi) as u8)
}

/// Transpose notes that start together on one source channel. For Root Fixed parts the
/// target pitch classes are assigned to the source notes with the least total movement,
/// so C3-E3-G3 over F becomes C3-F3-A3 (spec §5.2.5.1) instead of a parallel shift.
pub fn transpose_group(keys: &[u8], rule: &ChannelRule, chord: Chord, out: &mut [Option<u8>]) {
    for (i, &k) in keys.iter().enumerate() {
        out[i] = transpose(k, rule, chord);
    }
    let n = keys.len();
    if !(2..=6).contains(&n) || is_drum_part(rule.dest_ch) {
        return;
    }
    if keys.iter().any(|&k| rule.zone_for(k).ntr != Ntr::RootFixed || rule.zone_for(k).ntt == Ntt::Bypass) {
        return;
    }
    let mut pcs = [0u8; 6];
    for i in 0..n {
        match out[i] {
            Some(v) => pcs[i] = v % 12,
            None => return,
        }
    }
    let dist = |k: u8, pc: u8| -> i32 {
        let d = (pc as i32 - k as i32).rem_euclid(12);
        d.min(12 - d)
    };
    // Heap's algorithm over index permutations (n <= 6).
    let mut perm = [0usize, 1, 2, 3, 4, 5];
    let mut best = perm;
    let cost = |p: &[usize; 6]| (0..n).map(|i| dist(keys[i], pcs[p[i]])).sum::<i32>();
    let mut best_cost = cost(&perm);
    let mut c = [0usize; 6];
    let mut i = 1;
    while i < n {
        if c[i] < i {
            if i % 2 == 0 {
                perm.swap(0, i);
            } else {
                perm.swap(c[i], i);
            }
            let pc = cost(&perm);
            if pc < best_cost {
                best_cost = pc;
                best = perm;
            }
            c[i] += 1;
            i = 1;
        } else {
            c[i] = 0;
            i += 1;
        }
    }
    for i in 0..n {
        let k = keys[i] as i32;
        let pc = pcs[best[i]] as i32;
        let mut d = (pc - k).rem_euclid(12);
        if d > 6 {
            d -= 12;
        }
        let z = rule.zone_for(keys[i]);
        out[i] = Some(fold_into(k + d, z.lo, z.hi) as u8);
    }
}

// ---------------------------------------------------------------------------
// Guitar NTR
// ---------------------------------------------------------------------------

const OPEN: [u8; 6] = [64, 59, 55, 50, 45, 40]; // string 1..6
const POS_START: [u8; 4] = [0, 3, 5, 8];

/// Six-string voicing (string 1..6), None = muted string.
pub fn guitar_voicing(chord: Chord, position: usize) -> [Option<u8>; 6] {
    let chord = chord.casm();
    let tones = rot(mask_of(chord.ty.min(NUM_TYPES as u8 - 1)), chord.root);
    let start = POS_START[position.min(3)];
    let win = |s: usize, want: u16| -> Option<u8> {
        (start..=start + 4).map(|f| OPEN[s] + f).find(|n| want & (1 << (n % 12)) != 0)
    };
    let mut out = [None; 6];
    let bass_pc = chord.bass.unwrap_or(chord.root);
    // Lowest string that can take the bass note sets the bottom of the voicing.
    let low = (3..6).rev().find(|&s| win(s, 1 << bass_pc).is_some()).unwrap_or(3);
    out[low] = win(low, 1 << bass_pc);
    let mut covered = 1u16 << bass_pc;
    for s in (0..low).rev() {
        let fresh = tones & !covered;
        let n = win(s, if fresh != 0 { fresh } else { tones }).or_else(|| win(s, tones));
        if let Some(n) = n {
            covered |= 1 << (n % 12);
        }
        out[s] = n;
    }
    out
}

fn guitar(key: u8, rule: &ChannelRule, z: &Zone, chord: Chord) -> Option<u8> {
    let d = (key as i32 - rule.src_root as i32).rem_euclid(12);
    let position = ((key as i32 / 12) - 3).clamp(0, 3) as usize; // C2 (MIDI 36) = 1st position
    let v = guitar_voicing(chord, position);
    let string = match d {
        11 | 10 => 0,
        9 | 8 => 1,
        7 => 2,
        5 | 6 => 3,
        4 | 3 => 4,
        2 => 5,
        0 | 1 => {
            // Root (C) or fifth (C#): the lowest voiced note, or a fifth above it
            // (an octave over 1+8, which has no fifth).
            let low = v.iter().rev().flatten().next().copied()?;
            let n = match (d, chord.ty) {
                (0, _) => low,
                (_, 30) => low + 12,
                _ => low + 7,
            };
            return Some(fold_into(n as i32, z.lo, z.hi) as u8);
        }
        _ => return None,
    };
    v[string].map(|n| fold_into(n as i32, z.lo, z.hi) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sff::{ChannelRule, Rtr};

    fn rule(ntr: Ntr, ntt: Ntt, hk: u8, lo: u8, hi: u8) -> ChannelRule {
        let mut r = ChannelRule::default_for(12);
        r.zones = [Zone { ntr, ntt, high_key: hk, lo, hi, rtr: Rtr::PitchShift }; 3];
        r
    }

    fn names(keys: &[u8]) -> Vec<String> {
        keys.iter().map(|k| format!("{}{}", NOTE_NAMES[*k as usize % 12], *k as i32 / 12 - 2)).collect()
    }

    #[test]
    fn recognize_basics() {
        let r = Recognizer::new();
        let m = |ns: &[u8]| ns.iter().fold(0u16, |m, n| m | 1 << (n % 12));
        assert_eq!(r.recognize(m(&[60, 64, 67]), 0).unwrap().name(), "C");
        assert_eq!(r.recognize(m(&[57, 60, 64]), 9).unwrap().name(), "Am");
        assert_eq!(r.recognize(m(&[55, 59, 62, 65]), 7).unwrap().name(), "G7");
        assert_eq!(r.recognize(m(&[60, 64, 67, 69]), 0).unwrap().name(), "C6");
        assert_eq!(r.recognize(m(&[57, 60, 64, 67]), 9).unwrap().name(), "Am7");
        assert_eq!(r.recognize(m(&[60, 63, 66, 70]), 0).unwrap().name(), "Cm7b5");
        assert_eq!(r.recognize(m(&[60, 64, 70]), 0).unwrap().name(), "C7"); // omitted 5th
        assert_eq!(r.recognize(m(&[52, 60, 64, 67]), 4).unwrap().name(), "C/E");
        assert_eq!(r.recognize(m(&[50, 55, 59, 62]), 2).unwrap().name(), "G/D");
        assert_eq!(r.recognize(m(&[60, 61, 62]), 0).unwrap().ty, CANCEL);
        assert_eq!(r.recognize(m(&[60]), 0), None);
        assert_eq!(r.recognize(m(&[60, 67]), 0).unwrap().name(), "C1+5");
        assert_eq!(r.recognize(m(&[60, 65, 67]), 0).unwrap().name(), "Csus4");
        assert_eq!(r.recognize(m(&[62, 65, 69, 72]), 2).unwrap().name(), "Dm7");
        assert_eq!(r.recognize(m(&[60, 64, 67, 70, 74]), 0).unwrap().name(), "C9");
    }

    /// Recognize held MIDI keys the way live input does.
    fn keys(r: &Recognizer, ns: &[u8], on_bass: bool) -> Option<Chord> {
        let mask = ns.iter().fold(0u16, |m, n| m | 1 << (n % 12));
        let low = *ns.iter().min().unwrap();
        r.recognize_keys(mask, low % 12, ns.len() as u32, on_bass)
    }

    fn name(c: Option<Chord>) -> String {
        c.map(|c| c.name()).unwrap_or_else(|| "-".into())
    }

    /// One Data List row: every listed voicing (full and with optional notes left out),
    /// in all 12 roots with the root lowest, is recognised as `ty` in both Fingered modes.
    fn row(ty: u8, c_name: &str, voicings: &[&[u8]]) {
        let r = Recognizer::new();
        for v in voicings {
            for root in 0..12u8 {
                let ns: Vec<u8> = v.iter().map(|&i| 48 + root + i).collect();
                for on_bass in [false, true] {
                    let got = keys(&r, &ns, on_bass);
                    assert_eq!(got, Some(Chord::new(root, ty)), "{v:?} on {} (on_bass {on_bass})", NOTE_NAMES[root as usize]);
                }
            }
            assert_eq!(name(keys(&r, &v.iter().map(|&i| 48 + i).collect::<Vec<_>>(), true)), c_name);
        }
    }

    // One test per row of the Data List p.45 table, in table order. Interval 12 is the octave.
    #[test]
    fn dl01_one_plus_eight() {
        row(30, "C1+8", &[&[0, 12], &[0, 12, 24]]);
        // A single key is not a Fingered chord: the previous chord stays.
        let r = Recognizer::new();
        assert_eq!(keys(&r, &[48], true), None);
    }
    #[test]
    fn dl02_one_plus_five() {
        row(31, "C1+5", &[&[0, 7], &[0, 7, 12]]);
    }
    #[test]
    fn dl03_major() {
        row(0, "C", &[&[0, 4, 7], &[0, 7, 16], &[0, 4, 7, 12]]);
    }
    #[test]
    fn dl04_sixth() {
        row(1, "C6", &[&[0, 4, 7, 9], &[0, 7, 9]]);
    }
    #[test]
    fn dl05_major_seventh() {
        row(2, "Cmaj7", &[&[0, 4, 7, 11], &[0, 4, 11]]);
    }
    #[test]
    fn dl06_major_seventh_flat_five() {
        row(M7B5, "Cmaj7b5", &[&[0, 4, 6, 11]]);
    }
    #[test]
    fn dl07_major_seventh_sharp_eleven() {
        row(3, "Cmaj7#11", &[&[0, 2, 4, 6, 7, 11], &[0, 4, 6, 7, 11]]);
    }
    #[test]
    fn dl08_add_ninth() {
        row(4, "Cadd9", &[&[0, 2, 4, 7], &[0, 4, 7, 14]]);
    }
    #[test]
    fn dl09_major_seventh_ninth() {
        row(5, "Cmaj9", &[&[0, 2, 4, 7, 11], &[0, 2, 4, 11]]);
    }
    #[test]
    fn dl10_sixth_ninth() {
        row(6, "C6/9", &[&[0, 2, 4, 7, 9], &[0, 2, 4, 9]]);
    }
    #[test]
    fn dl11_flat_fifth() {
        row(FLAT5, "C(b5)", &[&[0, 4, 6]]);
    }
    #[test]
    fn dl12_augmented() {
        row(7, "Caug", &[&[0, 4, 8]]);
    }
    #[test]
    fn dl13_seventh_augmented() {
        row(29, "C7#5", &[&[0, 4, 8, 10]]);
    }
    #[test]
    fn dl14_major_seventh_augmented() {
        row(28, "Cmaj7#5", &[&[0, 4, 8, 11], &[0, 8, 11]]);
    }
    #[test]
    fn dl15_minor() {
        row(8, "Cm", &[&[0, 3, 7]]);
    }
    #[test]
    fn dl16_minor_sixth() {
        row(9, "Cm6", &[&[0, 3, 7, 9]]);
    }
    #[test]
    fn dl17_minor_seventh() {
        row(10, "Cm7", &[&[0, 3, 7, 10], &[0, 3, 10]]);
    }
    #[test]
    fn dl18_minor_seventh_flat_five() {
        row(11, "Cm7b5", &[&[0, 3, 6, 10]]);
    }
    #[test]
    fn dl19_minor_add_ninth() {
        row(12, "Cm(add9)", &[&[0, 2, 3, 7]]);
    }
    #[test]
    fn dl20_minor_seventh_ninth() {
        row(13, "Cm9", &[&[0, 2, 3, 7, 10], &[0, 2, 3, 10]]);
    }
    #[test]
    fn dl21_minor_seventh_eleventh() {
        row(14, "Cm11", &[&[0, 2, 3, 5, 7, 10], &[0, 3, 5, 7, 10], &[0, 2, 3, 5, 7], &[0, 3, 5, 7]]);
    }
    #[test]
    fn dl22_minor_major_seventh_flat_five() {
        row(MM7B5, "CmMaj7b5", &[&[0, 3, 6, 11]]);
    }
    #[test]
    fn dl23_minor_major_seventh() {
        row(15, "CmMaj7", &[&[0, 3, 7, 11], &[0, 3, 11]]);
    }
    #[test]
    fn dl24_minor_major_seventh_ninth() {
        row(16, "CmMaj9", &[&[0, 2, 3, 7, 11], &[0, 2, 3, 11]]);
    }
    #[test]
    fn dl25_diminished() {
        row(17, "Cdim", &[&[0, 3, 6]]);
    }
    #[test]
    fn dl26_diminished_seventh() {
        row(18, "Cdim7", &[&[0, 3, 6, 9]]);
    }
    #[test]
    fn dl27_seventh() {
        row(19, "C7", &[&[0, 4, 7, 10], &[0, 4, 10]]);
    }
    #[test]
    fn dl28_seventh_sus_four() {
        row(20, "C7sus4", &[&[0, 5, 7, 10]]);
    }
    #[test]
    fn dl29_seventh_ninth() {
        row(22, "C9", &[&[0, 2, 4, 7, 10], &[0, 2, 4, 10]]);
    }
    #[test]
    fn dl30_seventh_sharp_eleven() {
        row(23, "C7#11", &[&[0, 2, 4, 6, 7, 10], &[0, 4, 6, 7, 10]]);
    }
    #[test]
    fn dl31_seventh_thirteenth() {
        row(24, "C13", &[&[0, 4, 7, 9, 10], &[0, 4, 9, 10]]);
    }
    #[test]
    fn dl32_seventh_flat_five() {
        row(21, "C7b5", &[&[0, 4, 6, 10]]);
    }
    #[test]
    fn dl33_seventh_flat_ninth() {
        row(25, "C7b9", &[&[0, 1, 4, 7, 10], &[0, 1, 4, 10]]);
    }
    #[test]
    fn dl34_seventh_flat_thirteenth() {
        row(26, "C7b13", &[&[0, 4, 7, 8, 10]]);
    }
    #[test]
    fn dl35_seventh_sharp_ninth() {
        row(27, "C7#9", &[&[0, 3, 4, 7, 10], &[0, 3, 4, 10]]);
    }
    #[test]
    fn dl36_sus_four() {
        row(32, "Csus4", &[&[0, 5, 7]]);
    }
    #[test]
    fn dl37_sus_two() {
        row(33, "Csus2", &[&[0, 2, 7]]);
    }
    #[test]
    fn dl38_cancel() {
        let r = Recognizer::new();
        for root in 0..12u8 {
            for v in [[0u8, 1, 2], [2, 12, 13], [1, 2, 12]] {
                let ns = v.map(|i| 48 + root + i);
                assert_eq!(keys(&r, &ns, true).map(|c| c.ty), Some(CANCEL));
                assert_eq!(keys(&r, &ns, false).map(|c| c.ty), Some(CANCEL));
            }
        }
    }

    /// Every table entry, every root, every omission subset, in every inversion: the
    /// recognised chord always holds exactly the played pitch classes.
    #[test]
    fn table_sweep_inversions_are_consistent() {
        let r = Recognizer::new();
        for &(ty, req, opt) in FINGERED.iter() {
            for sub in 0..1u16 << opt.len() {
                let mut set: Vec<u8> = req.to_vec();
                set.extend(opt.iter().enumerate().filter(|(i, _)| sub & 1 << i != 0).map(|(_, &o)| o));
                let shape = mask_of_set(&set);
                for root in 0..12u8 {
                    let mask = rot(shape, root);
                    for &iv in &set {
                        let low = (root + iv) % 12;
                        let c = r.recognize(mask, low).unwrap();
                        if ty == CANCEL {
                            assert_eq!(c.ty, CANCEL);
                            continue;
                        }
                        let (want_req, want_opt) =
                            FINGERED.iter().find(|e| e.0 == c.ty).map(|e| (mask_of_set(e.1), mask_of_set(e.2))).unwrap();
                        let (cr, co) = (rot(want_req, c.root), rot(want_opt, c.root));
                        assert!(mask & cr == cr && mask & !(cr | co) == 0, "{set:?}+{root} low {low}: {}", c.name());
                        assert_eq!(c.bass, if c.root == low { None } else { Some(low) });
                        if iv == 0 {
                            // Root in the bass: always read with that root (ambiguous sets
                            // may pick a different type of the same root, never another root).
                            assert_eq!(c.root, root, "{set:?}+{root}: {}", c.name());
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn ambiguous_c6_am7() {
        let r = Recognizer::new();
        assert_eq!(name(keys(&r, &[48, 52, 55, 57], true)), "C6");
        assert_eq!(name(keys(&r, &[45, 48, 52, 55], true)), "Am7");
        // Neither root in the bass: the reading where the bass is the more stable chord tone.
        assert_eq!(name(keys(&r, &[52, 55, 57, 60], true)), "Am7/E"); // 5th of Am7, 3rd of C6
        assert_eq!(name(keys(&r, &[43, 45, 48, 52], true)), "C6/G"); // 5th of C6, 7th of Am7
        assert_eq!(name(keys(&r, &[52, 55, 57, 60], false)), "Am7");
        // Omitted notes: C G A = C6 (no 3rd), A C G = Am7 (no 5th).
        assert_eq!(name(keys(&r, &[48, 55, 57], true)), "C6");
        assert_eq!(name(keys(&r, &[45, 48, 55], true)), "Am7");
        assert_eq!(name(keys(&r, &[43, 45, 48], true)), "C6/G");
    }

    #[test]
    fn ambiguous_m6_m7b5() {
        let r = Recognizer::new();
        assert_eq!(name(keys(&r, &[48, 51, 55, 57], true)), "Cm6");
        assert_eq!(name(keys(&r, &[45, 48, 51, 55], true)), "Am7b5");
        assert_eq!(name(keys(&r, &[51, 55, 57, 60], true)), "Cm6/Eb"); // m3 beats b5
        assert_eq!(name(keys(&r, &[43, 45, 48, 51], true)), "Cm6/G"); // 5th beats b7
    }

    #[test]
    fn symmetric_chords_take_lowest_note_as_root() {
        let r = Recognizer::new();
        // dim7 C Eb F# A, aug C E G#, 7b5 C E F# Bb (tritone-symmetric).
        for (shape, ty) in [(&[0u8, 3, 6, 9][..], 18u8), (&[0, 4, 8][..], 7)] {
            for root in 0..12u8 {
                for inv in 0..shape.len() {
                    let mut ns: Vec<u8> = shape.iter().map(|&i| 48 + root + i).collect();
                    ns.rotate_left(inv);
                    for n in ns.iter_mut().skip(shape.len() - inv) {
                        *n += 12;
                    }
                    let low = ns[0] % 12;
                    assert_eq!(keys(&r, &ns, true), Some(Chord::new(low, ty)));
                }
            }
        }
        assert_eq!(name(keys(&r, &[48, 52, 54, 58], true)), "C7b5");
        assert_eq!(name(keys(&r, &[54, 58, 60, 64], true)), "F#7b5");
        assert_eq!(name(keys(&r, &[52, 54, 58, 60], true)), "C7b5/E"); // 3rd of C7b5, 7th of F#7b5
    }

    #[test]
    fn ambiguous_sus4_sus2_and_six_nine_m11() {
        let r = Recognizer::new();
        assert_eq!(name(keys(&r, &[48, 53, 55], true)), "Csus4");
        assert_eq!(name(keys(&r, &[53, 55, 60], true)), "Fsus2");
        assert_eq!(name(keys(&r, &[43, 48, 53], true)), "Csus4/G");
        assert_eq!(name(keys(&r, &[48, 50, 52, 55, 57], true)), "C6/9");
        assert_eq!(name(keys(&r, &[45, 48, 50, 52, 55], true)), "Am11");
        // Fewest omitted notes: C6/9 needs none, Am11 would be missing its 9th.
        assert_eq!(name(keys(&r, &[50, 52, 55, 57, 60], true)), "C6/9/D");
    }

    #[test]
    fn fingered_needs_three_notes_except_one_plus_five_and_eight() {
        let r = Recognizer::new();
        assert_eq!(keys(&r, &[48, 52], true), None); // C E
        assert_eq!(keys(&r, &[48, 51], true), None); // C Eb
        assert_eq!(keys(&r, &[48, 58], true), None); // C Bb
        assert_eq!(name(keys(&r, &[43, 48], true)), "C1+5/G");
        assert_eq!(name(keys(&r, &[43, 48], false)), "C1+5");
        assert_eq!(name(keys(&r, &[48, 60], false)), "C1+8");
    }

    #[test]
    fn on_bass_slash_chords() {
        let r = Recognizer::new();
        // Inversion: On Bass keeps the bass, Fingered plays the root.
        assert_eq!(name(keys(&r, &[52, 55, 60], true)), "C/E");
        assert_eq!(name(keys(&r, &[52, 55, 60], false)), "C");
        // A bass note outside the chord: C/F# On Bass, not a Fingered chord.
        assert_eq!(name(keys(&r, &[42, 48, 52, 55], true)), "C/F#");
        assert_eq!(keys(&r, &[42, 48, 52, 55], false), None);
        // A whole table chord over the bass wins over the slash reading: D C E G = Cadd9/D.
        assert_eq!(name(keys(&r, &[50, 60, 64, 67], true)), "Cadd9/D");
        // Only a complete three- or four-note chord goes over a foreign bass: Cm7/F# works,
        // but tension-laden sets do not become an unrelated root over the bass.
        assert_eq!(name(keys(&r, &[42, 48, 51, 55, 58], true)), "Cm7/F#");
        assert_eq!(keys(&r, &[48, 52, 55, 59, 65], true), None); // C E G B F, not G13/C
        assert_eq!(keys(&r, &[48, 52, 55, 58, 61, 63], true), None); // not Eb7b9/C
        assert_eq!(keys(&r, &[48, 52, 55, 58, 61, 63], false), None);
    }

    #[test]
    fn display_only_types_follow_casm_type() {
        assert_eq!(casm_type(M7B5), 3);
        assert_eq!(casm_type(FLAT5), 21);
        assert_eq!(casm_type(MM7B5), 17);
        for ty in 0..=CANCEL {
            assert_eq!(casm_type(ty), ty);
        }
        // The mapped type holds every played note (M7b5, (b5)) or only played notes (mM7b5).
        let (m7b5, flat5, mm7b5) = (mask_of(M7B5), mask_of(FLAT5), mask_of(MM7B5));
        assert_eq!(m7b5 & !mask_of(3), 0);
        assert_eq!(flat5 & !mask_of(21), 0);
        assert_eq!(mask_of(17) & !mm7b5, 0);
        // Chord mute is looked up with the CASM bit.
        let mut r = ChannelRule::default_for(12);
        r.chord_mute = 1 << 3;
        assert!(plays(&r, Chord::new(0, M7B5)));
        assert!(!plays(&r, Chord::new(0, FLAT5)));
        let c = Chord { root: 9, ty: MM7B5, bass: Some(0) };
        assert_eq!(Chord::unpack(c.pack(7)), Some((c, 7)));
    }

    #[test]
    fn pack_roundtrip() {
        let c = Chord { root: 9, ty: 10, bass: Some(7) };
        assert_eq!(Chord::unpack(c.pack(42)), Some((c, 42)));
        let c = Chord::new(11, CANCEL);
        assert_eq!(Chord::unpack(c.pack(1)).unwrap().0, c);
    }

    #[test]
    fn spec_root_trans_and_fixed() {
        // Wierzba §5.2.5.1: C3 E3 G3 in C -> F: Root Trans = F3 A3 C4, Root Fixed = C3 F3 A3.
        // (Yamaha C3 = MIDI 60.)
        let f = Chord::new(5, 0);
        let rt = rule(Ntr::RootTrans, Ntt::Chord, 11, 0, 127);
        let mut r2 = rt.clone();
        r2.src_type = 0;
        let out: Vec<u8> = [60, 64, 67].iter().map(|&k| transpose(k, &r2, f).unwrap()).collect();
        assert_eq!(names(&out), ["F3", "A3", "C4"]);
        let mut rf = rule(Ntr::RootFixed, Ntt::Chord, 11, 0, 127);
        rf.src_type = 0;
        let mut out = [None; 3];
        transpose_group(&[60, 64, 67], &rf, f, &mut out);
        assert_eq!(names(&out.map(|o| o.unwrap())), ["C3", "F3", "A3"]);
    }

    #[test]
    fn spec_high_key() {
        // §5.2.7, HIGH KEY = F: roots above F drop an octave.
        let mut r = rule(Ntr::RootTrans, Ntt::Melody, 5, 0, 127);
        r.src_type = 0;
        let play = |root| [60u8, 64, 67].map(|k| transpose(k, &r, Chord::new(root, 0)).unwrap());
        assert_eq!(names(&play(0)), ["C3", "E3", "G3"]);
        assert_eq!(names(&play(5)), ["F3", "A3", "C4"]);
        assert_eq!(names(&play(6)), ["F#2", "Bb2", "C#3"]);
    }

    #[test]
    fn spec_note_limit() {
        // §5.2.6, LOW = C3, HIGH = D4, root motion C C# Eb.
        let mut r = rule(Ntr::RootTrans, Ntt::Melody, 11, 60, 74);
        r.src_type = 0;
        let play = |root| {
            let mut v = [64u8, 67, 72].map(|k| transpose(k, &r, Chord::new(root, 0)).unwrap());
            v.sort();
            names(&v)
        };
        assert_eq!(play(0), ["E3", "G3", "C4"]);
        assert_eq!(play(1), ["F3", "Ab3", "C#4"]);
        assert_eq!(play(3), ["Eb3", "G3", "Bb3"]);
    }

    #[test]
    fn melody_major_to_minor() {
        let r = rule(Ntr::RootTrans, Ntt::Melody, 11, 0, 127);
        // CMaj7 source line C D E G B over Am7 -> A B C E G.
        let out: Vec<u8> = [60, 62, 64, 67, 71].iter().map(|&k| transpose(k, &r, Chord::new(9, 10)).unwrap()).collect();
        assert_eq!(names(&out), ["A3", "B3", "C4", "E4", "G4"]);
    }

    #[test]
    fn chord_table_maj7_to_triad_and_dom7() {
        let r = rule(Ntr::RootFixed, Ntt::Chord, 11, 0, 127);
        let v = |c| {
            let mut out = [None; 3];
            transpose_group(&[64, 67, 71], &r, c, &mut out);
            let mut v: Vec<u8> = out.iter().map(|o| o.unwrap()).collect();
            v.sort();
            names(&v)
        };
        assert_eq!(v(Chord::new(0, 0)), ["E3", "G3", "C4"]); // B -> C
        assert_eq!(v(Chord::new(0, 19)), ["E3", "G3", "Bb3"]);
        assert_eq!(v(Chord::new(7, 19)), ["D3", "F3", "B3"]); // G7: 3rd, 5th, 7th, minimal movement
    }

    #[test]
    fn bass_on_slash() {
        let mut r = rule(Ntr::RootTrans, Ntt::Melody, 11, 0, 127);
        r.bass_on = true;
        let c_over_e = Chord { root: 0, ty: 0, bass: Some(4) };
        assert_eq!(transpose(36, &r, c_over_e), Some(40));
    }

    #[test]
    fn drums_untouched() {
        let mut r = rule(Ntr::RootTrans, Ntt::Melody, 11, 0, 127);
        r.dest_ch = 9;
        assert_eq!(transpose(38, &r, Chord::new(7, 10)), Some(38));
    }

    /// Every NTT that follows the chord, over G1+8 and G1+5, for a CM7 and a Cm7 source.
    fn no_third_outputs(chord: Chord) -> Vec<u8> {
        let mut pcs = Vec::new();
        for (ntr, ntt) in [(Ntr::RootTrans, Ntt::Melody), (Ntr::RootFixed, Ntt::Chord), (Ntr::RootTrans, Ntt::Bass),
                           (Ntr::RootTrans, Ntt::HarmonicMinor), (Ntr::RootTrans, Ntt::Dorian5),
                           (Ntr::Guitar, Ntt::GuitarAllPurpose)] {
            for src_type in [2u8, 10] {
                let mut r = rule(ntr, ntt, 11, 0, 127);
                r.src_type = src_type;
                // Guitar mutes strings the voicing does not use (None).
                pcs.extend((48..72u8).filter_map(|k| transpose(k, &r, chord)).map(|n| n % 12));
            }
        }
        pcs
    }

    #[test]
    fn one_plus_eight_plays_only_the_root() {
        // Chromatic notes (Eb, Bb) included: nothing but G may come out.
        let pcs = no_third_outputs(Chord::new(7, 30));
        assert!(pcs.iter().all(|&p| p == 7), "{pcs:?}");
    }

    #[test]
    fn one_plus_five_is_quality_neutral() {
        // Root, 5th and the 2nd / 4th that major and minor share; never a 3rd, 6th or 7th.
        let pcs = no_third_outputs(Chord::new(7, 31));
        assert!(pcs.iter().all(|&p| matches!(p, 7 | 9 | 0 | 2)), "{pcs:?}");
        // Chord parts keep only root and 5th, whatever the source chord (sus, 1+8 and 1+5
        // included) and key (chord tones, tensions and chromatic notes alike).
        for ntr in [Ntr::RootFixed, Ntr::RootTrans] {
            for src_type in 0..34u8 {
                let mut r = rule(ntr, Ntt::Chord, 11, 0, 127);
                r.src_type = src_type;
                for k in 48..72u8 {
                    let out = transpose(k, &r, Chord::new(7, 31)).unwrap() % 12;
                    assert!(matches!(out, 7 | 2), "{ntr:?} src {src_type} key {k}: {out}");
                }
                let mut out = [None; 3];
                transpose_group(&[64, 67, 71], &r, Chord::new(7, 31), &mut out);
                assert!(out.iter().all(|o| matches!(o.unwrap() % 12, 7 | 2)), "{out:?}");
            }
        }
        // Melody: the 3rd goes to the 5th, the 7th to the octave.
        let m = rule(Ntr::RootTrans, Ntt::Melody, 11, 0, 127);
        let out: Vec<u8> = [60, 64, 67, 71].iter().map(|&k| transpose(k, &m, Chord::new(0, 31)).unwrap()).collect();
        assert_eq!(names(&out), ["C3", "G3", "G3", "C4"]);
    }

    #[test]
    fn source_recorded_over_one_plus_five() {
        // A C1+5 pattern (C D F G) follows other chords like a plain major source.
        let mut r = rule(Ntr::RootTrans, Ntt::Melody, 11, 0, 127);
        r.src_type = 31;
        let play = |c| [60u8, 62, 65, 67].map(|k| transpose(k, &r, c).unwrap());
        assert_eq!(names(&play(Chord::new(0, 31))), ["C3", "D3", "F3", "G3"]);
        assert_eq!(names(&play(Chord::new(9, 8))), ["A3", "B3", "D4", "E4"]);
        let mut c = rule(Ntr::RootFixed, Ntt::Chord, 11, 0, 127);
        c.src_type = 31;
        let mut out = [None; 2];
        transpose_group(&[60, 67], &c, Chord::new(0, 17), &mut out); // Cdim: 5th -> b5
        assert_eq!(names(&out.map(|o| o.unwrap())), ["C3", "F#3"]);
    }

    #[test]
    fn cancel_has_no_pitches() {
        let r = rule(Ntr::RootTrans, Ntt::Melody, 11, 0, 127);
        assert_eq!(transpose(60, &r, Chord::new(0, CANCEL)), None);
        assert!(!plays(&r, Chord::new(0, CANCEL)));
    }

    #[test]
    fn guitar_voicing_has_chord_tones() {
        for (root, ty) in [(0u8, 0u8), (7, 19), (9, 8), (2, 10)] {
            let c = Chord::new(root, ty);
            let v = guitar_voicing(c, 0);
            let tones = rot(mask_of(ty), root);
            for n in v.iter().flatten() {
                assert!(tones & (1 << (n % 12)) != 0, "{} has non-chord tone", c.name());
            }
            assert_eq!(v.iter().rev().flatten().next().unwrap() % 12, root, "{} bass", c.name());
        }
    }
}

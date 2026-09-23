//! Chord types, chord recognition, and style note transposition (NTR/NTT).
//!
//! Everything here is allocation-free so it can run on the real-time threads.

use crate::sff::{ChannelRule, Ntr, Ntt, Zone};

pub const CANCEL: u8 = 0x22;
pub const NUM_TYPES: usize = 34;

/// Chord type ids match the Yamaha source-chord / chord-mute numbering.
pub const TYPE_NAMES: [&str; 35] = [
    "", "6", "maj7", "maj7#11", "add9", "maj9", "6/9", "aug", "m", "m6", "m7", "m7b5", "m(add9)",
    "m9", "m11", "mMaj7", "mMaj9", "dim", "dim7", "7", "7sus4", "7b5", "9", "7#11", "13", "7b9",
    "7b13", "7#9", "maj7#5", "7#5", "1+8", "5", "sus4", "sus2", "cancel",
];

/// Pitch classes of each chord type relative to the root.
const TONES: [&[u8]; NUM_TYPES] = [
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
];

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
}

/// Pitch classes (relative to the root) of a chord type.
pub fn chord_tones(ty: u8) -> &'static [u8] {
    TONES[(ty as usize).min(NUM_TYPES - 1)]
}

fn mask_of(ty: u8) -> u16 {
    TONES[ty as usize].iter().fold(0, |m, &t| m | 1 << t)
}

fn rot(mask: u16, by: u8) -> u16 {
    let by = by % 12;
    ((mask << by) | (mask >> (12 - by))) & 0x0FFF
}

// ---------------------------------------------------------------------------
// Recognition
// ---------------------------------------------------------------------------

/// Precomputed "Fingered On Bass" recognizer: 4096 pitch-class masks × 12 lowest notes.
pub struct Recognizer {
    table: Box<[u16; 4096 * 12]>,
}

const NO_CHORD: u16 = 0xFFFF;

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
                if let Some(c) = Self::compute(mask, low) {
                    // [bass:4][bass_present:1][root:4][ty:6]
                    let b = c.bass.map(|b| 0x400 | (b as u16) << 11).unwrap_or(0);
                    table[mask as usize * 12 + low as usize] = b | (c.root as u16) << 6 | c.ty as u16;
                }
            }
        }
        Recognizer { table }
    }

    fn compute(mask: u16, low: u8) -> Option<Chord> {
        let n = mask.count_ones();
        // Three adjacent semitones = chord cancel.
        if n == 3 {
            for r in 0..12u8 {
                if rot(0b111, r) == mask {
                    return Some(Chord::new(0, CANCEL));
                }
            }
        }
        if n == 1 {
            return Some(Chord::new(low, 0));
        }
        if n == 2 {
            let other = (0..12u8).find(|&p| p != low && mask & (1 << p) != 0).unwrap();
            let iv = (other + 12 - low) % 12;
            return match iv {
                7 => Some(Chord::new(low, 31)),
                5 => Some(Chord { root: other, ty: 31, bass: Some(low) }.slash_or_plain()),
                4 => Some(Chord::new(low, 0)),
                3 => Some(Chord::new(low, 8)),
                10 => Some(Chord::new(low, 19)),
                11 => Some(Chord::new(low, 2)),
                8 => Some(Chord { root: other, ty: 0, bass: Some(low) }),
                9 => Some(Chord { root: other, ty: 8, bass: Some(low) }),
                _ => None,
            };
        }
        // Score every root/type: exact match beats omitted fifth; root on bass beats slash.
        let mut best: Option<(i32, Chord)> = None;
        for ty in (0..30u8).chain(32..34) {
            let tm = mask_of(ty);
            for root in 0..12u8 {
                let cm = rot(tm, root);
                let fifth = TONES[ty as usize].iter().find(|&&t| t == 7).map(|_| rot(1 << 7, root));
                let score = if cm == mask {
                    100
                } else if let Some(f) = fifth {
                    if TONES[ty as usize].len() >= 4 && cm & !f == mask {
                        60
                    } else {
                        continue;
                    }
                } else {
                    continue;
                };
                let score = score + if root == low { 20 } else { 0 } - ty as i32 / 8;
                if best.map_or(true, |(s, _)| score > s) {
                    let bass = if root == low { None } else { Some(low) };
                    best = Some((score, Chord { root, ty, bass }));
                }
            }
        }
        if let Some((_, c)) = best {
            return Some(c);
        }
        // On-bass: try the chord without the lowest note.
        let upper = mask & !(1 << low);
        if upper.count_ones() >= 3 {
            let lowest_upper = (1..12u8).map(|i| (low + i) % 12).find(|&p| upper & (1 << p) != 0).unwrap();
            if let Some(c) = Self::compute(upper, lowest_upper) {
                if c.ty != CANCEL {
                    return Some(Chord { bass: Some(low), ..Chord { bass: None, ..c } });
                }
            }
        }
        None
    }

    /// Recognize from a pitch-class mask and the lowest held pitch class.
    #[inline]
    pub fn recognize(&self, mask: u16, low: u8) -> Option<Chord> {
        if mask == 0 {
            return None;
        }
        let v = self.table[(mask & 0x0FFF) as usize * 12 + low as usize % 12];
        if v == NO_CHORD {
            return None;
        }
        let bass = if v & 0x400 != 0 { Some(((v >> 11) & 0xF) as u8) } else { None };
        Some(Chord { root: ((v >> 6) & 0xF) as u8, ty: (v & 0x3F) as u8, bass })
    }
}

impl Chord {
    fn slash_or_plain(self) -> Chord {
        match self.bass {
            Some(b) if b == self.root => Chord { bass: None, ..self },
            _ => self,
        }
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
        30 => s = [0, 0, 0, 0, 0, 0, 12],
        31 => s = [0, 2, 7, 5, 7, 9, 12],
        32 => s[2] = 5,
        33 => s[2] = 2,
        _ => {}
    }
    s
}

fn scale_map(d: u8, src: &[u8; 7], tgt: &[u8; 7]) -> i8 {
    let mut idx = 0;
    for i in 0..7 {
        if src[i] <= d {
            idx = i;
        }
    }
    let off = d as i8 - src[idx] as i8;
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
    let (si, so, _) = importance(src_ty);
    let (ti, to, _) = importance(tgt_ty);
    for i in 0..3 {
        if si[i] % 12 == d {
            return ti[i] as i8 - if si[i] >= 12 { 12 } else { 0 };
        }
    }
    if d == 0 {
        return 0;
    }
    if so[1] != 0 && so[1] % 12 == d {
        return if to[1] != 0 { to[1] as i8 } else { ti[1] as i8 };
    }
    scale_map(d, src_scale, tgt_scale)
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
#[inline]
pub fn plays(rule: &ChannelRule, chord: Chord) -> bool {
    if chord.ty == CANCEL {
        return is_drum_part(rule.dest_ch);
    }
    rule.chord_mute & (1u64 << chord.ty) != 0 && rule.note_mute & (1 << chord.root) != 0
}

/// Transpose one source note for the target chord. Returns None if the note should not sound.
pub fn transpose(key: u8, rule: &ChannelRule, chord: Chord) -> Option<u8> {
    if is_drum_part(rule.dest_ch) {
        return Some(key);
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
            let s = scale(rule.src_type, Ntt::Melody);
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
            let s = scale(rule.src_type, ntt);
            let t = scale(chord.ty, ntt);
            scale_map(d, &s, &t) as i32
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
    let tones = rot(mask_of(chord.ty.min(33)), chord.root);
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
            // Root (C) or fifth (C#): the lowest voiced note, or a fifth above it.
            let low = v.iter().rev().flatten().next().copied()?;
            let n = if d == 0 { low } else { low + 7 };
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
        assert_eq!(r.recognize(m(&[60]), 0).unwrap().name(), "C");
        assert_eq!(r.recognize(m(&[60, 67]), 0).unwrap().name(), "C5");
        assert_eq!(r.recognize(m(&[60, 65, 67]), 0).unwrap().name(), "Csus4");
        assert_eq!(r.recognize(m(&[62, 65, 69, 72]), 2).unwrap().name(), "Dm7");
        assert_eq!(r.recognize(m(&[60, 64, 67, 70, 74]), 0).unwrap().name(), "C9");
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

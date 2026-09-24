//! iReal Pro chord qualities to yahaha chord types. The table is mirrored in docs/ireal.md.

use crate::theory::Chord;

/// How closely a yahaha chord type matches an iReal quality.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fit {
    /// The same notes (tensions the yahaha type treats as optional count as the same).
    Exact,
    /// The nearest yahaha type; some tension is dropped or changed (see docs/ireal.md).
    Approx,
    /// Not an iReal quality we know: guessed from its spelling by [`fallback_type`].
    Fallback,
}

/// One row of the mapping table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quality {
    /// The quality as iReal spells it in a chart (`-7`, `^7#11`, `7alt`, ...).
    pub ireal: &'static str,
    /// yahaha chord type id (index into `theory::TYPE_NAMES`).
    pub ty: u8,
    pub fit: Fit,
}

const fn q(ireal: &'static str, ty: u8, fit: Fit) -> Quality {
    Quality { ireal, ty, fit }
}

use Fit::{Approx as A, Exact as E};

/// Every quality iReal Pro's chord editor offers, in its own order.
pub const QUALITIES: [Quality; 62] = [
    q("", 0, E),
    q("5", 31, E),
    q("2", 33, A),
    q("add9", 4, E),
    q("+", 7, E),
    q("o", 17, E),
    q("h", 11, E),
    q("sus", 32, E),
    q("^", 2, E),
    q("-", 8, E),
    q("^7", 2, E),
    q("-7", 10, E),
    q("7", 19, E),
    q("7sus", 20, E),
    q("h7", 11, E),
    q("o7", 18, E),
    q("^9", 5, E),
    q("^13", 5, A),
    q("6", 1, E),
    q("69", 6, E),
    q("^7#11", 3, E),
    q("^9#11", 3, A),
    q("^7#5", 28, E),
    q("-6", 9, E),
    q("-69", 9, A),
    q("-^7", 15, E),
    q("-^9", 16, E),
    q("-9", 13, E),
    q("-11", 14, A),
    q("-7b5", 11, E),
    q("h9", 11, A),
    // No minor-#5 type: keep the written root and the minor third, drop the #5 / b6
    // (re-rooting to Ab/C would be the same notes but shows the player the wrong root).
    q("-b6", 8, A),
    q("-#5", 8, A),
    q("9", 22, E),
    q("7b9", 25, E),
    q("7#9", 27, E),
    q("7#11", 23, E),
    q("7b5", 21, E),
    q("7#5", 29, E),
    q("9#11", 23, A),
    q("9b5", 21, A),
    q("9#5", 29, A),
    q("7b13", 26, E),
    q("7#9#5", 27, A),
    q("7#9b5", 27, A),
    q("7#9#11", 27, A),
    q("7b9#11", 25, A),
    q("7b9b5", 25, A),
    q("7b9#5", 25, A),
    q("7b9#9", 25, A),
    q("7b9b13", 25, A),
    q("7alt", 27, A),
    q("13", 24, E),
    q("13#11", 24, A),
    q("13b9", 24, A),
    q("13#9", 24, A),
    q("7b9sus", 20, A),
    q("7susadd3", 20, A),
    q("9sus", 20, A),
    q("13sus", 20, A),
    q("7b13sus", 20, A),
    q("11", 20, A),
];

/// The longest table quality `s` starts with (always matches: `""` is in the table).
pub(crate) fn longest_known_prefix(s: &str) -> &'static str {
    QUALITIES.iter().map(|q| q.ireal).filter(|q| s.starts_with(q)).max_by_key(|q| q.len()).unwrap_or("")
}

/// Map an iReal quality to a yahaha chord type.
pub fn map_quality(quality: &str) -> Quality {
    QUALITIES.iter().find(|q| q.ireal == quality).copied().unwrap_or(Quality {
        ireal: "",
        ty: fallback_type(quality),
        fit: Fit::Fallback,
    })
}

/// A yahaha type for a quality that isn't in the table (a custom `*...*` quality or an
/// unusual stack), read from its spelling: minor / major-7 / half-diminished / diminished /
/// augmented / sus prefixes first, then the most characteristic dominant tension.
pub fn fallback_type(q: &str) -> u8 {
    let has = |p: &str| q.contains(p);
    let ext = has("7") || has("9") || has("11") || has("13");
    if let Some(r) = q.strip_prefix('-').or_else(|| q.strip_prefix('m').filter(|r| !r.starts_with('a'))) {
        return if r.contains('^') || r.starts_with("M") {
            15
        } else if r.contains("b5") {
            11
        } else if ext {
            10
        } else if r.contains('6') {
            9
        } else {
            8
        };
    }
    if q.starts_with('^') || q.starts_with("maj") || q.starts_with('M') {
        return if has("#5") || has("+") { 28 } else if has("#11") { 3 } else { 2 };
    }
    if q.starts_with('h') {
        return 11;
    }
    if q.starts_with('o') || q.starts_with("dim") {
        return if ext { 18 } else { 17 };
    }
    if q.starts_with('+') || q.starts_with("aug") {
        return if ext { 29 } else { 7 };
    }
    if has("sus") {
        return if ext { 20 } else { 32 };
    }
    if has("alt") {
        return 27;
    }
    if q.starts_with('6') {
        return if has("9") { 6 } else { 1 };
    }
    if ext {
        return if has("#9") {
            27
        } else if has("b9") {
            25
        } else if has("#11") {
            23
        } else if has("b13") {
            26
        } else if has("#5") || has("+") {
            29
        } else if has("b5") {
            21
        } else if has("13") {
            24
        } else if has("9") {
            22
        } else {
            19
        };
    }
    0
}

/// A chart chord (root and bass as pitch classes, the quality as written) as a yahaha chord.
pub fn to_chord(root: u8, quality: &str, bass: Option<u8>) -> Chord {
    let m = map_quality(quality);
    let r = root % 12;
    Chord { root: r, ty: m.ty, bass: bass.map(|b| b % 12).filter(|&b| b != r) }
}

//! The `irealb://` chart obfuscation.
//!
//! Source: the scheme is described in the public write-ups of the iReal Pro link format
//! that the open-source readers are built on, e.g. the `unscramble` notes of
//! pianosnake/ireal-reader (<https://github.com/pianosnake/ireal-reader>) and the
//! "iReal Pro file format" discussion it links. This is a clean implementation of that
//! description; no code is copied from any project.
//!
//! The chart after the `1r34LbKcu7` marker is scrambled in 50-character blocks, taken from
//! the front while more than 51 characters remain; the tail (51 characters or fewer) is left
//! as it is. Inside a block, characters 0..5 trade places with 49..44 (mirror image) and
//! characters 10..24 with 39..26. The permutation is its own inverse, so the same step
//! scrambles and unscrambles. After unscrambling three abbreviations expand:
//! `Kcl` -> `| x` (a one-bar repeat), `LZ` -> ` |` and `XyQ` -> three empty cells.

/// The marker every scrambled chart starts with.
pub const MUSIC_PREFIX: &str = "1r34LbKcu7";

fn permute(block: &mut [char]) {
    for i in (0..5).chain(10..24) {
        block.swap(i, 49 - i);
    }
}

fn blocks(chars: &mut [char]) {
    let mut at = 0;
    while chars.len() - at > 51 {
        permute(&mut chars[at..at + 50]);
        at += 50;
    }
}

/// Unscramble an `irealb://` music field (with or without the `1r34LbKcu7` marker) into
/// plain chart text.
pub fn unscramble(s: &str) -> String {
    let s = s.strip_prefix(MUSIC_PREFIX).unwrap_or(s);
    let mut c: Vec<char> = s.chars().collect();
    blocks(&mut c);
    c.into_iter().collect::<String>().replace("Kcl", "| x").replace("LZ", " |").replace("XyQ", "   ")
}

/// The inverse of [`unscramble`]: plain chart text to an `irealb://` music field, marker
/// included. Round-trips any chart that doesn't itself contain `Kcl`, `LZ` or `XyQ`.
pub fn scramble(chart: &str) -> String {
    let s = chart.replace("| x", "Kcl").replace(" |", "LZ").replace("   ", "XyQ");
    let mut c: Vec<char> = s.chars().collect();
    blocks(&mut c);
    let mut out = String::from(MUSIC_PREFIX);
    out.extend(c);
    out
}

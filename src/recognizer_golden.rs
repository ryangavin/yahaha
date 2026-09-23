//! Golden recognizer tests driven by the Data List chord table (docs/genos-features.md §D).
//!
//! Every row is played in all 12 roots, in every inversion (each chord tone as the lowest
//! note) and with every combination of its optional "(…)" notes left out. The recognizer must
//! return the expected root, type and on-bass note. Expectations follow Fingered On Bass, which
//! is what the recognizer implements today: the root in the bass gives a plain chord, any other
//! lowest note becomes the on-bass note (E G C = C/E).
//!
//! Some pitch-class sets spell more than one row or root (C6 = Am7, dim7, aug, ...). When
//! exactly one of those readings has the lowest note as its root, that reading is the answer
//! (C E G A = C6, A C E G = Am7); that voicing is then tested under its own row. Otherwise the
//! voicing is ambiguous (E G A C: C6/E or Am7/E?). The per-row tests skip those;
//! `ambiguous_voicings` checks them and only asks for one of the valid readings until #2
//! settles the priorities.
//!
//! Rows whose behaviour is not implemented yet are `#[ignore = "M1 #<ticket>"]`: run them with
//! `cargo test --release -- --ignored recognizer_golden`, and un-ignore a row once it passes.

use crate::theory::{Chord, Recognizer, CANCEL, NOTE_NAMES};

struct Row {
    /// Row number in the Data List table (DL p.45).
    n: u8,
    name: &'static str,
    /// MIDI chord type (DL p.111); None for the rows with no code (M7b5, (b5), mM7b5).
    ct: Option<u8>,
    /// Semitones above the root, root first. 1+8 uses 12 for the octave.
    tones: &'static [u8],
    /// The notes in parentheses, which may be left out.
    optional: &'static [u8],
}

const fn row(n: u8, name: &'static str, ct: Option<u8>, tones: &'static [u8], optional: &'static [u8]) -> Row {
    Row { n, name, ct, tones, optional }
}

#[rustfmt::skip]
const TABLE: [Row; 38] = [
    row(1,  "1+8",     Some(30), &[0, 12],              &[]),
    row(2,  "1+5",     Some(31), &[0, 7],               &[]),
    row(3,  "M",       Some(0),  &[0, 4, 7],            &[]),
    row(4,  "6",       Some(1),  &[0, 4, 7, 9],         &[4]),
    row(5,  "M7",      Some(2),  &[0, 4, 7, 11],        &[7]),
    row(6,  "M7b5",    None,     &[0, 4, 6, 11],        &[]),
    row(7,  "M7(#11)", Some(3),  &[0, 2, 4, 6, 7, 11],  &[2]),
    row(8,  "(9)",     Some(4),  &[0, 2, 4, 7],         &[]),
    row(9,  "M7_9",    Some(5),  &[0, 2, 4, 7, 11],     &[7]),
    row(10, "6_9",     Some(6),  &[0, 2, 4, 7, 9],      &[7]),
    row(11, "(b5)",    None,     &[0, 4, 6],            &[]),
    row(12, "aug",     Some(7),  &[0, 4, 8],            &[]),
    row(13, "7aug",    Some(29), &[0, 4, 8, 10],        &[]),
    row(14, "M7aug",   Some(28), &[0, 4, 8, 11],        &[4]),
    row(15, "m",       Some(8),  &[0, 3, 7],            &[]),
    row(16, "m6",      Some(9),  &[0, 3, 7, 9],         &[]),
    row(17, "m7",      Some(10), &[0, 3, 7, 10],        &[7]),
    row(18, "m7b5",    Some(11), &[0, 3, 6, 10],        &[]),
    row(19, "m(9)",    Some(12), &[0, 2, 3, 7],         &[]),
    row(20, "m7(9)",   Some(13), &[0, 2, 3, 7, 10],     &[7]),
    row(21, "m7(11)",  Some(14), &[0, 2, 3, 5, 7, 10],  &[2, 10]),
    row(22, "mM7b5",   None,     &[0, 3, 6, 11],        &[]),
    row(23, "mM7",     Some(15), &[0, 3, 7, 11],        &[7]),
    row(24, "mM7(9)",  Some(16), &[0, 2, 3, 7, 11],     &[7]),
    row(25, "dim",     Some(17), &[0, 3, 6],            &[]),
    row(26, "dim7",    Some(18), &[0, 3, 6, 9],         &[]),
    row(27, "7",       Some(19), &[0, 4, 7, 10],        &[7]),
    row(28, "7sus4",   Some(20), &[0, 5, 7, 10],        &[]),
    row(29, "7(9)",    Some(22), &[0, 2, 4, 7, 10],     &[7]),
    row(30, "7(#11)",  Some(23), &[0, 2, 4, 6, 7, 10],  &[2]),
    row(31, "7(13)",   Some(24), &[0, 4, 7, 9, 10],     &[7]),
    row(32, "7b5",     Some(21), &[0, 4, 6, 10],        &[]),
    row(33, "7(b9)",   Some(25), &[0, 1, 4, 7, 10],     &[7]),
    row(34, "7(b13)",  Some(26), &[0, 4, 7, 8, 10],     &[]),
    row(35, "7(#9)",   Some(27), &[0, 3, 4, 7, 10],     &[7]),
    row(36, "sus4",    Some(32), &[0, 5, 7],            &[]),
    row(37, "sus2",    Some(33), &[0, 2, 7],            &[]),
    row(38, "cancel",  Some(CANCEL), &[0, 1, 2],        &[]),
];

/// One played voicing of a row.
struct Case {
    row: &'static Row,
    root: u8,
    /// MIDI notes, lowest first.
    notes: Vec<u8>,
    omitted: Vec<u8>,
}

impl Case {
    fn low(&self) -> u8 {
        self.notes[0] % 12
    }

    fn mask(&self) -> u16 {
        self.notes.iter().fold(0, |m, n| m | 1 << (n % 12))
    }

    fn expected(&self) -> Chord {
        let low = self.low();
        Chord { root: self.root, ty: self.row.ct.unwrap_or(0), bass: (low != self.root).then_some(low) }
    }

    fn describe(&self) -> String {
        let notes: Vec<String> =
            self.notes.iter().map(|&k| format!("{}{}", NOTE_NAMES[k as usize % 12], k as i32 / 12 - 2)).collect();
        let omitted = if self.omitted.is_empty() {
            String::new()
        } else {
            format!(" (omit {:?})", self.omitted)
        };
        format!("row {:>2} {} on {}: [{}]{omitted}", self.row.n, self.row.name, NOTE_NAMES[self.root as usize], notes.join(" "))
    }
}

/// The recognizer's input for a set of held notes. #2/#3 may widen this (1+8 needs the note
/// count, not just the pitch-class mask); every golden test goes through here.
fn recognize(r: &Recognizer, notes: &[u8]) -> Option<Chord> {
    let mask = notes.iter().fold(0u16, |m, n| m | 1 << (n % 12));
    let low = *notes.iter().min()? % 12;
    r.recognize(mask, low)
}

/// Every root x omission subset x inversion of a row. Voicings are close position from
/// the chosen lowest tone upward, starting at C2 (MIDI 48).
fn cases(row: &'static Row) -> Vec<Case> {
    let mut out = Vec::new();
    for root in 0..12u8 {
        for subset in 0..1u32 << row.optional.len() {
            let omitted: Vec<u8> =
                row.optional.iter().enumerate().filter(|(i, _)| subset & 1 << i != 0).map(|(_, &t)| t).collect();
            let tones: Vec<u8> = row.tones.iter().copied().filter(|t| !omitted.contains(t)).collect();
            // 1+8: the octave is a second key, not a new pitch class to invert on.
            let inversions = if row.tones.contains(&12) { 1 } else { tones.len() };
            for inv in 0..inversions {
                let mut notes = Vec::with_capacity(tones.len());
                let mut prev = 0u8;
                for (i, &t) in tones[inv..].iter().chain(&tones[..inv]).enumerate() {
                    let pc = (root + t) % 12;
                    let mut k = 48 + pc;
                    while i > 0 && k <= prev {
                        k += 12;
                    }
                    notes.push(k);
                    prev = k;
                }
                out.push(Case { row, root, notes, omitted: omitted.clone() });
            }
        }
    }
    out
}

/// Every (row, root) that can spell this pitch-class set, with or without its optional notes.
fn readings(mask: u16) -> Vec<(&'static Row, u8)> {
    let mut v: Vec<(&'static Row, u8)> = Vec::new();
    for row in TABLE.iter().filter(|r| r.ct != Some(CANCEL)) {
        for c in cases(row).into_iter().filter(|c| c.mask() == mask) {
            if !v.iter().any(|(r, root)| r.n == row.n && *root == c.root) {
                v.push((row, c.root));
            }
        }
    }
    v
}

/// The readings a player could mean by this voicing. With exactly one reading, or exactly
/// one whose root is the lowest note (C E G A = C6, A C E G = Am7), the voicing has a single
/// right answer; otherwise it is ambiguous and #2 has to pick.
fn intended(c: &Case) -> Vec<(&'static Row, u8)> {
    if c.row.ct == Some(CANCEL) {
        return vec![(c.row, c.root)];
    }
    let all = readings(c.mask());
    let rooted: Vec<_> = all.iter().copied().filter(|(_, root)| *root == c.low()).collect();
    if rooted.is_empty() { all } else { rooted }
}

fn ambiguous(c: &Case) -> bool {
    intended(c).len() > 1
}

/// Compare the recognizer with the table for one row; returns a readable failure report.
fn check_row(n: u8) {
    let r = Recognizer::new();
    let row = TABLE.iter().find(|r| r.n == n).unwrap();
    let mut fails = Vec::new();
    let mut checked = 0;
    for c in cases(row) {
        // Ambiguous voicings are checked in `ambiguous_voicings`; voicings that really spell
        // another row (Eb G Bb C is Eb6, not Cm7/Eb) are checked by that row.
        match intended(&c).as_slice() {
            [(rw, root)] if rw.n == c.row.n && *root == c.root => {}
            _ => continue,
        }
        checked += 1;
        let got = recognize(&r, &c.notes);
        let want = c.expected();
        let ok = match (row.ct, got) {
            (Some(CANCEL), Some(g)) => g.ty == CANCEL,
            (Some(_), Some(g)) => g == want,
            // No MIDI code yet: whatever comes back is a different row (C E F# B read as
            // Cmaj7#11 names a G that is not played). #2 decides the type and fills in `ct`.
            (None, _) | (_, None) => false,
        };
        if !ok {
            let want = if row.ct.is_some() {
                want.name()
            } else {
                let bass = want.bass.map_or(String::new(), |b| format!("/{}", NOTE_NAMES[b as usize]));
                format!("{}{}{bass} (type per #2)", NOTE_NAMES[want.root as usize], row.name)
            };
            let got = got.map_or("nothing".to_string(), |g| g.name());
            fails.push(format!("  {}  want {want}, got {got}", c.describe()));
        }
    }
    assert!(checked > 0, "row {n} {}: no voicing has a single reading", row.name);
    assert!(
        fails.is_empty(),
        "row {n} {}: {} of {checked} voicings wrong\n{}",
        row.name,
        fails.len(),
        fails.iter().take(40).cloned().collect::<Vec<_>>().join("\n")
    );
}

macro_rules! rows {
    ($($(#[$m:meta])* $name:ident = $n:expr;)*) => {
        $( $(#[$m])* #[test] fn $name() { check_row($n); } )*
    };
}

rows! {
    #[ignore = "M1 #2"] row_01_one_plus_eight = 1;
    row_02_one_plus_five = 2;
    row_03_major = 3;
    #[ignore = "M1 #2"] row_04_sixth = 4;
    row_05_major_seventh = 5;
    #[ignore = "M1 #2"] row_06_major_seventh_flat_five = 6;
    #[ignore = "M1 #2"] row_07_major_seventh_sharp_eleven = 7;
    row_08_add_nine = 8;
    row_09_major_seventh_nine = 9;
    row_10_six_nine = 10;
    #[ignore = "M1 #2"] row_11_flat_five = 11;
    row_12_augmented = 12;
    row_13_seventh_augmented = 13;
    #[ignore = "M1 #2"] row_14_major_seventh_augmented = 14;
    row_15_minor = 15;
    row_16_minor_sixth = 16;
    row_17_minor_seventh = 17;
    row_18_minor_seventh_flat_five = 18;
    row_19_minor_add_nine = 19;
    row_20_minor_seventh_nine = 20;
    #[ignore = "M1 #2"] row_21_minor_seventh_eleven = 21;
    #[ignore = "M1 #2"] row_22_minor_major_seventh_flat_five = 22;
    row_23_minor_major_seventh = 23;
    row_24_minor_major_seventh_nine = 24;
    row_25_diminished = 25;
    row_26_diminished_seventh = 26;
    row_27_seventh = 27;
    row_28_seventh_sus_four = 28;
    row_29_seventh_nine = 29;
    #[ignore = "M1 #2"] row_30_seventh_sharp_eleven = 30;
    row_31_seventh_thirteen = 31;
    row_32_seventh_flat_five = 32;
    row_33_seventh_flat_nine = 33;
    row_34_seventh_flat_thirteen = 34;
    row_35_seventh_sharp_nine = 35;
    row_36_sus_four = 36;
    row_37_sus_two = 37;
    row_38_cancel = 38;
}

/// Doubling notes in other octaves does not change the chord: every single-reading voicing
/// of every row, with its lowest note doubled an octave up and its root added above the top,
/// reads the same as the close voicing. (1+8 is its own doubling and Cancel is an exact
/// three-key shape, so both are left out; #2 settles what doubling does to them.)
#[test]
fn octave_doublings_keep_the_chord() {
    let r = Recognizer::new();
    let mut fails = Vec::new();
    for row in TABLE.iter().filter(|r| r.ct.is_some_and(|ct| ct != CANCEL && ct != 30)) {
        for c in cases(row).into_iter().filter(|c| !ambiguous(c)) {
            let mut doubled = c.notes.clone();
            doubled.push(c.notes[0] + 12);
            let top = c.notes[c.notes.len() - 1];
            let up = (c.root + 12 - top % 12) % 12;
            doubled.push(top + if up == 0 { 12 } else { up });
            let (close, got) = (recognize(&r, &c.notes), recognize(&r, &doubled));
            if close != got {
                let name = |g: Option<Chord>| g.map_or("nothing".to_string(), |g| g.name());
                fails.push(format!("  {} doubled {doubled:?}: close {}, doubled {}", c.describe(), name(close), name(got)));
            }
        }
    }
    assert!(fails.is_empty(), "{} doubled voicings changed\n{}", fails.len(), fails.iter().take(40).cloned().collect::<Vec<_>>().join("\n"));
}

/// Note sets that are not in the Data List table give no chord (the previous chord keeps
/// playing). The table is the complete list for Fingered, and only AI Fingered accepts
/// "less than three notes" (RM p.9), so in Fingered / Fingered On Bass a single key, a
/// two-key interval other than 1+5 and 1+8, a cluster, or Cancel with an extra key is no
/// chord.
#[test]
#[ignore = "M1 #2"]
fn off_table_inputs() {
    let r = Recognizer::new();
    let mut inputs: Vec<(String, Vec<u8>)> = Vec::new();
    for root in 0..12u8 {
        let k = 48 + root;
        inputs.push(("single key".into(), vec![k]));
        // 5 is 1+5 with the fifth below, 7 is 1+5 and 12 is 1+8.
        for iv in [1, 2, 3, 4, 6, 8, 9, 10, 11] {
            inputs.push((format!("two keys {iv} apart"), vec![k, k + iv]));
        }
        inputs.push(("cluster".into(), vec![k, k + 1, k + 2, k + 3]));
        inputs.push(("whole-tone cluster".into(), vec![k, k + 2, k + 4]));
        inputs.push(("Cancel + 3rd".into(), vec![k, k + 1, k + 2, k + 4]));
        inputs.push(("Cancel + 5th".into(), vec![k, k + 1, k + 2, k + 7]));
    }
    let mut fails = Vec::new();
    for (what, notes) in &inputs {
        if let Some(g) = recognize(&r, notes) {
            let names: Vec<String> =
                notes.iter().map(|&k| format!("{}{}", NOTE_NAMES[k as usize % 12], k as i32 / 12 - 2)).collect();
            fails.push(format!("  {what}: [{}]  want nothing, got {}", names.join(" "), g.name()));
        }
    }
    assert!(fails.is_empty(), "{} of {} off-table inputs gave a chord\n{}", fails.len(), inputs.len(), fails.iter().take(60).cloned().collect::<Vec<_>>().join("\n"));
}

/// Pitch-class sets with more than one reading: the result must be one of them, spelled
/// with the lowest note as on-bass when it is not the root.
#[test]
fn ambiguous_voicings() {
    let r = Recognizer::new();
    let mut fails = Vec::new();
    for row in &TABLE {
        for c in cases(row).into_iter().filter(ambiguous) {
            let low = c.low();
            let valid: Vec<Chord> = intended(&c)
                .into_iter()
                .filter_map(|(rw, root)| Some(Chord { root, ty: rw.ct?, bass: (low != root).then_some(low) }))
                .collect();
            let got = recognize(&r, &c.notes);
            if !got.is_some_and(|g| valid.contains(&g)) {
                let valid: Vec<String> = valid.iter().map(|v| v.name()).collect();
                fails.push(format!(
                    "  {}  want one of [{}], got {}",
                    c.describe(),
                    valid.join(", "),
                    got.map_or("nothing".to_string(), |g| g.name())
                ));
            }
        }
    }
    fails.dedup();
    assert!(fails.is_empty(), "{} ambiguous voicings wrong\n{}", fails.len(), fails.iter().take(60).cloned().collect::<Vec<_>>().join("\n"));
}

/// The harness itself: table sanity, and the expected case counts.
#[test]
fn table_is_consistent() {
    let names = crate::theory::TYPE_NAMES;
    for row in &TABLE {
        assert_eq!(row.tones[0], 0, "row {}", row.n);
        assert!(row.tones.windows(2).all(|w| w[0] < w[1]), "row {} tones not sorted", row.n);
        assert!(row.optional.iter().all(|t| row.tones.contains(t) && *t != 0), "row {} optional", row.n);
        if let Some(ct) = row.ct {
            assert!((ct as usize) < names.len(), "row {} ct {ct}", row.n);
        }
    }
    let mut cts: Vec<u8> = TABLE.iter().filter_map(|r| r.ct).collect();
    cts.sort();
    assert_eq!(cts, (0..=34).collect::<Vec<u8>>(), "every MIDI chord type appears exactly once");
    // M7: 12 roots x (4 inversions with the 5th + 3 without).
    assert_eq!(cases(&TABLE[4]).len(), 12 * 7);
    let c = &cases(&TABLE[4])[1];
    assert_eq!(c.describe(), "row  5 M7 on C: [E2 G2 B2 C3]");
}

//! Corpus self-consistency oracle (#8): the style authors' own note conversions, used as
//! ground truth for ours.
//!
//! Many styles keep several source channels for one part (one written for major chords,
//! another for minor ones) and let the CASM chord mute pick which one plays. Where the author
//! wrote a second source as an edited copy of the first, the second one is what they wanted
//! the first to become on that chord. So we play the first source through our transposer
//! (the public `theory::transpose_group`) on the chord the second one was written for, and
//! count how many of its notes land on the notes the author wrote. We do that with the
//! channel's own settings and with every NTT table in turn, and sum the counts per table,
//! per chord change, per NTR and per style. docs/oracle.md explains the method and its limits.
//!
//! The report holds only counts. No note of any style leaves this module.

use crate::library::Library;
use crate::sff::{ChannelRule, Ev, Ntr, Ntt, SectionId, Style};
use crate::theory::{self, Chord, NUM_TYPES, TYPE_NAMES};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

/// Tables tried on Root Trans and Root Fixed sources.
pub const TABLES: [Ntt; 11] = [
    Ntt::Bypass,
    Ntt::Melody,
    Ntt::Chord,
    Ntt::MelodicMinor,
    Ntt::MelodicMinor5,
    Ntt::HarmonicMinor,
    Ntt::HarmonicMinor5,
    Ntt::NaturalMinor,
    Ntt::NaturalMinor5,
    Ntt::Dorian,
    Ntt::Dorian5,
];
/// Tables tried on Guitar sources.
pub const GUITAR_TABLES: [Ntt; 3] = [Ntt::GuitarAllPurpose, Ntt::GuitarStroke, Ntt::GuitarArpeggio];

const NTRS: [Ntr; 3] = [Ntr::RootTrans, Ntr::RootFixed, Ntr::Guitar];

/// A pair counts only when this share of both sources' notes is aligned (same tick, same
/// number of notes) ...
const MIN_ALIGNED: f64 = 0.5;
/// ... at least this many notes are aligned ...
const MIN_NOTES: u32 = 4;
/// ... and this share of the aligned notes is within `NEAR` semitones of the other source's
/// note (after moving to its root): the second source is an edited copy of the first, not a
/// different line.
const MIN_NEAR: f64 = 0.5;
const NEAR: i32 = 2;

/// Notes compared, and how many our conversion got right: exact pitch, or pitch class.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Score {
    pub notes: u32,
    pub exact: u32,
    pub pc: u32,
}

impl Score {
    fn add(&mut self, o: Score) {
        self.notes += o.notes;
        self.exact += o.exact;
        self.pc += o.pc;
    }

    fn sum<'a>(it: impl IntoIterator<Item = &'a Score>) -> Score {
        let mut s = Score::default();
        it.into_iter().for_each(|x| s.add(*x));
        s
    }

    fn pct(n: u32, of: u32) -> f64 {
        if of == 0 { 0.0 } else { 100.0 * n as f64 / of as f64 }
    }

    fn cells(&self) -> String {
        format!("{:>7} {:>6.1}% {:>6.1}%", self.notes, Score::pct(self.exact, self.notes), Score::pct(self.pc, self.notes))
    }

    fn pinned(&self) -> String {
        format!("{} {} {}", self.notes, self.exact, self.pc)
    }
}

/// One source played on the chord another source of the same part was written for.
#[derive(Debug, Clone)]
pub struct Pair {
    pub section: SectionId,
    pub dest_ch: u8,
    pub from_ch: u8,
    pub to_ch: u8,
    /// The played chord: the other source's source chord.
    pub chord: Chord,
    pub from_type: u8,
    /// The converted source's NTR and NTT (its middle zone).
    pub ntr: Ntr,
    pub ntt: Ntt,
    /// With the source's own CASM settings.
    pub authored: Score,
    /// With every zone's table replaced, in `TABLES` or `GUITAR_TABLES` order.
    pub tables: Vec<(Ntt, Score)>,
}

impl Pair {
    /// "M>m": source chord type to played chord type.
    pub fn transition(&self) -> String {
        format!("{}>{}", type_name(self.from_type), type_name(self.chord.ty))
    }
}

#[derive(Debug, Default, Clone)]
pub struct StyleResult {
    pub file: String,
    pub pairs: Vec<Pair>,
    /// Chord-muted alternatives that are not edited copies of each other.
    pub unrelated: u32,
    /// Alternatives with the same source chord: nothing to convert.
    pub same_chord: u32,
    /// Alternatives muted on their own source chord, so their notes never sound as written
    /// and are no reference.
    pub muted_own: u32,
    /// Every chord-following source played on its own source chord, per NTR (`NTRS` order).
    /// The spec says that reproduces the pattern unchanged (RM p.28); Note Limit folding
    /// and Guitar voicing are what can still move a note.
    pub identity: [Score; 3],
}

impl StyleResult {
    fn authored(&self) -> Score {
        Score::sum(self.pairs.iter().map(|p| &p.authored))
    }
}

#[derive(Debug, Default)]
pub struct Report {
    pub styles: Vec<StyleResult>,
    pub errors: Vec<(String, String)>,
}

fn type_name(ty: u8) -> &'static str {
    match ty {
        0 => "M",
        t => TYPE_NAMES.get(t as usize).copied().unwrap_or("?"),
    }
}

fn ntr_index(n: Ntr) -> usize {
    NTRS.iter().position(|&x| x == n).unwrap_or(0)
}

/// Note-ons of one source channel, grouped by tick, keys sorted.
fn onsets(style: &Style, id: SectionId, ch: u8) -> BTreeMap<u32, Vec<u8>> {
    let mut out: BTreeMap<u32, Vec<u8>> = BTreeMap::new();
    if let Some(sec) = style.sections.get(&id) {
        for e in &sec.events {
            if let Ev::NoteOn { ch: c, key, vel } = e.ev
                && c == ch
                && vel > 0
            {
                out.entry(e.tick).or_default().push(key);
            }
        }
    }
    for v in out.values_mut() {
        v.sort_unstable();
    }
    out
}

/// Notes of `a` and `b` that match, counted as multisets.
fn common(a: &[u8], b: &[u8], key: impl Fn(u8) -> u8) -> u32 {
    let mut rest: Vec<u8> = b.iter().map(|&k| key(k)).collect();
    let mut n = 0;
    for &k in a {
        if let Some(i) = rest.iter().position(|&r| r == key(k)) {
            rest.swap_remove(i);
            n += 1;
        }
    }
    n
}

/// Convert the first half of every aligned tick for `chord` with `rule` and count the notes
/// that land on the second half.
fn score(aligned: &[(&[u8], &[u8])], rule: &ChannelRule, chord: Chord) -> Score {
    let mut s = Score::default();
    let mut out = [None; 16];
    for &(a, b) in aligned {
        let n = a.len().min(out.len());
        theory::transpose_group(&a[..n], rule, chord, &mut out[..n]);
        let got: Vec<u8> = out[..n].iter().flatten().copied().collect();
        s.notes += b.len() as u32;
        s.exact += common(&got, b, |k| k);
        s.pc += common(&got, b, |k| k % 12);
    }
    s
}

/// The rule with every zone of the table's family (Guitar or not) switched to `ntt`.
fn with_table(rule: &ChannelRule, ntt: Ntt) -> ChannelRule {
    let guitar = GUITAR_TABLES.contains(&ntt);
    let mut r = rule.clone();
    for z in r.zones.iter_mut() {
        if (z.ntr == Ntr::Guitar) == guitar {
            z.ntt = ntt;
        }
    }
    r
}

/// The chord a source was written for.
fn source_chord(rule: &ChannelRule) -> Chord {
    Chord::new(rule.src_root % 12, rule.src_type)
}

/// Does the source sound, unconverted, on its own source chord?
fn sounds_as_written(rule: &ChannelRule) -> bool {
    let c = source_chord(rule);
    (c.ty as usize) < NUM_TYPES && theory::plays(rule, c)
}

/// Every chord-muted alternative in one style, plus the identity check.
pub fn analyse(style: &Style, file: &str) -> StyleResult {
    let mut res = StyleResult { file: file.to_string(), ..Default::default() };
    for &id in style.sections.keys() {
        let rules = style.rules_for(id);
        let mut by_dest: BTreeMap<u8, Vec<&ChannelRule>> = BTreeMap::new();
        for r in rules.values().filter(|r| !theory::is_drum_part(r.dest_ch)) {
            by_dest.entry(r.dest_ch).or_default().push(r);
        }
        for group in by_dest.values() {
            let notes: Vec<BTreeMap<u32, Vec<u8>>> = group.iter().map(|r| onsets(style, id, r.src_ch)).collect();
            for (j, to) in group.iter().enumerate() {
                let chord = source_chord(to);
                if sounds_as_written(to) {
                    let own: Vec<(&[u8], &[u8])> = notes[j].values().map(|v| (v.as_slice(), v.as_slice())).collect();
                    res.identity[ntr_index(to.zones[1].ntr)].add(score(&own, to, chord));
                }
                for (i, from) in group.iter().enumerate() {
                    if i == j || (chord.ty as usize) < NUM_TYPES && theory::plays(from, chord) {
                        continue; // not an alternative: both sound on this chord
                    }
                    if !sounds_as_written(to) {
                        res.muted_own += 1;
                    } else if source_chord(from) == chord {
                        res.same_chord += 1;
                    } else if let Some(p) = pair(id, from, to, chord, &notes[i], &notes[j]) {
                        res.pairs.push(p);
                    } else {
                        res.unrelated += 1;
                    }
                }
            }
        }
    }
    res
}

fn pair(
    section: SectionId,
    from: &ChannelRule,
    to: &ChannelRule,
    chord: Chord,
    a: &BTreeMap<u32, Vec<u8>>,
    b: &BTreeMap<u32, Vec<u8>>,
) -> Option<Pair> {
    let aligned: Vec<(&[u8], &[u8])> = a
        .iter()
        .filter_map(|(t, ka)| b.get(t).filter(|kb| kb.len() == ka.len()).map(|kb| (ka.as_slice(), kb.as_slice())))
        .collect();
    let n: u32 = aligned.iter().map(|(x, _)| x.len() as u32).sum();
    let total = |m: &BTreeMap<u32, Vec<u8>>| m.values().map(Vec::len).sum::<usize>() as f64;
    if n < MIN_NOTES || (n as f64) < MIN_ALIGNED * total(a) || (n as f64) < MIN_ALIGNED * total(b) {
        return None;
    }
    let mut shift = (chord.root as i32 - (from.src_root % 12) as i32).rem_euclid(12);
    if shift > 6 {
        shift -= 12;
    }
    let near = aligned
        .iter()
        .flat_map(|(x, y)| x.iter().zip(y.iter()))
        .filter(|&(&p, &q)| (p as i32 + shift - q as i32).abs() <= NEAR)
        .count();
    if (near as f64) < MIN_NEAR * n as f64 {
        return None;
    }
    let z = &from.zones[1];
    let tables: &[Ntt] = if z.ntr == Ntr::Guitar { &GUITAR_TABLES } else { &TABLES };
    Some(Pair {
        section,
        dest_ch: from.dest_ch,
        from_ch: from.src_ch,
        to_ch: to.src_ch,
        chord,
        from_type: from.src_type,
        ntr: z.ntr,
        ntt: z.ntt,
        authored: score(&aligned, from, chord),
        tables: tables.iter().map(|&t| (t, score(&aligned, &with_table(from, t), chord))).collect(),
    })
}

/// Analyse every style under `paths` (folders searched recursively).
pub fn run(paths: &[PathBuf]) -> Report {
    let lib = Library::scan(paths);
    let mut rep = Report::default();
    for &id in lib.order() {
        let e = lib.entry(id);
        let file = e.path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
        match Style::load(&e.path) {
            Ok(style) => rep.styles.push(analyse(&style, &file)),
            Err(err) => rep.errors.push((file, format!("{err:#}"))),
        }
    }
    rep
}

/// Aggregates shared by the readable report and the pinned scores.
struct Totals {
    pairs: u32,
    unrelated: u32,
    same_chord: u32,
    muted_own: u32,
    authored: Score,
    identity: [Score; 3],
    /// Per table, over all pairs of that table's family.
    tables: Vec<(Ntt, Score)>,
    /// Per NTR (as authored): pairs and score.
    ntr: [(u32, Score); 3],
    /// Per chord change ("M>m").
    changes: BTreeMap<String, Change>,
}

/// One chord change: pairs, authored score, and every table's score.
type Change = (u32, Score, Vec<(Ntt, Score)>);

impl Report {
    fn totals(&self) -> Totals {
        let mut t = Totals {
            pairs: 0,
            unrelated: 0,
            same_chord: 0,
            muted_own: 0,
            authored: Score::default(),
            identity: [Score::default(); 3],
            tables: TABLES.iter().chain(GUITAR_TABLES.iter()).map(|&n| (n, Score::default())).collect(),
            ntr: [(0, Score::default()); 3],
            changes: BTreeMap::new(),
        };
        let add_tables = |into: &mut Vec<(Ntt, Score)>, from: &[(Ntt, Score)]| {
            for &(n, s) in from {
                match into.iter_mut().find(|(m, _)| *m == n) {
                    Some(e) => e.1.add(s),
                    None => into.push((n, s)),
                }
            }
        };
        for s in &self.styles {
            t.pairs += s.pairs.len() as u32;
            t.unrelated += s.unrelated;
            t.same_chord += s.same_chord;
            t.muted_own += s.muted_own;
            for (i, x) in s.identity.iter().enumerate() {
                t.identity[i].add(*x);
            }
            for p in &s.pairs {
                t.authored.add(p.authored);
                add_tables(&mut t.tables, &p.tables);
                let n = &mut t.ntr[ntr_index(p.ntr)];
                n.0 += 1;
                n.1.add(p.authored);
                let c = t.changes.entry(p.transition()).or_default();
                c.0 += 1;
                c.1.add(p.authored);
                add_tables(&mut c.2, &p.tables);
            }
        }
        t
    }

    /// The readable report `yahaha oracle` prints. Counts and percentages only.
    pub fn text(&self, list_pairs: bool) -> String {
        let mut o = String::new();
        let t = self.totals();
        let with = self.styles.iter().filter(|s| !s.pairs.is_empty()).count();
        let _ = writeln!(o, "styles {}  (with scored pairs {with}, unreadable {})", self.styles.len(), self.errors.len());
        let _ = writeln!(
            o,
            "pairs  {} scored; not scored: {} not edited copies, {} same source chord, {} muted on their own chord",
            t.pairs, t.unrelated, t.same_chord, t.muted_own
        );
        let _ = writeln!(o, "\n{:<26} {:>7} {:>7} {:>7}", "", "notes", "exact", "pitch");
        let _ = writeln!(o, "{:<26} {}", "as authored", t.authored.cells());
        let _ = writeln!(o, "{:<26} {}", "own chord (identity)", Score::sum(&t.identity).cells());
        let _ = writeln!(o, "\nNTT table, on every pair");
        for (n, s) in &t.tables {
            let _ = writeln!(o, "  {:<24} {}", format!("{n:?}"), s.cells());
        }
        let _ = writeln!(o, "\nNTR (as authored)   pairs");
        for (i, ntr) in NTRS.iter().enumerate() {
            let (c, s) = t.ntr[i];
            let _ = writeln!(o, "  {:<16} {c:>6}  {}", format!("{ntr:?}"), s.cells());
        }
        let _ = writeln!(o, "\nNTR, own chord (identity)");
        for (i, ntr) in NTRS.iter().enumerate() {
            let _ = writeln!(o, "  {:<24} {}", format!("{ntr:?}"), t.identity[i].cells());
        }
        // Which table the authors' own edits agree with, per chord change. Ties go to the
        // table listed first.
        let _ = writeln!(o, "\nchord change      pairs    notes  authored  best table");
        for (k, (c, a, tables)) in &t.changes {
            let mut best: Option<&(Ntt, Score)> = None;
            for e in tables {
                if best.is_none_or(|b| e.1.exact > b.1.exact) {
                    best = Some(e);
                }
            }
            let best = best.map(|(n, s)| format!("{n:?} {:.1}%", Score::pct(s.exact, s.notes))).unwrap_or_default();
            let _ = writeln!(o, "  {k:<14} {c:>6} {:>8} {:>8.1}%  {best}", a.notes, Score::pct(a.exact, a.notes));
        }
        let _ = writeln!(o, "\nstyle                                  pairs    notes  authored  identity");
        for s in self.styles.iter().filter(|s| !s.pairs.is_empty()) {
            let (a, id) = (s.authored(), Score::sum(&s.identity));
            let _ = writeln!(
                o,
                "  {:<36} {:>5} {:>8} {:>8.1}%  {:>7.1}%",
                s.file,
                s.pairs.len(),
                a.notes,
                Score::pct(a.exact, a.notes),
                Score::pct(id.exact, id.notes)
            );
            if list_pairs {
                for p in &s.pairs {
                    let _ = writeln!(
                        o,
                        "      {:<11} ch{:<2} src{:>2} on {:<8} (src{:>2}'s chord, {:?}/{:?})  {}",
                        p.section.name(),
                        p.dest_ch + 1,
                        p.from_ch + 1,
                        p.chord.name(),
                        p.to_ch + 1,
                        p.ntr,
                        p.ntt,
                        p.authored.cells()
                    );
                }
            }
        }
        for (f, e) in &self.errors {
            let _ = writeln!(o, "unreadable {f}: {e}");
        }
        o
    }

    /// The numbers the test pins (tests/oracle/scores.txt). Each line is a key and counts;
    /// score lines end in `notes exact pitch-class`.
    pub fn pinned(&self) -> String {
        let mut o = String::new();
        let t = self.totals();
        let _ = writeln!(o, "# yahaha oracle (docs/oracle.md). Counts only; score lines end in: notes exact pitch-class.");
        let _ = writeln!(o, "# Regenerate with UPDATE_GOLDEN=1 cargo test --release oracle.");
        let _ = writeln!(o, "styles {} {}", self.styles.len(), self.errors.len());
        let _ = writeln!(o, "pairs {} {} {} {}", t.pairs, t.unrelated, t.same_chord, t.muted_own);
        let _ = writeln!(o, "authored {}", t.authored.pinned());
        for (i, ntr) in NTRS.iter().enumerate() {
            let _ = writeln!(o, "identity {ntr:?} {}", t.identity[i].pinned());
        }
        for (n, s) in &t.tables {
            let _ = writeln!(o, "table {n:?} {}", s.pinned());
        }
        for (i, ntr) in NTRS.iter().enumerate() {
            let _ = writeln!(o, "ntr {ntr:?} {} {}", t.ntr[i].0, t.ntr[i].1.pinned());
        }
        for (k, (c, a, tables)) in &t.changes {
            let _ = writeln!(o, "change {k} authored {c} {}", a.pinned());
            for (n, s) in tables {
                let _ = writeln!(o, "change {k} {n:?} {}", s.pinned());
            }
        }
        for s in self.styles.iter().filter(|s| !s.pairs.is_empty()) {
            let _ = writeln!(
                o,
                "style {} {} {} {}",
                s.file.replace(char::is_whitespace, "_"),
                s.pairs.len(),
                s.authored().pinned(),
                Score::sum(&s.identity).pinned()
            );
        }
        o
    }
}

/// Changed lines between two pinned score files, with the exact-hit rate change.
pub fn delta(want: &str, got: &str) -> String {
    let parse = |t: &str| -> BTreeMap<String, Vec<i64>> {
        t.lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
            .map(|l| {
                let w: Vec<&str> = l.split_whitespace().collect();
                let n = w.iter().rposition(|x| x.parse::<i64>().is_err()).map_or(0, |i| i + 1);
                (w[..n].join(" "), w[n..].iter().filter_map(|x| x.parse().ok()).collect())
            })
            .collect()
    };
    let (a, b) = (parse(want), parse(got));
    let rate = |v: &[i64]| {
        let v = &v[v.len() - 3..];
        if v[0] > 0 { 100.0 * v[1] as f64 / v[0] as f64 } else { 0.0 }
    };
    let mut o = String::new();
    for k in a.keys().chain(b.keys().filter(|k| !a.contains_key(*k))) {
        let _ = match (a.get(k), b.get(k)) {
            (Some(x), Some(y)) if x == y => Ok(()),
            (Some(x), Some(y)) if x.len() >= 3 && y.len() >= 3 => {
                let (rx, ry) = (rate(x), rate(y));
                writeln!(o, "  {k}: {x:?} -> {y:?}  exact {rx:.1}% -> {ry:.1}% ({:+.1})", ry - rx)
            }
            (Some(x), Some(y)) => writeln!(o, "  {k}: {x:?} -> {y:?}"),
            (Some(x), None) => writeln!(o, "  {k}: {x:?} gone"),
            (None, Some(y)) => writeln!(o, "  {k}: new {y:?}"),
            (None, None) => Ok(()),
        };
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sff::{Cseg, Section, TimedEv};
    use std::path::Path;

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
    }

    /// A Main A whose Chord1 has a major source (ch 12, C E G) and a minor one (ch 3),
    /// routed by chord mute: the minor one plays on m, m6 ... mM7(9), the major one on the rest.
    fn toy(minor: [u8; 3]) -> Style {
        let minor_types: u64 = 0x7ff << 8;
        let mut maj = ChannelRule::default_for(11);
        maj.src_type = 0;
        maj.chord_mute = ((1u64 << 34) - 1) & !minor_types;
        let mut min = ChannelRule::default_for(11);
        min.src_ch = 2;
        min.src_type = 8;
        min.chord_mute = minor_types;
        let mut events = Vec::new();
        for bar in 0..2u32 {
            for (ch, keys) in [(11u8, [60u8, 64, 67]), (2, minor)] {
                for k in keys {
                    events.push(TimedEv { tick: bar * 1920, ev: Ev::NoteOn { ch, key: k, vel: 100 } });
                }
            }
        }
        let id = SectionId::Main(0);
        Style {
            name: "toy".into(),
            format: "SFF2".into(),
            ppq: 480,
            tempo_us: 500_000,
            timesig: (4, 4),
            init: vec![],
            sections: [(id, Section { id, start: 0, len: 3840, events })].into_iter().collect(),
            casm: vec![Cseg { sections: vec!["Main A".into()], rules: vec![maj, min] }],
            ots: vec![],
            other_chunks: vec![],
        }
    }

    fn table(p: &Pair, t: Ntt) -> Score {
        p.tables.iter().find(|(u, _)| *u == t).unwrap().1
    }

    #[test]
    fn scores_an_edited_copy_both_ways() {
        let r = analyse(&toy([60, 63, 67]), "toy");
        assert_eq!((r.pairs.len(), r.unrelated, r.same_chord, r.muted_own), (2, 0, 0, 0));
        let to_minor = r.pairs.iter().find(|p| p.chord.ty == 8).unwrap();
        assert_eq!(to_minor.transition(), "M>m");
        assert_eq!((to_minor.from_ch, to_minor.to_ch), (11, 2));
        // Bypass keeps the major 3rd: two of the three notes, twice.
        assert_eq!(table(to_minor, Ntt::Bypass), Score { notes: 6, exact: 4, pc: 4 });
        assert_eq!(table(to_minor, Ntt::MelodicMinor), Score { notes: 6, exact: 6, pc: 6 });
        assert_eq!(to_minor.authored, table(to_minor, Ntt::Chord), "ch12 is Root Fixed / Chord by default");
        let to_major = r.pairs.iter().find(|p| p.chord.ty == 0).unwrap();
        assert_eq!(to_major.transition(), "m>M");
        assert_eq!(table(to_major, Ntt::MelodicMinor), Score { notes: 6, exact: 6, pc: 6 });
        // Both sources reproduce themselves on their own source chord.
        assert_eq!(Score::sum(&r.identity), Score { notes: 12, exact: 12, pc: 12 });

        let rep = Report { styles: vec![r], errors: vec![] };
        let pinned = rep.pinned();
        assert!(pinned.contains("\npairs 2 0 0 0\n") && pinned.contains("\nchange M>m MelodicMinor 6 6 6\n"), "{pinned}");
        assert!(rep.text(true).contains("M>m"));
    }

    #[test]
    fn a_different_line_is_not_scored() {
        let r = analyse(&toy([48, 75, 82]), "toy");
        assert_eq!((r.pairs.len(), r.unrelated), (0, 2));
    }

    #[test]
    fn same_source_chord_and_muted_reference_are_not_scored() {
        // The minor source claims to be written for C major: nothing to convert into it.
        let mut s = toy([60, 63, 67]);
        s.casm[0].rules[1].src_type = 0;
        let r = analyse(&s, "toy");
        assert_eq!((r.pairs.len(), r.muted_own, r.same_chord), (0, 0, 1));
        // The minor source is muted on Cm itself: its notes never sound as written, so only
        // the other direction is scored.
        let mut s = toy([60, 63, 67]);
        s.casm[0].rules[1].chord_mute &= !(1 << 8);
        let r = analyse(&s, "toy");
        assert_eq!((r.pairs.len(), r.muted_own, r.same_chord), (1, 1, 0));
        assert_eq!(r.pairs[0].transition(), "m>M");
    }

    #[test]
    fn delta_shows_rate_change() {
        let d = delta("pairs 3 1 0 0\ntable Melody 10 5 6\n", "pairs 3 1 0 0\ntable Melody 10 7 8\nstyle X.sty 1 2 2 2 4 4 4\n");
        assert_eq!(
            d,
            "  table Melody: [10, 5, 6] -> [10, 7, 8]  exact 50.0% -> 70.0% (+20.0)\n  style X.sty: new [1, 2, 2, 2, 4, 4, 4]\n"
        );
        assert_eq!(delta("a 1\n", "# comment\na 1\n"), "");
    }

    /// Pins the corpus scores, so every change to the transposer reports what it did to them.
    /// Numbers only: nothing in tests/oracle/scores.txt is note content.
    #[test]
    fn corpus_scores() {
        let corpus = root().join("corpus");
        if !corpus.is_dir() {
            eprintln!("oracle: no corpus; skipping");
            return;
        }
        let rep = run(&[corpus]);
        assert!(!rep.styles.is_empty(), "corpus/ holds no styles");
        let got = rep.pinned();
        let file = root().join("tests/oracle/scores.txt");
        if std::env::var_os("UPDATE_GOLDEN").is_some() {
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(&file, &got).unwrap();
            eprintln!("oracle: wrote {}", file.display());
            return;
        }
        let want = std::fs::read_to_string(&file).unwrap_or_default();
        let d = delta(&want, &got);
        assert!(
            d.is_empty(),
            "oracle scores changed (want -> got):\n{d}\nIf the change is intended, regenerate with UPDATE_GOLDEN=1 cargo test --release oracle, commit tests/oracle/scores.txt and quote this delta in the PR."
        );
    }

    /// The committed scores are counts: every line is a key followed by integers.
    #[test]
    fn committed_scores_hold_only_numbers() {
        let text = std::fs::read_to_string(root().join("tests/oracle/scores.txt")).expect("tests/oracle/scores.txt");
        for line in text.lines().filter(|l| !l.starts_with('#')) {
            let w: Vec<&str> = line.split_whitespace().collect();
            let key = match w.first() {
                Some(&"change") => 3,
                Some(&"style") | Some(&"table") | Some(&"ntr") | Some(&"identity") => 2,
                _ => 1,
            };
            assert!(w.len() > key && w[key..].iter().all(|x| x.parse::<u32>().is_ok()), "not a score line: {line}");
        }
    }
}

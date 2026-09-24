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
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

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

/// Every chord-following source played on its own source chord, for one NTR.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Identity {
    /// Per note: does it come out as written?
    pub score: Score,
    /// Notes that came out neither as written nor as Note Limit's fold of what was written
    /// (`fold_target`). RM p.28 says the source chord plays the recorded data back, and RM
    /// p.30 that only notes outside the limit are moved (by octaves, into it), so each of
    /// these is a transposer miss.
    pub moved_in_limit: u32,
}

impl Identity {
    fn add(&mut self, o: Identity) {
        self.score.add(o.score);
        self.moved_in_limit += o.moved_in_limit;
    }
}

/// The Guitar check, per Guitar table (`GUITAR_TABLES` order) of each note's own zone.
///
/// A Guitar source is string-coded: a key names a string of a voicing, not a pitch, and
/// Source Root/Chord are ignored (RM p.29), so no chord plays it back as written and the
/// identity check does not apply. Instead every Guitar note is played on every chord its
/// channel plays (CASM chord and root mute, all 12 roots), and the output must be what a
/// guitar can sound for that chord: a chord tone, on the neck (not below the open low E)
/// and inside the Note Limit. Noise keys (MegaVoice, from `theory::GUITAR_NOISE` up) must
/// come back untouched on every chord.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct GuitarCheck {
    /// Guitar notes (note-ons), noise keys included.
    pub notes: u32,
    /// Of those, noise keys, and noise keys that came back untouched on every chord.
    pub noise: u32,
    pub noise_kept: u32,
    /// Note x chord plays of the other notes, and how many of them sounded (Stroke mutes
    /// strings), were a chord tone, and were in range.
    pub played: u32,
    pub sounded: u32,
    pub chord_tone: u32,
    pub in_range: u32,
}

impl GuitarCheck {
    fn add(&mut self, o: GuitarCheck) {
        self.notes += o.notes;
        self.noise += o.noise;
        self.noise_kept += o.noise_kept;
        self.played += o.played;
        self.sounded += o.sounded;
        self.chord_tone += o.chord_tone;
        self.in_range += o.in_range;
    }

    fn total(c: &[GuitarCheck; 3]) -> GuitarCheck {
        let mut t = GuitarCheck::default();
        c.iter().for_each(|x| t.add(*x));
        t
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
    /// The style's path below the folder it was found in, whitespace as `_`.
    pub file: String,
    pub pairs: Vec<Pair>,
    /// Chord-muted alternatives that are not edited copies of each other.
    pub unrelated: u32,
    /// Alternatives where one source starts fewer than `MIN_NOTES` notes in the section
    /// (often none): too little to tell a copy from a different line.
    pub too_few: u32,
    /// Alternatives with the same source chord: nothing to convert.
    pub same_chord: u32,
    /// Alternatives muted on their own source chord, so their notes never sound as written
    /// and are no reference.
    pub muted_own: u32,
    /// Every chord-following source played on its own source chord, per NTR (`NTRS` order)
    /// of each note's own zone. The spec says that reproduces the pattern unchanged (RM
    /// p.28); only Note Limit folding a note written outside the limit may still move it.
    pub identity: [Identity; 3],
    /// Guitar notes are not in `identity` (string codes have no as-written pitch); they
    /// get the Guitar check instead, per table.
    pub guitar: [GuitarCheck; 3],
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

impl Report {
    /// Keep only the styles (and unreadable files) `keep` accepts, by `StyleResult::file`.
    pub fn retain(&mut self, keep: impl Fn(&str) -> bool) {
        self.styles.retain(|s| keep(&s.file));
        self.errors.retain(|(f, _)| keep(f));
    }
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

/// The note-ons one source channel starts on one tick.
#[derive(Debug, Default)]
struct Onset {
    /// As the engine hands them to `transpose_group` (`engine::Prepared` and
    /// `emit_at_index`): consecutive note-ons of this channel, in event order, at most 8.
    groups: Vec<Vec<u8>>,
    /// All of them, sorted.
    keys: Vec<u8>,
}

/// Note-ons of one source channel by tick. The engine moves note-offs to the front of their
/// tick and drops aftertouch, sysex and meta events; any other event between two note-ons
/// of the channel splits the group, as it does there.
fn onsets(style: &Style, id: SectionId, ch: u8) -> BTreeMap<u32, Onset> {
    let mut out: BTreeMap<u32, Onset> = BTreeMap::new();
    let Some(sec) = style.sections.get(&id) else { return out };
    let mut open = false; // the previous kept event was a note-on of `ch` at this tick
    let mut tick = None;
    for e in &sec.events {
        if tick != Some(e.tick) {
            tick = Some(e.tick);
            open = false;
        }
        match e.ev {
            Ev::NoteOn { ch: c, key, .. } if c == ch => {
                let o = out.entry(e.tick).or_default();
                match o.groups.last_mut() {
                    Some(g) if open && g.len() < 8 => g.push(key),
                    _ => o.groups.push(vec![key]),
                }
                o.keys.push(key);
                open = true;
            }
            Ev::NoteOn { .. } | Ev::Cc { .. } | Ev::Pc { .. } | Ev::Bend { .. } => open = false,
            _ => {}
        }
    }
    for o in out.values_mut() {
        o.keys.sort_unstable();
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

/// Convert the first onset of every aligned tick for `chord` with `rule`, group by group,
/// and count the notes that land on the second onset's notes.
fn score(aligned: &[(&Onset, &Onset)], rule: &ChannelRule, chord: Chord) -> Score {
    let mut s = Score::default();
    let mut out = [None; 8];
    for &(a, b) in aligned {
        let mut got = Vec::with_capacity(a.keys.len());
        for g in &a.groups {
            theory::transpose_group(g, rule, chord, &mut out[..g.len()]);
            got.extend(out[..g.len()].iter().flatten());
        }
        s.notes += b.keys.len() as u32;
        s.exact += common(&got, &b.keys, |k| k);
        s.pc += common(&got, &b.keys, |k| k % 12);
    }
    s
}

/// Play a source on its own source chord and check every note against itself, per NTR of
/// the note's own zone.
fn identity(notes: &BTreeMap<u32, Onset>, rule: &ChannelRule, into: &mut [Identity; 3]) {
    identity_with(notes, rule, into, theory::transpose_group);
}

/// `identity` with the transposer passed in, so the tests can check the counting against a
/// transposer that misbehaves in known ways.
fn identity_with(
    notes: &BTreeMap<u32, Onset>,
    rule: &ChannelRule,
    into: &mut [Identity; 3],
    transpose: impl Fn(&[u8], &ChannelRule, Chord, &mut [Option<u8>]),
) {
    let chord = source_chord(rule);
    let mut out = [None; 8];
    for g in notes.values().flat_map(|o| &o.groups) {
        transpose(g, rule, chord, &mut out[..g.len()]);
        for (&k, &o) in g.iter().zip(&out[..g.len()]) {
            let z = rule.zone_for(k);
            if z.ntr == Ntr::Guitar {
                continue; // string-coded: see `GuitarCheck`
            }
            let id = &mut into[ntr_index(z.ntr)];
            id.score.notes += 1;
            id.score.exact += (o == Some(k)) as u32;
            id.score.pc += o.is_some_and(|o| o % 12 == k % 12) as u32;
            id.moved_in_limit += (o != Some(k) && o != Some(fold_target(k, z.lo, z.hi))) as u32;
        }
    }
}

/// The Guitar check (`GuitarCheck`) for one source channel's notes in one section.
fn guitar_check(notes: &BTreeMap<u32, Onset>, rule: &ChannelRule, into: &mut [GuitarCheck; 3]) {
    let mut count = [0u32; 128];
    for o in notes.values() {
        for &k in &o.keys {
            if rule.zone_for(k).ntr == Ntr::Guitar {
                count[k as usize] += 1;
            }
        }
    }
    let chords: Vec<Chord> = (0..12u8)
        .flat_map(|root| (0..NUM_TYPES as u8).map(move |ty| Chord::new(root, ty)))
        .filter(|&c| theory::plays(rule, c))
        .collect();
    let mut out = [None];
    for k in (0..128u8).filter(|&k| count[k as usize] > 0) {
        let n = count[k as usize];
        let z = rule.zone_for(k);
        let t = &mut into[GUITAR_TABLES.iter().position(|&g| g == z.ntt).unwrap_or(0)];
        t.notes += n;
        if k >= theory::GUITAR_NOISE {
            t.noise += n;
            let kept = chords.iter().all(|&c| {
                theory::transpose_group(&[k], rule, c, &mut out);
                out[0] == Some(k)
            });
            t.noise_kept += n * kept as u32;
            continue;
        }
        for &c in &chords {
            theory::transpose_group(&[k], rule, c, &mut out);
            t.played += n;
            let Some(o) = out[0] else { continue };
            t.sounded += n;
            let iv = (o as i32 - c.root as i32).rem_euclid(12) as u8;
            t.chord_tone += n * theory::chord_tones(c.ty).contains(&iv) as u32;
            // A Note Limit narrower than an octave may have to leave the neck.
            let (lo, hi) = (z.lo.min(z.hi), z.lo.max(z.hi));
            let neck = o >= LOW_E || hi < LOW_E + 11;
            t.in_range += n * (neck && (lo..=hi).contains(&o)) as u32;
        }
    }
}

/// The open low E string (E1): nothing a guitar plays is lower.
const LOW_E: u8 = 40;

/// Where Note Limit puts a note played unconverted: moved by octaves into `lo..=hi` when it
/// lies outside (RM p.30), to the octave nearest where it was written. A limit narrower than
/// an octave cannot hold every pitch class; for those our rule (#13, docs/genos-features.md)
/// is the octave nearest the limit, the lower one on a tie, and only octaves inside 0..=127
/// count. A reversed limit is read low to high. Written from the rule, not from
/// `theory::fold_into`, so the oracle does not grade the transposer against itself.
fn fold_target(k: u8, lo: u8, hi: u8) -> u8 {
    let (lo, hi) = (lo.min(hi), lo.max(hi));
    if (lo..=hi).contains(&k) {
        return k;
    }
    let outside = |n: u8| lo.saturating_sub(n) + n.saturating_sub(hi);
    (k % 12..=127)
        .step_by(12)
        .min_by_key(|&n| (outside(n), if outside(n) == 0 { n.abs_diff(k) } else { n }))
        .unwrap_or(k)
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
            let notes: Vec<BTreeMap<u32, Onset>> = group.iter().map(|r| onsets(style, id, r.src_ch)).collect();
            for (j, to) in group.iter().enumerate() {
                let chord = source_chord(to);
                if sounds_as_written(to) {
                    identity(&notes[j], to, &mut res.identity);
                }
                if to.zones.iter().any(|z| z.ntr == Ntr::Guitar) {
                    guitar_check(&notes[j], to, &mut res.guitar);
                }
                for (i, from) in group.iter().enumerate() {
                    if i == j || (chord.ty as usize) < NUM_TYPES && theory::plays(from, chord) {
                        continue; // not an alternative: both sound on this chord
                    }
                    if !sounds_as_written(to) {
                        res.muted_own += 1;
                    } else if source_chord(from) == chord {
                        res.same_chord += 1;
                    } else if [i, j].iter().any(|&x| note_count(&notes[x]) < MIN_NOTES) {
                        res.too_few += 1;
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

fn note_count(m: &BTreeMap<u32, Onset>) -> u32 {
    m.values().map(|o| o.keys.len() as u32).sum()
}

fn pair(
    section: SectionId,
    from: &ChannelRule,
    to: &ChannelRule,
    chord: Chord,
    a: &BTreeMap<u32, Onset>,
    b: &BTreeMap<u32, Onset>,
) -> Option<Pair> {
    let aligned: Vec<(&Onset, &Onset)> =
        a.iter().filter_map(|(t, oa)| b.get(t).filter(|ob| ob.keys.len() == oa.keys.len()).map(|ob| (oa, ob))).collect();
    let n: u32 = aligned.iter().map(|(x, _)| x.keys.len() as u32).sum();
    let total = |m: &BTreeMap<u32, Onset>| note_count(m) as f64;
    if n < MIN_NOTES || (n as f64) < MIN_ALIGNED * total(a) || (n as f64) < MIN_ALIGNED * total(b) {
        return None;
    }
    let mut shift = (chord.root as i32 - (from.src_root % 12) as i32).rem_euclid(12);
    if shift > 6 {
        shift -= 12;
    }
    let near = aligned
        .iter()
        .flat_map(|(x, y)| x.keys.iter().zip(y.keys.iter()))
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

/// A style's name in the report: its path below the folder it was found in (just the file
/// name when it was given directly), whitespace as `_` so the pinned file splits on spaces.
fn style_key(path: &Path, roots: &[PathBuf]) -> String {
    let rel = roots
        .iter()
        .find_map(|r| path.strip_prefix(r).ok().filter(|p| !p.as_os_str().is_empty()))
        .map(Path::to_path_buf)
        .or_else(|| path.file_name().map(PathBuf::from))
        .unwrap_or_default();
    rel.to_string_lossy().replace(char::is_whitespace, "_")
}

/// Analyse every style under `paths` (folders searched recursively).
pub fn run(paths: &[PathBuf]) -> Report {
    let lib = Library::scan(paths);
    let mut rep = Report::default();
    for &id in lib.order() {
        let e = lib.entry(id);
        let file = style_key(&e.path, paths);
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
    too_few: u32,
    same_chord: u32,
    muted_own: u32,
    authored: Score,
    identity: [Identity; 3],
    guitar: [GuitarCheck; 3],
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
            too_few: 0,
            same_chord: 0,
            muted_own: 0,
            authored: Score::default(),
            identity: [Identity::default(); 3],
            guitar: [GuitarCheck::default(); 3],
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
            t.too_few += s.too_few;
            t.same_chord += s.same_chord;
            t.muted_own += s.muted_own;
            for (i, x) in s.identity.iter().enumerate() {
                t.identity[i].add(*x);
            }
            for (i, x) in s.guitar.iter().enumerate() {
                t.guitar[i].add(*x);
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
            "pairs  {} scored; not scored: {} not edited copies, {} with too few notes, {} same source chord, {} muted on their own chord",
            t.pairs, t.unrelated, t.too_few, t.same_chord, t.muted_own
        );
        let _ = writeln!(o, "\n{:<26} {:>7} {:>7} {:>7}", "", "notes", "exact", "pitch");
        let _ = writeln!(o, "{:<26} {}", "as authored", t.authored.cells());
        let _ = writeln!(o, "{:<26} {}", "own chord (identity)", identity_total(&t.identity).score.cells());
        let _ = writeln!(o, "\nNTT table, on every pair");
        for (n, s) in &t.tables {
            let _ = writeln!(o, "  {:<24} {}", format!("{n:?}"), s.cells());
        }
        let _ = writeln!(o, "\nNTR (as authored)   pairs");
        for (i, ntr) in NTRS.iter().enumerate() {
            let (c, s) = t.ntr[i];
            let _ = writeln!(o, "  {:<16} {c:>6}  {}", format!("{ntr:?}"), s.cells());
        }
        // Notes that moved other than by a Note Limit fold break RM p.28: they are ours to fix.
        let _ = writeln!(o, "\nNTR of the note's zone, own chord (identity)       moved, not a Note Limit fold");
        for (i, ntr) in NTRS.iter().enumerate().filter(|(_, n)| **n != Ntr::Guitar) {
            let id = t.identity[i];
            let _ = writeln!(o, "  {:<24} {}  {:>7}", format!("{ntr:?}"), id.score.cells(), id.moved_in_limit);
        }
        // Guitar sources are string codes: no identity; the Guitar check instead.
        let _ = writeln!(o, "\nGuitar check, every chord the channel plays   notes  noise  kept      plays  sounded  chord tone  in range");
        for (i, n) in GUITAR_TABLES.iter().enumerate() {
            let g = t.guitar[i];
            let _ = writeln!(
                o,
                "  {:<42} {:>7} {:>6} {:>5} {:>10} {:>7.1}% {:>10.1}% {:>8.1}%",
                format!("{n:?}"),
                g.notes,
                g.noise,
                g.noise_kept,
                g.played,
                Score::pct(g.sounded, g.played),
                Score::pct(g.chord_tone, g.sounded),
                Score::pct(g.in_range, g.sounded)
            );
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
            let (a, id) = (s.authored(), identity_total(&s.identity).score);
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

    /// The numbers the test pins (tests/oracle/scores.txt). Each line is a key and either one
    /// count or a score: `notes exact pitch-class`.
    pub fn pinned(&self) -> String {
        let mut o = String::new();
        let t = self.totals();
        let _ = writeln!(o, "# yahaha oracle (docs/oracle.md). Counts only; score lines end in: notes exact pitch-class.");
        let _ = writeln!(o, "# Regenerate with UPDATE_GOLDEN=1 cargo test --release oracle.");
        let _ = writeln!(o, "styles {}", self.styles.len());
        let _ = writeln!(o, "unreadable {}", self.errors.len());
        let _ = writeln!(o, "pairs scored {}", t.pairs);
        let _ = writeln!(o, "pairs unrelated {}", t.unrelated);
        let _ = writeln!(o, "pairs too-few {}", t.too_few);
        let _ = writeln!(o, "pairs same-chord {}", t.same_chord);
        let _ = writeln!(o, "pairs muted-own {}", t.muted_own);
        let _ = writeln!(o, "authored {}", t.authored.pinned());
        for (i, ntr) in NTRS.iter().enumerate().filter(|(_, n)| **n != Ntr::Guitar) {
            let _ = writeln!(o, "identity {ntr:?} {}", t.identity[i].score.pinned());
            let _ = writeln!(o, "identity {ntr:?} moved-in-limit {}", t.identity[i].moved_in_limit);
        }
        for (i, n) in GUITAR_TABLES.iter().enumerate() {
            let g = t.guitar[i];
            // Counts that should stay 0 are named for the failure: noise-moved, not-chord-tone,
            // out-of-range. muted is Stroke leaving strings out.
            let _ = writeln!(o, "guitar {n:?} notes {}", g.notes);
            let _ = writeln!(o, "guitar {n:?} noise {}", g.noise);
            let _ = writeln!(o, "guitar {n:?} noise-moved {}", g.noise - g.noise_kept);
            let _ = writeln!(o, "guitar {n:?} plays {}", g.played);
            let _ = writeln!(o, "guitar {n:?} muted {}", g.played - g.sounded);
            let _ = writeln!(o, "guitar {n:?} not-chord-tone {}", g.sounded - g.chord_tone);
            let _ = writeln!(o, "guitar {n:?} out-of-range {}", g.sounded - g.in_range);
        }
        for (n, s) in &t.tables {
            let _ = writeln!(o, "table {n:?} {}", s.pinned());
        }
        for (i, ntr) in NTRS.iter().enumerate() {
            let _ = writeln!(o, "ntr {ntr:?} pairs {}", t.ntr[i].0);
            let _ = writeln!(o, "ntr {ntr:?} {}", t.ntr[i].1.pinned());
        }
        for (k, (c, a, tables)) in &t.changes {
            let _ = writeln!(o, "change {k} pairs {c}");
            let _ = writeln!(o, "change {k} authored {}", a.pinned());
            for (n, s) in tables {
                let _ = writeln!(o, "change {k} {n:?} {}", s.pinned());
            }
        }
        // Every style, so the test can pin exactly the styles listed here (see `pinned_styles`).
        for s in &self.styles {
            let id = identity_total(&s.identity);
            let _ = writeln!(o, "style {} identity {}", s.file, id.score.pinned());
            let _ = writeln!(o, "style {} moved-in-limit {}", s.file, id.moved_in_limit);
            let g = GuitarCheck::total(&s.guitar);
            if g.notes > 0 {
                let _ = writeln!(o, "style {} guitar {} {} {}", s.file, g.sounded, g.chord_tone, g.in_range);
            }
            if !s.pairs.is_empty() {
                let _ = writeln!(o, "style {} pairs {}", s.file, s.pairs.len());
                let _ = writeln!(o, "style {} authored {}", s.file, s.authored().pinned());
            }
        }
        for (f, _) in &self.errors {
            let _ = writeln!(o, "style {f} unreadable 1");
        }
        o
    }
}

fn identity_total(ids: &[Identity; 3]) -> Identity {
    let mut t = Identity::default();
    ids.iter().for_each(|x| t.add(*x));
    t
}

/// The styles a pinned score file covers.
pub fn pinned_styles(pinned: &str) -> BTreeSet<String> {
    pinned.lines().filter_map(|l| l.strip_prefix("style ")?.split_whitespace().next().map(str::to_string)).collect()
}

/// Changed lines between two pinned score files. Score lines (three counts: notes, exact,
/// pitch class) also show the exact-hit rate change; count lines just the counts.
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
    let rate = |v: &[i64]| if v[0] > 0 { 100.0 * v[1] as f64 / v[0] as f64 } else { 0.0 };
    let mut o = String::new();
    for k in a.keys().chain(b.keys().filter(|k| !a.contains_key(*k))) {
        let _ = match (a.get(k), b.get(k)) {
            (Some(x), Some(y)) if x == y => Ok(()),
            (Some(x), Some(y)) if x.len() == 3 && y.len() == 3 => {
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

/// `yahaha oracle <paths> --diff <pinned>`: the delta of this run against a pinned score
/// file, whatever root the run was given. Style keys are relative to the folder the run
/// searched, so a run on `corpus/MOX_v2` names a style `X.sty` where the pin (made on
/// `corpus/`) says `MOX_v2/X.sty`: each style takes the one pinned key that ends in its own.
/// When the run covers only some of the pinned styles, the corpus totals cannot be
/// compared, so only those styles' own lines are.
pub fn diff_against(rep: &mut Report, want: &str) -> String {
    let pinned = pinned_styles(want);
    let rekey = |f: &mut String| {
        if pinned.contains(f.as_str()) {
            return;
        }
        let tail = format!("/{f}");
        let mut m = pinned.iter().filter(|k| k.ends_with(&tail));
        if let (Some(k), None) = (m.next(), m.next()) {
            *f = k.clone();
        }
    };
    rep.styles.iter_mut().for_each(|s| rekey(&mut s.file));
    rep.errors.iter_mut().for_each(|(f, _)| rekey(f));
    let before = rep.styles.len() + rep.errors.len();
    rep.retain(|f| pinned.contains(f));
    let ran: BTreeSet<&str> =
        rep.styles.iter().map(|s| s.file.as_str()).chain(rep.errors.iter().map(|(f, _)| f.as_str())).collect();
    let mut o = String::new();
    if before > ran.len() {
        let _ = writeln!(o, "{} styles of this run are not in the pinned file; left out", before - ran.len());
    }
    let got = rep.pinned();
    if ran.len() == pinned.len() {
        o.push_str(&delta(want, &got));
        return o;
    }
    let _ = writeln!(o, "{} of {} pinned styles in this run: comparing their own lines, not the totals", ran.len(), pinned.len());
    let own = |t: &str| -> String {
        t.lines()
            .filter(|l| l.strip_prefix("style ").and_then(|r| r.split_whitespace().next()).is_some_and(|f| ran.contains(f)))
            .flat_map(|l| [l, "\n"])
            .collect()
    };
    o.push_str(&delta(&own(want), &own(&got)));
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
        let id = identity_total(&r.identity);
        assert_eq!((id.score, id.moved_in_limit), (Score { notes: 12, exact: 12, pc: 12 }, 0));

        let rep = Report { styles: vec![r], errors: vec![] };
        let pinned = rep.pinned();
        for line in ["pairs scored 2", "change M>m MelodicMinor 6 6 6", "style toy pairs 2", "style toy identity 12 12 12"] {
            assert!(pinned.contains(&format!("\n{line}\n")), "{line} missing from\n{pinned}");
        }
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

    /// A Root Trans / Bypass source with one zone over the whole keyboard.
    fn root_trans(src_root: u8, src_type: u8, high_key: u8, lo: u8, hi: u8) -> ChannelRule {
        let mut r = ChannelRule::default_for(13);
        r.src_root = src_root;
        r.src_type = src_type;
        for z in r.zones.iter_mut() {
            (z.ntr, z.ntt, z.high_key, z.lo, z.hi) = (Ntr::RootTrans, Ntt::Bypass, high_key, lo, hi);
        }
        r
    }

    fn one_onset(groups: &[&[u8]]) -> BTreeMap<u32, Onset> {
        let groups: Vec<Vec<u8>> = groups.iter().map(|g| g.to_vec()).collect();
        let mut keys: Vec<u8> = groups.concat();
        keys.sort_unstable();
        [(0, Onset { groups, keys })].into_iter().collect()
    }

    #[test]
    fn identity_buckets_each_note_by_its_own_zone() {
        // Middle zone Root Trans, top zone (above key 88) Guitar: the high note is a Guitar
        // note, whatever the middle zone says. Guitar notes are string codes: they are left
        // out of the identity check and get the Guitar check instead.
        let mut r = root_trans(0, 0, 11, 0, 127);
        r.mid_hi = 88;
        r.zones[2].ntr = Ntr::Guitar;
        r.zones[2].ntt = Ntt::GuitarStroke;
        let mut id = [Identity::default(); 3];
        identity(&one_onset(&[&[60, 64, 91, 100]]), &r, &mut id);
        assert_eq!(id[ntr_index(Ntr::RootTrans)].score.notes, 2);
        assert_eq!(id[ntr_index(Ntr::Guitar)].score.notes, 0);
        assert_eq!(id[ntr_index(Ntr::RootFixed)].score.notes, 0);
        let mut g = [GuitarCheck::default(); 3];
        guitar_check(&one_onset(&[&[60, 64, 91, 100]]), &r, &mut g);
        let st = g[1];
        // Two Guitar notes: 91 plays on all 408 chords (every one a chord tone on the neck),
        // 100 is a noise key and comes back untouched.
        assert_eq!((st.notes, st.noise, st.noise_kept, st.played), (2, 1, 1, 408));
        assert_eq!((st.chord_tone, st.in_range), (st.sounded, st.sounded));
        assert_eq!(g[0], GuitarCheck::default());
    }

    #[test]
    fn identity_tells_note_limit_folds_from_transposer_moves() {
        // Written below its own Note Limit (C3..B3 = 48..59): folding it up is RM p.30, not a miss.
        let r = root_trans(0, 0, 11, 48, 59);
        let mut id = [Identity::default(); 3];
        identity(&one_onset(&[&[36]]), &r, &mut id);
        let rt = id[ntr_index(Ntr::RootTrans)];
        assert_eq!((rt.score, rt.moved_in_limit), (Score { notes: 1, exact: 0, pc: 1 }, 0));

        // A transposer that moves every note down an octave: the note inside the limit (60)
        // and the one written below it (36, whose fold is 48) both count as moved. So does
        // a note below a limit narrower than an octave (40 into 48..55 folds to 52, #13).
        let down = |g: &[u8], _: &ChannelRule, _: Chord, out: &mut [Option<u8>]| {
            g.iter().zip(out.iter_mut()).for_each(|(&k, o)| *o = Some(k - 12));
        };
        let mut id = [Identity::default(); 3];
        identity_with(&one_onset(&[&[60, 36]]), &root_trans(0, 0, 11, 48, 71), &mut id, down);
        identity_with(&one_onset(&[&[40]]), &root_trans(0, 0, 11, 48, 55), &mut id, down);
        let rt = id[ntr_index(Ntr::RootTrans)];
        assert_eq!((rt.score, rt.moved_in_limit), (Score { notes: 3, exact: 0, pc: 3 }, 3));

        // Folding a note outside the limit to the wrong octave is a miss too; only the fold
        // into the limit (36 -> 48) is excused.
        let fold_two = |g: &[u8], _: &ChannelRule, _: Chord, out: &mut [Option<u8>]| {
            g.iter().zip(out.iter_mut()).for_each(|(&k, o)| *o = Some(k + 24));
        };
        let mut id = [Identity::default(); 3];
        identity_with(&one_onset(&[&[36, 24]]), &root_trans(0, 0, 11, 48, 71), &mut id, fold_two);
        let rt = id[ntr_index(Ntr::RootTrans)];
        assert_eq!((rt.score.exact, rt.moved_in_limit), (0, 1));
    }

    #[test]
    fn fold_target_is_the_octave_inside_the_limit() {
        assert_eq!([fold_target(36, 48, 59), fold_target(75, 48, 59), fold_target(50, 48, 59)], [48, 51, 50]);
        // Wider than an octave: the octave nearest where the note was written.
        assert_eq!([fold_target(36, 48, 71), fold_target(90, 48, 71)], [48, 66]);
        assert_eq!(fold_target(127, 0, 11), 7);
        // Narrower than an octave (#13): inside if the pitch class fits, else the octave
        // nearest the limit, the lower one on a tie; a reversed limit reads low to high.
        assert_eq!([fold_target(36, 48, 58), fold_target(36, 60, 48)], [48, 48]);
        assert_eq!([fold_target(43, 60, 64), fold_target(81, 60, 64), fold_target(66, 60, 60)], [67, 57, 54]);
        // At the MIDI edges only octaves that exist count.
        assert_eq!([fold_target(68, 120, 127), fold_target(10, 0, 5)], [116, 10]);
    }

    #[test]
    fn a_source_without_notes_is_too_few_not_unrelated() {
        let mut s = toy([60, 63, 67]);
        let id = SectionId::Main(0);
        s.sections.get_mut(&id).unwrap().events.retain(|e| !matches!(e.ev, Ev::NoteOn { ch: 2, .. }));
        let r = analyse(&s, "toy");
        assert_eq!((r.pairs.len(), r.unrelated, r.too_few), (0, 0, 2));
    }

    #[test]
    fn diff_against_a_subset_run_compares_only_its_styles() {
        let pin = "styles 2\npairs scored 5\nstyle MOX/X.sty identity 10 9 10\nstyle MOX/X.sty moved-in-limit 0\n\
                   style Other/Y.sty identity 4 4 4\nstyle Other/Y.sty moved-in-limit 0\n";
        let run = |file: &str| Report {
            styles: vec![StyleResult { file: file.into(), ..Default::default() }],
            errors: vec![],
        };
        // Keyed below corpus/MOX, the style still finds its pinned key; totals are skipped.
        let d = diff_against(&mut run("X.sty"), pin);
        assert_eq!(
            d,
            "1 of 2 pinned styles in this run: comparing their own lines, not the totals\n  \
             style MOX/X.sty identity: [10, 9, 10] -> [0, 0, 0]  exact 90.0% -> 0.0% (-90.0)\n"
        );
        // An unknown style is reported and left out.
        let d = diff_against(&mut run("Z.sty"), pin);
        assert!(d.starts_with("1 styles of this run are not in the pinned file; left out\n0 of 2 pinned"), "{d}");
    }

    #[test]
    fn onsets_group_like_the_engine() {
        // Ten notes on one tick: groups of 8 and 2. Another channel's note-on in between
        // splits a group; a note-off does not (the engine sorts offs to the front).
        let on = |tick, ch, key| TimedEv { tick, ev: Ev::NoteOn { ch, key, vel: 90 } };
        let mut events: Vec<TimedEv> = (0..10).map(|k| on(0, 3, 50 + k)).collect();
        events.extend([on(480, 3, 60), TimedEv { tick: 480, ev: Ev::NoteOff { ch: 3, key: 40 } }, on(480, 3, 64)]);
        events.extend([on(960, 3, 60), on(960, 4, 30), on(960, 3, 64)]);
        let mut s = toy([60, 63, 67]);
        let id = SectionId::Main(0);
        s.sections.get_mut(&id).unwrap().events = events;
        let o = onsets(&s, id, 3);
        let sizes = |t: u32| o[&t].groups.iter().map(Vec::len).collect::<Vec<_>>();
        assert_eq!((sizes(0), sizes(480), sizes(960)), (vec![8, 2], vec![2], vec![1, 1]));
        assert_eq!(o[&0].keys.len(), 10);
    }

    #[test]
    fn delta_shows_rate_change() {
        let d = delta(
            "pairs scored 3\ntable Melody 10 5 6\nstyle A/X.sty pairs 1\nstyle A/X.sty authored 10 5 5\nstyle A/X.sty identity 20 20 20\n",
            "pairs scored 4\ntable Melody 10 7 8\nstyle A/X.sty pairs 1\nstyle A/X.sty authored 10 9 9\nstyle A/X.sty identity 20 20 20\nstyle B/X.sty pairs 1\n",
        );
        assert_eq!(
            d,
            "  pairs scored: [3] -> [4]\n  \
             style A/X.sty authored: [10, 5, 5] -> [10, 9, 9]  exact 50.0% -> 90.0% (+40.0)\n  \
             table Melody: [10, 5, 6] -> [10, 7, 8]  exact 50.0% -> 70.0% (+20.0)\n  \
             style B/X.sty pairs: new [1]\n"
        );
        assert_eq!(delta("a 1\n", "# comment\na 1\n"), "");
    }

    #[test]
    fn styles_are_keyed_by_path_and_filtered_by_the_pin() {
        let roots = [PathBuf::from("/c")];
        assert_eq!(style_key(Path::new("/c/My Folder/X.sty"), &roots), "My_Folder/X.sty");
        let mut rep = Report {
            styles: ["A/X.sty", "B/X.sty", "C/new.sty"]
                .iter()
                .map(|f| StyleResult { file: f.to_string(), ..Default::default() })
                .collect(),
            errors: vec![("D/bad.sty".into(), "?".into())],
        };
        let pin = "styles 2\nstyle A/X.sty identity 0 0 0\nstyle B/X.sty identity 0 0 0\n";
        let keep = pinned_styles(pin);
        rep.retain(|f| keep.contains(f));
        assert_eq!(rep.styles.iter().map(|s| s.file.as_str()).collect::<Vec<_>>(), ["A/X.sty", "B/X.sty"]);
        assert!(rep.errors.is_empty());
    }

    /// Pins the corpus scores, so every change to the transposer reports what it did to them.
    /// Numbers only: nothing in tests/oracle/scores.txt is note content. Only the styles the
    /// file lists count, so adding styles to corpus/ does not move the pin.
    #[test]
    fn corpus_scores() {
        let corpus = root().join("corpus");
        if !corpus.is_dir() {
            eprintln!("oracle: no corpus; skipping");
            return;
        }
        let mut rep = run(&[corpus]);
        assert!(!rep.styles.is_empty(), "corpus/ holds no styles");
        let file = root().join("tests/oracle/scores.txt");
        if std::env::var_os("UPDATE_GOLDEN").is_some() {
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(&file, rep.pinned()).unwrap();
            eprintln!("oracle: wrote {}", file.display());
            return;
        }
        let want = std::fs::read_to_string(&file).unwrap_or_default();
        let keep = pinned_styles(&want);
        let before = rep.styles.len() + rep.errors.len();
        rep.retain(|f| keep.contains(f));
        let extra = before - rep.styles.len() - rep.errors.len();
        if extra > 0 {
            eprintln!("oracle: {extra} corpus styles are not in tests/oracle/scores.txt; not scored");
        }
        let d = delta(&want, &rep.pinned());
        assert!(
            d.is_empty(),
            "oracle scores changed (want -> got):\n{d}\nIf the change is intended, regenerate with UPDATE_GOLDEN=1 cargo test --release oracle, commit tests/oracle/scores.txt and quote this delta in the PR."
        );
    }

    /// The committed scores are counts: every line is a short key followed by one count or
    /// one score (three counts).
    #[test]
    fn committed_scores_hold_only_numbers() {
        let text = std::fs::read_to_string(root().join("tests/oracle/scores.txt")).expect("tests/oracle/scores.txt");
        for line in text.lines().filter(|l| !l.starts_with('#')) {
            let w: Vec<&str> = line.split_whitespace().collect();
            let key = w.iter().rposition(|x| x.parse::<u32>().is_err()).map_or(0, |i| i + 1);
            let max_key = match w.first() {
                Some(&"styles") | Some(&"unreadable") | Some(&"authored") => 1,
                Some(&"pairs") | Some(&"table") => 2,
                Some(&"identity") | Some(&"ntr") | Some(&"change") | Some(&"style") | Some(&"guitar") => 3,
                _ => 0,
            };
            assert!((1..=max_key).contains(&key) && matches!(w.len() - key, 1 | 3), "not a count line: {line}");
        }
    }
}

//! Golden snapshots: play tests/golden/chords.script on a few corpus styles and compare what
//! the band plays with the stored result, so every change shows up in review.
//!
//! The readable per-part note listing (`sim::snapshot`) transcribes the style's own patterns,
//! which are Yamaha's content, so it never leaves the developer's machine: it is written to the
//! git-ignored tests/golden/local/. What is committed is a digest of it: one line per bar and
//! part holding a hash of that part's notes, under the bar's header (our own script's
//! sections and chords). `UPDATE_GOLDEN=1` rewrites the digests and the local baselines.
//! See tests/golden/README.md.

use crate::sff::Style;
use crate::sim;
use std::path::{Path, PathBuf};

/// Chosen for coverage: Guitar NTR (stroke and all-purpose), minor-5th NTT tables and several
/// source chords, chord-mute routing to separate major/minor parts, SFF1, and a style
/// without CASM.
const STYLES: [&str; 6] = [
    "BluesOrganTrio.S930.STY",
    "JackDoesItAgain.S930.STY",
    "OrganCruise.S930.STY",
    "SmoothItOver.S930.STY",
    "TickingAway.T162.sty",
    "AustinCityBlues.S930.STY",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn find_style(name: &str) -> Option<PathBuf> {
    let mut stack = vec![root().join("corpus")];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.file_name().is_some_and(|f| f == name) {
                return Some(p);
            }
        }
    }
    None
}

/// Line diff (longest common subsequence) with the enclosing `bar` line as context and, for
/// a changed part line, the individual notes that differ.
fn diff(want: &str, got: &str) -> String {
    let (a, b): (Vec<&str>, Vec<&str>) = (want.lines().collect(), got.lines().collect());
    let (n, m) = (a.len(), b.len());
    let mut lcs = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i][j] = if a[i] == b[j] { lcs[i + 1][j + 1] + 1 } else { lcs[i + 1][j].max(lcs[i][j + 1]) };
        }
    }
    let mut out = Vec::new();
    let mut bar = "";
    let (mut i, mut j) = (0, 0);
    let mut removed: Vec<&str> = Vec::new();
    let mut added: Vec<&str> = Vec::new();
    let flush = |out: &mut Vec<String>, removed: &mut Vec<&str>, added: &mut Vec<&str>, bar: &str, shown: &mut String| {
        if removed.is_empty() && added.is_empty() {
            return;
        }
        if bar != shown.as_str() {
            out.push(format!("  {bar}"));
            *shown = bar.to_string();
        }
        for (k, r) in removed.iter().enumerate() {
            out.push(format!("- {r}"));
            if let Some(ad) = added.get(k) {
                out.push(format!("+ {ad}"));
                let words = |s: &str| s.split("  ").map(str::trim).map(String::from).collect::<Vec<_>>();
                let (wr, wa) = (words(r), words(ad));
                // Multiset difference, so a note that loses one of two copies still shows.
                fn minus<'a>(a: &'a [String], b: &[String]) -> Vec<&'a String> {
                    let mut rest: Vec<&String> = b.iter().collect();
                    a.iter()
                        .filter(|w| match rest.iter().position(|x| x == w) {
                            Some(i) => {
                                rest.remove(i);
                                false
                            }
                            None => true,
                        })
                        .collect()
                }
                let (gone, new) = (minus(&wr, &wa), minus(&wa, &wr));
                let list = |v: &[&String]| {
                    if v.is_empty() {
                        "nothing".to_string()
                    } else {
                        v.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                    }
                };
                if !(gone.is_empty() && new.is_empty()) && gone.len() + new.len() < wr.len() {
                    out.push(format!("    ^ {} -> {}", list(&gone), list(&new)));
                }
            }
        }
        for ad in added.iter().skip(removed.len()) {
            out.push(format!("+ {ad}"));
        }
        removed.clear();
        added.clear();
    };
    let mut shown = String::new();
    while i < n || j < m {
        if i < n && j < m && a[i] == b[j] {
            flush(&mut out, &mut removed, &mut added, bar, &mut shown);
            if a[i].starts_with("bar ") {
                bar = a[i];
            }
            i += 1;
            j += 1;
        } else if j < m && (i == n || lcs[i][j + 1] >= lcs[i + 1][j]) {
            added.push(b[j]);
            j += 1;
        } else {
            if a[i].starts_with("bar ") {
                bar = a[i];
            }
            removed.push(a[i]);
            i += 1;
        }
    }
    flush(&mut out, &mut removed, &mut added, bar, &mut shown);
    let total = out.len();
    let mut s: String = out.into_iter().take(120).collect::<Vec<_>>().join("\n");
    if total > 120 {
        s += &format!("\n  ... {} more diff lines", total - 120);
    }
    s
}

/// 64-bit FNV-1a. Fixed and dependency-free, so a digest means the same on every machine and
/// toolchain (std's `DefaultHasher` makes no such promise).
fn fnv1a(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3))
}

/// A part line's key: channel and part name (`ch11 Bass`).
fn part_key(line: &str) -> String {
    line.split_whitespace().take(2).collect::<Vec<_>>().join(" ")
}

/// The committed form of a listing. Lines before the first bar (style name, tempo, bar count)
/// and every `bar` header stay as they are: they come from our script. Each part line becomes
/// its key and the hash of the whole line, so the notes cannot be read back from it.
fn digest(listing: &str) -> String {
    let mut out = String::new();
    for line in listing.lines() {
        if line.starts_with("  ch") {
            out += &format!("  {:<12} {:016x}\n", part_key(line), fnv1a(line.trim()));
        } else {
            out += line;
            out.push('\n');
        }
    }
    out
}

/// One bar of a digest or listing: its header and its part lines as (key, line).
struct Bar<'a> {
    header: &'a str,
    parts: Vec<(String, &'a str)>,
}

/// Splits a digest or listing into the lines before the first bar and the bars.
fn bars(text: &str) -> (Vec<&str>, Vec<Bar<'_>>) {
    let (mut pre, mut bars) = (Vec::new(), Vec::<Bar>::new());
    for line in text.lines() {
        if line.starts_with("bar ") {
            bars.push(Bar { header: line, parts: Vec::new() });
        } else if line.starts_with("  ch")
            && let Some(bar) = bars.last_mut()
        {
            bar.parts.push((part_key(line), line));
        } else if bars.is_empty() && !line.is_empty() {
            pre.push(line);
        }
    }
    (pre, bars)
}

/// What changed between the stored digest and the current one: for each bar that differs, the
/// parts that changed and the expected and actual header. Returns the report and the indices
/// of the changed bars.
fn compare(want: &str, got: &str) -> (String, Vec<usize>) {
    let ((wpre, wbars), (gpre, gbars)) = (bars(want), bars(got));
    let mut out = Vec::new();
    if wpre != gpre {
        out.push(format!("  style header\n    want {}\n    got  {}", wpre.join(" / "), gpre.join(" / ")));
    }
    if wbars.len() != gbars.len() {
        out.push(format!("  bar count: want {}, got {}", wbars.len(), gbars.len()));
    }
    let mut changed = Vec::new();
    for i in 0..wbars.len().max(gbars.len()) {
        let (w, g) = (wbars.get(i), gbars.get(i));
        fn head<'a>(b: Option<&Bar<'a>>) -> &'a str {
            b.map_or("(none)", |b| b.header)
        }
        fn line<'a>(b: Option<&Bar<'a>>, k: &str) -> Option<&'a str> {
            b.and_then(|b| b.parts.iter().find(|p| p.0 == k)).map(|p| p.1)
        }
        let mut keys: Vec<&str> = Vec::new();
        for b in w.into_iter().chain(g) {
            for (k, _) in &b.parts {
                if !keys.contains(&k.as_str()) {
                    keys.push(k);
                }
            }
        }
        keys.sort_by_key(|k| k.get(2..).and_then(|k| k.split(' ').next()).and_then(|c| c.parse::<u8>().ok()));
        let parts: Vec<String> = keys
            .into_iter()
            .filter_map(|k| match (line(w, k), line(g, k)) {
                (Some(a), Some(b)) if a == b => None,
                (Some(_), Some(_)) => Some(format!("{k} changed")),
                (Some(_), None) => Some(format!("{k} gone")),
                _ => Some(format!("{k} new")),
            })
            .collect();
        if parts.is_empty() && head(w) == head(g) {
            continue;
        }
        changed.push(i);
        let what = if parts.is_empty() { "header".to_string() } else { parts.join(", ") };
        out.push(format!("  bar {}: {what}\n    want {}\n    got  {}", i + 1, head(w), head(g)));
    }
    let total = out.len();
    let mut s = out.into_iter().take(60).collect::<Vec<_>>().join("\n");
    if total > 60 {
        s += &format!("\n  ... {} more", total - 60);
    }
    (s, changed)
}

/// Just the given bars of a listing, for a readable diff of those bars only.
fn pick(listing: &str, which: &[usize]) -> String {
    let (_, bars) = bars(listing);
    let mut out = String::new();
    for b in which.iter().filter_map(|&i| bars.get(i)) {
        out += b.header;
        out.push('\n');
        for (_, line) in &b.parts {
            out += line;
            out.push('\n');
        }
    }
    out
}

#[test]
fn golden_snapshots() {
    let dir = root().join("tests/golden");
    let local = dir.join("local");
    let script = std::fs::read_to_string(dir.join("chords.script")).unwrap();
    let update = std::env::var("UPDATE_GOLDEN").is_ok_and(|v| v == "1");
    let mut failures = Vec::new();
    let mut ran = 0;
    for name in STYLES {
        let Some(path) = find_style(name) else {
            eprintln!("golden: {name} not in corpus; skipping");
            continue;
        };
        ran += 1;
        let style = Style::load(&path).unwrap();
        let got = sim::snapshot(&style, &script).unwrap();
        let got_digest = digest(&got);
        let current = local.join(format!("{name}.txt"));
        std::fs::create_dir_all(&local).unwrap();
        std::fs::write(&current, &got).unwrap();
        let file = dir.join(format!("{name}.digest"));
        let baseline = local.join(format!("{name}.baseline.txt"));
        if update {
            std::fs::write(&file, &got_digest).unwrap();
            std::fs::write(&baseline, &got).unwrap();
            eprintln!("golden: wrote {} and {}", file.display(), baseline.display());
            continue;
        }
        let Ok(want_digest) = std::fs::read_to_string(&file) else {
            failures.push(format!("{name}: no stored digest {}", file.display()));
            continue;
        };
        let base = std::fs::read_to_string(&baseline).ok();
        if want_digest == got_digest {
            // A fresh checkout has no baseline yet, and this listing is the committed one.
            if base.is_none() {
                std::fs::write(&baseline, &got).unwrap();
            }
            continue;
        }
        let (report, changed) = compare(&want_digest, &got_digest);
        let mut msg = format!("{name}: {} bar(s) changed\n{report}", changed.len());
        match base {
            Some(base) => {
                let stale = if digest(&base) == want_digest {
                    ""
                } else {
                    "\n  (this baseline does not match the committed digest: it is from another revision)"
                };
                msg += &format!(
                    "\n\n  notes in the changed bars, from {} (- baseline, + now){stale}\n{}",
                    baseline.display(),
                    diff(&pick(&base, &changed), &pick(&got, &changed))
                );
            }
            None => {
                msg += &format!(
                    "\n\n  no local baseline {}: run UPDATE_GOLDEN=1 on the last good revision to see which notes changed",
                    baseline.display()
                );
            }
        }
        msg += &format!("\n  full listing now: {}", current.display());
        failures.push(msg);
    }
    if ran == 0 {
        eprintln!("golden: no corpus; skipping");
    }
    assert!(
        failures.is_empty(),
        "{}\n\nIf the change is intended, regenerate with UPDATE_GOLDEN=1 cargo test --release golden and commit the digests.",
        failures.join("\n\n")
    );
}

/// Same script, same style, same listing: nothing in the snapshot depends on the run (two
/// separate loads, two engines). The listing must also carry what the golden test relies on:
/// the intro, a fill inside a bar, the ending running out, and the chord parts.
#[test]
fn snapshot_is_deterministic() {
    let Some(path) = find_style(STYLES[0]) else {
        return;
    };
    let script = "[IntroA] | C | - | Am/E - - [MainB] - | Fm6 | G7 - - [EndingA] - | - | - | - |";
    let a = sim::snapshot(&Style::load(&path).unwrap(), script).unwrap();
    let b = sim::snapshot(&Style::load(&path).unwrap(), script).unwrap();
    assert_eq!(a, b);
    for want in ["bar 1  Intro A", "> Fill In BB@4.0000", "bar 6  Ending A", "bar 7  stopped", "ch11 Bass ", "ch12 Chord1 "] {
        assert!(a.contains(want), "missing {want:?} in\n{a}");
    }
}

/// The committed digests hold nothing but our script's headers and hashes: every part line is
/// a key and 16 hex digits, and no line carries a note. Checked even without the corpus.
#[test]
fn committed_digests_hold_no_notes() {
    let dir = root().join("tests/golden");
    for name in STYLES {
        let file = dir.join(format!("{name}.digest"));
        let text = std::fs::read_to_string(&file).unwrap_or_else(|_| panic!("no stored digest {}", file.display()));
        assert!(!dir.join(format!("{name}.txt")).exists(), "a readable listing of {name} is in tests/golden; it belongs in tests/golden/local");
        for line in text.lines() {
            assert!(!line.contains('~'), "{name}: note listed: {line}");
            if line.starts_with("  ") {
                let words: Vec<&str> = line.split_whitespace().collect();
                assert!(
                    words.len() == 3 && words[0].starts_with("ch") && words[2].len() == 16 && words[2].chars().all(|c| c.is_ascii_hexdigit()),
                    "{name}: not a digest line: {line}"
                );
            }
        }
    }
    // The hash is pinned: changing it would invalidate every committed digest.
    assert_eq!(fnv1a("a"), 0xaf63_dc4c_8601_ec8c, "FNV-1a 64 reference vector");
    assert_eq!(digest("bar 1  Main A  C@1.0000\n  ch11 Bass    1.0000 C1~460\n").lines().nth(1), Some("  ch11 Bass    1b44b04c0f7fc400"));
}

/// Parts that play as written whatever the chord (drums, Root Fixed + Bypass) are only
/// counted, so even the local listings do not transcribe the style's own patterns for them.
#[test]
fn snapshots_do_not_list_parts_played_as_written() {
    let mut listings: Vec<(String, String)> = Vec::new();
    if let Some(path) = find_style("OrganCruise.S930.STY") {
        let style = Style::load(&path).unwrap();
        // Its fills route Phrase 1 (ch15) through Root Fixed + Bypass: a melodic part that
        // still plays as written.
        let prep = crate::engine::Prepared::new(&style);
        let fill = prep.sections[crate::engine::slot_of(crate::sff::SectionId::Fill(0))].as_ref().unwrap();
        assert!(fill.plays_as_written(8) && fill.plays_as_written(9) && fill.plays_as_written(14));
        assert!(!fill.plays_as_written(10), "the Bass follows the chord");
        let got = sim::snapshot(&style, "| C | - - - [MainA] | F |").unwrap();
        assert!(got.contains("  ch15 Phrase1 "), "Phrase 1 should still be counted:\n{got}");
        for line in got.lines().filter(|l| l.starts_with("  ch15 ")) {
            assert!(line.ends_with(" as written"), "Phrase 1 notes listed: {line}");
        }
        listings.push(("OrganCruise now".into(), got));
    }
    for (what, text) in &listings {
        for line in text.lines().filter(|l| l.starts_with("  ch9 ") || l.starts_with("  ch10 ")) {
            assert!(line.ends_with(" as written") && !line.contains('~'), "{what}: drum notes listed: {line}");
        }
    }
}

/// A button written on a beat fires for that beat (one tick early, see `sim::snapshot`) and
/// shows at the tick it was pressed, even when that is in the previous bar.
#[test]
fn buttons_count_for_their_beat() {
    let Some(path) = find_style(STYLES[0]) else {
        return;
    };
    let got = sim::snapshot(&Style::load(&path).unwrap(), "| C - [Break] - - | [StartStop] | - |").unwrap();
    let bars: Vec<&str> = got.lines().filter(|l| l.starts_with("bar ")).collect();
    assert_eq!(
        bars[0],
        "bar 1  Main A > Fill In BA@3.0000 > stopped@4.1919  C@1.0000  [Break]@2.1919  [StartStop]@4.1919",
        "{got}"
    );
    assert!(bars[1].starts_with("bar 2  stopped"), "{got}");
}

/// `^` lets go of the chord: with Sync Stop on the style stops there, and the next chord
/// starts it again.
#[test]
fn release_triggers_sync_stop() {
    let Some(path) = find_style(STYLES[0]) else {
        return;
    };
    let got = sim::snapshot(&Style::load(&path).unwrap(), "[SyncStop] | C - ^ - | - | F |").unwrap();
    let bars: Vec<&str> = got.lines().filter(|l| l.starts_with("bar ")).collect();
    assert_eq!(bars[0], "bar 1  Main A > stopped@3.0000  [SyncStop]@1.0000  C@1.0000  ^@3.0000", "{got}");
    assert!(bars[1].starts_with("bar 2  stopped"), "{got}");
    assert!(bars[2].starts_with("bar 3  Main A"), "{got}");
}

#[test]
fn script_grammar() {
    use crate::engine::Button;
    use sim::Step;
    let (steps, bars) = sim::parse_script("[IntroA] | C | Am G7 | [MainB] | F - - [EndingA] - | - | # end", 1920).unwrap();
    assert_eq!(bars, 4);
    let got: Vec<(u32, String)> = steps.iter().map(|s| (s.tick, s.label.clone())).collect();
    let want = [(0, "[IntroA]"), (0, "C"), (1920, "Am"), (2880, "G7"), (3840, "[MainB]"), (3840, "F"), (5280, "[EndingA]")];
    assert_eq!(got, want.map(|(t, l)| (t, l.to_string())));
    assert!(matches!(steps[4].step, Step::Button(Button::Main(1))));
    // No "|": one chord per bar, as `yahaha sim style "C Am F G7"` always worked.
    let (steps, bars) = sim::parse_script("C Am/E F G7", 1920).unwrap();
    assert_eq!(bars, 4);
    assert_eq!(steps[1].tick, 1920);
    assert!(matches!(steps[1].step, Step::Chord(c) if c.name() == "Am/E"));
    assert!(sim::parse_script("| C | [Nope] |", 1920).is_err());
    assert!(sim::parse_script("| C | [MainB]", 1920).is_err());
    assert!(sim::parse_script("| C | | G7 |", 1920).is_err(), "empty bar on one line");
    assert!(sim::parse_script("| C | [MainB] | G7 |", 1920).is_ok(), "a bar with only a button is not empty");
    // Labels keep the script's spelling; `^` releases the chord and takes a slot.
    let (steps, _) = sim::parse_script("| Dbmaj7 ^ [StartStop] - [StopAcmp] C1+5 |", 1920).unwrap();
    let got: Vec<(u32, &str)> = steps.iter().map(|s| (s.tick, s.label.as_str())).collect();
    assert_eq!(got, [(0, "Dbmaj7"), (480, "^"), (960, "[StartStop]"), (1440, "[StopAcmp]"), (1440, "C1+5")]);
    assert!(matches!(steps[1].step, Step::Release));
    assert!(matches!(steps[2].step, Step::Button(Button::StartStop)));
    assert!(matches!(steps[3].step, Step::Button(Button::StopAcmp)));
}

/// A digest mismatch names the bar, the parts that changed and both headers, and `pick` gives
/// the readable diff of just those bars.
#[test]
fn compare_names_bars_and_parts() {
    let want = "style  X\nbars   2\n\nbar 1  Main A  C@1.0000\n  ch11 Bass    1.0000 C1~460\n\nbar 2  Main A  G7@1.0000\n  ch11 Bass    1.0000 G1~460\n  ch12 Chord1  1.0000 B2~460\n";
    let got = "style  X\nbars   2\n\nbar 1  Main A  C@1.0000\n  ch11 Bass    1.0000 C1~460\n\nbar 2  Main B  G7@1.0000\n  ch11 Bass    1.0000 A1~460\n  ch13 Chord2  1.0000 D3~460\n";
    let (report, changed) = compare(&digest(want), &digest(got));
    assert_eq!(changed, [1]);
    assert_eq!(
        report,
        "  bar 2: ch11 Bass changed, ch12 Chord1 gone, ch13 Chord2 new\n    want bar 2  Main A  G7@1.0000\n    got  bar 2  Main B  G7@1.0000"
    );
    assert!(diff(&pick(want, &changed), &pick(got, &changed)).contains("^ 1.0000 G1~460 -> 1.0000 A1~460"));
    assert_eq!(compare(&digest(want), &digest(want)), (String::new(), vec![]));
}

#[test]
fn diff_is_readable() {
    let want = "bars 1\n\nbar 1  Main A  C@1.0000\n  ch11 Bass    1.0000 C1~460  2.0000 G1~460\n";
    let got = "bars 1\n\nbar 1  Main A  C@1.0000\n  ch11 Bass    1.0000 C1~460  2.0000 A1~460\n";
    assert_eq!(
        diff(want, got),
        "  bar 1  Main A  C@1.0000\n\
         -   ch11 Bass    1.0000 C1~460  2.0000 G1~460\n\
         +   ch11 Bass    1.0000 C1~460  2.0000 A1~460\n    \
         ^ 2.0000 G1~460 -> 2.0000 A1~460"
    );
    // Notes compare as a multiset: losing one of two copies still names the note.
    let want = "bar 1  Main A\n  ch11 Bass    1.0000 C1~460  1.0000 C1~460  2.0000 G1~460\n";
    let got = "bar 1  Main A\n  ch11 Bass    1.0000 C1~460  2.0000 G1~460\n";
    assert!(diff(want, got).ends_with("^ 1.0000 C1~460 -> nothing"), "{}", diff(want, got));
}

//! Golden snapshots: play tests/golden/chords.script on a few corpus styles and compare the
//! per-part note listing (`sim::snapshot`) with the stored one, so every change in what the
//! band plays shows up as a reviewed diff. `UPDATE_GOLDEN=1` rewrites the stored listings.
//! Only our own script's output is stored, never style data. See tests/golden/README.md.

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
                let gone: Vec<&String> = wr.iter().filter(|w| !wa.contains(w)).collect();
                let new: Vec<&String> = wa.iter().filter(|w| !wr.contains(w)).collect();
                if !gone.is_empty() && gone.len() + new.len() < wr.len() {
                    out.push(format!(
                        "    ^ {} -> {}",
                        gone.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "),
                        new.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                    ));
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

#[test]
fn golden_snapshots() {
    let dir = root().join("tests/golden");
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
        let file = dir.join(format!("{name}.txt"));
        if update {
            std::fs::write(&file, &got).unwrap();
            eprintln!("golden: wrote {}", file.display());
            continue;
        }
        match std::fs::read_to_string(&file) {
            Ok(want) if want == got => {}
            Ok(want) => failures.push(format!("{name}: snapshot changed (- stored, + now)\n{}", diff(&want, &got))),
            Err(_) => failures.push(format!("{name}: no stored snapshot {}", file.display())),
        }
    }
    if ran == 0 {
        eprintln!("golden: no corpus; skipping");
    }
    assert!(
        failures.is_empty(),
        "{}\n\nIf the change is intended, regenerate with UPDATE_GOLDEN=1 cargo test --release golden and commit the diff.",
        failures.join("\n\n")
    );
}

/// Same script, same style, same listing: nothing in the snapshot depends on the run.
#[test]
fn snapshot_is_deterministic() {
    let Some(path) = find_style(STYLES[0]) else {
        return;
    };
    let style = Style::load(&path).unwrap();
    let script = "[IntroA] | C | Am/E - - [MainB] - | Fm6 | G7 - - [EndingA] - | - |";
    assert_eq!(sim::snapshot(&style, script).unwrap(), sim::snapshot(&style, script).unwrap());
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
}

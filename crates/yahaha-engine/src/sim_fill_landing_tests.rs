//! Fills and where they land (#282, owner playtest 2026-09-26), with Auto Fill on: the
//! first press picks the fill, and every later press before the fill ends changes only the
//! Main it lands on. Pressing the fill's own Main while it plays repeats it once more.

use super::*;
use yahaha_sff::sff::SectionId::{Fill, Main};

/// SlowWalker (4/4, 75 BPM, Mains A-D with their fills).
fn style() -> Option<Box<Prepared>> {
    let p = yahaha_sff::library::corpus_dir().join("MOX_v2/SlowWalker.T552.sty");
    if !p.exists() {
        eprintln!("corpus missing; skipping");
        return None;
    }
    Some(Box::new(Prepared::new(&yahaha_sff::sff::Style::load(&p).unwrap())))
}

const A: u8 = 0;
const B: u8 = 1;
const C: u8 = 2;

/// Start on Main A with a C chord (Auto Fill on, the default), press `presses` (bar,
/// quarter-beats into it; a bar is 4 beats), and return the sections played in order,
/// each with the bar (0-based, from the start) it came in.
fn played(presses: &[(f64, u8)], bars: u64) -> Option<Vec<(SectionId, u64)>> {
    let p = style()?;
    let bar = (60e9 / p.bpm * (p.tpb as f64 / p.ppq as f64)) as u64;
    let mut script = vec![(0, Step::Chord(Chord::new(0, 0)))];
    script.extend(presses.iter().map(|&(at, i)| ((at * bar as f64) as u64, Step::Button(Button::Main(i)))));
    let mut seen: Vec<(SectionId, u64)> = Vec::new();
    let mut last_beat = 0;
    run_observed(p, &script, bars * bar, |e, now| {
        let s = e.snapshot(now);
        let Some(cur) = s.cur else { return };
        let at = (now + bar / 100) / bar;
        // A new section, or the same fill again from its top (its beat goes back to 1).
        let again = matches!(cur, Fill(_)) && s.beat < last_beat;
        last_beat = s.beat;
        if seen.last().map(|s| s.0) != Some(cur) || again {
            seen.push((cur, at));
        }
    });
    Some(seen)
}

/// A → press B: B's fill, then B.
#[test]
fn a_then_b() {
    let Some(seen) = played(&[(1.6, B)], 4) else { return };
    assert_eq!(seen, [(Main(A), 0), (Fill(B), 1), (Main(B), 2)]);
}

/// A → B, B: B's fill, then B (the second press re-confirms it).
#[test]
fn a_then_b_b() {
    let Some(seen) = played(&[(1.6, B), (1.7, B)], 4) else { return };
    assert_eq!(seen, [(Main(A), 0), (Fill(B), 1), (Main(B), 2)]);
}

/// A → B, A: B's fill, landing on A. Pressed before the fill starts and while it plays.
#[test]
fn a_then_b_a() {
    let Some(seen) = played(&[(1.55, B), (1.6, A)], 4) else { return };
    assert_eq!(seen, [(Main(A), 0), (Fill(B), 1), (Main(A), 2)], "before it starts");
    let Some(seen) = played(&[(1.55, B), (1.9, A)], 4) else { return };
    assert_eq!(seen, [(Main(A), 0), (Fill(B), 1), (Main(A), 2)], "while it plays");
}

/// A → B, C: B's fill, landing on C.
#[test]
fn a_then_b_c() {
    let Some(seen) = played(&[(1.55, B), (1.6, C)], 4) else { return };
    assert_eq!(seen, [(Main(A), 0), (Fill(B), 1), (Main(C), 2)]);
}

/// Mashing B across three fills: three B fills back to back, then B.
#[test]
fn mashing_b_repeats_its_fill() {
    let Some(seen) = played(&[(1.6, B), (1.9, B), (2.3, B), (2.6, B), (2.9, B)], 6) else { return };
    assert_eq!(seen, [(Main(A), 0), (Fill(B), 1), (Fill(B), 2), (Fill(B), 3), (Main(B), 4)]);
}

/// Mashing B, then A during the last fill: it lands on A (the repeat is called off).
#[test]
fn mashing_b_then_a_lands_on_a() {
    let Some(seen) = played(&[(1.6, B), (1.9, B), (2.3, B), (2.5, A)], 5) else { return };
    assert_eq!(seen, [(Main(A), 0), (Fill(B), 1), (Fill(B), 2), (Main(A), 3)]);
}

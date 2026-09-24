//! Form expansion: the written bars, played in order, as one linear list.

use super::chart::{BeatChord, Chart, ChartBar, JumpFrom, JumpTo};
use crate::theory::{CANCEL, NOTE_NAMES, TYPE_NAMES};

/// The most bars [`expand`] ever returns, whatever the chart says.
pub const MAX_BARS: usize = 10_000;

/// One bar as played.
#[derive(Debug, Clone, PartialEq)]
pub struct Bar {
    /// (beats, beat unit)
    pub time: (u8, u8),
    pub section: Option<char>,
    /// The written bar carries a section mark.
    pub section_start: bool,
    /// Chords by beat, in beat order. A beat with no chord (and a bar with none at all)
    /// holds the chord before.
    pub chords: Vec<BeatChord>,
    /// Index of the written bar in [`Chart::bars`].
    pub source: usize,
    /// Which time through the form, from 1.
    pub chorus: u32,
}

impl std::fmt::Display for Bar {
    /// `*A 4/4 | Cmaj7 . Am7 . |`: `*` marks a section start, `.` a beat that holds.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sec = self.section.map_or("  ".to_string(), |c| format!("{}{c}", if self.section_start { '*' } else { ' ' }));
        write!(f, "{sec} {}/{} |", self.time.0, self.time.1)?;
        for beat in 0..self.time.0.max(1) {
            match self.chords.iter().find(|c| c.beat == beat) {
                Some(c) if c.chord.ty == CANCEL => write!(f, " N.C.")?,
                Some(c) => {
                    let ty = TYPE_NAMES.get(c.chord.ty as usize).copied().unwrap_or("?");
                    write!(f, " {}{ty}", NOTE_NAMES[c.chord.root as usize % 12])?;
                    if let Some(b) = c.chord.bass {
                        write!(f, "/{}", NOTE_NAMES[b as usize % 12])?;
                    }
                }
                None => write!(f, " .")?,
            }
        }
        write!(f, " |")
    }
}

/// A repeat: from `start` to each `}` in `ends`, with its numbered endings.
struct Group {
    start: usize,
    ends: Vec<usize>,
    endings: Vec<(u8, usize)>,
    total: u32,
    max_ending: u8,
}

fn groups(bars: &[ChartBar]) -> (Vec<Group>, Vec<Option<usize>>) {
    let mut gs: Vec<Group> = Vec::new();
    let mut of: Vec<Option<usize>> = vec![None; bars.len()];
    let mut cur: Option<usize> = None;
    let mut prev_end: Option<usize> = None;
    let new = |start| Group { start, ends: Vec::new(), endings: Vec::new(), total: 2, max_ending: 0 };
    for (i, b) in bars.iter().enumerate() {
        // The bar after a `}` that isn't an ending leaves the repeat behind.
        if prev_end == i.checked_sub(1) && prev_end.is_some() && b.ending.is_none() && !b.repeat_start {
            cur = None;
        }
        if b.repeat_start {
            gs.push(new(i));
            cur = Some(gs.len() - 1);
        }
        if let (Some(k), Some(g)) = (b.ending, cur) {
            gs[g].endings.push((k, i));
            of[i] = Some(g);
        }
        if b.repeat_end {
            // A `}` with no `{` repeats from just after the last `}` (or the top).
            let g = *cur.get_or_insert_with(|| {
                gs.push(new(prev_end.map_or(0, |e| e + 1)));
                gs.len() - 1
            });
            gs[g].ends.push(i);
            of[i] = Some(g);
            prev_end = Some(i);
        }
    }
    for g in &mut gs {
        g.max_ending = g.endings.iter().map(|e| e.0).max().unwrap_or(0);
        let last = g.ends.last().copied().unwrap_or(g.start);
        let count = bars[g.start..=last.max(g.start)].iter().filter_map(|b| b.repeat_count).max();
        g.total = count.unwrap_or(2).max(g.max_ending as u32).max(1);
    }
    for (gi, g) in gs.iter().enumerate() {
        of[g.start].get_or_insert(gi);
    }
    (gs, of)
}

/// Play the chart `choruses` times through into a linear list of bars.
///
/// - **Repeats** `{ }` play twice, or as many times as a `3x`-style comment in the repeat
///   (or the highest ending number) says. A `}` with no `{` repeats from just after the last
///   `}`, or from the top.
/// - **Endings.** On pass `p` of `t`, ending `min(p, last - 1)` plays before the last pass
///   and the last ending on it, so `N1 N2` with `3x` plays N1, N1, N2. With only an `N1`,
///   the last pass skips it.
/// - **D.C. / D.S.** jump once per chorus, after the bar that carries them, to the top or
///   the segno. After the jump repeats play once and take their last ending. "al Fine" stops
///   after the Fine bar, "al Coda" goes from the "to coda" sign to the coda (the last bar
///   with a coda sign), "al 2nd End." takes that ending.
/// - **Choruses.** The coda and the Fine are the end of the song: before the last chorus
///   the form goes back to the top where the coda would start (or after the Fine). Without
///   a D.C./D.S., the "to coda" sign is taken on the last chorus at the last pass of its
///   repeat. `U` stops the song (restarts the form before the last chorus).
/// - The result never passes [`MAX_BARS`] bars and every jump costs a step from a budget,
///   so no chart can loop forever.
pub fn expand(chart: &Chart, choruses: u32) -> Vec<Bar> {
    let bars = &chart.bars;
    let n = bars.len();
    let mut out = Vec::new();
    if n == 0 {
        return out;
    }
    let choruses = choruses.clamp(1, 1000);
    let (gs, group_of) = groups(bars);
    let codas: Vec<usize> = (0..n).filter(|&i| bars[i].coda_before || bars[i].coda_after).collect();
    let dest = if codas.len() >= 2 { codas.last().copied() } else { None };
    let segno = bars.iter().position(|b| b.segno).unwrap_or(0);
    let has_jump = chart.has_jump();

    let mut chorus = 1;
    let mut i = 0;
    let mut jumped: Option<JumpTo> = None;
    let mut passes = vec![0u32; gs.len()];
    let mut came_back = false;
    let mut budget = MAX_BARS * 4;

    // The group whose repeat span holds bar `i`.
    let span_of = |i: usize| gs.iter().rposition(|g| g.start <= i && g.ends.last().is_some_and(|&e| i <= e));
    while out.len() < MAX_BARS && budget > 0 {
        budget -= 1;
        let last = chorus == choruses;
        macro_rules! restart_or_stop {
            () => {{
                if last {
                    break;
                }
                chorus += 1;
                i = 0;
                jumped = None;
                passes.iter_mut().for_each(|p| *p = 0);
                came_back = false;
                continue;
            }};
        }
        if i >= n {
            restart_or_stop!();
        }
        let b = &bars[i];
        let final_pass = |i: usize, passes: &[u32]| span_of(i).is_none_or(|g| passes[g].max(1) >= gs[g].total);
        let jm = jumped;
        let take_coda = |i: usize, passes: &[u32]| {
            dest.is_some_and(|d| d != i)
                && (jm == Some(JumpTo::Coda) || (!has_jump && last && final_pass(i, passes)))
        };
        if Some(i) == dest && !last {
            restart_or_stop!();
        }
        if b.coda_before && take_coda(i, &passes) {
            i = dest.unwrap_or(i);
            continue;
        }
        if let Some(g) = group_of[i] {
            let gr = &gs[g];
            if gr.start == i {
                if came_back {
                    came_back = false;
                } else {
                    passes[g] = 1;
                }
            }
            if let Some(k) = b.ending {
                let p = passes[g].max(1);
                let lastp = if gr.max_ending >= 2 { gr.max_ending } else { 0 };
                let want = match jumped {
                    Some(JumpTo::Ending(e)) => e,
                    Some(_) => lastp,
                    None if p >= gr.total => lastp,
                    None => p.min(gr.max_ending.saturating_sub(1) as u32).max(1) as u8,
                };
                if k != want {
                    let after = gr.ends.last().map_or(i + 1, |e| e + 1).max(i + 1);
                    // Only ever forward, so a malformed chart can't loop on its endings.
                    i = gr.endings.iter().find(|e| e.0 == want && e.1 > i).map_or(after, |e| e.1);
                    continue;
                }
            }
        }
        came_back = false;
        out.push(Bar { time: b.time, section: b.section, section_start: b.section_start, chords: b.chords.clone(), source: i, chorus });

        if (b.fine && jumped == Some(JumpTo::Fine)) || (b.end && (jumped.is_some() || !has_jump)) {
            restart_or_stop!();
        }
        if b.coda_after && take_coda(i, &passes) {
            i = dest.unwrap_or(i);
            continue;
        }
        if b.repeat_end
            && jumped.is_none()
            && let Some(g) = group_of[i]
            && passes[g].max(1) < gs[g].total
        {
            passes[g] = passes[g].max(1) + 1;
            came_back = true;
            i = gs[g].start;
            continue;
        }
        if let (Some(j), None) = (b.jump, jumped) {
            jumped = Some(j.to);
            i = match j.from {
                JumpFrom::Capo => 0,
                JumpFrom::Segno => segno,
            };
            continue;
        }
        i += 1;
    }
    out
}

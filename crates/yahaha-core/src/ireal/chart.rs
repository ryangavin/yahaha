//! The iReal Pro chart language: tokens, then the written bars with their chords on beats.
//!
//! The token set follows iReal Pro's own public description of its format
//! (<https://www.irealpro.com/ireal-pro-file-format>). Nothing here fails: unknown
//! characters are skipped, so any string gives some (possibly empty) chart.

use super::chords::{longest_known_prefix, to_chord};
use crate::theory::{CANCEL, Chord};

/// One lexical element of a chart.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// `T44`, `T34`, `T68`, ... (`T12` is 12/8).
    TimeSig(u8, u8),
    /// `|`
    Bar,
    /// `[`
    DoubleOpen,
    /// `]`
    DoubleClose,
    /// `{`
    RepeatOpen,
    /// `}`
    RepeatClose,
    /// `Z`, the final double bar.
    Final,
    /// `*A`, `*B`, `*C`, `*D`, `*V` (verse), `*i` (intro).
    Section(char),
    /// `N1`, `N2`, `N3`: the bar starts that numbered ending.
    Ending(u8),
    /// `S`
    Segno,
    /// `Q`: the coda sign, both the "to coda" point and the coda itself.
    Coda,
    /// `f`
    Fermata,
    /// `<...>`: free text, e.g. `D.S. al Coda`, `Fine`, `3x`. A leading `*nn` (vertical
    /// placement) is stripped.
    Comment(String),
    /// `x`: repeat the previous bar.
    RepeatBar,
    /// `r`: repeat the previous two bars.
    RepeatTwoBars,
    /// `n`: N.C.
    NoChord,
    /// `p`: a slash, the previous chord again.
    Slash,
    Chord(ChartChord),
    /// `(...)`: alternate chords shown above the chord before them.
    Alternate(Vec<ChartChord>),
    /// One empty cell.
    Space,
    /// `,`: separates chords without taking a cell.
    Comma,
    /// `s`: small chords from here.
    Small,
    /// `l`: large chords from here.
    Large,
    /// `Y`: extra vertical space.
    VSpace,
    /// `U`: the end, where the player stops.
    End,
}

/// A chord symbol as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChartChord {
    /// Pitch class; `None` for `W`, the invisible root (the previous chord's root).
    pub root: Option<u8>,
    /// The quality as written (`-7`, `^7#11`, ...), or a custom `*...*` quality's text.
    pub quality: String,
    pub bass: Option<u8>,
}

/// One cell of a written bar.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Empty,
    Chord { chord: ChartChord, alt: Vec<ChartChord> },
    NoChord,
    Slash,
}

/// A chord placed on a beat of a bar (0-based). N.C. is a chord of type `theory::CANCEL`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BeatChord {
    pub beat: u8,
    pub chord: Chord,
    /// The first alternate chord written above it, if any.
    pub alt: Option<Chord>,
}

/// Where a D.C. / D.S. jumps from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpFrom {
    /// D.C.: back to the top.
    Capo,
    /// D.S.: back to the segno.
    Segno,
}

/// Where a D.C. / D.S. pass ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpTo {
    /// "al Coda": at the "to coda" sign, go to the coda.
    Coda,
    /// "al Fine": stop at Fine.
    Fine,
    /// "al 1st/2nd/3rd End.": take that ending.
    Ending(u8),
    /// No "al ...": play on to the end.
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Jump {
    pub from: JumpFrom,
    pub to: JumpTo,
}

/// One bar as written, with its chords resolved and placed on beats.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartBar {
    /// (beats, beat unit). 4/4 until a `T` token says otherwise.
    pub time: (u8, u8),
    /// The section (`A`, `B`, `V`, `i`, ...) this bar is in.
    pub section: Option<char>,
    /// This bar carries the section mark.
    pub section_start: bool,
    pub cells: Vec<Cell>,
    /// The chords on beats (after resolving `x`, `r`, `W` and the cell layout).
    pub chords: Vec<BeatChord>,
    pub repeat_start: bool,
    pub repeat_end: bool,
    pub double_start: bool,
    pub double_end: bool,
    pub final_bar: bool,
    pub ending: Option<u8>,
    pub segno: bool,
    /// The coda sign sits before the bar's first chord.
    pub coda_before: bool,
    /// The coda sign sits after a chord (the jump comes at the end of the bar).
    pub coda_after: bool,
    pub fermata: bool,
    /// `U`: the player stops after this bar.
    pub end: bool,
    /// A `Fine` comment.
    pub fine: bool,
    pub jump: Option<Jump>,
    /// A `3x`-style comment: how many times the enclosing repeat plays.
    pub repeat_count: Option<u32>,
    pub comments: Vec<String>,
    /// 1 for `x` (repeat one bar), 2 for `r` (repeat two bars).
    pub bar_repeat: u8,
}

impl Default for ChartBar {
    fn default() -> Self {
        ChartBar {
            time: (4, 4),
            section: None,
            section_start: false,
            cells: Vec::new(),
            chords: Vec::new(),
            repeat_start: false,
            repeat_end: false,
            double_start: false,
            double_end: false,
            final_bar: false,
            ending: None,
            segno: false,
            coda_before: false,
            coda_after: false,
            fermata: false,
            end: false,
            fine: false,
            jump: None,
            repeat_count: None,
            comments: Vec::new(),
            bar_repeat: 0,
        }
    }
}

impl ChartBar {
    fn has_chord_cells(&self) -> bool {
        self.cells.iter().any(|c| !matches!(c, Cell::Empty))
    }
}

/// A parsed chart: the bars in written order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Chart {
    pub bars: Vec<ChartBar>,
}

impl Chart {
    /// Parse plain (unscrambled) chart text.
    pub fn parse(s: &str) -> Chart {
        parse_chart(s)
    }

    /// The chart has a D.C. or D.S.
    pub fn has_jump(&self) -> bool {
        self.bars.iter().any(|b| b.jump.is_some())
    }
}

fn note(c: char) -> Option<u8> {
    Some(match c {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    })
}

/// A note name at `s[*i..]` (letter plus optional `b`/`#`) as a pitch class.
fn read_note(s: &[char], i: &mut usize) -> Option<u8> {
    let pc = note(*s.get(*i)?)?;
    *i += 1;
    Some(match s.get(*i) {
        Some('b') => {
            *i += 1;
            (pc + 11) % 12
        }
        Some('#') => {
            *i += 1;
            (pc + 1) % 12
        }
        _ => pc,
    })
}

/// A chord symbol at `s[*i..]`, which starts with a note letter or `W`.
fn read_chord(s: &[char], i: &mut usize) -> Option<ChartChord> {
    let root = if s.get(*i) == Some(&'W') {
        *i += 1;
        None
    } else {
        Some(read_note(s, i)?)
    };
    let mut quality = String::new();
    if s.get(*i) == Some(&'*') {
        // Custom quality: *text*
        *i += 1;
        while let Some(&c) = s.get(*i) {
            *i += 1;
            if c == '*' {
                break;
            }
            quality.push(c);
        }
    } else {
        let rest: String = s[*i..].iter().take(12).collect();
        let known = longest_known_prefix(&rest);
        quality.push_str(known);
        *i += known.chars().count();
        // An unlisted stack like 7b9b13: keep the extra tension characters.
        while let Some(&c) = s.get(*i) {
            if !(c.is_ascii_digit() || matches!(c, 'b' | '#' | '^' | '+' | '-')) {
                break;
            }
            quality.push(c);
            *i += 1;
        }
    }
    let mut bass = None;
    if s.get(*i) == Some(&'/') {
        *i += 1;
        bass = read_note(s, i);
    }
    Some(ChartChord { root, quality, bass })
}

fn strip_placement(t: &str) -> String {
    let t = t.trim_start();
    let b = t.as_bytes();
    let t = if b.len() >= 3 && b[0] == b'*' && b[1].is_ascii_digit() && b[2].is_ascii_digit() { &t[3..] } else { t };
    t.trim().to_string()
}

/// Split a chart into tokens. Unknown characters are skipped.
pub fn tokenize(chart: &str) -> Vec<Token> {
    let s: Vec<char> = chart.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        let next = s.get(i + 1).copied();
        let tok = match c {
            'T' => {
                let (a, b) = (next.and_then(|c| c.to_digit(10)), s.get(i + 2).and_then(|c| c.to_digit(10)));
                if let (Some(a), Some(b)) = (a, b) {
                    i += 3;
                    let ts = match (a, b) {
                        (1, 2) => (12, 8),
                        (a, b) => (a as u8, b as u8),
                    };
                    if ts.0 > 0 && matches!(ts.1, 1 | 2 | 4 | 8) {
                        out.push(Token::TimeSig(ts.0, ts.1));
                    }
                } else {
                    i += 1;
                }
                continue;
            }
            '*' => {
                i += 1;
                if let Some(m) = next.filter(|c| c.is_ascii_alphanumeric()) {
                    i += 1;
                    out.push(Token::Section(m));
                }
                continue;
            }
            'N' => {
                i += 1;
                if let Some(d) = next.and_then(|c| c.to_digit(10)) {
                    i += 1;
                    if d > 0 {
                        out.push(Token::Ending(d as u8));
                    }
                }
                continue;
            }
            '<' => {
                let start = i + 1;
                let end = s[start..].iter().position(|&c| c == '>').map_or(s.len(), |p| start + p);
                out.push(Token::Comment(strip_placement(&s[start..end].iter().collect::<String>())));
                i = end + 1;
                continue;
            }
            '(' => {
                let start = i + 1;
                let end = s[start..].iter().position(|&c| c == ')').map_or(s.len(), |p| start + p);
                let mut alts = Vec::new();
                let mut j = start;
                while j < end {
                    if note(s[j]).is_some() || s[j] == 'W' {
                        let before = j;
                        match read_chord(&s[..end], &mut j) {
                            Some(ch) => alts.push(ch),
                            None => j = before + 1,
                        }
                    } else {
                        j += 1;
                    }
                }
                out.push(Token::Alternate(alts));
                i = end + 1;
                continue;
            }
            'A'..='G' | 'W' => {
                let before = i;
                match read_chord(&s, &mut i) {
                    Some(ch) => out.push(Token::Chord(ch)),
                    None => i = before + 1,
                }
                continue;
            }
            '|' => Token::Bar,
            '[' => Token::DoubleOpen,
            ']' => Token::DoubleClose,
            '{' => Token::RepeatOpen,
            '}' => Token::RepeatClose,
            'Z' => Token::Final,
            'S' => Token::Segno,
            'Q' => Token::Coda,
            'f' => Token::Fermata,
            'x' => Token::RepeatBar,
            'r' => Token::RepeatTwoBars,
            'n' => Token::NoChord,
            'p' => Token::Slash,
            ' ' => Token::Space,
            ',' => Token::Comma,
            's' => Token::Small,
            'l' => Token::Large,
            'Y' => Token::VSpace,
            'U' => Token::End,
            _ => {
                i += 1;
                continue;
            }
        };
        out.push(tok);
        i += 1;
    }
    out
}

/// Read a comment's navigation meaning into the bar.
fn read_comment(t: &str, bar: &mut ChartBar) {
    let l = t.to_lowercase();
    let ds = l.contains("d.s.") || l.contains("dal segno") || l.starts_with("ds ");
    let dc = l.contains("d.c.") || l.contains("da capo") || l.starts_with("dc ");
    if ds || dc {
        let to = if l.contains("coda") {
            JumpTo::Coda
        } else if l.contains("fine") {
            JumpTo::Fine
        } else if l.contains("1st") {
            JumpTo::Ending(1)
        } else if l.contains("2nd") {
            JumpTo::Ending(2)
        } else if l.contains("3rd") {
            JumpTo::Ending(3)
        } else {
            JumpTo::End
        };
        bar.jump = Some(Jump { from: if ds { JumpFrom::Segno } else { JumpFrom::Capo }, to });
    } else if l.trim() == "fine" || l.starts_with("fine ") {
        bar.fine = true;
    }
    // "3x", "4 x", "8x's": a number followed by x.
    let b = l.as_bytes();
    for (p, _) in l.match_indices('x') {
        let mut j = p;
        while j > 0 && b[j - 1] == b' ' {
            j -= 1;
        }
        let end = j;
        while j > 0 && b[j - 1].is_ascii_digit() {
            j -= 1;
        }
        let letter_after = b.get(p + 1).is_some_and(|c| c.is_ascii_alphabetic());
        if j < end
            && !letter_after
            && (j == 0 || !b[j - 1].is_ascii_alphanumeric())
            && let Ok(n) = l[j..end].parse::<u32>()
        {
            bar.repeat_count = Some(n.clamp(1, 99));
            break;
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Close {
    Bar,
    Double,
    Repeat,
    Final,
}

struct Builder {
    bars: Vec<ChartBar>,
    cur: ChartBar,
    content: bool,
    began: bool,
    time: (u8, u8),
    section: Option<char>,
}

impl Builder {
    fn emit(&mut self) {
        let mut b = std::mem::take(&mut self.cur);
        b.time = self.time;
        b.section = self.section;
        self.bars.push(b);
        self.content = false;
    }

    fn open(&mut self, repeat: bool) {
        if self.content {
            self.emit();
        }
        self.cur.cells.clear();
        if repeat {
            self.cur.repeat_start = true;
        } else {
            self.cur.double_start = true;
        }
        self.began = true;
    }

    fn close(&mut self, kind: Close) {
        let set = |b: &mut ChartBar| match kind {
            Close::Bar => {}
            Close::Double => b.double_end = true,
            Close::Repeat => b.repeat_end = true,
            Close::Final => b.final_bar = true,
        };
        if self.content || (self.began && !self.cur.cells.is_empty()) {
            set(&mut self.cur);
            self.emit();
        } else {
            // Nothing in this bar: the bar line (and any end-of-bar marks) belong to
            // the bar before. Start-of-bar marks (section, time, ending, segno) wait.
            self.cur.cells.clear();
            self.hand_back();
            if let Some(last) = self.bars.last_mut() {
                set(last);
            }
        }
        self.began = kind == Close::Bar;
    }

    /// Move end-of-bar marks from the (empty) current bar to the last bar.
    fn hand_back(&mut self) {
        let Some(last) = self.bars.last_mut() else { return };
        let c = &mut self.cur;
        last.comments.append(&mut c.comments);
        last.fermata |= std::mem::take(&mut c.fermata);
        last.end |= std::mem::take(&mut c.end);
        last.fine |= std::mem::take(&mut c.fine);
        if std::mem::take(&mut c.coda_before) {
            last.coda_after = true;
        }
        if let Some(j) = c.jump.take() {
            last.jump = Some(j);
        }
        if let Some(n) = c.repeat_count.take() {
            last.repeat_count = Some(n);
        }
    }
}

/// Parse plain chart text into written bars with their chords on beats.
///
/// **Bars.** `|`, `]`, `}` and `Z` close a bar; `[` and `{` open one. A bar is kept when it
/// has a chord, N.C., slash or repeat sign, or when it is only empty cells between bar lines
/// (a bar that holds the previous chord). Empty cells after `]`, `}` or `Z` are row padding.
///
/// **Beats (iReal's cell layout).** Each chord, N.C., slash and space is one cell; commas,
/// alternates and markers take none. With `n` cells in a bar of `B` beats, the chord in cell
/// `i` lands on beat `i` when `n` is 4 (iReal's standard bar width) and `B <= 4`, and on beat
/// `floor(i * B / n)` otherwise. So `C^7 A-7 ` puts A-7 on beat 3, `C,D,E,F` is one chord per
/// beat, and in 3/4 the fourth cell falls on beat 3. A chord that would share a beat with the
/// one before moves to the next beat; one that runs out of beats replaces the bar's last chord.
///
/// **Repeats and invisible roots.** `x` copies the previous bar's chords; `r` copies the two
/// before it into this bar and the next (when the next is empty). `W` takes the previous
/// chord's root (and its type too when no quality is written): `W/E` is the last chord over E.
/// A slash (`p`) takes a cell but places nothing: the chord just goes on.
pub fn parse_chart(chart: &str) -> Chart {
    let mut b = Builder { bars: Vec::new(), cur: ChartBar::default(), content: false, began: false, time: (4, 4), section: None };
    for t in tokenize(chart) {
        match t {
            Token::TimeSig(n, d) => b.time = (n, d),
            Token::Section(c) => {
                b.section = Some(c);
                b.cur.section_start = true;
            }
            Token::Ending(k) => b.cur.ending = Some(k),
            Token::Segno => b.cur.segno = true,
            Token::Coda => {
                if b.cur.has_chord_cells() || b.cur.bar_repeat > 0 {
                    b.cur.coda_after = true;
                } else {
                    b.cur.coda_before = true;
                }
            }
            Token::Fermata => b.cur.fermata = true,
            Token::End => b.cur.end = true,
            Token::Comment(text) => {
                read_comment(&text, &mut b.cur);
                b.cur.comments.push(text);
            }
            Token::RepeatBar | Token::RepeatTwoBars => {
                b.cur.bar_repeat = if t == Token::RepeatBar { 1 } else { 2 };
                b.content = true;
            }
            Token::NoChord => {
                b.cur.cells.push(Cell::NoChord);
                b.content = true;
            }
            Token::Slash => {
                b.cur.cells.push(Cell::Slash);
                b.content = true;
            }
            Token::Chord(chord) => {
                b.cur.cells.push(Cell::Chord { chord, alt: Vec::new() });
                b.content = true;
            }
            Token::Alternate(v) => {
                if let Some(Cell::Chord { alt, .. }) = b.cur.cells.iter_mut().rev().find(|c| matches!(c, Cell::Chord { .. })) {
                    alt.extend(v);
                }
            }
            Token::Space => b.cur.cells.push(Cell::Empty),
            Token::Comma | Token::Small | Token::Large | Token::VSpace => {}
            Token::Bar => b.close(Close::Bar),
            Token::DoubleOpen => b.open(false),
            Token::RepeatOpen => b.open(true),
            Token::DoubleClose => b.close(Close::Double),
            Token::RepeatClose => b.close(Close::Repeat),
            Token::Final => b.close(Close::Final),
        }
    }
    if b.content {
        b.emit();
    } else {
        b.hand_back();
    }
    let mut bars = b.bars;
    place_all(&mut bars);
    Chart { bars }
}

fn resolve(c: &ChartChord, prev: Option<Chord>) -> Option<Chord> {
    match c.root {
        Some(r) => Some(to_chord(r, &c.quality, c.bass)),
        None => {
            let p = prev?;
            let mut ch = if c.quality.is_empty() { Chord { bass: None, ..p } } else { to_chord(p.root, &c.quality, None) };
            ch.bass = c.bass.filter(|&b| b != ch.root);
            Some(ch)
        }
    }
}

fn place(cells: &[Cell], time: (u8, u8), prev: &mut Option<Chord>) -> Vec<BeatChord> {
    let n = cells.len().max(1);
    let beats = time.0.max(1) as usize;
    let mut out: Vec<BeatChord> = Vec::new();
    for (i, cell) in cells.iter().enumerate() {
        let (chord, alt) = match cell {
            Cell::Chord { chord, alt } => match resolve(chord, *prev) {
                Some(c) => (c, alt.first().and_then(|a| resolve(a, Some(c)))),
                None => continue,
            },
            Cell::NoChord => (Chord::new(0, CANCEL), None),
            Cell::Empty | Cell::Slash => continue,
        };
        let mut beat = if n == 4 && beats <= 4 { i.min(beats - 1) } else { i * beats / n };
        if let Some(last) = out.last() {
            beat = beat.max(last.beat as usize + 1);
        }
        if beat >= beats {
            beat = beats - 1;
            if out.last().is_some_and(|l| l.beat as usize == beat) {
                out.pop();
            }
        }
        out.push(BeatChord { beat: beat as u8, chord, alt });
        if chord.ty != CANCEL {
            *prev = Some(chord);
        }
    }
    out
}

fn place_all(bars: &mut [ChartBar]) {
    let mut prev: Option<Chord> = None;
    let mut pending_r: Option<Vec<BeatChord>> = None;
    for i in 0..bars.len() {
        let carry = pending_r.take();
        let chords = match bars[i].bar_repeat {
            1 => i.checked_sub(1).map(|p| bars[p].chords.clone()).unwrap_or_default(),
            2 => {
                pending_r = i.checked_sub(1).map(|p| bars[p].chords.clone());
                i.checked_sub(2).map(|p| bars[p].chords.clone()).unwrap_or_default()
            }
            _ => match carry {
                Some(c) if !bars[i].has_chord_cells() => c,
                _ => place(&bars[i].cells, bars[i].time, &mut prev),
            },
        };
        if let Some(last) = chords.iter().rev().find(|c| c.chord.ty != CANCEL) {
            prev = Some(last.chord);
        }
        bars[i].chords = chords;
    }
}

//! iReal Pro charts: `irealb://` / `irealbook://` playlists, the chart language, form
//! expansion into a linear list of bars, and iReal chord qualities as yahaha chords.
//!
//! Pure library code: nothing here touches the engine, the session or the UI. See
//! docs/ireal.md for the format notes and the chord table.
//!
//! ```text
//! parse(text) -> Vec<Playlist>            a link, or an HTML page / file holding links
//!   Playlist { name, songs: Vec<Song> }
//!   Song::chart() -> Chart                 written bars, chords on beats
//!   Song::bars(choruses) -> Vec<Bar>       the form played through, linear
//! ```

mod chart;
mod chords;
mod form;
mod scramble;
mod styles;
#[cfg(test)]
mod tests;

pub use chart::{
    BeatChord, Cell, Chart, ChartBar, ChartChord, Jump, JumpFrom, JumpTo, Token, parse_chart, tokenize,
};
pub use chords::{Fit, QUALITIES, Quality, fallback_type, map_quality, to_chord};
pub use form::{Bar, MAX_BARS, expand};
pub use scramble::{MUSIC_PREFIX, scramble, unscramble};
pub use styles::{Candidate, STYLE_WORDS, style_words, suggest_style};

use anyhow::{Result, bail};

/// One song of a playlist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Song {
    pub title: String,
    /// As iReal stores it, usually "Last First".
    pub composer: String,
    /// The chart's style label ("Medium Swing", "Bossa Nova", ...).
    pub style: String,
    /// Key as written: "C", "Eb", "A-" (minor).
    pub key: String,
    /// The user's transposition in semitones (not applied to the chart).
    pub transpose: i8,
    /// iReal's playback style ("Jazz-Medium Swing"), empty when the song uses the default.
    pub groove: String,
    /// Tempo in BPM, 0 when the link doesn't say.
    pub tempo: u16,
    /// Choruses iReal plays, 3 when the link doesn't say.
    pub repeats: u32,
    /// The chart, unscrambled.
    pub chart: String,
}

impl Song {
    /// The written chart.
    pub fn chart(&self) -> Chart {
        parse_chart(&self.chart)
    }

    /// The form played `choruses` times through (see [`expand`]).
    pub fn bars(&self, choruses: u32) -> Vec<Bar> {
        expand(&self.chart(), choruses)
    }

    /// The key's tonic as a pitch class, and whether it is minor.
    pub fn key_root(&self) -> Option<(u8, bool)> {
        let t: Vec<char> = self.key.trim().chars().collect();
        let base = match t.first()? {
            'C' => 0,
            'D' => 2,
            'E' => 4,
            'F' => 5,
            'G' => 7,
            'A' => 9,
            'B' => 11,
            _ => return None,
        };
        let (pc, rest) = match t.get(1) {
            Some('b') => ((base + 11) % 12, 2),
            Some('#') => ((base + 1) % 12, 2),
            _ => (base, 1),
        };
        Some((pc, t.get(rest) == Some(&'-')))
    }
}

/// A playlist link: its songs and, for a multi-song link, the playlist name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Playlist {
    pub name: Option<String>,
    pub songs: Vec<Song>,
}

/// Percent-decode (`%3D` -> `=`); `+` stays `+`. Bad escapes are kept as written and
/// invalid UTF-8 is replaced.
pub fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = |c: u8| (c as char).to_digit(16);
        if b[i] == b'%'
            && i + 2 < b.len()
            && let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2]))
        {
            out.push((h * 16 + l) as u8);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn html_unescape(s: &str) -> String {
    s.replace("&quot;", "\"").replace("&#39;", "'").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

/// Every playlist in `text`: a bare `irealb://` / `irealbook://` link, or an HTML page (or
/// any text) with such links in it. Fails only when there is no link at all.
pub fn parse(text: &str) -> Result<Vec<Playlist>> {
    // A bare link may hold unencoded spaces ("irealbook://Title=Last First=...").
    let t = text.trim();
    if (t.starts_with("irealb://") || t.starts_with("irealbook://")) && !t[1..].contains("ireal") {
        return Ok(vec![parse_url(t)?]);
    }
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(p) = ["irealb://", "irealbook://"].iter().filter_map(|s| text[at..].find(s)).min() {
        let start = at + p;
        let len = text[start..]
            .find(|c: char| matches!(c, '"' | '\'' | '<' | '>') || c.is_whitespace())
            .unwrap_or(text.len() - start);
        out.push(parse_url(&html_unescape(&text[start..start + len]))?);
        at = start + len.max(1);
    }
    if out.is_empty() {
        bail!("no irealb:// or irealbook:// link found");
    }
    Ok(out)
}

/// One `irealb://` (scrambled, current) or `irealbook://` (plain, old) link.
pub fn parse_url(url: &str) -> Result<Playlist> {
    if let Some(rest) = url.strip_prefix("irealb://") {
        Ok(parse_irealb(&percent_decode(rest)))
    } else if let Some(rest) = url.strip_prefix("irealbook://") {
        Ok(parse_irealbook(&percent_decode(rest)))
    } else {
        bail!("not an irealb:// or irealbook:// link")
    }
}

/// `irealb://`: songs are `Title=Composer=(unused)=Style=Key=Transpose=Music=Groove=Tempo=Repeats`,
/// joined by `===`, with the playlist name after the last one. Fields can be empty, so a
/// plain split on `===` can cut a song in two; instead each song is found by its music
/// field (the one starting with `1r34LbKcu7`) and read around it.
fn parse_irealb(s: &str) -> Playlist {
    let f: Vec<&str> = s.split('=').collect();
    let music: Vec<usize> = (0..f.len()).filter(|&i| f[i].starts_with(MUSIC_PREFIX)).collect();
    if music.is_empty() {
        // No scrambled chart: read it like the old format.
        return parse_irealbook(s);
    }
    let get = |i: Option<usize>| i.and_then(|i| f.get(i)).map_or("", |s| s.trim());
    let mut songs = Vec::new();
    for (n, &m) in music.iter().enumerate() {
        let next_start = music.get(n + 1).map_or(f.len(), |&nm| nm.saturating_sub(6));
        let after = |k: usize| (m + k < next_start).then_some(m + k);
        songs.push(Song {
            title: get(m.checked_sub(6)).to_string(),
            composer: get(m.checked_sub(5)).to_string(),
            style: get(m.checked_sub(3)).to_string(),
            key: get(m.checked_sub(2)).to_string(),
            transpose: get(m.checked_sub(1)).parse().unwrap_or(0),
            chart: unscramble(f[m]),
            groove: get(after(1)).to_string(),
            tempo: get(after(2)).parse().unwrap_or(0),
            repeats: get(after(3)).parse().ok().filter(|&r| r > 0).unwrap_or(3),
        });
    }
    // The playlist name follows the last song's fields and the `===` separator.
    let last = *music.last().unwrap_or(&0);
    let name = f.iter().skip(last + 4).rfind(|s| !s.trim().is_empty()).map(|s| s.trim().to_string());
    Playlist { name, songs }
}

/// `irealbook://`: `Title=Composer=Style=Key=n=Chart`, six fields per song, one after
/// the other (or joined by `===`).
fn parse_irealbook(s: &str) -> Playlist {
    let mut songs = Vec::new();
    let mut push = |f: &[&str]| {
        if f.iter().all(|x| x.trim().is_empty()) {
            return;
        }
        let g = |i: usize| f.get(i).map_or("", |s| s.trim()).to_string();
        songs.push(Song {
            title: g(0),
            composer: g(1),
            style: g(2),
            key: g(3),
            transpose: 0,
            groove: String::new(),
            tempo: 0,
            repeats: 3,
            chart: f.get(5).map_or(String::new(), |s| s.to_string()),
        });
    };
    if s.contains("===") {
        for chunk in s.split("===") {
            push(&chunk.splitn(6, '=').collect::<Vec<_>>());
        }
    } else {
        let f: Vec<&str> = s.split('=').collect();
        for chunk in f.chunks(6) {
            push(chunk);
        }
    }
    Playlist { name: None, songs }
}

//! A library style for an iReal Pro style label: "Bossa Nova" suggests a style whose name
//! or folder says bossa, "Jazz Waltz" a 3/4 jazz style. The table is mirrored in
//! docs/ireal.md ("Style suggestion").

/// iReal style words (lowercase substrings of the chart's style label or its playback
/// groove) and the words that find a matching style by name or folder, best first.
pub const STYLE_WORDS: &[(&str, &[&str])] = &[
    ("bossa", &["bossa", "brazil", "latin"]),
    ("samba", &["samba", "brazil", "latin"]),
    ("baiao", &["baiao", "brazil", "latin"]),
    ("salsa", &["salsa", "mambo", "latin"]),
    ("mambo", &["mambo", "salsa", "latin"]),
    ("cha", &["chacha", "cha cha", "latin"]),
    ("bolero", &["bolero", "rumba", "latin"]),
    ("rumba", &["rumba", "bolero", "latin"]),
    ("tango", &["tango", "latin"]),
    ("songo", &["songo", "latin"]),
    ("afro", &["afro", "6-8", "12-8", "latin"]),
    ("calypso", &["calypso", "soca", "reggae", "world"]),
    ("reggae", &["reggae", "ska", "dub"]),
    ("latin", &["latin"]),
    ("waltz", &["waltz", "3-4"]),
    ("ballad", &["ballad", "slow"]),
    ("gospel", &["gospel", "soul"]),
    ("blues", &["blues", "shuffle"]),
    ("shuffle", &["shuffle", "blues"]),
    ("second line", &["second line", "nola", "shuffle"]),
    ("funk", &["funk"]),
    ("soul", &["soul", "r&b"]),
    ("r&b", &["r&b", "soul"]),
    ("rock", &["rock"]),
    ("pop", &["pop"]),
    ("country", &["country"]),
    ("disco", &["disco"]),
    ("march", &["march"]),
    ("folk", &["folk"]),
    ("even 8", &["8beat", "8 beat", "pop", "jazz"]),
    ("bebop", &["bebop", "swing", "jazz"]),
    ("up tempo", &["swing", "jazz", "bigband", "big band"]),
    ("swing", &["swing", "jazz", "bigband", "big band"]),
    ("jazz", &["jazz", "swing"]),
];

/// A library style to rank.
#[derive(Clone, Copy, Debug)]
pub struct Candidate<'a> {
    pub id: usize,
    pub name: &'a str,
    pub folder: &'a str,
    /// Beats per bar, when the style is indexed.
    pub beats: Option<u8>,
    pub bpm: Option<f64>,
}

/// The words an iReal style label (and its groove) asks for, best first.
pub fn style_words(style: &str, groove: &str) -> Vec<&'static str> {
    let text = format!("{} {}", style, groove).to_lowercase();
    let mut out: Vec<&'static str> = Vec::new();
    for (key, words) in STYLE_WORDS {
        if text.contains(key) {
            for w in *words {
                if !out.contains(w) {
                    out.push(w);
                }
            }
        }
    }
    out
}

/// The best style for a chart's style label, beats per bar and tempo; None when no
/// style's name or folder has any of the words. Earlier words weigh more; a style in the
/// chart's metre and nearer its tempo wins a tie; then the lower id.
pub fn suggest_style<'a>(
    style: &str,
    groove: &str,
    beats: u8,
    tempo: Option<u16>,
    candidates: impl IntoIterator<Item = Candidate<'a>>,
) -> Option<usize> {
    let words = style_words(style, groove);
    if words.is_empty() {
        return None;
    }
    let n = words.len();
    let norm = |s: &str| s.to_lowercase().replace(['_', '&'], " ").replace("  ", " ");
    let mut best: Option<(u32, i64, usize)> = None;
    for c in candidates {
        let (name, folder) = (c.name.to_lowercase(), c.folder.to_lowercase());
        let (name_n, folder_n) = (norm(c.name), norm(c.folder));
        let mut score = 0u32;
        for (i, w) in words.iter().enumerate() {
            let weight = (n - i) as u32;
            let wn = norm(w);
            if name.contains(w) || name_n.contains(&wn) || name_n.replace(' ', "").contains(&wn.replace(' ', "")) {
                score += 3 * weight;
            } else if folder.contains(w) || folder_n.contains(&wn) {
                score += weight;
            }
        }
        if score == 0 {
            continue;
        }
        if c.beats == Some(beats) {
            score += 1;
        }
        let dist = match (tempo, c.bpm) {
            (Some(t), Some(b)) => (t as f64 - b).abs() as i64,
            _ => i64::MAX / 2,
        };
        let key = (score, -dist, usize::MAX - c.id);
        if best.is_none_or(|b| key > (b.0, b.1, usize::MAX - b.2)) {
            best = Some((score, -dist, c.id));
        }
    }
    best.map(|b| b.2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c<'a>(id: usize, name: &'a str, folder: &'a str, beats: u8, bpm: f64) -> Candidate<'a> {
        Candidate { id, name, folder, beats: Some(beats), bpm: Some(bpm) }
    }

    #[test]
    fn labels_find_styles_by_name_then_folder() {
        let lib = [
            c(0, "Cool Pop", "Pop&Rock", 4, 110.0),
            c(1, "BossaNova", "Latin", 4, 130.0),
            c(2, "Swing Club", "Swing&Jazz", 4, 140.0),
            c(3, "Jazz Waltz", "Swing&Jazz", 3, 150.0),
            c(4, "Samba City", "Latin", 4, 100.0),
        ];
        assert_eq!(suggest_style("Bossa Nova", "", 4, None, lib), Some(1));
        assert_eq!(suggest_style("Medium Swing", "", 4, Some(140), lib), Some(2));
        // A 3/4 jazz chart prefers the 3/4 style.
        assert_eq!(suggest_style("Jazz Waltz", "", 3, None, lib), Some(3));
        assert_eq!(suggest_style("Samba", "", 4, None, lib), Some(4));
        assert_eq!(suggest_style("Rock Pop", "", 4, None, lib), Some(0));
        assert_eq!(suggest_style("Klezmer", "", 4, None, lib), None);
        // The groove counts too.
        assert_eq!(suggest_style("", "Latin-Brazil: Bossa Acoustic", 4, None, lib), Some(1));
        // A style only its folder matches.
        assert_eq!(suggest_style("Latin", "", 4, Some(99), [c(9, "Thing", "Latin", 4, 100.0)]), Some(9));
    }

    #[test]
    fn tempo_breaks_a_tie() {
        let lib = [c(0, "Swing A", "", 4, 100.0), c(1, "Swing B", "", 4, 200.0)];
        assert_eq!(suggest_style("Up Tempo Swing", "", 4, Some(220), lib), Some(1));
        assert_eq!(suggest_style("Slow Swing", "", 4, Some(90), lib), Some(0));
    }
}

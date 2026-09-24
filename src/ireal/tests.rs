//! iReal Pro tests. Every chart here is synthetic: no real songs or playlists.

use super::*;
use crate::theory::{CANCEL, Chord};

fn enc(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

/// The first chord of each played bar, by name ("-" for a bar with none).
fn firsts(bars: &[Bar]) -> String {
    bars.iter().map(|b| b.chords.first().map_or("-".into(), |c| c.chord.name())).collect::<Vec<_>>().join(" ")
}

fn play(chart: &str, choruses: u32) -> String {
    firsts(&expand(&parse_chart(chart), choruses))
}

/// "name@beat" for each chord of each written bar.
fn beats(chart: &str) -> Vec<String> {
    parse_chart(chart)
        .bars
        .iter()
        .map(|b| b.chords.iter().map(|c| format!("{}@{}", c.chord.name(), c.beat)).collect::<Vec<_>>().join(" "))
        .collect()
}

fn cc(root: Option<u8>, quality: &str, bass: Option<u8>) -> ChartChord {
    ChartChord { root, quality: quality.into(), bass }
}

// ---------------------------------------------------------------------------
// Unscrambling
// ---------------------------------------------------------------------------

#[test]
fn scramble_round_trips() {
    let base = "*A{T44C^7 A-7 |D-7 G7 |N1E-7 A7 }N2D-7 G7 ]*B[F^7,E-7,|Bb7   |x |r|   |<D.S. al Coda>Z";
    for n in [0, 1, 10, 49, 50, 51, 52, 53, 99, 100, 101, 102, 150, 151, 152, 400] {
        let chart: String = base.chars().cycle().take(n).collect();
        let s = scramble(&chart);
        assert!(s.starts_with(MUSIC_PREFIX));
        assert_eq!(unscramble(&s), chart, "length {n}");
    }
}

#[test]
fn unscramble_permutes_50_char_blocks_and_leaves_the_tail() {
    // 52 distinct characters: one block is scrambled, the 2-char tail isn't.
    let plain: String = (0..52u8).map(|i| (b'0' + i) as char).collect();
    let p: Vec<char> = plain.chars().collect();
    let scr: String = unscramble(&plain); // the permutation is its own inverse
    let s: Vec<char> = scr.chars().collect();
    let swapped = |i: usize| i < 5 || (10..24).contains(&i) || i >= 45 || (26..40).contains(&i);
    for (i, &c) in s.iter().enumerate().take(50) {
        assert_eq!(c, p[if swapped(i) { 49 - i } else { i }], "position {i}");
    }
    assert_eq!(&s[50..], &p[50..]);
    // 51 characters or fewer are not scrambled at all.
    let short: String = plain.chars().take(51).collect();
    assert_eq!(unscramble(&short), short);
}

#[test]
fn unscramble_expands_abbreviations() {
    assert_eq!(unscramble("1r34LbKcu7C7XyQKclLZD7"), "C7   | x |D7");
}

// ---------------------------------------------------------------------------
// Playlists
// ---------------------------------------------------------------------------

const CHART1: &str = "*A{T44C^7 A-7 |D-7 G7 |N1E-7 A7 }N2D-7 G7 ]*B[F^7,E-7,D-7,C^7|B-7b5 E7b9 |A-7   |x Z";
const CHART2: &str = "[T34A-,B-7b5,E7b9, |A- |A- |E7 Z";

fn irealb_url() -> String {
    let s1 = format!("Test One=Doe John==Medium Swing=Eb=={}=Jazz-Medium Swing=160=2", scramble(CHART1));
    // Empty composer and unused field: "Test Two===Bossa Nova" would fool a split on "===".
    let s2 = format!("Test Two===Bossa Nova=A-=2={}==0=", scramble(CHART2));
    format!("irealb://{}", enc(&format!("{s1}==={s2}===My List")))
}

#[test]
fn decodes_irealb_playlists() {
    let lists = parse(&irealb_url()).unwrap();
    assert_eq!(lists.len(), 1);
    let pl = &lists[0];
    assert_eq!(pl.name.as_deref(), Some("My List"));
    assert_eq!(pl.songs.len(), 2);
    let a = &pl.songs[0];
    assert_eq!((a.title.as_str(), a.composer.as_str(), a.style.as_str(), a.key.as_str()), ("Test One", "Doe John", "Medium Swing", "Eb"));
    assert_eq!((a.transpose, a.groove.as_str(), a.tempo, a.repeats), (0, "Jazz-Medium Swing", 160, 2));
    assert_eq!(a.chart, CHART1);
    assert_eq!(a.key_root(), Some((3, false)));
    let b = &pl.songs[1];
    assert_eq!((b.title.as_str(), b.composer.as_str(), b.style.as_str(), b.key.as_str()), ("Test Two", "", "Bossa Nova", "A-"));
    assert_eq!((b.transpose, b.groove.as_str(), b.tempo, b.repeats), (2, "", 0, 3));
    assert_eq!(b.chart, CHART2);
    assert_eq!(b.key_root(), Some((9, true)));
    assert_eq!(b.bars(1).iter().map(|x| x.time).collect::<Vec<_>>(), vec![(3, 4); 4]);
}

#[test]
fn single_song_link_has_no_name() {
    let url = format!("irealb://{}", enc(&format!("Solo=Roe Jane==Waltz=F=={}=Jazz-Waltz=120=1", scramble("T34F^7 |C7 Z"))));
    let pl = parse_url(&url).unwrap();
    assert_eq!(pl.name, None);
    assert_eq!(pl.songs[0].tempo, 120);
    assert_eq!(pl.songs[0].repeats, 1);
    assert_eq!(play(&pl.songs[0].chart, 1), "Fmaj7 C7");
}

#[test]
fn decodes_irealbook_links_and_html() {
    let old = format!("irealbook://{}", enc("One=Doe John=Ballad=C=n=C |D Z=Two=Roe Jane=Waltz=F=n=T34F |C Z"));
    let html = format!("<html><body><a href=\"{}\">list</a>\n<a href='{old}'>old</a></body></html>", irealb_url().replace('&', "&amp;"));
    let lists = parse(&html).unwrap();
    assert_eq!(lists.len(), 2);
    assert_eq!(lists[0].songs.len(), 2);
    let o = &lists[1];
    assert_eq!(o.songs.len(), 2);
    assert_eq!((o.songs[0].title.as_str(), o.songs[0].composer.as_str(), o.songs[0].style.as_str(), o.songs[0].key.as_str()), ("One", "Doe John", "Ballad", "C"));
    assert_eq!(o.songs[1].chart, "T34F |C Z");
    assert_eq!(play(&o.songs[1].chart, 1), "F C");
    // A bare link may carry raw spaces.
    let bare = parse("  irealbook://Raw Title=Doe John=Ballad=C=n=C D |E Z\n").unwrap();
    assert_eq!((bare[0].songs[0].title.as_str(), bare[0].songs[0].composer.as_str()), ("Raw Title", "Doe John"));
    assert_eq!(play(&bare[0].songs[0].chart, 1), "C E");
    // "===" between irealbook songs works too.
    let sep = parse_url("irealbook://A=B=C=D=n=E7 Z===F=G=H=I=n=G7 Z").unwrap();
    assert_eq!(sep.songs.iter().map(|s| s.chart.as_str()).collect::<Vec<_>>(), ["E7 Z", "G7 Z"]);
}

#[test]
fn percent_decoding() {
    assert_eq!(percent_decode("a%3Db%20c+d"), "a=b c+d");
    assert_eq!(percent_decode("%E2%99%AD"), "\u{266d}");
    assert_eq!(percent_decode("%zz%4"), "%zz%4");
    assert!(parse("no link here").is_err());
    assert!(parse_url("http://x").is_err());
}

// ---------------------------------------------------------------------------
// Tokens
// ---------------------------------------------------------------------------

#[test]
fn tokenizes_every_token_type() {
    use Token::*;
    let t = tokenize("T44*A{C^7 A-7,|N1x}N2r|n p W/E sC7 lF7(Db7)YS Q f U <*72 3x>Z]*i[T12*V[T68");
    let c = |r, q: &str| Chord(cc(Some(r), q, None));
    assert_eq!(
        t,
        vec![
            TimeSig(4, 4), Section('A'), RepeatOpen, c(0, "^7"), Space, c(9, "-7"), Comma, Bar, Ending(1), RepeatBar,
            RepeatClose, Ending(2), RepeatTwoBars, Bar, NoChord, Space, Slash, Space, Chord(cc(None, "", Some(4))), Space,
            Small, c(0, "7"), Space, Large, c(5, "7"), Alternate(vec![cc(Some(1), "7", None)]), VSpace, Segno, Space,
            Coda, Space, Fermata, Space, End, Space, Comment("3x".into()), Final, DoubleClose, Section('i'), DoubleOpen,
            TimeSig(12, 8), Section('V'), DoubleOpen, TimeSig(6, 8),
        ]
    );
}

#[test]
fn chord_symbols() {
    let one = |s: &str| match tokenize(s).first() {
        Some(Token::Chord(c)) => c.clone(),
        t => panic!("{s}: {t:?}"),
    };
    assert_eq!(one("Bb-7/Ab"), cc(Some(10), "-7", Some(8)));
    assert_eq!(one("F#h7"), cc(Some(6), "h7", None));
    assert_eq!(one("C7b9sus"), cc(Some(0), "7b9sus", None));
    assert_eq!(one("C7#9b13"), cc(Some(0), "7#9b13", None)); // unlisted stack kept whole
    assert_eq!(one("C*m7*/G"), cc(Some(0), "m7", Some(7))); // custom quality
    assert_eq!(one("W7"), cc(None, "7", None));
    // Every table quality reads back whole, with a slash bass after it.
    for q in QUALITIES {
        assert_eq!(one(&format!("E{}/G#", q.ireal)), cc(Some(4), q.ireal, Some(8)), "{}", q.ireal);
    }
}

#[test]
fn comments_carry_navigation() {
    let bar = |s: &str| parse_chart(&format!("C {s}|")).bars[0].clone();
    let j = |s: &str| bar(s).jump;
    assert_eq!(j("<D.C. al Fine>"), Some(Jump { from: JumpFrom::Capo, to: JumpTo::Fine }));
    assert_eq!(j("<D.S. al Coda>"), Some(Jump { from: JumpFrom::Segno, to: JumpTo::Coda }));
    assert_eq!(j("<D.C. al Coda>"), Some(Jump { from: JumpFrom::Capo, to: JumpTo::Coda }));
    assert_eq!(j("<D.S. al Fine>"), Some(Jump { from: JumpFrom::Segno, to: JumpTo::Fine }));
    assert_eq!(j("<D.C. al 2nd End.>"), Some(Jump { from: JumpFrom::Capo, to: JumpTo::Ending(2) }));
    assert_eq!(j("<D.C.>"), Some(Jump { from: JumpFrom::Capo, to: JumpTo::End }));
    assert_eq!(j("<Solo>"), None);
    assert!(bar("<Fine>").fine);
    assert!(!bar("<D.C. al Fine>").fine);
    assert_eq!(bar("<3x>").repeat_count, Some(3));
    assert_eq!(bar("<Play 4 x>").repeat_count, Some(4));
    assert_eq!(bar("<8x's>").repeat_count, Some(8));
    assert_eq!(bar("<fox>").repeat_count, None);
    assert_eq!(bar("<*36 Vamp>").comments, ["Vamp"]);
}

// ---------------------------------------------------------------------------
// Bars and beats
// ---------------------------------------------------------------------------

#[test]
fn cell_layout_places_chords_on_beats() {
    assert_eq!(beats("C^7 A-7 |"), ["Cmaj7@0 Am7@2"]);
    assert_eq!(beats("C,D-,E-,F|"), ["C@0 Dm@1 Em@2 F@3"]);
    assert_eq!(beats("C^7   |"), ["Cmaj7@0"]);
    assert_eq!(beats("C^7 |"), ["Cmaj7@0"]);
    assert_eq!(beats("C D7 |"), ["C@0 D7@2"]); // 4 cells with the trailing space
    assert_eq!(beats("C D7|"), ["C@0 D7@2"]); // 3 cells: floor(2 * 4 / 3) = 2
    assert_eq!(beats("  C |"), ["C@2"]); // a chord in the third cell is on beat 3
    assert_eq!(beats("T34C D E |"), ["C@0 D@1 E@2"]);
    assert_eq!(beats("T34C,D,E, |"), ["C@0 D@1 E@2"]);
    assert_eq!(beats("T34C  D|"), ["C@0 D@2"]); // 4 cells in 3/4: cell 4 clamps to beat 3
    assert_eq!(beats("T68C  D  |"), ["C@0 D@3"]);
    assert_eq!(beats("T24C D |"), ["C@0 D@1"]);
    assert_eq!(beats("T54C  D  |"), ["C@0 D@2"]); // floor(3 * 5 / 6)
    // Too many chords: bumped to the next beat, the last one wins the last beat.
    assert_eq!(beats("C,D,E,F,G,A,B,C|"), ["C@0 D@1 E@2 C@3"]);
    // N.C., slashes, alternates.
    assert_eq!(beats("n |"), ["N.C.@0"]);
    assert_eq!(beats("C p p p |"), ["C@0"]);
    let b = &parse_chart("C7(Db7) |").bars[0].chords[0];
    assert_eq!(b.alt, Some(Chord::new(1, 19)));
}

#[test]
fn bars_and_barlines() {
    let c = parse_chart("*A{T34C |D }[E ]F Z*B[G |   |H");
    let bars = &c.bars;
    assert_eq!(bars.len(), 6); // H is not a note: the last bar is the empty one
    assert!(bars[0].repeat_start && bars[0].section_start && bars[0].time == (3, 4));
    assert!(bars[1].repeat_end && !bars[1].section_start && bars[1].section == Some('A'));
    assert!(bars[2].double_start && bars[2].double_end);
    assert!(bars[3].final_bar);
    assert!(bars[4].section_start && bars[4].section == Some('B') && bars[4].double_start);
    assert!(bars[5].chords.is_empty() && bars[5].cells == vec![Cell::Empty; 3]);
    // Row padding after ] / } / Z is not a bar; marks before [ belong to the next bar.
    let c = parse_chart("C Z   *BSN1[D |");
    assert_eq!(c.bars.len(), 2);
    assert!(c.bars[1].segno && c.bars[1].ending == Some(1) && c.bars[1].section_start);
    // End-of-bar marks after a bar line go back to that bar.
    let c = parse_chart("C |<D.C. al Fine>f]");
    assert_eq!(c.bars.len(), 1);
    assert!(c.bars[0].jump.is_some() && c.bars[0].fermata && c.bars[0].double_end);
    // Q before a bar's chords is at its start, after one at its end.
    let c = parse_chart("QC |D Q|");
    assert!(c.bars[0].coda_before && !c.bars[0].coda_after);
    assert!(c.bars[1].coda_after && !c.bars[1].coda_before);
}

#[test]
fn repeat_signs_and_invisible_roots() {
    // x repeats one bar, r two (into the empty bar after it).
    assert_eq!(play("C |x |D |E |r|   |F Z", 1), "C C D E D E F");
    assert_eq!(beats("C D |x |"), ["C@0 D@2", "C@0 D@2"]);
    assert_eq!(unscramble("C7XyQKclLZ"), "C7   | x |");
    assert_eq!(play(&unscramble("C7XyQKclLZ"), 1), "C7 C7");
    // At the top there is nothing to repeat.
    assert_eq!(play("x |r|   |C Z", 1), "- - - C");
    // W: the previous chord's root (and type when no quality is written).
    let c = parse_chart("C7 |W/E |W-7 |");
    assert_eq!(c.bars[1].chords[0].chord, Chord { root: 0, ty: 19, bass: Some(4) });
    assert_eq!(c.bars[2].chords[0].chord, Chord::new(0, 10));
    // W with nothing before it is dropped.
    assert!(parse_chart("W/E |").bars[0].chords.is_empty());
}

// ---------------------------------------------------------------------------
// Form expansion
// ---------------------------------------------------------------------------

#[test]
fn repeats_and_endings() {
    assert_eq!(play("{C |D }E Z", 1), "C D C D E");
    assert_eq!(play("{C |N1D }N2E |F Z", 1), "C D C E F");
    assert_eq!(play("{<3x>C |N1D }N2E |F Z", 1), "C D C D C E F");
    assert_eq!(play("{C |N1D }N2E }N3F |G Z", 1), "C D C E C F G");
    assert_eq!(play("{C |N1D }E Z", 1), "C D C E"); // only N1: skipped on the last pass
    assert_eq!(play("{C |D <4x>}E Z", 1), "C D C D C D C D E");
    // A } with no { repeats from the top, then from after the last }.
    assert_eq!(play("C |D }E Z", 1), "C D C D E");
    assert_eq!(play("C }D }E Z", 1), "C C D D E");
    // Two repeats in a row.
    assert_eq!(play("{C }{D }E Z", 1), "C C D D E");
    // Sections and times follow the played bars.
    let bars = expand(&parse_chart("*A{T34C |N1D }N2*BT44E Z"), 1);
    assert_eq!(bars.iter().map(|b| (b.section, b.time.0, b.source)).collect::<Vec<_>>(),
        [(Some('A'), 3, 0), (Some('A'), 3, 1), (Some('A'), 3, 0), (Some('B'), 4, 2)]);
    assert!(bars[3].section_start);
}

#[test]
fn da_capo_and_dal_segno() {
    assert_eq!(play("C |D <Fine>|E |F <D.C. al Fine>Z", 1), "C D E F C D");
    assert_eq!(play("C |SD |E Q|F <D.S. al Coda>Z Q[G |A Z", 1), "C D E F D E G A");
    assert_eq!(play("C |SD <Fine>|E <D.S. al Fine>Z", 1), "C D E D");
    assert_eq!(play("{C |D <Fine>}E |F <D.C. al Fine>Z", 1), "C D C D E F C D");
    // After the jump repeats play once and take the last ending.
    assert_eq!(play("{C |N1D }N2E |F <D.C.>Z", 1), "C D C E F C E F");
    assert_eq!(play("{C |D }E <D.C.>Z", 1), "C D C D E C D E");
    // D.C. al Coda with the "to coda" sign at the start of a bar.
    assert_eq!(play("C |QD |E <D.C. al Coda>Z Q[F Z", 1), "C D E C F");
    // D.C. al 2nd ending.
    assert_eq!(play("{C |N1D }N2E }N3F |G <D.C. al 2nd End.>Z", 1), "C D C E C F G C E G");
    // An ending never sends the player backwards.
    assert_eq!(play("{C |N1D }N2E |N3F |G Z", 1), "C D C E G");
    // D.S. with no segno goes to the top; al Coda with no coda plays to the end.
    assert_eq!(play("C |D <D.S. al Coda>|E Z", 1), "C D C D E");
}

#[test]
fn choruses_codas_and_end() {
    assert_eq!(play("C |D Z", 3), "C D C D C D");
    let bars = expand(&parse_chart("C |D Z"), 2);
    assert_eq!(bars.iter().map(|b| b.chorus).collect::<Vec<_>>(), [1, 1, 2, 2]);
    // The coda is played on the last chorus only.
    assert_eq!(play("C |SD |E Q|F <D.S. al Coda>Z Q[G |A Z", 2), "C D E F D E C D E F D E G A");
    // Without a D.S./D.C. the "to coda" is taken on the last chorus.
    assert_eq!(play("C |D Q|E |F Z Q[G Z", 2), "C D E F C D G");
    // ...at the last pass of its repeat.
    assert_eq!(play("{C |D Q}E Z Q[G Z", 1), "C D C D G");
    // Fine ends the song; before the last chorus the form restarts.
    assert_eq!(play("C |D <Fine>|E <D.C. al Fine>Z", 2), "C D E C D C D E C D");
    // U stops the player.
    assert_eq!(play("C |D U|E Z", 1), "C D");
    assert_eq!(play("C |D U|E Z", 2), "C D C D");
}

#[test]
fn expansion_is_bounded() {
    assert!(expand(&parse_chart("C |D Z"), u32::MAX).len() <= MAX_BARS);
    assert_eq!(expand(&parse_chart(""), 3), []);
    let bars = expand(&parse_chart("{C <99x>}"), 1000);
    assert!(bars.len() <= MAX_BARS && !bars.is_empty());
    // Jumps to themselves.
    for chart in ["S<D.S.>C |", "SC <D.S. al Coda>Q|Q", "{N1C }N1D }", "}}}}", "{{{{C", "N2C }N1D }", "QQQQ C Q|Q|<D.C. al Coda>"] {
        let bars = expand(&parse_chart(chart), 5);
        assert!(bars.len() <= MAX_BARS, "{chart}");
    }
}

// ---------------------------------------------------------------------------
// Chord mapping
// ---------------------------------------------------------------------------

#[test]
fn maps_every_quality() {
    use crate::theory::TYPE_NAMES;
    let named = |q: &str| {
        let c = to_chord(0, q, None);
        format!("{}{}", crate::theory::NOTE_NAMES[c.root as usize], TYPE_NAMES[c.ty as usize])
            + &c.bass.map_or(String::new(), |b| format!("/{}", crate::theory::NOTE_NAMES[b as usize]))
    };
    let want = [
        ("", "C"), ("5", "C1+5"), ("2", "Csus2"), ("add9", "Cadd9"), ("+", "Caug"), ("o", "Cdim"), ("h", "Cm7b5"),
        ("sus", "Csus4"), ("^", "Cmaj7"), ("-", "Cm"), ("^7", "Cmaj7"), ("-7", "Cm7"), ("7", "C7"), ("7sus", "C7sus4"),
        ("h7", "Cm7b5"), ("o7", "Cdim7"), ("^9", "Cmaj9"), ("^13", "Cmaj9"), ("6", "C6"), ("69", "C6/9"),
        ("^7#11", "Cmaj7#11"), ("^9#11", "Cmaj7#11"), ("^7#5", "Cmaj7#5"), ("-6", "Cm6"), ("-69", "Cm6"),
        ("-^7", "CmMaj7"), ("-^9", "CmMaj9"), ("-9", "Cm9"), ("-11", "Cm11"), ("-7b5", "Cm7b5"), ("h9", "Cm7b5"),
        ("-b6", "Cm"), ("-#5", "Cm"), ("9", "C9"), ("7b9", "C7b9"), ("7#9", "C7#9"), ("7#11", "C7#11"),
        ("7b5", "C7b5"), ("7#5", "C7#5"), ("9#11", "C7#11"), ("9b5", "C7b5"), ("9#5", "C7#5"), ("7b13", "C7b13"),
        ("7#9#5", "C7#9"), ("7#9b5", "C7#9"), ("7#9#11", "C7#9"), ("7b9#11", "C7b9"), ("7b9b5", "C7b9"),
        ("7b9#5", "C7b9"), ("7b9#9", "C7b9"), ("7b9b13", "C7b9"), ("7alt", "C7#9"), ("13", "C13"), ("13#11", "C13"),
        ("13b9", "C13"), ("13#9", "C13"), ("7b9sus", "C7sus4"), ("7susadd3", "C7sus4"), ("9sus", "C7sus4"),
        ("13sus", "C7sus4"), ("7b13sus", "C7sus4"), ("11", "C7sus4"),
    ];
    assert_eq!(want.len(), QUALITIES.len());
    for (q, name) in want {
        assert!(QUALITIES.iter().any(|x| x.ireal == q), "{q} not in the table");
        assert_eq!(named(q), name, "{q}");
        assert_ne!(map_quality(q).fit, Fit::Fallback, "{q}");
    }
    // Every type id is a real yahaha type (and not Cancel or 1+8).
    for q in QUALITIES {
        assert!((q.ty as usize) < TYPE_NAMES.len() && q.ty != CANCEL && q.ty != 30, "{}", q.ireal);
    }
}

#[test]
fn chord_mapping_details() {
    // Slash bass; a bass equal to the root is dropped.
    assert_eq!(to_chord(0, "7", Some(4)), Chord { root: 0, ty: 19, bass: Some(4) });
    assert_eq!(to_chord(0, "7", Some(0)), Chord::new(0, 19));
    // Minor #5 / b6 keep the written root (and any written bass).
    assert_eq!(to_chord(9, "-#5", Some(7)), Chord { root: 9, ty: 8, bass: Some(7) });
    assert_eq!(to_chord(0, "-b6", None), Chord::new(0, 8));
    assert_eq!(beats("C- C-#5 |C-6 C-b6 |"), ["Cm@0 Cm@2", "Cm6@0 Cm@2"]);
    // Unknown qualities fall back by spelling.
    for (q, ty) in [("7b9#13", 25), ("-7b9", 10), ("^7#9", 2), ("m7", 10), ("maj7", 2), ("dim", 17), ("+7", 29),
        ("9sus4", 20), ("7#9b13", 27), ("aug", 7), ("xyz", 0), ("-^11", 15), ("69#11", 6)] {
        let m = map_quality(q);
        assert_eq!((m.ty, m.fit), (ty, Fit::Fallback), "{q}");
    }
    // Through the chart: roots with accidentals and a slash chord.
    assert_eq!(beats("Bb-7/Ab |F#h7 |Gb^7 |"), ["Bbm7/Ab@0", "F#m7b5@0", "F#maj7@0"]);
}

// ---------------------------------------------------------------------------
// Robustness
// ---------------------------------------------------------------------------

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
}

#[test]
fn garbage_never_panics() {
    const ALPHA: &[u8] = b"ABCDEFGWb#^-+ohsuadlt0123456789/|[]{}ZTN*SQfxrnpY U<>(),.alt=%KclLZXyQ   ";
    let mut r = Lcg(7);
    for round in 0..3000 {
        let len = (r.next() % 300) as usize;
        let s: String = (0..len)
            .map(|_| if round % 5 == 0 { char::from_u32((r.next() % 0x2FF) as u32).unwrap_or('?') } else { ALPHA[(r.next() % ALPHA.len() as u64) as usize] as char })
            .collect();
        let _ = tokenize(&s);
        let chart = parse_chart(&s);
        let bars = expand(&chart, (r.next() % 4) as u32);
        assert!(bars.len() <= MAX_BARS);
        for b in &bars {
            assert!(b.chords.windows(2).all(|w| w[0].beat < w[1].beat), "{s:?}");
            assert!(b.chords.iter().all(|c| c.beat < b.time.0.max(1) && c.chord.root < 12));
        }
        let _ = unscramble(&s);
        assert_eq!(unscramble(&scramble(&s.replace(['K', 'L', 'X'], ""))), s.replace(['K', 'L', 'X'], ""));
        let _ = parse(&format!("irealb://{s}"));
        let _ = parse(&format!("<a href=\"irealbook://{s}\">"));
        let _ = parse(&format!("irealb://{}", enc(&format!("={s}=={}", scramble(&s)))));
        let _ = percent_decode(&s);
    }
}

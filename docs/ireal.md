# iReal Pro charts (`src/ireal`)

`yahaha::ireal` reads iReal Pro playlist links and turns a chart into a linear list of bars,
with yahaha chords on beats. It is a pure library module: it plays nothing and has no engine,
session or UI wiring.

```rust
use yahaha::ireal;

for list in ireal::parse(&text)? {        // a link, or an HTML page / file holding links
    for song in &list.songs {             // title, composer, style, key, tempo, repeats, chart
        for bar in song.bars(song.repeats) {
            // bar.time, bar.section, bar.chords: [BeatChord { beat, chord: theory::Chord, alt }]
        }
    }
}
```

To see what a link expands to, run `yahaha ireal <file or link> [--choruses N]`.

| Step | API | Notes |
|---|---|---|
| Links | `parse`, `parse_url`, `percent_decode` | Handles `irealb://` (scrambled, current) and `irealbook://` (plain, old). |
| Unscrambling | `unscramble`, `scramble` | Works in 50-character blocks. The source is cited in `scramble.rs`. |
| Tokens | `tokenize` → `Vec<Token>` | Never fails. Unknown characters are skipped. |
| Written bars | `parse_chart` → `Chart { bars: Vec<ChartBar> }` | Each bar has its marks, and its chords placed on beats. |
| Form | `expand(&chart, choruses)` → `Vec<Bar>` | At most `MAX_BARS` bars, and loops are guarded. |
| Chords | `QUALITIES`, `map_quality`, `to_chord`, `fallback_type` | See the table below. |

## Links

- **`irealb://`**: each song is `Title=Composer=(unused)=Style=Key=Transpose=Music=Groove=Tempo=Repeats`.
  - Songs are joined by `===`, and the playlist name comes after the last song.
  - Empty fields can produce `===` inside a song (for example `Title===Style` when there is no composer). So the parser doesn't split on `===`. It finds each song by its music field, which starts with `1r34LbKcu7`, and reads the other fields at fixed offsets from it.
  - Tempo 0 means iReal's default. Repeats defaults to 3.
- **`irealbook://`**: each song is `Title=Composer=Style=Key=n=Chart`. The songs follow one another, or are joined by `===`.
- **HTML and text files:** every `irealb://` or `irealbook://` link in the file is read. A link ends at a quote, `<`, `>` or whitespace, and `&amp;`-style entities are decoded.
- A bare link passed on its own may contain raw spaces.

## Chart tokens

| Token | Meaning |
|---|---|
| `T44` `T34` `T68` `T12` ... | Time signature. `T12` is 12/8. |
| `\|` `[` `]` `{` `}` `Z` | Bar line, double bar open/close, repeat open/close, final bar |
| `*A` `*B` `*C` `*D` `*V` `*i` | Section marks. The section carries on to later bars. |
| `N1` `N2` `N3` | This bar starts that numbered ending. |
| `S` `Q` `f` | Segno, coda, fermata |
| `<...>` | A comment. A leading `*nn` placement is stripped. The parser reads `D.C.`/`D.S.` with `al Coda`, `al Fine` or `al 1st/2nd/3rd End.`, plus `Fine` and repeat counts such as `3x`. |
| `x` / `r` | Repeat the previous bar / the previous two bars |
| `n` `p` `W` | N.C., slash, invisible root (`W/E` = the last chord over E) |
| `s` `l` `Y` `,` | Small chords, large chords, vertical space, separator. None of them take a cell. |
| `(...)` | Alternate chords, attached to the chord before them (`BeatChord::alt`) |
| `U` | End: the player stops here. |
| `Kcl` `LZ` `XyQ` | Scrambled-link abbreviations for `\| x`, ` \|` and three empty cells. They are expanded when the chart is unscrambled. |

## Placing chords on beats

iReal lays a bar out as cells.

- Each chord, N.C., slash and space takes one cell. Commas, alternates and marks take none.
- In a bar with `n` cells and `B` beats, the chord in cell `i` goes on:
  - beat `i`, when `n == 4` (iReal's standard bar width) and `B <= 4`;
  - beat `floor(i * B / n)` otherwise.
- Examples:
  - `C^7 A-7 ` puts A-7 on beat 3.
  - `C,D-,E-,F` has one chord per beat.
  - `C^7   ` is a whole bar of C.
  - In 3/4, a 4-cell bar maps cells to beats 1-2-3, and the 4th cell also goes on beat 3.
  - In 6/8, `C  D  ` puts D on beat 4.
- Collisions: a chord that lands on the same beat as the one before moves to the next beat. If no beat is left, it replaces the bar's last chord.
- Holding chords: a slash takes its cell but places nothing, so the chord carries on. A bar that is only empty cells between bar lines holds the previous chord. A bar's `chords` can be empty, and a beat with no chord holds the chord before it.

## Form expansion

- **Repeats:** a `{ }` repeat plays twice. A `3x`-style comment inside the repeat, or its highest ending number, raises the count. A `}` with no matching `{` repeats from just after the previous `}`, or from the top.
- **Endings:** on pass `p` of `t`, a pass before the last plays ending `min(p, last-1)` and the last pass plays the last ending.
  - `N1 N2` with `3x` plays N1, N1, N2.
  - With only an `N1`, the last pass skips it.
  - An ending never sends the player backwards.
- **D.C./D.S.:** jumps once per chorus, after the bar that carries it, to the top or to the segno.
  - After the jump, repeats play once and take their last ending (or the ending an "al 2nd End." names).
  - "al Fine" stops after the `Fine` bar.
  - "al Coda" jumps at the "to coda" sign to the coda. The coda is the last bar with a `Q`.
  - A `Q` before a bar's first chord jumps before that bar. A `Q` after a chord jumps after it.
- **Choruses:** the coda, `Fine` and `U` end the song.
  - Before the last chorus, the form goes back to the top where it would have ended.
  - When there is no D.C. or D.S., the "to coda" sign is taken on the last chorus, at the last pass of its repeat.
- **Guards:** the output is capped at `MAX_BARS` (10,000), and every step spends from a fixed budget. No chart can loop forever. This is fuzz-tested on random input.

## Chord mapping

Each iReal quality maps to a yahaha chord type (`theory::TYPE_NAMES`). The slash bass is kept, and dropped when it equals the root.

- **Exact** means the same notes. Tensions that yahaha's type treats as optional count as the same.
- **Approx** means the nearest yahaha type. The table says what changes.

Some rules decide the approximate choices:

- **Dominant chords with several alterations** keep the first altered tension written. This follows `7alt → 7(#9)`, so `7#9#5 → 7#9` and `7b9#11 → 7b9`.
- **13th chords with alterations** keep the 13.
- **Suspended dominants** (`9sus`, `13sus`, `7b9sus`, `7b13sus`, `11`) become `7sus4`.
- **Two minor chords with a raised 5th** are exactly a major chord over the written root, so they are re-rooted instead of approximated: `C-#5 = Ab/C` and `C-b6 = Abmaj7/C`.

| iReal | yahaha | Fit | Notes |
|---|---|---|---|
| (none) | M | exact | |
| `5` | 1+5 | exact | |
| `2` | sus2 | approx | iReal's "2" chord, read as sus2 |
| `add9` | add9 | exact | |
| `+` | aug | exact | |
| `o` | dim | exact | |
| `h` | m7b5 | exact | iReal plays ø as ø7. |
| `sus` | sus4 | exact | |
| `^` | maj7 | exact | The triangle alone is maj7. |
| `-` | m | exact | |
| `^7` | maj7 | exact | |
| `-7` | m7 | exact | |
| `7` | 7 | exact | |
| `7sus` | 7sus4 | exact | |
| `h7` | m7b5 | exact | |
| `o7` | dim7 | exact | |
| `^9` | maj9 | exact | |
| `^13` | maj9 | approx | Drops the 13. |
| `6` | 6 | exact | |
| `69` | 6/9 | exact | |
| `^7#11` | maj7#11 | exact | |
| `^9#11` | maj7#11 | approx | Drops the 9. |
| `^7#5` | maj7#5 | exact | |
| `-6` | m6 | exact | |
| `-69` | m6 | approx | Drops the 9. |
| `-^7` | mMaj7 | exact | |
| `-^9` | mMaj9 | exact | |
| `-9` | m9 | exact | |
| `-11` | m11 (m7(11)) | approx | Drops the 9. |
| `-7b5` | m7b5 | exact | |
| `h9` | m7b5 | approx | Drops the 9. |
| `-b6` | maj7, root +8, bass = root | exact (re-rooted) | C-b6 = Abmaj7/C |
| `-#5` | M, root +8, bass = root | exact (re-rooted) | C-#5 = Ab/C |
| `9` | 9 (7(9)) | exact | |
| `7b9` | 7b9 | exact | |
| `7#9` | 7#9 | exact | |
| `7#11` | 7#11 | exact | |
| `7b5` | 7b5 | exact | |
| `7#5` | 7#5 (7aug) | exact | |
| `9#11` | 7#11 | approx | Drops the 9. |
| `9b5` | 7b5 | approx | Drops the 9. |
| `9#5` | 7#5 | approx | Drops the 9. |
| `7b13` | 7b13 | exact | |
| `7#9#5` | 7#9 | approx | Drops the #5 and plays the 5th. |
| `7#9b5` | 7#9 | approx | Drops the b5 and plays the 5th. |
| `7#9#11` | 7#9 | approx | Drops the #11. |
| `7b9#11` | 7b9 | approx | Drops the #11. |
| `7b9b5` | 7b9 | approx | Drops the b5 and plays the 5th. |
| `7b9#5` | 7b9 | approx | Drops the #5 and plays the 5th. |
| `7b9#9` | 7b9 | approx | Drops the #9. |
| `7b9b13` | 7b9 | approx | Drops the b13. |
| `7alt` | 7#9 | approx | The #9 stands in for the whole altered stack. |
| `13` | 13 (7(13)) | exact | |
| `13#11` | 13 | approx | Drops the #11. |
| `13b9` | 13 | approx | Drops the b9. |
| `13#9` | 13 | approx | Drops the #9. |
| `7b9sus` | 7sus4 | approx | Drops the b9. |
| `7susadd3` | 7sus4 | approx | Drops the 3rd. |
| `9sus` | 7sus4 | approx | Drops the 9. |
| `13sus` | 7sus4 | approx | Drops the 9 and 13. |
| `7b13sus` | 7sus4 | approx | Drops the b13. |
| `11` | 7sus4 | approx | The dominant 11 is voiced as 9sus4, and the 9 is dropped. |

A quality that isn't in the table gets `Fit::Fallback`. This covers custom `*...*` qualities and unusual stacks such as `7#9b13`. `fallback_type` reads the quality from its spelling, in this order:

1. The prefix, if there is one:
   - `-` or `m`: a minor family
   - `^` or `maj`: maj7
   - `h`: m7b5
   - `o` or `dim`: dim or dim7
   - `+` or `aug`: aug or 7#5
2. `sus` gives 7sus4 or sus4.
3. `alt` gives 7#9.
4. A leading `6` gives 6 or 6/9.
5. The most characteristic dominant tension, in this order: #9, b9, #11, b13, #5, b5, 13, 9, then plain 7.

For example, `7#9b13` becomes 7#9.

Tests are in `src/ireal/tests.rs`. They use synthetic charts only; no real songs or playlists are committed.

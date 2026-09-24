# iReal Pro charts (`src/ireal`)

`yahaha::ireal` reads iReal Pro playlist links and turns a chart into a linear list of bars,
with yahaha chords on beats. It is a pure library module. The **chart player** (below) is
built on it: `src/engine/chart.rs` plays the bars, `src/session/chart.rs` imports and chooses
charts, and the app shows them.

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
- **Minor chords with a raised 5th** (`-#5`, `-b6`) keep the written root and become minor. yahaha has no minor-#5 type. `C-#5` has the same notes as `Ab/C`, but re-rooting it would show the player the wrong root and make the band follow an Ab chord.

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
| `-b6` | m | approx | Drops the b6. |
| `-#5` | m | approx | Drops the #5 and plays the 5th. |
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

## Chart player

In chart mode the band takes its chords and Main sections from the chosen chart instead of
from your left hand. The engine side is `src/engine/chart.rs`; the control side (import,
choosing a song, settings, style suggestion) is `src/session/chart.rs`; the commands and
state are `chart` in [app-api.md](app-api.md).

### How a chart plays

- **The plan.** Choosing a song expands its form `choruses` times (`expand`) and turns each
  bar into a `PlanBar`: the Main it plays, whether a section starts on it, the chord in
  effect as it begins, and up to 8 chords on beats. The plan is built on the control side
  and handed to the engine thread in a `Box` through its own ring. The plan it replaces
  comes back through another ring to be freed there (`tests/chart_no_alloc.rs`).
- **Bars.** One chart bar is one bar of the style. The chart moves on a bar at each bar line
  of the Main, Fill or Break playing (the engine's `on_bar` hook). It stays put during the
  Intro and the Ending.
- **Chords.** A chord goes in at its place in the bar: a chord on beat `b` of an `n`-beat
  chart bar goes in at `b/n` of the style's bar, counted in the chart's own beat unit. A 2/2
  or 12/8 chart's half-bar chord goes in at half the style's bar (quarter 3 of a 4/4 style), and
  a 6/8 chart's second eighth at a sixth of it. A chart in another metre than the style's (a 4/4
  chart on a 3/4 style) is stretched onto the style's bar the same way. The lead-sheet band draws
  the chords at the same places. Each bar line gives the chord the bar begins with. The engine
  wakes at each later chord (`hook_due` / `on_due`), so a chord between the style's quarter
  lines goes in on time. N.C. is Chord Cancel: rhythm only.
  Decision: a chord goes at its fraction of the bar, because iReal counts beats in the chart's
  beat unit and a chart bar is played as one style bar.
- **Sections.** Sections A, B, C and D play Main A, B, C and D. A verse (`V`) or the chart's
  own intro (`i`) keeps the Main before it, and bars before any mark play Main A.
  - A bar with a section mark starts that Main from its first bar, so the style's phrases
    line up with the chart's. The top of each chorus after the first, and the top of the
    loop each time it comes round, count as section marks.
  - With Auto Fill on, the bar before a section mark plays the new Main's fill (Fill In AA
    to DD, or the nearest the style has). A repeated section, such as A to A, gets its own
    fill, just as pressing the Main that is already playing does on a Genos (OM p.67). With
    Auto Fill off, the Main changes on the bar line with no fill, and a repeated section
    restarts its Main silently. The Genos has no such restart (pressing the lit Main
    always plays a fill; only Style Section Reset on TAP TEMPO restarts without one): this
    is a deliberate deviation, so the style's phrases stay in step with the chart's.
- **Intro and Ending.** `chart.intro` (default Intro A) plays before the first bar. The
  song's first chord already sounds during it. If you press an Intro yourself before
  starting, that one plays instead. After the last bar, `chart.ending` (default Ending A)
  plays. With no Ending set, the band stops at the end of the last bar.
- **Loop.** `setChartLoop` plays bars `[start, end)` over and over, with no Ending, until
  you stop or press an Ending. The app offers the whole song and each section. Fewer
  choruses drop a loop that ends past the new last bar.
- **Changing things while it plays.** The change the chart queued for the next bar line (a
  Main, a fill, the Ending or the stop) is the chart's own: turning chart mode off takes it
  back (the band plays on), and more choruses or a new loop or Ending queue the bar line
  again from the new plan. A new song starts from its first bar (its Main) at the next bar
  line. Turning chart mode on mid-song starts the chart from its first bar there too.
- **Your left hand.** A chord you play takes over at once and holds until the next bar
  line, where the chart takes over again (`chart.overridden` shows it). A chord played in
  the last half beat before a bar line is taken as an anticipation of the next bar: it holds
  through that bar as well, and the chart takes over at the line after it
  (`ANTICIPATE_BEATS` in `src/engine/chart.rs`). A Sync Start chord only starts the band:
  the chart's own chord plays from the first beat.
- **Transpose.** Keyboard transpose moves the chart's chords just as it moves the chords you
  play (the chart is "played" in its written key). Master transpose moves everything, as
  always.
- **Tempo.** Choosing a song with the band stopped sets the tempo to the chart's, when the
  chart has one (iReal's default is 0, meaning none).
- **Buttons still work.** Main, Fill, Break and Ending buttons do what they always do. The
  chart queues its next section change on the bar line before it, so it may replace a Main
  you pressed.

### Style suggestion

The chart's style label (`Song::style`, e.g. "Medium Swing") and iReal's playback groove
(`Song::groove`, e.g. "Latin-Brazil: Bossa Acoustic") pick words (`STYLE_WORDS` in
`src/ireal/styles.rs`). A library style scores by those words: in its name they count three
times as much as in its folder (the category), and earlier words count more. A style in
the chart's metre gets a point; ties go to the style nearest the chart's tempo.

| iReal says | Looks for |
|---|---|
| bossa, samba, baiao | bossa / samba / baiao, brazil, latin |
| salsa, mambo, cha, bolero, rumba, tango, songo, afro | that dance, then latin (afro: 6-8, 12-8) |
| calypso, reggae | calypso, soca, reggae, ska, dub |
| waltz | waltz, 3-4 (and the metre point for 3/4 styles) |
| ballad | ballad, slow |
| gospel, soul, r&b, blues, shuffle, second line, funk | that word and its neighbours |
| rock, pop, country, disco, march, folk | that word |
| even 8ths | 8beat, pop, jazz |
| swing, up tempo, bebop, jazz | swing, jazz, bigband |

With `chart.autoStyle` on (the default), choosing a song loads the suggested style.
Choosing another style in the browser overrides it until the next song is chosen, or for
good once Auto Style is off.

### Decisions

- **One chart bar = one style bar.** The Genos has no chart player, so nothing pins this
  down. Stretching a 3/4 chart over 4/4 bars would lose the chords' timing.
- **Section marks restart the Main.** This keeps a 4- or 8-bar Main pattern in phase with
  8-bar sections, the way a player presses Main on the section's first bar.
- **Fills lead into every section mark, and into each new chorus.** On a Genos, pressing
  the Main that is already playing plays its fill. An A-A-B-A chart gets a fill before each
  A, as a band would play one.
- **The left hand overrides until the next bar line**, as the task specifies. A shorter
  override (to the next chart chord) would cut a reharmonization off mid-bar.
- **A chord in the last half beat of a bar carries over into the next bar.** Players push
  (anticipate) a change by an eighth; ending the override at the line would drop a chord
  struck 20 ms early after 20 ms. Half a beat covers an anticipated eighth and a late
  hand, but not a chord played on the bar's last beat itself.
- **The chord names shown are yahaha's** (`Dm7`, `G7(9)`), from the mapped chords, not the
  chart's own spelling. They are the chart's chords as written: with Keyboard transpose on,
  the band plays them transposed, and the lane still shows the written key.
- **Playlists are kept in memory only** (nothing is written to disk). The app imports again
  from the file or the link.
- **Choruses are a setting (default 1)**, not the song's own repeat count (iReal's default
  is 3, meant for solos). The value carries over from song to song.
- **Chart chords don't wait for the chord-settle window.** They are exact and never
  rolled, so they go in through `apply_chord_unsettled`, from `process` (as the Chord
  Looper's do): on their tick, before that tick's notes. A chord you play still settles as
  usual, and takes over once it has.
- **The chart and the Chord Looper take turns.** Only one gives the band its chords: turning
  chart mode on stops a loop that plays or is armed, and a Chord Looper ON/OFF that would
  arm a loop turns chart mode off. The Genos has no chart player to copy; "the last one you
  turned on wins" is the least surprising. Recording is not affected: it records the
  chords you play over the chart. The engine makes the ON/OFF decision on its own state
  and reports it (`LooperSnap::chart_yields`), so one press works even straight after a
  memory is selected (#110).
- **Chart mode and the chosen chart are not Registration items.** The Genos has no
  equivalent group, and playlists live in memory only, so a registration could not bring
  the chart back.
- **Launchkey:** no pad for chart mode. Every pad page is already full, and chart mode is
  something you set up before playing, not during. The terminal UI has `M` (shift+m) and `( )`.

Tests are synthetic charts only: `src/engine/chart.rs` (the engine, on a corpus style),
`src/session_tests.rs` (`chart_player_*`), `src/ireal/styles.rs`, and
`tests/chart_no_alloc.rs`.

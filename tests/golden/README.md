# Golden tests

Two harnesses pin down chord behaviour, so that any change to it shows up as a diff someone has reviewed.

## Recognizer table (`src/recognizer_golden.rs`)

There is one test per row of the Data List chord table (docs/genos-features.md §D). Each test plays its chord in all 12 roots, in every inversion, and with every combination of the optional "(…)" notes left out. It checks the root, type and on-bass note the recognizer returns, using Fingered On Bass rules: if the root is the lowest note the chord has no on-bass note, and any other lowest note becomes the on-bass note.

Some pitch-class sets can be read as more than one chord. When exactly one of those readings has the lowest note as its root, that reading is the expected answer (C E G A = C6, A C E G = Am7). All other shared sets, such as E G A C, are covered by `ambiguous_voicings`. That test accepts any valid reading until #2 sets the priority rules.

Two more tests cover input outside the table rows. `octave_doublings_keep_the_chord` checks that doubling notes in other octaves gives the same chord. `off_table_inputs` (ignored until #2) expects no chord for note sets the table does not list: a single key, two keys other than 1+5 or 1+8, clusters, and Cancel with an extra key. The rows with no MIDI chord code (M7b5, (b5), mM7b5) fail until #2 decides which type they play as.

Rows that are not implemented yet are marked `#[ignore = "M1 #2"]`. To run them:

```sh
cargo test --release recognizer_golden -- --include-ignored
```

A failing row lists every wrong voicing, for example `row 21 m7(11) on C: [C2 D2 Eb2 F2 G2 Bb2]  want Cm11, got Ebmaj9/C`. Once the row passes, remove its `ignore`.

## Style snapshots (`src/golden.rs`)

`golden_snapshots` plays `chords.script` on six corpus styles and renders what each part plays as a readable listing (format below). The styles cover Guitar NTR (stroke and all-purpose), minor-5th NTT tables, chord-mute routing, SFF1, and a style with no CASM. When the corpus is missing, the test skips (see the setup notes in the top-level README).

### Why only digests are committed

This repo is public, and the styles are commercial Yamaha content. A listing of the notes each part plays, bar by bar, is a transcription of the style's bass and chord patterns, so committing it would redistribute them. The listings therefore stay on your machine, and the repo keeps only a digest that pins them down without revealing them.

| File | Committed | Contents |
| --- | --- | --- |
| `<style>.digest` | yes | The listing's style and bar header lines as they are (our own script's sections and chords), and for each part line only its channel, name and a 64-bit FNV-1a hash of the line. |
| `local/<style>.txt` | no (git-ignored) | The full listing from the latest run. |
| `local/<style>.baseline.txt` | no (git-ignored) | The full listing from the last `UPDATE_GOLDEN=1` run. It is used to show which notes changed. |

A digest looks like this:

```
bar 14  Main A > Fill In BB@4.0000  Caug@1.0000  [MainB]@3.1919
  ch10 Rhythm2 5d0c3e8a91f2b7c4
  ch11 Bass    a3f19e02c47d6b58
```

`committed_digests_hold_no_notes` checks that every committed part line is a key and a hash, and that no readable listing sits in this folder. Never commit anything from `local/`, and don't paste listing lines into issues, PRs or docs.

### When a digest differs

The test names each bar that changed, which parts changed, and the expected and actual header:

```
Example.S930.STY: 1 bar(s) changed
  bar 5: ch11 Bass changed
    want bar 5  Main A  Dm7@1.0000
    got  bar 5  Main A  Dm7@1.0000
```

If `local/<style>.baseline.txt` exists, the report then shows a line diff of the notes in just those bars. Changed lines are grouped by bar, and for each changed part line it lists the notes that differ:

```
  bar 5  Main A  Dm7@1.0000
-   ch11 Bass    1.0000 D1~480  2.0000 A0~480
+   ch11 Bass    1.0000 D1~480  2.0000 G0~480
    ^ 2.0000 A0~480 -> 2.0000 G0~480
```

The first passing run on a fresh checkout writes the baseline. If the digests already differ on your checkout, check out the last good revision and run `UPDATE_GOLDEN=1` there to get a baseline. The report says so if the baseline doesn't match the committed digest.

If the change is intended, regenerate the digests and the local baselines, then commit the digests together with the code change:

```sh
UPDATE_GOLDEN=1 cargo test --release golden
```

You can print the same listing for any style and script:

```sh
cargo run --release -- sim corpus/MOX_v2/FunkyFinger.S930.STY tests/golden/chords.script
cargo run --release -- sim corpus/MOX_v2/FunkyFinger.S930.STY "C Am F G7"
```

### Listing format

```
bar 14  Main A > Fill In BB@4.0000  Caug@1.0000  [MainB]@3.1919
  ch10 Rhythm2 14 as written
  ch11 Bass    1.0000 C1~480  2.0000 G0~480  ...
```

- The header line shows the bar number, then the section playing at the start of the bar. Each `> Section@beat.tick` marks a section change inside the bar, and after that come the script steps in the bar, spelled as the script wrote them. A button shows at the tick it was pressed (see below), so a button written on a bar line shows at the end of the bar before.
- Each part line (ch9–16, empty parts left out) lists every note the part starts in that bar, written as `beat.tick Note~length`.
  - Ticks are the style's own (ppq), and notes use Yamaha octave numbers (C3 = MIDI 60).
  - `~…` marks a note that is still sounding when the script ends.
  - A zero length means the note was cut on the same tick it started.
  - Notes show the pitch that sounds: the key sent plus any Retrigger Rule pitch shift the part is bent by.
  - `beat.tick C2>F2` marks a Pitch Shift. A note sounding C2 bends to F2 there, with no new attack, and keeps its length from where it started. It is listed in the bar where the bend happens.
  - Parts that play as written whatever the chord only get a count, `N as written`. These are the drum parts (ch9, ch10) and any part whose every source channel is Root Fixed (or Guitar) + Bypass for every key in that section. Listing them would copy the style's own patterns and says nothing about chord following.

## Script grammar

```
# comment: a token that starts with '#' comments out the rest of its line
[IntroA] | C | G7 |               # '|' separates bars
| Dm7 G7 | C - - [MainB] - |      # the chords, '-' holds and '^' releases in a bar split it evenly
[SyncStop] | C - ^ - | - | F |    # '^' lets go of the chord: Sync Stop stops there, F restarts
```

- **Chords:** a root (`C`, `F#`, `Bb`…), then a type suffix from `theory::TYPE_NAMES`, then an optional `/bass`. Examples: `C`, `Am7`, `Bm7b5`, `Cmaj7`, `Csus4`, `C1+5`, `C1+8`, `CmMaj7`, `Cm(add9)`, `G7#9`, `C/E`. The six-nine chord is written `C6(9)` (or `C69`), because in `C6/9` the `/` would be read as a bass note.
- **`-`:** holds the previous chord for one slot.
- **`^`:** lets go of every chord key for one slot (for Sync Stop). The engine remembers the last chord, and the next chord is a new press.
- **Buttons:** `[IntroA-D]`, `[MainA-D]`, `[Break]`, `[EndingA-D]`, `[AutoFill]`, `[StartStop]` (a toggle), `[Stop]`, `[SyncStart]`, `[SyncStop]` and `[StopAcmp]` take no time.
  - A button fires just before the next slot, one tick early, the way a player presses ahead of the beat. This means a press on a beat or bar line always counts for that beat.
  - The capture kit (`capture::plan`) times the same script differently: buttons half a beat early and chords a little early, because real MIDI jitters.
  - A button written between bars (`| [MainB] |`) fires at the start of the next bar.
- **Starting:** Sync Start is armed, so the first chord starts the style. An `[IntroX]` written before that chord picks the intro.
- **Empty bars:** `| |` on one line is an error, because it would drop a bar and shift every bar after it. Write `| - |` to hold a chord for a bar. A line that ends in `|` followed by a line that starts with `|` is fine.
- **Short form:** if the script has no `|` at all, every chord is its own bar (`"C Am F G7"`).

# Corpus self-consistency oracle (#8)

We have no Genos to record, so we check note conversion (NTR/NTT, spec §C.5) against the only ground truth the corpus holds: conversions that style authors wrote out by hand.

Code: `src/oracle.rs`. Command: `yahaha oracle corpus/`. Pinned scores: `tests/oracle/scores.txt`.

## Where the ground truth comes from

Many styles give one part several source channels and let the CASM chord mute pick one per chord. For example, a Bass has one source written over C major that plays on major-family chords, and a second one written over C minor that plays only on m, m6, m7 and so on. Where the second source is an edited copy of the first (same rhythm, a few notes moved), it is exactly what the author wanted the first source to turn into on a minor chord.

So for each such pair the oracle:
1. Takes the chord the **target** source was written for (its Source Root / Source Chord). On that chord the target plays unconverted, so its notes are the author's answer.
2. Plays the **other** source on that chord through our transposer, calling the public API `theory::transpose_group`. Notes are handed over in the groups the engine uses: consecutive note-ons of the channel on one tick, in event order, at most 8 (`engine::emit_at_index`). Root Fixed voice assignment depends on the group, so this keeps the oracle's result equal to what the engine plays.
3. Counts how many converted notes land on the author's notes.

Both directions are scored: major to minor and minor to major are separate pairs.

## What counts as a pair

Within one section, two non-drum source channels feeding the same destination part form a pair (A converted, B the reference) when all of these hold:

- **They are alternatives.** A is chord-muted on B's source chord (`theory::plays` is false), so on that chord only B sounds. Sources that sound together are layers, not alternatives.
- **B sounds as written.** B is not muted on its own source chord. Otherwise its notes are never heard unconverted and are no reference. These are counted as "muted on their own chord".
- **There is something to convert.** A's source chord differs from B's. These are counted as "same source chord".
- **Both have notes.** Each source starts at least 4 notes in the section. Otherwise the pair is counted as "too few notes": there is nothing to compare, and too little to tell a copy from a different line.
- **B is an edited copy of A.** Onsets are aligned when both sources start the same number of notes on the same tick. At least half of each source's notes must be aligned, with at least 4 notes in all. At least half of the aligned notes must be within 2 semitones of their partner, after moving A to B's root. Pairs that fail are counted as "not edited copies". They are different lines, and comparing them notes-for-notes would measure the arrangement, not the conversion.

These tests use only the style data, never our transposer. So the set of scored notes (the denominator) stays fixed when the transposer changes, and a score moves only when our conversion does.

## Scores

Per aligned tick, the converted notes of A and the authored notes of B are compared as multisets:
- **exact:** same MIDI note.
- **pitch:** same pitch class. This ignores octave placement (High Key, Note Limit), so it isolates the chord-type conversion.

Each pair is scored twice:
- **as authored:** with A's own CASM settings. This is a hypothetical: A is chord-muted on that chord, so real hardware never plays it there. Since most such sources are Root Trans + Bypass, it is close to the Bypass row and is not a measure of our fidelity.
- **per NTT table:** with every zone's table swapped for each table in turn, keeping A's NTR, High Key and limits. Root Trans and Root Fixed sources try Bypass, Melody, Chord and the eight minor tables. Guitar sources try the three Guitar tables.

The report sums these per NTT table, per chord change (for example `M>m`, with the table that agrees best), per NTR and per style. `--pairs` also lists every pair: section, part, channels, chord and score.

The report also has an **identity** check. Every chord-following source (not only paired ones) is played on its own source chord. RM p.28 says that plays back "the originally recorded data", so every note should come out as written. Notes are counted per NTR of the note's own zone. Notes in a Guitar zone are left out (see the Guitar check below). A note that moves is split into two kinds:
- **folded:** written outside the channel's own Note Limit and moved by octaves into it, exactly where RM p.30 puts it. That is Note Limit working as documented. A limit narrower than an octave cannot hold every pitch class; there the note may land on the octave nearest the limit, the lower one on a tie, which is our rule for that case (#13). No corpus style has such a limit.
- **moved inside Note Limit:** anything else, including a note outside the limit that lands in the wrong octave. These contradict RM p.28 and are transposer misses. The report prints them per NTR and the pin tracks them.

### Guitar check

Guitar sources are not scored on identity. There are two reasons:

- **The notes are string codes, not pitches.** Most Guitar strums in the corpus are stacked seconds (adjacent scale steps), one note per string. No guitar plays that cluster as pitches. The key picks a string of a voicing (B = string 1 down to D = string 6, C = the bass, C♯ = its fifth), and its octave picks the neck position. See docs/genos-features.md, "Guitar NTR voicing model". Playing such a source back "as written" would sound the cluster itself.
- **No chord plays a Guitar source back.** RM p.29 says Source Root/Chord are not applied when NTR is Guitar. The corpus agrees: the 42 Guitar rules declared over another chord (Cm11, Cm7, AM7, F♯M7, C6/9, C) use the same codes as the CM7 ones.

So every Guitar note is played on every chord its channel plays: all 12 roots, every CASM chord type, as the chord and root mutes allow. Each output must be what a guitar can sound for that chord. The check is independent of how our voicings are built. Per Guitar table of the note's own zone, the pin counts:

- `notes`: the Guitar note-ons. `noise`: those at key 96 (C7) or above. These are MegaVoice strum, fret and body noises, not pitches.
- `noise-moved`: noise keys that did not come back untouched on every chord. Should be 0.
- `plays`: note × chord plays of the other notes. `muted`: plays that sounded nothing, which is Stroke leaving strings out ("some notes may sound as if they are muted", RM p.30).
- `not-chord-tone`: a sounding output that is not a tone of the played chord. Should be 0.
- `out-of-range`: a sounding output below the open low E (MIDI 40) or outside the channel's Note Limit. Should be 0.

It does not check how many strings a voicing uses, or which string plays which tone. No manual page or corpus data says what the Genos does there, so any such check would only grade our model against itself. The unit tests in `theory::tests` cover those rules (for example, Arpeggio's four voices).

## Tracking changes

`oracle::tests::corpus_scores` recomputes the scores on `corpus/` and compares them with `tests/oracle/scores.txt`. Each line of that file is a key and either one count or one score (`notes exact pitch-class`). Styles are keyed by their path below `corpus/`, and only the styles the file lists are scored, so adding styles to the (unversioned) corpus does not move the pin; the test notes how many it left out. When a change moves any number, the test fails and prints each changed line, with the exact-hit rate before and after for score lines:

```
  table MelodicMinor: [249112, 165095, 189039] -> [249112, 166001, 189800]  exact 66.3% -> 66.6% (+0.4)
  identity RootTrans moved-in-limit: [1426] -> [0]
```

If the change is intended, run `UPDATE_GOLDEN=1 cargo test --release oracle`, commit the file, and quote the delta in the PR. `yahaha oracle corpus/ --diff tests/oracle/scores.txt` prints the same delta without the test harness. It also works on part of the corpus (`yahaha oracle corpus/MOX_v2 --diff tests/oracle/scores.txt`): each style is matched to the pinned key that ends in its path, and since the corpus totals cannot be compared then, only those styles' own lines are. Without `corpus/` the test skips, like the golden snapshots.

The file holds counts only: style paths, table names, chord-change labels and integers. `committed_scores_hold_only_numbers` checks that. No note, pitch or pattern from a style is ever written out.

## Limits

- **Authors are not the hardware.** A hand-written minor version shows what the author wanted, which need not be what any NTT table produces. Authors split channels exactly where a table would not do what they wanted, so these pairs lean towards the hard cases. A score of 100% is not the goal. Read the scores as comparisons: table against table, and before against after.
- **Mostly major and minor.** Almost all scored notes are `M>m` and `m>M`. Dominant, diminished, augmented and tension chords have a handful of pairs each, and Guitar sources have none. So the table scores say little about Chord-table voicing, the 5th variants (#11) or Guitar NTR (#12). Guitar has the Guitar check instead, which tests that the output is valid, not that it is what the hardware plays.
- **Ties are artifacts.** Each 5th-variant table scores within a few notes of its base table: the tables differ only over aug and dim chords (#11), and almost no scored pair converts to or from one. Before #11 landed, Melody and Natural Minor also tied on every chord change. Neither says anything about how Yamaha's tables relate.
- **Root movement is barely tested.** Source roots are nearly always C, so the played chord is on the same root. NTR, High Key and the root half of the conversion are exercised only through the identity check.
- **Octave.** Converted notes are folded into A's Note Limit. The reference is B's authored notes, which may lie outside B's own limit. Compare the pitch column to leave octave out.
- **Alignment is strict.** Only ticks where both sources start the same number of notes are compared. Rhythmic edits, added or dropped chord notes, and grace notes fall out of the comparison.
- **Nothing is played.** This is a note-for-note check of `transpose_group`. It does not run the engine, so RTR, chord changes mid-note and section changes are out of scope; the golden snapshots cover those.

## Pin history

- **#55 (NTT 5th-variation tables, #11)** moved the pin; #56, #59 and #57 did not. Measured by cherry-picking the oracle onto each merge commit. Across all pairs, exact: Harmonic Minor 64.1% -> 67.8%, Melodic Minor 66.3% -> 67.0%, Natural Minor 62.4% -> 66.5%, Dorian 64.0% -> 65.7%. Harmonic Minor now agrees best on both `M>m` (68.9% against Melodic Minor's 68.3%) and `m>M`, so the baseline's "Melodic Minor agrees best" no longer holds. As authored fell by 120 of 249112 notes (512 pitch class), all in Melodic Minor 5th pairs of BaroqueConcerto and KoolFunk between a 7 and a maj7 source: the authors' copies take the played chord's 7th, and #55 keeps the source's 7th ("other notes are not changed"). This is #55's playtest flag; see the PR #57 notes.
- **#60 (Guitar NTR, #12)** replaced the Guitar identity check with the Guitar check. Nothing else moved: no scored pair is Guitar, and the Root Trans and Root Fixed identity lines are unchanged.
  - **Why.** An earlier draft of #60 read Guitar sources as pitches fingered over CM7. On that basis, the Guitar identity had risen from 10.4% to 81.3% exact. The full corpus showed that reading was wrong. Of 14,153 Guitar strums of three or more strings, 13,372 are stacked seconds. In the Yamaha-authored T5Style and SX900 styles that is 13,372 of 13,879; in MOX_v2 it is 0 of 274. Those are string codes, so "as written" is not a meaningful target, and #60 decodes them as strings (docs/genos-features.md).
  - **Lines removed.** `identity Guitar 96638 78539 78539` and `identity Guitar moved-in-limit 18099` are gone. The corpus identity total now counts 431,928 notes instead of 528,566. Each style's `identity` and `moved-in-limit` lines drop their Guitar notes: 140 styles change `identity`, and 74 of them also change `moved-in-limit`.
  - **Lines added.** The `guitar` lines and one `style … guitar` line per style with a Guitar part. They count 97,156 Guitar notes. Identity counted 96,638, because it skipped the 518 notes of channels muted on their own declared source chord; the Guitar check ignores the source chord.
  - **Result.** Under #60's string model, `noise-moved`, `not-chord-tone` and `out-of-range` are 0 for all three tables, and Stroke mutes 719,196 of 22,506,408 plays. For comparison, the same check on earlier models, not pinned:

    | Table | Model | noise moved | not a chord tone | muted |
    |---|---|---|---|---|
    | All Purpose | before #60 (old string code) | 33 of 33 | 84 | 26,131 |
    | All Purpose | #60's earlier pitch model | 33 of 33 | 121,428 | 0 |
    | All Purpose | #60 | 0 | 0 | 0 |
    | Stroke | before #60 | 15,426 of 15,426 | 3,780 | 766,369 |
    | Stroke | pitch model | 15,426 of 15,426 | 7,749,312 | 0 |
    | Stroke | #60 | 0 | 0 | 719,196 |
    | Arpeggio | before #60 | 5,428 of 5,428 | 27,996 | 189,615 |
    | Arpeggio | pitch model | 5,428 of 5,428 | 0 | 0 |
    | Arpeggio | #60 | 0 | 0 | 0 |

    The pitch model's All Purpose and Stroke misses are its melody-scale passing tones. In string-coded sources those are the codes for strings 4 and 2, F and A.

## Baseline (208 styles)

- 5210 pairs scored (249112 notes) in 166 styles. Not scored: 1211 pairs that are not edited copies, 469 where one source has fewer than 4 notes in the section (325 of them none), 1884 with the same source chord, and 511 muted on their own chord.
- As authored: 60.8% exact, 69.3% pitch. 93% of the converted sources (4831 of 5210) are Root Trans + Bypass, because the chord mute already does the work, so "as authored" is close to Bypass (60.7%). It is a hypothetical, not our fidelity (see Scores).
- Tables across all pairs, exact: Melodic Minor 66.3%, Harmonic Minor 64.1%, Dorian 64.0%, Melody 62.4%, Natural Minor 62.4%, Chord 62.4%, Bypass 60.7%. For both `M>m` and `m>M`, the authors' own minor versions agree best with Melodic Minor, which only moves the 3rd. The 5th variants tie with their base tables (see Limits).
- Identity, per NTR of the note's zone:

  | NTR | notes | exact | pitch | moved inside Note Limit |
  |---|---|---|---|---|
  | Root Trans | 330282 | 86.1% | 100% | 1426 |
  | Root Fixed | 101646 | 99.4% | 100% | 0 |
  | Guitar | 96638 | 10.4% | 18.1% | 86606 |

  (#60 replaced the Guitar row with the Guitar check; see Pin history.)

  - Root Fixed: every miss is a Note Limit fold.
  - Root Trans: 44559 misses are folds. The other **1426 notes are a transposer bug**. `theory::transpose` applies High Key even when the chord root equals the Source Root: with Source Root E and High Key D#, playing Em7 (the source chord) shifts the pattern down an octave, although the notes lie inside their Note Limit. RM p.28 says the source chord plays the recorded data back. These are channels whose Source Root is above their High Key. The fix belongs in the transposer, not here; the pin will show `identity RootTrans moved-in-limit` drop to 0 when it lands.
  - Guitar: this row read string-coded sources as pitches. #60 replaced it with the Guitar check (see Pin history).

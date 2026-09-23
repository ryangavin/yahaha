# Corpus self-consistency oracle (#8)

We have no Genos to record, so we check note conversion (NTR/NTT, spec §C.5) against the only ground truth the corpus holds: conversions that style authors wrote out by hand.

Code: `src/oracle.rs`. Command: `yahaha oracle corpus/`. Pinned scores: `tests/oracle/scores.txt`.

## Where the ground truth comes from

Many styles give one part several source channels and let the CASM chord mute pick one per chord. For example, a Bass has one source written over C major that plays on major-family chords, and a second one written over C minor that plays only on m, m6, m7 and so on. Where the second source is an edited copy of the first (same rhythm, a few notes moved), it is exactly what the author wanted the first source to turn into on a minor chord.

So for each such pair the oracle:
1. Takes the chord the **target** source was written for (its Source Root / Source Chord). On that chord the target plays unconverted, so its notes are the author's answer.
2. Plays the **other** source on that chord through our transposer, calling the public API `theory::transpose_group` (the same call the engine makes for notes that start together).
3. Counts how many converted notes land on the author's notes.

Both directions are scored: major to minor and minor to major are separate pairs.

## What counts as a pair

Within one section, two non-drum source channels feeding the same destination part form a pair (A converted, B the reference) when all of these hold:

- **They are alternatives.** A is chord-muted on B's source chord (`theory::plays` is false), so on that chord only B sounds. Sources that sound together are layers, not alternatives.
- **B sounds as written.** B is not muted on its own source chord. Otherwise its notes are never heard unconverted and are no reference. These are counted as "muted on their own chord".
- **There is something to convert.** A's source chord differs from B's. These are counted as "same source chord".
- **B is an edited copy of A.** Onsets are aligned when both sources start the same number of notes on the same tick. At least half of each source's notes must be aligned, with at least 4 notes in all. At least half of the aligned notes must be within 2 semitones of their partner, after moving A to B's root. Pairs that fail are counted as "not edited copies". They are different lines, and comparing them notes-for-notes would measure the arrangement, not the conversion.

These tests use only the style data, never our transposer. So the set of scored notes (the denominator) stays fixed when the transposer changes, and a score moves only when our conversion does.

## Scores

Per aligned tick, the converted notes of A and the authored notes of B are compared as multisets:
- **exact:** same MIDI note.
- **pitch:** same pitch class. This ignores octave placement (High Key, Note Limit), so it isolates the chord-type conversion.

Each pair is scored twice:
- **as authored:** with A's own CASM settings.
- **per NTT table:** with every zone's table swapped for each table in turn, keeping A's NTR, High Key and limits. Root Trans and Root Fixed sources try Bypass, Melody, Chord and the eight minor tables. Guitar sources try the three Guitar tables.

The report sums these per NTT table, per chord change (for example `M>m`, with the table that agrees best), per NTR and per style. `--pairs` also lists every pair: section, part, channels, chord and score.

The report also has an **identity** check. Every chord-following source (not only paired ones) is played on its own source chord. The spec says that "reproduces the pattern unchanged" (RM p.28), so a miss here is ours, unless the note lies outside the channel's own Note Limit.

## Tracking changes

`oracle::tests::corpus_scores` recomputes the scores on `corpus/` and compares them with `tests/oracle/scores.txt`. When a change moves any number, the test fails and prints each changed line with its exact-hit rate before and after:

```
  table MelodicMinor: [249112, 165095, 189039] -> [249112, 166001, 189800]  exact 66.3% -> 66.6% (+0.4)
```

If the change is intended, run `UPDATE_GOLDEN=1 cargo test --release oracle`, commit the file, and quote the delta in the PR. `yahaha oracle corpus/ --diff tests/oracle/scores.txt` prints the same delta without the test harness. Without `corpus/` the test skips, like the golden snapshots.

The file holds counts only: style file names, table names, chord-change labels and integers. `committed_scores_hold_only_numbers` checks that. No note, pitch or pattern from a style is ever written out.

## Limits

- **Authors are not the hardware.** A hand-written minor version shows what the author wanted, which need not be what any NTT table produces. Authors split channels exactly where a table would not do what they wanted, so these pairs lean towards the hard cases. A score of 100% is not the goal. Read the scores as comparisons: table against table, and before against after.
- **Mostly major and minor.** Almost all scored notes are `M>m` and `m>M`. Dominant, diminished, augmented and tension chords have a handful of pairs each, and Guitar sources have none. So the table scores say little about Chord-table voicing, the 5th variants (#11) or Guitar NTR (#12). The identity check still covers Guitar.
- **Root movement is barely tested.** Source roots are nearly always C, so the played chord is on the same root. NTR, High Key and the root half of the conversion are exercised only through the identity check.
- **Octave.** Converted notes are folded into A's Note Limit. The reference is B's authored notes, which may lie outside B's own limit. Compare the pitch column to leave octave out.
- **Alignment is strict.** Only ticks where both sources start the same number of notes are compared. Rhythmic edits, added or dropped chord notes, and grace notes fall out of the comparison.
- **Nothing is played.** This is a note-for-note check of `transpose_group`. It does not run the engine, so RTR, chord changes mid-note and section changes are out of scope; the golden snapshots cover those.

## Baseline (first run, 208 styles)

- 5210 pairs scored (249112 notes) in 166 styles. Not scored: 1680 pairs that are not edited copies, 1884 with the same source chord, and 511 muted on their own chord.
- As authored: 60.8% exact, 69.3% pitch. 93% of the converted sources (4831 of 5210) are Root Trans + Bypass, because the chord mute already does the work, so "as authored" is close to Bypass (60.7%).
- Tables across all pairs, exact: Melodic Minor 66.3%, Harmonic Minor 64.1%, Dorian 64.0%, Melody = Natural Minor 62.4%, Chord 62.4%, Bypass 60.7%. For both `M>m` and `m>M`, the authors' own minor versions agree best with Melodic Minor, which only moves the 3rd.
- Identity: Root Fixed 98.9% exact, Root Trans 87.3% exact. Both are 100% on pitch class: every miss is an octave fold of a note written outside its own Note Limit. Guitar is 14.5% exact: `guitar()` re-voices patterns even on their own chord (#12).

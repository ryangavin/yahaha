# Keyboard Harmony

`src/harmony.rs` is our own implementation of the Genos Keyboard Harmony types. It follows the
behaviour the manuals describe (Owner's Manual p.56–57, Reference Manual p.46–47, Data List p.74)
and uses no Yamaha data. The manuals name the types but never give their voicings, so each
voicing rule below is our own design. Rules marked **(guess)** should be checked by ear against
a real Genos.

The module is pure and real-time safe: it does no allocation, locking or panicking. It is not
wired into the engine yet; a later ticket connects it to `live.rs` and `engine.rs`.

## API

| Item | What it does |
|---|---|
| `chord_zone(acmp, left, style_split, left_split) -> ChordZone` | The keyboard layout for one ACMP/LEFT combination: chord keys, LEFT-voice keys, lowest Right key |
| `harmony_chord(ty, acmp, left, style_chord, left_chord) -> Option<Chord>` | The chord the harmony follows |
| `voice(ty, melody, chord) -> Voicing` | The harmony keys for one melody key, highest first, with per-note strum delays |
| `harmonize(melody, vel, chord, &HarmonySettings, RightParts) -> Harmony` | `voice` plus Chord Note Only, Minimum Velocity, Volume and Assign |
| `route(assign, category, RightParts, n) -> Routing` | The Right parts the melody and each effect note sound on |
| `melody_of(held) -> Option<u8>` | The melody key when several keys are held (the highest) |
| `HarmonyTracker` | Remembers what each melody key added, so the key's note-off stops those notes |
| `MultiAssign` | Multi Assign: `press(key, parts) -> PartMask`, `release(key) -> PartMask` |
| `EchoGen` | Echo, Tremolo and Trill: `note_on`, `note_off`, `set_tempo`, `set_settings`, `all_off`, `next_due() -> Option<u64>`, `next_events(now, &mut [EchoEvent]) -> usize` |

Keys are MIDI note numbers. Times are nanoseconds on the engine clock. A `PartMask` has bit 0 for
Right 1, bit 1 for Right 2 and bit 2 for Right 3.

## Chord source (spec §6, OM p.56)

| ACMP | LEFT | Chord from | Chord keys | LEFT voice keys | Right parts |
|---|---|---|---|---|---|
| on | off | Style chord | ≤ Style split | none | > Style split |
| off | on | LEFT section | ≤ Left split | ≤ Left split | > Left split |
| on | on | Style chord | ≤ Style split | Style split < k ≤ Left split | > Left split |
| off | off | none | none | none | whole keyboard |

- A split key belongs to the section below it (F#2 and below is the left side).
- A Left split below the Style split is read as equal to it, because the Genos keeps Left ≥ Style.
- "1+5", "Octave", Multi Assign and the Echo category ignore the chord.
- A Cancel (N.C.) chord counts as no chord. **(guess)** With no chord, the chord-following types
  add nothing.
- **(guess)** With both ACMP and LEFT off, there is no chord.
- An on-bass note that is not a chord tone does not count as a chord tone.

## Voicing building blocks

- **Below search:** walk down from the melody. Take the first key of each chord pitch class, nearest
  first. Never take the melody's own pitch class, or the pitch class a semitone under it (that
  would be a minor 9th or major 7th against the melody).
- **Duet gap:** in the duet and trio types, the first harmony note is at least a minor 3rd from the
  melody. A two-voice harmony therefore never makes a second.
- **Close gap:** in the close types (4-way, Block, Full Chord, Strum), a whole step is allowed when
  the melody is a chord tone (for example A over G in C6). A passing melody note keeps a minor 3rd
  clear, so the block reads as a reharmonised tension (D over A–G–E in C6).
- **Four-part set:** used by 4-way and Block.
  - Triads gain a 6th (major and minor) or a diminished 7th in the "6th" variants, and a 7th (maj7,
    m7) in the "7th" variants.
  - Augmented, sus and (b5) chords gain a b7.
  - Five-note chords drop the 5th.
  - When the melody sits a semitone above a set tone (C over B in Cmaj7), that tone becomes the one a
    minor 3rd under the melody (A).
  - **(guess)** All of the above.

## Types

| Type | Rule | Side |
|---|---|---|
| Standard Duet 1 | The nearest chord tone below | below |
| Standard Duet 2 | The second chord tone below (an "open" duet) **(guess)** | below |
| Standard Trio | The two nearest chord tones below | below |
| Full Chord | Every chord tone (up to four) in close position below, plus the root under the lowest note **(guess: the added root)** | below |
| Rock Duet | The nearest root or 5th below, a power-chord harmony **(guess)** | below |
| Country Duet 1 | Tenor: the nearest chord tone above **(guess)** | above |
| Country Duet 2 | The tenor line an octave down (baritone register), else the nearest chord tone below **(guess)** | below |
| Country Trio | Tenor above plus the nearest other chord tone below **(guess)** | both |
| Block | 4-Way Close 1 plus the melody an octave down (locked hands) **(guess)** | below |
| 4-Way Close 1 | Three notes of the 6th-variant four-part set, close below | below |
| 4-Way Close 2 | The same with the 7th variant (maj7, m7) **(guess: which variant is which)** | below |
| 4-Way Close 3 | The 6th variant with the 9th replacing the root (not on dim chords). On 7(b9) and 7(#9) the chord's own altered 9th replaces the root instead (3-5-b7-b9, 3-5-b7-#9), never a natural 9th **(guess)** | below |
| 4-Way Close 4 | Close 2 plus the melody an octave down **(guess)** | below |
| 4-Way Open 1 | Close 1 in drop-2 **(guess)** | below |
| 4-Way Open 2 | Close 1 in drop-3 **(guess)** | below |
| 4-Way Open 3 | Close 1 in drop-2-and-4 **(guess)** | below |

In the Open types, a drop that would put a minor 9th against another voice (for example the B of
Cmaj7 under a C) is skipped and that voice stays in close position.
| 1+5 | A perfect 5th above; ignores the chord **(guess: above rather than below)** | above |
| Octave | An octave below; ignores the chord **(guess: below rather than above)** | below |
| Strum | Up to three chord tones in close position below. Each note is 15 ms after the previous one, stepping down from the melody **(guess: direction and timing)** | below |
| Multi Assign | Right-hand keys go to R1, R2, R3 in the order pressed. Each new key takes the first free part among those that are on; when all are busy it wraps. Ignores ACMP/LEFT | n/a |
| Echo | The struck note, then repeats at every Speed note value while held. Each repeat is ×3/4 of the one before, starting from Volume, until the level drops below 4 **(guess: decay, and stopping at release)** | n/a |
| Tremolo | Like Echo, but the repeats stay at Volume | n/a |
| Trill | With two or more keys held, the last two alternate every Speed note value, starting with the newer key. Other held keys fall silent **(guess)**. A single key just sounds | n/a |

Keys outside 0–127 are dropped, never wrapped.

## Detail settings (RM p.46–47)

- **Volume (0–127):** effect velocity = key velocity × Volume / 127, with 127 meaning unchanged
  **(guess: the scale)**. At 0 the effect is silent.
- **Speed:** 1/4, 1/6, 1/8, 1/12, 1/16, 1/32 **(guess: the list of values)**. At 120 BPM, 1/8 is
  250 ms. A tempo change applies from the next repeat after the one already scheduled.
  - Note length is 3/4 of the period for Echo/Tremolo and 9/10 for Trill **(guess)**.
  - Repeats count from the key press, not from the bar **(guess)**.
- **Assign:**
  - Auto: the melody sounds on every Right part that is on. The effect sounds on the first eligible
    part in the order R1, R2, R3 **(guess: one part rather than all of them)**.
  - Multi: with two or more eligible parts, the melody sounds on the first. Effect notes go to the
    other parts in turn, then back to the first. With one part, everything sounds there.
  - Right 1/2/3: the effect sounds on that part only if it is on **(guess)**.
  - In the Harmony category, a Mono/Legato/Crossfade part counts as off for the effect (RM p.46).
  - In Multi, a Mono part keeps playing the melody.
- **Chord Note Only:** applies to the Harmony category only. A melody note that is not a tone of the
  current chord gets no harmony. With no chord, nothing is harmonised. The chordless types (1+5,
  Octave) are unaffected.
- **Minimum Velocity:** the effect sounds when key velocity ≥ the setting. The manual says "above",
  and we read that as inclusive **(guess)**. In the Echo category, a key below the threshold sounds
  plainly with no repeats.
- Multi Assign has none of these settings.

## Behaviour left to the wiring ticket

- **(guess)** When several right-hand keys are held, only the highest (`melody_of`) is harmonised.
- **(guess)** A chord change while a melody key is held does not re-voice that key's harmony.
  `HarmonyTracker` returns the original notes at note-off.
- `EchoGen` queues immediate events (struck notes, note-offs) until the next `next_events`. A
  note-off whose note-on is still queued cancels it, so the queue stays bounded however many
  presses arrive between polls. The `effect` flag of a note-off always matches its note-on.
- `EchoGen` owns every note of the keys it is given, including the struck note. The engine should
  route those keys to `EchoGen` instead of playing them directly, and use `EchoEvent::effect` with
  `route` for Assign = Multi.

## Hear-test checklist

1. Standard Duet 2, Rock Duet, Country Duet 1/2 and Country Trio: do they add notes above or below,
   and at which interval?
2. 1+5 and Octave: above or below?
3. 4-Way Close 1–4 and Open 1–3: which variants the numbers mean, and whether Close 3/4 double
   the melody.
4. Block: is the melody doubled an octave down?
5. Full Chord: is the extra root there?
6. Strum: direction and spacing.
7. Echo: does it decay, and does it keep sounding after release?
8. Trill: which of the two keys sounds first, and what happens to the older held keys.
9. Assign Auto: are the effect notes on one part or on every part that is on?
10. Speed values, Volume scale and note lengths.

# Chord Looper

The Genos CHORD LOOPER (OM p.68–69, RM p.14–19): record the chords you play while the
style runs, then loop them, so the band carries on with the progression while both hands
are free. yahaha builds it as an engine chord source: while it loops, it feeds its chords
to the style exactly as if they were played.

Code: `src/looper.rs` (the sequence, pure), `src/engine/looper.rs` (recording, looping,
the bar-line timing, through the engine hooks), `src/session/looper.rs` (the eight
memories and the app API). Commands and state: [app-api.md](app-api.md), "Chord Looper"
and `looper`.

## Behaviour

| You press | While | What happens | Genos ref |
|---|---|---|---|
| REC/STOP | playing | REC flashes; recording starts at the next bar line, with the chord held then as its first chord | RM p.14 |
| REC/STOP | stopped | REC flashes and Sync Start turns on; the first chord starts the style and the recording together, on beat 1 | RM p.16 |
| REC/STOP | recording | recording stops; the style plays on | RM p.16 |
| START/STOP | recording | the style and the recording stop | RM p.16 |
| ON/OFF | recording | recording stops, ON/OFF flashes, the loop starts at the next bar line | RM p.15 |
| ON/OFF | a sequence, not looping | ON/OFF flashes; the loop starts at the next bar line, or when the style starts | RM p.18 |
| ON/OFF | looping | the loop stops at once; the style follows the keyboard again | RM p.15, p.19 |
| memory 1–8 | looping | the memory takes over at the next bar line, from its top | RM p.19 |
| memory 1–8 | not looping | a memory that holds a sequence replaces the current one | RM p.16 notice |
| Memory + 1–8 | | stores the current sequence (named `CLD_001`, …) | RM p.17 |

While the loop plays, chords from the keyboard are ignored, and so is letting go of them
(Sync Stop): the whole keyboard is yours to play. The Genos flashes the ACMP lamp then;
yahaha has no ACMP switch (the accompaniment always follows chords), so the app shows the
looper's state instead.

## Decisions

The manuals leave these open (genos-features.md §G.4). What yahaha does, and why:

- **Chord times snap to a 16th-note grid.** Because players land chords a little early
  or late, and a loop replays exactly what it recorded: snapping makes the replay tidy.
  A chord that snaps to the end of a bar belongs to the next bar's first beat, so an
  anticipated chord is recorded on the beat it anticipates.
- **The loop is whole bars: the bars recording ran through, counting the one in which it
  stopped.** Because loop playback starts at the next bar line anyway (RM p.15), so the
  bar in which you press ON/OFF is the last bar of the loop. Press it in the last bar you
  want, as the Genos tells you to press ON/OFF "just before the measure" (RM p.18). A chord
  anticipating the loop's restart (it would snap past the end) is dropped: the loop's
  first chord plays there.
- **The chord held when recording starts is the loop's first chord.** Because on the
  Genos you keep holding the chord across the bar line; with nothing held, the loop keeps
  the last chord of its previous pass at its top.
- **Capacity: 128 chord changes and 64 bars.** Because the sequence must be a fixed-size
  value the engine thread can record into without allocating. Recording stops as if
  REC/STOP had been pressed when either fills up.
- **Stopping the style while looping leaves the loop armed.** Because pressing ON/OFF
  before starting is how the Genos starts a performance with the loop (RM p.18 step 3):
  the next start plays the loop from its top.
- **When the loop stops, the style follows the last chord played on the keyboard during
  the loop, if any**; otherwise it keeps the loop's chord until you play one.
- **Choosing a memory while recording is refused**, with a message: the recording would
  otherwise be lost midway.
- **The key routing is unchanged while looping.** Keys left of the split still play the
  Left part when it is on, as on the Genos.
- **No .clb/.cld files, no Registration or Freeze yet.** The file formats are
  undocumented; memories last for the session. Registration Memory is not built yet.
- **No Launchkey pads yet.** Page 2 and page 3 each have one free pad; REC/STOP and
  ON/OFF belong together, so they wait for a pad-page rework. The terminal UI has `r`
  (REC/STOP) and `R` (ON/OFF); the app has a Chord Looper drawer.

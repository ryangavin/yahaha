# Split points, chord detection area (Upper/Lower), Left Hold, Manual Bass

Topic id: `split-detection` · Pass: 2026-09-25 · Manual: OM p.48-51, OM p.56-57, RM p.9, DL p.82, DL p.88 · yahaha: `docs/genos-features.md#split-points`, `src/parts.rs`, `src/session/keyboard.rs`, `src/session/chord.rs`, `src/live.rs`, `app/src/panels/settings/SplitPage.svelte`

Paraphrased notes only. No transcript text, manual text or frames are committed.

**Video coverage this pass: 4 of 4** (V1–V4 read), plus the Left Hold use in V5 (Casper, "without
an active style", assigned to sync-start-stop).

## Sources

| # | Video (channel) | URL | Timestamps used |
|---|---|---|---|
| V1 | Left Voice, splitting keyboard and one simple trick (Casper tutorSynth) | https://www.youtube.com/watch?v=WXKmjR68Dz4 | 01:03, 01:45–02:48, 04:07–04:22, 05:07–06:15, 09:13–09:47 |
| V2 | Song of the Week & Buttons Class: Left Hold (Keyboardamerica) | https://www.youtube.com/watch?v=QLE0ugVpTWU | 01:40–02:32, 03:52–04:17, 10:35–11:36 |
| V3 | Lost your upper Right 3 voice? Split function could be turned on (Leigh Wilbraham) | https://www.youtube.com/watch?v=8ZRDZFzYzIw | 00:04–01:50, 02:03–02:53, 03:18–03:44, 04:37–05:02 |
| V4 | Splitting Right Tracks in Yamaha (Arranger Tutorials) | https://www.youtube.com/watch?v=I2YiElHwx-s | 00:47–01:25, 03:04, 03:30–04:23 |
| V5 | Playing Yamaha Arrangers without active Style (Casper tutorSynth) | https://www.youtube.com/watch?v=WUV5RlGRDdk | 01:55–02:14 |

No stills taken for this topic (not needed: the transcripts describe the screens; still wanted if
someone checks the Left-voice-in-chord-zone question: V1 05:25, the Split window with Style at C2
and Left at C4).

## Genos behaviour (subtleties a player notices)

- Three independent split points: **Style** (top of the chord section), **Left** (Left voice vs Right
  voices) and **Right 3** (R1/R2 vs R3). Left can't go below Style; Right 3 can't go below Left.
  OM p.49-50.
- "Style + Left" sets the first two to one key, so the chord section and the Left voice share one
  area; setting the points separately gives a chord-only zone at the bottom, then a Left-voice zone.
  OM p.49-50. V1 sets Style to C2 and Left to C4 and describes it as the style reading chords up to
  C2 while the Left voice plays up to C4, so the Left voice can be reached with the right hand
  [V1 05:25–06:00]. Whether Left also sounds below C2 is still not stated outright (the phrasing
  suggests it does: the Left range is "up to" C4).
- Registration stores the split points (V1 with the Style box ticked, V4 for the Right 3 point).
  [V1 04:07, V4 03:04]
- Setting a split: hold the on-screen label and press a key, or L/R controls, or the Data dial.
  OM p.50. Every presenter uses hold-and-press. [V1 05:07, V3 00:44, V4 01:01] Shortcut: DIRECT
  ACCESS + ACMP or SYNC START opens the page. [V4 00:47]
- Right 3 split: with it set, R3 plays only above the Right 3 point and R1/R2 only between Left and
  Right 3 (a solo voice on top, a comp voice below). OM p.49, p.51 diagrams. In practice: guitar
  alone on the top zone with strings + brass below [V4 01:25]; sax + bells below and a third voice
  on the top octave [V3 02:15–02:40, 04:49]; Left + R1/R2 + R3 as three zones [V1 09:13–09:47,
  V4 04:01].
- R3 with its split on sounds over the whole right side only while it is the only Right part on;
  as soon as R1/R2 are on it is confined above its point, so "my Right 3 voice went silent" is a
  common support question. [V3 00:04–00:29, 02:03] Clearing it: set the point back to the bottom
  [V3 03:18–03:44] or hold the Right 3 label [V4 03:30]. *video-only*
- Left part off: a Right voice covers the whole keyboard (except the chord-detection keys while ACMP is on).
  OM p.48, p.56.
- ACMP off + LEFT on: the Left section still yields a chord for Keyboard Harmony and Multi Pad Chord
  Match. OM p.57, RM p.65.
- **Left Hold**: with LEFT on, the Left voice keeps sounding after release; sustained voices hold,
  decaying ones decay slowly as if sustained. Cancelled by stopping the Style/Song or turning Left
  Hold off. Has a panel button and an assignable function. OM p.49, RM p.141.
  In practice: an organ (or pad) Left voice with Left Hold on keeps ringing across chord changes, so
  chord moves sound smooth and forgiving [V2 01:40–02:32, 10:35–11:36]; V2 keeps it in the song's
  registration with the Left voice at full volume. V1 stores one registration with a guitar Left
  voice and Hold off and another with an organ and Hold on [V1 02:00–02:48]. V5 uses Left + Hold
  with the style *stopped* (Stop ACMP Off, a pad started by the left-hand chord) as a soft intro
  before starting the rhythm [V5 01:55–02:33].
- Chord Detection Area **Upper**: chords from the right of the Left split, the left hand plays a bass
  line, fingering forced to Fingered\* (no 1+5, 1+8, Cancel). OM p.51, RM p.9.
- **Manual Bass** (Upper only, defaults On when Upper is picked): mutes the Style's bass part and gives
  its voice to the Left part. OM p.51.
- Selecting an Ensemble Voice forces the area back to Lower. OM p.51.
- Storage: all three split points, Chord Detection Area, Manual Bass and Left Hold are in
  Registration; Freeze group Style, except Split Point (Right 3) which is Voice. Parameter Lock
  groups "Split Point" (all three) and "Fingering Type" (with Detection Area). Left Hold is also a
  Song Setup item; none of these are in OTS. DL p.82, DL p.88.

## Manual check

- Split types, constraints, Upper/Manual Bass, Left Hold's behaviour and storage:
  *manual-confirmed*.
- Left Hold in use (organ/pad Left with Hold across chord changes, also with the style stopped):
  *video-confirmed* (V2, V1, V5); consistent with OM p.49.
- Right 3 split use and the clearing gesture: *video-only*, consistent with OM p.49-50.
- Left voice below the Style point when the two are separate: still open (V1 suggests yes).

## yahaha today

- **One split point**, the Genos "Style + Left" point: chord section and Left voice share the keys at
  and below it; Right 1–3 play above (`src/session/chord.rs` `SetSplit`/`MoveSplit`, range MIDI
  24–96; `SplitPage.svelte` says so explicitly). No separate Style vs Left point, **no Right 3
  split** (R3 layers over R1/R2 across the right side).
- Split set by `[`/`]`, the Launchkey Split −/+ pads, the app's strip (drag) and −/+ buttons, and the
  `--split` CLI flag. No "hold label and press a key" capture.
- Left off: keys left of the split play the Right parts in Upper detection and in the Full Keyboard
  types, and give only the chord in Lower detection (README, PR #61 decisions on #31).
- Upper detection with Fingered\* and Manual Bass (defaults on, Left takes the Style Bass voice and
  fader, Left's octave not applied to the bass, Left on/off refused while Manual Bass sounds):
  built (`src/session/chord.rs`, `src/live.rs recompute`, #31/#61).
- **Left Hold: not built.** #31 is closed but its last comment says three split points and Left Hold
  were left open; no issue tracks them now.
- Registration and Parameter Lock: split point and fingering/detection are registrable and lockable
  (`src/api/param_lock.rs`: `SplitPoint`, `FingeringType`).

## Gaps

| Gap | Priority | Suggested next step |
|---|---|---|
| Left Hold missing (button, assignable, Registration/Song Setup item). Three videos use it: an organ/pad Left voice held across chord changes (V2, V1), and a held Left pad with the style stopped as a song intro (V5). | P1 | Small PR: `PartsCmd::SetLeftHold`, a per-part hold flag that sustains Left notes (CC64 on Left's channel, or note-offs held back) until Style stop / Left Hold off; assignable function + Registration section; pad/key |
| Right 3 split point missing (R3 cannot be put on top of R1/R2). Three videos show it as a common registration trick (solo voice on top). | P2 (upper end) | PR: `split_r3` in `Shared`, key routing sends R1/R2 below and R3 above it; constraint Right3 ≥ Left; app strip gets a second marker; Registration (Freeze Voice) |
| Separate Style and Left split points missing (chord-only zone below a Left-voice zone). | P2 | Same PR family as Right 3: split `split` into `split_style` and `split_left` with Left ≥ Style; "Style + Left" stays the default link |
| No "hold the label and press a key" split capture. | P2 | App: a "set from keyboard" toggle that takes the next key-down as the split |
| An Ensemble Voice does not force Lower detection (if yahaha adds Ensemble Voices). | – | Note for whoever builds Ensemble Voices |

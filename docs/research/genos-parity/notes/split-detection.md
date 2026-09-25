# Split points, chord detection area (Upper/Lower), Left Hold, Manual Bass

Topic id: `split-detection` · Pass: 2026-09-25 · Manual: OM p.48-51, OM p.56-57, RM p.9, DL p.82, DL p.88 · yahaha: `docs/genos-features.md#split-points`, `src/parts.rs`, `src/session/keyboard.rs`, `src/session/chord.rs`, `src/live.rs`, `app/src/panels/settings/SplitPage.svelte`

Paraphrased notes only. No transcript text, manual text or frames are committed.

**Video coverage this pass: none.** The four chosen videos were rate-limited (HTTP 429) on the
caption endpoint and had not arrived when this note was written. The note rests on the manuals and
yahaha's code; video-dependent points are marked "pending video".

## Sources

| # | Video (channel) | URL | Timestamps used |
|---|---|---|---|
| V1 | Left voice & splitting (Casper tutorSynth) | https://www.youtube.com/watch?v=WXKmjR68Dz4 | transcript pending |
| V2 | Left Hold (Keyboardamerica) | https://www.youtube.com/watch?v=QLE0ugVpTWU | transcript pending |
| V3 | Right 3 split (Leigh Wilbraham) | https://www.youtube.com/watch?v=8ZRDZFzYzIw | transcript pending |
| V4 | Splitting right tracks | https://www.youtube.com/watch?v=I2YiElHwx-s | transcript pending |

No stills taken for this topic (wanted, still pending: the Split & Fingering window with three
split markers in V1/V3, and the LEFT HOLD lamp in V2).

## Genos behaviour (subtleties a player notices)

- Three independent split points: **Style** (top of the chord section), **Left** (Left voice vs Right
  voices) and **Right 3** (R1/R2 vs R3). Left can't go below Style; Right 3 can't go below Left.
  OM p.49-50.
- "Style + Left" sets the first two to one key, so the chord section and the Left voice share one
  area; setting the points separately gives a chord-only zone at the bottom, then a Left-voice zone.
  OM p.49-50. Whether the Left voice also sounds in the chord-only zone is not clear from the
  diagrams: **pending video** (V1).
- Setting a split: hold the on-screen label and press a key, or L/R controls, or the Data dial.
  OM p.50.
- Right 3 split: with it set, R3 plays only above the Right 3 point and R1/R2 only between Left and
  Right 3 (a solo voice on top, a comp voice below). OM p.49, p.51 diagrams.
- Left part off: a Right voice covers the whole keyboard (outside the chord section when ACMP is on).
  OM p.48, p.56.
- ACMP off + LEFT on: the Left section still yields a chord for Keyboard Harmony and Multi Pad Chord
  Match. OM p.57, RM p.65.
- **Left Hold**: with LEFT on, the Left voice keeps sounding after release; sustained voices hold,
  decaying ones decay slowly as if sustained. Cancelled by stopping the Style/Song or turning Left
  Hold off. Has a panel button and an assignable function. OM p.49, RM p.141.
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

- Everything above is *manual-confirmed* except the Left-voice-in-chord-zone question (*pending
  video*).
- How players use Right 3 split and Left Hold day to day (the "feel" side) is *pending video*.

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
| Left Hold missing (button, assignable, Registration/Song Setup item). Genos players with a strings/pad Left voice over a Style expect the left-hand chord to keep ringing between chord changes. | P1 | Small PR: `PartsCmd::SetLeftHold`, a per-part hold flag that sustains Left notes (CC64 on Left's channel, or note-offs held back) until Style stop / Left Hold off; assignable function + Registration section; pad/key |
| Right 3 split point missing (R3 cannot be put on top of R1/R2). | P2 | PR: `split_r3` in `Shared`, key routing sends R1/R2 below and R3 above it; constraint Right3 ≥ Left; app strip gets a second marker; Registration (Freeze Voice) |
| Separate Style and Left split points missing (chord-only zone below a Left-voice zone). | P2 | Same PR family as Right 3: split `split` into `split_style` and `split_left` with Left ≥ Style; "Style + Left" stays the default link |
| No "hold the label and press a key" split capture. | P2 | App: a "set from keyboard" toggle that takes the next key-down as the split |
| An Ensemble Voice does not force Lower detection (if yahaha adds Ensemble Voices). | – | Note for whoever builds Ensemble Voices |

# Chord detection and fingering types (AI Fingered, AI Full Keyboard, On Bass, Multi Finger)

Topic id: `fingering` · Pass: 2026-09-25 · Manual: OM p.46, RM p.9, RM p.142, RM p.157, DL p.45 · yahaha: `docs/genos-features.md#c2-fingering-types-rm-p9-om-p46`, `src/fingering.rs`, `src/theory.rs`, `src/engine/settle.rs`, `src/live.rs` (`recompute`)

Paraphrased notes only. No transcript text, manual text or frames are committed.

**Video coverage this pass: none.** All seven chosen videos were rate-limited (HTTP 429) on the caption
endpoint and had not arrived when this note was written. Everything below is manual-confirmed or a
reading of yahaha's code; the video-dependent questions are listed as "pending video".

## Sources

| # | Video (channel) | URL | Timestamps used |
|---|---|---|---|
| V1 | Finger mode (Tiho) | https://www.youtube.com/watch?v=GstkJE7Kl6Y | transcript pending |
| V2 | Split & fingering (Keyboardamerica) | https://www.youtube.com/watch?v=CoJs8Uu4mqo | transcript pending |
| V3 | AI Fingered vs On Bass (Keyboard-Akademie) | https://www.youtube.com/watch?v=W2phsGwMgoA | transcript pending |
| V4 | AI Fingered bass inversions (Keyboard-Akademie) | https://www.youtube.com/watch?v=71G1T761qtA | transcript pending |
| V5 | Slash chords (Keyboard-Akademie) | https://www.youtube.com/watch?v=bQ5_yuCd6F4 | transcript pending |
| V6 | Fingering type on steroids (Casper tutorSynth) | https://www.youtube.com/watch?v=mE7v-mY_oI8 | transcript pending |
| V7 | AI Fingered + Parameter Lock (Keyboard-Akademie) | https://www.youtube.com/watch?v=KoJTv4VXNmw | transcript pending |

No stills taken for this topic.

## Genos behaviour (subtleties a player notices)

- Seven types: Single Finger, Multi Finger, Fingered, Fingered On Bass, Full Keyboard, AI Fingered,
  AI Full Keyboard; Fingered\* appears only with Upper detection. RM p.9, OM p.132.
- Fingered always puts the bass on the chord root; Fingered On Bass takes the lowest note of the
  chord section as bass. The manual says this explicitly as a contrast. RM p.9.
- AI Fingered is Fingered plus: fewer than three notes may name a chord, inferred "from the previous
  chord, etc." The inference rule is not given. RM p.9. The manual says nothing about AI Fingered and
  slash/on-bass chords; since it is "basically the same as Fingered", the manual reading is root bass.
  Two of the chosen videos (V3, V4) are specifically about AI Fingered and bass inversions, which
  suggests real behaviour may differ: **pending video**.
- AI Full Keyboard: play anywhere with both hands; not always appropriate depending on arrangement;
  9th, 11th and 13th chords cannot be played. OM p.46, RM p.9.
- Chord Cancel (root + b2 + 2) exists only in Fingered, Fingered On Bass and AI Fingered. OM p.46,
  DL p.45.
- 1+5 and 1+8 are chord types (DL p.45); Fingered\* (Upper detection) has neither, nor Cancel. RM p.9,
  OM p.51.
- A Fingered ⇄ Fingered On Bass toggle exists as an assignable function (pedal or panel button) and
  as a MIDI external-controller function. RM p.142, RM p.157.
- Bass Hold (assignable): freezes the style's bass note across chord changes; does not work in AI Full
  Keyboard. RM p.142.
- Sync Stop cannot be turned on in Full Keyboard or AI Full Keyboard. OM p.66.
- Fingering type is stored in Registration (Freeze group Style) and has a Parameter Lock group
  "Fingering Type", shared with Chord Detection Area (V7's subject). DL p.88, RM p.163.
- Recognition timing: the manuals give no latency, debounce or roll tolerance at all.

## Manual check

- Every bullet above except the AI Fingered bass question is *manual-confirmed* (refs inline).
- AI Fingered with slash chords: *pending video* (manual implies root bass; V3/V4 titles suggest
  otherwise). Manual wins for the spec until a video shows the Genos doing something else.
- Chord settle / recognition latency: *not in manual*; yahaha's 10 ms window is our own choice.
- Factory default fingering type: *not in manual* (no default is printed on RM p.9 or in the DL
  parameter chart).

## yahaha today

- All seven types plus Fingered\* are built (`src/fingering.rs`, module doc lists each rule). Default
  type is Fingered On Bass (`Fingering::default`).
- AI Fingered: three or more pitch classes are plain Fingered; fewer keep the previous chord if they
  fit it, else a dyad is inferred by interval (m3 → minor, M3 → major, m6/M6 → inversions, b7 → 7,
  M7 → maj7, 5th/4th → 1+5, tritone → the dominant nearer the previous chord on the circle of
  fifths, a 2nd → keep), a single note → major (1+8 if doubled). The result is always forced to root
  bass (`ai(...).map(root_bass)`), matching the manual's "same as Fingered".
- AI Full Keyboard: Full Keyboard reading first, else AI inference on the lowest two pitch classes;
  single notes are melody; 9/11/13 tensions dropped (`no_tensions`); a dyad restrike does not count
  as a new strike (#107).
- 1+5, 1+8 and Cancel are recognised with the CASM behaviour described in genos-features §C.3;
  `allows_cancel` restricts Cancel to the three types; Upper forces Fingered\* (`live.rs recompute`).
- Fingered ⇄ On Bass toggle: assignable function `FingeredOnBass` (`src/controllers.rs`).
- Sync Stop refused in Full/AI Full (`allows_sync_stop`).
- Bass Hold: **not built** (no function, no engine state).
- Chord settle: 10 ms default, 0–30 ms settable (`src/engine/settle.rs`, `setChordSettle`); rhythm
  parts never wait; Sync Start fires immediately on the first recognised chord; measured chord →
  first band note under Sync Start is 143–235 µs (README).
- Fingering in Registration and Parameter Lock: the registrables capture fingering
  (`src/session/registration/sections.rs` imports `Fingering`) and Parameter Lock exists
  (`src/session/param_lock.rs`).

## Gaps

| Gap | Priority | Suggested next step |
|---|---|---|
| AI Fingered may honour the lowest note as a slash bass on the real Genos (V3/V4); yahaha forces root bass. If true, AI Fingered users lose inversions/walk-downs. | P1 (if confirmed) | Read V3/V4 when captions arrive; if confirmed, drop `.map(root_bass)` for AI Fingered (maybe only for complete 3+ note chords) behind a test from the video's examples |
| AI Fingered two-note inference rule is our own guess; the Genos rule is undocumented. | P2 | Compare yahaha's `infer_dyad` table with the dyads demonstrated in V3/V6 when available; owner playtest |
| Bass Hold (assignable) missing. | P2 | Small PR: `Function::BassHold` (Toggle/Hold), engine keeps the bass root while on; refuse in AI Full Keyboard |
| Default fingering (yahaha: Fingered On Bass) may not match the Genos factory default. | P2 | Owner question; one-line change if wanted |
| Chord settle 10 ms has no Genos reference; the Genos's own recognition latency is unmeasured. | – | Keep; owner playtest only if rolled chords feel late |

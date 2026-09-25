# Chord detection and fingering types (AI Fingered, AI Full Keyboard, On Bass, Multi Finger)

Topic id: `fingering` · Pass: 2026-09-25 · Manual: OM p.46, RM p.9, RM p.142, RM p.157, DL p.45 · yahaha: `docs/genos-features.md#c2-fingering-types-rm-p9-om-p46`, `src/fingering.rs`, `src/theory.rs`, `src/engine/settle.rs`, `src/live.rs` (`recompute`)

Paraphrased notes only. No transcript text, manual text or frames are committed.

**Video coverage this pass: 3 videos** (V1, V2 and V8 read; V3–V7 still rate-limited, transcripts
pending). V8 is the Scan Genos 2 Q&A assigned to sync-start-stop; its AI Fingered segment is the
key evidence here.

## Sources

| # | Video (channel) | URL | Timestamps used |
|---|---|---|---|
| V1 | Genos Basic Functions explained: Finger Mode (Tiho) | https://www.youtube.com/watch?v=GstkJE7Kl6Y | 01:13–05:00 |
| V2 | Genos Tutorial (Split & Fingering) (Keyboardamerica) | https://www.youtube.com/watch?v=CoJs8Uu4mqo | 01:31–02:46, 03:13–04:19, 05:04–06:08 |
| V3 | AI Fingered vs On Bass (Keyboard-Akademie) | https://www.youtube.com/watch?v=W2phsGwMgoA | transcript pending |
| V4 | AI Fingered bass inversions (Keyboard-Akademie) | https://www.youtube.com/watch?v=71G1T761qtA | transcript pending |
| V5 | Slash chords (Keyboard-Akademie) | https://www.youtube.com/watch?v=bQ5_yuCd6F4 | transcript pending |
| V6 | Fingering type on steroids (Casper tutorSynth) | https://www.youtube.com/watch?v=mE7v-mY_oI8 | transcript pending |
| V7 | AI Fingered + Parameter Lock (Keyboard-Akademie) | https://www.youtube.com/watch?v=KoJTv4VXNmw | transcript pending |
| V8 | Genos 2: Questions You May Be Embarrassed to Ask (Scan Keyboards & Pianos) | https://www.youtube.com/watch?v=nrS_ZQObREk | 02:21–07:44, 08:46–13:09 (stills 09:40, 09:45, 10:08, 11:48, 11:51) |

Stills (private folder): `nrS_ZQObREk_0940_0`, `_0945_0/1`, `_1008_0`, `_1148_0`, `_1151_0/1`. At
09:40 and 11:48 the presenter holds a single key in the left hand and the Home screen's Style box
shows the previous chord (C). Two seconds after he adds a note below it (09:45, 11:51) the chord
readout has become a slash chord over B (a C chord on B; blurry at 480p but clearly two parts
split by a slash). The 10:08 readout is too blurry to read.

## Genos behaviour (subtleties a player notices)

- Seven types: Single Finger, Multi Finger, Fingered, Fingered On Bass, Full Keyboard, AI Fingered,
  AI Full Keyboard; Fingered\* appears only with Upper detection. RM p.9, OM p.132.
- Fingered always puts the bass on the chord root; Fingered On Bass takes the lowest note of the
  chord section as bass. The manual says this explicitly as a contrast. RM p.9.
- AI Fingered is Fingered plus: fewer than three notes may name a chord, inferred "from the previous
  chord, etc." The inference rule is not given. RM p.9. The manual says nothing about AI Fingered and
  slash/on-bass chords; since it is "basically the same as Fingered", the manual reading is root bass.
  **V8 shows otherwise.** In AI Fingered, play a chord, let go except one of its notes, and any note
  added below becomes the bass under the chord in force: C held as a single note plus B gives C/B,
  A held after Am plus G gives Am/G (the "Whiter Shade of Pale" descending bass). V8 also plays an
  E7 over G# with just two notes. [V8 09:23–09:47, 09:59–10:12, 11:38–12:05; stills 09:45, 11:51].
  The presenter pitches it as the way to play slash-chord songs without learning On Bass.
- AI Fingered with one key gives the major chord; two keys (root + minor third) give the minor.
  [V1 03:11–03:37]. Plain Fingered ignores one- or two-key input. [V1 02:43–02:57]
- Multi Finger: one key gives major, three keys are read as a full chord, but in V1 a two-key
  root + minor-third shape changes nothing. [V1 01:51–02:43, 03:50–04:05] V8 demonstrates Multi
  Finger minor with the *nearest* black key to the left (A + the black key just below it = Am) and
  7th with the nearest white key, i.e. the Single Finger shapes are adjacent keys. [V8 03:12–03:51,
  05:09–06:00]
- Presenters describe Multi Finger as the usual shipping setting; V8 says the instruments are
  "generally" set to Multi Finger by default and urges players to switch to AI Fingered.
  [V8 12:55–13:09; V2 01:31, 02:33–02:59 uses Multi Finger for students]
- AI Full Keyboard: play anywhere with both hands; not always appropriate depending on arrangement;
  it never recognises 9th, 11th or 13th chords. OM p.46, RM p.9.
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
  "Fingering Type", shared with Chord Detection Area (V7's subject). DL p.88, RM p.163. V2 says
  the opposite (the fingering type stays put whatever registration you recall) [V2 05:04–06:08]:
  *conflict*; most likely he is describing Freeze or an older firmware. Manual wins.
- Quick access: DIRECT ACCESS + the ACMP button opens the Split & Fingering page. [V8 02:21]
- Recognition timing: the manuals give no latency, debounce or roll tolerance at all.

## Manual check

- Type list, Fingered vs On Bass bass rule, AI Full Keyboard limits, Cancel/1+5/1+8, the toggle,
  Bass Hold, Sync Stop restriction, Registration/Lock: *manual-confirmed* (refs inline).
- AI Fingered slash bass: *video-only, conflicts with the manual's implied reading*. The manual never
  says AI Fingered forces root bass (it says so only of Fingered); V8 demonstrates the lowest
  added note becoming the bass, with the display showing the slash chord. Treat V8 as real
  behaviour; V3/V4 (still pending) should give the exact rule.
- AI Fingered one/two-key results and plain Fingered ignoring dyads: *video-only*, consistent with
  RM p.9.
- Multi Finger ignoring a root + minor-third dyad: *video-only* (V1 only, noisy auto-captions).
- Factory default Multi Finger: *video-only* (V8, V2); not printed in the manuals.
- Fingering in Registration: *conflict* (V2 says no; DL p.88 says yes). Manual wins.
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
  bass (`ai(...).map(root_bass)`), matching the manual's "same as Fingered". Traced for V8's
  examples: B+C after C is a 2nd, so the chord stays C (root bass), where the Genos shows C/B; G+A
  after Am stays Am, where the Genos shows Am/G; a two-note E7 shape over G# gives E7 or E
  with root bass, not E7/G#.
- Multi Finger two keys: Single Finger reading with any key below the root, so C + E♭ reads as
  E♭7 (the root is the highest key, C is a white key below it), where V1 saw no change. Only a
  perfect fifth up from the lowest key is 1+5.
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
| AI Fingered keeps a slash bass on the Genos (V8: C then B+C gives C/B, Am then G+A gives Am/G, two-note E7/G#); yahaha forces root bass, so descending-bass songs and walk-downs are lost in the type V8 tells everyone to use. | P1 | PR: AI Fingered takes the lowest held note as bass when it is not the root: for 3+ notes use the On Bass reading; for 1–2 notes, when the upper note(s) fit the previous chord, keep that chord over the lower note (C/B, Am/G), else infer as now with the lowest note as bass. Tests from V8's three examples; refine with V3/V4 |
| AI Fingered two-note inference rule is our own guess; the Genos rule is undocumented. | P2 | Compare yahaha's `infer_dyad` table with the dyads demonstrated in V3/V6 when available; owner playtest |
| Bass Hold (assignable) missing. | P2 | Small PR: `Function::BassHold` (Toggle/Hold), engine keeps the bass root while on; refuse in AI Full Keyboard |
| Multi Finger two-key shapes: yahaha reads any key below the root (C + E♭ = E♭7); the Genos Single Finger shapes are the nearest key to the left, and V1 saw a root + minor-third dyad do nothing. | P2 | Restrict two-key Multi Finger (and Single Finger) to the adjacent-key shapes, as `single_shape` already does for 3 keys (reach); confirm with V6 when available |
| Default fingering: yahaha Fingered On Bass; presenters say the Genos ships on Multi Finger (V8, V2). | P2 | Owner decision: keep On Bass (a deliberate choice) or match Multi Finger |
| Chord settle 10 ms has no Genos reference; the Genos's own recognition latency is unmeasured. | – | Keep; owner playtest only if rolled chords feel late |

# ChordReadout

## Identity (all stations)

- **Kind:** primitive
- **Built from:** — (uses the pure functions `splitChord` and `chordTones` from `app/src/ui/Stage/format.ts`, Stage.md Components row 1b, and its own `fit.ts`)
- **Purpose:** Shows the chord the band is following, big enough to read from the keys, with its notes and how it was fingered.
- **Boards:**
  - `Stage-Dark.dc.html:148-158` (the display's chord column: "Chord", the 128px "Am7", the tones A C E G with R m3 5 m7, "Fingered"); light: `Stage-Light.dc.html:124-134`. Spec: Stage.md › Display › Chord.
  - The `compact` size (the tall pages' now-playing block): `Channel-Dark.dc.html:150` (the 48px "Am7"); light: `Channel-Light.dc.html:114`. The same block, copied: `Harmony-Dark.dc.html:124` / light `:114`, `Rack-Dark.dc.html:131-135` / light `:113-117`, `Browser-Dark.dc.html:136-137` / light `:120-121`. Spec: Channel.md › Compact block and Kit additions › Compact now-playing block (canonical, CH-D1, CH-D2); Harmony.md Components row 3 and Kit additions › Compact now-playing block (`compactChordSize`).
- **Not this component's job:** no store, no API, no Tauri. It doesn't parse chords or spell notes itself: `splitChord` and `chordTones` (Stage/format.ts, specified in the Stage page spec) do, and this component only lays out what they return. It isn't a control (no click, no focus). In the `compact` size it draws the chord only: the tones, tempo, run dot and section beside it are `CompactNowPlaying`'s, and so is computing the room it gets (`space`). The "held" face (Stage D19) waits for contract change C4 and is not built (D9). No `use:tip` import: the tooltip is the `tip` key plus the `tipAction` the wiring passes (L3).

Its users and the props they pass:

| User | Props |
|---|---|
| Display (Stage.md row 33) | `{ name: chord.name, fingered: chord.fingered, fingeringName: chord.fingeringName, chordTones: keyboard.chordTones, transposeKeyboard: chord.transposeKeyboard, tip: 'display.chord', tipAction }` |
| CompactNowPlaying (Channel row 19, Harmony row 12) | `{ size: 'compact', name: chord.name, tip: 'display.chord', tipAction }` (no `space`: CH-D2, D3) |

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component. Types are local (lint forbids importing `app/src/lib` in `app/src/ui`); field names match `docs/app-api.md`.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `name` | `string \| null` | `null` | `chord.name`: the chord after Keyboard transpose ("Am7", "C#m7b5/G#", "N.C."). `null` or `''`: no chord. |
| `fingered` | `string \| null` | `null` | `chord.fingered`: the chord as fingered, before Keyboard transpose. Shown as "· played {fingered}" only when it is not null and differs from `name` (`display` size only). |
| `fingeringName` | `string` | `''` | `chord.fingeringName` ("Fingered", "Fingered On Bass", "AI Full Keyboard"); `display` size only. |
| `chordTones` | `number[]` | `[]` | `keyboard.chordTones`: pitch classes 0–11 of the chord as fingered; `display` size only. |
| `transposeKeyboard` | `number` | `0` | `chord.transposeKeyboard`, semitones; the tones are moved by it so they belong to `name` (Stage D31); `display` size only. |
| `size` | `'display' \| 'compact'` | `'display'` | `display`: the Stage's 300 × 162 column (label, 128px chord, tone columns, fingering line). `compact`: the tall pages' 48px chord alone, one line (D2). |
| `space` | `number \| undefined` | — | `compact` only: the px the chord may take. Undefined (the canonical tall page, CH-D2): the chord is always 48px and clips at its right edge. Given: it is drawn at `compactChordSize(width, space)` (fit.ts, D3). Ignored by `display`. |
| `tip` | `string \| undefined` | — | The tooltip key (`display.chord`), rendered as `data-tip` on the root; no attribute when undefined (L3). |
| `tipAction` | `Action<HTMLElement, string> \| undefined` | — | The app's `use:tip`, passed in by the wiring. When both it and `tip` are set the root gets `use:tipAction={tip}`; otherwise nothing (L3). |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| — | | |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Pure functions (`app/src/ui/ChordReadout/fit.ts`)

The fit maths, pure, tested in `app/src/ui/ChordReadout/fit.test.ts` (Stage D41, Harmony Check 18). `splitChord`, `chordTones` (and `sectionName`, `knobFraction`) are **not** here: they live in `app/src/ui/Stage/format.ts` (Stage.md row 1b) and are specified in the Stage page spec; this component imports them and never re-implements them.

```ts
/** The Stage chord's size in px: 128 when its width at 128px fits 300, else shrunk, never below 64 (Stage › Chord, D20). */
export function displayChordSize(width: number): number
/** The compact chord's size in px (Harmony.md Kit additions): min(base, max(32, floor(base × space / width))). */
export function compactChordSize(width: number, space: number, base = 48): number
```

- `displayChordSize(width)` = `128` when `width <= 300` or `width` isn't a positive finite number (jsdom measures 0), else `max(64, floor(128 × 300 / width))`.
- `compactChordSize(width, space, base)` = `base` when `width` isn't a positive finite number, else `min(base, max(32, floor(base × space / width)))`.

| Call | Result |
|---|---|
| `displayChordSize(0)`, `displayChordSize(180)`, `displayChordSize(300)` | `128` |
| `displayChordSize(301)` | `127` |
| `displayChordSize(450)` | `85` |
| `displayChordSize(600)` | `64` |
| `displayChordSize(900)` | `64` (floor at 64) |
| `compactChordSize(100, 150)` | `48` |
| `compactChordSize(200, 150)` | `36` |
| `compactChordSize(300, 150)` | `32` |
| `compactChordSize(0, 150)` | `48` |

**Measuring** (both sizes, when they fit): `width` is the `scrollWidth` of a hidden copy of the two runs drawn at the base size (128 or 48), so the measurement never depends on the size currently shown: a `span` with `aria-hidden="true"`, `position: absolute`, `visibility: hidden`, `white-space: nowrap`, `pointer-events: none`, the same classes as the visible runs at scale 1. It is re-measured in an `$effect` after each change of `name` (and of `space` in `compact`), and once more when `document.fonts.ready` resolves (DM Sans changes the width; D7). No timer, no `ResizeObserver`.

### Visual rules

- **Tokens used:** `--a`, `--t`, `--m`, `--d`, `--ba2`, `--ba`, `--font-sans`, `--text-12`, `--text-14`, `--text-20`, `--weight-thin`, `--weight-light`, `--weight-regular`, `--space-4`, `--space-6`, `--space-16`, and the new tokens below.

#### New tokens

Not in `app/src/ui/tokens/*` today; they land in the orchestrator's tokens contract PR before the build (L1). The builder uses them by name.

| Token | Dark | Light | Used for |
|---|---|---|---|
| `--text-128` | `128px` | `128px` | the Stage chord (kit › Tokens, "Scale tokens to add") (scale.css) |
| `--text-48` | `48px` | `48px` | the compact chord (Channel row 0, Harmony › Tokens to add) (scale.css) |
| `--tracking-128` | `-6px` | `-6px` | the Stage chord's base run (scale.css; named like WaitingChip's `--tracking-36`, D8) |
| `--tracking-48` | `-2px` | `-2px` | the compact chord's base run (Harmony calls it `--ls-chord`; D8) (scale.css) |

- **Element (`display`):** a `div`, the column: `width: 300px`, `height: 162px`, `box-sizing: border-box`, `display: flex`, `flex-direction: column`, `overflow: hidden`, `font-family: var(--font-sans)`, `font-variant-numeric: tabular-nums`. Three rows:
  1. **Label** "Chord": `--text-14` / 400 `--m`, height 16, line-height 16.
  2. **Chord** (`margin-top: var(--space-4)`): height 104, line-height 104px, `white-space: nowrap`, `color: var(--a)`, `text-shadow: var(--ba2)`. Two `span`s, `data-run="base"` and `data-run="ext"` (the latter omitted when the extension is empty): base `--weight-light` (300) with letter-spacing `--tracking-128` scaled; extension `--weight-thin` (200), letter-spacing 0. Size: the root sets `--chord-scale: <displayChordSize(width) / 128>` inline; the chord row's `font-size: calc(var(--text-128) * var(--chord-scale))` and the base run's `letter-spacing: calc(var(--tracking-128) * var(--chord-scale))`. The line-height stays 104px at every size, so nothing below moves when it shrinks. The base run and extension come from `splitChord(name)`; "N.C." is all base.
  3. **Tones row** (`margin-top: var(--space-6)`): height 32, `display: flex`, `align-items: flex-end`, `gap: var(--space-16)`, `white-space: nowrap`, `overflow: hidden`. One column per entry of `chordTones(chordTones, transposeKeyboard, root)` (root first, at most six; `root` is the chord's root as `splitChord` gives it, per the Stage page spec's signature): `data-part="tone"`, a column `min-width: 24px` that grows for "C#" or "Eb" (Stage D44), centred: the note name (`--text-20` / 300, line-height 18, `--t`) over its interval (`--text-12` / 400, line-height 14, `--m`). Then the **fingering line** (`data-part="fingering"`): `--text-14` / 400 `--m`, line-height 16, `margin-left: var(--space-4)` (20px after the last column with the gap), `min-width: 0`, `overflow: hidden`, `text-overflow: ellipsis`: `fingeringName`, or "{fingeringName} · played {fingered}" when `fingered` is not null and differs from `name` ("Fingered · played Am7").
- **No chord (`display`):** `name` null or `''`: the chord row reads "—" (U+2014) in `--d`, weight 300, at `--text-128`, no `text-shadow`, `data-contrast="dim"` (Stage D47, kit › Dimmed text); no `data-run` spans and no tone columns; the fingering line alone at the row's left (`margin-left: 0`) with no "played" part.
- **Element (`compact`):** a `div`, one line: `display: block`, `white-space: nowrap`, `flex: 0 1 auto`, `min-width: 0`, `overflow: clip` (not `hidden`, so the element keeps its text baseline in its parent's baseline row, D5), no `text-overflow`, `font-family: var(--font-sans)`, `font-size: calc(var(--text-48) * var(--chord-scale))` (`--chord-scale` 1 unless `space` is given), line-height 48px, `color: var(--a)`, `text-shadow: var(--ba)`. The same two runs: base 300 with `letter-spacing: calc(var(--tracking-48) * var(--chord-scale))`, extension 200 with letter-spacing 0. No label, no tones, no fingering line. No chord: "—" in `--d`, no shadow, `data-contrast="dim"`.
- **Size:** `display` 300 × 162 fixed (the display grid's chord column); `compact` 48 tall, as wide as its text, shrinkable to 0 by its flex parent and clipped at its right edge.
- **States drawn by:** chord / no chord (the "—"); transposed (the "· played" part); long chord (the scale under 1, `display`; clipped, `compact`). No focus (not focusable), no hover, no pressed.
- **Type:** DM Sans, tabular numerals, the chord as the state sends it (no case change).
- **Test hooks:** the root carries `data-size="display|compact"`; the runs `data-run="base|ext"`; each tone column `data-part="tone"` with its note in `[data-part="note"]` and its interval in `[data-part="interval"]`; the fingering line `data-part="fingering"`; the no-chord dash `data-part="none"` and `data-contrast="dim"`.
- **Contrast (AA, `tokens/contrast.test.ts`):** `--a` on `--g` (the chord; 128px and 48px are large text, 3:1; row "accent text on the ground", dark 7.80 / light 5.95, already proposed by WaitingChip), `--t` on `--g` (note names, 20px light: normal text, 4.5:1; existing row), `--m` on `--g` (label, intervals, fingering; existing row). `--d` (no-chord dash) is exempt (D47). No new failing pair.
- **Motion:** none. The size changes only when `name`, `space` or the fonts change.

### Accessibility

- **Role and name (`display`):** the root is `role="img"` (a bare `aria-label` on a `div` fails axe's `aria-prohibited-attr`) with `aria-label` "Chord {name}: {note names joined by spaces}, {fingeringName}" ("Chord Am7: A C E G, Fingered"), plus ", played {fingered}" when the fingering line shows it ("Chord Bm7: B D F# A, Fingered, played Am7"); no chord: "No chord, {fingeringName}". Every child is `aria-hidden="true"` (the hidden measuring copy too).
- **Role and name (`compact`):** no role and no `aria-label`; the runs are read in place ("Am7"), and the hidden measuring copy is `aria-hidden`. Its parent's group label ("Now playing: …, chord Am7 …") is the summary (D6).
- **Keyboard:** none; not focusable.
- **Tooltip id:** `display.chord` (exists in `app/src/help/tooltips.ts`), on the root in both sizes, through `tip` and `tipAction` (L3).

## Stories (Story station)

Title `Primitives/ChordReadout`, `layout: 'centered'`. Every story renders in dark and light (the toolbar theme). The meta's `args` are `{ tip: 'display.chord', tipAction: fn() }` (L3). Args come from the Stage board fixture (`chord`: name "Am7", fingered "Am7", fingeringName "Fingered", transposeKeyboard 0; `keyboard.chordTones` [9, 0, 4, 7]).

**Controls (argTypes):** `name`, `fingered`, `fingeringName` text (an empty text is no chord); `chordTones` an object control; `transposeKeyboard` a number; `size` a select of `display` / `compact`; `space` a number (cleared = undefined); `tip` text; `tipAction` an action.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ name: 'Am7', fingered: 'Am7', fingeringName: 'Fingered', chordTones: [9, 0, 4, 7], transposeKeyboard: 0 }` | the Stage display's chord column: "Chord", 128px violet "Am" (300) "7" (200) with its glow in dark, A C E G over R m3 5 m7, "Fingered" | `Board-{dark,light}.png` (Stage 49,169 300×162) | `getByRole('img', { name: 'Chord Am7: A C E G, Fingered' })`; `[data-run="base"]` text "Am", `[data-run="ext"]` text "7"; the four `[data-part="tone"]` read A/R, C/m3, E/5, G/m7; `[data-part="fingering"]` text "Fingered"; the root has `data-tip="display.chord"` and `tipAction` was called with the root and `'display.chord'` |
| `Transposed` | `{ name: 'Bm7', fingered: 'Am7', fingeringName: 'Fingered', chordTones: [9, 0, 4, 7], transposeKeyboard: 2 }` | Keyboard transpose +2: the tones of Bm7, the played chord on the fingering line | — (no board draws it) | tones read B, D, F#, A; fingering text "Fingered · played Am7"; the img's name is "Chord Bm7: B D F# A, Fingered, played Am7" (Stage Check 5) |
| `Flats` | `{ name: 'Ebm7', fingered: 'Ebm7', fingeringName: 'Fingered', chordTones: [3, 6, 10, 1] }` | flat spelling (root written with b): Eb Gb Bb Db, wider columns | — | tones read Eb, Gb, Bb, Db; base run "Ebm", extension "7" |
| `SharpNinth` | `{ name: 'C7(#9)', fingered: 'C7(#9)', fingeringName: 'Fingered', chordTones: [0, 4, 7, 10, 3] }` | a minor and a major third together: the minor reads #9 (Stage D20) | — | five tone columns; one interval reads "#9" and one "3" |
| `SixTones` | `{ name: 'C13', fingered: 'C13', fingeringName: 'AI Full Keyboard', chordTones: [0, 4, 7, 10, 2, 9] }` | the most columns, the fingering line cut with an ellipsis if it runs out of room | — (judged by Inspect) | six `[data-part="tone"]` |
| `LongChord` | `{ name: 'C#m7b5/G#', fingered: 'C#m7b5/G#', fingeringName: 'Fingered', chordTones: [1, 4, 7, 11] }` | the chord shrunk to fit its 300px on one line (Stage › Checks, story checks) | — (no board has a long chord; Inspect judges it fits and doesn't wrap) | base run "C#m", extension "7b5/G#"; the chord row's text has no line break (jsdom measures 0, so the scale stays 1 there; the fit maths is `fit.test.ts`) |
| `NoChord` | `{ name: null, fingered: null, fingeringName: 'Fingered', chordTones: [] }` | "—" in the dimmed grey, no tones, "Fingered" alone | — | `getByRole('img', { name: 'No chord, Fingered' })`; `[data-part="none"]` has `data-contrast="dim"`; no `[data-run]`, no `[data-part="tone"]` |
| `NoChordCancel` | `{ name: 'N.C.', fingered: 'N.C.', fingeringName: 'Fingered', chordTones: [] }` | the cancel chord: all base run, no tones | — | `[data-run="base"]` text "N.C."; no `[data-run="ext"]` (Stage Check 6) |
| `Compact` | `{ size: 'compact', name: 'Am7' }` | the tall pages' 48px chord, `--ba` glow in dark, one line | — (D4) | `data-size="compact"`; base "Am", extension "7"; no `img` role, no `[data-part="tone"]`; `data-tip="display.chord"` |
| `CompactNoChord` | `{ size: 'compact', name: null }` | "—" dimmed, no glow | — | `[data-part="none"]` with `data-contrast="dim"` |
| `CompactClipped` | `{ size: 'compact', name: 'C#m7b5/G#' }`, a decorator giving a 120px-wide flex row | CH-D2: the chord keeps 48px and is cut at its right edge, no ellipsis | — (judged by Inspect) | the root has no inline `--chord-scale` other than 1 |
| `CompactFitted` | `{ size: 'compact', name: 'C#m7b5/G#', space: 150 }` | the opt-in fit (D3): shrunk toward 32px in a real browser | — (judged by Inspect) | — (jsdom measures 0: scale 1) |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in the dark and light render. The `Board` crop is exactly the column's box (L6); the chord's glow that bleeds past it is judged in the Display's crop.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`); `fit.test.ts` passes every row of the fit table.
- `npm run shots -- ChordReadout` passes: `Board` scores at most 0.02 against its crop (or the Inspect agent judges the difference render noise), and axe finds no violation on any story (the no-chord dash carries `data-contrast="dim"`, D47). The Inspect agent looks at `LongChord` (fits 300, one line), `CompactClipped` (clipped, 48px) and `CompactFitted` (smaller) by eye.
- `splitChord` and `chordTones` are imported from `app/src/ui/Stage/format.ts`, not re-implemented; the component has no timer.
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · Pure helpers stay in Stage/format.ts.** `splitChord` and `chordTones` are Stage.md row 1b's, in `app/src/ui/Stage/format.ts`, specified by the Stage page spec; Channel.md's guess of `app/src/ui/ChordReadout/chord.ts` is superseded by Stage.md's path. Only the fit maths (`fit.ts`) belongs to this folder.
- **D2 · One component, two sizes.** The tall pages' compact chord is `size: 'compact'` of this component (Channel row 17, Harmony row 3, Rack row 18, Browser row 15), so the runs, the no-chord dash and the fit share one implementation; `compact` draws the chord only, and the tones beside it are CompactNowPlaying's (Channel and Harmony both put them in the block).
- **D3 · The compact fit follows Channel (CH-D2).** Channel is canonical for the tall-page frame (kit.md line 11, CH-D1): the compact chord never shrinks and clips at its right edge, the tones giving way first. Harmony's `compactChordSize` is still built in `fit.ts` (its Check 18) and reachable through the optional `space` prop, but CompactNowPlaying passes no `space`, so the default is CH-D2's; using the fit is a one-prop change if long chords prove common (Channel Follow-ups). Rack's RK-D3 agrees with Channel.
- **D4 · No crop for `compact`, and none of the tall-page boards cut here.** `crops/` holds only the Stage `Board` pair; the compact chord's width depends on its text and it is judged in CompactNowPlaying's crop and the Channel page's `Board` shot.
- **D5 · `overflow: clip` for the compact chord.** `overflow: hidden` on a flex item can move its baseline to its bottom edge in the compact block's baseline row; `clip` cuts the same way without making a scroll container, so the chord stays on the tones' and section's baseline.
- **D6 · Compact is read in place.** In `compact` the readout has no role or `aria-label`; the runs read as "Am7" and the parent block's group label carries the summary, so a screen reader doesn't hear the chord twice as an image.
- **D7 · Measure a hidden copy at the base size.** Both fits measure a hidden copy at 128 (or 48) px, never the shown element, so a shrunk chord can't feed back into its own measurement; the re-measure on `document.fonts.ready` covers the first paint with the fallback font. jsdom measures 0, which both functions treat as "fits".
- **D8 · Letter-spacing tokens follow WaitingChip.** The lane names letter-spacing `--tracking-<size>` (`--tracking-36` exists in WaitingChip's spec), so the chord's are `--tracking-128` (−6px) and `--tracking-48` (−2px); Harmony's `--ls-chord` is the same −2px under the lane's name, and the tokens PR adds only `--tracking-48`.
- **D9 · No "held" face yet.** Stage D19's dimmed chord with "held" needs `chord.held` (contract change C4, not landed); no `held` prop is added until then, so no story draws a state the engine can't send.
- **D10 · "Played" in the label.** Stage.md's `aria-label` template has no "played" part; when the fingering line shows "· played {fingered}" the label adds ", played {fingered}" so a screen reader hears what the eye sees.
- **D11 · The tone row never wraps.** Six columns plus a long fingering name can pass 300px; the row is `nowrap` with `overflow: hidden` and the fingering line ends in an ellipsis, so the column keeps its 162px (Stage › Chord leaves no room for a second line).
- **D12 · L3, tooltip props; not focusable.** The readout takes `tip` and `tipAction` and applies them to its root in both sizes (`display.chord` exists today); it is not a control, so it has no `Focused` story and no `aria-disabled` (L5 doesn't apply).
- **D13 · L1, new tokens.** `--text-128`, `--text-48`, `--tracking-128` and `--tracking-48` land in the orchestrator's tokens contract PR; this folder edits no token and adds no contrast row (its pairs are existing or WaitingChip's `--a` row).

Follow-ups: `display.chord`'s tooltip body still says the fingered chord is "shown small underneath"; it now follows the fingering name on the tones row ("· played Am7"), a one-line body fix with the next tooltips contract change. The held face (D9) once C4 lands.

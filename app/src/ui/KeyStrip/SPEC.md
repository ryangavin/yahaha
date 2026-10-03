# KeyStrip

## Identity (all stations)

- **Kind:** primitive
- **Built from:** —
- **Purpose:** Shows the keyboard under the player's hands: which keys they hold and on which part, where the split is, and where chord detection listens.
- **Boards:** `Stage-Dark.dc.html:392-403` (markup), `:516-538` (the data script that computes whites and blacks); light: `Stage-Light.dc.html:368-379`, `:487-509`. Crop box: Stage `24,820 1392×56`.
- **Not this component's job:** no store, no API, no Tauri: the range, held keys, split and detection come in as props. It doesn't play notes (the keys are a picture, as today), isn't focusable and has no pointer behaviour. It doesn't pick the range (the wiring does: `ui.keyRange`, else the connected Launchkey's). It doesn't draw chord tones (they moved to the display). It never imports `use:tip`: the wiring passes it as `tipAction` (L3). It doesn't decide which hand detection reads (the wiring passes `detectionLeft`).

## API (Component station)

### Props

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `range` | `49 \| 61 \| 88` | `61` | How many keys: 49 = C1–C5 (MIDI 36–84), 61 = C1–C6 (36–96), 88 = A-1–C7 (21–108). Yamaha numbering, C3 = 60. |
| `held` | `{ note: number; parts: number[]; zone?: 'left' \| 'right' }[]` | `[]` | The keys held, as the state's `keyboard.held` gives them (`HeldNote`). `note` is a MIDI note; `parts` the keyboard parts sounding it (0 Right 1, 1 Right 2, 2 Right 3, 3 Left), empty for a key that only feeds chord detection; `zone` the hand the engine put it in, which the `aria-label` reads (absent: `note <= split` is left, D2). Notes outside the range aren't drawn but still count in the `aria-label`. |
| `split` | `number` | `54` | The split point (`keyboard.leftSplit`), a MIDI note: keys at or below it are the left zone. 54 is F#2. |
| `detection` | `[number, number] \| null` | `null` | The keys chord detection reads, `[lo, hi]` MIDI notes inclusive (`keyboard.detection`); clipped to the range by `detectionBox`. Null, or nothing left after clipping: no line. |
| `detectionLeft` | `boolean` | `false` | Detection is the left hand (Lower): the line is teal with its glow. False (Upper, Full Keyboard): accent, no glow. |
| `tip` | `string \| undefined` | — | The tooltip key (`keystrip.keys`), rendered as `data-tip` on the root; no attribute when undefined (L3). |
| `tipAction` | `Action<HTMLElement, string> \| undefined` | — | The app's `use:tip`, passed in by the wiring; applied as `use:tipAction={tip}` on the root when both are set (L3). Stories pass `fn()`. |

There are no other props: the strip has no states beyond what these draw.

### Events

| Callback | Fires when | Payload |
|---|---|---|
| — | | |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### From the state

What the page wiring passes (the Stage lane transcribes this):

| Prop | From |
|---|---|
| `range` | `rangeFor(ui.keyRange, app.state.io.inputs)` (`app/src/panels/keystrip/keyboard.ts`): `ui.keyRange` if set, else the connected Launchkey's size, else 61 (Stage.md D25). `ui.keyRange` is app-only: `UiStore.keyRange` in `app/src/lib/store.svelte.ts` (`KeyRange \| null`, kept in `localStorage` as `yahaha.keys`), not part of `AppState`. |
| `held` | `keyboard.held` as is (with its `zone`) |
| `split` | `keyboard.leftSplit` |
| `detection` | `keyboard.detection` as is (the component clips it). In `AppState` it is always a pair (`KeyboardState.detection: [number, number]`); the prop's `null` exists for stories and for a page without state. |
| `detectionLeft` | `!chord.upper && chord.fingering !== 'fullKeyboard' && chord.fingering !== 'aiFullKeyboard'` (Stage.md D10: the left hand is Lower and not a Full Keyboard type; D9) |
| `tip` | `'keystrip.keys'` |
| `tipAction` | the app's `tip` action (`app/src/lib`) |

### Geometry: `app/src/ui/KeyStrip/keys.ts`

Pure functions, no DOM; the component computes every position with them and writes it as an inline `left` (and, for white keys and the detection line, `width`) in px, so vitest can read `style.left` / `style.width`; black keys take their size from `--black-key-width` / `--black-key-height`. All x values are in px from the inside left edge of the frame (the inner box is 1390 × 54). These constants are exported:

```ts
export type KeyRange = 49 | 61 | 88
export const RANGES: Record<KeyRange, [number, number]> = { 49: [36, 84], 61: [36, 96], 88: [21, 108] }
export const INNER_WIDTH = 1390
export const BLACK_WIDTH = 22
export const BLACK_HEIGHT = 32
```

`RANGES` repeats `app/src/panels/keystrip/keyboard.ts` `RANGES` (the library can't import from `panels`, which imports API types); `keys.test.ts` asserts the literal values above. Below, `[lo, hi] = RANGES[range]` and `W = whiteWidth(range)`.

| Function | Returns | Rule |
|---|---|---|
| `isBlack(note)` | `boolean` | pitch class `((note % 12) + 12) % 12` in {1, 3, 6, 8, 10}. |
| `whiteCount(range)` | `number` | white keys from `lo` to `hi` inclusive: 49 → 29, 61 → 36, 88 → 52. |
| `whiteWidth(range)` | `number` | `INNER_WIDTH / whiteCount(range)`, unrounded: 49 → 47.931…, 61 → 38.6111…, 88 → 26.7307…. |
| `whitesUpTo(note, range)` | `number` | the number of white keys in `[lo, min(note, hi)]`; 0 when `note < lo`. |
| `whiteKeys(range)` | `{ note, x, w, label }[]` | each white key low to high, `i` its index: `x = i · W`, `w = W`, `label = noteName(note)` for a C, else `''`. |
| `blackKeys(range)` | `{ note, x, w }[]` | each black key low to high, centred on the boundary after the white below it: `x = whitesUpTo(note, range) · W − 11`, `w = 22`. |
| `keyX(note, range)` | `number \| null` | the key's left edge: a white's `x` or a black's `x`, as above; null when `note < lo` or `note > hi` (not drawn). `whiteKeys` and `blackKeys` only ever return keys in `[lo, hi]`; the component draws only those, and matches `held` against them by `note`, so a held note outside the range finds no key and draws nothing. |
| `boundaryX(note, range)` | `number` | the x of the boundary just above `note` (where a split after that key is drawn): `whitesUpTo(note, range) · W`. So below the range it is 0, at or above `hi` it is 1390, and a white key followed by a black one shares its boundary with that black key (its centre), as `boundary()` in `keyboard.ts` does. |
| `splitX(split, range)` | `number \| null` | `Math.floor(boundaryX(split, range))`, the split marker's left edge; null when `split < lo` or `split >= hi` (nothing to divide: no marker). |
| `detectionBox(detection, range)` | `{ x, w } \| null` | null for a null `detection`. Else clip: `a = max(dlo, lo)`, `b = min(dhi, hi)`; null when `a > b`. `x = Math.floor(boundaryX(a − 1, range))`, `w = Math.floor(boundaryX(b, range)) − x`. |
| `noteName(note)` | `string` | Yamaha numbering, spelled as the engine names notes: `NAMES[pc] + (Math.floor(note / 12) − 2)`, with `NAMES = ['C', 'C#', 'D', 'Eb', 'E', 'F', 'F#', 'G', 'Ab', 'A', 'Bb', 'B']` (the same as `keyboard.ts`). |
| `pcName(note)` | `string` | `NAMES[pc]`, no octave. |
| `heldHue(parts)` | `'r1' \| 'r2' \| 'r3' \| 'l' \| 'm'` | the first part's hue: `0 → 'r1'`, `1 → 'r2'`, `2 → 'r3'`, `3 → 'l'`; empty `parts` (or an unknown part) → `'m'`. |
| `keysLabel(range, split, held)` | `string` | the strip's `aria-label`, below. |

Floors: the two 2px lines land on whole pixels so they stay crisp; the keys keep their fractional x and width (as the board's `toFixed(2)`).

**The `aria-label`** (`keysLabel`), clauses joined with ", ":

1. `Keys: split {noteName(split)}`: always, with the octave number.
2. `left hand {names}`: only when some held note is `<= split`. The names are `pcName` of those notes, low to high, separated by single spaces, a name already given dropped: **no octave numbers** (the left hand gives the chord, so its pitch classes are what matter).
3. `right hand {names}`: only when some held note is `> split`. The names are `noteName` of those notes, low to high, separated by single spaces, every one kept: **with octave numbers** (Yamaha, so 76 is E4).
4. `{range} keys`.

Hands go by the split as drawn (`note <= split`), not by the held note's `zone`, so the label always agrees with the picture. Held notes outside the range still count. `keysLabel` doesn't trust the order of `held` (the state says "low to high", a story may not): it sorts a copy by `note` ascending first, and a MIDI note listed twice is read once (in both hands).

**Worked examples** (each a vitest assertion in `keys.test.ts`):

- `whiteCount(61)` 36, `whiteCount(49)` 29, `whiteCount(88)` 52.
- `whiteKeys(61)[0]` → `{ note: 36, x: 0, w: 38.611…, label: 'C1' }`; `[7]` → `{ note: 48, x: 270.277…, label: 'C2' }`; `[35]` → `{ note: 96, x: 1351.388…, label: 'C6' }`; the non-empty labels in order are C1, C2, C3, C4, C5, C6.
- `whiteKeys(88)`: `[0]` is note 21 (A-1, label `''`), `[2]` is note 24 at `x` 53.461…, label `'C0'`; labels C0…C7. `whiteKeys(49)` labels C1…C5.
- `blackKeys(61)`: 25 keys; `[0]` → `{ note: 37, x: 27.611…, w: 22 }`; F#2 (54) at `x` 413.722… (`11 · W − 11`). `blackKeys(88)`: 36 keys, `[0]` is note 22 at 15.730…. `blackKeys(49)`: 20 keys.
- `keyX(43, 61)` (G1) → 154.444…; `keyX(54, 61)` → 413.722…; `keyX(35, 61)` and `keyX(97, 61)` → null; `keyX(21, 88)` → 0.
- `boundaryX(54, 61)` → 424.722…; `boundaryX(53, 61)` → 424.722… (F2's boundary is F#2's centre); `boundaryX(52, 61)` → 386.111…; `boundaryX(35, 61)` → 0; `boundaryX(96, 61)` and `boundaryX(127, 61)` → 1390.
- `splitX(54, 61)` → 424; `splitX(54, 49)` → 527; `splitX(54, 88)` → 534 (20 whites from A-1 to F2: `floor(20 · 26.7307…)`); `splitX(30, 61)` → null; `splitX(96, 61)` → null.
- `detectionBox([0, 54], 61)` → `{ x: 0, w: 424 }`; `detectionBox([55, 127], 61)` → `{ x: 424, w: 966 }`; `detectionBox([0, 127], 61)` → `{ x: 0, w: 1390 }`; `detectionBox([100, 127], 61)` → null; `detectionBox(null, 61)` → null.
- `noteName(54)` `'F#2'`, `noteName(60)` `'C3'`, `noteName(76)` `'E4'`, `noteName(21)` `'A-1'`, `noteName(70)` `'Bb3'`; `pcName(43)` `'G'`.
- `heldHue([3])` `'l'`, `heldHue([0, 3])` `'r1'`, `heldHue([1])` `'r2'`, `heldHue([])` `'m'`.
- `keysLabel(61, 54, boardKeys.held)` → `"Keys: split F#2, left hand G A C E, right hand E4 A4, 61 keys"` (Stage.md Check 18).
- `keysLabel(61, 54, [])` → `"Keys: split F#2, 61 keys"`.
- `keysLabel(88, 54, [{ note: 36, parts: [3] }, { note: 48, parts: [3] }, { note: 52, parts: [3] }])` → `"Keys: split F#2, left hand C E, 88 keys"` (C1 and C2 both read C, once).
- `keysLabel(49, 54, [{ note: 60, parts: [0] }, { note: 64, parts: [0] }, { note: 67, parts: [0] }])` → `"Keys: split F#2, right hand C3 E3 G3, 49 keys"`.
- `keysLabel(61, 54, [{ note: 81, parts: [0] }, { note: 43, parts: [3] }, { note: 76, parts: [0] }, { note: 76, parts: [1] }])` → `"Keys: split F#2, left hand G, right hand E4 A4, 61 keys"` (sorted; 76 twice read once).

### Visual rules

- **Tokens used:** existing: `--line`, `--g`, `--keyline`, `--solid-ink`, `--r1`, `--r2`, `--r3`, `--l`, `--m`, `--a`, `--bl`, `--t`, `--radius`, `--font-mono`, `--text-11`, `--weight-regular`, `--line-width`, `--space-6`. New (below): `--key-white`, `--key-white-left`, `--key-black`, `--key-black-left`, `--key-black-ring`, `--key-black-ring-left`, `--key-label`, `--key-glow-mix`.
- **New tokens** (kit.md › Tokens to add). They land with the tokens contract PR (the orchestrator), not in this component's PR; this PR never edits `app/src/ui/tokens/*` or `contrast.test.ts`.

  | Token | Dark | Light | Type |
  |---|---|---|---|
  | `--key-white` | `var(--grey-09)` (#161616, exists) | `var(--paper-98)` (new palette step #fbfbf9) | colour |
  | `--key-white-left` | `var(--teal-05)` (new #0d1a17) | `var(--teal-95)` (new #e6f1ee) | colour |
  | `--key-black` | `var(--black)` (#000) | `var(--grey-11)` (new #1b1b1b) | colour |
  | `--key-black-left` | `var(--black)` | `var(--teal-13)` (new #1a2a26) | colour |
  | `--key-black-ring` | `inset 0 0 0 var(--line-width) var(--grey-18)` (new #2e2e2e) | `none` | a whole `box-shadow` value |
  | `--key-black-ring-left` | `inset 0 0 0 var(--line-width) var(--teal-17)` (new #1d3a33) | `none` | a whole `box-shadow` value |
  | `--key-label` | `var(--d)` (#4d4d4d) | `var(--m)` (#646464) | colour |
  | `--key-glow-mix` | `45%` | `0%` | a `<percentage>`, used only as the second stop of `color-mix()`; `0%` makes the glow fully transparent, so light draws none |

  New palette steps for `palette.css`: `--paper-98` #fbfbf9, `--teal-05` #0d1a17, `--teal-95` #e6f1ee, `--grey-11` #1b1b1b, `--teal-13` #1a2a26, `--grey-18` #2e2e2e, `--teal-17` #1d3a33.
- **Component geometry** (axiom 2, declared once on the root, used by every rule below; no other literal sizes): `--strip-width: 1392px`, `--strip-height: 56px`, `--black-key-width: 22px`, `--black-key-height: 32px`, `--black-key-radius: 3px`, `--strip-line: 2px` (the detection line's height and the split marker's width), `--key-glow: 10px`, `--key-label-line: 14px`. `keys.ts`'s `INNER_WIDTH` (1390 = `--strip-width` minus two `--line-width`) and `BLACK_WIDTH` (22) are the same numbers for the position maths; `keys.test.ts` pins them.
- **Frame:** the root `section` is `--strip-width` × `--strip-height`, `box-sizing: border-box`, `position: relative`, a `--line-width` solid `--line` border, radius `--radius`, `--g` background, `overflow: hidden` (clips the keys, their glows and both lines to the rounded inner box), `cursor: default`. Its one child is the key layer: a `div` with `aria-hidden="true"`, `position: absolute; inset: 0` (the 1390 × 54 inner box); every key and both lines are absolutely positioned inside it, from its top-left.
- **White keys** (from `whiteKeys(range)`, first in the layer, low to high): a `div`, `position: absolute`, top 0, inline `left` x and `width` w (px), height 100% (54), `box-sizing: border-box`, `border-right: var(--line-width) solid var(--keyline)` (kept on a held key too, as the board draws it); fill `--key-white`, or `--key-white-left` when `note <= split`. A C carries `data-label="<noteName>"` and draws it as `::after { content: attr(data-label) }`; the key is `display: flex; flex-direction: column; justify-content: flex-end; align-items: center; padding-bottom: var(--space-6)` (the board's flex-end layout), so the label sits centred 6px above the bottom; `--font-mono` `--text-11` / `--weight-regular`, `line-height: var(--key-label-line)`, colour `--key-label`. Generated content isn't text in the accessibility tree, and the layer is `aria-hidden` anyway (see Contrast).
- **Black keys** (from `blackKeys(range)`, after every white): a `div`, `position: absolute`, top 0, inline `left` x (px), `--black-key-width` × `--black-key-height`, `border-radius: 0 0 var(--black-key-radius) var(--black-key-radius)`; fill `--key-black` with `box-shadow: var(--key-black-ring)`; at or below the split, `--key-black-left` with `var(--key-black-ring-left)`. In light both rings are `none` (valid `box-shadow`), so a light black key is a flat dark grey (#1b1b1b, left #1a2a26) with no edge. No label.
- **Held keys** (a drawn key whose `note` equals some `held[i].note`; the first such entry gives its parts): fill `var(--<heldHue(parts)>)` (`--r1`, `--r2`, `--r3`, `--l`, or `--m` for no parts), whatever its zone; glow `box-shadow: 0 0 var(--key-glow) color-mix(in srgb, var(--<hue>) var(--key-glow-mix), transparent)` (on a held black key this replaces its ring); a held C's label colour `--solid-ink`. Later keys paint over an earlier key's glow (the board's order).
- **Detection line** (from `detectionBox(detection, range)`, after the keys): a `div`, `position: absolute`, top 0, inline `left` x and `width` w, height `--strip-line`. With `detectionLeft`: `--l` fill, `box-shadow: var(--bl)`; otherwise `--a`, no shadow. Absent when the box is null.
- **Split marker** (from `splitX(split, range)`, last): a `div`, `position: absolute`, top 0, bottom 0 (the full 54), inline `left` x, width `--strip-line`, `--t` fill. Absent when null.
- **Nothing else changes:** no hover, press or focus look (not a control); cursor `default` everywhere (D35).
- **Light theme:** the same rules; the tokens make the white keys paper, the black keys flat dark grey without a ring, and the held glows (`--key-glow-mix` 0%) and `--bl` (`none`) draw nothing.
- **Type:** key labels JetBrains Mono 11 / 400, line-height 14, as `noteName` writes them ("C1"; "C-1" can't occur: 88 starts at A-1).
- **Contrast:** every text-on-surface pair the strip draws is a C label (generated content inside the `aria-hidden` layer; the region's `aria-label` carries the meaning). axe doesn't evaluate generated content, so no story needs an `a11y` exclude. Pairs:
  - Pass AA, add to `contrast.test.ts` with the tokens PR: light `--key-label` on `--key-white` 5.71:1, light `--key-label` on `--key-white-left` 5.12:1; dark `--solid-ink` on `--r1` 6.5, `--r2` 6.86, `--r3` 9.91, `--l` 9.75, `--m` 6.25; light `--solid-ink` on `--r1` 6.25, `--r2` 5.2, `--m` 5.92.
  - **Known failures (owner question O-contrast):** dark `--key-label` (#4d4d4d) on `--key-white` (#161616) 2.14:1 and on `--key-white-left` (#0d1a17) 2.11:1 (the board's dim label, D10); light `--solid-ink` (#fff) on `--l` (#008f78) 4.04:1 and on `--r3` (#c85f00) 4.12:1 (a held C in Left or Right 3). Not in `contrast.test.ts` until the owner answers; no axe exclude needed (generated content).
- **Motion:** none; held keys change when `held` changes.

**Test hooks (D41).** Every key is an element with `data-note="<note>"` and `data-zone="left|right"` (left when `note <= split`), whites first then blacks in DOM order; a held key also carries `data-held="true"` (absent, not `"false"`, on a key not held) and `data-hue="<heldHue(parts)>"`. The detection line carries `data-detection` (empty value) and `data-hue="l"` (with `detectionLeft`) or `"a"`. The split marker carries `data-split` (empty value). Positions are inline `left` / `width` styles in px.

### Accessibility

- **Role and name:** the root is a `section` with `aria-label={keysLabel(range, split, held)}` (a `region`). Its single child, the key layer `div`, carries `aria-hidden="true"`, which hides every key and both lines; the keys and lines carry no `aria-hidden` of their own.
- **Keyboard:** none. Nothing in the strip is focusable: no `tabindex`, no buttons (kit › Interaction conventions, Stage.md D36).
- **Tooltip id:** `keystrip.keys`, on the root (`data-tip`, wired at integration). Its body in `app/src/help/tooltips.ts` describes the old strip (a shaded band, chord-tone dots, "the engine doesn't report held keys"). This PR doesn't edit `tooltips.ts`; the orchestrator's tooltip contract change replaces the body with exactly:

  > The keys you are holding, coloured by the part that sounds them: Right 1–3 above the split, Left at or below it, grey where a key only feeds chord detection. The full-height line is the split point; the line along the top is where chord detection listens, teal when it reads the left hand. The keys are a picture: they don't play from the screen. Choose 49, 61 or 88 keys in Settings › Keyboard.

  (`title` "Keyboard" and `genos` "Keyboard (Split Point, chord detection area)" stay.)

## Stories (Story station)

Title `Primitives/KeyStrip`, `layout: 'centered'` (real size, 1392 × 56). Every story renders in dark and light. The fixture `app/src/ui/KeyStrip/KeyStrip.fixtures.ts` exports `boardKeys` (Stage.md › Board fixture: `keyboard` and `ui.keyRange`), exactly:

```ts
export const boardKeys = {
  range: 61,
  held: [
    { note: 43, parts: [3] }, { note: 45, parts: [3] }, { note: 48, parts: [3] }, { note: 52, parts: [3] },
    { note: 76, parts: [0] }, { note: 81, parts: [0] },
  ],
  split: 54,
  detection: [0, 54],
  detectionLeft: true,
} satisfies ComponentProps<typeof KeyStrip>
```

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `boardKeys` | 61 keys C1–C6; G1 A1 C2 E2 teal, E4 A4 blue; the left zone through F#2 in its teal-black; the teal detection line 0–424 with its glow; the white split marker at 424 | `Board-{dark,light}.png` (Stage 24,820 1392×56) | the region named "Keys: split F#2, left hand G A C E, right hand E4 A4, 61 keys" exists; 61 elements carry `data-note`; notes 43, 45, 48, 52 have `data-hue="l"`, 76 and 81 `data-hue="r1"`, and no other key has `data-held`; key 54 has `data-zone="left"`, key 55 `"right"`; `[data-split]` has `style.left` `424px`; `[data-detection]` has `data-hue="l"`, `style.left` `0px`, `style.width` `424px`; the story contains no element with `tabindex` and no `button` |
| `Range49` | `{ ...boardKeys, range: 49 }` | 29 whites, labels C1–C5, wider keys; split and detection line end at 527 | — (not on the Stage board) | 49 elements carry `data-note`; the region's name ends ", 49 keys"; `[data-split]` `style.left` `527px`; `[data-detection]` `style.width` `527px` |
| `Range88` | `{ ...boardKeys, range: 88 }` | 52 whites from A-1, labels C0–C7, narrow keys; split and detection line end at 534 | — (not on the board) | 88 elements carry `data-note`; the first is `data-note="21"`; `[data-split]` `style.left` `534px`; `[data-detection]` `style.width` `534px` |
| `NoHeld` | `{ ...boardKeys, held: [] }` | the empty keyboard: zones, line and marker only | — (the board holds keys) | the region is named "Keys: split F#2, 61 keys"; no element has `data-held` |
| `UpperDetection` | `{ ...boardKeys, detection: [55, 127], detectionLeft: false }` | the accent detection line from the split marker to the right end, no glow | — (the board shows Lower) | `[data-detection]` has `data-hue="a"`, `style.left` `424px`, `style.width` `966px` |
| `HeldNoPart` | `{ ...boardKeys, held: [{ note: 43, parts: [] }, { note: 46, parts: [] }, { note: 76, parts: [0] }] }` | a white (G1) and a black (Bb1) key held for the chord only, in `--m`; E4 in blue | — (not on the board) | keys 43 and 46 have `data-hue="m"`; the region is named "Keys: split F#2, left hand G Bb, right hand E4, 61 keys" |

No `Focused` story: the strip isn't focusable.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `app/src/ui/KeyStrip/keys.test.ts` asserts every worked example under Geometry.
- `npm run shots -- KeyStrip` passes: `Board` is 1392 × 56 and scores at most 0.02 against its crops in both themes; axe finds no violation on any story.
- Only listed tokens are used; no inline colours; the only literal sizes are the root's custom properties under Component geometry (the computed `left` / `width` inline styles are positions, not sizes).
- svelte-check and lint pass on the folder.

## Decisions

- D1 · `held` takes the state's `HeldNote` shape (`{ note, parts }`, `zone` ignored) so the wiring passes `keyboard.held` unchanged, and the hue is the first part's (kit › Key strip).
- D2 · Hands, zones and the label all go by `note <= split`, not by `zone`, so the picture and the label never disagree after the split moves while keys are held.
- D3 · The split marker's left edge and the detection line's edges are floored to whole pixels (`splitX`, `detectionBox`); this reproduces the board's 424 for F#2 (exact boundary 424.72) and keeps the 2px lines crisp, while the keys keep fractional positions as the board draws them.
- D4 · Every C is labelled with `noteName`, so 49 keys read C1–C5, 61 C1–C6 and 88 C0–C7; kit.md's "C1…C6" is the 61-key case.
- D5 · The left hand in the `aria-label` reads pitch classes without octaves and without repeats (it gives the chord); the right hand reads every note with its Yamaha octave; the split carries its octave; a hand with no held keys is left out.
- D6 · Held notes outside the range aren't drawn but are read in the `aria-label`, since the player is holding them.
- D7 · A held key with no parts draws in `--m` with the same 10px glow rule as a part hue, and a held black key's glow replaces its ring.
- D8 · No split marker when the split is below the range or at or above its top note (no boundary inside the strip); every key is then one zone.
- D9 · `detectionLeft` is derived by the wiring from `keyboard.detection` equalling `[0, leftSplit]` (Lower), so the component needs no chord-mode prop.
- D10 · The C labels are CSS generated content inside `aria-hidden` keys: they're decoration, the region's `aria-label` carries the meaning, and the board's dim dark label (`--d` on `--key-white`) stays as drawn.
- D11 · `RANGES` is copied into `keys.ts` (the library can't import `panels`, which imports API types) and pinned by a test.
- D12 · The wiring's `detectionLeft` derivation guards `detection != null` although `AppState` always sends a pair; the component's own `detection` prop stays nullable for stories.
- D13 · The eight `--key-*` tokens and seven palette steps land with the tokens contract PR (the orchestrator), not in this component's PR, with the values copied from kit.md › Tokens to add (lead decision 1).
- D14 · Component-only geometry (1392 × 56, the 22 × 32 black key, its 3px radius, the 2px lines, the 10px glow, the 14px label line) is declared once as custom properties on the root (lead decision 3); shared values use `scale.css` tokens.
- D15 · The key layer is one `aria-hidden` wrapper `div`, not `aria-hidden` on each element, so the DOM stays light (up to 88 keys) and nothing inside can be missed.
- D16 · A held white key keeps its 1px `--keyline` right border, as the board draws it; the C label sits by flex-end with `--space-6` bottom padding, as the board lays it out.
- D17 · `keysLabel` sorts `held` itself and reads a repeated MIDI note once; `keyX` is null outside the range.
- D18 · The light C labels on `--l` and `--r3`, and the dark C labels on the unheld keys, fail AA (Known failures, owner question O-contrast); they stay as the boards draw them until the owner answers.

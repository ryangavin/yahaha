# KeyStrip

## Identity (all stations)

- **Kind:** primitive
- **Built from:** —
- **Purpose:** Shows the keyboard under the player's hands: which keys they hold and on which part, where the split is, and where chord detection listens.
- **Boards:** `Stage-Dark.dc.html:392-403` (markup), `:516-538` (the data script that computes whites and blacks); light: `Stage-Light.dc.html:368-379`, `:487-509`. Crop box: Stage `24,820 1392×56`.
- **Not this component's job:** no store, no API, no Tauri: the range, held keys, split and detection come in as props. It doesn't play notes (the keys are a picture, as today), isn't focusable and has no pointer behaviour. It doesn't pick the range (the wiring does: `ui.keyRange`, else the connected Launchkey's). It doesn't draw chord tones (they moved to the display). No tooltip wiring (integration adds `use:tip`, see Accessibility).

## API (Component station)

### Props

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `range` | `49 \| 61 \| 88` | `61` | How many keys: 49 = C1–C5 (MIDI 36–84), 61 = C1–C6 (36–96), 88 = A-1–C7 (21–108). Yamaha numbering, C3 = 60. |
| `held` | `{ note: number; parts: number[] }[]` | `[]` | The keys held, as the state's `keyboard.held` gives them (its `zone` field may be present and is ignored). `note` is a MIDI note; `parts` the keyboard parts sounding it (0 Right 1, 1 Right 2, 2 Right 3, 3 Left), empty for a key that only feeds chord detection. Notes outside the range aren't drawn but still count in the `aria-label`. |
| `split` | `number` | `54` | The split point (`keyboard.leftSplit`), a MIDI note: keys at or below it are the left zone. 54 is F#2. |
| `detection` | `[number, number] \| null` | `null` | The keys chord detection reads, `[lo, hi]` MIDI notes inclusive (`keyboard.detection`); clipped to the range by `detectionBox`. Null, or nothing left after clipping: no line. |
| `detectionLeft` | `boolean` | `false` | Detection is the left hand (Lower): the line is teal with its glow. False (Upper, Full Keyboard): accent, no glow. |

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
| `range` | `ui.keyRange`, else the connected Launchkey's (`rangeFor(ui.keyRange, inputs)` in `app/src/panels/keystrip/keyboard.ts`), else 61 (Stage.md D25) |
| `held` | `keyboard.held` as is |
| `split` | `keyboard.leftSplit` |
| `detection` | `keyboard.detection` as is (the component clips it) |
| `detectionLeft` | `keyboard.detection[0] === 0 && keyboard.detection[1] === keyboard.leftSplit` (Lower; app-api.md › keyboard: Lower is `[0, split]`, Upper `[split + 1, 127]`, Full Keyboard `[0, 127]`) |

### Geometry: `app/src/ui/KeyStrip/keys.ts`

Pure functions, no DOM; the component computes every position with them and writes it as an inline `left` / `width` in px (so vitest can read `style.left`). All x values are in px from the inside left edge of the frame (the inner box is 1390 × 54). These constants are exported:

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
| `keyX(note, range)` | `number` | the key's left edge: a white's `x` or a black's `x`, as above. |
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

Hands go by the split as drawn (`note <= split`), not by the held note's `zone`, so the label always agrees with the picture. Held notes outside the range still count.

**Worked examples** (each a vitest assertion in `keys.test.ts`):

- `whiteCount(61)` 36, `whiteCount(49)` 29, `whiteCount(88)` 52.
- `whiteKeys(61)[0]` → `{ note: 36, x: 0, w: 38.611…, label: 'C1' }`; `[7]` → `{ note: 48, x: 270.277…, label: 'C2' }`; `[35]` → `{ note: 96, x: 1351.388…, label: 'C6' }`; the non-empty labels in order are C1, C2, C3, C4, C5, C6.
- `whiteKeys(88)`: `[0]` is note 21 (A-1, label `''`), `[2]` is note 24 at `x` 53.461…, label `'C0'`; labels C0…C7. `whiteKeys(49)` labels C1…C5.
- `blackKeys(61)`: 25 keys; `[0]` → `{ note: 37, x: 27.611…, w: 22 }`; F#2 (54) at `x` 413.722… (`11 · W − 11`). `blackKeys(88)`: 36 keys, `[0]` is note 22 at 15.730…. `blackKeys(49)`: 20 keys.
- `keyX(43, 61)` (G1) → 154.444…; `keyX(54, 61)` → 413.722….
- `boundaryX(54, 61)` → 424.722…; `boundaryX(53, 61)` → 424.722… (F2's boundary is F#2's centre); `boundaryX(52, 61)` → 386.111…; `boundaryX(35, 61)` → 0; `boundaryX(96, 61)` and `boundaryX(127, 61)` → 1390.
- `splitX(54, 61)` → 424; `splitX(54, 49)` → 527; `splitX(54, 88)` → 534 (20 whites from A-1 to F2: `floor(20 · 26.7307…)`); `splitX(30, 61)` → null; `splitX(96, 61)` → null.
- `detectionBox([0, 54], 61)` → `{ x: 0, w: 424 }`; `detectionBox([55, 127], 61)` → `{ x: 424, w: 966 }`; `detectionBox([0, 127], 61)` → `{ x: 0, w: 1390 }`; `detectionBox([100, 127], 61)` → null; `detectionBox(null, 61)` → null.
- `noteName(54)` `'F#2'`, `noteName(60)` `'C3'`, `noteName(76)` `'E4'`, `noteName(21)` `'A-1'`, `noteName(70)` `'Bb3'`; `pcName(43)` `'G'`.
- `heldHue([3])` `'l'`, `heldHue([0, 3])` `'r1'`, `heldHue([1])` `'r2'`, `heldHue([])` `'m'`.
- `keysLabel(61, 54, boardKeys.held)` → `"Keys: split F#2, left hand G A C E, right hand E4 A4, 61 keys"` (Stage.md Check 18).
- `keysLabel(61, 54, [])` → `"Keys: split F#2, 61 keys"`.
- `keysLabel(88, 54, [{ note: 36, parts: [3] }, { note: 48, parts: [3] }, { note: 52, parts: [3] }])` → `"Keys: split F#2, left hand C E, 88 keys"` (C1 and C2 both read C, once).
- `keysLabel(49, 54, [{ note: 60, parts: [0] }, { note: 64, parts: [0] }, { note: 67, parts: [0] }])` → `"Keys: split F#2, right hand C3 E3 G3, 49 keys"`.

### Visual rules

- **Tokens used:** `--line`, `--g`, `--keyline`, `--key-white`, `--key-white-left`, `--key-black`, `--key-black-left`, `--key-black-ring`, `--key-black-ring-left`, `--key-label`, `--key-glow-mix`, `--solid-ink`, `--r1`, `--r2`, `--r3`, `--l`, `--m`, `--a`, `--bl`, `--t`, `--radius`, `--font-mono`, `--text-11`, `--weight-regular`, `--line-width`, `--space-6`. To add (kit.md › Tokens to add): every `--key-*` token and `--key-glow-mix`.
- **Frame:** the root box is 1392 × 56: a `--line-width` `--line` border, radius `--radius` (4px), `--g` inside, `overflow: hidden` (clips the keys, their glows and both lines to the rounded inner box). Inside: 1390 × 54, the keys absolutely positioned from its top-left.
- **White keys** (from `whiteKeys(range)`, drawn first, low to high): top 0, `left` x, `width` w, full height (54), `box-sizing: border-box` with a 1px `--keyline` right border; fill `--key-white`, or `--key-white-left` when `note <= split`. A C carries its label at the bottom: centred horizontally, 6px up (`padding-bottom: --space-6`), JetBrains Mono (`--font-mono`) `--text-11` / `--weight-regular`, line-height 14px, `--key-label`. The label is generated content (`::after { content: attr(data-label) }`), so it isn't text in the accessibility tree (see Contrast).
- **Black keys** (from `blackKeys(range)`, drawn after every white): top 0, `left` x, 22 × 32, radius `0 0 3px 3px`; fill `--key-black` with its edge `box-shadow: var(--key-black-ring)`; at or below the split, `--key-black-left` with `var(--key-black-ring-left)`. No label.
- **Held keys** (a key whose `note` is in `held`, white or black): fill `var(--<heldHue(parts)>)` (`--r1`, `--r2`, `--r3`, `--l`, or `--m` for no parts), whatever its zone; glow `box-shadow: 0 0 10px color-mix(in srgb, var(--<hue>) var(--key-glow-mix), transparent)` (on a held black key this replaces its ring); a held C's label in `--solid-ink`. Later keys paint over an earlier key's glow (the board's order).
- **Detection line** (from `detectionBox(detection, range)`, after the keys): top 0, `left` x, `width` w, 2px tall. With `detectionLeft`: `--l`, `box-shadow: var(--bl)`; otherwise `--a`, no shadow. Absent when the box is null.
- **Split marker** (from `splitX(split, range)`, drawn last): top 0, bottom 0 (the full 54), `left` x, 2px wide, `--t`. Absent when null.
- **Nothing else changes:** no hover, press or focus look (not a control); cursor `default` everywhere (D35).
- **Light theme:** the same rules; the tokens make the black keys dark grey without a ring, and the held glows and `--bl` draw nothing.
- **Type:** key labels JetBrains Mono 11 / 400, line-height 14, as `noteName` writes them ("C1"; "C-1" can't occur: 88 starts at A-1).
- **Contrast:** no pair for `tokens/contrast.test.ts`: the key labels are decorative generated content (the region's `aria-label` carries the meaning). For the record, dark `--key-label` (`--d` #4d4d4d) on `--key-white` (#161616) is below 4.5:1 as the board draws it, and light `--solid-ink` #fff on `--l` #008f78 is about 4.0:1 (owner question in the lane report).
- **Motion:** none; held keys change when `held` changes.

**Test hooks (D41).** Every key is an element with `data-note="<note>"` and `data-zone="left|right"` (left when `note <= split`), whites first then blacks in DOM order; a held key also carries `data-held` and `data-hue="<heldHue(parts)>"`. The detection line carries `data-detection` and `data-hue="l"` (with `detectionLeft`) or `"a"`. The split marker carries `data-split`. Positions are inline `left` / `width` styles in px.

### Accessibility

- **Role and name:** the root is a `section` with `aria-label={keysLabel(range, split, held)}` (a `region`); everything inside it is `aria-hidden="true"`.
- **Keyboard:** none. Nothing in the strip is focusable: no `tabindex`, no buttons (kit › Interaction conventions, Stage.md D36).
- **Tooltip id:** `keystrip.keys`, on the root (`data-tip`, wired at integration). Its body in `tooltips.ts` still describes the old strip (a shaded band, chord-tone dots, "the engine doesn't report held keys"): it needs a rewrite in C5 (lane report).

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
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules (the computed `left` / `width` inline styles are positions, not sizes).
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

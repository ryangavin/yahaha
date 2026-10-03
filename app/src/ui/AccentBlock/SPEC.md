# AccentBlock

## Identity (all stations)

- **Kind:** primitive
- **Built from:** —
- **Purpose:** Marks "the device" in a solid accent block: the style's name on the style line (a click opens the Browser) and the knob page's name over the knobs.
- **Boards:**
  - `Stage-Dark.dc.html:132` (the style name "Sunday Drive Pop", 26 tall, 18px medium), `:294` (the knob page "Style", 22 tall, 13px); light: `Stage-Light.dc.html:108`, `:270`.
  - The swap-mode block (the knob page on a part's hue, "Swap R1") is drawn on no board; its values are kit › Knobs.
- **Not this component's job:** no store, no API, no Tauri. It doesn't open the Browser or know which style or page it names: the parent passes `label` and acts on `onpress`. It is not one of the four faces (kit › Faces: the accent block marks the device, it never means on, chosen or waiting) and has no off, pressed or disabled look. No hover change (D35).

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `label` | `string` | — | The text in the block ("Sunday Drive Pop", "Style", "Swap R1"). |
| `empty` | `string` | `'—'` | What the block shows when `label` is empty or only spaces, so it never collapses to a sliver. The style line passes `'No style'`. |
| `as` | `'span' \| 'button'` | `'span'` | `button`: a text button that calls `onpress` (the style name). `span`: a plain label (the knob page). |
| `size` | `'line' \| 'knob'` | `'line'` | `line`: 26 tall, 18px medium, padding 0 10 (the style line). `knob`: 22 tall, 13px regular, padding 0 8 (a band header row: the knob page). |
| `hue` | `'a' \| 'r1' \| 'r2' \| 'r3' \| 'l'` | `'a'` | The block's fill: the accent, or a part's hue in swap mode (kit › Knobs: "Swap R1" on `--r1`). |
| `width` | `number \| undefined` | — | A fixed width in px; a longer label ends in an ellipsis. Without it the block is as wide as its label and can still shrink inside a flex row (`min-width: 0`), ellipsizing the same way. |
| `name` | `string \| undefined` | — | The accessible name when the label alone isn't enough (`button` only): "Sunday Drive Pop: open the Browser". Default: the shown text. |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onpress` | `as: 'button'`: click, Space or Enter | `()` none |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Visual rules

- **Tokens used:** `--a`, `--r1`, `--r2`, `--r3`, `--l`, `--g`, `--focus`, `--font-sans`, `--text-13`, `--text-18`, `--weight-regular`, `--weight-medium`, `--space-8`, `--space-10`, `--line-width`, `--focus-offset`; new scale tokens (add to `scale.css`): `--block-height: 26px`, `--block-height-knob: 22px`.
- **Element:** `as: 'span'` a `<span>`; `as: 'button'` a `<button type="button">` with the browser look removed (border 0, margin 0, `appearance: none`, font family, size and weight set from the type below). Both: `display: block`, `box-sizing: border-box`, `flex: 0 1 auto`, `min-width: 0`, `max-width: 100%`, `overflow: hidden`, `white-space: nowrap`, `text-overflow: ellipsis`, `text-align: left`, radius 0 (square corners, as both board instances).
- **Face:** fill `var(--<hue>)` (`--a` by default), label `--g`. No border, bar, glow or shadow.
- **Size:**

  | Size | Height and line-height | Padding (top/bottom 0) | Type |
  |---|---|---|---|
  | `line` | `--block-height` (26) | `--space-10` each side | `--text-18`, `--weight-medium` (500) |
  | `knob` | `--block-height-knob` (22) | `--space-8` each side | `--text-13`, `--weight-regular` (400) |

  Width: the label plus the padding (measured on the board: "Sunday Drive Pop" at `line` is 168 wide, "Style" at `knob` 46), or `width`, or less when the row squeezes it; whenever the label doesn't fit, it ends in "…".
- **States drawn by:**
  - default: as above.
  - empty `label`: the `empty` text in the same face.
  - long label: clipped with an ellipsis at the block's width (above).
  - keyboard focus (`button` only, `:focus-visible`): a `--line-width` outline in `--focus`, `--focus-offset` outside the block; nothing on mouse focus.
  - No hover, pressed or disabled look.
- **Type:** DM Sans (`--font-sans`), tabular numerals, the label as given.
- **Cursor:** `pointer` for `button`, `default` for `span`.
- **Test hooks:** the element carries `data-face="accent"`, `data-hue="<hue>"` (`a`, `r1` …) and `data-size="<size>"`.
- **Contrast (AA 4.5:1, `tokens/contrast.test.ts`):** `--g` on `--a` (both sizes: 18px medium and 13px are normal text); `--g` on `--r1`, `--r2`, `--r3`, `--l` (the swap block, 13px). With today's tokens light `--g` on `--r3` (3.65) and on `--l` (3.58) fail (D4); the others pass.
- **Not checkable in jsdom:** the fill per hue, the sizes and the ellipsis; the `Board` and `KnobPage` crops and the `LongName` story (judged by Inspect) cover them.
- **Motion:** none.

### Accessibility

- **Role and name:**
  - `button`: a `button`, no `aria-pressed`; the accessible name is `name`, else the shown text (the label, or `empty`). The name carries the whole label even when the face is ellipsized.
  - `span`: no role; its text is read in place. `name` is ignored.
- **Keyboard:** `button`: Tab focuses it; Space or Enter calls `onpress` (native button). `span`: not focusable.
- **Tooltip id:** the parent's: the style name `browser.open`; the knob page none (not a control). Wired at integration, not here.

## Stories (Story station)

Title `Primitives/AccentBlock`, `layout: 'centered'`. Every story renders in dark and light (the toolbar theme).

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ label: 'Sunday Drive Pop', as: 'button', size: 'line', name: 'Sunday Drive Pop: open the Browser', empty: 'No style' }` | the style name: violet block, black 18px medium label, square corners | `Board-{dark,light}.png` (Stage 87,132 168×26) | a `button` named "Sunday Drive Pop: open the Browser" with `data-face="accent"`, `data-hue="a"`, `data-size="line"` and no `aria-pressed`; click → `onpress` called once with no arguments; focus it, press Enter → called twice; press Space → three times |
| `KnobPage` | `{ label: 'Style', size: 'knob' }` | the knob page block, 22 tall, 13px | `KnobPage-{dark,light}.png` (Stage 742,439 46×22) | no `button` in the story root; the text "Style" is in an element with `data-size="knob"` and `data-hue="a"`; nothing focusable |
| `SwapR1` | `{ label: 'Swap R1', size: 'knob', hue: 'r1' }` | the knob page in swap mode, on Right 1's blue (Stage state "Swap held") | — (no board draws swap mode) | `data-hue="r1"` |
| `LongName` | `{ label: 'Bossa Nova Lounge Session With Strings And Brushes Deluxe 2', as: 'button', width: 240, name: 'Bossa Nova Lounge Session With Strings And Brushes Deluxe 2: open the Browser' }` | a 59-character style name clipped at 240px with "…" | — (no board; Inspect judges the clip) | the button's accessible name is the whole name plus ": open the Browser" |
| `Empty` | `{ label: '', as: 'button', empty: 'No style' }` | the block reads "No style" | — | a `button` named "No style" |
| `Focused` | `{ label: 'Sunday Drive Pop', as: 'button', name: 'Sunday Drive Pop: open the Browser' }`, `parameters: { pseudo: { focusVisible: true } }` | the focus ring round the block | — | — |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in the dark and light render.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- AccentBlock` passes: each cropped story's screenshot is the crop's size and scores at most 0.02, and axe (colour contrast included) finds no violation on any story.
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · Element by prop.** `as: 'button' | 'span'` picks the element: the style name is a text button (Stage.md › Style line; the board draws a span, but the name opens the Browser), the knob page a plain label; one component keeps the two blocks identical in face.
- **D2 · Square corners.** Both board instances have no radius, unlike every faced control (`--radius` 4); the block keeps radius 0 so it reads as a label, not a button face.
- **D3 · Ellipsis, never wrap.** A long style name ends in "…" at whatever width the row leaves it (`min-width: 0`) or at `width`; the full name stays in the button's accessible name. The knob page names are short and fixed, so the same rule never bites there.
- **D4 · Swap hues.** The swap block is `--g` on the part's hue (kit › Knobs); in light, `--r3` and `--l` give 3.65 and 3.58 against `--g`, so those two part tokens need darkening in the tokens PR (or the owner picks another ink); `SwapR1`, the case the Stage state names, passes.
- **D5 · Empty text.** An empty label shows `empty` (default "—", "No style" from the style line) so the block keeps its height and something to click; `StyleState.name` is a string the session can leave empty before a style loads.
- **D6 · Weight follows size.** `line` is medium (500), `knob` regular (400), as the boards draw them; a separate weight prop isn't needed until a board draws another mix.
- **D7 · No disabled state.** Nothing on the boards disables the style name (the Browser exists today, so D32's interim rule doesn't apply); a disabled look would need a fifth meaning for the block, so there isn't one.

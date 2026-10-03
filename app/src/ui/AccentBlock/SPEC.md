# AccentBlock

## Identity (all stations)

- **Kind:** primitive
- **Built from:** —
- **Purpose:** Marks "the device" in a solid accent block: the style's name on the style line (a click opens the Browser) and the knob page's name over the knobs.
- **Boards:**
  - `Stage-Dark.dc.html:132` (the style name "Sunday Drive Pop", 26 tall, 18px medium), `:294` (the knob page "Style", 22 tall, 13px); light: `Stage-Light.dc.html:108`, `:270`.
  - The swap-mode block ("Swap R1") is drawn on no board; it is the same accent block (Stage D51, D12 here).
- **Not this component's job:** no store, no API, no Tauri. It doesn't open the Browser or know which style or page it names: the parent passes `label` and acts on `onpress`. It is not one of the four faces (kit › Faces: the accent block marks the device, it never means on, chosen or waiting) and has no off, pressed or disabled look. No hover change (D35).

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `label` | `string` | — | The text in the block ("Sunday Drive Pop", "Style", "Swap R1"). |
| `empty` | `string` | `'—'` | What the block shows when `label.trim() === ''` (empty or whitespace only; otherwise the label is drawn as given, untrimmed), so it never collapses to a sliver. The style line passes `'No style'`. |
| `as` | `'span' \| 'button'` | `'span'` | `button`: a text button that calls `onpress` (the style name). `span`: a plain label (the knob page). |
| `size` | `'line' \| 'knob'` | `'line'` | `line`: 26 tall, 18px medium, padding 0 10 (the style line). `knob`: 22 tall, 13px regular, padding 0 8 (a band header row: the knob page). |
| `width` | `number \| undefined` | — | A fixed width in px; a longer label ends in an ellipsis. Without it the block is as wide as its label and can still shrink inside a flex row (`min-width: 0`), ellipsizing the same way. |
| `name` | `string \| undefined` | — | The accessible name when the label alone isn't enough (`button` only), set as `aria-label`: "Sunday Drive Pop: open the Browser". Default: no `aria-label`, so the name is the shown text. The parent builds it from the shown text, so with no style StyleLine passes `label: ''`, `empty: 'No style'` and `name: 'No style: open the Browser'` (D9). |
| `tip` | `string \| undefined` | — | The tooltip key, rendered as `data-tip` on the element when `as: 'button'` (the style name passes `browser.open`, which exists in `tooltips.ts`). Ignored for `span` (not a control, no `data-tip`). L3. |
| `tipAction` | `Action<HTMLElement, string> \| undefined` | — | The app's `use:tip`, passed in by the wiring because the library can't import it. When `as: 'button'` and both `tip` and `tipAction` are set, the button gets `use:tipAction={tip}`. Ignored for `span`. Stories pass `fn()` (an action). L3. |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onpress` | `as: 'button'`: click, Space or Enter | `()` none |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Visual rules

- **Tokens used:** `--a`, `--g`, `--focus`, `--font-sans`, `--text-13`, `--text-18`, `--weight-regular`, `--weight-medium`, `--space-8`, `--space-10`, `--line-width`, `--focus-offset`, and the new tokens below.

#### New tokens

Not in `app/src/ui/tokens/*` today. They land in the orchestrator's tokens contract PR before this component is built; the builder uses them by name and never hard-codes the value (L1, L2).

| Token | Dark | Light | Used for |
|---|---|---|---|
| `--block-height` | `26px` | `26px` | the block's height and line-height at `line` (scale.css) |
| `--block-height-knob` | `22px` | `22px` | the block's height and line-height at `knob` (scale.css) |

- **Element:** `as: 'span'` a `<span>`; `as: 'button'` a `<button type="button">` with the browser look removed (border 0, margin 0, `appearance: none`, font family, size and weight set from the type below). Both: `display: inline-block`, `vertical-align: top` (an inline-block with `overflow: hidden` would otherwise sit on its bottom edge as baseline), `box-sizing: border-box`, `flex: 0 1 auto`, `min-width: 0`, `max-width: 100%`, `overflow: hidden`, `white-space: nowrap`, `text-overflow: ellipsis`, `text-align: left`, radius 0 (square corners, as both board instances).
- **Face:** fill `--a` always (swap mode too, Stage D51), label `--g`. No border, bar, glow or shadow.
- **Size:**

  | Size | Height and line-height | Padding (top/bottom 0) | Type |
  |---|---|---|---|
  | `line` | `--block-height` (26) | `--space-10` each side | `--text-18`, `--weight-medium` (500) |
  | `knob` | `--block-height-knob` (22) | `--space-8` each side | `--text-13`, `--weight-regular` (400) |

  Width: the label plus the padding (measured on the board: "Sunday Drive Pop" at `line` is 168 wide, "Style" at `knob` 46), or `width` (set as an inline `width: <n>px`), or less when the parent squeezes it; whenever the label doesn't fit, it ends in "…". Being `inline-block`, the block shrink-wraps its label in a block or inline parent (the stories' centred root), capped at the parent's width by `max-width: 100%`; in a flex row (StyleLine, the knob header) it is a flex item and `flex: 0 1 auto; min-width: 0` let the row shrink it (D8).
- **States drawn by:**
  - default: as above.
  - empty `label`: the `empty` text in the same face.
  - long label: clipped with an ellipsis at the block's width (above).
  - keyboard focus (`button` only, `:focus-visible`): a `--line-width` outline in `--focus`, `--focus-offset` outside the block; nothing on mouse focus.
  - No hover, pressed or disabled look.
- **Type:** DM Sans (`--font-sans`), tabular numerals, the label as given.
- **Cursor:** `pointer` for `button`, `default` for `span`.
- **Test hooks:** the element carries `data-face="accent"`, `data-hue="a"` (always; kept so parents' tests read the block the same way) and `data-size="<size>"`.
- **Contrast (AA 4.5:1, `tokens/contrast.test.ts`):** new row (lands with the tokens contract PR, L1): `--g` on `--a`, "style name and knob page label (AccentBlock)" (both sizes: 18px medium and 13px are normal text): 7.80 dark, 5.95 light, both pass today. No part-hue fill, so no other pair (D12).
- **Not checkable in jsdom:** the fill, the sizes and the ellipsis; the `Board` and `KnobPage` crops and the `LongName` story (judged by Inspect) cover them.
- **Motion:** none.

### Accessibility

- **Role and name:**
  - `button`: a `button`, no `aria-pressed`; the accessible name is `name`, else the shown text (the label, or `empty`). The name carries the whole label even when the face is ellipsized.
  - `span`: no role; its text is read in place. `name` is ignored.
- **Keyboard:** `button`: Tab focuses it; Space or Enter calls `onpress` (native button). `span`: not focusable.
- **Tooltip id:** through the `tip` and `tipAction` props (L3): the style name `browser.open` (exists in `tooltips.ts`), on the button as `data-tip`; the knob page none (a `span`, not a control: no `data-tip` even if `tip` is passed).

## Stories (Story station)

Title `Primitives/AccentBlock`, `layout: 'centered'`. Every story renders in dark and light (the toolbar theme). Every story's args also carry `onpress: fn()` and `tipAction: fn()` (actions, L3), so the table leaves them out; keys are pressed with `userEvent.keyboard`.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ label: 'Sunday Drive Pop', as: 'button', size: 'line', name: 'Sunday Drive Pop: open the Browser', empty: 'No style', tip: 'browser.open' }` | the style name: violet `--a` block, 18px medium label in `--g` (black in dark, the pale ground colour in light), square corners | `Board-{dark,light}.png` (Stage 87,132 168×26) | a `button` named "Sunday Drive Pop: open the Browser" with `data-face="accent"`, `data-hue="a"`, `data-size="line"`, `data-tip="browser.open"` and no `aria-pressed`; `tipAction` was called with (the button, `'browser.open'`); click → `onpress` called once with no arguments; focus it, press Enter → called twice; press Space → three times |
| `KnobPage` | `{ label: 'Style', size: 'knob', tip: 'browser.open' }` | the knob page block, 22 tall, 13px | `KnobPage-{dark,light}.png` (Stage 742,439 46×22) | no `button` in the story root; the text "Style" is in an element with `data-size="knob"` and `data-hue="a"`; nothing focusable; no `[data-tip]` (a span ignores `tip`) and `tipAction` not called |
| `SwapR1` | `{ label: 'Swap R1', size: 'knob' }` | the knob page in swap mode: "Swap R1" in the same accent block (Stage D51; the part's hue shows on its lamp and strip) | — (no board draws swap mode) | the text "Swap R1" is in an element with `data-hue="a"` and `data-size="knob"` |
| `LongName` | `{ label: 'Bossa Nova Lounge Session With Strings And Brushes Deluxe 2', as: 'button', width: 240, name: 'Bossa Nova Lounge Session With Strings And Brushes Deluxe 2: open the Browser' }` | a 59-character style name clipped at 240px with "…" | — (no board; Inspect judges the clip) | the button's accessible name is the whole name plus ": open the Browser" |
| `Empty` | `{ label: '', as: 'button', empty: 'No style' }` | the block reads "No style" | — | a `button` named "No style" |
| `Blank` | `{ label: '   ', as: 'button', empty: 'No style' }` | a whitespace-only label counts as empty: the block reads "No style" | — | a `button` whose text is "No style" |
| `NoStyle` | `{ label: '', as: 'button', empty: 'No style', name: 'No style: open the Browser', tip: 'browser.open' }` | the style line before a style loads: the block reads "No style" | — | a `button` named "No style: open the Browser" whose text is "No style"; click → `onpress` called once |
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
- **D4 · Swap hues.** Superseded by D12. The swap block is `--g` on the part's hue; in light, `--r3` and `--l` give 3.65 and 3.58 against `--g`, so those two part tokens need darkening in the tokens PR (or the owner picks another ink); `SwapR1`, the case the Stage state names, passes.
- **D5 · Empty text.** An empty label shows `empty` (default "—", "No style" from the style line) so the block keeps its height and something to click; `StyleState.name` is a string the session can leave empty before a style loads.
- **D6 · Weight follows size.** `line` is medium (500), `knob` regular (400), as the boards draw them; a separate weight prop isn't needed until a board draws another mix.
- **D7 · No disabled state.** Nothing on the boards disables the style name (the Browser exists today, so D32's interim rule doesn't apply); a disabled look would need a fifth meaning for the block, so there isn't one.
- **D8 · Inline-block.** The element is `display: inline-block` (`vertical-align: top`), not `block`, so on its own it is as wide as its label; in a flex row it is a flex item either way and `min-width: 0` lets the row shrink it.
- **D9 · No-style name.** With no style the parent keeps the "{shown text}: open the Browser" template, so StyleLine passes `name: 'No style: open the Browser'` with `empty: 'No style'`; the component itself never composes a name.
- **D10 · Tooltip props (L3).** `tip` and `tipAction` apply only to the `button` form (the style name, `browser.open`); a `span` is not a control and carries neither.
- **D11 · New tokens and changed hues (L1, L2).** `--block-height`, `--block-height-knob`, the five `--g`-on-fill contrast rows and the changed light `--r3` (#b05300) and `--l` (#007c68) land in the tokens contract PR; no story draws those two hues, so no story fails axe meanwhile. This supersedes D4's "or the owner picks another ink". (D12 drops the hue rows and the two changed values.)
- **D12 · No hue prop: the swap block is the accent block (review; Stage D51).** Stage D51 keeps the knob header's block `--g` on `--a` in swap mode, reading `knobs.pageName` ("Swap R1"); the part's hue shows on its lamp and strip. So `hue` is gone, the block is always `--a`, and the swap rows and changed light `--r3` (#b05300) and `--l` (#007c68) go with it: they also disagreed with Stage C6's values for the same palette entries (`--orange-700` #a24d00, `--teal-700` #007360). `SwapR1` shows "Swap R1" on the accent. kit.md › Knobs says the same ("on the same accent block").

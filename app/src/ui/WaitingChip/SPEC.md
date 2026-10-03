# WaitingChip

## Identity (all stations)

- **Kind:** primitive
- **Built from:** —
- **Purpose:** Names what comes next (the next section, a style waiting for the bar line) in an outline, so it reads as "coming" rather than "playing".
- **Boards:**
  - `Stage-Dark.dc.html:110` (the count row's next section "Main C", 26 tall, 18px), `:166` (the display's next section "Main C", 48 tall, 36px); light: `Stage-Light.dc.html:86`, `:142`.
  - The `line` size (the style line's queued style) is drawn on no board; its values are Stage.md › Style line, "Queued style".
- **Not this component's job:** no store, no API, no Tauri. Not a control: it has no click, focus or tooltip. It doesn't decide what is next, map a section name to its Genos name or to a hue (kit › Section names, Hue roles: the parent passes `label` and `hue`), or hide itself from screen readers (the count row's parts are `aria-hidden` by the CountRow). Not the waiting face of a button or a pad (Fade armed, Looper armed, a queued pad): those are Button, LampButton and Pad.

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `label` | `string` | — | The text in the outline: a section's shown name ("Main C", "Intro II", "Break") or a style's name, drawn as given (not trimmed). Empty or only whitespace (`label.trim() === ''`): nothing is rendered (no empty outline, D7). |
| `hue` | `'intro' \| 'main' \| 'ending' \| 'brk' \| 'fill' \| 'a' \| 't'` | `'t'` | The kit hue role of the border and the text: a section hue for a section, `a` for a queued style (the accent), `t` for a neutral waiting chip (kit › Faces, "or `--t`"). |
| `size` | `'count' \| 'line' \| 'display'` | `'count'` | `count`: 26 tall, 18px light, padding 0 8 (the count row). `line`: 26 tall, 14px regular, padding 0 8, at most 200 wide with an ellipsis (the style line's queued style). `display`: 48 tall, 36px light, letter-spacing −1, padding 0 10 (the display's next section). |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| — | | |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Visual rules

- **Tokens used:** `--intro`, `--main`, `--ending`, `--brk`, `--fill`, `--a`, `--t`, `--radius`, `--line-width`, `--font-sans`, `--text-14`, `--text-18`, `--text-36`, `--weight-light`, `--weight-regular`, `--space-8`, `--space-10`, and the new tokens below.

#### New tokens

Not in `app/src/ui/tokens/*` today. They land in the orchestrator's tokens contract PR before this component is built; the builder uses them by name and never hard-codes the value (L1, L2).

| Token | Dark | Light | Used for |
|---|---|---|---|
| `--chip-height` | `26px` | `26px` | the chip's height at `count` and `line` (scale.css) |
| `--chip-height-display` | `48px` | `48px` | the chip's height at `display` (scale.css) |
| `--chip-max` | `200px` | `200px` | the `line` chip's maximum width, border included (scale.css) |
| `--tracking-36` | `-1px` | `-1px` | letter-spacing of 36px light text (scale.css) |
| `--main` (changed) | unchanged (`--green-400` #4fd66a) | #1c8040 → **#1c7e3f** (4.42 → 4.53 on `--g`) | the Main hue's text on the ground; the light palette entry `--green-700` changes, so light `--ok` moves with it |
| `--intro` (changed) | unchanged (`--gold-400` #bfb24e) | #857a1f → **#796f1c** (3.87 → 4.53) | the Intro hue's text on the ground; light `--gold-700` changes |
| `--brk` (changed) | #8f62a8 → **#9063a9** (4.48 → 4.54 on black) | unchanged (`--plum-600` #7a4a96) | the Break hue's text on the ground; dark `--plum-400` changes |

Each changed value is the smallest step in HSL lightness (hue and saturation kept) that reaches 4.5:1 on that theme's `--g`.

- **Element:** a `<span>`, `display: inline-block`, `box-sizing: border-box`, `white-space: nowrap`, `vertical-align: middle`. `count` and `display`: `flex: none`. `line`: `flex: 0 1 auto`, `min-width: 0`, `max-width: var(--chip-max)`, `overflow: hidden`, `text-overflow: ellipsis`. The `flex` values matter only when the parent is a flex container (the flex item is blockified, so `inline-block` is ignored there): in StyleLine's flex row a crowded line can shrink the `line` chip below its 200px cap and it ellipsizes at whatever width it gets. In a block or inline parent (the stories' centred root) `flex` does nothing: the chip is as wide as its label, at most 200px (D8).
- **Face (every size):** transparent background; border `--line-width` solid in the hue (`var(--<hue>)`); radius `--radius` (4); text in the hue. No glow, no fill, no bar.
- **Size:**

  | Size | Height | Padding (top/bottom 0) | Line-height | Type | Letter-spacing | Width |
  |---|---|---|---|---|---|---|
  | `count` | `--chip-height` (26) | `--space-8` each side | `calc(var(--chip-height) - 2 * var(--line-width))` (24) | `--text-18`, `--weight-light` (300) | 0 | the label; never truncated |
  | `line` | `--chip-height` (26) | `--space-8` each side | 24 (as `count`) | `--text-14`, `--weight-regular` (400) | 0 | the label, at most `--chip-max` (200, border included); longer: `overflow: hidden; text-overflow: ellipsis` |
  | `display` | `--chip-height-display` (48) | `--space-10` each side | `calc(var(--chip-height-display) - 2 * var(--line-width))` (46) | `--text-36`, `--weight-light` (300) | `--tracking-36` (−1px) | the label; never truncated |

  Measured on the board: "Main C" at `count` is 75 × 26, at `display` 130 × 48.
- **States drawn by:** one face; the hue is the only thing that changes. Empty or whitespace-only `label`: no element at all (the component renders nothing, not even a wrapper).
- **Type:** DM Sans (`--font-sans`), tabular numerals, the label as given (no case change).
- **Test hooks:** the span carries `data-face="waiting"`, `data-hue="<hue>"` (`main`, `a` …) and `data-size="<size>"`.
- **Contrast (AA, `tokens/contrast.test.ts`):** the text is drawn on the ground through the transparent face, so the pairs are each hue on `--g`, at 4.5:1 (the `count` and `line` sizes are normal text: 18px light and 14px). Existing row: `--t` on `--g`. New rows (land with the tokens contract PR, L1), each "section or accent hue text on the ground (WaitingChip, GroupHeader legend)": `--intro`, `--main`, `--ending`, `--brk`, `--fill`, `--a` on `--g`. Today's ratios, dark / light: `--intro` 9.69 / **3.87**, `--main` 11.16 / **4.42**, `--ending` 4.96 / 5.20, `--brk` **4.48** / 5.73, `--fill` 7.27 / 4.51, `--a` 7.80 / 5.95. The three in bold fail; the changed values under New tokens fix them (D4, L2). `display` (36px, large text) needs only 3:1, which all pass today.
- **Not checkable in jsdom:** the border and text colour (a custom property per hue), the sizes and the ellipsis; the `Board` and `Display` crops and the `LongStyle` story (judged by Inspect) cover them.
- **Motion:** none.

### Accessibility

- **Role and name:** none: a plain `<span>` whose text is read in place. No `aria-label`, no role, not focusable. When ellipsized (`line`), the full label is still the span's text, so assistive tech reads it whole.
- **Keyboard:** none.
- **Tooltip id:** none (not a control; the parent's readout carries its own, e.g. `display.position` on the count row).

## Stories (Story station)

Title `Primitives/WaitingChip`, `layout: 'centered'`. Every story renders in dark and light (the toolbar theme).

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ label: 'Main C', hue: 'main', size: 'count' }` | the count row's next section: 1px green outline, 18px green "Main C" | `Board-{dark,light}.png` (Stage 667,71 75×26) | the text "Main C" is in a span with `data-face="waiting"`, `data-hue="main"`, `data-size="count"`; no `button`, no focusable element |
| `Display` | `{ label: 'Main C', hue: 'main', size: 'display' }` | the display's next section, 48 tall, 36px | `Display-{dark,light}.png` (Stage 557,191 130×48) | `data-size="display"` |
| `QueuedStyle` | `{ label: 'Coastal Highway', hue: 'a', size: 'line' }` | the style line's queued style in the accent | — (no board draws a queued style) | `data-hue="a"`, `data-size="line"`, text "Coastal Highway" |
| `LongStyle` | `{ label: 'Another Very Long Style Name That Keeps Going', hue: 'a', size: 'line' }` | capped at 200px, ending in an ellipsis | — (no board; Inspect judges the cap and the ellipsis) | the span's text is the whole label |
| `IntroArmed` | `{ label: 'Intro II', hue: 'intro', size: 'display' }` | stopped with Intro II armed (Stage.md D4) | — | `data-hue="intro"` |
| `Break` | `{ label: 'Break', hue: 'brk', size: 'count' }` | the Break next, in its violet | — | `data-hue="brk"` |
| `Fill` | `{ label: 'Fill', hue: 'fill', size: 'count' }` | a fill next, in its grey-blue | — | `data-hue="fill"` |
| `Ending` | `{ label: 'Ending I', hue: 'ending', size: 'count' }` | an ending next, in its red | — | `data-hue="ending"` |
| `Empty` | `{ label: '', hue: 'main', size: 'count' }` | nothing (no next) | — | no `[data-face="waiting"]` in the story root |
| `Blank` | `{ label: '   ', hue: 'main', size: 'count' }` | nothing (a whitespace-only label counts as empty, D7) | — | no `[data-face="waiting"]` in the story root |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in the dark and light render. No `Focused` story: the chip isn't focusable.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- WaitingChip` passes: each cropped story's screenshot is the crop's size and scores at most 0.02, and axe (colour contrast included) finds no violation on any story once the tokens contract PR has landed (L2). Until then two stories are expected to fail axe: `Board` in light (`--main` 4.42) and `Break` in dark (`--brk` 4.48); every other story passes today (`Display` and `IntroArmed` are large text).
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · A span, not a button.** All three uses (count row, display, style line) are readouts the boards draw as spans; nothing is clicked, so the chip has no role, focus or tooltip.
- **D2 · Hue set.** `hue` takes the five section hues, `a` (a queued style, Stage.md D3) and `t` (kit › Faces' neutral waiting label); the rec and lamp waiting faces belong to the Looper's and Fade's buttons, not here. The default is `t` so a missing hue never paints a section colour.
- **D3 · Empty renders nothing.** With no next section the parent hides "→" and the chip; an empty `label` renders no element, so an empty outline can never show.
- **D4 · Contrast.** The `count` and `line` sizes are normal text and need 4.5:1 on the ground; light `--main` (4.42), light `--intro` (3.87) and dark `--brk` (4.48) miss it, so those three tokens need darkening (light) or lightening (dark) in the tokens PR; the spec doesn't work around it with a second colour.
- **D5 · Only `line` truncates.** Section names are short and known ("Ending III" is the longest), so `count` and `display` never ellipsize; only a style name can run long, capped at 200px (Stage.md D34). The 200px includes the border and padding.
- **D6 · Line-height from the height.** The text is centred by `line-height = height − 2 × border` (24, 46), as the boards do, so the label's baseline matches the board in every size.
- **D7 · Whitespace is empty.** A label that is empty after `trim()` renders nothing, so a parent passing `' '` can't draw a blank outline; any other label is drawn untrimmed.
- **D8 · Shrink only in a flex row.** `line`'s `flex: 0 1 auto; min-width: 0` lets StyleLine's flex row squeeze it; anywhere else the chip is its label's width capped at `--chip-max`, and that's the width the `LongStyle` story shows.
- **D9 · New tokens and changed hues (L1, L2).** `--chip-height`, `--chip-height-display`, `--chip-max`, `--tracking-36`, the six hue-on-`--g` contrast rows and the three changed values (light `--main` #1c7e3f, light `--intro` #796f1c, dark `--brk` #9063a9) land in the tokens contract PR; until then `Board` (light) and `Break` (dark) fail axe.
- **D10 · No tooltip props (L3).** The chip isn't interactive, so it takes neither `tip` nor `tipAction`.

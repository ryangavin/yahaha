# PartMarks

## Identity (all stations)

- **Kind:** primitive
- **Built from:** —
- **Purpose:** The small marks after a part's sound name that say the sound was edited, its plugin is missing or failed, the part is off, or it plays the Manual Bass.
- **Boards:**
  - `Stage-Dark.dc.html:194` (Right 2's sound cell: the edited dot), `:202-203` (Right 3's sound cell: ⚠ then "off"), `:262-264` (the fader strip name's marks: edited dot, ⚠, ✕; strip 2 shows the dot, strip 3 the ⚠); light: `Stage-Light.dc.html:170`, `:178-179`, `:238-240`.
  - ✕ (failed) and "bass" are not drawn rendered on any board: ✕ only in the strip template (`:264`), "bass" only in Stage.md › Sounds row. Their look is fixed here.
- **Not this component's job:** no store, no API, no Tauri. It doesn't decide when a part is edited, missing, failed, off or on the bass: the parent (SoundCell, FaderStrip) works that out from `keyboardParts[i]` and passes booleans. Not the sound number or name, not the part tag, no click (the marks sit inside the parent's button), no tooltip, and no accessible text of its own (the parent's `aria-label` says it, built with `marksText`, below).

Its users and the props they pass, from `keyboardParts[i]` (Stage › Sounds row; kit › FaderStrip):

| User | Props |
|---|---|
| SoundCell (the sound button, after the name) | `{ edited: soundEdited, missing: plugin.missing, failed: plugin.status === 'failed', off: !on && !sounding && !playsBass, bass: playsBass }` (size `cell`) |
| FaderStrip (the name button, keyboard parts 1–4 only) | `{ size: 'strip', edited: soundEdited, missing: plugin.missing, failed: plugin.status === 'failed' }` |

## API (Component station)

### Props

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `edited` | `boolean` | `false` | The part's sound was changed since it was loaded (`soundEdited`): the 5px dot. |
| `missing` | `boolean` | `false` | The part's plugin isn't installed (`plugin.missing`): the orange ⚠. Wins over `failed`. |
| `failed` | `boolean` | `false` | The part's plugin failed to load or crashed (`plugin.status === 'failed'`): the red ✕, drawn only when `missing` is false. |
| `off` | `boolean` | `false` | The part is off and silent: the word "off". Not drawn when `bass` is true (a part playing the Manual Bass sounds). |
| `bass` | `boolean` | `false` | The part plays the Style's bass under Manual Bass (`playsBass`): the word "bass". |
| `size` | `'cell' \| 'strip'` | `'cell'` | The gap between marks: `cell` 8px (the sound cell, where the name, number and marks are 8 apart), `strip` 4px (the fader strip's name button). Nothing else changes. |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| — | | |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Pure functions (`app/src/ui/PartMarks/marks.ts`)

- `visibleMarks(m: { edited?, missing?, failed?, off?, bass? }): Mark[]`, `type Mark = 'edited' | 'missing' | 'failed' | 'off' | 'bass'`: the marks drawn, in drawing order `edited`, `missing` or `failed`, `off`, `bass`; `failed` dropped when `missing`; `off` dropped when `bass`. The component draws exactly this list.
- `marksText(m): string`: the words a parent appends to its `aria-label`, in Stage's order `{, edited}{, off}{, plugin missing | , plugin failed}{, bass}`, from `visibleMarks(m)`: `{ edited: true }` → `", edited"`; `{ missing: true, failed: true, off: true }` → `", off, plugin missing"`; `{ failed: true }` → `", plugin failed"`; `{ off: true, bass: true }` → `", bass"`; `{}` → `""`. (Drawing order puts the ⚠ / ✕ before "off"; the spoken order puts "off" first, as Stage's `aria-label` template does.)

### Visual rules

- **Tokens used:** `--t`, `--warn`, `--ending`, `--m`, `--font-sans`, `--text-12`, `--weight-regular`, `--space-4`, `--space-8`. All exist in `app/src/ui/tokens/*` today: no new tokens and no new contrast rows (L1).
- **Root:** a `span`, `display: inline-flex`, `align-items: center`, `flex: none`, gap `--space-8` (`cell`) or `--space-4` (`strip`), `white-space: nowrap`, `aria-hidden="true"`. The root carries the type, so the words never inherit the parent's 14px: `font-family: var(--font-sans); font-size: var(--text-12); font-weight: var(--weight-regular); line-height: normal; color: var(--m)` (D8). When `visibleMarks` is empty the component renders **nothing** (no root element), so the parent's flex gap adds no space after the name.
- **Marks**, each a child of the root with `flex: none` and `data-mark="<mark>"`, in `visibleMarks` order:
  - **edited:** a `span`, 5 × 5, `border-radius: 50%`, `background: var(--t)`. `data-hue="t"`.
  - **missing:** an inline `svg`, the ⚠. Geometry as attributes, colour and stroke in the component's CSS (D9):
    ```svelte
    <svg data-mark="missing" data-hue="warn" class="warn" width="12" height="12" viewBox="0 0 12 12">
      <path d="M6 1.5 L11 10.5 H1 Z" /><path d="M6 5 V7.4" /><circle cx="6" cy="8.9" r="0.6" />
    </svg>
    ```
    ```css
    .warn { fill: none; stroke: var(--warn); stroke-width: 1; stroke-linejoin: round; }
    .warn circle { fill: var(--warn); stroke: none; }
    ```
  - **failed:** an inline `svg`, the ✕, the same way:
    ```svelte
    <svg data-mark="failed" data-hue="ending" class="fail" width="12" height="12" viewBox="0 0 12 12">
      <path d="M2.5 2.5 L9.5 9.5 M9.5 2.5 L2.5 9.5" />
    </svg>
    ```
    ```css
    .fail { fill: none; stroke: var(--ending); stroke-width: 1.5; stroke-linecap: round; }
    ```
  - **off:** a `span` with the text "off", taking the root's type and colour (12px, regular, `--m`, `line-height: normal`; the root centres it on the parent's row). `data-hue="m"`.
  - **bass:** a `span` with the text "bass", the same as "off". `data-hue="m"`.
- **Size:** the root's width is its marks plus the gaps; its height is the tallest mark (5, 12, or the 12px text's normal line height, about 16).
- **States drawn by:** which marks are present (above). No hover or focus look: the root isn't a control; the parent's button carries cursor and focus.
- **Type:** DM Sans, 12px, weight 400, lower case as written, set on the root (above). No `font-variant-numeric`: the marks hold no digits, so the root doesn't set it.
- **Contrast (AA 4.5:1, `tokens/contrast.test.ts`):** `--m` on `--g` ("off", "bass" on the ground; the pair exists). The dot and the glyphs are graphics whose meaning is also in the parent's `aria-label`; `--t`, `--warn` and `--ending` on `--g` are each 3:1 or more in both themes.
- **Motion:** none.

### Accessibility

- **Role and name:** `aria-hidden="true"` on the root; no role. The parent's button `aria-label` carries the words, built with `marksText` (Stage: "Right 3 sound: 57 Brass Section, off, plugin missing. Opens the quick sound list"; FaderStrip: "Right 2, Silk Strings, edited: open Channel").
- **Keyboard:** not focusable.
- **Tooltip id:** none; the parent's (`launchkey.fader_sound` on the sound cell, `mixer.strip.select` on the strip name). The board's `title="Sound edited"` on the dot is dropped (D4).

## Stories (Story station)

Title `Primitives/PartMarks`, `layout: 'centered'`. Every story renders in dark and light (the toolbar theme). No `Focused` story: it isn't focusable.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ edited: true }` | Right 2's sound cell: the 5px white dot (black in light) | `Board-{dark,light}.png` (Stage 545,371 5×5) | the root has `aria-hidden="true"`; exactly one `[data-mark]`, `data-mark="edited"` with `data-hue="t"` |
| `Missing` | `{ missing: true }` | the orange ⚠ alone | `Missing-{dark,light}.png` (Stage 674,368 12×12: Right 3's ⚠) | one `[data-mark]`: `missing`, `data-hue="warn"` |
| `MissingOff` | `{ missing: true, failed: true, off: true }` | Right 3's cell on the Stage fixture (missing, its status failed, off): ⚠ then "off", 8 apart; no ✕ | — (its box is fractional: the "off" text width and a half-pixel row) | the `[data-mark]` values in order are `missing`, `off`; no `failed`; the text "off" is in the canvas |
| `Failed` | `{ failed: true }` | the red ✕ | — (no board draws it rendered) | one `[data-mark]`: `failed`, `data-hue="ending"` |
| `Bass` | `{ bass: true }` | Left under Manual Bass: "bass" | — (no board draws it) | one `[data-mark]`: `bass`; the text "bass" |
| `Everything` | `{ edited: true, missing: true, off: true }` | the most marks one cell can show: dot, ⚠, "off" | — (no board draws it) | marks in order `edited`, `missing`, `off` |
| `Strip` | `{ size: 'strip', edited: true, missing: true }` | the strip name's marks, 4px apart | — (the board's strips show one mark each) | marks in order `edited`, `missing` |
| `None` | `{}` | nothing: no root, no space | — | the canvas has no `[data-mark]` and no `[aria-hidden]` element from the component |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in the dark and light render. Colours are checked by the cropped screenshots; the plays check `data-mark` and `data-hue`.

**Unit tests** (`app/src/ui/PartMarks/marks.test.ts`, vitest): `visibleMarks` for each single mark; `{ missing, failed }` → `['missing']`; `{ off, bass }` → `['bass']`; `{ edited, failed, off }` → `['edited', 'failed', 'off']`; `{}` → `[]`. `marksText` for each example in "Pure functions" above.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`); `marks.test.ts` passes.
- `npm run shots -- PartMarks` passes for `Board` and `Missing` (score at most 0.02, or the Inspect agent judges any difference to be render noise).
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules (5, 12, the SVG paths).
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · Booleans in, precedence inside.** The parent passes the raw facts (`missing`, `failed`, `off`, `bass`) and PartMarks applies the two rules (⚠ wins over ✕; "bass" wins over "off") in `visibleMarks`, so SoundCell and FaderStrip can't disagree.
- **D2 · Order.** Drawn: edited dot, ⚠ or ✕, "off", "bass" (the board's order, with "bass" last as Stage › Sounds row has it); spoken: edited, off, plugin missing or failed, bass (Stage's `aria-label` template).
- **D3 · Nothing when empty.** With no marks the component renders no element, so a cell without marks has no stray 8px after the name, as the board draws Right 1 and Left.
- **D4 · No titles.** The board's `title="Sound edited"` on the dot is dropped: the marks are `aria-hidden` and the parent's tooltip and `aria-label` explain them; a native title would be a second, untranslated tooltip.
- **D5 · Own dot, not StatusDot.** The 5px edited dot is drawn here with the same values as StatusDot `size: 'sm'`, so PartMarks stays a primitive with no children (Stage's Components row 8).
- **D6 · `size` is only the gap.** The strip and the cell draw the same marks at the same size; only the gap differs (4 on the strip's name button, 8 in the sound cell), and the strip never passes `off` or `bass` (kit › FaderStrip: keyboard parts' dot, ⚠ and ✕ only).
- **D7 · Text line height.** "off" and "bass" keep the board's `line-height: normal` (no fixed value), so they centre on the parent's row exactly as drawn.
- **D8 · Type on the root.** The font tokens and `--m` sit on the root span, not on each word, so "off" and "bass" can't pick up the parent button's 14px or its colour; there are no digits, so no tabular numerals.
- **D9 · SVG geometry in attributes, paint in CSS.** Paths, `viewBox` and size are attributes; `fill`, `stroke`, `stroke-width` and the joins are CSS on the svg (presentation attributes can't take `var()`), and the ⚠'s `circle` overrides them with `fill: var(--warn); stroke: none`. StatusLine draws its ⚠ the same way.

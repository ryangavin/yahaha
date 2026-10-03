# BandSends

## Identity (all stations)

- **Kind:** primitive
- **Built from:** —
- **Purpose:** Shows how much reverb, chorus and delay the band is sent to, and opens Effects when clicked.
- **Boards:**
  - `Stage-Dark.dc.html:143` (the end of the style line: "Band  Reverb 40  Chorus 12  Delay 0"); light: `Stage-Light.dc.html:119`. Spec: Stage.md › Display › Style line (the "Band sends" row), D34, D58, D32.
- **Not this component's job:** no store, no API, no Tauri. It doesn't send `setBandSend` or change a level: it is a readout that opens Effects, and the parent (StyleLine, then the wiring) decides what "open" does (Stage D32 interim: the Effects drawer). It doesn't place itself in the style line (StyleLine gives it the 16px left margin and `flex: none`). No face: it is a text button (kit › Faces, Text buttons). No `use:tip` import (L3).

Its user: StyleLine (Stage.md row 29) passes `{ sends: home.bandSends, tip: 'display.band_sends', tipAction, onopen: () => onopen('effects') }`.

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component. The type is local (lint forbids importing `app/src/lib` in `app/src/ui`); its fields match `docs/app-api.md` › `home.bandSends`, so the state's entries (which also carry `name` and `effectName`) pass as they are.

```ts
/** One entry of `home.bandSends`; only `block` and `level` are read. */
export type BandSend = { block: 'reverb' | 'chorus' | 'variation'; level: number }
```

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `sends` | `BandSend[]` | `[]` | `home.bandSends`. Each value is found by `block`, never by index: `reverb` → Reverb, `chorus` → Chorus, `variation` → Delay (D58). A block with no entry reads "–". |
| `tip` | `string \| undefined` | `'display.band_sends'` | The tooltip key, rendered as `data-tip` on the button; no attribute when set to undefined (L3). The key is new (contract change C5, D5). |
| `tipAction` | `Action<HTMLElement, string> \| undefined` | — | The app's `use:tip`, passed in by the wiring. When both it and `tip` are set the button gets `use:tipAction={tip}`; otherwise nothing (L3). |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onopen` | click, Enter or Space on the button | `()`; the parent opens the Effects page (Stage D32: the Effects drawer until #519) |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Visual rules

- **Tokens used:** `--m`, `--t2`, `--a`, `--focus`, `--line-width`, `--focus-offset`, `--font-sans`, `--text-14`, `--text-18`, `--weight-light`, `--weight-regular`, `--space-10`, and the new token below.

#### New tokens

| Token | Dark | Light | Used for |
|---|---|---|---|
| `--control-height-text` | `24px` | `24px` | the text button's height (scale.css; the style line's 24-tall text buttons) |

It lands in the orchestrator's tokens contract PR before the build (L1). Nothing else is new: every colour is an existing role.

- **Element:** one `<button type="button">`: `display: inline-flex`, `align-items: baseline`, `gap: var(--space-10)`, `height: var(--control-height-text)` (24), `padding: 0`, `border: 0`, `background: transparent`, `white-space: nowrap`, `flex: none`, `font-family: var(--font-sans)`, `font-variant-numeric: tabular-nums`, `cursor: pointer`, `text-align: left`. Line-heights are `normal`; the baseline row places the words and the 18px values together as the board draws them.
- **Items, in this order** (four, with the gap of 10 between them):
  1. "Band": `--text-14` / 400, `--m`.
  2. – 4. One `span` per send, `data-block="reverb|chorus|variation"`, in the order Reverb, Chorus, Delay whatever the array's order: the word and one normal space ("Reverb ") in `--text-14` / 400 `--t2`, then the value (`data-part="value"`) at `--text-18` / 300 in `--a` with `data-hue="a"`, no extra gap. The value is `Math.round(level)` as a decimal ("40", "12", "0"). A block with no entry: "–" (U+2013) in `--m` in the value's place, same size and weight, `data-hue="m"` (D58), so the button keeps its width and the three words always show.
- **Size:** 24 tall, as wide as its text (about 252px at the board's values: "Band Reverb 40 Chorus 12 Delay 0"). Never shrinks or wraps (Stage D34: only the style name shrinks).
- **States drawn by:**
  - normal: as above.
  - no `home` yet (`sends` empty): "Band Reverb – Chorus – Delay –", each dash `--m`.
  - keyboard focus: `outline: var(--line-width) solid var(--focus)` at `outline-offset: var(--focus-offset)` on `:focus-visible`; nothing on mouse focus.
  - No hover or pressed look (Stage D35); no disabled state (the Effects drawer always exists, D3).
- **Type:** DM Sans, tabular numerals; words regular, values light, sentence case.
- **Test hooks:** `data-block` on each pair, `data-part="value"` and `data-hue="a|m"` on each value.
- **Contrast (AA, `tokens/contrast.test.ts`):** `--m` on `--g` ("Band", the dash: existing row), `--t2` on `--g` (the words; existing row), `--a` on `--g` (the values, 18px light: normal text, 4.5:1; dark 7.80, light 5.95; the row "accent text on the ground" WaitingChip's spec already adds). No failing pair.
- **Motion:** none.

### Accessibility

- **Role and name:** a `button` with `aria-label` "Band sends: reverb {r}, chorus {c}, delay {d}. Opens Effects" ("Band sends: reverb 40, chorus 12, delay 0. Opens Effects"; a missing block reads "not known", D4: "Band sends: reverb not known, chorus not known, delay not known. Opens Effects"). Its visible text is inside the button and needs no `aria-hidden` (the label wins).
- **Keyboard:** Tab focuses it; Enter and Space call `onopen` (native button).
- **Tooltip id:** `display.band_sends` (new: contract change C5, Stage.md › Contract changes needed), the `tip` default, through `tip` and `tipAction` (L3).

## Stories (Story station)

Title `Primitives/BandSends`, `layout: 'centered'`. Every story renders in dark and light (the toolbar theme). The meta's `args` are `{ onopen: fn(), tip: 'display.band_sends', tipAction: fn() }` (L3). The board's `sends` are the Stage board fixture's `home.bandSends`: `[{ block: 'reverb', name: 'Reverb', effectName: 'Hall 1', level: 40 }, { block: 'chorus', name: 'Chorus', effectName: 'Chorus 1', level: 12 }, { block: 'variation', name: 'Delay', effectName: 'Delay LCR', level: 0 }]`.

**Controls (argTypes):** `sends` an object control; `tip` text; `onopen`, `tipAction` actions.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ sends: <the board's three> }` | "Band" grey, then Reverb 40, Chorus 12, Delay 0 with the values in violet light | — (no crop file yet, D1; box `Stage 603,133 260×24`) | the button named "Band sends: reverb 40, chorus 12, delay 0. Opens Effects" exists; its text, whitespace collapsed, is "Band Reverb 40 Chorus 12 Delay 0"; each `[data-part="value"]` has `data-hue="a"`; `data-tip="display.band_sends"` and `tipAction` was called with the button and `'display.band_sends'`; click → `onopen` called once |
| `Keyboard` | `{ sends: <the board's three> }` | — | — | `button.focus()`, `userEvent.keyboard('{Enter}')` → `onopen` called once; `userEvent.keyboard(' ')` → called twice |
| `AnyOrder` | `{ sends: [{ block: 'variation', level: 0 }, { block: 'reverb', level: 40 }, { block: 'chorus', level: 12 }] }` | the same line: values found by block, not by index (D58) | — | the `[data-block]` spans read in the order reverb, chorus, variation with values 40, 12, 0 |
| `NotYet` | `{ sends: [] }` | before the engine sends `home`: three grey dashes, the words kept | — | three values read "–" with `data-hue="m"`; the name is "Band sends: reverb not known, chorus not known, delay not known. Opens Effects" |
| `OneMissing` | `{ sends: [{ block: 'reverb', level: 40 }, { block: 'chorus', level: 12 }] }` | Delay "–" | — | the variation value reads "–", `data-hue="m"` |
| `Full` | `{ sends: [{ block: 'reverb', level: 127 }, { block: 'chorus', level: 127 }, { block: 'variation', level: 127 }] }` | the widest values, still one line | — | values read 127 |
| `Focused` | `{ sends: <the board's three> }`, `parameters: { pseudo: { focusVisible: true } }` | the 1px focus ring 2px outside the button | — (the boards draw no focus) | — |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in the dark and light render; the box is the button's own (L6).

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- BandSends` passes: axe finds no violation on any story, and, once the crop station has cut `Board-{dark,light}.png` from the box above (D1), `Board` scores at most 0.02 against it (or the Inspect agent judges the difference render noise).
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · No crop file yet.** `app/src/ui/BandSends/` is new and has no `crops/`; the crop station cuts `Board-{dark,light}.png` later from `Stage 603,133 260×24` (the button's 24px band centred in the 32px style line at y 129, its left edge 16px after One Touch 4, its right edge at the display's content edge), a box measured by eye on `docs/design/push/png/Stage-Dark.png` that the crop station confirms against the render before cutting.
- **D2 · One text button, no face.** As the board draws it (Stage D34): one place to open Effects, drawn as its text only; the kit's four faces are for switches and actions.
- **D3 · Never disabled.** Effects has an interim target today (the Effects drawer, Stage D32), so the button is always enabled; there is no `disabled` prop.
- **D4 · Missing reads "not known" to a screen reader.** The dash is a visual placeholder; "reverb –" would be read as "en dash", so the label says "not known" for a block with no entry.
- **D5 · Tooltip by key plus action (L3); contract change.** The button takes `tip` (default `display.band_sends`) and `tipAction`. `display.band_sends` is not in `tooltips.ts` yet: contract change needed, Stage.md C5, which lands before the build; until then the story's `tip` is a key the catalog test doesn't know.
- **D6 · The margin is the parent's.** Stage › Style line gives the button a 16px left margin; StyleLine sets it, so BandSends can be shown alone at its real size.
- **D7 · L1, one new token.** `--control-height-text` (24px) lands in `scale.css` with the tokens contract PR; if another spec in the lane has already named the 24px text-button height, the tokens PR keeps that name and this spec follows it.

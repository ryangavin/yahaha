# RackReadout

## Identity (all stations)

- **Kind:** primitive
- **Built from:** StatusDot (the 5px modified dot)
- **Build after:** StatusDot, with `hue: 't'` and `size: 'sm'` (merged); the `current` look also needs StatusDot's hue union to gain `'g'` (D6).
- **Purpose:** Says which rack is under the player's hands (its name, its Quick Rack button, whether it has unsaved changes) and opens the Rack page.
- **Boards:**
  - `Stage-Dark.dc.html:178-181` (the Sounds row's first cell: "Rack · A1" over "Sunday drive" with the modified dot; the `cell` size); light: `Stage-Light.dc.html:154-157`.
  - `Channel-Dark.dc.html:80-84` (the app bar, page variant: "Rack A1 Sunday drive •" on one line; the `line` size, canonical, D1); light: `Channel-Light.dc.html:44-48`. The same line on `Effects-Dark.dc.html:73` / `Effects-Light.dc.html:57`, `Harmony-Dark.dc.html:54` / `Harmony-Light.dc.html:44`, `Browser-Dark.dc.html:66` / `Browser-Light.dc.html:50`.
  - `Rack-Dark.dc.html:63` (the app bar on the Rack page: the readout as the current item, a white block; `line` with `current`); light: `Rack-Light.dc.html:45`.
  - `FirstRun-Dark.png` draws "Rack · —" over "New rack"; this spec follows Stage.md D21 ("Rack" alone when no button is loaded), so that drawing has no story (D9).
- **Not this component's job:** no store, no API, no Tauri. It doesn't find the rack's name, the loaded Quick Rack button or the modified flag: the wiring passes `name`, `slot` and `modified` (Stage.md › Sounds row, D21; Rack.md RK-D23's `rackName` is the wiring's choice, D7). It doesn't open the Rack page: it calls `onpress` and the wiring opens the page (interim: the Rack drawer, Stage.md D32). It doesn't place itself: the Sounds row's grid cell and the app bar's margins and flex are its parents' (SoundsRow, AppBar), except the shrink rule of the `line` size (D3). The tooltip is the parent's key passed as `tip`, wired by the `tipAction` it passes (L3).

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `name` | `string` | — | The live rack's name as shown ("Sunday drive", "Recovered: Sunday drive", "Untitled rack"), drawn as given. Empty: "Untitled rack" (D7). |
| `slot` | `string \| null` | `null` | The Quick Rack button that holds the live rack, as bank letter and number ("A1"). `null`: no slot is drawn and the label reads "Rack" alone (Stage.md D21). The pure `rackSlot` below computes it. |
| `modified` | `boolean` | `false` | `liveRack.modified`: the 5px modified dot after the name. |
| `size` | `'cell' \| 'line'` | `'cell'` | `cell`: the Stage's two-line cell in the Sounds row (44 tall, fills its container's width). `line`: the app bar's one-row form on every other page (32 tall, D1). |
| `current` | `boolean` | `false` | The readout is the page on view (the Rack page): the chosen face, `aria-current="page"`, and a click does nothing. Only with `size: 'line'`; ignored with `cell` (D2). |
| `tip` | `string \| undefined` | — | The tooltip key, rendered as `data-tip` on the `<button>` (the parents pass `stage.rack_name`, which exists in `tooltips.ts`). No attribute when undefined (L3). |
| `tipAction` | `Action<HTMLElement, string> \| undefined` | — | The app's `use:tip`, passed in by the wiring. When both it and `tip` are set the `<button>` gets `use:tipAction={tip}`; otherwise nothing (L3). |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onpress` | click, Space or Enter, unless `current` (with `size: 'line'`) | `()` |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Pure functions (`app/src/ui/RackReadout/rack.ts`)

- `rackSlot(bank: number, buttons: { loaded: boolean }[]): string | null`: the letter `String.fromCharCode(65 + bank)` and the 1-based index of the first entry with `loaded` true ("A1"); no such entry: `null`. The wiring calls it with `quickRacks.bank` and `quickRacks.buttons` (Stage.md D21: only the bank on view is known until contract change C1, `quickRacks.loaded`, lands; then the wiring passes that slot instead and this function goes).
- `rackLabel(r: { name: string; slot: string | null; modified: boolean; current: boolean }): string`: the accessible name, "Rack: {name}{, modified}{, on Quick Rack {slot}}. {Opens the Rack page | This page}" with `name` after the "Untitled rack" fallback. Examples: `{ name: 'Sunday drive', slot: 'A1', modified: true, current: false }` → "Rack: Sunday drive, modified, on Quick Rack A1. Opens the Rack page"; `{ name: 'Sunday drive', slot: null, modified: false, current: false }` → "Rack: Sunday drive. Opens the Rack page"; the first with `current: true` → "Rack: Sunday drive, modified, on Quick Rack A1. This page"; `{ name: '', slot: null, modified: false, current: false }` → "Rack: Untitled rack. Opens the Rack page".

### Visual rules

- **Tokens used:** `--t`, `--m`, `--g`, `--line`, `--focus`, `--font-sans`, `--text-12`, `--text-14`, `--weight-regular`, `--weight-medium`, `--space-2`, `--space-6`, `--space-10`, `--line-width`, `--focus-offset`. All exist in `app/src/ui/tokens/*` today: no new tokens (L1).
- **Element:** one native `<button type="button">`, `margin: 0`, no radius (except the current block, below), `background: transparent`, `font-family: var(--font-sans)`, `font-variant-numeric: tabular-nums`, `white-space: nowrap`, `text-align: left`, `box-sizing: border-box`, cursor `pointer`. Its text parts are separate spans, each with `data-part`: `label` ("Rack"), `sep` (" · ", `cell` only, only with a slot), `slot` ("A1", only with a slot), `name`; then the dot. No literal spaces between the spans in `line` (the gap spaces them; Rack.md RK-D29).
- **`cell`** (the Stage):
  - The button is `width: 100%` of its container, height 44, `border: 0; border-top: var(--line-width) solid var(--line)` (the Sounds row's 1px top edge, drawn by each cell, so the content box is 43), padding 0, a flex column, `justify-content: center`, `align-items: flex-start`, gap `--space-2`.
  - Line 1: a span, 12 / 400 (`--text-12`, `--weight-regular`), line-height 16, `--m`: `label` "Rack", then, with a slot, `sep` " · " and `slot` "A1" in `--t`. So it reads "Rack · A1", or "Rack".
  - Line 2: a flex row, items centred, gap `--space-6`, `min-width: 0`, `max-width: 100%`: `name` 14 / 400, line-height 16, `--t`, `min-width: 0`, `overflow: hidden`, `text-overflow: ellipsis`; then the dot when `modified`.
  - The two lines and the gap are 34px, centred in the 43 (4.5 above and below), so nothing sets a vertical offset by hand (Stage.md › Sounds row).
- **`line`** (the app bar, not current; Channel.md › Kit additions › App bar, page variant):
  - The button is 32 tall, no border, padding 0, `display: flex; align-items: baseline; gap: var(--space-6)`, `flex: 0 1 auto; min-width: 0` (the one item in the app bar that shrinks, D3), 14 / 400, line-height normal.
  - `label` "Rack" `--m`; `slot` "A1" `--t` (absent without a slot); `name` `--t`, `min-width: 0`, `overflow: hidden`, `text-overflow: ellipsis`; the dot, `align-self: center`, when `modified`.
  - With `align-items: baseline` the text line sits at the top of the 32px box and the dot at its middle, as the board draws them (text cap height at y 30–39, dot at 39–43 in a box at y 26).
- **`line` with `current`** (the Rack page; Rack.md › Kit additions › App bar, page variant):
  - The chosen face: `background: var(--t)`, every text part and the dot in `--g`, weight 500 (`--weight-medium`), 24 tall, padding `0 var(--space-10)`, items centred (`align-items: center`), gap `--space-6`, radius 0, `align-self: flex-end` (it sits on the app bar's line, as the board draws it), cursor `default`. Shrinks and ellipsizes as `line`.
  - `data-face="chosen"` on the button; `data-face="off"` in every other look (D8).
- **The dot:** a `span` with `data-dot="modified"`, `display: inline-flex`, `flex: none`, holding the StatusDot (Children). Absent when `modified` is false (no space kept).
- **States drawn by:** `modified` (the dot); `slot` (the slot span, and in `cell` the " · "); `current` (the block above). No hover or pressed look (kit D35). Keyboard focus: a `--line-width` outline in `--focus` at `--focus-offset` on `:focus-visible`, also on the current block (it stays focusable).
- **Type:** DM Sans, tabular numerals; `cell` 12 / 400 and 14 / 400 at line-height 16; `line` 14 / 400 (current 500), line-height normal.
- **Contrast (AA 4.5:1, `tokens/contrast.test.ts`):** existing rows `--m` on `--g` and `--t` on `--g`; the current block's `--g` on `--t` is Button's new row ("chosen label (Button)", 21:1 dark, 16.72:1 light), which lands with the tokens contract PR before this build. No new rows.
- **Motion:** none.

### Children

| Child | When | Props passed | Callback |
|---|---|---|---|
| `StatusDot` | `modified` | `{ size: 'sm', hue: current && size === 'line' ? 'g' : 't', glow: false }` (decorative, `aria-hidden`: the button's `aria-label` says "modified") | — |

### Accessibility

- **Role and name:** a `button`; `aria-label` = `rackLabel({ name, slot, modified, current: current && size === 'line' })`. With `current` (and `line`): `aria-current="page"`. No `aria-pressed`, no `aria-disabled` (the current block isn't disabled; it is the page, D2).
- **Keyboard:** Tab focuses it (also when current, first in the app bar's tab order on the Rack page); Space or Enter calls `onpress`, except when current.
- **Tooltip id:** `stage.rack_name` (exists), passed as `tip` with `tipAction` (L3).

## Stories (Story station)

Title `Primitives/RackReadout`. Every story renders in dark and light (the toolbar theme). The meta's `args` are `{ onpress: fn(), tipAction: fn() }` (L3). `cell` stories render inside a decorator `div` of `width: 200px` (the Sounds row's rack column), `layout: 'centered'`; `line` stories render as they are, `layout: 'centered'`.

**Controls (argTypes):** `name`, `slot`, `tip` text (`slot` cleared = `null`); `modified`, `current` boolean; `size` a select of `cell` / `line`; `onpress`, `tipAction` actions.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ name: 'Sunday drive', slot: 'A1', modified: true, tip: 'stage.rack_name' }` | the Stage's Sounds row rack cell at the board fixture (`liveRack` "Sunday drive", modified; Quick Rack A1 loaded): "Rack · A1" over "Sunday drive •", the 1px top edge | `Board-{dark,light}.png` (Stage 49,351 200×44) | the button named "Rack: Sunday drive, modified, on Quick Rack A1. Opens the Rack page" has `data-tip="stage.rack_name"`, `data-face="off"`, no `aria-current`; its `[data-part]` spans read "Rack", " · ", "A1", "Sunday drive"; it contains one `[data-dot="modified"]`; `tipAction` was called with the button and `'stage.rack_name'`; click, Space, Enter → `onpress` called 3 times |
| `NoSlot` | `{ name: 'Sunday drive' }` | no loaded Quick Rack button: "Rack" alone over the name, no dot | — (no board draws it, D9) | the spans read "Rack", "Sunday drive"; no `[data-part="sep"]`, no `[data-part="slot"]`, no `[data-dot]`; the name is "Rack: Sunday drive. Opens the Rack page" |
| `Untitled` | `{ name: '' }` | "Untitled rack" | — | the `name` span reads "Untitled rack" |
| `LongName` | `{ name: 'Recovered: Sunday drive for the late set at the harbour', slot: 'B8', modified: true }` | the name ends in "…" inside the 200px; the dot stays | — (no board draws it) | the dot is still in the DOM; the `name` span's `textContent` is the whole name (the ellipsis is CSS) |
| `Line` | `{ size: 'line', name: 'Sunday drive', slot: 'A1', modified: true, tip: 'stage.rack_name' }` | the app bar's one-row readout: "Rack" grey, "A1 Sunday drive" white, the dot | `Line-{dark,light}.png` (Channel 107,26 152×32) | the spans read "Rack", "A1", "Sunday drive" (no `sep`); one `[data-dot="modified"]`; click → `onpress` called once |
| `LineNoSlot` | `{ size: 'line', name: 'Sunday drive' }` | "Rack Sunday drive" | — | no `[data-part="slot"]` |
| `Current` | `{ size: 'line', current: true, name: 'Sunday drive', slot: 'A1', modified: true, tip: 'stage.rack_name' }` | the Rack page's app bar: the white 24px block, dark text and dot | `Current-{dark,light}.png` (Rack 108,35 171×24) | `aria-current="page"`, `data-face="chosen"`, the name ends "This page"; click, then Enter → `onpress` not called; the button has no `aria-disabled` |
| `CurrentCell` | `{ current: true, name: 'Sunday drive', slot: 'A1' }` | `current` is ignored in `cell`: the Stage look | — | no `aria-current`; `data-face="off"`; click → `onpress` called once |
| `LineSqueezed` | `Line`'s args with `name: 'Recovered: Sunday drive for the late set at the harbour'`, in a decorator `div` of `width: 160px; display: flex` | the `line` form shrinks with its parent: the name ellipsizes, "Rack", the slot and the dot keep their size | — | the dot is in the DOM |
| `Focused` | `Board`'s args, `parameters: { pseudo: { focusVisible: true } }` | the focus ring | — | — |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in dark and light; each crop is exactly the button's box (L6). The `Line` and `Current` boxes are measured on the renders (the `line` box starts at the "Rack" text's left edge and ends after the dot; the current block is the white fill), so the crop station confirms them when it cuts (D9).

**Unit tests** (`app/src/ui/RackReadout/rack.test.ts`, vitest): `rackSlot(0, [{ loaded: true }, …])` → "A1"; `rackSlot(1, [f, f, t])` → "B3"; no loaded entry → `null`; `[]` → `null`. `rackLabel` for each example in "Pure functions".

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`); `rack.test.ts` passes.
- `npm run shots -- RackReadout` passes for `Board`, `Line` and `Current` (score at most 0.02, or the Inspect agent judges any difference render noise), and axe finds no violation on any story.
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules (44, 32, 24, 16).
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · One component, two sizes; Channel's name wins.** The tall pages each named the app bar's one-row readout differently (Effects `variant="bar"`, Harmony `variant inline`, Browser `layout row`, Channel `size line`, Rack a new `RackReadoutTab`). Channel.md › Kit additions is canonical for the tall-page frame (kit.md line 11), so it is `size: 'line'` here, beside the Stage's `size: 'cell'` (default), and Rack's tab is `current` on the same component, not a new one.
- **D2 · `current` is a look, not disabled.** On the Rack page the readout is the page on view (Rack.md): `aria-current="page"`, the chosen face, still focusable, and a click does nothing, so it carries no `aria-disabled` (L5). It applies only to `line`; the Stage's cell is never the current page.
- **D3 · The width rules of the tall pages, reconciled to Channel.** Effects caps the name at 200px, Browser at 160, Rack at 240, Harmony has no ellipsis; Channel has no cap and makes the readout the one shrinking item (`flex: 0 1 auto; min-width: 0`, name ellipsized, "Rack", the slot and the dot kept). The component follows Channel: the `line` root shrinks and the name ellipsizes; the left margin (16 or 24px after the wordmark, which the specs also disagree on) is AppBar's, not this component's.
- **D4 · Items on the baseline.** The `line` form keeps Channel's `align-items: baseline` (the text at the top of the 32px box, the dot centred), which is what the page boards draw; Rack's current block is centred, as its board draws.
- **D5 · Spans per part.** Every size draws "Rack", the slot and the name as separate `data-part` spans (Rack.md RK-D29), so tests read "Rack", "A1", "Sunday drive" one by one on every page; in `cell` a `sep` span holds " · ", so the first line's `textContent` is still "Rack · A1" (Stage.md Check 7).
- **D6 · The current dot is `--g`.** On the white block the dot is `--g` (Rack.md: "`--g` text and dot"); StatusDot's `hue` union has no `g` today, so StatusDot must gain `'g'` before the current look is built (a one-value change to StatusDot's spec and component, outside this folder; reported to the lead).
- **D7 · The name is the wiring's.** Stage.md reads `liveRack.name` raw and Rack.md RK-D23 wants `rackName(liveRack)` ("Untitled rack" for a new rack) everywhere; the component draws the `name` it is given and only falls back to "Untitled rack" when it is empty, so either rule works and the Stage switches by changing what its wiring passes.
- **D8 · `data-face`.** The button carries `data-face="chosen"` when current and `"off"` otherwise (kit D41), so page tests read the current item without colours.
- **D9 · Crops.** `Board` is the folder's existing crop (Stage 49,351 200×44). `Line-{dark,light}.png` (Channel 107,26 152×32) and `Current-{dark,light}.png` (Rack 108,35 171×24) don't exist yet; they are measured on the renders and cut later by the crop station. FirstRun's "Rack · —" contradicts Stage.md D21, so no story copies it.
- **D10 · L3, L5.** `tip` (rendered as `data-tip`) and `tipAction` (applied when both are set); stories use `stage.rack_name` (exists) and `tipAction: fn()`. The readout is never disabled, so it never renders `aria-disabled`.

Follow-ups: when contract change C1 (`quickRacks.loaded`) lands, the wiring passes its slot and `rackSlot` goes; StatusDot gains `hue: 'g'` (D6).

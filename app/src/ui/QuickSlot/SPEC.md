# QuickSlot

## Identity (all stations)

- **Kind:** primitive
- **Built from:** — (uses the shared `longpress` action, `app/src/ui/actions/longpress`, spec `app/src/ui/actions/longpress/SPEC.md`; reads Pad's pure `padK` from `app/src/ui/Pad/led.ts` for the flash, D6)
- **Purpose:** One Quick Rack button: press it to load the rack it holds, hold it to put the live rack on it, and see at a glance which one is loaded, which are empty and when Store or Clear is waiting for a press.
- **Boards:**
  - `Browser-Dark.dc.html:161-166` (the eight slots: A1 "Sunday drive" loaded, A2 "Warm keys", A3 "Lead synth", A4 "Organ" stored, A5–A8 "Empty"); the face rules as data at `:442-449`; light: `Browser-Light.dc.html:145-150`, data `:438-445`.
  - Specified in `docs/specs/push/Browser.md` › Kit additions › Library frame › Quick Racks bar (the slots table, BR-D19, BR-D20, BR-D33).
  - No board draws a slot armed for Store or Clear, waiting for a save, or missing (D9).
- **Not this component's job:** no store, no API, no Tauri. It doesn't know what a press does: the parent (QuickRacksBar) turns `onpress` into `pressQuickRack` or `clearQuickRack` and `onlongpress` into `storeRack`, and decides which slots wear the armed outline. No timer of its own: the long press is `use:longpress`, the flash is the `led` prop (axiom 10). It doesn't raise the save or unsaved-changes prompts (Prompts #504; the interim `RackPrompt` is the bar's overlay). The tooltip is the parent's key passed as `tip`, wired by `tipAction` (L3).

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `index` | `string` | — | The slot's code: the bank letter and the slot + 1 ("A1"…"H8"). |
| `rack` | `'empty' \| 'stored' \| 'loaded' \| 'missing'` | `'empty'` | What the button holds, from `QuickRackButton` (types.ts): `rack` null → `empty`; `missing` → `missing`; `loaded` → `loaded`; else `stored`. |
| `name` | `string` | `''` | The rack's name (`QuickRackButton.name`); ignored when `rack` is `empty` or `missing` (types.ts: it is empty then). |
| `armed` | `'store' \| 'saving' \| 'clear' \| null` | `null` | The waiting outline: `store` = Store armed (`quickRacks.store`), flashing in lime; `saving` = this slot is `quickRacks.storeWaiting` (steady lime); `clear` = Clear armed and this slot holds something (steady `--ending`). The parent decides per slot (QuickRacksBar). |
| `led` | `number` | `0` | The LED clock in beats (`ledBeats(surface.clock, now, receivedMs)`, Pad › `led.ts`); drives the Store-armed flash only. |
| `storable` | `boolean` | `true` | Whether a long press or right-click stores here. False while `quickRacks.readOnly` or Clear is armed (the parent's): the long press is off. |
| `width` | `number \| undefined` | — | A fixed width in px (the board's 77), inline `width: <n>px`. Default: fills its grid cell (`width: 100%`). |
| `tip` | `string \| undefined` | — | The tooltip key (`quick.1`…`quick.8`, by slot), rendered as `data-tip`; no attribute when undefined (L3). |
| `tipAction` | `Action<HTMLElement, string> \| undefined` | — | The app's `use:tip`; applied as `use:tipAction={tip}` only when both are set (L3). |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onpress` | click, Space or Enter, in every state (an empty slot is pressable, BR-D20); not for the click that ends a long press (the action swallows it) | `()` |
| `onlongpress` | held 350 ms (`--long-press`) without moving more than 4px, or right-clicked, or the context-menu key (the action), while `storable` | `()` |

**Long press wiring.** The `<button>` carries `use:longpress={{ onlongpress, disabled: !storable || onlongpress === undefined }}`. No `onlongrelease`: storing happens on the long press itself.

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Visual rules

- **Tokens used:** `--g`, `--t`, `--m`, `--d`, `--btn`, `--warn`, `--ending`, `--lamp-line`, `--focus`, `--focus-offset`, `--line-width`, `--radius`, `--control-height`, `--font-sans`, `--font-mono`, `--text-11`, `--text-12`, `--weight-regular`, `--space-4`, `--space-6`, `--space-10`; new: `--stored-ring`, `--slot-glow-mix` (Browser.md › Kit additions › Tokens), `--sel-index`, `--slot-gap`, `--slot-line` (below); through the action, `--long-press` (LampButton's spec).

#### New tokens

Not in `app/src/ui/tokens/*` today; they land in the orchestrator's tokens contract PR before this component is built (L1).

| Token | Dark | Light | Used for |
|---|---|---|---|
| `--stored-ring` | `inset 0 -2px 0 color-mix(in srgb, var(--t) 40%, transparent)` | the same | a stored slot's bottom lamp (Browser.md) |
| `--slot-glow-mix` | `25%` | `0%` | the loaded slot's glow strength (Browser.md) |
| `--sel-index` | `color-mix(in srgb, var(--g) 55%, transparent)` | the same | the index on the loaded (white) slot (D4) |
| `--slot-gap` | `5px` | `5px` | the gap between the index and the name (`scale.css`) |
| `--slot-line` | `13px` | `13px` | the name's line height, two lines in 26 of the 32px (`scale.css`) |
| `--orange-700` (changed) | — | `#c85f00` → **`#a04c00`** | light `--warn`, so "Missing" passes on `--btn` (Contrast, L2) |

- **Box:** a native `<button type="button">`, `box-sizing: border-box`, height `--control-height` (32), width `width` px or 100%, `min-width: 0`, `margin: 0`, `border: 0`, radius `--radius`, padding `0 var(--space-6)`, `display: flex`, `align-items: center`, `justify-content: flex-start`, gap `--slot-gap`, `text-align: left`, `overflow: hidden`, `position: relative`, `cursor: pointer`, `font-family: var(--font-sans)`. The edge is drawn with `box-shadow` only (never `border`), so every face has the same box (as Button D3).
- **Index:** a `span`, `flex: none`, `--font-mono`, `--text-11`, `--weight-regular`, line-height normal, `aria-hidden="true"` (the accessible name carries it).
- **Name:** a `span`, `flex: 1 1 auto`, `min-width: 0`, `--text-12`, `--weight-regular`, `line-height: var(--slot-line)`, `white-space: normal`, at most two lines: `display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: 2; overflow: hidden` (D7).
- **Missing mark:** before "Missing", an inline 11 × 11 `svg` (`aria-hidden`), the ⚠ of PartMarks (geometry as attributes, paint in CSS, PartMarks D9): `<svg class="warn" width="11" height="11" viewBox="0 0 12 12"><path d="M6 1.5 L11 10.5 H1 Z" /><path d="M6 5 V7.4" /><circle cx="6" cy="8.9" r="0.6" /></svg>` with `.warn { fill: none; stroke: var(--warn); stroke-width: 1; stroke-linejoin: round } .warn circle { fill: var(--warn); stroke: none }`; it and the word sit in one `span` (`display: inline-flex; align-items: center; gap: var(--space-4)`).
- **Faces** (`data-face` on the `<button>`; `data-state` = `rack`):

  | `rack` | `data-face` | Fill | Name text | Index | Edge |
  |---|---|---|---|---|---|
  | `empty` | `off`, plus `data-empty="true"` (BR-D33) | none (`transparent`) | "Empty" `--m` | `--m` (D3) | `inset 0 0 0 var(--line-width) var(--d)` |
  | `stored` | `off` | `--btn` | `name` `--t` | `--m` | `--stored-ring` |
  | `missing` | `off` | `--btn` | ⚠ then "Missing" in `--warn` | `--m` | `--stored-ring` (D8) |
  | `loaded` | `chosen` | `--t` | `name` `--g` | `--sel-index` | `0 0 var(--space-10) color-mix(in srgb, var(--t) var(--slot-glow-mix), transparent)` (outside the box, L6) |

- **Armed** (`armed` set): `data-face="waiting"` and `data-hue="lamp"` (`store`, `saving`) or `"ending"` (`clear`); fill, text and index stay as the table gives for `rack`; the edge (ring or glow) is replaced by `inset 0 0 0 var(--line-width) <hue>`, the hue `--lamp-line` (D5) or `--ending`. `store` flashes: the hue is drawn at strength `k = padK('bright', 'flash', led)` (1 while `frac(led) < 0.5`, else 0.18) as `color-mix(in srgb, <hue> calc(var(--k) * 100%), transparent)` with `--k` set inline as a number (as Pad's `--k`) and `data-k` = `String(k)`; `saving` and `clear` are steady (`--k` 1). `data-empty` stays on an empty slot.
- **Keyboard focus** (`:focus-visible`): a `--line-width` outline in `--focus` at `--focus-offset` outside the box (the grid has a 4px gap, so it isn't clipped). Nothing on mouse focus.
- No hover or pressed look (kit D35); no disabled state (D2).
- **Type:** index JetBrains Mono 11; name DM Sans 12 / 400, sentence case as given.
- **Contrast (AA 4.5:1, `tokens/contrast.test.ts`):** existing rows `--m` on `--g` ("Empty", the empty index), `--m` on `--btn` (the stored index; 5.38 dark, 4.61 light). Button's new rows `--t` on `--btn` (stored name) and `--g` on `--t` (loaded name). New rows: `--sel-index` on `--t` ("loaded slot index (QuickSlot)": 4.76:1 dark, 5.63:1 light, passes); `--warn` on `--btn` ("missing slot (QuickSlot)"): dark 8.54:1 passes, **light 3.21:1 fails** with `#c85f00`, and 4.00:1 with AccentBlock's proposed `#b05300`; proposed light `--orange-700` `#a04c00`: 4.63:1 on `--btn`, 5.27:1 on `--g` (which also gives AccentBlock's `--g` on `--r3` 5.27:1, above its 4.55) (L2, D8). The armed outlines and the ⚠ are graphics, not text.
- **Motion:** the Store-armed flash, from `led` only (no CSS animation, no timer).

### Accessibility

- **Role and name:** a `button`, accessible name set as `aria-label`: "Quick Rack {index}, {name}" (stored), "…, {name}, loaded" (loaded), "Quick Rack {index}, empty", "Quick Rack {index}, rack missing"; then, when armed, ", tap to store here" (`store`), ", waiting for the rack to be saved" (`saving`), ", tap to clear" (`clear`) (D10). No `aria-pressed`, no `aria-disabled` (L5: never disabled). `data-tip` only with `tip`.
- **Keyboard:** Tab focuses it (an empty one too); Space or Enter calls `onpress`; the context-menu key (Menu or Shift+F10) is the long press, via the action.
- **Tooltip id:** `quick.1`…`quick.8` by slot (the parent passes it), with `tipAction` (L3). The long press is described in those keys' text and in `quick.store_rack`.

## Stories (Story station)

Title `Primitives/QuickSlot`, `layout: 'centered'`. Every story renders in dark and light. Meta `args`: `{ width: 77, led: 0.25, onpress: fn(), onlongpress: fn(), tipAction: fn() }` (axiom 7, L3; `led` 0.25 is the board's LED reading, the flash at full).

**Controls (argTypes):** `index`, `name`, `tip` text; `rack` a select of its four values; `armed` a select with `none` first (undefined → `null`), `store`, `saving`, `clear`; `led` a number (`step: 0.05`), so the flash can be scrubbed; `width` a number (cleared = undefined); `storable` boolean; the callbacks and `tipAction` actions.

**Timing in plays (L4):** `fireEvent.pointerDown` / `pointerUp(button, { pointerId: 1, button: 0, clientX: 0, clientY: 0 })`, real time, the long press awaited with `waitFor(…, { timeout: 1000 })`.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ index: 'A1', rack: 'loaded', name: 'Sunday drive', tip: 'quick.1' }` | the first slot on the Browser board: the loaded rack, a white block with "Sunday drive" on two lines | `Board-{dark,light}.png` (Browser 48,440 77×32; the glow is outside, judged in QuickRacksBar's crop, L6) | the button named "Quick Rack A1, Sunday drive, loaded" has `data-face="chosen"`, `data-state="loaded"`, no `data-empty`, `data-tip="quick.1"`; `tipAction` called with it and `'quick.1'`; click → `onpress` called once |
| `Stored` | `{ index: 'A2', rack: 'stored', name: 'Warm keys', tip: 'quick.2' }` | a stored slot: `--btn`, the bottom lamp ring, two-line name | `Stored-{dark,light}.png` (Browser 129,440 77×32) | `data-face="off"`, `data-state="stored"`; Space, then Enter → `onpress` called 2 times |
| `StoredOneLine` | `{ index: 'A4', rack: 'stored', name: 'Organ', tip: 'quick.4' }` | a one-line name | `StoredOneLine-{dark,light}.png` (Browser 291,440 77×32) | — |
| `Empty` | `{ index: 'A5', rack: 'empty', tip: 'quick.5' }` | no fill, a 1px `--d` ring, "Empty" | `Empty-{dark,light}.png` (Browser 48,476 77×32; see D3) | `data-face="off"`, `data-empty="true"`, no `aria-disabled`; named "Quick Rack A5, empty"; click → `onpress` called once |
| `Missing` | `{ index: 'A3', rack: 'missing', tip: 'quick.3' }` | ⚠ and "Missing" in `--warn` | — (no board draws one, D9) | named "Quick Rack A3, rack missing"; the text "Missing"; `data-state="missing"` |
| `StoreArmed` | `{ index: 'A2', rack: 'stored', name: 'Warm keys', armed: 'store', tip: 'quick.2' }` | the lime outline at full (led 0.25) | — (D9) | `data-face="waiting"`, `data-hue="lamp"`, `data-k="1"`; the name ends ", tap to store here" |
| `StoreArmedLow` | `StoreArmed` args with `led: 0.75` | the flash's low phase | — | `data-k="0.18"` |
| `Saving` | `{ index: 'A5', rack: 'empty', armed: 'saving', tip: 'quick.5' }` | a steady lime outline on the empty slot | — | `data-face="waiting"`, `data-empty="true"`, `data-k="1"` |
| `ClearArmed` | `{ index: 'A1', rack: 'loaded', name: 'Sunday drive', armed: 'clear', storable: false, tip: 'quick.1' }` | the white block, its glow replaced by a 1px `--ending` ring | — | `data-face="waiting"`, `data-hue="ending"`; pointer down, wait 500 ms (real time) → `onlongpress` not called; pointer up |
| `LongPress` | `{ index: 'A5', rack: 'empty', tip: 'quick.5' }` | — | — | pointer down → `await waitFor(() => expect(onlongpress).toHaveBeenCalledTimes(1), { timeout: 1000 })`; pointer up; `fireEvent.click(button)` → `onpress` not called; `fireEvent.contextMenu(button)` → `onlongpress` called 2 times |
| `NotStorable` | `{ index: 'A2', rack: 'stored', name: 'Warm keys', storable: false }` | — (read-only Quick Racks) | — | `fireEvent.contextMenu(button)` → `onlongpress` not called; click → `onpress` called once |
| `LongName` | `{ index: 'A6', rack: 'stored', name: 'Rhodes Soft + Strings + Choir Aahs' }` | the name clipped after two lines | — | — |
| `Focused` | `Stored` args, `pseudo: { focusVisible: true }` | the focus ring | — | — |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in dark and light; each crop is exactly the slot's own box (L6).

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- QuickSlot` passes for the four cropped stories (score at most 0.02, or the Inspect agent judges the difference render noise), and axe finds no violation on any story once the tokens contract PR has landed (until then `Missing` fails in light, 3.21:1).
- The long press comes only from `use:longpress` and the flash only from `led`; the component has no `setTimeout`.
- Only listed tokens are used; no inline colours, no literal sizes (the inline `width` and `--k` are props).
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · `rack` and `armed` are props, not a `QuickRackButton`.** The parent maps the API's button (`rack`, `name`, `missing`, `loaded`) and the bar's arm state to two small unions, so the primitive takes no API type and every face is a control.
- **D2 · Never disabled.** An empty slot is a store target and a press on it is the session's to refuse (BR-D20); read-only only turns the long press off (`storable` false). So QuickSlot has no `disabled` and never renders `aria-disabled` (L5).
- **D3 · The empty slot's index is `--m`.** Browser.md draws it in `--d`, which is 2.48:1 (dark) and 1.94:1 (light) on `--g` and fails AA; `--d` is the disabled role and can't change for one label, and the slot isn't disabled, so the index takes `--m` like "Empty" (6.25:1, 5.24:1) (L2). The `Empty` crop differs from the board in those two glyphs only, under the threshold; the Inspect agent judges it.
- **D4 · `--sel-index`.** The loaded slot's index is `--g` at 55%, a new token (Browser.md wrote the `color-mix` inline); kept apart from `--sel-mut` (60% / 70%) so the board's value holds in both themes.
- **D5 · Lime outline is `--lamp-line`.** Browser.md says the Store-armed outline is `--lamp`; `--lamp` on the ground is 3.74:1 in light, so the waiting lime is LampButton's `--lamp-line`, as Button D23 and LampButton D15 do.
- **D6 · The flash is Pad's.** "Flashing on the LED clock as a Next pad's border" is `padK('bright', 'flash', led)` from `app/src/ui/Pad/led.ts`, imported, not copied (Pad is built earlier in Stage's order); `saving` and `clear` are steady.
- **D7 · Two lines, clipped with the clamp.** "At most two lines, clipped" is `-webkit-line-clamp: 2`, which ends a cut name in an ellipsis on the second line; the board's names fit.
- **D8 · Missing is a stored face, in a passing orange.** A missing slot still holds a rack id, so it wears the stored look (`--btn`, the ring) with ⚠ and "Missing"; light `--warn` on `--btn` fails (3.21:1), so the tokens PR changes light `--orange-700` to `#a04c00` (4.63:1), which also satisfies AccentBlock's `--r3` change (`#b05300`) and should replace it (one value for both).
- **D9 · Armed and missing have no crop.** The Browser board draws Store and Clear disarmed and no missing slot; those stories are checked by `data-face` / `data-hue` / `data-k` and by eye.
- **D10 · Armed names.** The armed outline is the only sign of what a press will do, so the accessible name says it (", tap to store here", ", waiting for the rack to be saved", ", tap to clear"); Browser.md's template ends at ", loaded" and is extended, not changed.
- **D11 · L3, tooltip props.** `tip` / `tipAction` as LampButton; `quick.1`…`quick.8` exist in `tooltips.ts` today.
- **D12 · L4, plays.** Pointer plays use `fireEvent.pointerDown/Up` with `{ pointerId: 1, button: 0, clientX: 0, clientY: 0 }` and real time; QuickSlot calls no pointer-capture API.
- **D13 · No crop files yet.** The folder holds only this spec; the crops are cut later from the Browser renders (L6).

# QuickRacksBar

## Identity (all stations)

- **Kind:** complex
- **Built from:** GroupHeader, Button, LampButton, QuickSlot (D1: GroupHeader and Button each need one addition before this is built)
- **Purpose:** The eight Quick Racks of the bank on view, with the bank pager, Store and Clear: press a slot to load its rack, hold it to put the live rack there.
- **Boards:** `Browser-Dark.dc.html:151-168` (the header "Quick Racks ◀ A ▶ … Store ✕" and the 4 × 2 slots); light: `Browser-Light.dc.html:135-152`. Specified in `docs/specs/push/Browser.md` › Kit additions › Library frame › Quick Racks bar (BR-D19, BR-D20, BR-D27, BR-D33); commands in `docs/app-api.md` › Quick Racks and `QuickRackCmd` (`app/src/lib/api/types.ts`).
- **Not this component's job:** no store, no API, no Tauri. It reads `QuickRacksState` and the Clear arm from props and sends through `onsend`; it never holds `clearArmed` itself (the page does, Browser.md › Components, so Esc on the page and the wiring's "Store armed drops Clear" rule can reach it). It doesn't handle Esc (the page root does) and doesn't draw the save or unsaved-changes prompts (Prompts #504; until then the wiring passes today's `RackPrompt` as the `prompt` snippet, BR-D27). Not the Quick Racks page (the app bar's Quick Racks tab), which has its own screen spec.

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `quickRacks` | `QuickRacksState` | — | `bank`, `buttons` (8), `store`, `storeWaiting`, `readOnly` (types.ts; a type-only import from `app/src/lib/api/types`). |
| `clearArmed` | `boolean` | `false` | Clear is armed (app-only, BR-D19). |
| `led` | `number` | `0` | The LED clock in beats, passed to every slot for the Store-armed flash. |
| `width` | `number \| undefined` | — | A fixed width in px (320), inline `width: <n>px`; default fills its container. |
| `prompt` | `Snippet \| undefined` | — | Drawn over the slots grid when given (the interim `RackPrompt`, BR-D27). |
| `tipAction` | `Action<HTMLElement, string> \| undefined` | — | The app's `use:tip`, passed to every child (L3). |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onsend` | a control sends a command (table below) | `(cmd: QuickRackCmd)` |
| `onclearArmed` | Clear arms or disarms (table below) | `(on: boolean)` |

**What each press does** (`bank` = `quickRacks.bank`, `i` = the slot 0–7, `b` = `quickRacks.buttons[i]`):

| Control | Condition | Calls, in order |
|---|---|---|
| ◀ | enabled (`bank > 0`) | `onsend({ type: 'stepQuickRackBank', delta: -1 })` |
| ▶ | enabled (`bank < 7`) | `onsend({ type: 'stepQuickRackBank', delta: 1 })` |
| Store (toggle) | `clearArmed` | `onclearArmed(false)`, then `onsend({ type: 'toggleQuickRackStore' })` |
| Store (toggle) | otherwise | `onsend({ type: 'toggleQuickRackStore' })` |
| ✕ | `clearArmed` | `onclearArmed(false)` |
| ✕ | not armed, `quickRacks.store` | `onsend({ type: 'toggleQuickRackStore' })`, then `onclearArmed(true)` |
| ✕ | not armed | `onclearArmed(true)` |
| slot press | `clearArmed` and `b.rack !== null` (stored, loaded or missing) | `onsend({ type: 'clearQuickRack', bank, slot: i })`, then `onclearArmed(false)` |
| slot press | `clearArmed` and empty | nothing (Clear stays armed) |
| slot press | otherwise (Store armed or not: the session stores or loads) | `onsend({ type: 'pressQuickRack', slot: i })` |
| slot long press / right-click | not `readOnly`, not `clearArmed` | `onsend({ type: 'storeRack', slot: i })` |

Store and ✕ are disabled (no call) while `readOnly`; ◀ on bank A and ▶ on bank H likewise.

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| `prompt` | Optional. Rendered in an absolutely positioned box covering the slots grid (`position: absolute; inset: 0` inside the grid's `position: relative` wrapper), above the slots. |

### Children and the props passed

A `<section aria-label="Quick Racks">`, a column, width `width` px or 100%:

1. **GroupHeader** `{ title: 'Quick Racks', size: 'sm' }` (32 tall, gap 8; D1), its `children` snippet holding, in order:
   - **Button** `{ size: 'icon-sm', symbol: 'prev', name: 'Previous bank', disabled: bank === 0, tip: 'quick.bank_prev', tipAction, onpress }`;
   - the **bank letter**: a `span` with `data-tip="quick.bank"` (and `use:tipAction` when given), 18 / 300 `--t` (`--text-18`, `--weight-light`), its visible text `String.fromCharCode(65 + bank)` ("A") in an `aria-hidden` span, then a visually hidden span "Bank A of 8" (D4);
   - **Button** `{ size: 'icon-sm', symbol: 'next', name: 'Next bank', disabled: bank === 7, tip: 'quick.bank_next', tipAction, onpress }`;
   - a `span` (`display: flex; align-items: center; gap: var(--space-8); margin-left: auto`) holding **LampButton** `{ label: 'Store', size: 'sm', on: store, disabled: readOnly, tip: 'quick.store', tipAction, ontoggle }` and **Button** `{ size: 'icon-sm', symbol: 'cross', name: 'Clear: arm, then tap a slot to empty it', pressed: clearArmed, waiting: clearArmed, hue: 'ending', disabled: readOnly, tip: 'quick.clear', tipAction, onpress }`.
2. The **slots grid**, `margin-top: var(--space-8)`, `position: relative`, `display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); grid-auto-rows: var(--control-height); gap: var(--space-4)` (77 × 32 at 320), eight **QuickSlot**s for `buttons[0..7]`: `{ index: letter + (i + 1), rack, name: b.name, armed, led, storable: !readOnly && !clearArmed, tip: 'quick.' + (i + 1), tipAction, onpress, onlongpress }`, no `width` (they fill their cells), where
   - `rack` = `b.rack === null` → `'empty'`; `b.missing` → `'missing'`; `b.loaded` → `'loaded'`; else `'stored'`;
   - `armed` = `storeWaiting === i` → `'saving'`; else `store` → `'store'`; else `clearArmed && b.rack !== null` → `'clear'`; else `null`.
   Then the `prompt` snippet's box, when given.

Height: 32 + 8 + 32 + 4 + 32 = 108.

### Visual rules

- **Tokens used:** `--t`, `--control-height`, `--text-18`, `--weight-light`, `--space-4`, `--space-8`, `--line-width` (the visually hidden span's 1px box); the children's own. No new tokens of its own (QuickSlot's, Button's and GroupHeader's land with them, L1).
- **States drawn by:** the children (Store lit when `store`; ✕ in the waiting face in `--ending` when `clearArmed`; the slots' faces and outlines; ◀ ▶ Store ✕ disabled per the table). The bar adds nothing of its own.
- **Contrast:** `--t` on `--g` (the bank letter) exists; every other text is a child's row. No new rows (L1).
- **Motion:** the slots' Store-armed flash, from `led` (QuickSlot).

### Accessibility

- **Role and name:** a `region` named "Quick Racks" (the `section`'s `aria-label`), with the heading "Quick Racks" (GroupHeader, `h2`). Buttons as the children's; ✕ has `aria-pressed` = `clearArmed`; Store `aria-pressed` = `store`.
- **Keyboard:** tab order ◀, ▶, Store, ✕, slots 1–8 (the bank letter isn't focusable); Space or Enter presses; the context-menu key long-presses a slot. Esc disarming Clear is the page's (Browser.md › Keyboard).
- **Tooltip id:** `quick.bank_prev`, `quick.bank`, `quick.bank_next`, `quick.store`, `quick.clear`, `quick.1`…`quick.8`.

## Stories (Story station)

Title `Components/QuickRacksBar`, `layout: 'centered'`. Every story renders in dark and light. Fixtures (axiom 12): `app/src/ui/QuickRacksBar/QuickRacksBar.fixtures.ts` exports `boardQuickRacks: QuickRacksState` (Browser.md › Board fixture): `bank 0, store false, storeWaiting null, readOnly false`, buttons 1–4 `{ rack: 'rack-sunday-drive', name: 'Sunday drive', missing: false, loaded: true }`, `{ rack: 'rack-warm-keys', name: 'Warm keys', … loaded: false }`, `{ rack: 'rack-lead-synth', name: 'Lead synth', … }`, `{ rack: 'rack-organ', name: 'Organ', … }`, and 5–8 `{ rack: null, name: '', missing: false, loaded: false }`. Meta `args`: `{ quickRacks: boardQuickRacks, clearArmed: false, led: 0.25, width: 320, onsend: fn(), onclearArmed: fn(), tipAction: fn() }`.

**Controls (argTypes):** `quickRacks` an object control, plus its fields broken out under `table.category: 'quickRacks'` as `bank` (number 0–7), `store`, `readOnly` (booleans), `storeWaiting` (number, cleared = null), mapped into `quickRacks` by the meta's `render`; `clearArmed` boolean; `led` a number (`step: 0.05`); `width` a number; `onsend`, `onclearArmed`, `tipAction` actions. `prompt` gets no argType (a snippet is content, `stories.test.ts`).

**Timing in plays (L4):** pointer plays use `fireEvent.pointerDown` / `pointerUp(el, { pointerId: 1, button: 0, clientX: 0, clientY: 0 })` and real time (`waitFor(…, { timeout: 1000 })`).

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | — (the meta's) | the Browser board's bar: bank A, A1 loaded, A2–A4 stored, A5–A8 empty | `Board-{dark,light}.png` (Browser 48,400 320×108; A1's glow left of x 48 is outside, judged in the page shot, L6) | a region named "Quick Racks"; the slots read, in order, "A1 Sunday drive" (`data-face="chosen"`), "A2 Warm keys", "A3 Lead synth", "A4 Organ" (`data-face="off"`), "A5 Empty"… "A8 Empty" (`data-face="off"`, `data-empty="true"`, no `aria-disabled`); "Previous bank" has `aria-disabled="true"`; click A2 → `onsend` called with `{ type: 'pressQuickRack', slot: 1 }`; pointer down on A5, `waitFor` → `onsend` called with `{ type: 'storeRack', slot: 4 }`, pointer up, and no `pressQuickRack { slot: 4 }` was sent; click "Store" → `{ type: 'toggleQuickRackStore' }`; click "Next bank" → `{ type: 'stepQuickRackBank', delta: 1 }`; click the ✕ → `onclearArmed(true)` and `onsend` called no further |
| `StoreArmed` | `{ quickRacks: { …boardQuickRacks, store: true } }` | Store lit; every slot outlined in lime at full (led 0.25) | — (not drawn, QuickSlot D9) | every slot has `data-face="waiting"` and `data-hue="lamp"`; click the ✕ → `onsend` called with `{ type: 'toggleQuickRackStore' }`, then `onclearArmed(true)` |
| `ClearArmed` | `{ clearArmed: true }` | ✕ in the `--ending` waiting face; A1–A4 outlined in `--ending`; A5–A8 unchanged | — | the ✕ has `aria-pressed="true"`, `data-face="waiting"`, `data-hue="ending"`; A1–A4 `data-hue="ending"`, A5 `data-face="off"`; click A2 → `onsend` called with `{ type: 'clearQuickRack', bank: 0, slot: 1 }`, then `onclearArmed(false)`; click A5 → no further call; pointer down on A3, wait 500 ms → no `storeRack`; click "Store" → `onclearArmed(false)`, then `{ type: 'toggleQuickRackStore' }` |
| `Saving` | `{ quickRacks: { …boardQuickRacks, storeWaiting: 4 } }` | A5 outlined steady in lime (waiting for the save) | — | A5 has `data-face="waiting"`, `data-k="1"`; the others not |
| `ReadOnly` | `{ quickRacks: { …boardQuickRacks, readOnly: true } }` | Store and ✕ dimmed; slots still load | — | "Store" and the ✕ have `aria-disabled="true"`; click "Store" → `onsend` not called; click A2 → `pressQuickRack { slot: 1 }`; `fireEvent.contextMenu(A2)` → no `storeRack` |
| `LastBank` | `{ quickRacks: { …boardQuickRacks, bank: 7, buttons: <the board's eight, every `loaded` false> } }` | "H", ▶ disabled, H1…H8 | — | the text "H" is shown and "Bank H of 8" is in the region; "Next bank" has `aria-disabled="true"`; the first slot is named "Quick Rack H1, Sunday drive" |
| `Missing` | `{ quickRacks: { …boardQuickRacks, buttons: <A3 as `{ rack: 'rack-lead-synth', name: '', missing: true, loaded: false }`> } }` | A3 reads ⚠ "Missing" | — | the slot named "Quick Rack A3, rack missing"; with `clearArmed: true` re-rendered it is `data-hue="ending"` (missing slots can be cleared, D3) |
| `Prompt` | `{ prompt: <a snippet rendering `<div role="group" aria-label="Save rack">` with a "Save rack" button> }` | the prompt box over the grid | — | a group named "Save rack" is inside the region, after the eight slots |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in dark and light (L6). The `Board` crop includes every child, so GroupHeader, Button, LampButton and QuickSlot must be built first.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- QuickRacksBar` passes for `Board` (score at most 0.02, or the Inspect agent judges the difference render noise), and axe finds no violation on any story once the tokens contract PR has landed (`Missing` fails in light until then, QuickSlot D8).
- The commands sent are exactly the table's; nothing sends `pressQuickRack` while Clear is armed.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · Two child additions (outside this folder).** Browser.md's rows 2 and 8 need Button `size: 'icon-sm'` (28 × 28, 11px glyph, for ◀ ▶ ✕) and a `symbol: 'cross'` (the 10 × 10 SVG cross, `M2.5 2.5 L9.5 9.5 M9.5 2.5 L2.5 9.5`, stroke `currentColor` 1.2, so it takes the face's label colour: `--t2` off, `--ending` waiting, `--d` disabled), and GroupHeader `size: 'sm'` (32 tall, `--control-height`, gap `--space-8`). Neither is in their SPEC.md today; they land in those specs before this component is built.
- **D2 · Clear lives in the page.** `clearArmed` is a prop and `onclearArmed` its setter, so the page can disarm it on Esc and the wiring can drop it when `quickRacks.store` turns true from anywhere (the Launchkey, F5) (BR-D19); the bar only follows the table.
- **D3 · Clear acts on any slot that holds a rack.** "A stored or loaded slot" is read as `rack !== null`, so a missing slot (its rack gone) can be cleared too: that is the one fix for it, and `clearQuickRack` takes any button.
- **D4 · The bank letter's name.** A plain `span` can't carry `aria-label` (axe's `aria-prohibited-attr`), so the visible "A" is `aria-hidden` and a visually hidden "Bank A of 8" follows it.
- **D5 · Storing is off while Clear is armed.** A long press while Clear is armed does nothing (Browser.md), so the slots get `storable: false` then; read-only does the same.
- **D6 · `saving` beats `store`.** A slot in `storeWaiting` shows the steady outline even if Store is armed again, so the slot that's waiting for the save stays findable.
- **D7 · The prompt is a snippet.** The library can't import `panels/quickracks/RackPrompt.svelte`, so the wiring passes it as `prompt`, drawn over the grid as `QuickBar.svelte` does (BR-D27); Prompts (#504) replaces it.
- **D8 · `onsend` is narrow.** The bar's `onsend` takes `QuickRackCmd`, a subset of the page's `AppCmd`, so the page passes its own `onsend` straight through.
- **D9 · L3.** Every child gets its key and the shared `tipAction`; all of `quick.bank_prev`, `quick.bank`, `quick.bank_next`, `quick.store`, `quick.clear`, `quick.1`…`quick.8` exist today. `quick.bank` and `quick.clear` are rewritten by Browser.md C1 (contract change needed: "drop 'click a letter'"; "arm it, then tap a Quick Rack button to empty it").
- **D10 · No crop files yet.** The folder holds only this spec; the crop is cut later from the Browser renders (L6).

# BankRow

## Identity (all stations)

- **Kind:** primitive
- **Built from:** —
- **Purpose:** Shows the Quick Racks bank on view as eight cells, so the player sees which slot the rack is about to be stored to and what each slot holds.
- **Boards:**
  - `Prompts-Dark.dc.html:410-419` (the Store prompt's row: A1 loaded, A2–A4 stored, A5 waiting with "here", A6–A8 empty); light: `Prompts-Light.dc.html:402-411`.
- **Not this component's job:** no store, no API, no Tauri. It is a picture, not a control: no click, no focus, no tooltip, no callback. It doesn't pick the bank or the slot (the Store prompt passes `quickRacks.bank`, `.buttons` and `.storeWaiting`) and isn't the Quick Racks bar or pad page (those are controls of their own).

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `bank` | `number` | — | The bank on view, 0–7 (A–H): the letter of every slot label and of the `aria-label`. |
| `buttons` | `BankCell[]` | — | The eight buttons of that bank, in slot order: `quickRacks.buttons`. `BankCell` is `{ rack: string \| null; missing: boolean; loaded: boolean }`, declared in this folder, structurally a subset of the API's `QuickRackButton` (D6). |
| `waiting` | `number \| null` | `null` | The slot asked for (`quickRacks.storeWaiting`), 0–7; null: no waiting cell. |

`bankRowLabel(bank, buttons, waiting)` and `slotLabel(bank, i)` are pure functions exported from `app/src/ui/BankRow/bankRow.ts` (D6): `slotLabel(0, 4)` is "A5" (`String.fromCharCode(65 + bank)` then `i + 1`, as `quickLabel` in `app/src/lib/api/quick-racks.ts`).

### Events

| Callback | Fires when | Payload |
|---|---|---|
| — | | |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Visual rules

- **Tokens used:** `--btn`, `--t`, `--t2`, `--g`, `--warn`, `--d`, `--radius`, `--line-width`, `--font-sans`, `--text-11`, `--text-13`, `--weight-regular`, `--weight-medium`, `--space-4`, `--space-6`, `--space-14`, `--space-16`, `--bank-row-height` (new).

#### New tokens

Not in `app/src/ui/tokens/*` today; they land in the orchestrator's tokens contract PR before this component is built (L1).

| Token | Dark | Light | Used for |
|---|---|---|---|
| `--bank-row-height` | `44px` | `44px` | the row's and each cell's height (`scale.css`) |

- **Row:** a `<div role="img" aria-label={bankRowLabel(…)}>`, `height: var(--bank-row-height)`, `display: grid; grid-template-columns: repeat(8, minmax(0, 1fr)); gap: var(--space-6)`, `width: 100%`, `min-width: 0` (in the Dialog's 390px content width each cell is 43.5 × 44).
- **Cell:** a `<div data-slot={i} data-face=… aria-hidden="true">`, `border-radius: var(--radius)`, `box-sizing: border-box`, `position: relative`, `padding: var(--space-6) 0 0 calc(var(--space-6) + var(--line-width))` (6 top, 7 left), no border. Its label is a `<span data-hue=…>` with `slotLabel(bank, i)`, `--text-13` / `--weight-medium`, line-height `--space-16`.
- **Faces**, from `buttons[i]` and `waiting`, exclusive, ranked top to bottom (the first that applies wins: waiting, then loaded, then missing, then stored, then empty; Prompts.md PR-D27, PR-D29):

  | Cell | When | Fill | Edge | Label | `data-face` | label `data-hue` |
  |---|---|---|---|---|---|---|
  | Waiting | `i === waiting` | `--btn` | `box-shadow: inset 0 0 0 var(--line-width) var(--t)` (D2) | `--t`, plus "here" | `waiting` | `t` |
  | Loaded | `loaded` | `--t` | none | `--g` | `chosen` | `g` |
  | Missing | `missing`, not loaded | `--btn` | none | `--warn` (Prompts.md PR-D8) | `off` | `warn` |
  | Stored | `rack` not null | `--btn` | none | `--t2` | `off` | `t2` |
  | Empty | `rack` null | `--btn` | none | `--d` | `off` | `d` |

- **"here"** (the waiting cell only): a `<span>` "here", `--text-11` / `--weight-regular`, line-height `--space-14`, `--t2`, `position: absolute; left: calc(var(--space-6) + var(--line-width)); bottom: calc(var(--space-4) + var(--line-width))` (7 and 5 from the cell's edge, as the board's 6 and 4 inside its 1px border).
- **States drawn by:** the five faces above; a stored slot being stored over is the waiting face (waiting outranks); a loaded rack whose file is gone reads loaded. No hover, focus or pressed look (not a control); `cursor: default`.
- **Type:** DM Sans, labels 13/500, "here" 11/400, `font-variant-numeric: tabular-nums`.
- **Contrast (AA 4.5:1, `tokens/contrast.test.ts`):** the cells are `aria-hidden` (the row's `aria-label` is the whole reading), so axe skips their text, as Prompts.md › Bank row says. The pairs drawn: `--g` on `--t` (Button's new row, 21:1 / 16.72:1), `--t` on `--btn` (Button's new row, 18.10:1 / 14.70:1), `--t2` on `--btn` (exists; also "here"). Below AA and exempt as hidden picture text: `--d` on `--btn` (dimmed, the kit's), and `--warn` on `--btn` in light, 3.21:1 (4.00:1 after AccentBlock's light `--orange-700` change); dark 8.54:1. No new row (D4).
- **Motion:** none.

### Accessibility

- **Role and name:** the row is `role="img"` with `aria-label` from `bankRowLabel(bank, buttons, waiting)`: "Bank {letter}: " then the cells grouped into runs of consecutive slots in the same state, each run "{first} {state}" or "{first} to {last} {state}", states `loaded`, `stored`, `missing`, `empty`, and the waiting cell `chosen` (a run of its own), runs joined by ", " ("Bank A: A1 to A4 stored, A5 chosen, A6 to A8 empty"). The state of a cell for the label uses the same rank as the faces. Every cell is `aria-hidden="true"`.
- **Keyboard:** none; nothing in it is focusable.
- **Tooltip id:** none (not a control, L3).

## Stories (Story station)

- **Title:** `Primitives/BankRow` (D5).
- **Layout:** `centered`, the row inside a 390px-wide wrapper (a story decorator: the Dialog's content width).

Every story renders in dark and light. The fixture buttons are Prompts.md › Board fixture's: A1–A4 hold `rack-sunday`, `rack-warm`, `rack-organ`, `rack-ballad` (`missing` false), A5–A8 `{ rack: null, missing: false, loaded: false }`; "stored" below means `{ rack: <id>, missing: false, loaded: false }`.

**Controls (argTypes):** `bank` a number 0–7; `waiting` a number 0–7 (cleared = null); `buttons` an object control.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ bank: 0, buttons: <storeState: A1–A4 stored, A5–A8 empty>, waiting: 4 }` | the Store fixture: A1–A4 stored, A5 waiting with "here", A6–A8 empty | `Board-{dark,light}.png` (Prompts 293,243 390×44), mask `['[data-slot="0"]']` (the board draws A1 loaded, Prompts.md PR-D20) | the `img` is named "Bank A: A1 to A4 stored, A5 chosen, A6 to A8 empty"; `[data-slot="4"]` has `data-face="waiting"` and contains "here"; `[data-slot="0"]` is `data-face="off"` with its label `data-hue="t2"`; `[data-slot="7"]`'s label is `data-hue="d"`; every cell is `aria-hidden="true"` |
| `Loaded` | `Board` args with A1 `loaded: true` | the board as drawn: A1 in the chosen face | `Loaded-{dark,light}.png` (Prompts 293,243 390×44), no mask | named "Bank A: A1 loaded, A2 to A4 stored, A5 chosen, A6 to A8 empty"; `[data-slot="0"]` is `data-face="chosen"`, label `data-hue="g"` |
| `Missing` | `{ bank: 0, buttons: [A1 `{ rack: 'rack-gone', missing: true, loaded: false }`, A2 `{ rack: 'rack-warm', missing: false, loaded: true }`, A3–A8 empty], waiting: 4 }` | A1's label in `--warn`, A2 loaded | — (no board draws a missing slot) | named "Bank A: A1 missing, A2 loaded, A3 to A4 empty, A5 chosen, A6 to A8 empty"; A1's label `data-hue="warn"` |
| `WaitingOverStored` | `Board` args with `waiting: 3` | A4 holds Ballad and is being stored over: still the waiting face with "here" | — (not drawn) | `[data-slot="3"]` is `data-face="waiting"`; named "Bank A: A1 to A3 stored, A4 chosen, A5 to A8 empty" |
| `WaitingLoaded` | `{ bank: 0, buttons: [A1 loaded, A2–A8 empty], waiting: 0 }` | waiting outranks loaded | — | named "Bank A: A1 chosen, A2 to A8 empty"; `[data-slot="0"]` is `data-face="waiting"` |
| `LoadedMissing` | `{ bank: 0, buttons: [A1 `{ rack: 'rack-sunday', missing: true, loaded: true }`, A2–A8 empty], waiting: 4 }` | loaded outranks missing (the live rack whose file is gone) | — | `[data-slot="0"]` is `data-face="chosen"`; named "Bank A: A1 loaded, A2 to A4 empty, A5 chosen, A6 to A8 empty" |
| `AllEmpty` | `{ bank: 1, buttons: <eight empty>, waiting: null }` | bank B, nothing stored, no waiting cell | — | named "Bank B: B1 to B8 empty"; no `[data-face="waiting"]`; no "here" |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in dark and light; each crop is exactly the row's box (L6). The cells are 43.5px wide, so their edges fall on half pixels; the row's box is whole.

**Unit tests** (`app/src/ui/BankRow/bankRow.test.ts`, `npx vitest run src/ui/BankRow`): `bankRowLabel` on each story's args gives the name above; `slotLabel(0, 4)` is "A5", `slotLabel(7, 7)` "H8".

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`); `bankRow.test.ts` passes.
- `npm run shots -- BankRow` passes: `Board` (masked) and `Loaded` match their crops (score at most 0.02, or the Inspect agent judges the difference render noise), and axe finds no violation on any story.
- `--bank-row-height` is in the tokens contract PR before the build.
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · A picture, not a control.** The row is `role="img"` with one `aria-label` and `aria-hidden` cells (Prompts.md › Bank row): no focus, no tooltip, no callback; the Store prompt's buttons are the controls.
- **D2 · The waiting edge is an inset shadow.** Prompts.md draws the waiting cell with a 1px `--t` border and padding shrunk by 1; here the edge is `box-shadow: inset 0 0 0 1px`, so the padding and the label stay put in every face (the same pixels; as Button D3). "here" sits 7 and 5 from the cell's edge, the board's 6 and 4 inside its border.
- **D3 · The waiting cell is the row's own drawing.** A `--btn` fill with a `--t` edge, not the kit's transparent Waiting face (Prompts.md PR-D29, a named kit exception); `data-face="waiting"` still says what it means.
- **D4 · No contrast rows.** The cells' text is `aria-hidden` and the row's name is the reading, so axe skips it and `contrast.test.ts` gets no row for `--warn` on `--btn` (light 3.21:1) or `--d` on `--btn`. L2 proposal, should the owner want the missing label to pass anyway: light `--warn` `#9a4800` (4.95:1 on `--btn`, 5.63:1 on `--g`); not needed for this component.
- **D5 · Primitives title.** The lane convention files primitives under `Primitives/`; Prompts.md names no BankRow story.
- **D6 · Local types and helpers.** `app/src/ui` may not import `app/src/lib/api/types.ts` (lint), so the cell type is `BankCell` (the three fields the row reads) declared in this folder, and `slotLabel` repeats `quickLabel`'s one line; the wiring passes `quickRacks.buttons` as is (structurally compatible). `bankRowLabel` lives here too, as Prompts.md asks for a pure function.
- **D7 · L1, one new token.** `--bank-row-height` 44px (scale.css), so the row's height isn't a literal (axiom 2); the 7px and 5px offsets are sums of existing tokens.
- **D8 · Two crops of one box.** The fixture's A1 is stored (the rack was never saved, PR-D20), so `Board` masks cell A1; `Loaded` draws the board's A1 loaded and checks the whole row unmasked; both crop pairs (`Board-*`, `Loaded-*`) are cut from the same box.
- **D9 · No crop file exists yet.** `app/src/ui/BankRow/crops/` is cut later by the crop station at `Prompts 293,243 390×44` (measured on the render: cells from x 293, 43–44 wide with 6px gaps, y 243–286).

Follow-ups: whether the missing slot's light `--warn` label should pass AA although hidden (D4).

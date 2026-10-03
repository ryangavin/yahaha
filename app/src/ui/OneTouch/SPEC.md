# OneTouch

## Identity (all stations)

- **Kind:** complex
- **Built from:** Button
- **Build after:** the primitives lane's Button has landed; the `sm` size also needs Button's 32 × 28 size and light label weight (Rack.md Components row 3: `size: 'ots-sm'`, `weight: 300`), which Button's spec doesn't have yet (D5).
- **Purpose:** Applies one of the style's four One Touch Settings at once, and shows which one was applied last.
- **Boards:**
  - `Stage-Dark.dc.html:136-142` (the style line: "One Touch", then 1–4 with 2 chosen; `md`, with the label); light: `Stage-Light.dc.html:112-118`.
  - The app bar, page variant (the same group, `md` with the label): `Channel-Dark.dc.html:80-106` (inside the bar's lines) / `Channel-Light.dc.html:44-70`; `Rack-Dark.dc.html:64-70` / `Rack-Light.dc.html:46-52`.
  - `Rack-Dark.dc.html:281-287` (the Rack page's One Touch group: 32 × 28 buttons, light digits, no label; `sm`); light: `Rack-Light.dc.html:263-269`.
- **Not this component's job:** no store, no API, no Tauri. It doesn't send `recallOts` or ask about unsaved rack changes: it calls `onrecall(index)` and the wiring sends (Stage.md D17; contract change C2 for "never asks"). It doesn't know the style or its racks: the wiring passes the count, the applied number and the rack names (computed with `otsRackNames`, below). It doesn't place itself (the style line's `margin-left: auto`, the app bar's margins are the parents'). Not One Touch Link, its timing or the OTS → rack line (Rack.md's `OneTouchGroup`). The buttons' faces are Button's.

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `count` | `number` | `4` | How many One Touch Settings the style has (`ots.settings.length`, 0–4). Buttons past it are disabled. |
| `applied` | `number` | `0` | The last one recalled, 1-based (`ots.applied`); 0 = none. That button wears the chosen face. |
| `racks` | `(string \| null)[]` | `[]` | Per button, the name of the user's rack it loads instead of the style's own (`otsRackNames(ots.racks)`); `null` or missing entry: the style's own. Only the `aria-label`s change (", loads rack Organ"). |
| `size` | `'md' \| 'sm'` | `'md'` | `md`: 32 × 32 buttons, 14px regular digits (the style line and the app bar). `sm`: 32 × 28 buttons, 14px light (300) digits (the Rack page, Rack.md row 22). |
| `label` | `boolean` | `true` | The leading "One Touch" word. The Rack page's group passes `false` (its header names it). |
| `tips` | `string[]` | `['ots.1', 'ots.2', 'ots.3', 'ots.4']` | The tooltip key of each button, rendered as its `data-tip` (L3). |
| `tipAction` | `Action<HTMLElement, string> \| undefined` | — | The app's `use:tip`, passed to each Button with its key (L3). |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onrecall` | button `n` is pressed (click, Space or Enter), unless it is disabled; also on the chosen one (the wiring re-applies it, as the hardware does) | `(index: number)` 0-based (`n − 1`), for `recallOts { index }` |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Pure functions (`app/src/ui/OneTouch/ots.ts`)

- `otsRackNames(racks: OtsRack[]): (string | null)[]`: per entry, `rack === null` → `null`; `missing` → "{rack} (missing)" (the id, since the engine sends an empty `name`; Rack.md › One Touch); else `name`.
- `otsLabel(n: number, applied: number, rack: string | null | undefined): string`: "One Touch {n}{, applied}{, loads rack {rack}}" ("One Touch 2, applied"; "One Touch 4, loads rack Organ").
- `otsGroupLabel(applied: number): string`: "One Touch: {applied} applied", or "One Touch: none applied" when 0.

### Children

In DOM order: the label (when `label`), then four Buttons, `n` = 1…4:

| Child | Props passed | Callback → |
|---|---|---|
| `Button` `n`, `md` | `{ label: String(n), size: 'icon', chosen: n === applied, pressed: n === applied, disabled: n > count, name: otsLabel(n, applied, racks[n − 1]), tip: tips[n − 1], tipAction }` | `onpress()` → `onrecall(n − 1)` |
| `Button` `n`, `sm` | the same, with `size: 'ots-sm'` and `weight: 300` in place of `size: 'icon'` (D5) | the same |

The chosen face is Button's (`--t` fill, `--g` digit, medium weight, also in `sm`: the chosen digit is 500, Rack.md row 22); a disabled button is Button's disabled face (digit `--d`, `aria-disabled="true"`, focusable, no press).

### Visual rules

- **Tokens used:** `--m`, `--font-sans`, `--text-14`, `--weight-regular`, `--space-4`. No new tokens (L1); Button's sizes are Button's.
- **Root:** a `div`, `role="group"`, `display: inline-flex; align-items: center; gap: var(--space-4)`, `flex: none`, `white-space: nowrap`. Height: the buttons' (32 in `md`, 28 in `sm`).
- **Label** (when `label`): a span "One Touch", 14 / 400 (`--text-14`, `--weight-regular`), `--m`, line-height normal, `margin-right: var(--space-4)` (so 8px to button 1 with the gap), `aria-hidden="true"` (the group's name says it). Not a control.
- **Buttons:** four, gap 4. At the board's font the `md` group with its label is 219 × 32 (label 71, 8, four 32s with gaps 4); `sm` without the label is 140 × 28.
- **States drawn by:** which button is chosen (`applied`) and which are disabled (`count`), through the Buttons; nothing of its own. No hover look.
- **Type:** the label's above; the digits are Button's (14 regular, chosen 500; `sm` 300, chosen 500).
- **Contrast:** the label is `--m` on `--g` (existing row). The buttons' pairs are Button's (`--t2` on `--btn`, `--g` on `--t`).
- **Motion:** none.

### Accessibility

- **Role and name:** the root is `role="group"` with `aria-label` = `otsGroupLabel(applied)` ("One Touch: 2 applied"). Each button: `aria-label` `otsLabel(…)`, `aria-pressed` `"true"` on the applied one and `"false"` on the others (Button's `pressed`), `aria-disabled="true"` only past `count` (L5).
- **Keyboard:** Tab reaches each button in order, the disabled ones too; Space or Enter recalls (nothing when disabled). No arrow keys.
- **Tooltip id:** `ots.1` … `ots.4` (exist; C5 rewrites their bodies to "applies at once, Launchkey Racks page"), via `tips` and `tipAction`.
- **Launchkey:** the Racks pad page's bottom row, pads 1–4 (Stage.md D16); nothing here.

## Stories (Story station)

Title `Components/OneTouch`, `layout: 'centered'`. Every story renders in dark and light. The meta's `args` are `{ onrecall: fn(), tipAction: fn() }` plus each story's.

**Controls (argTypes):** `count` a number (0–4) and `applied` a number (0–4); `racks` an `object`; `size` a select of `md` / `sm`; `label` boolean; `tips` an `object`; `onrecall`, `tipAction` actions.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ count: 4, applied: 2 }` | the Stage's group at the board fixture (`ots.applied` 2): "One Touch", 1, the white 2, 3, 4 | `Board-{dark,light}.png` (Stage 368,129 219×32) | the group named "One Touch: 2 applied" holds 4 buttons; "One Touch 2, applied" has `aria-pressed="true"` and `data-face="chosen"`; buttons 1, 3, 4 have `aria-pressed="false"`, `data-face="off"` and no `aria-disabled`; their `data-tip`s are `ots.1` … `ots.4`; `tipAction` was called 4 times; click "One Touch 3" → `onrecall` called with `2` (Stage.md Check 3) |
| `TwoOts` | `{ count: 2, applied: 1 }` | a style with two OTS: 3 and 4 dimmed | — (no board draws it) | buttons 3 and 4 have `aria-disabled="true"`; click 4, then focus it and press Enter → `onrecall` not called; click 1 → called with `0` |
| `NoneApplied` | `{ count: 4, applied: 0 }` | nothing chosen since the style loaded | — | the group is named "One Touch: none applied"; no button has `aria-pressed="true"` |
| `NoOts` | `{ count: 0, applied: 0 }` | a style without One Touch: all four dimmed | — | 4 buttons with `aria-disabled="true"` |
| `Racks` | `{ count: 4, applied: 2, racks: [null, 'Jazz', null, 'Organ'] }` | the same look; the labels name the racks | — | button 4 is named "One Touch 4, loads rack Organ"; button 2 "One Touch 2, applied, loads rack Jazz" |
| `Small` | `{ size: 'sm', label: false, count: 4, applied: 2, racks: [null, null, null, 'Organ'] }` | the Rack page's group: 32 × 28 buttons, light digits, no label | `Small-{dark,light}.png` (Rack 1091,422 140×28) | no "One Touch" text in the canvas; the group is still named "One Touch: 2 applied"; button 4 is named "One Touch 4, loads rack Organ" (Rack.md Check 13) |
| `Focused` | `Board`'s args, `parameters: { pseudo: { focusVisible: true } }` | the focus rings | — | — |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in dark and light; each crop is the group's box (L6). The boxes are measured on the renders (the `Board` box starts at the "One Touch" text's left edge), so the crop station confirms them when it cuts (D7).

**Unit tests** (`app/src/ui/OneTouch/ots.test.ts`, vitest): `otsRackNames([{ rack: null, name: '', missing: false }, { rack: 'r-org', name: 'Organ', missing: false }, { rack: 'r-old', name: '', missing: true }])` → `[null, 'Organ', 'r-old (missing)']`; `otsLabel(2, 2, null)` → "One Touch 2, applied"; `otsLabel(4, 2, 'Organ')` → "One Touch 4, loads rack Organ"; `otsGroupLabel(0)` → "One Touch: none applied", `otsGroupLabel(3)` → "One Touch: 3 applied".

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`); `ots.test.ts` passes.
- `npm run shots -- OneTouch` passes for `Board` and `Small` (score at most 0.02, or the Inspect agent judges any difference render noise), and axe finds no violation on any story.
- Only listed tokens are used; no inline colours, no literal sizes.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · One component for every copy (the fold).** The Stage's style line, the app bar's page variant (Channel.md › Kit additions, canonical; Effects, Harmony, Browser and Rack copy it "unchanged") and the Rack page's group are the same control (Rack.md RK-D20); Rack.md row 22's `size: 'sm'` and `label: boolean` are props here, so there is one OneTouch.
- **D2 · Applies at once.** A press calls `onrecall(index)` with no dialog of its own (Stage.md D17); a rack prompt can still come from the session until contract change C2.
- **D3 · Rack names in every copy.** Rack.md says both of its copies read ", loads rack {name}"; the Stage's labels didn't mention racks. The labels add it wherever `racks` is passed, so every copy says the same; at the Stage fixture (`ots.racks` empty) the Stage's labels are unchanged ("One Touch 2, applied").
- **D4 · The chosen one still recalls.** Pressing the applied button calls `onrecall` again (re-applying the OTS, as the panel button does); nothing is disabled except past `count`.
- **D5 · `sm` needs Button sizes that aren't specified yet.** The 32 × 28 button with a 300-weight digit is Rack.md row 3's `size: 'ots-sm'` and `weight: 300`; Button's spec has neither. This spec passes those names; if Button's lane names them differently, its names win. Until they exist, `Small` can't be built (reported to the lead as a change outside this folder).
- **D6 · The label is hidden from screen readers.** "One Touch" is drawn text; the group's `aria-label` ("One Touch: 2 applied") already says it, so the span is `aria-hidden` and isn't read twice.
- **D7 · Crops.** The folder is new and has no crops yet: `Board-{dark,light}.png` (Stage 368,129 219×32) and `Small-{dark,light}.png` (Rack 1091,422 140×28) are measured on the renders and cut later by the crop station. The app bar's copy is pixel-identical to `Board` (Channel 278,26 219×32), so it has no story of its own; AppBar's page-variant crop covers it.
- **D8 · L3, tooltips per item.** `tips` holds one key per button (default `ots.1`–`ots.4`, all in `tooltips.ts`), each passed to its Button with the shared `tipAction`; stories pass `tipAction: fn()`. L5: `aria-disabled` only on the buttons past `count`.

Follow-ups: Button gains the `ots-sm` size and `weight: 300` (D5).

# PadBank

## Identity (all stations)

- **Kind:** complex
- **Built from:** GroupHeader, Button, PadGrid
- **Purpose:** The band's Pads section: which pad page the Launchkey is on, the Pad Bank ▲ ▼ that page through them, and the 16 pads.
- **Boards:** `Stage-Dark.dc.html:319-357` (the Pads section: header, ▲ ▼ column, grid); light: `Stage-Light.dc.html:295-333`. Box `690,590 626×186` on both boards (18px below the Knobs section).
- **Not this component's job:** no store, no API, no Tauri, no timer. It doesn't send commands, read Shift or know about Sound latch: it reports pad presses and ▲ ▼ presses, and the wiring decides what to send (including Stage.md D18's `setLayer none`). It doesn't draw pads or group lines (PadGrid and Pad).

## API (Component station)

### Props

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `pageName` | `string` | — | The page on the pads, shown in the header: `Sections`, `Racks`, `Chord`, `Multi Pads`, `Setup` (`Racks` while Sound is held or latched). |
| `pageNumber` | `number` | — | The page's 1-based place in the page order (the `n` of "Bank n/m"). |
| `pageCount` | `number` | — | How many pages the order has (the `m`). |
| `pads` | `PadState[]` (16) | — | Passed to PadGrid (PadGrid's SPEC defines `PadState`). |
| `fallback` | `boolean` | `false` | Passed to PadGrid; also hides the header's legend (the legend explains the Sections colours only). |
| `led` | `number` | `0` | Passed to PadGrid (the LED clock in beats). |
| `running` | `boolean` | `false` | Passed to PadGrid (Start / Stop idle when false). |
| `upDisabled` | `boolean` | `false` | Pad Bank ▲ does nothing now (computed by the wiring, From the state). |
| `downDisabled` | `boolean` | `false` | Pad Bank ▼ does nothing now. |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onpad` | PadGrid's `onpad` fires (a pad pressed: pointer down, Enter or Space on an enabled pad) | `(index: number)` the 0-based pad index into `pads.pads` |
| `onbankup` | Pad Bank ▲ is pressed (Button's `onpress`: click, Enter or Space), unless `upDisabled` | `()` |
| `onbankdown` | Pad Bank ▼ is pressed, unless `downDisabled` | `()` |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Children

| Child | Where | Props passed |
|---|---|---|
| `GroupHeader` | the header row, top, full width (626 × 36, its hairline at the bottom) | `{ title: 'Pads' }`; its content snippet holds, in order: the page name, the legend (when `fallback` is false), and the Bank pager (Visual rules) |
| `Button` (▲) | the ▲ ▼ column, top | `{ label: '▲', variant: 'icon', name: 'Pad bank up', disabled: upDisabled, onpress: onbankup }` |
| `Button` (▼) | the ▲ ▼ column, bottom | `{ label: '▼', variant: 'icon', name: 'Pad bank down', disabled: downDisabled, onpress: onbankdown }` |
| `PadGrid` | right of the column | `{ pads, fallback, led, running, onpad }` |

### Visual rules

- **Tokens used:** `--m`, `--t`, `--intro`, `--main`, `--ending`, `--brk`, `--fill`, `--font-sans`, `--text-12`, `--text-13`, `--text-14`, `--weight-regular`, `--space-4`, `--space-6`, `--space-8`, `--space-12`. The header's hairline, the buttons and the pads are their children's.
- **Size:** fixed 626 × 186 (the Stage's 1440 × 900 layout; Stage.md D1 scales the page). A flex column:
  1. **Header** (GroupHeader), 626 × 36 (y 0–35 in the box; board y 590–625, hairline on row 625).
  2. **Pad row**, 5px below the header (y 41), 626 × 145, `display: flex`, `gap: --space-8` (8px), `align-items: flex-start`:
     - the **▲ ▼ column**: 32 wide, 142 tall, 3px below the row's top (y 44, the pads' top), `box-sizing: border-box`, padding 18px top and bottom, flex column, `justify-content: space-between`: ▲ (32 × 32) at y 62 and ▼ at y 136 in the box (board ▲ `690,652`, ▼ `690,726`);
     - **PadGrid** (586 × 145) at x 40 (board `730,631`): its own 3px group-line band on top, so its pads start at y 44, level with the column.
- **Header content** (in GroupHeader's row: one line, `white-space: nowrap`, items centred vertically, 12px apart):
  - **Page name:** `pageName`, `--text-14` (14px), `--weight-regular`, `--t`.
  - **Legend** (only when `fallback` is false; `aria-hidden="true"`): 4px extra before it (`margin-left: --space-4`, so 16px after the page name), five items 12px apart (`--space-12`), each a 10 × 2 bar (radius 1px) then, 6px later (`--space-6`), the word, `--text-12` (12px) `--weight-regular`; bar and word both in the item's hue: `Intro` `--intro`, `Main` `--main`, `Ending` `--ending`, `Break` `--brk`, `Fill` `--fill`. Each item carries `data-hue` (`intro`, `main`, `ending`, `brk`, `fill`).
  - **Bank pager:** pushed to the right edge (`margin-left: auto`): `Bank` `--text-13` (13px) `--weight-regular` `--m`, a space, then `{pageNumber}/{pageCount}` in `--t` (same size), e.g. `Bank 1/5`. Integers, no padding.
- **States drawn by:** Sections (legend shown) or fallback (no legend); ▲ ▼ enabled or disabled (Button's disabled face). Everything else is the children's.
- **Contrast (AA 4.5:1, `tokens/contrast.test.ts`):** on `--g`: `--t` (page name, page numbers), `--m` (`Bank`; `Pads` is GroupHeader's), and the legend words `--intro`, `--main`, `--ending`, `--brk`, `--fill`. With today's values light `--intro` (3.87) and `--main` (4.42), and dark `--brk` (4.48), fall short on `--g`; that is the tokens' to fix.
- **Motion:** none of its own; `led` reaches the pads.

### Accessibility

- **Role and name:** a `<section aria-label="Pads">` (a region). The ▲ ▼ are buttons named `Pad bank up` and `Pad bank down`; the pads are named by Pad. The legend is `aria-hidden` (a colour key; each pad's name already says what it is). The header text reads as plain text: `Pads`, the page name, `Bank 1/5`.
- **Keyboard:** Tab order (kit D36): ▲, ▼, then pads 1–16. Enter or Space presses the focused one.
- **Disabled:** a disabled ▲ or ▼ is `aria-disabled="true"`, stays focusable and calls nothing (Button).

| Control | Callback | The wiring sends | Disabled when | Tooltip | Launchkey | `aria-label` |
|---|---|---|---|---|---|---|
| ▲ | `onbankup()` | `surface.controls[padBankUp].action`; with `ui.shift`, its `shiftAction` (Left on/off) | that action (with Shift: `shiftAction`) is null | `padpage.prev` | Pad Bank ▲ (left of the pads) | `Pad bank up` |
| ▼ | `onbankdown()` | `surface.controls[padBankDown].action`; with `ui.shift`, its `shiftAction` (OTS Link on/off) | that action (with Shift: `shiftAction`) is null | `padpage.next` | Pad Bank ▼ (left of the pads) | `Pad bank down` |
| pad `i` | `onpad(i)` | `pads.pads[i].action` (then `setLayer none` while Sound is latched, below) | `pads.pads[i].action` is null | Pad's SPEC | the pad | Pad's `padLabel`, e.g. `Main C, queued (pad 11)` |

`padBankUp` and `padBankDown` are the `surface.controls` entries with those `id`s (the first two of the 17). Both tooltip keys exist in `tooltips.ts` and already describe the Shift layer.

### From the state

The page wiring (`app/src/pages/StageWiring.svelte`, Stage.md D46) builds the props and handles the callbacks. `upCtl` = `surface.controls.find(c => c.id === 'padBankUp')`, `downCtl` likewise; `shift` = `ui.shift` (the computer keyboard's Shift included).

| Prop / callback | From `AppState` / what the wiring sends |
|---|---|
| `pageName` | `pads.pageName` |
| `pageNumber`, `pageCount` | `pads.pageNumber`, `pads.pageCount` |
| `pads[i]` | `{ label: pads.pads[i].label, level: pads.pads[i].level, anim: pads.pads[i].anim, disabled: pads.pads[i].action === null }` |
| `fallback` | `pads.page !== 'sections' \|\| surface.layer.type === 'sound'` |
| `led` | `ledBeats(surface.clock, now, receivedMs)` (`app/src/ui/Pad/led.ts`), once per animation frame |
| `running` | `transport.running` |
| `upDisabled` | `(shift ? upCtl.shiftAction : upCtl.action) === null` |
| `downDisabled` | `(shift ? downCtl.shiftAction : downCtl.action) === null` |
| `onbankup()` | `send(shift ? upCtl.shiftAction : upCtl.action)` |
| `onbankdown()` | `send(shift ? downCtl.shiftAction : downCtl.action)` |
| `onpad(i)` | `send(pads.pads[i].action)`; then, if the screen's Sound latch is on (the LampRow's Sound lamp was latched by a click, Stage.md D18), `send({ type: 'setLayer', layer: { type: 'none' } })` at once, without waiting for a reply, so a refused pad action still releases the latch. The action is the one on view, i.e. the Racks page's while Sound is latched (`pads.pads` already is the Racks page then). `onpad` itself carries only the index; PadBank knows nothing of the latch. Hardware pad taps release the latch only once contract change C3 lands. |

### Fixtures (`app/src/ui/PadBank/PadBank.fixtures.ts`)

- **`boardPadBank`** (`satisfies Omit<PadBankProps, 'onpad' | 'onbankup' | 'onbankdown'>`; stories and FullBand add the callbacks as actions), from Stage.md › Board fixture: `{ ...boardPadGrid, pageName: 'Sections', pageNumber: 1, pageCount: 5, upDisabled: true, downDisabled: false }`. So: `fallback: false`, `led: 0.25`, `running: true`, and the 16 pads of `boardPadGrid` (PadGrid's SPEC lists them: pads 3 and 7 `off`; 10 `bright` `solid`; 11 `bright` `flash`; 16 `bright` `solid`; the rest `dim` `solid`; no pad disabled). ▲ is disabled because the fixture's `padBankUp` `action` and `shiftAction` are null; ▼ is enabled.
- **`chordPadBank`**: `{ ...chordPadGrid, pageName: 'Chord', pageNumber: 3, pageCount: 5, upDisabled: false, downDisabled: false }` (Chord is page 3 in the default order).

## Stories (Story station)

- **Title:** `Components/PadBank`
- **Layout:** `centered` (real size, 626 × 186).

Every story renders in dark and light. `onpad`, `onbankup`, `onbankdown` are actions (`fn()`). Controls: PadBank's own props, plus PadGrid's (`pads`, `fallback`, `led`, `running`) under the category `PadGrid` and the ▲ ▼ props (`upDisabled`, `downDisabled`) under `Button`.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `boardPadBank` | the board's Pads section: header with the Sections legend and `Bank 1/5`, ▲ ▼, the Sections pads | `Board-{dark,light}.png`: Stage 690,590 626×186 | a region named `Pads`; its text includes `Pads`, `Sections`, `Bank 1/5`; five legend items with `data-hue` `intro`, `main`, `ending`, `brk`, `fill`; the button named `Pad bank up` has `aria-disabled="true"`; `Pad bank down` doesn't; 18 buttons in all |
| `BankButtons` | `boardPadBank` | — | — | click `Pad bank up` → `onbankup` not called; click `Pad bank down` → `onbankdown` called once; focus `Pad bank down` and press Enter → called twice |
| `PadPress` | `boardPadBank` | — | — | click `Main C, queued (pad 11)` → `onpad` called with `10`; `onbankup` and `onbankdown` not called |
| `ChordPage` | `chordPadBank` | a page without its own spec, in the fallback face (Stage.md Check 8): `Chord`, no legend, `Bank 3/5`, the state's labels on the pads, no group lines | — (`PadsChord-Dark.dc.html` draws the real Chord page, with its own legend, group lines and sentence-case captions, not the fallback face) | no legend items; text includes `Chord` and `Bank 3/5`; every pad has `data-hue="t"`; pad 10 is named `STOP ACMP (pad 10)`; pad 1 `Unused (pad 1)` with `aria-disabled="true"` |
| `SoundHeld` | `{ ...boardPadBank, pageName: 'Racks', fallback: true, pads: <Racks> }`, where `<Racks>` is pads 1–8 `QUICK 1`…`QUICK 8` (`QUICK 1` `bright` `solid`, the rest `dim`), pads 9–12 `OTS 1`…`OTS 4` `dim`, 13 `BANK -` `off`, 14 `BANK +` `dim`, 15 `STORE` `dim`, 16 `''` `off` disabled; every other pad enabled, all `solid` | Sound held or latched on Sections: the Racks pads in the fallback face, `Bank 1/5` (the page on view stays Sections) | — (no board draws it) | text includes `Racks` and `Bank 1/5`; no legend items; pad 1 is named `QUICK 1, playing (pad 1)` with `data-face="playing"`; pad 13 `BANK -, not available (pad 13)` |
| `LastPage` | `{ ...chordPadBank, pageNumber: 3, pageCount: 3, downDisabled: true }` | the last page of a three-page order: ▼ disabled | — | `Pad bank down` has `aria-disabled="true"`; click it → `onbankdown` not called; click `Pad bank up` → `onbankup` called once; text includes `Bank 3/3` |
| `Focused` | `boardPadBank`; `parameters: { pseudo: { focusVisible: ['button[aria-label="Pad bank down"]'] } }` | the focus ring on ▼ | — | — |

What jsdom can't check (the legend's colours, the layout's pixel positions, the pads' looks) is covered by the `Board` crop (`npm run shots -- PadBank`).

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- PadBank` passes: `Board` is 626 × 186 and scores at most 0.02 against its crop (the disabled ▲'s glyph is `--d` where the board draws `--t2`: about 40 pixels, D3); axe finds no violation.
- Only listed tokens are used; no inline colours; the sizes that aren't tokens are only those in Visual rules (626 × 186, the 5px gap under the header, the column's 32 × 142 with its 3px offset and 18px padding, the legend's 10 × 2 bars).
- svelte-check and lint pass on the folder.

## Decisions

- D1. PadBank takes `upDisabled` / `downDisabled` computed by the wiring (with `ui.shift`) and reports plain `onbankup()` / `onbankdown()`; the ▲ ▼ keep their glyphs and names under Shift, since the tooltips already say what Shift does and the board draws no Shift label there.
- D2. Stage.md D18's latch release is the wiring's: `onpad(i)` carries only the index, and the wiring sends the pad's `action` and then `setLayer none`; PadBank and PadGrid know nothing of Sound.
- D3. The `Board` story disables ▲ (the fixture's `padBankUp` action is null), so its glyph is `--d`; the board draws it in `--t2`, a difference far under the 0.02 score, so the crop needs no mask.
- D4. The legend is `aria-hidden`: it is a colour key for the eye, and each pad's accessible name already says what the pad is.
- D5. The legend shows only when `fallback` is false (Sections); a page drawn with the fallback face has no group lines for a legend to explain (Stage.md D33).
- D6. The PadGrid sits 5px under the header with its own 3px line band, so the pads land on the board's y 634 and the ▲ ▼ column keeps the board's pixels (▲ at 652, ▼ at 726).
- D7. While Sound is held or latched the header reads `Racks` with the page on view's `Bank n/m` (`pads.pageName` and `pads.pageNumber` as the state gives them), as the hardware's pages don't move.

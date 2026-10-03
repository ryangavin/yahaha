# PadBank

## Identity (all stations)

- **Kind:** complex
- **Built from:** GroupHeader, Button, PadGrid
- **Purpose:** The band's Pads section: which pad page the Launchkey is on, the Pad Bank ▲ ▼ that page through them, and the 16 pads.
- **Boards:** `Stage-Dark.dc.html:319-357` (the Pads section: header, ▲ ▼ column, grid); light: `Stage-Light.dc.html:295-333`. Box `690,590 626×186` on both boards (18px below the Knobs section).
- **Not this component's job:** no store, no API, no Tauri, no timer. It doesn't send commands, read Shift or know about the Sound latch: it reports pad presses and ▲ ▼ presses, and the wiring decides what to send (including Stage.md D18's `setLayer none`). It doesn't draw pads or group lines (PadGrid and Pad), the header row, title or "Bank n/m" counter (GroupHeader), or the ▲ ▼ faces (Button). No tooltips (wired at integration).

## API (Component station)

### Props

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `pageName` | `string` | — | The page on the pads, shown in the header: `Sections`, `Racks`, `Chord`, `Multi Pads`, `Setup` (`Racks` while Sound is held or latched). |
| `pageNumber` | `number` | — | The page's 1-based place in the page order (the `n` of "Bank n/m"). |
| `pageCount` | `number` | — | How many pages the order has (the `m`). |
| `pads` | `PadData[]` (16) | — | Passed to PadGrid (PadGrid's SPEC defines `PadData`). |
| `fallback` | `boolean` | `false` | Passed to PadGrid; also hides the header's legend (the legend explains the Sections colours only). |
| `led` | `number` | `0` | Passed to PadGrid (the LED clock in beats). |
| `running` | `boolean` | `false` | Passed to PadGrid (Start / Stop idle when false). |
| `upDisabled` | `boolean` | `false` | Pad Bank ▲ does nothing now (computed by the wiring, From the state). |
| `downDisabled` | `boolean` | `false` | Pad Bank ▼ does nothing now. |

Exported from `PadBank.svelte`'s module script:

```ts
import type { PadData } from '../PadGrid/PadGrid.svelte'

export interface PadBankProps {
  pageName: string
  pageNumber: number
  pageCount: number
  pads: PadData[] // exactly 16
  fallback?: boolean // default false
  led?: number // default 0
  running?: boolean // default false
  upDisabled?: boolean // default false
  downDisabled?: boolean // default false
  onpad: (index: number) => void
  onbankup: () => void
  onbankdown: () => void
}
```

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onpad` | PadGrid's `onpad` fires (a pad pressed: pointer down, Enter or Space on an enabled pad) | `(index: number)` the 0-based pad index into `pads.pads` |
| `onbankup` | Pad Bank ▲'s Button `onpress` (click, Enter or Space), unless `upDisabled` (Button then calls nothing) | `()` |
| `onbankdown` | Pad Bank ▼'s Button `onpress`, unless `downDisabled` | `()` |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Children

| Child | Where | Props passed |
|---|---|---|
| `GroupHeader` | the header row, top, full width | `{ title: 'Pads', count: { label: 'Bank', value: \`${pageNumber}/${pageCount}\` } }`, `level` and `width` left at their defaults (an `h2`; fills PadBank's 626px root, D8); its `children` snippet holds, in order, the page name `span` and (when `fallback` is false) the legend `span` (Visual rules) |
| `Button` (▲) | the ▲ ▼ column, top | `{ symbol: 'up', size: 'icon', name: 'Pad bank up', disabled: upDisabled, onpress: onbankup }` (no `label`) |
| `Button` (▼) | the ▲ ▼ column, bottom | `{ symbol: 'down', size: 'icon', name: 'Pad bank down', disabled: downDisabled, onpress: onbankdown }` (no `label`) |
| `PadGrid` | right of the column | `{ pads, fallback, led, running, onpad }` |

### Visual rules

- **Tokens used:** `--t`, `--intro`, `--main`, `--ending`, `--brk`, `--fill`, `--font-sans`, `--text-12`, `--text-14`, `--weight-regular`, `--control-height` (the column's width), `--space-2` (the legend bar's height), `--space-4`, `--space-6`, `--space-8`, `--space-12`. The header's hairline, title and counter, the buttons and the pads are their children's. No new tokens.
- **Component geometry** (axiom 2), declared once as custom properties on the root `section`; rules use these names, never the numbers:

  | Property | Value | What |
  |---|---|---|
  | `--pad-bank-width` | `626px` | the box's width |
  | `--pad-bank-height` | `186px` | the box's height |
  | `--pad-bank-header-gap` | `5px` | from the header's bottom to the pad row |
  | `--pad-bank-column-top` | `3px` | the ▲ ▼ column's offset below the pad row's top (PadGrid's line band) |
  | `--pad-bank-column-height` | `142px` | the ▲ ▼ column's height (the two pad rows) |
  | `--pad-bank-column-pad` | `18px` | the column's top and bottom padding |
  | `--legend-bar-width` | `10px` | a legend bar's width |

- **Size:** `width: var(--pad-bank-width)`, `height: var(--pad-bank-height)`, fixed (the Stage's 1440 × 900 layout; Stage.md D1 scales the page), `box-sizing: border-box`, `margin: 0`, `display: flex`, `flex-direction: column`:
  1. **Header** (GroupHeader), 36 tall, the root's width (y 0–35 in the box; board y 590–625, hairline on row 625).
  2. **Pad row** (`data-part="row"`), `margin-top: var(--pad-bank-header-gap)` (y 41), `display: flex`, `gap: var(--space-8)`, `align-items: flex-start`:
     - the **▲ ▼ column** (`data-part="bank"`): `width: var(--control-height)` (32), `height: var(--pad-bank-column-height)`, `margin-top: var(--pad-bank-column-top)` (y 44, the pads' top), `box-sizing: border-box`, `padding: var(--pad-bank-column-pad) 0`, `display: flex`, `flex-direction: column`, `justify-content: space-between`: ▲ (32 × 32) at y 62 and ▼ at y 136 in the box (board ▲ `690,652`, ▼ `690,726`);
     - **PadGrid** (586 × 145) at x 40 (board `730,631`): its own 3px group-line band on top, so its pads start at y 44, level with the column.
- **Header content** (GroupHeader's `children`; each top-level element is a flex item 12px after the one before, GroupHeader's gap):
  - **Page name** (`data-part="page"`, a `span`): `pageName`, `--text-14`, `--weight-regular`, `--t`.
  - **Legend** (`data-part="legend"`, a `span`, only when `fallback` is false; `aria-hidden="true"`): `margin-left: var(--space-4)` (so 16px after the page name), `display: flex`, `align-items: center`, `gap: var(--space-12)`, `--text-12`, `--weight-regular`. Five items, each a `span` with `display: flex`, `align-items: center`, `gap: var(--space-6)`, `color: var(--<hue>)` and `data-hue="<hue>"`, holding a bar `span` (`width: var(--legend-bar-width)`, `height: var(--space-2)`, `border-radius: calc(var(--space-2) / 2)`, `background: var(--<hue>)`) then the word: `Intro` (`intro`), `Main` (`main`), `Ending` (`ending`), `Break` (`brk`), `Fill` (`fill`). This is the markup GroupHeader's `Pads` story copies.
  - **Bank counter:** GroupHeader's `count` (`Bank` in `--m`, then `1/5` in `--t`, 13px, at the right end). Integers, no padding.
- **States drawn by:** Sections (legend shown) or fallback (no legend); ▲ ▼ enabled or disabled (Button's disabled look, `--d` glyph). Everything else is the children's.
- **Contrast (AA 4.5:1).** Ratios from today's tokens (dark / light).
  - In `contrast.test.ts` already: `--t` on `--g` (the page name; 21.00 / 16.72). The title and counter pairs are GroupHeader's.
  - Pass in both themes, add to `contrast.test.ts` with the tokens PR: `--ending` on `--g` (4.96 / 5.20) and `--fill` on `--g` (7.27 / 4.51), "legend word (PadBank)".
  - **Known failures (owner question O-contrast)**; until the owner answers, every PadBank story's axe check excludes exactly these selectors, plus Pad's `PAD_CONTRAST_EXCLUDE` for the pads:

    | Selector | Pair | Dark | Light |
    |---|---|---|---|
    | `[data-part="legend"] > [data-hue="intro"]` | `--intro` on `--g` | 9.69 | **3.87** |
    | `[data-part="legend"] > [data-hue="main"]` | `--main` on `--g` | 11.16 | **4.42** |
    | `[data-part="legend"] > [data-hue="brk"]` | `--brk` on `--g` | **4.48** | 5.73 |

- **Motion:** none of its own; `led` reaches the pads.

### Accessibility

- **Role and name:** a `<section aria-label="Pads">` (a region). Inside: `Pads` is a level-2 heading (GroupHeader's `h2`); the page name and `Bank 1/5` are plain text. The ▲ ▼ are buttons named `Pad bank up` and `Pad bank down` (their glyphs `aria-hidden`, Button); the pads are named by Pad. The legend is `aria-hidden` (a colour key; each pad's name already says what it is).
- **Keyboard:** Tab order (kit D36): ▲, ▼, then pads 1–16. Enter or Space presses the focused one.
- **Disabled:** a disabled ▲ or ▼ is `aria-disabled="true"`, stays focusable and calls nothing (Button).

| Control | Callback | The wiring sends | Disabled when | Tooltip (wired at integration) | Launchkey | `aria-label` |
|---|---|---|---|---|---|---|
| ▲ | `onbankup()` | `upCtl.action`; with `shift`, `upCtl.shiftAction` (Left on/off) | that action (with `shift`: `shiftAction`) is null | `padpage.prev` | Pad Bank ▲ (left of the pads) | `Pad bank up` |
| ▼ | `onbankdown()` | `downCtl.action`; with `shift`, `downCtl.shiftAction` (OTS Link on/off) | that action (with `shift`: `shiftAction`) is null | `padpage.next` | Pad Bank ▼ (left of the pads) | `Pad bank down` |
| pad `i` | `onpad(i)` | `pads.pads[i].action` (then `setLayer none` while `soundLatched`, below) | `pads.pads[i].action` is null | PadGrid's `padTip` | the pad | Pad's `padLabel`, e.g. `Main C, queued (pad 11)` |

`upCtl` and `downCtl` are the `surface.controls` entries with `id` `padBankUp` and `padBankDown` (the first two of the 17). Both tooltip keys exist in `tooltips.ts` and already describe the Shift layer.

### From the state

The page wiring (`app/src/pages/StageWiring.svelte`, Stage.md D46) builds the props and handles the callbacks. `upCtl` = `surface.controls.find(c => c.id === 'padBankUp')`, `downCtl` likewise. `shift` is `ui.shift`: app-only state in `app/src/lib/store.svelte.ts` (the `UiStore` getter `shiftLatched || shiftHeld`: Shift latched on screen, or the computer keyboard's Shift held), not a field of `AppState`. `soundLatched` is the wiring's own flag (the screen's Sound latch, Stage.md D18), not `AppState` either: LampRow's `onhold(hold, latch)` sets it to `latch` for a Sound hold and clears it on `{ type: 'none' }` (LampRow's From the state).

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
| `onpad(i)` | `send(pads.pads[i].action)`; then, if `soundLatched` is true, `send({ type: 'setLayer', layer: { type: 'none' } })` at once, without waiting for a reply, so a refused pad action still releases the latch, and `soundLatched = false`. The action is the one on view, i.e. the Racks page's while Sound is latched (`pads.pads` already is the Racks page then). `onpad` itself carries only the index; PadBank knows nothing of the latch. Hardware pad taps release the latch only once contract change C3 lands. |

### Fixtures (`app/src/ui/PadBank/PadBank.fixtures.ts`)

- **`boardPadBank`** (`satisfies Omit<PadBankProps, 'onpad' | 'onbankup' | 'onbankdown'>`; stories and FullBand add the callbacks as actions), from Stage.md › Board fixture: `{ ...boardPadGrid, pageName: 'Sections', pageNumber: 1, pageCount: 5, upDisabled: true, downDisabled: false }`. So: `fallback: false`, `led: 0.25`, `running: true`, and the 16 pads of `boardPadGrid` (PadGrid's SPEC lists them: pads 3 and 7 `off`; 10 `bright` `solid`; 11 `bright` `flash`; 16 `bright` `solid`; the rest `dim` `solid`; no pad disabled). ▲ is disabled because the fixture's `padBankUp` `action` and `shiftAction` are null; ▼ is enabled.
- **`chordPadBank`** (same `satisfies`): `{ ...chordPadGrid, pageName: 'Chord', pageNumber: 3, pageCount: 5, upDisabled: false, downDisabled: false }` (Chord is page 3 in the default order).
- **`soundHeldPadBank`** (same `satisfies`): `{ ...boardPadBank, pageName: 'Racks', fallback: true, pads }` where `pads` is: pads 1–8 `QUICK 1`…`QUICK 8` (`QUICK 1` `bright`, the rest `dim`); pads 9–12 `OTS 1`…`OTS 4` `dim`; 13 `BANK -` `off`; 14 `BANK +` `dim`; 15 `STORE` `dim`; 16 `''` `off` with `disabled: true`; every other pad `disabled: false`; every `anim` `solid`.

## Stories (Story station)

- **Title:** `Components/PadBank`
- **Layout:** `centered` (real size, 626 × 186).
- **Meta:** `args: { onpad: fn(), onbankup: fn(), onbankdown: fn() }`; `parameters: { a11y: { context: { exclude: [...PAD_CONTRAST_EXCLUDE, '[data-part="legend"] > [data-hue="intro"]', '[data-part="legend"] > [data-hue="main"]', '[data-part="legend"] > [data-hue="brk"]'] } } }`, for every story.

Every story renders in dark and light. Controls: `pageName` text; `pageNumber` and `pageCount` number controls (`min: 1`, `step: 1`); PadGrid's (`pads` object, `fallback` boolean, `led` number with `step: 0.05`, `running` boolean) under the category `PadGrid`; `upDisabled` and `downDisabled` booleans under `Button`. Plays press a button with `userEvent.click`, a pad with `fireEvent.pointerDown(button, { button: 0, pointerId: 1 })`.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `boardPadBank` | the board's Pads section: header with the Sections legend and `Bank 1/5`, ▲ ▼, the Sections pads | `Board-{dark,light}.png`: Stage 690,590 626×186 | a region named `Pads`; inside it a heading level 2 named `Pads`; its text includes `Sections` and `Bank 1/5`; five legend items with `data-hue` `intro`, `main`, `ending`, `brk`, `fill`; the button named `Pad bank up` has `aria-disabled="true"`; `Pad bank down` doesn't; 18 buttons in all |
| `BankButtons` | `boardPadBank` | — | — | click `Pad bank up` → `onbankup` not called; click `Pad bank down` → `onbankdown` called once; focus `Pad bank down` and press Enter → called twice |
| `PadPress` | `boardPadBank` | — | — | pointer down on `Main C, queued (pad 11)` → `onpad` called with `10`; `onbankup` and `onbankdown` not called |
| `ChordPage` | `chordPadBank` | a page without its own spec, in the fallback look (Stage.md Check 8): `Chord`, no legend, `Bank 3/5`, the state's labels on the pads, no group lines | — (`PadsChord-Dark.dc.html` draws the real Chord page, with its own legend, group lines and sentence-case captions, not the fallback look) | no `[data-part="legend"]`; text includes `Chord` and `Bank 3/5`; every pad has `data-hue="t"`; pad 10 is named `STOP ACMP (pad 10)`; pad 1 `Unused (pad 1)` with `aria-disabled="true"` |
| `SoundHeld` | `soundHeldPadBank` | Sound held or latched on Sections: the Racks pads in the fallback look, `Bank 1/5` (the page on view stays Sections) | — (no board draws it) | text includes `Racks` and `Bank 1/5`; no `[data-part="legend"]`; pad 1 is named `QUICK 1, playing (pad 1)` with `data-face="solid"`, `data-state="playing"`; pad 13 is named `BANK -, not available (pad 13)` |
| `LastPage` | `{ ...chordPadBank, pageNumber: 3, pageCount: 3, downDisabled: true }` | the last page of a three-page order: ▼ disabled | — | `Pad bank down` has `aria-disabled="true"`; click it → `onbankdown` not called; click `Pad bank up` → `onbankup` called once; text includes `Bank 3/3` |
| `Focused` | `boardPadBank`; `parameters: { pseudo: { focusVisible: ['button[aria-label="Pad bank down"]'] } }` | the focus ring on ▼ | — | — |

What jsdom can't check (the legend's colours, the layout's pixel positions, the pads' looks) is covered by the `Board` crop (`npm run shots -- PadBank`).

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- PadBank` passes: `Board` is 626 × 186 and scores at most 0.02 against its crop (the disabled ▲'s glyph is `--d` where the board draws `--t2`: about 40 pixels, D3); axe finds no violation (with the Meta's exclusions).
- Only listed tokens are used; no inline colours; the only literal sizes are the seven geometry properties' values on the root.
- svelte-check and lint pass on the folder.

## Decisions

- D1. PadBank takes `upDisabled` / `downDisabled` computed by the wiring (with `ui.shift`) and reports plain `onbankup()` / `onbankdown()`; the ▲ ▼ keep their glyphs and names under Shift, since the tooltips already say what Shift does and the board draws no Shift label there.
- D2. Stage.md D18's latch release is the wiring's: `onpad(i)` carries only the index, and the wiring sends the pad's `action` and then `setLayer none` while its own `soundLatched` flag (set by LampRow's `onhold`) is true; PadBank and PadGrid know nothing of Sound.
- D3. The `Board` story disables ▲ (the fixture's `padBankUp` action is null), so its glyph is `--d`; the board draws it in `--t2`, a difference far under the 0.02 score, so the crop needs no mask.
- D4. The legend is `aria-hidden`: it is a colour key for the eye, and each pad's accessible name already says what the pad is.
- D5. The legend shows only when `fallback` is false (Sections); a page drawn in the fallback look has no group lines for a legend to explain (Stage.md D33).
- D6. The PadGrid sits 5px under the header with its own 3px line band, so the pads land on the board's y 634 and the ▲ ▼ column keeps the board's pixels (▲ at 652, ▼ at 726).
- D7. While Sound is held or latched the header reads `Racks` with the page on view's `Bank n/m` (`pads.pageName` and `pads.pageNumber` as the state gives them), as the hardware's pages don't move.
- D8. GroupHeader gets no `width`: it fills PadBank's root, which is fixed at `--pad-bank-width` (626), so the width is stated once.
- D9. "Bank n/m" is GroupHeader's `count` prop (its users table), not PadBank's own markup; the ▲ ▼ are Button `size: 'icon'` with `symbol: 'up'` / `'down'` and no label.
- D10. The section keeps `aria-label="Pads"` (rather than `aria-labelledby` the heading), so its region name stays `Pads` whatever GroupHeader's heading text becomes.

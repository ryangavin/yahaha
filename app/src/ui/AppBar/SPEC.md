# AppBar

## Identity (all stations)

- **Kind:** complex
- **Built from:** PageTabs, LaunchkeyStatus, HealthSlot
- **Purpose:** The strip along the top of the screen: the app's name, the page tabs, whether the Launchkey is there and whether the audio is healthy.
- **Boards:** `Stage-Dark.dc.html:72-93` (the `header`: wordmark, tabs with Stage chosen, the right area with Launchkey connected and the calm "Audio"); light: `Stage-Light.dc.html:48-69`. Crop box `Stage 24,24 1392×36`. The page boards (`SettingsSystem-Dark.dc.html:53-82`, light `SettingsSystem-Light.dc.html:42-71`; `LibrarySounds-Dark.dc.html:63-92`, …) draw the page variant, which adds the rack readout and One Touch between the wordmark and the tabs; that variant is the Channel spec's (#501), not this component.
- **Not this component's job:** no store, no API, no Tauri, no import from `app/src/lib`. It doesn't pick the health text (HealthSlot does, from the inputs AppBar passes through), doesn't open pages (it calls back; the wiring opens the page, or today's drawer until it exists, Stage.md D32), doesn't handle Alt shortcuts, and doesn't scale itself (Stage.md D1 is the app shell's). The tabs', the Launchkey status' and the health slot's own looks are their components' and aren't restated here.

## API (Component station)

### Props

`PageId` is from `app/src/ui/PageTabs/pages.ts`; `HealthTarget` from `app/src/ui/HealthSlot/health.ts`.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `page` | `PageId \| null` | `null` | The page on view, passed to PageTabs as `chosen`. `null`: no tab chosen. |
| `connected` | `boolean` | `false` | The Launchkey is connected, passed to LaunchkeyStatus. |
| `health` | `{ failedPart: 0 \| 1 \| 2 \| 3 \| null; synthOn: boolean; dropouts: number; bufferFrames: number \| null; cpu: number \| null }` | `{ failedPart: null, synthOn: true, dropouts: 0, bufferFrames: null, cpu: null }` (calm) | HealthSlot's inputs, spread onto it unchanged (their meanings are HealthSlot's props). |
| `width` | `number \| undefined` | — | A fixed width in px. Default: fills its container's width (the Stage's content box gives it 1392). Stories pass 1392 (D1). |
| `tipAction` | `Action<HTMLElement, string> \| undefined` (`svelte/action`) | — | The app's tooltip action (`use:tip`), passed in by the wiring because the library can't import it. AppBar hands it to PageTabs, and builds LaunchkeyStatus' and HealthSlot's `tip` attachments from it (Children). |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onchoose` | PageTabs' `onchoose`, passed through: a tab other than the chosen one is chosen | `(id: PageId)` |
| `onopen` | HealthSlot's `onopen`, passed through: the health slot's button is pressed (only trouble rows and "Audio off" have one) | `(target: HealthTarget)`: `{ page: 'channel', part }` for a failed plugin, `{ page: 'settings', tab: 'system' }` for the rest |

### From the state

What the page wiring passes and does (it reads `app.state` and app-side state; AppBar doesn't):

| Prop / callback | Fed from / does |
|---|---|
| `page` | `ui.page` (app-only, Stage.md D2) once pages exist; until then PageTabs' interim rule (its SPEC, "Interim: `chosen` is this tab while") |
| `connected` | `app.state.pads.connected` |
| `health.failedPart` | `firstFailedPart(app.state.keyboardParts.map(p => p.plugin))` (HealthSlot's `health.ts`) |
| `health.synthOn` | `app.state.io.synth !== null` |
| `health.dropouts` | the times `app.state.io.synth.dropouts` rose in the last 30 s, counted by the wiring |
| `health.bufferFrames` | `app.state.io.synth?.bufferFrames ?? null` |
| `health.cpu` | `app.state.meters.cpu.total`, `null` before the first meters frame |
| `tipAction` | `tip` from `app/src/lib/tooltip/tip.svelte.ts` |
| `onchoose(id)` | `ui.page = id` once that page exists; until then PageTabs' interim action |
| `onopen(target)` | `channel`: Channel for that part (#501), until then `panels/channel/nav.svelte.ts` `show(part)`; `settings` › `system`: Settings › System (#532), until then the Settings drawer on its Audio tab (Stage.md D32) |

Launchkey mapping: none for any part of the app bar.

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Children

Left to right inside the bar:

| Order | Element / child | Props passed | Placement |
|---|---|---|---|
| 1 | wordmark | AppBar's own `span`, text "yahaha" | at the bar's left edge |
| 2 | `PageTabs` | `{ chosen: page, tipAction, onchoose }` | `margin-left: auto`, so it is right-aligned against the right area with the bar's 8px gap between them |
| 3 | right area | AppBar's own `div` | `flex: none`, width `--app-bar-right` (196px), height `--bar-height` (36px), `display: flex; align-items: center` |
| 3a | separator | AppBar's own `span`, `aria-hidden="true"` | first in the right area |
| 3b | `LaunchkeyStatus` | `{ connected, tip: tipAction ? fromAction(tipAction, () => 'launchkey.status') : undefined }` | in a cell `span`: `margin-left: var(--space-8)`, `flex: none`, `display: flex` |
| 3c | `HealthSlot` | `{ ...health, tip: tipAction ? fromAction(tipAction, () => 'app.health') : undefined, onopen }`; no `width`, so it takes `flex: 1 1 auto` | in a cell `div`: `flex: 1 1 0; min-width: 0; display: flex; align-self: flex-start` (top of the right area, D7). HealthSlot fills the cell, keeps its own 8px left padding and puts its text at the cell's right edge, which is the bar's right edge; that leaves about 96px for the text at the board's font, and longer texts end in "…" (HealthSlot D4) |

`fromAction` is from `svelte/attachments`. At 1392 wide on the Stage (bar at x 24): wordmark x 24–83, tabs x 527–1212, separator x 1220, Launchkey status from x 1229, "Audio" ending at x 1416.

### Visual rules

- **Tokens used:** `--t`, `--line`, `--font-sans`, `--text-18`, `--weight-medium`, `--space-8`, `--line-width`, `--bar-height` (new, 36px, D5), `--separator-height` (new, 16px; ChosenTabs adds it too), `--app-bar-right` (new, 196px, D5), `--tracking-wordmark` (new, −0.2px, D5).
- **Root:** a `header`, `box-sizing: border-box`, height `--bar-height` (36), width `width` px or 100% of its container, no padding, no background (the page's `--g` shows through), `display: flex; align-items: center; gap: var(--space-8)`, `font-variant-numeric: tabular-nums`. Never wraps; the tabs and the right area never shrink (`flex: none`), so a narrower bar only narrows the empty space between the wordmark and the tabs.
- **Vertical:** items are centred in the full 36px (their centre line 18px from the top: on the board, y 42; the right-area separator spans y 34–49 and the dot y 39–44), except the health cell, at the top (D7).
- **The bottom line:** 1px (`--line-width`) of `--t` on the bar's bottom pixel row (board row 59), drawn as `box-shadow: inset 0 calc(-1 * var(--line-width)) 0 var(--t)` on the root, so the children paint over it (D2). The chosen tab's 24px block, also `--t`, reaches that row and so stands on the line.
- **Wordmark:** "yahaha", DM Sans 18 / 500 (`--font-sans`, `--text-18`, `--weight-medium`), letter-spacing `--tracking-wordmark` (−0.2px), `--t`, `line-height: normal`, `white-space: nowrap`, `flex: none`, cursor `default`. Not a control.
- **Right-area separator:** `--line-width` (1px) wide, `--separator-height` (16px) tall, `--line` fill, `flex: none`, centred vertically.
- **States drawn by:** the children only: which tab is chosen (PageTabs), connected or not (LaunchkeyStatus), the health text, colour and button (HealthSlot). The wordmark, the line and the separator never change. No hover or pressed look (kit: Hover, press and cursor).
- **Contrast (AA 4.5:1, `tokens/contrast.test.ts`):** `--t` on `--g` (wordmark; exists). The children list their own pairs.
- **Motion:** none.

### Accessibility

- **Role and name:** the root is a `header` (the page's `banner` landmark), with no `aria-label`. Inside: the `nav` "Pages" (PageTabs), the `status` "Launchkey connected" / "Launchkey not connected" (LaunchkeyStatus), and the `status` "Audio health: {text}" ("Audio health: fine" when calm) holding a `button` named by its text when the row has a target (HealthSlot). The wordmark is plain text; the separator is `aria-hidden`.
- **Keyboard:** Tab order is the nine tabs left to right, then the health button when there is one (kit › Interaction conventions). Nothing else in the bar is focusable.
- **Tooltip id:** none of its own. The tabs carry theirs (PageTabs), the Launchkey status `launchkey.status`, the health slot `app.health` (new, Contract change C5; one key for every state, Stage.md D45); behaviour through `tipAction`.

## Stories (Story station)

Title `Components/AppBar`, `layout: 'centered'`, and on the meta `parameters.shots = { viewport: { width: 1440, height: 900 } }` (D6). Every story renders in dark and light (the toolbar theme). The meta's args are the Board args plus `onchoose: fn()`, `onopen: fn()`, `tipAction: fn()`; each story overrides only what it lists. Controls: `page` (a select of the nine ids and `null`), `connected` (boolean), `width` (number), and the five `health` fields grouped under "HealthSlot" (`table.category`: `failedPart` a select of 0–3 and `null`, `synthOn` boolean, `dropouts` and `bufferFrames` numbers, `cpu` a 0–1 range); callbacks are actions.

`calm` below is `{ failedPart: null, synthOn: true, dropouts: 0, bufferFrames: 256, cpu: 0.2 }`, the Stage board fixture's values (R3's plugin is missing, so it isn't "failed").

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ page: 'stage', connected: true, health: calm, width: 1392 }` | the Stage board's app bar | `Board-{dark,light}.png` (Stage 24,24 1392×36) | the `banner` holds the text "yahaha", the `navigation` "Pages" with 9 buttons ("Stage" has `aria-current="page"`), the `status` "Launchkey connected" and the `status` "Audio health: fine", which holds no button; press Tab 10 times → focus has left the story (9 tab stops only) |
| `LaunchkeyNotConnected` | `{ connected: false }`, `parameters.a11y.config.rules: [{ id: 'color-contrast', enabled: false }]` | the hollow dot and the dimmed "Launchkey" in place | — (no board draws it) | the `status` "Launchkey not connected" exists |
| `HealthFailed` | `{ health: { ...calm, failedPart: 2 } }` | "R3 failed" in red, a text button | — (D8) | the `status` "Audio health: R3 failed" holds a `button` named "R3 failed"; press Tab 10 times → that button has focus (after the 9 tabs); press Enter → `onopen` called once with `{ page: 'channel', part: 2 }` |
| `HealthAudioOff` | `{ health: { ...calm, synthOn: false, bufferFrames: null, cpu: null } }` | "Audio off" in grey, a button | — (D8) | click the `button` "Audio off" → `onopen` called with `{ page: 'settings', tab: 'system' }` |
| `HealthBufferHint` | `{ health: { ...calm, dropouts: 3, bufferFrames: 128 } }` | the longest text, "3 dropouts · buffer 256?", cut with an ellipsis at the bar's right edge; the tabs and the Launchkey status stay where `Board` has them | — (D8) | the `status` is named "Audio health: 3 dropouts · buffer 256?" and its button is named "3 dropouts · buffer 256?" (the cut is CSS; Inspect checks it) |
| `HealthDropouts` | `{ health: { ...calm, dropouts: 2 } }` | "2 dropouts" in red | — (D8) | click the `button` "2 dropouts" → `onopen` called with `{ page: 'settings', tab: 'system' }` |
| `HealthCpu` | `{ health: { ...calm, cpu: 0.74 } }` | "CPU 74%" in red | — (D8) | the `status` "Audio health: CPU 74%" holds a `button` named "CPU 74%" |
| `LibraryChosen` | `{ page: 'library' }` | Library chosen, past the separator | — (D8) | "Library" has `aria-current="page"`; "Stage" has none |
| `SettingsChosen` | `{ page: 'settings', health: { ...calm, cpu: 0.74 } }` | Settings chosen, its block just left of the right area, with "CPU 74%" (the Settings › System board's state) | — (D8) | "Settings" has `aria-current="page"`; click "Effects" → `onchoose` called once with `'effects'` |
| `NoPage` | `{ page: null }` | every tab grey | — | no button has `aria-current` |
| `Focused` | Board args, `parameters: { pseudo: { focusVisible: ['[aria-current="page"]'] } }` | the focus ring on the chosen Stage tab | — | — |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in the dark and light render.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- AppBar` passes: `Board`'s screenshot is 1392 × 36 and scores at most 0.02 against its crop (or the Inspect agent judges the difference render noise and says so), and axe finds no violation on any story, `LaunchkeyNotConnected`'s `color-contrast` excepted. This needs two `scripts/shots.ts` items: the per-story viewport (Stage.md D39) and honouring `parameters.a11y.config.rules` (LaunchkeyStatus D3).
- Inspect confirms by eye: `HealthBufferHint` ends in "…" inside the bar with the tabs at `Board`'s x; `LaunchkeyNotConnected` shows the hollow ring; `LibraryChosen` and `SettingsChosen` match the tabs on `LibrarySounds-*.png` and `SettingsSystem-*.png`.
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · Width.** The bar fills its container by default (the Stage's content box is 1392 wide) and takes an optional `width` in px, as LampButton does, so its stories render at the board's 1392 without a wrapper.
- **D2 · The line under the children.** The board's `border-bottom` on a 36px border-box header would centre the items in 35px, but its render centres them in the full 36 (separator y 34–49, dot y 39–44) with the chosen block on row 59; an inset bottom shadow draws the same 1px `--t` line, keeps the centre at 18px and lets the block cover it.
- **D3 · HealthSlot picks the text.** AppBar passes HealthSlot's five inputs through as one `health` object and its `onopen(target)` back out; it neither picks the row (HealthSlot's `health()`, Stage.md D14 order) nor knows the targets.
- **D4 · Long health text is cut.** The right area stays 196px so the tabs keep their x on every board, which leaves about 96px for the health text; "3 dropouts · buffer 256?" shows cut with "…", and its full text stays in the slot's accessible name (owner question, with HealthSlot D4).
- **D5 · New scale tokens.** `--bar-height: 36px` (the app bar; the 36px hairline header rows can share it), `--app-bar-right: 196px` and `--tracking-wordmark: -0.2px` are added to `app/src/ui/tokens/scale.css` (UI library contract), with ChosenTabs' `--separator-height: 16px` (the right-area separator is the same 1 × 16).
- **D6 · Shots viewport.** The bar is wider than `scripts/shots.ts`' default 1000 × 600 viewport, so the meta sets `parameters.shots.viewport` to 1440 × 900, which shots.ts gains with the Stage (Stage.md D39); until then the `Board` shot can't be taken at full width.
- **D7 · Health cell at the top.** HealthSlot is 35px tall (the bar less its line, HealthSlot D5), so AppBar puts it at the top of the right area (`align-self: flex-start`), the box its own `Board` crop uses (`1316,24 100×35`), rather than half a pixel lower.
- **D8 · Crops only for the Stage bar.** The boards that show Library or Settings chosen or "CPU 74%" draw the page variant of the bar (rack readout and One Touch, #501), so those stories have no whole-bar crop; Inspect judges them by eye against those boards.
- **D9 · The wordmark is text.** "yahaha" is a plain `span`, not a heading, link or control (the board draws none; the Stage tab is the way home).
- **D10 · Tooltips from one action.** The wiring passes one `tipAction`; AppBar hands it to PageTabs (each tab's key) and turns it into the `tip` attachments LaunchkeyStatus and HealthSlot take, with `fromAction` and their keys `launchkey.status` and `app.health`.

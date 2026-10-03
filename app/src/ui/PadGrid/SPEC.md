# PadGrid

## Identity (all stations)

- **Kind:** complex
- **Built from:** Pad
- **Purpose:** The Launchkey's 16 pads as on the hardware, two rows of eight, with the Sections page's coloured group lines showing which pads belong together.
- **Boards:** `Stage-Dark.dc.html:337-355` (the grid, its nine group-line spans and the pad loop); light: `Stage-Light.dc.html:313-331`. Box `730,631 586×145` on both boards (a 3px band for the group lines, then the two pad rows from y 634).
- **Not this component's job:** no store, no API, no Tauri, no timer. It doesn't know the page name, the Bank pager or Sound latch (PadBank and the wiring). It doesn't draw a pad (Pad does); it places the 16 Pads, picks each pad's caption and family, draws the group lines, and reports which pad was pressed.

## API (Component station)

### Props

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `pads` | `PadState[]` (16 entries) | — | The 16 pads in hardware order: `pads[0..7]` the top row (pads 1–8), `pads[8..15]` the bottom row (pads 9–16). `PadState` is `{ label: string; level: 'off' \| 'dim' \| 'bright'; anim: 'solid' \| 'flash' \| 'pulse'; disabled: boolean }`, exported from `PadGrid.svelte`'s module script. |
| `fallback` | `boolean` | `false` | false: the Sections page (fixed captions and families from `SECTIONS`, group lines). true: the fallback face for a page whose spec hasn't landed (each pad captioned with its `label`, no group lines; Stage.md D33). |
| `led` | `number` | `0` | The LED clock in beats (Pad's `ledBeats`), passed to every Pad. |
| `running` | `boolean` | `false` | The band is playing. On Sections, Start / Stop (pad 16) is drawn from its `level` only while `running`; stopped, it is passed as `dim` (D3). Unused under `fallback`. |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onpad` | a Pad's `onpress` fires (pointer down, Enter or Space on an enabled pad) | `(index: number)` the pad's 0-based index into `pads` (pad 11 → `10`) |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Data (`app/src/ui/PadGrid/pages.ts`)

Plain constants, no logic beyond the run maths; `pages.test.ts` checks them.

**`SECTIONS`**: the Sections page, 16 entries `{ caption, family, tip }` in pad order (DECISIONS H3; captions use U+00A0, written ` ` here, so only whole words wrap):

| Pad | `caption` | `family` | `tip` |
|---|---|---|---|
| 1 | `Intro I` | `intro` | `section.intro1` |
| 2 | `Intro II` | `intro` | `section.intro2` |
| 3 | `Intro III` | `intro` | `section.intro3` |
| 4 | `Sync Start` (plain space) | `util` | `transport.sync_start` |
| 5 | `Ending I` | `ending` | `section.ending1` |
| 6 | `Ending II` | `ending` | `section.ending2` |
| 7 | `Ending III` | `ending` | `section.ending3` |
| 8 | `Auto Fill` (plain space) | `util` | `transport.auto_fill` |
| 9 | `Main A` | `main` | `section.main_a` |
| 10 | `Main B` | `main` | `section.main_b` |
| 11 | `Main C` | `main` | `section.main_c` |
| 12 | `Main D` | `main` | `section.main_d` |
| 13 | `Break` | `brk` | `section.break` |
| 14 | `Tap` | `util` | `tempo.tap` |
| 15 | `Sync Stop` (plain space) | `util` | `transport.sync_stop` |
| 16 | `Start / Stop` (plain spaces) | `start` | `transport.start_stop` |

**`SECTIONS_RUNS`**: the group lines, `{ row: 0 | 1; col: number; count: number; hue: 'intro' | 'main' | 'ending' | 'brk' | 'util' }[]`, in this order. A run's line is at `x = 74 · col`, `width = 74 · count − 6` within the grid box (the pads' columns are 68 wide with 6px gaps, so the line spans exactly its pads); `y = 0` for row 0 and `y = 74` for row 1. Board x is the grid box's 730 plus x; board y is 631 or 705.

| # | Pads | `row`, `col`, `count` | `hue` | x, width (in the box) | Board x, y |
|---|---|---|---|---|---|
| 1 | 1–3 Intro | 0, 0, 3 | `intro` | 0, 216 | 730, 631 |
| 2 | 4 Sync Start | 0, 3, 1 | `util` | 222, 68 | 952, 631 |
| 3 | 5–7 Ending | 0, 4, 3 | `ending` | 296, 216 | 1026, 631 |
| 4 | 8 Auto Fill | 0, 7, 1 | `util` | 518, 68 | 1248, 631 |
| 5 | 9–12 Main | 1, 0, 4 | `main` | 0, 290 | 730, 705 |
| 6 | 13 Break | 1, 4, 1 | `brk` | 296, 68 | 1026, 705 |
| 7 | 14 Tap | 1, 5, 1 | `util` | 370, 68 | 1100, 705 |
| 8 | 15 Sync Stop | 1, 6, 1 | `util` | 444, 68 | 1174, 705 |
| 9 | 16 Start / Stop | 1, 7, 1 | `util` | 518, 68 | 1248, 705 |

Each utility pad has its own line (each is its own function), as the board draws it (D2).

**`FALLBACK_TIPS`**: `{ racks: 'padpage.racks', chord: 'padpage.chord', multiPads: 'padpage.multi_pads', setup: 'padpage.setup' }` and **`UNUSED_TIP`** `'launchkey.unused'`: the tooltip keys integration puts on fallback pads (by the page shown; Racks while Sound is held), and on fallback pads with an empty label. Every key in this file exists in `tooltips.ts`.

### Children

| Child | Where | Props passed |
|---|---|---|
| `Pad` × 16 | grid cell `i` (row `floor(i / 8)`, column `i mod 8`), in DOM order 1–16 | `index: i + 1`; `caption`: `fallback ? pads[i].label : SECTIONS[i].caption`; `family`: `fallback ? 'util' : SECTIONS[i].family`; `level`: `pads[i].level`, except pad 16 on Sections while `running` is false: `pads[i].level === 'off' ? 'off' : 'dim'`; `anim`: `pads[i].anim`; `led`; `fallback`; `disabled`: `pads[i].disabled`; `onpress`: `() => onpad(i)` |

### Visual rules

- **Tokens used:** `--intro`, `--main`, `--ending`, `--brk`, `--util` (the group lines). Everything inside a pad is Pad's.
- **Size:** fixed 586 × 145 (the Stage's 1440 × 900 layout; Stage.md D1 scales the whole page).
- **Layout:** `position: relative`. The pads are a CSS grid starting 3px below the box's top (`margin-top: 3px` or `top: 3px`): `grid-template-columns: repeat(8, 68px)`, `grid-auto-rows: 68px`, `column-gap: 6px`, `row-gap: 6px` (8 × 68 + 7 × 6 = 586; 2 × 68 + 6 = 142). So pad `c` of a row starts at x `74 · c` in the box, the top row at y 3, the bottom row at y 77.
- **Group lines** (Sections only, `fallback` false): one `<span aria-hidden="true" data-run="<first>-<last>" data-hue="<hue>">` per `SECTIONS_RUNS` entry, before the pads in the DOM, `position: absolute` at its x and y, its width, height 2px, square ends, filled with the run's hue token (`--util` for utility runs). The lines sit 3px above their pads' top edges, in the gap between the box's top (or the 6px row gap) and the pads, so a pad never covers one. Under `fallback` there are no lines (the 3px band stays, empty).
- **States drawn by:** Sections (lines, captions from `SECTIONS`) or fallback (no lines, captions from `label`); everything else is each Pad's.
- **Contrast:** none of its own (the lines are not text).
- **Motion:** none of its own; `led` reaches every Pad.

### Accessibility

- **Role and name:** a plain `div`; the 16 pads are its buttons, named by Pad. PadBank's `<section aria-label="Pads">` is the region.
- **Keyboard:** Tab moves through pads 1–16 in order (top row, then bottom row); the lines are not focusable.
- **Test hooks:** each line has `data-run` (`1-3`, `4-4`, `5-7`, `8-8`, `9-12`, `13-13`, `14-14`, `15-15`, `16-16`) and `data-hue` (`intro`, `util`, `ending`, `main`, `brk`).
- **Tooltip id:** per pad, `SECTIONS[i].tip` on Sections, else `FALLBACK_TIPS[page]` or `UNUSED_TIP` (wired at integration; Pad's SPEC has the table).
- **Launchkey:** the 16 pads (DAW port notes 96–103, 112–119).

### From the state

PadBank passes these through unchanged; the wiring's table is in PadBank's SPEC.

| Prop / callback | From `AppState` / what the wiring sends |
|---|---|
| `pads[i]` | `{ label: pads.pads[i].label, level: pads.pads[i].level, anim: pads.pads[i].anim, disabled: pads.pads[i].action === null }` |
| `fallback` | `pads.page !== 'sections' \|\| surface.layer.type === 'sound'` |
| `led` | `ledBeats(surface.clock, now, receivedMs)` |
| `running` | `transport.running` |
| `onpad(i)` | `send(pads.pads[i].action)`, then `setLayer none` while Sound is latched (PadBank's SPEC) |

### Fixtures (`app/src/ui/PadGrid/PadGrid.fixtures.ts`)

- **`boardPadGrid`** (`satisfies Omit<PadGridProps, 'onpad'>`), from Stage.md › Board fixture: `fallback: false`, `led: 0.25`, `running: true`, and `pads` (label as the dev mock's Sections page names them; every `anim` not named is `solid`; every `disabled` false, since every Sections pad has an action):

  | Pad | `label` | `level` | `anim` | Pad | `label` | `level` | `anim` |
  |---|---|---|---|---|---|---|---|
  | 1 | `INTRO 1` | `dim` | `solid` | 9 | `MAIN A` | `dim` | `solid` |
  | 2 | `INTRO 2` | `dim` | `solid` | 10 | `MAIN B` | `bright` | `solid` |
  | 3 | `INTRO 3` | `off` | `solid` | 11 | `MAIN C` | `bright` | `flash` |
  | 4 | `SYNC ST` | `dim` | `solid` | 12 | `MAIN D` | `dim` | `solid` |
  | 5 | `ENDING 1` | `dim` | `solid` | 13 | `BREAK` | `dim` | `solid` |
  | 6 | `ENDING 2` | `dim` | `solid` | 14 | `TAP` | `dim` | `solid` |
  | 7 | `ENDING 3` | `off` | `solid` | 15 | `SYNC STP` | `dim` | `solid` |
  | 8 | `AUTOFILL` | `dim` | `solid` | 16 | `START` | `bright` | `solid` |

- **`chordPadGrid`**: the Chord page as the dev mock draws it, under the fallback face: `fallback: true`, `led: 0.25`, `running: true`; pads 1–8 `{ label: '', level: 'off', anim: 'solid', disabled: true }`; pads 9–16 `{ level, anim: 'solid', disabled: false }` with labels and levels `MAN BASS` off, `STOP ACMP` dim, `SPLIT -` dim, `SPLIT +` dim, `KBD TR -` dim, `KBD TR +` dim, `TR RESET` dim, `RETRIG` dim.

## Stories (Story station)

- **Title:** `Components/PadGrid`
- **Layout:** `centered` (real size, 586 × 145).

Every story renders in dark and light. `onpad` is an action (`fn()`). Controls: `fallback`, `running`, `led` (number, step 0.05) and, grouped under the category `Pad` (`table.category`), `pads` as an object control.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `boardPadGrid` | the Sections page as on the board: Main B playing, Main C queued at full flash, Intro III and Ending III absent, Start / Stop running, the nine group lines | `Board-{dark,light}.png`: Stage 730,631 586×145 | 16 buttons; 9 `[data-run]` lines with `data-hue` `intro`, `util`, `ending`, `util`, `main`, `brk`, `util`, `util`, `util` in that order; the button named `Main B, playing (pad 10)` has `data-face="playing"`; `Main C, queued (pad 11)` has `data-face="next"` and numeral `NEXT`; pads 3 and 7 have `data-face="absent"`; pad 16 is named `Start / Stop, running (pad 16)` |
| `Presses` | `boardPadGrid` | — | — | click `Main C, queued (pad 11)` → `onpad` called with `10`; click `Intro I (pad 1)` → called with `0`; called twice in all |
| `Stopped` | `boardPadGrid` with `running: false` | the band stopped: Start / Stop idle (the other pads as the state gives them) | — (the board's band runs) | pad 16 is named `Start / Stop, stopped (pad 16)` with `data-face="idle"` |
| `IntroArmed` | `boardPadGrid` with `running: false`, pads 1–2 `{ level: 'bright', anim: 'pulse' }`, pads 10 and 11 `dim`, `led: 1` | Stage.md › States "Intro armed (stopped)": the armed Intros pulsing at their peak | — (no board draws it) | pads 1 and 2 have `data-face="armed"`; pad 1 is named `Intro I, armed (pad 1)` |
| `ChordPage` | `chordPadGrid` | the fallback face (Stage.md Check 8): labels as captions, no group lines, the top row blank | — (`PadsChord` draws the real Chord page, not the fallback) | no `[data-run]` line; every pad has `data-hue="t"`; pad 10 is named `STOP ACMP (pad 10)`; pad 1 is named `Unused (pad 1)` and `aria-disabled="true"`; pad 9 is named `MAN BASS, not available (pad 9)`; clicking pad 1 → `onpad` not called |

No `Focused` story: PadGrid itself isn't focusable; its pads' focus ring is Pad's `Focused`.

What jsdom can't check (the line colours and positions, the pads' looks) is covered by the `Board` crop (`npm run shots -- PadGrid`).

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `pages.test.ts`: `SECTIONS` has 16 entries with the captions above (U+00A0 where shown) and these families and tips; `SECTIONS_RUNS` covers each of pads 1–16 exactly once, and its x and width follow `74 · col` and `74 · count − 6` (run 5: x 0, width 290).
- `npm run shots -- PadGrid` passes: `Board` is 586 × 145 and scores at most 0.02 against its crop; axe finds no violation.
- Only listed tokens are used; no inline colours; the sizes that aren't tokens are only those in Visual rules (586 × 145, 68 columns and rows, gaps 6, line band 3, line height 2, `74 · col`, `74 · count − 6`).
- svelte-check and lint pass on the folder.

## Decisions

- D1. PadGrid owns the Sections page's captions, families, runs and tooltip keys (`pages.ts`) and takes the state's pads plus `fallback`, so the wiring passes the same shape on every page and the fixed Sections table lives in one place.
- D2. Each utility pad on Sections gets its own group line (pads 4, 8, 14, 15 and 16), as the board draws them, not one line over 14–16 as kit.md › Pads words it.
- D3. Start / Stop (pad 16) is idle while the band is stopped (`running` false), since the state lights it bright in both cases (dev mock: `START` / `STOP` bright, only its colour differs) and Stage.md › States wants it idle when stopped.
- D4. `onpad` carries the 0-based index into `pads.pads` (pad 11 → 10), so the wiring indexes the state directly.
- D5. Under `fallback` the 3px line band stays empty, so the grid keeps its box and PadBank's layout doesn't move between pages.
- D6. The grid is a fixed 586 × 145 with 68px columns, the 1440 × 900 layout's numbers; scaling is the app shell's (Stage.md D1).

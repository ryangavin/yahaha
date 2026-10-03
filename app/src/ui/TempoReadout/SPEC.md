# TempoReadout

## Identity (all stations)

- **Kind:** complex
- **Built from:** StatusDot
- **Purpose:** Shows the tempo in big numbers and whether the band is running, waiting for Sync Start, or stopped.
- **Boards:**
  - `Stage-Dark.dc.html:168-172` (the display's tempo row: "104" 32px, "BPM", and at the right a green dot with "Running"); light: `Stage-Light.dc.html:144-148`. Spec: Stage.md › Display › Section and tempo (the tempo row), D27, D44.
- **Not this component's job:** no store, no API, no Tauri. Read-only (Stage D27): it doesn't set the tempo (Tempo ±, Tap, Style tempo and knob 8 do), has no click, no focus and no typed entry. It doesn't place itself in the section column (SectionReadout gives it the 50px above). It doesn't draw the dot itself: StatusDot does. Not the compact block's tempo (CompactNowPlaying's, 18px). No `use:tip` import (L3).

Its user: SectionReadout (Stage.md row 31) passes `{ tempo: transport.tempo, running: transport.running, syncStart: transport.syncStart, tip: 'display.tempo', tipAction }`.

Its child and the props it passes:

| Run state | StatusDot props |
|---|---|
| `running` | `{ hue: 'ok' }` (solid, with its glow) |
| stopped with `syncStart` | `{ hue: 'ok', hollow: true }` |
| stopped | no StatusDot (left out, so its space collapses) |

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `tempo` | `number` | — (required) | `transport.tempo`, BPM; shown rounded to a whole BPM with `Math.round` (Stage D44: 103.6 → "104"). |
| `running` | `boolean` | `false` | `transport.running`. |
| `syncStart` | `boolean` | `false` | `transport.syncStart`: Sync Start armed; read only while stopped. |
| `width` | `number \| undefined` | — | Stories only: a fixed width in px (the board's 490). Without it the row fills its container's width (D3). |
| `tip` | `string \| undefined` | `'display.tempo'` | The tooltip key on the number, rendered as `data-tip`; no attribute when set to undefined (L3). |
| `tipAction` | `Action<HTMLElement, string> \| undefined` | — | The app's `use:tip`, passed in by the wiring. When both it and `tip` are set the number gets `use:tipAction={tip}`; otherwise nothing (L3). |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| — | | |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Visual rules

- **Tokens used:** `--t`, `--t2`, `--m`, `--ok`, `--font-sans`, `--text-14`, `--text-32`, `--weight-light`, `--weight-regular`, `--space-6`, `--space-8`, and the new token below. StatusDot draws its own dot and glow.

#### New tokens

| Token | Dark | Light | Used for |
|---|---|---|---|
| `--tracking-32` | `-0.5px` | `-0.5px` | the tempo number's letter-spacing (scale.css; the lane's `--tracking-<size>` naming, as WaitingChip's `--tracking-36`; Harmony's `--ls-section` is the same −0.5px) |

It lands in the orchestrator's tokens contract PR before the build (L1).

- **Element:** a root `div`: `display: flex`, `align-items: baseline`, `gap: var(--space-6)`, `height: 40px`, `white-space: nowrap`, `font-family: var(--font-sans)`, `font-variant-numeric: tabular-nums`; `width: <width>px` when `width` is set. Three items:
  1. **Number** (`data-part="tempo"`, a `span`): `Math.round(tempo)`, `--text-32` / 300, line-height 40px, letter-spacing `--tracking-32`, `--t`. Carries `data-tip` and the tip action.
  2. **Unit** "BPM": `--text-14` / 400, `--m`.
  3. **Run state** (`data-part="run"`, a `span`): `margin-left: auto`, `align-self: center`, `display: inline-flex`, `align-items: center`, `gap: var(--space-8)`, `--text-14` / 400. `data-run` and `data-hue` say which:

     | State | `data-run` | Dot | Word | `data-hue` |
     |---|---|---|---|---|
     | `running` | `running` | StatusDot `{ hue: 'ok' }`: solid 6px `--ok`, its glow in dark | "Running" in `--ok` | `ok` |
     | not running, `syncStart` | `sync` | StatusDot `{ hue: 'ok', hollow: true }`: a 1px `--ok` ring | "Sync start" in `--t2` | `t2` |
     | not running | `stopped` | none (no element; the gap goes with it) | "Stopped" in `--m` | `m` |

- **Size:** 40 tall; fills its container's width (490 in the Stage's section column) or `width`.
- **States drawn by:** the run state above; the number only changes text.
- **Type:** DM Sans, tabular numerals (the number keeps its width as it changes); the number light, the words regular.
- **Contrast (AA, `tokens/contrast.test.ts`):** `--t` on `--g` (the number, 32px: large text; existing row), `--m` on `--g` ("BPM", "Stopped"; existing row), `--t2` on `--g` ("Sync start"; existing row), `--ok` on `--g` ("Running", 14px: normal text, 4.5:1). Light `--ok` (`--green-700` #1c8040) is 4.42:1 on `--g` today and fails (L2): the fix is the `--green-700` change already proposed (WaitingChip D9: #1c7e3f, 4.53; Stage C6 proposes #19733a for the `--btn` pairs, which also passes on `--g`); this spec adds the row `['--ok', '--g', 'run state text (TempoReadout)']` to the tokens PR's list and no value of its own.
- **Motion:** none. The dot's glow is StatusDot's and static.

### Accessibility

- **Role and name:** none: plain text read in place ("104 BPM Running"); the dot is decorative (StatusDot's default `aria-hidden`), since the word beside it says the state. Not a live region (the tempo changes too often to announce).
- **Keyboard:** none; not focusable.
- **Tooltip id:** `display.tempo` (exists in `app/src/help/tooltips.ts`) on the number, the `tip` default (L3).

## Stories (Story station)

Title `Components/TempoReadout`, `layout: 'centered'`. Every story renders in dark and light (the toolbar theme). The meta's `args` are `{ tip: 'display.tempo', tipAction: fn(), width: 490 }` (L3; 490 is the section column's width on the board). Args come from the Stage board fixture (`transport.tempo` 104, `running` true, `syncStart` false).

**Controls (argTypes):** `tempo`, `width` numbers (cleared `width` = undefined); `running`, `syncStart` booleans; `tip` text; `tipAction` an action. StatusDot's props are not controls here: the run state picks them.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ tempo: 104, running: true, syncStart: false }` | "104" over a light 32px, "BPM" grey, a green glowing dot with "Running" at the right | `Board-{dark,light}.png` (Stage 373,291 490×40) | the root's `textContent`, whitespace collapsed, starts with "104 BPM" and ends with "Running" (Stage Check 5); `[data-part="tempo"]` has `data-tip="display.tempo"` and `tipAction` was called with it and `'display.tempo'`; `[data-part="run"]` has `data-run="running"` and `data-hue="ok"`; it holds one StatusDot with `data-face="solid"`, `data-hue="ok"` |
| `Rounded` | `{ tempo: 103.6, running: true }` | the tempo rounded to a whole BPM | — | `[data-part="tempo"]` text is "104" |
| `SyncStart` | `{ tempo: 104, running: false, syncStart: true }` | stopped with Sync Start armed: a hollow green ring, "Sync start" in light grey | — (no board draws it) | `data-run="sync"`, `data-hue="t2"`; the dot has `data-face="hollow"`; text ends with "Sync start" |
| `Stopped` | `{ tempo: 104, running: false, syncStart: false }` | "Stopped" grey, no dot | — (no board draws it) | `data-run="stopped"`, `data-hue="m"`; no StatusDot (`[data-face]` absent in the run state); text ends with "Stopped" |
| `RunningIgnoresSync` | `{ tempo: 104, running: true, syncStart: true }` | running wins: "Running" | — | `data-run="running"` |
| `Slowest` | `{ tempo: 40, running: true }` | the lowest tempo | — | number text "40" |
| `Fastest` | `{ tempo: 280, running: true }` | the highest tempo, three digits | — | number text "280" |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in the dark and light render. The `Board` crop is the row's own box (L6); the dot's glow is inside it.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- TempoReadout` passes: `Board` scores at most 0.02 against its crop (or the Inspect agent judges the difference render noise), and axe finds no violation on any story once the tokens PR's light `--ok` lands; until then `Board`, `Rounded`, `RunningIgnoresSync`, `Slowest` and `Fastest` in light fail `color-contrast` on "Running" and nothing else (D4).
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules (40, 6px dot via StatusDot).
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · Complex, because of StatusDot.** The row is built from StatusDot (Stage.md row 30) and passes it the run state's look; the dot is left out when stopped so "Stopped" sits at the right edge with no gap before it (Stage › Section and tempo: "the dot's space collapses"; StatusDot's table).
- **D2 · `data-run` and `data-hue` on the run state.** The tests read the state from `data-run="running|sync|stopped"` (the name the Channel compact block uses for the same three states) and the word's colour from `data-hue`, never computed colours (kit D41).
- **D3 · Width.** The row fills its container (the section column's 490); a `width` prop fixes it for the stories so the shot matches the crop's box, as CountRow D9 does.
- **D4 · L2, light `--ok` on the ground.** "Running" is 14px text in `--ok` on `--g`; light `--ok` (#1c8040) is 4.42:1. The change of `--green-700` the lane already proposes (WaitingChip D9, Stage C6) fixes it; this spec proposes no third value and lists only the contrast row.
- **D5 · Tooltip on the number only (L3).** Stage puts `display.tempo` on the number; "BPM" and the run state carry none. The readout isn't a control, so it has no `Focused` story.
- **D6 · L1, one new token.** `--tracking-32` (−0.5px) lands in `scale.css` with the tokens contract PR.
- **D7 · Not a live region.** The tempo moves with Tempo ± held and the run state with every start and stop; announcing them would talk over the player, so the row is read when the user reaches it.

Follow-ups: typed tempo entry on the readout, if players ask (Stage Follow-ups).

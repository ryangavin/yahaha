# SectionReadout

## Identity (all stations)

- **Kind:** complex
- **Built from:** StatusDot, WaitingChip, TempoReadout (and the pure functions `sectionName` from `app/src/ui/Stage/format.ts`, `nextSection` and `sectionHue` from `app/src/ui/CountRow/count.ts`)
- **Purpose:** Shows, large, the section the band is playing and the one coming next, with the tempo and run state beneath.
- **Boards:**
  - `Stage-Dark.dc.html:160-173` (the display's right column: the green dot and "Section", "Main B" 44px with its glow, "next", the "Main C" chip, then the tempo row); light: `Stage-Light.dc.html:136-149`. Spec: Stage.md › Display › Section and tempo, D4, D28, D29, D43; kit › Section names, Hue roles, Count row item 3.
- **Not this component's job:** no store, no API, no Tauri. It decides nothing about what is playing or next on its own: the count row's rules (`nextSection`, CountRow's spec) and the Genos names (`sectionName`, the Stage page spec) do, and it imports them rather than re-implementing them. Not a control: no click, no focus. It doesn't draw the dot, the chip or the tempo row itself: StatusDot, WaitingChip and TempoReadout do. No `use:tip` import (L3).

Its user: Display (Stage.md row 33) passes `{ running: transport.running, section: transport.section, queued: transport.queued, landing: transport.landing, main: transport.main, pendingIntro: transport.pendingIntro, tempo: transport.tempo, syncStart: transport.syncStart, tempoTip: 'display.tempo', tipAction }`.

Its children and the props it passes (`playing`, `next` and the hues are computed below):

| Child | Props |
|---|---|
| StatusDot (the label's dot) | `{ hue: playingHue when running, else 'main', visible: running }` (Stage D28, D43: hidden, space kept) |
| WaitingChip (the next section) | `{ label: sectionName(next), hue: sectionHue(next), size: 'display' }`, only when `next` is not null |
| TempoReadout | `{ tempo, running, syncStart, tip: tempoTip, tipAction }` |

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component. Types are local (lint forbids importing `app/src/lib` in `app/src/ui`); `SectionHue` is CountRow's (`count.ts`).

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `running` | `boolean` | `false` | `transport.running`. |
| `section` | `string \| null` | `null` | `transport.section`, SFF name ("Main B", "Fill In BA"); `null` when stopped. `''` counts as `null` (CountRow D14). |
| `queued` | `string \| null` | `null` | `transport.queued`. `''` counts as `null`. |
| `landing` | `string \| null` | `null` | `transport.landing`. `''` counts as `null`. |
| `main` | `0 \| 1 \| 2 \| 3` | `0` | `transport.main`: the Main played or returned to; stopped, the Main the band will start on (Stage D4). |
| `pendingIntro` | `0 \| 1 \| 2 \| null` | `null` | `transport.pendingIntro`: the Intro armed to play at the start. |
| `tempo` | `number` | — (required) | `transport.tempo`, passed to TempoReadout. |
| `syncStart` | `boolean` | `false` | `transport.syncStart`, passed to TempoReadout. |
| `width` | `number \| undefined` | — | Stories only: a fixed width in px (the board's 490). Without it the column fills its container (D5). |
| `tempoTip` | `string \| undefined` | `'display.tempo'` | TempoReadout's `tip` (L3). |
| `tipAction` | `Action<HTMLElement, string> \| undefined` | — | The app's `use:tip`, passed through to TempoReadout (L3). |

**What it shows** (one `$derived`, from the imports; `NBSP` ties a name to its numeral, kit › Section names):

- `playingRaw` = `running && section ? section : 'Main ' + 'ABCD'[main]` (stopped, or running with no section: the Main to start on, D3).
- `playing` = `sectionName(playingRaw)` ("Main B", "Intro II", "Break", "Fill").
- `playingHue` = running with a section: `sectionHue(section)`; otherwise `'m'` (stopped: `--m`, no glow, Stage D4).
- `next` = `nextSection({ running, section, queued, landing, pendingIntro })` (CountRow's function: a fill or the Break queued → its landing; a queued Main → it; stopped → the armed Intro; else `null`), with `''` inputs turned into `null` first.
- `nextHue` = `sectionHue(next)` from the SFF name, before `sectionName` (CountRow D16).

### Events

| Callback | Fires when | Payload |
|---|---|---|
| — | | |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Visual rules

- **Tokens used:** `--m`, `--intro`, `--main`, `--ending`, `--brk`, `--fill`, `--font-sans`, `--text-14`, `--text-44`, `--weight-light`, `--weight-regular`, `--space-4`, `--space-8`, `--space-16`, and the new tokens below. StatusDot, WaitingChip and TempoReadout bring their own.

#### New tokens

| Token | Dark | Light | Used for |
|---|---|---|---|
| `--tracking-44` | `-1.5px` | `-1.5px` | the playing section's letter-spacing (scale.css; the lane's `--tracking-<size>` naming) |
| `--text-glow-mix` | `30%` | `0%` | the playing section's 18px text glow in its hue (kit › Tokens to add; dark equals the board's `--bm` for Main, light draws nothing) (dark.css, light.css) |

They land in the orchestrator's tokens contract PR before the build (L1).

- **Element:** a root `div`, a column: `display: flex`, `flex-direction: column`, `height: 162px`, `overflow: hidden`, `font-family: var(--font-sans)`, `font-variant-numeric: tabular-nums`; `width: <width>px` when set. Three rows:
  1. **Label** (16 tall, `display: flex`, `align-items: center`, `gap: var(--space-8)`): the StatusDot, then "Section" `--text-14` / 400 `--m`, line-height 16.
  2. **Playing and next** (`margin-top: var(--space-4)`; 52 tall, `display: flex`, `align-items: center`, `gap: var(--space-16)`, `white-space: nowrap`):
     - the playing name (`data-part="playing"`, `data-hue="<playingHue>"`): `--text-44` / 300, line-height 52px, letter-spacing `--tracking-44`, `color: var(--<playingHue>)`; while running, `text-shadow: 0 0 18px color-mix(in srgb, var(--<playingHue>) var(--text-glow-mix), transparent)` (Stage D29); stopped, no `text-shadow`.
     - only when `next` is not null: "next" (`data-part="next-word"`) `--text-14` / 400 `--m`, then the WaitingChip (`display` size: 48 tall, 36px, 1px border in the hue). With no next, neither is rendered (Stage › Section and tempo: "hidden").
  3. **Tempo row** (`margin-top: 50px`): the TempoReadout, 40 tall, filling the column's width. (16 + 4 + 52 + 50 + 40 = 162.)
- **Size:** 162 tall, the column's width (490 in the display grid).
- **States drawn by:**
  - running: the dot visible in the section's hue with its glow, the name in its hue with its text glow, the next chip when there is one, TempoReadout "Running".
  - stopped: the dot `visible: false` (space kept, so nothing shifts when the band starts), the name "Main {A–D}" in `--m` with no glow, the chip only for an armed Intro (in `--intro`, what will happen), TempoReadout "Stopped" or "Sync start".
  - light theme: the same markup; `--text-glow-mix` 0% and StatusDot's `--dot-glow-mix` 0% draw no glow.
- **Type:** DM Sans, tabular numerals, the Genos names as `sectionName` returns them (no case change).
- **Test hooks:** `data-part="playing"` with `data-hue` (`main`, `intro`, `ending`, `brk`, `fill`, `m`), `data-part="next-word"`, and the children's own (`data-face="waiting"` and `data-hue` on the chip, StatusDot's, TempoReadout's `data-run`).
- **Contrast (AA, `tokens/contrast.test.ts`):** the playing name is 44px (large text, 3:1): each section hue on `--g` passes 3:1 in both themes (WaitingChip's rows, which check 4.5:1, cover it); "Section" and "next" are `--m` on `--g` (existing row). The chip's and the tempo row's pairs are their own specs'. No new failing pair.
- **Motion:** none of its own; it redraws when the transport fields change.

### Accessibility

- **Role and name:** none: the text reads in place, in order ("Section Main B next Main C 104 BPM Running"); the dot is decorative (StatusDot's `aria-hidden`). Not a live region: the count row's `role="status"` is the one that says what's playing (D4).
- **Keyboard:** none; nothing focusable (TempoReadout isn't either).
- **Tooltip id:** none on the section name (Stage gives it none); `display.tempo` on the tempo number, through TempoReadout (`tempoTip`, `tipAction`, L3).

## Stories (Story station)

Title `Components/SectionReadout`, `layout: 'centered'`. Every story renders in dark and light (the toolbar theme). The meta's `args` are `{ tempo: 104, tempoTip: 'display.tempo', tipAction: fn(), width: 490 }` (L3). The board's args are the Stage board fixture's `transport`: running true, section "Main B", queued "Fill In CC", landing "Main C", main 1, pendingIntro null, syncStart false. In the plays, compare names with `NBSP` turned into a space (`text.replace(/ /g, ' ')`).

**Controls (argTypes):** `running`, `syncStart` booleans; `section`, `queued`, `landing` text (an empty text is `null`); `main` a select of 0–3; `pendingIntro` a select with `'none'` mapped to `null` (CountRow D17); `tempo`, `width` numbers; `tempoTip` text; `tipAction` an action. The children's props are not controls: they are derived (the table above).

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ running: true, section: 'Main B', queued: 'Fill In CC', landing: 'Main C', main: 1 }` | the green dot and "Section", "Main B" 44px green with its glow (dark), "next", the "Main C" chip, then "104 BPM … Running" | `Board-{dark,light}.png` (Stage 373,169 490×162) | `[data-part="playing"]` reads "Main B" with `data-hue="main"`; "next" is shown; the chip reads "Main C" with `data-face="waiting"`, `data-hue="main"`, `data-size="display"`; the label's StatusDot has `data-hue="main"` and no inline `visibility: hidden`; the tempo row's text starts with "104 BPM" and ends with "Running" (Stage Check 5) |
| `Stopped` | `{ running: false, section: null, queued: null, landing: null, main: 1 }` | stopped on Main B: grey name, no glow, dot hidden with its space kept, no next, "Stopped" | — (no board draws it) | playing reads "Main B" with `data-hue="m"`; the StatusDot's `style.visibility` is `hidden`; no `[data-part="next-word"]` and no `[data-face="waiting"]`; the tempo row ends with "Stopped" (Stage Check 5) |
| `IntroArmed` | `{ running: false, main: 0, pendingIntro: 1 }` | stopped with Intro II armed: grey "Main A", next "Intro II" in the Intro hue (Stage D4) | — | playing "Main A", `data-hue="m"`; the chip reads "Intro II" with `data-hue="intro"` |
| `SyncStart` | `{ running: false, main: 0, syncStart: true }` | Sync Start armed: the run state "Sync start" with a hollow ring | — | the tempo row's run state has `data-run="sync"` |
| `FillPlaying` | `{ running: true, section: 'Fill In AB', queued: null, landing: 'Main B', main: 0 }` | a fill playing: "Fill" in the fill hue, next "Main B" | — | playing reads "Fill" with `data-hue="fill"`; chip "Main B", `data-hue="main"`; the dot's `data-hue="fill"` |
| `BreakPlaying` | `{ running: true, section: 'Fill In BA', queued: null, landing: 'Main A', main: 0 }` | the Break in violet, next "Main A" | — | playing "Break", `data-hue="brk"` |
| `IntroPlaying` | `{ running: true, section: 'Intro C', queued: null, landing: null, main: 0 }` | "Intro III" in the Intro hue, no next | — | playing "Intro III", `data-hue="intro"`; no chip |
| `EndingPlaying` | `{ running: true, section: 'Ending C', queued: null, landing: null }` | "Ending III", the longest name, in the Ending red | — | playing "Ending III", `data-hue="ending"` |
| `MainQueued` | `{ running: true, section: 'Main A', queued: 'Main D', landing: null, main: 0 }` | a Main queued for the bar line | — | chip "Main D" |
| `EmptyStrings` | `{ running: true, section: 'Main B', queued: '', landing: '', main: 1 }` | empty names count as none | — | no chip, no "next" |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in the dark and light render. The `Board` crop is the column's own box (L6); it holds the section dot's and the name's glows, which StatusDot's and the name's own crops can't.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- SectionReadout` passes: `Board` scores at most 0.02 against its crop (or the Inspect agent judges the difference render noise), and axe finds no violation on any story once the tokens PR has landed (until then, only the children's known pairs: WaitingChip's light `--main` and dark `--brk` chips, TempoReadout's light "Running").
- `sectionName`, `nextSection` and `sectionHue` are imported, not re-implemented.
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules (16, 52, 50, 162).
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · Names from `sectionName`, next and hue from CountRow.** Stage.md row 1b and kit › Section names make `sectionName` (Stage/format.ts) the one Genos-name function; Stage › Section and tempo says what is playing and next "follow the count row's rules", so `nextSection` and `sectionHue` are imported from `CountRow/count.ts` (built earlier, row 25) rather than written a second time.
- **D2 · State fields in, not resolved names.** The readout takes the `transport` fields as props (Stage.md › Props and callbacks) and derives what it shows, so the Display passes slices of state and the rules live in one place; the stories set them directly.
- **D3 · Running with no section.** The state never sends it, but a running band with `section` null or `''` shows the stopped face ("Main {A–D}" in `--m`, no glow) rather than an empty name.
- **D4 · Not announced.** The count row (`role="status"`, `aria-live="off"`) already summarises playing and next; this column reads in place and carries no role, so the same news isn't read twice.
- **D5 · Width.** The column fills its container (the display grid's 490); a `width` prop fixes it for the stories, as CountRow D9 and TempoReadout D3 do.
- **D6 · The stopped dot keeps its hue prop.** Stopped, the hidden dot is passed `hue: 'main'` (it isn't drawn); only `visible` matters, and the label doesn't shift when the band starts (Stage D43).
- **D7 · L1, new tokens.** `--tracking-44` (−1.5px, scale.css) and `--text-glow-mix` (dark 30%, light 0%, kit › Tokens to add) land in the orchestrator's tokens contract PR; this folder edits no token and adds no contrast row (the section-hue rows are WaitingChip's).
- **D8 · L3, tooltip passed through.** The only tooltip in the column is the tempo number's; the readout takes `tempoTip` and `tipAction` and hands them to TempoReadout. Nothing here is focusable, so there is no `Focused` story.

Follow-ups: CountRow's spec defines its own `sectionLabel` in `count.ts`, which duplicates `sectionName` in `Stage/format.ts` (kit › Section names wants one function); folding CountRow onto `sectionName` is a one-line change for whoever builds the later of the two.

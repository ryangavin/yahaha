# CountRow

## Identity (all stations)

- **Kind:** complex
- **Built from:** BeatBlocks, WaitingChip
- **Purpose:** Tells the player, in one line, where the band is in the bar, what section is playing, what comes next and when it will land.
- **Boards:**
  - `Stage-Dark.dc.html:98-113` (the count row in the section row's middle: four beat blocks, "Bar 3/4", "Main B" → "Main C" in the waiting face, "fill after bar 4"); light: `Stage-Light.dc.html:74-89`.
  - The same row sits in the section row on every board; the Stage is the one these crops come from.
- **Not this component's job:** no store, no API, no Tauri, no timer, no `requestAnimationFrame`. It doesn't keep time: the page wiring passes `now` once per animation frame and `receivedMs` (the `now` at which the state arrived), and CountRow computes the moment from them with the pure functions in `count.ts`. It sends nothing and opens nothing (a readout). It doesn't draw the blocks or the next-section chip itself: BeatBlocks and WaitingChip do. No tooltip wiring (integration).

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component. Types are declared in `count.ts` (lint forbids importing `app/src/lib` or the API types, L2); field names match `docs/app-api.md`.

```ts
/** The fields of `surface.clock` the row reads (app-api.md › surface.clock). */
export type CountClock = {
  atMs: number            // ms, the session clock when the state was read
  running: boolean
  tempo: number           // BPM (quarter notes per minute)
  beatsPerBar: number     // quarter notes per bar: 4 in 4/4, 3 in 3/4 and 6/8
  sectionAnchorMs: number // ms
  sectionAnchorBeats: number
}
export type SectionHue = 'intro' | 'main' | 'ending' | 'brk' | 'fill'
export type MainTiming = 'immediate' | 'nextBar'
export type IntroEndingTiming = 'nextBar' | 'endOfSection'
```

| Prop | Type | Default | Fed from (`AppState` path) | Meaning |
|---|---|---|---|---|
| `clock` | `CountClock` | — (required) | `surface.clock` (the six fields above; the others are ignored) | The section clock's anchors. `clock.running` is the one source of "running" for the whole row. |
| `now` | `number` | — (required) | the wiring's session-clock ms (`performance.now()` based), once per animation frame | The moment to draw. |
| `receivedMs` | `number` | — (required) | the wiring: the `now` at which this state arrived | Turns `now` into session time: `t = clock.atMs + (now − receivedMs)`. |
| `sectionBars` | `number \| null` | `null` | `transport.sectionBars` | Bars in the section playing; `null` when stopped. Hides "Bar" when `null`. |
| `section` | `string \| null` | `null` | `transport.section` | The section playing, SFF name (`Main B`, `Fill In BA`); `null` when stopped. |
| `queued` | `string \| null` | `null` | `transport.queued` | The section queued next, SFF name. |
| `landing` | `string \| null` | `null` | `transport.landing` | The Main a fill or the Break lands on. |
| `main` | `0 \| 1 \| 2 \| 3` | `0` | `transport.main` | The Main the band starts on when stopped (A–D). |
| `pendingIntro` | `0 \| 1 \| 2 \| null` | `null` | `transport.pendingIntro` | The Intro armed to play at the start (A–C). |
| `syncStart` | `boolean` | `false` | `transport.syncStart` | Sync Start is armed. |
| `mainTiming` | `MainTiming` | `'nextBar'` | `styleSettings.mainTiming` | Section Change Timing, To Main. |
| `introEndingTiming` | `IntroEndingTiming` | `'nextBar'` | `styleSettings.introEndingTiming` | Section Change Timing, Inside Intro/Ending. |
| `width` | `number \| undefined` | — | the stories only (SectionRow passes nothing) | A fixed width in px (the board's 894). Without it the row fills its container: `flex: 1 1 0; min-width: 0` (D9). |

Not props: `transport.bar`, `transport.beat`, `transport.beatsPerBar` (D2, D3).

### Events

| Callback | Fires when | Payload |
|---|---|---|
| — | | |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Pure functions (`app/src/ui/CountRow/count.ts`)

Everything the row shows is computed by these; the component only renders `countView(...)` and sets `aria-label` to `countLabel(...)`. Tests: `app/src/ui/CountRow/count.test.ts` (vitest), one `it` per table row below. `NBSP` is U+00A0.

```ts
export function sectionPosition(clock: CountClock, now: number, receivedMs: number): number
export function currentBeat(clock: CountClock, now: number, receivedMs: number): number | null
export function currentBar(clock: CountClock, now: number, receivedMs: number, sectionBars: number | null): number | null
export function sectionLabel(name: string): string
export function sectionHue(name: string): SectionHue
export function nextSection(n: NextInput): string | null
export function countWhen(w: WhenInput): CountWhen | null
export function countView(p: CountInput): CountView
export function countLabel(v: CountView): string

export type NextInput = { running: boolean; section: string | null; queued: string | null; landing: string | null; pendingIntro: 0 | 1 | 2 | null }
export type WhenInput = { running: boolean; section: string | null; queued: string | null; bar: number | null; sectionBars: number | null; syncStart: boolean; mainTiming: MainTiming; introEndingTiming: IntroEndingTiming }
export type CountWhen = { text: string; sentence: string }
export type CountInput = { clock: CountClock; now: number; receivedMs: number; sectionBars: number | null; section: string | null; queued: string | null; landing: string | null; main: 0 | 1 | 2 | 3; pendingIntro: 0 | 1 | 2 | null; syncStart: boolean; mainTiming: MainTiming; introEndingTiming: IntroEndingTiming }
export type CountView = {
  running: boolean
  count: number               // blocks: clock.beatsPerBar
  beat: number | null         // currentBeat
  hue: SectionHue             // the blocks' hue: sectionHue(section), 'main' when section is null
  bar: number | null          // currentBar; null hides "Bar" (stopped, or sectionBars null)
  sectionBars: number | null
  playing: string | null      // sectionLabel of the playing (or stopped-on) name; null hides the sections item
  playingHue: SectionHue | 'm'// 'm' when stopped
  next: string | null         // sectionLabel of nextSection; null hides the arrow and the chip
  nextHue: SectionHue | null
  when: CountWhen | null      // null hides the When item
}
```

**`sectionPosition`** is app-api.md › surface.clock: `t = clock.atMs + (now − receivedMs)`; `pos = clock.running ? max(0, sectionAnchorBeats + (t − sectionAnchorMs) · tempo / 60000) : 0`. **`currentBeat`** is `floor(pos) mod beatsPerBar + 1`, or `null` when `!clock.running`. **`currentBar`** is `floor(pos / beatsPerBar) + 1`, capped at `sectionBars` when that isn't null (D4), or `null` when `!clock.running`.

The clock `B` below is the board's: `{ atMs: 10000, running: true, tempo: 104, beatsPerBar: 4, sectionAnchorMs: 10000, sectionAnchorBeats: 10.25 }`. At 104 BPM a beat is 576.9 ms.

| Function and input | Output |
|---|---|
| `currentBeat(B, 10000, 10000)` (the board moment; pos 10.25) | `3` |
| `currentBar(B, 10000, 10000, 4)` | `3` |
| `currentBeat(B, 10577, 10000)` (one beat later, 577 ms; pos 11.2501) | `4` |
| `currentBar(B, 10577, 10000, 4)` | `3` |
| `currentBeat(B, 11154, 10000)` / `currentBar(…, 4)` (two beats later; pos 12.2503) | `1` / `4` |
| `currentBeat(B, 5577, 5000)` (state received at 5000: t = 10577) | `4` |
| `currentBeat({ …B, running: false }, 10000, 10000)` / `currentBar(…, 4)` (stopped) | `null` / `null` |
| `sectionPosition({ …B, sectionAnchorBeats: 0 }, 9990, 10000)` (t before the anchor) / `currentBeat` of it | `0` (clamped) / `1` |
| `currentBeat({ …B, beatsPerBar: 3 }, 10000, 10000)` / `currentBar(…, 4)` (3/4, and 6/8) | `2` / `4` |
| `currentBar({ …B, sectionAnchorBeats: 16.5 }, 10000, 10000, 4)` (pos run past the section's end before the re-anchor; uncapped 5) | `4` |
| `currentBar({ …B, sectionAnchorBeats: 16.5 }, 10000, 10000, null)` | `5` |
| `sectionLabel('Main B')` | `'Main' + NBSP + 'B'` |
| `sectionLabel('Main A')`, `'Main D'` | `'Main' + NBSP + 'A'`, `'Main' + NBSP + 'D'` |
| `sectionLabel('Intro A')`, `'Intro B'`, `'Intro C'`, `'Intro D'` | `'Intro' + NBSP + 'I'`, `…'II'`, `…'III'`, `…'IV'` |
| `sectionLabel('Ending A')`, `'Ending B'`, `'Ending C'`, `'Ending D'` | `'Ending' + NBSP + 'I'`, `…'II'`, `…'III'`, `…'IV'` |
| `sectionLabel('Fill In BA')` | `'Break'` |
| `sectionLabel('Fill In AA')`, `'Fill In CC'`, `'Fill In AB'`, `'Fill In DA'` | `'Fill'` |
| `sectionLabel('Intro E')`, `'Outro'`, `''` (match none: shown as given) | `'Intro E'`, `'Outro'`, `''` |
| `sectionHue('Intro A')`, `'Intro E'` | `'intro'` |
| `sectionHue('Main B')` | `'main'` |
| `sectionHue('Ending C')` | `'ending'` |
| `sectionHue('Fill In BA')` | `'brk'` |
| `sectionHue('Fill In CC')` | `'fill'` |
| `sectionHue('Outro')` (no known first word, D7) | `'fill'` |
| `nextSection({ running: true, section: 'Main B', queued: 'Fill In CC', landing: 'Main C', pendingIntro: null })` (board: a fill queued) | `'Main C'` |
| `nextSection({ running: true, section: 'Fill In AB', queued: null, landing: 'Main B', … })` (a fill playing) | `'Main B'` |
| `nextSection({ running: true, section: 'Main A', queued: 'Fill In BA', landing: 'Main A', … })` (the Break queued) | `'Main A'` |
| `nextSection({ running: true, section: 'Fill In AB', queued: null, landing: null, … })` (fill, landing missing) | `null` |
| `nextSection({ running: true, section: 'Main A', queued: 'Main B', landing: null, … })` | `'Main B'` |
| `nextSection({ running: true, section: 'Main B', queued: null, landing: null, … })` | `null` |
| `nextSection({ running: false, …, pendingIntro: 0 })` | `'Intro A'` |
| `nextSection({ running: false, …, pendingIntro: null })` | `null` |
| `countWhen` W = `{ running: true, section: 'Main B', queued: 'Fill In CC', bar: 3, sectionBars: 4, syncStart: false, mainTiming: 'nextBar', introEndingTiming: 'nextBar' }` (board: fill queued) | `{ text: 'fill after bar 3', sentence: 'The fill lands after bar 3.' }` |
| `countWhen({ …W, section: 'Fill In AB', queued: null, bar: 1, sectionBars: 1 })` (fill playing) | `{ text: 'fill after bar 1', sentence: 'The fill lands after bar 1.' }` |
| `countWhen({ …W, section: 'Main A', queued: 'Fill In BA', bar: 2 })` (Break queued) | `{ text: 'break after bar 2', sentence: 'The break lands after bar 2.' }` |
| `countWhen({ …W, section: 'Fill In BA', queued: null, bar: 1, sectionBars: 1 })` (Break playing) | `{ text: 'break after bar 1', sentence: 'The break lands after bar 1.' }` |
| `countWhen({ …W, section: 'Intro B', queued: 'Ending B', bar: 2, sectionBars: 6, introEndingTiming: 'endOfSection' })` | `{ text: 'after bar 6', sentence: 'The change lands after bar 6.' }` |
| `countWhen({ …W, section: 'Ending A', queued: 'Ending C', bar: 1, sectionBars: 4, introEndingTiming: 'endOfSection' })` | `{ text: 'after bar 4', sentence: 'The change lands after bar 4.' }` |
| `countWhen({ …W, section: 'Intro A', queued: 'Intro B', bar: 2, sectionBars: 6, introEndingTiming: 'endOfSection' })` (Intro to Intro is always next bar) | `{ text: 'after bar 2', sentence: 'The change lands after bar 2.' }` |
| `countWhen({ …W, section: 'Intro A', queued: 'Ending A', bar: 2, sectionBars: 6, introEndingTiming: 'endOfSection' })` (into Ending I waits for the next bar line) | `{ text: 'after bar 2', … }` |
| `countWhen({ …W, section: 'Intro B', queued: 'Ending B', bar: 2, sectionBars: null, introEndingTiming: 'endOfSection' })` (no length known) | `{ text: 'after bar 2', … }` |
| `countWhen({ …W, section: 'Intro B', queued: 'Ending B', bar: 2, sectionBars: 6 })` (`nextBar`) | `{ text: 'after bar 2', … }` |
| `countWhen({ …W, section: 'Main A', queued: 'Main B', bar: 3, sectionBars: 8, mainTiming: 'immediate' })` | `{ text: 'next beat', sentence: 'The change lands on the next beat.' }` |
| `countWhen({ …W, section: 'Intro A', queued: 'Main A', bar: 2, mainTiming: 'immediate' })` (To Main applies from an Intro too) | `{ text: 'next beat', … }` |
| `countWhen({ …W, section: 'Main A', queued: 'Main B', bar: 3, sectionBars: 8 })` (`nextBar`) | `{ text: 'after bar 3', sentence: 'The change lands after bar 3.' }` |
| `countWhen({ …W, section: 'Main A', queued: 'Ending A', bar: 7, sectionBars: 8 })` | `{ text: 'after bar 7', … }` |
| `countWhen({ …W, section: 'Main B', queued: null })` (nothing queued) | `null` |
| `countWhen({ …W, running: false, section: null, queued: null, bar: null, sectionBars: null, syncStart: true })` | `{ text: 'sync start', sentence: 'Sync start armed.' }` |
| `countWhen({ …W, running: false, section: null, queued: null, bar: null, sectionBars: null, syncStart: false })` | `null` |
| `countLabel(countView(board))` (the Board story's args; Stage.md Check 4) | `'Beat 3 of 4, bar 3 of 4. Main B playing, Main C next. The fill lands after bar 3.'` |
| `countLabel` of the board one beat later (`now: 10577`) | `'Beat 4 of 4, bar 3 of 4. Main B playing, Main C next. The fill lands after bar 3.'` |
| `countLabel` of `NoNext` | `'Beat 3 of 4, bar 3 of 4. Main B playing.'` |
| `countLabel` of `Stopped` | `'Stopped on Main C.'` |
| `countLabel` of `IntroArmed` | `'Stopped on Main C, Intro I next.'` |
| `countLabel` of `SyncStartArmed` | `'Stopped on Main C. Sync start armed.'` |
| `countLabel` of `IntroArmed` with `syncStart: true` | `'Stopped on Main C, Intro I next. Sync start armed.'` |
| `countLabel` of `MainImmediate` | `'Beat 2 of 4, bar 3 of 8. Main A playing, Main B next. The change lands on the next beat.'` |
| `countLabel` of a running view with `sectionBars: null` (beat 3, Main B, nothing queued) | `'Beat 3 of 4. Main B playing.'` |

The rules behind the table:

- **`sectionLabel`** (kit › Section names): `Intro X` / `Ending X` with X in A–D → the word, NBSP, `I` `II` `III` `IV`; `Main X` with X in A–D → `Main`, NBSP, X; `Fill In BA` → `Break`; any other `Fill In XY` (X, Y in A–D) → `Fill`; anything else unchanged.
- **`sectionHue`** (kit › Hue roles), by first word: `Intro` → `intro`, `Main` → `main`, `Ending` → `ending`; `Fill In BA` → `brk`; other `Fill In …` → `fill`; anything else → `fill` (D7).
- **`nextSection`**: stopped → `Intro {A+pendingIntro}` or `null`. Running: when the queued or the playing section is a fill or the Break (a name starting `Fill In `) → `landing` (may be `null`); otherwise → `queued`.
- **`countWhen`**, the first that applies (D5, D6). A *fill* is a `Fill In` name other than `Fill In BA`; the *Break* is `Fill In BA`.
  1. Stopped: `syncStart` → "sync start" / "Sync start armed."; else `null`.
  2. A fill queued or playing → "fill after bar {bar}" / "The fill lands after bar {bar}."
  3. The Break queued or playing → "break after bar {bar}" / "The break lands after bar {bar}."
  4. `queued` is an Intro or Ending, `section` is an Intro or Ending, `introEndingTiming` is `endOfSection`, `sectionBars` isn't null, and it is neither Intro → Intro nor into `Ending A` → "after bar {sectionBars}" / "The change lands after bar {sectionBars}."
  5. `queued` is a Main and `mainTiming` is `immediate` → "next beat" / "The change lands on the next beat."
  6. Anything else queued → "after bar {bar}" / "The change lands after bar {bar}."
  7. Nothing queued → `null`.

  `{bar}` is `currentBar(...)`, the same number the Bar item shows.
- **`countView`**: running → `playing = section ? sectionLabel(section) : null`, `playingHue = sectionHue(section)`; stopped → `playing = sectionLabel('Main ' + 'ABCD'[main])`, `playingHue = 'm'`. `next = nextSection(...)` through `sectionLabel`, `nextHue = sectionHue(next)`. `hue = sectionHue(section ?? 'Main A')`. `bar = sectionBars === null ? null : currentBar(...)`.
- **`countLabel`**: up to three sentences joined by one space, every NBSP replaced by a plain space (D8):
  1. Running and `beat` not null: "Beat {beat} of {count}, bar {bar} of {sectionBars}." or, with `bar` null, "Beat {beat} of {count}."
  2. Running with `playing`: "{playing} playing, {next} next." or, no next, "{playing} playing."; running without `playing`: no sentence. Stopped: "Stopped on {playing}, {next} next." or "Stopped on {playing}."
  3. `when.sentence`, when there is one.

### Children

Passed down explicitly from `countView(...)` of the row's props. CountRow never restates a child's look.

| Where | Child | Props |
|---|---|---|
| item 1 | `BeatBlocks` | `{ count: view.count, beat: view.beat, hue: view.hue }` (BeatBlocks draws the stopped look itself when `beat` is `null`) |
| item 3, after the arrow | `WaitingChip` | `{ label: view.next, hue: view.nextHue, size: 'count' }`, rendered only when `view.next` isn't null (its look at `size: 'count'` is WaitingChip's spec; it carries `data-face="waiting"`, `data-hue`, `data-size="count"`) |

### Visual rules

- **Tokens used:** `--t`, `--m`, `--intro`, `--main`, `--ending`, `--brk`, `--fill`, `--font-sans`, `--text-18`, `--weight-light`, `--weight-regular`, `--space-8`, `--space-20`, `--space-24`, `--control-height`. (BeatBlocks and WaitingChip bring their own.)
- **Layout:** the root is a flex row, `height: var(--control-height)` (32px), `align-items: center`, `justify-content: safe center` (centred; if it ever overflows, it overflows on the right), `gap: var(--space-20)` (20px) between items, `white-space: nowrap`, `overflow: hidden`, `min-width: 0`. Width: `width` px when given, else `flex: 1 1 0` (fills the section row's middle). No padding, border, background (the ground shows through) or shadow.
- **Items, left to right**, each rendered only when it has something to say (not rendered at all, not `display: none`):
  1. **Beat blocks**, always: `BeatBlocks` (Children). 108 × 24 in 4/4.
  2. **Bar**, when `view.bar` isn't null: "Bar" in `--m`, `--weight-regular` (400), a plain space, then "{bar}/{sectionBars}" (e.g. "3/4", "10/12") in `--t`, `--weight-light` (300). No space around the slash.
  3. **Sections**, when `view.playing` isn't null: a flex row, `align-items: center`, `gap: var(--space-8)` (8px): the playing name, `--weight-light`, colour `var(--<playingHue>)` (stopped: `--m`), no glow and no box; then, when `view.next` isn't null, "→" (U+2192) in `--m`, `--weight-light`, then the `WaitingChip`. With no next, the arrow and the chip are both absent.
  4. **When**, when `view.when` isn't null: `view.when.text` in `--t`, `--weight-light`.
- **Type:** DM Sans (`--font-sans`), `--text-18` (18px), `line-height: var(--space-24)` (24px), weight `--weight-light` (300) unless an item says 400, tabular numerals (`font-variant-numeric: tabular-nums`), sentence case exactly as the text above (lower-case "fill after bar 3", "next beat", "sync start"; "Bar" capitalised). Section names keep their NBSP, so "Intro I" never breaks (and the row never wraps anyway).
- **States** (each a story):
  - **Playing** (`clock.running`): blocks with the current beat lit, Bar, playing name in its hue, optional arrow and chip, optional When.
  - **Stopped** (`!clock.running`): blocks all later (BeatBlocks' stopped look), no Bar, "Main {A–D}" from `main` in `--m`; the armed Intro as the chip (in `--intro`) after the arrow when `pendingIntro` isn't null; "sync start" when `syncStart`.
  - **No next:** no arrow, no chip.
  - **Nothing queued:** no When.
  - **Unknown name:** shown as given (`sectionLabel`), hue `fill`. An empty `section` (`''`) counts as `null`: no sections item while running (the playing name, arrow and chip are all absent; the label drops its second sentence).
  - No hover, press or focus look (not a control); `cursor: default`.
- **Test hooks (D41):** the playing name carries `data-item="playing"` and `data-hue="<playingHue>"` (`main`, …, or `m` stopped); the arrow `data-item="arrow"`; the Bar item `data-item="bar"`; the When item `data-item="when"` and `data-shot-mask="when"` (Stage.md D39); the root `data-running="true|false"`. The blocks carry BeatBlocks' `data-beat`, `data-downbeat`, `data-hue`; the chip carries WaitingChip's `data-face="waiting"` and `data-hue`. Colours are checked by the stories' screenshots, never computed in vitest.
- **Contrast (AA 4.5:1, rows to add to `tokens/contrast.test.ts`):** `--m` on `--g` and `--t` on `--g` (exist); each section hue as text on the ground: `--intro`, `--main`, `--ending`, `--brk`, `--fill` on `--g` (18px at weight 300 is not large text, so 4.5:1 applies). Today three of those fail and need a token change before `npm run shots -- CountRow`'s axe pass and those rows can be green (D10): light `--main` (#1c8040 on #f2f1ee, 4.42), light `--intro` (#857a1f, 3.87), dark `--brk` (#8f62a8 on #000, 4.48). The rest pass (dark `--intro` 9.69, `--main` 11.16, `--ending` 4.96, `--fill` 7.27; light `--ending` 5.20, `--brk` 5.73, `--fill` 4.51).
- **Motion:** none of its own. The blocks move and the Bar, label and When update when `now` changes (the wiring passes it once per animation frame); every change is instant (no transition) and comes from props (axiom 10).

### Accessibility

- **Role and name:** the root is `role="status"` with `aria-label={countLabel(view)}` and `aria-live="off"` (D11). Every item inside is `aria-hidden="true"` (BeatBlocks hides itself; the Bar, Sections and When spans each carry `aria-hidden="true"`), so the label is the only thing read.
- **Keyboard:** not focusable; no keys. No `Focused` story.
- **Tooltip id:** `display.position` (exists in `app/src/help/tooltips.ts`) on the root element; wired at integration with `use:tip`, not here. Its body still describes today's position readout (a light per beat, ⤷) and should be rewritten with C5 (Stage.md › Contract changes).
- **Launchkey:** none (a readout; the Launchkey shows the sections on its pads).

## Stories (Story station)

Title `Components/CountRow`, `layout: 'centered'`. Every story passes `width: 894` (the board's box) so it draws as in the section row. Every story renders in dark and light (the toolbar theme). Fixtures live in `app/src/ui/CountRow/CountRow.fixtures.ts` (axiom 12): `boardClock` = `{ atMs: 10000, running: true, tempo: 104, beatsPerBar: 4, sectionAnchorMs: 10000, sectionAnchorBeats: 10.25 }`, `stoppedClock` = `{ ...boardClock, running: false, sectionAnchorBeats: 0 }`, and `boardCount` = the `Board` args (copied from Stage.md › Board fixture: `surface.clock`, `transport`, `styleSettings`, D12). Below, *running(x)* is `{ clock: { ...boardClock, sectionAnchorBeats: x }, now: 10000, receivedMs: 10000 }` and *stopped* is `{ clock: stoppedClock, now: 10000, receivedMs: 10000, sectionBars: null, section: null, queued: null, landing: null, main: 2 }`; props not named take their defaults.

Controls: every own prop (`clock` and the others as object, number, text, boolean and select controls). No child argTypes: BeatBlocks' and WaitingChip's props are all derived from the row's props, never set by a story (axiom 3). No callbacks, so no actions.

Every Play below also checks: the root has `role="status"` and the exact `aria-label` given; the text checks use testing-library's default normaliser, which reads an NBSP as a space ("Main B").

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `boardCount` = running(10.25) + `{ sectionBars: 4, section: 'Main B', queued: 'Fill In CC', landing: 'Main C', main: 2, pendingIntro: null, syncStart: false, mainTiming: 'nextBar', introEndingTiming: 'nextBar', width: 894 }`; `parameters.shots = { mask: ['[data-shot-mask="when"]'] }` | the board moment: blocks 1–2 past, 3 current in Main green, 4 later; "Bar 3/4"; "Main B" green → "Main C" chip; "fill after bar 3" | `Board-{dark,light}.png` (Stage 175,68 894×32), When masked (D1) | four `[data-beat]`: `past`, `past`, `current`, `later`; block 1 `data-downbeat`; block 3 `data-hue="main"`; `[data-item="bar"]` text "Bar 3/4"; `[data-item="playing"]` text "Main B", `data-hue="main"`; `[data-item="arrow"]` "→"; one `[data-face="waiting"]` with text "Main C" and `data-hue="main"`; `[data-item="when"]` text "fill after bar 3" and `data-shot-mask="when"`; `aria-label` "Beat 3 of 4, bar 3 of 4. Main B playing, Main C next. The fill lands after bar 3." |
| `BoardNextBeat` | `boardCount` with `now: 10577` | one beat later (577 ms at 104 BPM): block 4 current | — (not drawn) | `[data-beat]`: `past`, `past`, `past`, `current`; block 4 `data-hue="main"`; "Bar 3/4"; `aria-label` "Beat 4 of 4, bar 3 of 4. Main B playing, Main C next. The fill lands after bar 3." |
| `Stopped` | *stopped* | blocks all `--btn` (block 1 edged), no Bar, "Main C" in `--m`, nothing else | — (the board is drawn playing) | every `[data-beat]` is `later`; no `[data-hue]` inside the blocks; no `[data-item="bar"]`, `[data-item="arrow"]`, `[data-face="waiting"]` or `[data-item="when"]`; `[data-item="playing"]` "Main C" with `data-hue="m"`; root `data-running="false"`; `aria-label` "Stopped on Main C." |
| `SyncStartArmed` | *stopped* + `{ syncStart: true }` | as Stopped, then "sync start" in `--t` | — | `[data-item="when"]` text "sync start"; no chip; `aria-label` "Stopped on Main C. Sync start armed." |
| `IntroArmed` | *stopped* + `{ pendingIntro: 0 }` | "Main C" `--m` → "Intro I" chip in `--intro` | — | `[data-item="arrow"]` present; `[data-face="waiting"]` text "Intro I", `data-hue="intro"`; no When; `aria-label` "Stopped on Main C, Intro I next." |
| `FillQueued` | running(23.5) + `{ sectionBars: 8, section: 'Main A', queued: 'Fill In AA', landing: 'Main A', main: 0 }` | Fill Self on Main A: beat 4 current, "Bar 6/8", "Main A" → "Main A" chip, "fill after bar 6" | — | block 4 `current`; "Bar 6/8"; chip "Main A"; When "fill after bar 6"; `aria-label` "Beat 4 of 4, bar 6 of 8. Main A playing, Main A next. The fill lands after bar 6." |
| `FillPlaying` | running(1.5) + `{ sectionBars: 1, section: 'Fill In AB', queued: null, landing: 'Main B', main: 1 }` | "Fill" in `--fill` (current block too) → "Main B" chip in `--main`; "Bar 1/1"; "fill after bar 1" | — | block 2 `current`, `data-hue="fill"`; `[data-item="playing"]` "Fill", `data-hue="fill"`; chip "Main B" `data-hue="main"`; `aria-label` "Beat 2 of 4, bar 1 of 1. Fill playing, Main B next. The fill lands after bar 1." |
| `Break` | running(2.5) + `{ sectionBars: 1, section: 'Fill In BA', queued: null, landing: 'Main C', main: 2 }` | "Break" in `--brk` (current block too) → "Main C" chip; "break after bar 1" | — | block 3 `data-hue="brk"`; playing "Break" `data-hue="brk"`; chip "Main C"; When "break after bar 1"; `aria-label` "Beat 3 of 4, bar 1 of 1. Break playing, Main C next. The break lands after bar 1." |
| `IntroEnding` | running(5) + `{ sectionBars: 6, section: 'Intro B', queued: 'Ending B', main: 0, introEndingTiming: 'endOfSection' }` | "Intro II" in `--intro` → "Ending II" chip in `--ending`; "Bar 2/6"; "after bar 6" | — | block 2 `data-hue="intro"`; playing "Intro II" `data-hue="intro"`; chip "Ending II" `data-hue="ending"`; When "after bar 6"; `aria-label` "Beat 2 of 4, bar 2 of 6. Intro II playing, Ending II next. The change lands after bar 6." |
| `EndingPlaying` | running(6) + `{ sectionBars: 4, section: 'Ending C', main: 0 }` | "Ending III" in `--ending` (current block too), no next, no When | — | block 3 `data-hue="ending"`; playing "Ending III" `data-hue="ending"`; no arrow, chip or When; `aria-label` "Beat 3 of 4, bar 2 of 4. Ending III playing." |
| `MainImmediate` | running(9.5) + `{ sectionBars: 8, section: 'Main A', queued: 'Main B', main: 1, mainTiming: 'immediate' }` | "Main A" → "Main B" chip; "next beat" | — | When "next beat"; `aria-label` "Beat 2 of 4, bar 3 of 8. Main A playing, Main B next. The change lands on the next beat." |
| `MainNextBar` | as `MainImmediate` with `mainTiming: 'nextBar'` | the same, "after bar 3" | — | When "after bar 3"; `aria-label` ends "The change lands after bar 3." |
| `NoNext` | running(10.25) + `{ sectionBars: 4, section: 'Main B', main: 1 }` | blocks, "Bar 3/4", "Main B"; no arrow, chip or When | — | no `[data-item="arrow"]`, `[data-face="waiting"]` or `[data-item="when"]`; `aria-label` "Beat 3 of 4, bar 3 of 4. Main B playing." |
| `ThreeFour` | `{ clock: { ...boardClock, beatsPerBar: 3 }, now: 10000, receivedMs: 10000, sectionBars: 4, section: 'Main A', main: 0 }` | 3/4 (and 6/8, D2): three blocks, beat 2 current, "Bar 4/4", "Main A" | — | three `[data-beat]`: `past`, `current`, `later`; "Bar 4/4"; `aria-label` "Beat 2 of 3, bar 4 of 4. Main A playing." |
| `LongName` | running(38.5) + `{ sectionBars: 12, section: 'Intro D', queued: 'Ending D', main: 0, introEndingTiming: 'endOfSection' }` | the widest real row: "Bar 10/12", "Intro IV" → "Ending IV", "after bar 12", on one line inside 894px | — (judged by Inspect) | block 3 `current`; "Bar 10/12"; playing "Intro IV"; chip "Ending IV"; When "after bar 12"; `aria-label` "Beat 3 of 4, bar 10 of 12. Intro IV playing, Ending IV next. The change lands after bar 12." |

Section hues covered: `main` (Board), `intro` (IntroEnding, IntroArmed), `ending` (EndingPlaying, IntroEnding's chip), `brk` (Break), `fill` (FillPlaying), `m` (Stopped).

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in the dark and light render. Only `Board` has a crop: the Stage board draws only that moment. The crop is the count row's whole box (it includes the current block's glow, which spills about 2px around block 3).

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`); `npx vitest run src/ui/CountRow/count.test.ts` passes every row of the Pure functions table.
- `npm run shots -- CountRow` passes: the `Board` screenshot is 894 × 32 and scores at most 0.02 against `crops/Board-{dark,light}.png` with the When item masked (or unmasked, D1), or the Inspect agent judges the difference render noise; axe finds no violation on any story (after the D10 token change).
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · Board text differs from the crop (Stage.md D5, L3).** The `Board` story uses the fixture's state, so the When item reads "fill after bar 3" where the png reads "fill after bar 4"; the item carries `data-shot-mask="when"` and the story sets `parameters.shots = { mask: ['[data-shot-mask="when"]'] }` (Stage.md D39). Until `shots.ts` supports masks, the one-glyph difference (about 100 differing pixels of 28 608) is far under the 0.02 score (572), so the crop check passes unmasked.
- **D2 · Beats are quarter notes.** The blocks and "Beat n of N" use `surface.clock.beatsPerBar` (quarter notes: 3 in 6/8), never `transport.beatsPerBar` (the numerator: 6 in 6/8), because the section position counts quarter notes; 6/8 draws three blocks, as 3/4 does.
- **D3 · Bar from the clock.** The Bar item and every "{bar}" in the When text come from `currentBar` (the same `pos` as the blocks), not `transport.bar`, because the state is republished on change rather than as time passes, so `transport.bar` can lag the blocks by a bar or more (kit › Count row names `transport.bar`; this corrects it).
- **D4 · Bar capped at the section's length.** Between a section's end and the engine's re-anchor, `pos` can run past the end (app-api.md › surface.clock), so `currentBar` is capped at `sectionBars` and never reads "Bar 5/4".
- **D5 · Fill before Break.** "A fill" means a `Fill In` name other than `Fill In BA`, and the Break is `Fill In BA` (kit › Hue roles), so the When rules' fill and Break cases never overlap; a Break queued during a fill reads as the fill.
- **D6 · End-of-section rule follows the engine.** "after bar {sectionBars}" applies only when an Intro or Ending is playing and another Intro or Ending is queued under `endOfSection`, except Intro → Intro and into Ending I, which wait for the next bar line (app-api.md › `setIntroEndingTiming`); the kit's "an Intro or Ending queued with endOfSection" would also have matched a Main → Ending change, which waits for the next bar line.
- **D7 · Unknown section hue.** A name with no known first word (the state never sends one; a defensive case) takes `fill`, the quietest section hue, so it never reads as Main's green or as trouble red.
- **D8 · Plain spaces in the label.** The visible names keep their NBSP (kit › Section names); `countLabel` uses plain spaces, so the `aria-label` matches Stage.md Check 4 character for character.
- **D9 · Width.** The row fills its container (`flex: 1 1 0; min-width: 0`) in the section row; a `width` prop (LampButton's convention) fixes it for the stories, which all pass the board's 894 so the shot matches the crop's box. It never wraps; if it ever overflowed it would clip on the right (`justify-content: safe center`, `overflow: hidden`), but the widest real row (`LongName`) is about 560px.
- **D10 · Section hue contrast.** Section names are 18px light text in their hue on the ground, so each hue needs 4.5:1; light `--main` (4.42), light `--intro` (3.87) and dark `--brk` (4.48) fall short today. The fix is a token change in `app/src/ui/tokens/` (darker light `--main`/`--intro` palette steps, a lighter dark `--brk`), outside this component; until it lands those `contrast.test.ts` rows and axe on the stories that draw those hues fail.
- **D11 · Not announced every beat.** The root is `role="status"` (kit) with `aria-live="off"`, so the per-frame label changes are never spoken; a screen reader reads the label when the user moves to it. Announcing section changes is a follow-up.
- **D12 · Fixture home.** The row's board args live in `CountRow.fixtures.ts` (copied from Stage.md › Board fixture) because the Stage fixture (`app/src/ui/Stage/Stage.fixtures.ts`) is built last; the Stage fixture may import them.
- **D13 · Stopped hue.** Stopped, the playing name is "Main {A–D}" from `transport.main` in `--m` (`data-hue="m"`, Stage.md D4) and the blocks take `main` as their (unused) hue; the armed Intro's chip keeps its own `--intro`, since it is what will happen.

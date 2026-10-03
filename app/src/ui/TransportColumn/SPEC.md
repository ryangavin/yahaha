# TransportColumn

## Identity (all stations)

- **Kind:** complex
- **Built from:** GroupHeader, Button (specs on `spec/ui-primitives`: `app/src/ui/GroupHeader/SPEC.md`, `app/src/ui/Button/SPEC.md`)
- **Purpose:** The band's transport and tempo buttons: start and stop the band, reset or fade it, call a fill, and nudge or reset the tempo, the same buttons the Launchkey has.
- **Boards:** `Stage-Dark.dc.html:360-388`; light: `Stage-Light.dc.html:336-364`. Crop box: Stage `1328,432 88×368`.
- **Not this component's job:** no store, no API, no Tauri: `running` and `fade` come in as props and every press goes out as a callback. It draws no face, bar or glyph of its own: every button is a `Button` with props (Button draws the faces, the running bar, the ▲ ▼ + − glyphs, the hold mode and pointer capture). No tempo repeat timer: Tempo + and − report down and up through Button's `hold` / `onhold`, and the page wiring repeats (`app/src/lib/tempoHold.ts`, which also sends `resetTempo` when both are held). No tooltip wiring (integration adds `use:tip`). No Shift layer (Shift + Play and Shift + Stop are the Launchkey's way to Reset and Fade; the screen has its own buttons for them).

## API (Component station)

### Props

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `running` | `boolean` | `false` | The band is playing: Start / Stop shows its green bar, is `aria-pressed="true"` and its name says "running". |
| `fade` | `'off' \| 'armed' \| 'fadingIn' \| 'fadingOut' \| 'holding'` | `'off'` | The fade state (`transport.fade`): `armed` draws Fade in Button's waiting face, `fadingIn`, `fadingOut` and `holding` in its on face. |

### Events

| Callback | Fires when | Payload | The wiring sends |
|---|---|---|---|
| `onstartstop` | Start / Stop pressed (click, Space or Enter) | `()` | `startStop` |
| `onstop` | Stop pressed | `()` | `stop` |
| `onsectionreset` | Reset pressed | `()` | `sectionReset` |
| `onfade` | Fade pressed | `()` | `toggleFade` |
| `onfillup` | Fill ▲ pressed | `()` | `fillUp` |
| `onfilldown` | Fill ▼ pressed | `()` | `fillDown` |
| `ontempo` | Tempo + or − goes down or up under the pointer: Button's `onhold(down)` | `(dir: 1 \| -1, down: boolean)`: `dir` 1 for Tempo +, −1 for Tempo − | `tempoHold.set(dir, down)`: `tempoUp` / `tempoDown` at once and repeating while held; both held: `resetTempo` |
| `ontempostep` | Tempo + or − pressed from the keyboard (Space or Enter): Button's `onpress`, which with `hold` fires only for a keyboard click | `(dir: 1 \| -1)` | `tempoUp` (1) or `tempoDown` (−1), one step |
| `onstyletempo` | Style tempo pressed | `()` | `resetTempo` |

Every callback is optional; its default does nothing (so `boardTransport` can hold data only).

**Tempo ± (Button `hold`).** Each tempo Button gets `hold: true`, `onhold: (down) => ontempo(dir, down)` and `onpress: () => ontempostep(dir)`. What Button gives (its spec, Events and D11): a primary-button `pointerdown` calls `onhold(true)` and takes pointer capture; the `pointerup`, `pointercancel` or `lostpointercapture` that ends it calls `onhold(false)` once; the pointer's `click` then calls no `onpress`; no long press. From the keyboard, Space or Enter is a click with `detail === 0`, so `onpress` fires: one tempo step per key press. Space clicks once on keyup; a held Enter repeats at the system key-repeat rate, each repeat one more step. The keyboard has no hold-to-accelerate and no both-held reset; Style tempo is the keyboard's way to reset. This is today's `HwButton` behaviour in `app/src/panels/header/TransportBar.svelte` (`onclick` sends one step, `onhold` drives `tempoHold`), so `tempoHold.ts` is unchanged. The window losing focus mid-hold is the wiring's: it calls `tempoHold.releaseAll()` on the window's `blur`.

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### From the state

What the page wiring passes (the Stage lane transcribes this):

| Prop / callback | From / sends |
|---|---|
| `running` | `transport.running` |
| `fade` | `transport.fade` |
| `onstartstop` | `startStop` |
| `onstop` | `stop` |
| `onsectionreset` | `sectionReset` |
| `onfade` | `toggleFade` |
| `onfillup`, `onfilldown` | `fillUp`, `fillDown` |
| `ontempo(dir, down)` | `tempoHold.set(dir, down)` (a `TempoHold` the wiring owns, built with `app.send`; it calls `releaseAll()` on `<svelte:window onblur>`) |
| `ontempostep(dir)` | `tempoUp` when `dir` is 1, `tempoDown` when −1 |
| `onstyletempo` | `resetTempo` |

Nothing here is disabled: every command is valid in every state (Stop and Reset do nothing while stopped, as on the hardware).

### Children

| Where | Child | Props |
|---|---|---|
| top | `GroupHeader` | `{ title: 'Transport' }` (fills the 88px column; GroupHeader's users table) |
| row 1 | `Button` | `{ label: 'Start / Stop', size: 'band', compact: true, strong: true, bar: running, pressed: running, name: startStopName(running), onpress: onstartstop }` |
| row 2 | `Button` | `{ label: 'Stop', size: 'band', name: 'Stop: stops the band, never starts it (Stop)', onpress: onstop }` |
| row 3, left | `Button` | `{ label: 'Reset', size: 'pair', name: 'Section reset: restart the section from its first bar (Shift + Play)', onpress: onsectionreset }` |
| row 3, right | `Button` | `{ label: 'Fade', size: 'pair', on: fadeOn(fade), waiting: fade === 'armed', hue: 't2', pressed: fadeOn(fade), name: fadeName(fade), onpress: onfade }` |
| row 4 (`role="group"` `aria-label="Fills"`), left | `Button` | `{ label: 'Fill', symbol: 'up', size: 'pair', name: 'Fill Up: a fill, then the next Main up (at Main D, its own fill)', onpress: onfillup }` |
| row 4, right | `Button` | `{ label: 'Fill', symbol: 'down', size: 'pair', name: 'Fill Down: a fill, then the next Main down (at Main A, its own fill)', onpress: onfilldown }` |
| middle | `GroupHeader` | `{ title: 'Tempo' }` |
| row 5 | `Button` | `{ label: 'Tempo', symbol: 'plus', size: 'band', hold: true, name: 'Tempo up (Scene Launch)', onhold: (down) => ontempo(1, down), onpress: () => ontempostep(1) }` |
| row 6 | `Button` | `{ label: 'Tempo', symbol: 'minus', size: 'band', hold: true, name: 'Tempo down (Function)', onhold: (down) => ontempo(-1, down), onpress: () => ontempostep(-1) }` |
| row 7 | `Button` | `{ label: 'Style tempo', size: 'band', compact: true, name: 'Style tempo: back to the tempo the style came with (Scene Launch and Function together)', onpress: onstyletempo }` |

What Button draws from these (its Visual rules): `band` 88 × 32, label left at 8px, 14px (`compact` 13); `pair` 41 × 32, centred, 13px. `strong` puts Start / Stop's label in `--t` at medium weight. `bar: running` keeps the bar's room in both states (`bar: false` when stopped, so the label doesn't move when the band starts) and draws the 2px `--ok` bar with the `--bg` glow while running. `symbol: 'up' | 'down'` draws ▲ / ▼ at `--text-9` after "Fill"; `symbol: 'plus' | 'minus'` draws + / − (U+2212) at the label's size and `--weight-light` after "Tempo". So Stop and Tempo ± read 14px, Start / Stop, Style tempo, Reset, Fade and the Fills 13px, as the board draws them.

**Helpers**, in `app/src/ui/TransportColumn/names.ts`, unit-tested by `app/src/ui/TransportColumn/names.test.ts` (each line below is one assertion):

- `startStopName(running: boolean): string`: `true` → `"Start / Stop, running (Play). The same control as pad 16"`; `false` → `"Start / Stop, stopped (Play). The same control as pad 16"`.
- `fadeName(fade): string`: `off` → `"Fade in/out (Shift + Stop)"`; `armed` → `"Fade in/out, fade in armed (Shift + Stop)"`; `fadingIn` → `"Fade in/out, fading in (Shift + Stop)"`; `fadingOut` → `"Fade in/out, fading out (Shift + Stop)"`; `holding` → `"Fade in/out, holding silent (Shift + Stop)"`.
- `fadeOn(fade): boolean`: `true` for `fadingIn`, `fadingOut`, `holding`; `false` for `off`, `armed`.

### Controls

| Button | Face (Button's) | Callback | Command | Tooltip | Launchkey |
|---|---|---|---|---|---|
| Start / Stop | off face, `strong` (`--t` 13 / 500); running: the bar, `aria-pressed="true"` | `onstartstop` | `startStop` | `transport.start_stop` | Play; pad 16 |
| Stop | off face, 14 | `onstop` | `stop` | `transport.stop` | Stop |
| Reset | off face, 13 | `onsectionreset` | `sectionReset` | `transport.section_reset` | Shift + Play |
| Fade | off: off face (`--t2` label on `--btn`, as the board draws it); `armed`: waiting face in `--t2`; `fadingIn` / `fadingOut` / `holding`: on (lamp) face (Stage.md D13) | `onfade` | `toggleFade` | `transport.fade` | Shift + Stop |
| Fill ▲ | off face, 13, ▲ at 9 | `onfillup` | `fillUp` | `transport.fill_up` | — |
| Fill ▼ | off face, 13, ▼ at 9 | `onfilldown` | `fillDown` | `transport.fill_down` | — |
| Tempo + | off face, 14, + at 300 | `ontempo(1, down)`; keyboard `ontempostep(1)` | `tempoUp`, repeating (`tempoHold`) | `tempo.up` | Scene Launch |
| Tempo − | off face, 14, − at 300 | `ontempo(-1, down)`; keyboard `ontempostep(-1)` | `tempoDown`, repeating | `tempo.down` | Function |
| Style tempo | off face, 13 | `onstyletempo` | `resetTempo` (also: Tempo + and − held together, in `tempoHold`) | `tempo.reset` | Scene Launch and Function together |

Each tooltip key goes on its Button at integration (lead decision 4: no play asserts `data-tip` on a Button, and Button gets no `tip` prop); all nine keys exist in `app/src/help/tooltips.ts`. Start / Stop here and pad 16 are one control: the same label, the same green.

### Visual rules

- **Tokens used (TransportColumn's own layout):** `--space-6`, `--space-8`, `--space-20` (`scale.css`, exist); `--button-band-width` (88px) and `--button-pair-width` (41px), new in `scale.css` with Button's PR (Button D5), which lands before this component's. TransportColumn uses no colour or type token itself: the children draw every face, label and glyph. No new token for the tokens contract PR.
- **Component geometry** (axiom 2, declared once on the root): `--transport-height: 368px` (the band's height). No other literal sizes.
- **Box:** the root `section` is `--button-band-width` × `--transport-height`, `display: flex; flex-direction: column`, no fill, no border. From its top (y relative to the column; Stage x 1328, y 432 + this):

  | Item | y | Size |
  |---|---|---|
  | GroupHeader "Transport" | 0 | 88 × 36 |
  | Start / Stop | 44 | 88 × 32 |
  | Stop | 82 | 88 × 32 |
  | Reset \| Fade | 120 | two 41 × 32, gap 6 (Reset x 0, Fade x 47) |
  | Fills group: Fill ▲ \| Fill ▼ | 158 | two 41 × 32, gap 6 |
  | GroupHeader "Tempo" | 210 | 88 × 36 |
  | Tempo + | 254 | 88 × 32 |
  | Tempo − | 292 | 88 × 32 |
  | Style tempo | 330 | 88 × 32 |
  | empty | 362–368 | |

  That is: header; `margin-top: var(--space-8)`; a column `div` of four rows with `gap: var(--space-6)`; `margin-top: var(--space-20)`; header; `var(--space-8)`; a column `div` of three rows with `gap: var(--space-6)`. The two pair rows are grids `grid-template-columns: var(--button-pair-width) var(--button-pair-width)`, `column-gap: var(--space-6)`. Each Button is a direct child of its row (no wrapper).
- **States:** running (Button's bar) or stopped (no bar, its room kept); fade off, armed, on (Button's faces). Nothing else changes; no hover or pressed look (D35); keyboard focus is each Button's ring.
- **Type:** as Button draws it (DM Sans, tabular numerals); no type rule of TransportColumn's own.
- **Contrast (AA 4.5:1):** every text-on-surface pair in the column, all drawn by the children and all passing:
  - `--m` on `--g` (GroupHeader titles): exists in `contrast.test.ts`.
  - `--t2` on `--btn` (every off label but Start / Stop): exists.
  - `--t` on `--btn` (Start / Stop's `strong` label): Button's new pair; add to `contrast.test.ts` with the tokens PR (Button lists it as "strong label, open caret (Button)").
  - `--t2` on `--g` (Fade armed: transparent waiting face on the band's ground): exists.
  - `--lamp-ink` on `--lamp` (Fade fading or holding): exists.
  - No known failures.
- **Motion:** none; the bar and Fade's face follow the props.

### Accessibility

- **Role and name:** the root is a `section` with `aria-label="Transport and tempo"` (a `region`). Buttons are named as in Children (`name`). Start / Stop is `aria-pressed` `true` while running and `false` when stopped (Button D8, its `Running` / `Stopped` stories); its name also says running or stopped, as pad 16's does. Fade is `aria-pressed` `true` while fading in, fading out or holding, `false` when off or armed. The other seven have no `aria-pressed`. The Fills pair is a `div` with `role="group"` and `aria-label="Fills"`.
- **Keyboard:** Tab order is the DOM order: Start / Stop, Stop, Reset, Fade, Fill ▲, Fill ▼, Tempo +, Tempo −, Style tempo. Space and Enter press each button; on Tempo ± each key press is one step (`ontempostep`), as in "Tempo ±" above. Each stays focusable; none is ever disabled.
- **Tooltip ids:** in the Controls table, wired at integration.

## Stories (Story station)

Title `Components/TransportColumn`, `layout: 'centered'` (88 × 368). Every story renders in dark and light. Meta `args`: every callback is an action (`onstartstop: fn()`, `onstop: fn()`, `onsectionreset: fn()`, `onfade: fn()`, `onfillup: fn()`, `onfilldown: fn()`, `ontempo: fn()`, `ontempostep: fn()`, `onstyletempo: fn()`). Controls: `running` (boolean), `fade` (select of the five values).

The fixture `app/src/ui/TransportColumn/TransportColumn.fixtures.ts` exports `boardTransport` (Stage.md › Board fixture: `transport.running` true, `transport.fade` "off"), data only, exactly:

```ts
export const boardTransport = { running: true, fade: 'off' } satisfies Partial<ComponentProps<typeof TransportColumn>>
```

Plays find buttons with `within(canvasElement).getByRole('button', { name })` and dispatch pointer events with `fireEvent` from `storybook/test` on the Button element itself.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `boardTransport` | the running column: Start / Stop in white with its green bar, every other button off, Fade off | `Board-{dark,light}.png` (Stage 1328,432 88×368) | the region "Transport and tempo" holds 9 buttons in the order of the Keyboard line; the button named "Start / Stop, running (Play). The same control as pad 16" has `aria-pressed="true"` and contains exactly one `[data-bar]` element; Fade has `aria-pressed="false"`, `data-face="off"`; the group named "Fills" contains exactly the two buttons named "Fill Up: …" and "Fill Down: …"; the row after Stop holds Reset then Fade (Stage.md Check 13); `await userEvent.click` on each of Fill ▲, Style tempo, Reset, Stop, Start / Stop, Fade → `onfillup`, `onstyletempo`, `onsectionreset`, `onstop`, `onstartstop`, `onfade` each called exactly once, with no arguments |
| `TempoPresses` | `boardTransport` | as Board | — (same pixels as Board) | with `plus` = Tempo + and `minus` = Tempo −: `fireEvent.pointerDown(plus, { button: 0, pointerId: 1 })` → `ontempo` called once, with `(1, true)`; `fireEvent.pointerUp(plus, { button: 0, pointerId: 1 })` → `ontempo` called twice in all, last with `(1, false)`; `fireEvent.click(plus, { detail: 1 })` → `ontempostep` not called; `minus.focus()`, `await userEvent.keyboard('{Enter}')` → `ontempostep` called once, with `(-1)`; at the end `ontempo` has 2 calls, `ontempostep` 1, and `onstartstop`, `onstop`, `onsectionreset`, `onfade`, `onfillup`, `onfilldown`, `onstyletempo` 0 |
| `Stopped` | `{ ...boardTransport, running: false }` | Start / Stop without its bar, its label where it was | — (the board draws running) | the button is named "Start / Stop, stopped (Play). The same control as pad 16", has `aria-pressed="false"`, and contains no `[data-bar]` element |
| `FadeArmed` | `{ running: false, fade: 'armed' }` | Fade in the waiting face (`--t2` outline, no fill), stopped | — (the board draws fade off) | Fade is named "Fade in/out, fade in armed (Shift + Stop)", `data-face="waiting"`, `data-hue="t2"`, `aria-pressed="false"` |
| `Fading` | `{ running: true, fade: 'fadingOut' }` | Fade lit (lamp face) while the band fades out | — (the board draws fade off) | Fade is named "Fade in/out, fading out (Shift + Stop)", `data-face="on"`, `aria-pressed="true"` |
| `Holding` | `{ running: false, fade: 'holding' }` | Fade still lit while the style is held silent, stopped | — (not on the board) | Fade `data-face="on"`, `aria-pressed="true"`, named "Fade in/out, holding silent (Shift + Stop)" |
| `Focused` | `boardTransport`, `parameters: { pseudo: { focusVisible: true } }` | each button's focus ring | — (the board draws no focus) | — |

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `app/src/ui/TransportColumn/names.test.ts` asserts every line under Helpers (`npx vitest run src/ui/TransportColumn`).
- `npm run shots -- TransportColumn` passes: `Board` is 88 × 368 and scores at most 0.02 against its crops; axe finds no violation on any story.
- Only listed tokens are used; no inline colours; the only literal size is `--transport-height`.
- svelte-check and lint pass on the folder.

## Decisions

- D1 · Start / Stop's running bar is Button's `bar` (`bar: running`, so `false` keeps the bar's room when stopped and the label never moves), drawn with Button's `--bar-bottom` and `--bar-lift`; TransportColumn draws no overlay of its own.
- D2 · Start / Stop is `pressed` while running (Button D8 and its `Running` story); its name still says running or stopped, as pad 16's does.
- D3 · Fade is a Button `size: 'pair'` with `on` (fading in, fading out, holding), `waiting` (armed) with `hue: 't2'`, and `pressed` equal to `on` (Button's `FadeOn` and `Waiting` stories). Off, it is Button's off face with the `--t2` label the board draws.
- D4 · Tempo ± are Buttons with `hold: true`: `onhold` reports pointer down and up to `ontempo`, so the wiring's `TempoHold` repeats and detects both-held; a keyboard press is one step through `ontempostep`, as today's `HwButton`. The component runs no timer and adds no listeners or pointer capture of its own (Button owns both).
- D5 · Stop's name is "Stop: stops the band, never starts it (Stop)": the board's "(fade with hold)" is wrong, since `stop` only stops (app-api.md, tooltip `transport.stop`).
- D6 · Every name ends with its Launchkey mapping in parentheses where it has one (the board's pattern for Tempo ±); Reset gains "(Shift + Play)" and Fade "(Shift + Stop)", which the board's names lack.
- D7 · The column is 88 × 368, the band's height, so its story's shot matches the board box; its content ends at 362. The width is Button's `--button-band-width`; the height is the component's own `--transport-height`.
- D8 · Nothing in the column is ever disabled: every command is valid stopped or playing.
- D9 · Separate callbacks per button (`onfillup`, `onfilldown`, …) rather than one `onfill(delta)`, so each Controls row maps to one action in Storybook and one command in the wiring.
- D10 · Tempo ± have two callbacks: `ontempo(dir, down)` for the pointer hold and `ontempostep(dir)` for a keyboard press, because Button's `hold` mode sends the keyboard click through `onpress` only.
- D11 · `startStopName`, `fadeName` and `fadeOn` live in `names.ts` beside the component, with their own unit test, so the names are checked without rendering.

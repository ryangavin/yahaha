# TransportColumn

## Identity (all stations)

- **Kind:** complex
- **Built from:** GroupHeader, Button, LampButton
- **Purpose:** The band's transport and tempo buttons: start and stop the band, reset or fade it, call a fill, and nudge or reset the tempo, the same buttons the Launchkey has.
- **Boards:** `Stage-Dark.dc.html:360-388`; light: `Stage-Light.dc.html:336-364`. Crop box: Stage `1328,432 88×368`.
- **Not this component's job:** no store, no API, no Tauri: `running` and `fade` come in as props and every press goes out as a callback. No tempo repeat timer: Tempo + and − report down and up, and the page wiring repeats (`app/src/lib/tempoHold.ts`, which also sends `resetTempo` when both are held). No tooltip wiring (integration adds `use:tip`). No Shift layer (Shift + Play and Shift + Stop are the Launchkey's way to Reset and Fade; the screen has its own buttons for them).

## API (Component station)

### Props

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `running` | `boolean` | `false` | The band is playing: Start / Stop shows its green bar and its name says "running". |
| `fade` | `'off' \| 'armed' \| 'fadingIn' \| 'fadingOut' \| 'holding'` | `'off'` | The fade state (`transport.fade`): `armed` draws Fade in the waiting face, `fadingIn`, `fadingOut` and `holding` in the on face. |

### Events

| Callback | Fires when | Payload | The wiring sends |
|---|---|---|---|
| `onstartstop` | Start / Stop clicked (or Space / Enter) | `()` | `startStop` |
| `onstop` | Stop clicked | `()` | `stop` |
| `onsectionreset` | Reset clicked | `()` | `sectionReset` |
| `onfade` | Fade clicked | `()` | `toggleFade` |
| `onfillup` | Fill ▲ clicked | `()` | `fillUp` |
| `onfilldown` | Fill ▼ clicked | `()` | `fillDown` |
| `ontempo` | Tempo + or − goes down or up (below) | `(dir: 1 \| -1, down: boolean)`: `dir` 1 for Tempo +, −1 for Tempo − | `tempoHold.set(dir, down)`: `tempoUp` / `tempoDown` at once and repeating while held; both held: `resetTempo` |
| `onstyletempo` | Style tempo clicked | `()` | `resetTempo` |

Every callback is optional; its default does nothing (so `boardTransport` can hold data only).

**Tempo down and up.** Each tempo button sits in a wrapper `div` (88 × 32, the column's flex item) that listens to the events bubbling from its Button; the Button gets no `onpress`.

- `ontempo(dir, true)` on `pointerdown` with the primary button (`event.button === 0`), and the wrapper calls `setPointerCapture(event.pointerId)` on the Button element so the release comes back; and on `keydown` of Space or Enter when `event.repeat` is false.
- `ontempo(dir, false)` on the matching `pointerup`, `pointercancel` or `lostpointercapture`, on `keyup` of Space or Enter, and on `blur` while down.
- A per-button `down` flag (component state, not a timer) makes each press report exactly one down and one up, whichever of those events ends it.

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
| `ontempo(dir, down)` | `tempoHold.set(dir, down)` (a `TempoHold` the wiring owns, built with `app.send`; it calls `releaseAll()` when the window loses focus) |
| `onstyletempo` | `resetTempo` |

Nothing here is disabled: every command is valid in every state (Stop and Reset do nothing while stopped, as on the hardware).

### Children

| Where | Child | Props |
|---|---|---|
| top | `GroupHeader` | `{ title: 'Transport' }` |
| row 1 | `Button` | `{ variant: 'band', label: 'Start / Stop', name: startStopName(running), onpress: onstartstop }`, with the label snippet below; in the Start / Stop wrapper |
| row 2 | `Button` | `{ variant: 'band', label: 'Stop', name: 'Stop: stops the band, never starts it (Stop)', onpress: onstop }` |
| row 3, left | `Button` | `{ variant: 'pair', label: 'Reset', name: 'Section reset: restart the section from its first bar (Shift + Play)', onpress: onsectionreset }` |
| row 3, right | `LampButton` | `{ label: 'Fade', size: 'cell', on: fade is 'fadingIn' \| 'fadingOut' \| 'holding', waiting: fade === 'armed', hue: 't2', name: fadeName(fade), ontoggle: () => onfade() }` |
| row 4 (`role="group"` `aria-label="Fills"`), left | `Button` | `{ variant: 'pair', label: 'Fill ▲', name: 'Fill Up: a fill, then the next Main up (at Main D, its own fill)', onpress: onfillup }`, glyph snippet |
| row 4, right | `Button` | `{ variant: 'pair', label: 'Fill ▼', name: 'Fill Down: a fill, then the next Main down (at Main A, its own fill)', onpress: onfilldown }`, glyph snippet |
| middle | `GroupHeader` | `{ title: 'Tempo' }` |
| row 5 | `Button` | `{ variant: 'band', label: 'Tempo +', name: 'Tempo up (Scene Launch)' }`, glyph snippet; in a tempo wrapper with `dir` 1 |
| row 6 | `Button` | `{ variant: 'band', label: 'Tempo −', name: 'Tempo down (Function)' }`, glyph snippet; in a tempo wrapper with `dir` −1 |
| row 7 | `Button` | `{ variant: 'band', label: 'Style tempo', name: 'Style tempo: back to the tempo the style came with (Scene Launch and Function together)', onpress: onstyletempo }`, label snippet |

`label` is always the plain text above (the default accessible name when there's no `name`, and what the snippet shows). The snippets are TransportColumn's own content, passed as Button's `children` (Needs from primitives) and styled by TransportColumn:

| Button | Snippet content | Its style |
|---|---|---|
| Start / Stop | `<span>Start / Stop</span>` | `--t`, `--weight-medium`, `--text-13` |
| Style tempo | `<span>Style tempo</span>` | `--text-13` (colour and weight the Button's) |
| Fill ▲, Fill ▼ | `Fill <span>▲</span>` / `Fill <span>▼</span>` | the glyph span `--text-9` (to add, kit.md › Scale tokens) |
| Tempo +, Tempo − | `Tempo <span>+</span>` / `Tempo <span>−</span>` (U+2212) | the glyph span `--weight-light` |

Stop and Reset use the plain `label` (no snippet). The `band` Button's label is 14px and the `pair`'s 13px (Button's own rules), so Stop and Tempo ± read 14, Reset and the Fills 13, and the two band buttons with a 13px label get it from their snippet.

**Accessible names** (the board's, with the state where it changes):

- `startStopName(running)`: `"Start / Stop, running (Play). The same control as pad 16"` when running, else `"Start / Stop, stopped (Play). The same control as pad 16"`.
- `fadeName(fade)`: `off` → `"Fade in/out (Shift + Stop)"`; `armed` → `"Fade in/out, fade in armed (Shift + Stop)"`; `fadingIn` → `"Fade in/out, fading in (Shift + Stop)"`; `fadingOut` → `"Fade in/out, fading out (Shift + Stop)"`; `holding` → `"Fade in/out, holding silent (Shift + Stop)"`.

### Controls

| Button | Face | Callback | Command | Tooltip | Launchkey |
|---|---|---|---|---|---|
| Start / Stop | off face, label `--t` 13 / 500; running: the green bar | `onstartstop` | `startStop` | `transport.start_stop` | Play; pad 16 |
| Stop | off face, 14 | `onstop` | `stop` | `transport.stop` | Stop |
| Reset | off face, 13 | `onsectionreset` | `sectionReset` | `transport.section_reset` | Shift + Play |
| Fade | lamp: off; `armed` waiting in `--t2`; `fadingIn` / `fadingOut` / `holding` on (Stage.md D13) | `onfade` | `toggleFade` | `transport.fade` | Shift + Stop |
| Fill ▲ | off face, 13, ▲ at 9 | `onfillup` | `fillUp` | `transport.fill_up` | — |
| Fill ▼ | off face, 13, ▼ at 9 | `onfilldown` | `fillDown` | `transport.fill_down` | — |
| Tempo + | off face, 14, + at 300 | `ontempo(1, down)` | `tempoUp`, repeating (`tempoHold`) | `tempo.up` | Scene Launch |
| Tempo − | off face, 14, − at 300 | `ontempo(-1, down)` | `tempoDown`, repeating | `tempo.down` | Function |
| Style tempo | off face, 13 | `onstyletempo` | `resetTempo` (also: Tempo + and − held together, in `tempoHold`) | `tempo.reset` | Scene Launch and Function together |

Each tooltip key goes on the Button / LampButton element as `data-tip` at integration; all nine exist in `app/src/help/tooltips.ts`. Start / Stop here and pad 16 are one control: the same label, the same green.

### Visual rules

- **Tokens used (TransportColumn's own):** `--ok`, `--bg`, `--t`, `--weight-medium`, `--weight-light`, `--text-13`, `--text-9` (to add), `--space-6`, `--space-8`, `--space-20`. The children draw their own faces.
- **Box:** 88 × 368 (the band's height), a column, no fill, no border. From its top (y relative to the column; Stage x 1328, y 432 + this):

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

  That is: header, 8 (`--space-8`), a column of four rows with gap 6 (`--space-6`), 20 (`--space-20`), header, 8, a column of three rows with gap 6. The two pair rows are grids `41px 41px`, column gap 6.
- **Start / Stop's bar** (TransportColumn draws it, not Button): the Start / Stop wrapper is `position: relative`, 88 × 32; while `running`, an `aria-hidden` span inside it, after the Button, absolutely placed `left: 8px; right: 8px; bottom: 5px` (so 72 × 2, its top 25px down the button), height 2, radius 1, `--ok` fill, `box-shadow: var(--bg)`, `pointer-events: none`. Not running: no span. It carries `data-bar="running"` (test hook).
- **Fade's faces** come from LampButton (`size: 'cell'`, 13px, filling its 41 × 32 cell): off is the lamp's off face; `waiting` with `hue: 't2'` is the waiting face (transparent, 1px `--t2` border, `--t2` label); on is the lamp face. Its root carries `data-face="off|waiting|on"` (LampButton's hook).
- **States:** running (bar) or stopped (no bar); fade off, armed, on. Nothing else changes; no hover or pressed look (D35); keyboard focus is each child's ring.
- **Type:** as the children draw it, plus the snippet styles in Children: Start / Stop 13 / 500 `--t`; Style tempo 13; the Fill glyphs 9px; the tempo glyphs weight 300; DM Sans, tabular numerals.
- **Contrast (AA 4.5:1, `tokens/contrast.test.ts`):** `--t` on `--btn` (Start / Stop's label); `--t2` on `--g` (Fade armed: transparent face on the band's ground). The children's own pairs are in their specs.
- **Motion:** none; the bar and Fade's face follow the props.

### Accessibility

- **Role and name:** the root is a `section` with `aria-label="Transport and tempo"` (a `region`). Buttons are named as in Children (`name`); Fade is a `button` with `aria-pressed` (true when on). Start / Stop has no `aria-pressed`: it's an action, and its name says running or stopped. The Fills pair is a `div` with `role="group"` and `aria-label="Fills"`.
- **Keyboard:** Tab order is the DOM order: Start / Stop, Stop, Reset, Fade, Fill ▲, Fill ▼, Tempo +, Tempo −, Style tempo. Space and Enter press each button (Tempo ±: down on keydown, up on keyup). Each stays focusable; none is ever disabled.
- **Tooltip ids:** in the Controls table, wired at integration.

## Stories (Story station)

Title `Components/TransportColumn`, `layout: 'centered'` (88 × 368). Every story renders in dark and light. Every callback is an action (`fn()`). Controls: `running` (boolean), `fade` (select of the five values).

The fixture `app/src/ui/TransportColumn/TransportColumn.fixtures.ts` exports `boardTransport` (Stage.md › Board fixture: `transport.running` true, `transport.fade` "off"), data only, exactly:

```ts
export const boardTransport = { running: true, fade: 'off' } satisfies Partial<ComponentProps<typeof TransportColumn>>
```

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `boardTransport` | the running column: Start / Stop in white with its green bar, every other button off, Fade off | `Board-{dark,light}.png` (Stage 1328,432 88×368) | the region "Transport and tempo" holds 9 buttons in the order of the Keyboard line; the button named "Start / Stop, running (Play). The same control as pad 16" exists and `[data-bar="running"]` exists; Fade has `aria-pressed="false"`, `data-face="off"`; the group named "Fills" contains exactly the Fill ▲ and Fill ▼ buttons; the row after Stop holds Reset then Fade (Stage.md Check 13); click Fill ▲ → `onfillup` called once; click Style tempo → `onstyletempo` called once; click Reset → `onsectionreset`; click Stop → `onstop`; click Start / Stop → `onstartstop`; click Fade → `onfade` |
| `TempoPresses` | `boardTransport` | as Board | — (same pixels as Board) | pointerdown on Tempo + → `ontempo` called with `(1, true)`; pointerup → `(1, false)`; focus Tempo −, keydown Space → `(-1, true)`; a second keydown Space with `repeat: true` → no call; keyup Space → `(-1, false)`; `ontempo` called 4 times in all; a click on Tempo + calls no other callback |
| `Stopped` | `{ ...boardTransport, running: false }` | Start / Stop without its bar | — (the board draws running) | the button is named "Start / Stop, stopped (Play). The same control as pad 16"; no `[data-bar]` |
| `FadeArmed` | `{ running: false, fade: 'armed' }` | Fade in the waiting face (`--t2` outline), stopped | — (the board draws fade off) | Fade is named "Fade in/out, fade in armed (Shift + Stop)", `data-face="waiting"`, `aria-pressed="false"` |
| `Fading` | `{ running: true, fade: 'fadingOut' }` | Fade lit (lamp face) while the band fades out | — (the board draws fade off) | Fade is named "Fade in/out, fading out (Shift + Stop)", `data-face="on"`, `aria-pressed="true"` |
| `Holding` | `{ running: false, fade: 'holding' }` | Fade still lit while the style is held silent, stopped | — (not on the board) | Fade `aria-pressed="true"`, named "Fade in/out, holding silent (Shift + Stop)" |
| `Focused` | `boardTransport`, `parameters: { pseudo: { focusVisible: true } }` | each button's focus ring | — (the board draws no focus) | — |

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- TransportColumn` passes: `Board` is 88 × 368 and scores at most 0.02 against its crops; axe finds no violation on any story.
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules.
- svelte-check and lint pass on the folder.

## Decisions

- D1 · TransportColumn draws Start / Stop's running bar itself, as an `aria-hidden` span over the Button in a relative wrapper, since nothing else in the library has it (kit: the bar survives only in pads and on Start / Stop); Button gains no bar prop.
- D2 · Start / Stop's label sits centred like the other band buttons; the board's `padding-bottom: 3px` (which lifts it 1.5px) is dropped: the bar at bottom 5 doesn't touch the text, and the difference is a few pixels in the shot.
- D3 · Start / Stop has no `aria-pressed` (it sends one action, not a switch state); its name says "running" or "stopped", as pad 16's does.
- D4 · Fade is a LampButton (`size: 'cell'`), since it's a switch with an on face and a waiting face (D13); off, it reads in the lamp's `--m`, not the board's `--t2`, like every other switch (Metronome, Unison), within the shot's threshold.
- D5 · Tempo ± report down and up from a wrapper around each Button (pointer events with capture on the Button, Space / Enter keydown without repeat and keyup, blur), so the wiring's `TempoHold` repeats and detects both-held; the component runs no timer.
- D6 · Stop's name is "Stop: stops the band, never starts it (Stop)": the board's "(fade with hold)" is wrong, since `stop` only stops (app-api.md, tooltip `transport.stop`).
- D7 · Every name ends with its Launchkey mapping in parentheses where it has one (the board's pattern for Tempo ±); Reset gains "(Shift + Play)" and Fade "(Shift + Stop)", which the board's names lack.
- D8 · The column is 88 × 368, the band's height, so its story's shot matches the board box; its content ends at 362.
- D9 · Nothing in the column is ever disabled: every command is valid stopped or playing.
- D10 · Separate callbacks per button (`onfillup`, `onfilldown`, …) rather than one `onfill(delta)`, so each Controls row maps to one action in Storybook and one command in the wiring.

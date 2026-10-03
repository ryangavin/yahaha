# LampRow

## Identity (all stations)

- **Kind:** complex
- **Built from:** LampButton, Button
- **Purpose:** The row of buttons under the faders: each part's On/Off (hold it to swap the part's sound), then Harm/Arp, Sound, L Hold, Looper and the fader page, the same buttons as under the Launchkey's faders.
- **Boards:** `Stage-Dark.dc.html:271-286` (headers `:271-274`, the row `:275-285`); light: `Stage-Light.dc.html:247-262`. Box `Stage 24,748 654×52` (headers `24,748 654×18`, the row `24,768 654×32`). The lamp CSS is `Stage-Dark.dc.html:66` / `Stage-Light.dc.html:42`.
- **Not this component's job:** no store, no API, no Tauri, no timer of its own (long press is the children's `use:longpress`, through their `onlongpress` / `onlongrelease`). It doesn't send: it calls back, and the page wiring sends the command its table names. It doesn't know the pads: releasing a latched Sound after a pad press is the wiring's (Stage.md D18). It doesn't draw a lamp's face (LampButton and Button do). No tooltips: the keys are cited per control below and wired at integration. On the Style fader page it doesn't draw the Style parts' mutes (#507, D7).

## API (Component station)

### Props

`LampRowProps` (every prop below and every callback in Events) is exported from `app/src/ui/LampRow/types.ts`, so FaderBank can take it whole. Every callback is optional: one that isn't passed is simply not called.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `partOn` | `[boolean, boolean, boolean, boolean]` | `[true, true, true, true]` | Each keyboard part's switch (Right 1, Right 2, Right 3, Left): the lamp reads "On" or "Off". |
| `partSounding` | `[boolean, boolean, boolean, boolean]` | `[true, true, true, true]` | Each part sounds: the lamp is lit (Stage.md D12). |
| `swapPart` | `0 \| 1 \| 2 \| 3 \| null` | `null` | The part whose swap hold is on (its lamp reads "Swap"). |
| `soundOn` | `boolean` | `false` | The Sound layer is on (held or latched, from the screen or the hardware): the Sound lamp is lit. |
| `harmArp` | `boolean` | `false` | Harmony/Arpeggio is on. |
| `leftHold` | `boolean` | `false` | Left Hold is on. |
| `looper` | `'off' \| 'recArmed' \| 'recording' \| 'loopArmed' \| 'looping'` | `'off'` | The Chord Looper's mode: chooses the Looper cell's child and face (table below). |
| `page` | `'panel' \| 'style'` | `'panel'` | The fader page: the master button's label. |
| `shift` | `boolean` | `false` | Shift is held: a click is a Shift-click (lane convention 4). It is the app-only `ui.shift` (in `app/src/lib`, not `AppState`), which already ORs the computer keyboard's Shift and `surface.shift`. Changes the part lamps', Looper's and master button's click and their accessible names. |
| `masterDisabled` | `boolean` | `false` | The master button is disabled (its state-carried action for the current `shift` is null). |
| `width` | `number \| undefined` | — | A fixed width in px. Unset: fills its container (654 in FaderBank). Stories set 654. |

The part names are fixed and in this order: "Right 1", "Right 2", "Right 3", "Left" (`keyboardParts` is always these four).

### Events

| Callback | Fires when | Payload | The wiring sends |
|---|---|---|---|
| `onpart` | a part lamp is clicked (Space, Enter, or a click that isn't the end of a long press), without Shift, and that part's swap isn't on | `(part: 0 \| 1 \| 2 \| 3)` | `togglePart { part }` |
| `onpartchannel` | a part lamp is clicked with `shift` true | `(part)` | `surface.controls[faderButton{part + 1}].shiftAction` when not null (`selectPart { part }` on the Panel page), then opens Channel for the part (Stage.md D32 interim: `show(part)` in `panels/channel/nav.svelte.ts`) (D14) |
| `onhold` | the swap or Sound hold changes (the hold table) | `(hold: Hold, latch: boolean)`, `Hold` = `{ type: 'none' } \| { type: 'sound' } \| { type: 'swap'; part: 0 \| 1 \| 2 \| 3 }` (exported from `holds.ts`); `latch` is true only for a Sound click that latches it | `setLayer { layer: hold }`; with `latch`, the wiring also remembers the latch so the next screen pad press releases it (D18) |
| `onharmarp` | Harm/Arp clicked (Shift or not) | `()` | `toggleHarmonyArp` |
| `onlefthold` | L Hold clicked (Shift or not) | `()` | `toggleLeftHold` |
| `onlooper` | Looper clicked without Shift | `()` | `looperOnOff` |
| `onlooperrec` | Looper long-pressed (350 ms) or right-clicked, or clicked with `shift` true | `()` | `looperRec` |
| `onmaster` | the master button clicked without Shift (not disabled) | `()` | `surface.controls[masterButton].action` |
| `onmastershift` | the master button clicked with `shift` true (not disabled) | `()` | `surface.controls[masterButton].shiftAction` (whatever it is; D15) |

**Long press and right-click** come from the children's shared `use:longpress` action (`app/src/ui/actions/longpress`): held 350 ms (`--long-press`) without moving more than 4px calls the child's `onlongpress` (pointer still down) and swallows the click that follows; `onlongrelease` comes on that press's `pointerup` or `pointercancel`. A right-click (or the keyboard's Menu key / Shift+F10) is the same pair: `onlongpress`, then `onlongrelease` on the right button's release (macOS) or straight after (Windows, keyboard); the action prevents the browser menu. LampRow has no `contextmenu` handler of its own.

**The hold table** (Stage.md D18, Check 11). Each cell's **row** is chosen from the props when its press starts (below); the click, long press and long release of that press all act by that row, so the state's answer to a long press (the lamp now reading "Swap", Sound now lit) can't change what the same press's release does. `—` means nothing is called.

| Row | Chosen when | Click (Shift false) | Click (Shift true) | Long press | Long release |
|---|---|---|---|---|---|
| `part` | part cell *i*, `swapPart !== i` | `onpart(i)` | `onpartchannel(i)` | `onhold({ type: 'swap', part: i }, false)` | — (latched until the next click) |
| `partSwapped` | part cell *i*, `swapPart === i` | `onhold({ type: 'none' }, false)` | `onpartchannel(i)` | — | `onhold({ type: 'none' }, false)` |
| `soundOff` | Sound, `soundOn` false | `onhold({ type: 'sound' }, true)` (latch) | as Shift false | `onhold({ type: 'sound' }, false)` at the 350 ms mark | `onhold({ type: 'none' }, false)` (momentary) |
| `soundOn` | Sound, `soundOn` true | `onhold({ type: 'none' }, false)` | as Shift false | — | `onhold({ type: 'none' }, false)` (any press lets go) |
| `harmArp` | Harm/Arp | `onharmarp()` | `onharmarp()` | — (a no-op `onlongpress` is passed, so the long press's click is swallowed, D8) | — |
| `leftHold` | L Hold | `onlefthold()` | `onlefthold()` | — (no-op, as Harm/Arp) | — |
| `looper` | Looper | `onlooper()` | `onlooperrec()` | `onlooperrec()` | — |
| `master` | the master button | `onmaster()` | `onmastershift()` | (no `onlongpress` passed: the action is off; a right-click only loses its browser menu) | — |

A right-click is a long press and its release at once (or on the right button's release): on a part it latches the swap, on a swapped part it lets go, on Looper it records, and on Sound it holds Sound for as long as the right button is down where the platform allows it (macOS), or for an instant (Windows, the Menu key: `setLayer sound` then `none`) (D3). Long press and right-click ignore `shift`. Keyboard Space and Enter are a click; there is no keyboard hold.

**Choosing the row** (D9). Per cell, LampRow keeps the row of the press in progress:

- the cell's wrapper `div` (Children) handles `onpointerdown` (any button) by choosing the row from the props at that moment and keeping it;
- its `onkeydown` drops a kept row (a key press is a new press);
- a child callback that comes with no kept row (a keyboard click, the Menu key) chooses the row from the props then and keeps it;
- the click (`ontoggle` / `onpress`) and `onlongrelease` act by the kept row, then drop it; `onlongpress` acts by it and keeps it;
- `shift` is read when the click comes.

The pure parts are in `app/src/ui/LampRow/holds.ts`, tested by `holds.test.ts` (one assertion per cell of the hold table above):

```ts
export type Hold = { type: 'none' } | { type: 'sound' } | { type: 'swap'; part: 0 | 1 | 2 | 3 }
export type Row = 'part' | 'partSwapped' | 'soundOff' | 'soundOn' | 'harmArp' | 'leftHold' | 'looper' | 'master'
/** The row for cell 1–9 from the props now. */
export function rowFor(cell: number, p: { swapPart: 0 | 1 | 2 | 3 | null; soundOn: boolean }): Row
export type Effect =
  | { call: 'onpart' | 'onpartchannel'; part: 0 | 1 | 2 | 3 }
  | { call: 'onhold'; hold: Hold; latch: boolean }
  | { call: 'onharmarp' | 'onlefthold' | 'onlooper' | 'onlooperrec' | 'onmaster' | 'onmastershift' }
  | null
/** What a click, a long press and a long release do; `part` is the cell's part (0–3) or null. */
export function clickEffect(row: Row, part: 0 | 1 | 2 | 3 | null, shift: boolean): Effect
export function longPressEffect(row: Row, part: 0 | 1 | 2 | 3 | null): Effect
export function longReleaseEffect(row: Row, part: 0 | 1 | 2 | 3 | null): Effect
```

### Children

The row is a 9-column grid of cell wrappers: each a `div` with `data-cell="1"` … `"9"`, `min-width: 0`, `display: flex` (its child fills it), holding one child and handling `onpointerdown` / `onkeydown` as above. No child gets a `tip` (tooltips are wired at integration).

| Cell | Child | Props passed | Callbacks |
|---|---|---|---|
| 1–4 (part *i*) | `LampButton` | `{ label: swapPart === i ? 'Swap' : partOn[i] ? 'On' : 'Off', on: partSounding[i], size: 'cell', name: <Accessible names> }` | `ontoggle` → the click (its `on` payload is ignored); `onlongpress`, `onlongrelease` → the hold table |
| 5 | `LampButton` | `{ label: 'Harm/Arp', on: harmArp, size: 'cell', name: 'Harmony/Arpeggio' }` | `ontoggle` → click; `onlongpress: () => {}` (no-op, D8) |
| 6 | `LampButton` | `{ label: 'Sound', on: soundOn, size: 'cell', name: <below> }` | `ontoggle`, `onlongpress`, `onlongrelease` → the hold table |
| 7 | `LampButton` | `{ label: 'L Hold', on: leftHold, size: 'cell', name: 'Left Hold' }` | `ontoggle` → click; `onlongpress: () => {}` (no-op, D8) |
| 8 | `LOOPER_CELL[looper]`, below | | |
| 9 | `Button` | `{ label: page === 'panel' ? 'Panel' : 'Style', size: 'cell', disabled: masterDisabled, name: <below> }` (no `pressed`: no `aria-pressed`) | `onpress` → the click (`shift ? onmastershift() : onmaster()`); no `onlongpress` |

`LOOPER_CELL`, the Looper cell's child by `looper` (Stage.md D13, kit › Lamp row; D5). LampButton has no waiting face, so the two armed modes are a Button in its waiting face:

| `looper` | Child and props (besides `label: 'Looper'`, `size: 'cell'`, `name`) | Face (`data-face`) | `aria-pressed` |
|---|---|---|---|
| `off` | `LampButton { on: false }` | `off` | false |
| `recArmed` | `Button { waiting: true, hue: 'rec', pressed: false }` | `waiting`, `data-hue="rec"`: `--rec` outline and label | false |
| `recording` | `LampButton { on: true, rec: true }` | `record` (solid `--rec`, `--solid-ink` label) | true |
| `loopArmed` | `Button { waiting: true, hue: 'lamp', pressed: false }` | `waiting`, `data-hue="lamp"`: `--lamp` outline and label | false |
| `looping` | `LampButton { on: true }` | `on` (lamp) | true |

Both children get the Looper's callbacks: the LampButton's `ontoggle` and the Button's `onpress` → the click; `onlongpress` → `onlooperrec()`; `onlongrelease` → nothing. When the mode changes between a LampButton and a Button the cell's child is replaced, so keyboard focus on it is lost (D5).

**LampButton's local flip.** LampButton flips its own pressed state on a click until its `on` prop changes. LampRow relies on the next state to correct it: a click whose command changes `on` (a toggle) is corrected when the state comes back; one that doesn't (a swap let go, a Shift-click to Channel, a refused toggle such as L Hold under Manual Bass) leaves the flipped face until `on` next changes. Plays therefore never assert `aria-pressed` after a click. The fix is a LampButton `controlled` prop (Cross-lane needs).

### Accessible names

LampButton's spec has the parent say what a long press does in the name (its Accessibility); the names here do, as the board's do.

- **Part lamp *i*:** swap not on, without Shift `"{part} {on|off}. Long press: swap mode"` from `partOn` ("Right 1 on. Long press: swap mode", "Right 3 off. Long press: swap mode"); with Shift `"Open Channel for {part}. Long press: swap mode"`. Swap on for *i*: `"{part} swap held"` ("Right 1 swap held"), with Shift `"Open Channel for {part}"`. `aria-pressed` is LampButton's (`partSounding[i]`).
- **Harm/Arp** "Harmony/Arpeggio"; **L Hold** "Left Hold" (each with `aria-pressed` = on).
- **Sound:** not lit "Sound layer. Long press: hold"; lit "Sound layer" (any press lets go). `aria-pressed` = `soundOn`.
- **Looper:** "Chord Looper", plus ", recording armed", ", recording", ", loop armed" for those modes, then ". Long press: loop rec" ("Chord Looper, loop armed. Long press: loop rec"); with `shift` true "Chord Looper record".
- **Master button:** without Shift `"Fader page is {Panel|Style}: click for {Style|Panel}"` ("Fader page is Panel: click for Style", the board's); with `shift` true `"Fader page button with Shift"`, which names the button and not what its `shiftAction` does (D15).

### Visual rules

The root is a column, `width` (or its container's) × 52: the **headers** (`--header-height`), a `--space-2` gap, the **row** (`--control-height`, 32).

**Geometry** declared once as custom properties on the root (axiom 2): `--header-height: 18px` (the headers row), `--header-line: 16px` (the header text's line height). Everything else uses `scale.css` tokens.

- **Headers:** a grid, `grid-template-columns: repeat(9, minmax(0, 1fr))`, `column-gap: var(--space-8)`, height `--header-height`, no wrap. Two cells, each stretched to the full 18 (`align-self: stretch`), `box-sizing: border-box`, `border-bottom: var(--line-width) solid var(--line)`, no padding, `--text-12` / `--weight-regular`, line-height `--header-line`, `--m`; the text sits at the top of its box (its line box is y 0–16, y 16 is empty, the hairline is y 17), as the board draws it. "Part on/off" spans columns 1–4 (`grid-column: 1 / 5`); the other spans columns 5–9 (`grid-column: 5 / 10`), `display: flex`: "Functions" at the left and "Launchkey fader buttons 5–9" (an en dash) pushed right (`margin-left: auto`). Plain text, not headings, not focusable.
- **Row:** `margin-top: var(--space-2)`, height `--control-height`, `position: relative`, a grid `repeat(9, minmax(0, 1fr))` with `gap: var(--space-8)`; each cell wrapper fills its cell. At 654 wide a column is `(654 − 64) / 9 = 65.556`.
- **Divider:** an `aria-hidden` span, `--line`, `position: absolute; top: 0; bottom: 0; width: var(--line-width); left: calc((100% - 8 * var(--space-8)) * 4 / 9 + 3 * var(--space-8) + var(--space-4))`: the middle of the gap between columns 4 and 5 (4 columns, 3 gaps and 4px; at 654 that is 290.22). Drawn on the row only, not the headers (the headers' two hairlines already split there).
- **Tokens used (own):** `--line`, `--line-width`, `--m`, `--text-12`, `--weight-regular`, `--space-2`, `--space-4`, `--space-8`, `--control-height`, `--font-sans`. The children use their own. No new tokens.
- **Size:** `width` × 52; no size variants.
- **States drawn by:** the children's faces (the tables above); LampRow itself draws no state. The Looper's five faces and the Swap label are stories.
- **Type:** headers DM Sans 12 / 400, sentence case as written, tabular numerals.
- **Contrast** (text on its surface, AA 4.5:1):

  | Pair | Where | Dark | Light | |
  |---|---|---|---|---|
  | `--m` on `--g` | headers | 6.25 | 5.24 | row exists |
  | `--rec` on `--g` | Looper rec-armed (Button waiting face over the ground) | 5.67 | 5.27 | add to `contrast.test.ts` with the tokens PR |
  | `--lamp` on `--g` | Looper loop-armed | 13.24 | **3.74** | dark: add with the tokens PR; light: known failure |

  The lamps' own pairs (`--m` on `--btn`, `--lamp-ink` on `--lamp`, `--solid-ink` on `--rec`, `--t2` on `--btn`) are LampButton's and Button's.

  **Known failures (owner question O-contrast):** `--lamp` on `--g` in light, 3.74:1 (`#4f8a0e` on `#f2f1ee`), the loop-armed Looper's label. Until the owner answers, the meta's `parameters.a11y.context.exclude` is `KNOWN_CONTRAST_FAILURES` from `app/src/ui/LampRow/a11y.ts`, `['[data-cell="8"] [data-face="waiting"][data-hue="lamp"]']`, which matches only that element (in both themes; the dark pair is still checked by `contrast.test.ts`). FaderBank reuses the list.
- **Motion:** none. (The hardware flashes REC and ON/OFF when armed; the screen draws armed as the steady waiting face, D13.)

### Accessibility

- **Role and name:** a `div` with `role="group"` and `aria-label="Fader buttons"`; inside, nine buttons with the names above, in grid order. The headers are visible text inside the group.
- **Keyboard:** Tab walks the nine buttons left to right; Space or Enter is a click (Shift as `shift` says). The Menu key or Shift+F10 on a focused button is a right-click (the action).
- **Tooltip ids** (cited for integration, which wires them; no child gets a `tip` prop and no play asserts `data-tip`): part lamps `part.right1.on`, `part.right2.on`, `part.right3.on`, `part.left.on`, and `part.swap` while that part's swap is on; Harm/Arp `harmony.switch`; Sound `launchkey.sound`; L Hold `detection.left_hold`; Looper `looper.on_off` (its body names Shift; `looper.rec` stays the Looper page's key, D6); master `mixer.page` (`mixer.layer`'s body is reached from the layer tabs, D6). All exist in `app/src/help/tooltips.ts`.
- **Launchkey:** part lamps = Panel fader buttons 1–4 (hold + knob: swap; Shift: `selectPart`, D14); Harm/Arp = fader button 5; Sound = fader button 6 (hold); L Hold = fader button 7; Looper = fader button 8 (Shift + 8: REC); master button = the button under the master fader (Shift: its `shiftAction`).

### From the state

The page wiring (`app/src/pages/StageWiring.svelte`) fills the props from `AppState` and the app-only `ui` state, and sends what each callback names. Nothing here imports `AppState`.

| Prop / callback | From `AppState` (or `ui`) / sends |
|---|---|
| `partOn[i]` | `keyboardParts[i].on` |
| `partSounding[i]` | `keyboardParts[i].sounding` |
| `swapPart` | `surface.layer.type === 'swap' ? surface.layer.part : null` |
| `soundOn` | `surface.layer.type === 'sound'` |
| `harmArp` | `harmonyArp.on` |
| `leftHold` | `chord.leftHold` |
| `looper` | `looper.mode` |
| `page` | `mixer.faderPage` |
| `shift` | `ui.shift`: app-only, in `app/src/lib`, not `AppState`; it already ORs the computer keyboard's Shift and `surface.shift` |
| `masterDisabled` | `ui.shift ? surface.controls[masterButton].shiftAction == null : surface.controls[masterButton].action == null` (`masterButton` is the control with `id` `"masterButton"`) |
| `onpart(i)` | `togglePart { part: i }` |
| `onpartchannel(i)` | `surface.controls[faderButton{i + 1}].shiftAction` if not null, then Channel for part *i* (D32 interim: `show(i)` in `panels/channel/nav.svelte.ts`) (D14) |
| `onhold(hold, latch)` | `setLayer { layer: hold }`; keeps `soundLatched = latch` for a Sound hold, clears it on `none` |
| `onharmarp()` | `toggleHarmonyArp` |
| `onlefthold()` | `toggleLeftHold` (a refusal under Manual Bass shows on the status line) |
| `onlooper()` | `looperOnOff` |
| `onlooperrec()` | `looperRec` |
| `onmaster()` | `surface.controls[masterButton].action` |
| `onmastershift()` | `surface.controls[masterButton].shiftAction` |

## Stories (Story station)

- **Title:** `Components/LampRow`.
- **Layout:** `centered`; every story sets `width: 654` and renders in dark and light.
- Every LampRow prop is a control; callbacks are actions (`fn()`). Child props the stories change (the Looper cell's child and face) are reached through LampRow's own props (`looper`), so no child control group is needed.
- The meta sets `parameters: { a11y: { context: { exclude: KNOWN_CONTRAST_FAILURES } } }` (from `./a11y.ts`).
- Plays never assert `aria-pressed` after a click (LampButton's local flip, above) and never assert `data-tip`.

Board args: `B = { partOn: [true, true, false, true], partSounding: [true, true, false, true], swapPart: null, soundOn: false, harmArp: false, leftHold: false, looper: 'off', page: 'panel', shift: false, masterDisabled: false, width: 654 }` (Stage.md › Board fixture).

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `B` | "Part on/off" and "Functions … Launchkey fader buttons 5–9" over hairlines; On, On (lit), Off, On (lit); the divider; Harm/Arp, Sound, L Hold, Looper off; Panel | `Board-{dark,light}.png`: Stage 24,748 654×52 | the group "Fader buttons" holds 9 buttons; "Right 1 on. Long press: swap mode" has `aria-pressed="true"`, "Right 3 off. Long press: swap mode" `false` and reads "Off"; the button "Chord Looper. Long press: loop rec" has `data-face="off"`; the button "Fader page is Panel: click for Style" reads "Panel" and has no `aria-pressed`; the headers read "Part on/off", "Functions", "Launchkey fader buttons 5–9" |
| `PartClicks` | `B` | — | — | click "Right 1 on. Long press: swap mode" → `onpart` called with 0, `onhold` not called; click "Harmony/Arpeggio" → `onharmarp` once; click "Left Hold" → `onlefthold` once; click "Chord Looper. Long press: loop rec" → `onlooper` once; click "Fader page is Panel: click for Style" → `onmaster` once; focus "Right 2 on. Long press: swap mode" and press Enter (`userEvent.keyboard`) → `onpart` last called with 1 |
| `SwapHeld` | `{ ...B, swapPart: 0 }` | Right 1's lamp reads "Swap" (still lit) | — | the button "Right 1 swap held" reads "Swap", `aria-pressed="true"`; click it → `onhold` called with `{ type: 'none' }, false` and `onpart` not called |
| `SoundLit` | `{ ...B, soundOn: true }` | the Sound lamp lit | — | the button "Sound layer" has `aria-pressed="true"`; click → `onhold` called with `{ type: 'none' }, false` |
| `SoundLatch` | `B` | — | — | click "Sound layer. Long press: hold" → `onhold` called with `{ type: 'sound' }, true` |
| `LooperRecArmed` | `{ ...B, looper: 'recArmed' }` | Looper in the waiting face, red outline and label | — | "Chord Looper, recording armed. Long press: loop rec": `data-face="waiting"`, `data-hue="rec"`, `aria-pressed="false"`; click → `onlooper` once |
| `LooperRecording` | `{ ...B, looper: 'recording' }` | Looper solid record red | — | "Chord Looper, recording. Long press: loop rec": `data-face="record"`, `aria-pressed="true"` |
| `LooperLoopArmed` | `{ ...B, looper: 'loopArmed' }` | Looper in the waiting face, lamp-green outline and label | — | "Chord Looper, loop armed. Long press: loop rec": `data-face="waiting"`, `data-hue="lamp"` (excluded from axe, O-contrast) |
| `Looping` | `{ ...B, looper: 'looping' }` | Looper lit | — | "Chord Looper. Long press: loop rec": `data-face="on"`, `aria-pressed="true"` |
| `AllOn` | `{ ...B, partOn: [true, true, true, true], partSounding: [true, true, true, true], harmArp: true, leftHold: true, soundOn: true, looper: 'looping' }` | every lamp lit | — | buttons 1–8 have `aria-pressed="true"` |
| `OnNotSounding` | `{ ...B, partOn: [true, true, true, true], partSounding: [true, false, false, false] }` (Right 1 soloed) | Right 2–Left read "On" but are unlit | — | "Right 2 on. Long press: swap mode" reads "On" with `aria-pressed="false"` |
| `StylePage` | `{ ...B, page: 'style' }` | the master button reads "Style" | — | the button "Fader page is Style: click for Panel" reads "Style" |
| `ShiftHeld` | `{ ...B, shift: true }` | the same faces (Shift changes no face) | — | the buttons are named "Open Channel for Right 1. Long press: swap mode" … "Open Channel for Left. Long press: swap mode", "Chord Looper record", "Fader page button with Shift"; click "Open Channel for Right 2. Long press: swap mode" → `onpartchannel(1)` and `onpart` not called; click "Chord Looper record" → `onlooperrec` once; click "Fader page button with Shift" → `onmastershift` once, `onmaster` not called; click "Harmony/Arpeggio" → `onharmarp` once |
| `MasterDisabled` | `{ ...B, masterDisabled: true }` | the master button's label dimmed | — | the master button has `aria-disabled="true"`; click → neither `onmaster` nor `onmastershift` called |
| `Focused` | `B`; `parameters: { pseudo: { focusVisible: ['[data-cell="1"] button'] } }` | the focus ring on Right 1's lamp only | — | — |

**The long-press checks** (Stage.md Check 11) need fake timers, which a story `play` can't drive cleanly, so they are a vitest component test, `app/src/ui/LampRow/LampRow.test.ts` (`@testing-library/svelte`, `vi.useFakeTimers()`, events with `fireEvent` on the button: `pointerDown` / `pointerUp` with `{ button: 0, buttons: 1, pointerId: 1, clientX: 10, clientY: 10 }`, `contextMenu` with `{ buttons: 0 }`), rendering `LampRow` with `B` and spy callbacks. `rerender` stands for the state's answer.

1. pointerDown on "Right 1 on. Long press: swap mode", advance 350 ms → `onhold` called with `{ type: 'swap', part: 0 }, false`; `rerender({ ...B, swapPart: 0 })` (the state's answer); pointerUp, click → `onpart` not called and `onhold` called once in all (the row was chosen at pointerDown: latched).
2. pointerDown on "Right 1 on. Long press: swap mode", advance 349 ms, pointerUp, click → `onpart(0)`, `onhold` not called.
3. Sound not lit: pointerDown, advance 350 ms → `onhold` called with `{ type: 'sound' }, false` before any pointerUp; `rerender({ ...B, soundOn: true })`; pointerUp → `onhold` last called with `{ type: 'none' }, false`; the click after it calls nothing more.
4. Sound lit (`soundOn: true`): pointerDown, advance 400 ms, pointerUp, click → `onhold` called once, with `{ type: 'none' }, false`.
5. Looper: pointerDown, advance 350 ms → `onlooperrec` once; pointerUp, click → `onlooper` not called.
6. Right-click (`contextMenu`, `buttons: 0`) on "Right 2 on. Long press: swap mode" → `onhold` called once, with `{ type: 'swap', part: 1 }, false`, and the event's `defaultPrevented` is true; on "Chord Looper. Long press: loop rec" → `onlooperrec` once; on "Sound layer. Long press: hold" (not lit) → `onhold` called with `{ type: 'sound' }, false` then `{ type: 'none' }, false`.
7. pointerDown on "Right 1 on. Long press: swap mode", pointerMove 6px, advance 350 ms → nothing fired (moved over 4px).
8. Swap on for Right 1 (`swapPart: 0`): pointerDown on "Right 1 swap held", advance 350 ms, pointerUp, click → `onhold` called once, with `{ type: 'none' }, false`; `onpart` not called.
9. Harm/Arp: pointerDown, advance 350 ms, pointerUp, click → `onharmarp` not called (the no-op long press swallowed the click); then a plain click → `onharmarp` once. The same for L Hold and `onlefthold`.
10. Looper armed (`looper: 'loopArmed'`, the Button child): pointerDown, advance 350 ms → `onlooperrec` once; pointerUp, click → `onlooper` not called.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`); `npx vitest run src/ui/LampRow` passes `LampRow.test.ts` and `holds.test.ts`.
- `npm run shots -- LampRow` passes: `Board` against `crops/Board-{dark,light}.png` at most 0.02; axe finds no violation on any story outside the Known failure it excludes (O-contrast).
- Only listed tokens and the root's two custom properties are used; no inline colours; no literal sizes outside the Visual rules.
- svelte-check and lint pass on the folder.

## Decisions

- D1. Part lamps and Sound report holds through one callback, `onhold(hold, latch)`, carrying `setLayer`'s own payload, so the swap and Sound rules (D18) live here and are tested with fake timers here, and the wiring only maps it to `setLayer` and remembers the latch.
- D2. While a part's swap is on, any press on that part's lamp lets go (a click at once, a long press on its release) and none toggles the part, as today's mixer strip does (`app/src/panels/mixer/Strip.svelte`).
- D3. A right-click is what the shared longpress action makes of it, a long press and its release: on Sound that is a hold for as long as the right button is down (macOS) or an instant (Windows, the Menu key), never a latch; the latch is a plain click. (Replaces the earlier right-click-latches decision.)
- D4. Shift-click on Looper sends `looperRec`, mirroring Shift + fader button 8 (parity), although kit › Lamp row's Shift-click column has "—" for Looper.
- D5. LampButton has no waiting face, so the Looper cell renders LampButton for off, recording (`rec`) and looping, and Button `size: 'cell'` with `waiting` and `hue: 'rec' | 'lamp'` for the two armed modes; the child is replaced when the mode crosses between them, which drops keyboard focus on it (rare: arming comes from a click on it, which keeps focus only within the same child).
- D6. Each button has one tooltip key (an element carries one `data-tip`): Looper `looper.on_off` and the master button `mixer.page`, the first key of each pair kit › Lamp row lists; integration wires them.
- D7. Until #507 (PadsPage2) specs the Style page's lamps, the row on the Style fader page stays the Panel row (the part lamps still toggle the keyboard parts with their own commands) and only the master button reads "Style"; #507 replaces cells 1–8 with the Style parts' mutes.
- D8. Harm/Arp and L Hold get a no-op `onlongpress`, so a long press on them does nothing and its click is swallowed (kit: "—" for their long press), rather than turning into a toggle when the finger lifts.
- D9. The hold-table row is chosen from the props at the press's `pointerdown` (or, for a keyboard press, at its first callback) and kept for that press, so the state's answer to a long press (Swap shown, Sound lit) can't turn the same press's release into a let-go.
- D10. LampButton's local flip is left as is (it has no `controlled` prop): the next state corrects a toggle, and a click that doesn't change `on` keeps the flipped face until `on` next changes; the fix is asked of the primitives lane.
- D11. The divider spans only the 32px row, not the headers, as the board draws it.
- D12. The headers are plain text inside the group, not headings or labels for sub-groups, since the grid can't nest groups without `display: contents`.
- D13. Armed Looper states are the steady waiting face, not flashing (no timer; the kit draws no flash for lamps).
- D14. Shift-click on a part lamp sends what Shift + Panel fader button 1–4 sends on the hardware, `surface.controls[faderButton{i+1}].shiftAction` (`selectPart`), and then opens Channel for the part, so the screen and the hardware select the same part and the screen also shows it.
- D15. app-api.md contradicts itself on the master button's `shiftAction` (kit and LampRow's earlier text: `stepFaderLayer`; app-api's example: `toggleFaderPage`, and its list of buttons whose Shift layer differs omits it). The wiring sends whatever `shiftAction` the state carries, and the Shift name "Fader page button with Shift" names the button, not the action, so it can't lie.
- D16. Every name that has a long press says it ("Right 1 on. Long press: swap mode", "Sound layer. Long press: hold", "Chord Looper. Long press: loop rec"), as LampButton's spec asks of its parent; the board's longer tails ("(knobs edit this part). Shift: open Channel") are left to the tooltip.

## Cross-lane needs

- LampButton (primitives lane): `controlled` (no local flip), so a click that the state refuses or that doesn't change `on` never shows the wrong face (D10).
- Docs / API contract: app-api.md's `masterButton` `shiftAction` (text vs example) and kit › Lamp row's `stepFaderLayer` disagree; one should be fixed (D15).
- Tokens contract PR (orchestrator): the `--rec` on `--g` row and the dark `--lamp` on `--g` row in `contrast.test.ts`.

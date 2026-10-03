# LampRow

## Identity (all stations)

- **Kind:** complex
- **Built from:** LampButton, Button
- **Purpose:** The row of buttons under the faders: each part's On/Off (hold it to swap the part's sound), then Harm/Arp, Sound, L Hold, Looper and the fader page, the same buttons as under the Launchkey's faders.
- **Boards:** `Stage-Dark.dc.html:271-286` (headers `:271-274`, the row `:275-285`); light: `Stage-Light.dc.html:247-262`. Box `Stage 24,748 654×52` (headers `24,748 654×18`, the row `24,768 654×32`). The lamp CSS is `Stage-Dark.dc.html:66` / `Stage-Light.dc.html:42`.
- **Not this component's job:** no store, no API, no Tauri, no timer of its own (long press is the children's `use:longpress`, through their `onlongpress` / `onlongrelease`). It doesn't send: it calls back, and the page wiring sends the command its table names. It doesn't know the pads: releasing a latched Sound after a pad press is the wiring's (Stage.md D18). It doesn't draw a lamp's face (LampButton and Button do). It never imports `use:tip`: the wiring passes it as `tipAction` (L3). On the Style fader page it draws Stage.md's fallback until #507 specs the Style parts' mutes (D7).

## API (Component station)

### Props

`LampRowProps` (every prop below and every callback in Events) is exported from `app/src/ui/LampRow/types.ts`, so FaderBank can take it whole. Every callback is optional: one that isn't passed is simply not called. Every prop gets a JSDoc comment.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `partOn` | `[boolean, boolean, boolean, boolean]` | `[true, true, true, true]` | Each keyboard part's switch (Right 1, Right 2, Right 3, Left): the lamp reads "On" or "Off". |
| `partSounding` | `[boolean, boolean, boolean, boolean]` | `[true, true, true, true]` | Each part sounds: the lamp is lit (Stage.md D12). |
| `swapPart` | `0 \| 1 \| 2 \| 3 \| null` | `null` | The part whose swap hold is on (its lamp reads "Swap"). |
| `soundOn` | `boolean` | `false` | The Sound layer is on (held or latched, from the screen or the hardware): the Sound lamp is lit. |
| `harmArp` | `boolean` | `false` | Harmony/Arpeggio is on. |
| `leftHold` | `boolean` | `false` | Left Hold is on. |
| `looper` | `'off' \| 'recArmed' \| 'recording' \| 'loopArmed' \| 'looping'` | `'off'` | The Chord Looper's mode: the Looper lamp's face (table below). |
| `page` | `'panel' \| 'style'` | `'panel'` | The fader page: the master button's label, and whether `styleButtons` apply. |
| `styleButtons` | `{ label: string; on: boolean; disabled: boolean }[] \| null` | `null` | The Style page's fallback buttons 1–8 (Stage.md › Band), from `surface.controls` `faderButton1` … `faderButton8`. Used only when `page` is `'style'` and this isn't null: cells 1–5, 7 and 8 show entries 0–4, 6, 7; entry 5 is ignored (cell 6 stays Sound on both pages). Exactly 8 entries. |
| `shift` | `boolean` | `false` | Shift is held: a click is a Shift-click. It is the app-only `ui.shift` (in `app/src/lib`, not `AppState`), which already ORs the computer keyboard's Shift and `surface.shift`. Changes what a part lamp's, Looper's and the master button's click calls; no name or face changes (D16). |
| `masterDisabled` | `boolean` | `false` | The master button is disabled (its state-carried action for the current `shift` is null). |
| `width` | `number \| undefined` | — | A fixed width in px. Unset: fills its container (654 in FaderBank). Stories set 654. |
| `tipAction` | `Action<HTMLElement, string> \| undefined` | — | The app's `use:tip`, passed to every child with that child's `tip` (Children; L3). Stories pass `fn()`. |

The part names are fixed and in this order: "Right 1", "Right 2", "Right 3", "Left" (`keyboardParts` is always these four).

### Events

| Callback | Fires when | Payload | The wiring sends |
|---|---|---|---|
| `onpart` | a part lamp is clicked (Space, Enter, or a click that isn't the end of a long press), without Shift, and that part's swap isn't on | `(part: 0 \| 1 \| 2 \| 3)` | `togglePart { part }` |
| `onpartchannel` | a part lamp is clicked with `shift` true | `(part: 0 \| 1 \| 2 \| 3)` | `surface.controls[faderButton{part + 1}].shiftAction` when not null (`selectPart { part }` on the Panel page), then opens Channel for the part (Stage.md D32 interim: `show(part)` in `panels/channel/nav.svelte.ts`) (D14) |
| `onhold` | the swap or Sound hold changes (the hold table) | `(hold: Hold, latch: boolean)`, `Hold` = `{ type: 'none' } \| { type: 'sound' } \| { type: 'swap'; part: 0 \| 1 \| 2 \| 3 }` (exported from `holds.ts`); `latch` is true only when Sound is latched (a click, or a right-click, D3) | `setLayer { layer: hold }`; with `latch`, the wiring also remembers the latch so the next screen pad press releases it (Stage.md D18) |
| `onharmarp` | Harm/Arp clicked (Shift or not) | `()` | `toggleHarmonyArp` |
| `onlefthold` | L Hold clicked (Shift or not) | `()` | `toggleLeftHold` |
| `onlooper` | Looper clicked without Shift | `()` | `looperOnOff` |
| `onlooperrec` | Looper long-pressed (350 ms) or right-clicked, or clicked with `shift` true | `()` | `looperRec` |
| `onmaster` | the master button clicked without Shift (not disabled) | `()` | `surface.controls[masterButton].action` |
| `onmastershift` | the master button clicked with `shift` true (not disabled) | `()` | `surface.controls[masterButton].shiftAction` (whatever it is; D15) |
| `onstylebutton` | a Style page fallback button (cells 1–5, 7, 8 while `styleButtons` apply) is clicked, not disabled | `(i: number /* 0–7, the fader button's index */)` | `surface.controls[faderButton{i + 1}]`'s `shiftAction` with `ui.shift`, else its `action` (kit › Interaction conventions, Parity) |

**Long press and right-click** come from the children's shared `use:longpress` action (`app/src/ui/actions/longpress`): held 350 ms (`--long-press`) without moving more than 4px calls the child's `onlongpress` (pointer still down) and swallows the click that follows; `onlongrelease` comes on that press's `pointerup` or `pointercancel`. A right-click (or the keyboard's Menu key / Shift+F10) is the same pair: `onlongpress`, then `onlongrelease` on the right button's release (macOS) or straight after (Windows, keyboard); the action prevents the browser menu. LampRow tells a right-click from a held press by a capture-phase `contextmenu` listener on the cell wrapper (Choosing the row), which runs before the action's own listener on the button.

**The hold table** (Stage.md D18, D62, Check 11). Each cell's **row** is chosen from the props when its press starts (below); the click, long press and long release of that press all act by that row, so the state's answer to a long press (the lamp now reading "Swap", Sound now lit) can't change what the same press's release does. `—` means nothing is called. "Right-click" is a long press whose press began with a `contextmenu`.

| Row | Chosen when | Click (Shift false) | Click (Shift true) | Long press | Long release | Right-click (long press, then release) |
|---|---|---|---|---|---|---|
| `part` | part cell *i*, `swapPart !== i` | `onpart(i)` | `onpartchannel(i)` | `onhold({ type: 'swap', part: i }, false)` | — (latched until the next click) | as long press, then — |
| `partSwapped` | part cell *i*, `swapPart === i` | `onhold({ type: 'none' }, false)` | `onpartchannel(i)` | — | `onhold({ type: 'none' }, false)` | —, then `onhold({ type: 'none' }, false)` |
| `soundOff` | Sound, `soundOn` false | `onhold({ type: 'sound' }, true)` (latch) | as Shift false | `onhold({ type: 'sound' }, false)` at the 350 ms mark | `onhold({ type: 'none' }, false)` (momentary) | `onhold({ type: 'sound' }, true)` (latch), then — (D3) |
| `soundOn` | Sound, `soundOn` true | `onhold({ type: 'none' }, false)` | as Shift false | — | `onhold({ type: 'none' }, false)` (any press lets go) | —, then `onhold({ type: 'none' }, false)` |
| `harmArp` | Harm/Arp | `onharmarp()` | `onharmarp()` | — (a no-op `onlongpress` is passed, so the long press's click is swallowed, D8) | — | — |
| `leftHold` | L Hold | `onlefthold()` | `onlefthold()` | — (no-op, as Harm/Arp) | — | — |
| `looper` | Looper | `onlooper()` | `onlooperrec()` | `onlooperrec()` | — | `onlooperrec()`, then — |
| `master` | the master button | `onmaster()` | `onmastershift()` | (no `onlongpress` passed: the action is off; a right-click only loses its browser menu) | — | — |
| `styleButton` | cells 1–5, 7, 8 while `styleButtons` apply | `onstylebutton(i)` | `onstylebutton(i)` | (no `onlongpress` passed) | — | — |

Long press and right-click ignore `shift`. Keyboard Space and Enter are a click; there is no keyboard hold (the Menu key is a right-click).

**Choosing the row** (D9). Per cell, LampRow keeps the row of the press in progress and whether it is a right-click:

- the cell's wrapper `div` (Children) handles `onpointerdown` (any button) by choosing the row from the props at that moment and keeping it, with `rightClick` false;
- its `oncontextmenucapture` (capture phase, so before the child's action) chooses the row from the props then and keeps it with `rightClick` true; it doesn't call `preventDefault` (the action does);
- its `onkeydown` drops a kept row (a key press is a new press);
- a child callback that comes with no kept row (a keyboard click) chooses the row from the props then and keeps it, `rightClick` false;
- the click (`ontoggle` / `onpress`) and `onlongrelease` act by the kept row, then drop it; `onlongpress` acts by it and keeps it;
- `shift` is read when the click comes.

The pure parts are in `app/src/ui/LampRow/holds.ts`, tested by `holds.test.ts` (one assertion per cell of the hold table above):

```ts
export type Part = 0 | 1 | 2 | 3
export type Hold = { type: 'none' } | { type: 'sound' } | { type: 'swap'; part: Part }
export type Row = 'part' | 'partSwapped' | 'soundOff' | 'soundOn' | 'harmArp' | 'leftHold' | 'looper' | 'master' | 'styleButton'
/** The row for cell 1–9 from the props now. */
export function rowFor(cell: number, p: { swapPart: Part | null; soundOn: boolean; page: 'panel' | 'style'; styleButtons: unknown[] | null }): Row
export type Effect =
  | { call: 'onpart' | 'onpartchannel'; part: Part }
  | { call: 'onhold'; hold: Hold; latch: boolean }
  | { call: 'onstylebutton'; index: number }
  | { call: 'onharmarp' | 'onlefthold' | 'onlooper' | 'onlooperrec' | 'onmaster' | 'onmastershift' }
  | null
/** What a click, a long press and a long release do; `cell` is 1–9 (the part is cell − 1 for cells 1–4; the style button's index is cell − 1). */
export function clickEffect(row: Row, cell: number, shift: boolean): Effect
export function longPressEffect(row: Row, cell: number, rightClick: boolean): Effect
export function longReleaseEffect(row: Row, cell: number, rightClick: boolean): Effect
```

### Children

The row is a 9-column grid of cell wrappers: each a `div` with `data-cell="1"` … `"9"`, `min-width: 0`, `display: flex` (its child fills it), holding one child and handling `onpointerdown`, `oncontextmenucapture` and `onkeydown` as above. Every child gets `tipAction` and the `tip` below (L3).

| Cell | Child | Props passed | Callbacks |
|---|---|---|---|
| 1–4 (part *i*) | `LampButton` | `{ label: swapPart === i ? 'Swap' : partOn[i] ? 'On' : 'Off', on: partSounding[i], size: 'cell', name: <Accessible names>, tip: swapPart === i ? 'part.swap' : PART_TIP[i] }` (`PART_TIP` = `part.right1.on`, `part.right2.on`, `part.right3.on`, `part.left.on`) | `ontoggle` → the click (its `on` payload is ignored); `onlongpress`, `onlongrelease` → the hold table |
| 5 | `LampButton` | `{ label: 'Harm/Arp', on: harmArp, size: 'cell', name: 'Harmony/Arpeggio', tip: 'harmony.switch' }` | `ontoggle` → click; `onlongpress: () => {}` (no-op, D8) |
| 6 | `LampButton` | `{ label: 'Sound', on: soundOn, size: 'cell', name: 'Sound', tip: 'launchkey.sound' }` | `ontoggle`, `onlongpress`, `onlongrelease` → the hold table |
| 7 | `LampButton` | `{ label: 'L Hold', on: leftHold, size: 'cell', name: 'Left Hold', tip: 'detection.left_hold' }` | `ontoggle` → click; `onlongpress: () => {}` (no-op, D8) |
| 8 | `LampButton` | `{ label: 'Looper', size: 'cell', name: <below>, tip: 'looper.on_off', ...LOOPER_FACE[looper] }` | `ontoggle` → the click; `onlongpress` → `onlooperrec()`; `onlongrelease` → nothing |
| 9 | `Button` | `{ label: page === 'panel' ? 'Panel' : 'Style', size: 'cell', disabled: masterDisabled, name: <below>, tip: 'mixer.page' }` (no `pressed`: no `aria-pressed`) | `onpress` → the click; no `onlongpress` |
| 1–5, 7, 8 while `styleButtons` apply | `LampButton` | `{ label: s.label, on: s.on, disabled: s.disabled, size: 'cell', name: s.label, tip: 'mixer.style.mute' }` with `s = styleButtons[cell − 1]` | `ontoggle` → the click; no `onlongpress` |

`LOOPER_FACE`, the Looper lamp's face by `looper` (Stage.md D13, D49, kit › Lamp row; LampButton's `waiting` and `hue`):

| `looper` | LampButton props | Face (`data-face`) | `aria-pressed` |
|---|---|---|---|
| `off` | `{ on: false }` | `off` | false |
| `recArmed` | `{ on: false, waiting: true, hue: 'rec' }` | `waiting`, `data-hue="rec"`: `--rec` outline and label | false |
| `recording` | `{ on: true, rec: true }` | `record` (solid `--rec`, `--solid-ink` label) | true |
| `loopArmed` | `{ on: false, waiting: true, hue: 'lamp' }` | `waiting`, `data-hue="lamp"`: `--lamp-line` outline and label | false |
| `looping` | `{ on: true }` | `on` (lamp) | true |

**LampButton is controlled** (Stage.md D49): `aria-pressed` follows `on` alone and a click changes nothing until the next state. LampButton's own spec still describes a local flip until `on` changes; plays here therefore never assert `aria-pressed` after a click, so they pass either way (D10).

### Accessible names

Kit › Lamp row: the state is in `aria-pressed` and the face, so a name says what the button is; what a long press or Shift does is in its tooltip (D16).

- **Part lamp *i*:** the part, "Right 1" … "Left"; while its swap is on "{part}, swap held" ("Right 1, swap held"). `aria-pressed` is LampButton's (`partSounding[i]`).
- **Harm/Arp** "Harmony/Arpeggio"; **Sound** "Sound"; **L Hold** "Left Hold".
- **Looper:** "Looper", plus ", recording armed" or ", loop armed" for the armed modes (which `aria-pressed` can't say): "Looper, loop armed". Recording and looping are `aria-pressed="true"`.
- **Master button:** `"Fader page is {Panel|Style}: click for {Style|Panel}"` ("Fader page is Panel: click for Style", the board's).
- **Style page fallback buttons:** the label as the state sends it.

### Visual rules

The root is a column, `width` (or its container's) × 52: the **headers** (`--header-height`), a `--space-2` gap, the **row** (`--control-height`, 32).

**Geometry** declared once as custom properties on the root (axiom 2): `--header-height: 18px` (the headers row), `--header-line: 16px` (the header text's line height). Everything else uses `scale.css` tokens.

- **Headers:** a grid, `grid-template-columns: repeat(9, minmax(0, 1fr))`, `column-gap: var(--space-8)`, height `--header-height`, no wrap. Two cells, each stretched to the full 18 (`align-self: stretch`), `box-sizing: border-box`, `border-bottom: var(--line-width) solid var(--line)`, no padding, `--text-12` / `--weight-regular`, line-height `--header-line`, `--m`; the text sits at the top of its box (its line box is y 0–16, y 16 is empty, the hairline is y 17), as the board draws it. "Part on/off" spans columns 1–4 (`grid-column: 1 / 5`); the other spans columns 5–9 (`grid-column: 5 / 10`), `display: flex`: "Functions" at the left and "Launchkey fader buttons 5–9" (an en dash) pushed right (`margin-left: auto`). Plain text, not headings, not focusable. The headers don't change on the Style page.
- **Row:** `margin-top: var(--space-2)`, height `--control-height`, `position: relative`, a grid `repeat(9, minmax(0, 1fr))` with `gap: var(--space-8)`; each cell wrapper fills its cell. At 654 wide a column is `(654 − 64) / 9 = 65.556`.
- **Divider:** an `aria-hidden` span, `--line`, `position: absolute; top: 0; bottom: 0; width: var(--line-width); left: calc((100% - 8 * var(--space-8)) * 4 / 9 + 3 * var(--space-8) + var(--space-4))`: the middle of the gap between columns 4 and 5 (4 columns, 3 gaps and 4px; at 654 that is 290.22). Drawn on the row only, not the headers (the headers' two hairlines already split there).
- **Tokens used (own):** `--line`, `--line-width`, `--m`, `--text-12`, `--weight-regular`, `--space-2`, `--space-4`, `--space-8`, `--control-height`, `--font-sans`. The children use their own (LampButton's `--lamp-line` for the loop-armed face). No new tokens.
- **Size:** `width` × 52; no size variants here (the half size is added to this spec later).
- **States drawn by:** the children's faces (the tables above); LampRow itself draws no state. The Looper's five faces and the Swap label are stories.
- **Type:** headers DM Sans 12 / 400, sentence case as written, tabular numerals.
- **Contrast** (text on its surface, AA 4.5:1):

  | Pair | Where | Dark | Light | |
  |---|---|---|---|---|
  | `--m` on `--g` | headers | 6.25 | 5.24 | row exists |
  | `--rec` on `--g` | Looper rec-armed (LampButton waiting face over the ground) | 5.67 | 5.27 | LampButton's new row (tokens PR) |
  | `--lamp-line` on `--g` | Looper loop-armed | 13.24 | 4.53 | LampButton's new row and token (tokens PR) |

  The lamps' own pairs (`--m` on `--btn`, `--lamp-ink` on `--lamp`, `--solid-ink` on `--rec`, `--t2` on `--btn`) are LampButton's and Button's. No known failures and no `a11y` exclude (L2): the loop-armed lime is LampButton's `--lamp-line` (LampButton D15), which passes; a disabled Style page button's `--d` label is exempt (`aria-disabled`, Stage.md D47).
- **Motion:** none. (The hardware flashes REC and ON/OFF when armed; the screen draws armed as the steady waiting face, D13.)

### Accessibility

- **Role and name:** a `div` with `role="group"` and `aria-label="Fader buttons"`; inside, nine buttons with the names above, in grid order. The headers are visible text inside the group.
- **Keyboard:** Tab walks the nine buttons left to right; Space or Enter is a click (Shift as `shift` says). The Menu key or Shift+F10 on a focused button is a right-click (the action).
- **Tooltip ids** (passed as each child's `tip`, with `tipAction`): part lamps `part.right1.on`, `part.right2.on`, `part.right3.on`, `part.left.on`, and `part.swap` while that part's swap is on; Harm/Arp `harmony.switch`; Sound `launchkey.sound`; L Hold `detection.left_hold`; Looper `looper.on_off` (its body names the long press, Stage.md C5; `looper.rec` stays the Looper page's key, D6); master `mixer.page` (its body names the Shift-click, C5; `mixer.layer` is reached from the layer tabs, D6); the Style page's fallback buttons `mixer.style.mute`. All exist in `app/src/help/tooltips.ts`; the `looper.on_off` and `mixer.page` body changes are C5's.
- **Launchkey:** part lamps = Panel fader buttons 1–4 (hold + knob: swap; Shift: `selectPart`, D14); Harm/Arp = fader button 5; Sound = fader button 6 (hold); L Hold = fader button 7; Looper = fader button 8 (Shift + 8: REC); master button = the button under the master fader (Shift: its `shiftAction`). On the Style page fader buttons 1–8 are the state's.

### From the state

The page wiring (`app/src/pages/StageWiring.svelte`) fills the props from `AppState` and the app-only `ui` state, and sends what each callback names. Nothing here imports `AppState`. `ctl(id)` is the `surface.controls` entry with that `id`.

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
| `styleButtons` | `mixer.faderPage === 'style'` ? for *i* 0–7, with `c = ctl('faderButton{i + 1}')`: `{ label: c.label, on: c.level === 'bright', disabled: (ui.shift ? c.shiftAction : c.action) === null }` : `null` |
| `shift` | `ui.shift`: app-only, in `app/src/lib`, not `AppState`; it already ORs the computer keyboard's Shift and `surface.shift` |
| `masterDisabled` | `(ui.shift ? ctl('masterButton').shiftAction : ctl('masterButton').action) === null` |
| `tipAction` | the app's `tip` action (`app/src/lib`) |
| `onpart(i)` | `togglePart { part: i }` |
| `onpartchannel(i)` | `ctl('faderButton{i + 1}').shiftAction` if not null, then Channel for part *i* (D32 interim: `show(i)` in `panels/channel/nav.svelte.ts`) (D14) |
| `onhold(hold, latch)` | `setLayer { layer: hold }`; keeps `soundLatched = latch` for a Sound hold, clears it on `none` |
| `onharmarp()` | `toggleHarmonyArp` |
| `onlefthold()` | `toggleLeftHold` (a refusal under Manual Bass shows on the status line) |
| `onlooper()` | `looperOnOff` |
| `onlooperrec()` | `looperRec` |
| `onmaster()` | `ctl('masterButton').action` |
| `onmastershift()` | `ctl('masterButton').shiftAction` |
| `onstylebutton(i)` | `ui.shift ? ctl('faderButton{i + 1}').shiftAction : ctl('faderButton{i + 1}').action` |

## Stories (Story station)

- **Title:** `Components/LampRow`.
- **Layout:** `centered`; every story sets `width: 654` and renders in dark and light.
- The meta's `args` are `B` plus every callback and `tipAction` as `fn()`. Every LampRow prop is a control (`partOn`, `partSounding`, `styleButtons` objects; `swapPart` a select of none / 0–3; `looper` and `page` selects; `soundOn`, `harmArp`, `leftHold`, `shift`, `masterDisabled` booleans; `width` a number); callbacks and `tipAction` are actions. Child props the stories change (the Looper lamp's face) are reached through LampRow's own props (`looper`), so no child control group is needed.
- No `a11y` exclude.
- Plays never assert `aria-pressed` after a click (D10).
- Pointer plays (L4): `fireEvent.pointerDown(button, { pointerId: 1, button: 0, clientX: 0, clientY: 0 })`; long presses in plays use real time with `waitFor(…, { timeout: 1000 })`.

Board args: `B = { partOn: [true, true, false, true], partSounding: [true, true, false, true], swapPart: null, soundOn: false, harmArp: false, leftHold: false, looper: 'off', page: 'panel', styleButtons: null, shift: false, masterDisabled: false, width: 654 }` (Stage.md › Board fixture). `S = stylePageFaderBank.lamps` from `../FaderBank/FaderBank.fixtures`, plus `width: 654`.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `B` | "Part on/off" and "Functions … Launchkey fader buttons 5–9" over hairlines; On, On (lit), Off, On (lit); the divider; Harm/Arp, Sound, L Hold, Looper off; Panel | `Board-{dark,light}.png`: Stage 24,748 654×52 | the group "Fader buttons" holds 9 buttons; "Right 1" has `aria-pressed="true"` and `data-tip="part.right1.on"`; "Right 3" has `aria-pressed="false"` and reads "Off"; the button "Looper" has `data-face="off"` and `data-tip="looper.on_off"`; the button "Fader page is Panel: click for Style" reads "Panel", has `data-tip="mixer.page"` and no `aria-pressed`; the headers read "Part on/off", "Functions", "Launchkey fader buttons 5–9"; `tipAction` was called 9 times, once with the "Sound" button and `'launchkey.sound'` |
| `PartClicks` | `B` | — | — | click "Right 1" → `onpart` called with 0, `onhold` not called; click "Harmony/Arpeggio" → `onharmarp` once; click "Left Hold" → `onlefthold` once; click "Looper" → `onlooper` once; click "Fader page is Panel: click for Style" → `onmaster` once; focus "Right 2" and press Enter (`userEvent.keyboard`) → `onpart` last called with 1 |
| `SwapHeld` | `{ ...B, swapPart: 0 }` | Right 1's lamp reads "Swap" (still lit) | — | the button "Right 1, swap held" reads "Swap", has `aria-pressed="true"` and `data-tip="part.swap"`; click it → `onhold` called with `{ type: 'none' }, false` and `onpart` not called |
| `SoundLit` | `{ ...B, soundOn: true }` | the Sound lamp lit | — | the button "Sound" has `aria-pressed="true"`; click → `onhold` called with `{ type: 'none' }, false` |
| `SoundLatch` | `B` | — | — | click "Sound" → `onhold` called with `{ type: 'sound' }, true` |
| `LooperRecArmed` | `{ ...B, looper: 'recArmed' }` | Looper in the waiting face, red outline and label | — | "Looper, recording armed": `data-face="waiting"`, `data-hue="rec"`, `aria-pressed="false"`; click → `onlooper` once |
| `LooperRecording` | `{ ...B, looper: 'recording' }` | Looper solid record red | — | "Looper": `data-face="record"`, `aria-pressed="true"` |
| `LooperLoopArmed` | `{ ...B, looper: 'loopArmed' }` | Looper in the waiting face, lime (`--lamp-line`) outline and label | — | "Looper, loop armed": `data-face="waiting"`, `data-hue="lamp"` |
| `Looping` | `{ ...B, looper: 'looping' }` | Looper lit | — | "Looper": `data-face="on"`, `aria-pressed="true"` |
| `AllOn` | `{ ...B, partOn: [true, true, true, true], partSounding: [true, true, true, true], harmArp: true, leftHold: true, soundOn: true, looper: 'looping' }` | every lamp lit | — | buttons 1–8 have `aria-pressed="true"` |
| `OnNotSounding` | `{ ...B, partOn: [true, true, true, true], partSounding: [true, false, false, false] }` (Right 1 soloed) | Right 2–Left read "On" but are unlit | — | "Right 2" reads "On" with `aria-pressed="false"` |
| `StylePage` | `S` | the Style page fallback: RHY1, RHY2, BASS, CHD1, CHD2 (unlit), Sound, PHR1, PHR2, and the master button reading "Style" | — (no board draws the Style page) | the buttons "RHY1" … "PHR2" exist with `data-tip="mixer.style.mute"`; "CHD2" has `aria-pressed="false"`; the 6th button is "Sound"; click "RHY2" → `onstylebutton(1)` and `onpart` not called; click "PHR1" → `onstylebutton(6)`; the button "Fader page is Style: click for Panel" reads "Style" |
| `StylePageNoStyle` | `{ ...B, page: 'style' }` (`styleButtons` null) | the Panel row with the master button reading "Style" | — | the button "Fader page is Style: click for Panel" reads "Style"; the button "Right 1" exists |
| `ShiftHeld` | `{ ...B, shift: true }` | the same faces and names (Shift changes neither) | — | click "Right 2" → `onpartchannel(1)` and `onpart` not called; click "Looper" → `onlooperrec` once, `onlooper` not called; click "Fader page is Panel: click for Style" → `onmastershift` once, `onmaster` not called; click "Harmony/Arpeggio" → `onharmarp` once |
| `MasterDisabled` | `{ ...B, masterDisabled: true }` | the master button's label dimmed | — | the master button has `aria-disabled="true"`; click → neither `onmaster` nor `onmastershift` called |
| `Focused` | `B`; `parameters: { pseudo: { focusVisible: ['[data-cell="1"] button'] } }` | the focus ring on Right 1's lamp only | — | — |

**The long-press checks** (Stage.md Check 11) need fake timers and rerenders, which a story `play` can't drive cleanly, so they are a vitest component test, `app/src/ui/LampRow/LampRow.test.ts` (`@testing-library/svelte`, `vi.useFakeTimers()`, events with `fireEvent` on the button: `pointerDown` / `pointerUp` with `{ button: 0, buttons: 1, pointerId: 1, clientX: 10, clientY: 10 }`, `contextMenu` with `{ buttons: 0 }`), rendering `LampRow` with `B` and spy callbacks. `rerender` stands for the state's answer.

1. pointerDown on "Right 1", advance 350 ms → `onhold` called with `{ type: 'swap', part: 0 }, false`; `rerender({ ...B, swapPart: 0 })` (the state's answer); pointerUp, click → `onpart` not called and `onhold` called once in all (the row was chosen at pointerDown: latched).
2. pointerDown on "Right 1", advance 349 ms, pointerUp, click → `onpart(0)`, `onhold` not called.
3. Sound not lit: pointerDown, advance 350 ms → `onhold` called with `{ type: 'sound' }, false` before any pointerUp; `rerender({ ...B, soundOn: true })`; pointerUp → `onhold` last called with `{ type: 'none' }, false`; the click after it calls nothing more.
4. Sound lit (`soundOn: true`): pointerDown, advance 400 ms, pointerUp, click → `onhold` called once, with `{ type: 'none' }, false`.
5. Looper: pointerDown, advance 350 ms → `onlooperrec` once; pointerUp, click → `onlooper` not called.
6. Right-click (`contextMenu`, `buttons: 0`) on "Right 2" → `onhold` called once, with `{ type: 'swap', part: 1 }, false`, and the event's `defaultPrevented` is true; on "Looper" → `onlooperrec` once; on "Sound" (not lit) → `onhold` called once, with `{ type: 'sound' }, true` (latched, D3); with `soundOn: true`, a right-click on "Sound" → `onhold` called with `{ type: 'none' }, false`.
7. pointerDown on "Right 1", pointerMove 6px, advance 350 ms → nothing fired (moved over 4px).
8. Swap on for Right 1 (`swapPart: 0`): pointerDown on "Right 1, swap held", advance 350 ms, pointerUp, click → `onhold` called once, with `{ type: 'none' }, false`; `onpart` not called.
9. Harm/Arp: pointerDown, advance 350 ms, pointerUp, click → `onharmarp` not called (the no-op long press swallowed the click); then a plain click → `onharmarp` once. The same for L Hold and `onlefthold`.
10. Looper armed (`looper: 'loopArmed'`): pointerDown, advance 350 ms → `onlooperrec` once; pointerUp, click → `onlooper` not called.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`); `npx vitest run src/ui/LampRow` passes `LampRow.test.ts` and `holds.test.ts`.
- `npm run shots -- LampRow` passes: `Board` against `crops/Board-{dark,light}.png` at most 0.02; axe finds no violation on any story.
- Only listed tokens and the root's two custom properties are used; no inline colours; no literal sizes outside the Visual rules.
- svelte-check and lint pass on the folder.

## Decisions

- D1. Part lamps and Sound report holds through one callback, `onhold(hold, latch)`, carrying `setLayer`'s own payload, so the swap and Sound rules (Stage.md D18) live here and are tested with fake timers here, and the wiring only maps it to `setLayer` and remembers the latch.
- D2. While a part's swap is on, any press on that part's lamp lets go (a click at once, a long press on its release) and none toggles the part, as today's mixer strip does (`app/src/panels/mixer/Strip.svelte`).
- D3. A right-click on Sound not lit latches it (`onhold(sound, true)`, nothing on its release), as Stage.md D62 and kit › Interaction conventions say; LampRow tells a right-click by its capture-phase `contextmenu` on the cell, since the shared action reports a right-click as a long press with a release (longpress D5). Every other right-click is the long press and its release (replaces the earlier "Sound held while the right button is down").
- D4. Shift-click on Looper sends `looperRec`, mirroring Shift + fader button 8 (parity), although kit › Lamp row's Shift-click column has "—" for Looper.
- D5. The Looper cell is always a LampButton; the armed modes are LampButton's waiting face (`waiting`, `hue: 'rec' | 'lamp'`, LampButton D13), so the child is never swapped and focus stays (replaces the earlier Button child for armed modes).
- D6. Each button has one tooltip key (an element carries one `data-tip`): Looper `looper.on_off` and the master button `mixer.page`, the first key of each pair kit › Lamp row lists; C5 rewrites their bodies to name the long press and the Shift-click.
- D7. On the Style fader page, with `styleButtons` given, cells 1–5, 7 and 8 are Stage.md › Band's fallback (the fader buttons' state labels, lit when `bright`, sending their `action` or, with Shift, `shiftAction`, disabled when null, `mixer.style.mute`), cell 6 stays Sound and cell 9 the master button; without `styleButtons` the Panel row stays. #507 replaces the fallback.
- D8. Harm/Arp and L Hold get a no-op `onlongpress`, so a long press on them does nothing and its click is swallowed (kit: "—" for their long press), rather than turning into a toggle when the finger lifts.
- D9. The hold-table row is chosen from the props at the press's `pointerdown` or `contextmenu` (or, for a keyboard press, at its first callback) and kept for that press, so the state's answer to a long press (Swap shown, Sound lit) can't turn the same press's release into a let-go.
- D10. LampButton is controlled per Stage.md D49; LampButton's spec still describes a local flip, so plays never assert `aria-pressed` after a click and pass either way.
- D11. The divider spans only the 32px row, not the headers, as the board draws it.
- D12. The headers are plain text inside the group, not headings or labels for sub-groups, since the grid can't nest groups without `display: contents`.
- D13. Armed Looper states are the steady waiting face, not flashing (no timer; the kit draws no flash for lamps).
- D14. Shift-click on a part lamp sends what Shift + Panel fader button 1–4 sends on the hardware, `ctl('faderButton{i+1}').shiftAction` (`selectPart`), and then opens Channel for the part, so the screen and the hardware select the same part and the screen also shows it.
- D15. app-api.md's example gives the master button's `shiftAction` as `toggleFaderPage`, while kit › Lamp row and Stage.md D59 say Shift-click steps the layer (`stepFaderLayer`). The wiring sends whatever `shiftAction` the state carries, so the screen and the hardware agree; the name doesn't describe the Shift-click, so it can't be wrong.
- D16. Names follow kit › Lamp row: the part ("Right 1"), since the lit state is `aria-pressed`, and Stage.md Check 11 finds the lamp by that name; abbreviations are spelled out ("Harmony/Arpeggio", "Left Hold"); states `aria-pressed` can't carry are added ("Right 1, swap held", "Looper, loop armed"). Long press and Shift aren't in the names (they are in the tooltips), and Shift doesn't rename anything (replaces the earlier "Right 1 on. Long press: swap mode" names).
- D17. L3: LampRow sets each child's `tip` from its fixed table and passes its one `tipAction` to all nine (replaces the earlier "no child gets a tip").

Follow-ups: LampButton's spec should drop its local flip for Stage.md D49 (D10); kit › Lamp row's "label no-wrap with ellipsis" needs LampButton `cell` to ellipsize long state labels on the Style page fallback; app-api.md's `masterButton` `shiftAction` example should match Stage.md D59 (D15); Stage.md D62 and longpress D5 should name LampRow's Sound right-click latch (D3).

# LampRow

## Identity (all stations)

- **Kind:** complex
- **Built from:** LampButton, Button
- **Purpose:** The row of buttons under the faders: each part's On/Off (hold it to swap the part's sound), then Harm/Arp, Sound, L Hold, Looper and the fader page, the same buttons as under the Launchkey's faders.
- **Boards:** `Stage-Dark.dc.html:271-286` (headers `:271-274`, the row `:275-285`); light: `Stage-Light.dc.html:247-262`. Box `Stage 24,748 654×52` (headers `24,748 654×18`, the row `24,768 654×32`). The lamp CSS is `Stage-Dark.dc.html:66` / `Stage-Light.dc.html:42`.
- **Not this component's job:** no store, no API, no Tauri, no timer of its own (long press is LampButton's `use:longpress`). It doesn't send: it calls back, and the page wiring sends the command its table names. It doesn't know the pads: releasing a latched Sound after a pad press is the wiring's (Stage.md D18). It doesn't draw a lamp's face (LampButton and Button do). On the Style fader page it doesn't draw the Style parts' mutes (#507, D7).

## API (Component station)

### Props

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `partOn` | `[boolean, boolean, boolean, boolean]` | `[true, true, true, true]` | Each keyboard part's switch (Right 1, Right 2, Right 3, Left): the lamp reads "On" or "Off". |
| `partSounding` | `[boolean, boolean, boolean, boolean]` | `[true, true, true, true]` | Each part sounds: the lamp is lit (Stage.md D12). |
| `swapPart` | `0 \| 1 \| 2 \| 3 \| null` | `null` | The part whose swap hold is on (its lamp reads "Swap"). |
| `soundOn` | `boolean` | `false` | The Sound layer is on (held or latched, from the screen or the hardware): the Sound lamp is lit. |
| `harmArp` | `boolean` | `false` | Harmony/Arpeggio is on. |
| `leftHold` | `boolean` | `false` | Left Hold is on. |
| `looper` | `'off' \| 'recArmed' \| 'recording' \| 'loopArmed' \| 'looping'` | `'off'` | The Chord Looper's mode: chooses the Looper lamp's face (table below). |
| `page` | `'panel' \| 'style'` | `'panel'` | The fader page: the master button's label. |
| `shift` | `boolean` | `false` | Shift is held (`ui.shift`): a click is a Shift-click (lane convention 4). Changes the part lamps', Looper's and master button's click and their accessible names. |
| `masterDisabled` | `boolean` | `false` | The master button is disabled (its state-carried action for the current `shift` is null). |
| `width` | `number \| undefined` | — | A fixed width in px. Unset: fills its container (654 in FaderBank). Stories set 654. |

The part names are fixed and in this order: "Right 1", "Right 2", "Right 3", "Left" (`keyboardParts` is always these four).

### Events

| Callback | Fires when | Payload | The wiring sends |
|---|---|---|---|
| `onpart` | a part lamp is clicked (Space, Enter, or a click released before 350 ms), without Shift, and that part's swap isn't on | `(part: 0 \| 1 \| 2 \| 3)` | `togglePart { part }` |
| `onpartchannel` | a part lamp is clicked with `shift` true | `(part)` | opens Channel for the part (Stage.md D32 interim: `ChannelView`, `panels/channel/nav.svelte.ts` `show(part)`) |
| `onhold` | the swap or Sound hold changes (rules below) | `(hold: { type: 'none' } \| { type: 'sound' } \| { type: 'swap'; part: 0 \| 1 \| 2 \| 3 }, latch: boolean)`; `latch` is true only for a Sound click that latches it | `setLayer { layer: hold }`; with `latch`, the wiring also remembers the latch so the next screen pad press releases it (D18) |
| `onharmarp` | Harm/Arp clicked (Shift or not) | `()` | `toggleHarmonyArp` |
| `onlefthold` | L Hold clicked (Shift or not) | `()` | `toggleLeftHold` |
| `onlooper` | Looper clicked without Shift | `()` | `looperOnOff` |
| `onlooperrec` | Looper long-pressed (350 ms) or right-clicked, or clicked with `shift` true | `()` | `looperRec` |
| `onmaster` | the master button clicked without Shift (not disabled) | `()` | `surface.controls[masterButton].action` (`toggleFaderPage`) |
| `onmastershift` | the master button clicked with `shift` true (not disabled) | `()` | `surface.controls[masterButton].shiftAction` (`stepFaderLayer { delta: 1 }` per app-api.md's text) |

**The holds** (Stage.md D18, Check 11). Long press is `use:longpress` inside LampButton: 350 ms (`--long-press`) held without moving more than 4px fires `onlongpress` (pointer still down) and swallows the click that follows; `onlongrelease` comes on the pointerup or pointercancel of a press that fired. Right-click (`contextmenu`, which LampRow `preventDefault`s) does what long press does, except on Sound (below).

| Lamp | Click (Shift false) | Long press | Long release | Right-click |
|---|---|---|---|---|
| Part *i*, swap not on for *i* | `onpart(i)` | `onhold({ type: 'swap', part: i }, false)` (stays on after release: latched) | — | as long press |
| Part *i*, swap on for *i* (`swapPart === i`) | `onhold({ type: 'none' }, false)` | — | `onhold({ type: 'none' }, false)` | `onhold({ type: 'none' }, false)` |
| Sound, `soundOn` false | `onhold({ type: 'sound' }, true)` (latch) | `onhold({ type: 'sound' }, false)` at the 350 ms mark, pointer still down | `onhold({ type: 'none' }, false)` (momentary) | as a click (latch): right-click has no hold to measure |
| Sound, `soundOn` true | `onhold({ type: 'none' }, false)` | — | `onhold({ type: 'none' }, false)` (any press lets go on release) | `onhold({ type: 'none' }, false)` |
| Looper | `onlooper()` | `onlooperrec()` | — | `onlooperrec()` |
| Harm/Arp, L Hold | `onharmarp()`, `onlefthold()` | — (a long press does nothing; its click is swallowed) | — | nothing (`preventDefault` only) |

With `shift` true a click on a part lamp calls `onpartchannel(i)` (whatever its swap), on Looper `onlooperrec()`, on Sound, Harm/Arp and L Hold the same as without Shift. Long press and right-click ignore `shift`. Keyboard (Space, Enter) is a click; there is no keyboard long press, and the context-menu key (which fires `contextmenu`) is the keyboard's way to the right-click column.

### Children

The row is a 9-column grid; every cell holds one child, filling it.

| Cell | Child | Props passed |
|---|---|---|
| 1–4 (part *i*) | `LampButton` | `{ label: swapPart === i ? 'Swap' : partOn[i] ? 'On' : 'Off', on: partSounding[i], size: 'cell', name: <part name, below>, controlled: true, tip: swapPart === i ? 'part.swap' : PART_TIP[i] }`; callbacks `ontoggle`, `onlongpress`, `onlongrelease`, `oncontextmenu` wired to the hold table |
| 5 | `LampButton` | `{ label: 'Harm/Arp', on: harmArp, size: 'cell', name: 'Harmony/Arpeggio', controlled: true, tip: 'harmony.switch' }` |
| 6 | `LampButton` | `{ label: 'Sound', on: soundOn, size: 'cell', name: 'Sound layer', controlled: true, tip: 'launchkey.sound' }` |
| 7 | `LampButton` | `{ label: 'L Hold', on: leftHold, size: 'cell', name: 'Left Hold', controlled: true, tip: 'detection.left_hold' }` |
| 8 | `LampButton` | `{ label: 'Looper', size: 'cell', controlled: true, tip: 'looper.on_off', name: <below>, ...LOOPER_FACE[looper] }` |
| 9 | `Button` | `{ label: page === 'panel' ? 'Panel' : 'Style', variant: 'cell', disabled: masterDisabled, name: <below>, tip: 'mixer.page' }`; `onpress` → `shift ? onmastershift() : onmaster()` |

`PART_TIP` = `part.right1.on`, `part.right2.on`, `part.right3.on`, `part.left.on`.

`LOOPER_FACE`, the Looper lamp's face by `looper` (Stage.md D13, kit › Lamp row):

| `looper` | LampButton props | Face (`data-face` on the button) | `aria-pressed` |
|---|---|---|---|
| `off` | `{ on: false }` | off | false |
| `recArmed` | `{ on: false, waiting: true, hue: 'rec' }` | waiting, `--rec` border and label (`data-hue="rec"`) | false |
| `recording` | `{ on: true, rec: true }` | record (solid `--rec`, `--solid-ink` label) | true |
| `loopArmed` | `{ on: false, waiting: true, hue: 'lamp' }` | waiting, `--lamp` border and label (`data-hue="lamp"`) | false |
| `looping` | `{ on: true }` | on (lamp) | true |

`waiting` and `hue` are LampButton props this spec needs (Needs from primitives): the waiting face is transparent, `1px solid` in the hue, label in the hue, weight 400.

### Accessible names

- **Part lamp:** without Shift `"{part} {on|off}"` from `partOn` ("Right 1 on", "Right 3 off"), and while its swap is on `"{part} swap held"` ("Right 1 swap held"); with `shift` true `"Open Channel for {part}"`. `aria-pressed` is LampButton's (`partSounding[i]`).
- **Harm/Arp** "Harmony/Arpeggio"; **Sound** "Sound layer"; **L Hold** "Left Hold" (each with `aria-pressed` = on).
- **Looper:** "Chord Looper", plus ", recording armed", ", recording", ", loop armed" for those modes ("Chord Looper, loop armed"); with `shift` true "Chord Looper record".
- **Master button:** without Shift `"Fader page is {Panel|Style}: click for {Style|Panel}"` ("Fader page is Panel: click for Style", the board's); with `shift` true `"Next fader layer"`.

### Visual rules

The root is a column, `width` (or its container's) × 52: the **headers** (18 tall), 2px gap, the **row** (32 tall).

- **Headers:** a grid, `grid-template-columns: repeat(9, minmax(0, 1fr))`, `column-gap: 8px` (`--space-8`), height 18, no wrap. Two cells, each `box-sizing: border-box`, `border-bottom: 1px solid var(--line)`, 12 / 400 (`--text-12`), line-height 16, `--m`: "Part on/off" spanning columns 1–4 (`grid-column: 1 / 5`); and spanning columns 5–9 (`grid-column: 5 / 10`), `display: flex`: "Functions" at the left and "Launchkey fader buttons 5–9" (an en dash) pushed right (`margin-left: auto`). Plain text, not headings, not focusable.
- **Row:** `margin-top: 2px`, height 32, `position: relative`, a grid `repeat(9, minmax(0, 1fr))` with `gap: 8px`; each child fills its cell (`min-width: 0`). At 654 wide a column is `(654 − 64) / 9 = 65.556`.
- **Divider:** a 1px `--line` line, `position: absolute; top: 0; bottom: 0; width: 1px; left: calc((100% − 64px) × 4 / 9 + 28px)`, `aria-hidden`: the middle of the gap between columns 4 and 5 (4 columns, 3 gaps and 4px; at 654 that is 290.22). Drawn on the row only, not the headers (the headers' two hairlines already split there).
- **Tokens used (own):** `--line`, `--m`, `--text-12`, `--weight-regular`, `--space-8`, `--font-sans`. The children use their own.
- **Size:** `width` × 52; no size variants.
- **States drawn by:** the children's faces (the props tables above); LampRow itself draws no state. The Looper's five faces and the Swap label are stories.
- **Type:** headers DM Sans 12 / 400, sentence case as written, tabular numerals.
- **Contrast** (`tokens/contrast.test.ts`): `--m` on `--g` (headers, exists). Rows to add for the children's new faces: `--rec` on `--g` and `--lamp` on `--g` (the waiting Looper's label on the ground: transparent face). `--lamp` on `--g` in light (`#4f8a0e` on `#f2f1ee`) is about 3.7:1, under AA: see D9 (`--rec` on `--g` passes in both themes, about 5.3:1 light, 5.7:1 dark).
- **Motion:** none. (The hardware flashes REC and ON/OFF when armed; the screen draws armed as the steady waiting face, D13.)

### Accessibility

- **Role and name:** a `div` with `role="group"` and `aria-label="Fader buttons"`; inside, nine buttons with the names above, in grid order. The headers are visible text inside the group.
- **Keyboard:** Tab walks the nine buttons left to right; Space or Enter is a click (Shift as `shift` says). The context-menu key on a focused button is a right-click.
- **Tooltip ids** (on each child as `data-tip`, via the child's `tip` prop): part lamps `part.right1.on`, `part.right2.on`, `part.right3.on`, `part.left.on`, and `part.swap` while that part's swap is on; Harm/Arp `harmony.switch`; Sound `launchkey.sound`; L Hold `detection.left_hold`; Looper `looper.on_off` (its body names Shift; `looper.rec` stays the Looper page's key, D6); master `mixer.page` (`mixer.layer`'s body is reached from the layer tabs, D6). All exist in `app/src/help/tooltips.ts`.
- **Launchkey:** part lamps = Panel fader buttons 1–4 (hold + knob: swap; Shift: select the part); Harm/Arp = fader button 5; Sound = fader button 6 (hold); L Hold = fader button 7; Looper = fader button 8 (Shift + 8: REC); master button = the button under the master fader (Shift: step the layer).

### From the state

The page wiring (`app/src/pages/StageWiring.svelte`) fills the props from `AppState` and sends what each callback names. Nothing here imports `AppState`.

| Prop / callback | From `AppState` / sends |
|---|---|
| `partOn[i]` | `keyboardParts[i].on` |
| `partSounding[i]` | `keyboardParts[i].sounding` |
| `swapPart` | `surface.layer.type === 'swap' ? surface.layer.part : null` |
| `soundOn` | `surface.layer.type === 'sound'` |
| `harmArp` | `harmonyArp.on` |
| `leftHold` | `chord.leftHold` |
| `looper` | `looper.mode` |
| `page` | `mixer.faderPage` |
| `shift` | `ui.shift` (app-only; includes the computer keyboard's Shift) |
| `masterDisabled` | `ui.shift ? surface.controls[masterButton].shiftAction == null : surface.controls[masterButton].action == null` (`masterButton` is the control with `id` `"masterButton"`) |
| `onpart(i)` | `togglePart { part: i }` |
| `onpartchannel(i)` | Channel for part *i* (D32 interim: `show(i)` in `panels/channel/nav.svelte.ts`) |
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
- Every LampRow prop is a control; callbacks are actions (`fn()`). Child props the stories change (`LampButton` `waiting`, `hue`) are reached through LampRow's own props (`looper`), so no child control group is needed.

Board args: `B = { partOn: [true, true, false, true], partSounding: [true, true, false, true], swapPart: null, soundOn: false, harmArp: false, leftHold: false, looper: 'off', page: 'panel', shift: false, masterDisabled: false, width: 654 }` (Stage.md › Board fixture).

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `B` | "Part on/off" and "Functions … Launchkey fader buttons 5–9" over hairlines; On, On (lit), Off, On (lit); the divider; Harm/Arp, Sound, L Hold, Looper off; Panel | `Board-{dark,light}.png`: Stage 24,748 654×52 | the group "Fader buttons" holds 9 buttons; "Right 1 on" has `aria-pressed="true"`, "Right 3 off" `false` and reads "Off"; "Looper" button named "Chord Looper" has `data-face="off"`; the button "Fader page is Panel: click for Style" reads "Panel"; the headers read "Part on/off", "Functions", "Launchkey fader buttons 5–9" |
| `PartClicks` | `B` | — | — | click "Right 1 on" → `onpart` called with 0, `onhold` not called; click "Harmony/Arpeggio" → `onharmarp` once; click "Left Hold" → `onlefthold` once; click "Chord Looper" → `onlooper` once; click "Fader page is Panel: click for Style" → `onmaster` once |
| `SwapHeld` | `{ ...B, swapPart: 0 }` | Right 1's lamp reads "Swap" (still lit) | — | the button "Right 1 swap held" reads "Swap", `aria-pressed="true"`, `data-tip="part.swap"`; click it → `onhold` called with `{ type: 'none' }, false` and `onpart` not called |
| `SoundLit` | `{ ...B, soundOn: true }` | the Sound lamp lit | — | "Sound layer" `aria-pressed="true"`; click → `onhold` called with `{ type: 'none' }, false` |
| `SoundLatch` | `B` | — | — | click "Sound layer" → `onhold` called with `{ type: 'sound' }, true` |
| `LooperRecArmed` | `{ ...B, looper: 'recArmed' }` | Looper in the waiting face, red outline and label | — | "Chord Looper, recording armed": `data-face="waiting"`, `data-hue="rec"`, `aria-pressed="false"` |
| `LooperRecording` | `{ ...B, looper: 'recording' }` | Looper solid record red | — | "Chord Looper, recording": `data-face="record"`, `aria-pressed="true"` |
| `LooperLoopArmed` | `{ ...B, looper: 'loopArmed' }` | Looper in the waiting face, lamp-green outline and label | — | "Chord Looper, loop armed": `data-face="waiting"`, `data-hue="lamp"` |
| `Looping` | `{ ...B, looper: 'looping' }` | Looper lit | — | "Chord Looper": `data-face="on"`, `aria-pressed="true"` |
| `AllOn` | `{ ...B, partOn: [true, true, true, true], partSounding: [true, true, true, true], harmArp: true, leftHold: true, soundOn: true, looper: 'looping' }` | every lamp lit | — | buttons 1–8 have `aria-pressed="true"` |
| `OnNotSounding` | `{ ...B, partOn: [true, true, true, true], partSounding: [true, false, false, false] }` (Right 1 soloed) | Right 2–Left read "On" but are unlit | — | "Right 2 on" reads "On" with `aria-pressed="false"` |
| `StylePage` | `{ ...B, page: 'style' }` | the master button reads "Style" | — | the button "Fader page is Style: click for Panel" reads "Style" |
| `ShiftHeld` | `{ ...B, shift: true }` | the same faces (Shift changes no face) | — | the buttons are named "Open Channel for Right 1" … "Open Channel for Left", "Chord Looper record", "Next fader layer"; click "Open Channel for Right 2" → `onpartchannel(1)` and `onpart` not called; click "Chord Looper record" → `onlooperrec` once; click "Next fader layer" → `onmastershift` once, `onmaster` not called; click "Harmony/Arpeggio" → `onharmarp` once |
| `MasterDisabled` | `{ ...B, masterDisabled: true }` | the master button's label dimmed | — | the master button has `aria-disabled="true"`; click → neither `onmaster` nor `onmastershift` called |
| `Focused` | `B`; `parameters: { pseudo: { focusVisible: ['button:first-of-type'] } }` | the focus ring on Right 1's lamp | — | — |

**The long-press checks** (Stage.md Check 11) need fake timers, which a story `play` can't drive cleanly, so they are a vitest component test, `app/src/ui/LampRow/LampRow.test.ts` (`@testing-library/svelte`, `vi.useFakeTimers()`, pointer events with `fireEvent`), rendering `LampRow` with `B` and spy callbacks:

1. pointerdown on "Right 1 on", advance 350 ms → `onhold` called with `{ type: 'swap', part: 0 }, false`; pointerup, click → `onpart` not called and `onhold` called once in all.
2. pointerdown on "Right 1 on", advance 349 ms, pointerup, click → `onpart(0)`, `onhold` not called.
3. Sound not lit: pointerdown, advance 350 ms → `onhold` called with `{ type: 'sound' }, false` before any pointerup; pointerup → `onhold` last called with `{ type: 'none' }, false`; the click after it calls nothing more.
4. Sound lit (`soundOn: true`): pointerdown, advance 400 ms, pointerup → `onhold` called once, with `{ type: 'none' }, false`.
5. Looper: pointerdown, advance 350 ms → `onlooperrec` once; pointerup, click → `onlooper` not called.
6. Right-click (`contextmenu`) on "Right 2 on" → `onhold` with `{ type: 'swap', part: 1 }, false`, and the event's `defaultPrevented` is true; on "Chord Looper" → `onlooperrec`; on "Sound layer" (not lit) → `onhold` with `{ type: 'sound' }, true`.
7. pointerdown on "Right 1 on", pointermove 6px, advance 350 ms → nothing fired (moved over 4px).
8. Swap on for Right 1 (`swapPart: 0`): pointerdown, advance 350 ms, pointerup → `onhold` called once, with `{ type: 'none' }, false`; `onpart` not called.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`); `npx vitest run src/ui/LampRow` passes the long-press test.
- `npm run shots -- LampRow` passes: `Board` against `crops/Board-{dark,light}.png` at most 0.02, axe clean on every story.
- Only listed tokens are used; no inline colours; no literal sizes outside the Visual rules.
- svelte-check and lint pass on the folder.

## Decisions

- D1. Part lamps and Sound report holds through one callback, `onhold(hold, latch)`, carrying `setLayer`'s own payload, so the swap and Sound rules (D18) live here and are tested with fake timers here, and the wiring only maps it to `setLayer` and remembers the latch.
- D2. While a part's swap is on, any press on that part's lamp lets go (a click at once, a long press on its release) and none toggles the part, as today's mixer strip does (`app/src/panels/mixer/Strip.svelte`).
- D3. Right-click on Sound latches or lets go like a click, since a right-click has no hold to measure; on the part lamps and Looper it does what long press does (kit › Interaction conventions).
- D4. Shift-click on Looper sends `looperRec`, mirroring Shift + fader button 8 (parity), although kit › Lamp row's Shift-click column has "—" for Looper.
- D5. Shift-click on Harm/Arp, Sound and L Hold is a plain click, as the hardware's Shift + those buttons does the same as without.
- D6. Each button carries one tooltip key (an element can carry one `data-tip`): Looper `looper.on_off` and the master button `mixer.page`, the first key of each pair kit › Lamp row lists.
- D7. Until #507 (PadsPage2) specs the Style page's lamps, the row on the Style fader page stays the Panel row (the part lamps still toggle the keyboard parts with their own commands) and only the master button reads "Style"; #507 replaces cells 1–8 with the Style parts' mutes.
- D8. The master button mirrors `surface.controls[masterButton]` (`action`, and `shiftAction` with Shift), disabled when that is null, like Pad Bank ▲ ▼ and Track ◀ ▶, so the screen and the hardware can't disagree.
- D9. The Looper's loop-armed waiting face draws its label in `--lamp` on the ground, which in light is about 3.7:1, under AA; it stays (Stage.md D13's face), the story `LooperLoopArmed` configures axe's `color-contrast` to skip `[data-hue="lamp"]`, and the owner is asked (report).
- D10. The lamps are controlled (`controlled: true`): they show exactly `on` from the state and don't flip on click, so a refused toggle (Left under Manual Bass) never shows the wrong face.
- D11. The divider spans only the 32px row, not the headers, as the board draws it.
- D12. The headers are plain text inside the group, not headings or labels for sub-groups, since the grid can't nest groups without `display: contents`.
- D13. Armed Looper states are the steady waiting face, not flashing (no timer; the kit draws no flash for lamps).

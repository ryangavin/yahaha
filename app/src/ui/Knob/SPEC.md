# Knob

## Identity (all stations)

- **Kind:** primitive
- **Built from:** —
- **Purpose:** Shows what one of the Launchkey's eight knobs does and where it stands, and turns it from the screen.
- **Boards:**
  - `Stage-Dark.dc.html:304-313` (the knob column) and its data `:477-494` (names, values, the arc and the tip dot); light: `Stage-Light.dc.html:280-289`, data `:449-466`.
  - Crops: the eight knobs are whole pixels at `730 + 74·i, 476`, 68 × 96 (i = 0…7, knob 1 first) on `docs/design/push/png/Stage-Dark.png` and `Stage-Light.png`.
- **Not this component's job:** no store, no API, no Tauri (lint forbids those imports under `app/src/ui`). It doesn't send `turnKnob` or `resetKnob`: it calls `onturn(delta)` and `onreset()` and the page wiring sends. It doesn't coalesce steps per animation frame (the wiring sums the deltas of one frame into one `turnKnob`). It doesn't read `event.shiftKey` (fine drag comes from the `shift` prop). No timer. No tooltip (wired at integration). The ▲ ▼ page buttons and the header are KnobBank's.

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component. The type `KnobFunction` is declared in `knob.ts` as `string` (the API's union can't be imported here; the functions below treat any unknown string as "other").

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `index` | `number` | `0` | The knob's place, 0–7 (knob 1 is 0). Used in the accessible name ("Knob 1: …") and as `data-knob`. |
| `function` | `string` | `'none'` | What the knob does, as the state names it (`dynamics`, `retriggerRate`, `tempo`, `none`, `swapSound`, …). Picks the plain name (`knobName`), the tempo value rule (`knobValue`) and the No Assign face (`'none'`). |
| `name` | `string` | `''` | The state's full name ("Dynamics Control"); shown only for a function `knobName` has no plain word for. |
| `short` | `string` | `''` | The Genos code ("DynCtrl"), shown under the ring. Not shown for No Assign. |
| `value` | `string` | `''` | The value as the state writes it ("127", "1/8", "Off", "0%", "104 BPM", "23 Rhodes Soft"); empty for No Assign. Split by `knobValue`. |
| `level` | `number \| null` | `null` | Where the knob is, 0–127; null for tempo, No Assign and swap mode's knob 1. |
| `tempo` | `number \| null` | `null` | The tempo in BPM, used for the arc only when `level` is null and `function` is `'tempo'`. |
| `shift` | `boolean` | `false` | Fine drag: one step per 12px instead of 4px (`ui.shift`, which already includes the computer keyboard's Shift). |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onturn` | a pointer move during a drag in which the step count is not 0; a wheel event; ArrowUp/Right, ArrowDown/Left, PageUp, PageDown while focused. Never for No Assign. | `(delta: number)`: the steps of that one event, up/clockwise positive: drag `knobSteps(anchorY − y, shift)`; wheel `+1` for `deltaY < 0`, `−1` for `deltaY > 0` (nothing for 0); arrows `±1`; PageUp `+10`, PageDown `−10`. |
| `onreset` | double-click. Never for No Assign. | `()` |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### From the state

The page wiring (`app/src/pages/StageWiring.svelte`, through KnobBank) feeds each prop and maps each callback; this table is what it transcribes.

| Prop / callback | From `AppState` / sends |
|---|---|
| `index` | the knob's position i in `knobs.knobs` (0–7) |
| `function`, `name`, `short`, `value`, `level` | `knobs.knobs[i].function`, `.name`, `.short`, `.value`, `.level` |
| `tempo` | `transport.tempo` |
| `shift` | `ui.shift` |
| `onturn(delta)` | `turnKnob { knob: i, delta }`, the deltas of one animation frame summed into one command (the wiring's). In swap mode (`surface.layer.type == 'swap'`) the session runs it as `turnSwapKnob` for the held part, so the screen sends the same command. |
| `onreset()` | `resetKnob { knob: i }` |

### Pure functions (`app/src/ui/Knob/knob.ts`)

All exported, no DOM, each with a vitest in `knob.test.ts` asserting the examples.

| Function | Rule | Examples |
|---|---|---|
| `knobFraction(level: number \| null, tempo: number \| null): number` | `level` not null: `clamp(level / 127, 0, 1)`. Else `tempo` not null: `clamp((tempo − 40) / 240, 0, 1)`. Else `0`. (The Knob passes `tempo` only when `function` is `'tempo'`, else null.) No rounding. | `(127, null)` → 1; `(51, null)` → 0.40157…; `(0, null)` → 0; `(null, 104)` → 0.26667 (0.267 to 3 places, Stage.md Check 12); `(null, 300)` → 1; `(null, 20)` → 0; `(null, null)` → 0 |
| `knobDeg(fraction: number): number` | `Math.round(fraction × 270)`: the arc in whole degrees. | 1 → 270; 0.40157 → 108; 0.26667 → 72; 0 → 0 |
| `tipDot(fraction: number): { left: number, top: number }` | `θ = (225 + knobDeg(fraction))°` in radians; the dot's centre is at radius 21 from the ring's centre (22, 22) at angle θ measured clockwise from the top, so the 6px dot's top-left corner is `left = 22 + 21·sin θ − 3`, `top = 22 − 21·cos θ − 3`, each rounded to 0.1 (`Math.round(x × 10) / 10`). px, relative to the ring's 44 × 44 box. | fraction 1 (θ 135°) → `{ left: 33.8, top: 33.8 }` (bottom right); fraction 0 (θ 225°) → `{ left: 4.2, top: 33.8 }` (bottom left); fraction 0.267 (tempo 104, 72°, θ 297°) → `{ left: 0.3, top: 9.5 }`; fraction 51/127 (108°, θ 333°) → `{ left: 9.5, top: 0.3 }` |
| `knobSteps(travel: number, fine: boolean): number` | `Math.trunc(travel / (fine ? 12 : 4))`, with −0 returned as 0. `travel` is px moved up (positive) since the anchor. | `(8, false)` → 2 (Check 12); `(7, false)` → 1; `(3, false)` → 0; `(−5, false)` → −1; `(−3, false)` → 0; `(11, true)` → 0; `(12, true)` → 1; `(24, true)` → 2 |
| `knobName(fn: string, name: string): string` | Stage.md D6: `dynamics` "Dynamics", `retriggerRate` "Retrig rate", `retriggerOnOff` "Retrigger", `trackMuteA` "Mute A", `trackMuteB` "Mute B", `swing` "Swing", `tempo` "Tempo", `splitPoint` "Split", `harmonyArp` "Harm/Arp", `harmonyVolume` "Harm level", `metronomeVolume` "Click level", `swapSound` "Sound", `none` "---"; any other function returns `name` as given. | `('dynamics', 'Dynamics Control')` → "Dynamics"; `('none', 'No Assign')` → "---"; `('swapSound', 'Right 1 Sound')` → "Sound"; `('partReverb', 'Right 1 Reverb')` → "Right 1 Reverb" |
| `splitUnit(value: string): { text: string, unit: string }` | A value ending in a digit then "%" splits off the "%": `/^(.*\d)%$/` → `{ text: <before>, unit: '%' }`; anything else → `{ text: value, unit: '' }`. | "0%" → `{ '0', '%' }`; "38%" → `{ '38', '%' }`; "127" → `{ '127', '' }`; "%" → `{ '%', '' }`; "5.0 kHz" → `{ '5.0 kHz', '' }`; "" → `{ '', '' }` |
| `knobValue(fn: string, value: string): { text: string, unit: string }` | For `fn === 'tempo'`, first drops a trailing " BPM" (`/\s*BPM$/`); then `splitUnit`. | `('tempo', '104 BPM')` → `{ '104', '' }`; `('swing', '0%')` → `{ '0', '%' }`; `('dynamics', '127')` → `{ '127', '' }`; `('none', '')` → `{ '', '' }` |

### Visual rules

- **Tokens used:** `--g`, `--t2`, `--m`, `--d`, `--a`, `--ring-rest` (new, kit › Tokens to add: dark `--grey-24` #3d3d3d, light `--stone-83` #d6d5d1), `--mbg`, `--focus`, `--font-sans`, `--text-12`, `--text-22`, `--weight-light`, `--weight-regular`, `--space-2`, `--line-width`, `--focus-offset`.
- **Size:** a fixed 68 × 96 column, a flex column with `align-items: center`, `text-align: center`, no padding, no background, no border. From the top:
  1. **Name** line: 14 tall, line-height 14, 12px (`--text-12`) regular, `--t2`; the text is `knobName(function, name)`. No Assign: "---" in `--d`.
  2. **Value** line: 22 tall, line-height 22, 22px (`--text-22`) light (`--weight-light`), `--a`; the text is `knobValue(function, value).text`, then, when `unit` is not empty, a unit span `--space-2` (2px) after it: 12px regular, `--a`. No Assign: empty (the line keeps its 22px).
  3. **Ring:** 44 × 44, `margin-top: 2px` (`--space-2`), `border-radius: 50%`, `position: relative`, background `conic-gradient(from 225deg, var(--a) 0deg <deg>deg, var(--ring-rest) <deg>deg 270deg, transparent 270deg 360deg)` with `<deg> = knobDeg(fraction)` and `fraction = knobFraction(level, function === 'tempo' ? tempo : null)`. Over it a disc, `position: absolute; inset: 2px; border-radius: 50%; background: var(--g)`, so the ring reads as a 2px arc. Then the **tip dot**: `position: absolute`, 6 × 6, `border-radius: 50%`, `--a` fill, `left`/`top` from `tipDot(fraction)` in px; `aria-hidden="true"`. The dot shows at every fraction, 0 included. The open quarter (270°–360°, the bottom) is transparent.
  4. **Code** line: 14 tall, line-height 14, 12px regular, `--m`; the text is `short`. No Assign: empty (the line keeps its 14px).
- **States drawn by:**
  - assigned: as above; the arc length follows `fraction`.
  - No Assign (`function` `'none'`): name "---" in `--d`; value and code empty; arc and rest both `--mbg` (`conic-gradient(from 225deg, var(--mbg) 0deg 270deg, transparent 270deg 360deg)`), no tip dot element; `aria-disabled="true"`; default cursor; still focusable.
  - dragging: no change of look; cursor `ns-resize` (Stage.md D35).
  - keyboard focus: a `--line-width` outline in `--focus`, `--focus-offset` outside the 68 × 96 column, on `:focus-visible` only.
  - No hover or pressed look (D35). Cursor `pointer` when assigned, `default` for No Assign.
- **Overflow:** each of the three text lines is `white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 68px`, so a long name ("Right 1 Reverb"), a long value ("23 Rhodes Soft") or a long code ends in "…" inside the column (D6).
- **Type:** DM Sans (`--font-sans`), `font-variant-numeric: tabular-nums`, case as given.
- **Contrast (AA 4.5:1, `tokens/contrast.test.ts`):** add `['--a', '--g', 'knob value and unit (Knob)']`; `--t2` on `--g` (name) and `--m` on `--g` (code) are already listed. `--d` (No Assign) is exempt. The arc is not text.
- **Motion:** none. The arc and dot follow `level` / `tempo`; the component draws what it is given.
- **Test hooks (Stage.md D41):** the root carries `data-knob="<index>"` and `data-arc="<knobDeg(fraction)>"` (whole degrees; No Assign `0`), plus `data-face="disabled"` for No Assign (absent otherwise) and `data-dragging` while a drag is on. The name, value, unit and code elements carry `data-part="name" | "value" | "unit" | "code"` (the unit element exists only when `unit` is not empty); the tip dot carries `data-part="dot"` with its `left`/`top` as an inline style (`left: 0.3px; top: 9.5px`), which jsdom reads. The arc colours (custom properties in a gradient) are checked by the crops.

### Behaviour

- **Drag** (assigned only): `pointerdown` with the primary button sets pointer capture and the anchor `anchorY = clientY`. Each `pointermove` computes `steps = knobSteps(anchorY − clientY, shift)`; when `steps ≠ 0` it calls `onturn(steps)` once and moves the anchor by what was used, `anchorY −= steps × (shift ? 12 : 4)`, so the leftover travel carries into the next move and a Shift change mid-drag never double counts (D4). `pointerup` or `pointercancel` ends the drag and releases capture; nothing is called on release. Horizontal movement is ignored. Example (Stage.md Check 12): press, then one move 8px up → `onturn(2)`; two moves of 4px → `onturn(1)` twice.
- **Wheel** (assigned only): each `wheel` event with `deltaY < 0` calls `onturn(1)`, `deltaY > 0` calls `onturn(-1)`, whatever its size or `deltaMode`; `deltaY` 0 does nothing. The listener is not passive and calls `preventDefault()` so the page doesn't scroll.
- **Double-click** (assigned only): `onreset()`. The two clicks' presses move nothing, so no `onturn` comes with it.
- **Keys** when focused (assigned only): ArrowUp and ArrowRight `onturn(1)`, ArrowDown and ArrowLeft `onturn(-1)`, PageUp `onturn(10)`, PageDown `onturn(-10)` (D3); each calls `preventDefault()` and `stopPropagation()` so the window's ← → (`stepStyle`) don't fire (Stage.md D36). Other keys pass through. No Assign handles no keys (they propagate).

### Accessibility

- **Role and name:** the root is `role="slider"`, `tabindex="0"`, `aria-orientation="vertical"`, `aria-label` "Knob {index + 1}: {knobName}" ("Knob 1: Dynamics"; No Assign "Knob 7: No Assign"), `aria-valuemin="0"`, `aria-valuemax="127"`, `aria-valuenow` `Math.round(fraction × 127)` (Dynamics 127; tempo 104 → 34; swap knob 1 → 0) (D2), `aria-valuetext` "{knobName} {value}" with the state's whole value ("Dynamics 127", "Swing 0%", "Tempo 104 BPM"; No Assign "No Assign"). No Assign adds `aria-disabled="true"`.
- **Keyboard:** Tab focuses it (No Assign too); arrows and PageUp/PageDown step it as above.
- **Tooltip id:** `knobs.knob` (exists in `app/src/help/tooltips.ts`), on every knob, No Assign included; wired at integration (`data-tip`).
- **Launchkey:** knob `index + 1` (the eight encoders); the hardware turn sends the same `turnKnob`. The page ▲ ▼ are KnobBank's.
- **Swap mode:** nothing changes in the Knob. While a part's swap is held (`surface.layer` swap) the state gives knob 1 `function` `swapSound` (name "Right 1 Sound", value "23 Rhodes Soft", `level` null) and knobs 2–8 the part's mix; the screen sends `turnKnob` and the session runs it as `turnSwapKnob`.

## Stories (Story station)

- **Title:** `Primitives/Knob`.
- **Layout:** `centered` (real size, 68 × 96).

Every story renders in dark and light. Callbacks are actions (`fn()`); every prop is a control. Crop boxes are on Stage at 1440 × 900, the same box on `Stage-Dark.png` and `Stage-Light.png`.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ index: 0, function: 'dynamics', name: 'Dynamics Control', short: 'DynCtrl', value: '127', level: 127 }` | knob 1 at full: "Dynamics", "127", the whole 270° arc with the dot bottom right, "DynCtrl" | `Board-{dark,light}.png`: Stage 730,476 68×96 | the slider named "Knob 1: Dynamics" has `aria-valuetext` "Dynamics 127", `aria-valuenow` "127", `data-arc` "270"; `[data-part="dot"]` style has `left: 33.8px` and `top: 33.8px`; texts "Dynamics", "127", "DynCtrl". pointerdown at clientY 100, pointermove to clientY 92, pointerup → `onturn` called once, with `2`. Double-click → `onreset` called once, `onturn` not called again. Focus, ArrowUp → `onturn(1)`; ArrowLeft → `onturn(-1)`; PageUp → `onturn(10)`; PageDown → `onturn(-10)`; wheel `deltaY −100` → `onturn(1)`; wheel `deltaY 3` → `onturn(-1)`. |
| `RetrigRate` | `{ index: 1, function: 'retriggerRate', name: 'Retrigger Rate', short: 'RtgRate', value: '1/8', level: 51 }` | "Retrig rate", "1/8", a 108° arc, dot at the top left | `RetrigRate-{dark,light}.png`: Stage 804,476 68×96 | name text "Retrig rate"; `data-arc` "108"; dot `left: 9.5px; top: 0.3px`; `aria-valuetext` "Retrig rate 1/8". |
| `Off` | `{ index: 2, function: 'retriggerOnOff', name: 'Retrigger On/Off', short: 'RtgOnOff', value: 'Off', level: 0 }` | a zero value: rest ring only, the dot at the start (bottom left) | `Off-{dark,light}.png`: Stage 878,476 68×96 | `data-arc` "0"; the dot exists with `left: 4.2px; top: 33.8px`; `aria-valuenow` "0". |
| `Percent` | `{ index: 5, function: 'swing', name: 'Swing', short: 'Swing', value: '0%', level: 0 }` | the "%" split off as a small unit | `Percent-{dark,light}.png`: Stage 1100,476 68×96 | `[data-part="value"]` text starts "0" and `[data-part="unit"]` text is "%"; `aria-valuetext` "Swing 0%". |
| `NoAssign` | `{ index: 6, function: 'none', name: 'No Assign', short: '---', value: '', level: null }` | "---" dimmed, empty value and code, a `--mbg` ring, no dot | `NoAssign-{dark,light}.png`: Stage 1174,476 68×96 | the slider named "Knob 7: No Assign" has `aria-disabled` "true", `data-face` "disabled", `aria-valuetext` "No Assign"; name text "---"; value and code text empty; no `[data-part="dot"]`. Drag 8px up, wheel, double-click, ArrowUp → `onturn` and `onreset` not called. It is still reachable with Tab. |
| `Tempo` | `{ index: 7, function: 'tempo', name: 'Tempo', short: 'Tempo', value: '104 BPM', level: null, tempo: 104 }` | "104" without "BPM", a 72° arc | `Tempo-{dark,light}.png`: Stage 1248,476 68×96 (the board draws 73°, D7) | value text "104", no unit element, no "BPM" in the visible text; `aria-valuetext` "Tempo 104 BPM"; `data-arc` "72"; dot `left: 0.3px; top: 9.5px`; `aria-valuenow` "34". |
| `Fine` | `Board` args plus `shift: true` | as `Board` | — (the look doesn't change) | pointerdown at clientY 100, move to 89 → `onturn` not called; move to 88 → `onturn(1)`; move to 76 → `onturn(1)` again; called twice in all. |
| `Swap` | `{ index: 0, function: 'swapSound', name: 'Right 1 Sound', short: 'Sound', value: '23 Rhodes Soft', level: null }` | swap mode's knob 1: "Sound", the value cut with "…", arc at 0 with the dot | — (no board draws swap mode) | name text "Sound"; `aria-valuetext` "Sound 23 Rhodes Soft"; `data-arc` "0"; drag 4px up → `onturn(1)`. |
| `LongName` | `{ index: 2, function: 'partReverb', name: 'Right 1 Reverb', short: 'RevR1', value: '40', level: 40 }` | a function with no plain word: the state's name, cut with "…" at 68px | — (no board draws it on a knob) | name text "Right 1 Reverb" (the whole string; the "…" is CSS); `aria-label` "Knob 3: Right 1 Reverb". |
| `Focused` | `Board` args; `parameters: { pseudo: { focusVisible: true } }` | the focus ring round the 68 × 96 column | — | — |

What jsdom can't check (the conic arc, its colours, the `--g` disc, the dot's look, the unit's 2px gap, the ellipsis) is covered by the cropped stories, and by `Swap`, `LongName` and `Focused` judged in Storybook.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `knob.test.ts` asserts every example in the Pure functions table (`npx vitest run src/ui/Knob`).
- `npm run shots -- Knob` passes: each cropped story is 68 × 96 and scores at most 0.02 against its crop; axe finds no violation on any story.
- Only listed tokens are used (`--ring-rest` lands with the kit's tokens); no colour literals; sizes only as in Visual rules.
- svelte-check and lint pass on the folder.

## Decisions

- D1. The accessible name is "Knob n: {plain name}" and the value text "{plain name} {state value}", so a screen reader hears the word the screen shows and the whole value ("104 BPM").
- D2. `aria-valuenow` is `round(fraction × 127)` on a 0–127 range for every knob (tempo 104 reads 34, swap knob 1 reads 0), because a slider must carry a number and `aria-valuetext` carries the meaning.
- D3. PageUp/PageDown send ±10 steps, from kit › Interaction conventions; kit › Knob names only the arrows.
- D4. A drag re-anchors by the travel it used (4px or 12px a step), so leftover travel carries over and a Shift change mid-drag never repeats or loses a step.
- D5. A knob with `level` null that isn't tempo or No Assign (swap mode's knob 1) draws an empty arc with the dot at the start: the knob is endless and the state gives no position.
- D6. Each text line ends in an ellipsis at the column's 68px; the full text stays in `aria-label` and `aria-valuetext`.
- D7. The arc is in whole degrees and the dot rounded to 0.1px, the board's own rounding; the board draws the tempo knob at a fraction of 0.27 (73°, dot 0.5, 9.1) while the fixture's 104 BPM gives 0.267 (72°, dot 0.3, 9.5), a sub-pixel difference inside the crop score.
- D8. No Assign stays focusable with `aria-disabled` and the `knobs.knob` tooltip, and calls nothing (kit › Faces, Disabled).
- D9. The wheel steps one per event whatever its `deltaY` size, up positive.
- D10. The "%" unit is drawn in `--a`, as the board draws it; kit › Knob gives it no colour.
- D11. Only "%" splits off as a unit; other units ("5.0 kHz" on the Effects boards) stay in the value text until the Effects spec (#519) extends `splitUnit`.
- D12. Fine drag follows the `shift` prop (`ui.shift`), never the event's `shiftKey`, so the on-screen and keyboard Shift act the same.

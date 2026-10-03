# Knob

## Identity (all stations)

- **Kind:** primitive
- **Built from:** —
- **Purpose:** Shows what one of the Launchkey's eight knobs does and where it stands, and turns it from the screen.
- **Boards:**
  - `Stage-Dark.dc.html:304-313` (the knob column) and its data `:477-494` (names, values, the arc and the tip dot); light: `Stage-Light.dc.html:280-289`, data `:449-466`.
  - Crops: the eight knobs are whole pixels at `730 + 74·i, 476`, 68 × 96 (i = 0…7, knob 1 first) on `docs/design/push/png/Stage-Dark.png` and `Stage-Light.png`.
- **Not this component's job:** no store, no API, no Tauri (lint forbids those imports under `app/src/ui`). It doesn't send `turnKnob` or `resetKnob`: it calls `onturn(delta)` and `onreset()` and the page wiring sends. It doesn't coalesce steps per animation frame (the wiring sums the deltas of one frame into one `turnKnob`). It doesn't read `event.shiftKey` (fine drag comes from the `shift` prop). No timer. The tooltip is the parent's key passed as `tip`, wired by the `tipAction` the parent passes (L3); the library never imports `use:tip`. The ▲ ▼ page buttons and the header are KnobBank's.

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component. The knob's function is typed `string`, not the API's `KnobFunction` union: `app/src/ui` may not import `app/src/lib/api/types.ts` (lint), so `Knob.svelte` and `knob.ts` take a plain `string` and treat any function they don't name as "other" (D15). No `KnobFunction` alias is declared in the Knob folder.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `index` | `number` | `0` | The knob's place, 0–7 (knob 1 is 0). Used in the accessible name ("Knob 1: …") and as `data-knob`. |
| `fn` | `string` | `'none'` | What the knob does, as the state's `knobs.knobs[i].function` names it (`dynamics`, `retriggerRate`, `tempo`, `none`, `swapSound`, …). Named `fn` because `function` is a reserved word and can't be a destructured local (D14). Picks the plain name (`knobName`), the spoken name (`knobSpoken`), the tempo value rule (`knobValue`) and the No Assign face (`'none'`). |
| `name` | `string` | `''` | The state's full name ("Dynamics Control"); shown only for a function `knobName` has no plain word for. |
| `short` | `string` | `''` | The Genos code ("DynCtrl"), shown under the ring. Not shown for No Assign. |
| `value` | `string` | `''` | The value as the state writes it ("127", "1/8", "Off", "0%", "104 BPM", "23 Rhodes Soft"); empty for No Assign. Split by `knobValue`. |
| `level` | `number \| null` | `null` | Where the knob is, 0–127; null for tempo, No Assign and swap mode's knob 1. |
| `tempo` | `number \| null` | `null` | The tempo in BPM, used for the arc only when `level` is null and `fn` is `'tempo'`. |
| `shift` | `boolean` | `false` | Fine drag: one step per 12px instead of 4px. |
| `tip` | `string \| undefined` | — | The tooltip key (`knobs.knob` on every knob, No Assign included), rendered as `data-tip` on the slider; no attribute when undefined (L3). |
| `tipAction` | `Action<HTMLElement, string> \| undefined` | — | The app's `use:tip`, passed in by the wiring (the library can't import it). When both it and `tip` are set the slider gets `use:tipAction={tip}`; otherwise nothing (L3). `Action` is `import type { Action } from 'svelte/action'`. |

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

| Prop / callback | From `AppState` (or app-only state) / sends |
|---|---|
| `index` | the knob's position i in `knobs.knobs` (0–7) |
| `fn` | `knobs.knobs[i].function` |
| `name`, `short`, `value`, `level` | `knobs.knobs[i].name`, `.short`, `.value`, `.level` |
| `tempo` | `transport.tempo` |
| `shift` | `ui.shift`: app-only state in `app/src/lib/store.svelte.ts` (`UiStore`), not `AppState`; it already ORs the computer keyboard's Shift and `surface.shift` |
| `onturn(delta)` | `turnKnob { knob: i, delta }`, the deltas of one animation frame summed into one command (the wiring's). In swap mode (`surface.layer.type == 'swap'`) the session runs it as `turnSwapKnob` for the held part, so the screen sends the same command. |
| `onreset()` | `resetKnob { knob: i }` |
| `tip`, `tipAction` | KnobBank passes `'knobs.knob'` and its own `tipAction` (the wiring's `tip` action from `app/src/lib/tooltip/tip.svelte.ts`) |

### Pure functions (`app/src/ui/Knob/knob.ts`)

All exported, no DOM, each with a vitest in `app/src/ui/Knob/knob.test.ts` asserting the examples. `knobFraction` lives here, not in `app/src/ui/Stage/format.ts` where Stage.md's Components row 1b lists it; Stage's `format.ts` re-exports it if the page needs it (D20). `knob.ts` also exports the ring geometry the component and `tipDot` share: `KNOB_RING = 44` (the ring's side, px), `KNOB_DOT = 6` (the tip dot's side), `KNOB_DOT_RADIUS = 21` (the dot centre's distance from the ring centre).

| Function | Rule | Examples |
|---|---|---|
| `knobFraction(level: number \| null, tempo: number \| null): number` | `level` not null: `clamp(level / 127, 0, 1)`. Else `tempo` not null: `clamp((tempo − 40) / 240, 0, 1)`. Else `0`. (The Knob passes `tempo` only when `fn` is `'tempo'`, else null.) No rounding. | `(127, null)` → 1; `(51, null)` → 0.40157…; `(0, null)` → 0; `(null, 104)` → 0.26667 (0.267 to 3 places, Stage.md Check 12); `(null, 300)` → 1; `(null, 20)` → 0; `(null, null)` → 0 |
| `knobAria(fn: string, level: number \| null, tempo: number \| null): { min: number, max: number, now: number }` | The slider's range and value, Stage.md D61. `fn === 'none'` → `{ 0, 0, 0 }`. `fn === 'tempo'` and `level` null → `{ min: 40, max: 280, now: Math.round(clamp(tempo ?? 40, 40, 280)) }` (the range `knobFraction` spans). Any other → `{ min: 0, max: 127, now: level === null ? 0 : Math.round(clamp(level, 0, 127)) }` (D2). | `('dynamics', 127, null)` → `{ 0, 127, 127 }`; `('retriggerRate', 51, 104)` → `{ 0, 127, 51 }`; `('tempo', null, 104)` → `{ 40, 280, 104 }`; `('tempo', null, 300)` → `{ 40, 280, 280 }`; `('tempo', null, null)` → `{ 40, 280, 40 }`; `('none', null, null)` → `{ 0, 0, 0 }`; `('swapSound', null, null)` → `{ 0, 127, 0 }` |
| `knobDeg(fraction: number): number` | `Math.round(fraction × 270)`: the arc in whole degrees. | 1 → 270; 0.40157 → 108; 0.26667 → 72; 0 → 0 |
| `tipDot(fraction: number): { left: number, top: number }` | `θ = (225 + knobDeg(fraction))°` in radians; `c = KNOB_RING / 2` (22), `r = KNOB_DOT_RADIUS` (21), `h = KNOB_DOT / 2` (3); the dot's top-left corner is `left = c + r·sin θ − h`, `top = c − r·cos θ − h`, each rounded to 0.1 (`Math.round(x × 10) / 10`). px, relative to the ring's box, θ clockwise from the top. | fraction 1 (θ 135°) → `{ left: 33.8, top: 33.8 }` (bottom right); fraction 0 (θ 225°) → `{ left: 4.2, top: 33.8 }` (bottom left); fraction 0.26667 (tempo 104, 72°, θ 297°) → `{ left: 0.3, top: 9.5 }`; fraction 51/127 (108°, θ 333°) → `{ left: 9.5, top: 0.3 }` |
| `knobSteps(travel: number, fine: boolean): number` | `Math.trunc(travel / (fine ? 12 : 4))`, with −0 returned as 0. `travel` is px moved up (positive) since the anchor. | `(8, false)` → 2 (Check 12); `(7, false)` → 1; `(3, false)` → 0; `(−5, false)` → −1; `(−3, false)` → 0; `(11, true)` → 0; `(12, true)` → 1; `(24, true)` → 2 |
| `knobName(fn: string, name: string): string` | The shown name, Stage.md D6: `dynamics` "Dynamics", `retriggerRate` "Retrig rate", `retriggerOnOff` "Retrigger", `trackMuteA` "Mute A", `trackMuteB` "Mute B", `swing` "Swing", `tempo` "Tempo", `splitPoint` "Split", `harmonyArp` "Harm/Arp", `harmonyVolume` "Harm level", `metronomeVolume` "Click level", `swapSound` "Sound", `none` "---"; any other function returns `name` as given. | `('dynamics', 'Dynamics Control')` → "Dynamics"; `('none', 'No Assign')` → "---"; `('swapSound', 'Right 1 Sound')` → "Sound"; `('partReverb', 'Right 1 Reverb')` → "Right 1 Reverb" |
| `knobSpoken(fn: string, name: string): string` | The spoken name (D1): `fn === 'none'` → "No Assign" (the literal, whatever `name` says); else `knobName(fn, name)`. | `('none', 'No Assign')` → "No Assign"; `('none', '')` → "No Assign"; `('dynamics', 'Dynamics Control')` → "Dynamics"; `('partReverb', 'Right 1 Reverb')` → "Right 1 Reverb" |
| `knobValueText(fn: string, name: string, value: string): string` | `aria-valuetext`: `knobSpoken(fn, name)`, then, only when `value` is not empty, one space and `value` as given (no trailing space). | `('dynamics', 'Dynamics Control', '127')` → "Dynamics 127"; `('tempo', 'Tempo', '104 BPM')` → "Tempo 104 BPM"; `('none', 'No Assign', '')` → "No Assign"; `('swing', 'Swing', '0%')` → "Swing 0%" |
| `splitUnit(value: string): { text: string, unit: string }` | A value ending in a digit then "%" splits off the "%": `/^(.*\d)%$/` → `{ text: <before>, unit: '%' }`; anything else → `{ text: value, unit: '' }`. | "0%" → `{ '0', '%' }`; "38%" → `{ '38', '%' }`; "127" → `{ '127', '' }`; "%" → `{ '%', '' }`; "5.0 kHz" → `{ '5.0 kHz', '' }`; "" → `{ '', '' }` |
| `knobValue(fn: string, value: string): { text: string, unit: string }` | For `fn === 'tempo'`, first drops a trailing " BPM" (`/\s*BPM$/`); then `splitUnit`. | `('tempo', '104 BPM')` → `{ '104', '' }`; `('swing', '0%')` → `{ '0', '%' }`; `('dynamics', '127')` → `{ '127', '' }`; `('none', '')` → `{ '', '' }` |

### Visual rules

- **Tokens used:** colours `--g`, `--t2`, `--m`, `--d`, `--a`, `--mbg`, `--focus`, and the new `--ring-rest` (below); scale `--font-sans`, `--text-12`, `--text-22`, `--weight-light`, `--weight-regular`, `--space-2`, `--line-width`, `--focus-offset`.
- **New token `--ring-rest`** (kit › Tokens to add), the ring past the value: dark `#3d3d3d` (a new palette step, kit's name `--grey-24`), light `#d6d5d1` (a new palette step, kit's name `--stone-83`). Neither palette step exists in `palette.css` today. It lands with the tokens contract PR (the orchestrator), not in this component's PR; this PR never edits `app/src/ui/tokens/*` (D18).
- **Component geometry (axiom 2):** the root rule declares, once, `--knob-width: 68px`, `--knob-height: 96px`, `--knob-line: 14px` (name and code line height), `--knob-value-line: 22px` (value line height); the root also sets `--knob-ring` and `--knob-dot` from `knob.ts` (`style:--knob-ring="{KNOB_RING}px"`, `style:--knob-dot="{KNOB_DOT}px"`), so the ring's size has one source shared with `tipDot`. Every other rule uses these or the scale tokens above; no other length literals (D17). The gradient's angles (225deg, 270deg, 360deg) are the arc's definition, not sizes.
- **Size:** a fixed `--knob-width` × `--knob-height` column (68 × 96), a flex column with `align-items: center`, `text-align: center`, no padding, no background, no border. From the top:
  1. **Name** line: height and line-height `--knob-line`, `--text-12` `--weight-regular`, `--t2`; the text is `knobName(fn, name)`. No Assign: "---" in `--d`, and the name element carries `data-contrast="dim"` (Stage.md D47; kit › Faces, Dimmed text); assigned knobs have no `data-contrast`.
  2. **Value** line: a block, height and line-height `--knob-value-line`, `--text-22` `--weight-light`, `--a`. Its content is the text node `knobValue(fn, value).text`, then, when `unit` is not empty, an inline unit `span` with `margin-left: var(--space-2)`, `vertical-align: baseline` (it sits on the value's baseline), `--text-12` `--weight-regular`, `--a`, inheriting the line's line-height. No Assign: empty (the line keeps its height).
  3. **Ring:** `--knob-ring` × `--knob-ring`, `margin-top: var(--space-2)`, `border-radius: 50%`, `position: relative`, `flex: none`, background `conic-gradient(from 225deg, var(--a) 0deg <deg>deg, var(--ring-rest) <deg>deg 270deg, transparent 270deg 360deg)` with `<deg> = knobDeg(fraction)` and `fraction = knobFraction(level, fn === 'tempo' ? tempo : null)`. Over it a disc, `position: absolute; inset: var(--space-2); border-radius: 50%; background: var(--g)`, so the ring reads as a 2px arc. Then the **tip dot**: `position: absolute`, `--knob-dot` × `--knob-dot`, `border-radius: 50%`, `--a` fill, `left`/`top` from `tipDot(fraction)` in px as an inline style; `aria-hidden="true"`. The dot shows at every fraction, 0 included. The open quarter (270°–360°, the bottom) is transparent.
  4. **Code** line: height and line-height `--knob-line`, `--text-12` `--weight-regular`, `--m`; the text is `short`. No Assign: empty (the line keeps its height).
- **States drawn by:**
  - assigned: as above; the arc length follows `fraction`.
  - level unknown (`level` null and `fn` not `'tempo'` or `'none'`: swap mode's knob 1): an empty arc (`<deg>` 0, all `--ring-rest`) with the dot at the start (D5).
  - No Assign (`fn` `'none'`): name "---" in `--d`; value and code empty; arc and rest both `--mbg` (`conic-gradient(from 225deg, var(--mbg) 0deg 270deg, transparent 270deg 360deg)`), no tip dot element; `aria-disabled="true"` (an assigned knob has no `aria-disabled` attribute, L5); default cursor; still focusable.
  - dragging: no change of look; cursor `ns-resize` (Stage.md D35).
  - keyboard focus: a `--line-width` outline in `--focus`, `--focus-offset` outside the column, on `:focus-visible` only.
  - No hover or pressed look (D35). Cursor `pointer` when assigned, `default` for No Assign.
- **Overflow:** each of the three text lines is `white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: var(--knob-width)`, so a long name ("Right 1 Reverb"), a long value ("23 Rhodes Soft") or a long code ends in "…" inside the column (D6).
- **Type:** DM Sans (`--font-sans`), `font-variant-numeric: tabular-nums`, case as given.
- **Contrast (AA 4.5:1):**
  - `--a` on `--g` (value and unit): dark 7.80, light 5.95, passes. Add `['--a', '--g', 'knob value and unit (Knob)']` to `contrast.test.ts` with the tokens PR (the orchestrator), not in this PR.
  - `--t2` on `--g` (name) and `--m` on `--g` (code): already in `contrast.test.ts`.
  - The arc and the dot are graphics, not text.
  - **Dimmed text (Stage.md D47, owner):** `--d` on `--g`, the No Assign name "---" (dark 2.48, light 1.94), stays as drawn. It is on kit › Faces' list of `data-contrast="dim"` carriers ("a No Assign knob's ---"), and axe's `color-contrast` rule skips exactly those elements (`*:not([data-contrast="dim"])`). No token changes and no `contrast.test.ts` row for it (L2 doesn't apply: D47 keeps the pair). The story meta sets the same rule for the Storybook a11y panel: `parameters.a11y.config.rules: [{ id: 'color-contrast', selector: '*:not([data-contrast="dim"])' }]` (D21). No other pair fails today.
- **Motion:** none. The arc and dot follow `level` / `tempo`; the component draws what it is given.
- **Test hooks (Stage.md D41):** the root carries `data-knob="<index>"` and `data-arc="<knobDeg(fraction)>"` (whole degrees; No Assign `0`), and `data-dragging="true"` while a drag is on (the attribute is absent otherwise). No `data-face`: the Knob has none of the kit's faces, and No Assign is read from `aria-disabled`, as Button D4 and LampButton D2 read disabled (D13). The name, value, unit and code elements carry `data-part="name" | "value" | "unit" | "code"` (the unit element exists only when `unit` is not empty); the tip dot carries `data-part="dot"` with its `left`/`top` as an inline style (`left: 0.3px; top: 9.5px`), which jsdom reads. The root carries `data-tip="<tip>"` when `tip` is set (L3), and the No Assign name `data-contrast="dim"`. The arc colours (custom properties in a gradient) are checked by the crops.

### Behaviour

All pointer, wheel, double-click and key listeners are on the root (the `role="slider"` element) itself, never on `window` or `document`.

- **Drag** (assigned only): `pointerdown` with the primary button (`button === 0`) calls `el.setPointerCapture?.(e.pointerId)` inside `try`/`catch` (jsdom has no pointer capture, and a synthetic event's id may not be an active pointer, L4), records `anchorY = clientY` and the `pointerId`, and sets `data-dragging="true"`. Each `pointermove` on the root with that `pointerId` (capture routes moves outside the column to the root in a browser; in jsdom the plays dispatch them on the root) computes `steps = knobSteps(anchorY − clientY, shift)`; when `steps ≠ 0` it calls `onturn(steps)` once and moves the anchor by what was used, `anchorY −= steps × (shift ? 12 : 4)`, so the leftover travel carries into the next move and a Shift change mid-drag never double counts (D4). `pointerup`, `pointercancel` or `lostpointercapture` ends the drag, calls `el.releasePointerCapture?.(id)` when `el.hasPointerCapture?.(id)` is true (also inside `try`/`catch`, L4), and removes `data-dragging`; nothing is called on release. A `pointermove` with no drag on does nothing. Horizontal movement is ignored. Example (Stage.md Check 12): press, then one move 8px up → `onturn(2)`; two moves of 4px → `onturn(1)` twice.
- **Wheel** (assigned only): each `wheel` event with `deltaY < 0` calls `onturn(1)`, `deltaY > 0` calls `onturn(-1)`, whatever its size or `deltaMode`; `deltaY` 0 does nothing. The listener is not passive (attached with `{ passive: false }` in an `$effect`, since Svelte's `onwheel` attribute is passive) and calls `preventDefault()` so the page doesn't scroll.
- **Double-click** (assigned only): `onreset()`. The two clicks' presses move nothing, so no `onturn` comes with it.
- **Keys** when focused (assigned only, `keydown` on the root): ArrowUp and ArrowRight `onturn(1)`, ArrowDown and ArrowLeft `onturn(-1)`, PageUp `onturn(10)`, PageDown `onturn(-10)` (D3); each calls `preventDefault()` and `stopPropagation()` so the window's ← → (`stepStyle`) don't fire (Stage.md D36). Other keys pass through. No Assign handles no keys (they propagate).

### Accessibility

- **Role and name:** the root is `role="slider"`, `tabindex="0"`, `aria-orientation="vertical"`, `aria-label` exactly `` `Knob ${index + 1}: ${knobSpoken(fn, name)}` `` ("Knob 1: Dynamics"; No Assign "Knob 7: No Assign", never "Knob 7: ---"), `aria-valuemin`, `aria-valuemax` and `aria-valuenow` from `knobAria(fn, level, tempo)` (Stage.md D61: Dynamics 0 / 127 / 127; tempo 104 → 40 / 280 / 104; No Assign 0 / 0 / 0; swap knob 1 0 / 127 / 0) (D2), `aria-valuetext` `knobValueText(fn, name, value)` ("Dynamics 127", "Swing 0%", "Tempo 104 BPM"; No Assign "No Assign", no trailing space). No Assign adds `aria-disabled="true"`.
- **Keyboard:** Tab focuses it (No Assign too); arrows and PageUp/PageDown step it as above.
- **Tooltip id:** `knobs.knob` (exists in `app/src/help/tooltips.ts`), on every knob, No Assign included, passed as `tip` with `tipAction` (L3). Its last sentence needs a change (D16, contract change needed).
- **Launchkey:** knob `index + 1` (the eight encoders); the hardware turn sends the same `turnKnob`. The page ▲ ▼ are KnobBank's.
- **Swap mode:** nothing changes in the Knob. While a part's swap is held (`surface.layer` swap) the state gives knob 1 `function` `swapSound` (name "Right 1 Sound", value "23 Rhodes Soft", `level` null) and knobs 2–8 the part's mix; the screen sends `turnKnob` and the session runs it as `turnSwapKnob`.

## Stories (Story station)

- **Title:** `Primitives/Knob`.
- **Layout:** `centered` (real size, 68 × 96).
- **Meta:** `args: { onturn: fn(), onreset: fn(), tipAction: fn() }` (axiom 7; `tipAction` an action, L3); `parameters: { a11y: { config: { rules: [{ id: 'color-contrast', selector: '*:not([data-contrast="dim"])' }] } } }` for every story (D21). Every story's args include `tip: 'knobs.knob'` unless its row says otherwise. Controls: `fn` is a `select` whose options are the API's 25 knob functions copied as a literal array `KNOB_FUNCTIONS` in `Knob.stories.ts` (`none`, `dynamics`, `retriggerRate`, `retriggerOnOff`, `trackMuteA`, `trackMuteB`, `tempo`, `swing`, `partVolume`, `harmonyVolume`, `metronomeVolume`, `partPan`, `partReverb`, `partChorus`, `partDelay`, `fxReturn`, `fxParam`, `delayTime`, `harmonyArp`, `splitPoint`, `insertOn`, `insertSetting`, `partSend`, `rotaryFast`, `swapSound`, the order of `KnobFunction` in `app/src/lib/api/types.ts`; copied because `app/src/ui` may not import it); `index` a `number` (min 0, max 7); `level` and `tempo` `number`; `name`, `short`, `value`, `tip` text; `shift` boolean; `onturn`, `onreset`, `tipAction` actions.

Every story renders in dark and light. Crop boxes are on Stage at 1440 × 900, the same box on `Stage-Dark.png` and `Stage-Light.png`; each is exactly the knob's own 68 × 96 column (L6; the focus ring, outside it, is judged in Storybook). Plays use `fireEvent` from `@testing-library/svelte` (or `storybook/test`) on the slider element itself: `fireEvent.pointerDown(slider, { button: 0, pointerId: 1, clientY })`, `fireEvent.pointerMove(slider, { pointerId: 1, clientY })`, `fireEvent.pointerUp(slider, { button: 0, pointerId: 1, clientY })`, `fireEvent.dblClick(slider)`, `fireEvent.wheel(slider, { deltaY })`, `fireEvent.keyDown(slider, { key })`.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ index: 0, fn: 'dynamics', name: 'Dynamics Control', short: 'DynCtrl', value: '127', level: 127 }` | knob 1 at full: "Dynamics", "127", the whole 270° arc with the dot bottom right, "DynCtrl" | `Board-{dark,light}.png`: Stage 730,476 68×96 | the slider named "Knob 1: Dynamics" has `aria-valuetext` "Dynamics 127", `aria-valuemin` "0", `aria-valuemax` "127", `aria-valuenow` "127", `data-arc` "270", no `data-dragging`, no `aria-disabled`, `data-tip` "knobs.knob"; `tipAction` was called with (the slider, `'knobs.knob'`); no element has `data-contrast`; `[data-part="dot"]` style has `left: 33.8px` and `top: 33.8px`; texts "Dynamics", "127", "DynCtrl". pointerDown at clientY 100 → `data-dragging` "true"; pointerMove to clientY 92; pointerUp → `onturn` called once, with `2`, and `data-dragging` gone. dblClick → `onreset` called once, `onturn` not called again. keyDown ArrowUp → `onturn(1)`; ArrowLeft → `onturn(-1)`; PageUp → `onturn(10)`; PageDown → `onturn(-10)`; wheel `deltaY −100` → `onturn(1)`; wheel `deltaY 3` → `onturn(-1)`. |
| `RetrigRate` | `{ index: 1, fn: 'retriggerRate', name: 'Retrigger Rate', short: 'RtgRate', value: '1/8', level: 51 }` | "Retrig rate", "1/8", a 108° arc, dot at the top left | `RetrigRate-{dark,light}.png`: Stage 804,476 68×96 | name text "Retrig rate"; `data-arc` "108"; dot `left: 9.5px; top: 0.3px`; `aria-valuetext` "Retrig rate 1/8". |
| `Off` | `{ index: 2, fn: 'retriggerOnOff', name: 'Retrigger On/Off', short: 'RtgOnOff', value: 'Off', level: 0 }` | a zero value: rest ring only, the dot at the start (bottom left) | `Off-{dark,light}.png`: Stage 878,476 68×96 | `data-arc` "0"; the dot exists with `left: 4.2px; top: 33.8px`; `aria-valuenow` "0". |
| `Percent` | `{ index: 5, fn: 'swing', name: 'Swing', short: 'Swing', value: '0%', level: 0 }` | the "%" split off as a small unit | `Percent-{dark,light}.png`: Stage 1100,476 68×96 | `[data-part="value"]` text starts "0" and `[data-part="unit"]` text is "%"; `aria-valuetext` "Swing 0%". |
| `NoAssign` | `{ index: 6, fn: 'none', name: 'No Assign', short: '---', value: '', level: null }` | "---" dimmed, empty value and code, a `--mbg` ring, no dot | `NoAssign-{dark,light}.png`: Stage 1174,476 68×96 | the slider named exactly "Knob 7: No Assign" has `aria-disabled` "true", no `data-face`, `aria-valuetext` exactly "No Assign", `aria-valuemin`, `aria-valuemax` and `aria-valuenow` all "0", `data-tip` "knobs.knob"; name text "---" and the name element has `data-contrast="dim"`; value and code text empty; no `[data-part="dot"]`. pointerDown at 100, pointerMove to 92, pointerUp; wheel `deltaY −100`; dblClick; keyDown ArrowUp → `onturn` and `onreset` not called, and no `data-dragging` appeared. It is still reachable with Tab (`tabindex` "0"). |
| `Tempo` | `{ index: 7, fn: 'tempo', name: 'Tempo', short: 'Tempo', value: '104 BPM', level: null, tempo: 104 }` | "104" without "BPM", a 72° arc | `Tempo-{dark,light}.png`: Stage 1248,476 68×96 (the board draws 73°, D7) | value text "104", no unit element, no "BPM" in the visible text; `aria-valuetext` "Tempo 104 BPM"; `data-arc` "72"; dot `left: 0.3px; top: 9.5px`; `aria-valuemin` "40", `aria-valuemax` "280", `aria-valuenow` "104". |
| `Fine` | `Board` args plus `shift: true` | as `Board` | — (the look doesn't change) | pointerDown at clientY 100, pointerMove to 89 → `onturn` not called; pointerMove to 88 → `onturn(1)`; pointerMove to 76 → `onturn(1)` again; called twice in all. |
| `Swap` | `{ index: 0, fn: 'swapSound', name: 'Right 1 Sound', short: 'Sound', value: '23 Rhodes Soft', level: null }` | swap mode's knob 1: "Sound", the value cut with "…", an empty arc with the dot at the start (D5) | — (no board draws swap mode) | name text "Sound"; `aria-valuetext` "Sound 23 Rhodes Soft"; `aria-valuenow` "0" (max "127"); `data-arc` "0"; pointerDown at 100, pointerMove to 96 → `onturn(1)`. |
| `LongName` | `{ index: 2, fn: 'partReverb', name: 'Right 1 Reverb', short: 'RevR1', value: '40', level: 40 }` | a function with no plain word: the state's name, cut with "…" at 68px | — (no board draws it on a knob) | name text "Right 1 Reverb" (the whole string; the "…" is CSS); `aria-label` "Knob 3: Right 1 Reverb". |
| `NoTip` | `Board` args with `tip: undefined` | as `Board` | — (same look as `Board`) | no `data-tip` attribute on the slider; `tipAction` not called (L3: applied only when both are set) |
| `Focused` | `Board` args; `parameters: { pseudo: { focusVisible: true } }` (the primitives' form) | the focus ring round the 68 × 96 column | — | — |

What jsdom can't check (the conic arc, its colours, the `--g` disc, the dot's look, the unit's 2px gap and baseline, the ellipsis) is covered by the cropped stories, and by `Swap`, `LongName` and `Focused` judged in Storybook.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `knob.test.ts` asserts every example in the Pure functions table (`npx vitest run src/ui/Knob`).
- `knob.test.ts` also asserts every `knobAria` example.
- `npm run shots -- Knob` passes: each cropped story is 68 × 96 and scores at most 0.02 against its crop; axe finds no violation on any story, with `color-contrast` skipping only `[data-contrast="dim"]` (Stage.md D47; the shots tool's `axe.configure` is the Stage lane's build-time item, Stage.md › Gap against today).
- Only listed tokens are used (`--ring-rest` arrives with the tokens contract PR; until it lands the shots for this component can't pass, so the Knob PR merges after it); no colour literals; no length literals outside the root's custom properties (Visual rules).
- This PR doesn't touch `app/src/ui/tokens/*`, `contrast.test.ts` or `app/src/help/tooltips.ts`.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · Spoken name.** The accessible name is "Knob n: {spoken name}" and the value text "{spoken name} {state value}", so a screen reader hears the word the screen shows and the whole value ("104 BPM"). The spoken name is the shown name except for No Assign, which is spoken "No Assign" where the screen shows "---" (`knobSpoken`).
- **D2 · Slider numbers.** The slider's numbers follow Stage.md D61 (`knobAria`): 0–127 with `level`, the tempo knob 40–280 with the rounded BPM, No Assign 0 / 0 / 0. D61 is silent on a knob with `level` null that isn't tempo (swap mode's knob 1): it reads 0 on 0–127, matching its empty arc (D5). The tempo value is clamped into 40–280 so `aria-valuenow` never leaves its range.
- **D3 · Page keys.** PageUp/PageDown send ±10 steps, from kit › Interaction conventions; kit › Knob names only the arrows.
- **D4 · Drag anchor.** A drag re-anchors by the travel it used (4px or 12px a step), so leftover travel carries over and a Shift change mid-drag never repeats or loses a step.
- **D5 · No position.** A knob with `level` null that isn't tempo or No Assign (swap mode's knob 1) draws an empty arc with the dot at the start: the knob is endless and the state gives no position, and the Knob keeps no memory of past turns (it draws only what it is given).
- **D6 · Ellipsis.** Each text line ends in an ellipsis at the column's 68px; the full text stays in `aria-label` and `aria-valuetext`.
- **D7 · Rounding.** The arc is in whole degrees and the dot rounded to 0.1px, the board's own rounding; the board draws the tempo knob at a fraction of 0.27 (73°, dot 0.5, 9.1) while the fixture's 104 BPM gives 0.267 (72°, dot 0.3, 9.5), a sub-pixel difference inside the crop score.
- **D8 · No Assign focusable.** No Assign stays focusable with `aria-disabled` and the `knobs.knob` tooltip, and calls nothing (kit › Faces, Disabled).
- **D9 · Wheel.** The wheel steps one per event whatever its `deltaY` size, up positive.
- **D10 · Unit colour.** The "%" unit is drawn in `--a`, as the board draws it; kit › Knob gives it no colour.
- **D11 · Only "%".** Only "%" splits off as a unit; other units ("5.0 kHz" on the Effects boards) stay in the value text until the Effects spec (#519) extends `splitUnit`.
- **D12 · Fine drag.** Fine drag follows the `shift` prop (`ui.shift`, app-only state), never the event's `shiftKey`, so the on-screen and keyboard Shift act the same.
- **D13 · No face.** No `data-face` on the Knob: it has none of the kit's four faces, and, as Button D4 and LampButton D2 do, disabled (No Assign) is read from `aria-disabled`; kit D41's `disabled` value is not used.
- **D14 · `fn`.** The prop is `fn`, not `function`: `function` is a reserved word, so `let { function } = $props()` can't compile. KnobBank maps the state's `function` field onto it.
- **D15 · Plain string.** The function is a plain `string`, since `app/src/ui` can't import the API's `KnobFunction`; an unknown function shows `name`. The story's select copies the union's 25 values so a builder picks real ones.
- **D16 · Tooltip text (contract change needed).** The `knobs.knob` tooltip's last sentence ("with no level to show, the pointer shows the last movement") doesn't match D5. Decision: keep D5 (the screen draws only the state) and change the tooltip. **Contract change needed** (`app/src/help/tooltips.ts`, `knobs.knob`, a contract file): replace that sentence with "The knobs are endless like the Launchkey's: the ring shows where the knob is, and a knob with nothing to show (the Sound knob while a part's swap is held) draws an empty ring."
- **D17 · Geometry.** Literal sizes (axiom 2): shared values are `scale.css` tokens; the knob's own geometry (68, 96, 14, 22) is declared once as custom properties on the root, and the ring and dot sizes come from `knob.ts`'s constants (also used by `tipDot`) through `style:` custom properties.
- **D18 · L1, new tokens.** `--ring-rest` and its two new palette steps (#3d3d3d dark, #d6d5d1 light) and the `--a`-on-`--g` contrast row land with the tokens contract PR, not here.
- **D19 · L3, tooltip props.** The Knob takes `tip` and `tipAction` like every interactive primitive (L3), replacing the earlier "wired at integration" rule; the slider is the element that gets `data-tip`, and KnobBank passes `knobs.knob`.
- **D20 · Where `knobFraction` lives.** Stage.md row 1b puts `knobFraction` in `app/src/ui/Stage/format.ts`, but the Knob is built long before the Stage and is its only user, so it lives in `Knob/knob.ts`; the Stage re-exports it if it needs it. Its `tempo` is `number | null` (kit › Knob writes `number`) because the Knob passes null for every knob that isn't tempo.
- **D21 · Dimmed text, not an owner question.** The earlier "O-contrast" exclusion list is replaced by Stage.md D47 (an owner decision): the No Assign "---" carries `data-contrast="dim"` and axe skips exactly those elements; the Storybook meta mirrors the shots tool's rule so the a11y panel agrees.
- **D22 · L4 and L5.** Pointer capture and release sit in `try`/`catch` (L4); `aria-disabled` is present only on No Assign (L5).

Follow-ups: the shots tool's `axe.configure` for `data-contrast="dim"` (Stage.md D47, the Stage lane's build-time item) must land before `npm run shots -- Knob` can pass `NoAssign`; once `.storybook/preview.ts` sets the same `color-contrast` rule globally, the per-story `parameters.a11y.config` can go.

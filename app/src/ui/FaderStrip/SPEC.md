# FaderStrip

## Identity (all stations)

- **Kind:** primitive
- **Built from:** — (it places a `PartMarks` primitive after its name; PartMarks is a leaf with no behaviour, see Children)
- **Purpose:** One fader of the band: shows how loud a part (or the Style, the Multi Pads, the master) is set and how loud it plays, and lets the player drag, scroll or key it to a new level.
- **Boards:**
  - `Stage-Dark.dc.html:244-266` (the strip markup inside the faders grid) and its data script `:425-473` (`F`, `LAYER`, `strips`: values, meter and cap maths); light: `Stage-Light.dc.html:220-242`, data `:397-445`.
  - Nine instances side by side in the band's strips row, `Stage` `24,476 654×272`, each `(654 − 64) / 9 = 65.556` wide with 8 between: no strip has a whole-pixel box, so this primitive has no crop of its own. Its pixels are checked inside `FaderBank`'s `Board` crop.
- **Not this component's job:** no store, no API, no Tauri, no timer. It doesn't read `AppState`: the parent passes the value, hue, meter and flags. It doesn't compute the held peak (the page wiring does, with `holdPeak` from this folder's `meter.ts`), doesn't coalesce sends per animation frame (the wiring does), doesn't know what command a move sends, and doesn't wire its tooltip (it renders the key it is given as `data-tip`; integration attaches the help behaviour). It doesn't know the strip's sound name.

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `index` | `number` (1–9) | — | The strip's place in the bank, 1-based; only used in the unused strip's name ("Fader 7 unused"). |
| `label` | `string` | `''` | The name under the fader, as shown: `'Right 1'`, `'Right 2'`, `'Right 3'`, `'Left'`, `'Style'`, `'Multi Pad'`, `'Master'`, or a rack target's label as the state gives it (`'PANR2'`, `'HARMARP'`). Ignored when `value` is null (the strip then reads "—"). |
| `value` | `number \| null` | `null` | The set value, 0–127 (a level, a pan or a send, by `layer`). Null: the strip is **unused**. |
| `layer` | `'volume' \| 'pan' \| 'reverb' \| 'chorus' \| 'delay'` | `'volume'` | What `value` is. Anything but `volume` is the **layer** look (no meter, fill and cap in `--t`, the value carries the layer word). The parent passes `volume` for the Style, Multi Pad and Master strips whatever the bank's layer is. |
| `hue` | `'r1' \| 'r2' \| 'r3' \| 'l' \| 'a' \| 't2' \| 't'` | `'t'` | The strip's hue role (a token name without `--`): parts `r1 r2 r3 l`, Style `a`, Multi Pad `t2`, Master `t`. |
| `meter` | `{ peak: number; rms: number; hold: number } \| null` | `null` | Linear amplitudes 0–1 (as `meters` sends them; above 1 clamps): the left bar is `peak`, the right bar `rms`, the tick `hold` (the held peak the wiring computes with `holdPeak(...).value`). Null: no meter. Also not drawn when `value` is null, `layer` isn't `volume`, or `rackTarget` is true, whatever `meter` holds. |
| `off` | `boolean` | `false` | **Part off** (the part doesn't sound): value and name `--d`; fill and cap at 35% of their colour. The meter still draws as given (a silent part reads empty; D2). |
| `rackTarget` | `boolean` | `false` | The live rack's controller map gives this fader another target: the label is that target's (`'PANR2'`), value, name, fill and cap `--t2`, no fill glow, no meter. |
| `waiting` | `boolean` | `false` | **Soft takeover**: the hardware fader hasn't reached the value; draws "↕" and the ghost line at `position`. |
| `position` | `number \| null` | `null` | Where the hardware fader is, 0–127. The ghost line draws only when `waiting` is true and `position` isn't null. |
| `edited` | `boolean` | `false` | Sound-edited mark after the name (keyboard parts only). |
| `missing` | `boolean` | `false` | Plugin-missing mark ⚠ after the name. |
| `failed` | `boolean` | `false` | Plugin-failed mark ✕ after the name. The parent passes it only when the plugin failed and isn't missing; if both are true the strip shows ⚠ only. |
| `name` | `string \| undefined` | — | The strip's spoken name, used in the slider's `aria-label` and `aria-valuetext` and the name button's `aria-label`. Default: `label`. |
| `nameAction` | `string` | `''` | What the name button does, in words, for its accessible name: `'open Channel'`, `'open the Style fader page'`, `'open Multi Pads'`, `'open Effects'`, `'open the Rack page'`. Empty: the name is plain text, not a control. |
| `tip` | `string \| undefined` | — | Tooltip key put on the fader element as `data-tip` (FaderBank chooses it by strip, layer and takeover; see Accessibility). |
| `nameTip` | `string \| undefined` | — | Tooltip key put on the name button as `data-tip`. |
| `width` | `number \| undefined` | — | A fixed width in px. Unset: fills its container (the bank's grid column). Stories set the board's `65.556`. |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onchange` | the strip's whole-number value changes by a drag (once per `pointermove` that changes it), a wheel notch, an arrow or Page key, or a double-click reset; never for a change that leaves the value as it was; never when unused | `(value: number)` the new value, 0–127 |
| `onrelease` | the `pointerup` or `pointercancel` that ends a drag in which the value changed | `(value: number)` the last value of that drag (so the wiring always sends it, Stage.md D23) |
| `onopen` | click, Enter or Space on the name button (only when `nameAction` isn't empty and the strip isn't unused) | `()` |

The strip keeps a local **shown value**: it takes `value` whenever that prop changes, and every change the strip makes (drag, wheel, key, reset) sets it at once, before calling `onchange`, so the cap follows the hand without waiting for the state (D1). Every step below (drag, `stepValue`) starts from the shown value.

### Children

| Child | Where | Props |
|---|---|---|
| `PartMarks` (primitive, primitives lane) | inside the name button, after the name, gap 4 | `{ edited, missing, failed: failed && !missing }` from this strip's props; its `off` and `bass` marks false (the strip never shows them). Not rendered when the strip is unused. |

### Pure functions

Three files in this folder, each a set of exported pure functions with a vitest file beside it (`meter.test.ts`, `drag.test.ts`, `value.test.ts`) asserting every example below. `clamp(x, lo, hi)` is `Math.min(hi, Math.max(lo, x))`; rounding is `Math.round` (halves up).

**`meter.ts`** (Stage.md D7)

- `TRAVEL = 223` (px, the fader's travel; the same as the `--travel` token).
- `height(x: number): number` — the bar height in px for a linear amplitude. `x <= 0` → 0; else `Math.round(223 × clamp((20 × log10(x) + 60) / 60, 0, 1))` (a −60…0 dBFS scale). Examples: `height(0.0724)` = 138; `height(0.0537)` = 129; `height(0.0224)` = 100; `height(0.0180)` = 93; `height(0.0316)` = 112; `height(0.0248)` = 104; `height(0.1259)` = 156; `height(0.0897)` = 145; `height(0.1445)` = 161; `height(0.1020)` = 149; `height(0.001)` = 0; `height(0)` = 0; `height(1)` = 223; `height(2)` = 223.
- `tickBottom(hold: number): number` = `5 + height(hold)`, the held-peak tick's bottom in px from the fader's bottom. Examples (the board's nine holds): `0.1259` → 161, `0.0447` → 128, `0` → 5, `0.0631` → 139, `0.2188` → 179, `0.0023` → 32, `0.2512` → 183.
- `type Hold = { peak: number; atMs: number; value: number }` — `peak` the level caught, `atMs` when (`meters.atMs`), `value` what to draw now.
- `holdPeak(prev: Hold | null, peak: number, atMs: number): Hold` — the held peak after a meter frame of `peak` at `atMs`. Held for `HOLD_MS = 1500` ms, then falling `FALL_DB_PER_S = 20` dB/s, never below the current peak. With `elapsed = max(0, atMs − prev.atMs)`: `fallen = elapsed <= 1500 ? prev.peak : prev.peak × 10^(−(elapsed − 1500) / 1000)` (20 dB a second is ×0.1 a second in amplitude). If `prev` is null or `peak >= fallen`: returns `{ peak, atMs, value: peak }` (caught anew). Else returns `{ peak: prev.peak, atMs: prev.atMs, value: fallen }`. Examples, with `h = holdPeak(null, 0.2, 1000)` = `{ peak: 0.2, atMs: 1000, value: 0.2 }`:
  - `holdPeak(h, 0.05, 2500)` → `{ peak: 0.2, atMs: 1000, value: 0.2 }` (held for exactly 1500 ms);
  - `holdPeak(h, 0.05, 3000)` → value `0.2 × 10^−0.5` ≈ 0.06325 (to 4 places), peak 0.2, atMs 1000;
  - `holdPeak(h, 0, 3500)` → value ≈ 0.02;
  - `holdPeak(h, 0.05, 3500)` → `{ peak: 0.05, atMs: 3500, value: 0.05 }` (the fall went under the current peak);
  - `holdPeak(h, 0.3, 1100)` → `{ peak: 0.3, atMs: 1100, value: 0.3 }`;
  - `holdPeak(h, 0.1, 900)` (a frame older than the hold) → elapsed 0, `{ peak: 0.2, atMs: 1000, value: 0.2 }`;
  - `holdPeak(null, 0, 10000)` → `{ peak: 0, atMs: 10000, value: 0 }`.

**`drag.ts`** (Stage.md D23)

- `dragValue(v0: number, y0: number, y: number): number` = `clamp(Math.round(v0 + (y0 − y) × 127 / 223), 0, 127)`; `v0`, `y0` the shown value and pointer `clientY` at `pointerdown`, `y` the pointer's `clientY` now. Relative: the value never jumps to the pointer. Examples: `dragValue(90, 200, 160)` = 113 (Stage.md Check 9); `dragValue(90, 200, 200)` = 90; `dragValue(90, 200, 240)` = 67; `dragValue(72, 300, 299)` = 73 (72.57 rounds up); `dragValue(120, 300, 0)` = 127; `dragValue(10, 0, 300)` = 0.
- `stepValue(v: number, delta: number): number` = `clamp(v + delta, 0, 127)`. Examples: `stepValue(90, 1)` = 91; `stepValue(90, -10)` = 80; `stepValue(125, 10)` = 127; `stepValue(3, -10)` = 0.
- `keyDelta(key: string): number | null` — `ArrowUp`, `ArrowRight` → 1; `ArrowDown`, `ArrowLeft` → −1; `PageUp` → 10; `PageDown` → −10; anything else null (not handled; Home and End aren't handled, D5).
- `wheelDelta(deltaY: number): number` — `deltaY < 0` → 1, `deltaY > 0` → −1, 0 → 0. One step per wheel event, whatever its size.
- `resetValue(layer): number` — `volume` 100, `pan` 64, `reverb` / `chorus` / `delay` 0.

**`value.ts`** (the text and the set-level geometry)

- `valueText(layer, v: number | null): string` — null → `''`; `volume` → the number (`"90"`); `pan` → `"C"` at 64, `"L{64 − v}"` below (`44` → `"L20"`, `0` → `"L64"`), `"R{v − 64}"` above (`84` → `"R20"`, `127` → `"R63"`); `reverb` → `"Rev 40"`, `chorus` → `"Cho 12"`, `delay` → `"Dly 0"` (the word, a space, the number).
- `spokenValue(layer, v: number): string` — `volume` → `"90"`; `pan` → `"pan centre"`, `"pan left 20"`, `"pan right 20"`; `reverb` → `"reverb 40"`, `chorus` → `"chorus 12"`, `delay` → `"delay 0"`.
- `capTop(v: number): number` = `Math.round((1 − v / 127) × 223)`, the set level's distance from the track's top in px. Examples: 90 → 65; 72 → 97; 64 → 111; 80 → 83; 100 → 47; 127 → 0; 0 → 223; 50 → 135; 84 → 76; 44 → 146; 40 → 153.
- `fillBox(layer, v: number): { top: number; height: number }` — the fill's box in px from the fader's top. Levels and sends: `top = 24 + capTop(v)`, `height = 223 − capTop(v)` (from the track's bottom up to the level; 90 → top 89, height 158; 0 → top 247, height 0). Pan (Stage.md D8: from the 64 line, `capTop(64)` = 111): `v >= 64` → `top = 24 + capTop(v)`, `height = 111 − capTop(v)`; `v < 64` → `top = 135`, `height = capTop(v) − 111`. Examples: pan 84 → top 100, height 35; pan 44 → top 135, height 35; pan 64 → top 135, height 0; pan 127 → top 24, height 111; pan 0 → top 135, height 112.
- `ghostTop(position: number): number` = `capTop(position) + 24`. Example: 50 → 159.

### Visual rules

Geometry is in px inside the strip's own box: `width` (65.556 on the board) × 272, a column of the **fader** (the slider, 252 tall, full width, `position: relative`, no background or border, padding 0) over the **name** (20 tall, full width). Horizontal positions are from the strip's centre line (`50%`), as the board places them; every part below is `position: absolute` inside the fader and `aria-hidden`. Each carries `data-part` as named.

| Part (`data-part`) | Box | Drawn |
|---|---|---|
| `value` | left 0, right 0, top 0, height 20; line-height 20; centred; no wrap | `valueText(layer, shown)`; 18 / 300 (`--text-18`, `--weight-light`); colour below |
| `meter-track` (two) | left `calc(50% − 17px)` and `calc(50% − 5px)`; top 24, bottom 5 (223 tall); width 10 | `--mbg` |
| `meter-peak` | left `calc(50% − 17px)`, bottom 5, width 10, height `height(meter.peak)` px (inline style) | the hue at `--meter-mix`: `color-mix(in srgb, var(--<hue>) var(--meter-mix), transparent)` |
| `meter-rms` | left `calc(50% − 5px)`, bottom 5, width 10, height `height(meter.rms)` px | as `meter-peak` |
| `hold` | left `calc(50% − 17px)`, width 22, height 1, bottom `tickBottom(meter.hold)` px (also at `hold` 0: it then sits on the track's bottom, as the board draws; D4) | `--peak` |
| `groove` | left `calc(50% + 10px)`, top 24, bottom 5, width 3 | `--track`; unused: `repeating-linear-gradient(to bottom, var(--line) 0 3px, transparent 3px 7px)` (3 on, 4 off) |
| `fill` | left `calc(50% + 10px)`, width 3, `top` / `height` from `fillBox(layer, shown)` (inline style) | colour below; glow `box-shadow: 0 0 6px color-mix(in srgb, var(--<hue>) var(--fill-glow-mix), transparent)` only in the Plain row (then `data-glow` is on the element); none otherwise |
| `cap` | left `calc(50% + 3px)`, width 10, height 3, top `capTop(shown) + 23` px | colour below |
| `ghost` | left `calc(50% − 22px)`, width 42, height 0, top `ghostTop(position)` px; `border-top: 1px dashed var(--m)` | only while `waiting` and `position` isn't null |
| `wait` | left 0, top 24; 13px (`--text-13`), line-height 13 | "↕" in `--m`, only while `waiting` |

The meter parts (`meter-track` ×2, `meter-peak`, `meter-rms`, `hold`) are drawn together or not at all: drawn when `meter` isn't null, `value` isn't null, `layer` is `volume` and `rackTarget` is false. Not drawn means not in the DOM. The set level (groove, fill, cap) outranks the meter: it comes later in the DOM. Unused: no `fill`, no `cap` in the DOM.

**The name** (`data-part="name"`): a `<button type="button">` (a `<span>` when `nameAction` is empty or the strip is unused), 20 tall, full width, padding 0, no background, no border, no radius; `display: flex`, centred both ways, gap 4 (`--space-4`), no wrap. The label 13 / 500 (`--text-13`, `--weight-medium`), line-height 16, colour below; then `PartMarks`. Unused: the text "—" in `--d`, no marks.

**Colours by state.** One row applies, the first that matches, top to bottom. `data-hue` (the token name without `--`) is on the `value`, `name`, `fill` and `cap` elements; at 35% the fill and cap carry the base token's name and the strip's root element carries `data-off`.

| State | When | `value` text | `name` | `fill`, `cap` | Fill glow | Meter |
|---|---|---|---|---|---|---|
| Unused | `value` null | empty | "—" `--d` | not drawn | — | none |
| Part off | `off` | `--d` | `--d` | `color-mix(in srgb, <c> var(--off-mix), transparent)`, `<c>` being `--t` when `layer` isn't `volume`, else the hue | none | as given (none in a layer) |
| Rack target | `rackTarget` | `--t2` | `--t2` | `--t2` | none | none |
| Layer | `layer` isn't `volume` | `--t` | the hue | `--t` | none | none |
| Plain | otherwise | the hue | the hue | the hue | yes | as given |

`waiting` adds "↕" and the ghost to any row but Unused. The marks show in every row but Unused. The root also carries `data-layer` (the `layer` value) and `data-waiting` / `data-rack` / `data-unused` when those apply.

- **Tokens used:** `--t`, `--t2`, `--m`, `--d`, `--r1`, `--r2`, `--r3`, `--l`, `--a`, `--mbg`, `--line`, `--focus`, `--track` (new, kit), `--peak` (new, kit), `--meter-mix` (new, kit), `--fill-glow-mix` (new, kit), `--off-mix` (new, not in the kit: `35%` in both themes), `--travel` (new, kit: 223px), `--font-sans`, `--text-13`, `--text-18`, `--weight-light`, `--weight-medium`, `--space-4`, `--line-width`, `--focus-offset`.
- **Size:** `width` (or the container's) × 272. The fader 252, the name 20. The track runs from 24 to 247 inside the fader (`--travel` 223).
- **States drawn by:** the colour table above, plus:
  - waiting: "↕" and the dashed ghost line;
  - dragging: `cursor: ns-resize` on the fader and `data-dragging` on it; at rest the fader and the name button use `cursor: pointer`; unused: `default` everywhere (D35);
  - keyboard focus: the fader and the name button each show a `--line-width` `--focus` outline at `--focus-offset` on `:focus-visible`; nothing on mouse focus;
  - no hover or pressed look.
- **Type:** DM Sans (`--font-sans`), tabular numerals; value 18 / 300; name 13 / 500; "↕" 13 / 400. Case as given. The value never wraps (the widest, "Rev 127", is about 60px at 18 / 300; it may overhang the column into the 8px gaps).
- **Contrast** (`tokens/contrast.test.ts` rows to add): `--r1`, `--r2`, `--r3`, `--l`, `--a` on `--g` (value 18 / 300 and name 13 / 500 in the part and Style hues). `--t2`, `--t`, `--m` on `--g` exist. `--d` text (part off, unused) is under 4.5:1 by design (D3).
- **Motion:** none of its own. The meter moves only when the `meter` prop changes (the wiring passes a new frame about 30 times a second); the cap moves with the shown value.

### Accessibility

- **Role and name:** the fader is `role="slider"`, `tabindex="0"`, `aria-orientation="vertical"`, `aria-valuemin="0"`, `aria-valuemax="127"`, `aria-valuenow` the shown value, `aria-label` `"{name} {what}"` where `what` is `volume`, `pan`, `reverb send`, `chorus send`, `delay send` by `layer` ("Right 1 volume", "Style volume", "Master volume", "Right 2 reverb send"); a rack target reads `"{name}, rack fader"` ("PANR2, rack fader"). `aria-valuetext` is `"{name} {spokenValue(layer, shown)}"`, plus `", hardware fader away"` while `waiting`: "Right 1 90", "Right 2 72, hardware fader away", "Right 1 pan left 20", "Right 1 reverb 40".
- **Unused:** the fader is a `<div role="img" aria-label="Fader {index} unused">` ("Fader 7 unused"), no `tabindex`, no slider attributes; it handles no pointer, wheel or key, and calls nothing; the name "—" is an `aria-hidden` span (D10).
- **Name button:** `aria-label` `"{name}{marks}: {nameAction}"`, marks being ", edited", ", plugin missing", ", plugin failed" in that order as they show ("Right 2, edited: open Channel", "Right 3, plugin missing: open Channel", "Style: open the Style fader page", "Master: open Effects", "PANR2: open the Rack page").
- **Keyboard:** Tab focuses the fader, then the name button. On the fader: ArrowUp / ArrowRight +1, ArrowDown / ArrowLeft −1, PageUp +10, PageDown −10 (`keyDelta`, `stepValue`); each handled key calls `preventDefault` and `stopPropagation`, so the window's ← → (`stepStyle`) don't fire while a fader has focus (Stage.md D36). Other keys pass through untouched. Enter or Space on the name button calls `onopen`.
- **Pointer:** `pointerdown` with the primary button anywhere on the fader (the value text included) records `v0` (the shown value) and `y0` (`clientY`) and captures the pointer (`setPointerCapture`, called only if the element has it, as jsdom doesn't); each `pointermove` during the press sets the shown value to `dragValue(v0, y0, clientY)` and calls `onchange` if that changed it; `pointerup` or `pointercancel` ends the press, releases the capture and calls `onrelease(shown)` if the press changed the value (D6). A `wheel` event over the fader calls `preventDefault` and steps by `wheelDelta(deltaY)`. `dblclick` on the fader sets the shown value to `resetValue(layer)` (calling `onchange` if that changed it).
- **Tooltip ids** (the parent passes them as `tip` / `nameTip`; listed so the parent's table can be checked): fader — Right 1–3, Left in volume `mixer.panel.right1`, `mixer.panel.right2`, `mixer.panel.right3`, `mixer.panel.left`; in pan `mixer.part.pan`, reverb `mixer.part.reverb`, chorus `mixer.part.chorus`, delay `mixer.part.variation`; Style `mixer.style_level`; Multi Pad `mixer.pad_level`; Master `mixer.master`; any strip while `waiting` `mixer.pickup`; rack target `launchkey.fader_rack`; unused `launchkey.fader_unused`. Name — parts `mixer.strip.select`, Style `mixer.style_level`, Multi Pad `mixer.pad_level`, Master `mixer.master`, rack target `launchkey.fader_rack`. All exist in `app/src/help/tooltips.ts`.
- **Launchkey:** strip *n* is fader *n* (1–8) on the active fader page; strip 9 is the master fader. The strip doesn't know; FaderBank's table says it.

## Stories (Story station)

- **Title:** `Primitives/FaderStrip`.
- **Layout:** `centered`. Every story sets `width: 65.556` (the board's column) and renders in dark and light.
- **Crops:** none, for every story: each instance on the board is 65.556 wide at a fractional x, so no story has a whole-pixel box. The strips' pixels are checked by `Components/FaderBank` › `Board` (crop `Stage 24,432 654×368`), whose nine strips are this table's `Board`, `RightTwoWaiting`, `PartOff`, `Left`, `Style`, `MultiPad`, `Unused` (×2) and `Master`.
- Callbacks are actions (`onchange`, `onrelease`, `onopen`: `fn()`). Every prop is a control.

Shared args: `R1 = { index: 1, label: 'Right 1', value: 90, hue: 'r1', meter: { peak: 0.0724, rms: 0.0537, hold: 0.1259 }, nameAction: 'open Channel', tip: 'mixer.panel.right1', nameTip: 'mixer.strip.select', width: 65.556 }`. `R3 = { index: 3, label: 'Right 3', value: 64, hue: 'r3', off: true, missing: true, meter: { peak: 0, rms: 0, hold: 0 }, nameAction: 'open Channel', tip: 'mixer.panel.right3', nameTip: 'mixer.strip.select', width: 65.556 }`.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `R1` (strip 1 of the board fixture) | Right 1 at 90: blue value, two blue meter bars 138 and 129 tall, the white tick 161 from the bottom, the cap 88 down with the fill's glow, the blue name | — (fractional; see Crops) | the slider named "Right 1 volume" has `aria-valuenow="90"`, `aria-valuetext="Right 1 90"`, `tabindex="0"`, `data-tip="mixer.panel.right1"`; `[data-part=value]` reads "90" with `data-hue="r1"`; `[data-part=meter-peak]` has inline height `138px`, `[data-part=meter-rms]` `129px`, `[data-part=hold]` bottom `161px`; `[data-part=fill]` top `89px`, height `158px`, has `data-glow`; `[data-part=cap]` top `88px`; the button named "Right 1: open Channel" has `data-tip="mixer.strip.select"` |
| `RightTwoWaiting` | `{ index: 2, label: 'Right 2', value: 72, hue: 'r2', meter: { peak: 0.0224, rms: 0.0180, hold: 0.0447 }, waiting: true, position: 50, edited: true, nameAction: 'open Channel', tip: 'mixer.pickup', nameTip: 'mixer.strip.select', width: 65.556 }` (strip 2 of the board) | pink strip at 72, "↕" top-left, the dashed ghost 159 down, the edited dot after "Right 2" | — | `aria-valuetext` is "Right 2 72, hardware fader away"; `[data-part=wait]` reads "↕"; `[data-part=ghost]` top `159px`; root has `data-waiting`; name button "Right 2, edited: open Channel"; the slider's `data-tip` is `mixer.pickup` |
| `Edited` | `{ ...R1, index: 2, label: 'Right 2', value: 72, hue: 'r2', meter: { peak: 0.0224, rms: 0.0180, hold: 0.0447 }, edited: true, tip: 'mixer.panel.right2' }` | the edited dot alone, no takeover | — | no `[data-part=wait]`, no `[data-part=ghost]`; name button "Right 2, edited: open Channel" |
| `PartOff` | `R3` (strip 3 of the board: off, plugin missing) | grey "64" and "Right 3", fill and cap orange at 35%, the empty meter with its tick on the bottom, the orange ⚠ | — | root has `data-off`; value and name `data-hue="d"`; fill and cap `data-hue="r3"`; fill has no `data-glow`; `[data-part=meter-peak]` and `[data-part=meter-rms]` height `0px`, `[data-part=hold]` bottom `5px`; name button "Right 3, plugin missing: open Channel" |
| `Failed` | `{ ...R3, missing: false, failed: true }` | the red ✕ in place of ⚠ | — | name button "Right 3, plugin failed: open Channel" |
| `Left` | `{ index: 4, label: 'Left', value: 80, hue: 'l', meter: { peak: 0.0316, rms: 0.0248, hold: 0.0631 }, nameAction: 'open Channel', tip: 'mixer.panel.left', nameTip: 'mixer.strip.select', width: 65.556 }` (strip 4) | teal strip at 80 | — | meter heights `112px` and `104px`, hold bottom `139px`; cap top `106px` |
| `Style` | `{ index: 5, label: 'Style', value: 100, hue: 'a', meter: { peak: 0.1259, rms: 0.0897, hold: 0.2188 }, nameAction: 'open the Style fader page', tip: 'mixer.style_level', nameTip: 'mixer.style_level', width: 65.556 }` (strip 5) | violet group strip | — | slider "Style volume"; heights `156px`, `145px`, hold `179px`; cap top `70px`; name button "Style: open the Style fader page" |
| `MultiPad` | `{ index: 6, label: 'Multi Pad', value: 90, hue: 't2', meter: { peak: 0, rms: 0, hold: 0.0023 }, nameAction: 'open Multi Pads', tip: 'mixer.pad_level', nameTip: 'mixer.pad_level', width: 65.556 }` (strip 6) | grey-white group strip, silent, its tick low at 32 | — | heights `0px`, hold bottom `32px`; value `data-hue="t2"` |
| `Master` | `{ index: 9, label: 'Master', value: 100, hue: 't', meter: { peak: 0.1445, rms: 0.1020, hold: 0.2512 }, nameAction: 'open Effects', tip: 'mixer.master', nameTip: 'mixer.master', width: 65.556 }` (strip 9) | white master strip | — | slider "Master volume"; heights `161px`, `149px`, hold `183px`; name button "Master: open Effects" |
| `Unused` | `{ index: 7, label: '', value: null, hue: 't', tip: 'launchkey.fader_unused', width: 65.556 }` (strips 7, 8) | no value, the dashed groove, "—" in `--d` | — | no element has role `slider`; the element named "Fader 7 unused" has role `img`, no `tabindex`, `data-tip="launchkey.fader_unused"`; no `[data-part=cap]`, no `[data-part=fill]`; no button; a pointerdown / pointermove (40px up) / pointerup, a wheel and an ArrowUp on it call no callback |
| `RackTarget` | `{ index: 2, label: 'PANR2', value: 64, hue: 'r2', rackTarget: true, meter: { peak: 0.0224, rms: 0.0180, hold: 0.0447 }, nameAction: 'open the Rack page', tip: 'launchkey.fader_rack', nameTip: 'launchkey.fader_rack', width: 65.556 }` | "PANR2", value, fill and cap in `--t2`, no meter although one is passed | — | slider "PANR2, rack fader"; no `[data-part^=meter]`, no `[data-part=hold]`; value and name `data-hue="t2"`; name button "PANR2: open the Rack page" |
| `Pan` | `{ ...R1, layer: 'pan', value: 44, tip: 'mixer.part.pan' }` | "L20" in white, the white fill growing down from the 64 line to the cap, no meter | — | value "L20" with `data-hue="t"`; slider "Right 1 pan", `aria-valuetext` "Right 1 pan left 20"; fill top `135px`, height `35px`; no meter parts; name `data-hue="r1"` |
| `PanRight` | `{ ...R1, layer: 'pan', value: 84, tip: 'mixer.part.pan' }` | "R20", the fill growing up from the 64 line | — | value "R20"; fill top `100px`, height `35px` |
| `Reverb` | `{ ...R1, layer: 'reverb', value: 40, tip: 'mixer.part.reverb' }` | "Rev 40" in white, white fill and cap, no meter | — | value "Rev 40"; slider "Right 1 reverb send", valuetext "Right 1 reverb 40"; no meter parts; fill height `70px` |
| `Delay` | `{ ...R1, layer: 'delay', value: 0, tip: 'mixer.part.variation' }` | "Dly 0", the cap on the track's bottom | — | value "Dly 0"; cap top `246px`; fill height `0px` |
| `Top` | `{ ...R1, value: 127, meter: { peak: 1, rms: 0.9, hold: 1 } }` | the top of the range: cap at 23, full peak bar, tick at 228 | — | cap top `23px`; peak height `223px`; hold bottom `228px` |
| `Bottom` | `{ ...R1, value: 0, meter: { peak: 0, rms: 0, hold: 0 } }` | the bottom of the range | — | cap top `246px`; fill height `0px` |
| `Drags` | `R1` | — | — | pointerdown on the slider at `clientY` 200, pointermove to 160 → `onchange` last called with 113 and `aria-valuenow` is "113"; pointerup → `onrelease` called once, with 113; then pointerdown at 200 and pointerup without moving → `onrelease` still called once in all |
| `Steps` | `R1` | — | — | focus the slider; ArrowUp → `onchange(91)`; PageDown → `onchange(81)`; ArrowLeft → `onchange(80)`; a `wheel` with `deltaY` −100 → `onchange(81)`; double-click → `onchange(100)`; double-click again → `onchange` not called again; the ArrowUp keydown event has `defaultPrevented` true |
| `PanReset` | `{ ...R1, layer: 'pan', value: 44, tip: 'mixer.part.pan' }` | — | — | double-click → `onchange(64)`; value reads "C" |
| `OpensName` | `R1` | — | — | click the button "Right 1: open Channel" → `onopen` called once; Tab from the slider focuses that button |
| `Focused` | `R1`; `parameters: { pseudo: { focusVisible: ['[role=slider]'] } }` | the focus ring around the fader | — | — |

What jsdom can't check (colours, `color-mix`, the glow, the dashed groove, how it all lines up) is checked by FaderBank's `Board` crop in Chrome; the inline px values in the plays are what jsdom can.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `meter.test.ts`, `drag.test.ts` and `value.test.ts` assert every example in Pure functions (`npx vitest run src/ui/FaderStrip`).
- `npm run shots -- FaderStrip` passes: no story has a crop, so this is axe on every story in Chrome (with D3's exemption); FaderBank's shots cover the pixels.
- Only listed tokens are used; no inline colours; no literal sizes outside the Visual rules (the inline px come from the pure functions).
- svelte-check and lint pass on the folder.

## Decisions

- D1. The strip draws a local shown value during a drag, wheel or key change and takes the `value` prop again whenever it changes, so the cap follows the hand at once without waiting for the state's round trip.
- D2. A part that is off keeps its meter track and held-peak tick, as the board draws Right 3, rather than hiding the meter as kit › FaderStrip says; a part that doesn't sound reads empty anyway.
- D3. `--d` text (a part that is off, the unused "—") stays as the board draws it although it is under 4.5:1, because it marks absence (WCAG 1.4.3's inactive-component exemption in spirit); the stories configure axe's `color-contrast` rule to skip `[data-hue="d"]` (owner question in the lane report).
- D4. The held-peak tick draws at `hold` 0, on the track's bottom (`bottom: 5px`), as the board's Right 3 shows, rather than disappearing.
- D5. Home and End aren't handled (kit › Interaction conventions names only arrows and Page keys, and one key jumping the master to 127 is risky).
- D6. `onrelease` fires only when the press changed the value, so a click, or the two presses of a double-click, send nothing extra.
- D7. The spoken value uses words ("pan left 20", "reverb 40") where the screen shows abbreviations ("L20", "Rev 40").
- D8. Pan's fill grows from `capTop(64)` (111px down the track), not the track's geometric middle (111.5), so pan 64 draws no fill and L20 and R20 are the same length.
- D9. A rack target's value, fill and cap are `--t2` like its name (the kit gives only the name's colour), with no meter, since its value isn't the part's level.
- D10. The unused fader is `role="img"` named "Fader 7 unused" and not focusable (Stage.md Check 1), not a disabled slider.
- D11. The slider's name is the strip name plus what it moves ("Right 1 volume") and its value text the kit's "Right 1 90"; the board's sound name in the label ("Right 1 · Stage Grand") is left out because the strip doesn't know the sound.
- D12. The unused groove's dashes are `--line` in both themes, as the kit says; the light board draws them `#c4c3bf` (`--track`), a few hundred pixels under the shot threshold.
- D13. Tooltip keys come in as props (`tip`, `nameTip`) rendered as `data-tip`, because `app/src/ui` may not import the `tip` action from `app/src/lib`; integration attaches the help behaviour to `[data-tip]`.
- D14. Part off in a layer draws its fill and cap at 35% of `--t` (the layer colour), as the board's data script does, not 35% of the part hue.

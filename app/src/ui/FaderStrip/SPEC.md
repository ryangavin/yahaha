# FaderStrip

## Identity (all stations)

- **Kind:** primitive
- **Built from:** — (it places a `PartMarks` primitive after its name; PartMarks is a leaf with no behaviour, see Children)
- **Purpose:** One fader of the band: shows how loud a part (or the Style, the Multi Pads, the master) is set and how loud it plays, and lets the player drag, scroll or key it to a new level.
- **Boards:**
  - `Stage-Dark.dc.html:244-266` (the strip markup inside the faders grid) and its data script `:425-473` (`F`, `LAYER`, `strips`: values, meter and cap maths); light: `Stage-Light.dc.html:220-242`, data `:397-445`.
  - Nine instances side by side in the band's strips row, `Stage` `24,476 654×272`, each `(654 − 64) / 9 = 65.556` wide with 8 between: no strip has a whole-pixel box, so this primitive has no crop of its own. Its pixels are checked inside `FaderBank`'s `Board` crop.
- **Not this component's job:** no store, no API, no Tauri, no timer. It doesn't read `AppState`: the parent passes the value, hue, meter and flags. It doesn't compute the held peak (the page wiring does, with `holdPeak` from this folder's `meter.ts`), doesn't coalesce sends per animation frame (the wiring does), doesn't know what command a move sends, and doesn't wire its tooltip (it renders the key it is given as `data-tip` on its own two elements; integration attaches the help behaviour). It doesn't know the strip's sound name (D11, D16).

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
| `off` | `boolean` | `false` | **Part off** (the part doesn't sound): value and name `--d`; fill, cap and meter bars at 35% of their colour. The meter still draws as given (a silent part reads empty; D2, D15). |
| `rackTarget` | `boolean` | `false` | The live rack's controller map gives this fader another target: the label is that target's (`'PANR2'`), value, name, fill and cap `--t2`, no fill glow, no meter. |
| `waiting` | `boolean` | `false` | **Soft takeover**: the hardware fader hasn't reached the value; draws "↕" and the ghost line at `position`. |
| `position` | `number \| null` | `null` | Where the hardware fader is, 0–127. The ghost line draws only when `waiting` is true and `position` isn't null. |
| `edited` | `boolean` | `false` | Sound-edited mark after the name (keyboard parts only). |
| `missing` | `boolean` | `false` | Plugin-missing mark ⚠ after the name. |
| `failed` | `boolean` | `false` | Plugin-failed mark ✕ after the name: the raw fact (`plugin.status === 'failed'`). PartMarks drops the ✕ when `missing` is also true (its D1), and the name button's words follow PartMarks' `marksText`. |
| `name` | `string \| undefined` | — | The strip's spoken name, used in the slider's `aria-label` and `aria-valuetext` and the name button's `aria-label`. Default: `label`. |
| `nameAction` | `string` | `''` | What the name button does, in words, for its accessible name: `'open Channel'`, `'open the Style fader page'`, `'open Multi Pads'`, `'open Effects'`, `'open the Rack page'`. Empty: the name is a plain `<span>`, not a control (Visual rules › The name). |
| `tip` | `string \| undefined` | — | Tooltip key put on the strip's own fader element as `data-tip` (FaderBank chooses it by strip, layer and takeover; see Accessibility). |
| `nameTip` | `string \| undefined` | — | Tooltip key put on the strip's own name button as `data-tip`. Not rendered on the plain-`<span>` name. |
| `width` | `number \| undefined` | — | A fixed width in px. Unset: fills its container (the bank's grid column). Stories set the board's `65.556`. |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onchange` | the strip's whole-number value changes by a drag (once per `pointermove` that changes it), a wheel notch, an arrow or Page key, or a double-click reset; never for a change that leaves the value as it was; never when unused | `(value: number)` the new value, 0–127 |
| `onrelease` | the `pointerup` or `pointercancel` that ends a drag in which the value changed | `(value: number)` the last value of that drag (so the wiring always sends it, Stage.md D23) |
| `onopen` | click, Enter or Space on the name button (only when `nameAction` isn't empty and the strip isn't unused) | `()` |

The strip keeps a local **shown value**: it takes `value` whenever that prop changes, except during a drag, and every change the strip makes (drag, wheel, key, reset) sets it at once, before calling `onchange`, so the cap follows the hand without waiting for the state (D1). Every step below (drag, `stepValue`) starts from the shown value. **During a drag** (from the `pointerdown` that starts it to its `pointerup` or `pointercancel`) a `value` prop change is ignored: the shown value keeps following the pointer from the `v0` and `y0` taken at `pointerdown`, which are never reset. The first `value` change after the drag ends is taken as usual (D17).

### Children

| Child | Where | Props |
|---|---|---|
| `PartMarks` (primitive, primitives lane) | inside the name element, after the label, 4 apart (the name's gap) | `{ size: 'strip', edited, missing, failed }` from this strip's props, passed raw (PartMarks applies ⚠-over-✕); `off` and `bass` left false (the strip never shows them, PartMarks D6). Not rendered when the strip is unused. PartMarks renders nothing when it has no mark. |

The name button's accessible words for the marks come from PartMarks' pure `marksText({ edited, missing, failed })` (`app/src/ui/PartMarks/marks.ts`), so the strip and the marks can't disagree.

### Pure functions

Three files in this folder, each a set of exported pure functions with a vitest file beside it (`meter.test.ts`, `drag.test.ts`, `value.test.ts`) asserting every example below. `clamp(x, lo, hi)` is `Math.min(hi, Math.max(lo, x))`; rounding is `Math.round` (halves up).

**`meter.ts`** (Stage.md D7)

- `TRAVEL = 223` (px, the fader's travel; the same as the `--travel` token).
- `height(x: number): number` — the bar height in px for a linear amplitude. `x <= 0` → 0; else `Math.round(223 × clamp((20 × log10(x) + 60) / 60, 0, 1))` (a −60…0 dBFS scale). Examples: `height(0.0724)` = 138; `height(0.0537)` = 129; `height(0.0224)` = 100; `height(0.0180)` = 93; `height(0.0316)` = 111 (111.48; the board draws 112 from its own rounding and Stage.md's fixture says 112, a 1px difference under the shot threshold: the formula wins, D18); `height(0.0248)` = 104; `height(0.1259)` = 156; `height(0.0897)` = 145; `height(0.1445)` = 161; `height(0.1020)` = 149; `height(0.001)` = 0; `height(0)` = 0; `height(1)` = 223; `height(2)` = 223.
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

**Geometry.** The strip's own sizes are declared once as custom properties on its root element, and every rule below uses them (axiom 2); shared values use `scale.css` tokens by name. No other literal sizes; the inline px come from the pure functions.

| Custom property (on the root) | Value | What |
|---|---|---|
| `--strip-height` | `272px` | the whole strip |
| `--fader-height` | `252px` | the fader (the slider) |
| `--row-height` | `20px` | the value row and the name row |
| `--track-top` | `24px` | the track's top inside the fader |
| `--track-bottom` | `5px` | the track's bottom gap (`--fader-height` − `--track-top` − `--travel` = 5) |
| `--meter-width` | `10px` | each meter bar, and the cap's width |
| `--meter-peak-x` | `-17px` | the peak bar's left edge from the centre line |
| `--meter-rms-x` | `-5px` | the RMS bar's left edge from the centre line |
| `--tick-width` | `22px` | the held-peak tick |
| `--groove-x` | `10px` | the groove's and the fill's left edge from the centre line |
| `--groove-width` | `3px` | the groove, the fill, and the cap's height |
| `--cap-x` | `3px` | the cap's left edge from the centre line |
| `--ghost-x` | `-22px` | the ghost line's left edge from the centre line |
| `--ghost-width` | `42px` | the ghost line |
| `--name-line` | `16px` | the name label's line height |
| `--glow-blur` | `6px` | the fill glow's blur |
| `--dash` | `3px` | the unused groove's dash; its period is `calc(var(--dash) + var(--space-4))` (3 on, 4 off) |

Geometry is inside the strip's own box: `width` (65.556 on the board) × `--strip-height`, a column of the **fader** (the slider, `--fader-height` tall, full width, `position: relative`, no background or border, padding 0) over the **name** (`--row-height` tall, full width). Horizontal positions are from the strip's centre line (`50%`), as the board places them; every part below is `position: absolute` inside the fader and `aria-hidden`. Each carries `data-part` as named.

| Part (`data-part`) | Box | Drawn |
|---|---|---|
| `value` | left 0, right 0, top 0, height and line-height `--row-height`; centred; no wrap | `valueText(layer, shown)`; `--text-18` / `--weight-light`; colour below |
| `meter-track` (two) | left `calc(50% + var(--meter-peak-x))` and `calc(50% + var(--meter-rms-x))`; top `--track-top`, bottom `--track-bottom` (`--travel` tall); width `--meter-width` | `--mbg` |
| `meter-peak` | left `calc(50% + var(--meter-peak-x))`, bottom `--track-bottom`, width `--meter-width`, height `height(meter.peak)` px (inline style) | the Meter column of the colour table |
| `meter-rms` | left `calc(50% + var(--meter-rms-x))`, bottom `--track-bottom`, width `--meter-width`, height `height(meter.rms)` px | as `meter-peak` |
| `hold` | left `calc(50% + var(--meter-peak-x))`, width `--tick-width`, height `--line-width`, bottom `tickBottom(meter.hold)` px (also at `hold` 0: it then sits on the track's bottom, as the board draws; D4) | `--peak` |
| `groove` | left `calc(50% + var(--groove-x))`, top `--track-top`, bottom `--track-bottom`, width `--groove-width` | `--track`; unused: `repeating-linear-gradient(to bottom, var(--line) 0 var(--dash), transparent var(--dash) calc(var(--dash) + var(--space-4)))` |
| `fill` | left `calc(50% + var(--groove-x))`, width `--groove-width`, `top` / `height` from `fillBox(layer, shown)` (inline style) | colour below; glow `box-shadow: 0 0 var(--glow-blur) color-mix(in srgb, var(--<hue>) var(--fill-glow-mix), transparent)` only in the Plain row (then `data-glow` is on the element); none otherwise |
| `cap` | left `calc(50% + var(--cap-x))`, width `--meter-width`, height `--groove-width`, top `capTop(shown) + 23` px | colour below |
| `ghost` | left `calc(50% + var(--ghost-x))`, width `--ghost-width`, height 0, top `ghostTop(position)` px; `border-top: var(--line-width) dashed var(--m)` | only while `waiting` and `position` isn't null |
| `wait` | left 0, top `--track-top`; `--text-13` / `--weight-regular`, line-height `--text-13` | "↕" in `--m`, only while `waiting` |

The meter parts (`meter-track` ×2, `meter-peak`, `meter-rms`, `hold`) are drawn together or not at all: drawn when `meter` isn't null, `value` isn't null, `layer` is `volume` and `rackTarget` is false. Not drawn means not in the DOM. The set level (groove, fill, cap) outranks the meter: it comes later in the DOM. Unused: no `fill`, no `cap` in the DOM.

**The name** (`data-part="name"`), `--row-height` tall, full width, padding 0, no background, no border, no radius; `display: flex`, centred both ways, gap `--space-4`, no wrap. The label `--text-13` / `--weight-medium`, line-height `--name-line`, colour below; then `PartMarks`. It is:

- a `<button type="button">` when `nameAction` isn't empty and the strip isn't unused: `cursor: pointer`, `data-tip` = `nameTip`, the accessible name under Accessibility;
- otherwise a `<span>`, `aria-hidden="true"` (the slider's name already says it), no `data-tip`, `cursor: default`, not focusable (D19). Unused: the text "—" in `--d`, no marks.

**Colours by state.** One row applies, the first that matches, top to bottom. `data-hue` (the token name without `--`) is on the `value`, `name`, `fill` and `cap` elements; at 35% the fill and cap carry the base token's name and the strip's root element carries `data-off`.

| State | When | `value` text | `name` | `fill`, `cap` | Fill glow | Meter (`meter-peak`, `meter-rms`) |
|---|---|---|---|---|---|---|
| Unused | `value` null | empty, the element present with `data-hue="d"` | "—" `--d`, `data-hue="d"` | not drawn | — | none |
| Part off | `off` | `--d` | `--d` | `color-mix(in srgb, <c> var(--off-mix), transparent)`, `<c>` being `--t` when `layer` isn't `volume`, else the hue | none | `color-mix(in srgb, var(--<hue>) var(--off-mix), transparent)` (D15; none in a layer) |
| Rack target | `rackTarget` | `--t2` | `--t2` | `--t2` | none | none |
| Layer | `layer` isn't `volume` | `--t` | the hue | `--t` | none | none |
| Plain | otherwise | the hue | the hue | the hue | yes | `color-mix(in srgb, var(--<hue>) var(--meter-mix), transparent)` |

`waiting` adds "↕" and the ghost to any row but Unused. The marks show in every row but Unused. The root also carries `data-layer` (the `layer` value) and `data-waiting` / `data-rack` / `data-unused` when those apply.

- **Tokens used:** `--t`, `--t2`, `--m`, `--d`, `--r1`, `--r2`, `--r3`, `--l`, `--a`, `--mbg`, `--line`, `--focus`, `--track`, `--peak`, `--meter-mix`, `--fill-glow-mix`, `--off-mix`, `--travel`, `--font-sans`, `--text-13`, `--text-18`, `--weight-light`, `--weight-regular`, `--weight-medium`, `--space-4`, `--line-width`, `--focus-offset`.
- **New tokens** (values from kit.md › Tokens to add; `--off-mix` is this spec's). They land with the tokens contract PR (the orchestrator), not in this component's PR; this PR never edits `app/src/ui/tokens/*` or `contrast.test.ts`.

  | Token | Dark | Light | File |
  |---|---|---|---|
  | `--track` | `var(--grey-20)` #333 | `var(--stone-76)` #c4c3bf | `dark.css`, `light.css` |
  | `--peak` | `color-mix(in srgb, var(--white) 85%, transparent)` | `var(--t)` | `dark.css`, `light.css` |
  | `--meter-mix` | `75%` | `60%` | `dark.css`, `light.css` |
  | `--fill-glow-mix` | `45%` | `0%` | `dark.css`, `light.css` |
  | `--off-mix` | `35%` | `35%` | `dark.css`, `light.css` |
  | `--travel` | `223px` | (same) | `scale.css` |
- **Size:** `width` (or the container's) × `--strip-height`. The fader `--fader-height`, the name `--row-height`. The track runs from 24 to 247 inside the fader (`--travel` 223).
- **States drawn by:** the colour table above, plus:
  - waiting: "↕" and the dashed ghost line;
  - dragging: `cursor: ns-resize` on the fader and `data-dragging` on it; at rest the fader and the name button use `cursor: pointer`; the plain-`<span>` name and everything of an unused strip `default` (D35);
  - keyboard focus: the fader and the name button each show a `--line-width` `--focus` outline at `--focus-offset` on `:focus-visible`; nothing on mouse focus;
  - no hover or pressed look.
- **Type:** DM Sans (`--font-sans`), tabular numerals; value 18 / 300 (`--text-18`, `--weight-light`); name 13 / 500 (`--text-13`, `--weight-medium`); "↕" 13 / 400 (`--text-13`, `--weight-regular`). Case as given. The value never wraps (the widest, "Rev 127", is about 60px at 18 / 300; it may overhang the column into the 8px gaps).
- **Contrast** (text on `--g`, AA 4.5:1; ratios measured from the theme values):

  | Pair | Where | Dark | Light | |
  |---|---|---|---|---|
  | `--r1` on `--g` | value, name | 6.50 | 5.53 | add to `contrast.test.ts` with the tokens PR |
  | `--r2` on `--g` | value, name | 6.86 | 4.60 | add with the tokens PR |
  | `--a` on `--g` | Style value, name | 7.80 | 5.95 | add with the tokens PR |
  | `--r3` on `--g` | value, name | 9.91 | **3.65** | dark: add with the tokens PR; light: known failure |
  | `--l` on `--g` | value, name | 9.75 | **3.58** | dark: add with the tokens PR; light: known failure |
  | `--t2`, `--t`, `--m` on `--g` | Multi Pad, Master, layer and rack text, "↕" | ≥ 6.25 | ≥ 5.24 | rows exist |
  | `--d` on `--g` | part off, unused "—" | **2.48** | **1.94** | known failure (D3) |

  **Known failures (owner question O-contrast):** `--r3` on `--g` light 3.65:1; `--l` on `--g` light 3.58:1; `--d` on `--g` 2.48:1 dark, 1.94:1 light. Until the owner answers, every story's axe check excludes exactly these elements, through the meta's `parameters.a11y.context.exclude`, from `KNOWN_CONTRAST_FAILURES` exported by `app/src/ui/FaderStrip/a11y.ts` (FaderBank reuses it):

  ```ts
  /** Elements whose text fails AA today (owner question O-contrast); axe skips them until it is answered. */
  export const KNOWN_CONTRAST_FAILURES = [
    '[data-part="value"][data-hue="r3"]', '[data-part="name"][data-hue="r3"]',
    '[data-part="value"][data-hue="l"]', '[data-part="name"][data-hue="l"]',
    '[data-hue="d"]',
  ]
  ```

  The exclusion applies in both themes (a story's parameters can't vary by theme); the dark rows are still checked by `contrast.test.ts`.
- **Motion:** none of its own. The meter moves only when the `meter` prop changes (the wiring passes a new frame about 30 times a second); the cap moves with the shown value.

### Accessibility

- **Role and name:** the fader is `role="slider"`, `tabindex="0"`, `aria-orientation="vertical"`, `aria-valuemin="0"`, `aria-valuemax="127"`, `aria-valuenow` the shown value, `aria-label` `"{name} {what}"` where `what` is `volume`, `pan`, `reverb send`, `chorus send`, `delay send` by `layer` ("Right 1 volume", "Style volume", "Master volume", "Right 2 reverb send"); a rack target reads `"{name}, rack fader"` ("PANR2, rack fader"). `aria-valuetext` is `"{name} {spokenValue(layer, shown)}"`, plus `", hardware fader away"` while `waiting`: "Right 1 90", "Right 2 72, hardware fader away", "Right 1 pan left 20", "Right 1 reverb 40".
- **Unused:** the fader is a `<div role="img" aria-label="Fader {index} unused">` ("Fader 7 unused"), no `tabindex`, no slider attributes; it handles no pointer, wheel or key, and calls nothing; the name "—" is an `aria-hidden` span (D10).
- **Name button:** the strip owns its `aria-label`: `"{name}{marksText({ edited, missing, failed })}: {nameAction}"`, `marksText` from `app/src/ui/PartMarks/marks.ts` (", edited", ", plugin missing" or ", plugin failed", in that order as they show; ⚠ wins): "Right 2, edited: open Channel", "Right 3, plugin missing: open Channel" (also with `failed` true), "Style: open the Style fader page", "Master: open Effects", "PANR2: open the Rack page". No sound name (D11, D16). The plain-`<span>` name is `aria-hidden`.
- **Keyboard:** Tab focuses the fader, then the name button. On the fader: ArrowUp / ArrowRight +1, ArrowDown / ArrowLeft −1, PageUp +10, PageDown −10 (`keyDelta`, `stepValue`); each handled key calls `preventDefault` and `stopPropagation`, so the window's ← → (`stepStyle`) don't fire while a fader has focus (Stage.md D36). Other keys pass through untouched. Enter or Space on the name button calls `onopen`.
- **Pointer:** every pointer handler (`onpointerdown`, `onpointermove`, `onpointerup`, `onpointercancel`) is on the fader element itself, never on `window` (D20). `pointerdown` with the primary button (`button === 0`) anywhere on the fader (the value text included) records `v0` (the shown value), `y0` (`clientY`) and its `pointerId`, and captures the pointer with `el.setPointerCapture?.(pointerId)` (guarded: jsdom has none), so the moves keep coming to the fader when the pointer leaves it. Each `pointermove` of that `pointerId` during the press sets the shown value to `dragValue(v0, y0, clientY)` and calls `onchange` if that changed it; `pointerup` or `pointercancel` of that pointer ends the press, releases the capture with `el.releasePointerCapture?.(pointerId)` and calls `onrelease(shown)` if the press changed the value (D6). A move with no press in progress does nothing. A `wheel` event over the fader calls `preventDefault` and steps by `wheelDelta(deltaY)`. `dblclick` on the fader sets the shown value to `resetValue(layer)` (calling `onchange` if that changed it). Plays dispatch these events on the slider with `fireEvent` (`fireEvent.pointerDown(slider, { button: 0, pointerId: 1, clientY: 200 })`, then `pointerMove` / `pointerUp` with the same `pointerId`).
- **Tooltip ids** (the parent passes them as `tip` / `nameTip`; listed so the parent's table can be checked): fader — Right 1–3, Left in volume `mixer.panel.right1`, `mixer.panel.right2`, `mixer.panel.right3`, `mixer.panel.left`; in pan `mixer.part.pan`, reverb `mixer.part.reverb`, chorus `mixer.part.chorus`, delay `mixer.part.variation`; Style `mixer.style_level`; Multi Pad `mixer.pad_level`; Master `mixer.master`; any strip while `waiting` `mixer.pickup`; rack target `launchkey.fader_rack`; unused `launchkey.fader_unused`. Name — parts `mixer.strip.select`, Style `mixer.style_level`, Multi Pad `mixer.pad_level`, Master `mixer.master`, rack target `launchkey.fader_rack`. All exist in `app/src/help/tooltips.ts`.
- **Launchkey:** strip *n* is fader *n* (1–8) on the active fader page; strip 9 is the master fader. The strip doesn't know; FaderBank's table says it.

## Stories (Story station)

- **Title:** `Primitives/FaderStrip`.
- **Layout:** `centered`. Every story sets `width: 65.556` (the board's column) and renders in dark and light.
- **Crops:** none, for every story: each instance on the board is 65.556 wide at a fractional x, so no story has a whole-pixel box. The strips' pixels are checked by `Components/FaderBank` › `Board` (crop `Stage 24,432 654×368`), whose nine strips are this table's `Board`, `RightTwoWaiting`, `PartOff`, `Left`, `Style`, `MultiPad`, `Unused` (×2) and `Master`.
- Callbacks are actions (`onchange`, `onrelease`, `onopen`: `fn()`). Every prop is a control.
- The meta sets `parameters: { a11y: { context: { exclude: KNOWN_CONTRAST_FAILURES } } }` (from `./a11y.ts`; Visual rules › Contrast).

Shared args: `R1 = { index: 1, label: 'Right 1', value: 90, hue: 'r1', meter: { peak: 0.0724, rms: 0.0537, hold: 0.1259 }, nameAction: 'open Channel', tip: 'mixer.panel.right1', nameTip: 'mixer.strip.select', width: 65.556 }`. `R3 = { index: 3, label: 'Right 3', value: 64, hue: 'r3', off: true, missing: true, failed: true, meter: { peak: 0, rms: 0, hold: 0 }, nameAction: 'open Channel', tip: 'mixer.panel.right3', nameTip: 'mixer.strip.select', width: 65.556 }`.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `R1` (strip 1 of the board fixture) | Right 1 at 90: blue value, two blue meter bars 138 and 129 tall, the white tick 161 from the bottom, the cap 88 down with the fill's glow, the blue name | — (fractional; see Crops) | the slider named "Right 1 volume" has `aria-valuenow="90"`, `aria-valuetext="Right 1 90"`, `tabindex="0"`, `data-tip="mixer.panel.right1"`; `[data-part=value]` reads "90" with `data-hue="r1"`; `[data-part=meter-peak]` has inline height `138px`, `[data-part=meter-rms]` `129px`, `[data-part=hold]` bottom `161px`; `[data-part=fill]` top `89px`, height `158px`, has `data-glow`; `[data-part=cap]` top `88px`; the button named "Right 1: open Channel" has `data-tip="mixer.strip.select"` |
| `RightTwoWaiting` | `{ index: 2, label: 'Right 2', value: 72, hue: 'r2', meter: { peak: 0.0224, rms: 0.0180, hold: 0.0447 }, waiting: true, position: 50, edited: true, nameAction: 'open Channel', tip: 'mixer.pickup', nameTip: 'mixer.strip.select', width: 65.556 }` (strip 2 of the board) | pink strip at 72, "↕" top-left, the dashed ghost 159 down, the edited dot after "Right 2" | — | `aria-valuetext` is "Right 2 72, hardware fader away"; `[data-part=wait]` reads "↕"; `[data-part=ghost]` top `159px`; root has `data-waiting`; name button "Right 2, edited: open Channel"; the slider's `data-tip` is `mixer.pickup` |
| `Edited` | `{ ...R1, index: 2, label: 'Right 2', value: 72, hue: 'r2', meter: { peak: 0.0224, rms: 0.0180, hold: 0.0447 }, edited: true, tip: 'mixer.panel.right2' }` | the edited dot alone, no takeover | — | no `[data-part=wait]`, no `[data-part=ghost]`; name button "Right 2, edited: open Channel" |
| `PartOff` | `R3` (strip 3 of the board: off, plugin missing, its status failed) | grey "64" and "Right 3", fill and cap orange at 35%, the empty meter with its tick on the bottom, the orange ⚠ and no ✕ | — | root has `data-off`; value and name `data-hue="d"`; fill and cap `data-hue="r3"`; fill has no `data-glow`; `[data-part=meter-peak]` and `[data-part=meter-rms]` height `0px`, `[data-part=hold]` bottom `5px`; exactly one `[data-mark]`, `missing`; name button "Right 3, plugin missing: open Channel" |
| `Failed` | `{ ...R3, missing: false, failed: true }` | the red ✕ in place of ⚠ | — | name button "Right 3, plugin failed: open Channel" |
| `Left` | `{ index: 4, label: 'Left', value: 80, hue: 'l', meter: { peak: 0.0316, rms: 0.0248, hold: 0.0631 }, nameAction: 'open Channel', tip: 'mixer.panel.left', nameTip: 'mixer.strip.select', width: 65.556 }` (strip 4) | teal strip at 80 | — | meter heights `111px` and `104px` (D18), hold bottom `139px`; cap top `106px`; value and name `data-hue="l"` (excluded from axe, O-contrast) |
| `Style` | `{ index: 5, label: 'Style', value: 100, hue: 'a', meter: { peak: 0.1259, rms: 0.0897, hold: 0.2188 }, nameAction: 'open the Style fader page', tip: 'mixer.style_level', nameTip: 'mixer.style_level', width: 65.556 }` (strip 5) | violet group strip | — | slider "Style volume"; heights `156px`, `145px`, hold `179px`; cap top `70px`; name button "Style: open the Style fader page" |
| `MultiPad` | `{ index: 6, label: 'Multi Pad', value: 90, hue: 't2', meter: { peak: 0, rms: 0, hold: 0.0023 }, nameAction: 'open Multi Pads', tip: 'mixer.pad_level', nameTip: 'mixer.pad_level', width: 65.556 }` (strip 6) | grey-white group strip, silent, its tick low at 32 | — | heights `0px`, hold bottom `32px`; value `data-hue="t2"` |
| `Master` | `{ index: 9, label: 'Master', value: 100, hue: 't', meter: { peak: 0.1445, rms: 0.1020, hold: 0.2512 }, nameAction: 'open Effects', tip: 'mixer.master', nameTip: 'mixer.master', width: 65.556 }` (strip 9) | white master strip | — | slider "Master volume"; heights `161px`, `149px`, hold `183px`; name button "Master: open Effects" |
| `Unused` | `{ index: 7, label: '', value: null, hue: 't', tip: 'launchkey.fader_unused', width: 65.556 }` (strips 7, 8) | no value, the dashed groove, "—" in `--d` | — | no element has role `slider`; the element named "Fader 7 unused" has role `img`, no `tabindex`, `data-tip="launchkey.fader_unused"`; `[data-part=value]` is empty with `data-hue="d"`; `[data-part=name]` is a `span` reading "—" with `data-hue="d"` and `aria-hidden="true"`; no `[data-part=cap]`, no `[data-part=fill]`, no `[data-mark]`; no button; `fireEvent` pointerDown / pointerMove (40px up) / pointerUp, a wheel and an ArrowUp keyDown on it call no callback |
| `PlainName` | `{ ...R1, nameAction: '' }` | the same strip; the name isn't a control | — | no button; `[data-part=name]` is a `span` with `aria-hidden="true"`, no `data-tip`, computed `cursor` `default`; the slider "Right 1 volume" is still there |
| `RackTarget` | `{ index: 2, label: 'PANR2', value: 64, hue: 'r2', rackTarget: true, meter: { peak: 0.0224, rms: 0.0180, hold: 0.0447 }, nameAction: 'open the Rack page', tip: 'launchkey.fader_rack', nameTip: 'launchkey.fader_rack', width: 65.556 }` | "PANR2", value, fill and cap in `--t2`, no meter although one is passed | — | slider "PANR2, rack fader"; no `[data-part^=meter]`, no `[data-part=hold]`; value and name `data-hue="t2"`; name button "PANR2: open the Rack page" |
| `Pan` | `{ ...R1, layer: 'pan', value: 44, tip: 'mixer.part.pan' }` | "L20" in white, the white fill growing down from the 64 line to the cap, no meter | — | value "L20" with `data-hue="t"`; slider "Right 1 pan", `aria-valuetext` "Right 1 pan left 20"; fill top `135px`, height `35px`; no meter parts; name `data-hue="r1"` |
| `PanRight` | `{ ...R1, layer: 'pan', value: 84, tip: 'mixer.part.pan' }` | "R20", the fill growing up from the 64 line | — | value "R20"; fill top `100px`, height `35px` |
| `Reverb` | `{ ...R1, layer: 'reverb', value: 40, tip: 'mixer.part.reverb' }` | "Rev 40" in white, white fill and cap, no meter | — | value "Rev 40"; slider "Right 1 reverb send", valuetext "Right 1 reverb 40"; no meter parts; fill height `70px` |
| `Delay` | `{ ...R1, layer: 'delay', value: 0, tip: 'mixer.part.variation' }` | "Dly 0", the cap on the track's bottom | — | value "Dly 0"; cap top `246px`; fill height `0px` |
| `Top` | `{ ...R1, value: 127, meter: { peak: 1, rms: 0.9, hold: 1 } }` | the top of the range: cap at 23, full peak bar, tick at 228 | — | cap top `23px`; peak height `223px`; hold bottom `228px` |
| `Bottom` | `{ ...R1, value: 0, meter: { peak: 0, rms: 0, hold: 0 } }` | the bottom of the range | — | cap top `246px`; fill height `0px` |
| `Drags` | `R1` | — | — | `fireEvent.pointerDown(slider, { button: 0, pointerId: 1, clientY: 200 })`, `fireEvent.pointerMove(slider, { pointerId: 1, clientY: 160 })` → `onchange` last called with 113 and `aria-valuenow` is "113"; `fireEvent.pointerUp(slider, { pointerId: 1, clientY: 160 })` → `onrelease` called once, with 113; then pointerDown at 200 and pointerUp without moving → `onrelease` still called once in all; a pointerMove with no press → `onchange` not called again |
| `Steps` | `R1` | — | — | focus the slider; ArrowUp → `onchange(91)`; PageDown → `onchange(81)`; ArrowLeft → `onchange(80)`; a `wheel` with `deltaY` −100 → `onchange(81)`; double-click → `onchange(100)`; double-click again → `onchange` not called again; the ArrowUp keydown event has `defaultPrevented` true |
| `PanReset` | `{ ...R1, layer: 'pan', value: 44, tip: 'mixer.part.pan' }` | — | — | double-click → `onchange(64)`; value reads "C" |
| `OpensName` | `R1` | — | — | click the button "Right 1: open Channel" → `onopen` called once; Tab from the slider focuses that button |
| `Focused` | `R1`; `parameters: { pseudo: { focusVisible: ['[role=slider]'] } }` | the focus ring around the fader | — | — |

What jsdom can't check (colours, `color-mix`, the glow, the dashed groove, how it all lines up) is checked by FaderBank's `Board` crop in Chrome; the inline px values in the plays are what jsdom can.

**A prop change during a drag** (D17) needs a rerender, which a story `play` can't do, so it is a vitest component test, `app/src/ui/FaderStrip/FaderStrip.test.ts` (`@testing-library/svelte`, `fireEvent` on the slider, spy callbacks), rendering `R1`:

1. pointerDown at `clientY` 200; `rerender({ ...R1, value: 50 })`; pointerMove to 160 → `onchange` called with 113 (from `v0` 90, not 50) and `aria-valuenow` "113"; pointerUp → `onrelease(113)`.
2. Then `rerender({ ...R1, value: 113 })` → `aria-valuenow` "113"; `rerender({ ...R1, value: 60 })` → "60" (props are taken again after the drag).
3. Without a drag: `rerender({ ...R1, value: 40 })` → `aria-valuenow` "40" and `onchange` not called.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `meter.test.ts`, `drag.test.ts` and `value.test.ts` assert every example in Pure functions, and `FaderStrip.test.ts` passes (`npx vitest run src/ui/FaderStrip`).
- `npm run shots -- FaderStrip` passes: no story has a crop, so this is axe on every story in Chrome, which finds no violation outside the Known failures it excludes (O-contrast); FaderBank's shots cover the pixels.
- Only listed tokens and the root's custom properties are used; no inline colours; no literal sizes outside the Geometry table (the inline px come from the pure functions).
- The component PR doesn't touch `app/src/ui/tokens/*` or `contrast.test.ts` (the new tokens land with the tokens contract PR first).
- svelte-check and lint pass on the folder.

## Decisions

- D1. The strip draws a local shown value during a drag, wheel or key change and takes the `value` prop again whenever it changes, so the cap follows the hand at once without waiting for the state's round trip.
- D2. A part that is off keeps its meter track and held-peak tick, as the board draws Right 3, rather than hiding the meter as kit › FaderStrip says; a part that doesn't sound reads empty anyway.
- D3. `--d` text (a part that is off, the unused "—") stays as the board draws it although it is under 4.5:1, because it marks absence (WCAG 1.4.3's inactive-component exemption in spirit); it is listed under Known failures with the light `--r3` and `--l` text, and axe excludes exactly those elements (`KNOWN_CONTRAST_FAILURES`) until the owner answers O-contrast.
- D4. The held-peak tick draws at `hold` 0, on the track's bottom (`bottom: 5px`), as the board's Right 3 shows, rather than disappearing.
- D5. Home and End aren't handled (kit › Interaction conventions names only arrows and Page keys, and one key jumping the master to 127 is risky).
- D6. `onrelease` fires only when the press changed the value, so a click, or the two presses of a double-click, send nothing extra.
- D7. The spoken value uses words ("pan left 20", "reverb 40") where the screen shows abbreviations ("L20", "Rev 40").
- D8. Pan's fill grows from `capTop(64)` (111px down the track), not the track's geometric middle (111.5), so pan 64 draws no fill and L20 and R20 are the same length.
- D9. A rack target's value, fill and cap are `--t2` like its name (the kit gives only the name's colour), with no meter, since its value isn't the part's level.
- D10. The unused fader is `role="img"` named "Fader 7 unused" and not focusable (Stage.md Check 1), not a disabled slider.
- D11. The slider's name is the strip name plus what it moves ("Right 1 volume") and its value text the kit's "Right 1 90"; the board's sound name in the label ("Right 1 · Stage Grand") is left out because the strip doesn't know the sound.
- D12. The unused groove's dashes are `--line` in both themes, as the kit says; the light board draws them `#c4c3bf` (`--track`), a few hundred pixels under the shot threshold.
- D13. Tooltip keys come in as props (`tip`, `nameTip`) rendered as `data-tip` on the strip's own fader and name button (not on a child primitive), because `app/src/ui` may not import the `tip` action from `app/src/lib`; integration attaches the help behaviour to `[data-tip]`.
- D14. Part off in a layer draws its fill and cap at 35% of `--t` (the layer colour), as the board's data script does, not 35% of the part hue.
- D15. A part that is off draws its meter bars at 35% of its hue (`--off-mix`), like its fill and cap, rather than the board's transparent bars or the full `--meter-mix`; a part that is off is silent, so this only shows if a meter frame says otherwise.
- D16. The strip owns its name button's `aria-label` and builds the marks' words with PartMarks' `marksText`; it has no sound name, so the name reads "Right 2, edited: open Channel", not PartMarks' example "Right 2, Silk Strings, edited: open Channel" (that example is for PartMarks' spec to correct).
- D17. A `value` prop change during a drag is ignored (the drag's `v0` and `y0` stay), so a state echo or a hardware move can't make the cap jump under the hand; the first change after the drag is taken, and the wiring always sends the drag's last value on release (D6).
- D18. The meter heights follow the formula: `height(0.0316)` = 111 (111.48), not the 112 that Stage.md's fixture states and the board drew with its own rounding; the 1px difference is under the shot threshold.
- D19. A name with no action (`nameAction` empty) is an `aria-hidden` `<span>` with no `data-tip` and `cursor: default`: it isn't a control, and the slider's name already says it.
- D20. The drag's pointer handlers are on the fader itself, with pointer capture (guarded with `?.` for jsdom), not on `window`, so plays drive it with `fireEvent` on the slider.

## Cross-lane needs

- PartMarks (primitives lane): its Accessibility example for FaderStrip, "Right 2, Silk Strings, edited: open Channel", should read "Right 2, edited: open Channel" (D16). No prop change.
- Tokens contract PR (orchestrator): `--track`, `--peak`, `--meter-mix`, `--fill-glow-mix`, `--off-mix`, `--travel` and the contrast rows, per Visual rules.

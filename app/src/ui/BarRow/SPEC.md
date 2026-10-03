# BarRow

## Identity (all stations)

- **Kind:** primitive
- **Built from:** — (imports the pure `splitUnit` from `app/src/ui/Knob/splitUnit.ts`, Effects.md component 47, which lands with the Knob edit first; D9)
- **Purpose:** Shows one setting as a label, a bar and a number, and sets it by dragging across the row, the wheel or the arrow keys (the Push-style parameter control: no slider or knob on screen).
- **Boards:**
  - `Effects-Dark.dc.html:217-222` (the readout grid's rows Note, Feedback, Tone, Return, Band send, Pad send; data `:578-591`); light: `Effects-Light.dc.html:201-206` (data `:562-575`). The `param` variant.
  - `Harmony-Dark.dc.html:176-185` (Volume with "Knob 5", Touch limit); light: `Harmony-Light.dc.html:166-175`. The `setting` variant.
- **Not this component's job:** no store, no API, no Tauri. It doesn't know which command a value goes to: the parent passes `value`, `min`, `max`, `defaultValue` and `display`, and acts on `onchange`. It never formats a value for a unit (the parent passes `display`, from the state's `SettingState.display` or its own text). No hairline list around it (the column or grid is the parent's; the row draws only its own one hairline, D4). No knob page, no tooltip text, no "Knob n" lookup (the parent passes `sublabel`).

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `variant` | `'param' \| 'setting'` | `'param'` | The skin and the bar's base (D2). `param`: the Effects readout (a 96 / bar / 76 / 28 grid, 36 tall, accent value, knob code, hairline below; the bar spans `min`–`max`). `setting`: the settings-column row (64px label cell, bar, 44px value, hairline above; the bar spans 0–`max`). |
| `size` | `'row' \| 'row-sm'` | `'row'` | `setting` only: 44 tall (`row`) or 36 (`row-sm`). Ignored by `param`, which is always 36. The names match ChosenTabs' row sizes, so a ChoiceRow and a BarRow of one size are one height. |
| `label` | `string` | — | The name ("Feedback", "Volume", "Touch limit"). Drawn, and the slider's `aria-label`. |
| `sublabel` | `string \| undefined` | — | `setting` only: a second label line ("Knob 5"). Absent: one line. Ignored by `param`. |
| `value` | `number` | — | The state's value (an integer). |
| `min` | `number` | `0` | The lowest value. |
| `max` | `number` | `127` | The highest value. |
| `defaultValue` | `number \| undefined` | — | What a double-click sets (a return's 64, Volume's 100, Touch limit's 1). Undefined: double-click does nothing. |
| `display` | `string \| undefined` | — | The value as text, unit included ("38%", "5.0 kHz", "1/8", "375 ms"), from the state. Split by `splitUnit` into the number and the unit. Undefined: the shown value as an integer. |
| `code` | `string \| undefined` | — | `param` only: the Launchkey knob that moves it ("K5"); empty cell when undefined. Ignored by `setting`. |
| `disabled` | `boolean` | `false` | Shown, focusable, sends nothing (L5: `aria-disabled="true"` only when true). |
| `tip` | `string \| undefined` | — | The tooltip key (`fx.param.delay_feedback`, `harmony.volume`), rendered as `data-tip` on the slider element (L3). |
| `tipAction` | `Action<HTMLElement, string> \| undefined` | — | The app's `use:tip`, passed in by the wiring; applied as `use:tipAction={tip}` only when both are set (L3). Stories pass `fn()`. |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onchange` | a drag, wheel notch, key or double-click produces a whole value different from the baseline (Behaviour below); never while `disabled` | `(value: number)` the new value, clamped to `min`–`max` |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Behaviour

One rule set for both variants (D1, D3; Effects FX-D9, Harmony HA-D7, HA-D26):

- **Shown value.** `shown = pending ?? value`. `pending` is the last value this row sent; it is cleared (no timer) as soon as the `value` prop changes. The bar, `aria-valuenow` and every step start from `shown`, so two quick ArrowUps from 100 send 101 then 102. The text is `display` when given (the state's, so it follows the echo; D5), else `String(shown)`.
- **Drag.** `pointerdown` with `button` 0 anywhere on the element (not only the bar) stores `v0 = shown`, `x0 = clientX` and `width` = the bar element's (`[data-part="bar"]`) `getBoundingClientRect().width`, and calls `setPointerCapture?.(pointerId)` inside try/catch (L4). Each `pointermove` while pressed computes `dragValue(v0, x − x0, width, min, max, base)` and stores it; if no frame is pending it schedules one `requestAnimationFrame` (handle kept in `frame`, 0 when none). The frame clears `frame` and, if the stored value differs from the baseline, calls `onchange` and makes it the new baseline and `pending`. The baseline is `shown` at pointerdown, then each value the gesture sent. `pointerup` (and `pointercancel`) calls `cancelAnimationFrame(frame)`, sets `frame` 0 and sends the stored value once if it differs from the baseline. A press without movement sends nothing. The element carries `data-dragging` while pressed (cursor `ew-resize`).
- **Wheel.** One step per event whatever its size: `deltaY` < 0 is +1, > 0 is −1; when `deltaY` is 0, `deltaX` > 0 is +1 and < 0 is −1; both 0: nothing (D6). The event is `preventDefault`ed (the page doesn't scroll). Sends at once.
- **Keys.** → and ↑ +1, ← and ↓ −1, PageUp / PageDown ±10, Home `min`, End `max`; Space and Enter do nothing. Every one of these keys, Space and Enter included, is `preventDefault`ed and `stopPropagation`ed, so the window handler (`app/src/lib/shortcuts.ts`: Space Start / Stop, Enter the Browser, PageUp/PageDown the pad page, ← → `stepStyle`) never sees them from a focused row. Sends at once.
- **Double-click.** Sends `defaultValue` once when it differs from `shown`.
- **Clamp and send.** Every new value is clamped to `min`–`max` and rounded; a step that leaves it unchanged (at a bound) sends nothing. A sent value becomes `pending`.
- **Disabled.** Keys, Space and Enter are still stopped (so a focused disabled row doesn't start the transport); nothing is sent, the wheel isn't prevented, no drag starts.
- **Maths** in `app/src/ui/BarRow/bar.ts`, pure and tested; `base` is `min` for `param`, `0` for `setting` (D2):
  - `fraction(value, min, max, base = min)` = `(value − base) / (max − base)`, clamped to 0–1; 0 when `max === base`.
  - `fillPercent(value, min, max, base = min)` = `fraction × 100` as a CSS percentage with one decimal ("78.7%" for 100 of 0–127 at base 0; "0.8%" for 1 of 1–127 at base 0; "42.2%" for 38 of 0–90).
  - `dragValue(v0, dx, width, min, max, base = min)` = `v0` when `width` ≤ 0, else `clamp(round(v0 + dx × (max − base) / width), min, max)`. `dragValue(38, 48, 96, 0, 90)` is 83; `dragValue(100, 24, 239, 0, 127, 0)` is 113; `dragValue(1, −50, 239, 1, 127, 0)` is 1.

### Visual rules

- **Tokens used:** `--m`, `--d`, `--t`, `--t2`, `--a`, `--g`, `--line`, `--mbg`, `--past`, `--focus`, `--font-sans`, `--text-12`, `--text-13`, `--text-18`, `--text-22`, `--weight-light`, `--weight-regular`, `--space-2`, `--space-12`, `--line-width`, `--focus-offset`, and the new tokens below.

#### New tokens

Not in `app/src/ui/tokens/*` today; they land in the tokens contract PR (`scale.css`, the same in both themes) before the build (L1). Harmony.md's `--bar-height` (2px) and `--row-height` (44px) are renamed here: both names are already taken by other specs (ChosenTabs' `--bar-height` 36px, FaderStrip's `--row-height` 20px; D8).

| Token | Dark | Light | Used for |
|---|---|---|---|
| `--bar-track` | `2px` | `2px` | the bar's track and fill height |
| `--label-cell` | `64px` | `64px` | the `setting` label cell (ChoiceRow uses it too) |
| `--value-cell` | `44px` | `44px` | the `setting` value cell |
| `--settings-row` | `44px` | `44px` | `setting` at `row` (ChoiceRow uses it too) |
| `--settings-row-sm` | `36px` | `36px` | `setting` at `row-sm`, and every `param` row |
| `--readout-label` | `96px` | `96px` | the `param` label column |
| `--readout-value` | `76px` | `76px` | the `param` value column |
| `--readout-code` | `28px` | `28px` | the `param` code column |
| `--lh-14`, `--lh-16`, `--lh-20`, `--lh-24` | `14px`, `16px`, `20px`, `24px` | same | the line-heights below |
| `--space-1` | `1px` | `1px` | the two-line label's gap |

- **The element:** one `<div tabindex="0" role="slider">` (not a `<button>`: axe allows no slider role on one), its parent's width, `box-sizing: border-box`, no padding, items centred, text left, cursor `pointer` (`default` when disabled, `ew-resize` with `data-dragging`). Carries `data-variant`.
- **`param`:** height `--settings-row-sm` (36), `border-bottom: var(--line-width) solid var(--line)`, a grid `var(--readout-label) minmax(0, 1fr) var(--readout-value) var(--readout-code)`, column-gap `--space-12`:
  - **Label** (`data-part="label"`) `--text-13` / 400 `--m`, no wrap.
  - **Bar** (`data-part="bar"`, `aria-hidden`): a `--bar-track` track in `--mbg` filling its cell, with a `--a` fill from the left, `width: fillPercent(shown, min, max)`.
  - **Value** (`data-part="value"`) right-aligned, no wrap: the number `--text-18` / 300, line-height `--lh-20`, `--a`; then the unit (`data-part="unit"`) `--text-12` / 400, `--a`, `margin-left: var(--space-2)`; no unit element when the split gives none.
  - **Code** (`data-part="code"`) `--text-12` / 400 `--t2`, no wrap; empty when `code` is undefined.
- **`setting`:** height `--settings-row` (44) or `--settings-row-sm` (36) by `size`, `border-top: var(--line-width) solid var(--line)`, a flex row, gap `--space-12`, no wrap:
  - **Label cell** (`data-part="label"`), `width: var(--label-cell)`, `flex: none`, a column with gap `--space-1`, `white-space: nowrap`, `overflow: visible` ("Touch limit" at 13px is about 64px and may run a pixel into the gap): the label `--text-13` / 400, line-height `--lh-16`, `--m`; the sublabel (`data-part="sublabel"`, only when given) `--text-12` / 400, line-height `--lh-14`, `--d`.
  - **Bar** (`data-part="bar"`, `aria-hidden`, `flex: 1`): a `--bar-track` track in `--past` with a `--t2` fill from the left, `width: fillPercent(shown, min, max, 0)` (Volume 100 → 78.7%; Touch limit 1 → 0.8%, a hairline of fill, HA-D16).
  - **Value** (`data-part="value"`), `width: var(--value-cell)`, `flex: none`, right-aligned, `--text-22` / 300, line-height `--lh-24`, `--t`, the text (split by `splitUnit` like `param`; setting values are integers, so no unit shows today).
- **States drawn by:**
  - enabled: as above.
  - dragging: cursor `ew-resize`; nothing else changes.
  - disabled: label, sublabel, value and unit `--d`; no fill (the track only); cursor `default`.
  - keyboard focus (`:focus-visible`): a `--line-width` `--focus` outline, `--focus-offset` outside the whole row; nothing on mouse focus.
  - No hover, no glow.
- **Type:** DM Sans, tabular numerals on the value, the label as given.
- **Contrast (AA 4.5:1, `tokens/contrast.test.ts`):**
  - existing rows: `--m` on `--g` (labels), `--t` on `--g` (the `setting` value), `--t2` on `--g` (the code).
  - new row: `--a` on `--g`, "readout value and unit (BarRow)": 7.79 dark, 5.94 light, both pass.
  - **known failure (O-contrast, as FaderStrip D3, Knob, KnobBank):** the `setting` sublabel "Knob 5" is `--d` on `--g`, 2.48 dark, 1.94 light, and it is information, not a disabled label. Until the owner answers, the `VolumeSetting` and `Board` stories set `parameters.a11y.context.exclude: ['[data-part="sublabel"]']` (D7). Disabled text is exempt.
- **Not checkable in jsdom:** the bar's drawn width and colours; the crops cover them.
- **Motion:** none.

### Accessibility

- **Role and name:** `role="slider"`, `aria-label` = `label` (the sublabel is not part of the name), `aria-valuemin`, `aria-valuemax`, `aria-valuenow` = `shown`, `aria-valuetext`: `param` "{label} {text}" ("Feedback 38%", "Note 1/8"); `setting` "{label} {text} of {max}" ("Volume 100 of 127", "Touch limit 1 of 127"), with `{text}` the shown text (D5).
- **Keyboard:** Tab reaches it; the keys in Behaviour; it is one tab stop.
- **Tooltip id:** the parent's key, e.g. `fx.param.delay_feedback`, `fx.variation_return`, `harmony.volume`, `harmony.touch_limit` (all exist in `tooltips.ts`).

## Stories (Story station)

- **Title:** `Primitives/BarRow`.
- **Layout:** `centered`, each story in a decorator of the board's row width: 332px for `param` (the Effects column is 332.5, D10), 371px for `setting`. Every story has `tipAction: fn()` and `onchange: fn()` in the meta.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ label: 'Feedback', value: 38, min: 0, max: 90, defaultValue: 40, display: '38%', code: 'K6', tip: 'fx.param.delay_feedback' }` | the first `param` readout in reading order with a unit: muted label, accent bar at 42.2%, "38" + "%", code | `Board-{dark,light}.png`: Effects 702,291 332×36 | — |
| `Unit` | `{ label: 'Tone', value: 50, min: 10, max: 200, display: '5.0 kHz', code: 'K7', tip: 'fx.param.delay_tone' }` | the split "5.0" + "kHz" | `Unit-{dark,light}.png`: Effects 702,327 332×36 | — |
| `NoUnit` | `{ label: 'Note', value: 2, min: 0, max: 7, display: '1/8', code: 'K5', tip: 'fx.param.delay_note' }` | a text value with no unit element | `NoUnit-{dark,light}.png`: Effects 702,255 332×36 | `aria-valuetext` is "Note 1/8"; no `[data-part="unit"]` |
| `NoCode` | `{ label: 'Pad send', value: 20, min: 0, max: 127, display: '20%', tip: 'fx.variation_pad' }` | an empty code cell | — (the right column starts at x 1058.5) | — |
| `VolumeSetting` | `{ variant: 'setting', label: 'Volume', sublabel: 'Knob 5', value: 100, min: 0, max: 127, defaultValue: 100, tip: 'harmony.volume' }` | the `setting` skin, two-line label, 78.7% fill, "100" | `VolumeSetting-{dark,light}.png`: Harmony 1021,256 371×44 | `aria-valuetext` is "Volume 100 of 127"; the fill's inline width is "78.7%" |
| `TouchLimit` | `{ variant: 'setting', size: 'row-sm', label: 'Touch limit', value: 1, min: 1, max: 127, defaultValue: 1, tip: 'harmony.touch_limit' }` | 36 tall, one-line label, the hairline of fill at the bottom of the range | `TouchLimit-{dark,light}.png`: Harmony 1021,300 371×36 | the fill's inline width is "0.8%" |
| `Drags` | `Board`'s args | — | — | with `[data-part="bar"]`'s `getBoundingClientRect` stubbed to width 96 and `requestAnimationFrame` stubbed to run at once: `pointerDown` at clientX 0 (no `onchange`), `pointerMove` to 48, `pointerUp` → `onchange` called once, with 83 |
| `Keys` | `Board`'s args | — | — | focus, press → : `onchange(39)` and the event's `defaultPrevented` is true; press End: `onchange(90)`; press Space: `onchange` not called, `defaultPrevented` true |
| `Wheel` | `Board`'s args | — | — | a wheel event with `deltaY: -100` → `onchange(39)`; `deltaY: 0, deltaX: 0` → not called |
| `Reset` | `{ label: 'Return', value: 36, min: 0, max: 127, defaultValue: 64, display: '36', code: 'K8', tip: 'fx.variation_return' }` | — (right column, fractional x) | — | double-click → `onchange(64)` once |
| `Disabled` | `Board`'s args plus `disabled: true` | `--d` label and value, no fill | — (no board draws one) | `aria-disabled` is "true"; → and a drag call nothing; Space is still `defaultPrevented` |
| `Focused` | `Board`'s args; `parameters: { pseudo: { focusVisible: true } }` | the focus ring | — | — |

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`); `bar.ts` has unit tests for the examples under Behaviour › Maths.
- `npm run shots -- BarRow` passes: each cropped story's screenshot is the crop's size and scores at most 0.02, and axe (colour contrast included) finds no violation on any story apart from the excluded sublabel (D7). Above 0.02, the Inspect agent may judge the difference render noise and say so.
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · Readout and BarRow are one component.** Effects' Readout and Harmony's BarRow are the same control (a label, a 2px bar and a value; one `role="slider"` element; drag across the row, wheel, keys, double-click to default); they differ only in skin, so they are one component with `variant` `param` (Effects) and `setting` (Harmony and the Settings pages). Effects' `app/src/ui/Readout/readout.ts` becomes `app/src/ui/BarRow/bar.ts`, with Harmony's `fillPercent` beside `fraction` and `dragValue`.
- **D2 · The bar's base follows the variant.** Effects (FX-D10) fills and drags over `min`–`max`; Harmony (HA-D16, HA-D7) over 0–`max` (127 per bar width, Touch limit 1 a hairline of fill). One maths with a `base` (`min` or 0) keeps both screen specs' numbers exactly; the variant picks it.
- **D3 · One pointer, key and send model.** Effects FX-D9's and Harmony HA-D7 / HA-D26's rules are merged: relative drag over the bar's width, one send per animation frame, the last value once on release against the gesture's baseline, `pending` until the state changes, keys / wheel / double-click sending at once, every handled key plus Space and Enter prevented and stopped.
- **D4 · The hairline is the variant's.** `param` draws its line below (Effects' readout grid), `setting` above (Harmony's settings column, where every row type has a top hairline); the list around them is the parent's.
- **D5 · Text follows the state.** When `display` is given (the state's formatted value, e.g. "5.0 kHz" for a raw 50) the text and `aria-valuetext` follow the state while the bar and `aria-valuenow` follow `shown`, since the component can't format an unconfirmed raw value; the echo arrives within a frame or two. Without `display` the integer follows `shown`.
- **D6 · Wheel axis.** Effects counts `deltaX` > 0 as +; Harmony ignores `deltaX`. The merged rule reads `deltaY` first and `deltaX` only when `deltaY` is 0, which satisfies both specs' checks (a notch up is +1; no delta, nothing).
- **D7 · The sublabel's contrast is a known failure (L2).** "Knob 5" is `--d` on `--g` (2.48 dark, 1.94 light) as Harmony draws it; this joins the open owner question O-contrast (FaderStrip D3) rather than proposing a new `--d`, which would change every disabled label; the stories exclude `[data-part="sublabel"]` from axe until it is answered.
- **D8 · Token names.** Harmony.md's `--bar-height` (2px) and `--row-height` (44px) collide with ChosenTabs' `--bar-height` (36px) and FaderStrip's `--row-height` (20px), so the track is `--bar-track` and the row heights `--settings-row` / `--settings-row-sm`; the other tokens keep Harmony.md's names.
- **D9 · `splitUnit` comes from Knob.** The unit split is Effects' shared pure function (`app/src/ui/Knob/splitUnit.ts`, Effects.md › Knob unit split); this folder imports it rather than copying it, so the Knob edit lands first.
- **D10 · The 332px column.** The Effects board's readout columns are 332.5px wide; the stories and crops use 332, and the half pixel lands in the bar's flexible cell, under the screenshot threshold. The right column (x 1058.5) has no whole-pixel crop.
- **D11 · Name.** The folder is `BarRow`, not `Readout`: Rack.md's component 9 is also called `Readout` (a bare value with vertical knob-style drag, RK-D19), so this name keeps `app/src/ui/Readout/` free for it.
- **D12 · No crop files yet.** The `crops/` folder doesn't exist; the boxes above are for the crop station.
- **D13 · Story defaults.** `Board`'s `defaultValue` 40 is the delay kind's Feedback default (`crates/yahaha-fx/src/fx/kinds.rs`); in the app it comes from the state's `SettingState.default`.

Follow-ups: fine steps (Shift + drag or arrows) as the knobs have (Effects follow-up); Channel.md's `BarReadout` (a 76px label, the bar alone as the slider, a 58px value slot) is a third skin of the same control and could become a third `variant` here when the Channel lane builds it.

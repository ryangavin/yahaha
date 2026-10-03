# LampButton

## Identity (all stations)

- **Kind:** primitive
- **Built from:** — (uses the shared `longpress` action, `app/src/ui/actions/longpress`, spec `app/src/ui/actions/longpress/SPEC.md`)
- **Purpose:** Turns a part, a mode or a function on or off, and shows at a glance which it is.
- **Boards:**
  - `Stage-Dark.dc.html:97` (Accomp, lit, with the ACMP code), `:115` (Metronome, off, joined to its ▾ caret), `:116` (Unison, off), `:277-284` (the band's lamp row: part On/Off with long press, Harm/Arp, Sound, L Hold, Looper with long press); light: `Stage-Light.dc.html:73`, `:91`, `:92`, `:253-260`.
  - `Stage-Metronome-Dark.dc.html:123` (Metronome lit, joined); light: `Stage-Metronome-Light.dc.html:94`.
  - `SettingsChord-Dark.dc.html:178` (Manual Bass, disabled, 64 × 28); light: `SettingsChord-Light.dc.html:161`.
  - `Looper-Dark.dc.html:175` (Rec / Stop, the record lamp); light: `Looper-Light.dc.html:154`.
  - The kit's lamp rules: `Stage-Dark.dc.html:66`, `Stage-Light.dc.html:42`.
  - The armed (waiting) face: no board draws a LampButton armed (Looper draws `looping`; its armed outlines appear only in the state readout, which is not a LampButton), so it has no crop (D10). Its rules come from the Looper board notes (`Looper-Dark.dc.html:25-31`) and kit › Lamp row ("`recArmed` → waiting face in `--rec`; `loopArmed` → waiting face in `--lamp`").
- **Not this component's job:** no store, no API, no Tauri. It doesn't know what it switches: the parent passes `on` and `waiting` and acts on `ontoggle`, `onlongpress` and `onlongrelease`. The tooltip is the parent's key passed in as `tip`, wired by the `tipAction` the parent passes (L3); the library never imports `use:tip`. Long press comes only from the shared `use:longpress` action, never a timer of its own; Shift-click stays the parent's (it reads `ui.shift` and decides what a click means). Not the white "chosen" block, the waiting chips of the display or the ▾ caret beside Metronome: those are other components (`Button`, `WaitingChip`). Its own waiting face is only the armed lamp (Looper's Rec armed / Loop armed in the band's lamp row).

## API (Component station)

### Props

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `label` | `string` | — | The word on the face. |
| `on` | `boolean` | `false` | Lit (lamp face) or off. Controlled: the face and `aria-pressed` follow `on` alone; a click calls `ontoggle(!on)` and changes nothing until the parent passes a new `on` (Stage D49, D17). |
| `code` | `string \| undefined` | — | Small code after the label, e.g. `ACMP`. |
| `disabled` | `boolean` | `false` | Shown, not pressable (`aria-disabled="true"`, stays focusable; no `aria-disabled` attribute at all when enabled, L5). No toggle and no long press. |
| `rec` | `boolean` | `false` | The record lamp: lit is the solid `--rec` face with `--solid-ink` label instead of the lamp face. |
| `waiting` | `boolean` | `false` | The waiting (armed) face while not pressed: transparent fill, a 1px outline and the label in `hue` (Visual rules). Precedence: on (or record) > waiting > off, so it shows only while `on` is false. It never changes `aria-pressed`. |
| `hue` | `'lamp' \| 'rec'` | `'lamp'` | The waiting face's colour: `lamp` (Looper's Loop armed) draws in `--lamp-line`, `rec` (Rec armed) in `--rec`. Ignored by the other faces. |
| `size` | `'md' \| 'sm' \| 'cell'` | `'md'` | `md`: 32px tall, 14px label, 16px side padding (section row). `sm`: 28px, 13px, 14px (settings and strip rows). `cell`: 32px, 13px, no padding, fills its container's width (the band's lamp row). |
| `width` | `number \| undefined` | — | A fixed width in px with the label centred and no side padding (settings rows' 64px On/Off, strips' 72px). It wins over the size's width and padding: with `md` or `sm` the padding becomes 0 and the width is `width`; with `cell` the button is `width` px instead of filling its container. Height and label size still come from `size` (D11). |
| `join` | `'start' \| 'end' \| undefined` | — | Joined to a neighbour with no gap between their faces: `start` rounds only the left corners (`border-radius: var(--radius) 0 0 var(--radius)`, i.e. `4px 0 0 4px`), `end` only the right (`0 var(--radius) var(--radius) 0`, i.e. `0 4px 4px 0`). Undefined: all four corners `--radius`. The Metronome lamp is `join: 'start'` beside its caret (the parent, `MetronomeSplit`, sets the 1px gap). |
| `name` | `string \| undefined` | — | The accessible name when the label alone isn't enough (`Right 1 on`). Default: the label, plus the code if any. |
| `tip` | `string \| undefined` | — | The tooltip key from `app/src/help/tooltips.ts` (e.g. `transport.acmp`), rendered as `data-tip` on the `<button>`; no attribute when undefined (L3). |
| `tipAction` | `Action<HTMLElement, string> \| undefined` | — | The app's `use:tip`, passed in by the wiring. When both it and `tip` are set the `<button>` gets `use:tipAction={tip}`; otherwise nothing (L3). |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `ontoggle` | click, Space or Enter, unless disabled; not for the click that ends a long press (it is swallowed) | `(on: boolean)` the state asked for, `!on` (the button doesn't change until the parent passes it, D17) |
| `onlongpress` | the button is held 350 ms (`--long-press`) without moving more than 4px, while still held; or right-clicked; unless disabled (longpress SPEC) | `()` |
| `onlongrelease` | the press that fired `onlongpress` ends (pointer up or cancelled; a right-click's release) | `()` |

**Long press wiring.** The `<button>` carries `use:longpress={{ onlongpress, onlongrelease, disabled: disabled || onlongpress === undefined }}`. With no `onlongpress` the action is off, so clicks behave exactly as before. A long press never toggles: `ontoggle` isn't called, and `aria-pressed` stays what it was until the parent changes `on` (D1).

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Visual rules

- **Tokens used:** `--btn`, `--m`, `--d`, `--lamp`, `--lamp-ink`, `--lamp-line` (new), `--rec`, `--solid-ink`, `--focus`, `--radius`, `--font-sans`, `--text-12`, `--text-13`, `--text-14`, `--weight-regular`, `--weight-medium`, `--space-6`, `--space-14`, `--space-16`, `--control-height`, `--control-height-compact`, `--line-width`, `--focus-offset`, `--code-opacity`; and, through the action, `--long-press` (new).
- **Size:** height 32 (`md`, `cell`) or 28 (`sm`); width from the label plus side padding, or `width`, or the container (`cell`); `width` wins over both (see Props). Never wraps. `join` changes only the corner radii, never the size. **The width follows the label's weight:** without `width`, an `md` or `sm` button is its label's (and code's) text width plus its padding, and a lit label is medium (500) where an off one is regular (400), so the same label is wider lit: Metronome is 107px off and 108px on (`JoinStart` against `JoinStartOn`), Accomp ACMP is 127px on. A parent that needs a stable width passes `width` (D12). The waiting face uses the regular weight, so it is as wide as off.
- **States drawn by:**
  - off: `--btn` face, `--m` label, regular weight; code `--m`. `data-face="off"`.
  - on: `--lamp` face, `--lamp-ink` label, medium weight; code in `--lamp-ink` at `--code-opacity`. `data-face="on"`.
  - on, `rec`: `--rec` face, `--solid-ink` label and code (the code at full ink: at `--code-opacity` it fails AA on `--rec`). `data-face="record"`.
  - waiting (`waiting` true and not pressed): `background: transparent`; the 1px outline is `box-shadow: inset 0 0 0 var(--line-width) var(--<hue token>)` (no `border`, so the box is the same size as every other face, D13); label and code in the hue token, regular weight, the code at full opacity; `--lamp-line` for `hue: 'lamp'`, `--rec` for `hue: 'rec'`. `data-face="waiting"` and `data-hue="lamp"` or `"rec"` (`data-hue` only in this face). `rec` doesn't change the waiting face.
  - disabled: label and code turn `--d`, the code at full opacity (the face and the waiting outline unchanged), `cursor: default`, no press (D14); `data-face="disabled"` and `data-contrast="dim"` on the `<button>` (D18).
  - joined (`join`): the radii above; the face is otherwise unchanged in every state.
  - long press held: no change of its own (the parent may change `on` in answer).
  - keyboard focus: a `--line-width` outline in `--focus`, `--focus-offset` outside the face (the outline follows the joined radii).
  - No bar, border, glow or hover change (the kit draws none).
- **Test hook:** `data-face` on the `<button>`: `off`, `on`, `record` or `waiting` from the face as drawn, or `disabled` while disabled (D18); `data-hue` in the waiting face only (kept while a waiting lamp is disabled).
- **Type:** DM Sans, sentence case as given, tabular numerals; label 14px (`md`) or 13px (`sm`, `cell`); code 12px regular, `--space-6` after the label.

#### New tokens

Not in `app/src/ui/tokens/*` today; they land in the orchestrator's tokens contract PR before this component is built (L1). The component uses them by name and never their values.

| Token | Dark | Light | Used for |
|---|---|---|---|
| `--long-press` | `350ms` | `350ms` | the long-press time, through the action (`scale.css`) |
| `--lamp-line` | `var(--lime-400)` (`#9fe04a`, the same as `--lamp`) | `var(--lime-750)`, a new palette step `--lime-750: #477b0c` in `palette.css` | the lime of a waiting face's outline and label on the ground (`hue: 'lamp'`); `--lamp` itself stays the fill (D15) |

- **Contrast (AA 4.5:1, `tokens/contrast.test.ts`):** existing rows `--m` on `--btn`; `--lamp-ink` on `--lamp`, and at `--code-opacity`; `--solid-ink` on `--rec`. New rows: `--rec` on `--g` ("armed record lamp label (LampButton)": 5.67:1 dark, 5.27:1 light, passes) and `--lamp-line` on `--g` ("armed loop lamp label (LampButton)": 13.24:1 dark; light 4.53:1 with the new value). With today's tokens the light pair would be `--lamp` (`#4f8a0e`) on `--g` (`#f2f1ee`) at 3.74:1, which fails, and no single `--lamp` passes both it and `--lamp-ink` (black) on `--lamp` (that needs `#4b820d` or lighter, which is at most 4.13:1 on `--g`), hence the separate token (L2). The tokens contract PR lands before this component is built (D7), so `ArmedLoop` passes axe like every other story; no story carries an a11y exemption. Disabled `--d` is exempt. (`join` adds no pair.)
- **Motion:** none.

### Accessibility

- **Role and name:** a `button` with `aria-pressed` (`"true"` or `"false"`, from `on`; the waiting face is `"false"`); the accessible name is `name`, or the label plus the code, set as `aria-label`. `aria-disabled="true"` only when disabled; enabled buttons have no `aria-disabled` attribute (L5). When the button has a long press or is armed, the parent says so in `name` or the tooltip ("Right 1 on. Long press: swap mode", "Looper, rec armed").
- **Keyboard:** Tab focuses it (also when disabled); Space or Enter toggles it. The long press from the keyboard is the platform's context-menu key (Menu or Shift+F10), via the action.
- **Tooltip id:** the parent's control id passed as `tip` (e.g. `transport.acmp`, `part.right1.on`, `looper.rec`), with `tipAction` (L3).

## Stories (Story station)

Title `Primitives/LampButton`, `layout: 'centered'` unless the row says otherwise. Every story renders in dark and light (the toolbar theme). The meta's `args` are `{ ontoggle: fn(), onlongpress: fn(), onlongrelease: fn(), tipAction: fn() }` (axiom 7; `tipAction` an action so the story test accepts it, L3), so every story has a long press; a short click still toggles. The existing stories (`Off`, `On`, `Toggles`, `Disabled`, `Recording`, `PartOn`, `LongLabel`, `Focused`) keep their args exactly; new stories add a real `tip` where the table gives one.

**Controls (argTypes):** `label`, `code`, `name`, `tip` text; `on`, `disabled`, `rec`, `waiting` boolean; `size` a select of `md` / `sm` / `cell`; `hue` a select of `lamp` / `rec`; `join` a select with an empty option for undefined, `start`, `end`; `width` a number (cleared = undefined); `ontoggle`, `onlongpress`, `onlongrelease`, `tipAction` actions.

**Timing in plays (L4):** pointer events are `fireEvent.pointerDown` / `pointerUp(button, { pointerId: 1, button: 0, clientX: 0, clientY: 0 })` (jsdom 30 has `PointerEvent`). Plays use real time: "wait 500 ms" is `await new Promise((r) => setTimeout(r, 500))`, and a long press is awaited with `waitFor(…, { timeout: 1000 })`. The action's fake-timer cases are in its own unit tests.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ label: 'Accomp', code: 'ACMP', on: true, tip: 'transport.acmp' }` | the first LampButton on the Stage board at the board fixture (`transport.acmp` true): lit, with the code | `Board-{dark,light}.png` (Stage 24,68 127×32; the same box as `On`) | the button named `Accomp ACMP` has `aria-pressed="true"`, `data-face="on"`, `data-tip="transport.acmp"`; `tipAction` was called with the button and `'transport.acmp'` |
| `Off` | `{ label: 'Metronome' }` | the off face: `--btn` fill, grey label | `Off-{dark,light}.png` (Stage 1114,68 107×32; see D4) | `data-face` is `off`; the button has no `aria-disabled` attribute and no `data-hue` |
| `On` | `{ label: 'Accomp', code: 'ACMP', on: true }` | lime face, ink label, the small code | `On-{dark,light}.png` (Stage 24,68 127×32) | — |
| `Toggles` | `{ label: 'Unison' }` | — | — | click → `ontoggle` called once with `true`, and `aria-pressed` is still `false` and `data-face` still `off` (controlled: the args don't change, D17); focus it, Space → called with `true` again; Enter → again; 3 calls, every one `true`; `onlongpress` not called |
| `Disabled` | `{ label: 'Off', size: 'sm', width: 64, disabled: true, name: 'Manual Bass, works with Upper on' }` | dimmed label on the off face, 64 × 28 | `Disabled-{dark,light}.png` (SettingsChord 858,234 64×28) | `aria-disabled` is `true`, `data-face="disabled"`, `data-contrast="dim"`; click → still not pressed and `ontoggle` not called; `fireEvent.pointerDown(button, { pointerId: 1, button: 0, clientX: 0, clientY: 0 })`, wait 500 ms (real time) → `onlongpress` not called; `fireEvent.pointerUp` (same init) → `onlongrelease` not called |
| `Recording` | `{ label: 'Rec / Stop', rec: true, on: true }` | the solid record-red face | — | `data-face` is `record` |
| `PartOn` | `{ label: 'On', size: 'cell', on: true, name: 'Right 1 on' }`, `layout: 'padded'` | a part lamp filling its container, 13px label | — (the band's cells are fractional widths) | the button named `Right 1 on` is pressed |
| `LongLabel` | `{ label: 'Port sends mapped', size: 'sm', on: true }` | the longest real lamp label stays on one line | — | — |
| `JoinStart` | `{ label: 'Metronome', join: 'start' }` | the Metronome lamp off, square right corners where its caret joins | `JoinStart-{dark,light}.png` (Stage 1093,68 107×32) | — |
| `JoinStartOn` | `{ label: 'Metronome', join: 'start', on: true }` | the joined lamp lit (Metronome on) | `JoinStartOn-{dark,light}.png` (Stage-Metronome 1092,68 108×32) | — |
| `JoinEnd` | `{ label: 'Metronome', join: 'end' }` | square left corners, rounded right | — (no board draws an end-joined lamp) | — |
| `Armed` | `{ label: 'Looper', size: 'cell', waiting: true, hue: 'rec', name: 'Looper, rec armed. Long press: loop rec', tip: 'looper.rec' }`, `layout: 'padded'` | Looper's Rec armed in the lamp row: no fill, 1px `--rec` outline, `--rec` 13px regular label | — (no board draws an armed LampButton, D10) | `data-face="waiting"`, `data-hue="rec"`, `aria-pressed="false"`; click → `ontoggle` called once with `true`, and the face is still `data-face="waiting"` (the parent, not the click, lights it, D17) |
| `ArmedLoop` | `{ label: 'Looper', size: 'cell', waiting: true, hue: 'lamp', name: 'Looper, loop armed', tip: 'looper.on_off' }`, `layout: 'padded'` | Loop armed: the outline and label in `--lamp-line` | — (D10) | `data-face="waiting"`, `data-hue="lamp"`. Expected to fail axe in light until `--lamp-line` lands (Contrast) |
| `LongPress` | `{ label: 'On', size: 'cell', on: true, name: 'Right 1 on. Long press: swap mode', tip: 'part.right1.on' }`, `layout: 'padded'` | a part lamp's long press (swap): the face doesn't change | — | `fireEvent.pointerDown(button, { pointerId: 1, button: 0, clientX: 0, clientY: 0 })` → `onlongpress` not called at once; `await waitFor(() => expect(onlongpress).toHaveBeenCalledTimes(1), { timeout: 1000 })` (real time); `fireEvent.pointerUp(button, { pointerId: 1, button: 0, clientX: 0, clientY: 0 })` → `onlongrelease` called once; `fireEvent.click(button)` → `ontoggle` not called and `aria-pressed` still `true`; then `userEvent.click(button)` → `ontoggle` called once with `false` (real timers: about 0.35 s per theme) |
| `RightClick` | `{ label: 'Looper', size: 'cell', name: 'Looper. Long press: loop rec', tip: 'looper.on_off' }`, `layout: 'padded'` | a right-click does the long press | — | `fireEvent.contextMenu(button)` (no `buttons`, so the button is up) → `onlongpress` then `onlongrelease` called once each; `ontoggle` not called; `aria-pressed` still `false` |
| `Focused` | `{ label: 'Metronome' }`, `pseudo: { focusVisible: true }` | the focus ring | — | — |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in the dark and light render; each crop is exactly the button's own box (the focus ring, outside it, is never in a crop; L6).

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- Each cropped story's screenshot matches its crop (`npm run shots -- LampButton`: score at most 0.02, or the Inspect agent judges any difference to be render noise), and axe finds no violation on any story, except `ArmedLoop` in light until the tokens contract PR (D7, D15) lands.
- The long press comes only from `use:longpress`; the component has no `setTimeout`.
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · A long press never toggles.** The click that ends a long press is swallowed by the action, so `ontoggle` isn't called; what the long press does (swap, Loop rec, Sound held) is the parent's, through `onlongpress` and `onlongrelease`.
- **D2 · `data-face` on the lamp.** The button carries `data-face="off|on|record|waiting"` (kit D41; `waiting` from D13) so page tests read the face without computed colours. While disabled it is `disabled` (D18 replaces this decision's earlier "keeps the face it would have").
- **D3 · `join` covers both ends.** Stage.md lists only `join: 'start'`; `end` is added for symmetry (a lamp to the right of a joined neighbour) and costs one radius rule; it has no board instance, so no crop.
- **D4 · The `Off` crop predates the caret.** `Off-{dark,light}.png` was cut from the #499 render, where Metronome stood alone at 1114,68; in today's render (#536) Metronome is joined at 1093,68, which `JoinStart` crops. The `Off` crop is kept unchanged (it is still a true picture of the unjoined off face); recutting it from Unison (Stage 1229,68 76×32) is a follow-up if the orchestrator wants every crop traceable to the current render.
- **D5 · Board story.** The first LampButton in reading order on the Stage board is Accomp, lit at the board fixture (`transport.acmp` true), so `Board` has `On`'s args and box.
- **D6 · The action is off without `onlongpress`.** Passing only `onlongrelease` does nothing; with no `onlongpress` the button behaves exactly as it did before long press existed.
- **D7 · L1, new tokens.** `--long-press` (350ms) and `--lamp-line` (dark `--lime-400`, light new `--lime-750: #477b0c`) and the two new contrast rows land in the orchestrator's tokens contract PR before the build; this spec never edits `tokens/*`.
- **D8 · L3, tooltip props.** LampButton takes `tip` (rendered as `data-tip`) and `tipAction` (applied as `use:tipAction={tip}` when both are set); stories pass `tipAction: fn()` in the meta and real keys (`transport.acmp`, `part.right1.on`, `looper.rec`, `looper.on_off`), all of which exist in `tooltips.ts` today.
- **D9 · L5, `aria-disabled`.** Rendered as `"true"` only when disabled and left off entirely when enabled (today's component renders `"false"`; the build changes that).
- **D10 · The armed face has no crop.** The Looper board is drawn `looping` and its armed outlines are in the state readout, not on a LampButton, and the Stage lamp row's Looper is off, so `Armed` and `ArmedLoop` are checked by `data-face`/`data-hue` and by eye, not by shots.
- **D11 · `width` wins.** A set `width` replaces the size's width and side padding (padding 0, label centred) and stops `cell` from filling its container; height and label size still follow `size`.
- **D12 · Width follows the weight.** An unfixed button is as wide as its label at the weight drawn, so lighting it widens it by the medium weight's extra width (Metronome 107 → 108px), as the boards draw it; parents needing a fixed width pass `width`.
- **D13 · The waiting face (lead's call).** `waiting` (default false) and `hue: 'lamp' | 'rec'` (default `lamp`) draw transparent fill, a 1px inset-shadow outline (so the box never changes size) and the label in the hue, with `data-face="waiting"` and `data-hue`; precedence on (or record) > waiting > off; used by LampRow for Looper's Rec armed (`rec`) and Loop armed (`lamp`).
- **D14 · Disabled code.** When disabled, the code turns `--d` at full opacity with the label, in every face.
- **D15 · L2, the lime on the ground.** `--lamp` on `--g` in light is 3.74:1 and no one lime passes both that and black ink on the lamp face, so the waiting lime is a new token `--lamp-line` (light `#477b0c`, 4.53:1; dark the same lime as `--lamp`, 13.24:1); `--rec` on `--g` passes in both themes (5.67:1, 5.27:1). `ArmedLoop` (light) fails axe until it lands.
- **D16 · L4, plays.** Pointer plays use `fireEvent.pointerDown/Up` with `{ pointerId: 1, button: 0, clientX: 0, clientY: 0 }` and real time (`waitFor` timeout 1000, or a 500 ms real wait); LampButton calls no pointer-capture API.
- **D17 · Controlled (review; Stage D49).** `aria-pressed` and the face follow `on` alone; a click, Space or Enter calls `ontoggle(!on)` and draws nothing new until the parent passes the next `on`, so a lamp lit from `sounding` or `surface.layer` never shows a state the engine refused. Today's local flip goes; `Toggles` and `Armed` check `ontoggle`'s payload and an unchanged face, which a local flip would fail.
- **D18 · `data-face="disabled"` and `data-contrast="dim"` (review).** Stage D49 lists `disabled` among the lamp's `data-face` values, as Stage.md › The base Button and kit D41 do for every faced control; Button D25 takes the same rule. A disabled lamp's `--d` label carries `data-contrast="dim"` (Stage.md › The base Button, D47).
- **D19 · `waiting` is a boolean plus `hue` (review).** Stage.md row 2 and D49 write `waiting: 'lamp' | 'rec'`; this spec keeps D13's `waiting: boolean` with `hue: 'lamp' | 'rec'` (the same two faces, and Button's waiting/hue pair has the same shape). Stage's `waiting="rec"` is `waiting hue="rec"` here.

Follow-ups: Stage.md row 2 and D49 could adopt D19's prop shape; kit.md › Dimmed text's carriers list needs a row for disabled controls (Button D26).

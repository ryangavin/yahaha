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
- **Not this component's job:** no store, no API, no Tauri. It doesn't know what it switches: the parent passes `on` and acts on `ontoggle`, `onlongpress` and `onlongrelease`. No tooltip (wired at integration). Long press comes only from the shared `use:longpress` action, never a timer of its own; Shift-click stays the parent's (it reads `ui.shift` and decides what a click means). Not the white "chosen" block, the outlined "waiting" face or the ▾ caret beside Metronome: those are other components (`Button`, `WaitingChip`).

## API (Component station)

### Props

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `label` | `string` | — | The word on the face. |
| `on` | `boolean` | `false` | Lit (lamp face) or off. The button follows it whenever the parent changes it; a click flips it locally until then. |
| `code` | `string \| undefined` | — | Small code after the label, e.g. `ACMP`. |
| `disabled` | `boolean` | `false` | Shown, not pressable (`aria-disabled`, stays focusable). No toggle and no long press. |
| `rec` | `boolean` | `false` | The record lamp: lit is the solid `--rec` face with `--solid-ink` label instead of the lamp face. |
| `size` | `'md' \| 'sm' \| 'cell'` | `'md'` | `md`: 32px tall, 14px label, 16px side padding (section row). `sm`: 28px, 13px, 14px (settings and strip rows). `cell`: 32px, 13px, no padding, fills its container's width (the band's lamp row). |
| `width` | `number \| undefined` | — | A fixed width in px with the label centred and no side padding (settings rows' 64px On/Off, strips' 72px). |
| `join` | `'start' \| 'end' \| undefined` | — | Joined to a neighbour with no gap between their faces: `start` rounds only the left corners (`border-radius: var(--radius) 0 0 var(--radius)`, i.e. `4px 0 0 4px`), `end` only the right (`0 var(--radius) var(--radius) 0`, i.e. `0 4px 4px 0`). Undefined: all four corners `--radius`. The Metronome lamp is `join: 'start'` beside its caret (the parent, `MetronomeSplit`, sets the 1px gap). |
| `name` | `string \| undefined` | — | The accessible name when the label alone isn't enough (`Right 1 on`). Default: the label, plus the code if any. |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `ontoggle` | click, Space or Enter, unless disabled; not for the click that ends a long press (it is swallowed) | `(on: boolean)` the new state |
| `onlongpress` | the button is held 350 ms (`--long-press`) without moving more than 4px, while still held; or right-clicked; unless disabled (longpress SPEC) | `()` |
| `onlongrelease` | the press that fired `onlongpress` ends (pointer up or cancelled; a right-click's release) | `()` |

**Long press wiring.** The `<button>` carries `use:longpress={{ onlongpress, onlongrelease, disabled: disabled || onlongpress === undefined }}`. With no `onlongpress` the action is off, so clicks behave exactly as before. A long press never toggles: the local pressed state doesn't change, `ontoggle` isn't called, and `aria-pressed` stays what it was until the parent changes `on` (D1).

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Visual rules

- **Tokens used:** `--btn`, `--m`, `--d`, `--lamp`, `--lamp-ink`, `--rec`, `--solid-ink`, `--focus`, `--radius`, `--font-sans`, `--text-12`, `--text-13`, `--text-14`, `--weight-regular`, `--weight-medium`, `--space-6`, `--space-14`, `--space-16`, `--control-height`, `--control-height-compact`, `--line-width`, `--focus-offset`, `--code-opacity`; and, through the action, `--long-press` (350ms, which the kit adds to `scale.css`).
- **Size:** height 32 (`md`, `cell`) or 28 (`sm`); width from the label plus side padding, or `width`, or the container (`cell`). Never wraps. `join` changes only the corner radii, never the size.
- **States drawn by:**
  - off: `--btn` face, `--m` label, regular weight; code `--m`. `data-face="off"`.
  - on: `--lamp` face, `--lamp-ink` label, medium weight; code in `--lamp-ink` at `--code-opacity`. `data-face="on"`.
  - on, `rec`: `--rec` face, `--solid-ink` label and code (the code at full ink: at `--code-opacity` it fails AA on `--rec`). `data-face="record"`.
  - disabled: the label turns `--d` (on or off face unchanged, `data-face` unchanged), default cursor, no press.
  - joined (`join`): the radii above; the face is otherwise unchanged in every state.
  - long press held: no change of its own (the parent may change `on` in answer).
  - keyboard focus: a `--line-width` outline in `--focus`, `--focus-offset` outside the face (the outline follows the joined radii).
  - No bar, border, glow or hover change (the kit draws none).
- **Test hook:** `data-face` on the `<button>`, from the pressed state as drawn: `off`, `on` or `record` (D2).
- **Type:** DM Sans, sentence case as given, tabular numerals; label 14px (`md`) or 13px (`sm`, `cell`); code 12px regular, `--space-6` after the label.
- **Contrast (AA 4.5:1, `tokens/contrast.test.ts`):** `--m` on `--btn`; `--lamp-ink` on `--lamp`, and at `--code-opacity`; `--solid-ink` on `--rec`. Disabled `--d` is exempt. (`join` adds no pair.)
- **Motion:** none.

### Accessibility

- **Role and name:** a `button` with `aria-pressed`; the accessible name is `name`, or the label plus the code. When the button has a long press, the parent says so in `name` or the tooltip ("Right 1 on. Long press: swap mode").
- **Keyboard:** Tab focuses it (also when disabled); Space or Enter toggles it. The long press from the keyboard is the platform's context-menu key (Menu or Shift+F10), via the action.
- **Tooltip id:** the parent's control id (e.g. `transport.acmp`, `part.right1.on`); wired at integration, not here.

## Stories (Story station)

Title `Primitives/LampButton`, `layout: 'centered'` unless the row says otherwise. Every story renders in dark and light (the toolbar theme). The meta's `args` are `{ ontoggle: fn(), onlongpress: fn(), onlongrelease: fn() }` (axiom 7), so every story has a long press; a short click still toggles.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ label: 'Accomp', code: 'ACMP', on: true }` | the first LampButton on the Stage board at the board fixture (`transport.acmp` true): lit, with the code | `Board-{dark,light}.png` (Stage 24,68 127×32; the same box as `On`) | the button named `Accomp ACMP` has `aria-pressed="true"` and `data-face="on"` |
| `Off` | `{ label: 'Metronome' }` | the off face: `--btn` fill, grey label | `Off-{dark,light}.png` (Stage 1114,68 107×32; see D4) | `data-face` is `off` |
| `On` | `{ label: 'Accomp', code: 'ACMP', on: true }` | lime face, ink label, the small code | `On-{dark,light}.png` (Stage 24,68 127×32) | — |
| `Toggles` | `{ label: 'Unison' }` | — | — | click → `aria-pressed` is `true` and `ontoggle` was last called with `true`; Space → `false`, called with `false`; Enter → `true`; called 3 times; `onlongpress` not called |
| `Disabled` | `{ label: 'Off', size: 'sm', width: 64, disabled: true, name: 'Manual Bass, works with Upper on' }` | dimmed label on the off face, 64 × 28 | `Disabled-{dark,light}.png` (SettingsChord 858,234 64×28) | `aria-disabled` is `true`; click → still not pressed and `ontoggle` not called; `fireEvent.pointerDown(button, { button: 0, pointerId: 1 })`, wait 500 ms → `onlongpress` not called |
| `Recording` | `{ label: 'Rec / Stop', rec: true, on: true }` | the solid record-red face | — | `data-face` is `record` |
| `PartOn` | `{ label: 'On', size: 'cell', on: true, name: 'Right 1 on' }`, `layout: 'padded'` | a part lamp filling its container, 13px label | — (the band's cells are fractional widths) | the button named `Right 1 on` is pressed |
| `LongLabel` | `{ label: 'Port sends mapped', size: 'sm', on: true }` | the longest real lamp label stays on one line | — | — |
| `JoinStart` | `{ label: 'Metronome', join: 'start' }` | the Metronome lamp off, square right corners where its caret joins | `JoinStart-{dark,light}.png` (Stage 1093,68 107×32) | — |
| `JoinStartOn` | `{ label: 'Metronome', join: 'start', on: true }` | the joined lamp lit (Metronome on) | `JoinStartOn-{dark,light}.png` (Stage-Metronome 1092,68 108×32) | — |
| `JoinEnd` | `{ label: 'Metronome', join: 'end' }` | square left corners, rounded right | — (no board draws an end-joined lamp) | — |
| `LongPress` | `{ label: 'On', size: 'cell', on: true, name: 'Right 1 on. Long press: swap mode' }`, `layout: 'padded'` | a part lamp's long press (swap): the face doesn't change | — | `fireEvent.pointerDown(button, { button: 0, pointerId: 1, clientX: 10, clientY: 10 })` → `onlongpress` not called at once; `await waitFor(() => expect(onlongpress).toHaveBeenCalledTimes(1), { timeout: 1000 })`; `fireEvent.pointerUp(button, { button: 0, pointerId: 1 })` → `onlongrelease` called once; `fireEvent.click(button)` → `ontoggle` not called and `aria-pressed` still `true`; then `userEvent.click(button)` → `ontoggle` called once with `false` (real timers: about 0.35 s per theme) |
| `RightClick` | `{ label: 'Looper', size: 'cell', name: 'Looper. Long press: loop rec' }`, `layout: 'padded'` | a right-click does the long press | — | `fireEvent.contextMenu(button)` → `onlongpress` then `onlongrelease` called once each; `ontoggle` not called; `aria-pressed` still `false` |
| `Focused` | `{ label: 'Metronome' }`, `pseudo: { focusVisible: true }` | the focus ring | — | — |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in the dark and light render. Every new control (`join`) is a select control with an empty option for undefined; the callbacks are actions.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- Each cropped story's screenshot matches its crop (`npm run shots -- LampButton`: score at most 0.02, or the Inspect agent judges any difference to be render noise).
- The long press comes only from `use:longpress`; the component has no `setTimeout`.
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · A long press never toggles.** The click that ends a long press is swallowed by the action, so the local pressed state and `ontoggle` are untouched; what the long press does (swap, Loop rec, Sound held) is the parent's, through `onlongpress` and `onlongrelease`.
- **D2 · `data-face` on the lamp.** The button carries `data-face="off|on|record"` (kit D41) so page tests read the face without computed colours; disabled is read from `aria-disabled`, and `data-face` keeps the face it would have (kit › Faces: "the face it would have").
- **D3 · `join` covers both ends.** Stage.md lists only `join: 'start'`; `end` is added for symmetry (a lamp to the right of a joined neighbour) and costs one radius rule; it has no board instance, so no crop.
- **D4 · The `Off` crop predates the caret.** `Off-{dark,light}.png` was cut from the #499 render, where Metronome stood alone at 1114,68; in today's render (#536) Metronome is joined at 1093,68, which `JoinStart` crops. The `Off` crop is kept unchanged (it is still a true picture of the unjoined off face); recutting it from Unison (Stage 1229,68 76×32) is a follow-up if the orchestrator wants every crop traceable to the current render.
- **D5 · Board story.** The first LampButton in reading order on the Stage board is Accomp, lit at the board fixture (`transport.acmp` true), so `Board` has `On`'s args and box.
- **D6 · The action is off without `onlongpress`.** Passing only `onlongrelease` does nothing; with no `onlongpress` the button behaves exactly as it did before long press existed.

# LampButton

## Identity (all stations)

- **Kind:** primitive
- **Built from:** —
- **Purpose:** Turns a part, a mode or a function on or off, and shows at a glance which it is.
- **Boards:**
  - `Stage-Dark.dc.html:92` (Accomp, lit, with the ACMP code), `:110-111` (Metronome, Unison, off), `:272-279` (the band's lamp row: part On/Off, Harm/Arp, Sound, L Hold, Looper); light: `Stage-Light.dc.html:68`, `:86-87`, `:248-255`.
  - `SettingsChord-Dark.dc.html:173` (Manual Bass, disabled, 64 × 28); light: `SettingsChord-Light.dc.html:156`.
  - `Looper-Dark.dc.html:170` (Rec / Stop, the record lamp); light: `Looper-Light.dc.html:149`.
  - The kit's lamp rules: `Stage-Dark.dc.html:61`, `Stage-Light.dc.html:37` (KIT-DEBATE.md, "Lamp button").
- **Not this component's job:** no store, no API, no Tauri. It doesn't know what it switches: the parent passes `on` and acts on `ontoggle`. No tooltip (wired at integration). No long-press or Shift behaviour (the parent's job). Not the white "chosen" block or the outlined "waiting" face: those are other components.

## API (Component station)

### Props

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `label` | `string` | — | The word on the face. |
| `on` | `boolean` | `false` | Lit (lamp face) or off. The button follows it whenever the parent changes it; a click flips it locally until then. |
| `code` | `string \| undefined` | — | Small code after the label, e.g. `ACMP`. |
| `disabled` | `boolean` | `false` | Shown, not pressable (`aria-disabled`, stays focusable). |
| `rec` | `boolean` | `false` | The record lamp: lit is the solid `--rec` face with `--solid-ink` label instead of the lamp face. |
| `size` | `'md' \| 'sm' \| 'cell'` | `'md'` | `md`: 32px tall, 14px label, 16px side padding (section row). `sm`: 28px, 13px, 14px (settings and strip rows). `cell`: 32px, 13px, no padding, fills its container's width (the band's lamp row). |
| `width` | `number \| undefined` | — | A fixed width in px with the label centred and no side padding (settings rows' 64px On/Off, strips' 72px). |
| `name` | `string \| undefined` | — | The accessible name when the label alone isn't enough (`Right 1 on`). Default: the label, plus the code if any. |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `ontoggle` | click, Space or Enter, unless disabled | `(on: boolean)` the new state |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Visual rules

- **Tokens used:** `--btn`, `--m`, `--d`, `--lamp`, `--lamp-ink`, `--rec`, `--solid-ink`, `--focus`, `--radius`, `--font-sans`, `--text-12`, `--text-13`, `--text-14`, `--weight-regular`, `--weight-medium`, `--space-6`, `--space-14`, `--space-16`, `--control-height`, `--control-height-compact`, `--line-width`, `--focus-offset`, `--code-opacity`.
- **Size:** height 32 (`md`, `cell`) or 28 (`sm`); width from the label plus side padding, or `width`, or the container (`cell`). Never wraps.
- **States drawn by:**
  - off: `--btn` face, `--m` label, regular weight; code `--m`.
  - on: `--lamp` face, `--lamp-ink` label, medium weight; code in `--lamp-ink` at `--code-opacity`.
  - on, `rec`: `--rec` face, `--solid-ink` label and code (the code at full ink: at `--code-opacity` it fails AA on `--rec`).
  - disabled: the label turns `--d` (on or off face unchanged), default cursor, no press.
  - keyboard focus: a `--line-width` outline in `--focus`, `--focus-offset` outside the face.
  - No bar, border, glow or hover change (the kit draws none).
- **Type:** DM Sans, sentence case as given, tabular numerals; label 14px (`md`) or 13px (`sm`, `cell`); code 12px regular, `--space-6` after the label.
- **Contrast (AA 4.5:1, `tokens/contrast.test.ts`):** `--m` on `--btn`; `--lamp-ink` on `--lamp`, and at `--code-opacity`; `--solid-ink` on `--rec`. Disabled `--d` is exempt.
- **Motion:** none.

### Accessibility

- **Role and name:** a `button` with `aria-pressed`; the accessible name is `name`, or the label plus the code.
- **Keyboard:** Tab focuses it (also when disabled); Space or Enter toggles it.
- **Tooltip id:** the parent's control id (e.g. `transport.acmp`, `part.on`); wired at integration, not here.

## Stories (Story station)

Title `Primitives/LampButton`, `layout: 'centered'` unless the row says otherwise. Every story renders in dark and light (the toolbar theme).

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Off` | `{ label: 'Metronome' }` | the off face: `--btn` fill, grey label | `Off-{dark,light}.png` (Stage 1114,68 107×32) | — |
| `On` | `{ label: 'Accomp', code: 'ACMP', on: true }` | lime face, ink label, the small code | `On-{dark,light}.png` (Stage 24,68 127×32) | — |
| `Toggles` | `{ label: 'Unison' }` | — | — | click → `aria-pressed` is `true` and `ontoggle` was last called with `true`; Space → `false`, called with `false`; Enter → `true`; called 3 times |
| `Disabled` | `{ label: 'Off', size: 'sm', width: 64, disabled: true, name: 'Manual Bass, works with Upper on' }` | dimmed label on the off face, 64 × 28 | `Disabled-{dark,light}.png` (SettingsChord 858,234 64×28) | `aria-disabled` is `true`; click → still not pressed and `ontoggle` not called |
| `Recording` | `{ label: 'Rec / Stop', rec: true, on: true }` | the solid record-red face | — | — |
| `PartOn` | `{ label: 'On', size: 'cell', on: true, name: 'Right 1 on' }`, `layout: 'padded'` | a part lamp filling its container, 13px label | — (the band's cells are fractional widths) | the button named `Right 1 on` is pressed |
| `LongLabel` | `{ label: 'Port sends mapped', size: 'sm', on: true }` | the longest real lamp label stays on one line | — | — |
| `Focused` | `{ label: 'Metronome' }`, `pseudo: { focusVisible: true }` | the focus ring | — | — |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in the dark and light render.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- Each cropped story's screenshot matches its crop (`npm run shots -- LampButton`: score at most 0.02, or the Inspect agent judges any difference to be render noise).
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules.
- svelte-check and lint pass on the folder.

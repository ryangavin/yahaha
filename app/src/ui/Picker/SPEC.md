# Picker

## Identity (all stations)

- **Kind:** primitive
- **Built from:** —
- **Purpose:** Picks one of more choices than fit side by side (an added send's effect type, the Master compressor's type), showing the current one with a ▾.
- **Boards:** none on the Effects board (Effects.md › Components 43: the added send isn't drawn open). `Effects-Master-Dark.dc.html` (#519) draws a Type ▾ ("Natural ▾" beside Compressor), in a different face (D2); its spec isn't written yet.
- **Not this component's job:** no store, no API, no Tauri. It doesn't know the list: the parent passes `options` in its order (including the current value when the list lacks it, Effects.md › Title row) and acts on `onchoose`. No visible label ("Type" is the parent's text before it), no popover list of its own (the system list, D1).

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `options` | `{ value: string, label: string }[]` | — | The choices, in the caller's order. |
| `value` | `string` | — | The chosen option's `value` (the state's). The `<select>` shows exactly this; a change doesn't move it (the parent does). |
| `label` | `string` | — | The `<select>`'s `aria-label` ("Phaser type"). Never drawn. |
| `tip` | `string \| undefined` | — | The tooltip key (`fx.send_kind`), rendered as `data-tip` on the `<select>` (L3). |
| `tipAction` | `Action<HTMLElement, string> \| undefined` | — | The app's `use:tip`, applied as `use:tipAction={tip}` on the `<select>` only when both are set (L3). Stories pass `fn()`. |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onchoose` | the `<select>`'s `change` event (a pick in the system list, or arrow keys on a closed select where the platform changes it) | `(value: string)` the picked option's `value` |

After the call the component sets the `<select>`'s value back to `value` (controlled, D3), so it shows the state until the parent passes the new value.

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Visual rules

- **Tokens used:** `--t`, `--g`, `--focus`, `--font-sans`, `--text-13`, `--weight-regular`, `--space-8`, `--space-10`, `--space-24`, `--line-width`, `--focus-offset`, `--tab-block` (24px, ChosenTabs' new token), and `--text-10` (10px, new, already proposed by another spec; L1).
- **The element:** a wrapper `span` (`position: relative`, `display: inline-flex`, `flex: none`) holding a native `<select>` and the caret.
- **`<select>`:** height `--tab-block` (24), `box-sizing: border-box`, padding `0 var(--space-24) 0 var(--space-10)`, `--text-13` / 400, DM Sans, `--t` fill, `--g` text, no border, radius 0, `appearance: none`, `data-face="chosen"`, cursor `pointer`, width the widest option (the native select's own sizing).
- **Caret** (`data-part="caret"`, `aria-hidden`): "▾", `--text-10`, `--g`, `position: absolute`, `right: var(--space-8)`, centred vertically, `pointer-events: none`.
- **Options:** the system list's own drawing (D1); no option styling.
- **States drawn by:**
  - default: the chosen block (the kit's chosen face at the 24px size).
  - keyboard focus (`:focus-visible`): a `--line-width` `--focus` outline, `--focus-offset` outside the select.
  - No hover, pressed, disabled (D4) or glow.
- **Type:** DM Sans 13 / 400, the label as given.
- **Contrast (AA 4.5:1, `tokens/contrast.test.ts`):** `--g` on `--t` ("chosen tab label on its block", the row ChosenTabs adds; 21 dark, 16.72 light).
- **Not checkable in jsdom:** the system list; the face (no crop, D2) is judged by the Inspect agent against Effects.md's text.
- **Motion:** none.

### Accessibility

- **Role and name:** a native `<select>` (`combobox`), named by `aria-label` = `label`; each option its `label`.
- **Keyboard:** the platform's (Tab to it; Space, Enter or Alt + ↓ opens the list; arrows move). The component handles no key itself.
- **Tooltip id:** `fx.send_kind` (the added send's type; exists); #519 gives the Master compressor's.

## Stories (Story station)

- **Title:** `Primitives/Picker`.
- **Layout:** `centered`. Meta: `tipAction: fn()`, `onchoose: fn()`.
- `KINDS` is `SEND_KINDS` (`app/src/panels/effects/sendKinds.ts`) mapped to `{ value: kind, label: name }`: Hall, Room, Stage, Plate, Chorus, Celeste, Flanger, Delay 1/8, Delay 1/8., Delay 1/4, Ping-Pong, Phaser.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ options: KINDS, value: 'phaser', label: 'Phaser type', tip: 'fx.send_kind' }` | the white block reading "Phaser" with the ▾ (the added send of Effects.md's Phaser) | — (no board draws it, D2) | the combobox named "Phaser type" has 12 options and value "phaser" |
| `Chooses` | `Board`'s args | — | — | change the select to "room" → `onchoose('room')` once; the select's value is "phaser" again (controlled) |
| `LongLabel` | `{ options: KINDS, value: 'dottedEighth', label: 'Delay 1/8. type', tip: 'fx.send_kind' }` | the longest kind name fits on one line with the caret clear of it | — | — |
| `Focused` | `Board`'s args; `parameters: { pseudo: { focusVisible: true } }` | the focus ring | — | — |

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- Picker` passes: no story is cropped, so the run checks axe only (colour contrast included) and finds no violation; the Inspect agent judges the face against Visual rules.
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · A styled native `<select>`.** As Effects.md says: keyboard, screen reader and the system list come free; a Push-skin popover list is Effects' follow-up, not this spec.
- **D2 · Effects' face, not the Master board's.** Effects.md › Picker draws the chosen block (24px `--t` with `--g` text and a ▾); the Effects-Master board draws "Natural ▾" as accent text on the ground. This spec follows Effects.md, the spec that owns the part; #519's spec decides whether the Master's type is this Picker or a text button, and if it is a Picker in the accent face, adds a `face` prop then. So there is no crop.
- **D3 · Controlled.** The select shows `value` and is reset to it after each `change`, so it always shows the state (as ChosenTabs D7); the parent's new `value` moves it.
- **D4 · No `disabled`.** No board or screen spec draws a disabled Picker; the prop is left out until one does (it would need the kit's focusable-disabled rule, which a native `<select disabled>` breaks).
- **D5 · Token reuse.** The 24px height is ChosenTabs' `--tab-block` (Harmony.md's `--chosen-block` is the same value; it names the Stage lane's token first), and the caret size the `--text-10` another spec already proposes.
- **D6 · No crop files.** There is no crop and no `crops/` folder (D2).

Follow-ups: the Picker as a popover list in the Push skin (Effects follow-up); #519 decides the Master type's face (D2).

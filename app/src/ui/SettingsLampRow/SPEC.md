# SettingsLampRow

## Identity (all stations)

- **Kind:** primitive (a row of LampButtons)
- **Built from:** LampButton
- **Purpose:** Shows one or more on/off settings as lamps in a settings column ("Chord note only"), each switched with one click.
- **Boards:** `Harmony-Dark.dc.html:186-188` ("Chord note only", off); light: `Harmony-Light.dc.html:176-178`.
- **Not this component's job:** no store, no API, no Tauri. It doesn't know what a lamp switches: the parent passes each lamp's `on` and acts on `ontoggle`. No label cell (a lamp names itself), no hairline list around it, and not the band's lamp row (`LampRow`, nine cells; Harmony.md › Components 7).

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `lamps` | `SettingsLamp[]` | — | The lamps, left to right. `SettingsLamp` is exported from `app/src/ui/SettingsLampRow/types.ts` (below). |
| `label` | `string \| undefined` | — | With two or more lamps, the row's `role="group"` `aria-label`; undefined: no group role (one lamp names itself). |
| `tipAction` | `Action<HTMLElement, string> \| undefined` | — | The app's `use:tip`, passed to every LampButton, which applies it with its own `tip` (L3). Stories pass `fn()`. |

`SettingsLamp`:

| Field | Type | Default | Meaning |
|---|---|---|---|
| `id` | `string` | — | What `ontoggle` carries. Unique in `lamps`. |
| `label` | `string` | — | The word on the lamp ("Chord note only"). |
| `on` | `boolean` | — | Lit or off, from the state. |
| `code` | `string \| undefined` | — | A small code after the label. |
| `name` | `string \| undefined` | — | The accessible name ("Chord note only, on"). Default: LampButton's (the label plus the code). |
| `width` | `number \| undefined` | — | A fixed width (the Settings pages' 64px On / Off). |
| `disabled` | `boolean \| undefined` | `false` | Shown, not switchable. |
| `tip` | `string \| undefined` | — | The tooltip key (`harmony.chord_note_only`). |

Passed to each LampButton: `size: 'sm'`, `label`, `on`, `code`, `name`, `width`, `disabled`, `tip`, `tipAction`, and `ontoggle: (on) => ontoggle(id, on)`.

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `ontoggle` | a lamp's `ontoggle` (click, Space or Enter, not disabled) | `(id: string, on: boolean)` the lamp's id and its new state |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Visual rules

- **Tokens used:** `--line`, `--line-width`, `--space-8`, and `--settings-row-sm` (36px, new, defined in `app/src/ui/BarRow/SPEC.md` › New tokens; L1). No token of its own.
- **The row:** a `div`, its parent's width, `box-sizing: border-box`, height `--settings-row-sm` (36), `border-top: var(--line-width) solid var(--line)`, a flex row, items centred, gap `--space-8`, left-aligned, no wrap, no padding, no label cell (the first lamp starts at the row's left edge).
- **Lamps:** LampButton `size sm` (28 tall, 13px; HA-D6: the board's 30px lamp is the kit's 28), `flex: none`, centred vertically. Their faces, focus ring and disabled look are LampButton's.
- **States drawn by:** only LampButton's. The row has no hover, focus or disabled look.
- **Contrast:** LampButton's rows (`--m` on `--btn`, `--lamp-ink` on `--lamp`); none of its own.
- **Motion:** none.

### Accessibility

- **Role and name:** one lamp: no role on the row; the LampButton is a `button` with `aria-pressed` and its `name`. Two or more with `label`: `role="group"` `aria-label={label}`.
- **Keyboard:** each lamp is a tab stop (LampButton's); Space or Enter switches it.
- **Tooltip id:** per lamp, `tip` (`harmony.chord_note_only`, exists).

## Stories (Story station)

- **Title:** `Primitives/SettingsLampRow`.
- **Layout:** `centered`, in a 371px-wide decorator (the Harmony settings column). Meta: `tipAction: fn()`, `ontoggle: fn()`.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ lamps: [{ id: 'chordNoteOnly', label: 'Chord note only', on: false, name: 'Chord note only, off', tip: 'harmony.chord_note_only' }] }` | the hairline and one off lamp at the left | `Board-{dark,light}.png`: Harmony 1021,336 371×36 | the button named "Chord note only, off" has `aria-pressed="false"`; click → `ontoggle('chordNoteOnly', true)` |
| `On` | `Board`'s lamp with `on: true, name: 'Chord note only, on'` | the lit lamp | — (the board draws it off) | `aria-pressed="true"` |
| `Several` | `{ label: 'Arpeggio switches', lamps: [{ id: 'hold', label: 'Hold', on: true, tip: 'harmony.arp_hold' }, { id: 'keepKeyOn', label: 'Keep Key On', on: false, tip: 'harmony.arp_keep_key_on' }] }` | two lamps, gap 8, as the arpeggio kind (#527) will draw them | — (#527's board) | the row has `role="group"` named "Arpeggio switches"; click Keep Key On → `ontoggle('keepKeyOn', true)` |
| `Focused` | `Board`'s args; `parameters: { pseudo: { focusVisible: true } }` | the lamp's focus ring | — | — |

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- SettingsLampRow` passes: the cropped story's screenshot is the crop's size and scores at most 0.02, and axe finds no violation on any story.
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · Kept apart from ChoiceRow and BarRow.** All three are settings rows with a top hairline, but this one has no label cell and its content is lamps, so folding it into a generic row would only move the difference into a prop; it stays its own small component, sharing the row tokens.
- **D2 · `ontoggle` carries the id.** One callback for the row with `(id, on)` lets the settings column wire a row of several lamps without a callback per item.
- **D3 · Group only for several.** One lamp already names itself, so a group around it would be noise for a screen reader; a row of two or more takes `label` and becomes a `role="group"`.
- **D4 · Controlled through LampButton.** The row passes `on` from the state; whether LampButton also flips locally until the echo is LampButton's own rule (Harmony.md › Gap asks for it to become fully controlled), not this row's.
- **D5 · `Several` uses #527's names.** No board draws two lamps in a settings row yet; the story uses the arpeggio switches Harmony.md lists for #527 (Hold, Keep Key On) with their existing tooltip keys, and has no crop.
- **D6 · No crop files yet.** The `crops/` folder doesn't exist; the box above is for the crop station.

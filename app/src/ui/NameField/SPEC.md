# NameField

## Identity (all stations)

- **Kind:** primitive
- **Built from:** —
- **Purpose:** Lets the player type a name (a rack's, a new sound's) in a prompt, starting from a suggestion that typing replaces.
- **Boards:**
  - `Prompts-Dark.dc.html:420-426` (Store: "Rack name", "Rhodes Soft + Strings" with the drawn caret), `:449-455` (Sound names: the R1 tag, "New sound name", "Rhodes Soft 2"); light: `Prompts-Light.dc.html:412-418`, `:441-447`.
  - Unsaved changes (never saved) and Rename use it too; no board draws them (Prompts.md › Unsaved changes, Rename).
- **Not this component's job:** no store, no API, no Tauri. It doesn't decide the suggestion, validate a name against other racks or send anything: the prompt owns `value` (bound), derives its buttons' disabled state and `aria-label`s from it, and runs its primary action from `onenter` when that is enabled. It doesn't take focus on mount (the overlay does, Prompts.md PR-D5) and doesn't handle Esc or Tab (the overlay's trap). Not a search box, not the inline rename in `RacksTab.svelte`.

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `id` | `string` | — | The input's id, and the label's `for` (`{titleId}-field-{n}`: `dlg-store-field-0`). |
| `label` | `string` | — | The label's words, sentence case ("Rack name", "New sound name"). |
| `tag` | `{ text: string; hue: 'r1' \| 'r2' \| 'r3' \| 'l' } \| undefined` | — | The part tag before the label (Sound names): `{ text: 'R1', hue: 'r1' }`. Undefined: no tag. |
| `value` | `string` | `''` | The text, `$bindable` (`bind:value` on the input). The prompt sets its initial value (its suggestion, or the story's typed value) and reads it back. |
| `ariaLabel` | `string` | — | The input's `aria-label` ("Rack name", "New rack name", "Right 1 new sound name"). |
| `tip` | `string \| undefined` | — | The tooltip key (`quick.save_name`, `rack.save_as_name`, `rack.sound_name`, `library.rack_rename_name`), rendered as `data-tip` on the `<input>`; no attribute when undefined (L3). |
| `tipAction` | `Action<HTMLElement, string> \| undefined` | — | The app's `use:tip`, passed in by the wiring. When both it and `tip` are set the `<input>` gets `use:tipAction={tip}`; otherwise nothing (L3). |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onenter` | Enter is pressed in the input (`keydown`, `key === 'Enter'`, not while composing: `event.isComposing` false); the field calls `preventDefault()` first. It fires whatever the value: the prompt decides whether its primary action is enabled. | `()` |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Visual rules

- **Tokens used:** `--t`, `--m`, `--r1`, `--r2`, `--r3`, `--l`, `--font-sans`, `--text-12`, `--text-18`, `--weight-light`, `--weight-regular`, `--weight-medium`, `--space-4`, `--space-8`, `--space-16`, `--space-24`, `--control-height`, `--line-width`. No new token: every size is one of these (the input's bottom padding is `calc(var(--space-4) - var(--line-width))`, 3px). Light `--r3` and `--l` change value (Contrast).
- **Structure:** a `<div>`, `display: flex; flex-direction: column; gap: var(--space-4)`, `width: 100%`, `min-width: 0` (it fills its container: the Dialog's 390px content width). In it, the label then the input. Box height 52 (16 + 4 + 32).
- **Label:** `<label for={id}>`, `--text-12` / `--weight-regular`, line-height `--space-16`, `--m`. With `tag`: the label is a row, `display: flex; align-items: center; gap: var(--space-8)`, height `--space-16`; first a `<span data-hue={tag.hue}>` with `tag.text` at `--text-12` / `--weight-medium`, line-height `--space-16`, colour `var(--r1)` / `--r2` / `--r3` / `--l`; then the words in a `<span>`.
- **Input:** `<input type="text">`, `height: var(--control-height)` (32), `box-sizing: border-box`, `padding: var(--space-4) 0 calc(var(--space-4) - var(--line-width))` (the 24px text line, 4 above, 3 below, then the 1px underline inside the box), `width: 100%`, `margin: 0`, `border: 0; border-bottom: var(--line-width) solid var(--t)`, `border-radius: 0`, `background: none`, `--text-18` / `--weight-light`, line-height `--space-24`, `color: var(--t)`, `caret-color: var(--t)`, `outline: none`, `font-family: var(--font-sans)`, `cursor: text`. Attributes `spellcheck="false"`, `autocomplete="off"`, `maxlength="80"`. A value wider than the field scrolls inside the input (native), never wraps.
- **Selection on focus:** `onfocus` calls `input.select()`, so on every focus the whole value is selected and typing replaces it (Prompts.md › Name field › Value; D3).
- **States drawn by:**
  - filled: the value in `--t` 18/300 over the `--t` underline.
  - empty (trimmed value `''`): nothing but the underline (no placeholder); `aria-invalid="true"` (D2).
  - focused: the caret (`--t`) and the native selection; no ring, no change of the underline (Prompts.md PR-D6, D4). The board's drawn caret (a 1 × 22 bar after the text) is this real caret; unfocused stories have none, 22 pixels under the threshold.
  - tagged: the part tag before the label in its hue, `data-hue` on it.
  - No hover or pressed look; no disabled state (the prompt disables its buttons, not the field).
- **Type:** DM Sans (`--font-sans`); label 12/400 (tag 12/500), input 18/300; sentence case as given; the input has `font-variant-numeric: tabular-nums`.
- **Contrast (AA 4.5:1, `tokens/contrast.test.ts`):** `--m` on `--g` (label) and `--t` on `--g` (value) exist. New rows for the part tag (land with the tokens contract PR, L1): `--r1` on `--g` "part tag (NameField)" 6.50:1 dark, 5.53:1 light; `--r2` on `--g` 6.86:1, 4.60:1; `--r3` on `--g` 9.91:1 dark, **3.65:1 light, fails**; `--l` on `--g` 9.75:1 dark, **3.58:1 light, fails**. L2: the two light failures take the changed values AccentBlock's spec already proposes (`app/src/ui/AccentBlock/SPEC.md`: light `--r3` `#c85f00` → `#b05300`, 4.55:1 on `--g`; light `--l` `#008f78` → `#007c68`, 4.55:1), listed here as changed tokens too (D6). The underline is `--t`, a non-text edge (21:1 / 16.72:1).

#### Changed tokens

| Token | Dark | Light | Why |
|---|---|---|---|
| `--r3` (changed) | unchanged | `#c85f00` → `#b05300` (the `--orange-700` palette step; light `--warn` moves with it) | the R3 tag on `--g`: 3.65 → 4.55 (AccentBlock's value, D6) |
| `--l` (changed) | unchanged | `#008f78` → `#007c68` (the `--teal-700` step) | the L tag on `--g`: 3.58 → 4.55 (AccentBlock's value, D6) |

- **Motion:** none (the caret's blink is the browser's).

### Accessibility

- **Role and name:** a `textbox` (native `<input type="text">`) named by `aria-label={ariaLabel}`; the visible `<label for>` is also associated (clicking it focuses the input). `aria-invalid="true"` while `value.trim()` is empty; no `aria-invalid` attribute otherwise (as L5). `data-tip` only with `tip`.
- **Keyboard:** Tab focuses it (selecting the text); typing edits; Enter calls `onenter` (D5). Esc and Tab-wrapping are the overlay's.
- **Tooltip id:** the prompt's key passed as `tip`: `quick.save_name` (Store), `rack.save_as_name` (Unsaved, never saved), `rack.sound_name` (Sound names), `library.rack_rename_name` (Rename; new, a contract change, Prompts.md C1), with `tipAction` (L3).

## Stories (Story station)

- **Title:** `Primitives/NameField` (D7).
- **Layout:** `centered`, the field inside a 390px-wide wrapper (a story decorator: the Dialog's content width), so the field and its crop are 390 wide.

Every story renders in dark and light. The meta's `args` are `{ onenter: fn(), tipAction: fn() }` (axiom 7; L3).

**Controls (argTypes):** `id`, `label`, `value`, `ariaLabel`, `tip` text; `tag` an object control (empty for undefined); `onenter`, `tipAction` actions.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ id: 'dlg-store-field-0', label: 'Rack name', value: 'Rhodes Soft + Strings', ariaLabel: 'Rack name', tip: 'quick.save_name' }` | Store's field, filled, unfocused | `Board-{dark,light}.png` (Prompts 293,303 390×52) | the textbox named "Rack name" has value "Rhodes Soft + Strings", id `dlg-store-field-0`, `data-tip="quick.save_name"`, `spellcheck="false"`, `autocomplete="off"`, `maxlength="80"` and no `aria-invalid`; the `label` element's `for` is `dlg-store-field-0`; `tipAction` was called with the input and `'quick.save_name'` |
| `Tagged` | `{ id: 'dlg-names-field-0', label: 'New sound name', tag: { text: 'R1', hue: 'r1' }, value: 'Rhodes Soft 2', ariaLabel: 'Right 1 new sound name', tip: 'rack.sound_name' }` | Sound names' field: the R1 tag in `--r1`, then the label | `Tagged-{dark,light}.png` (Prompts 293,594 390×52) | the label contains an element with text "R1" and `data-hue="r1"`; the textbox is named "Right 1 new sound name" |
| `TagLeft` | `{ id: 'dlg-names-field-1', label: 'New sound name', tag: { text: 'L', hue: 'l' }, value: 'Warm Pad', ariaLabel: 'Left new sound name', tip: 'rack.sound_name' }` | the Left tag in `--l` | — (the board draws only R1) | the tag has `data-hue="l"` |
| `Empty` | `{ id: 'dlg-store-field-0', label: 'Rack name', value: '', ariaLabel: 'Rack name', tip: 'quick.save_name' }` | the underline alone, the invalid state | — (not drawn) | the textbox has `aria-invalid="true"`; type "Ballad" → no `aria-invalid` attribute; clear it and type three spaces → `aria-invalid="true"` again |
| `Enter` | `Board` args | — | — | focus the textbox → `selectionStart` 0 and `selectionEnd` 21 (the whole value selected); `userEvent.keyboard('Organ')` → value "Organ" (the selection replaced); press Enter → `onenter` called once with no arguments; the value is still "Organ" |
| `LongValue` | `Board` args with `value` an 80-character name (`'Sunday drive with the strings up and the pads all the way down for the last chorus'`, cut to 80) | the value scrolls inside the input, the field stays 390 × 52 | — (no board draws it) | type one more character → the value's length stays 80 |
| `Focused` | `Board` args; `parameters: { pseudo: { focusVisible: true } }` | a focused field draws no ring (Prompts.md PR-D6); the caret and selection need real focus, which `shots.ts` drops, so Inspect sees them with this story's play focusing the input | — | focus the textbox (so Inspect sees the caret and selection in Storybook) |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in dark and light; each crop is exactly the field's own box, label to underline (L6). The boards draw a caret (1 × 22) after each value; the unfocused stories have none, 22 pixels of 20,280, inside the threshold.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- NameField` passes: `Board` and `Tagged` match their crops (score at most 0.02, or the Inspect agent judges the difference render noise), and axe finds no violation on any story once the changed light `--l` lands (`TagLeft` in light fails colour contrast until then, D6).
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · Value is bound, the prompt owns it.** `value` is `$bindable`, as Prompts.md › Props of the primitives gives; the field holds no copy, so a prompt's disabled primary and its `aria-label`s follow every keystroke, and a re-render after a refusal keeps what was typed.
- **D2 · The field marks itself invalid.** `aria-invalid="true"` follows the trimmed value inside the field (Prompts.md › Name field › Input), so every prompt gets it for free; absent, not `"false"`, when valid (the L5 pattern). Rename's "unchanged" is not invalid: only its button is disabled.
- **D3 · Select on every focus.** `onfocus` selects the whole value (Tab, the overlay's initial focus, a click); on a mouse click the browser may then place the caret where the pointer let go, which is the platform's choice and is left alone (no `mouseup` override).
- **D4 · No focus ring.** The underline is `--t` already, so a focused field shows its caret and selection and no `--focus` outline (Prompts.md PR-D6, a kit exception, PR-D24). `outline: none` on the input is that exception, not an oversight.
- **D5 · Enter is the field's.** The field calls `onenter` on Enter (not while an IME composes) and prevents the default, so a prompt's test can press Enter with no overlay; whether anything is sent is the prompt's call (disabled primary, one send per answer, Prompts.md PR-D26).
- **D6 · L2, part-tag contrast.** The R3 and L tags at 12px on the light ground are 3.65:1 and 3.58:1. The fix is the one AccentBlock's spec already proposes (light `--r3` `#b05300`, `--l` `#007c68`, both 4.55:1), not a new tag token, so the part hues stay one set; the tokens contract PR lands them. Until it does, `TagLeft` in light fails axe.
- **D7 · Primitives title.** Prompts.md › Checks names `Components/NameField`; the lane convention files primitives under `Primitives/`, so the story title is `Primitives/NameField` and the export names (`Empty`, `Focused`) are the ones Prompts.md names.
- **D8 · No new tokens.** Every size is an existing token (`--control-height` 32, `--space-16` and `--space-24` as line heights, the 3px bottom padding as `--space-4` less `--line-width`); `maxlength` 80 is an attribute, not a style.
- **D9 · No crop file exists yet.** `app/src/ui/NameField/crops/` is cut later by the crop station at the boxes in the story table (measured on the render: Store's field underline at y 354, Sound names' at 645, both 293–682).
- **D10 · `tip` is a string.** `app/src/ui` may not import the app's types (lint), so `tip` is `string`, as Button's; the wiring passes a `TipKey`.

Follow-ups: `library.rack_rename_name` (Rename's field tooltip) is contract change C1 in Prompts.md; until it lands Rename's field uses no tip.

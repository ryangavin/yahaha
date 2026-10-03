# MetronomeSplit

## Identity (all stations)

- **Kind:** complex
- **Built from:** LampButton, Button
- **Purpose:** Turns the metronome on or off, and opens its settings from the small ▾ beside it.
- **Boards:**
  - `Stage-Dark.dc.html:115` (off, caret closed); light: `Stage-Light.dc.html:91`.
  - `Stage-Metronome-Dark.dc.html:123` (on, caret open, the popover below it); light: `Stage-Metronome-Light.dc.html:94`.
- **Not this component's job:** no store, no API, no Tauri: it takes `on` and the caret's state as props and calls back. It doesn't draw the metronome popover (spec #509; the popover's spec says where it renders and may add a snippet here), and it doesn't decide when the caret is enabled (the parent passes `settingsDisabled`, D2). No tooltip wiring (integration). No Launchkey mapping (there is none). No keyboard shortcut of its own (`.` toggles the metronome through the app's global key handler, not here). The faces, glyph sizes and focus rings are the children's: this spec gives only the group's layout, its props and how they map onto the children.

## API (Component station)

### Props

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `on` | `boolean` | `false` | The metronome is on: the lamp is lit. The wiring feeds it from `metronome.on`. |
| `settingsDisabled` | `boolean` | `true` | The caret is shown but not pressable. `true` until the metronome popover (#509) is built (Stage.md D32); the wiring passes `false` once it exists. |
| `settingsOpen` | `boolean` | `false` | The popover is open: the caret's `aria-expanded` and its open look. Ignored while `settingsDisabled` (D6). The wiring owns it (the popover's open state, app-side). |
| `popoverId` | `string \| undefined` | — | The popover element's id; when given, the caret carries `aria-controls` with it (D5). |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `ontoggle` | the lamp is clicked, or Space or Enter on it (LampButton's `ontoggle`) | `(on: boolean)` the new state. The wiring sends `toggleMetronome` (no payload; app-api.md › Metronome). |
| `onopensettings` | the caret is clicked, or Space or Enter on it, unless `settingsDisabled` (Button's `onpress`) | `()` nothing. Fires whether or not `settingsOpen` is true; the wiring closes an open popover on it (D3). |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Children

| Child | Order | Props passed (from this component's props) | Callback → |
|---|---|---|---|
| `LampButton` | 1st | `{ label: 'Metronome', on, size: 'md', join: 'start' }` | `ontoggle(on)` → this `ontoggle(on)` |
| `Button` (the caret) | 2nd | `{ label: '▾', variant: 'caret', name: 'Metronome settings', haspopup: 'dialog', expanded: settingsOpen && !settingsDisabled, controls: popoverId, disabled: settingsDisabled }` | `onpress()` → this `onopensettings()` |

- `join: 'start'` is LampButton's (the primitives lane adds it): the lamp's right corners are square (radius `4px 0 0 4px`) so lamp and caret read as one split button.
- `variant: 'caret'` is Button's 20 × 32 caret face (the primitives lane's Button spec, its `Board` and `CaretExpanded` stories): the ▾ glyph at 10px, radius `0 4px 4px 0`, glyph `--m` closed, `--t` when `expanded`, `--d` when `disabled`. `haspopup`, `expanded` and `controls` set the button's `aria-haspopup`, `aria-expanded` and `aria-controls`. These prop names are assumed from the kit; where the Button spec names them differently, its names win and this table follows (D7).

### Visual rules

- **Layout:** the root is a `span`, `display: inline-flex`, `align-items: center`, `gap: var(--line-width)` (1px: the ground shows through between lamp and caret), no background, padding or border, `white-space: nowrap`. Lamp, then caret, both 32 tall. On the board the group measures 128 × 32 off (lamp 107, gap 1, caret 20) and 129 × 32 on (the lit label's medium weight is about 1px wider); the lamp's width is LampButton's (its label plus 16px each side), never set here.
- **Tokens used:** `--line-width` (the gap). Nothing else: every colour, size and radius belongs to LampButton or Button.
- **States drawn by:** the children: lamp off or lit (LampButton `on`); caret closed, open or disabled (Button `variant: 'caret'` with `expanded` / `disabled`); keyboard focus on either (their own `--focus` rings; the root draws none and doesn't clip them: no `overflow`).
- **Type:** the children's.
- **Contrast:** none of its own (no text of its own). The pairs it relies on are LampButton's (`--m` on `--btn`, `--lamp-ink` on `--lamp`) and Button's caret (`--m` on `--btn`, `--t` on `--btn`); the Button spec lists them for `tokens/contrast.test.ts`.
- **Motion:** none.

### Accessibility

- **Role and name:** the root is `role="group"` with `aria-label="Metronome"`. Inside it: the lamp, a `button` with `aria-pressed` and the name "Metronome" (LampButton); the caret, a `button` named "Metronome settings" (D4) with `aria-haspopup="dialog"`, `aria-expanded` (`"true"` only when `settingsOpen` and not `settingsDisabled`), `aria-controls` = `popoverId` when given, and `aria-disabled="true"` while `settingsDisabled` (it stays focusable, like every disabled control in the kit).
- **Keyboard:** Tab reaches the lamp, then the caret (also when disabled). Space or Enter on the lamp toggles it; on the caret, it calls `onopensettings()` (nothing when disabled). No arrow keys.
- **Tooltip id:** lamp `metronome.on` (exists); caret `metronome.settings` (new: Stage.md contract change C5). Both elements carry a `data-tip` once wired at integration, not here.

## Stories (Story station)

Title `Components/MetronomeSplit`, `layout: 'centered'` (real size). Every story renders in dark and light (the toolbar theme). Controls: `on`, `settingsDisabled`, `settingsOpen`, `popoverId`; the callbacks are actions (`ontoggle`, `onopensettings`). The child props this component sets are all derived from its own props or fixed, so there is no separate child control group.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{}` (every default: off, caret disabled) | the Stage board's group at the fixture moment (`metronome.on` false; #509 not built): off lamp, disabled caret (glyph `--d`) | `Board-{dark,light}.png` (Stage 1093,68 128×32). The board draws the caret enabled (`--m`), so only the glyph's few pixels differ, far under 0.02 (D2) | group named "Metronome"; the button "Metronome" has `aria-pressed="false"`; the button "Metronome settings" has `aria-disabled="true"`, `aria-haspopup="dialog"`, `aria-expanded="false"` and no `aria-controls`; click it → `onopensettings` not called; focus it and press Enter → still not called |
| `SettingsEnabled` | `{ settingsDisabled: false }` | the caret once #509 exists: glyph `--m`, exactly the board's drawing | `SettingsEnabled-{dark,light}.png` (Stage 1093,68 128×32) | "Metronome settings" has `aria-disabled="false"`; click → `onopensettings` called once with no arguments; Space → twice; Enter → three times; `ontoggle` never called |
| `On` | `{ on: true }` | the lit lamp beside the disabled caret | — (the boards' only lit instance has the popover open: `SettingsOpen`) | "Metronome" has `aria-pressed="true"` |
| `SettingsOpen` | `{ on: true, settingsDisabled: false, settingsOpen: true, popoverId: 'metro-pop' }` | lit lamp, open caret (glyph `--t`) | `SettingsOpen-{dark,light}.png` (Stage-Metronome 1092,68 129×32) | "Metronome settings" has `aria-expanded="true"` and `aria-controls="metro-pop"`; click → `onopensettings` called once (D3) |
| `OpenButDisabled` | `{ settingsOpen: true }` | disabled wins: glyph `--d`, not expanded | — (no board draws it) | "Metronome settings" has `aria-expanded="false"` and `aria-disabled="true"` (D6) |
| `Toggles` | `{}` | — | — | click "Metronome" → `aria-pressed="true"`, `ontoggle` last called with `true`; Space → `"false"`, called with `false`; Enter → `"true"`, called with `true`; `ontoggle` called 3 times, `onopensettings` never |
| `Focused` | `{ settingsDisabled: false }`, `parameters: { pseudo: { focusVisible: true } }` | both focus rings, unclipped by the group | — | — |

Crop positions are `board x,y w×h` in the 1440×900 renders (`docs/design/push/png/<Board>-Dark.png` and `-Light.png`), the same box in dark and light.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- MetronomeSplit` passes: each cropped story's screenshot is the crop's size and scores at most 0.02 (or the Inspect agent judges the difference render noise), and axe (colour contrast included) finds no violation on any story.
- Only `--line-width` is used directly; no inline colours, no literal sizes.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · The caret is a `Button`.** The primitives lane's Button draws the 20 × 32 caret (`variant: 'caret'`, its `Board` story), so this component composes it rather than drawing its own; Stage.md › Components row 26's "Built from" (LampButton, Button) stands.
- **D2 · Caret disabled by default.** `settingsDisabled` defaults to `true` until #509 (Stage.md D32), so `Board` draws the shipping interim (glyph `--d`) where the board draws it enabled; `SettingsEnabled` matches the board exactly in the same box.
- **D3 · One callback for the caret.** A caret press always calls `onopensettings()`; closing the popover (Escape, a click outside, a second caret press while `settingsOpen`) is the popover's and the wiring's job (#509).
- **D4 · Caret name.** "Metronome settings", short; the board's longer "on/off, volume, bell on beat 1" belongs in the `metronome.settings` tooltip body.
- **D5 · `aria-controls`.** Set only from `popoverId`, since the popover renders elsewhere and doesn't exist until #509.
- **D6 · Disabled beats open.** With `settingsDisabled`, `expanded` is passed `false` whatever `settingsOpen` says.
- **D7 · Button prop names.** `variant: 'caret'`, `haspopup`, `expanded`, `controls` and `onpress` are assumed from the kit and the primitives lane's Button crops; the Button spec's names win where they differ.

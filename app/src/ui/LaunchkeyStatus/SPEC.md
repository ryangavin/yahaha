# LaunchkeyStatus

## Identity (all stations)

- **Kind:** complex
- **Built from:** StatusDot
- **Purpose:** Tells you at a glance whether the Launchkey is connected, so you know its pads and buttons drive the arranger.
- **Boards:** `Stage-Dark.dc.html:89` (connected: the green dot and "Launchkey"); light: `Stage-Light.dc.html:65`. The same instance on every page board (e.g. `SettingsSystem-Dark.dc.html:78`, light `SettingsSystem-Light.dc.html:67`). No board draws it not connected.
- **Not this component's job:** no store, no API, no Tauri, no import from `app/src/lib`. It doesn't know about the Launchkey: the parent passes `connected`. It isn't a control (nothing to click, not focusable). It doesn't place itself: the separator before it and the 8px after that are AppBar's. The dot's drawing (6 × 6, fill, glow, ring) is StatusDot's and isn't restated here.

## API (Component station)

### Props

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `connected` | `boolean` | `false` | A Launchkey DAW port is connected. True: the solid green dot and the muted label. False: the hollow dot and the dimmed label. |
| `tip` | `Attachment<HTMLElement> \| undefined` (`svelte/attachments`) | — | The tooltip attachment, applied to the root span (as HealthSlot's `tip`). The parent passes `fromAction(tipAction, () => 'launchkey.status')`. Stories pass `fn()`. |

### From the state

| Prop | Fed from |
|---|---|
| `connected` | `app.state.pads.connected` (docs/app-api.md › pads: set once, at start) |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| — | | |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Children

| Order | Child | Props passed |
|---|---|---|
| 1 | `StatusDot` | connected: `{ hue: 'ok' }` (solid `--ok` with its glow; `glow` and `size` left at their defaults, `true` and `'md'`). Not connected: `{ hue: 'd', hollow: true }` (a 1px `--d` ring, no glow). No `name`: the dot is decorative, the root's `aria-label` says what it means. |
| 2 | label | LaunchkeyStatus' own `span`, text "Launchkey" (Visual rules) |

### Visual rules

- **Tokens used:** `--m`, `--d`, `--font-sans`, `--text-14`, `--weight-regular`, `--space-8`.
- **Root:** a `span`, `display: inline-flex; align-items: center; gap: var(--space-8); white-space: nowrap`, no padding, no background; cursor `default`.
- **Size:** the content: the 6px dot, the 8px gap, the label. Height is the label's line box (`line-height: normal`, about 18px at 14px). About 82.7px wide at the board's font: the width follows the text, so it is never a whole number of pixels (D2). Never wraps or shrinks.
- **Label:** "Launchkey" in both states, DM Sans 14 / 400 (`--font-sans`, `--text-14`, `--weight-regular`), sentence case, tabular numerals.
- **States drawn by:**
  - connected: label `--m`, `data-hue="m"` on the label; StatusDot `{ hue: 'ok' }`.
  - not connected: label `--d`, `data-hue="d"` on the label; StatusDot `{ hue: 'd', hollow: true }`.
  - No focus, hover or pressed look (it isn't a control; kit: Hover, press and cursor).
- **Test hooks:** the root carries `data-connected="true" | "false"` and `data-tip="launchkey.status"`; the label carries `data-hue`; the dot carries StatusDot's `data-face` (`solid` | `hollow`) and `data-hue` (`ok` | `d`).
- **Contrast (AA 4.5:1, `tokens/contrast.test.ts`):** `--m` on `--g` (exists). The not-connected label, `--d` on `--g`, is below AA on purpose (2.5:1 dark, 1.9:1 light; D3).
- **Motion:** none: connecting and disconnecting switch at once.

### Accessibility

- **Role and name:** the root `span` has `role="status"` and `aria-label` "Launchkey connected" (`connected` true) or "Launchkey not connected" (false). The dot is `aria-hidden` (StatusDot's, without `name`); the visible "Launchkey" is covered by the `aria-label`.
- **Keyboard:** not focusable, not in the Tab order (kit › Interaction conventions).
- **Tooltip id:** `launchkey.status` (exists in `app/src/help/tooltips.ts`): the static `data-tip="launchkey.status"` on the root, and the `tip` attachment there when given. It shows on hover only, since the element isn't focusable.
- **Launchkey mapping:** none (it reports the Launchkey; no Launchkey control changes it).

## Stories (Story station)

Title `Components/LaunchkeyStatus`, `layout: 'centered'`. Every story renders in dark and light (the toolbar theme). Every story passes `tip: fn()`. Controls: `connected` (boolean). It has no callbacks and isn't focusable, so there is no `Focused` story.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ connected: true }` | the green dot (glowing in dark, flat in light) and "Launchkey" in `--m` | — (D2) | the `status` named "Launchkey connected" has `data-tip="launchkey.status"`, `data-connected="true"` and the text "Launchkey"; the label has `data-hue="m"`; the dot has `data-face="solid"` and `data-hue="ok"`; `tip` was called once; nothing in the story is focusable |
| `NotConnected` | `{ connected: false }`, `parameters: { a11y: { config: { rules: [{ id: 'color-contrast', enabled: false }] } } }` | the hollow `--d` ring and "Launchkey" in `--d` | — (no board draws it) | the `status` named "Launchkey not connected" has `data-connected="false"` and the text "Launchkey"; the label has `data-hue="d"`; the dot has `data-face="hollow"` and `data-hue="d"` |

This component has no crops: the connected pixels are checked inside `Components/AppBar` › `Board` (crop `Stage 24,24 1392×36`); the not-connected look is judged by Inspect on `NotConnected` and on `Components/AppBar` › `LaunchkeyNotConnected`.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- LaunchkeyStatus` finds no axe violation on any story, with `NotConnected`'s `color-contrast` rule off (D3); the connected pixels pass in `npm run shots -- AppBar`.
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · Own label, child dot.** LaunchkeyStatus draws the word itself and passes the dot's look to StatusDot; the green glow is StatusDot's solid `ok` glow (`--dot-glow-mix`, which in dark equals the board's `--bg`), so this component names no glow token.
- **D2 · No crop.** Its box is the dot, the gap and the text width (about 82.7 × 18 px from x 1229 on the board), not whole pixels, so it has no crop of its own; AppBar's `Board` crop checks it at its board position.
- **D3 · The dimmed label stays `--d`.** Not connected, the label is `--d` as kit › App bar draws it (the "absent" dimming), below AA; the state is carried by the accessible name, so `NotConnected` (here and in AppBar) turns off axe's `color-contrast` rule. `scripts/shots.ts` runs axe without story parameters today, so it must honour `parameters.a11y.config.rules` (a UI-library contract change) before those stories pass `npm run shots`; an owner question offers `--m` instead.
- **D4 · Same word in both states.** The visible text is "Launchkey" whether connected or not; only the dot, the colour and the accessible name change.
- **D5 · Tooltip by attachment.** The root is this component's own element, so it takes the tooltip as a `tip` attachment, as HealthSlot does (HealthSlot D7), and writes the static `data-tip` itself so Stage.md Check 15 holds without the wiring.

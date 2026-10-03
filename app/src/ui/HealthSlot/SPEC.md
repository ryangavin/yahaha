# HealthSlot

## Identity (all stations)

- **Kind:** primitive
- **Built from:** —
- **Purpose:** Says "Audio" quietly while the sound is fine, and names the trouble in red (a failed plugin, no audio, dropouts, a busy CPU) with a click that opens where to fix it.
- **Boards:**
  - `Stage-Dark.dc.html:91` (calm: "Audio" in muted grey); light: `Stage-Light.dc.html:67`. Every other board's app bar draws the same calm slot.
  - `SettingsSystem-Dark.dc.html:80` (trouble: "CPU 74%" in Ending red); light: `SettingsSystem-Light.dc.html:69`.
  - The other trouble texts are listed in the boards' comment (`Stage-Dark.dc.html:90`) but not drawn.
- **Not this component's job:** no store, no API, no Tauri, no clock. It doesn't read `AppState` or keep a dropout history: the parent passes the inputs below (the dropout count over the last 30 s is the wiring's, kept from `io.synth.dropouts` against its own clock), and acts on `onopen`. It doesn't open pages itself. It is not the Launchkey status (`LaunchkeyStatus`) and not the status line (`StatusLine`).

## API (Component station)

### Props

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `failedPart` | `0 \| 1 \| 2 \| 3 \| null` | `null` | The first keyboard part, in the order R1 R2 R3 L (0–3), whose `plugin.status` is `failed` and whose `plugin.missing` isn't true. The wiring finds it with `firstFailedPart` (below). |
| `synthOn` | `boolean` | `true` | False when `io.synth` is null (no audio). |
| `dropouts` | `number` | `0` | Audio dropouts in the last 30 s: how many times `io.synth.dropouts` rose in that window, counted by the wiring. |
| `bufferFrames` | `number \| null` | `null` | `io.synth.bufferFrames`, the buffer in use (64 … 1024). |
| `cpu` | `number \| null` | `null` | `meters.cpu.total` (1.0 = the whole buffer). Null before the first meters frame. |
| `width` | `number \| undefined` | — | A fixed width in px (stories: 100, the slot's room on the board). Default: fills its container's width (the app bar gives it what's left of its right area). |
| `tip` | `Attachment<HTMLElement> \| undefined` | — | The tooltip attachment (`svelte/attachments`), applied to the button when there is one, else to the slot (D7). The parent passes `fromAction(tip, () => 'app.health')`. Stories pass `fn()`. |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onopen` | click, Enter or Space on the slot's button (only the rows with a target have one) | `(target: HealthTarget)`: `{ page: 'channel', part: 0 \| 1 \| 2 \| 3 }` for a failed plugin; `{ page: 'settings', tab: 'system' }` for every other trouble row. The parent opens it (Stage.md D32's interim targets until Channel and Settings › System land). |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### What it shows (`health.ts`)

A pure function `health(input)` in `app/src/ui/HealthSlot/health.ts` picks the one text from the props, the first row that applies (Stage.md D14: a failed plugin first, since it's silent):

| # | When | Text (exact) | Hue (`data-hue`) | Target |
|---|---|---|---|---|
| 1 | `failedPart` isn't null | `R1 failed`, `R2 failed`, `R3 failed` or `L failed` | `ending` | `{ page: 'channel', part: failedPart }` |
| 2 | `synthOn` is false | `Audio off` | `m` | `{ page: 'settings', tab: 'system' }` |
| 3 | `dropouts` ≥ 3 and `bufferFrames` isn't null and < 1024 | `{dropouts} dropouts · buffer {suggestBuffer(bufferFrames)}?`, e.g. `3 dropouts · buffer 256?` (at 128 frames) | `ending` | settings › system |
| 4 | `dropouts` ≥ 1 | `1 dropout`, or `{dropouts} dropouts` (e.g. `2 dropouts`) | `ending` | settings › system |
| 5 | `cpu` isn't null and ≥ 0.70 | `CPU {Math.round(cpu × 100)}%`, e.g. `CPU 74%` (0.70 → `CPU 70%`) | `ending` | settings › system |
| 6 | otherwise (calm) | `Audio` | `m` | none |

It returns `{ text, hue: 'ending' | 'm', target: HealthTarget | null, label }`, where `label` is `Audio health: fine` for row 6 and `Audio health: {text}` otherwise (e.g. `Audio health: CPU 74%`). Also in `health.ts`:

- `suggestBuffer(frames: number): number`: the next size up, `Math.min(frames * 2, 1024)` (64 → 128, 128 → 256, 256 → 512, 512 → 1024) (D3).
- `firstFailedPart(plugins: ({ status: string; missing?: boolean } | undefined)[]): 0 | 1 | 2 | 3 | null`: the index of the first entry (R1, R2, R3, L) with `status === 'failed'` and `missing !== true`; null if none. The wiring calls it with the four keyboard parts' `plugin` (absent for a part without one).
- `HealthTarget` (the type above) is exported for the wiring.

### Visual rules

- **Tokens used:** `--m`, `--ending`, `--focus`, `--font-sans`, `--text-14`, `--weight-regular`, `--space-8`, `--line-width`, `--focus-offset`.
- **Structure:** a `<span role="status">` with `aria-label` = `label`, `data-hue` = `hue`: `box-sizing: border-box`, `display: flex; align-items: center; justify-content: flex-end` (the text sits at the right edge), height 35px (the app bar's 36px less its 1px bottom border; D5), `padding-left: var(--space-8)`, `min-width: 0`, width `width` px or `flex: 1 1 auto` in its container, `color: var(--m)` or `var(--ending)` from `hue`, no background (the app bar's ground shows through). Inside it:
  - with a target: one `<button type="button">`: margin 0, padding 0, border 0, background transparent, `color: inherit`, `font: inherit`, `max-width: 100%`, `min-width: 0`, cursor `pointer`, holding the text in a `<span>` with `overflow: hidden; text-overflow: ellipsis; white-space: nowrap`.
  - without one (calm): the text directly, in the same ellipsis `<span>`.
- **Size:** 35px tall; width as above. The text never wraps; when it's wider than the slot less its 8px left padding, it ends in "…" (the slot's own box doesn't clip, so the focus ring shows).
- **States drawn by:** colour only (no face, no fill, no border; a text button, kit › Text buttons):
  - calm: `Audio` in `--m`, plain text, default cursor.
  - no audio: `Audio off` in `--m`, a button.
  - failed plugin, buffer hint, dropouts, CPU: the text in `--ending`, a button.
  - too long: ellipsis (the buffer hint in the 100px the app bar has, D4).
  - keyboard focus (`:focus-visible` on the button): a `--line-width` solid outline in `--focus`, `--focus-offset` outside the button. Nothing on mouse focus.
  - No hover or pressed look (kit: Hover, press and cursor).
- **Type:** DM Sans (`--font-sans`), `--text-14`, `--weight-regular`, `line-height: normal`, sentence case as listed, `font-variant-numeric: tabular-nums` (so "CPU 74%" doesn't jitter as the number changes).
- **Contrast (AA 4.5:1, `tokens/contrast.test.ts`):** `--m` on `--g` (exists); `--ending` on `--g` (new row: "health slot trouble (HealthSlot)"; 5.0:1 dark, 5.2:1 light).
- **Motion:** none. The text changes when the props change; no blink, no timer.

### Accessibility

- **Role and name:** the slot is `role="status"` (polite live region) named by `aria-label` (`Audio health: fine`, `Audio health: R3 failed`, …), so a change is announced. The button inside it, when there is one, is named by its text (`R3 failed`, `Audio off`, `CPU 74%`).
- **Keyboard:** with a button, Tab reaches it (in the app bar after the page tabs); Enter or Space calls `onopen` (native button). Calm, nothing in the slot is focusable.
- **Tooltip id:** `app.health` (new, contract change C5 in Stage.md), one key for every state (D45), through the `tip` prop: on the button, or on the span when calm.

## Stories (Story station)

Title `Primitives/HealthSlot`, `layout: 'centered'`. Every story renders in dark and light (the toolbar theme). Every story passes `onopen: fn()` and `tip: fn()`.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ failedPart: null, synthOn: true, dropouts: 0, bufferFrames: 256, cpu: 0.2, width: 100 }` | the Stage board fixture: calm, `Audio` in muted grey at the right edge (R3's plugin is missing, so it isn't "failed") | `Board-{dark,light}.png` (Stage 1316,24 100×35) | the `status` element is named `Audio health: fine`, has `data-hue="m"` and text `Audio`; `queryByRole('button')` is null |
| `Cpu` | `{ synthOn: true, dropouts: 0, bufferFrames: 256, cpu: 0.74, width: 100 }` | `CPU 74%` in Ending red | `Cpu-{dark,light}.png` (SettingsSystem 1316,24 100×35) | the status is named `Audio health: CPU 74%`, `data-hue="ending"`; click the button `CPU 74%` → `onopen` called with `{ page: 'settings', tab: 'system' }` |
| `Failed` | `{ failedPart: 2, synthOn: true, bufferFrames: 256, cpu: 0.2, width: 100 }` | `R3 failed` in red | — (not drawn on a board) | the status is named `Audio health: R3 failed`, `data-hue="ending"`; click the button `R3 failed` → `onopen` called with `{ page: 'channel', part: 2 }` |
| `FailedBeatsOff` | `{ failedPart: 3, synthOn: false, width: 100 }` | `L failed`: a failed plugin outranks no audio | — | the button is named `L failed` |
| `AudioOff` | `{ synthOn: false, width: 100 }` | `Audio off` in muted grey, clickable | — | the status is named `Audio health: Audio off`, `data-hue="m"`; focus the button `Audio off`, press Enter → `onopen` called with `{ page: 'settings', tab: 'system' }` |
| `Dropouts` | `{ dropouts: 2, bufferFrames: 256, cpu: 0.2, width: 100 }` | `2 dropouts` in red | — | the button is named `2 dropouts`; Space → `onopen` called with `{ page: 'settings', tab: 'system' }` |
| `OneDropout` | `{ dropouts: 1, bufferFrames: 256, width: 100 }` | `1 dropout` (singular) | — | the button is named `1 dropout` |
| `BufferHint` | `{ dropouts: 3, bufferFrames: 128, cpu: 0.74 }` | the whole `3 dropouts · buffer 256?` in red, no width set (outranks CPU) | — | the status is named `Audio health: 3 dropouts · buffer 256?`; the button is named `3 dropouts · buffer 256?` |
| `BufferHintInBar` | `{ dropouts: 3, bufferFrames: 128, width: 100 }` | the same text cut to the app bar's room, ending in "…" (D4) | — (not drawn; text measurement) | the status is still named `Audio health: 3 dropouts · buffer 256?` |
| `LargestBuffer` | `{ dropouts: 3, bufferFrames: 1024, width: 100 }` | `3 dropouts`: no larger buffer to suggest | — | the button is named `3 dropouts` |
| `Focused` | `{ cpu: 0.74, bufferFrames: 256, width: 100 }`, `parameters: { pseudo: { focusVisible: true } }` | the focus ring around `CPU 74%` | — (the boards draw no focus) | — |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in the dark and light render. The box is the slot's room on the board: from 4px after the Launchkey label's last pixel (x 1311) to the app bar's right edge (x 1416), and from the app bar's top to just above its 1px bottom line (y 24 to 59), so the crops hold only the slot's text and the ground.

## Checks

- **Vitest, pure (`app/src/ui/HealthSlot/health.test.ts`):** each row of the table, and its order: `health({ failedPart: 2, synthOn: false, dropouts: 5, bufferFrames: 128, cpu: 0.9 })` is `R3 failed` (row 1 wins); `{ synthOn: false, dropouts: 5 }` → `Audio off`; `{ dropouts: 3, bufferFrames: 128, cpu: 0.9 }` → `3 dropouts · buffer 256?`; `{ dropouts: 4, bufferFrames: 512 }` → `4 dropouts · buffer 1024?`; `{ dropouts: 3, bufferFrames: 1024 }` → `3 dropouts`; `{ dropouts: 3, bufferFrames: null }` → `3 dropouts`; `{ dropouts: 1 }` → `1 dropout`; `{ cpu: 0.695 }` → `Audio` (the threshold is on the raw value); `{ cpu: 0.70 }` → `CPU 70%`; `{ cpu: 0.744 }` → `CPU 74%`; `{ cpu: null }` → `Audio` with label `Audio health: fine` and target null. `suggestBuffer` for 64, 128, 256, 512, 1024 → 128, 256, 512, 1024, 1024. `firstFailedPart([undefined, {status:'playing'}, {status:'failed', missing:true}, {status:'failed'}])` → 3; with no failed entry → null.
- **Vitest, the stories' `play` (`npx vitest run src/ui`):** the plays above: the status name, `data-hue`, the button's presence and name, `onopen`'s payload on click, Enter and Space.
- **Screenshots (`npm run shots -- HealthSlot`):** `Board` and `Cpu` against their crops (100×35, score at most 0.02). They cover what jsdom can't: the colours by hue, the right alignment, the vertical centring in 35px. `BufferHintInBar` (ellipsis) and `Focused` (ring) are judged by Inspect, without a crop. axe passes on every story.
- **Contrast:** add the row `['--ending', '--g', 'health slot trouble (HealthSlot)']` to `tokens/contrast.test.ts` (a plant file: the orchestrator lands it).

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`); `health.test.ts` passes.
- Each cropped story's screenshot matches its crop (`npm run shots -- HealthSlot`: score at most 0.02, or the Inspect agent judges any difference to be render noise); axe finds no violation.
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules (the 35px height is the only one).
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · Inputs, not state.** The slot takes the few values it needs as props and a pure `health()` picks the text; the 30 s dropout window is counted by the wiring (it owns the clock), so the component has no timer (axiom 10).
- **D2 · No `DropoutWatch.show`.** The buffer hint is rule 3 (3 dropouts in 30 s, buffer below 1024, the same numbers as `app/src/lib/dropouts.svelte.ts`) computed from the window count, not the watch's latched `show`: the slot has no dismiss, so a latched hint would never go away; it now ends when the window empties.
- **D3 · The hint suggests the next size.** "buffer 256?" is a question about a size to try, so it names the next size up (double, at most 1024), not the size in use; the boards' "3 dropouts · buffer 256?" is the 128-frame case. kit.md says `io.synth.bufferFrames`; this changes it.
- **D4 · Long texts ellipsize.** The slot has about 96px of text room in the fixed 196px right area, so "3 dropouts · buffer 256?" shows as "3 dropouts · bu…" in the app bar (kit: "ellipsis when long"); the full text is in the status's name. Whether to shorten it is an owner question.
- **D5 · 35px tall.** The slot is the app bar's height less its 1px bottom line, with the text centred, which is where the board's text sits (the board centres a 36px right area in the bar's 35px content box).
- **D6 · Buttons named by their text.** The status carries "Audio health: …"; the button inside is named by its visible text only, so a screen reader doesn't hear "Audio health" twice.
- **D7 · Tooltip through an attachment prop.** The focusable element is the button or (calm) the span, so the parent can't place `use:tip` itself; the component takes `tip` (an attachment) and applies it to whichever exists. Stories pass `fn()`, which the story test counts as an action.
- **D8 · "Audio off" is grey but clickable.** No synth isn't an alarm the player caused (it can be a choice in Settings), so it stays `--m` as the kit says, yet it opens Settings › System like the red rows.
- **D9 · CPU rounds, the threshold doesn't.** The text rounds to a whole percent; the 0.70 test is on the raw value, so 0.695 stays calm rather than showing "CPU 70%" in red.

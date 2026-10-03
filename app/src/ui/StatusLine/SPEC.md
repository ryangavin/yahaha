# StatusLine

## Identity (all stations)

- **Kind:** primitive
- **Built from:** —
- **Purpose:** Says what the app just did or refused (the last notice or error), in one line between the band and the keys; a click puts it away.
- **Boards:**
  - `Prompts-Dark.dc.html:401` (an error: the ⚠ and "Can't delete Sunday drive: it's the loaded rack."); light: `Prompts-Light.dc.html:393`.
  - `Prompts-More-Dark.dc.html:416` (a notice: "Added SlowWalker.T552 to the library and loaded it."); light: `Prompts-More-Light.dc.html:397`.
  - Empty, on the Stage: the 20px gap between the band and the keys, `Stage-Dark.dc.html:389-391` (the end of the band and the `KEYS` comment); light: `Stage-Light.dc.html:365-367`. The Stage board draws no element there (D2).
- **Not this component's job:** no store, no API, no Tauri: the parent passes the message's fields and sends `clearMessage` on `onclear`. It never decides what to say, never coaches ("press this to…") and never times itself out: a message stays until the state clears it (a successful style change or `clearMessage`). It is not the health slot (audio trouble lives in `HealthSlot`), not the help footer and not a toast.

## API (Component station)

### Props

The props mirror `state.message` (`{ seq, text, error } | null`, `docs/app-api.md` › `message`), passed as plain values: a null message is `text: null`.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `text` | `string \| null` | `null` | `message.text`. `null` or `''`: no message, the line is empty. |
| `error` | `boolean` | `false` | `message.error`: the ⚠ shows before the text. Ignored while there is no text. |
| `seq` | `number` | `0` | `message.seq`. A new `seq` with the same text re-renders the text (`{#key seq}`), so a screen reader announces the repeat as a new message. |
| `width` | `number \| undefined` | — | A fixed width in px (stories: 1392, the line's width on every display page). Default: fills its container's width. |
| `tip` | `Attachment<HTMLElement> \| undefined` | — | The tooltip attachment (`svelte/attachments`), applied to the button while there is a message (D7). The parent passes `fromAction(tip, () => 'display.status')`. Stories pass `fn()`. |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onclear` | click, Enter or Space on the line's button (only exists while there is a message) | `()` none; the parent sends `clearMessage` |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Visual rules

- **Tokens used:** `--t`, `--warn`, `--focus`, `--font-sans`, `--text-14`, `--weight-regular`, `--space-8`, `--space-20`, `--line-width`, `--focus-offset`.
- **Structure:** a `<p role="status" aria-live="polite">`, margin 0, `box-sizing: border-box`, height `--space-20` (20px), `display: flex; align-items: center`, `overflow: visible` (so the focus ring isn't clipped), transparent background (the ground shows through). With a message it holds one `<button type="button">`; without one it holds nothing.
- **The button:** `display: flex; align-items: center; gap: var(--space-8)`; `max-width: 100%`; height 20; margin 0; padding 0; border 0; background transparent; `color: var(--t)`; `font: inherit`; text-align left; cursor `pointer`. Inside it, in order:
  1. with `error`: the warn mark, an inline `<svg aria-hidden="true" data-hue="warn" width="12" height="12" viewBox="0 0 12 12">`, `flex: none`, drawn exactly as `PartMarks`' ⚠ (the same three shapes): a triangle `<path d="M6 1.5 L11 10.5 H1 Z" fill="none" stroke-width="1" stroke-linejoin="round">`, a stem `<path d="M6 5 V7.4" stroke-width="1">`, both `stroke: var(--warn)`, and a dot `<circle cx="6" cy="8.9" r="0.6">` with `fill: var(--warn)` and no stroke. It sits vertically centred (4px above and below in the 20px row).
  2. the text, a `<span>`: `min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap`, so a long message ends in "…" inside the width and never wraps.
- **Size:** 20px tall always (empty too, so the keys never move); width `width` px or 100% of the container. The text starts at the line's left edge (no ⚠) or 20px in (12px mark + 8px gap).
- **States drawn by:**
  - empty (`text` null or `''`): nothing drawn; the 20px row stays.
  - notice: the text in `--t`, no mark.
  - error: the warn mark in `--warn`, 8px gap, the text in `--t` (the text stays `--t`, not orange: the mark says it's an error, D3).
  - long text: ellipsis at the right edge.
  - keyboard focus (`:focus-visible` on the button): a `--line-width` solid outline in `--focus`, `--focus-offset` outside the button. Nothing on mouse focus.
  - No hover or pressed look, no fill, no border, no glow (kit: Hover, press and cursor).
- **Type:** DM Sans (`--font-sans`), `--text-14`, `--weight-regular`, line-height 20px (`--space-20`), sentence case as given (the text is shown exactly as the state sends it), `font-variant-numeric: tabular-nums`.
- **Contrast (AA 4.5:1, `tokens/contrast.test.ts`):** `--t` on `--g` (exists). The warn mark is a non-text graphic (WCAG 1.4.11, 3:1): `--warn` on `--g` is 9.9:1 dark and 3.6:1 light; no new row (the test checks text pairs only).
- **Motion:** none. A new message replaces the text at once; no fade, no timer.

### Accessibility

- **Role and name:** the `<p>` is `role="status"` with `aria-live="polite"` (no accessible name of its own), so each new text is announced. The button's accessible name is the text, prefixed "Error: " when `error` (an `aria-label`, so a screen reader hears what the ⚠ shows): `Error: Can't delete Sunday drive: it's the loaded rack.`. The svg is `aria-hidden`.
- **Keyboard:** while there is a message, Tab reaches the button (after the band, before nothing: the key strip isn't focusable); Enter or Space calls `onclear` (native button). Empty, nothing in the line is focusable.
- **Tooltip id:** `display.status` (exists in `app/src/help/tooltips.ts`), through the `tip` prop on the button. Empty, no element carries it (nothing to hover or focus).

## Stories (Story station)

Title `Primitives/StatusLine`, `layout: 'centered'`. Every story renders in dark and light (the toolbar theme). Every story passes `onclear: fn()` and `tip: fn()`.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ text: null, width: 1392 }` | the Stage at the board fixture (`message` null): an empty 20px row on the ground | `Board-{dark,light}.png` (Stage 24,800 1392×20) | the `status` element exists and is empty (`textContent` is `''`); `queryByRole('button')` is null |
| `Error` | `{ text: "Can't delete Sunday drive: it's the loaded rack.", error: true, seq: 4, width: 1392 }` | the warn mark, then the white text | `Error-{dark,light}.png` (Prompts 24,800 1392×20) | the button named `Error: Can't delete Sunday drive: it's the loaded rack.` exists inside the `status` element; the svg with `data-hue="warn"` exists |
| `Notice` | `{ text: 'Added SlowWalker.T552 to the library and loaded it.', seq: 5, width: 1392 }` | the text alone, from the left edge | `Notice-{dark,light}.png` (Prompts-More 24,800 1392×20) | the button is named `Added SlowWalker.T552 to the library and loaded it.`; no element with `data-hue="warn"` |
| `Clears` | `{ text: 'One Touch 3 applied · Recovered: Sunday drive', seq: 6, width: 1392 }` | — | — (not drawn on a status line; QuickRacks draws the text elsewhere) | click the button → `onclear` called once with no arguments; focus it and press Enter → called twice; Space → three times |
| `LongText` | `{ text: 'Rhodes Soft + Strings could not load: the plugin Sampler Deluxe is missing, so Right 1 plays its SoundFont voice until the plugin is installed and scanned again.', error: true, seq: 7, width: 600 }` | the text ends in "…" on one line inside 600px, the row stays 20px | — (the boards draw no long message) | the button's accessible name is the whole text with "Error: " |
| `Focused` | `{ text: "Can't delete Sunday drive: it's the loaded rack.", error: true, seq: 4, width: 1392 }`, `parameters: { pseudo: { focusVisible: true } }` | the 1px focus ring 2px outside the button (around the mark and the text) | — (the boards draw no focus) | — |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in the dark and light render. The Prompts boards draw a wash over the Stage above y 820; at the status line it is the ground colour in both themes (dark: black at 62% over black; light: the ground at 72% over the ground), so the crops' background is plain `--g`, as the story's.

## Checks

- **Vitest (the stories' `play`, `npx vitest run src/ui`):** the plays above: role `status`, the button and its name, `data-hue="warn"` on the mark only with `error`, `onclear` on click, Enter and Space, nothing focusable when empty.
- **Screenshots (`npm run shots -- StatusLine`):** `Board`, `Error` and `Notice` against their crops (1392×20, score at most 0.02). They cover what jsdom can't: the mark's drawing and colour, the 8px gap, the text colour and size, the empty row's height. `LongText` (ellipsis, text measurement) and `Focused` (the ring) are judged by Inspect, without a crop. axe passes on every story.
- **Contrast:** no new pair (`--t` on `--g` exists).

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- Each cropped story's screenshot matches its crop (`npm run shots -- StatusLine`: score at most 0.02, or the Inspect agent judges any difference to be render noise); axe finds no violation.
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules (the svg's 12 × 12 and its path geometry are the only literals).
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · Kit's look everywhere.** The status line is the kit's (`--t` text, `--warn` mark) on every page; the tall-page boards' older line (LibraryRacks, LibraryInstruments: `--t2` text in an `--m` paragraph, no ⚠) is not a second variant.
- **D2 · Board story is empty.** At the Stage board fixture `message` is null, so `Board` renders the empty 20px row and its crop is the Stage board's empty gap at `24,800 1392×20`.
- **D3 · Text stays white on errors.** Only the mark is orange; the board draws the error text in `--t`, and red or orange text would read as the health slot's trouble.
- **D4 · The ⚠ is the board's svg, not a glyph.** A font ⚠ renders differently per platform and wouldn't match the crop; the svg is the same drawing `PartMarks` uses for missing.
- **D5 · No action in the line.** `message` has no action field, so the line has no "Undo" or "Save rack" button of its own (QuickRacks and Prompts notes mention them); the only action is clearing it. An action on a message is a follow-up for the contract.
- **D6 · Health isn't a message.** The SettingsSystem board's status line "CPU 74%: above 70%" is not `state.message`; CPU and dropouts show in `HealthSlot`, and the status line shows only what the state's `message` says.
- **D7 · Tooltip through an attachment prop.** The button exists only while there is a message, so the parent can't put `use:tip` on it; the component takes `tip` (an attachment) and applies it to the button. Stories pass `fn()`, which the story test counts as an action.
- **D8 · Error in the name.** The button's accessible name gets "Error: " before the text when `error`, since the ⚠ is hidden from screen readers.
- **D9 · `seq` re-announces.** The text is keyed on `seq`, so the same text sent twice (two refusals) is read out twice, as the API intends (`seq` "counts as two messages").

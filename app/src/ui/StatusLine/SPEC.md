# StatusLine

## Identity (all stations)

- **Kind:** primitive
- **Built from:** —
- **Purpose:** Says what the app just did or refused (the last notice or error), in one line between the band and the keys; a click puts it away.
- **Boards:**
  - `Prompts-Dark.dc.html:401` (an error: the ⚠ and "Can't delete Sunday drive: it's the loaded rack."; drawn over the scrim while a prompt is up, as plain text with no button: the `inert` line, D13); light: `Prompts-Light.dc.html:393`.
  - `Prompts-More-Dark.dc.html:416` (a notice: "Added SlowWalker.T552 to the library and loaded it."); light: `Prompts-More-Light.dc.html:397`.
  - Empty, on the Stage: the 20px gap between the band and the keys, `Stage-Dark.dc.html:389-391` (the end of the band and the `KEYS` comment); light: `Stage-Light.dc.html:365-367`. The Stage board draws no element there (D2).
- **Not this component's job:** no store, no API, no Tauri: the parent passes the message's fields and sends `clearMessage` on `onclear`. It never decides what to say, never coaches ("press this to…") and never times itself out: a message stays until the state clears it (a successful style change or `clearMessage`). It is not the health slot (audio trouble lives in `HealthSlot`), not the help footer and not a toast. It doesn't know a prompt is up: the overlay passes `inert` to its own copy and the page wiring passes `hidden` (from `ui.modal`, through `Stage`'s `statusHidden`) to the page's copy (Prompts.md › Kit additions).

## API (Component station)

### Props

The props mirror `state.message` (`{ seq, text, error } | null`, `docs/app-api.md` › `message`), passed as plain values: a null message is `text: null`.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `text` | `string \| null` | `null` | `message.text`. `null` or `''`: no message, the line is empty. |
| `error` | `boolean` | `false` | `message.error`: the ⚠ shows before the text. Ignored while there is no text. |
| `seq` | `number` | `0` | `message.seq`. A new `seq` re-creates the button's contents (`{#key seq}` inside the button, D10), so a screen reader announces a repeat of the same text as a new message, and the button itself (with its focus) stays. |
| `width` | `number \| undefined` | — | A fixed width in px, set as an inline `style:width="{width}px"` on the `<p>` (stories: 1392, the line's width on every display page). Unset: no inline width, and the `<p>`'s CSS `width: 100%` fills its container's content box (D11). |
| `tip` | `string \| null` | `'display.status'` | The tooltip key (`app/src/help/tooltips.ts`), rendered as `data-tip={tip}` on the button while there is a message (L3, D7). Left out (or `undefined`) it is the default; `null` turns it off: no `data-tip` and no action. |
| `tipAction` | `Action<HTMLElement, string> \| undefined` | — | The app's `use:tip` action, passed in by the wiring (the library can't import it). When `tip` and `tipAction` are both set the button gets `use:tipAction={tip}`; otherwise no action is applied (L3, D7). Stories pass `fn()`. |
| `inert` | `boolean` | `false` | The line over a prompt (the overlay's copy): the same drawing, but the message is plain text in a `<span>`, with no button, nothing focusable, no `data-tip` and no `onclear` (Prompts.md PR-D23, D13). Only a prop: it is never forwarded as the HTML `inert` attribute (the live region must keep speaking). |
| `hidden` | `boolean` | `false` | The page's own copy while a prompt is up: `aria-hidden="true"` and `data-hidden` on the `<p>`, `visibility: hidden`, the 20px row kept, and drawn as `inert` (no button), so one live region speaks and nothing hidden is focusable (D14, D15). Only a prop: not the HTML `hidden` attribute (which would drop the row). |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onclear` | click, Enter or Space on the line's button (only exists while there is a message, and not `inert` or `hidden`) | `()` none; the parent sends `clearMessage` |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Visual rules

- **Tokens used:** `--t`, `--warn`, `--focus`, `--font-sans`, `--text-14`, `--weight-regular`, `--space-8`, `--space-20`, `--line-width`, `--focus-offset`. All exist in `app/src/ui/tokens/*` today: no new tokens and no new contrast rows (L1). The button's height is `var(--space-20)`.
- **Structure:** a `<p role="status" aria-live="polite">`, margin 0, `box-sizing: border-box`, `width: 100%` (overridden by the inline width when `width` is set), `min-width: 0`, height `--space-20` (20px), `display: flex; align-items: center`, `overflow: visible` (so the focus ring isn't clipped), transparent background (the ground shows through). It always carries `data-shot-mask="status"` (the screenshot mask of an overlay or page story, Prompts.md › Checks; D16). With a message it holds one `<button type="button">` (or, `inert` or `hidden`, one `<span class="line">`, below); without one it holds nothing.
- **The inert line** (`inert` or `hidden`, with a message): a `<span class="line">` in place of the button, with the button's layout (`display: flex; align-items: center; gap: var(--space-8)`; `max-width: 100%`; `min-width: 0`; height 20; `color: var(--t)`), the same children inside the same `{#key seq}` block (the warn mark, the text span with the hidden "Error: "), `cursor: default`, no tabindex, no `data-tip`, no action, no handler. It draws exactly as the button does.
- **Hidden:** with `hidden`, the `<p>` has `aria-hidden="true"`, `data-hidden` and `visibility: hidden` (scoped CSS on `[data-hidden]`); its 20px row stays, so the keys never move. Without `hidden`, no `aria-hidden` and no `data-hidden` attribute.
- **The button:** `display: flex; align-items: center; gap: var(--space-8)`; `max-width: 100%`; `min-width: 0`; height 20; margin 0; padding 0; border 0; background transparent; `color: var(--t)`; `font: inherit`; text-align left; cursor `pointer`. No `aria-label`: its name is its content (D8). Its children are inside one `{#key seq}` block (the button is outside it, D10), in order:
  1. with `error`: the warn mark, an inline svg drawn exactly as `PartMarks`' ⚠ (PartMarks D9: geometry in attributes, paint in CSS), `flex: none`, vertically centred (4px above and below in the 20px row):
     ```svelte
     <svg aria-hidden="true" data-hue="warn" class="warn" width="12" height="12" viewBox="0 0 12 12">
       <path d="M6 1.5 L11 10.5 H1 Z" /><path d="M6 5 V7.4" /><circle cx="6" cy="8.9" r="0.6" />
     </svg>
     ```
     ```css
     .warn { flex: none; fill: none; stroke: var(--warn); stroke-width: 1; stroke-linejoin: round; }
     .warn circle { fill: var(--warn); stroke: none; }
     ```
  2. the text, a `<span class="text">`: `min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap`, so a long message ends in "…" inside the width and never wraps. With `error` it starts with a visually hidden `<span class="hidden-word">Error: </span>` (scoped CSS: `position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap`, the same rule as `app.css`'s `.visually-hidden`, which the library can't rely on), then the message text.
- **Size:** 20px tall always (empty too, so the keys never move); width `width` px or 100% of the container. The text starts at the line's left edge (no ⚠) or 20px in (12px mark + 8px gap).
- **States drawn by:**
  - empty (`text` null or `''`): nothing drawn; the 20px row stays.
  - notice: the text in `--t`, no mark.
  - error: the warn mark in `--warn`, 8px gap, the text in `--t` (the text stays `--t`, not orange: the mark says it's an error, D3).
  - long text: ellipsis at the right edge.
  - inert (over a prompt): the same pixels as notice or error; only the markup differs (a span, not a button).
  - hidden: nothing visible; the row's space kept.
  - keyboard focus (`:focus-visible` on the button): a `--line-width` solid outline in `--focus`, `--focus-offset` outside the button. Nothing on mouse focus.
  - No hover or pressed look, no fill, no border, no glow (kit: Hover, press and cursor).
- **Type:** DM Sans (`--font-sans`), `--text-14`, `--weight-regular`, line-height 20px (`--space-20`), sentence case as given (the text is shown exactly as the state sends it), `font-variant-numeric: tabular-nums`.
- **Contrast (AA 4.5:1, `tokens/contrast.test.ts`):** `--t` on `--g` (exists). The warn mark is a non-text graphic (WCAG 1.4.11, 3:1): `--warn` on `--g` is 9.9:1 dark and 3.6:1 light; no new row (the test checks text pairs only).
- **Motion:** none. A new message replaces the text at once; no fade, no timer.

### Accessibility

- **Role and name:** the `<p>` is `role="status"` with `aria-live="polite"` (no accessible name of its own), so each new text is announced: the live region reads the content it gains, which is the same words as the button's name. The button has no `aria-label` in either case; its accessible name is its content: the text for a notice (`Added SlowWalker.T552 to the library and loaded it.`), and the hidden "Error: " plus the text for an error (`Error: Can't delete Sunday drive: it's the loaded rack.`), so the announcement and the name both say "Error" (D8). The svg is `aria-hidden`.
- **Keyboard:** while there is a message, Tab reaches the button (after the band, before nothing: the key strip isn't focusable); Enter or Space calls `onclear` (native button). A new message or `seq` while the button has focus keeps the focus (only the button's contents are keyed). When the message clears (the parent's `clearMessage` after `onclear`), the button is removed and focus falls to the document, as for any removed control (D12). Empty, nothing in the line is focusable. `inert` or `hidden`: nothing in the line is focusable either; if the button had focus when either turns true it is removed and focus falls to the document (the overlay then takes focus, Prompts.md PR-D5).
- **Live region:** `inert` keeps `role="status"` and `aria-live="polite"`, so a refusal that arrives while a prompt is up is announced from the overlay's copy; `hidden` adds `aria-hidden="true"`, so the page's copy says nothing meanwhile (Prompts.md PR-D23).
- **Tooltip id:** `display.status` (exists in `app/src/help/tooltips.ts`), the `tip` default, on the button as `data-tip` plus `use:tipAction` (L3). Empty, `inert` or `hidden`, no element carries it (nothing to hover or focus).

## Stories (Story station)

Title `Primitives/StatusLine`, `layout: 'centered'`. Every story renders in dark and light (the toolbar theme). Every story passes `onclear: fn()`, `tip: 'display.status'` and `tipAction: fn()`. Every play that has a button also checks it has `data-tip="display.status"`.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ text: null, width: 1392 }` | the Stage at the board fixture (`message` null): an empty 20px row on the ground | `Board-{dark,light}.png` (Stage 24,800 1392×20) | the `status` element exists and is empty (`textContent` is `''`); `queryByRole('button')` is null |
| `Error` | `{ text: "Can't delete Sunday drive: it's the loaded rack.", error: true, seq: 4, width: 1392 }` | the warn mark, then the white text | `Error-{dark,light}.png` (Prompts 24,800 1392×20) | the button named `Error: Can't delete Sunday drive: it's the loaded rack.` exists inside the `status` element; the svg with `data-hue="warn"` exists |
| `Notice` | `{ text: 'Added SlowWalker.T552 to the library and loaded it.', seq: 5, width: 1392 }` | the text alone, from the left edge | `Notice-{dark,light}.png` (Prompts-More 24,800 1392×20) | the button is named `Added SlowWalker.T552 to the library and loaded it.`; no element with `data-hue="warn"` |
| `Clears` | `{ text: 'One Touch 3 applied · Recovered: Sunday drive', seq: 6, width: 1392 }` | — | — (not drawn on a status line; QuickRacks draws the text elsewhere) | click the button → `onclear` called once with no arguments; focus it and press Enter → called twice; Space → three times (the story's args don't change, so the button stays) |
| `LongText` | `{ text: 'Rhodes Soft + Strings could not load: the plugin Sampler Deluxe is missing, so Right 1 plays its SoundFont voice until the plugin is installed and scanned again.', error: true, seq: 7, width: 600 }` | the text ends in "…" on one line inside 600px, the row stays 20px | — (the boards draw no long message) | the button's accessible name is the whole text with "Error: " |
| `Focused` | `{ text: "Can't delete Sunday drive: it's the loaded rack.", error: true, seq: 4, width: 1392 }`, `parameters: { pseudo: { focusVisible: true } }` | the 1px focus ring 2px outside the button (around the mark and the text) | — (the boards draw no focus) | — |
| `Inert` | `{ text: "Can't delete Sunday drive: it's the loaded rack.", error: true, seq: 4, width: 1392, inert: true }` | the line as the Prompts board draws it over the scrim: the warn mark and the text, no button | `Inert-{dark,light}.png` (Prompts 24,800 1392×20; the same box as `Error`) | the `status` element has `aria-live="polite"`, `data-shot-mask="status"` and text "Error: Can't delete Sunday drive: it's the loaded rack."; `queryByRole('button')` is null; no element inside has `data-tip` or a tabindex; the svg with `data-hue="warn"` exists; `tipAction` was not called |
| `Hidden` | `{ text: 'Added SlowWalker.T552 to the library and loaded it.', seq: 5, width: 1392, hidden: true }` | the page's copy while a prompt is up: nothing visible, the 20px row kept | `Hidden-{dark,light}.png` (Stage 24,800 1392×20; the same box as `Board`: an empty row) | the `<p>` has `aria-hidden="true"`, `data-hidden` and `data-shot-mask="status"`; it contains no button and nothing focusable (`queryByRole('status')` is null, since it is hidden; query the `<p>` by `[data-shot-mask="status"]`) |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in the dark and light render. The Prompts boards draw a wash over the Stage above y 820; at the status line it is the ground colour in both themes (dark: black at 62% over black; light: the ground at 72% over the ground), so the crops' background is plain `--g`, as the story's.

## Checks

- **Vitest (the stories' `play`, `npx vitest run src/ui`):** the plays above: role `status`, the button and its name, `data-hue="warn"` on the mark only with `error`, `onclear` on click, Enter and Space, nothing focusable when empty.
- **Vitest, component (`app/src/ui/StatusLine/StatusLine.test.ts`, `@testing-library/svelte`, since a play can't change its args):** render the `Error` args with `seq: 4`, focus the button, `rerender({ seq: 5 })` (same text): the same button element is `document.activeElement` and its name is unchanged (D10). `rerender({ text: 'Added SlowWalker.T552 to the library and loaded it.', error: false, seq: 6 })`: still the same focused button, now named by the new text, no `data-hue="warn"`. `rerender({ text: null })`: no button, the `status` element's `textContent` is `''`. With no `width`, the `<p>` has no inline `style.width` (D11); with `width: 600`, its `style.width` is `600px`. Inert and hidden (D13–D15): render the `Error` args, focus the button, `rerender({ inert: true })`: no button, the text and the warn mark still inside the `status` element, `document.activeElement` is `document.body`; `rerender({ inert: false })`: the button is back (named as before). `rerender({ hidden: true })`: `aria-hidden="true"` and `data-hidden` on the `<p>`, no button; `rerender({ hidden: false })`: neither attribute. Every render has `data-shot-mask="status"` on the `<p>`, empty too.
- **Vitest, over the scrim (Prompts.md › Checks 9):** `PromptOverlay`'s test checks its one `role="status"` region (an inert StatusLine) reads the refusal with the ⚠ and no button; `Stage`'s test checks `statusHidden` reaches this component as `hidden`. Those tests are the overlay's and Stage's, not this folder's.
- **Screenshots (`npm run shots -- StatusLine`):** `Board`, `Error`, `Notice`, `Inert` and `Hidden` against their crops (1392×20, score at most 0.02). They cover what jsdom can't: the mark's drawing and colour, the 8px gap, the text colour and size, the empty row's height. `LongText` (ellipsis, text measurement) and `Focused` (the ring) are judged by Inspect, without a crop. axe passes on every story.
- **Contrast:** no new pair (`--t` on `--g` exists).

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`); `StatusLine.test.ts` passes.
- Each cropped story's screenshot matches its crop (`npm run shots -- StatusLine`: score at most 0.02, or the Inspect agent judges any difference to be render noise); axe finds no violation.
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules (the svg's 12 × 12 and its path geometry, and the hidden word's `1px` box, are the only literals).
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · Kit's look everywhere.** The status line is the kit's (`--t` text, `--warn` mark) on every page; the tall-page boards' older line (LibraryRacks, LibraryInstruments: `--t2` text in an `--m` paragraph, no ⚠) is not a second variant.
- **D2 · Board story is empty.** At the Stage board fixture `message` is null, so `Board` renders the empty 20px row and its crop is the Stage board's empty gap at `24,800 1392×20`.
- **D3 · Text stays white on errors.** Only the mark is orange; the board draws the error text in `--t`, and red or orange text would read as the health slot's trouble.
- **D4 · The ⚠ is the board's svg, not a glyph.** A font ⚠ renders differently per platform and wouldn't match the crop; the svg is the same drawing `PartMarks` uses for missing.
- **D5 · No action in the line.** `message` has no action field, so the line has no "Undo" or "Save rack" button of its own (QuickRacks and Prompts notes mention them); the only action is clearing it. An action on a message is a follow-up for the contract.
- **D6 · Health isn't a message.** The SettingsSystem board's status line "CPU 74%: above 70%" is not `state.message`; CPU and dropouts show in `HealthSlot`, and the status line shows only what the state's `message` says.
- **D7 · Tooltip by key plus action (L3).** The button exists only while there is a message, so the parent can't put `use:tip` on it; the component takes `tip` (the key, default `display.status`, as `data-tip`) and `tipAction` (the app's action) and applies them to the button. No attachment prop. Stories pass `tip: 'display.status'` and `tipAction: fn()`.
- **D8 · Error in the name, as content.** "Error: " is a visually hidden word inside the button, not an `aria-label`, so the button's name and what the live region announces are the same words; a notice's button has no `aria-label` either.
- **D9 · `seq` re-announces.** The button's contents are keyed on `seq`, so the same text sent twice (two refusals) is read out twice, as the API intends (`seq` "counts as two messages").
- **D10 · The key block is inside the button.** `{#key seq}` wraps the mark and the text span, never the button, so a new message doesn't drop keyboard focus.
- **D11 · Unset width fills the container.** Without `width`, the `<p>` is `width: 100%` of its container; the display page's wiring gives it the 1392px row, so it never sizes to its text.
- **D12 · Clearing drops focus.** When the message clears the button is removed and focus goes to the document; the line doesn't move focus elsewhere (there's nothing after it to take it).
- **D13 · `inert` is plain text with the same drawing.** Over a prompt the line stays readable but has nothing to interact with (Prompts.md PR-D23): the button becomes a `<span>` with the same layout and children, so the overlay's copy matches the board's `<p>` with no button pixel for pixel, and the `Inert` crop is the `Error` box. The prop is not the HTML `inert` attribute, which would also silence the live region.
- **D14 · `hidden` keeps the row and silences the copy.** The page's own line goes `aria-hidden="true"` with `visibility: hidden` (not `display: none` and not the HTML `hidden` attribute), so the 20px row and the keys under it don't move while one live region, the overlay's, speaks. `data-hidden` is the test hook, since jsdom applies no component CSS.
- **D15 · `hidden` implies inert.** A hidden line renders no button, so no focusable element sits inside `aria-hidden` (axe's `aria-hidden-focus`) and the hidden copy can't be tabbed to; it needs no separate `inert` from the wiring.
- **D16 · `data-shot-mask="status"` always.** The `<p>` carries it in every state (Prompts.md › Kit additions), so any page or overlay story can mask the line without knowing whether there's a message; this component's own crops don't mask it.
- **D17 · The ⚠ stays the svg.** Prompts.md › Kit additions says the board's 12px svg ⚠ against the kit's glyph is a mismatch where "the glyph wins"; this spec keeps D4's svg (the same drawing as the board and `PartMarks`, so the `Error` and `Inert` crops match). The kit text should name the svg; recorded for the kit's owner, not changed here.
- **D18 · No new tokens or contrast rows.** `inert` and `hidden` draw nothing new (L1).
- **D19 · Crop files.** `Inert-{dark,light}.png` (Prompts 24,800 1392×20) and `Hidden-{dark,light}.png` (Stage 24,800 1392×20) don't exist yet; the crop station cuts them from the same boxes as `Error` and `Board`.

Follow-ups: kit.md › Status line should say the clear button is the page's line only (the overlay's is inert) and that the ⚠ is the 12px svg (D17); kit.md is not this lane's to edit.

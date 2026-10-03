# StyleLine

## Identity (all stations)

- **Kind:** complex
- **Built from:** Button, AccentBlock, WaitingChip, OneTouch, BandSends
- **Build after:** Button, AccentBlock, WaitingChip, OneTouch and BandSends have landed.
- **Purpose:** The top line of the Stage's display: which style is loaded (and which is coming), steps to the previous or next style, applies a One Touch Setting, and shows the band's effect sends.
- **Boards:** `Stage-Dark.dc.html:130-144` (◀, "Sunday Drive Pop" in the accent block, ▶, "Pop · 4/4", One Touch 1–4 with 2 chosen, "Band Reverb 40 Chorus 12 Delay 0"); light: `Stage-Light.dc.html:106-120`. Crop box `Stage 49,129 814×32`. The queued style (the "→" and the accent chip) is drawn on no board; its values are Stage.md › Style line, "Queued style".
- **Not this component's job:** no store, no API, no Tauri. It doesn't resolve Track ◀ ▶'s actions, Shift or neighbours (the wiring passes `prevDisabled` / `nextDisabled` and sends the right action on `onprev` / `onnext`, Stage.md D38), look up the library entry for the folder or the queued name (the wiring passes them), or open the Browser or Effects (it calls back; Stage.md D3, D32). The faces, glyphs and focus rings are the children's; this spec is the row's layout, the category text, the queued slot, and the props each child gets.

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `name` | `string` | — | `style.name`. Empty: the accent block reads "No style" (AccentBlock's `empty`). |
| `folder` | `string \| null` | `null` | The `folder` of the library entry whose `id` is `style.id` ("Pop", "Pop/Ballad"); `null` when the entry isn't found or the library isn't loaded. |
| `timeSignature` | `[number, number]` | — | `style.timeSignature` (never null), shown as "4/4". |
| `queued` | `boolean` | `false` | A style waits for the bar line (`preview.queued` not null): the category is replaced by "→" and the queued chip. |
| `queuedName` | `string \| null` | `null` | The `name` of the library entry whose `id` is `preview.queued`; `null` (not found, library not loaded): the chip reads "next style" (Stage.md D44). |
| `prevDisabled` | `boolean` | `false` | ◀ is shown but not pressable. The wiring: with `shift`, `surface.controls[trackPrev].shiftAction === null`; without, its `action === null` or `surface.trackPrev === null` (Stage.md D38). |
| `nextDisabled` | `boolean` | `false` | ▶, the same with `trackNext`. |
| `otsCount` | `number` | `4` | `ots.settings.length`, for OneTouch's `count`. |
| `otsApplied` | `number` | `0` | `ots.applied` (1-based, 0 none), for OneTouch's `applied`. |
| `otsRacks` | `(string \| null)[]` | `[]` | `otsRackNames(ots.racks)`, for OneTouch's `racks`. |
| `bandSends` | `{ block: FxBlock; name: string; effectName: string; level: number }[]` | `[]` | `home.bandSends`, passed to BandSends (which finds each value by `block`, Stage.md D58). |
| `tipAction` | `Action<HTMLElement, string> \| undefined` | — | The app's `use:tip`, passed to every child and used on the category span (L3). |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onprev` | ◀ pressed, unless `prevDisabled` | `()` → the wiring sends `surface.controls[trackPrev].action` (`stepStyle { delta: -1 }` at the fixture), or its `shiftAction` with Shift |
| `onnext` | ▶ pressed, unless `nextDisabled` | `()` → the same with `trackNext` |
| `onbrowser` | the style name pressed | `()` → opens the Browser (`ui.browser = true`, Stage.md D3) |
| `onrecall` | a One Touch button pressed (OneTouch's `onrecall`) | `(index: number)` 0-based → `recallOts { index }` |
| `onbandsends` | the band sends pressed (BandSends' press callback) | `()` → the Effects page (interim: the Effects drawer, Stage.md D32) |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Pure functions (`app/src/ui/StyleLine/category.ts`)

- `categoryText(folder: string | null, timeSignature: [number, number]): string`: the last `/`-segment of `folder`, " · ", then "{n}/{d}"; `folder` null, empty, or with an empty last segment: the metre alone. `('Pop', [4, 4])` → "Pop · 4/4"; `('Pop/Ballad', [3, 4])` → "Ballad · 3/4"; `(null, [6, 8])` → "6/8"; `('', [4, 4])` → "4/4" (Stage.md D26, D44).

### Children

Left to right, in DOM order:

| # | Child | Props passed (from StyleLine's props) | Callback → |
|---|---|---|---|
| 1 | `Button` (◀) | `{ symbol: 'prev', size: 'icon', disabled: prevDisabled, name: 'Previous style (Track left)', tip: 'style.prev', tipAction }` | `onpress()` → `onprev()` |
| 2 | `AccentBlock` (style name) | `{ as: 'button', size: 'line', label: name, empty: 'No style', name: `${name.trim() === '' ? 'No style' : name}: open the Browser`, tip: 'browser.open', tipAction }` | `onpress()` → `onbrowser()` |
| 3 | `Button` (▶) | `{ symbol: 'next', size: 'icon', disabled: nextDisabled, name: 'Next style (Track right)', tip: 'style.next', tipAction }` | `onpress()` → `onnext()` |
| 4a | the category span (not queued) | its text `categoryText(folder, timeSignature)`; `data-tip="display.timesig"` and `use:tipAction` (D4) | — |
| 4b | the queued slot (queued): "→" then `WaitingChip` | `{ label: queuedName ?? 'next style', hue: 'a', size: 'line' }` | — |
| 5 | `OneTouch` | `{ count: otsCount, applied: otsApplied, racks: otsRacks, size: 'md', label: true, tipAction }` (its default `tips`, `ots.1`–`ots.4`) | `onrecall(index)` → `onrecall(index)` |
| 6 | `BandSends` | `{ sends: bandSends, tip: 'display.band_sends', tipAction }` (D6) | its press callback → `onbandsends()` |

### Visual rules

- **Tokens used:** `--m`, `--font-sans`, `--text-14`, `--weight-regular`, `--space-4`, `--space-8`, `--space-16`. No new tokens (L1).
- **Root:** a `div`, `width: 100%` of its container (814 in the Stage's display), height 32, `display: flex; align-items: center; gap: var(--space-8)`, `flex-wrap: nowrap`, `min-width: 0`, `font-family: var(--font-sans)`, `font-variant-numeric: tabular-nums`.
- **Fit (Stage.md D34):** every child is `flex: none` except the style name, which shrinks (`flex: 0 1 auto; min-width: 0`, AccentBlock's ellipsis). The board's content is about 820px in 814, so the name gives up the few pixels; nothing else changes size.
- **Category span:** 14 / 400 (`--text-14`, `--weight-regular`), `--m`, line-height 18, `margin-left: var(--space-4)` (12px after ▶ with the gap), `white-space: nowrap`, `flex: none`. `data-part="category"`.
- **Queued slot:** a span in the category's place, `margin-left: var(--space-4)`, `display: inline-flex; align-items: center; gap: var(--space-8)`, `flex: none`: "→" (a span, 14 / 400 `--m`, `aria-hidden="true"`), then the WaitingChip (26 tall, 14px, `--a`, at most 200 wide with an ellipsis: WaitingChip's `line` size). `data-part="queued"`.
- **One Touch:** `margin-left: auto` (pushed right), `flex: none`.
- **Band sends:** `margin-left: var(--space-16)` (24px after button 4 with the gap), `flex: none`.
- **States drawn by:** `queued` (category or queued slot); the children's own (◀ ▶ disabled, the chosen One Touch, the sends' values).
- **Type:** the category's above; the rest is the children's.
- **Contrast:** `--m` on `--g` (existing row). The children list theirs (AccentBlock `--g` on `--a`; WaitingChip `--a` on `--g`, Stage.md C6's row; Button's).
- **Motion:** none.

### Accessibility

- **Role and name:** the root is a plain `div` with no role (D3). The category span is plain text, not focusable; the queued slot's text is read as "next style" or the queued name (the arrow is `aria-hidden`), preceded by a visually hidden "Queued:" span so a screen reader hears "Queued: Another Very Long Style Name" (D5).
- **Keyboard:** Tab order is DOM order: ◀, the style name, ▶, One Touch 1–4, band sends (Stage.md D36). No keys of its own.
- **Tooltip id:** the children's (`style.prev`, `browser.open`, `style.next`, `ots.1`–`ots.4`, `display.band_sends`) and the category's `display.timesig` (exists). `display.band_sends` is new: Stage.md contract change C5.

## Stories (Story station)

Title `Components/StyleLine`. Every story renders inside a decorator `div` of `width: 814px` (the display's content width), `layout: 'centered'`. Every story renders in dark and light. The meta's `args` are the `Board` args plus `onprev: fn()`, `onnext: fn()`, `onbrowser: fn()`, `onrecall: fn()`, `onbandsends: fn()`, `tipAction: fn()`.

**Controls (argTypes):** `name`, `folder`, `queuedName` text (`folder`, `queuedName` cleared = `null`); `timeSignature`, `otsRacks`, `bandSends` `object`; `queued`, `prevDisabled`, `nextDisabled` boolean; `otsCount`, `otsApplied` numbers (0–4); the callbacks and `tipAction` actions. Grouped by child: "Track" (`prevDisabled`, `nextDisabled`), "AccentBlock" (`name`), "Category" (`folder`, `timeSignature`, `queued`, `queuedName`), "OneTouch" (`otsCount`, `otsApplied`, `otsRacks`), "BandSends" (`bandSends`).

Board sends (`boardSends`): `[{ block: 'reverb', name: 'Reverb', effectName: 'Hall 1', level: 40 }, { block: 'chorus', name: 'Chorus', effectName: 'Chorus 1', level: 12 }, { block: 'variation', name: 'Delay', effectName: 'Delay LCR', level: 0 }]`.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ name: 'Sunday Drive Pop', folder: 'Pop', timeSignature: [4, 4], otsCount: 4, otsApplied: 2, bandSends: boardSends }` | the Stage board's style line | `Board-{dark,light}.png` (Stage 49,129 814×32) | the buttons in DOM order are named "Previous style (Track left)", "Sunday Drive Pop: open the Browser", "Next style (Track right)", "One Touch 1" … "One Touch 4", then the band sends ("Band sends: reverb 40, chorus 12, delay 0. Opens Effects"); `[data-part="category"]` reads "Pop · 4/4" and has `data-tip="display.timesig"`; no `[data-part="queued"]`; "One Touch 2, applied" has `aria-pressed="true"`; click ◀ → `onprev` called once; click the style name → `onbrowser` once; click ▶ → `onnext` once; click "One Touch 3" → `onrecall` called with `2`; click the band sends → `onbandsends` once |
| `Queued` | `Board`'s args with `queued: true, queuedName: 'Another Very Long Style Name'` | "→" and the accent chip in the category's place, ellipsized at 200px | — (no board draws it) | no `[data-part="category"]`; `[data-part="queued"]` contains the text "Another Very Long Style Name" and "Queued:" |
| `QueuedUnknown` | `Board`'s args with `queued: true, queuedName: null` | the chip reads "next style" | — | the queued slot contains "next style" |
| `LongName` | `Board`'s args with `name: 'Sunday Drive Pop with a Very Long Name for the Late Set Here'` (60 characters), `queued: true, queuedName: 'Another Very Long Style Name'` | the line stays one row inside 814px; the name ends in "…"; the chip is at most 200px (Stage.md's screenshot check) | — (no board draws it; judged by Inspect) | the style name button's `aria-label` holds the whole name |
| `NoFolder` | `Board`'s args with `folder: null` | the metre alone: "4/4" | — | `[data-part="category"]` reads "4/4" |
| `AtTheEnd` | `Board`'s args with `nextDisabled: true` | the last style in the library: ▶ dimmed | — | "Next style (Track right)" has `aria-disabled="true"`; click → `onnext` not called; ◀ has no `aria-disabled` (Stage.md Check 14) |
| `ThreeOts` | `Board`'s args with `otsCount: 3, otsApplied: 0` | One Touch 4 dimmed, none chosen | — | "One Touch 4" has `aria-disabled="true"` |
| `NoSends` | `Board`'s args with `bandSends: []` | before `home` arrives: each value "–" | — | the band sends button's text contains "–" three times |
| `NoStyle` | `Board`'s args with `name: ''` | the accent block reads "No style" | — | the button named "No style: open the Browser" exists |
| `Focused` | `Board`'s args, `parameters: { pseudo: { focusVisible: true } }` | the children's focus rings | — | — |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in dark and light; the crop is the row's box (L6).

**Unit tests** (`app/src/ui/StyleLine/category.test.ts`, vitest): `categoryText` for the four examples above and `('Pop/', [4, 4])` → "4/4".

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`); `category.test.ts` passes.
- `npm run shots -- StyleLine` passes for `Board` (score at most 0.02, or the Inspect agent judges any difference render noise); `LongName` is judged by eye (one row, ellipsis, chip ≤ 200px); axe finds no violation on any story, except the pairs Stage.md C6 names until it lands.
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules (32, 18).
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · Track rules live in the wiring.** ◀ ▶ take only `prevDisabled` / `nextDisabled` and call `onprev` / `onnext`; which action (or Shift's `shiftAction`) to send and when there is no neighbour is resolved from `surface.controls` and `ui.shift` by the wiring (Stage.md D38), so the line stays free of `AppState` (D46).
- **D2 · Library lookups in the wiring.** The wiring passes `folder` and `queuedName` (looked up by id in the `library` prop); `categoryText` turns them into text here, so its fallbacks (D44) are unit-tested in one place.
- **D3 · No group.** Stage.md names each control and gives the line no role; a plain `div` keeps the tab stops and adds no landmark.
- **D4 · The category carries a tooltip without being a control.** Stage.md gives the non-control category `display.timesig`; the span gets `data-tip` and `use:tipAction` (hover only, not focusable), as the chord column does with `display.chord`.
- **D5 · The queued style is read out.** The "→" is decorative; a visually hidden "Queued:" before the chip lets a screen reader say what the outlined name means. It adds no visible text.
- **D6 · BandSends' props.** BandSends' spec is written alongside this one; this spec passes `sends` (the `home.bandSends` array), `tip: 'display.band_sends'` and `tipAction`, and maps its press callback to `onbandsends`, per Stage.md › Style line. Where BandSends' SPEC names its props or callback differently, its names win and this table follows.
- **D7 · Fit.** Only the style name shrinks (Stage.md D34); the queued chip is capped by WaitingChip's `line` size (200px), One Touch and the sends keep their size, and the line never wraps.
- **D8 · L3 throughout.** Every child gets the one `tipAction`; the keys are fixed here (`style.prev`, `browser.open`, `style.next`, `ots.1`–`ots.4` through OneTouch, `display.band_sends`, `display.timesig`); `display.band_sends` is contract change C5 (new key), the rest exist.

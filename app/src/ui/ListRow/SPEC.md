# ListRow

## Identity (all stations)

- **Kind:** primitive
- **Built from:** —
- **Purpose:** One line of a list you pick from (a Library page, a style folder): its name and how many things are in it, with the one you're on drawn as a white block.
- **Boards:**
  - `Browser-Dark.dc.html:143-148` (the Library pages: Styles chosen with "1,284", Sounds 886, Instruments 4, Racks 10, Style map with no count), `:195-200` (the style folders: Pop & Rock chosen with 212, Ballad 98, …); light: `Browser-Light.dc.html:127-132`, `:179-184`.
  - Specified in `docs/specs/push/Browser.md` › Kit additions › Library frame › Library pages, and › Styles page › Folders (BR-D7, BR-D25).
- **Not this component's job:** no store, no API, no Tauri. It doesn't know what it opens: the parent passes `label`, `count` and `current` and acts on `onpress`. It doesn't scroll itself into view (the folder list does), doesn't draw the list around it (LibraryPages, FolderList) and has no hover look (kit D35). The tooltip is the parent's key passed as `tip`, wired by the parent's `tipAction` (L3). Not a tab (ChosenTabs) and not a listbox option (the style rows are StyleRow).

Its users:

| User | `label` | `count` | `current` | `name` | `tip` |
|---|---|---|---|---|---|
| LibraryPages | "Styles", "Sounds", "Instruments", "Racks", "Style map" | `library.count`, `soundLibrary.patches.length`, `plugins.list.length`, `racks.length`; none for Style map | `'page'` on the page shown | default ("Sounds, 886") | `library.tab_styles` (new, C1), `library.tab_sounds`, `library.tab_instruments`, `library.tab_racks`, `library.tab_map` |
| FolderList (Browser) | the top-level folder's name, or "Library root" | entries in it and its subfolders | `true` on the chosen category's folder | "{name}, {count} styles" | `browser.folder` |

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `label` | `string` | — | The name on the left ("Styles", "Pop & Rock"). |
| `count` | `number \| undefined` | — | The count on the right, formatted with `count.toLocaleString('en-US')` ("1,284"; BR-D25: never the machine's locale). Undefined: no count element at all (Style map). |
| `current` | `boolean \| 'page'` | `false` | This row is the one chosen: drawn as the chosen block, with `aria-current="page"` (`'page'`, the Library pages) or `aria-current="true"` (`true`, a folder). `false`: no `aria-current` attribute and the off look. |
| `name` | `string \| undefined` | — | The accessible name. Default: the label, then ", " and the formatted count when there is one ("Sounds, 886"; "Style map"). |
| `width` | `number \| undefined` | — | A fixed width in px (the board's 320 and 200), set as an inline `width: <n>px`. Default: fills its container (`width: 100%`). |
| `tip` | `string \| undefined` | — | The tooltip key, rendered as `data-tip` on the `<button>`; no attribute when undefined (L3). |
| `tipAction` | `Action<HTMLElement, string> \| undefined` | — | The app's `use:tip`, passed in by the wiring; applied as `use:tipAction={tip}` only when both it and `tip` are set (L3). |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onpress` | click, Space or Enter (the native button's click), also on the current row (D3) | `()` |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Visual rules

- **Tokens used:** `--g`, `--t`, `--t2`, `--m`, `--line`, `--sel-mut` (new, Browser.md › Kit additions › Tokens), `--focus`, `--focus-offset`, `--line-width`, `--control-height`, `--font-sans`, `--text-13`, `--text-14`, `--weight-light`, `--weight-regular`, `--weight-medium`, `--space-8`, `--space-12`.

#### New tokens

Not in `app/src/ui/tokens/*` today; it lands in the orchestrator's tokens contract PR before this component is built (L1). Browser.md › Kit additions › Tokens defines it; this spec only uses it.

| Token | Dark | Light | Used for |
|---|---|---|---|
| `--sel-mut` | `color-mix(in srgb, var(--g) 60%, transparent)` | `color-mix(in srgb, var(--g) 70%, transparent)` | the count on the chosen block |

- **Box:** a native `<button type="button">`, `box-sizing: border-box`, height `--control-height` (32, the hairline included), width `width` px or 100%, `margin: 0`, no radius, `border: 0; border-bottom: var(--line-width) solid var(--line)`, padding `0 var(--space-8)`, `display: flex`, `align-items: center`, `justify-content: flex-start`, gap `--space-12`, `white-space: nowrap`, `text-align: left`, `font-family: var(--font-sans)`, `background: transparent`, `cursor: pointer`.
- **Name:** a `span`, `flex: 1 1 auto`, `min-width: 0`, `overflow: hidden`, `text-overflow: ellipsis`, `--text-14`, line-height normal.
- **Count:** a `span`, `flex: none`, `--text-13`, `--weight-light` (300), tabular numerals.
- **States drawn by** (`data-face` on the `<button>`):
  - off (`off`): no fill; name `--t2` at `--weight-regular`; count `--m`; bottom border `--line`.
  - chosen (`chosen`, `current` true or `'page'`): `background: var(--t)`; name `--g` at `--weight-medium` (500); count `--sel-mut`; bottom border `--t`, so the block is the full 32px.
  - keyboard focus (`:focus-visible`): a `--line-width` outline drawn **inside** the row, `outline-offset: calc(-1 * var(--focus-offset))`, in `--focus` on the off face and in `--g` on the chosen face (D2). Nothing on mouse focus.
  - No hover or pressed look (kit D35); no disabled state (no board row is disabled).
- **Type:** DM Sans, sentence case as given; name 14, count 13 / 300 with tabular numerals.
- **Contrast (AA 4.5:1, `tokens/contrast.test.ts`):** existing rows `--t2` on `--g`, `--m` on `--g`; `--g` on `--t` is Button's new row ("chosen label (Button)": 21:1 dark, 16.72:1 light). New row: `--sel-mut` on `--t` ("chosen row count (ListRow)"): 5.74:1 dark (`#666666` on `#ffffff`), 8.49:1 light; passes. `--sel-mut` is a `color-mix`, which today's test can't resolve: the tokens contract PR adds it as `['--g', '--t', 'chosen row count (ListRow)', <opacity>]` with an opacity token per theme (0.6 dark, 0.7 light), or teaches the resolver `color-mix`; the PR's call (D5).
- **Motion:** none.

### Accessibility

- **Role and name:** a `button`; its accessible name is `name`, or the default above, set as `aria-label`. `aria-current="page"` or `"true"` per `current`, absent when not current. No `aria-pressed` (it navigates, it doesn't toggle). `data-tip` only with `tip`.
- **Keyboard:** Tab focuses it; Space or Enter calls `onpress`. No arrow keys of its own (the lists are plain button columns, in the tab order).
- **Tooltip id:** the parent's key passed as `tip` (`library.tab_sounds`, `browser.folder`, …), with `tipAction` (L3).

## Stories (Story station)

Title `Primitives/ListRow`, `layout: 'centered'`. Every story renders in dark and light. Meta `args`: `{ onpress: fn(), tipAction: fn() }` (axiom 7, L3).

**Controls (argTypes):** `label`, `name`, `tip` text; `count` and `width` numbers (cleared = undefined); `current` a select of `false` / `true` / `page`, mapped to `false` / `true` / `'page'`; `onpress`, `tipAction` actions.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ label: 'Styles', count: 1284, current: 'page', width: 320, tip: 'library.tab_styles' }` | the first ListRow on the Browser board: the chosen Styles page, a white block, "1,284" in `--sel-mut` | `Board-{dark,light}.png` (Browser 48,228 320×32) | the button named "Styles, 1,284" has `aria-current="page"`, `data-face="chosen"`, `data-tip="library.tab_styles"`; `tipAction` was called with it and `'library.tab_styles'`; click → `onpress` called once |
| `Off` | `{ label: 'Sounds', count: 886, width: 320, tip: 'library.tab_sounds' }` | an off row: `--t2` name, `--m` count, the hairline | `Off-{dark,light}.png` (Browser 48,260 320×32) | `data-face="off"`; no `aria-current` attribute; the name is "Sounds, 886"; Space, then Enter → `onpress` called 2 times |
| `NoCount` | `{ label: 'Style map', width: 320, tip: 'library.tab_map' }` | a row with no count | `NoCount-{dark,light}.png` (Browser 48,356 320×32) | the button's accessible name is exactly "Style map"; it has one text child span |
| `Folder` | `{ label: 'Pop & Rock', count: 212, current: true, name: 'Pop & Rock, 212 styles', width: 200, tip: 'browser.folder' }` | the chosen style folder at 200 wide | `Folder-{dark,light}.png` (Browser 417,176 200×32) | `aria-current="true"`; `data-face="chosen"` |
| `FolderOff` | `{ label: 'Ballad', count: 98, name: 'Ballad, 98 styles', width: 200, tip: 'browser.folder' }` | an off folder | `FolderOff-{dark,light}.png` (Browser 417,208 200×32) | `data-face="off"` |
| `LongName` | `{ label: 'Swing & Jazz Big Band Classics', count: 1284, width: 200 }` | the name ends in an ellipsis; the count stays whole | — (not drawn) | the row is 32 tall (`getBoundingClientRect` is not checked in jsdom; judged by Inspect) |
| `Focused` | `Off` args, `pseudo: { focusVisible: true }` | the inset focus ring in `--focus` | — | — |
| `FocusedChosen` | `Board` args, `pseudo: { focusVisible: true }` | the inset focus ring in `--g` on the white block | — | — |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in dark and light; each crop is exactly the row's own box (L6). The board's folder order is placeholder data (BR-D7), but the Ballad row at 417,208 is a true picture of an off folder row.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- ListRow` passes for the five cropped stories (score at most 0.02, or the Inspect agent judges the difference render noise), and axe finds no violation on any story once the tokens contract PR (`--sel-mut`) has landed.
- Only listed tokens are used; no inline colours, no literal sizes (the inline `width` is the prop's).
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · One row for pages and folders.** The Library pages and the style folders draw the same 32px hairline row (Browser.md row 20), so one primitive serves both; only `aria-current`'s value (`page` for pages, `true` for a folder), the width and the accessible name differ, and those are props.
- **D2 · Inset focus ring.** The ring is drawn inside the row (`outline-offset` negative) because the folder list scrolls and would clip an outside ring, and rows touch with no gap; on the chosen block it is `--g`, since `--focus` is the block's own colour.
- **D3 · The current row still presses.** A click on the current row calls `onpress` like any other; the parent decides it's a no-op. Keeping it a live button keeps the list's tab order and names stable.
- **D4 · The count is formatted here.** `count` is a number and the row formats it with `toLocaleString('en-US')` (BR-D25), so every user shows "1,284" alike and tests don't follow the machine's locale.
- **D5 · L1, `--sel-mut`.** The token is Browser.md's (Kit additions › Tokens); it and the new contrast row land in the tokens contract PR, which decides how the test resolves its `color-mix` (an opacity row or a resolver change).
- **D6 · L3, tooltip props.** `tip` (`data-tip`) and `tipAction` (applied when both are set). `library.tab_styles` is not in `tooltips.ts` today: contract change needed (Browser.md C1); every other key the stories use exists.
- **D7 · L5.** ListRow has no disabled state, so it never renders `aria-disabled`.
- **D8 · No crop files yet.** The folder holds only this spec; the crops above are cut later from `docs/design/push/png/Browser-Dark.png` and `Browser-Light.png` (L6).

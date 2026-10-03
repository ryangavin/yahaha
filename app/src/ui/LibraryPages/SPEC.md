# LibraryPages

## Identity (all stations)

- **Kind:** complex
- **Built from:** ListRow
- **Purpose:** The list of Library pages (Styles, Sounds, Instruments, Racks, Style map) with how much each holds; click one to go there.
- **Boards:** `Browser-Dark.dc.html:142-149` (the nav: Styles chosen "1,284", Sounds 886, Instruments 4, Racks 10, Style map); light: `Browser-Light.dc.html:126-133`. Specified in `docs/specs/push/Browser.md` › Kit additions › Library frame › Library pages (BR-D18, BR-D25).
- **Not this component's job:** no store, no API, no Tauri. It doesn't set `ui.libraryTab` or host the page (the wiring does, BR-D18); it reads no `AppState`: the counts come in as numbers. No shortcuts of its own (Alt+S and Alt+Y are `lib/shortcuts.ts`'s). The tooltips are fixed keys per row, wired by the `tipAction` the parent passes (L3).

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `current` | `LibraryPage` | — | The page shown: `'styles' \| 'sounds' \| 'instruments' \| 'racks' \| 'map'`. The type is exported from this folder (`LibraryPages.svelte`'s module script or `types.ts`), the same values as `LibraryTab` in `app/src/lib/store.svelte.ts` once C2 adds `'styles'` (D2). |
| `counts` | `{ styles: number; sounds: number; instruments: number; racks: number }` | — | The counts, read by the wiring: `library.count`, `soundLibrary.patches.length`, `plugins.list.length`, `racks.length` (BR-D25). Style map has none. |
| `width` | `number \| undefined` | — | A fixed width in px (320), inline `width: <n>px`; default fills its container. |
| `tipAction` | `Action<HTMLElement, string> \| undefined` | — | The app's `use:tip`, passed to every row (L3). |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onchoose` | a row's `onpress` (click, Space, Enter), the current row included (ListRow D3) | `(page: LibraryPage)` |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Children and the props passed

A `<nav aria-label="Library pages">`, a column (`display: flex; flex-direction: column`), no gap, no padding, width `width` px or 100%. Five ListRows in this order, each `{ label, count, current, tip, tipAction, onpress: () => onchoose(page) }` with no `width` (they fill the nav) and no `name` (ListRow's default "{label}, {count}"):

| Page | `label` | `count` | `tip` |
|---|---|---|---|
| `styles` | "Styles" | `counts.styles` | `library.tab_styles` (new, C1) |
| `sounds` | "Sounds" | `counts.sounds` | `library.tab_sounds` |
| `instruments` | "Instruments" | `counts.instruments` | `library.tab_instruments` |
| `racks` | "Racks" | `counts.racks` | `library.tab_racks` |
| `map` | "Style map" | — | `library.tab_map` |

`current` is `'page'` on the row whose page is `current`, `false` on the others. Five rows at 32 = 160 tall.

### Visual rules

- **Tokens used:** none of its own beyond layout (`--line` etc. are ListRow's). No new tokens, no new contrast rows (ListRow's cover every text, L1).
- **States drawn by:** only `current` (which row is the white block) and the counts.
- **Motion:** none.

### Accessibility

- **Role and name:** a `nav` landmark named "Library pages"; each row a button with `aria-current="page"` on the page shown (ListRow).
- **Keyboard:** each row is a tab stop in order; Space or Enter chooses. No roving focus (five rows; Browser.md's tab order lists them as plain stops).
- **Tooltip id:** per row, above.

## Stories (Story station)

Title `Components/LibraryPages`, `layout: 'centered'`. Every story renders in dark and light. Fixtures (axiom 12): `app/src/ui/LibraryPages/LibraryPages.fixtures.ts` exports `boardCounts = { styles: 1284, sounds: 886, instruments: 4, racks: 10 }` (Browser.md › Board fixture). Meta `args`: `{ counts: boardCounts, width: 320, onchoose: fn(), tipAction: fn() }`.

**Controls (argTypes):** `current` a select of the five pages; `counts` an object control; `width` a number; `onchoose`, `tipAction` actions. ListRow has no per-row args to forward (every row's props come from `current` and `counts`).

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ current: 'styles' }` | the Browser board's list: Styles chosen, the four other rows off | `Board-{dark,light}.png` (Browser 48,228 320×160) | a navigation named "Library pages" holds five buttons, named in order "Styles, 1,284", "Sounds, 886", "Instruments, 4", "Racks, 10", "Style map"; the first has `aria-current="page"` and `data-face="chosen"`, the others no `aria-current`; their `data-tip`s are the five keys above; click "Sounds, 886" → `onchoose` called once with `'sounds'`; click "Style map" → called with `'map'` |
| `Sounds` | `{ current: 'sounds' }` | the Sounds page chosen | — (no board draws this frame on another page, D3) | "Sounds, 886" has `aria-current="page"`; "Styles, 1,284" has none |
| `Empty` | `{ current: 'styles', counts: { styles: 0, sounds: 0, instruments: 0, racks: 0 } }` | an empty library: every count "0" | — | the first button is named "Styles, 0" |
| `Large` | `{ current: 'map', counts: { styles: 60000, sounds: 12480, instruments: 37, racks: 215 } }` | big counts with separators; Style map chosen with no count | — | "Styles, 60,000" and "Sounds, 12,480" are present |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in dark and light (L6).

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- LibraryPages` passes for `Board` (score at most 0.02, or the Inspect agent judges the difference render noise), and axe finds no violation on any story once the tokens contract PR (`--sel-mut`) has landed.
- `library.tab_styles` is in `tooltips.ts` (C1) before the build, or the tooltip catalog test fails.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · Counts as numbers, not the state.** The component takes four numbers rather than `AppState`, so it stays a pure list and the wiring reads `library.count`, `soundLibrary.patches.length`, `plugins.list.length` and `racks.length` (BR-D25).
- **D2 · Its own page type.** The library can't import `app/src/lib/store.svelte.ts`, so `LibraryPage` is declared here with `LibraryTab`'s values plus `'styles'`; C2 (Browser.md) adds `'styles'` to `LibraryTab`, after which the wiring passes `ui.libraryTab` straight in (the two types must stay equal; the wiring's type check enforces it).
- **D3 · Only the Browser board's frame is cropped.** The other Library boards (`LibraryRacks`, `LibrarySounds`, `LibraryInstruments`, `StyleMap`) draw an older frame (the pages as a tab row across the top, the slots in two columns); Browser.md says every Library page keeps this frame (Variants of this screen), so the Browser board is the reference and those boards' specs (#515–#518) follow it.
- **D4 · No roving focus.** Five buttons in the tab order, as Browser.md's tab order lists them; a `nav` of links-like buttons, not a tablist (the page content isn't a tabpanel of this list).
- **D5 · L3, tooltips.** Each row gets its fixed key and the shared `tipAction`; `library.tab_styles` is a contract change needed (Browser.md C1), the other four exist.
- **D6 · No crop files yet.** The folder holds only this spec; the crop is cut later from the Browser renders (L6).

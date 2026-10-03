# PageTabs

## Identity (all stations)

- **Kind:** complex
- **Built from:** ChosenTabs, twice (`size: 'page'`): pages 1–7, then pages 8–9, inside PageTabs' own `nav` with its own separator between them (D1)
- **Purpose:** Shows which page you're on and takes you to another one: Stage, Channel, Effects, Quick Racks, Multi Pads, Looper, Harm/Arp, then Library and Settings.
- **Boards:** `Stage-Dark.dc.html:74-85` (the `nav`, Stage chosen); light: `Stage-Light.dc.html:50-61`. The page boards draw the same tabs with another one chosen, inside the page variant of the app bar (Settings chosen: `SettingsSystem-Dark.dc.html:63-74`, light `SettingsSystem-Light.dc.html:52-63`; Library chosen: `LibrarySounds-Dark.dc.html:73-84`, light `LibrarySounds-Light.dc.html:60-71`).
- **Not this component's job:** no store, no API, no Tauri, no import from `app/src/lib`. It doesn't know which page is open: the parent passes `chosen`. It doesn't open anything: a choice calls `onchoose(id)` and the page wiring opens the page, or today's drawer until that page exists (table below; Stage.md D32). It doesn't handle or announce shortcuts (the window key handler, `app/src/lib/keys.ts` and `NAV` in `app/src/lib/nav.ts`, owns the Alt keys; the table's Shortcut column is information for the wiring and the tooltips' `app_keys`, D8). It doesn't decide what choosing the page already on view means (the wiring ignores it, D6). It doesn't draw the app bar's 1px bottom line, the wordmark or the right area (AppBar's). Every tab's look, focus and keyboard behaviour are ChosenTabs' and aren't restated here; the `nav` and the separator are PageTabs' own (Visual rules).

## API (Component station)

### The page list (`pages.ts`)

`app/src/ui/PageTabs/pages.ts` exports (importing only the `TabItem` type from `app/src/ui/ChosenTabs/types.ts`):

```ts
export type PageId =
  | 'stage' | 'channel' | 'effects' | 'quickRacks' | 'multiPads' | 'looper' | 'harmArp'
  | 'library' | 'settings'

export type PageTab = TabItem & { id: PageId }

/** The first run, before the separator: rows 1–7 of the table below. */
export const DISPLAY_PAGES: readonly PageTab[]

/** The second run, after the separator: rows 8–9. */
export const FULL_PAGES: readonly PageTab[]

/** All nine, left to right: `[...DISPLAY_PAGES, ...FULL_PAGES]`. */
export const PAGES: readonly PageTab[]
```

`PageId` is `ui.page`'s type (Stage.md D2) without `rack`, which has no tab. The app may import `PAGES`, the two runs and `PageId` from here; this folder never imports from the app.

### The tabs

In this order, left to right. Each row is one `TabItem` with exactly the fields `id`, `label` and `tip`: no `name` (the accessible name is the label, D4) and no `disabled` (every tab is always choosable: until its page exists it opens today's equivalent, D32). Rows 1–7 are `DISPLAY_PAGES`, rows 8–9 `FULL_PAGES`. The Shortcut column is not a `TabItem` field and isn't passed to ChosenTabs: it is information for the wiring (`keys.ts`, `NAV`) and for each tooltip's `app_keys` (D8). "Interim" is what the page wiring does on `onchoose(id)` until that page's spec is built, and when it passes the tab as `chosen` meanwhile (Stage.md D32; the wiring's, not this component's).

| # | `label` | `id` | `tip` | Shortcut (information only) | Interim on `onchoose(id)` | Interim: `chosen` is this tab while |
|---|---|---|---|---|---|---|
| 1 | Stage | `stage` | `view.stage` | `Alt+G` (new, Stage.md D37; Contract change C5 adds it to `view.stage`) | close every open drawer, the Browser and the Channel view; `ui.view = 'stage'` (D2) | no drawer, no Browser and no Channel view is open, and `ui.view` is `stage` |
| 2 | Channel | `channel` | `nav.channel` (new, Contract change C5) | `Alt+N` (new, D37) | today's `ChannelView` for the selected part in the display's place: `panels/channel/nav.svelte.ts` `show(ui.selectedPart)` | that Channel view is open |
| 3 | Effects | `effects` | `nav.effects` | `Alt+E` | `NAV` Effects `toggle()` (`ui.toggleDrawer('effects')`) | `ui.effects` |
| 4 | Quick Racks | `quickRacks` | `nav.quick` | `Alt+R` | `NAV` Quick Racks `toggle()` (Library on its Racks tab) | `ui.view` is `library` and `ui.libraryTab` is `racks` |
| 5 | Multi Pads | `multiPads` | `nav.multipad` | `Alt+P` | `ui.toggleDrawer('multipad')` | `ui.multipad` |
| 6 | Looper | `looper` | `nav.looper` | `Alt+L` | `ui.toggleDrawer('looper')` | `ui.looper` |
| 7 | Harm/Arp | `harmArp` | `nav.harmony` | `Alt+H` | `ui.toggleDrawer('harmony')` | `ui.harmony` |
| 8 | Library (first of `FULL_PAGES`, after the separator) | `library` | `view.library` | `Alt+B` | `NAV` Library `toggle()` (`toggleLibrary()`) | `ui.view` is `library` (when Quick Racks isn't the match, D3) |
| 9 | Settings | `settings` | `nav.settings` | `Alt+T` | `ui.toggleDrawer('settings')` | `ui.settings` |

The interim `chosen` is the first row, in table order, whose condition holds, else `null` (for example while the Browser, the Rack drawer or Charts is open: none of them has a tab). Once a page's spec lands, its row becomes `ui.page = id`, and `chosen` is `ui.page`. In both, an `onchoose(id)` whose `id` is the page already on view (`id === chosen`) does nothing in the wiring: a tab never closes its own page or drawer (D6). Launchkey mapping: none (no Launchkey control picks a page).

### Props

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `chosen` | `PageId \| null` | `null` | The page on view: that tab is drawn chosen and carries `aria-current="page"`. `null`: every tab is off. |
| `tipAction` | `Action<HTMLElement, string> \| undefined` (`svelte/action`) | — | The app's tooltip action (`use:tip`), passed in by the wiring because the library can't import it; handed to ChosenTabs, which applies it to each tab with that tab's `tip`. Without it each tab still carries its `data-tip`. |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onchoose` | either ChosenTabs' `onchoose`, passed through unfiltered: a click, or Space or Enter, on any tab, the chosen one included (ChosenTabs D6; PageTabs D6) | `(id: PageId)` that tab's `id` |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Children

Inside PageTabs' root `nav`, in this order:

| Order | Element / child | Props passed |
|---|---|---|
| 1 | `ChosenTabs` | `{ tabs: DISPLAY_PAGES, chosen, size: 'page', tipAction, onchoose }` |
| 2 | separator | PageTabs' own `span`, `aria-hidden="true"` (Visual rules) |
| 3 | `ChosenTabs` | `{ tabs: FULL_PAGES, chosen, size: 'page', tipAction, onchoose }` |

Both runs get the same `chosen`, `tipAction` and `onchoose`, passed through unchanged: a run whose tabs don't include `chosen` draws every tab off (ChosenTabs' `chosen` names an id it may not hold). No `label` (ChosenTabs ignores it at `page`; the `nav` carries the name). `onchoose`'s `string` id is always a `PageId`, since every tab comes from `pages.ts`.

The box is the `nav`'s: 36 tall (`--bar-height`), as wide as the first run, the separator with its two 8px margins and the second run (about 685px at the board's font; not a whole number of pixels).

### Visual rules

- **Tokens used:** `--line`, `--line-width`, `--space-8`, `--separator-height` (new, 16px, D7), `--bar-height` (new, 36px, D7). The tabs themselves use ChosenTabs' at `size: 'page'` (its Visual rules: 36px tabs, `10px 10px 0` padding, 14 / 400 labels in `--m`, the 24px `--t` block with a `--g` label on the chosen tab, the `--focus` ring).
- **Root:** `<nav aria-label="Pages">`, `display: flex; align-items: stretch; flex: none`, height `--bar-height`, no padding, no gap, no background, no border (D9).
- **Separator:** a `span`, `aria-hidden="true"`, `flex: none`, `align-self: center`, width `--line-width` (1px), height `--separator-height` (16px), `background: var(--line)`, `margin: 0 var(--space-8)`. Never changes.
- **Size:** the content's (above). It never wraps and never shrinks.
- **States drawn by:** the chosen tab (one of nine, or none), and keyboard focus on a tab: both ChosenTabs'.
- **The bottom line:** PageTabs draws no line; the chosen block reaches the bottom of its 36px box, and AppBar draws its 1px `--t` line on that same pixel row (AppBar D2), so the block stands on the line.
- **Type:** ChosenTabs' (DM Sans 14 / 400, tabular numerals).
- **Contrast:** nothing of its own; ChosenTabs lists `--m` on `--g` and `--g` on `--t`.
- **Motion:** none.

### Accessibility

- **Role and name:** a `nav` landmark named "Pages" (PageTabs' own `aria-label`) holding nine `button`s named by their labels, "Stage" … "Settings" (the two runs are plain `div`s, ChosenTabs' `page` size; the separator is `aria-hidden`); the chosen one has `aria-current="page"`. The Alt keys aren't announced (no `aria-keyshortcuts`, D8). Not a `tablist`: these change the page (ChosenTabs D1).
- **Keyboard:** every tab is a Tab stop, left to right (kit › Interaction conventions: the app bar's tabs come first); Space or Enter chooses it. No arrow keys (the window's ← → keep stepping the style).
- **Tooltip id:** per tab, the `tip` column above, as `data-tip` on each button and through `tipAction` when given.

## Stories (Story station)

Title `Components/PageTabs`, `layout: 'centered'`. Every story renders in dark and light (the toolbar theme). The meta's args: `{ chosen: 'stage', onchoose: fn(), tipAction: fn() }` (`fn()` as the action: the story test counts it as an action, and a spy returning nothing is a valid Svelte action). Controls: `chosen`, a select with `options: ['none', ...PAGES.map((p) => p.id)]` and `mapping: { none: null }`.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ chosen: 'stage' }` | the Stage board's tabs: Stage in the white block, the rest grey, the separator before Library | — (D5) | the `navigation` named "Pages" holds 9 buttons named, in order, Stage, Channel, Effects, Quick Racks, Multi Pads, Looper, Harm/Arp, Library, Settings; the nav's element children are, in order, a `div` with 7 buttons, a `span` with `aria-hidden="true"` and no text, and a `div` with 2 buttons; "Stage" has `aria-current="page"` and `data-face="chosen"`; the other 8 buttons have `data-face="off"` and no `aria-current`; their `data-tip`s are, in order, `view.stage`, `nav.channel`, `nav.effects`, `nav.quick`, `nav.multipad`, `nav.looper`, `nav.harmony`, `view.library`, `nav.settings`; `tipAction` was called 9 times |
| `ChannelChosen` | `{ chosen: 'channel' }` | a display tab other than Stage chosen | — | "Channel" has `aria-current="page"`; no other button has it |
| `LibraryChosen` | `{ chosen: 'library' }` | the first tab past the separator chosen | — (D5) | "Library" has `aria-current="page"`; no other button has it |
| `SettingsChosen` | `{ chosen: 'settings' }` | the last tab chosen, its block at the right end | — (D5) | "Settings" has `aria-current="page"`; no other button has it |
| `NoneChosen` | `{ chosen: null }` | every tab grey (a drawer without a tab is open) | — | no button has `aria-current`; no element has `data-face="chosen"` |
| `Chooses` | `{ chosen: 'stage' }` | — | — | click "Effects" → `onchoose` last called with `'effects'`; focus "Settings", press Enter → last called with `'settings'`; focus "Looper", press Space → last called with `'looper'`; click "Stage" (the chosen tab) → last called with `'stage'` (D6); `onchoose` called 4 times; "Stage" still has `aria-current="page"` (the parent owns `chosen`) |
| `Focused` | `{ chosen: 'stage' }`, `parameters: { pseudo: { focusVisible: ['[aria-current="page"]'] } }` | the focus ring around the chosen Stage tab | — | — |

Crop positions would be `board x,y w×h` in the 1440×900 renders; PageTabs has none (D5).

**Vitest, pure** (`app/src/ui/PageTabs/pages.test.ts`): `DISPLAY_PAGES` has 7 items and `FULL_PAGES` 2, and `PAGES` equals `[...DISPLAY_PAGES, ...FULL_PAGES]`; the nine items' `id`, `label` and `tip` are the table's rows in order; every item's keys are exactly `id`, `label`, `tip` (no `name`, `disabled` or shortcut); ids are unique.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`); `pages.test.ts` passes.
- `npm run shots -- PageTabs` finds no axe violation on any story. No story has a crop; the tabs' pixels are checked by `npm run shots -- AppBar` (`Board`) and `npm run shots -- ChosenTabs` (`Board`).
- Only listed tokens are used (`--line`, `--line-width`, `--space-8`, `--separator-height`, `--bar-height`); no inline colours, no literal sizes. The two new tokens come with the orchestrator's tokens contract PR (D7); this component's PR doesn't edit `scale.css`.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · Two runs and a separator.** ChosenTabs holds only tabs (its D8), so PageTabs draws the `nav`, the first run (`DISPLAY_PAGES`), its own separator and the second run (`FULL_PAGES`), passing the same `chosen` to both runs.
- **D2 · Stage's interim action.** kit › App bar says when Stage is chosen but not what clicking it does: it closes every open drawer, the Browser and the Channel view and sets `ui.view = 'stage'`, so Stage is chosen afterwards.
- **D3 · Interim precedence.** While drawers stand in for pages, `chosen` is the first tab in table order whose condition holds, so Library on its Racks tab reads as Quick Racks, not Library; with no match it is `null`.
- **D4 · Labels as names.** Each tab's accessible name is its label; the page boards' `aria-label="Back to stage"` on an unchosen Stage tab isn't used, so the name stays the same on every page.
- **D5 · No crop of its own.** The board box `527,24 685×36` has no whole-pixel width (the tabs are text widths) and its bottom row (y 59) is the app bar's `--t` line, which PageTabs doesn't draw, so a PageTabs shot would differ there by about 0.025 of the crop; the same pixels are checked in AppBar's `Board` crop, where the line is drawn, and by ChosenTabs' `Board` crop (with its bottom row judged by Inspect).
- **D6 · Re-choosing passes through; the wiring ignores it.** PageTabs passes every `onchoose` through, the chosen tab included (ChosenTabs D6), and the wiring ignores a choose of the page already on view, so a tab never closes its own page or drawer; to close a drawer the player picks another tab, presses its Alt key again or presses Esc, as today.
- **D7 · New scale tokens.** `--bar-height: 36px`, `--separator-height: 16px`, `--app-bar-right: 196px` and `--tracking-wordmark: -0.2px` land in `app/src/ui/tokens/scale.css` with the orchestrator's tokens contract PR, since the UI-library contract files aren't this lane's; PageTabs uses `--bar-height` and `--separator-height`, AppBar all four, and ChosenTabs reads `--bar-height`.
- **D8 · No `aria-keyshortcuts`.** ChosenTabs' `TabItem` has no `shortcut` field, so the tabs don't announce their Alt keys; the table's Shortcut column stays as information for the wiring and the tooltips' `app_keys`, and announcing them is a follow-up (below).
- **D9 · PageTabs owns the `nav`.** ChosenTabs at `page` renders a plain `div` and leaves the landmark to its parent, so PageTabs' root is its own `<nav aria-label="Pages">` (flex row, stretched, `--bar-height` tall) holding both runs and the separator.

### Follow-ups

- **ChosenTabs `shortcut` → `aria-keyshortcuts`** (a request to the primitives lane): an optional `TabItem.shortcut` that ChosenTabs writes as the button's `aria-keyshortcuts`; once it lands, `pages.ts` adds the Shortcut column's values and `Board` checks them.

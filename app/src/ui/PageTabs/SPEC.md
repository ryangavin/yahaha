# PageTabs

## Identity (all stations)

- **Kind:** complex
- **Built from:** ChosenTabs (`kind: 'nav'`, `size: 'page'`)
- **Purpose:** Shows which page you're on and takes you to another one: Stage, Channel, Effects, Quick Racks, Multi Pads, Looper, Harm/Arp, then Library and Settings.
- **Boards:** `Stage-Dark.dc.html:74-85` (the `nav`, Stage chosen); light: `Stage-Light.dc.html:50-61`. The page boards draw the same tabs with another one chosen, inside the page variant of the app bar (Settings chosen: `SettingsSystem-Dark.dc.html:63-74`, light `SettingsSystem-Light.dc.html:52-63`; Library chosen: `LibrarySounds-Dark.dc.html:73-84`, light `LibrarySounds-Light.dc.html:60-71`).
- **Not this component's job:** no store, no API, no Tauri, no import from `app/src/lib`. It doesn't know which page is open: the parent passes `chosen`. It doesn't open anything: a choice calls `onchoose(id)` and the page wiring opens the page, or today's drawer until that page exists (table below; Stage.md D32). It doesn't handle shortcuts (the window key handler, `app/src/lib/keys.ts` and `NAV` in `app/src/lib/nav.ts`, owns the Alt keys; the tabs only announce them). It doesn't draw the app bar's 1px bottom line, the wordmark or the right area (AppBar's). Every tab's look, the separator's look, focus and keyboard behaviour are ChosenTabs' and aren't restated here.

## API (Component station)

### The page list (`pages.ts`)

`app/src/ui/PageTabs/pages.ts` exports (importing only the `TabItem` type from `app/src/ui/ChosenTabs/types.ts`):

```ts
export type PageId =
  | 'stage' | 'channel' | 'effects' | 'quickRacks' | 'multiPads' | 'looper' | 'harmArp'
  | 'library' | 'settings'

/** The nine page tabs, left to right: the rows of the table below. */
export const PAGES: readonly (TabItem & { id: PageId })[]
```

`PageId` is `ui.page`'s type (Stage.md D2) without `rack`, which has no tab. The app may import `PAGES` and `PageId` from here; this folder never imports from the app.

### The tabs

In this order, left to right. Each row is one `TabItem` in `PAGES`: `id`, `label`, `tip`, `shortcut`, and `separatorBefore: true` on Library only; no `name` (the accessible name is the label, D4) and no `disabled` (every tab is always choosable: until its page exists it opens today's equivalent, D32). "Interim" is what the page wiring does on `onchoose(id)` until that page's spec is built, and when it passes the tab as `chosen` meanwhile (Stage.md D32; the wiring's, not this component's).

| # | `label` | `id` | `tip` | `shortcut` | Interim on `onchoose(id)` | Interim: `chosen` is this tab while |
|---|---|---|---|---|---|---|
| 1 | Stage | `stage` | `view.stage` | `Alt+G` (new, Stage.md D37; Contract change C5 adds it to `view.stage`) | close every open drawer, the Browser and the Channel view; `ui.view = 'stage'` (D2) | no drawer, no Browser and no Channel view is open, and `ui.view` is `stage` |
| 2 | Channel | `channel` | `nav.channel` (new, Contract change C5) | `Alt+N` (new, D37) | today's `ChannelView` for the selected part in the display's place: `panels/channel/nav.svelte.ts` `show(ui.selectedPart)` | that Channel view is open |
| 3 | Effects | `effects` | `nav.effects` | `Alt+E` | `NAV` Effects `toggle()` (`ui.toggleDrawer('effects')`) | `ui.effects` |
| 4 | Quick Racks | `quickRacks` | `nav.quick` | `Alt+R` | `NAV` Quick Racks `toggle()` (Library on its Racks tab) | `ui.view` is `library` and `ui.libraryTab` is `racks` |
| 5 | Multi Pads | `multiPads` | `nav.multipad` | `Alt+P` | `ui.toggleDrawer('multipad')` | `ui.multipad` |
| 6 | Looper | `looper` | `nav.looper` | `Alt+L` | `ui.toggleDrawer('looper')` | `ui.looper` |
| 7 | Harm/Arp | `harmArp` | `nav.harmony` | `Alt+H` | `ui.toggleDrawer('harmony')` | `ui.harmony` |
| 8 | Library (`separatorBefore`) | `library` | `view.library` | `Alt+B` | `NAV` Library `toggle()` (`toggleLibrary()`) | `ui.view` is `library` (when Quick Racks isn't the match, D3) |
| 9 | Settings | `settings` | `nav.settings` | `Alt+T` | `ui.toggleDrawer('settings')` | `ui.settings` |

The interim `chosen` is the first row, in table order, whose condition holds, else `null` (for example while the Browser, the Rack drawer or Charts is open: none of them has a tab). Once a page's spec lands, its row becomes `ui.page = id`, and `chosen` is `ui.page`. Launchkey mapping: none (no Launchkey control picks a page).

### Props

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `chosen` | `PageId \| null` | `null` | The page on view: that tab is drawn chosen and carries `aria-current="page"`. `null`: every tab is off. |
| `tipAction` | `Action<HTMLElement, string> \| undefined` (`svelte/action`) | — | The app's tooltip action (`use:tip`), passed in by the wiring because the library can't import it; handed to ChosenTabs, which applies it to each tab with that tab's `tip`. Without it each tab still carries its `data-tip`. |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onchoose` | ChosenTabs' `onchoose`: a click, or Space or Enter, on a tab that isn't the chosen one (a click on the chosen tab calls nothing, ChosenTabs D6) | `(id: PageId)` that tab's `id` |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Children

| Child | Props passed |
|---|---|
| `ChosenTabs` | `{ items: PAGES, chosen, label: 'Pages', kind: 'nav', size: 'page', tipAction, onchoose }`: `chosen`, `tipAction` and `onchoose` passed through unchanged (`onchoose`'s `string` id is always a `PageId`, since every item comes from `PAGES`). |

PageTabs renders ChosenTabs as its only element (no wrapper of its own), so its box is ChosenTabs' box: the `nav` named "Pages", 36 tall, as wide as the nine tabs plus the separator and its 8px margins (about 685px at the board's font; not a whole number of pixels).

### Visual rules

- **Tokens used:** none of its own; ChosenTabs' at `size: 'page'` (its Visual rules: 36px tabs, `10px 10px 0` padding, 14 / 400 labels in `--m`, the 24px `--t` block with a `--g` label on the chosen tab, the 1 × 16 `--line` separator with 8px margins, the `--focus` ring).
- **Size:** ChosenTabs' (above). It never wraps and never shrinks.
- **States drawn by:** the chosen tab (one of nine, or none), and keyboard focus on a tab: both ChosenTabs'.
- **The bottom line:** PageTabs draws no line; the chosen block reaches the bottom of its 36px box, and AppBar draws its 1px `--t` line on that same pixel row (AppBar D2), so the block stands on the line.
- **Type:** ChosenTabs' (DM Sans 14 / 400, tabular numerals).
- **Contrast:** nothing of its own; ChosenTabs lists `--m` on `--g` and `--g` on `--t`.
- **Motion:** none.

### Accessibility

- **Role and name:** a `nav` landmark named "Pages" (ChosenTabs' `label`) holding nine `button`s named by their labels, "Stage" … "Settings"; the chosen one has `aria-current="page"`; each has `aria-keyshortcuts` from its `shortcut` ("Alt+G" … "Alt+T"). Not a `tablist`: these change the page (ChosenTabs D1).
- **Keyboard:** every tab is a Tab stop, left to right (kit › Interaction conventions: the app bar's tabs come first); Space or Enter chooses it. No arrow keys (the window's ← → keep stepping the style).
- **Tooltip id:** per tab, the `tip` column above, as `data-tip` on each button and through `tipAction` when given.

## Stories (Story station)

Title `Components/PageTabs`, `layout: 'centered'`. Every story renders in dark and light (the toolbar theme). The meta's args: `{ chosen: 'stage', onchoose: fn(), tipAction: fn() }` (`fn()` as the action: the story test counts it as an action, and a spy returning nothing is a valid Svelte action). Controls: `chosen`, a select of the nine ids and `null`.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ chosen: 'stage' }` | the Stage board's tabs: Stage in the white block, the rest grey, the separator before Library | — (D5) | the `navigation` named "Pages" holds 9 buttons named, in order, Stage, Channel, Effects, Quick Racks, Multi Pads, Looper, Harm/Arp, Library, Settings; "Stage" has `aria-current="page"` and `data-face="chosen"`; no other button has `aria-current` or `data-face`; their `data-tip`s are, in order, `view.stage`, `nav.channel`, `nav.effects`, `nav.quick`, `nav.multipad`, `nav.looper`, `nav.harmony`, `view.library`, `nav.settings`; their `aria-keyshortcuts` are, in order, Alt+G, Alt+N, Alt+E, Alt+R, Alt+P, Alt+L, Alt+H, Alt+B, Alt+T; `tipAction` was called 9 times |
| `ChannelChosen` | `{ chosen: 'channel' }` | a display tab other than Stage chosen | — | "Channel" has `aria-current="page"`; no other button has it |
| `LibraryChosen` | `{ chosen: 'library' }` | the first tab past the separator chosen | — (D5) | "Library" has `aria-current="page"`; no other button has it |
| `SettingsChosen` | `{ chosen: 'settings' }` | the last tab chosen, its block at the right end | — (D5) | "Settings" has `aria-current="page"`; no other button has it |
| `NoneChosen` | `{ chosen: null }` | every tab grey (a drawer without a tab is open) | — | no button has `aria-current`; no element has `data-face="chosen"` |
| `Chooses` | `{ chosen: 'stage' }` | — | — | click "Effects" → `onchoose` last called with `'effects'`; focus "Settings", press Enter → last called with `'settings'`; focus "Looper", press Space → last called with `'looper'`; `onchoose` called 3 times |
| `Focused` | `{ chosen: 'stage' }`, `parameters: { pseudo: { focusVisible: ['[aria-current="page"]'] } }` | the focus ring around the chosen Stage tab | — | — |

Crop positions would be `board x,y w×h` in the 1440×900 renders; PageTabs has none (D5).

**Vitest, pure** (`app/src/ui/PageTabs/pages.test.ts`): `PAGES` has 9 items whose `id`, `label`, `tip` and `shortcut` are the table's rows in order; only `library` has `separatorBefore: true`; no item has `name` or `disabled`; ids are unique.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`); `pages.test.ts` passes.
- `npm run shots -- PageTabs` finds no axe violation on any story. No story has a crop; the tabs' pixels are checked by `npm run shots -- AppBar` (`Board`) and `npm run shots -- ChosenTabs` (`Board`).
- Only listed tokens are used (none of its own); no inline colours, no literal sizes.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · One ChosenTabs.** PageTabs is ChosenTabs with the fixed page list: the separator before Library is that item's `separatorBefore` (ChosenTabs D9), so PageTabs owns only `PAGES`, `PageId` and the mapping to the wiring.
- **D2 · Stage's interim action.** kit › App bar says when Stage is chosen but not what clicking it does: it closes every open drawer, the Browser and the Channel view and sets `ui.view = 'stage'`, so Stage is chosen afterwards.
- **D3 · Interim precedence.** While drawers stand in for pages, `chosen` is the first tab in table order whose condition holds, so Library on its Racks tab reads as Quick Racks, not Library; with no match it is `null`.
- **D4 · Labels as names.** Each tab's accessible name is its label; the page boards' `aria-label="Back to stage"` on an unchosen Stage tab isn't used, so the name stays the same on every page.
- **D5 · No crop of its own.** The board box `527,24 685×36` has no whole-pixel width (the tabs are text widths) and its bottom row (y 59) is the app bar's `--t` line, which PageTabs doesn't draw, so a PageTabs shot would differ there by about 0.025 of the crop; the same pixels are checked in AppBar's `Board` crop, where the line is drawn, and by ChosenTabs' `Board` crop (with its bottom row judged by Inspect).
- **D6 · Re-clicking the chosen tab does nothing.** ChosenTabs calls nothing for the chosen tab (its D6), so today's `NAV` `toggle()` never closes a drawer from a click on its own chosen tab; to close a drawer the player picks another tab, presses its Alt key again or presses Esc, as today.

# Browser

Library › Styles: the style browser. A tall page in place of the Stage's display: the whole
library, by folder, filterable, with the Launchkey band at half height so the player keeps the
faders and the pads while they look for a style. Flow: find a style.

- **Issue:** #513 · **Flow:** Find a style · **Boards:** `docs/design/push/Browser-Dark.dc.html`,
  `Browser-Light.dc.html`; pictures `docs/design/push/png/Browser-Dark.png`, `Browser-Light.png`
  (1440 × 900). The spec stands without them: every value a builder needs is below, in
  [kit.md](kit.md) or in [Stage.md](Stage.md); the board lines in the Components table are for
  cutting crops.
- **Built from:** [kit.md](kit.md): App bar, Section row, the faces and tokens, the Pad, Knob and
  FaderStrip rules, the Key strip and the Status line; [Stage.md](Stage.md): the rack readout,
  One Touch, the chord runs and tones, the Track buttons. The tall-page parts the kit names as
  "owned elsewhere" (the app bar's page variant, the compact now-playing block, the half band,
  the keys status row) and the Library frame (the Library pages list, the Quick Racks bar) are
  specified in **Kit additions** below until the Channel spec (#501) lands them in kit.md
  (BR-D2). This file specifies the Styles page, which is the Browser's own.
- **Copied from:** Stage (#500). What the Stage draws and this page keeps is named, not
  restated; what differs is here.
- **Variants of this screen:** the other Library pages (Sounds #515, Instruments #516, Racks
  #517, Style map #518) keep the frame (left column, hairline, zone header shape) and spec their
  own page content.
- **Glance order:** the cursor row (the solid block), then the loaded style's dot and the
  Track ◀ ▶ marks, then the chosen folder, then the compact block's chord and section. Nothing
  else is brighter than `--t` on `--g`.
- **Before building:** C1 (tooltip keys) must land first; C2–C4 don't block (see Contract
  changes needed). Stage.md's C5 (its tooltip keys), the kit's new tokens, the `longpress`
  action, the app-only `ui.page` (Stage.md D2) and the shots.ts masks (Stage.md D39) are
  assumed to have landed with the Stage; if this page is built first, it lands them itself
  (BR-D3). Until `ui.page` exists, `ui.view` (`'stage' | 'library'`, `app/src/lib/store.svelte.ts`)
  is the same thing for this page: every `ui.page = 'library'` below reads `ui.view = 'library'`.

## Layout

At 1440 × 900, laid out by the `Browser` component (`Pages/Browser`) and scaled by the app
shell exactly as the Stage (Stage.md D1). Padding 24 all round; a column.

| Region | Box | What's in it | Spec |
|---|---|---|---|
| App bar | `24,24 1392×36` | wordmark, rack readout, One Touch, page tabs (Library chosen), Launchkey status, health slot | kit › App bar + Kit additions › App bar, page variant |
| Section row | `24,68 1392×32` | Accomp, count row, Metronome ▾, Unison, Panic, ? | kit › Section row, unchanged |
| Page | `24,112 1392×468` | the left column, a hairline, the Styles page | below |
| Half band | `24,600 1392×176` | faders, Track, knobs, pads, transport, at half height | Kit additions › Half band |
| Keys row | `24,796 1392×80` | the status row (20) over the key strip (56) | Kit additions › Keys status row; kit › Key strip |

Vertical rhythm: app bar, 8, section row, 12, page, 20, band, 20, keys row (24 + 36 + 8 + 32 +
12 + 468 + 20 + 176 + 20 + 80 + 24 = 900). Every box below is at 1440 × 900 and includes
borders.

The page (`<section aria-label="Library: Styles">`) has no box: `--g` ground, `overflow:
hidden`, and an inner area inset 24 left and right, 16 top and bottom: `48,128 1344×436`, a row,
gap 24:

| Part | Box | Spec |
|---|---|---|
| Left column | `48,128 320×436` | Kit additions › Library frame (compact block, Library pages, Quick Racks bar) |
| Hairline | `392,128 1×436` | a 1px `--line` divider, `aria-hidden` |
| Page content | `417,128 975×436` | Styles page, below |

## Styles page

`417,128 975×436`, a column: the **zone header** (36) then the **body** (400, padding-top 12).

### Zone header

`417,128 975×36`, `border-bottom: 1px solid var(--line)`, items stretched, gap 12, no wrap.

| Control | Face | Reads | Sends / does | Tooltip | Launchkey |
|---|---|---|---|---|---|
| "Styles" | 14 / 400 `--m`, centred vertically (`align-self: center`, as are the view name and the count); not a control | — | — | `library.tab_styles` (new) | — |
| View name | 14 / 400 `--t`; not a control | the chosen category (BR-D4): a folder's name ("Pop & Rock"), or "All styles", "Favourites", "Recent" | — | — | — |
| Count | 14 / 300 `--m`; not a control | with no filter, the category's count ("212"); with a filter, "{shown} of {count}" ("9 of 212"); while `library.pending` > 0, " · {pending} indexing" follows ("212 · 40 indexing"); before the entries are fetched, "—". Every count on the page uses `toLocaleString('en-US')` ("1,284"; BR-D25) | — | — | — |
| Views (`role="tablist"`, `aria-label="Views"`, `margin-left: auto`, no gap between tabs: their padding spaces them) | three header tabs (kit › Faders header tabs: 35 tall, padding `11px 10px 0`, 13 / 400, `--m`; chosen = `aria-selected="true"`, the chosen face as a 22px block on the bottom, `--g` text): "All", "Favourites", "Recent", then a normal space and its count at weight 300 in the tab's own colour ("All 1,284"; the counts never change with the filter) | chosen when the category is `all`, `favourites`, `recents`; none chosen while a folder is. Counts: All = `library.count`; Favourites and Recent = the paths of each that are in the fetched library (0 before the fetch) | sets the category (app-only `prefs.category`, BR-D4); one tab stop with roving ← → between the tabs (as today's `panels/library/Library.svelte` tab row); each tab `aria-controls` the listbox | `browser.all`, `browser.favourites`, `browser.recents` | — |
| Filter | a `<label>` 220 × 30, centred vertically, `border-bottom: 1px solid var(--m)`, gap 8: a 13 × 13 magnifier (SVG below) `--m`, a visually hidden "Filter styles", and an `<input type="search">` (flex 1, 28 tall, no border or background, 13 / 400 `--t`, `caret-color: var(--t)`, placeholder "Filter by name, tempo or time" in `--m`, `autocomplete="off"`, `spellcheck="false"`) | `query` (app-only) | filters the list (below); holds focus while the page is open and drives the list from the keyboard (Keyboard below) | `browser.filter` | — |
| Open file… | off face, 28 tall, padding 0 14, 13px, centred vertically, `aria-haspopup="dialog"` | — | opens the system file chooser (Prompts-More, #505, item G) for the library's types (`.sty .prs .sst .bcs .pcs .pst .fps`); the chosen file is sent as `loadStylePath { path }`; Cancel in the chooser does nothing. Until the chooser exists (C3): disabled (BR-D5) | `browser.open_file` (new) | — |

The magnifier: `<svg width="13" height="13" viewBox="0 0 14 14"><circle cx="6" cy="6" r="4.5"
fill="none" stroke="var(--m)" stroke-width="1.2"/><path d="M9.5 9.5 L13 13" stroke="var(--m)"
stroke-width="1.2"/></svg>`, `aria-hidden`.

`aria-label`s: the filter input "Filter styles"; Open file… "Open a style file from anywhere: it
is added to the library and loaded"; each view tab "{name}, {count} styles" ("Favourites, 36
styles").

**The filter (BR-D6).** `query` trimmed, case-insensitive. A row matches when the query is a
substring of its name, of its file name without folder and extension (`stem` in
`panels/browser/model.ts`), or of its folder; or when the query is a whole number equal to its
tempo rounded ("104"); or when the query is `n/d` equal to its time signature ("3/4"). Empty
query: every row of the category. Filtering never re-sorts.

### Body

`417,164 975×400`, padding-top 12 (inner `417,176 975×388`), a row, gap 24: the **folders**
(200) and the **list column** (751).

#### Folders

`<nav aria-label="Style folders">` at `417,176 200×388`, a column of 32-tall rows,
`align-self: flex-start` (it is as tall as its rows: 352 for 11). More than 12 folders: it
scrolls (`overflow-y: auto; max-height: 388px`, scrollbar hidden: `scrollbar-width: none`;
wheel, and the chosen row is scrolled into view when it changes).

One row per **top-level folder** of the library (BR-D7): the first `/`-segment of each entry's
`folder`, in the order each folder's first entry appears in the library's own order (the
component never re-sorts; `folderTree` in `model.ts` is used for the counts, not the order),
with its count = entries in it and its subfolders. Files at the root (`folder` "") add a first row
"Library root" only when there are any. A row is a `<button>` 200 × 32, padding 0 8, no radius,
`border-bottom: 1px solid var(--line)`, items centred, gap 12, left-aligned, no wrap: the name
14 / 400 `--t2` (flex 1, ellipsis) and the count 13 / 300 `--m`. **Chosen** (`aria-current="true"`):
the chosen face as a block (`--t` fill, name `--g` at weight 500, count `--sel-mut`, bottom
border `--t`). Click sets the category to that folder (`{ kind: 'folder', path }`), deselects
the views, and scrolls the list to centre the kept cursor (Keyboard below). Tooltip
`browser.folder`. `aria-label` "{name}, {count} styles". No Launchkey mapping. A remembered
folder that is no longer in the library reads as All (as today's `Browser.svelte`); before the
entries are fetched the list is empty whatever the category.

#### List column

`641,176 751×388`, a column: the column header (32), the listbox (288, nine rows), then (12
below) the footer (44).

**Column header** `641,176 751×32`, padding 0 12, `border-bottom: 1px solid var(--line)`, the
row grid (below), items centred, 12 / 400 `--m`, no wrap, `aria-hidden`: "Track", "", "Name",
"Folder", "BPM", "Time", "Sections", "File", "".

**Row grid:** `grid-template-columns: 44px 32px minmax(0, 1fr) 88px 44px 40px 126px 44px 32px`,
`column-gap: 10px`, padding 0 12. At 751 the name column is 197 wide; cells start at x 653, 707,
749, 956, 1054, 1108, 1158, 1294, 1348.

**Listbox** `641,208 751×288`, `role="listbox"`, `aria-label` "Styles in {view name}, in Track
order (folder, then name)" ("Styles in Pop & Rock, in Track order (folder, then name)"; Recent:
"Styles in Recent, newest first"),
`position: relative`, `overflow-y: auto`, scrollbar hidden (`scrollbar-width: none`, BR-D8),
`overscroll-behavior: contain`. Rows are 32 tall and virtualised as today
(`panels/browser/Browser.svelte`: only the rows in view plus 8 above and below exist, each
`position: absolute` at `top = index × 32` inside a spacer `rows × 32` tall), so a 60,000-style
library costs nine rows. The wheel scrolls the list and leaves the cursor where it is; the
keyboard moves the cursor and keeps it in view (Keyboard below).

**Order and sorting (BR-D9).** There is no sort control. The rows are the library's own order
(folder, then name; `library()` entries as sent), the order Track ◀ ▶ and `stepStyle` step
through, so the marks and the hardware always agree with the list. Recent is the one
exception: newest first (`prefs.recents`). Favourites keep library order.

**What the rows are (`visibleRows` in `model.ts`):** the category's entries (All: every entry;
a folder: entries whose `folder` is it or under it; Favourites: entries whose `path` is in
`prefs.favourites`; Recent: `prefs.recents` paths that are in the library), then the filter.

#### Row

`role="option"`, 32 tall (box-sizing border-box), padding 0 12, the row grid, items centred,
`border-bottom: 1px solid var(--line)`, `cursor: pointer`, `data-face="off"`; `id`
`style-{entry.id}`. Every cell is left-aligned and its text `--t` unless the table says
otherwise; the column header's labels are left-aligned too. **Cursor row**
(`aria-selected="true"`, `data-face="chosen"`): the chosen face as a block: `--t` fill, text
`--g`, muted text `--sel-mut`, off lamps and the disabled ▶ `--sel-off`, an enabled ▶ `--g`,
bottom border `--t` (Kit additions › Tokens). One row is the cursor at all times when the list
isn't empty (Keyboard below).

| Cell | Width | Face | Reads |
|---|---|---|---|
| Track | 44 | items centred vertically, left-aligned, gap 6, 12 / 400 `--m` (cursor: `--sel-mut`); `aria-hidden` | **loaded** (`entry.id === style.id`): a 6px round `--ok` dot with `--bg` glow (Stage.md › Section and tempo, the Running dot), no glyph; **prev** (`entry.id === surface.trackPrev.id`): "◀"; **next** (`surface.trackNext.id`): "▶"; else empty. A style can't be two of these. |
| Star | 32 | a 32 × 31 text button, centred: a 12 × 12 star (SVG below); favourite: fill and stroke `--t` (cursor: `--g`); not: no fill, stroke `--d` (cursor: `--sel-mut`); `aria-pressed` | `prefs.favourites.has(entry.path)` |
| Name | 197 | items centred, gap 8, no wrap: the name 14 `--t` (cursor `--g`), `min-width: 0`, ellipsis; weight 500 when loaded or queued, else 400 | `entry.name`; **queued** (`entry.id === preview.queued`): after the name, a tag "next bar": 18 tall, padding 0 6, 1px border in the row's text colour (`--t`; cursor `--g`), no radius, 11 / 400, line-height 16, `flex: none`; its border flashes on the LED clock as a Next pad's (kit › Pad: full and 18% with `frac(beats) < 0.5`) |
| Folder | 88 | 13 / 400 `--m` (cursor `--sel-mut`), ellipsis | BR-D10: in a folder category, `entry.folder` with the chosen folder's path and its `/` removed ("Pop & Rock/8Beat" → "8Beat"; a file right in the folder: empty); in All, Favourites and Recent, the whole `entry.folder`; root files: empty |
| BPM | 44 | 15 / 300 `--t` (cursor `--g`) | `entry.tempo` rounded (`formatTempo`: `Math.round`; null: empty); pending: "…" |
| Time | 40 | 15 / 300 `--t` (cursor `--g`) | `entry.timeSignature` as "4/4" (null: empty); pending: empty |
| Sections | 126 | eleven 8 × 2 bars, radius 1, in four groups: Intro I–III, Main A–D, Break, Ending I–III; 2px between bars of a group, 8px between groups (3 × 8 + 2 × 2 = 28, 38, 8, 28: 126 with the three 8px gaps); `role="img"`, `aria-label` "Sections: {the lit ones, comma-separated}" ("Sections: Intro I, Intro II, Main A, …") | `sectionLamps(entry.sections)` (`model.ts`: Intro A–C → I–III, Main A–D, Break, Ending A–C → I–III, from the entry's summary text): lit = `--t2` (cursor `--g`), unlit = `--track` (cursor `--sel-off`); pending: no bars |
| File | 44 | 12 / 400 `--m` (cursor `--sel-mut`) | `entry.format` ("SFF1", "SFF2"); null: "—"; pending: "…" |
| ▶ | 32 | a 32 × 28 button, no face, 11px, radius 4; `--d` (cursor `--sel-off`) when it can't act, `--t` when it can, `--a` while this row auditions (glyph "■") | Preview, below |

The star: `<svg width="12" height="12" viewBox="0 0 12 12"><path d="M6 1 L7.5 4.3 L11 4.6 L8.3
6.9 L9.1 10.4 L6 8.6 L2.9 10.4 L3.7 6.9 L1 4.6 L4.5 4.3 Z" stroke-width="1"/></svg>`.

**Row actions.**

- **Click on the row** (anywhere but its buttons): the row becomes the cursor; then exactly
  what the Load button would do for that row (Footer): stopped, `loadStyle { id }` and back to
  the Stage; playing, `queueStyle { id }` and the page stays (BR-D11). Where the Load button
  would be disabled (the loaded style, the queued style while playing, an error entry) the
  cursor moves and nothing is sent. Tooltip `browser.row` (error entries `browser.row_error`).
  The row and its buttons call `preventDefault()` on `pointerdown`, so the filter keeps focus
  (BR-D14).
- **Star:** `prefs.toggleFavourite(entry.path)`, app-only, remembered on this computer
  (`panels/browser/prefs.svelte.ts`). Tooltip `browser.favourite`. `aria-label` "Add {name} to
  favourites" / "Remove {name} from favourites".
- **▶ (Preview):** stopped (`transport.running` false) and the entry `ok`: sends
  `auditionStyle { id }`; while that row auditions (`preview.audition.id === entry.id`) it reads
  "■" in `--a` and sends `stopAudition`. Playing, pending or error: `aria-disabled="true"`,
  `--d`, default cursor. Tooltip `browser.preview` (auditioning: `browser.preview_stop`).
  `aria-label` "Preview {name}" ("Preview {name} (stop the band first)" while playing; "Stop
  previewing {name}" while auditioning).
- **Hover:** no look change (kit D35); with Preview on select, a hover starts the preview
  dwell (Footer).

`aria-label` template for the row: "{name}{, folder cell}{, tempo BPM}{, time}" (a part whose
cell is empty is dropped with its comma; a pending entry reads "{name}{, folder}, indexing"),
then, in this order, each that applies: ", loaded and playing" (loaded and running) / ", loaded"
(loaded, stopped); ", loads at the next bar" (queued); ", Track left loads this" / ", Track
right loads this"; ", previewing" (auditioning); ", unreadable: {error}" (error). Examples:
"Coastal Highway, Pop, 112 BPM, 4/4, loads at the next bar"; "Sunday Drive Pop, Pop, 104 BPM,
4/4, loaded, previewing".

**Error entry** (`entry.status` `error`): the name in `--t` as any row; the Folder, BPM, Time,
Sections and File cells are replaced by one cell spanning them (`grid-column: 4 / 9`) with
`entry.error` ("unreadable" when null) 12 / 400 `--ending` (the same on the cursor's block),
ellipsis; ▶ disabled. It can be the cursor, starred and filtered; it never loads, and the Load
button is disabled on it.

**Pending entry** (`status` `pending`, still being indexed): cells as the table says; it loads
and queues like an `ok` entry everywhere (row click, Enter, the Load button: the engine indexes
it on load); only ▶ and Preview on select skip it.

#### Footer

`641,508 751×44`, one row, items centred, gap 16.

| Control | Face | Reads | Sends / does | Tooltip |
|---|---|---|---|---|
| Preview on select | LampButton `size md` (32 tall, 14px, padding 0 16; BR-D12), label "Preview on select", no code | `prefs.autoPreview` (on = lamp face) | toggles it (app-only, remembered) | `browser.auto_preview` |
| Note | 13 / 400, no wrap, `min-width: 0`, ellipsis; not a control; `role="status"`, `aria-live="polite"` | playing: "While stopped" in `--m`; stopped and auditioning: "Previewing {name} · bar {bar}/{bars} · {chord}" in `--t` (name from the library entry whose id is `preview.audition.id`, else "style"; no chord: the chord part is dropped); stopped, no audition: empty | — | — |
| Cancel | off face, 44 tall, padding 0 18, 14 / 400 `--t2`, `margin-left: auto` | — | back to the Stage without loading: `ui.page = 'stage'` (BR-D13). Leaving the page by any route (Cancel, Esc, the Stage tab, Alt+B, Alt+S, a page tab, a stopped load) stops an audition: the wiring sends `stopAudition` on teardown when `preview.audition` is set, as today's browser does. A queued style stays queued (there is no command to unqueue it: Follow-ups) | `browser.close` |
| Load | 260 × 44, padding 0 16, radius 4, items centred, `justify-content: space-between`, no wrap: "Load {name}" 14 / 500 (ellipsis), then, while playing and enabled or queued, "next bar" 12 / 400 `--t2` (in both faces) | the cursor row | Faces and sends by state, below | `browser.load` (new) stopped; `browser.queue` playing |

**The Load button** acts on the cursor row (BR-D11). "Loadable" = `status` `ok` or `pending`.

| State | Face | Label | Click |
|---|---|---|---|
| Stopped, cursor on a loadable style that isn't loaded | off face (`--btn`, `--t`), no right text | "Load {name}" | `loadStyle { id }`, then `ui.page = 'stage'` |
| Playing, cursor on a loadable style that isn't loaded or queued | off face, right text "next bar" | "Load {name}" | `queueStyle { id }`; the page stays; the row gets its tag |
| Playing, cursor on the queued style (the board) | waiting face in `--t`: transparent, `1px solid var(--t)`, `--t` text, right text "next bar"; the border flashes with the row's tag (LED clock); `aria-disabled="true"` | "Load {name}" + "next bar" | nothing |
| Cursor on the loaded style (stopped or playing) | off face, `aria-disabled="true"`, label `--d`, no right text | "Load {name}" | nothing |
| Cursor on an error entry, or the list is empty | off face, `aria-disabled="true"`, label `--d`, no right text | "Load {name}" (empty list: "Load") | nothing |

`aria-label`: "Load {name}" / "Load {name} at the next bar" / "{name} loads at the next bar
(queued)" / "{name} is loaded" / "{name} can't be loaded: {error}" / "Load: nothing to load".

A stopped load returns to the Stage because that is what today's browser does on Enter; a
queued one stays so the player sees it land, when the row's dot moves and the tag goes
(BR-D11).

### Keyboard, focus and pointer

- The filter input takes focus when the page opens and keeps it while the player uses the list
  (`role="combobox"`, `aria-expanded="true"`, `aria-controls` the listbox,
  `aria-autocomplete="list"`, `aria-activedescendant` the cursor row's id). The rows and their
  star and ▶ buttons are `tabindex="-1"` (pointer and the keys below; BR-D14).
- In the filter: **↑ ↓** move the cursor one row, **PgUp PgDn** eight (9 visible − 1), **Home
  End** to the first and last; the cursor is scrolled into view (to the top or bottom edge, not
  centred). **Enter** = the Load button's click for the cursor row (nothing where it is
  disabled); **Shift+Enter** = stopped, the cursor row's ▶ click (nothing where ▶ is disabled);
  playing, the Load button's click; **Ctrl/⌘+D** stars the cursor row; **Esc** = `ui.escape()`
  (a drawer open over the page closes first; with nothing open, back to the Stage as Cancel);
  **← →** edit the text (the global `stepStyle` arrows don't fire in a text field:
  `app/src/lib/shortcuts.ts` `isTextField`). Typing filters at once, and the cursor is kept on
  the same style when it is still shown, else the first row (`keepCursor` in `model.ts`),
  scrolled into view centred. A folder or view click keeps the cursor the same way and centres
  it.
- **Global keys:** the page doesn't gate them (today's `ui.browser` gate in `shortcuts.ts`
  goes with the modal): outside a text field they work as everywhere (← → `stepStyle`, Space
  Start / Stop, the Alt letters). The plain **Enter** binding in `app/src/lib/keys.ts` (`enter:
  { app: 'browser' }`, today "open the browser") opens this page from the Stage and does
  nothing extra on it (Gap).
- **The cursor on open (BR-D15):** the queued style if there is one, else the loaded style,
  scrolled to the centre of the listbox (or as near as the list allows: the board's list is at
  its top). If that style isn't in the remembered category, the category becomes All first.
  While the page is open the cursor follows `preview.queued` when it changes to a style, and
  `style.id` when it changes (Track ◀ ▶, a pad, the hardware), switching to All the same way
  when the style isn't in the category.
- **Tab order** (DOM order): app bar (rack readout, One Touch, tabs, health slot), section
  row, Library pages, Quick Racks (◀, ▶, Store, ✕, slots 1–8), zone header (the views' tab
  stop, filter, Open file…), folders, footer (Preview on select, Cancel, Load), half band
  (fader header tabs, each strip's fader then its name, the lamp row, Track ◀ ▶, knob ▲ ▼ and
  pad ▲ ▼ in the shared header, knobs 1–8, pads 1–16, transport), the status line when it has
  a message, the keys button. The compact block and the listbox are not in the tab order; the
  rows' star and ▶ are pointer-only (plus Ctrl+D and Shift+Enter). Every other control takes
  focus on click as usual; the filter regains focus only when the page opens or the player
  tabs or clicks into it.
- **Alt+S** (today's Browser key) and the Stage's style name open this page (`ui.page =
  'library'`, `ui.libraryTab = 'styles'`, BR-D1); on the page Alt+S goes back to the Stage
  (a toggle, as today); **Alt+B** toggles Library on its last tab; **Esc** anywhere on the page
  with nothing open over it goes back to the Stage.
- Pointer: kit › Faces (no hover look; pointer cursor on enabled controls and on the rows).
- **Preview on select:** with it on and the band stopped, an `ok` row that is hovered or made
  the cursor for 600 ms (`PREVIEW_DELAY_MS` in the wiring, a dwell timer like the long press: it
  measures the player's gesture, BR-D16) sends `auditionStyle { id }` unless it already
  auditions; one timer: a new hover or cursor move restarts it for that row; leaving the
  listbox cancels a pending dwell. Pending or error rows and playing: nothing.

## States

| State | What changes |
|---|---|
| Playing, a style queued (the board) | as drawn: the cursor on the queued row, its tag and the Load button flashing on the LED clock; ▶ disabled on every row; "While stopped" |
| Playing, nothing queued | the cursor on the loaded row: Load disabled; another row: Load "next bar" |
| Stopped | ▶ enabled; Load "Load {name}" without "next bar"; the note empty; the compact block's dot hidden (space kept) and the section in `--m` (Stage.md D4) |
| Stopped, previewing | the auditioning row's ▶ reads "■" in `--a`; the note "Previewing {name} · bar 2/4 · Am"; the Load button unchanged |
| Preview on select on | the lamp lit; hover and cursor dwell preview while stopped |
| A folder chosen (the board) | the folder row chosen; no view tab chosen; the header "Styles {folder} {count}" |
| All / Favourites / Recent | that tab chosen (22px block); no folder chosen; the header "Styles All styles 1,284" etc.; Recent newest first |
| Filter typed | the header "{shown} of {count}"; rows filtered, order kept; the cursor kept or first |
| The listbox is empty | one centred text in it (top 24, 14 / 400 `--m`, centred), the **first** that applies: (1) entries not fetched and `library.count` > 0: "Reading the library…"; (2) no entries and `library.scanning`: "Scanning the style folders…"; (3) `library.count` 0: "No styles yet: the library is empty."; (4) the query isn't empty: "No style matches “{query}”"; (5) Favourites: "No favourites yet: star a style with ☆ (or Ctrl+D)."; (6) Recent: "Nothing loaded yet."; (7) otherwise (a folder with no readable rows can't happen) nothing. The Load button reads "Load", disabled; Enter does nothing; the folders list is empty in (1)–(3); the header count reads "—" in (1), "0" in (2)–(3) |
| Scanning with entries | rows as usual, the header's " · {pending} indexing" |
| Pending rows (`library.pending` > 0) | "…" in BPM and File, no lamps |
| An unreadable style | its row's error text in `--ending`; ▶ and Load disabled on it |
| Track with no neighbour (`surface.trackPrev`/`trackNext` null) | no ◀ / ▶ mark; the half band's Track button disabled (kit, Stage.md D38) |
| Store armed (`quickRacks.store`) | every slot in the waiting face in `--lamp`, flashing on the LED clock; Store lit; Clear disarmed |
| Clear armed (app-only) | ✕ in the waiting face in `--ending` (`aria-pressed`); every stored or loaded slot in the waiting face in `--ending`; Store disarmed |
| Quick Racks read-only (`quickRacks.readOnly`) | Store and ✕ disabled; slots still load |
| A slot's rack gone (`missing`) | the slot's name in `--warn` with a 11px ⚠ before it; a press is refused by the session (the status line says so) |
| Open file… without a chooser (C3) | disabled, tooltip `browser.open_file` |
| No Launchkey, no synth, trouble, a message | kit › App bar, Status line (the status row's line) |
| Light theme | the same markup; tokens only (kit › Tokens, Kit additions › Tokens); no glow |

## Board fixture

The state and moment for the `Pages/Browser` › `Board` story and its shots:
`app/src/ui/Browser/Browser.fixtures.ts` exports `boardState` (a full `AppState`),
`boardLibrary` (a `LibraryList`), `boardPrefs` (the four remembered fields of `BrowserPrefs`
in `panels/browser/prefs.svelte.ts`: `category`, `favourites`, `recents`, `autoPreview`),
`boardQuery` (`''`), `boardNow` and `boardMeterHolds`. It starts from the Stage's `boardState` (Stage.md › Board fixture: the same
moment, clock, transport, chord, parts, faders, meters, knobs, pads, keys), and overrides the
fields below. If `app/src/ui/Stage/Stage.fixtures.ts` isn't there yet, this file carries those
fields itself, copied from Stage.md.

- **Moment:** `boardNow = 10000`, `receivedMs = 10000`; `surface.clock` as the Stage's (bar 3
  beat 3, `ledAnchorBeats` 0.25: the LED clock reads 0.25, `frac < 0.5`, so the queued tag and
  the Load border are at full).
- **Library** (`boardLibrary`, revision 1): 1,284 entries, generated, in library order: by
  lowercased folder, then lowercased name, then path (`Library::sort` in
  `crates/yahaha-sff/src/library.rs`). Top-level folders and counts: Ballad 98, Ballroom 102,
  Country 76, Dance 164, Entertainment 90, Latin 118, Movie & Show 64, Pop & Rock 212, R&B 87,
  Swing & Jazz 141, World 132 (= 1,284; no root files), and that alphabetical order is the
  folder list's (BR-D7). The board's folder order (Pop & Rock first, then Ballad, Dance, …) is
  placeholder data, like its "fill after bar 4": the screenshot check masks the folder list.
  The first nine entries of Pop & Rock, in order (name, subfolder, tempo, time, sections,
  format, favourite):
  1. Unplugged Ballad Pop, 8Beat, 76, 4/4, `Main ABD · Intro AB · Ending AC · Fill ABD`, SFF2, no
  2. Brit Pop Anthem, Pop, 128, 4/4, `Main ABCD · Intro AB · Ending ABC · Fill ABCD · Break`, SFF2, yes
  3. Coastal Highway, Pop, 112, 4/4, `Main ABCD · Intro ABC · Ending ABC · Fill ABCD · Break`, SFF2, yes
  4. Pop Shuffle, Pop, 108, 12/8, the same as 3, SFF2, no
  5. Sunday Drive Pop, Pop, 104, 4/4, `Main ABCD · Intro AB · Ending AB · Fill ABCD · Break`, SFF2, yes
  6. Waltz Pop, Pop, 84, 3/4, `Main ABCD · Intro ABC · Ending ABC · Fill ABCD` (no Break), SFF2, no
  7. Garage Summer, Rock, 138, 4/4, `Main ABCD · Intro A · Ending BC · Fill ABCD` (no Break), SFF1, no
  8. Soft Rock Radio, Rock, 92, 4/4, `Main ABCD · Intro AB · Ending ABC · Fill ABCD · Break`, SFF2, no
  9. Motown Pop Soul, Soul, 116, 4/4, `Main ABCD · Intro ABC · Ending ABC · Fill ABCD · Break`, SFF2, no

  The lamps these give (Intro I–III, Main A–D, Break, Ending I–III): row 1 `110 1101 0 101`,
  row 2 `110 1111 1 111`, rows 3, 4 and 9 all lit, row 5 `110 1111 1 110`, row 6 `111 1111 0 111`,
  row 7 `100 1111 0 011`, row 8 `110 1111 1 111`. Then 203 filler entries in `Pop & Rock/Soul`
  named "Soul Style 001"… "Soul Style 203" (they sort after Motown Pop Soul), and each other
  folder's count of fillers "{Folder} Style 001"… right in that folder, all 120 BPM, 4/4,
  every section, SFF2, `status` `ok`. Paths `/Library/Styles/{folder}/{name}.sty` (SFF1) or
  `.prs` (SFF2); ids 1…1284 in library order. `voices`, `harmonyTypes`, `arpPatterns` as the
  dev mock's.
- `style`: id and name of Sunday Drive Pop (entry 5 above), path its path, format SFF2, tempo
  104, timeSignature [4, 4], sections those its summary lists. `library`: revision 1, count
  1284, position = its index, pending 0, roots `["/Library/Styles"]`, scanning false.
- `surface.trackPrev` = Pop Shuffle `{ id, name, path }`, `trackNext` = Waltz Pop;
  `surface.controls` trackPrev and trackNext set (`stepStyle` ∓1).
- `preview`: `queued` = Coastal Highway's id; `audition` null. `transport.running` true (the
  Stage's transport).
- `boardPrefs`: category `{ kind: 'folder', path: 'Pop & Rock' }`, `autoPreview` true, `favourites` = the paths of Brit Pop Anthem, Coastal Highway, Sunday Drive Pop and the
  first 33 fillers of Ballad (36), `recents` = Sunday Drive Pop's path then the first 11 fillers
  of Dance (12). The cursor is on Coastal Highway (BR-D15: the queued style), and the listbox
  is scrolled to the top (row 3 is in view from the top, so no centring moves it).
- `quickRacks`: bank 0, store false, storeWaiting null, readOnly false; buttons 1–4: "Sunday
  drive" (`loaded` true), "Warm keys", "Lead synth", "Organ" (`loaded` false), each with a rack
  id; 5–8 empty (`rack` null, name ""); `missing` false everywhere. `liveRack`: name "Sunday
  drive", modified true, its id = button 1's. `ots.applied` 2, four settings. `racks`: 10 racks
  (the four above and six more). `soundLibrary.patches`: 886 patches (generated, numbered);
  `plugins.list`: 4 entries. (The Library pages' counts: 1,284 · 886 · 4 · 10.)
- `pads.pageName` "Sections", `transport.landing` "Main C" (the pads header's "next Main C").
- Everything else (parts, faders, meters, knobs, pads, keys, `keyboard.held` 43 45 48 52 left
  and 76 81 right, `leftSplit` 54, `detection` [0, 54], `ui.keyRange` 61, `message` null): the
  Stage's fixture values.

Board texts that differ from this fixture on purpose: the count row's "fill after bar 4" (Stage.md
D5; masked), the folder list's order (BR-D7; masked), and the One Touch group's `aria-label` on
the board mentions Shift + pads (Stage.md D16; not visible). The light board's `--m` (`#6e6e6e`) and the light board's chosen-row text
(`#ffffff` where the tokens give `--g` `#f2f1ee`, BR-D17) are under the per-pixel threshold.

## Components

In build order; "Exists" is whether it is in `app/src/ui` today, and "Stage" names the entry in
Stage.md › Components it reuses (the same component, with the props noted). Board lines are
dark / light.

| # | Component | Kind | Built from | Exists | Board lines (dark / light) | Spec |
|---|---|---|---|---|---|---|
| 0 | tokens | — | — | yes; add `--sel-mut`, `--sel-off`, `--slot-glow-mix` | `:root` 52, 57 / 36, 41 | Kit additions › Tokens |
| 1 | LampButton | primitive | longpress | yes (Stage 2) | 157, 241 / 141, 225 | `md` (Preview on select), `sm` (Store), `cell` (lamp row) |
| 2 | Button | primitive | longpress | Stage 3 | 154, 156, 158, 189, 243, 326–327, 335–336, 340–341 / −16 | variants `icon` 32 × 32; **new** `icon-sm` 28 × 28 (11px: bank ◀ ▶, knob and pad ▲ ▼, ✕), `track` 40 × 32 (12px), `sm` 28 tall padding 0 14 13px (Open file…), `lg` 44 tall padding 0 18 (Cancel), `half` 40 × 24 and `half-wide` 88 × 24 (the half transport) |
| 3 | ChosenTabs | primitive | — | Stage 4 | 74–85, 179–183, 261–273 / 58–69, 163–167, 245–257 | `size page` (app bar), `size header` (views, fader header) |
| 4 | WaitingChip | primitive | — | Stage 5 | 110 / 94 | the count row's chip; **new** size `tag` (18 tall, 11px, no radius: the row's "next bar" tag) |
| 5 | AccentBlock | primitive | — | Stage 6 | 131, 334 / 115, 318 | **new** size `compact` (16 tall, 13 / 500, padding 0 6) |
| 6 | StatusDot | primitive | — | Stage 7 | 89, 133, 211 / 73, 117, 195 | solid with `--bg`, hollow, hidden-with-space |
| 7 | PartMarks | primitive | — | Stage 8 | 295–297 / 279–281 | 11px ⚠ and ✕ in the half strip's name |
| 8 | GroupHeader | primitive | — | Stage 9 | 152, 175, 259, 324, 332, 380 / 136, 159, 243, 308, 316, 364 | 36 tall hairline row; **new** `size sm` 32 tall (Quick Racks) |
| 9 | FaderStrip | primitive | — | Stage 11 | 277–299; data 495–545 / 261–283; 491–541 | **new** `size half` (Kit additions › Half band) |
| 10 | Knob | primitive | — | Stage 12 | 345–355; data 548–564 / 329–339; 544–560 | **new** `size half` |
| 11 | Pad | primitive | — | Stage 13 | 356–376 / 340–360 | **new** `size half` |
| 12 | KeyStrip | primitive | — | Stage 14 | 407–416; data 566–587 / 391–400; 562–583 | kit › Key strip, unchanged |
| 13 | StatusLine | primitive | — | Stage 15 | 400 / 384 | kit › Status line, inside the keys status row |
| 14 | HealthSlot | primitive | — | Stage 16 | 91 / 75 | kit |
| 15 | ChordReadout | primitive | — | Stage 17 | 136–137 / 120–121 | **new** `size compact` (48px, the tones beside it) |
| 16 | RackReadout | primitive | StatusDot | Stage 20 | 66 / 50 | **new** `layout row` (one line, in the app bar) |
| 17 | SectionLamps | primitive (new) | — | no | 220–232; data 484–489 / 204–216; 480–485 | Row › Sections |
| 18 | StarButton | primitive (new) | — | no | 212 / 196 | Row › Star |
| 19 | FilterField | primitive (new) | — | no | 184–188 / 168–172 | Zone header › Filter |
| 20 | ListRow (a hairline row: name + count, chosen as a block) | primitive (new) | — | no | 143–148, 195–200 / 127–132, 179–184 | Library pages, Folders |
| 21 | QuickSlot | primitive (new) | longpress | no | 161–166; data 442–449 / 145–150; 438–445 | Kit additions › Quick Racks bar |
| 22 | PageTabs | complex | ChosenTabs | Stage 22 | 74–85 / 58–69 | kit › App bar |
| 23 | LaunchkeyStatus | complex | StatusDot | Stage 23 | 89 / 73 | kit |
| 24 | OneTouch | complex | Button | Stage 28 | 67–73 / 51–57 | Stage.md › Style line; here in the app bar |
| 25 | AppBar | complex | PageTabs, LaunchkeyStatus, HealthSlot, RackReadout, OneTouch | Stage 24 | 64–93 / 48–77 | kit › App bar + Kit additions › App bar, page variant (props `rack`, `ots` shown on tall pages) |
| 26 | CountRow, MetronomeSplit, SectionRow | complex | (Stage 25–27) | Stage | 96–120 / 80–104 | kit › Section row |
| 27 | CompactBlock | complex (new) | AccentBlock, StatusDot, ChordReadout | no | 129–140 / 113–124 | Kit additions › Compact block |
| 28 | LibraryPages | complex (new) | ListRow | no | 142–149 / 126–133 | Kit additions › Library pages |
| 29 | QuickRacksBar | complex (new) | GroupHeader, Button, LampButton, QuickSlot | no | 151–168 / 135–152 | Kit additions › Quick Racks bar |
| 30 | LibraryFrame | complex (new) | CompactBlock, LibraryPages, QuickRacksBar | no | 127–171 / 111–155 | Kit additions › Library frame |
| 31 | ZoneHeader | complex (new) | ChosenTabs, FilterField, Button | no | 175–190 / 159–174 | Styles page › Zone header |
| 32 | FolderList | complex (new) | ListRow | no | 194–201 / 178–185 | Styles page › Folders |
| 33 | StyleRow | complex (new) | StarButton, WaitingChip, SectionLamps, Button | no | 210–235 / 194–219 | Styles page › Row |
| 34 | StyleList | complex (new) | StyleRow | no | 205–237 / 189–221 | Styles page › List column (virtualised) |
| 35 | BrowserFooter | complex (new) | LampButton, Button | no | 240–248 / 224–232 | Styles page › Footer |
| 36 | StylesPage | complex (new) | ZoneHeader, FolderList, StyleList, BrowserFooter | no | 174–251 / 158–235 | Styles page |
| 37 | HalfFaderBank | complex (new) | GroupHeader, ChosenTabs, FaderStrip (half), LampRow | no | 258–320 / 242–304 | Kit additions › Half band |
| 38 | TrackColumn | complex (new) | GroupHeader, Button | no | 323–329 / 307–313 | Kit additions › Half band |
| 39 | HalfKnobPadBank | complex (new) | GroupHeader, AccentBlock, Button, Knob (half), Pad (half) | no | 331–377 / 315–361 | Kit additions › Half band |
| 40 | HalfTransport | complex (new) | GroupHeader, Button | no | 379–395 / 363–379 | Kit additions › Half band |
| 41 | HalfBand | complex (new) | HalfFaderBank, TrackColumn, HalfKnobPadBank, HalfTransport | no | 256–396 / 240–380 | Kit additions › Half band |
| 42 | KeysRow | complex (new) | StatusLine, Button, KeyStrip | no | 398–417 / 382–401 | Kit additions › Keys status row |
| 43 | Browser (page, `Pages/Browser`) | complex | AppBar, SectionRow, LibraryFrame, StylesPage, HalfBand, KeysRow | no | whole board | this file |

Components take props and call callbacks; none reads `app.state` or sends (Stage.md D46). The
page wiring `app/src/pages/BrowserWiring.svelte` (outside `app/src/ui`) reads `app.state`,
`app.library`, `prefs` and `ui`, keeps `now`, `receivedMs` and the meter holds as the Stage's
wiring does, passes them down, and maps each callback to its command or interim target.

## Gap against today

| Area | In `app/src` now | Change |
|---|---|---|
| Where the browser lives | `panels/browser/Browser.svelte`: a modal `Overlay` over the Launchkey mirror while `ui.browser`; Alt+S and the display's style name toggle it; `lib/keys.ts` binds plain Enter to it (`enter: { app: 'browser' }`); `lib/shortcuts.ts` makes every global key inert while it is open | A Library page: `ui.page = 'library'` with `ui.libraryTab = 'styles'` (BR-D1). `ui.browser` goes (and its `escape()` and `shortcuts.ts` branches); `LibraryTab` gains `'styles'`; `nav.ts`'s Browser entry (`nav.styles`) and the Enter binding open the page (a toggle, as today); the global keys stay live on the page outside text fields |
| Library frame | `panels/library/Library.svelte`: a tab row (Racks, Sounds, Instruments, Style map), a target part selector, the docked Rack panel and `knobracks` Quick Racks row | The left column of this spec (Kit additions › Library frame): pages as a hairline list, the compact block, the Quick Racks bar. Until the other Library specs land, choosing Sounds, Instruments, Racks or Style map keeps the left column and hosts today's `Library.svelte` (its own tab row included) in the page content box `417,128 975×436` on that tab (D32 interim; BR-D18), so the Styles row is always there to come back by |
| List logic | `panels/browser/model.ts` (`indexLibrary`, `folderTree`, `visibleRows`, `sectionLamps`, `keepCursor`, `moveCursor`), `prefs.svelte.ts` (favourites, recents, auto preview, category, `localStorage`) | Kept as they are, moved under `app/src/ui/Browser/` or imported from where they are (the lane's call); `visibleRows` gains the tempo and time-signature match (BR-D6); the folder list uses depth-0 nodes only (BR-D7); the Folder cell strips the chosen folder's prefix (BR-D10) |
| Row | `panels/browser/Row.svelte`, `Lamps.svelte` (the engine's pad LED colours for the lamps, "▶ ‹ ›" marks, a Queued / Next bar button, a 36px row) | The row above: 32px, the kit's tokens (lamps `--t2` / `--track`, no LED colours), the dot and ◀ ▶ in the Track column, the "next bar" tag, the ▶ preview button only (queueing is the row click and the Load button) |
| Sidebar | `panels/browser/Sidebar.svelte`: All / Favourites / Recent buttons then an indented folder tree | Views move to the zone header's tabs; the folders list is flat (top level) |
| Footer | the audition status, a Toggle, two `HwButton`s "◀ Track" / "Track ▶" with the neighbours' names | Preview on select (LampButton), the note, Cancel, Load. Track ◀ ▶ are the half band's (Kit additions) |
| Keys | the filter input drives the list (`onkey`); Esc closes the overlay | The same keys; Esc goes back to the Stage; Enter's load returns to the Stage only while stopped (BR-D11) |
| Open file… | nothing (the API has `loadStylePath`; no chooser in the app) | New: the button and, once C3 lands, the chooser; until then disabled |
| Half band, keys status row | none (the Stage has the full band; the key strip has a "cheek") | New: Kit additions › Half band and Keys status row |
| Quick Racks | `panels/knobracks/KnobRackPanel.svelte` (the bar: bank letters, 8 buttons, Store, Clear) | The Quick Racks bar of Kit additions, in the kit's tokens; the same commands |
| Screenshot tool | `app/scripts/shots.ts`: one 1000 × 600 viewport, no masks | The Stage lane's item (Stage.md › Gap, D39): per-story viewport and masks. This page's board story needs it too; whichever lane builds first lands it |

## Contract changes needed

C1 blocks the build: without its keys `TipKey` doesn't type-check and the tooltip catalog test
fails. It lands first, as its own small contract PR. C2–C4 don't block.

1. **C1 · Tooltips** (`app/src/help/tooltips.ts`, `app/docs/controls.md`), **lands before the
   build.** New keys: `library.tab_styles` ("Styles": every style in the library, by folder, in
   Track order; Alt+S), `browser.open_file` ("Open file…": a style file from anywhere, added to
   the library and loaded; the library's types), `browser.load` ("Load": loads the highlighted
   style; while the band plays, at the next bar line; Enter), `display.compact` (the now-playing
   block: the loaded style, tempo, run state, chord and playing section, for the tall pages),
   `keystrip.detect` (where chord detection listens: lower, upper or the full keyboard),
   `keystrip.held` (the keys held, left hand as chord tones, right hand with octaves). Rewrites:
   `browser.filter` (name, file name, folder, a tempo or a time signature; the keys as in
   Keyboard above), `browser.row` (the dot marks the loaded style, ◀ ▶ the Track neighbours;
   click loads, or queues while the band plays), `browser.close` (Cancel or Esc: back to the
   Stage, stops a preview), `browser.queue` (the Load button while the band plays: "loads
   this style on the next bar line"; Enter), `quick.bank` (drop "in the Quick Racks drawer,
   click a letter"), `quick.clear` ("Clear: arm it, then tap a Quick Rack button to empty it;
   the rack itself stays in your racks"), `nav.styles` ("Shows Library › Styles; press again
   for the Stage"), `nav.library` / `view.library` ("shows the Library page": styles, sounds,
   instruments, racks and the style map; the band stays).
2. **C2 · `ui.libraryTab` gains `'styles'`** (`app/src/lib/store.svelte.ts`, `lib/nav.ts`,
   `lib/keys.ts`, `lib/shortcuts.ts`): the Browser is a Library tab (BR-D1); `ui.browser` and
   its branches go; the Enter binding and `nav.styles` open the tab. Until it lands the page
   wiring keeps a local flag and `ui.browser` opens the page. Not a contract file by AGENTS.md's
   list, but shared with the other Library lanes: it lands with C1. Stage.md D3 and its style
   line say the name sets `ui.browser = true`; with C2 that reads `ui.page = 'library'` +
   `libraryTab = 'styles'` (a one-line Stage.md edit when C2 lands).
3. **C3 · File chooser** (`app/src-tauri`: the Tauri dialog plugin and a `pickStyleFile`
   command, or the dialog plugin's JS API; `app/src/lib/api/*` for the dev mock): Open file…
   needs the system open panel. Until then the button is disabled with its tooltip (BR-D5).
4. **C4 · Unqueue** (`AppCmd`, both mocks, `EVERY_CMD`, docs/app-api.md): there is no way to
   cancel a queued style (`preview.queued`) short of loading another. A `cancelQueuedStyle`
   command would let Cancel un-queue. Until then Cancel only leaves the page (BR-D13) and the
   queued style lands.

## Checks

Vitest (`npx vitest run` on `app/src/ui/Browser` and the component tests), each against the
board fixture unless it says otherwise. They read roles, names, attributes, `data-face` /
`data-hue` and the commands sent (a fake `send`), never computed colours or layout.

1. Rows: nine options in the listbox; their names in order (Unplugged Ballad Pop … Motown Pop
   Soul); the third (`Coastal Highway`) has `aria-selected="true"` and `data-face="chosen"`, and
   contains the "next bar" tag; the fifth has the loaded dot (`data-hue="ok"`) and no glyph;
   the fourth's Track cell reads "◀", the sixth's "▶"; the row `aria-label` of the third is the
   template's example.
2. Pure (`model.ts`): `sectionLamps` on each of the nine summaries gives the lamp patterns in
   the fixture; `visibleRows` in Pop & Rock with query "104" returns Sunday Drive Pop (the
   tempo) and Soul Style 104 (the name), "12/8" only Pop Shuffle, "shuffle" only Pop Shuffle,
   "rock" the two Rock rows (the folder) and the 212 don't all match "8beat"; with category
   `recents` the order is `prefs.recents`'s; `folderName(entry, category)` (new, BR-D10) gives
   "8Beat" for row 1 in the folder and "Pop & Rock/8Beat" in All.
3. Folders: eleven `<button>`s in the nav in library order (Ballad, Ballroom, Country, Dance,
   Entertainment, Latin, Movie & Show, Pop & Rock, R&B, Swing & Jazz, World) with their counts;
   "Pop & Rock" has `aria-current="true"` and `data-face="chosen"`; no view tab is `aria-selected`; clicking
   "Ballad" makes it chosen, deselects Pop & Rock, and the header reads "Styles Ballad 98";
   clicking the All tab deselects every folder and the header reads "1,284".
4. Header count: with query "shuffle" the count reads "1 of 212" and the All tab still "All
   1,284"; with `library.pending` 40 it reads "212 · 40 indexing".
5. Load button: reads "Load Coastal Highway" and "next bar", `data-face="waiting"`,
   `aria-disabled="true"`, and a click sends nothing; with the cursor on Waltz Pop (ArrowDown
   three times in the filter) it is `data-face="off"` and a click sends `queueStyle { id }`;
   with `transport.running` false and the cursor on Waltz Pop a click sends `loadStyle { id }`
   and `ui.page` becomes `stage`; on Sunday Drive Pop it is `aria-disabled`.
6. Row click: on Waltz Pop while playing sends `queueStyle`; stopped, `loadStyle`; on the
   loaded row, and on the queued row while playing, nothing; the cursor moves in every case
   and the filter input keeps focus.
7. Keyboard (the filter focused): ArrowDown moves `aria-activedescendant` to Pop Shuffle's id;
   End to the last row (Soul Style 203) and Home to the first; PageDown from the first to the
   ninth; Enter = check 5's click; Shift+Enter while playing sends `queueStyle` for the cursor
   row, stopped `auditionStyle`; Ctrl+D toggles the cursor row's star (`aria-pressed`); Escape
   sets `ui.page` to `stage`; typing "walt" leaves one row and the cursor on it.
8. Preview: while playing every row's ▶ is `aria-disabled` and sends nothing; stopped, Waltz
   Pop's ▶ sends `auditionStyle { id }`; with `preview.audition = { id: Waltz Pop, bar: 2, bars:
   4, chord: 'Am' }` that row's button reads "■", `data-hue="a"`, sends `stopAudition`, and the
   footer note reads "Previewing Waltz Pop · bar 2/4 · Am".
9. Preview on select: the lamp has `aria-pressed="true"`; a click calls `prefs.setAutoPreview
   (false)`; stopped, with it on, hovering Waltz Pop and advancing fake timers 600 ms sends
   `auditionStyle`; leaving the listbox before 600 ms sends nothing; playing, nothing.
10. Star: Brit Pop Anthem's star has `aria-pressed="true"`; a click calls `toggleFavourite`
    with its path and doesn't load (no command sent).
11. Empty states: with query "zzz" the listbox has no options, the text "No style matches
    “zzz”", and the Load button "Load" is `aria-disabled`; with `app.library.entries` empty and
    `library.count` 1284, "Reading the library…"; with category `favourites` and no favourites,
    "No favourites yet: star a style with ☆ (or Ctrl+D)."
12. Error entry: with row 7's status `error` ("not a style file"), its row shows that text,
    its ▶ is `aria-disabled`, Enter on it sends nothing, and its `aria-label` ends ", unreadable:
    not a style file".
13. Open file…: `aria-disabled` while the wiring has no chooser; with a fake chooser resolving
    "/tmp/x.sty", a click sends `loadStylePath { path: '/tmp/x.sty' }`; resolving null sends
    nothing.
14. Library pages: five rows reading "Styles 1,284", "Sounds 886", "Instruments 4", "Racks 10",
    "Style map"; Styles has `aria-current="page"`; clicking Sounds calls the opener with
    `sounds` (interim: today's Library on that tab).
15. Quick Racks bar: slots 1–8 read "A1 Sunday drive" (`data-face="chosen"`), "A2 Warm keys",
    "A3 Lead synth", "A4 Organ" (`data-face="off"`), "A5 Empty"… (`data-face="empty"`,
    `aria-disabled` is **not** set: an empty slot is still a store target); a click on A2 sends
    `pressQuickRack { slot: 1 }`; a 350 ms press on A5 sends `storeRack { slot: 4 }` and no
    press; Store sends `toggleQuickRackStore`; with `quickRacks.store` true every slot has
    `data-face="waiting"`; ✕ then A2 sends `clearQuickRack { bank: 0, slot: 1 }` and disarms;
    ✕ then A5 (empty) sends nothing and stays armed; ✕ while `quickRacks.store` is true sends
    `toggleQuickRackStore`; ◀ is `aria-disabled` on bank A, ▶ sends `stepQuickRackBank
    { delta: 1 }`.
16. Compact block: reads "Sunday Drive Pop", "104" "BPM", the dot `data-hue="ok"` (stopped:
    hidden), the chord runs "Am" and "7", the tones "A C E G", "Main B" `data-hue="main"`;
    stopped, "Main A"-style fallback in `data-hue="m"` (Stage.md D4 with `transport.main`).
17. Half band: nine strips with the Stage's values; strip 2 shows "↕"; a 10px upward drag on
    strip 1 sends its `set` with `volume` 119 (90 + round(10 × 127 / 44)); a 20px drag sends
    127 (clamped from 148); the Track ◀ sends `surface.controls[trackPrev].action`;
    pad 10 `data-face="solid"`, pad 11 `data-face="waiting"`; the transport's ▶ has
    `aria-pressed="true"` and sends `startStop`; knob 8 reads "104".
18. Keys status row: "Split F#2", "Detect lower", "Left G A C E" (`data-hue="l"`), "Right E4
    A4" (`data-hue="r1"`), the "61 keys" button; with `detection` [55, 127] "Detect upper", with
    [0, 127] "Detect full"; no held keys: "Left —", "Right —"; held 43 and 55 (both G) in the
    left zone: "Left G" once; a click on the keys button calls `ui.setKeyRange(88)` (61 → 88 →
    49 → 61).
19. Every interactive element has a `data-tip` in the catalog (the existing tooltip test).
20. Tab order: the focusable elements in DOM order are the sequence in Keyboard › Tab order;
    no option, star or ▶ has `tabindex` ≥ 0.

**Story and screenshot checks** (`npm run shots -- Browser`, real Chrome):

- `Pages/Browser` › `Board` (export `Board`, layout `fullscreen`, `parameters.shots = {
  viewport: { width: 1440, height: 900 }, mask: ['[data-shot-mask="when"]',
  '[data-shot-mask="folders"]'] }`; the folders nav carries `data-shot-mask="folders"`) renders
  `Browser` with `boardState`, `boardLibrary`, `boardPrefs`, `boardNow`, `boardMeterHolds`,
  unscaled, in both themes, against `app/src/ui/Browser/crops/Board-dark.png` and
  `Board-light.png` (copies of `docs/design/push/png/Browser-Dark.png` and `Browser-Light.png`):
  at most 0.02 of the unmasked pixels differ. Needs the shots.ts item in the gap table.
- The same story covers what vitest can't: the chosen block's mixed text, the lamps' two
  tones, the queued tag and Load border at full (LED phase 0.25), the half band's meters, rings
  and pads, the key strip.
- `Pages/Browser` › `Stopped` (the board state with `transport.running` false, `preview.queued`
  null, `preview.audition` Waltz Pop bar 2): ▶ enabled, "■" on Waltz Pop, the note, the Load
  button "Load Coastal Highway" without "next bar", the compact block's dot hidden; no crop,
  judged by Inspect.
- `Pages/Browser` › `Empty` (query "zzz") and `Reading` (no entries): the texts centred; no crop.
- `Components/StyleRow` › `LongName` (a 60-character name, queued, in a folder with a
  20-character subfolder): the name ends in an ellipsis before the tag, the folder cell in an
  ellipsis, the row stays 32 tall; no crop.
- `Components/CompactBlock` › `LongChord` (`C#m7b5/G#`, section "Ending III"): the tones hide
  and the chord fits 320 (Kit additions › Compact block); no crop.
- axe finds no violation on any story.

## Kit additions

What this page needs that kit.md doesn't have. The Channel spec (#501) owns the half band, the
compact block, the page app bar and the keys status row by kit.md's own note; whichever of
#501 and this spec lands first puts them in kit.md, and the other refers to it (BR-D2). The
Library frame (Library pages, Quick Racks bar) is shared by the five Library boards and goes in
kit.md too.

### Tokens

Add to `palette.css` and both theme files:

| Token | Dark | Light | Used by |
|---|---|---|---|
| `--sel-mut` | `color-mix(in srgb, var(--g) 60%, transparent)` | `color-mix(in srgb, var(--g) 70%, transparent)` | muted text on a chosen block (a row's folder, file, Track glyph; a page's count) |
| `--sel-off` | `color-mix(in srgb, var(--g) 20%, transparent)` | `color-mix(in srgb, var(--g) 30%, transparent)` | unlit lamps and the disabled ▶ on a chosen block |
| `--slot-glow-mix` | `25%` | `0%` | the loaded Quick Rack slot's 10px glow: `0 0 10px color-mix(in srgb, var(--t) var(--slot-glow-mix), transparent)` |
| `--stored-ring` | `inset 0 -2px 0 color-mix(in srgb, var(--t) 40%, transparent)` | the same | a stored Quick Rack slot's bottom lamp |

Scale: `--text-8` (the half transport's ▲ ▼), `--text-15` (the row's BPM and Time, the half
fader value), `--text-48` (the compact chord), `--text-24` (the compact section),
`--travel-half: 44px` (the half fader's travel). The preview dwell (600 ms) is the wiring's
`PREVIEW_DELAY_MS`, not a token: it drives a timer, not a style.

The chosen row's text is `--g` (the kit's chosen face), also in light where the board painted
`#ffffff` (BR-D17).

### App bar, page variant

On every page that isn't the Stage (the Stage's display is gone, so its rack readout and One
Touch ride here), between the wordmark and the page tabs:

- **Rack readout**, `margin-left: 16px`, a text button 32 tall, items on the baseline, gap 6,
  14 / 400, no wrap: "Rack" `--m`, the slot "A1" `--t`, the name `--t` (`min-width: 0`,
  `max-width: 160px`, ellipsis: the one thing in the bar that shrinks; everything else is
  `flex: none`, so a long rack name never pushes the tabs), then the 5px `--t` modified dot
  (centred). Reads, sends, tooltip (`stage.rack_name`) and
  `aria-label` as Stage.md › Sounds row › Rack; only the layout differs (one line, no "Rack ·"
  separator: the slot follows "Rack" after the gap).
- **One Touch**, `margin-left: 12px`: Stage.md › Style line › One Touch, unchanged (the group,
  the four 32 × 32 buttons, `recallOts`, `ots.1`…`ots.4`).
- The tabs keep `margin-left: auto`; the Library tab is chosen (`aria-current="page"`) on every
  Library page; the Stage tab carries `aria-label="Back to stage"` here.

### Compact block

`320 × 84` at the top of a tall page's left column (`aria-label="Now playing"`, `role="group"`,
tooltip `display.compact`, not a control; `data-hue` hooks as the Stage's). A column of 16 +
12 + 48 = 76px, with 8px of the block's height empty below it (the block is 84 so the next
part's 16px margin lands where the board has it):

- **Row 1**, 16 tall, items centred, gap 8, no wrap: the style name as an accent block (`size
  compact`: padding 0 6, 13 / 500, line-height 16, `--g` on `--a`, `min-width: 0`, ellipsis;
  `style.name`; a span, not a button: the Browser is open); `margin-left: auto`, the tempo
  `transport.tempo` rounded, 18 / 300, line-height 16, `--t`, then "BPM" 12 / 400 `--m` with
  3px left margin; then the run-state dot (6px): running → `--ok` with `--bg`; stopped with
  `syncStart` → a hollow 1px `--ok` ring; stopped → `visibility: hidden` (space kept). `role="img"`,
  `aria-label` "Running" / "Sync start" / "Stopped".
- **Row 2**, 12 below, 48 tall, items on the baseline, gap 14, no wrap: the chord at 48 / 300,
  line-height 48, letter-spacing −2, `--a`, `text-shadow: var(--ba)`, in the Stage's two runs
  (`splitChord`, Stage.md D30: base 300, extension 200 with letter-spacing 0); the tones 14 /
  300, letter-spacing 3, `--t2`: the note names of the chord shown (Stage.md › Chord: moved by
  `transposeKeyboard`, spelled by the root, at most six, space-separated; `min-width: 0`,
  `overflow: hidden`); `margin-left: auto`, the playing section (kit › Section names) 24 / 300,
  line-height 48, letter-spacing −0.5, in its hue, no glow (`flex: none`); stopped, the Main
  the band will start on in `--m` (Stage.md D4). No chord: "—" in `--d`, no shadow, no tones.
- **Fit:** the chord and the section are `flex: none`; the tones shrink first (ellipsis). When
  the chord at 48px plus 14 plus the section is wider than 320, the chord shrinks to
  `max(28, floor(48 × (320 − 14 − sectionWidth) / chordWidth))` px, measured as the Stage's chord
  is (`scrollWidth` at 48px after each change), letter-spacing −2 × size / 48, and the tones are
  hidden.
- `aria-label` on the group: "{style name}, {tempo} BPM, {run state}, chord {name}: {tones},
  {section} playing" ("Sunday Drive Pop, 104 BPM, running, chord Am7: A C E G, Main B
  playing"; stopped: "…, stopped, …, starts on Main A"; no chord: "no chord").

### Library frame

A tall Library page's left column, `320 × 436`, a column: the compact block (84), then
(16 below) the **Library pages**, then (12 below) the **Quick Racks bar**. Then a 1px `--line`
hairline, 24px to its right, full height, and the page content 24px right of that.

#### Library pages

`<nav aria-label="Library pages">`, five ListRows (32 tall, padding 0 8, `border-bottom: 1px
solid var(--line)`, items centred, gap 12, left-aligned, no wrap: the name 14 / 400 `--t2` flex
1, the count 13 / 300 `--m`): "Styles" `library.count`, "Sounds" `soundLibrary.patches.length`,
"Instruments" `plugins.list.length`, "Racks" `racks.length`, "Style map" (no count); counts
with thousands separators. The current page (`aria-current="page"`) is the chosen face as a
block (`--t` fill, name `--g`, count `--sel-mut`, bottom border `--t`). Click: `ui.libraryTab`
(`styles` | `sounds` | `instruments` | `racks` | `map`); until a page's spec is built, its row
shows today's `panels/library/Library.svelte` on that tab (BR-D18). Tooltips
`library.tab_styles` (new), `library.tab_sounds`, `library.tab_instruments`,
`library.tab_racks`, `library.tab_map`. `aria-label` "{name}, {count}" ("Sounds, 886"). The
current page's name is at weight 500. Shortcuts: Styles Alt+S and Style map Alt+Y (existing);
Sounds, Instruments and Racks have none (Alt+R is the Quick Racks page tab, kit › App bar;
today's `nav.ts` entry for Alt+R changes with the kit's app bar, not here).

#### Quick Racks bar

`<section aria-label="Quick Racks">`, a column: the header, then the slots 8px below it (the
board's `gap: 4px` plus the grid's `margin-top: 4px`):

- **Header** (GroupHeader `size sm`: 32 tall, `border-bottom: 1px solid var(--line)`, items
  centred, gap 8, no wrap): "Quick Racks" 14 `--m`; **◀** and **▶** 28 × 28 off face, 11px
  (`stepQuickRackBank { delta: -1 | 1 }`; ◀ disabled on bank A, ▶ on bank H; tooltips
  `quick.bank_prev`, `quick.bank_next`; `aria-label` "Previous bank" / "Next bank"; Launchkey:
  Racks pad page Bank −/+) around the **bank letter** 18 / 300 `--t` (`"A"` + `quickRacks.bank`;
  not a control; `aria-label` "Bank A of 8"; tooltip `quick.bank`); `margin-left: auto`,
  **Store** LampButton `size sm` (on = `quickRacks.store`; sends `toggleQuickRackStore`;
  disabled when `readOnly`; tooltip `quick.store`; Launchkey: Racks page Store); **✕** 28 × 28
  off face with a 10 × 10 cross (`<svg viewBox="0 0 12 12"><path d="M2.5 2.5 L9.5 9.5 M9.5 2.5
  L2.5 9.5" stroke="var(--t2)" stroke-width="1.2"/></svg>`): arms **Clear**, app-only
  (`aria-pressed`; armed, the button wears the waiting face in `--ending`: transparent, 1px
  `--ending` border, the cross in `--ending`); the next press on a stored or loaded slot sends
  `clearQuickRack { bank, slot }` and disarms; a press on an empty slot does nothing and stays
  armed; a long press while armed does nothing; pressing ✕ again, Esc, or leaving the page
  disarms. Arming Clear while Store is armed sends `toggleQuickRackStore` (Store disarms);
  pressing Store while Clear is armed disarms Clear first. Disabled when `readOnly`; tooltip
  `quick.clear`; `aria-label` "Clear: arm, then tap a slot to empty it".
- **Slots** (8px below the header): a grid `repeat(4, minmax(0, 1fr))`, gap 4, rows 32 (77 × 32
  at 320): eight QuickSlot buttons for `quickRacks.buttons[0..7]`, each padding 0 6, radius 4,
  items centred, gap 5, left-aligned, `overflow: hidden`: the index "A1"… (the bank letter and
  slot + 1) JetBrains Mono 11, `flex: none`, then the name 12 / 400, line-height 13, wrapping
  (`white-space: normal`, at most two lines, clipped).

  | Face (`data-face`) | When | Fill | Text | Index | Edge |
  |---|---|---|---|---|---|
  | `empty` | `rack` null | none | "Empty" `--m` | `--d` | `inset 0 0 0 1px var(--d)` |
  | `off` (stored) | a rack, not loaded | `--btn` | `name` `--t` | `--m` | `--stored-ring` |
  | `chosen` (loaded) | `loaded` | `--t` | `--g` | `color-mix(in srgb, var(--g) 55%, transparent)` | glow at `--slot-glow-mix` |
  | `waiting` | Store armed (every slot, in `--lamp`), `storeWaiting` = this slot (steady, `--lamp`), or Clear armed (stored and loaded slots, in `--ending`) | as its face | as its face | as its face | `1px solid <hue>` in place of the edge; Store armed: flashing on the LED clock as a Next pad's border |

  A `missing` slot wears `off` with the name in `--warn` after an 11px ⚠. Click: `pressQuickRack
  { slot }` (with Store armed the session stores; with Clear armed the screen sends
  `clearQuickRack` instead). Long press (350 ms) or right-click: `storeRack { slot }` (one-step
  capture; nothing while `readOnly`). Tooltips `quick.1`…`quick.8`. `aria-label` "Quick Rack
  {A1}, {name | empty}{, loaded}{, rack missing}". Launchkey: the Racks pad page's top row;
  hold Sound + a pad = `storeRack`. Every slot is focusable (an empty one is a store target).

### Half band

`1392 × 176` under a tall page, margin-top 20: four sections in a row, gap 12: **Faders**
`24,600 614×176`, **Track** `650,600 40×176`, **Knobs and Pads** `702,600 614×176`, **Transport**
`1328,600 88×176`. Every section starts with a 36-tall hairline header. The same commands,
tooltips and Launchkey mappings as the full band (kit › Full band); only the drawing changes.

#### Faders (half)

Header as kit › Faders (the page tabs and layer tabs, the "Faders · Reverb" rule). Then (8
below) a 132-tall column: the strips row (84), the lamp-row headers (14), and the lamp row (32,
2px below).

- **Strips row** `24,644 614×84`, `repeat(9, minmax(0, 1fr))`, gap 8 (61.1 wide). Each
  FaderStrip `size half`: a 66-tall fader button over an 18-tall name button.
  - **Value** at the top, 16 tall, centred, 15 / 300 in the hue (layers and faces as the full
    strip).
  - **Travel** 44px, from top 19 to bottom 3 (`--travel-half: 44px`). Meter bars 6 wide at
    `50% − 11px` and `50% − 3px` on `--mbg`, peak left and RMS right, in the hue at
    `--meter-mix`, growing from bottom 3: `heightHalf(x) = round(44 × clamp((20·log10(x) + 60) /
    60, 0, 1))`. The peak tick 14 × 1 `--peak` at `50% − 11px`, bottom `3 + heightHalf(hold)`.
  - **Groove** 3 wide at `50% + 6px`, top 19 bottom 3, `--track`; fill from bottom 3 to the
    level (`round(44 × value / 127)` tall) in the hue with `0 0 6px` at `--fill-glow-mix`; the cap
    8 × 2 at `50% + 1px`, `top = round((1 − value/127) × 44) + 18`. Soft takeover: "↕" 12px `--m`
    at top-left, and a 30-wide dashed 1px `--m` line at `50% − 15px`, `top = round((1 −
    position/127) × 44) + 19`. Unused: the dashed groove (kit) and no cap; part off: fill and cap
    at 35%.
  - **Drag:** as the kit's fader with 44 for 223: `value = clamp(round(v0 + (y0 − y) × 127 /
    44), 0, 127)`; wheel, keys, double-click as the kit.
  - **Name button** 18 tall, centred, gap 4: the name 12 / 500 in the hue, then PartMarks at
    11px (the dot 5px). Clicks as kit › Faders.
- **Lamp-row headers** `24,728 614×14`: as kit › Lamp row at 12px, line-height 12, bottom
  hairline.
- **Lamp row** `24,744 614×32`: kit › Lamp row, unchanged (LampButton `cell`, 12px labels, the
  divider after column 4).

#### Track

Header "Track" 14 `--m`. Then (8 below) a column, gap 8: **◀** and **▶**, 40 × 32 off face,
12px: Stage.md › Style line › ◀ ▶ (`surface.controls[trackPrev | trackNext]`, Shift, disabled
rule D38; tooltips `style.prev`, `style.next`; `aria-label` "Previous style (Track left)",
"Next style (Track right)"). On this page they also move the list's cursor to the style that
loads (Keyboard above).

#### Knobs and Pads (half)

One header `702,600 614×36`, items centred, gap 8, no wrap: "Knobs" 14 `--m`; the knob page as
an accent block (kit › Knobs: padding 0 8, 13px, line-height 22; swap mode on the part's hue);
**▲ ▼** 28 × 28 off face, 11px (`stepKnobPage`, disabled as the kit); "Page" 13 `--m` with
`{pageNumber}/{pageCount}` `--t`; `margin-left: auto`, "Pads" 14 `--m`; `pads.pageName` 14 `--t`
and, on Sections while something is next, " · next " in `--t` then the next section's name in
its hue (kit › Count row item 3 gives which); **▲ ▼** 28 × 28 (`surface.controls[padBankUp |
padBankDown]`); "Bank" 13 `--m` with `{pads.pageNumber}/{pads.pageCount}` `--t`. No legend.

- **Knobs row** `702,644 614×48`, `repeat(8, minmax(0, 1fr))`, column-gap 6 (71.5 wide). Knob
  `size half`, a button 48 tall, a column centred: a 32px ring (`conic-gradient(from 225deg,
  var(--a) 0 <deg>, var(--ring-rest) <deg> 270deg, transparent 270deg)`, a `--g` disc inset 2)
  with the **value inside** it, 12 / 300 `--a`, no wrap (the "%" stays in the text); a 4px `--a`
  tip dot centred at radius 15 from the ring's centre (`left = 16 + 15·sin θ − 2`, `top = 16 −
  15·cos θ − 2`, θ = 225° + deg); then the **name** 12 / 400 `--t2`, 14 tall, 2px below (the
  kit's plain word, Stage.md D6). No Assign: ring and rest `--mbg`, no dot, no value, name "—"
  in `--d`. No code line. Drag, wheel, keys, `aria-valuetext`, tooltip as kit › Knob.
- **Pads row** `702,700 614×60`, `repeat(8, minmax(0, 1fr))`, column-gap 6, rows 28, row-gap 4
  (71.5 × 28). Pad `size half`: radius 4, 1px border (transparent idle), padding 0 2, the
  caption 12 / 500, line-height 26, letter-spacing −0.2, centred, no wrap, clipped; no numeral,
  no bar except Next. Faces by `level` and `anim` (kit › Pad): Idle `--btn` with the caption in
  the family hue (utility `--t2`); Absent `--d`; Playing solid hue with `--solid-ink` (Start /
  Stop running: `--ok`); Next: border in the hue, `--btn`, caption `--t` with line-height 23,
  and a 2px bar (left and right 8, bottom 3, radius 1) in the hue with `0 0 6px` at
  `--bar-glow-mix`, flashing as the kit; Armed: border in the hue, `--btn`, caption `--t`,
  pulsing as the kit, no bar; On: `--lamp`. Captions: the kit's Sections captions with "Sync
  start" and "Sync stop" in sentence case, each followed (3px) by a CSS shape in `currentColor`:
  "Sync start" by ▶, a 6 × 7 triangle (`border-left: 6px solid; border-top and -bottom: 3.5px
  solid transparent`); "Sync stop" by ■, a 6 × 6 square; "Auto fill"; "Start" for Start / Stop,
  running or not (the face says which). Other pages: the fallback (Stage.md D33) at this size.
  `aria-label` as the kit's, with the kit's captions ("Start / Stop (pad 16), playing"; "Sync
  Start (pad 4)"), action and tooltips as the kit.

#### Transport (half)

Header "Transport". Then (8 below) a grid `40px 40px`, rows 24, gap `3px 8px`, five rows
(5 × 24 + 4 × 3 = 132): **▶ | ■**, **Reset | Fade**, **Fill ▲ | Fill ▼**, **+ | −**, **Style
tempo** (`grid-column: 1 / 3`). All off face, radius 4, centred: ▶ 12px `--t`, `aria-pressed` =
running, with the running 2px `--ok` bar (left and right 8, bottom 3, `--bg`) and 3px bottom
padding; ■ 12px `--t2`; Reset, Fade, Fill, Style tempo 11px; the ▲ ▼ at 8px; + and − 14 / 300.
Sends, faces (Fade's waiting and on), tooltips, repeats and Launchkey as kit › Transport and
tempo. `aria-label`s: "Start / Stop" (+ ", running"), "Stop", "Section reset", "Fade" (+
", armed" / ", fading"), "Fill up", "Fill down", "Tempo up", "Tempo down", "Style tempo".

### Keys status row

`24,796 1392×20` on a tall page, items centred, gap 16, no wrap, then the key strip 4px below
(`24,820 1392×56`, kit › Key strip). Left to right, each item one text 13 / 400: a label in
`--m`, a normal space, then its value in `--t` unless said otherwise:

- The **status line** (kit › Status line), `flex: 1`, `min-width: 0`.
- **Split** "Split" + the split key's name: `keyboard.leftSplit` as Yamaha pitch (C3 = 60,
  sharps): "F#2" for 54. Tooltip `keystrip.split`. Not a control here.
- **Detect** "Detect" + "lower" (`keyboard.detection` = `[0, split]`), "upper" (`[split + 1,
  127]`), "full" (`[0, 127]`); anything else "{lo}–{hi}" as key names. Tooltip `keystrip.detect`
  (new).
- **Left** "Left" + the held keys whose `zone` is `left` (`keyboard.held[].zone`) as note
  names without octave, each pitch class once, in ascending order of the keys, spelled as the
  Stage's tones (flats when the chord's root has a flat or is F, else sharps), in `--l`
  ("G A C E"); none: "—" in `--m`.
- **Right** "Right" + the held keys whose `zone` is `right`, with octaves, sharps, ascending
  ("E4 A4") in `--r1`; none: "—" in `--m`. Tooltip `keystrip.held` (new) on Left and Right.
- **Keys** a text button 20 tall, 13 `--t`: "{n} keys" (`ui.keyRange` or the Launchkey's);
  click cycles 61 → 88 → 49 → 61 (`ui.setKeyRange`); tooltip `keystrip.range`; `aria-label`
  "Keyboard size {n} keys. Click for 49, 61 or 88".

## Decisions

- **BR-D1 · The Browser is Library › Styles.** `ui.page = 'library'` with `ui.libraryTab =
  'styles'` (a new tab value, C2), not today's modal. The Stage's style name (Stage.md D3) and
  Alt+S open it; the Stage tab, Cancel and Esc go back. `ui.browser` goes with the modal.
- **BR-D2 · Tall-page parts.** The half band, compact block, page app bar, keys status row and
  the Library frame are in Kit additions here. kit.md says the Channel spec adds them; whichever
  of #501 and this spec lands first moves them into kit.md, and the other refers to it. Values
  here are from this board, which copied them verbatim from Channel-Dark (the two boards'
  half-band markup is identical).
- **BR-D3 · Build order.** This spec assumes the Stage's contract (its tooltip keys, the kit
  tokens, `longpress`, the shots.ts masks) is in; if the Browser is built first it lands them.
- **BR-D4 · One category.** The views (All, Favourites, Recent) and the folders are one choice
  (`prefs.category`): choosing one deselects the other. The header names it. Remembered on this
  computer (as today).
- **BR-D5 · Open file… until the chooser lands.** Disabled with its tooltip (the kit's interim
  rule), not hidden: the board draws it.
- **BR-D6 · Filter.** Today's substring match on name, file stem and folder, plus a whole number
  = tempo and `n/d` = time signature, so the board's placeholder is true. No tokens, no ranges.
- **BR-D7 · Flat folders.** The folder list shows the top-level folders only, with counts that
  include subfolders, in library order (the engine sorts by lowercased folder name, so the
  list is alphabetical; the board's Pop & Rock-first order is placeholder data and the shot
  masks the list); the subfolder shows in the Folder column. Today's indented tree is dropped
  (the board has none); a drill-down is a follow-up.
- **BR-D8 · No scrollbars.** The list and the folder list hide their scrollbars (the board
  draws none and has `overflow: hidden`); the wheel and the keys scroll, and the cursor is
  always in view. A position indicator is a follow-up.
- **BR-D9 · No sort.** Order is the library's (folder, then name) so Track ◀ ▶ and the list
  agree; Recent is newest first. Favourites keep library order.
- **BR-D10 · Folder cell.** In a folder: the path below the chosen folder, empty for a file
  right in it; in All, Favourites and Recent: the whole folder. The board's single words are
  the subfolders of Pop & Rock.
- **BR-D11 · Load.** A row click, Enter and the Load button do the same: stopped, `loadStyle` and
  back to the Stage (today's browser closes on load); playing, `queueStyle` and the page stays so
  the player sees it land. The queued row is the cursor on open, and the Load button on it is
  the waiting face, disabled.
- **BR-D12 · Footer lamp.** Preview on select is the kit's LampButton `md` (14px, padding 0 16);
  the board's 13px / 0 14 is a drift and the tokens win.
- **BR-D13 · Cancel.** Back to the Stage without loading (and stops a preview). It can't unqueue
  (no command, C4); the queued style lands.
- **BR-D14 · The filter drives the list.** As today: the input keeps focus (the rows and their
  buttons swallow `pointerdown`'s focus), the listbox is `aria-activedescendant`-driven, rows
  and their buttons are not in the tab order; the stars and previews have keys (Ctrl+D,
  Shift+Enter). Other controls take focus normally; the global keys are not gated by the page.
- **BR-D15 · Cursor on open.** The queued style, else the loaded one, centred; it follows a
  queue or a style change from anywhere, and the category switches to All when that style
  isn't in it (a Track press while browsing Ballad shows where the band went).
- **BR-D16 · Dwell timer.** Preview on select's 600 ms is a gesture measure like the long
  press (kit › Interaction conventions), not motion, so it is allowed.
- **BR-D17 · Chosen text in light.** `--g` (`#f2f1ee`), the kit's chosen face, where the light
  board's script painted `#ffffff`; under the diff threshold.
- **BR-D18 · Other Library pages until their specs land.** Choosing Sounds, Instruments, Racks
  or Style map shows today's `Library.svelte` on that tab in place of the whole page (the
  kit's interim rule); the Styles row brings this page back.
- **BR-D19 · Clear is app-only.** ✕ arms a screen-side clear; the next press on a stored or
  loaded slot sends `clearQuickRack`. Store is the session's (`quickRacks.store`). One of them
  is armed at a time (arming one disarms the other). Both show as the waiting face, Store in
  `--lamp` (flashing, as the hardware's pads do), Clear in `--ending`.
- **BR-D20 · Empty slot presses.** An empty slot is focusable and pressable: a click sends
  `pressQuickRack` (the session refuses it and says so), a long press stores there. It isn't
  drawn disabled.
- **BR-D21 · The preview column while playing.** ▶ stays, disabled, so the row's shape doesn't
  change when the band stops; the footer's "While stopped" says why.
- **BR-D22 · Half strip travel.** 44px (top 19 to bottom 3) with the full strip's maths scaled;
  drag resolution follows (one px ≈ 3 steps), wheel and arrows stay ±1.
- **BR-D23 · Keys status row.** Left shows the held left-hand keys as chord tones (no
  octaves), Right the held right-hand keys with octaves, as the kit's key-strip `aria-label`
  reads them. Detect reads lower / upper / full from `keyboard.detection`.
- **BR-D24 · Compact block's style name is not a control** on the Browser (it would open the
  page we're on). On other tall pages it opens the Browser (their specs say so).
- **BR-D25 · Counts.** Library pages' counts read `library.count`, `soundLibrary.patches.length`,
  `plugins.list.length`, `racks.length`; Style map has none. Thousands separators everywhere a
  count is shown, always `en-US` grouping (`toLocaleString('en-US')`), so tests and shots don't
  follow the machine's locale.
- **BR-D26 · Error rows stay.** Unreadable files are listed (the engine lists them) with their
  reason, can be starred and filtered, never load; Track skips them (the engine does).

## Follow-ups

- C3 (the file chooser) and C4 (unqueue) as their own PRs; the Open file… button and Cancel
  pick them up with no spec change.
- A folder drill-down (subfolders in the folder list) if libraries with deep trees need it
  (BR-D7).
- A scroll position indicator for 60,000-style libraries (BR-D8).
- Per-row Track neighbours when the queued style, not the loaded one, is the step origin
  (`stepStyle` steps from the queued style: the ◀ ▶ marks follow `surface.trackPrev/Next`,
  which the engine already computes from it; nothing to do unless the engine doesn't).
- The compact block as a button on the other tall pages (BR-D24).
- Half-band stories and crops shared with the Channel spec once kit.md owns them (BR-D2).

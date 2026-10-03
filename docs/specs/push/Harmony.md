# Harmony

The Harm/Arp display page: the Keyboard Harmony / Arpeggio switch, the type, and the settings
the type has, with the compact now-playing block beside it. The band and the keys stay below it,
so the player sets up the harmony without leaving the band.

- **Issue:** #505 · **Flow:** Pads, loops, harmony · **Boards:**
  `docs/design/push/Harmony-Dark.dc.html`, `Harmony-Light.dc.html`; pictures
  `docs/design/push/png/Harmony-Dark.png`, `Harmony-Light.png` (1440 × 900). The spec stands
  without them: every value a builder needs is below, in [kit.md](kit.md) or in
  [Stage.md](Stage.md); the board lines in the Components table are for cutting crops.
- **Built from:** [kit.md](kit.md): App bar (the page variant, in Kit additions below), Section
  row, Full band, Key strip, Status line, the faces and tokens; and the compact now-playing
  block (Kit additions). This file specifies the page's own right column: the header, the
  category tabs, the type grid and the settings column.
- **Copied from:** Stage (#500). The section row, band, status line and keys are the Stage's,
  unchanged; this page replaces only the display (`24,112 1392×300`). One band difference: the
  Harm/Arp lamp (fader button 5) is lit, because `harmonyArp.on` is true on the board.
- **Variants of this screen** (they spec only what differs): Harmony-Arp (#527): an arpeggio
  pattern selected, with the arpeggio's settings. The settings column below is a `kind` switch
  so that spec adds the `arpeggio` kind without touching the others (HA-D15).
- **Glance order** (what must read first, brightest to quietest): the chosen type (the white
  block in the grid) and the On lamp, then the chord (48px, accent), then the type name in the
  header. The page's only glows are the chord's `--ba`, the two `--bg` status dots (the
  compact block's run dot, the app bar's Launchkey dot) and the kit's band glows.
- **Before building:** the Stage lane (#500) must have landed: it builds the kit components
  this page reuses (AppBar, SectionRow, FullBand, StatusLine, KeyStrip, ChosenTabs, LampButton
  with `data-face`, Stage.md D41 and Components 2–4), `ui.page` (Stage.md D2), the
  page-wiring pattern (D46), `app/src/ui/Stage/Stage.fixtures.ts` and the shots masks (D39).
  Tooltips on components: a Svelte action can't go on a component, so each kit component that
  is a control takes a `tip` prop (the key) and puts `use:tip` on its own element; the Stage
  lane adds it to LampButton and the rest as it builds them (Stage.md Check 15 needs it too),
  and this page's new components take the same prop. Then Stage C5 (tooltips) and this page's
  C-HA1 (two tooltip keys) must land; the rest doesn't block (see Contract changes needed).

## Layout

At 1440 × 900, laid out like the Stage (Stage.md D1: the app shell scales it, the component
never does). Padding 24 all round; a column.

| Region | Box | What's in it | Spec |
|---|---|---|---|
| App bar | `24,24 1392×36` | wordmark, rack readout, One Touch 1–4, page tabs (Harm/Arp chosen), Launchkey status, health slot | Kit additions › App bar, page variant |
| Section row | `24,68 1392×32` | Accomp, count row, Metronome ▾, Unison, Panic, ? | kit › Section row |
| Display page | `24,112 1392×300` | compact block, divider, the Harmony / Arpeggio column | below |
| Band | `24,432 1392×368` | faders, knobs, pads, transport and tempo | kit › Full band |
| Status line | `24,800 1392×20` | `state.message` | kit › Status line |
| Keys | `24,820 1392×56` | the key strip, 61 keys | kit › Key strip |

Vertical rhythm: app bar, 8, section row, 12, display page, 20, band, 20 (the status line),
keys: 24 + 36 + 8 + 32 + 12 + 300 + 20 + 368 + 20 + 56 + 24 = 900.

## Display page

`24,112 1392×300`, ground, `position: relative`, `overflow: hidden`, no border (unlike the
Stage's display, Stage.md D42: this is a page, not the Stage's display; HA-D21). Inside, the
content box is `48,128 1344×268` (24 left and right, 16 top and bottom), a row, gap 24:

| Part | Box | Spec |
|---|---|---|
| Left column | `48,128 320×268` | the compact now-playing block at its top (`48,128 320×84`); the 184px under it is empty ground (the board's "air") |
| Divider | `392,128 1×268` | 1px `--line`, `aria-hidden` |
| Right column | `417,128 975×268` | a column: header, categories, the lists row |

The right column, top to bottom (heights; the rest, 24px at the bottom, is empty):

| Part | Box | Height |
|---|---|---|
| Header | `417,128 975×36` | 36 |
| Category tabs | `417,172 975×32` | 32, 8 below the header |
| Lists row | `417,212 975×160` | 160, 8 below the tabs: the type grid `417,212 580×160`, gap 24, the settings column `1021,212 371×160` |

### Compact now-playing block

Kit additions › Compact now-playing block, at `48,128 320×84`, reading the same fields as
the Stage's display. Nothing on this page differs from the kit's definition.

### Header

`417,128 975×36`, one row, `border-bottom: 1px solid var(--line)` (border-box: 35px of
content over the hairline), items centred, gap 12, no wrap. It is a `role="group"` with
`aria-label` "Harmony / Arpeggio: {typeName}, {on | off}" ("Harmony / Arpeggio: Standard Duet
1, on"; empty name: "Harmony / Arpeggio: no type, on").

| Control | Face | Reads | Sends / does | Tooltip | Launchkey |
|---|---|---|---|---|---|
| Title | "Harmony / Arpeggio" 14 / 400 `--m`; not a control | — | — | — | — |
| Type name | 14 / 400 `--t`, `min-width: 0`, ellipsis; not a control | `harmonyArp.typeName`; empty (library not loaded, or no name yet): "—" in `--d` | — | — | — |
| Category caption | 13 / 400 `--m`, gap 12 after the name; shown only in two cases (HA-D3), else absent; not a control | `mode` `harmony` with `harmonyArp.category` exactly "Echo": "Echo"; `mode` `arpeggio` with a non-empty `category`: "Arpeggio · {category}" ("Arpeggio · Up & Down"); any other case (the Harmony category, an empty or unknown category): no caption | — | — | — |
| On | LampButton `size md` (32 tall, padding 0 16, 14px), `margin-left: auto`, label "On" when lit and "Off" when not (HA-D4), code "Fader button 5" (12px, 6px after the label, `--code-opacity` when lit) | `harmonyArp.on` (`aria-pressed`, the face and the label all from this one field: the lamp is controlled by its `on` prop and keeps no state of its own, as every lamp on the band; a click sends and waits for the state) | `toggleHarmonyArp` | `harmony.switch` | fader button 5 (Panel page); the band's Harm/Arp lamp is the same switch |

`aria-label` of the lamp: "Harmony/Arpeggio on (fader button 5)" / "Harmony/Arpeggio off
(fader button 5)". Keyboard shortcut Shift+J (the `J` entry in `app/src/lib/keys.ts`, exists).

### Category tabs

`417,172 975×32`, one row, items centred, no wrap, gap 0 (the separator's margins and the
label's right margin are the only spaces; tabs inside a group touch). Two tab groups and a
label:

- **Harmony categories** (`role="group"` `aria-label="Harmony categories"`): three ChosenTabs
  `size row` (Kit additions): "Harmony", "Echo", "Multi Assign", padding 0 10, 13 / 400. Fixed:
  they don't come from the library (HA-D2).
- A 1 × 16 `--line` separator, 10px margins each side (`aria-hidden`).
- "Arpeggio" 13 / 400 `--m`, 4px right margin; not a control.
- **Arpeggio categories** (`role="group"` `aria-label="Arpeggio categories"`): one ChosenTabs
  `size row` per distinct `category` of `library().arpPatterns`, in order of first appearance,
  labelled as the state gives it ("Up & Down", "Random", "As Played", "Chord Stab", "Broken
  Chord", "Guitar", "Sequence" with today's library; HA-D2), padding 0 10 (the board's 9px was a
  fit tweak; at 10 the row is about 880px wide, inside 975). Library not loaded: no tabs after
  the "Arpeggio" label.

The chosen tab (`aria-pressed="true"`, chosen face as a 24px block centred in the 32px row,
`--g` text, weight 400 like the kit's page tabs) is the **viewed category** (HA-D1): an
app-only value `{ group: 'harmony' | 'arpeggio', name }` the page wiring keeps while the page
is mounted. On mount, and whenever the selected type changes (`harmonyArp.mode`,
`harmonyType` or `arpPattern` changes, from anywhere: the screen, the hardware, a rack load),
it is the selected type's category; a click on a tab sets it to that tab until the next such
change. The selected type's category is: `mode` `arpeggio` → `{ arpeggio, harmonyArp.category }`;
`mode` `harmony` → `{ harmony, "Multi Assign" }` when `harmonyArp.typeName` is "Multi Assign",
else `{ harmony, harmonyArp.category }` ("Harmony" or "Echo"; any other value counts as
"Harmony", HA-D20). When the viewed category has no tab (mode `arpeggio` with an empty
`arpPatterns`): no tab is chosen and the grid is empty; when the list arrives, the tab appears
chosen. Clicking a tab only changes what the grid shows; it sends nothing, so browsing never
stops an Echo or an arpeggio (changing the type does, app-api.md › Keyboard Harmony /
Arpeggio). Clicking the chosen tab does nothing (cursor stays `pointer`: it is an enabled
control). Unchosen tabs: `--m` text, no face. Tooltip `harmony.category` (new, C-HA1) on every
tab. The category tabs replace the kit's tab label rule with their own `aria-label`:
"{name}: {n} types" / "{name}: {n} patterns", "1 type" / "1 pattern" in the singular
("Harmony: 19 types", "Multi Assign: 1 type", "Up & Down: 5 patterns"), plus ", selected
type here" when the selected type is in it, and no ", chosen" (the chosen tab is read from
`aria-pressed`). No Launchkey mapping (the hardware has no type control beyond the switch).

### Type grid

`417,212 580×160`, a `role="group"` `aria-label` "{viewed category} types" / "... patterns"
("Harmony types", "Up & Down patterns"), a grid `repeat(4, minmax(0, 1fr))`, `grid-auto-rows:
32px`, column-gap 12, no row gap (cells 136 × 32), filled row by row. Its items are the types
of the viewed category:

| Viewed category | Items | Index sent with |
|---|---|---|
| Harmony | the entries of `library().harmonyTypes` whose `category` is "Harmony" and whose `name` is not "Multi Assign" (19 today, Data List order) | `setHarmonyType { index }`, the entry's index in `harmonyTypes` |
| Echo | the entries whose `category` is "Echo" (Echo, Tremolo, Trill) | `setHarmonyType { index }` |
| Multi Assign | the entry named "Multi Assign" | `setHarmonyType { index }` |
| an arpeggio category | the entries of `library().arpPatterns` with that `category` | `setArpPattern { index }`, the entry's index in `arpPatterns` |

Each item is a button, 32 tall, padding 0 8, 14px, left-aligned, `min-width: 0`, no wrap,
ellipsis, radius 0, `border-top: 1px solid var(--line)` (border-box; the first row's hairline
is on the 212 line, and no hairline closes the last row): a hairline list. The **selected
type** is the entry at `harmonyArp.harmonyType` in `harmonyTypes` when `mode` is `harmony`,
else the entry at `harmonyArp.arpPattern` in `arpPatterns` (by index; `typeName` and
`category` are the engine's reading of the same index, HA-D23). It wears the chosen face:
`--t` fill, `--g` label, weight 500, and its top hairline is transparent (the block replaces
it); `aria-pressed="true"`. Every other item: no fill, `--t2` label, weight 400,
`aria-pressed="false"`. Cells past the last item are empty grid cells, not buttons and not
drawn (HA-D5). More than 20 items (five rows) would overflow the 160px: today's longest list is
19 and the grid clips (`overflow: hidden`); a longer library is a follow-up. Because the grid
clips and the cells touch, an item's focus ring is drawn inside it: `outline-offset: -2px`
(the kit's 1px `--focus` outline otherwise). Click: the command in the table; nothing when the
item is already selected (HA-D1; cursor stays `pointer`). Tooltip `harmony.type` on a
Harmony-list item, `harmony.pattern` on a pattern. `aria-label` "{name}" plus ", selected" when
selected. `data-face="chosen"` on the selected item (Stage.md D41). Library lists empty (the
app's `library` starts with empty `harmonyTypes` and `arpPatterns` until `session.library()`
returns): the grid is empty; the header still names the type from `harmonyArp.typeName`.

### Settings column

`1021,212 371×160`, a column of rows, a `role="group"` with `aria-label` "{typeName}
settings" ("Standard Duet 1 settings"; empty name: "Settings"). Which rows it has is the
**kind** of the selected type, the first that applies:

| Kind | When | Rows, top to bottom (heights sum to 160 or less) |
|---|---|---|
| `arpeggio` | `mode` `arpeggio` | spec #527 (Harmony-Arp). Until it lands: Assign 44 (without Multi) · Volume 44 · More 36 (HA-D15) |
| `multi` | `typeName` "Multi Assign" | Note 44 (HA-D10) |
| `echo` | `category` "Echo" | Assign 44 · Volume 44 · Speed 36 · Touch limit 36 (HA-D9) |
| `harmony` | anything else | Assign 44 · Volume 44 · Touch limit 36 · Chord note only 36 |

Every row has `border-top: 1px solid var(--line)` (box-sizing border-box), items centred, gap
12, no wrap, and starts with a **label** cell 64 wide (`flex: none`, `white-space: nowrap`,
`overflow: visible`: "Touch limit" at 13px is about 64px and may run a pixel or two into the
gap), 13 / 400 `--m`. Three
row shapes carry every setting (Kit additions › Settings rows): a **ChoiceRow** (tabs), a
**BarRow** (a value with a bar) and a **LampRow** (lamp buttons). The rows:

| Row | Shape | Label | Face | Reads | Sends | Tooltip | Launchkey |
|---|---|---|---|---|---|---|---|
| Assign | ChoiceRow 44, `role="group"` `aria-label="Assign: which Right parts sound the effect"` | "Assign" | tabs Auto, Multi, R1, R2, R3; Multi only in the `harmony` and `echo` kinds (RM p.46) | `harmonyArp.assign`: `auto` → Auto, `multi` → Multi, `right1..3` → R1..R3; in the `arpeggio` kind a `multi` shows Auto chosen (it plays as Auto) | `setHarmonyAssign { assign }` | `harmony.assign` | — |
| Volume | BarRow 44, slider 0–127 | "Volume" over a second line (HA-D8) | the bar and the value | `harmonyArp.volume` | `setHarmonyVolume { volume }` | `harmony.volume` | the knob of the Rack knob page whose target is `harmonyVolume` (knob 5 with the default map); a Panel fader with that target |
| Touch limit | BarRow 36, slider 1–127 | "Touch limit" | the bar and the value | `harmonyArp.touchLimit` | `setTouchLimit { velocity }` | `harmony.touch_limit` | — |
| Chord note only | LampRow 36 | none | LampButton `size sm` "Chord note only" | `harmonyArp.chordNoteOnly` | `setChordNoteOnly { on: !chordNoteOnly }` | `harmony.chord_note_only` | — |
| Speed | ChoiceRow 36, `role="group"` `aria-label="Speed: the repeat rate"` | "Speed" | ChosenTabs `row-sm` 1/4, 1/6, 1/8, 1/12, 1/16, 1/32 (28 tall, 22px block, 13px, padding 0 8) | `harmonyArp.speed` | `setHarmonySpeed { speed }` | `harmony.speed` | — |
| Note | 44 tall, a hairline-topped row of text; no label cell | none | "Multi Assign has no settings: each right-hand key goes to Right 1, 2 and 3 in turn." 13 / 400 `--m`, line-height 16, wrapping, starting at the row's left edge (no padding), centred vertically; not a control | — | — | — | — |
| More | 36 tall, a hairline-topped row; no label cell (interim, HA-D15) | none | a text button "Quantize, Hold, Velocity, Keep Key On…" 13 / 400 `--t2`, left-aligned, centred vertically; `aria-label` "More arpeggio settings: Quantize, Hold, Velocity, Keep Key On. Opens the Harmony/Arpeggio panel" | — | opens today's Harmony drawer (`ui.toggleDrawer('harmony')`, opened, not toggled closed), where those settings are until #527 | `harmony.more` (new, C-HA1) | — |

Assign tabs: ChosenTabs `size row` (Auto and Multi padding 0 10, 13 / 400 `--m` unchosen) and
the three part tabs R1, R2, R3 (padding 0 8, 13 / 500, in `--r1`, `--r2`, `--r3` when
unchosen, `data-hue`). The chosen tab wears the chosen face, a 24px white block with `--g`
text, whichever it is: a chosen R1 loses its hue and its `data-hue` (HA-D14: `data-hue` says
what colour is drawn). `aria-label`s "Auto", "Multi", "Right 1", "Right 2", "Right 3", plus ",
chosen" on the chosen one. Clicking the chosen tab sends nothing. Speed's tabs follow the same
rules: `aria-label` "1/4" … "1/32" plus ", chosen", `data-face="chosen"` on the chosen one,
`--m` text unchosen, and the chosen tab sends nothing.

Volume's label is two lines (gap 1): "Volume" 13 / 400, line-height 16, `--m`, over "Knob {n}"
12 / 400, line-height 14, `--d`, where `n` is 1 + the index of the first entry of
`liveRack.controls.knobs` whose `kind` is `harmonyVolume` (5 with the default map); no such
knob: the second line is absent and the label is one line (HA-D8). Touch limit's label is one
line. Speed's tabs are 28 tall (a 22px block) so the row keeps 36.

BarRow drawing (both sliders), from the label: the **bar** (`flex: 1`, `aria-hidden`) is a 2px
`--past` track with a 2px `--t2` fill from the left, `width: value / 127 × 100%` (Volume 100 →
78.7%; Touch limit 1 → 0.8%, a hairline of fill, HA-D16); then the **value** 44 wide (`flex:
none`), right-aligned, 22 / 300, line-height 24, `--t`, the integer. The row is one
`<div role="slider" tabindex="0">` with `aria-label` "Volume" / "Touch limit",
`aria-valuemin`, `aria-valuemax`, `aria-valuenow` and `aria-valuetext` "Volume 100 of 127" /
"Touch limit 1 of 127". Pointer (HA-D7, as the fader's, Stage.md D23): press anywhere on the
row and drag horizontally, with pointer capture; `value = clamp(round(v0 + (x − x0) × 127 /
w), min, max)` with `v0`, `x0` the value and pointer x at pointerdown and `w` the bar
element's `getBoundingClientRect().width` read at pointerdown (239 at 1440: 371 − 64 − 12 − 12
− 44), never a jump to the pointer. A send goes out when the whole-number value changes, at
most once per animation frame, and on pointerup only if the value changed since the last send
(a click without movement sends nothing). Vertical wheel: up (`deltaY < 0`) +1, down −1, per
notch; `deltaX` is ignored. Keys: ArrowRight and ArrowUp +1, ArrowLeft and ArrowDown −1,
PageUp/PageDown ±10, Home/End to min/max (the slider stops propagation, kit › Interaction
conventions); double-click resets Volume to 100 and Touch limit to 1 (one send, of the default,
when it differs). Cursor `pointer`, `ew-resize` while dragging.

Chord note only: LampButton `size sm` (28 tall, 13px; the board's 30px rounds to the kit's
size, HA-D6), centred vertically in its 36px row, left-aligned, label "Chord note only", no
code, `aria-pressed` = `chordNoteOnly`; `aria-label` "Chord note only, on" / ", off".

## Band, keys and status on this page

The kit's full band, key strip and status line, unchanged, as on the Stage (Stage.md › Band,
keys and status on the Stage). The Harm/Arp lamp in the lamp row reads `harmonyArp.on`, so it
is lit while the header's On is: the same switch, drawn twice on purpose (the band is the
hardware's mirror; the header is the page's). The knobs show whatever page `knobs.page` is: on
the Rack page knob 5 ("Harm level", Stage.md D6) is this page's Volume.

## States

| State | What changes |
|---|---|
| Standard Duet 1, on (the board) | as drawn: kind `harmony`; Harmony tab chosen; the first grid item the white block; Assign Auto; Volume 100 "Knob 5"; Touch limit 1; Chord note only off |
| Switch off | header lamp reads "Off", off face; band Harm/Arp lamp off; nothing else changes (the type and settings stay) |
| An Echo type (Echo, Tremolo, Trill) | header caption "Echo"; Echo tab chosen; grid of three; kind `echo`: Assign, Volume, Speed, Touch limit |
| Multi Assign | Multi Assign tab chosen; grid of one; kind `multi`: the note only |
| An arpeggio pattern | caption "Arpeggio · {category}"; that arpeggio tab chosen; grid of its patterns; kind `arpeggio` (#527; interim per HA-D15: Assign, Volume, and the More row to today's drawer) |
| Harm/Arp tab or Alt+H pressed while on this page | back to the Stage (`ui.page` `stage`), as the drawer toggled closed, closing any drawer open over the page too; Esc runs `ui.escape()` (drawers and overlays close first) and returns to the Stage only when nothing was open (HA-D24) |
| Help mode until #508 lands | as the Stage's interim: `?` toggles `tips.help` and today's tooltip footer, nothing on the page changes |
| Assign Right 2 | R2 wears the white block (no hue); R1 and R3 keep their hues |
| Assign Multi, then an arpeggio | Auto shows chosen; Multi absent |
| Browsing another category | that tab chosen; the grid shows its items with no white block (the selected type is elsewhere); the header and settings keep the selected type |
| Type changed from the hardware or a rack load (`typeName` changes) | the viewed category snaps to the selected type's; its block shows |
| Library lists empty (not loaded yet) | empty grid; three Harmony tabs, no arpeggio tabs; header names `typeName` ("—" when empty); in mode `arpeggio` no tab is chosen |
| Rack map without a Harmony volume knob | Volume's "Knob n" line absent |
| Stopped band | compact block: the run dot hidden, the section in `--m` (Kit additions › Compact block) |
| No chord | compact block chord "—" in `--d`, no tones |
| No Launchkey | app bar status hollow dot; fader button 5 is still the switch on screen; everything works |
| A refusal or notice | the status line |
| Help mode | spec #508 |

Light theme: the same markup; only tokens change (kit › Tokens). The chord's `--ba` is `none`
in light; the page has no other glow of its own.

## Board fixture

The state and moment that reproduce the board, for the `Pages/Harmony` › `Board` story and its
shots: `app/src/ui/Harmony/Harmony.fixtures.ts` exports `boardState` (a full `AppState`),
`boardLibrary` (a `LibraryList`), `boardNow` and `boardMeterHolds`. `boardState` is the
Stage's `boardState` (`app/src/ui/Stage/Stage.fixtures.ts`, Stage.md › Board fixture: the same
clock, transport, chord, parts, faders, meters, knobs, pads and keys) with these fields
replaced; `boardNow` (10000) and `boardMeterHolds` are the Stage's:

- `harmonyArp`: on true, mode "harmony", harmonyType 0, arpPattern 0, typeName "Standard Duet
  1", category "Harmony", volume 100, speed "1/8", assign "auto", chordNoteOnly false,
  touchLimit 1, arp { quantize "off", hold false, pedalHold false, velocity "original",
  fixedVelocity 100, keepKeyOn false }.
- `liveRack.controls`: the default map (`defaultControlMap()` in `app/src/lib/api/types.ts`):
  knobs 5 `harmonyVolume`, so Volume reads "Knob 5".
- `boardLibrary` (the lists are not in `AppState`: the app keeps them in `app.library`, from
  `session.library()`; the page takes them as a `library` prop), a `LibraryList`: revision 1,
  `entries` the dev mock's (`app/src/lib/api/mock.ts` `STYLES` through `entryOf`, which hold
  the entry whose `id` is the fixture's `style.id` with folder "Pop", as the Stage's fixture
  describes), `voices` the mock's `VOICES`, and `harmonyTypes` and `arpPatterns` the mock's
  (`app/src/lib/api/mock-harmony.ts` `HARMONY_TYPES`, `ARP_PATTERNS`: the 23 types, Multi
  Assign at index 19 with category "Harmony"; the 23 patterns in seven categories). This page
  takes no library from the Stage's fixture (it exports none).
- Page: the story renders `Harmony` with `page: 'harmArp'` (the active tab is a prop; in the
  app it is `ui.page`, Stage.md D2); the viewed category is the selected type's (Harmony), as
  on mount.
- The band: as the Stage's fixture, with the Harm/Arp lamp lit by `harmonyArp.on`.

Board texts that differ from this fixture on purpose: the count row reads "fill after bar 3",
not the board's 4 (Stage.md D5; masked, as on the Stage); the board's One Touch `aria-label`s
name "Shift + pads 9 to 12", which the Launchkey can't do (Stage.md D16; not a pixel). The
board's category tabs read "UpDown", "AsPlayed", "ChordStab", "BrokenChord" where the spec
draws the state's "Up & Down", "As Played", "Chord Stab", "Broken Chord", and its arpeggio tabs
have 9px padding where the spec has 10 (HA-D2): the tabs row from "Arpeggio" to its end is
masked (`data-shot-mask="arp-tabs"` on the arpeggio group and its label). The light board's
`--m` (`#6e6e6e`) and `--lamp-ink` (`#111`) differ from the tokens (`#646464`, `#000`) within
the per-pixel threshold, as on the Stage (three lamps are lit here: Accomp, Harm/Arp twice).

## Components

Every part of the screen, in build order: a component is built only after everything in its
"Built from" column. Each gets `app/src/ui/<Name>/` with a SPEC.md per
`docs/factory/spec-template.md`; crops come from the board lines below (dark board / light
board; the light board's lines are the dark's minus 10 throughout). "Exists" is whether it is
in `app/src/ui` today; "Stage" means the Stage lane builds it (Stage.md › Components) and this
page reuses it. Components marked **new** are this page's own or its kit additions.

| # | Component | Kind | Built from | Exists | Board lines (dark / light) | Spec |
|---|---|---|---|---|---|---|
| 0 | tokens | — | — | yes; the kit's additions (Stage) | `:root` lines 40, 45 / 30, 35 | kit › Tokens |
| 1 | `longpress`, LampButton, Button, ChosenTabs, WaitingChip, AccentBlock, StatusDot, PartMarks, GroupHeader, BeatBlocks, FaderStrip, Knob, Pad, KeyStrip, StatusLine, HealthSlot | primitive | — | Stage | the band 196–364 / 186–354; keys 367–378 / 357–368 | Stage.md › Components 1–16 |
| 2 | ChosenTabs `size row` and `row-sm` **new sizes** | primitive | — | Stage (add the sizes) | 142–153, 168–174 / 132–143, 158–164 (`row-sm`: not on this board) | Kit additions › ChosenTabs row |
| 3 | ChordReadout `size compact` **new size** | primitive | — | Stage (add the size) | 124 / 114 | Kit additions › Compact block |
| 4 | RackReadout `variant inline` **new variant** | primitive | StatusDot | Stage (add the variant) | 54 / 44 | Kit additions › App bar, page variant |
| 5 | BarRow **new** | primitive | — | no | 176–185 / 166–175 | Kit additions › Settings rows; Settings column |
| 6 | ChoiceRow **new** | primitive | ChosenTabs | no | 166–175 / 156–165 | Kit additions › Settings rows |
| 7 | LampRow **new** | primitive | LampButton | no | 186–188 / 176–178 | Kit additions › Settings rows |
| 8 | TypeGrid **new** | primitive | — | no | 158–162; data 530–542 / 148–152; 513–525 | Type grid |
| 9 | CategoryTabs **new** | complex | ChosenTabs | no | 141–154; data 518–528 / 131–144; 507–511 | Category tabs |
| 10 | HarmonyHeader **new** | complex | LampButton | no | 134–138 / 124–128 | Header |
| 11 | SettingsColumn **new** (its Note and More rows are its own markup, not components) | complex | ChoiceRow, BarRow, LampRow | no | 165–189 / 155–179 | Settings column |
| 12 | CompactNowPlaying **new** | complex | AccentBlock, StatusDot, ChordReadout | no | 117–128 / 107–118 | Kit additions › Compact block |
| 13 | OneTouch, PageTabs, LaunchkeyStatus | complex | Button, ChosenTabs, StatusDot | Stage | 55–61, 62–73, 77 / 45–51, 52–63, 67 | kit › App bar; Stage.md › Style line |
| 14 | AppBar `variant page` **new variant** | complex | PageTabs, LaunchkeyStatus, HealthSlot, RackReadout, OneTouch | Stage (add the variant) | 52–81 / 42–71 | Kit additions › App bar, page variant |
| 15 | CountRow, MetronomeSplit, SectionRow, LampRow (band), FaderBank, KnobBank, PadGrid, PadBank, TransportColumn, FullBand | complex | as Stage.md › Components 25–27, 34–40 | Stage | 84–108, 196–364 / 74–98, 186–354 | kit |
| 16 | HarmonyPage **new** | complex | HarmonyHeader, CategoryTabs, TypeGrid, SettingsColumn, CompactNowPlaying | no | 111–193 / 101–183 | Display page |
| 17 | Harmony (page, `Pages/Harmony`) **new** | complex | AppBar, SectionRow, HarmonyPage, FullBand, StatusLine, KeyStrip | no | whole board | this file |

The band's lamp row (Stage.md component 34) and this page's LampRow (7) are different
components: the band's is nine cells; this one is a settings row. Also from the Stage lane,
not components: `app/src/ui/Stage/Stage.fixtures.ts` (the fixture's base), the
`app/src/pages/` wiring pattern, `ui.page`, and the shots masks.

Components take props and call callbacks; none reads `app.state`, `app.library` or sends. The
page wiring (`app/src/pages/HarmonyWiring.svelte`, outside `app/src/ui`, the Stage's pattern,
Stage.md D46) reads `app.state`, `app.library` and `ui.page`, keeps `now`, `receivedMs`, the
meter holds and the viewed category, passes them down (`state`, `library`, `page`, `now`,
`viewed`), and sends commands.

## Gap against today

| Area | In `app/src` now | Change |
|---|---|---|
| Harmony / Arpeggio | `panels/harmony/Harmony.svelte`: a drawer (`Overlay`, `ui.harmony`) with a Toggle, the type name, ◀ ▶ step buttons, a Harmony / Arpeggio `Choice`, one list grouped by category, and `Field` / `HSlider` / `Choice` / `Toggle` settings | Replaced by the page: a display tab (`ui.page` `harmArp`, Stage.md D2), category tabs instead of the mode choice and the grouped list, the settings column; no step buttons (HA-D11). The drawer and its tests (`Harmony.test.ts`) stay in the code, the drawer reached only from the arpeggio kind's More row, until #527 lands (HA-D15); then both go, and the page's checks (below) are the page's own from the start |
| Navigation | `lib/nav.ts`: Harmony/Arp Alt+H toggles the drawer | The tab and Alt+H set `ui.page` `harmArp`, and back to `stage` from the page (kit › App bar, D2; HA-D24); Esc on the page returns to the Stage |
| Shortcuts | `lib/keys.ts`: `J` (Shift+J) toggles the switch, `L` (Shift+L) steps the type, `*` toggles Arpeggio Hold | Unchanged (HA-D11) |
| Compact block | none (the Stage display's chord and tempo are 128px and 32px) | New, from the kit addition below; shares `splitChord`, the tone spelling and the fit maths with the Stage's ChordReadout |
| App bar | Stage lane's AppBar (page tabs, status, health) | The page variant adds the rack readout and One Touch after the wordmark |
| Settings rows | `panels/settings/Field.svelte`, `HSlider.svelte`, `Choice.svelte` (old tokens) | The kit's rows (BarRow, ChoiceRow, LampRow), which the Settings pages (#528–#533) can reuse |
| Screenshot tool | `app/scripts/shots.ts` (one 1000 × 600 viewport, no masks) | The Stage lane's per-story viewport and masks (Stage.md D39); this page adds a second mask |

## Contract changes needed

C-HA1 blocks the build (a tooltip key: without it `TipKey` doesn't type-check and the tooltip
catalog test fails); it lands with the Stage's C5 or as its own small PR. The Stage's C5 already
covers `nav.harmony` (its body says "opens the panel"; the rewrite to "shows the page" is C5's
rule for every `nav.*`). Nothing else on this page needs the API to change.

1. **C-HA1 · Tooltips** (`app/src/help/tooltips.ts`; then `app/docs/controls.md`
   regenerated with `npm run docs:controls` in `app/`, since `controls-doc.test.ts` requires
   the file to equal the render), **lands before the build**. Two new keys in the Keyboard
   Harmony / Arpeggio group: `harmony.category`, the full entry `{ title: 'Category', body:
   'Shows this category\'s Harmony types or arpeggio patterns. The type you have stays until
   you pick another.', genos: 'Keyboard Harmony / Arpeggio type', keys: [], launchkey: null }`,
   and `harmony.more`, `{ title: 'More arpeggio settings', body: 'Opens the Harmony/Arpeggio
   panel for Quantize, Hold, Velocity and Keep Key On, until they are on this page.', genos:
   'Arpeggio', keys: [], launchkey: null }` (removed with #527). And `harmony.volume` gains
   `launchkey: 'Knob 5 on the Rack knob page (with the default controller map); a Panel fader
   mapped to Harmony volume'` (today null). Keys the page no longer uses stay in the catalog
   for now (`harmony.mode_harmony`, `harmony.mode_arpeggio`, `harmony.prev_type`,
   `harmony.next_type`, `harmony.close`); dropping them is a follow-up with the drawer's
   removal.
2. **Suggested, not needed (follow-up):** give Multi Assign its own `category` ("Multi Assign")
   in `harmonyTypes` (`crates/yahaha-core/src/harmony.rs` `Category`, `tests/fixtures/library.json`,
   the dev mocks, app-api.md) so the screen needn't group it by name (HA-D2). Until then, and
   without it, the page groups by name.

## Checks

Vitest (`npx vitest run` on the page and component tests), each against the board fixture
unless it says otherwise. They read roles, names, attributes and the commands sent (a fake
`send`), and the `data-face` / `data-hue` hooks (kit › Faces, Stage.md D41); never computed
colours or layout. Checks that cross the wiring (the viewed category, the page setter, the
drawer opener: 2, 7, 17) render `HarmonyWiring` over the mock session with the fixture state
and a fake `send`; the rest render the pure components with props.

1. Header: the group's `aria-label` is "Harmony / Arpeggio: Standard Duet 1, on"; the lamp
   reads "On", has `aria-pressed="true"` and `data-face="on"`, and a click sends
   `toggleHarmonyArp`; with `on` false it reads "Off". With category "Echo" the caption reads
   "Echo"; with mode "arpeggio" and category "Up & Down", "Arpeggio · Up & Down"; with
   category "Harmony", no caption element.
2. Category tabs: the Harmony group has three tabs, the arpeggio group seven, named "Up &
   Down" … "Sequence" (from the mock's `ARP_PATTERNS`); "Harmony" is `aria-pressed="true"`
   (`data-face="chosen"`) and its `aria-label` "Harmony: 19 types, selected type here".
   Clicking "Echo" sends nothing and the grid then lists Echo, Tremolo, Trill with no
   `data-face="chosen"` item; the header still reads "Standard Duet 1". Then a state with mode
   "arpeggio", `arpPattern` 13, `typeName` "Alberti 16", category "Broken Chord": the "Broken
   Chord" tab is chosen and the grid shows four patterns with "Alberti 16" chosen. With an
   empty `arpPatterns`, no arpeggio tabs, and in mode "arpeggio" no tab is chosen.
3. Type grid: 19 items in Data List order, "Standard Duet 1" `aria-pressed="true"`; clicking
   "Block" sends `setHarmonyType { index: 8 }`; clicking the chosen item sends nothing; the
   grid has no element for cells 20; on the Multi Assign tab one item, and clicking it sends
   `setHarmonyType { index: 19 }`; on "Random", clicking "Dice 16" sends `setArpPattern { index:
   5 }`. Items on a Harmony list carry `data-tip="harmony.type"`, patterns `harmony.pattern`.
4. Settings column, kind `harmony`: four rows in order Assign, Volume, Touch limit, Chord note
   only; Assign's "Auto" is chosen and clicking "Right 2" sends `setHarmonyAssign { assign:
   'right2' }` (its tab carries `data-hue="r2"` and `aria-label` "Right 2"); with `assign`
   "right2" it is `data-face="chosen"`; "Multi" is present.
5. Kind `echo` (`harmonyType` 21, typeName "Tremolo", category "Echo"): rows Assign, Volume,
   Speed, Touch limit; no Chord note only; Speed has six tabs, "1/8" chosen
   (`data-face="chosen"`), clicking "1/16" sends `setHarmonySpeed { speed: '1/16' }`, clicking
   "1/8" sends nothing.
6. Kind `multi` (`harmonyType` 19, typeName "Multi Assign"): the column (a `role="group"`
   named "Multi Assign settings") holds one row with the note text and no `role="slider"`,
   inner `role="group"` or button.
7. Kind `arpeggio` interim (mode "arpeggio", `arpPattern` 0): rows Assign, Volume and More;
   Assign has no "Multi"; with `assign` "multi", "Auto" is chosen; the More button's click
   calls the drawer opener (`ui.toggleDrawer('harmony')` with the drawer closed → open).
8. Volume: `role="slider"` named "Volume", `aria-valuenow` 100, `aria-valuetext` "Volume 100
   of 127"; the label's second line reads "Knob 5"; with a map whose knobs have no
   `harmonyVolume`, no second line; with the bar's `getBoundingClientRect` stubbed to 239px
   wide, a pointerdown then a pointermove 24px to the right then pointerup sends exactly one
   `setHarmonyVolume { volume: 113 }` (100 + round(24 × 127 / 239)); pointerdown and pointerup
   without a move send nothing; wheel one notch up sends 101; ArrowDown 99; ArrowRight 101;
   PageUp 110; End 127; double-click at 100 sends nothing and at 90 sends 100; the bar's fill
   carries `style="width: 78.7%"` (one decimal, HA-D16).
9. Touch limit: `role="slider"` named "Touch limit", min 1, `aria-valuetext` "Touch limit 1 of
   127"; ArrowDown at 1 sends nothing; ArrowUp sends `setTouchLimit { velocity: 2 }`;
   double-click at 40 sends 1.
10. Chord note only: `aria-pressed="false"`, `data-face="off"`; a click sends
    `setChordNoteOnly { on: true }`; `aria-label` "Chord note only, off".
11. Bar maths (pure, `app/src/ui/BarRow/bar.ts`): `fillPercent(100, 127)` is "78.7%",
    `fillPercent(1, 127)` "0.8%"; `dragValue(100, 0, 24, 239, 0, 127)` is 113;
    `dragValue(1, 0, −50, 239, 1, 127)` is 1.
12. Compact block: its `aria-label` is "Now playing: Sunday Drive Pop, 104 BPM, running,
    chord Am7, Main B"; the style name reads "Sunday Drive Pop" and a click opens the Browser;
    the tempo reads "104" and "BPM"; the run dot has `data-state="running"`; the chord's base
    run is "Am" and its extension "7"; the tones read "A C E G"; the section reads "Main B"
    with `data-hue="main"`. Stopped (`transport.running` false, `main` 1): the dot has
    `data-state="stopped"`, the section reads "Main B" with `data-hue="m"`, the label ends
    "stopped, chord Am7, Main B". No chord: "—", no tones element, the label says "no chord".
    `transposeKeyboard` 2 with name "Bm7": tones "B D F# A".
13. App bar, page variant: the rack readout reads "Rack A1 Sunday drive" with the modified
    dot, and a click opens the Rack page's interim target (the Rack drawer); without a loaded
    button it reads "Rack Sunday drive"; One Touch 2 is chosen and clicking 3 sends
    `recallOts { index: 2 }`; the Harm/Arp tab has `aria-current="page"`.
14. Band: the Harm/Arp lamp in the lamp row has `aria-pressed="true"`; with `harmonyArp.on`
    false both it and the header lamp are off.
15. Every interactive element has a `data-tip` in the catalog (the existing tooltip test).
16. Keyboard: the Tab order is the DOM order: app bar (the rack readout, One Touch 1–4, the
    page tabs, the health slot when it is a button), section row (Accomp, Metronome, its
    caret, Unison, Panic, ?), the compact block's style name, the header lamp, the category
    tabs (Harmony, Echo, Multi Assign, then the arpeggio tabs), the grid items in order, the
    settings rows top to bottom (Assign tabs, Volume, Touch limit, Chord note only), then the
    band and the status line as the kit orders them. The divider, the title, the type name,
    the tempo, the tones and the section are not focusable.
17. Page: with `ui.page` "harmArp" the Harm/Arp tab has `aria-current="page"`; pressing the
    tab sets `ui.page` to "stage"; Escape with the Harmony drawer open closes the drawer and
    leaves the page as it is; Escape again sets `ui.page` to "stage" (HA-D24).

**Story and screenshot checks** (`npm run shots -- Harmony`, real Chrome), for what jsdom
can't see:

- `Pages/Harmony` › `Board` (export `Board`, layout `fullscreen`, `parameters.shots = {
  viewport: { width: 1440, height: 900 }, mask: ['[data-shot-mask="when"]',
  '[data-shot-mask="arp-tabs"]'] }`) renders `Harmony` with `boardState`, `boardNow` and
  `boardMeterHolds`, unscaled, in both themes, against `app/src/ui/Harmony/crops/Board-dark.png`
  and `Board-light.png` (copies of `docs/design/push/png/Harmony-Dark.png` and
  `Harmony-Light.png`, 1440 × 900): at most 0.02 of the unmasked pixels differ. The arp-tabs
  mask covers the board's spellings and 9px padding (Board fixture); the Chord note only lamp is
  28px against the board's 30 (HA-D6), within the budget.
- The same story covers the chosen blocks, the hairline grid, the bars, the chord's two weights
  and its `--ba` glow (dark) and none (light).
- `Components/SettingsColumn` › `Echo` (typeName "Tremolo") and `MultiAssign`: the four rows
  with Speed, and the note; no crop (the board doesn't draw them), judged by Inspect.
- `Components/TypeGrid` › `Browsing` (viewed "Random", selected elsewhere): two items, no white
  block; no crop.
- `Components/CategoryTabs` › `Full` (the seven arpeggio categories at 10px padding): the row
  stays one line inside 975px; no crop.
- `Components/CompactNowPlaying` › `LongChord` (`name: "C#m7b5/G#"`): the chord shrinks and the
  section keeps its full name; no crop.
- axe finds no violation on any story.

## Decisions

- **HA-D1 · Category tabs browse; they don't send.** A tab changes only what the grid shows
  (an app-only viewed category). Changing the type stops an Echo or an arpeggio at once
  (app-api.md), so a tab that picked a type would cut the music while the player looks around.
  The viewed category snaps to the selected type's whenever `typeName` changes, so the
  hardware's and a rack's changes always show their block. Clicking the selected item or the
  chosen tab sends nothing.
- **HA-D2 · Grouping and names.** The three Harmony tabs are fixed (Harmony, Echo, Multi
  Assign), as the board draws them. The API gives Multi Assign the category "Harmony", so the
  page groups it by name; a category of its own in `harmonyTypes` is a suggested follow-up
  (Contract changes 2). The arpeggio tabs come from `arpPatterns` and read the categories as
  the state spells them ("Up & Down", "As Played", "Chord Stab", "Broken Chord"), not the board's
  compressed "UpDown", "AsPlayed", "ChordStab", "BrokenChord": the state's words are what the
  Launchkey display and the drawer show today, and they fit (about 880px of 975 at 10px padding,
  which every tab in the row shares).
- **HA-D3 · Header caption.** The type name stands alone for the Harmony category (the board);
  an Echo type adds "Echo" and an arpeggio "Arpeggio · {category}" (the Harmony-Arp board), so
  the player sees which list the type belongs to when its name doesn't say.
- **HA-D4 · The switch lamp.** Label "On" / "Off" by state (the board draws "On" while on),
  with the code "Fader button 5" so the hardware button is named where the switch is. Its
  `aria-label` names the function ("Harmony/Arpeggio on (fader button 5)").
- **HA-D5 · Empty grid cells.** The board closes its grid with a blank button ("No type");
  the spec leaves cells past the last item empty and not focusable.
- **HA-D6 · Lamp size.** The board's 30px "Chord note only" lamp is the kit's `size sm` (28px,
  13px label); the kit has no 30px size and the 2px difference is within the screenshot budget.
- **HA-D7 · Bar sliders.** Volume and Touch limit are `role="slider"` rows with the fader's
  pointer model (Stage.md D23): relative to the press point, scaled by the bar's width, sends
  at most once per frame and always on release; wheel, arrows, PageUp/PageDown and
  double-click reset as the kit's faders and knobs. The board draws them as readouts with a
  slider role and no drag maths.
- **HA-D8 · "Knob n".** Volume's second line names the Rack-page knob the live rack's map puts
  on `harmonyVolume` (knob 5 by default), from `liveRack.controls.knobs`; a map without one
  shows no line, rather than a knob that doesn't exist.
- **HA-D9 · Echo settings.** The board says Echo, Tremolo and Trill "add Speed" without drawing
  it: Speed is a 36px ChoiceRow of six tabs between Volume and Touch limit, and Chord note only
  is dropped for the Echo category (it applies to the Harmony category only, docs/harmony.md),
  so the column stays 160px.
- **HA-D10 · Multi Assign.** No settings (RM p.46): one hairline-topped row of muted text says
  so, so the column isn't blank.
- **HA-D11 · No step buttons.** The drawer's ◀ ▶ (`stepHarmonyArpType`) go: on the page every
  type is one click, and the hardware has no type control, so parity needs nothing. Shift+L
  keeps stepping the type and Shift+J the switch (the `L` and `J` entries in `keys.ts`;
  lowercase `l` is Left on/off), and `*` Arpeggio Hold.
- **HA-D12 · Compact block rules** (Kit additions): the run dot follows the Stage's run state
  (solid running, hollow sync start, hidden stopped with its space kept); the section is
  `transport.section` in its hue while running and `Main {A+main}` in `--m` stopped (Stage.md
  D4); the chord fits by the Stage's rule scaled to 48px (floor 32px); the tones are the
  Stage's note names without intervals.
- **HA-D13 · Page app bar.** The rack readout is one line ("Rack A1 Sunday drive ●") reading
  the Stage's RackReadout fields; the One Touch group is the Stage's (D16, D17: applies at once,
  Launchkey Racks page pads 1–4; the board's "Shift + pads 9–12" is wrong).
- **HA-D14 · A chosen part tab is white.** R1–R3 in Assign wear the chosen face (white block,
  `--g` label) when chosen, like every chosen thing; the hue shows while unchosen. A hue-filled
  block would be a fifth face.
- **HA-D15 · The arpeggio kind is #527's.** The settings column is a `kind` switch (`harmony`,
  `echo`, `multi`, `arpeggio`) and the grid, tabs and header are kind-agnostic, so the
  Harmony-Arp spec adds only the `arpeggio` kind's rows (Quantize, Velocity, Hold, Keep Key On)
  and the grid area's extra rows. Until it lands, an arpeggio shows Assign (no Multi) and
  Volume, which both apply to it today, the grid lists its patterns, and a More row opens
  today's drawer (the kit's interim rule, Stage.md D32: today's equivalent exists), so
  Quantize, Hold, Velocity and Keep Key On stay reachable from the screen. The drawer is
  removed with #527.
- **HA-D16 · Bar fill.** `value / 127` for both bars, one decimal place, so a Touch limit of 1
  shows a hairline of fill (the board's 1%) and the two bars share one rule.
- **HA-D17 · Fixture.** The board is the Stage's moment with Harmony on: the fixture is the
  Stage's `boardState` with `harmonyArp`, the library lists and the default rack map replaced,
  so the two boards' shared pixels come from one source.
- **HA-D18 · Hover, cursor, focus, tab order:** the kit's (Stage.md D35, D36); the sliders use
  `ew-resize` while dragging, the horizontal twin of the faders' `ns-resize`.
- **HA-D19 · Tooltips.** Two new keys: `harmony.category` for both tab groups and
  `harmony.more` for the interim More row; the existing `harmony.*` keys cover the rest. The compact block carries the Stage's keys (`browser.open`,
  `display.tempo`, `display.chord`); the rack readout `stage.rack_name`; One Touch `ots.1`–`ots.4`.
- **HA-D20 · Library not loaded, and unknown categories.** Fixed Harmony tabs, no arpeggio
  tabs, an empty grid, and the header's name from state ("—" when the state's `typeName` is
  empty); nothing is disabled, so the page needs no loading face. A `harmonyArp.category` the
  page doesn't know (empty, or one a newer engine adds) counts as "Harmony": the Harmony tab
  and the `harmony` kind, the fullest set of settings.
- **HA-D21 · No display border.** This page's display box has no 1px border (the Channel-style
  pages don't; the Stage's does, Stage.md D42), so its content box is `48,128 1344×268`, 1px
  off the Stage's display content. The boxes above are the board's.
- **HA-D22 · Grid overflow.** Twenty cells fit; a list longer than that is clipped. Today's
  longest is 19 (Harmony); a scrolling grid is a follow-up if a library ever grows past 20 in
  one category.
- **HA-D23 · Identity by index.** The selected type is the list entry at the state's index
  (`harmonyType` or `arpPattern` by `mode`); the kind, header caption and tab snapping read
  `typeName` and `category`, which the engine sets from that same index. Tests set all of
  them together.
- **HA-D24 · Leaving the page.** The Harm/Arp tab and Alt+H on this page return to the
  Stage (`ui.page` `stage`) and close any drawer open over it, as the drawer toggled closed.
  Escape runs `ui.escape()` (`app/src/lib/store.svelte.ts`: drawers and overlays close
  first); when nothing was open it sets `ui.page` to `stage`, the way it sends Library back
  to the Stage today. The `ui.page` branch is added to `ui.escape()` by the Stage lane's D2
  work or here, whichever lands first; the page component itself listens to no keys. The
  other tabs switch pages as the kit says.

## Kit additions

Parts this page needs that more than one board draws, written here in full because kit.md is
owned by the Stage lane (#537) and the Channel spec (#501, which kit.md names as the owner of the
page variant and the compact block) isn't written yet. The kit's owner moves them into kit.md
unchanged; until then this section is their definition.

### App bar, page variant

The kit's App bar (kit › App bar) with two parts added after the wordmark, because a page
replaces the Stage display that carries them (`Channel-Dark.dc.html` lines 77–101 are the
source; this board's 52–81 copy it). Every non-Stage display page and the tall pages use it.

- **Rack readout** (`RackReadout` `variant inline`), `margin-left: 16px` on top of the bar's
  gap 8 (so 24px after the wordmark: the wordmark is 59px wide at 18 / 500, and the readout
  starts at x = 24 + 59 + 24 = 107), a text button 32 tall, items on the baseline, gap 6, 14 /
  400, no wrap: "Rack" `--m`, then the slot `--t`
  (the bank letter and button number, Stage.md › Sounds row: "A1"; absent when no
  `quickRacks.buttons` entry is `loaded`, Stage.md D21), then `liveRack.name` `--t`, then a 5px
  round `--t` dot (`align-self: center`) when `liveRack.modified`. Click: opens the Rack page
  (Stage.md D32 interim: the Rack drawer). Tooltip `stage.rack_name`. `aria-label` "Rack:
  {name}{, modified}{, on Quick Rack A1}. Opens the Rack page". No ellipsis (the name has room
  here; a 60-character name pushes the tabs, which is a follow-up).
- **One Touch**, `margin-left: 12px` on top of the gap 8 (20px after the readout; its x
  depends on the rack name's width, as on the board), the Stage's group (Stage.md › Style
  line, One Touch):
  "One Touch" 14 `--m` with 4px right margin, four 32 × 32 buttons, gap 4, chosen = applied,
  disabled past `ots.settings.length`, `recallOts { index }` at once (D17), tooltips
  `ots.1`–`ots.4`, `aria-label`s as the Stage's.
- The page tabs, right area, Launchkey status and health slot are the kit's, unchanged; the
  active tab is the page's own (`aria-current="page"`).

### Compact now-playing block

`CompactNowPlaying`, 320 × 84, drawn at the top-left of a page's content box on every page
that replaces the display (`Channel-Dark.dc.html` lines 142–153 are the source; this board's
117–128 copy it). A `role="group"` `aria-label` "Now playing: {style name}, {tempo} BPM,
{running | sync start | stopped}, {chord {name} | no chord}, {section as shown}" ("Now
playing: Sunday Drive Pop, 104 BPM, running, chord Am7, Main B"; stopped with no chord: "Now
playing: Sunday Drive Pop, 104 BPM, stopped, no chord, Main B"). A column: a 16px row, 12px
gap, a 48px row; the 8px left at the bottom of the 84 is empty (the box is the board's; the
content is 76).

**Row 1** (16 tall, items centred, gap 8, no wrap):

| Control | Face | Reads | Sends / does | Tooltip |
|---|---|---|---|---|
| Style name | AccentBlock as a text button: padding 0 6, 13 / 500, line-height 16, `--g` on `--a`, `min-width: 0`, ellipsis; `aria-label` "{name}: open the Browser" | `style.name` (never empty: the engine always has a style loaded; an empty string shows "—") | opens the Browser (`ui.browser = true`, Stage.md D3) | `browser.open` |
| Tempo | `margin-left: auto`; the number 18 / 300, line-height 16, `--t`, then "BPM" 12 / 400 `--m` with 3px left margin; not a control | `transport.tempo` rounded (`Math.round`) | — | `display.tempo` |
| Run dot | 6px round StatusDot: `transport.running` → `--ok` with `--bg` (`data-state="running"`); stopped with `transport.syncStart` → a hollow 1px `--ok` ring (`sync`); stopped → `visibility: hidden`, space kept (`stopped`); `role="img"` `aria-label` "Running" / "Sync start" / "Stopped" | `transport.running`, `transport.syncStart` | — | — |

**Row 2** (48 tall, items on the baseline, gap 14, no wrap):

- **Chord** (`ChordReadout` `size compact`, `flex: none`): `chord.name` at 48 / 300,
  line-height 48, letter-spacing −2, `--a`, `text-shadow: var(--ba)`, in the Stage's two runs
  (Stage.md D30, `splitChord`: the extension at weight 200, letter-spacing 0). No chord: "—"
  in `--d`, no shadow. **Fit:** when the chord at 48px is wider than the space left for it
  (320 − 14 − the tones' width − 14 − the section's width, measured with the section at its full
  name), the font shrinks to `max(32, floor(48 × space / width))` px, `width` its `scrollWidth`
  at 48px, measured after each change of `chord.name`, `keyboard.chordTones` or
  `transport.section`; letter-spacing scales (−2 × size / 48). Tooltip `display.chord`.
- **Tones** (`flex: 0 1 auto`, see below): the note names of the Stage's tones (Stage.md › Chord: `keyboard.chordTones`
  moved by `chord.transposeKeyboard`, root first, at most six, the Stage's spelling), joined
  by spaces, 14 / 300, letter-spacing 3, `--t2`, no intervals ("A C E G"). No chord: absent.
- **Section** (`margin-left: auto`, `flex: none`): 24 / 300, line-height 48, letter-spacing
  −0.5. Running: `transport.section` in the Genos spelling (kit › Section names) in its hue
  (`data-hue`), no glow. Stopped: `Main {A + transport.main}` in `--m` (Stage.md D4,
  `data-hue="m"`). Not a control.
- **When it still doesn't fit** (the chord at its 32px floor, six tones and a long section
  can pass 320px): the row is `overflow: hidden` and the tones give way: the tones element
  gets `min-width: 0; overflow: hidden; text-overflow: ellipsis` and is the only item that
  shrinks (`flex: 0 1 auto`); the chord and the section keep their full width.

The block is not focusable except its style name. Light: `--ba` is `none`; nothing else changes.

### ChosenTabs, size `row`

A third size of the kit's ChosenTabs (Stage.md component 4: `page` 36 tall with a 24px bottom
block, `header` 35 with 22): **`row`**, 32 tall, 13 / 400, padding 0 10 (callers may pass 0 8),
tabs touching (gap 0), the chosen face as a 24px block centred vertically (`background:
linear-gradient(var(--t), var(--t)) center / 100% 24px no-repeat`, `--g` text, weight 400 as
the page tabs keep it, `data-face="chosen"`), unchosen `--m` text on nothing (or a caller's
hue and weight, with `data-hue`, dropped while chosen). A fourth, **`row-sm`**, is 28 tall
with a 22px block, 13px, padding 0 8 (the Speed tabs). Each tab is a button with
`aria-pressed`, `aria-label` its text plus ", chosen" when chosen; a click on the chosen tab
calls nothing; the group is a `role="group"` with its label. Used by the category tabs, Assign
and Speed here, and by the Settings pages' choices (#528–#533).

### Settings rows

Three row shapes for a settings column; each is `border-top: 1px solid var(--line)`
(border-box), items centred, gap 12, no wrap, and starts with a 64px label cell (`flex: none`,
13 / 400 `--m`, optionally a second line 12 / 400 `--d`, line-heights 16 and 14, gap 1):

| Row | Height | Content after the label |
|---|---|---|
| ChoiceRow | 44 (or 36 with `row-sm` tabs) | a ChosenTabs `row` group |
| BarRow | 44 or 36 | a 2px bar (`flex: 1`, `--past` track, `--t2` fill from the left, `width: value / max × 100%` to one decimal) then the value, 44 wide, right-aligned, 22 / 300, line-height 24, `--t`; the row is one `<div role="slider" tabindex="0">` named by `aria-label` (Settings column › BarRow drawing gives the pointer and key maths) |
| LampRow | 36 | one or more LampButton `size sm`, gap 8, left-aligned, no label cell |

A row of plain text (this page's Note) or a single text button (its More row) is the
column's own markup: hairline top, no label cell, content at the left edge, centred
vertically.

Pure maths in `app/src/ui/BarRow/bar.ts`: `fillPercent(value, max)` → a CSS percentage string
with one decimal; `dragValue(v0, x0, x, w, min, max)` → `clamp(round(v0 + (x − x0) × 127 / w),
min, max)`: the scale is 127 per `w` px for both the 0–127 and the 1–127 range (so
`dragValue(100, 0, 24, 239, 0, 127)` is 113 and `dragValue(1, 0, −50, 239, 1, 127)` is 1).

## Follow-ups

- Multi Assign as its own category in `harmonyTypes` (Contract changes 2), then HA-D2's
  name test goes.
- Drop the drawer and its unused tooltip keys (`harmony.mode_*`, `harmony.prev_type`,
  `harmony.next_type`, `harmony.close`) once every display page has landed.
- A scrolling or paged type grid if a category ever has more than 20 entries (HA-D22).
- The page app bar with a very long rack name (no ellipsis today).
- The Live Control percentages (ArpVel, ArpGateT, ArpUnitM; docs/arpeggio.md "Not done yet")
  have no control anywhere; a follow-up with the Harmony-Arp spec.
- A Launchkey control for the type (step with Shift + something) if players ask; today the
  hardware has only the switch and the volume knob.

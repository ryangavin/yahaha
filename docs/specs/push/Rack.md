# Rack

The rack: what is under your hands, as one page. Its four keyboard parts with their sounds
and mix, the keyboard settings it saves (split, transpose, Harmony/Arp, Manual Bass, Left
Hold), its controller map and its send effects, and the style's One Touch links to racks. A
tall page: it replaces the Stage's display, and the band below goes half height so the
faders, Start / Stop and the keys stay in reach.

- **Issue:** #503 · **Flow:** Change a sound · **Boards:** `docs/design/push/Rack-Dark.dc.html`,
  `Rack-Light.dc.html`; pictures `docs/design/push/png/Rack-Dark.png`, `Rack-Light.png`
  (1440 × 900). The spec stands without them: every value a builder needs is below, in
  [kit.md](kit.md) or in [Stage.md](Stage.md); the board lines in the Components table are for
  cutting crops.
- **Built from:** [kit.md](kit.md): Section row, the faces, tokens and interaction conventions,
  FaderStrip, Knob, Pad, Key strip, Status line; [Stage.md](Stage.md): the chord runs (D30),
  the sound cell marks, the rack readout's slot rule (D21), the interim link rule (D32). What
  the kit doesn't have yet (the app bar's page variant, the compact block, the half band, the
  keys row) is specified under **Kit additions** below, so this file is complete on its own;
  the Channel spec (#501) owns the same parts on its board and the kit takes them from
  whichever lands first (RK-D1).
- **Variant of this screen:** Rack-MapEdit (#517), the controller map editor that Edit map
  opens; it specs only its delta against this file.
- **Glance order** (what must read first): the rack's name and whether it is modified (the
  head row), then each part's sound and On lamp, then the part values in their hues. The
  compact block's chord (48px) stays quieter than on the Stage; nothing here outshines the
  head row's white Save block.
- **Before building:** RK-C1 and the Stage's C5 (tooltip keys: the page uses `app.health`,
  `nav.channel` and `metronome.settings` through the kit) must land first; the rest don't
  block (see Contract changes needed). The Stage's C1 (`quickRacks.loaded`) and its `AppBar`, `SectionRow`,
  `FaderStrip`, `Knob`, `Pad`, `KeyStrip` and `StatusLine` components are reused; building the
  Rack before the Stage lane means building those from the kit first (Components table).

## Layout

At 1440 × 900, laid out by the `Rack` component (`app/src/ui/Rack/`) and scaled by the app
shell exactly as the Stage (Stage.md D1). Padding 24 all round; a column, no page box:
alignment and hairlines only.

| Region | Box | What's in it | Spec |
|---|---|---|---|
| App bar | `24,24 1392×36` | wordmark, the rack readout as the current item, One Touch 1–4, page tabs (none chosen), Launchkey status, health slot | Kit additions › App bar, page variant |
| Section row | `24,68 1392×32` | Accomp, count row, Metronome ▾, Unison, Panic, ? | kit › Section row, unchanged |
| Page | `24,112 1392×468` | left column (compact block, Keyboard group), divider, right column (head row, parts table, three groups) | below |
| Half band | `24,600 1392×176` | faders, Track ◀ ▶, knobs and pads, transport | Kit additions › Half band |
| Keys row | `24,796 1392×80` | status line and key readouts (20), the key strip (56) | Kit additions › Keys row |

Vertical rhythm: app bar, 8, section row, 12, page, 20, half band, 20, keys row
(24 + 36 + 8 + 32 + 12 + 468 + 20 + 176 + 20 + 80 + 24 = 900).

The page is a `section` with `aria-label="Rack page: {rack name}"` (the head row's name), a
row: **left column** `24,112 320×468` (`flex: none; overflow: hidden`), a 1px `--line`
**divider** `368,112 1×468` with 24px margins each side, and the **right column**
`393,112 1023×468` (`flex: 1; min-width: 0`). `overflow: hidden` on the page. The half band is
a `section` `aria-label="Band"`; the keys row's `section` carries the key strip's label (Kit
additions › Keys row); the app bar is the kit's `header` and the section row its toolbar.

### Opening, leaving and props

The page shows when the app-only `ui.page` is `rack` (Stage.md D2: `UiStore.page`, added by
the Stage lane; the Rack lane adds it if it goes first). It replaces the Stage under
`ui.view === 'stage'`; Library (`ui.view === 'library'`) draws over it and "Back to stage"
returns to it, since `ui.page` is unchanged. In: the app bar's rack readout on any page, Alt+O
(toggles `rack` ↔ `stage`), a rack-target strip name. Out: the Stage tab (`ui.page = 'stage'`),
Alt+O again, any display tab (which sets its page or, until its spec, runs today's `NAV`
entry over this page, kit › App bar). Esc does nothing here (RK-D21). An interim Channel
target (D32) sets `ui.page = 'stage'` and then `channelNav.show(part)`, so today's
`ChannelView` shows in the Stage's display (RK-D21).

`Rack` (`app/src/ui/Rack/Rack.svelte`) is pure. Its props: `state: AppState`; `now` and
`receivedMs` (ms, as the Stage); `meterHolds: number[]`, nine held peaks for strips 1–9 as
linear amplitudes (the Stage's `boardMeterHolds` shape); `keyRange: 49 | 61 | 88` (the wiring
resolves it: `rangeFor(ui.keyRange, io.inputs.map(i => i.name))` from
`app/src/panels/keystrip/keyboard.ts`, the strip's rule today); `saveAs: { open: boolean;
error: string | null }`; `shift: boolean` (`ui.shift`, for Shift-click and `shiftAction`);
`help: boolean` (`tips.help`, the ? button); `dropouts: { show: boolean; count: number }`
(the wiring's `DropoutWatch`, for the health slot as the Stage's wiring passes it). Its
callbacks: `onSend(cmd: AppCmd)`; `onOpen(target)` with `target` one of `'stage'` (the Stage
tab), `{ kind: 'page', page }` (any other page tab: the kit's tab rule), `'browser'`,
`'harmArp'`, `'effects'` (the Master strip name), `'multiPads'` (the Multi Pad strip name),
`'settingsSystem'` (the health slot), `'mapEditor'`, `'otsChooser'`, `{ kind: 'quickList',
part }`, `{ kind: 'channel', part }` (strip names, the health slot's failed part, Shift-click
on a part lamp, Sends 4–6, Insert), `{ kind: 'replace', part }`, `{ kind: 'editor', part }`;
`onHelp()` (toggles help mode, #508); `onSaveAs(action)` with `'open'`, `'close'`, `{ name }`
(submit) or `'typed'`; `onKeyRange(n)`. The wiring (`app/src/pages/RackWiring.svelte`,
RK-D14) maps each `onOpen` target to its page or interim (D32), `editor` to
`app.pluginEditor(part, true)` (`app/src/lib/api/session.ts`), `replace` to
`ui.openLibrary('sounds', part)` (RK-C2), and keeps the Save as… state (Head row).

## Left column

### Compact block

`24,112 320×84`, a `role="group"` with the `aria-label` below, a column, `overflow: hidden`.
The Channel board's block, specified here (Kit additions › Compact block gives the same text;
this is where it is used).

- **Row 1** `24,112 320×16`, items centred, gap 8, no wrap:
  - **Style name**, an accent block as a text button (`min-width: 0`, ellipsis, padding 0 6,
    13 / 500, line-height 16, `--g` on `--a`, no radius): `style.name`. Click `onOpen('browser')`:
    the Browser (`ui.browser = true`; Stage.md D3). Tooltip `browser.open`. `aria-label`
    "{name}: open the Browser".
  - **Tempo** (`margin-left: auto`): `transport.tempo` rounded to a whole BPM (Stage.md D44) at
    18 / 300, line-height 16, `--t`, then "BPM" 12 / 400 `--m` with a 3px left margin. Not a
    control. Tooltip `display.tempo`.
  - **Run dot**, a 6px round `role="img"`: running → `--ok` fill with `--bg`,
    `aria-label="Running"`; stopped with `transport.syncStart` → a hollow 1px `--ok` ring,
    "Sync start"; stopped → `visibility: hidden` (space kept), no label (RK-D2).
- **Row 2** `24,140 320×48`, 12px below row 1, items on the baseline, gap 14, no wrap:
  - **Chord**: `chord.name` at 48 / 300, line-height 48, letter-spacing −2, `--a`,
    `text-shadow: var(--ba)`, drawn in the Stage's two runs (Stage.md D30: the base at 300, the
    extension at 200 with letter-spacing 0). No chord: "—" in `--d`, no shadow. It does not
    shrink: the chord element is `min-width: 0; overflow: hidden` (clipped at its own right
    edge, no ellipsis) while the tones and the section are `flex: none` and keep their size
    (RK-D3). Tooltip `display.chord`.
  - **Tones**: the chord's tones as note names, 14 / 300, letter-spacing 3, `--t2`, separated by
    spaces ("A C E G"): the same tones, order and spelling as the Stage's tone columns
    (Stage.md › Chord, D20, D31), at most six. No chord: nothing. `flex: none`.
  - **Playing section** (`margin-left: auto; flex: none`): the playing section's name (kit ›
    Section names) at 24 / 300, line-height 48, letter-spacing −0.5, in its hue, no glow. Which
    section is playing follows kit › Count row item 3; stopped, the Main to start on in `--m`
    (Stage.md D4). Not a control.
  - `aria-label` on the block: "Now playing: {style.name}, {tempo} BPM, {Running | Sync start |
    Stopped}, chord {name | none}, {section}" ("Now playing: Sunday Drive Pop, 104 BPM,
    Running, chord Am7, Main B").

### Keyboard group

`24,212 320×248`, 16px below the compact block, `role="group"` `aria-label="Keyboard settings in
this rack"`, a column of hairline rows (every row `box-sizing: border-box; border-bottom: 1px
solid var(--line)`, items centred, gap 8). The 120px under it (to the page's bottom at 580) is
empty on purpose.

| Row | Box | Left | Right |
|---|---|---|---|
| Header | `24,212 320×32` | "Keyboard" 14 `--m` | "in this rack" 12 `--m` (`margin-left: auto`) |
| Split | `24,244 320×36` | "Split" 14 `--t`, `flex: 1` | value, −, + |
| Transpose | `24,280 320×36` | "Transpose" 14 `--t`, `flex: 1` | value, −, + |
| Harmony/Arp | `24,316 320×36` | LampButton | type (`margin-left: auto`) |
| Manual Bass | `24,352 320×36` | LampButton | note (`margin-left: auto`) |
| Left Hold | `24,388 320×36` | LampButton | note (`margin-left: auto`) |
| Quick Rack | `24,424 320×36` | "Quick Rack" 14 `--t`, `flex: 1` | slot, Store |

- **Value** (Split, Transpose, the Quick Rack slot): 18 / 300 `--t`; Split and Transpose
  `width: 44px; text-align: right`. Split reads `chord.splitName` ("F#2"); Transpose reads
  `chord.transposeKeyboard` signed ("0", "+2", "−3", a true minus U+2212, as `signed` in
  `app/src/panels/rack/rack.ts`); the slot reads the Stage's rack slot rule (Stage.md › Sounds
  row, D21: "A1"; no loaded button on the bank on view → "—"). Not controls. Tooltips
  `split.display`, `transpose.display`, `quick.bank`.
- **− and +**: 28 × 28 off-face buttons, 14 / 300, the glyphs "−" (U+2212) and "+". Split −/+
  send `moveSplit { delta: -1 | 1 }` (tooltips `split.down`, `split.up`; `aria-label` "Split
  point down" / "up"; Launchkey: Chord pad page, Split − / +). Transpose −/+ send
  `stepTranspose { keyboard: -1 | 1, master: 0 }` (tooltips `transpose.keyboard_down`,
  `transpose.keyboard_up`; "Transpose down a semitone" / "up"; Launchkey: Chord pad page, Kbd
  Tr − / +). Disabled at the range's end: Split at 24 and 96, Transpose at −12 and 12.
- **LampButtons** are `size sm` (28 tall, padding 0 14, 13px, no code):
  - **Harmony/Arp**: label "Harmony/Arp"; on = `harmonyArp.on`; sends `toggleHarmonyArp`;
    tooltip `harmony.switch`; `aria-label` "Harmony/Arpeggio in this rack: {on | off}";
    Launchkey: fader button 5. Its **type** is a text button, 28 tall, 13 `--m`, reading
    `harmonyArp.typeName` ("Standard Duet 1"); click `onOpen('harmArp')`: the Harm/Arp page
    (D32: the Harm/Arp drawer, `ui.toggleDrawer('harmony')`, opened, until that spec lands).
    Tooltip `rack.harmony_type` (exists; its body is rewritten in RK-C1). `aria-label`
    "Harmony type: {typeName}. Opens Harm/Arp".
  - **Manual Bass**: label "Manual Bass"; on = `chord.manualBass`; sends `toggleManualBass`;
    disabled (the kit's disabled face: label `--d`, `aria-disabled`, no opacity, RK-D4) when
    `chord.upper` is false; tooltip `detection.manual_bass`; `aria-label` "Manual Bass: {on |
    off}" plus ", works only with Upper chord detection" while disabled; Launchkey: Chord pad
    page, Manual Bass. Its **note**, 12 `--m`: "Upper only" while disabled, else empty.
  - **Left Hold**: label "Left Hold"; on = `chord.leftHold`; sends `toggleLeftHold`; tooltip
    `detection.left_hold`; `aria-label` "Left Hold: {on | off} (fader button 7)"; Launchkey:
    fader button 7. Note "Fader button 7" 12 `--m`, always.
  - **Store** (the Quick Rack row): label "Store", `size sm` with padding 0 14; on =
    `quickRacks.store`; sends `toggleQuickRackStore`; disabled when `quickRacks.readOnly`;
    tooltip `quick.store`; `aria-label` "Store this rack to a Quick Rack: arms Store, then tap a
    slot" (", armed" while on); Launchkey: Racks pad page, Store. The slot is tapped on the
    Racks pad page or the Quick Racks page (#515); nothing on this page is a slot.

## Right column

### Head row

`393,112 1023×36`, a hairline row, items centred, gap 12, no wrap:

| Item | Face | Reads | Sends / does | Tooltip |
|---|---|---|---|---|
| "Rack" | 14 `--m` | — | — | — |
| Name | 18 / 400 `--t`, `min-width: 0`, ellipsis | `liveRack.name` as it is ("Recovered: Sunday drive" included); `liveRack.id` null and name "" or "New rack" → "Untitled rack" (`rackName` in `panels/rack/rack.ts`) | — | — |
| Badge | 12 / 500 `--a` | "Mine" when `liveRack.id` is not null; "Unsaved" in `--m` when it is (RK-D5) | — | — |
| Modified | a 5px round `--t` dot, gap 6, "Modified" 13 `--t` | shown while `liveRack.modified`; hidden (removed) otherwise | — | — |
| Hint | 12 `--m`, `min-width: 0`, ellipsis | "Changes play now and autosave here until you save." (fixed text); replaced by the name field while saving as (below) | — | — |
| Name field | see Saving as | shown while `saveAs.open` | `saveRackAs` | `rack.save_as_name` |
| Buttons | a right group (`margin-left: auto; flex: none`, items centred, gap 12) holding the three below, so they stay at the right edge whichever are shown | — | — | — |
| Revert | a text button, 28 tall, padding 0 6, 13 `--m` | shown while `liveRack.modified` and `liveRack.id` not null; otherwise removed | `revertRack` | `rack.revert` |
| Save as… | off face, 28 tall, padding 0 14, 13px, label `--t` | always | `onSaveAs('open')` (below) | `rack.save_as` |
| Save | chosen face (`--t` fill, `--g` label, 500), 28 tall, padding 0 18, 13px | disabled when `liveRack.modified` is false and `liveRack.id` is not null | `saveRack` (no `soundNames`; the engine's `soundNames` prompt follows when it needs them) | `rack.save` |

`aria-label`s: Revert "Revert to the saved rack", Save as… "Save as a new rack", Save "Save
rack{, nothing to save}". The badge and the modified chip are text, not controls.

**Saving as** (RK-D6). The state is the wiring's (`saveAs` prop): `{ open, error }`. Save as…
(`onSaveAs('open')`) replaces the hint with a name field, 28 tall, 220 wide, padding 0 8, 1px
`--line` border, radius 4, `--g` fill, 13px `--t`, `aria-label="New rack name"`, tooltip
`rack.save_as_name`, focused on open and prefilled with the name the head row shows (the
`rackName` text, "Recovered: …" kept). Enter calls `onSaveAs({ name })` (trimmed; empty does
nothing); Esc (the field's own keydown, with `stopPropagation`, so the window's Esc doesn't
fire) or a click on Save as… again calls `onSaveAs('close')`; each input calls
`onSaveAs('typed')`. The wiring: on `{ name }` it sends `saveRackAs { name }` and records
`message?.seq ?? -1` at that moment (`sentSeq`); it sets `error` to `message.text` when a
message with `seq` > `sentSeq` and `error` true arrives, clears `error` on `'typed'`, `'open'`
and `'close'`, and sets `open` false when `liveRack.id` changes to a non-null id while open, or
when `liveRack.prompt` becomes a `soundNames` prompt (which carries the name on). With `error`
set, it shows after the field, 12px `--warn`, `role="alert"`, ellipsis.

### Parts table

`393,156 1023×220`, 8px below the head row, `role="table"` `aria-label="Your hands: the four
keyboard parts"`. A header row and four part rows (R1, R2, R3, L, `keyboardParts[0..3]`), each
`role="row"`, a grid `32px 184px 56px 48px 44px 40px 40px 40px 60px 36px 120px minmax(0, 1fr)`
(the last column is 235 wide), column gap 8, items centred, bottom hairline, `box-sizing:
border-box`.

- **Header** `393,156 1023×28`, 12 `--m`, no wrap: "", "Sound", "On", "Level", "Pan", "Reverb",
  "Chorus", "Delay", "Sends 4–6", "Octave", "Insert", "Plugin" (plain `span`s with
  `role="columnheader"`).
- **Part rows** 48 tall: `393,184`, `393,232`, `393,280`, `393,328` (1023 wide each). Each
  cell below is wrapped in a `role="cell"` span that is the grid item (the header's in
  `role="columnheader"` spans), so the table's roles nest as ARIA asks; the cell span has
  `min-width: 0; display: flex; align-items: center` and its control fills it.

Each row's cells, left to right. "The hue" is the part's hue (`--r1 --r2 --r3 --l`); "off"
means `keyboardParts[i].sounding` is false (Left under Manual Bass sounds, so it is drawn on
even with `on` false, as on the Stage).

| # | Cell | Face | Reads | Sends / does | Tooltip | Launchkey |
|---|---|---|---|---|---|---|
| 1 | Tag | "R1" / "R2" / "R3" / "L", 14 / 500 in the hue; `--d` when off. Not a control (the strip names in the band open Channel, RK-D18); `aria-hidden` (the row's controls name the part) | `sounding` | — | — | — |
| 2 | Sound | a text button, 184 × 44, a column (`justify-content: center`, no gap: the two lines, 34 tall together, sit 5px from the top and bottom), left-aligned: line 1 (18 tall, gap 6, no wrap): the sound number 12 `--m` (`--d` when off), the name 14 `--t` (`--m` when off; `min-width: 0`, ellipsis), then the Stage's marks in the Stage's order and sizes (Stage.md › Sounds row: the 5px `--t` dot when `soundEdited`, the 12px `--warn` ⚠ when `plugin.missing`, the 12px `--ending` ✕ when `plugin.status` is `failed` and not missing, "off" 12 `--m` when `on` is false and the part doesn't sound, "bass" 12 `--m` when `playsBass`); line 2 (16 tall, 12 `--m`, ellipsis): the instrument line | number and name exactly as Stage.md › Sounds row (D22), Left under Manual Bass included (the Style's Bass voice, `voiceName`); the instrument line: RK-D7 | `onOpen({ kind: 'quickList', part })`: the quick sound list (#514; D32: Library › Sounds loading into the part, `ui.openLibrary('sounds', i)`) | `launchkey.fader_sound` | swap mode, knob 1 |
| 3 | On | LampButton `size sm`, `width 48`, `text 12` (48 × 28, 12px): "On" / "Off" from `on`; "Swap" while `surface.layer` is `{ type: 'swap', part: i }`; lit from `sounding`; accessible name (`name`) "{part name} {on \| off \| swap}", following the label | `on`, `sounding`, `surface.layer` | click `togglePart { part: i }` (in swap: `setLayer { layer: { type: 'none' } }`); long press or right-click `setLayer { layer: { type: 'swap', part: i } }`, latched until the next click (kit › Lamp row, Stage.md D12); Shift-click `onOpen({ kind: 'channel', part })` (D32) | `part.right1.on` … `part.left.on` | fader button 1–4 |
| 4 | Level | readout in the hue | `volume` | `setPartVolume { part, volume }`; reset 100 | `mixer.panel.right1` … `mixer.panel.left` | Panel fader 1–4 (Vol layer), swap knob 2 |
| 5 | Pan | readout | `pan` as "C" (64), "L{64 − pan}", "R{pan − 64}" (`panLabel`) | `setPartPan { part, pan }`; reset 64 | `mixer.part.pan` | Pan layer, swap knob 3 |
| 6 | Reverb | readout | `strip.sends[0]` | `setPartSend { part, send: 'reverb', value }`; reset 0 | `mixer.part.reverb` | Reverb layer, swap knob 4 |
| 7 | Chorus | readout | `strip.sends[1]` | `setPartSend { send: 'chorus' }`; reset 0 | `mixer.part.chorus` | Chorus layer, swap knob 5 |
| 8 | Delay | readout | `strip.sends[2]` | `setPartSend { send: 'variation' }`; reset 0 | `mixer.part.variation` | Delay layer, swap knob 6 |
| 9 | Sends 4–6 | a text button, 32 tall, 15 / 300 in the hue (`--d` off), left-aligned, no wrap, `min-width: 0; overflow: hidden; text-overflow: ellipsis` | the part's level to each added send (`strip.sends[s.send]` for each `effects.sends` entry with `send` ≥ 3, in order), joined by " · ", then " · —" once when fewer than three are added; no added send: "—" ("0 · —" with one added send at 0; "0 · 12 · 0" with three) | `onOpen({ kind: 'channel', part })`: Channel for the part (#501; D32), where sends 4–6 are set (RK-D8). `aria-label` "{part name} sends 4 to 6: {text}. Opens Channel" | `mixer.strip.send` | swap knob 8 (send 4) |
| 10 | Octave | readout, −2..2 signed ("0", "−1", "+1") | `octave` | `setPartOctave { part, octave }`; reset 0; drag and wheel step 1 per notch or 4px as below | `part.octave` (new) | Chord pad page: none; swap: none |
| 11 | Insert | a text button, 32 tall, items on the baseline, gap 6, no wrap, `min-width: 0; overflow: hidden; text-overflow: ellipsis`: the kind name 14 (`--t`; `--m` when the kind is `none`, the slot is off (`strip.inserts[0].on` false) or the part is off) then its first setting's value 15 / 300 in the hue (`--d` when the slot or the part is off), nothing for `none` | `strip.inserts[0].name` ("None", "Tremolo"), `strip.inserts[0].settings[0].value` as a number | `onOpen({ kind: 'channel', part })`: Channel for the part (#501; D32), where the insert's kind and settings are set (RK-D8). `aria-label` "{part name} insert 1: {kind name}{, amount n}{, off}. Opens Channel" | `mixer.strip.insert_kind` | swap knob 7 (the amount) |
| 12 | Plugin | a row of 28-tall buttons, gap 8, no wrap (`min-width: 0`): RK-D9 below | `plugin`, `soundEdited`, `plugins.list` | below | below | — |

**Readouts** (cells 4–8 and 10; the `Readout` component): a `<button role="slider">`, 32 tall,
no face (transparent, no border), left-aligned, 18 / 300 in the hue, `--d` when the part is off
(the value still shows and still works). `aria-label` "{part name} {column}" ("Right 2 pan"),
`aria-valuemin`, `aria-valuemax` (0–127; octave −2..2), `aria-valuenow`, `aria-valuetext` the
value as shown ("L12", "−1"). Pointer: press and drag vertically with pointer capture, one step per whole 4px
travelled (Shift: per 12px), up positive, the value moving from where it was at pointerdown
(never a jump); wheel ±1 per notch; arrows ±1, PageUp/PageDown ±10 (octave: ±1 both);
double-click resets to the value given in the table. Each change sends the cell's command with
the whole value (clamped), at most once per animation frame (the latest value), and the last
value on pointerup. `ns-resize` while dragging. In swap mode for the part, the hardware knobs
move the same values and the readouts follow the state (nothing extra to do).

**Plugin cell** (RK-D9). Buttons are 28 tall, 12px, radius 4; off face unless said:

| When | Buttons, left to right |
|---|---|
| `plugin` absent (a SoundFont voice) | nothing |
| `plugin.missing` | **Replace…** (off face, padding 0 12, label `--t`; `onOpen({ kind: 'replace', part })`: Library › Sounds for the part, `ui.openLibrary('sounds', i)`; the pick there sends `replacePartSound` once RK-C2 lands and `assignSound` until then; tooltip `rack.replace`; `aria-label` "Replace the missing plugin on {part name} with another sound"), then "silent until replaced" 12 `--m`. The row dims only from `sounding`: a missing plugin on a part that is on is undimmed, with ⚠ and Replace… |
| otherwise | **Edit** (padding 0 10, `--t2`; `onOpen({ kind: 'editor', part })`, which the wiring runs as `app.pluginEditor(part, true)`; disabled unless `plugin.editor` and `plugin.status` is `playing`; tooltip `part.plugin_edit`; "Open the {plugin.name} editor for {part name}"), then **Save sound** when `soundEdited` (padding 0 10, `--t2`; sends `saveSound { part }`; tooltip `sounds.save_over`; "Save the edited sound on {part name}") else **Reload** (padding 0 10, `--t2`; sends `reloadPartPlugin { part }`; disabled unless `plugin.status` is `failed` or `muted`; tooltip `part.plugin_reload`; "Reload the plugin on {part name}"), then **In process**, a LampButton `size sm` with `pad 10` (padding 0 10, 12px): on = the `plugins.list` entry with `plugin.id`'s `inProcess`; sends `setPluginInProcess { id, inProcess: !on }`; disabled when that entry is absent or its `canRunInProcess` is false; while `inProcessPending(plugin, entry)` (moved to `app/src/ui/Rack/plugin.ts` from `app/src/panels/parts/parts.ts`: the setting differs from where the plugin runs now) the lamp wears the waiting face in `--lamp` (its `waiting` prop, RK-D9); tooltip `part.plugin_in_process`; "{part name} plugin runs in process: {on | off}{, applies at its next load}" |

### Groups row

`393,392 1023×156`, 16px below the parts table, a grid of three columns `repeat(3, minmax(0,
1fr))`, column gap 24: **Controller map** `393,392 325×156`, **Send effects** `742,392 325×156`,
**One Touch** `1091,392 325×156`. Each is a `role="group"` column: a 28-tall hairline header
(items centred, gap 8; title 14 `--m`; a right item with `margin-left: auto`) and 32-tall rows
(items centred). The 32px under the row (to 580) is empty.

#### Controller map

`aria-label="Controller map"`. Header: "Controller map"; right, 12 `--m`: "Default" when
`liveRack.controls` equals `defaultControlMap()` (`app/src/lib/api/types.ts`), else "Edited".

| Row | Box | Left (`width: 76px`, 13 `--t`, `flex: none`) | Right (12 `--t2`, `min-width: 0`, ellipsis) |
|---|---|---|---|
| 1 | `393,420 325×32` | "Faders 1–4" | `mapLine(controls.faders, 'faders')` |
| 2 | `393,452 325×32` | "Knobs 1–8" | `mapLine(controls.knobs, 'knobs')` |
| 3 | `393,484 325×32` | "Drives faders 1–4 and knob page 2" 12 `--m`, `flex: 1`, ellipsis | **Edit map**, off face, 28 tall, padding 0 12, 12px, label `--t` |

Rows 1–2 have gap 12; row 3 gap 12. `mapLine` is a pure function (`app/src/ui/Rack/map.ts`,
RK-D10): each target's label, joined by " · ". Labels, with `{P}` the part's short tag (R1, R2,
R3, L): `none` "—"; `partLevel` "{P} level"; `partPan` "{P} pan"; `partReverb` "{P} reverb";
`partChorus` "{P} chorus"; `partDelay` "{P} delay"; `partSend` "{P} send {send + 1}";
`partInsertOn` "{P} insert {slot + 1}"; `partInsertSetting` "{P} insert {slot + 1} setting
{setting + 1}"; `harmonyArp` "Harm/Arp"; `rotaryFast` "Rotary"; `splitPoint` "Split";
`harmonyVolume` "Harmony"; `metronomeVolume` "Metronome"; `tempo` "Tempo"; any other kind
"?". Two rules compact it: a run of two or more consecutive `partLevel` targets reads as their
tags then " volumes" ("R1 R2 R3 L volumes"); and on the faders line, when every fader `i` is
`partLevel` of part `i` (the default), the line reads the full names "Right 1 · Right 2 ·
Right 3 · Left". The default map therefore reads "Right 1 · Right 2 · Right 3 · Left" and
"R1 R2 R3 L volumes · Harmony · Metronome · — · Tempo".

Edit map calls `onOpen('mapEditor')`: the map editor (Rack-MapEdit, #517); until it lands,
D32: the Rack drawer (`ui.toggleDrawer('rack')`, opened), which edits the map today (RK-D11).
Tooltip `rack.map`.
`aria-label` "Edit the controller map". Launchkey: none (the map is edited on screen only).

#### Send effects

`aria-label="Send effects in this rack"`. Header: "Send effects"; right: **+ Add send**, a text
button 24 tall, 12 `--t`, sends `addSend { kind: 'hall' }` (the kind is changed on Effects,
#519; RK-D12); disabled when `effects.sends` has six entries. Tooltip `fx.send_add`.
`aria-label` "Add a send effect".

Rows, each `gap: 8`, no wrap; the index 13 / 300 `--m` `width: 12px`; the name 13 `--t`; the
qualifier 12 `--m`:

| Row | Box | Content |
|---|---|---|
| 1 | `742,420 325×32` | "1", "Reverb", qualifier, Keep type |
| 2 | `742,452 325×32` | "2", "Chorus", qualifier, Keep type |
| 3 | `742,484 325×32` | "3", "Delay", qualifier, Keep type |
| 4 | `742,516 325×32` | the added sends (hidden when none) |

- Rows 1–3 are `effects.sends[0..2]` (the style's reverb, chorus and delay buses). The
  **qualifier** is "from style" when `setByRack` is false, else the send's kind `name`
  ("Celeste": the type the rack keeps). **Keep type** is a LampButton 26 tall, padding 0 10,
  12px (`margin-left: auto`): on = `setByRack`; sends `setRackSendOverride { send, on: !setByRack
  }`; tooltip `fx.send_rack_override`; `aria-label` "Send {n} keeps its type ({kind name}) in
  this rack: {on | off}" (the kind name always). Launchkey: none.
- Row 4 lists the added sends (`effects.sends` entries with `send` ≥ 3), each "{send + 1}"
  (the index style) then its `name` 13 `--t`, 12px apart; then "added" 12 `--m` once; then
  **Remove**, a text button 26 tall, 12 `--ending` (`margin-left: auto`), which removes the
  last added send: `removeSend { send: <last>.send }`; tooltip `fx.send_remove`; `aria-label`
  "Remove send {n}, {name}". Any added send is removed on Effects (#519); this page removes
  the last so the row needs one button (RK-D12). The board draws send 4 on the Delay row and
  no Keep type for Delay; this layout corrects both (RK-D12) and the board story masks the
  group.
- Fewer than three entries in `effects.sends` (an older state): the missing rows are hidden.

#### One Touch

`aria-label="One Touch Settings for {style.name}"`. Header: "One Touch"; right 12 `--m`
(`min-width: 0`, ellipsis): "for {style.name}".

| Row | Box | Content |
|---|---|---|
| 1 | `1091,420 325×32` | buttons 1–4 (gap 4), Link (`margin-left: auto`) |
| 2 | `1091,452 325×32` | "Timing", the timing tabs |
| 3 | `1091,484 325×32` | the OTS → rack line, Change (`margin-left: auto`) |

- **1–4**: 32 × 28 buttons, 14 / 300 (the chosen one 400), off face; `ots.applied` (1-based)
  marks the chosen face (`aria-pressed`); buttons past `ots.settings.length` disabled. Click
  sends `recallOts { index }` at once (Stage.md D17). Tooltips `ots.1` … `ots.4`. `aria-label`
  "One Touch {n}{, applied}{, loads rack {ots.racks[n−1].name}}" (the rack part only when that
  entry's `rack` is not null). Launchkey: Racks pad page, bottom row pads 1–4 (Stage.md D16).
  The app bar's One Touch group (Kit additions) is the same control drawn twice; both exist
  on this page as the board draws them.
- **Link**: a LampButton `size sm`, `text 12` (padding 0 14, 12px), label "Link"; on = `ots.link`; sends
  `toggleOtsLink`; tooltip `ots.link`; `aria-label` "One Touch Link: Main A to D pick OTS 1 to
  4. {On | Off}"; Launchkey: Setup pad page, OTS LINK; Shift + Pad Bank ▼.
- **Timing**: "Timing" 13 `--t`, `width: 56px`; a `role="tablist"` `aria-label="One Touch Link
  timing"` of two tabs, 24 tall, padding 0 10, 12px, no radius: "Immediate"
  (`immediate`) and "At Main change" (`mainChange`); the chosen one (`ots.linkTiming`) is the
  chosen face (`--t` fill, `--g` text, `aria-selected="true"`), the other `--m` on nothing.
  Click sends `setOtsLinkTiming { timing }`. Both tabs are in the tab order as plain buttons
  (Space and Enter select; no roving tabindex, no arrow keys, no tab panel: the tabs set a
  value, as the kit's ChosenTabs do). Tooltip `ots.link_timing` on each tab. Launchkey: none
  (Settings › Style, #529, has it too).
- **OTS → rack line**: "OTS {list}" 13 `--t` (`min-width: 56px; flex: none`, so it grows for
  "OTS 1, 2, 3, 4"), gap 8, then the sentence 13 `--t2` (`min-width: 0`, ellipsis), from the
  `ots.racks` entries whose `rack` is not null, in order: one → "OTS 4" and "loads Organ";
  several → "OTS 2, 4" and "load Jazz, Organ" (names in order, ", "); none → "OTS 1–4" and
  "the style's own"; a `missing` entry's name is followed by " (missing)". Not a control.
  **Change**: a text button 26 tall, 12 `--m`, `onOpen('otsChooser')`: the OTS → rack chooser
  (Library › Racks, #516, RK-D13; D32 until then: the Rack drawer, `ui.toggleDrawer('rack')`,
  whose OTS cards hold the selects today). Tooltip `ots.rack`.
  `aria-label` "Choose which of your racks each OTS loads, or the style's own".

## Band, keys and status on the Rack

The half band, the keys row and the status line as Kit additions describe them, unchanged.
The fader page, layer, knob page and pad page are the state's; the Rack page adds nothing. The
Section row is the kit's, verbatim.

## States

| State | What changes |
|---|---|
| Playing, modified, bank A1 loaded (the board) | as drawn |
| Rack unmodified | no Modified chip, no Revert, Save disabled; the app bar readout has no dot |
| New rack (`liveRack.id` null) | name "Untitled rack", badge "Unsaved" (`--m`), Revert hidden, Save enabled (it saves as new), slot "—" |
| Recovered rack | the name reads "Recovered: {name}"; otherwise as a saved rack |
| Saving as | the name field in place of the hint (Head row); a refused name shows `message.text` beside it |
| A rack prompt (`liveRack.prompt`) | Prompts (#504); until then the Rack drawer opens over the page with the prompt (RK-C3) |
| Part off | tag, values and name dimmed as the table says; the On lamp off |
| Swap held for a part | its On reads "Swap"; the knob page block in the band reads "Swap R1" on the hue (kit › Knobs); the readouts follow the knobs |
| Sound held or latched | the band's Sound lamp on; the half-band pads are the Racks page in the fallback face (Stage.md D33) |
| Missing plugin | ⚠ on the sound; the plugin cell shows Replace… and "silent until replaced"; the row dims only when the part is off (`sounding`), as R3 on the board |
| Failed plugin (installed) | ✕ on the sound; Reload enabled; the health slot names the part (kit › App bar) |
| Plugin loading | Edit and Reload disabled; the instrument line ends "· loading…" |
| Edited sound | the dot on the sound; "Save sound" in place of Reload |
| In process pending | the In process lamp in the waiting face |
| Upper detection off | Manual Bass disabled, "Upper only" |
| Quick Racks read-only | Store disabled |
| Store armed | Store lit; the Racks pad page flashes (hardware) |
| OTS with a rack | the OTS line names it; that button's `aria-label` says so |
| Style with fewer OTS | buttons past `ots.settings.length` disabled (both groups) |
| Six sends | + Add send disabled |
| No added send | row 4 hidden; every part's Sends 4–6 reads "—" |
| Controller map edited | "Edited" in the map header; the lines name the targets |
| Stopped | compact block: run dot hidden, section in `--m`; the count row stopped (Stage.md D4) |
| No chord | "—" in `--d`, no tones |
| No Launchkey | app bar status hollow; the band still works from the screen |
| No synth / audio trouble | the health slot (kit › App bar) |
| A refusal or notice | the status line in the keys row (`state.message`) |

Light theme: the same markup; only tokens change (kit › Tokens). The compact chord's `--ba` is
`none` in light, as is every glow.

## Board fixture

The state and moment that reproduce the board, for the `Pages/Rack` › `Board` story and its
shots: `app/src/ui/Rack/Rack.fixtures.ts` exports `boardState`, `boardNow` and
`boardMeterHolds`. It starts from the Stage's `boardState` (`app/src/ui/Stage/Stage.fixtures.ts`:
the same clock, transport, style, chord, keyboard, pads, knobs, faders, meters, lamps and
Launchkey), with these fields set or changed. Fields in neither take the dev mock's initial
values.

- **Moment:** `boardNow = 10000`, `receivedMs = 10000`, `surface.clock` as the Stage (bar 3 beat
  3, phase 0.25; the LED clock at 0.25, so the Next pad is at full).
- Props beside the state: `keyRange` 61, `saveAs` `{ open: false, error: null }` (the story
  passes them; `ui.page` is `rack` in the app, not in the fixture).
- `liveRack`: name "Sunday drive", id "sunday-drive", modified true, controls
  `defaultControlMap()`, prompt null. `quickRacks`: bank 0, button 0 `loaded` true (rack
  "sunday-drive", name "Sunday drive"), the rest empty; store false, storeWaiting null,
  readOnly false.
- `style.name` "Sunday Drive Pop"; `transport` running, section "Main B", tempo 104 (as the
  Stage).
- `chord`: name "Am7", fingered "Am7", upper false, manualBass false, manualBassActive false,
  split 54, splitName "F#2", transposeKeyboard 0, transposeMaster 0, leftHold false;
  `keyboard.chordTones` [9, 0, 4, 7].
- `harmonyArp`: on false, mode "harmony", harmonyType 0, typeName "Standard Duet 1", category
  "Harmony".
- `ots`: four settings ("OTS 1"–"OTS 4"), applied 2, link false, linkTiming "mainChange",
  racks `[{ rack: null, name: "", missing: false } × 3, { rack: "organ", name: "Organ", missing:
  false }]`, racksReadOnly false.
- `plugins`: available true, scanning false, instances 3; `list` `[{ id: "aumu Smp7 Fake",
  name: "Sampler Deluxe", manufacturer: "Fake Instruments", version: "0.9.0", format: "AUv2",
  lastError: null, inProcess: true, canRunInProcess: true, new: false, racks: 1, sounds: 2 }]`;
  `missing` `[{ id: "aumu Brs1 Fake", name: "Brass Deluxe", manufacturer: "Fake Instruments",
  racks: 1, sounds: 1 }]`; `needsAttention` `[{ id: "sunday-drive", name: "Sunday drive",
  parts: [2] }]`.
- `soundLibrary.patches` lists three (the state carries each `number`, so the list needn't be
  dense): `{ id: "stage-grand", name: "Stage Grand", category: "piano", number: 1, source: {
  kind: "plugin", componentId: "aumu Smp7 Fake", hasState: true, origin: { kind: "factory",
  number: 4 } } }`, `{ id: "silk-strings", name: "Silk Strings", category: "strings", number:
  41, source: { kind: "plugin", componentId: "aumu Smp7 Fake", hasState: true, origin: { kind:
  "factory", number: 9 } } }`, `{ id: "brass-section", name: "Brass Section", category:
  "brass", number: 57, source: { kind: "plugin", componentId: "aumu Brs1 Fake", hasState: true,
  origin: { kind: "factory", number: 2 } } }`; `tags` [], `favourite` false, `available` true,
  `note` null on each.
- `keyboardParts` (shared fields: `strip.sends[3..5]` [0, 0, 0], `strip.inserts[1]` `{ kind:
  "none", name: "None", on: false, settings: [] }`, `strip.comp` off, `eq` flat, `patch` null,
  `playsBass` false; the older mirrored fields match the strip: `reverb`, `chorus`, `variation`
  = `strip.sends[0..2]`, `insert` = `{ effect: "tremolo", on: true, amount: 30 }` on R2 and
  `{ effect: "distortion", on: false, amount: 64 }` (the default) on the others, `eq` flat;
  `plugin` fields not named: stage null, error null, cpu 0.02, overruns 0,
  recentOverruns 0, inProcessFallback false, outOfProcess false, preset null, presetKey null):
  - R1 "Right 1": on true, sounding true, volume 90, pan 64, `strip.sends` [40, 0, 0, …],
    octave 0, `strip.inserts[0]` `{ kind: "none", name: "None", on: false, settings: [] }`,
    `sound` `{ id: "saved:stage-grand", name: "Stage Grand" }`, `voiceName` "Stage Grand",
    `plugin` `{ id: "aumu Smp7 Fake", name: "Sampler Deluxe", manufacturer: "Fake
    Instruments", status: "playing", preset: "Concert D", presetKey: "f:4", editor: true,
    missing: false }`; soundEdited false.
  - R2 "Right 2": on, sounding, 72, pan 52 (L12), sends [56, 12, 0, …], octave 0,
    `strip.inserts[0]` `{ kind: "tremolo", name: "Tremolo", on: true, settings: [{ name:
    "Depth", value: 30, min: 0, max: 127, default: 64, display: "30" }, { name: "Note", value:
    2, min: 0, max: 7, default: 2, display: "1/8" }, { name: "Shape", value: 0, min: 0, max:
    127, default: 0, display: "0" }] }`; `sound` `{ id: "saved:silk-strings", name: "Silk
    Strings" }`, `voiceName` "Silk Strings", plugin Sampler Deluxe playing, preset "Strings",
    presetKey "f:9", editor true; soundEdited true.
  - R3 "Right 3": on false, sounding false, 64, pan 72 (R8), sends [32, 0, 0, …], octave 0,
    insert none; `sound` `{ id: "saved:brass-section", name: "Brass Section" }`, `voiceName`
    "Brass Section"; plugin `{ id: "aumu Brs1 Fake", name: "Brass Deluxe", manufacturer:
    "Fake Instruments", status: "failed", error: "not installed", missing: true, editor: false
    }`; soundEdited false.
  - L "Left": on, sounding, 80, pan 64, sends [48, 0, 0, …], octave −1, insert none; `sound`
    `{ id: "saved:silk-strings", name: "Silk Strings" }`, `voiceName` "Silk Strings", plugin
    Sampler Deluxe playing, preset "Strings", presetKey "f:9", editor true; soundEdited false.
- `effects.sends`: `[{ send: 0, kind: "hall", name: "Hall", returnLevel: 64, fromStyle: true,
  setByRack: false }, { send: 1, kind: "celeste", name: "Celeste", returnLevel: 64, fromStyle:
  true, setByRack: true }, { send: 2, kind: "dottedEighth", name: "Delay 1/8.", returnLevel:
  64, fromStyle: true, setByRack: false }, { send: 3, kind: "phaser", name: "Phaser",
  returnLevel: 64, fromStyle: false, setByRack: true }]`, each with its kind's default
  `params` (app-api.md › Send kinds).
- `transport.main` stays the Stage's 2: the queued Main C (app-api.md › transport: "playing
  or queued to follow"); Check 2's stopped case sets it to 1.
- The half band draws the Stage's `surface.faders`, `meters`, `boardMeterHolds`, `knobs`
  (Style page) and `pads` (Sections, pad 10 playing, 11 Next, 16 running). In half-band
  pixels (travel 44; the formulas in Kit additions › Half band): meter heights peak / RMS px
  R1 27 / 25, R2 20 / 18, L 22 / 20, Style 31 / 29, Multi Pad 0 / 0, Master 32 / 29, and no
  meter on R3 (off) or strips 7–8; held-peak tick bottoms (px from the fader's bottom) 34, 27,
  none, 29, 37, 8, none, none, 38; caps at `top` 31 (90), 37 (72), 40 (64, at 35%), 34 (80),
  27 (100), 31 (90), none, none, 27 (100); fader 2's ghost line at `top` 46 (position 50).
  Tempo knob arc 72° ((104 − 40) / 240 × 270, as the Stage).
- `message` null (the status line is empty, as the board). Props: `help` false, `shift`
  false, `dropouts` `{ show: false, count: 0 }`.

The board's count row reads "fill after bar 4" (Stage.md D5); its send-effects group differs
from this spec's layout (RK-D12); and its Manual Bass is drawn at 45% opacity (RK-D4): the
screenshot check masks the first two. The light board's `--m` (`#6e6e6e`) differs from the
token (`#646464`) within the per-pixel threshold, as on the Stage.

## Components

Every part of the screen, in build order; a component is built only after everything in its
"Built from" column. Each gets `app/src/ui/<Name>/` with a SPEC.md per
`docs/factory/spec-template.md`; crop boxes are the boxes in this spec and the kit. Board lines
are the dark board's / the light board's. "Exists" is whether it is in `app/src/ui` today;
"Stage #n" means the Stage spec's Components table already lists it, and it is reused as is
unless a variant is named. **New** marks components this spec adds.

| # | Component | Kind | Built from | Exists | Board lines (dark / light) | Spec |
|---|---|---|---|---|---|---|
| 0 | tokens | — | — | yes; add the kit's new tokens | `:root` 49, 54 / 31, 36 | kit › Tokens |
| 1 | `longpress` | primitive | — | no (Stage #1) | — | kit › Interaction conventions |
| 2 | LampButton | primitive | longpress | yes (`size` md/sm/cell and `width` exist); needs Stage #2's additions (`join`, `onlongpress`, `onlongrelease`, `data-face`) plus, on `sm`, `pad: 10 \| 14` (the horizontal padding, default 14) and `text: 12 \| 13` (the label size, default 13): In process is `sm, pad 10, text 12`, Link `sm, text 12`, the part On `sm, width 48, text 12`; `size: 'xs'` (26 tall, padding 0 10, 12px: Keep type); and `waiting: boolean` (the waiting face in `--lamp`) | 94, 112, 156–170, 207, 222, 260–265, 286 / 76, 94, 138–152, 189, 204, 242–247, 268 | kit › Faces |
| 3 | Button | primitive | longpress | no (Stage #3); add `sm` (28 tall, padding 0 10/12/14/18, 12–13px, label `--t2` or `--t`), `square-sm` 28 × 28 (11–14px), `track` 40 × 32 (12px), `ots-sm` 32 × 28 (14px), `half` 40 × 24 (11–14px, the half transport) and `wide` 88 × 24 | 146–147, 185–187, 220–223, 248, 374–375, 383–384, 388–389, 431–441 / −18 | kit › Faces |
| 3a | **TextButton** (new) | primitive | — | no | 157, 185, 255, 272, 296, 454 / 139, 167, 237, 254, 278, 436 | a faceless button: heights 20 / 24 / 26 / 28, 12–13px, colour `--t`, `--t2`, `--m` or `--ending`, no padding unless given (Revert 0 6); the kit's text-button rules (kit › Faces). Used by Revert, + Add send, Remove, Change, the Harm/Arp type, Sends 4–6, Insert, the Sound cell, "61 keys", the style name is an AccentBlock button |
| 4 | ChosenTabs | primitive | — | no (Stage #4); add `size: 'mini'` (24 tall, padding 0 10, 12px, the chosen as a full block) | 288–292, 309–321 / 270–274, 291–303 | One Touch › Timing (`mini`); the half band's header tabs are the kit's `header` size (RK-D24) |
| 5 | AccentBlock | primitive | — | no (Stage #6); add `size: 'compact'` (16 tall, padding 0 6, 13 / 500) | 127, 382 / 109, 364 | Compact block; kit › Knobs |
| 6 | StatusDot | primitive | — | no (Stage #7) | 86, 129 / 68, 111 | Compact block |
| 7 | PartMarks | primitive | — | no (Stage #8) | 202–203, 343–345 / 184–185, 325–327 | Parts table › Sound |
| 8 | GroupHeader | primitive | — | no (Stage #9); add heights 28 and 32 | 139–142, 179–188, 234–237, 253–256, 277–280 / 121–124, 161–170, 216–219, 235–238, 259–262 | Keyboard group, Head row, Groups row |
| 9 | **Readout** (new) | primitive | — | no | 208–214 / 190–196 | Parts table › Readouts |
| 10 | **SettingRow** (new): label, value, − + | primitive | Button | no | 143–154 / 125–136 | Keyboard group |
| 11 | **SettingLamp** (new): a LampButton + a note or TextButton, in a hairline row | primitive | LampButton, TextButton | no | 155–171 / 137–153 | Keyboard group |
| 12 | FaderStrip | primitive | — | no (Stage #11); add `size: 'half'` | 325–349; data 518–567 / 307–331; 499–548 | Kit additions › Half band |
| 13 | Knob | primitive | — | no (Stage #12); add `size: 'half'` | 393–403; data 571–587 / 375–385; 552–568 | Kit additions › Half band |
| 14 | Pad | primitive | — | no (Stage #13); add `size: 'half'` | 404–424 / 386–406 | Kit additions › Half band |
| 15 | KeyStrip | primitive | — | no (Stage #14) | 456–465; data 589–609 / 438–447; 570–590 | kit › Key strip |
| 16 | StatusLine | primitive | — | no (Stage #15) | 449 / 431 | kit › Status line |
| 17 | HealthSlot | primitive | — | no (Stage #16) | 88 / 70 | kit › App bar |
| 18 | ChordReadout | primitive | — | no (Stage #17); add `size: 'compact'` (48px, one line, tones inline) | 131–135 / 113–117 | Compact block |
| 19 | **RackReadoutTab** (new): the app bar's rack readout as the current block | primitive | StatusDot | no | 63 / 45 | Kit additions › App bar |
| 20 | **KeyReadouts** (new): Split, Detect, Left, Right, keys | primitive | TextButton | no | 450–454 / 432–436 | Kit additions › Keys row |
| 21 | **MapLine** (new, `mapLine` and its text) | primitive | — | no | 238–245 / 220–227 | Controller map |
| 22 | OneTouch | complex | Button | no (Stage #28); add `size: 'sm'` (32 × 28 buttons) | 64–70, 281–287 / 46–52, 263–269 | Kit additions › App bar; One Touch |
| 23 | PageTabs | complex | ChosenTabs | no (Stage #22); add `current: null` | 71–82 / 53–64 | kit › App bar |
| 24 | LaunchkeyStatus | complex | StatusDot | no (Stage #23) | 86 / 68 | kit › App bar |
| 25 | AppBar | complex | PageTabs, LaunchkeyStatus, HealthSlot, RackReadoutTab, OneTouch | no (Stage #24); add the `page` variant | 61–90 / 43–72 | Kit additions › App bar |
| 26 | CountRow, MetronomeSplit, SectionRow | complex | as the Stage | no (Stage #25–27) | 93–117 / 75–99 | kit › Section row |
| 27 | **CompactBlock** (new) | complex | AccentBlock, StatusDot, ChordReadout | no | 125–136 / 107–118 | Compact block |
| 28 | **KeyboardGroup** (new) | complex | GroupHeader, SettingRow, SettingLamp | no | 138–172 / 120–154 | Keyboard group |
| 29 | **RackHead** (new) | complex | Button, TextButton, StatusDot | no | 179–188 / 161–170 | Head row |
| 30 | SoundCell, `variant: 'rack'` (Stage #21 with no tag, two lines in a 184 × 44 column: the number, name and marks over the instrument line) | complex | TextButton, PartMarks | no (Stage #21) | 198–206 / 180–188 | Parts table › Sound |
| 31 | **PluginActions** (new) | complex | Button, LampButton | no | 219–225 / 201–207 | Parts table › Plugin |
| 32 | **PartRow** (new) | complex | SoundCell, LampButton, Readout, TextButton, PluginActions | no | 196–226 / 178–208 | Parts table |
| 33 | **PartsTable** (new) | complex | PartRow | no | 191–228 / 173–210 | Parts table |
| 34 | **MapGroup** (new) | complex | GroupHeader, MapLine, Button | no | 233–250 / 215–232 | Controller map |
| 35 | **SendsGroup** (new) | complex | GroupHeader, LampButton, TextButton | no | 252–274 / 234–256 | Send effects |
| 36 | **OneTouchGroup** (new) | complex | GroupHeader, OneTouch, LampButton, ChosenTabs, TextButton | no | 276–298 / 258–280 | One Touch |
| 37 | **RackPage** (new): the two columns | complex | CompactBlock, KeyboardGroup, RackHead, PartsTable, MapGroup, SendsGroup, OneTouchGroup | no | 120–301 / 102–283 | Left and Right column |
| 38 | LampRow (band) | complex | LampButton, Button | no (Stage #34) | 350–366 / 332–348 | kit › Lamp row |
| 39 | FaderBank | complex | GroupHeader, ChosenTabs, FaderStrip, LampRow | no (Stage #35); add `size: 'half'` | 306–368 / 288–350 | Kit additions › Half band |
| 40 | **TrackColumn** (new) | complex | GroupHeader, Button | no | 371–377 / 353–359 | Kit additions › Half band |
| 41 | KnobBank, PadGrid, PadBank | complex | as the Stage | no (Stage #36–38); add `size: 'half'` with `header: false` (HalfBand draws the one shared Knobs-and-pads header, #43) | 379–425 / 361–407 | Kit additions › Half band |
| 42 | TransportColumn | complex | GroupHeader, Button | no (Stage #39); add `size: 'half'` | 427–443 / 409–425 | Kit additions › Half band |
| 43 | **HalfBand** (new) | complex | FaderBank, TrackColumn, KnobBank, PadBank, TransportColumn | no | 304–444 / 286–426 | Kit additions › Half band |
| 44 | **KeysRow** (new) | complex | StatusLine, KeyReadouts, KeyStrip | no | 447–466 / 429–448 | Kit additions › Keys row |
| 45 | Rack (page, `Pages/Rack`) | complex | AppBar, SectionRow, RackPage, HalfBand, KeysRow | no | whole board | this file |

Components take props and call callbacks; none reads `app.state` or sends. The page wiring
(`app/src/pages/RackWiring.svelte`, outside `app/src/ui`; RK-D14) reads `app.state`, keeps
`now`, `receivedMs` and the meter holds as the Stage's wiring does, passes them down, and
maps each callback to its command, its editor call or its interim target (D32).

## Gap against today

| Area | In `app/src` now | Change |
|---|---|---|
| The rack | `panels/rack/RackPanel.svelte` (the right-hand drawer, `ui.rack`, Alt+O, and Library's dock), `RackSlot.svelte` (one part: sliders, EQ, compressor, inserts, sends), `rack.ts`, `strip.ts` | Replaced by the Rack page (`ui.page = 'rack'`, Stage.md D2): readouts instead of sliders, the strip details on Channel (#501). The drawer stays mounted until #517, #516 and #504 land (its map editor, OTS selects and prompts are the interim targets, D32), then goes. `rack.ts`'s `rackName`, `panLabel`, `signed`, `targetLabel` move to `app/src/ui/Rack/` |
| Rack navigation | `lib/nav.ts` entry `nav.rack` (Alt+O toggles the drawer); the Stage's rack name | Alt+O and the rack readout open the page (`ui.page`, a new `UiStore` field per Stage.md D2, added by whichever lane goes first); `nav.rack`'s body says so (RK-C1) |
| Sound names on faders | `panels/launchkey` strips show the sound names | The half band's strips (kit › FaderStrip) |
| Plugin actions | `RackSlot` (Edit, Reload, In process, Replace…), `panels/parts/parts.ts` (`inProcessPending`, `pluginStatusLine`) | `PluginActions`; `inProcessPending` moves to `app/src/ui/Rack/plugin.ts` with its tests; the instrument line is a new `instrumentLine` in `app/src/ui/Rack/sound.ts` (RK-D7), beside today's `instrumentName` in `panels/sounds/model.ts`, which stays for the Library footer |
| Send effects | `RackPanel`'s Send effects section (type selects, return sliders, Override, Remove, Add send with a kind select) | `SendsGroup`: Keep type, Remove the last, Add send (Hall); types and returns on Effects (#519) |
| One Touch | `RackPanel`'s OTS cards (rack selects, Link, timing) and `panels/parts/parts.ts` (`otsLine`) | `OneTouchGroup`; the selects move to Library › Racks (#516) |
| Compact block, half band, keys row | none (`panels/channel/ChannelView.svelte` has no band of its own) | New (Kit additions); shared with Channel (#501) |
| Screenshot tool | `scripts/shots.ts` | The Stage's gap item (per-story viewport and masks, Stage.md D39) is needed here too; built once, by whichever lane goes first |

## Contract changes needed

RK-C1 and the Stage's C5 block the build (`TipKey` and the tooltip catalog test: the kit's
app bar and section row use C5's `app.health`, `nav.channel` and `metronome.settings`).
RK-C2 and RK-C3 don't; each says what the screen does until it lands. The Stage's C1
(`quickRacks.loaded`) applies here too and is not repeated.

1. **RK-C1 · Tooltips** (`app/src/help/tooltips.ts`, `app/docs/controls.md`), **lands before the
   build**: one new key, `part.octave` (the octave readout: −2..2, wheel and arrows, OTS
   recalls set it too); rewrite `rack.harmony_type` (it opens the Harm/Arp page, not the
   drawer), `nav.rack` ("shows the Rack page", Alt+O) and `stage.rack_name` (the readout opens
   the Rack page; on it, it is the current page); `rack.map` gains "Edit map opens the
   editor"; `fx.send_remove` says the page's Remove takes the last added send and Effects
   removes any; `keystrip.range` says a click cycles 49, 61, 88.
2. **RK-C2 · A replace mode for Library › Sounds** (`app/src/panels/library/SoundsTab.svelte`,
   `app/src/lib/store.svelte.ts`; owned by the Library › Sounds spec, #513): `ui.openLibrary`
   gains a `replace: true` option, under which a row's click sends `replacePartSound` (the
   mix kept) instead of `assignSound`. Until it lands, Replace… opens Library › Sounds as it
   is and the pick sends `assignSound`, which resets the part's mix to the sound's defaults;
   the `rack.replace` tooltip's "keeps its level, pan, sends and octave" is true only after
   RK-C2 (RK-C1 may say "once replace lands" meanwhile). The quick sound list (#514) takes
   the same option.
3. **RK-C3 · Prompts on the page** (Prompts, #504, owns `liveRack.prompt`): until it lands, the
   wiring opens today's Rack drawer (`ui.toggleDrawer('rack')`, opened, not toggled) whenever
   `liveRack.prompt` goes from null to a prompt, and the drawer's own prompt block (Save first
   / Discard and switch / Keep editing; the sound-name fields) answers it, as today. The page
   draws nothing for the prompt (D32: today's equivalent exists; RK-D22). When #504 lands the
   wiring stops opening the drawer.

## Checks

Vitest (`npx vitest run` on the page and component tests), each against the board fixture
unless it says otherwise. They read roles, names, attributes and the commands sent (a fake
`send`) and the `data-face` / `data-hue` hooks (kit, Stage.md D41); never computed colours or
layout.

1. Layout: in DOM order, the `header`, the toolbar "Switches and helpers", the `section`
   "Rack page: Sunday drive", the `section` "Band", then the `section` whose name starts
   "Keys:"; the app bar's rack readout has `aria-current="page"`, reads "Rack A1 Sunday drive"
   with the modified dot, and no page tab has `aria-current`.
2. Compact block: a group named "Now playing: Sunday Drive Pop, 104 BPM, Running, chord Am7,
   Main B"; the style button reads "Sunday Drive Pop" and a click calls `onOpen('browser')`;
   "104 BPM"; the run dot's `aria-label` is "Running"; the chord's runs are "Am" and "7"; the
   tones read "A C E G"; the section reads "Main B" with `data-hue="main"`. Stopped
   (`transport.running` false, `main` 1): the dot is hidden, the section "Main B" has
   `data-hue="m"`, no tones when `chord.name` is null ("—").
3. Keyboard group: Split reads "F#2", − sends `moveSplit {delta: -1}`, + `{delta: 1}`;
   Transpose "0", + sends `stepTranspose {keyboard: 1, master: 0}`; at `transposeKeyboard` 12
   the + is `aria-disabled`; Harmony/Arp has `aria-pressed="false"` and a click sends
   `toggleHarmonyArp`; "Standard Duet 1" is a button whose click calls `onOpen('harmArp')`;
   Manual Bass is `aria-disabled` with `data-face="disabled"` and sends nothing, and reads
   "Upper only"; with `chord.upper` true it sends `toggleManualBass` and the note is empty;
   Left Hold sends `toggleLeftHold`; the slot reads "A1"; Store sends `toggleQuickRackStore`
   and is `aria-disabled` when `quickRacks.readOnly`.
4. Head row: "Sunday drive", "Mine", "Modified"; Revert sends `revertRack`; Save
   (`data-face="chosen"`) sends `saveRack`; Save as… calls `onSaveAs('open')`; with `saveAs`
   `{open: true, error: null}` a textbox named "New rack name" is focused and reads "Sunday
   drive", typing calls `onSaveAs('typed')`, Enter calls `onSaveAs({name: "Sunday drive 2"})`,
   Esc calls `onSaveAs('close')` and the window's Esc handler doesn't run; with `error` "A
   rack called Sunday drive 2 exists" the alert shows it. Unmodified with an id: no Revert, no
   "Modified", Save `aria-disabled`. Id null, name "New rack": "Untitled rack" (head row and
   app bar readout alike), "Unsaved", Save enabled. The wiring (its own test): after
   `onSaveAs({name})` it sent `saveRackAs {name}`; a later `message` `{seq: sent + 1, error:
   true, text}` sets `error`; a `liveRack.id` change to "sunday-drive-2" sets `open` false.
5. Parts table: four rows; R2's sound reads "41 Silk Strings" with the edited dot and the line
   "Sampler Deluxe · Strings"; R3's shows ⚠, the line "Brass Deluxe · missing", its tag
   `data-hue="d"`; a click on R1's sound calls `onOpen({kind: 'quickList', part: 0})`; its
   `aria-label` is the Stage template's ("Right 1 sound: 1 Stage Grand. Opens the quick sound
   list"). `instrumentLine` (pure): the eleven cases of RK-D7 (a plugin with a preset, without
   one, missing, failed, loading, muted; no plugin with a `saved:` SoundFont patch, a `saved:`
   plugin-sourced patch, an `sf:` id, a GM voice, and nothing known → "").
6. On lamps: R1 On is named "Right 1 on" with `aria-pressed="true"`, R3 reads "Off"; a click on
   R1 sends `togglePart {part: 0}`; a 350 ms press sends `setLayer {layer: {type: 'swap',
   part: 0}}` and no toggle; with `surface.layer` `{type: 'swap', part: 0}` it reads "Swap" and
   a click sends `setLayer {layer: {type: 'none'}}` (fake timers).
7. Readouts: R2's pan slider is named "Right 2 pan", `aria-valuetext` "L12", `aria-valuenow`
   52; ArrowUp on it sends `setPartPan {part: 1, pan: 53}`; a wheel notch down on R1's level
   sends `setPartVolume {part: 0, volume: 89}`; a 12px upward drag on L's reverb sends one
   `setPartSend {part: 3, send: 'reverb', value: 51}` (one per frame); double-click on R2's
   level sends `setPartVolume {part: 1, volume: 100}`; L's octave reads "−1", ArrowDown sends
   `setPartOctave {part: 3, octave: -2}` and ArrowDown again sends nothing (clamped). R3's
   readouts have `data-hue="d"`.
8. Sends 4–6 reads "0 · —" on every row; with three added sends and R1's `strip.sends`
   `[40,0,0,5,6,7]` it reads "5 · 6 · 7"; with none, "—"; a click calls `onOpen({kind:
   'channel', part})`. Insert: R2 reads "Tremolo" and "30"; R1 reads "None" with no value and
   `data-hue="m"`; with R2's slot `on` false its name has `data-hue="m"` and the label ends
   ", off. Opens Channel".
9. Plugin cell: R1 shows Edit, Reload (`aria-disabled`) and In process (`aria-pressed="true"`);
   R2 shows "Save sound" whose click sends `saveSound {part: 1}`; R3 shows Replace… (a click
   calls `onOpen({kind: 'replace', part: 2})`) and "silent until replaced", and no Edit; In
   process on R1 sends `setPluginInProcess {id: "aumu Smp7 Fake", inProcess: false}`; with
   `plugin.status` "failed" and `missing` false on R1, Reload sends `reloadPartPlugin {part:
   0}` and the sound shows ✕; with the list entry's `canRunInProcess` false the lamp is
   `aria-disabled`; a part with no `plugin` has an empty cell. Edit on R1 calls `onOpen({kind:
   'editor', part: 0})`, and the wiring's test shows it calls `pluginEditor(0, true)`.
10. `inProcessPending` (pure): the `inProcessPending` assertions in
    `app/src/panels/rack/RackPanel.test.ts` move with it to `app/src/ui/Rack/plugin.test.ts`;
    the lamp wears `data-face="waiting"` when it is true.
11. Controller map: "Default"; the lines read "Right 1 · Right 2 · Right 3 · Left" and "R1 R2
    R3 L volumes · Harmony · Metronome · — · Tempo"; with knob 7 `rotaryFast` the header reads
    "Edited" and the knobs line ends "Rotary · Tempo"; with fader 2 `partPan` of part 1 the
    faders line reads "R1 level · R2 pan · R3 L volumes". `mapLine` (pure) covers each label
    in the table, a two-part run ("R1 R2 volumes"), and an unknown kind ("?"). Edit map calls
    `onOpen('mapEditor')`.
12. Send effects: rows "1 Reverb from style", "2 Chorus Celeste", "3 Delay from style"; send 2's
    Keep type has `aria-pressed="true"` and a click sends `setRackSendOverride {send: 1, on:
    false}`; send 1's sends `{send: 0, on: true}`; row 4 reads "4 Phaser added" and Remove
    sends `removeSend {send: 3}`; + Add send sends `addSend {kind: 'hall'}`; with six sends it
    is `aria-disabled`; with three sends row 4 is absent.
13. One Touch: button 2 `aria-pressed="true"` and `data-face="chosen"`; 3 sends `recallOts
    {index: 2}`; button 4's `aria-label` is "One Touch 4, loads rack Organ"; Link sends
    `toggleOtsLink`; the "At Main change" tab has `aria-selected="true"`, "Immediate" sends
    `setOtsLinkTiming {timing: 'immediate'}`; the line reads "OTS 4" "loads Organ"; with
    `ots.racks` all null it reads "OTS 1–4" "the style's own"; with racks on 2 and 4 it reads
    "OTS 2, 4" "load Jazz, Organ"; Change calls `onOpen('otsChooser')`. The app bar's One
    Touch group sends the same `recallOts`.
14. Half band: nine strips; strip 1's slider `aria-valuetext` "Right 1 90"; a 20px upward drag
    on it sends its `set` with `volume` 148 clamped to 127 (`round(20 × 127 / 44) = 58`,
    90 + 58); strip 2 shows "↕"; strips 7–8 read "—" and aren't focusable; the Reverb tab sends
    `setFaderLayer {layer: 'reverb'}`; the lamp row and master button as Stage.md check 11;
    Track ◀ sends `surface.controls[trackPrev].action`; knob ▲ is `aria-disabled` on page 1;
    the pads header reads "Sections · next Main C"; pad 10 `data-face="solid"`, pad 11
    `data-face="waiting"`, pad 16 reads "Start"; ▶ sends `startStop`, Style tempo
    `resetTempo`, Fill ▲ `fillUp`.
15. Half-band maths (pure, `app/src/ui/FaderStrip/meter.ts` with `travel` 44): `height(0.0724,
    44)` is 27, `height(0, 44)` is 0; `capTop(90, 44)` is 13.
16. Keys row: the status line is empty and has no focusable content; with `message` `{text:
    "Left plays the bass while Manual Bass is on", error: true}` it shows the text and ⚠ and a
    click sends `clearMessage`; the readouts read "Split F#2", "Detect lower", "Left G A C E"
    (`data-hue="l"`), "Right E4 A4" (`data-hue="r1"`), "61 keys"; a click on "61 keys" calls
    `onKeyRange(88)` (and with `keyRange` 88, `onKeyRange(49)`); with no held keys Left and
    Right read "—"; with `keyboard.detection` `[0, 127]` Detect reads "full". The wiring's
    test: with `ui.keyRange` null the prop is `rangeFor(null, names)`: 61 with no Launchkey
    among `io.inputs`, 49 with the mock's "Launchkey 49 MK4 …".
17. Every interactive element has a `data-tip` in the catalog (the existing tooltip test); the
    name field carries `rack.save_as_name`.
18. Tab order: the app bar's readout, the One Touch buttons, the tabs, the section row, the
    compact block's style button, the Keyboard group top to bottom, the head row's buttons
    (and the name field while open), then each part row left to right, the three groups, the
    band, the keys row's "61 keys".

**Story and screenshot checks** (`npm run shots -- Rack`, real Chrome), for what jsdom can't
see:

- `Pages/Rack` › `Board` (export `Board`, layout `fullscreen`, `parameters.shots = { viewport:
  { width: 1440, height: 900 }, mask: ['[data-shot-mask="when"]', '[data-shot-mask="sends"]'] }`;
  the Send effects group carries `data-shot-mask="sends"`) renders `Rack` with `boardState`,
  `boardNow` and `boardMeterHolds`, unscaled, in both themes, against
  `app/src/ui/Rack/crops/Board-dark.png` and `Board-light.png` (copies of
  `docs/design/push/png/Rack-Dark.png` and `Rack-Light.png`, 1440 × 900): at most 0.02 of the
  unmasked pixels differ. It covers the compact chord's glow, the half band's meters, arcs and
  pad faces, the dimmed part row and the disabled Manual Bass.
- `Components/PartRow` › `SoundFont` (no `plugin`, `sound` `{ id: "sf:GeneralUser.sf2:0:0",
  name: "Grand Piano" }`): the instrument line reads the font's name and the plugin cell is
  empty; no crop. › `Failed` (`plugin.status` failed, not missing): ✕ and Reload enabled.
  › `Loading` (status loading, stage "instantiating"): the line ends "· loading…", Edit and
  Reload disabled.
- `Components/SendsGroup` › `ThreeAdded` (sends 4–6 Phaser, Hall, Room): row 4 reads "4 Phaser
  5 Hall 6 Room added" in one line within 325px, Remove at the end; › `None`: three rows, no
  row 4. No crop.
- `Components/RackHead` › `LongName` (a 60-character rack name, modified): the name ends in an
  ellipsis, the hint is cut before the buttons, the buttons keep their size; › `SavingAs`: the
  name field in place of the hint.
- `Components/CompactBlock` › `LongChord` (`name: "C#m7b5/G#"`): the chord stays on one line
  and is clipped at the block's right edge (RK-D3); judged by Inspect.
- `Components/OneTouchGroup` › `TwoRacks` (racks on OTS 2 and 4): the line reads "OTS 2, 4"
  "load Jazz, Organ" with an ellipsis before Change.
- axe finds no violation on any story.

## Decisions

- **RK-D1 · Shared parts are specified here.** The compact block, the half band, the keys row
  and the app bar's page variant are drawn on the Channel board and this one; the kit doesn't
  have them yet. This file specifies them in full under Kit additions so a builder needs no
  other document; the Channel spec (#501) will say the same, and kit.md takes the text from
  whichever merges first. A difference between the two is a bug in the later one.
- **RK-D2 · Run dot.** The compact block's dot is the Stage tempo row's run state in 6px: solid
  green running, a hollow ring for Sync start, hidden (space kept) when stopped, so the row
  never shifts.
- **RK-D3 · Compact chord doesn't shrink.** The chord element clips at its own right edge
  (`min-width: 0; overflow: hidden`) and the tones and section keep their size, so a long
  chord never pushes anything out of the 320px block; it is not measured (the Stage's fit rule
  is for 128px). The tones follow the Stage's spelling.
- **RK-D4 · Disabled face.** Manual Bass disabled wears the kit's disabled face (label `--d`),
  not the board's 45% opacity; one disabled look everywhere (kit › Faces).
- **RK-D5 · Badge.** "Mine" when the live rack has a saved rack of the user's behind it
  (`liveRack.id`); "Unsaved" otherwise. Style racks are never the live rack (an OTS copies into
  the parts), so there is no "Style" badge here.
- **RK-D6 · Save as… inline.** The board has no name field; the page keeps a name field in the
  head row (the wiring's `saveAs` state), as today's drawer does, rather than wait for Prompts
  (#504). Prompts owns `liveRack.prompt`, not this field.
- **RK-D7 · Instrument line.** A new pure `instrumentLine(part, patches, soundFontFile)` in
  `app/src/ui/Rack/sound.ts`. With a `plugin` (whatever its status): `plugin.name`, then " · "
  and `plugin.preset` when it is not null; then " · missing" when `missing`, else " · failed"
  when the status is `failed`, " · loading…" while `loading`, " · muted" when `muted`
  ("Sampler Deluxe · Concert D", "Brass Deluxe · missing"). Without a plugin: the SoundFont
  file's name without its `.sf2` extension, from the part's `sound.id`: for `saved:<id>`, the
  `source.file` of that `soundLibrary.patches` entry (a plugin-sourced patch with no `plugin`
  on the part reads the `componentId`'s plugin name from `plugins.list`, else the id); for
  `sf:<file>:<bank>:<program>`, `<file>`; no `sound` (a GM voice), `io.soundFontFile`; nothing
  known, "" (an empty line, 16px kept). This differs from today's `instrumentName`
  (`panels/sounds/model.ts`, which names the SoundFont for a failed plugin and "the synth" as
  a last resort); that function stays for the Library footer.
- **RK-D8 · Sends 4–6 and Insert are readouts that open Channel.** The board draws them as
  buttons with no editor of their own; the part's strip (Channel, #501) has the controls, and
  swap-mode knobs 7 and 8 move the insert amount and send 4 from the hardware. No popover.
- **RK-D9 · Plugin cell.** Edit opens the editor through the shell (`app.pluginEditor(part,
  true)`, the Session's `pluginEditor`, not a command: app-api.md › Instrument plugins) and
  needs a playing plugin with an editor. The
  second button is Save sound while the sound is edited (`saveSound`, which overwrites the
  user's own sound or makes a new one), else Reload, enabled only when the plugin is failed or
  muted (the engine refuses it otherwise). In process shows the plugin's setting
  (`plugins.list[].inProcess`), and the waiting face while the setting hasn't taken effect
  (`inProcessPending`), instead of today's ↻. A missing plugin shows Replace… only; a part on
  a SoundFont voice shows nothing.
- **RK-D10 · Map lines.** A pure `mapLine` with the label table above; runs of part levels
  compact to "R1 R2 R3 L volumes" and the default fader map reads the full part names, so the
  default reads as the board's.
- **RK-D11 · Edit map interim.** Until #517 lands, Edit map opens today's Rack drawer, which
  has the map's selects (D32: today's equivalent exists). Alt+O opens the page (Stage.md D2);
  the drawer is reached only from Edit map, Change and the prompts until their specs land.
- **RK-D12 · Send effects rows.** Every style send (1–3) gets its Keep type lamp, Delay
  included (the command exists for all three and the board left Delay's out); the added sends
  share a fourth row with one Remove that takes the last one (a row with three Removes
  doesn't fit 325px); + Add send adds a Hall, since the board has no kind picker and Effects
  (#519) sets kinds. The board story masks the group.
- **RK-D13 · OTS → rack chooser.** Change opens Library › Racks (#516), where each rack's row
  can be linked to an OTS; until then the Rack drawer's OTS cards. The page shows the links
  as one line.
- **RK-D14 · Page wiring.** `Rack` is pure; `app/src/pages/RackWiring.svelte` connects it as
  `StageWiring` does, and `App.svelte` mounts it when `ui.page` is `rack`, in the D1 scaler.
  The wiring also owns the editor call and the interim targets.
- **RK-D15 · Half-band fader maths.** The half strip is the full strip with `travel` 44 and
  the board's offsets (value 16, meters from 19 to 3 above the bottom): the same formulas as
  kit › FaderStrip with 44 for 223, so one `app/src/ui/FaderStrip/meter.ts` serves both:
  `height(x, travel = 223)` and `capTop(value, travel = 223) = round((1 − value / 127) ×
  travel)`, both exported, the defaults keeping the Stage's checks as they are.
- **RK-D16 · Half-band pad captions.** The half pads use the board's short captions ("Sync
  start ▶", "Auto fill", "Sync stop ■", "Start"), no numerals and no "NEXT"/"ARMED" word: the
  face carries the state. The full band keeps the kit's captions.
- **RK-D17 · Keys row readouts.** Detect reads "lower" when `keyboard.detection[1]` ≤
  `keyboard.leftSplit`, "upper" when `detection[0]` > `leftSplit`, else "full"; Left lists
  the held notes whose `zone` is `left` as pitch classes ("G A C E"), Right the held notes
  whose `zone` is `right` with their Yamaha octave ("E4 A4", C3 = 60), both in the order of
  `keyboard.held` (low to high), spelled with sharps (C C# D D# E F F# G G# A A# B); "—" with
  none. The "{n} keys" button shows the `keyRange` prop (the wiring resolves it with
  `rangeFor(ui.keyRange, io.inputs.map(i => i.name))`, `panels/keystrip/keyboard.ts`: the
  choice, else the connected Launchkey's size by its port name, else 61) and a click calls
  `onKeyRange` with the next of 49 → 61 → 88 → 49 from the shown value; the wiring calls
  `ui.setKeyRange(n)`. Until Settings › Keyboard (#530) lands.
- **RK-D18 · Tags are not controls.** The board draws the part tags as text; the half band's
  strip names open Channel, so the tag needn't. The Stage's tags stay buttons there.
- **RK-D19 · Readout drag.** A readout steps like a knob (one per 4px, Shift one per 12px)
  rather than like a fader, since there is no track to map a travel onto; the value never
  jumps to the pointer.
- **RK-D20 · Two One Touch groups.** The app bar's and the page's One Touch buttons are the
  same control drawn twice, as the board does; both exist, both send `recallOts`.
- **RK-D21 · Leaving the page.** Esc is not a page key (the window handler keeps it for the
  Browser and Library); the Stage tab, Alt+O and the display tabs leave. An interim Channel
  target first returns to the Stage, since today's `ChannelView` lives in the Stage's
  display; when #501 lands, `ui.page = 'channel'` replaces that.
- **RK-D22 · Prompts interim.** The Rack drawer's prompt block can't be mounted on its own
  (it shares the drawer's state), so until Prompts (#504) the wiring opens the drawer when a
  prompt appears, and the drawer answers it as today.
- **RK-D23 · Names.** The app bar readout and the head row both show `rackName` (so a new
  rack reads "Untitled rack" in both places); the Stage's readout should do the same
  (Stage.md › Sounds row reads `liveRack.name` raw).
- **RK-D24 · Small departures from the board and the kit.** On the Rack page no page tab is
  ever chosen, drawer or not (the readout is the current item). The half band's layer tabs
  use the kit's `header` tab padding (10px, not the board's 8px): one tab size in the kit,
  and the 2px per tab is within the screenshot threshold.
- **RK-D25 · Table semantics.** The parts table is a real ARIA table (`table` › `row` ›
  `cell` / `columnheader`); the controls sit inside the cells, so axe's required-children
  rule holds and screen readers read the column names.

## Follow-ups

- Remove any added send from the page (a button per send, "✕" 12px `--ending` after each
  name; `removeSend { send }` exists), if the owner wants it here rather than on Effects.
- A face for a plugin's `inProcessFallback` warning on the row (today's "⚠ in process"); the
  health slot and the instrument line don't show it.
- Rack-MapEdit (#517): the editor's delta against this spec.
- The compact block, half band and keys row move into kit.md once #501 or this spec merges
  (RK-D1).
- Per-part insert 2 and the compressor are on Channel only; a reach from here if players ask.

## Kit additions

What this screen needs that kit.md doesn't have yet. Written as kit sections, to be moved
there as they are (RK-D1). Boxes are at 1440 × 900 on the Rack and Channel boards.

### App bar, page variant

The kit's app bar (`24,24 1392×36`, the same wordmark, tabs and right area) with two items after
the wordmark, on every page that replaces the display (Channel, Effects, Quick Racks, Multi
Pads, Looper, Harm/Arp, Rack):

- **Rack readout**, a `<button>` 16px after the wordmark, `align-self: flex-end`, 24 tall,
  `box-sizing: border-box`, padding 0 10, no border, no radius, items centred, gap 6, 14px, no
  wrap: "Rack", the slot (Stage.md D21 rule; nothing when none), the name
  (`rackName(liveRack)`, RK-D23; `max-width: 240px`, ellipsis), then a 5px round dot when
  `liveRack.modified`. On the Rack page it is the current item: the chosen face (`--t` fill,
  `--g` text and dot, weight 500, `aria-current="page"`), still focusable (first in the tab
  order), `cursor: default`, and a click does nothing. On other pages it is a text button, 14
  / 400 `--t` (the dot `--t`), `cursor: pointer`, that opens the Rack page (`ui.page =
  'rack'`). Tooltip `stage.rack_name`. `aria-label` "Rack: {name}{, modified}{, on Quick Rack
  A1}. {This page | Opens the Rack page}".
- **One Touch**, 12px after the readout: the Stage's One Touch group (Stage.md › Style line:
  "One Touch" 14 `--m`, four 32 × 32 buttons, gap 4, `recallOts`, tooltips `ots.1`–`ots.4`),
  32 tall.
- **Page tabs** (`margin-left: auto`): as the kit; on the Rack page no tab is chosen (the
  readout is), so `PageTabs` takes `current: null`, and that holds while a drawer is open over
  the page (RK-D24: the kit's "chosen while its drawer is open" rule is the Stage's).
- **Landmarks:** the app bar is a `header`; the section row is `role="toolbar"`
  `aria-label="Switches and helpers"` (both as the boards draw them; the kit should say so).

### Compact block

`320×84` at the top of a tall page's left column: specified under Left column › Compact block
above (rows, type, controls, `aria-label`). Beat, bar and the next section are not in it; the
count row in the section row has them.

### Half band

`24,600 1392×176` on tall pages, margin-top 20 under the page. Four sections in a row, gap 12:
**Faders** `24,600 614×176`, **Track** `650,600 40×176`, **Knobs and pads** `702,600 614×176`,
**Transport** `1328,600 88×176`. Everything the full band has, at half height; the commands,
tooltips and Launchkey mappings are the full band's (kit › Full band) and are not repeated
here. There is no on-screen Shift (every Shift function has its own control).

**Faders.** The header row (36 tall, hairline) is the kit's: "Faders", Panel | Style, a
separator, "Layer", Vol | Pan | Reverb | Chorus | Delay, all the kit's `header` tabs (35
tall, padding `11px 10px 0`; the board's layer tabs are 2px narrower each, RK-D24). Then 8px,
the strips (84 tall), the lamp-row headers (14 tall:
12px, line-height 12, bottom hairline, "Part on/off" over columns 1–4, "Functions" and
"Launchkey fader buttons 5–9" over 5–9), 2px, the lamp row (32 tall, 12px labels, the kit's
Lamp row). Nine strip columns `repeat(9, minmax(0, 1fr))`, gap 8 (61.1 wide at 1440).

The **half FaderStrip** (`size: 'half'`), a column 84 tall: the fader (a `role="slider"` button
66 tall, `position: relative`, no face) over the name button (18 tall, 12 / 500 in the hue, the
marks at 11px, centred, gap 4). Inside the fader, with `TOP = 19` and `TRAVEL = 44`:

| Part | Box inside the fader | Drawing |
|---|---|---|
| Value | top 0, 16 tall, centred | 15 / 300 in the hue (layers: `--t`); the kit's value texts |
| Meter backgrounds | `left: 50% − 11px` and `50% − 3px`, width 6, top 19, bottom 3 | `--mbg`; hidden (opacity 0) when unused or layered |
| Meter bars | the same lefts, bottom 3, width 6 | peak (left) and RMS (right), heights `height(x, 44)` px, the hue at `--meter-mix`; none when off, unused or layered |
| Peak tick | `left: 50% − 11px`, width 14, height 1, bottom `3 + height(hold, 44)` | `--peak`; drawn only when the bars are (none when off, unused or layered) |
| Groove | `left: 50% + 6px`, width 3, top 19, bottom 3 | `--track`; unused: the kit's dashed gradient |
| Fill | the groove's left, bottom 3, width 3, height `44 − capTop` | the hue (layers `--t`; off: the hue at 35%), glow `0 0 6px` at `--fill-glow-mix` in Vol only |
| Cap | `left: 50% + 1px`, width 8, height 2, `top = capTop + 19 − 1` | the hue (as the fill) |
| Ghost | `left: 50% − 15px`, width 30, a 1px dashed `--m` top edge, `top = round((1 − position / 127) × 44) + 19` | while `waiting` |
| ↕ | top 0, left 0, 12px `--m` | while `waiting` |

`capTop = round((1 − value / 127) × 44)` (unused: 44); `height(x, travel) = round(travel ×
clamp((20·log10(x) + 60) / 60, 0, 1))`, 0 for x = 0 (the kit's formula with the travel as a
parameter, RK-D15). Drag: `value = clamp(round(v0 + (y0 − y) × 127 / 44), 0, 127)` (the kit's
rule with 44). Pan draws from the middle (value 64) as the kit says. The kit's layers, part
off, unused and rack-target rules apply unchanged.

**Track.** Header "Track" (36 tall, hairline, 14 `--m`), 8px, then ◀ and ▶: 40 × 32 off-face
buttons, 12px, gap 8, bound to `surface.controls[trackPrev]` / `[trackNext]` as the Stage's
style line (Stage.md D38; tooltips `style.prev`, `style.next`).

**Knobs and pads.** A column 176 tall whose content is 160 (header 36, 8, knobs 48, 8, pads
60); the last 16px stay empty at the bottom. `HalfBand` draws the one header row, and the
half KnobBank and PadBank under it take `header: false`. The header (36 tall, hairline, items centred, gap 8, no wrap): "Knobs"
14 `--m`; the knob page accent block (padding 0 8, 13px, line-height 22; swap mode as the kit);
▲ and ▼ as 28 × 28 off-face buttons, 11px (`stepKnobPage`, disabled as the kit); "Page" 13
`--m` with `{pageNumber}/{pageCount}` in `--t`; then (`margin-left: auto`) "Pads" 14 `--m`;
the pad page name 14 `--t` and, on Sections while a next section exists, " · " then "next
{next}" in that section's hue (`Sections · next Main C`); ▲ ▼ 28 × 28 (`padBankUp` /
`padBankDown` actions); "Bank" 13 `--m` with `{pads.pageNumber}/{pads.pageCount}` in `--t`. No
legend.

Then 8px, the **knobs** (48 tall): `repeat(8, minmax(0, 1fr))`, column gap 6 (71.5 wide). The
**half Knob** (`size: 'half'`) is a button, a centred column: a 32px ring (`conic-gradient(from
225deg, var(--a) 0 <deg>, var(--ring-rest) <deg> 270deg, transparent 270deg)`; No Assign:
both `--mbg`) with a `--g` disc inset 2px holding the value 12 / 300 `--a` (the kit's value
text; empty for No Assign; a trailing "%" stays in the text); a 4px `--a` tip dot at `left =
16 + 15·sin θ − 2`, `top = 16 − 15·cos θ − 2` (θ = 225° + deg, clockwise from up) on the
ring, none for No Assign; then 2px and the name 12 `--t2` (`--d` for No Assign, which reads
"—"), 14 tall. No code line. Control, drag and wheel as the kit's Knob.

Then 8px, the **pads** (60 tall): `repeat(8, minmax(0, 1fr))`, rows 28, column gap 6, row gap
4. The **half Pad** (`size: 'half'`): 28 tall, padding 0 2, radius 4, 1px border (transparent
idle), 12 / 500, line-height 26, letter-spacing −0.2, centred, no wrap, `overflow: hidden`; no
numeral, no group lines. Faces from `level` and `anim` as the kit's table, drawn as: Idle `--btn`
fill, caption in the family hue (utility `--t2`); Absent `--btn`, caption `--d`; Playing the hue
fill and border, caption `--solid-ink`; Next `--btn`, 1px hue border, caption `--t`
(line-height 23) and a 2px hue bar (left 8, right 8, bottom 3, radius 1) with `0 0 6px` glow at
`--bar-glow-mix`, flashing on the LED clock; Armed `--btn`, hue border, caption `--t`, glow at
`--glow-mix`, pulsing; On (a utility switch) `--lamp` fill and border, `--lamp-ink`. Start /
Stop running is Playing in `--ok`. Captions on Sections (RK-D16): Intro I, Intro II, Intro III,
Sync start ▶, Ending I, Ending II, Ending III, Auto fill, Main A, Main B, Main C, Main D,
Break, Tap, Sync stop ■, Start — where ▶ is a CSS triangle (width 0, `border-left: 6px solid
currentColor`, 3.5px transparent top and bottom, 3px left margin) and ■ a 6 × 6 `currentColor`
square, 3px left margin. Other pages caption from `pads.pads[i].label` with the fallback face
(Stage.md D33). `aria-label`, tooltips and actions as the kit's Pad.

**Transport.** Header "Transport" (36 tall, hairline), 8px, then a grid `40px 40px`, rows 24,
gap `3px 8px`: ▶ | ■, Reset | Fade, Fill ▲ | Fill ▼, + | −, Style tempo (spanning both
columns). All off face, radius 4: ▶ 12px `--t` with `padding: 0 0 3px` and, while running, a
2px `--ok` bar (left 8, right 8, bottom 3, radius 1, `--bg` glow), `aria-pressed` = running
(`startStop`; "Start / Stop"); ■ 12px `--t2` (`stop`); Reset and Fade 11px (`sectionReset`,
`toggleFade`; Fade's faces as the kit); Fill ▲ / ▼ 11px with the arrow at 8px (`fillUp`,
`fillDown`; a `role="group"` "Fills"); + and − 14 / 300 (`tempoUp`, `tempoDown`, repeating
while held); Style tempo 11px (`resetTempo`). The same `aria-label`s and tooltips as the kit's
Transport and tempo (the glyphs' labels say the word: "Start / Stop", "Stop").

### Keys row

`24,796 1392×80` on tall pages, margin-top 20 under the half band, a column:

- **Status row** `24,796 1392×20`, items centred, gap 16, no wrap: the kit's **Status line**
  (`flex: 1; min-width: 0`, `role="status"`, 14px, line-height 20, ellipsis; `state.message`,
  its button and tooltip as the kit), then five readouts, 13 `--m` with the value in `--t`:
  "Split {chord.splitName}"; "Detect {lower | upper | full}" (RK-D17); "Left {notes}" with the
  notes in `--l`; "Right {notes}" with the notes in `--r1`; and "{keyRange} keys", a text
  button 20 tall, 13 `--t` (click: RK-D17; tooltip `keystrip.range`; `aria-label` "Keyboard
  size {n} keys. Click for 49, 61 or 88"). Left and Right read "—" with nothing held
  (RK-D17 gives the note rules). Tooltips: `keystrip.split` on Split, `keystrip.keys` on
  Detect, Left and Right (not controls).
- **Key strip** `24,820 1392×56`, 4px below: kit › Key strip, unchanged (its `aria-label` on
  the row's `section`: "Keys: split F#2, left hand G A C E, right hand E4 A4, 61 keys").

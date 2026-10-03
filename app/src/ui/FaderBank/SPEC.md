# FaderBank

## Identity (all stations)

- **Kind:** complex
- **Built from:** GroupHeader, ChosenTabs, FaderStrip, LampRow
- **Purpose:** The band's faders, as on the Launchkey: nine strips (the four keyboard parts, the Style, the Multi Pads, two unused, the master) with their levels and meters, the fader page and layer they control, and the buttons under them.
- **Boards:** `Stage-Dark.dc.html:223-288` (header `:224-240`, strips grid `:243-268`, lamp row `:271-286`; the strips' data script `:425-473`); light: `Stage-Light.dc.html:199-264` (data `:397-445`). Box `Stage 24,432 654×368`: header `24,432 654×36`, strips `24,476 654×272`, LampRow `24,748 654×52`.
- **Not this component's job:** no store, no API, no Tauri, no timer. It doesn't read `AppState`, compute meters or held peaks, coalesce sends, or decide what a strip's name opens: the page wiring does (see "From the state"). It doesn't restate FaderStrip's, LampRow's, ChosenTabs' or GroupHeader's drawing; it lays them out and passes their props. The Style fader page's strips and lamps are #507's (D5).

## API (Component station)

### Props

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `page` | `'panel' \| 'style'` | `'panel'` | The fader page: the chosen page tab. |
| `layer` | `'volume' \| 'pan' \| 'reverb' \| 'chorus' \| 'delay'` | `'volume'` | The fader layer: the chosen layer tab, the header's "· {Layer}", and the layer strips 1–4 get. |
| `strips` | `StripData[]` (exactly 9) | — | The nine strips' data, in order (below). |
| `lamps` | `LampRowProps` | — | Everything LampRow takes, callbacks included, spread onto it unchanged (`width` left unset so it fills the bank). |

`StripData` is FaderStrip's data props, the ones the state decides: `{ label: string; value: number \| null; meter: { peak, rms, hold } \| null; off?: boolean; rackTarget?: boolean; waiting?: boolean; position?: number \| null; edited?: boolean; missing?: boolean; failed?: boolean }` (defaults as FaderStrip's). FaderBank adds the rest by the strip's place (Strips, below).

### Events

| Callback | Fires when | Payload | The wiring sends |
|---|---|---|---|
| `onchoosepage` | a page tab is chosen (click, or ChosenTabs' keys) | `(page: 'panel' \| 'style')` | `setFaderPage { page }` |
| `onchooselayer` | a layer tab is chosen | `(layer)` | `setFaderLayer { layer }` |
| `onfader` | strip *i*'s FaderStrip `onchange` | `(i: number /* 0–8 */, value: number)` | `surface.faders[i].set` with its value field filled (below), at most once per animation frame, the frame's last value |
| `onfaderrelease` | strip *i*'s FaderStrip `onrelease` | `(i, value)` | the same command with `value`, at once (flushes the frame) |
| `onname` | strip *i*'s FaderStrip `onopen` | `(i)` | by strip: parts → Channel for the part, or the Rack page for a rack target; Style → `setFaderPage { page: 'style' }`; Multi Pad → the Multi Pads page; Master → Effects at the master (Stage.md D32 interims) |

LampRow's callbacks are in `lamps` and pass through untouched.

### Children

| Where | Child | Props |
|---|---|---|
| Header | `GroupHeader` | `{ title: 'Faders', detail: layer === 'volume' ? undefined : LAYER_WORD[layer] }`, its snippet holding the header content below |
| Header, after the title | `ChosenTabs` (pages) | `{ tabs: [{ id: 'panel', label: 'Panel' }, { id: 'style', label: 'Style' }], chosen: page, size: 'header', label: 'Fader page (master button)', tip: 'mixer.page' }`; `onchoose` → `onchoosepage` |
| Header, after "Layer" | `ChosenTabs` (layers) | `{ tabs: [{ id: 'volume', label: 'Vol', name: 'Volume' }, { id: 'pan', label: 'Pan', name: 'Pan' }, { id: 'reverb', label: 'Reverb', name: 'Reverb send' }, { id: 'chorus', label: 'Chorus', name: 'Chorus send' }, { id: 'delay', label: 'Delay', name: 'Delay send' }], chosen: layer, size: 'header', label: 'Fader layer', tip: 'mixer.layer' }`; `onchoose` → `onchooselayer` |
| Strips grid, column *i* + 1 | `FaderStrip` | `{ ...strips[i], index: i + 1, hue: HUE[i], layer: i < 4 ? layer : 'volume', nameAction, tip, nameTip }` by the Strips table; `onchange(v)` → `onfader(i, v)`, `onrelease(v)` → `onfaderrelease(i, v)`, `onopen()` → `onname(i)`; `width` unset (fills the column) |
| Below the strips | `LampRow` | `{ ...lamps }` |

`LAYER_WORD` = `pan` "Pan", `reverb` "Reverb", `chorus` "Chorus", `delay` "Delay" (the header detail; GroupHeader draws "Faders · Reverb" with the detail in `--t`).

### Strips (Panel page; kit › Faders)

FaderBank sets these by the strip's place; `strips[i]` supplies the rest.

| Strip | `hue` (`HUE[i]`) | `layer` given | `nameAction` | `tip` (the fader) | `nameTip` | Launchkey |
|---|---|---|---|---|---|---|
| 1 Right 1 | `r1` | `layer` | `'open Channel'`; rack target `'open the Rack page'` | rack target `launchkey.fader_rack`; else `waiting` `mixer.pickup`; else by layer: `mixer.panel.right1`, `mixer.part.pan`, `mixer.part.reverb`, `mixer.part.chorus`, `mixer.part.variation` | `mixer.strip.select` (rack target `launchkey.fader_rack`) | fader 1 |
| 2 Right 2 | `r2` | `layer` | as strip 1 | as strip 1, volume key `mixer.panel.right2` | as strip 1 | fader 2 |
| 3 Right 3 | `r3` | `layer` | as strip 1 | volume key `mixer.panel.right3` | as strip 1 | fader 3 |
| 4 Left | `l` | `layer` | as strip 1 | volume key `mixer.panel.left` | as strip 1 | fader 4 |
| 5 Style | `a` | `volume` | `'open the Style fader page'` | `waiting` `mixer.pickup`, else `mixer.style_level` | `mixer.style_level` | fader 5 |
| 6 Multi Pad | `t2` | `volume` | `'open Multi Pads'` | `waiting` `mixer.pickup`, else `mixer.pad_level` | `mixer.pad_level` | fader 6 |
| 7, 8 | `t` (unused: drawn `--d` by FaderStrip) | `volume` | `''` | `launchkey.fader_unused` | — | faders 7, 8 (unused on Panel) |
| 9 Master | `t` | `volume` | `'open Effects'` | `waiting` `mixer.pickup`, else `mixer.master` | `mixer.master` | master fader |

Any strip whose `value` is null gets `nameAction: ''` and `tip: 'launchkey.fader_unused'` (no synth makes the master unused).

### Visual rules

The root is a `section` with `aria-label="Faders"`, 654 × 368 (the Full band's Faders width; `flex: none`), a column:

| Part | Box (from the bank's top-left) | Height |
|---|---|---|
| Header (`GroupHeader`) | `0,0 654×36` (its 1px `--line` bottom included) | 36 |
| Strips grid | `0,44 654×272`: 8 below the header; `display: grid; grid-template-columns: repeat(9, minmax(0, 1fr)); gap: 8px` (columns `(654 − 64) / 9 = 65.556` wide, at x = 73.556 × *i*) | 272 |
| `LampRow` | `0,316 654×52`, straight under the strips | 52 |

**Header content** (GroupHeader's snippet), one row after the title, `display: flex; align-items: stretch; gap: 12px` (`--space-12`), no wrap, the same 12px between the title and the first tabs:

1. the page `ChosenTabs` (Panel | Style);
2. a separator: `1 × 16`, `--line`, `align-self: center`, `aria-hidden`;
3. the layer group: `display: flex; align-items: stretch; gap: 4px` (`--space-4`): "Layer" 14 / 400 `--m`, `align-self: center` (plain text, `aria-hidden`, since the tablist is named "Fader layer"), then the layer `ChosenTabs`.

- **Tokens used (own):** `--line`, `--m`, `--text-14`, `--weight-regular`, `--space-4`, `--space-8`, `--space-12`, `--font-sans`. Children use their own.
- **Size:** fixed 654 × 368; no variants (the tall pages' half band is a different layout, #501).
- **States drawn by:** the children (the layer tab, the header detail, each strip's state, the lamps). FaderBank adds none of its own.
- **Type:** "Layer" DM Sans 14 / 400 `--m`, sentence case.
- **Contrast:** `--m` on `--g` (exists). The rest is the children's.
- **Motion:** none.

### Accessibility

- **Role and name:** `section` named "Faders". Inside, in DOM order (which is the tab order, Stage.md D36): the two tablists ("Fader page (master button)", "Fader layer"), then each strip's slider and name button, strip 1 to 9 (unused strips have neither), then LampRow's group "Fader buttons".
- **Keyboard:** the children's (ChosenTabs' arrows inside a tablist, FaderStrip's arrows and Page keys, LampRow's buttons).
- **Tooltip ids:** page tabs `mixer.page`; layer tabs `mixer.layer`; the strips' by the Strips table; LampRow's own. All exist in `app/src/help/tooltips.ts`.
- **Launchkey:** page tabs = the button under the master fader; layer tabs = Shift + that button; strips by the Strips table; the lamp row as LampRow says.

### From the state

The page wiring fills the props from `AppState` and the meter frames, and maps each callback; FaderBank imports none of it.

| Prop / callback | From `AppState`, `meters` / sends |
|---|---|
| `page` | `mixer.faderPage` |
| `layer` | `mixer.faderLayer` |
| `strips[i].value` | `surface.faders[i].value` (null → unused) |
| `strips[i].label`, *i* 0–3 | `keyboardParts[i].name`, or, when `surface.faders[i].set?.type === 'moveRackFader'`, `surface.faders[i].label` as given |
| `strips[i].label`, *i* 4, 5, 8 | `'Style'`, `'Multi Pad'`, `'Master'` (fixed words; the state's labels are capitals) |
| `strips[i].label`, *i* 6, 7 | `''` |
| `strips[i].rackTarget`, *i* 0–3 | `surface.faders[i].set?.type === 'moveRackFader'` |
| `strips[i].off`, *i* 0–3 | `!keyboardParts[i].sounding` |
| `strips[i].edited`, *i* 0–3 | `keyboardParts[i].soundEdited === true` |
| `strips[i].missing`, *i* 0–3 | `keyboardParts[i].plugin?.missing === true` |
| `strips[i].failed`, *i* 0–3 | `keyboardParts[i].plugin?.status === 'failed' && !missing` |
| `strips[i].waiting`, `.position` | `surface.faders[i].waiting`, `surface.faders[i].position` |
| `strips[i].meter.peak`, `.rms`, *i* 0–3 | the `meters.channels` entry whose `channel` is `keyboardParts[i].channel` (no entry: 0, 0) |
| `strips[4].meter.peak`, `.rms` (Style) | the largest `peak` and the largest `rms` over channels 9–16 (Stage.md D7) |
| `strips[5].meter.peak`, `.rms` (Multi Pad) | the same over channels 5–8 |
| `strips[8].meter.peak`, `.rms` (Master) | `max(meters.master[0], meters.master[1])`, `max(meters.masterRms[0], meters.masterRms[1])` |
| `strips[i].meter.hold` | `holdPeak(prevHold[i], peak, meters.atMs).value` (FaderStrip's `meter.ts`), the wiring keeping `prevHold[i]` per strip across frames |
| `strips[i].meter`, *i* 6, 7 | `null` |
| `strips[i].meter`, *i* 0–3 in a layer other than volume or a rack target | `null` (FaderStrip ignores it there anyway) |
| no synth (`io.synth` null) | `meters.channels` empty: every meter reads 0; `surface.faders[8].value` null makes the master unused |
| `lamps` | LampRow's "From the state" table |
| `onchoosepage(page)` | `setFaderPage { page }` |
| `onchooselayer(layer)` | `setFaderLayer { layer }` |
| `onfader(i, v)`, `onfaderrelease(i, v)` | `surface.faders[i].set` with its field set to `v`: `volume` for `setPartVolume`, `setStyleVolume`, `setMultiPadVolume`, `setMasterVolume`, `setStylePartVolume`, `moveRackFader`; `pan` for `setPartPan`; `value` for `setPartSend`, `setStylePartSend`. Nothing when `set` is null. `onfader` at most once per animation frame (the latest), `onfaderrelease` at once. |
| `onname(i)`, *i* 0–3 | rack target: the Rack page (D32 interim: `ui.toggleDrawer('rack')`, opened); else Channel for part *i* (D32 interim: `show(i)`) |
| `onname(4)` | `setFaderPage { page: 'style' }` |
| `onname(5)` | the Multi Pads page (D32 interim: `ui.toggleDrawer('multipad')`, opened) |
| `onname(8)` | Effects at the master (D32 interim: `ui.toggleDrawer('effects')`, opened) |

### Fixture

`app/src/ui/FaderBank/FaderBank.fixtures.ts` exports the Board's data (Stage.md › Board fixture), which FullBand imports:

```ts
/** The nine held peaks of the board, strips 1–9, linear (tick bottoms 161, 128, 5, 139, 179, 32, –, –, 183 px). */
export const boardMeterHolds = [0.1259, 0.0447, 0, 0.0631, 0.2188, 0.0023, 0, 0, 0.2512]

/** FaderBank's props on the Stage board, callbacks left out (stories add actions). */
export const boardFaderBank = {
  page: 'panel',
  layer: 'volume',
  strips: [
    { label: 'Right 1', value: 90, meter: { peak: 0.0724, rms: 0.0537, hold: 0.1259 } },
    { label: 'Right 2', value: 72, edited: true, waiting: true, position: 50, meter: { peak: 0.0224, rms: 0.018, hold: 0.0447 } },
    { label: 'Right 3', value: 64, off: true, missing: true, meter: { peak: 0, rms: 0, hold: 0 } },
    { label: 'Left', value: 80, meter: { peak: 0.0316, rms: 0.0248, hold: 0.0631 } },
    { label: 'Style', value: 100, meter: { peak: 0.1259, rms: 0.0897, hold: 0.2188 } },
    { label: 'Multi Pad', value: 90, meter: { peak: 0, rms: 0, hold: 0.0023 } },
    { label: '', value: null, meter: null },
    { label: '', value: null, meter: null },
    { label: 'Master', value: 100, meter: { peak: 0.1445, rms: 0.102, hold: 0.2512 } },
  ],
  lamps: {
    partOn: [true, true, false, true],
    partSounding: [true, true, false, true],
    swapPart: null,
    soundOn: false,
    harmArp: false,
    leftHold: false,
    looper: 'off',
    page: 'panel',
    shift: false,
    masterDisabled: false,
  },
}
```

Each strip's `hold` is `boardMeterHolds[i]` (written out above so the object reads alone; a test asserts they agree). Right 3 is `missing`, so not `failed` (its plugin status is failed, but missing wins). The fixture is typed against FaderBank's exported props type (`satisfies`), callbacks omitted.

## Stories (Story station)

- **Title:** `Components/FaderBank`.
- **Layout:** `centered` (the bank is a fixed 654 × 368). Every story renders in dark and light.
- Args spread `boardFaderBank` and add actions: `onchoosepage`, `onchooselayer`, `onfader`, `onfaderrelease`, `onname`, and inside `lamps` LampRow's (`onpart`, `onpartchannel`, `onhold`, `onharmarp`, `onlefthold`, `onlooper`, `onlooperrec`, `onmaster`, `onmastershift`), each `fn()`. Controls: FaderBank's own (`page`, `layer`, `strips`, `lamps` as objects), plus, grouped under "FaderStrip 1" (`table.category`), strip 1's `value`, `waiting`, `position`, `off` mapped into `strips[0]`, and under "LampRow" `looper`, `soundOn`, `swapPart` mapped into `lamps`.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `boardFaderBank` | the board's faders: header "Faders", Panel and Vol chosen; Right 1 90, Right 2 72 waiting with the ghost and the edited dot, Right 3 64 off with ⚠, Left 80, Style 100, Multi Pad 90, two unused, Master 100, their meters and ticks; the lamp row | `Board-{dark,light}.png`: Stage 24,432 654×368 | the section "Faders" holds 7 sliders (strips 7, 8 are not sliders; the elements "Fader 7 unused" and "Fader 8 unused" have no `tabindex`); the tab "Panel" and the tab "Volume" have `aria-selected="true"`; the header reads "Faders" without "·"; slider "Right 2 volume" has `aria-valuetext` "Right 2 72, hardware fader away" and its strip has `[data-part=ghost]`; the name button "Right 3, plugin missing: open Channel" exists; the group "Fader buttons" holds 9 buttons |
| `DragsStrip` | `boardFaderBank` | — | — | (Stage.md Check 9) pointerdown on "Right 1 volume" at `clientY` 300, pointermove to 260 → `onfader` called with 0, 113; pointerup → `onfaderrelease` called with 0, 113; ArrowUp on "Master volume" → `onfader(8, 101)` |
| `ChoosesTabs` | `boardFaderBank` | — | — | click the tab "Reverb send" → `onchooselayer('reverb')`; click the tab "Style" → `onchoosepage('style')` |
| `NamesOpen` | `boardFaderBank` | — | — | click "Style: open the Style fader page" → `onname(4)`; click "Right 1: open Channel" → `onname(0)`; click "Master: open Effects" → `onname(8)` |
| `ReverbLayer` | `{ ...boardFaderBank, layer: 'reverb', strips: strips 1–4 with values 40, 12, 0, 20 and `meter: null`, waiting false (the rest as Board) }` | header "Faders · Reverb" with "Reverb" in `--t`, the Reverb tab chosen; strips 1–4 read "Rev 40", "Rev 12", "Rev 0", "Rev 20" in white (Right 3's "Rev 0" grey, off) with white fills and no meters; Style, Multi Pad and Master unchanged | — (the board draws only the Vol layer) | the tab "Reverb send" has `aria-selected="true"`; the header's text is "Faders · Reverb"; the sliders "Right 1 reverb send" … "Left reverb send" exist and their strips have no `[data-part^=meter]`; strip 1's value reads "Rev 40"; slider "Style volume" still has `[data-part=meter-peak]`; strip 1's slider has `data-tip="mixer.part.reverb"`; click the tab "Volume" → `onchooselayer('volume')` |
| `PanLayer` | `{ ...boardFaderBank, layer: 'pan', strips: strips 1–4 values 64, 44, 64, 84, `meter: null`, waiting false }` | "Faders · Pan"; "C", "L20", "C", "R20" with fills from the 64 line | — | strip 2's value reads "L20", strip 4's "R20"; the slider "Right 2 pan" has `aria-valuetext` "Right 2 pan left 20" |
| `StylePage` | `{ ...boardFaderBank, page: 'style', lamps: { ...boardFaderBank.lamps, page: 'style' } }` | the Style page tab chosen and the master button reading "Style"; the strips as given (the Style parts' strips are #507's) | — | the tab "Style" has `aria-selected="true"`; the button "Fader page is Style: click for Panel" exists |
| `RackTarget` | Board with `strips[1] = { label: 'PANR2', value: 64, rackTarget: true, meter: null }` | strip 2 reads "PANR2" in `--t2`, no meter | — | slider "PANR2, rack fader" has `data-tip="launchkey.fader_rack"`; the button "PANR2: open the Rack page" → click → `onname(1)` |
| `NoSynth` | Board with every `meter` 0 (`{ peak: 0, rms: 0, hold: 0 }`, strips 7–8 `null`) and `strips[8] = { label: '', value: null, meter: null }` | the meters empty, the master unused ("—", dashed) | — | the element "Fader 9 unused" exists with role `img`; there are 6 sliders |
| `SwapHeld` | Board with `lamps.swapPart: 0` | Right 1's lamp reads "Swap" | — | the button "Right 1 swap held" reads "Swap" |
| `Focused` | `boardFaderBank`; `parameters: { pseudo: { focusVisible: ['[role=slider]'] } }` | the focus ring on every fader (the pseudo-states addon) | — | — |

Only `Board` has a crop: it is the one state the board draws at this box. The fixture reproduces the board exactly (values, meters, ticks, the ghost, the marks), so no mask is needed.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- A fixture test (`FaderBank.fixtures.test.ts`) asserts each `strips[i].meter?.hold` equals `boardMeterHolds[i]` (0 where the meter is null) and the fixture satisfies FaderBank's props type.
- `npm run shots -- FaderBank` passes: `Board` against `crops/Board-{dark,light}.png` at most 0.02; axe clean on every story (FaderStrip's D3 exemption for `[data-hue="d"]` applies).
- Only listed tokens are used; no inline colours; no literal sizes outside the Visual rules.
- svelte-check and lint pass on the folder.

## Decisions

- D1. The wiring maps the state into each strip's data (values, labels, flags, meters, the group strips' maxima and the held peaks); FaderBank adds only what the strip's place decides (hue, the layer it gets, the name's action, the tooltip keys), so FaderBank stays pure and its tables are the kit's Panel page.
- D2. The group strips' meters are the largest peak and the largest RMS of their channels, computed by the wiring (Stage.md D7); FaderStrip only draws.
- D3. Strip labels 5, 6 and 9 are the fixed words "Style", "Multi Pad", "Master", not the state's capitals (`STYLE`); strips 1–4 use `keyboardParts[i].name`, and a rack target's label exactly as the state gives it ("PANR2").
- D4. A strip is a rack target when its `set` is `moveRackFader`, which the API defines exactly, rather than by comparing labels (the API's labels are capitals, Stage.md's fixture writes "Right 1").
- D5. On the Style fader page FaderBank lays out whatever strips it is given with the Panel page's hues and keys until #507 (PadsPage2) specs the Style parts' strips and lamps; the page tab and master button already follow `page`.
- D6. "Layer" is `aria-hidden` text beside the tablist named "Fader layer", so it isn't read twice.
- D7. The header's "· Reverb" names the layer by its full word ("Pan", "Reverb", "Chorus", "Delay"), as the layer tabs do, in `--t`, with " · " in `--m`.
- D8. The fixture writes each strip's `hold` out in full rather than reading `boardMeterHolds`, so the fixture reads alone, and a test keeps the two in step.
- D9. The bank is a fixed 654 × 368 (the Full band's Faders section), since the tall pages' half band is its own layout (#501).

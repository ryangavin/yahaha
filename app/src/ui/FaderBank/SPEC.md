# FaderBank

## Identity (all stations)

- **Kind:** complex
- **Built from:** GroupHeader, ChosenTabs, FaderStrip, LampRow
- **Purpose:** The band's faders, as on the Launchkey: nine strips (the four keyboard parts, the Style, the Multi Pads, two unused, the master) with their levels and meters, the fader page and layer they control, and the buttons under them.
- **Boards:** `Stage-Dark.dc.html:223-288` (header `:224-240`, strips grid `:243-268`, lamp row `:271-286`; the strips' data script `:425-473`); light: `Stage-Light.dc.html:199-264` (data `:397-445`). Box `Stage 24,432 654×368`: header `24,432 654×36`, strips `24,476 654×272`, LampRow `24,748 654×52`.
- **Not this component's job:** no store, no API, no Tauri, no timer. It doesn't read `AppState`, compute meters or held peaks, coalesce sends, or decide what a strip's name opens: the page wiring does (see "From the state"). It doesn't restate FaderStrip's, LampRow's, ChosenTabs' or GroupHeader's drawing; it lays them out and passes their props. It never imports `use:tip`: the wiring passes it as `tipAction` (L3). The Style fader page's own design (the Style parts' strips and mutes) is #507's; until then FaderBank draws Stage.md's fallback (D5).

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component. `FaderBankProps` and `StripData` are exported from `app/src/ui/FaderBank/types.ts`.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `page` | `'panel' \| 'style'` | `'panel'` | The fader page: the chosen page tab, and which Strips table applies. |
| `layer` | `'volume' \| 'pan' \| 'reverb' \| 'chorus' \| 'delay'` | `'volume'` | The fader layer: the chosen layer tab, the header's "· {Layer}", and the layer strips 1–4 (Panel) or 1–8 (Style) get. |
| `strips` | `StripData[]` (exactly 9) | — | The nine strips' data, in order (below). |
| `lamps` | `Omit<LampRowProps, 'width' \| 'tipAction'>` | — | Everything LampRow takes, callbacks included, spread onto it unchanged (`width` left unset so it fills the bank; `tipAction` is FaderBank's own). |
| `tipAction` | `Action<HTMLElement, string> \| undefined` | — | The app's `use:tip`, passed in by the wiring (L3) and handed to both ChosenTabs, every FaderStrip and LampRow. Stories pass `fn()`. |

`StripData` is FaderStrip's data props, the ones the state decides: `{ label: string; value: number \| null; meter: { peak: number; rms: number; hold: number } \| null; off?: boolean; rackTarget?: boolean; waiting?: boolean; position?: number \| null; edited?: boolean; missing?: boolean; failed?: boolean }` (defaults as FaderStrip's). FaderBank adds the rest by the strip's place and the page (Strips, below).

### Events

| Callback | Fires when | Payload | The wiring sends |
|---|---|---|---|
| `onchoosepage` | a page tab is chosen (ChosenTabs' `onchoose`) | `(page: 'panel' \| 'style')` | `setFaderPage { page }` |
| `onchooselayer` | a layer tab is chosen | `(layer: 'volume' \| 'pan' \| 'reverb' \| 'chorus' \| 'delay')` | `setFaderLayer { layer }` |
| `onfader` | strip *i*'s FaderStrip `onchange` | `(i: number /* 0–8 */, value: number)` | `surface.faders[i].set` with its value field filled (below), at most once per animation frame, the frame's last value |
| `onfaderrelease` | strip *i*'s FaderStrip `onrelease` | `(i: number, value: number)` | the same command with `value`, at once (flushes the frame) |
| `onname` | strip *i*'s FaderStrip `onopen` | `(i: number)` | by strip (From the state; Stage.md D32 interims) |

Every callback is optional; one that isn't passed is not called. LampRow's callbacks are in `lamps` and pass through untouched.

### Children

| Where | Child | Props |
|---|---|---|
| Header | `GroupHeader` | `{ title: 'Faders', detail: layer === 'volume' ? undefined : LAYER_WORD[layer] }` (`level` and `width` at their defaults: an `h2` filling the bank's 654), its `children` snippet holding the header content below |
| Header, after the title | `ChosenTabs` (pages) | `{ tabs: [{ id: 'panel', label: 'Panel', tip: 'mixer.page' }, { id: 'style', label: 'Style', tip: 'mixer.page' }], chosen: page, size: 'header', label: 'Fader page (master button)', tipAction }`; `onchoose` → `onchoosepage` |
| Header, after "Layer" | `ChosenTabs` (layers) | `{ tabs: [{ id: 'volume', label: 'Vol', name: 'Volume' }, { id: 'pan', label: 'Pan', name: 'Pan' }, { id: 'reverb', label: 'Reverb', name: 'Reverb send' }, { id: 'chorus', label: 'Chorus', name: 'Chorus send' }, { id: 'delay', label: 'Delay', name: 'Delay send' }]` (each with `tip: 'mixer.layer'`), `chosen: layer, size: 'header', label: 'Fader layer', tipAction }`; `onchoose` → `onchooselayer` |
| Strips grid, column *i* + 1 | `FaderStrip` | `{ ...strips[i], index: i + 1, hue, layer, nameAction, tip, nameTip }` by the Strips tables, plus `tipAction`; `onchange(v)` → `onfader(i, v)`, `onrelease(v)` → `onfaderrelease(i, v)`, `onopen()` → `onname(i)`; `width` unset (fills the column) |
| Below the strips | `LampRow` | `{ ...lamps, tipAction }` |

`LAYER_WORD` = `pan` "Pan", `reverb` "Reverb", `chorus` "Chorus", `delay` "Delay" (the header detail; GroupHeader draws "Faders · Reverb" with the detail in `--t`).

### Strips, Panel page (kit › Faders)

FaderBank sets these by the strip's place; `strips[i]` supplies the rest. "By layer" is the fader `tip` for `layer` `volume`, `pan`, `reverb`, `chorus`, `delay`.

| Strip | `hue` | `layer` given | `nameAction` | `tip` (the fader) | `nameTip` | Launchkey |
|---|---|---|---|---|---|---|
| 1 Right 1 | `r1` | `layer` | `'open Channel'`; rack target `'open the Rack page'` | rack target `launchkey.fader_rack`; else by layer `mixer.panel.right1`, `mixer.part.pan`, `mixer.part.reverb`, `mixer.part.chorus`, `mixer.part.variation` | `mixer.strip.select` (rack target `launchkey.fader_rack`) | fader 1 |
| 2 Right 2 | `r2` | `layer` | as strip 1 | as strip 1, volume key `mixer.panel.right2` | as strip 1 | fader 2 |
| 3 Right 3 | `r3` | `layer` | as strip 1 | as strip 1, volume key `mixer.panel.right3` | as strip 1 | fader 3 |
| 4 Left | `l` | `layer` | as strip 1 | as strip 1, volume key `mixer.panel.left` | as strip 1 | fader 4 |
| 5 Style | `a` | `volume` | `'show the Style faders'` | `mixer.style_level` | `mixer.style_level` | fader 5 |
| 6 Multi Pad | `t2` | `volume` | `'open Multi Pads'` | `mixer.pad_level` | `mixer.pad_level` | fader 6 |
| 7, 8 | `t` (unused: drawn `--d` by FaderStrip) | `volume` | `''` | `launchkey.fader_unused` | — | faders 7, 8 (unused on Panel) |
| 9 Master | `t` | `volume` | `'open Effects'` | `mixer.master` | `mixer.master` | master fader |

While a strip is `waiting` its fader keeps the key above; FaderStrip puts `mixer.pickup` on its "↕" itself (FaderStrip D23).

### Strips, Style page (fallback until #507, Stage.md › Band)

| Strip | `hue` | `layer` given | `nameAction` | `tip` (the fader) | `nameTip` | Launchkey |
|---|---|---|---|---|---|---|
| 1–8 (the Style parts, labelled by the state: "RHYTHM 1"…) | `a` | `layer` | `''` (the name is plain text) | by layer `mixer.style.volume`, — (Pan: the engine sends them unused), `mixer.style.reverb`, `mixer.style.chorus`, `mixer.style.variation` | — | faders 1–8 |
| 9 Master | as on Panel | | | | | master fader |

**Any page:** a strip whose `value` is null gets `nameAction: ''`, `tip: 'launchkey.fader_unused'` and no `nameTip` (no synth makes the master unused).

### Visual rules

The root is a `section` with `aria-label="Faders"`, 654 × 368 (the Full band's Faders width; `flex: none`), a column:

| Part | Box (from the bank's top-left) | Height |
|---|---|---|
| Header (`GroupHeader`) | `0,0 654×36` (its 1px `--line` bottom included) | 36 |
| Strips grid | `0,44 654×272`: `margin-top: var(--space-8)` below the header; `display: grid; grid-template-columns: repeat(9, minmax(0, 1fr)); gap: var(--space-8)` (columns `(654 − 64) / 9 = 65.556` wide, at x = 73.556 × *i*) | 272 |
| `LampRow` | `0,316 654×52`, straight under the strips | 52 |

**Component geometry** (axiom 2, declared once on the root): `--bank-width: 654px`, `--bank-height: 368px`, `--separator-height: 16px`. No other literal sizes.

**Header content** (GroupHeader's snippet), each a flex item after the title (GroupHeader's row has `gap: var(--space-12)`), in order:

1. the page `ChosenTabs` (Panel | Style);
2. a separator `span`: `width: var(--line-width); height: var(--separator-height)`, `--line` fill, `align-self: center`, `aria-hidden`;
3. the layer group, a `div`: `display: flex; align-items: stretch; gap: var(--space-4)`: "Layer" `--text-14` / `--weight-regular` `--m`, `align-self: center` (a `span`, `aria-hidden`, since the tablist is named "Fader layer"), then the layer `ChosenTabs`.

- **Tokens used (own):** `--line`, `--line-width`, `--m`, `--text-14`, `--weight-regular`, `--space-4`, `--space-8`, `--font-sans`. Children use their own. No new tokens.
- **Size:** fixed `--bank-width` × `--bank-height` (654 × 368); no size variants here (the tall pages' half band is added later, by another spec change).
- **States drawn by:** the children (the chosen tabs, the header detail, each strip's state, the lamps). FaderBank adds none of its own.
- **Type:** "Layer" DM Sans 14 / 400 `--m`, sentence case.
- **Contrast:** `--m` on `--g` (exists). The rest is the children's; no exemption of FaderBank's own (FaderStrip's dimmed text carries `data-contrast="dim"`, Stage.md D47).
- **Motion:** none.

### Accessibility

- **Role and name:** `section` named "Faders". Inside, in DOM order (which is the tab order, Stage.md D36): the two tablists ("Fader page (master button)", "Fader layer"), then each strip's slider and name button, strip 1 to 9 (unused strips have neither; the Style page's strips 1–8 have no name button), then LampRow's group "Fader buttons".
- **Keyboard:** the children's (ChosenTabs' arrows inside a tablist, FaderStrip's arrows, Page keys, Home and End, LampRow's buttons).
- **Tooltip ids:** page tabs `mixer.page`; layer tabs `mixer.layer`; the strips' by the Strips tables; LampRow's own. All exist in `app/src/help/tooltips.ts`.
- **Launchkey:** page tabs = the button under the master fader; layer tabs = Shift + that button; strips by the Strips tables; the lamp row as LampRow says.

### From the state

The page wiring fills the props from `AppState` and the meter frames, and maps each callback; FaderBank imports none of it. `PART_LABELS` = `['RIGHT 1', 'RIGHT 2', 'RIGHT 3', 'LEFT']` (the engine's own labels, `crates/yahaha-engine/src/launchkey.rs`).

| Prop / callback | From `AppState`, `meters` / sends |
|---|---|
| `page` | `mixer.faderPage` |
| `layer` | `mixer.faderLayer` |
| `strips[i].value` | `surface.faders[i].set === null ? null : surface.faders[i].value` (no `set`: unused, kit › FaderStrip) |
| `strips[i].rackTarget`, Panel, *i* 0–3 | `surface.faders[i].label !== PART_LABELS[i]` (Stage.md D63); false elsewhere |
| `strips[i].label`, Panel, *i* 0–3 | rack target: `surface.faders[i].label` as given ("PANR2"); else `keyboardParts[i].name` ("Right 1") |
| `strips[i].label`, Panel, *i* 4, 5, 8 | `'Style'`, `'Multi Pad'`, `'Master'` (fixed words; the state's are capitals) |
| `strips[i].label`, Style page, *i* 0–7 | `surface.faders[i].label` as given ("RHYTHM 1") |
| `strips[i].label`, *i* 6, 7 on Panel | `''` |
| `strips[i].off`, `.edited`, `.missing`, `.failed`, Panel, *i* 0–3, not a rack target | `!keyboardParts[i].sounding`; `keyboardParts[i].soundEdited === true`; `keyboardParts[i].plugin?.missing === true`; `keyboardParts[i].plugin?.status === 'failed'` (raw; FaderStrip drops the ✕ under ⚠); all false elsewhere (the Style page's strips have no marks) |
| `strips[i].waiting`, `.position` | `surface.faders[i].waiting`, `surface.faders[i].position` |
| `strips[i].meter.peak`, `.rms`, Panel, *i* 0–3 | the `meters.channels` entry whose `channel` is `keyboardParts[i].channel` (no entry: 0, 0) |
| `strips[4].meter.peak`, `.rms` (Panel's Style) | the largest `peak` and the largest `rms` over channels 9–16 (Stage.md D7) |
| `strips[5].meter.peak`, `.rms` (Panel's Multi Pad) | the same over channels 5–8 |
| `strips[8].meter.peak`, `.rms` (Master, either page) | `max(meters.master[0], meters.master[1])`, `max(meters.masterRms[0], meters.masterRms[1])` |
| `strips[i].meter.hold` | `holdPeak(prevHold[i], peak, meters.atMs).value` (FaderStrip's `meter.ts`), the wiring keeping `prevHold[i]` per strip across frames |
| `strips[i].meter` null | Panel *i* 6, 7; a rack target; Panel *i* 0–3 in a layer other than volume; the Style page's *i* 0–7 |
| no synth (`io.synth` null) | `meters.channels` empty: every meter reads 0; `surface.faders[8].set` null makes the master unused |
| `lamps` | LampRow's "From the state" table |
| `tipAction` | the app's `tip` action (`app/src/lib`) |
| `onchoosepage(page)` | `setFaderPage { page }` |
| `onchooselayer(layer)` | `setFaderLayer { layer }` |
| `onfader(i, v)`, `onfaderrelease(i, v)` | `surface.faders[i].set` with its field set to `v`: `volume` for `setPartVolume`, `setStyleVolume`, `setMultiPadVolume`, `setMasterVolume`, `setStylePartVolume`, `moveRackFader`; `pan` for `setPartPan`; `value` for `setPartSend`, `setStylePartSend`. Nothing when `set` is null. `onfader` at most once per animation frame (the latest), `onfaderrelease` at once. |
| `onname(i)`, Panel *i* 0–3 | rack target: the Rack page (D32 interim: `ui.toggleDrawer('rack')`, opened); else Channel for part *i* (D32 interim: `show(i)` in `panels/channel/nav.svelte.ts`) |
| `onname(4)`, Panel | `setFaderPage { page: 'style' }` |
| `onname(5)`, Panel | the Multi Pads page (D32 interim: `ui.toggleDrawer('multipad')`, opened) |
| `onname(8)` | Effects at the master (D32 interim: `ui.toggleDrawer('effects')`, opened) |

### Fixture

`app/src/ui/FaderBank/FaderBank.fixtures.ts` exports the Board's data (Stage.md › Board fixture), which FullBand imports:

```ts
/** The nine held peaks of the board, strips 1–9, linear (tick bottoms 161, 128, 5, 139, 179, 32, –, –, 183 px). */
export const boardMeterHolds = [0.1259, 0.0447, 0, 0.0631, 0.2188, 0.0023, 0, 0, 0.2512]

/** FaderBank's props on the Stage board, callbacks and tipAction left out (stories add actions). */
export const boardFaderBank = {
  page: 'panel',
  layer: 'volume',
  strips: [
    { label: 'Right 1', value: 90, meter: { peak: 0.0724, rms: 0.0537, hold: 0.1259 } },
    { label: 'Right 2', value: 72, edited: true, waiting: true, position: 50, meter: { peak: 0.0224, rms: 0.018, hold: 0.0447 } },
    { label: 'Right 3', value: 64, off: true, missing: true, failed: true, meter: { peak: 0, rms: 0, hold: 0 } },
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
    styleButtons: null,
  },
} satisfies Omit<FaderBankProps, 'onchoosepage' | 'onchooselayer' | 'onfader' | 'onfaderrelease' | 'onname' | 'tipAction'>

/** The Style fader page's fallback (Stage.md › Band): eight Style parts from the dev mock's labels, Master as on Panel. */
export const stylePageFaderBank = {
  ...boardFaderBank,
  page: 'style',
  strips: [
    { label: 'RHYTHM 1', value: 100, meter: null }, { label: 'RHYTHM 2', value: 100, meter: null },
    { label: 'BASS', value: 100, meter: null }, { label: 'CHORD 1', value: 100, meter: null },
    { label: 'CHORD 2', value: 100, meter: null }, { label: 'PAD', value: 100, meter: null },
    { label: 'PHRASE 1', value: 100, meter: null }, { label: 'PHRASE 2', value: 100, meter: null },
    boardFaderBank.strips[8],
  ],
  lamps: {
    ...boardFaderBank.lamps,
    page: 'style',
    styleButtons: [
      { label: 'RHY1', on: true, disabled: false }, { label: 'RHY2', on: true, disabled: false },
      { label: 'BASS', on: true, disabled: false }, { label: 'CHD1', on: true, disabled: false },
      { label: 'CHD2', on: false, disabled: false }, { label: '', on: false, disabled: true },
      { label: 'PHR1', on: true, disabled: false }, { label: 'PHR2', on: true, disabled: false },
    ],
  },
} satisfies Omit<FaderBankProps, 'onchoosepage' | 'onchooselayer' | 'onfader' | 'onfaderrelease' | 'onname' | 'tipAction'>
```

Each strip's `hold` is `boardMeterHolds[i]` (written out above so the object reads alone; a test asserts they agree). Right 3 carries both `missing` and `failed` raw (its plugin status is failed and it is missing); FaderStrip and PartMarks draw only the ⚠ (PartMarks D1). The Style page's labels and lamp labels are placeholders in the dev mock's form (no board draws that page, D10); entry 6 of `styleButtons` is ignored (cell 6 stays Sound).

## Stories (Story station)

- **Title:** `Components/FaderBank`.
- **Layout:** `centered` (the bank is a fixed 654 × 368). Every story renders in dark and light.
- The meta's `args` spread `boardFaderBank` and add actions: `onchoosepage`, `onchooselayer`, `onfader`, `onfaderrelease`, `onname`, `tipAction`, and inside `lamps` LampRow's (`onpart`, `onpartchannel`, `onhold`, `onharmarp`, `onlefthold`, `onlooper`, `onlooperrec`, `onmaster`, `onmastershift`, `onstylebutton`), each `fn()`. Controls: FaderBank's own (`page` and `layer` selects; `strips` and `lamps` objects), plus, grouped under "FaderStrip 1" (`table.category`), strip 1's `value`, `waiting`, `position`, `off` mapped into `strips[0]`, and under "LampRow" `looper`, `soundOn`, `swapPart` mapped into `lamps`.
- No `a11y` exclude: FaderStrip's dimmed text carries `data-contrast="dim"` (Stage.md D47); until Stage.md C6 lands, the light `--r3` / `--l` texts are the only `color-contrast` violations (FaderStrip D21).
- Pointer plays (L4): `fireEvent.pointerDown` / `pointerMove` / `pointerUp(slider, { pointerId: 1, button: 0, clientX: 0, clientY })`.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `boardFaderBank` | the board's faders: header "Faders", Panel and Vol chosen; Right 1 90, Right 2 72 waiting with the ghost and the edited dot, Right 3 64 off with ⚠, Left 80, Style 100, Multi Pad 90, two unused, Master 100, their meters and ticks; the lamp row | `Board-{dark,light}.png`: Stage 24,432 654×368 | the section "Faders" holds 7 sliders (strips 7, 8 are not sliders; the elements "Fader 7 unused" and "Fader 8 unused" have role `group` and no `tabindex`); the tab "Panel" and the tab "Volume" have `aria-selected="true"`, the tab "Panel" has `data-tip="mixer.page"` and "Volume" `data-tip="mixer.layer"`; the heading reads "Faders" without "·"; slider "Right 2 volume" has `aria-valuetext` "Right 2 72, hardware fader away" and its strip has `[data-part=ghost]`; the name button "Right 3, plugin missing: open Channel" exists; the group "Fader buttons" holds 9 buttons; `tipAction` was called with the slider "Right 1 volume" and `'mixer.panel.right1'` |
| `DragsStrip` | `boardFaderBank` | — | — | (Stage.md Check 9) pointerDown on "Right 1 volume" at `clientY` 300, pointerMove to 260 → `onfader` called with 0, 113; pointerUp → `onfaderrelease` called with 0, 113; ArrowUp on "Master volume" → `onfader(8, 101)` |
| `ChoosesTabs` | `boardFaderBank` | — | — | click the tab "Reverb send" → `onchooselayer('reverb')`; click the tab "Style" → `onchoosepage('style')` |
| `NamesOpen` | `boardFaderBank` | — | — | click "Style: show the Style faders" → `onname(4)`; click "Right 1: open Channel" → `onname(0)`; click "Master: open Effects" → `onname(8)` |
| `ReverbLayer` | `{ ...boardFaderBank, layer: 'reverb', strips }` with strips 1–4 `{ ...board strip, value: 40 / 12 / 0 / 20, meter: null, waiting: false, position: null }` and strips 5–9 as Board | header "Faders · Reverb" with "Reverb" in `--t`, the Reverb tab chosen; strips 1–4 read "Rev 40", "Rev 12", "Rev 0", "Rev 20" in white (Right 3's "Rev 0" grey, off) with white fills and no meters; Style, Multi Pad and Master unchanged | — (the Stage board draws only Vol; Effects-Dark's header crop is GroupHeader's) | the tab "Reverb send" has `aria-selected="true"`; the heading's `textContent` is "Faders · Reverb"; the sliders "Right 1 reverb send" … "Left reverb send" exist and their strips have no `[data-part^=meter]`; strip 1's value reads "Rev 40"; slider "Style volume" still has `[data-part=meter-peak]`; slider "Right 1 reverb send" has `data-tip="mixer.part.reverb"`; click the tab "Volume" → `onchooselayer('volume')` |
| `PanLayer` | `{ ...boardFaderBank, layer: 'pan', strips }` with strips 1–4 values 64, 44, 64, 84, `meter: null`, `waiting: false`, `position: null` | "Faders · Pan"; "C", "L20", "C", "R20" with fills from the 64 line | — | strip 2's value reads "L20", strip 4's "R20"; the slider "Right 2 pan" has `aria-valuetext` "Right 2 pan left 20" |
| `StylePage` | `stylePageFaderBank` | the Style page tab chosen; eight violet strips named by the state ("RHYTHM 1" …) with no meters and plain-text names; Master as on Panel; the lamp row's buttons 1–5, 7, 8 showing the Style parts' labels, Sound in place, the master button reading "Style" | — (no board draws the Style page, D10) | the tab "Style" has `aria-selected="true"`; the slider "RHYTHM 1 volume" has `data-tip="mixer.style.volume"` and no `[data-part^=meter]` in its strip; strips 1–8's `[data-part=name]` are `span`s and the only strip name button is "Master: open Effects"; the button "Fader page is Style: click for Panel" exists; the button "RHY1" exists |
| `StylePageReverb` | `{ ...stylePageFaderBank, layer: 'reverb' }` | the Style parts in the reverb layer | — | the slider "RHYTHM 1 reverb send" has `data-tip="mixer.style.reverb"` |
| `RackTarget` | `boardFaderBank` with `strips[1] = { label: 'PANR2', value: 64, rackTarget: true, meter: null }` | strip 2 reads "PANR2" in `--t2`, no meter | — | slider "PANR2, rack fader" has `data-tip="launchkey.fader_rack"`; click the button "PANR2: open the Rack page" → `onname(1)` |
| `NoSynth` | `boardFaderBank` with every non-null `meter` `{ peak: 0, rms: 0, hold: 0 }` and `strips[8] = { label: '', value: null, meter: null }` | the meters empty, the master unused ("—", dashed) | — | the element "Fader 9 unused" exists with role `group`; there are 6 sliders |
| `SwapHeld` | `boardFaderBank` with `lamps: { ...boardFaderBank.lamps, swapPart: 0 }` | Right 1's lamp reads "Swap" | — | the button "Right 1, swap held" reads "Swap" |
| `Focused` | `boardFaderBank`; `parameters: { pseudo: { focusVisible: ['[role=slider]'] } }` | the focus ring on every fader (the pseudo-states addon) | — | — |

Only `Board` has a crop: it is the one state the board draws at this box. The fixture reproduces the board exactly (values, meters, ticks, the ghost, the marks), so no mask is needed.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- A fixture test (`FaderBank.fixtures.test.ts`) asserts each `boardFaderBank.strips[i].meter?.hold` equals `boardMeterHolds[i]` (0 where the meter is null); both fixtures satisfy the props type (`satisfies`, checked by svelte-check).
- `npm run shots -- FaderBank` passes: `Board` against `crops/Board-{dark,light}.png` at most 0.02; axe clean on every story (D47's dimmed text skipped by the shots tool; before C6, only the light `--r3` / `--l` texts, named as C6).
- Only listed tokens and the root's custom properties are used; no inline colours; no literal sizes outside the Visual rules.
- svelte-check and lint pass on the folder.

## Decisions

- D1. The wiring maps the state into each strip's data (values, labels, flags, meters, the group strips' maxima and the held peaks); FaderBank adds only what the strip's place and the page decide (hue, the layer it gets, the name's action, the tooltip keys), so FaderBank stays pure and its tables are the kit's Panel page and Stage.md's Style fallback.
- D2. The group strips' meters are the largest peak and the largest RMS of their channels, computed by the wiring (Stage.md D7); FaderStrip only draws.
- D3. Panel strip labels 5, 6 and 9 are the fixed words "Style", "Multi Pad", "Master", not the state's capitals (`STYLE`, `M.PAD`); strips 1–4 use `keyboardParts[i].name`, and a rack target's label exactly as the state gives it ("PANR2").
- D4. A Panel fader 1–4 is a rack target when its `surface.faders[i].label` differs from the engine's `PART_LABELS[i]` (Stage.md D63, kit › FaderStrip), not when its `set` is `moveRackFader` (replaces the earlier rule; the two agree today, D63 is the screen spec's).
- D5. On the Style fader page FaderBank draws Stage.md › Band's fallback until #507: strips 1–8 the Style parts by the state's labels, `--a`, no meter or marks, plain-text names, the `mixer.style.*` keys by layer; strip 9 Master; the lamp row's own fallback is LampRow's (replaces the earlier "Panel hues and keys").
- D6. "Layer" is `aria-hidden` text beside the tablist named "Fader layer", so it isn't read twice.
- D7. The header's "· Reverb" names the layer by its full word ("Pan", "Reverb", "Chorus", "Delay"), as the layer tabs do, in `--t`, with " · " in `--m` (GroupHeader's `detail`).
- D8. The fixture writes each strip's `hold` out in full rather than reading `boardMeterHolds`, so the fixture reads alone, and a test keeps the two in step.
- D9. The bank is a fixed 654 × 368 (the Full band's Faders section); its half-height variant is added to this spec later, not here.
- D10. The Style page's stories use placeholder labels in the dev mock's form ("RHYTHM 1", "RHY1"), since no board draws the page; they have no crop.
- D11. L3: each ChosenTabs item carries its own `tip` (`mixer.page`, `mixer.layer`), as ChosenTabs' `TabItem` takes it, and FaderBank's one `tipAction` goes to both ChosenTabs, every FaderStrip and LampRow (replaces the earlier run-level `tip`).
- D12. A strip is unused when its `set` is null (kit › FaderStrip), so the wiring passes `value: null` then whatever `value` the state carries.
- D13. On the Style page a strip in a send layer draws FaderStrip's layer look (the value word and fill in `--t`, the name in `--a`), so the value reads "Rev 40" as on Panel; its `aria-valuetext` is FaderStrip's ("RHYTHM 1 reverb 40"), Stage.md's "{label} {value}" in the volume layer.
- D14. Right 3's fixture strip passes `missing` and `failed` raw, as FaderStrip's props take them (PartMarks applies ⚠ over ✕), replacing the earlier `failed: false` pre-filter.

Follow-ups: the tokens contract PR's contrast rows and C6 (FaderStrip D21); #507 replaces the Style page fallback (D5).

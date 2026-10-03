# KnobBank

## Identity (all stations)

- **Kind:** complex
- **Built from:** GroupHeader, AccentBlock, Button, Knob
- **Purpose:** The band's eight knobs with their Knob Assign page: which page is up, stepping it, and turning each knob.
- **Boards:**
  - `Stage-Dark.dc.html:291-317` (header `:292-296`, ▲ ▼ `:298-301`, knob grid `:302-315`); light: `Stage-Light.dc.html:267-293`.
  - Crop: the bank is whole pixels at `690,432 626×140` on `docs/design/push/png/Stage-Dark.png` and `Stage-Light.png`.
- **Not this component's job:** no store, no API, no Tauri. It doesn't send `stepKnobPage`, `turnKnob` or `resetKnob`: it calls `onpage`, `onturn`, `onreset` and the page wiring sends (and sums one animation frame's turns per knob). It doesn't restate how a Knob, a Button, the AccentBlock or the GroupHeader draws; it places them and passes their props. No timer, no tooltip wiring (integration adds `data-tip`).

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component. `KnobData` is declared in `KnobBank.svelte`'s module script (or `knob.ts`) as `{ function: string, name: string, short: string, value: string, level: number | null }`, the shape of `knobs.knobs[i]`.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `knobs` | `KnobData[]` (8) | — | The eight knobs, knob 1 first. Fewer than eight leaves the rest of the grid empty; more are not drawn. |
| `pageName` | `string` | — | The accent block's word: "Style", "Rack", "Pan", "Reverb", "Chorus", "Delay", or in swap mode "Swap R1" … "Swap L", as given. |
| `pageNumber` | `number` | `1` | 1-based page number; shown, and ▲ is disabled at 1 or below. |
| `pageCount` | `number` | `6` | Pages in all; shown, and ▼ is disabled when `pageNumber >= pageCount`. |
| `swapPart` | `0 \| 1 \| 2 \| 3 \| null` | `null` | The keyboard part whose swap is held, or null. Not null: the accent block takes the part's hue (0 `r1`, 1 `r2`, 2 `r3`, 3 `l`). |
| `tempo` | `number \| null` | `null` | Passed to each Knob as `tempo` (the tempo knob's arc). |
| `shift` | `boolean` | `false` | Passed to each Knob as `shift` (fine drag). |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onpage` | ▲ pressed (`-1`) or ▼ pressed (`1`) (Button `onpress`: click, Space or Enter), unless that button is disabled | `(delta: -1 \| 1)` |
| `onturn` | Knob i calls its `onturn(delta)` | `(knob: number, delta: number)`: `knob` is i (0–7) |
| `onreset` | Knob i calls its `onreset()` | `(knob: number)` |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### From the state

What the page wiring (`app/src/pages/StageWiring.svelte`, through FullBand's `knobs` prop) transcribes.

| Prop / callback | From `AppState` / sends |
|---|---|
| `knobs` | `knobs.knobs` |
| `pageName` | `knobs.pageName` |
| `pageNumber`, `pageCount` | `knobs.pageNumber`, `knobs.pageCount` |
| `swapPart` | `surface.layer.part` when `surface.layer.type == 'swap'`, else null |
| `tempo` | `transport.tempo` |
| `shift` | `ui.shift` |
| `onpage(delta)` | `stepKnobPage { delta }` |
| `onturn(knob, delta)` | `turnKnob { knob, delta }`, one per animation frame per knob with that frame's deltas summed (the wiring's); in swap mode the session runs it as `turnSwapKnob` |
| `onreset(knob)` | `resetKnob { knob }` |

### Children

| Where | Child | Props passed |
|---|---|---|
| Header row | `GroupHeader` | `{ title: 'Knobs' }`, with the two items below in its snippet (after the title) |
| Header, after the title | `AccentBlock` | `{ label: pageName, as: 'span', hue: swapPart === null ? undefined : ['r1', 'r2', 'r3', 'l'][swapPart] }` |
| Header, right end | the page readout (KnobBank's own span) | — (see Visual rules) |
| Page column, top | `Button` | `{ label: '▲', variant: 'icon', name: 'Knob page up', disabled: pageNumber <= 1, onpress: () => onpage(-1) }` |
| Page column, bottom | `Button` | `{ label: '▼', variant: 'icon', name: 'Knob page down', disabled: pageNumber >= pageCount, onpress: () => onpage(1) }` |
| Knob grid, cell i (0–7) | `Knob` | `{ index: i, function: knobs[i].function, name: knobs[i].name, short: knobs[i].short, value: knobs[i].value, level: knobs[i].level, tempo, shift, onturn: (d) => onturn(i, d), onreset: () => onreset(i) }` |

### Visual rules

- **Tokens used:** `--m`, `--t`, `--font-sans`, `--text-13`, `--weight-regular`, `--space-6`, `--space-8`. (The children bring their own.)
- **Size:** fixed 626 × 140 (the band is laid out at 1440 × 900 and the shell scales it, Stage.md D1). A flex column, no padding, no background, no border:
  1. **Header**, 36 tall: the `GroupHeader` row (its hairline included; the hairline is the row's bottom pixel). After the title, gap 12 (the GroupHeader's), the `AccentBlock`. At the right end, `margin-left: auto`, the page readout: one span, 13px (`--text-13`) regular, `--m`, reading "Page " then a span in `--t` with `{pageNumber}/{pageCount}` ("Page 1/6"). Its right edge is the bank's right edge.
  2. **Row**, `margin-top: 8px` (`--space-8`), 96 tall, `display: flex`, gap 8 (`--space-8`):
     - the **page column**: 32 wide, a flex column, `justify-content: center`, gap 6 (`--space-6`): ▲ then ▼, each the 32 × 32 `Button`. At 1440 that puts ▲ at `690,489` and ▼ at `690,527` (13px above ▲ and below ▼ inside the 96px row).
     - the **knob grid**: `flex: 1` (586 wide), `display: grid; grid-template-columns: repeat(8, minmax(0, 1fr)); column-gap: 6px` (`--space-6`), 96 tall; each cell holds one 68 × 96 `Knob` (586 − 7 × 6 = 544, / 8 = 68). Knob i sits at `x = 730 + 74·i`, `y = 476`.
- **States drawn by:**
  - normal: the accent block in `--a` (AccentBlock's default).
  - swap mode (`swapPart` not null): the accent block in the part's hue (`hue` `r1` / `r2` / `r3` / `l`); `pageName` reads "Swap R1" etc. as the state gives it; the page readout keeps the page the knobs return to (the state's `pageNumber`).
  - first page: ▲ disabled (Button's disabled face). Last page: ▼ disabled. One page only (`pageCount` 1): both disabled.
  - keyboard focus: each child shows its own ring.
- **Type:** DM Sans, `font-variant-numeric: tabular-nums`. The page readout as above; nothing else is KnobBank's own text.
- **Contrast:** `--m` on `--g` and `--t` on `--g` (page readout) are already in `tokens/contrast.test.ts`; the children list theirs.
- **Motion:** none.
- **Test hooks:** the root is a `section` with `aria-label="Knobs"`; the page readout carries `data-part="page"`. The accent block's hue is read from AccentBlock's `data-hue` (see Needs below); the knobs from Knob's hooks.

### Accessibility

- **Role and name:** a `section` named "Knobs" (`aria-label`). Inside, in DOM and tab order: ▲ ("Knob page up"), ▼ ("Knob page down"), then knobs 1–8 (sliders "Knob 1: Dynamics" … "Knob 8: Tempo"). The header's texts are plain text, not focusable.
- **Keyboard:** Tab moves ▲, ▼, knob 1 … knob 8 (Stage.md D36); a disabled ▲ or ▼ stays focusable and does nothing; each knob handles its own arrows.
- **Tooltip ids** (wired at integration, `data-tip`): ▲ and ▼ `knobs.page`; each knob `knobs.knob` (both exist in `app/src/help/tooltips.ts`). The accent block and page readout carry none.
- **Launchkey:** ▲ is the encoder page ▲ button, ▼ the encoder page ▼ (`stepKnobPage`); knob i is encoder i + 1.

| Control | Callback | Sends (wiring) | Disabled when | Tooltip | Launchkey | `aria-label` |
|---|---|---|---|---|---|---|
| ▲ | `onpage(-1)` | `stepKnobPage { delta: -1 }` | `pageNumber <= 1` | `knobs.page` | encoder page ▲ | "Knob page up" |
| ▼ | `onpage(1)` | `stepKnobPage { delta: 1 }` | `pageNumber >= pageCount` | `knobs.page` | encoder page ▼ | "Knob page down" |
| Knob i | `onturn(i, delta)`, `onreset(i)` | `turnKnob { knob: i, delta }`, `resetKnob { knob: i }` | `knobs[i].function == 'none'` (Knob's `aria-disabled`) | `knobs.knob` | encoder i + 1 | "Knob {i + 1}: {knobName}", e.g. "Knob 1: Dynamics" |

## Fixture (`app/src/ui/KnobBank/KnobBank.fixtures.ts`)

Values from Stage.md › Board fixture (`knobs`, `transport.tempo`); `name` from the dev mock (`app/src/lib/api/mock-knobs.ts`), which the board fixture doesn't fix. FullBand imports `boardKnobBank`. Exports, exactly:

```ts
export const boardKnobBank = {
  pageName: 'Style', pageNumber: 1, pageCount: 6, swapPart: null, tempo: 104, shift: false,
  knobs: [
    { function: 'dynamics', name: 'Dynamics Control', short: 'DynCtrl', value: '127', level: 127 },
    { function: 'retriggerRate', name: 'Retrigger Rate', short: 'RtgRate', value: '1/8', level: 51 },
    { function: 'retriggerOnOff', name: 'Retrigger On/Off', short: 'RtgOnOff', value: 'Off', level: 0 },
    { function: 'trackMuteA', name: 'Style Track Mute A', short: 'StyMuteA', value: 'Off', level: 0 },
    { function: 'trackMuteB', name: 'Style Track Mute B', short: 'StyMuteB', value: 'Off', level: 0 },
    { function: 'swing', name: 'Swing', short: 'Swing', value: '0%', level: 0 },
    { function: 'none', name: 'No Assign', short: '---', value: '', level: null },
    { function: 'tempo', name: 'Tempo', short: 'Tempo', value: '104 BPM', level: null },
  ],
}

/** Swap mode on Right 1 (no board draws it): knob 1 the sound, 2–8 the part's mix, names as the dev mock gives them. */
export const swapKnobBank = {
  ...boardKnobBank, pageName: 'Swap R1', swapPart: 0,
  knobs: [
    { function: 'swapSound', name: 'Right 1 Sound', short: 'Sound', value: '1 Stage Grand', level: null },
    { function: 'partVolume', name: 'Right 1 Volume', short: 'Right1', value: '90', level: 90 },
    { function: 'partPan', name: 'Right 1 Pan', short: 'PanR1', value: 'C', level: 64 },
    { function: 'partReverb', name: 'Right 1 Reverb', short: 'RevR1', value: '40', level: 40 },
    { function: 'partChorus', name: 'Right 1 Chorus', short: 'ChoR1', value: '0', level: 0 },
    { function: 'partDelay', name: 'Right 1 Delay', short: 'DlyR1', value: '0', level: 0 },
    { function: 'insertSetting', name: 'Right 1 Insert 1 Setting 1', short: 'R1 I1.1', value: '0', level: 0 },
    { function: 'partSend', name: 'Right 1 Send 4', short: 'R1 Snd4', value: '0', level: 0 },
  ],
}

/** The last page, Delay 6/6 (the Effects board's knob values). */
export const lastPageKnobBank = {
  ...boardKnobBank, pageName: 'Delay', pageNumber: 6,
  knobs: [
    { function: 'partDelay', name: 'Right 1 Delay', short: 'DlyR1', value: '0', level: 0 },
    { function: 'partDelay', name: 'Right 2 Delay', short: 'DlyR2', value: '16', level: 16 },
    { function: 'partDelay', name: 'Right 3 Delay', short: 'DlyR3', value: '0', level: 0 },
    { function: 'partDelay', name: 'Left Delay', short: 'DlyL', value: '0', level: 0 },
    { function: 'delayTime', name: 'Delay Time', short: 'DlyTime', value: '1/8', level: 51 },
    { function: 'fxParam', name: 'Delay Feedback', short: 'DlyFdbk', value: '38%', level: 54 },
    { function: 'fxParam', name: 'Delay Tone', short: 'DlyTone', value: '5.0 kHz', level: 27 },
    { function: 'fxReturn', name: 'Delay Return', short: 'DlyRtn', value: '36', level: 36 },
  ],
}
```

## Stories (Story station)

- **Title:** `Components/KnobBank`.
- **Layout:** `centered` (real size, 626 × 140).

Every story renders in dark and light. Callbacks are actions; controls are KnobBank's own props, plus the eight knobs' `knobs` array as one object control (category "Knob"). Crop boxes are on Stage at 1440 × 900.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `boardKnobBank` | the board's bank: "Knobs", the "Style" block, "Page 1/6", ▲ disabled, ▼, the eight Style knobs | `Board-{dark,light}.png`: Stage 690,432 626×140 (the board draws ▲ in the enabled colour, D2) | the region "Knobs" holds 2 buttons and 8 sliders; `[data-part="page"]` text is "Page 1/6"; the AccentBlock reads "Style" and carries its default `data-hue="a"`; "Knob page up" has `aria-disabled` "true", a click → `onpage` not called; click "Knob page down" → `onpage(1)`; the slider "Knob 7: No Assign" has `aria-disabled` "true"; on "Knob 1: Dynamics" pointerdown at clientY 100, move to 92, up → `onturn(0, 2)` (Stage.md Check 12); double-click it → `onreset(0)`; the slider "Knob 8: Tempo" has `data-arc` "72". |
| `Swap` | `swapKnobBank` | the block "Swap R1" in Right 1's blue; knob 1 "Sound" with "1 Stage Grand" | — (no board draws swap mode) | the block's text is "Swap R1" and its element has `data-hue="r1"`; "Page 1/6" unchanged; the slider "Knob 1: Sound" has `aria-valuetext` "Sound 1 Stage Grand"; drag "Knob 2: Right 1 Volume" 8px up → `onturn(1, 2)`; ▼ click → `onpage(1)` (paging stays live in swap mode, D1). |
| `LastPage` | `lastPageKnobBank` | "Delay", "Page 6/6", ▼ disabled, ▲ live | — (Effects-Dark draws 6/6 but with ▼ in the enabled colour) | "Knob page down" has `aria-disabled` "true", a click → `onpage` not called; "Knob page up" click → `onpage(-1)`; `[data-part="page"]` text "Page 6/6". |
| `Focused` | `boardKnobBank`; `parameters: { pseudo: { focusVisible: ['[data-knob="0"]'] } }` | knob 1's focus ring inside the bank | — | — |

What jsdom can't check (layout, the hue of the swap block, the knobs' arcs) is covered by `Board`'s crop and by `Swap` and `Focused` judged in Storybook.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- KnobBank` passes: `Board` is 626 × 140 and scores at most 0.02 against its crop; axe finds no violation on any story.
- `KnobBank.fixtures.ts` exports `boardKnobBank`, `swapKnobBank` and `lastPageKnobBank` exactly as above.
- Only listed tokens are used; no colour literals; sizes only as in Visual rules. No child's look restated.
- svelte-check and lint pass on the folder.

## Decisions

- D1. ▲ ▼ stay live in swap mode: `stepKnobPage` changes the page the knobs return to, and the hardware's page buttons do the same.
- D2. ▲ is disabled on page 1 (kit › Knobs) although the board draws it in the enabled colour; the glyph's colour difference is far under the crop score.
- D3. The bank is a fixed 626 × 140, since the band is laid out at 1440 × 900 and only the shell scales (Stage.md D1); the half band of the tall pages is the Channel spec's variant (#501).
- D4. Swap mode's block hue follows `swapPart` (0 `r1`, 1 `r2`, 2 `r3`, 3 `l`) and its label is `pageName` as given; the page readout keeps the state's `pageNumber`.
- D5. ▼ is disabled when `pageNumber >= pageCount` and ▲ when `pageNumber <= 1`, so out-of-range numbers never leave a live button that does nothing.
- D6. The page readout is plain text inside the header, not a live region: the page is also in each knob's name and the tooltip, and a page step isn't urgent to announce.
- D7. `name` in the fixtures comes from the dev mock, since Stage.md's Board fixture doesn't fix it; only knobs whose function has no plain word (swap knobs 2–8, the Delay page) show it.

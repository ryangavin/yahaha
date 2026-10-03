# KnobBank

## Identity (all stations)

- **Kind:** complex
- **Built from:** GroupHeader, AccentBlock, Button, Knob
- **Purpose:** The band's eight knobs with their Knob Assign page: which page is up, stepping it, and turning each knob.
- **Boards:**
  - `Stage-Dark.dc.html:291-317` (header `:292-296`, ▲ ▼ `:298-301`, knob grid `:302-315`); light: `Stage-Light.dc.html:267-293`.
  - Crop: the bank is whole pixels at `690,432 626×140` on `docs/design/push/png/Stage-Dark.png` and `Stage-Light.png`.
- **Not this component's job:** no store, no API, no Tauri. It doesn't send `stepKnobPage`, `turnKnob` or `resetKnob`: it calls `onpage`, `onturn`, `onreset` and the page wiring sends (and sums one animation frame's turns per knob). It doesn't restate how a Knob, a Button, the AccentBlock or the GroupHeader draws (the "Page 1/6" counter is GroupHeader's `count`); it places them and passes their props. No timer, no tooltip wiring (integration adds `data-tip`).

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component. `KnobData` is declared and exported in `KnobBank.svelte`'s `<script module lang="ts">` block, `export type KnobData = { function: string; name: string; short: string; value: string; level: number | null }`, the shape of `knobs.knobs[i]` (a plain `string` function, since `app/src/ui` can't import the API types). `KnobBank.fixtures.ts` imports it with `import type { KnobData } from './KnobBank.svelte'`. Nothing in the Knob folder declares it (D8).

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

| Prop / callback | From `AppState` (or app-only state) / sends |
|---|---|
| `knobs` | `knobs.knobs` |
| `pageName` | `knobs.pageName` |
| `pageNumber`, `pageCount` | `knobs.pageNumber`, `knobs.pageCount` |
| `swapPart` | `surface.layer.part` when `surface.layer.type == 'swap'`, else null |
| `tempo` | `transport.tempo` |
| `shift` | `ui.shift`: app-only state in `app/src/lib/store.svelte.ts` (`UiStore`), not `AppState`; it already ORs the computer keyboard's Shift and `surface.shift` |
| `onpage(delta)` | `stepKnobPage { delta }` |
| `onturn(knob, delta)` | `turnKnob { knob, delta }`, one per animation frame per knob with that frame's deltas summed (the wiring's); in swap mode the session runs it as `turnSwapKnob` |
| `onreset(knob)` | `resetKnob { knob }` |

### Children

Every prop below exists in that child's spec (`origin/spec/ui-primitives`: Button, AccentBlock, GroupHeader; `app/src/ui/Knob/SPEC.md`).

| Where | Child | Props passed |
|---|---|---|
| Header row | `GroupHeader` | `{ title: 'Knobs', id: titleId, width: BANK_WIDTH, count: { label: 'Page', value: `${pageNumber}/${pageCount}` } }`, with the AccentBlock below as its `children` snippet. `titleId` is `` `${$props.id()}-title` `` (Svelte 5's per-instance id), so two banks never share an id. |
| Header, after the title (GroupHeader's `children`) | `AccentBlock` | `{ label: pageName, size: 'knob', hue: swapPart === null ? 'a' : (['r1', 'r2', 'r3', 'l'] as const)[swapPart] }` (`as` stays its default `'span'`) |
| Page column, top | `Button` | `{ symbol: 'up', size: 'icon', name: 'Knob page up', disabled: pageNumber <= 1, onpress: () => onpage(-1) }` |
| Page column, bottom | `Button` | `{ symbol: 'down', size: 'icon', name: 'Knob page down', disabled: pageNumber >= pageCount, onpress: () => onpage(1) }` |
| Knob grid, cell i (0–7) | `Knob` | `{ index: i, fn: knobs[i].function, name: knobs[i].name, short: knobs[i].short, value: knobs[i].value, level: knobs[i].level, tempo, shift, onturn: (d) => onturn(i, d), onreset: () => onreset(i) }` |

### Visual rules

- **Tokens used:** `--space-6`, `--space-8`, `--control-height`. No colour token: KnobBank draws no text or fill of its own (the children bring theirs).
- **Component geometry (axiom 2):** `KnobBank.svelte` declares `const BANK_WIDTH = 626` once; it is GroupHeader's `width` prop and, through `style:--bank-width="{BANK_WIDTH}px"` on the root, the root's width. The root rule declares `--bank-height: 140px` and `--bank-row-height: 96px` (the knob row, the Knob's own 96). No other length literals; every other length is a token above or a child's own.
- **Size:** fixed `--bank-width` × `--bank-height` (626 × 140; the band is laid out at 1440 × 900 and the shell scales it, Stage.md D1). A `section`, a flex column, no padding, no background, no border:
  1. **Header**: the `GroupHeader` (36 tall, its hairline the row's bottom pixel; its own gap of 12 before the AccentBlock; its counter "Page" in `--m` and "1/6" in `--t` at the right end, whose right edge is the bank's right edge).
  2. **Row**, `margin-top: var(--space-8)`, height `--bank-row-height`, `display: flex`, `gap: var(--space-8)`:
     - the **page column**: `width: var(--control-height)` (32), a flex column, `justify-content: center`, `gap: var(--space-6)`: ▲ then ▼, each the 32 × 32 `Button` `size: 'icon'`. At 1440 that puts ▲ at `690,489` and ▼ at `690,527` (13px above ▲ and below ▼ inside the 96px row).
     - the **knob grid**: `flex: 1` (586 wide), `display: grid; grid-template-columns: repeat(8, minmax(0, 1fr)); column-gap: var(--space-6)`, height `--bank-row-height`; each cell holds one 68 × 96 `Knob` (586 − 7 × 6 = 544, / 8 = 68). Knob i sits at `x = 730 + 74·i`, `y = 476`.
- **States drawn by:**
  - normal: the accent block in `--a` (`hue: 'a'`).
  - swap mode (`swapPart` not null): the accent block in the part's hue (`hue` `r1` / `r2` / `r3` / `l`); `pageName` reads "Swap R1" etc. as the state gives it; the counter keeps the page the knobs return to (the state's `pageNumber`).
  - first page: ▲ disabled (Button's disabled: its off face, glyph in `--d`). Last page: ▼ disabled. One page only (`pageCount` 1): both disabled.
  - keyboard focus: each child shows its own ring.
- **Type:** none of KnobBank's own; the counter is GroupHeader's.
- **Contrast:** KnobBank adds no pair; the children list theirs (GroupHeader's `--m`/`--t` on `--g` exist; AccentBlock's `--g` on `--a` and `--g` on `--r1`; Knob's `--a` on `--g`, added with the tokens PR).
  - **Known failures (owner question O-contrast):** the No Assign knob's name, `--d` on `--g` (dark 2.48, light 1.94), in `Board` and `Focused`; those stories set `parameters.a11y.context.exclude: ['[data-knob][aria-disabled="true"] [data-part="name"]']` until the owner answers. AccentBlock's light `--g` on `--r3` (3.65) and on `--l` (3.58) are its own known failures (AccentBlock D4); no KnobBank story uses those hues (`Swap` is `r1`).
- **Motion:** none.
- **Test hooks:** the root is a `section` with `aria-labelledby` set to GroupHeader's heading id (`titleId`), so its accessible name is "Knobs" with no second, separate label; it also carries `data-bank="knobs"`. The page counter is read as text ("Page 1/6", with "1/6" in its own element: GroupHeader's). The accent block's hue is read from AccentBlock's `data-hue`; the knobs from Knob's hooks (`data-knob`, `data-arc`, `aria-disabled`); the page buttons from Button's `aria-disabled`.

### Accessibility

- **Role and name:** a `section` (region) named "Knobs" by `aria-labelledby` pointing at GroupHeader's `h2` (D9). Inside, in DOM and tab order: ▲ ("Knob page up"), ▼ ("Knob page down"), then knobs 1–8 (sliders "Knob 1: Dynamics" … "Knob 8: Tempo"). The header's texts are plain text, not focusable.
- **Keyboard:** Tab moves ▲, ▼, knob 1 … knob 8 (Stage.md D36); a disabled ▲ or ▼ stays focusable and does nothing; each knob handles its own arrows.
- **Tooltip ids** (wired at integration, `data-tip`; no play asserts them): ▲ and ▼ `knobs.page`; each knob `knobs.knob` (both exist in `app/src/help/tooltips.ts`; Knob D16 lists a text change to `knobs.knob`). The accent block and the counter carry none.
- **Launchkey:** ▲ is the encoder page ▲ button, ▼ the encoder page ▼ (`stepKnobPage`); knob i is encoder i + 1.

| Control | Callback | Sends (wiring) | Disabled when | Tooltip | Launchkey | `aria-label` |
|---|---|---|---|---|---|---|
| ▲ | `onpage(-1)` | `stepKnobPage { delta: -1 }` | `pageNumber <= 1` | `knobs.page` | encoder page ▲ | "Knob page up" |
| ▼ | `onpage(1)` | `stepKnobPage { delta: 1 }` | `pageNumber >= pageCount` | `knobs.page` | encoder page ▼ | "Knob page down" |
| Knob i | `onturn(i, delta)`, `onreset(i)` | `turnKnob { knob: i, delta }`, `resetKnob { knob: i }` | `knobs[i].function == 'none'` (Knob's `aria-disabled`) | `knobs.knob` | encoder i + 1 | "Knob {i + 1}: {knobSpoken}", e.g. "Knob 1: Dynamics", "Knob 7: No Assign" |

## Fixture (`app/src/ui/KnobBank/KnobBank.fixtures.ts`)

Values from Stage.md › Board fixture (`knobs`, `transport.tempo`); `name` and `short` from the dev mock (`app/src/lib/api/mock-knobs.ts`), which the board fixture doesn't fix. Every `function` value is a member of `KnobFunction` in `app/src/lib/api/types.ts`, the strings the dev mock and the session emit (D10). FullBand imports `boardKnobBank`. Exports, exactly:

```ts
import type { KnobData } from './KnobBank.svelte'

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
  ] satisfies KnobData[],
}

/** Swap mode on Right 1 (no board draws it): knob 1 the sound, 2–8 the part's mix as the dev mock's swap map gives it (level, pan, reverb, chorus, delay, insert 1's first setting, send 4). */
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
  ] satisfies KnobData[],
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
  ] satisfies KnobData[],
}
```

## Stories (Story station)

- **Title:** `Components/KnobBank`.
- **Layout:** `centered` (real size, 626 × 140).
- **Meta:** `args: { onpage: fn(), onturn: fn(), onreset: fn() }` (axiom 7). Controls are KnobBank's own props: `pageName` text, `pageNumber` and `pageCount` `number` (min 1), `swapPart` a `select` of null / 0 / 1 / 2 / 3, `tempo` `number`, `shift` boolean, and `knobs` one `object` control (category "Knob").

Every story renders in dark and light. Crop boxes are on Stage at 1440 × 900. Plays find controls by role and name inside the region "Knobs" and use `fireEvent` / `userEvent` from `storybook/test` on the control itself (Knob's drag: `fireEvent.pointerDown(slider, { button: 0, pointerId: 1, clientY })`, `fireEvent.pointerMove(slider, { pointerId: 1, clientY })`, `fireEvent.pointerUp(slider, { button: 0, pointerId: 1, clientY })`; `fireEvent.dblClick(slider)`).

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `boardKnobBank`; `parameters.a11y.context.exclude: ['[data-knob][aria-disabled="true"] [data-part="name"]']` (Known failures) | the board's bank: "Knobs", the "Style" block, "Page 1/6", ▲ disabled, ▼, the eight Style knobs | `Board-{dark,light}.png`: Stage 690,432 626×140 (the board draws ▲ in the enabled colour, D2) | the region named "Knobs" (its `aria-labelledby` is the id of the heading level 2 "Knobs") holds 2 buttons and 8 sliders; the region's text contains "Page 1/6" and an element whose text is exactly "1/6"; the element with `data-size="knob"` reads "Style" and has `data-hue="a"`; "Knob page up" has `aria-disabled` "true", a click → `onpage` not called; click "Knob page down" → `onpage(1)`; the slider "Knob 7: No Assign" has `aria-disabled` "true"; on "Knob 1: Dynamics" pointerDown at clientY 100, pointerMove to 92, pointerUp → `onturn(0, 2)` (Stage.md Check 12); dblClick it → `onreset(0)`; the slider "Knob 8: Tempo" has `data-arc` "72". |
| `Swap` | `swapKnobBank` | the block "Swap R1" in Right 1's blue; knob 1 "Sound" with "1 Stage Grand" | — (no board draws swap mode) | the `data-size="knob"` element's text is "Swap R1" and it has `data-hue="r1"`; the region's text still contains "Page 1/6"; the slider "Knob 1: Sound" has `aria-valuetext` "Sound 1 Stage Grand"; drag "Knob 2: Right 1 Volume" from clientY 100 to 92 → `onturn(1, 2)`; "Knob 7: Right 1 Insert 1 Setting 1" and "Knob 8: Right 1 Send 4" exist (no plain word: the state's name); ▼ click → `onpage(1)` (paging stays live in swap mode, D1). |
| `LastPage` | `lastPageKnobBank` | "Delay", "Page 6/6", ▼ disabled, ▲ live | — (Effects-Dark draws 6/6 but with ▼ in the enabled colour) | "Knob page down" has `aria-disabled` "true", a click → `onpage` not called; "Knob page up" click → `onpage(-1)`; the region's text contains "Page 6/6". |
| `Focused` | `boardKnobBank`; `parameters: { pseudo: { focusVisible: ['[data-knob="0"]'] }, a11y: { context: { exclude: ['[data-knob][aria-disabled="true"] [data-part="name"]'] } } }` | knob 1's focus ring inside the bank, and no other ring | — | — |

`Focused` uses the pseudo-states addon's selector form (`focusVisible: ['<selector>']`, supported by `storybook-addon-pseudo-states`, installed in `.storybook/main.ts`) rather than the primitives' `focusVisible: true`, because `true` would draw the ring on all ten focusable children at once; the primitives use `true` since each has one focusable element (D11).

What jsdom can't check (layout, the hue of the swap block, the knobs' arcs) is covered by `Board`'s crop and by `Swap` and `Focused` judged in Storybook.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- KnobBank` passes: `Board` is 626 × 140 and scores at most 0.02 against its crop; axe finds no violation on any story (with the Known failures' exclusion).
- `KnobBank.fixtures.ts` exports `boardKnobBank`, `swapKnobBank` and `lastPageKnobBank` exactly as above.
- Only listed tokens are used; no colour literals; no length literals outside `BANK_WIDTH` and the root's custom properties. No child's look restated; the page counter is GroupHeader's `count`, not KnobBank markup.
- This PR doesn't touch `app/src/ui/tokens/*`, `contrast.test.ts`, `docs/app-api.md` or the children's folders.
- svelte-check and lint pass on the folder.

## Decisions

- D1. ▲ ▼ stay live in swap mode: `stepKnobPage` changes the page the knobs return to, and the hardware's page buttons do the same.
- D2. ▲ is disabled on page 1 (kit › Knobs) although the board draws it in the enabled colour; the glyph's colour difference is far under the crop score (Button D1).
- D3. The bank is a fixed 626 × 140, since the band is laid out at 1440 × 900 and only the shell scales (Stage.md D1); the half band of the tall pages is the Channel spec's variant (#501).
- D4. Swap mode's block hue follows `swapPart` (0 `r1`, 1 `r2`, 2 `r3`, 3 `l`) and its label is `pageName` as given; the counter keeps the state's `pageNumber`.
- D5. ▼ is disabled when `pageNumber >= pageCount` and ▲ when `pageNumber <= 1`, so out-of-range numbers never leave a live button that does nothing.
- D6. The page counter is plain text inside the header (GroupHeader's `count`), not a live region: the page is also in each knob's name and the tooltip, and a page step isn't urgent to announce.
- D7. `name` and `short` in the fixtures come from the dev mock, since Stage.md's Board fixture doesn't fix them; only knobs whose function has no plain word (swap knobs 2–8, the Delay page) show `name`.
- D8. `KnobData` lives in `KnobBank.svelte`'s module script: KnobBank is the only component that takes the state's knob objects (the Knob takes separate props), so the type stays in the KnobBank PR and the Knob PR owns nothing of it.
- D9. The region is named by `aria-labelledby` pointing at GroupHeader's heading (GroupHeader D2's `id`), not by an `aria-label`, so "Knobs" is said once and the name follows the heading. The id comes from `$props.id()`.
- D10. The fixture's swap knobs 7 and 8 are `insertSetting` and `partSend`, as the dev mock's swap map (`swapFns` in `app/src/lib/api/mock-knobs.ts`) and `KnobFunction` in `app/src/lib/api/types.ts` emit them. `docs/app-api.md` › `knobs` lists only 21 of the union's 25 functions (it lacks `insertOn`, `insertSetting`, `partSend`, `rotaryFast`); the doc is what's behind, so the fixture keeps the real values, and the doc gap goes to the orchestrator as a contract docs change.
- D11. `Focused` uses the pseudo-states addon's selector form to light only knob 1's ring; the primitives' `true` form would light every focusable child.
- D12. The page buttons are Button `size: 'icon'` with `symbol: 'up'` / `'down'` and an empty label; their accessible names are the `name` prop, since a symbol-only Button's default name would be "up" / "down".

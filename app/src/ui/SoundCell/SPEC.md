# SoundCell

## Identity (all stations)

- **Kind:** complex
- **Built from:** PartMarks
- **Build after:** PartMarks (with its `marksText` helper) has landed.
- **Purpose:** Shows which sound a keyboard part plays (its number, its name, and marks for edited, missing, failed, off or bass), with the part's tag that opens Channel and the sound that opens the quick sound list.
- **Boards:**
  - `Stage-Dark.dc.html:182-212` (the Sounds row's four part cells: R1 "1 Stage Grand"; R2 "41 Silk Strings" with the edited dot; R3 dimmed, "57 Brass…", ⚠ and "off"; L "41 Silk Strings"); light: `Stage-Light.dc.html:158-188`.
  - `FirstRun-Dark.png` / `FirstRun-Light.png` (R2 and R3 off without a plugin mark: "57 Brass Se…" then "off").
- **Not this component's job:** no store, no API, no Tauri. It doesn't look up the sound number, pick the sound name or decide which marks apply from `keyboardParts[i]`: the wiring passes them (the pure helpers below do the lookups). It doesn't open Channel or the quick sound list: it calls `ontag` and `onsound` and the wiring opens them (interim: today's `ChannelView`, and Library › Sounds for the part; Stage.md D32). It draws no marks itself (PartMarks does). It doesn't size its column (SoundsRow's grid does). Rack.md's two-line `variant: 'rack'` and Channel's 32-tall sound button are not folded in here (D8).

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `part` | `0 \| 1 \| 2 \| 3` | — | The keyboard part, in part order: 0 Right 1, 1 Right 2, 2 Right 3, 3 Left. Gives the tag ("R1", "R2", "R3", "L"), the hue (`r1`, `r2`, `r3`, `l`) and the part name in the `aria-label`s ("Right 1" …), from one table in the component. |
| `number` | `number \| null` | `null` | The sound's number in the user's library (`soundNumber`, below); `null`: no number is drawn. |
| `name` | `string` | — | The sound's name: `sound.name`, else `voiceName` (Stage.md D22; under Manual Bass the Style's Bass voice, `voiceName`). |
| `on` | `boolean` | `true` | `keyboardParts[i].on`. |
| `sounding` | `boolean` | `true` | `keyboardParts[i].sounding` (on, or Left playing the bass under Manual Bass). False: the dimmed look (tag and number `--d`, name `--m`). |
| `edited` | `boolean` | `false` | `soundEdited`: PartMarks' dot. |
| `missing` | `boolean` | `false` | `plugin.missing`: PartMarks' ⚠. |
| `failed` | `boolean` | `false` | `plugin.status === 'failed'`: PartMarks' ✕ (PartMarks drops it when `missing`). |
| `bass` | `boolean` | `false` | `playsBass`: PartMarks' "bass" (it drops "off"). |
| `tagTip` | `string \| undefined` | — | The tag's tooltip key (the Stage passes `mixer.strip.select`), as `data-tip` on the tag button (L3). |
| `soundTip` | `string \| undefined` | — | The sound's tooltip key (the Stage passes `launchkey.fader_sound`), as `data-tip` on the sound button (L3). |
| `tipAction` | `Action<HTMLElement, string> \| undefined` | — | The app's `use:tip`, passed in by the wiring; each button with a key gets `use:tipAction={key}` when both are set (L3). |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `ontag` | the tag button: click, Space or Enter | `()` (the parent knows the part; SoundsRow maps it to `onchannel(part)`) |
| `onsound` | the sound button: click, Space or Enter | `()` (SoundsRow maps it to `onsounds(part)`) |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Pure functions (`app/src/ui/SoundCell/sound.ts`)

- `soundNumber(sound: SoundTag | undefined, patches: PatchInfo[]): number | null`: the `number` of the patch whose `id` is `sound.id` without its `saved:` prefix; a `sf:…` or `au:…` id, a missing `sound`, or no such patch → `null` (Stage.md › Sounds row). `{ id: 'saved:p41' }` with the fixture's patches → 41.
- `soundName(p: { sound?: SoundTag; voiceName: string; playsBass: boolean }): string`: `playsBass` → `voiceName`; else `sound?.name` if non-empty, else `voiceName` (Stage.md D22).
- `soundLabel(c: { part, number, name, edited, missing, failed, off, bass }): string`: "{part name} sound: {number }{name}{marks}. Opens the quick sound list", where `{marks}` is PartMarks' `marksText({ edited, missing, failed, off, bass })`. Examples (Stage.md): "Right 2 sound: 41 Silk Strings, edited. Opens the quick sound list"; "Right 3 sound: 57 Brass Section, off, plugin missing. Opens the quick sound list"; `{ part: 3, number: null, name: 'Finger Bass', bass: true, … }` → "Left sound: Finger Bass, bass. Opens the quick sound list".
- `PART_TAGS = ['R1', 'R2', 'R3', 'L']`, `PART_HUES = ['r1', 'r2', 'r3', 'l']`, `PART_NAMES = ['Right 1', 'Right 2', 'Right 3', 'Left']`.

### Children

| Child | Where | Props passed | Callback |
|---|---|---|---|
| `PartMarks` | the sound button, after the name | `{ size: 'cell', edited, missing, failed, off: !on && !sounding, bass }` | — |

### Visual rules

- **Tokens used:** `--t`, `--m`, `--d`, `--line`, `--r1`, `--r2`, `--r3`, `--l`, `--focus`, `--font-sans`, `--text-12`, `--text-14`, `--weight-regular`, `--weight-medium`, `--space-4`, `--space-8`, `--control-height`, `--line-width`, `--focus-offset`. No new tokens (L1).
- **Root:** a `div` (`data-part="{0..3}"`), `width: 100%` of its container, height 44, `box-sizing: border-box`, `border-top: var(--line-width) solid var(--line)` (the Sounds row's 1px top edge, drawn by each cell; content box 43), `display: flex; align-items: center; gap: var(--space-4)`, `min-width: 0`, `font-family: var(--font-sans)`, `font-variant-numeric: tabular-nums`.
- **Tag:** a `<button type="button">`, width `--control-height` (32), height 43, `flex: none`, padding 0, no border or background, `text-align: left` (its text vertically centred, a button's default), 14 / 500 (`--text-14`, `--weight-medium`), line-height normal, `white-space: nowrap`, in the part's hue `--<hue>`; `data-hue="<hue>"`. Not sounding: `--d`, `data-hue="d"`, `data-contrast="dim"` (Stage.md D47).
- **Sound:** a `<button type="button">`, `flex: 1; min-width: 0`, height 43, padding 0, no border or background, `display: flex; align-items: center; gap: var(--space-8)`, `white-space: nowrap`, `text-align: left`, line-heights normal (the row's centring places everything):
  - the number (when not `null`), a span `data-part="number"`, 12 / 400 `--m`, `flex: none`; not sounding: `--d` with `data-contrast="dim"`;
  - the name, a span `data-part="name"`, 14 / 400 `--t`, `min-width: 0`, `overflow: hidden`, `text-overflow: ellipsis`; not sounding: `--m`;
  - PartMarks (Children), `flex: none` (its own root).
- **States drawn by:** `sounding` (the dimmed tag, number and name, above); the marks (PartMarks); no hover or pressed look (kit D35); cursor `pointer` on both buttons. Keyboard focus: each button gets a `--line-width` outline in `--focus` at `--focus-offset` on `:focus-visible`.
- **Type:** DM Sans, tabular numerals; tag 14 / 500, number 12 / 400, name 14 / 400.
- **Contrast (AA 4.5:1, `tokens/contrast.test.ts`):** existing rows `--t` on `--g`, `--m` on `--g`. The tags are `--r1`, `--r2`, `--r3`, `--l` on `--g`: those rows are Stage.md contract change C6's (the part hues on `--g` and `--btn`); until C6 lands, any failing tag in light is reported by axe as C6 and Inspect names it so (D6). Dimmed `--d` text carries `data-contrast="dim"` and is exempt (Stage.md D47).
- **Motion:** none.

### Accessibility

- **Role and name:** two buttons. The tag: `aria-label` "{part name}: open Channel" ("Right 1: open Channel"). The sound: `aria-label` = `soundLabel({ part, number, name, edited, missing, failed, off: !on && !sounding, bass })`. The children of the sound button are visible text; PartMarks is `aria-hidden`. Neither button has `aria-pressed` or `aria-disabled` (always pressable).
- **Keyboard:** Tab reaches the tag, then the sound; Space or Enter calls `ontag` / `onsound`.
- **Tooltip id:** the tag `mixer.strip.select`, the sound `launchkey.fader_sound` (both exist; C5 rewrites their bodies, not their keys), passed as `tagTip` / `soundTip` with `tipAction` (L3).

## Stories (Story station)

Title `Components/SoundCell`, `layout: 'centered'`. Every story renders inside a decorator `div` of `width: 145px` (the board's cell is 145.5 wide; D5). Every story renders in dark and light. The meta's `args` are `{ ontag: fn(), onsound: fn(), tipAction: fn(), tagTip: 'mixer.strip.select', soundTip: 'launchkey.fader_sound' }`.

**Controls (argTypes):** `part` a select of 0–3; `number` a number (cleared = `null`); `name`, `tagTip`, `soundTip` text; `on`, `sounding`, `edited`, `missing`, `failed`, `bass` boolean; `ontag`, `onsound`, `tipAction` actions. PartMarks' props are all derived, so there is no child group.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ part: 0, number: 1, name: 'Stage Grand' }` | Right 1 at the board fixture: blue "R1", "1", "Stage Grand", no marks | `Board-{dark,light}.png` (Stage 257,351 145×44) | the tag button "Right 1: open Channel" has `data-hue="r1"`, `data-tip="mixer.strip.select"`; the sound button is named "Right 1 sound: 1 Stage Grand. Opens the quick sound list" and has `data-tip="launchkey.fader_sound"`; no `[data-mark]`; `tipAction` called twice; click the tag → `ontag` called once, `onsound` not; Enter on the sound → `onsound` called once |
| `Edited` | `{ part: 1, number: 41, name: 'Silk Strings', edited: true }` | Right 2: pink tag, the edited dot after the name | — (the cell's x is fractional on the board, 410.5) | the sound's name is "Right 2 sound: 41 Silk Strings, edited. Opens the quick sound list" (Stage.md's example); one `[data-mark="edited"]` |
| `Missing` | `{ part: 2, number: 57, name: 'Brass Section', on: false, sounding: false, missing: true, failed: true }` | Right 3 at the board fixture: dimmed tag and number, grey name ending "…", ⚠ then "off" | `Missing-{dark,light}.png` (Stage 564,351 145×44) | the tag has `data-hue="d"` and `data-contrast="dim"`; `[data-part="number"]` has `data-contrast="dim"`; the marks in order are `missing`, `off` (no `failed`); the sound's name is "Right 3 sound: 57 Brass Section, off, plugin missing. Opens the quick sound list" |
| `Off` | `{ part: 2, number: 57, name: 'Brass Section', on: false, sounding: false }` | an off part with no plugin trouble: "57 Brass Se…" and "off" | `Off-{dark,light}.png` (FirstRun 564,351 145×44) | one `[data-mark="off"]`; the sound's name ends ", off. Opens the quick sound list" |
| `Failed` | `{ part: 2, number: 57, name: 'Brass Section', failed: true }` | a failed (not missing) plugin: the red ✕ | — (no board draws it rendered) | one `[data-mark="failed"]`; the name contains ", plugin failed" |
| `Bass` | `{ part: 3, number: null, name: 'Finger Bass', on: false, sounding: true, bass: true }` | Left under Manual Bass: drawn as sounding (teal tag, white name), "bass", no "off" | — (no board draws it) | the tag has `data-hue="l"`; no number span; marks `bass` only; the sound's name is "Left sound: Finger Bass, bass. Opens the quick sound list" |
| `NoNumber` | `{ part: 0, number: null, name: 'Sampler Deluxe' }` | a plugin or GM sound: no number | — | no `[data-part="number"]` |
| `LongName` | `{ part: 1, number: 141, name: 'Silk Strings Ensemble with Slow Attack', edited: true, missing: true }` | the name ellipsizes; the number and both marks keep their size | — | both marks are in the DOM; the name span's `textContent` is the whole name |
| `Focused` | `Board`'s args, `parameters: { pseudo: { focusVisible: true } }` | both focus rings | — | — |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in dark and light; each crop is the cell's box, the 1px top edge included (L6).

**Unit tests** (`app/src/ui/SoundCell/sound.test.ts`, vitest): `soundNumber` for `saved:p41` (41), `sf:0:48` (`null`), `au:aumu Smp7 Fake` (`null`), `undefined` (`null`), an unknown `saved:` id (`null`); `soundName` for `playsBass` (the voice name), a `sound` with a name, a `sound` with an empty name, no `sound`; `soundLabel` for the three examples above.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`); `sound.test.ts` passes.
- `npm run shots -- SoundCell` passes for `Board`, `Missing` and `Off` (score at most 0.02, or the Inspect agent judges any difference render noise), and axe finds no violation on any story, except light part-hue tags until C6 lands (D6).
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules (44, 43).
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · Two buttons, one cell.** The tag (Channel) and the sound (the quick sound list) are separate buttons in one cell, as Stage.md › Sounds row has them; each has its own `aria-label` and tooltip key.
- **D2 · Facts in, marks decided by PartMarks.** The cell passes the raw facts and `off: !on && !sounding`; PartMarks drops ✕ under ⚠ and "off" under "bass", so the cell, the strip and the `aria-label` (`marksText`) never disagree.
- **D3 · The part is an index.** `part` 0–3 gives the tag, hue and part name from one table (`PART_TAGS`, `PART_HUES`, `PART_NAMES`), so a parent can't pair R2's tag with R3's hue.
- **D4 · Each cell draws its own top edge.** The Sounds row's 1px `--line` top edge is drawn by each cell (and by RackReadout's `cell`), so the 145 × 44 crops include it and SoundsRow is only a grid.
- **D5 · Story width 145px.** The board's part cells are 145.5px wide at fractional x for R2 and L; the stories use a 145px decorator so the crops (R1 at 257, R3 at 564) are whole pixels; the half pixel only moves the ellipsis point by a fraction of a glyph.
- **D6 · L2, the part hues as text.** The tags are part hues on `--g`; Stage.md's contract change C6 adds those rows and fixes the light hues that fail; until it lands axe reports exactly those pairs on the light stories, and Inspect names them C6.
- **D7 · `Off` crop from FirstRun.** The Stage board has no off part without a plugin mark; FirstRun's R3 (564,351) is one, so `Off-{dark,light}.png` is cut there; the cell's other text on that board (name, number) matches the args.
- **D8 · Other screens' sound cells are not folded in.** Rack.md row 30 (`variant: 'rack'`, two lines, no tag) and Channel's 32-tall sound button differ in layout and children; they stay with those screens' lanes, which may extend this component or build their own, and say so in their specs.
- **D9 · L3, tooltips per item.** The cell takes `tagTip` and `soundTip` (keys `mixer.strip.select` and `launchkey.fader_sound`, both in `tooltips.ts`) and one `tipAction`; stories pass `tipAction: fn()`. Neither button is ever disabled, so no `aria-disabled` (L5).

Follow-ups: Rack's two-line sound cell and Channel's sound button (D8).

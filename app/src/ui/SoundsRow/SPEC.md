# SoundsRow

## Identity (all stations)

- **Kind:** complex
- **Built from:** RackReadout, SoundCell
- **Build after:** RackReadout and SoundCell have landed.
- **Purpose:** The bottom line of the Stage's display: the rack under the player's hands and the sound each of the four keyboard parts plays, each a way into the Rack page, Channel or the quick sound list.
- **Boards:** `Stage-Dark.dc.html:177-213` (the row at the board fixture: Rack · A1 "Sunday drive •", R1 Stage Grand, R2 Silk Strings •, R3 dimmed with ⚠ and "off", L Silk Strings); light: `Stage-Light.dc.html:153-189`. Crop box `Stage 49,351 814×44`.
- **Not this component's job:** no store, no API, no Tauri. It doesn't read `liveRack`, `quickRacks`, `keyboardParts` or `soundLibrary`: the wiring (through Display) passes the rack's three values and each part's sound values, computed with RackReadout's `rackSlot` and SoundCell's `soundNumber` / `soundName`. It doesn't open pages: it calls `onrack`, `onchannel(part)` and `onsounds(part)` (Stage.md D32 gives the interim targets). The cells' looks are their components'; this spec is the grid and the props each cell gets.

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `rack` | `{ name: string; slot: string \| null; modified: boolean }` | — | The live rack: `name` as shown, `slot` from `rackSlot(quickRacks.bank, quickRacks.buttons)` ("A1" or `null`, Stage.md D21), `modified` = `liveRack.modified`. |
| `parts` | `PartSound[]` (4 entries, part order R1, R2, R3, L) | — | Each part's sound: `{ number: number \| null; name: string; on: boolean; sounding: boolean; edited: boolean; missing: boolean; failed: boolean; bass: boolean }` (SoundCell's props of the same names; `PartSound` is exported from `app/src/ui/SoundsRow/types.ts`). Fewer than four: the missing cells aren't drawn (the grid keeps their columns). |
| `tipAction` | `Action<HTMLElement, string> \| undefined` | — | The app's `use:tip`, passed to every child (L3). |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onrack` | the rack readout is pressed (RackReadout's `onpress`) | `()` → the Rack page (interim: the Rack drawer, `ui.toggleDrawer('rack')`, opened) |
| `onchannel` | a part's tag is pressed (SoundCell's `ontag`) | `(part: 0 \| 1 \| 2 \| 3)` → Channel for that part (interim: `channelNav.show(part)`) |
| `onsounds` | a part's sound is pressed (SoundCell's `onsound`) | `(part: 0 \| 1 \| 2 \| 3)` → the quick sound list for the part (#514; interim: `ui.openLibrary('sounds', part)`) |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Children

Left to right, in DOM order:

| # | Child | Props passed | Callback → |
|---|---|---|---|
| 1 | `RackReadout` | `{ ...rack, size: 'cell', tip: 'stage.rack_name', tipAction }` | `onpress()` → `onrack()` |
| 2–5 | `SoundCell` × 4, part `i` = 0…3 | `{ part: i, ...parts[i], tagTip: 'mixer.strip.select', soundTip: 'launchkey.fader_sound', tipAction }` | `ontag()` → `onchannel(i)`; `onsound()` → `onsounds(i)` |

What the wiring passes, from the board fixture's state (Stage.md › Board fixture), so the stories' args are these values:

| Child value | From |
|---|---|
| `rack` | `{ name: liveRack.name, slot: rackSlot(quickRacks.bank, quickRacks.buttons), modified: liveRack.modified }` → `{ name: 'Sunday drive', slot: 'A1', modified: true }` |
| `parts[i]` | `{ number: soundNumber(kp.sound, soundLibrary.patches), name: soundName(kp), on: kp.on, sounding: kp.sounding, edited: kp.soundEdited ?? false, missing: kp.plugin?.missing ?? false, failed: kp.plugin?.status === 'failed', bass: kp.playsBass }` with `kp = keyboardParts[i]` |

### Visual rules

- **Tokens used:** `--space-8`. Everything else is the children's.
- **Layout:** the root is a `div`, `width: 100%` of its container (814 in the Stage's display), height 44, `display: grid; grid-template-columns: 200px repeat(4, minmax(0, 1fr)); column-gap: var(--space-8)`, `align-items: stretch`. At 814 the part cells are 145.5 wide (Stage.md › Sounds row). Each child fills its grid cell and draws its own 1px `--line` top edge (SoundCell D4; RackReadout's `cell` likewise), so the gaps between cells have no line, as the board draws.
- **States drawn by:** the children only.
- **Type:** the children's.
- **Contrast:** none of its own; the children list theirs (SoundCell's part hues are Stage.md C6's rows).
- **Motion:** none.

### Accessibility

- **Role and name:** the root is a plain `div` with no role (D2). Inside it, in reading order: the rack readout button, then per part the tag button and the sound button (their names are the children's).
- **Keyboard:** Tab order is DOM order: rack, R1 tag, R1 sound, R2 tag, …, L sound (Stage.md D36). No keys of its own.
- **Tooltip id:** none of its own; the children carry `stage.rack_name`, `mixer.strip.select` and `launchkey.fader_sound`.

## Stories (Story station)

Title `Components/SoundsRow`. Every story renders inside a decorator `div` of `width: 814px` (the display's content width), `layout: 'centered'`. Every story renders in dark and light. The meta's `args` are the `Board` args plus `onrack: fn()`, `onchannel: fn()`, `onsounds: fn()`, `tipAction: fn()`.

**Controls (argTypes):** `rack` and `parts` `object` controls (grouped "RackReadout" and "SoundCell"); `onrack`, `onchannel`, `onsounds`, `tipAction` actions.

Board parts (`boardParts`, exported from `SoundsRow.stories.ts` for reuse): R1 `{ number: 1, name: 'Stage Grand', on: true, sounding: true, edited: false, missing: false, failed: false, bass: false }`; R2 `{ number: 41, name: 'Silk Strings', on: true, sounding: true, edited: true, … false }`; R3 `{ number: 57, name: 'Brass Section', on: false, sounding: false, edited: false, missing: true, failed: true, bass: false }`; L `{ number: 41, name: 'Silk Strings', on: true, sounding: true, … false }`.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ rack: { name: 'Sunday drive', slot: 'A1', modified: true }, parts: boardParts }` | the Stage board's Sounds row | `Board-{dark,light}.png` (Stage 49,351 814×44) | Stage.md Check 7: R2's sound button contains "41" and "Silk Strings" and one `[data-mark="edited"]`, and its name is "Right 2 sound: 41 Silk Strings, edited. Opens the quick sound list"; R3's sound contains `[data-mark="missing"]` and `[data-mark="off"]`, and the tag "Right 3: open Channel" has `data-hue="d"`; the rack button's spans read "Rack", " · ", "A1", "Sunday drive" and it holds `[data-dot="modified"]`. Then click the rack → `onrack` called once; click "Right 1: open Channel" → `onchannel` called with `0`; click R3's sound → `onsounds` called with `2`; the canvas has 9 buttons, and pressing Tab 9 times from the rack (focused first) walks them in the order rack, R1 tag, R1 sound, …, L sound |
| `NoSlot` | `Board`'s args with `rack: { name: 'Sunday drive', slot: null, modified: false }` | "Rack" alone over the name, no dot (the loaded rack's button in another bank, D21) | — | the rack's name is "Rack: Sunday drive. Opens the Rack page" |
| `Failed` | `Board`'s args with R3 `{ …, missing: false, failed: true, on: true, sounding: true }` | R3 sounding with the red ✕ (Stage › States, failed plugin) | — | R3's sound contains `[data-mark="failed"]`, no `off`; its tag has `data-hue="r3"` |
| `ManualBass` | `Board`'s args with L `{ number: null, name: 'Finger Bass', on: false, sounding: true, bass: true, … false }` | Left playing the Style's bass: drawn as on, "bass" | — | L's sound name is "Left sound: Finger Bass, bass. Opens the quick sound list" |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in dark and light; the crop is the row's box (L6).

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- SoundsRow` passes for `Board` (score at most 0.02, or the Inspect agent judges any difference render noise), and axe finds no violation on any story, except the light part-hue tags until Stage.md C6 lands (SoundCell D6).
- Only `--space-8` is used directly, plus the literal 44 and 200px of the Visual rules.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · Values in, not the state.** The row takes `rack` and four `parts` as plain values; the lookups (slot, number, name) are RackReadout's and SoundCell's pure helpers, called by the wiring, so the row never sees `AppState` (Stage.md D46).
- **D2 · No landmark or group.** Stage.md gives the row no role or name; each button's `aria-label` already names its part, so a group would only add a stop for screen readers. The root is a plain `div`.
- **D3 · Part callbacks carry the index.** SoundCell's `ontag` and `onsound` take no payload; the row adds the part index (`onchannel(i)`, `onsounds(i)`), matching the Stage's `onopen({ channel: part })` and `onopen({ sounds: part })` targets.
- **D4 · Tooltip keys are fixed here.** The row passes `stage.rack_name`, `mixer.strip.select` and `launchkey.fader_sound` (all in `tooltips.ts`; C5 rewrites the last two's bodies, not their keys) and one `tipAction` to every child (L3).
- **D5 · The grid owns no lines.** Each cell draws its own top edge (SoundCell D4), so the 8px gaps between cells stay ground, as the board draws.

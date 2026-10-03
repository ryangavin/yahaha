# StatusDot

## Identity (all stations)

- **Kind:** primitive
- **Built from:** —
- **Purpose:** A small round light that says at a glance whether something is connected, running, waiting or changed, in the colour of what it belongs to.
- **Boards:**
  - `Stage-Dark.dc.html:89` (Launchkey status, connected: green with its glow), `:161` (the dot before "Section", in the Main hue with its glow), `:171` (the run state "Running": green with its glow), `:180` (the rack readout's 5px "modified" dot, white, no glow); light: `Stage-Light.dc.html:65`, `:137`, `:147`, `:156`.
  - Other boards, same faces: `Channel-Dark.dc.html:147` (the compact "Running" dot, named, `role="img"`), `LibraryRacks-Dark.dc.html:213` ("Sounding", named), `Prompts-More-Dark.dc.html:465` (white dot with a glow: waiting for the Finder), `Browser-Dark.dc.html:211` (the loaded style's track dot); light: `Channel-Light.dc.html:111`, `LibraryRacks-Light.dc.html:186`, `Prompts-More-Light.dc.html:446`, `Browser-Light.dc.html:191`.
  - The hollow ring (Launchkey not connected, Sync start armed) is not drawn on any board; its look is fixed here (kit › App bar, Stage › Section and tempo).
- **Not this component's job:** no store, no API, no Tauri. It doesn't know what it reports: the parent picks `hue`, `hollow` and `visible` from state. No text beside it (the parent's label: "Launchkey", "Running", "Section"), no tooltip, no click, no motion of its own. Not the knob's tip dot (Knob draws that) and not the sound cell's "edited" mark (PartMarks).

Its users and the props they pass:

| User | State | Props |
|---|---|---|
| LaunchkeyStatus | `pads.connected` true | `{ hue: 'ok' }` |
| LaunchkeyStatus | `pads.connected` false | `{ hue: 'd', hollow: true }` |
| SectionReadout | the playing section's hue; running | `{ hue: <section hue> }` (`'main'` on the board) |
| SectionReadout | stopped (D43: hidden, space kept) | `{ hue: <section hue>, visible: false }` |
| TempoReadout | `transport.running` | `{ hue: 'ok' }` |
| TempoReadout | stopped with `transport.syncStart` | `{ hue: 'ok', hollow: true }` |
| TempoReadout | stopped | no StatusDot (the parent leaves it out, so its space collapses) |
| RackReadout | `liveRack.modified` | `{ size: 'sm', hue: 't', glow: false }` |
| Channel compact block (#501) | running | `{ hue: 'ok', name: 'Running' }` |

## API (Component station)

### Props

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `hue` | `'ok' \| 'd' \| 't' \| 'intro' \| 'main' \| 'ending' \| 'brk' \| 'fill'` | `'ok'` | The colour role (kit › Hue roles): `ok` connected / running / sounding, `d` absent (not connected), `t` white (modified, waiting on the player), and the five section hues. |
| `hollow` | `boolean` | `false` | A 1px ring in the hue with nothing inside, and never a glow: "armed, not yet" (Sync start) or "not there" (Launchkey not connected). |
| `glow` | `boolean` | `true` | The 6px glow around a solid dot. Ignored when `hollow`. The rack's modified dot passes `false`. |
| `size` | `'md' \| 'sm'` | `'md'` | `md`: 6 × 6 (every status dot). `sm`: 5 × 5 (the rack readout's modified dot). |
| `visible` | `boolean` | `true` | `false` hides the dot but keeps its box (`visibility: hidden`), so the text after it doesn't move (the section dot while stopped, Stage D43). |
| `name` | `string \| undefined` | — | When given, the dot is an image with this accessible name (`role="img"`, `aria-label`), for a dot with no word beside it ("Running" in the Channel's compact block, "Sounding"). Without it the dot is decorative (`aria-hidden="true"`): its parent's text or `aria-label` says what it means. |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| — | | |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Visual rules

- **Tokens used:** `--ok`, `--d`, `--t`, `--intro`, `--main`, `--ending`, `--brk`, `--fill`, `--dot-glow-mix` (kit › Tokens to add: dark `70%`, light `0%`), `--line-width`.
- **Element:** one `span`, `display: inline-block`, `flex: none`, `box-sizing: border-box`, `border-radius: 50%`, vertical-align middle. It carries `data-hue="<hue>"`, `data-face="solid" | "hollow"`, `data-glow="true" | "false"` (whether a glow is drawn: `glow && !hollow`) and `data-size="md" | "sm"`.
- **Size:** `md` 6 × 6; `sm` 5 × 5. Never stretches or shrinks.
- **States drawn by:**
  - solid (default): `background: var(--<hue>)`, no border.
  - solid with glow (`glow` true, the default): plus `box-shadow: 0 0 6px color-mix(in srgb, var(--<hue>) var(--dot-glow-mix), transparent)`. For `ok` in dark that is exactly the board's `--bg`; in light `--dot-glow-mix` is 0%, so the shadow is transparent and nothing draws (Stage › States, Light theme).
  - solid without glow (`glow` false): no `box-shadow`.
  - hollow: `background: transparent`, `border: var(--line-width) solid var(--<hue>)`, no `box-shadow` (whatever `glow` says).
  - hidden (`visible` false): `visibility: hidden`, set as an inline style (`style:visibility`) so a test can read it; the 6 × 6 (or 5 × 5) box stays in the layout.
  - No hover, focus or pressed look: it isn't a control.
- **Type:** none (no text).
- **Contrast:** no text, so no `tokens/contrast.test.ts` pair. The dot is never the only carrier of its meaning (its parent's word or `aria-label` says it, or `name` does), so WCAG 1.4.11's 3:1 doesn't bind it; light `--d` on `--g` (the hollow "not connected" ring) is about 2:1 and that is accepted (D6).
- **Motion:** none. A blinking dot would need a clock prop; nothing on the boards blinks.

### Accessibility

- **Role and name:** without `name`, `aria-hidden="true"` and no role. With `name`, `role="img"` and `aria-label` = `name`.
- **Keyboard:** not focusable.
- **Tooltip id:** none of its own; the parent's control carries the tooltip (`launchkey.status`, `display.tempo`, `stage.rack_name`).

## Stories (Story station)

Title `Primitives/StatusDot`, `layout: 'centered'`. Every story renders in dark and light (the toolbar theme). There is no `Focused` story: it isn't focusable.

**Glow crops.** The glowing dots' crops are the 6 × 6 dot plus 6px of ground on every side (18 × 18), so the glow is in the picture. Those stories set `parameters: { shots: { pad: 6 } }`, which asks `npm run shots` to grow its clip by 6px round the story's root (D5). The 5 × 5 `Small` crop has no glow and no padding.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ hue: 'ok' }`; `shots: { pad: 6 }` | the Launchkey status dot, connected: solid green with its glow (no glow in light) | `Board-{dark,light}.png` (Stage 1223,33 18×18: the dot at 1229,39 plus 6px) | the dot (`[data-hue]`) has `aria-hidden="true"`, `data-hue="ok"`, `data-face="solid"`, `data-glow="true"`, `data-size="md"`; there is no `img` role in the canvas |
| `Section` | `{ hue: 'main' }`; `shots: { pad: 6 }` | the dot before "Section" while Main B plays: Main green with its glow | `Section-{dark,light}.png` (Stage 368,168 18×18: the dot at 374,174 plus 6px) | `data-hue="main"`, `data-face="solid"`, `data-glow="true"` |
| `Intro` | `{ hue: 'intro' }` | the section dot in another section's hue, its glow in that hue (Stage D29) | — (the boards draw only Main) | `data-hue="intro"`, `data-glow="true"` |
| `Stopped` | `{ hue: 'main', visible: false }` | nothing visible; the 6 × 6 box is still laid out | — (nothing to see) | the dot (`[data-hue="main"]`) is still in the DOM and its inline `style.visibility` is `hidden` (jsdom has no layout; `visibility: hidden` is what keeps the 6px box, so checking it is enough) |
| `Disconnected` | `{ hue: 'd', hollow: true }` | the Launchkey not connected: a 1px dimmed ring, empty inside, no glow | — (no board draws it) | `data-face="hollow"`, `data-hue="d"`, `data-glow="false"` |
| `SyncStart` | `{ hue: 'ok', hollow: true }` | Sync start armed: a 1px green ring, empty inside, no glow | — (no board draws it) | `data-face="hollow"`, `data-hue="ok"`, `data-glow="false"` |
| `Small` | `{ size: 'sm', hue: 't', glow: false }` | the rack readout's modified dot: 5 × 5 white (`--t`: black in light), no glow | `Small-{dark,light}.png` (Stage 139,380 5×5) | `data-size="sm"`, `data-hue="t"`, `data-glow="false"` |
| `Named` | `{ hue: 'ok', name: 'Running' }` | the Channel's compact running dot, read out on its own | — (the same pixels as `Board`) | an `img` named "Running" exists and has `data-hue="ok"`; it has no `aria-hidden` |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in the dark and light render. The glow's look (the `color-mix`, and its absence in light) is checked by the `Board` and `Section` screenshots; jsdom can't compute it, so the plays check `data-glow` only.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- StatusDot` passes for `Board`, `Section` and `Small` (score at most 0.02, or the Inspect agent judges any difference to be render noise), with the `pad: 6` clip for the first two (D5).
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules (6, 5).
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · Props, not a state enum.** The faces are `hue` × `hollow` × `glow` × `size` rather than a `state` enum (`connected`, `running`, …), because the dot doesn't know what it reports and every user maps its own state (table above); a fixed enum would need a new member for each new user.
- **D2 · One glow formula for every hue.** The glow is always `0 0 6px color-mix(in srgb, <hue> var(--dot-glow-mix), transparent)`, never `--bg` or `--bw` directly: for `ok` and `main` in dark it equals the board's `--bg`, and it gives an Intro or a Fill dot a glow in its own hue (Stage D29); Prompts-More's white dot gets 70% white instead of `--bw`'s 60%, a difference that spec may override.
- **D3 · `sm` lives here.** The rack readout's 5px modified dot is StatusDot `size: 'sm'` (as Stage's Components table builds RackReadout from StatusDot), not a separate mark; PartMarks draws the identical 5px edited dot itself so it stays a primitive.
- **D4 · Decorative unless named.** Every Stage dot sits next to a word or inside a control whose `aria-label` already says it, so the dot is `aria-hidden` by default; `name` makes it a named `img` for the few dots that stand alone (Channel's compact "Running", LibraryRacks' "Sounding").
- **D5 · Glow crops are padded.** The glowing dots' crops include 6px of ground on every side so the glow is compared too; `scripts/shots.ts` needs a per-story `parameters.shots.pad` (px added round the root's box) for these to match in size. Until that lands, the Inspect agent compares them by eye.
- **D6 · The hollow ring's light contrast is accepted.** Light `--d` (#b0afab) on `--g` is about 2:1, under the 3:1 for graphics, but the ring never carries meaning alone: "Launchkey" turns `--d` and its `aria-label` says "not connected".
- **D7 · Hidden keeps its space.** `visible: false` is `visibility: hidden`, not removal, so the section label doesn't shift when the band starts (Stage D43); a parent that wants the space to collapse (the stopped run state) leaves the dot out.

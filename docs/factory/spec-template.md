# Component spec template

Every component in the library gets one spec at `app/src/ui/<Name>/SPEC.md`. The spec is the factory's raw material: the line never invents anything the spec doesn't say. Each station reads only the sections marked for it, so a section must stand on its own.

Board crops live beside the spec in `app/src/ui/<Name>/crops/`, one PNG per story and theme (`<story>-dark.png`, `<story>-light.png`), cut from the Push canvas renders (`../yahaha-research/push-canvas/render/`). They are the pictures the Inspect station compares against.

Copy everything below the line.

---

```markdown
# <Name>

## Identity (all stations)

- **Kind:** primitive | complex
- **Built from:** — | <Name>, <Name> (complex only; these must already be merged)
- **Purpose:** one sentence, in the player's words. ("Turns a part, a mode or a function on or off.")
- **Boards:** where it appears on the canvas, as `<board file>:<lines>`, at least one dark and one light.
- **Not this component's job:** what it must not do. (For every component: no store, no API, no Tauri. Data comes in as props, actions go out as callbacks.)

## API (Component station)

### Props

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `label` | `string` | — | The word on the face. |
| `on` | `boolean` | `false` | Lit (lime face) or off. |
| `code` | `string \| undefined` | — | Small code after the label, e.g. `ACMP`. |
| `disabled` | `boolean` | `false` | Shown, not pressable. |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `ontoggle` | click, Space or Enter | `(on: boolean)` the new state |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Visual rules

- **Tokens used:** `--lamp`, `--lamp-ink`, `--btn`, `--d`, … (semantic tokens only, from `app/src/ui/tokens/`; a missing token is an andon pull, never a literal value).
- **Size:** fixed (w × h) or fills its container; min/max where it matters.
- **States drawn by:** what changes between states (face fill, label colour, outline, glow). One line per state.
- **Type:** size, weight, case, tabular numerals or not.
- **Motion:** e.g. "queued pads flash at the beat"; or none.

### Accessibility

- **Role and name:** e.g. a `button` with `aria-pressed`; the accessible name is the label plus the code.
- **Keyboard:** which keys do what.
- **Tooltip id:** the key it will use in `app/src/help/tooltips.ts` (wired at integration, not here).

## Stories (Story station)

One row per story. Every state in Visual rules has at least one story. Every story renders in dark and light (the toolbar theme), so there's no separate light story.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Off` | `{ label: 'Metronome' }` | the off face: `--btn` fill, grey label | `Off-{dark,light}.png` | — |
| `On` | `{ label: 'Accomp', code: 'ACMP', on: true }` | lime face, black label, small code | `On-{dark,light}.png` | — |
| `Toggles` | `{ label: 'Unison' }` | — | — | click → `aria-pressed` is `true` and `ontoggle` was called with `true`; press Space → `false` |
| `Disabled` | `{ label: 'Manual Bass', disabled: true }` | dimmed label, no press | `Disabled-{dark,light}.png` | click → `ontoggle` not called |
| `LongLabel` | `{ label: 'Retrigger rate' }` | the longest real label fits without wrapping | — | — |

Rules for the table:
- **Args** use real labels and values from the boards, never "Lorem" or "Button".
- **Play** is written as plain steps and expectations; the Story station turns them into a `play` function.
- **Controls:** every prop is a control (complex components: plus each changed child prop, grouped by child). Callbacks are actions. See [storybook-axioms.md](storybook-axioms.md).
- Add a story for each edge the boards show: the longest label, an empty value, the extremes of a range.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes.
- Each cropped story's screenshot matches its crop (diff score under the threshold, or the Inspect agent judges any difference to be render noise).
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules.
- svelte-check and lint pass on the folder.
```

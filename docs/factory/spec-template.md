# Component spec template

Every component in the library gets one spec at `app/src/ui/<Name>/SPEC.md`. The spec is the factory's raw material: the line never invents anything the spec doesn't say. Each station reads only the sections marked for it, so a section must stand on its own.

Board crops live beside the spec in `app/src/ui/<Name>/crops/`, one PNG per story and theme (`<story>-dark.png`, `<story>-light.png`), cut from the Push canvas renders (`../yahaha-research/push-canvas/render/`, 1440×900 at 1x). The spec gives each crop's box on its board, so anyone can cut it again. They are the pictures the Inspect station compares against.

The examples below are LampButton's real ones (`app/src/ui/LampButton/SPEC.md` is the worked example). Copy everything below the line.

---

```markdown
# <Name>

## Identity (all stations)

- **Kind:** primitive | complex
- **Built from:** — | <Name>, <Name> (complex only; these must already be merged)
- **Purpose:** one sentence, in the player's words. ("Turns a part, a mode or a function on or off.")
- **Boards:** where it appears on the canvas, as `<board file>:<lines>`, at least one dark and one light. Check the line numbers in the board file itself; KIT-DEBATE.md's ranges are stale.
- **Not this component's job:** what it must not do. (For every component: no store, no API, no Tauri. Data comes in as props, actions go out as callbacks.)

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component (it becomes the Storybook docs and control description).

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `label` | `string` | — | The word on the face. |
| `on` | `boolean` | `false` | Lit (lamp face) or off. |
| `code` | `string \| undefined` | — | Small code after the label, e.g. `ACMP`. |
| `disabled` | `boolean` | `false` | Shown, not pressable. |
| `size` | `'md' \| 'sm' \| 'cell'` | `'md'` | Size variants, one per size the boards draw: `md` 32px tall, 14px label (section row); `sm` 28px, 13px (settings rows); `cell` 32px, fills its container (the band). |
| `width` | `number \| undefined` | — | A fixed width in px, where the boards fix one (settings rows' 64px On/Off). |
| `name` | `string \| undefined` | — | Accessible-name override, where the visible label isn't enough (`Right 1 on` for a part's "On"). Default: the label plus the code. |

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
- **Size:** per size variant: fixed (w × h) or fills its container; min/max where it matters.
- **States drawn by:** what changes between states (face fill, label colour, outline, glow). One line per state, including keyboard focus (a `--focus` outline).
- **Type:** size, weight, case, tabular numerals or not.
- **Contrast:** each text token on each surface token it's drawn on, so `app/src/ui/tokens/contrast.test.ts` can check it (AA 4.5:1; disabled text is exempt).
- **Motion:** e.g. "queued pads flash at the beat", driven by a prop (a beat or a flash phase), never a timer; or none.

### Accessibility

- **Role and name:** e.g. a `button` with `aria-pressed`; the accessible name is the label plus the code, or `name`.
- **Keyboard:** which keys do what.
- **Tooltip id:** the key it will use in `app/src/help/tooltips.ts` (wired at integration, not here).

## Stories (Story station)

- **Title:** `Primitives/<Name>` (or `Components/<Name>`, `Pages/<Name>`).
- **Layout:** `centered` (real size) unless a row says otherwise; a page is `fullscreen` at 1440×900.

One row per story. Every state in Visual rules has at least one story, and every size variant one. Every story renders in dark and light (the toolbar theme), so there's no separate light story. A crop's box is `<board> x,y w×h` in the render, the same box in the dark and the light board.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Off` | `{ label: 'Metronome' }` | the off face: `--btn` fill, grey label | `Off-{dark,light}.png`: Stage 1114,68 107×32 | — |
| `On` | `{ label: 'Accomp', code: 'ACMP', on: true }` | lime face, ink label, small code | `On-{dark,light}.png`: Stage 24,68 127×32 | — |
| `Toggles` | `{ label: 'Unison' }` | — | — | click → `aria-pressed` is `true` and `ontoggle` was called with `true`; press Space → `false` |
| `Disabled` | `{ label: 'Off', size: 'sm', width: 64, disabled: true, name: 'Manual Bass, works with Upper on' }` | dimmed label, no press | `Disabled-{dark,light}.png`: SettingsChord 858,234 64×28 | click → `ontoggle` not called |
| `PartOn` | `{ label: 'On', size: 'cell', on: true, name: 'Right 1 on' }`; layout `padded` | the `cell` size filling its container | — (fractional cell widths) | the button named `Right 1 on` is pressed |
| `LongLabel` | `{ label: 'Port sends mapped', size: 'sm', on: true }` | the longest real label fits without wrapping | — | — |
| `Focused` | `{ label: 'Metronome' }`; `parameters: { pseudo: { focusVisible: true } }` | the keyboard focus ring (pseudo-states addon) | — | — |

Rules for the table:
- **Args** use real labels and values from the boards, never "Lorem" or "Button".
- **Play** is written as plain steps and expectations; the Story station turns them into a `play` function.
- **Controls:** every prop is a control (complex components: plus each changed child prop, grouped by child). Callbacks are actions. See [storybook-axioms.md](storybook-axioms.md).
- **Focus:** every focusable component has a `Focused` story (pseudo-states, not story-only CSS).
- **Crops:** only where the board draws the story at whole-pixel size; say why when a story has none.
- Add a story for each edge the boards show: the longest label, an empty value, the extremes of a range.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- <Name>` passes: each cropped story's screenshot is the crop's size and scores at most 0.02 (the share of differing pixels), and axe (colour contrast included) finds no violation on any story. Above 0.02, the Inspect agent may still judge the difference render noise and say so.
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules.
- svelte-check and lint pass on the folder.
```

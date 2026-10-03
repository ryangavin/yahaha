# StageArt

## Identity (all stations)

- **Kind:** primitive
- **Built from:** —
- **Purpose:** The quiet picture behind the right of the Stage display: a soft violet glow over a dark horizon that sets the mood without competing with the chord or the section.
- **Boards:**
  - `Stage-Dark.dc.html:124-125` (the art layer and the fade layer over it); light: `Stage-Light.dc.html:100-101`. The token values are the same board's `:root` (kit › Tokens to add, "The art tokens"). Spec: Stage.md › Display › Art, D9.
- **Not this component's job:** no store, no API, no Tauri. It draws nothing from state: it doesn't follow the style, its key or its genre (Stage D9, DECISIONS S4). Not a control: no click, focus or tooltip. It doesn't place itself in the display beyond hugging the right edge: the Display is its `position: relative` container and draws everything else over it.

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `width` | `string` | `'38%'` | The art's width, a CSS length or percentage of its positioned container. The Display passes nothing (the 38% of its 1390px inner box, Stage › Display); the stories pass `'100%'` inside a 528 × 298 frame (D2). |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| — | | |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Visual rules

- **Tokens used:** `--stage-art`, `--stage-art-fade` (both new, below).

#### New tokens

Not in `app/src/ui/tokens/*` today; they land in `dark.css` and `light.css` with the orchestrator's tokens contract PR before the build (L1). They are literal colours on purpose: the art is a picture, not a role (kit › Tokens, D9). Values exactly as kit › Tokens to add gives them:

| Token | Dark | Light | Used for |
|---|---|---|---|
| `--stage-art` | `radial-gradient(circle at 62% 44%, #4a3b86 0, #3d3070 3%, rgba(165,140,255,0.2) 7%, rgba(165,140,255,0.06) 18%, rgba(165,140,255,0) 34%), radial-gradient(ellipse 80% 44% at 40% 104%, #06050a 0, #06050a 58%, rgba(0,0,0,0) 60%), radial-gradient(ellipse 70% 38% at 92% 100%, #0e0b16 0, #0e0b16 62%, rgba(0,0,0,0) 64%), linear-gradient(180deg, #000 0%, #0b0814 55%, #1c1033 100%)` | `radial-gradient(circle at 62% 44%, #d9cff5 0, #c9bcf0 4%, rgba(155,130,240,0.2) 9%, rgba(155,130,240,0.06) 20%, rgba(155,130,240,0) 34%), radial-gradient(ellipse 80% 44% at 40% 104%, #dcd6ea 0, #dcd6ea 58%, rgba(220,214,234,0) 60%), radial-gradient(ellipse 70% 38% at 92% 100%, #e6e0ee 0, #e6e0ee 62%, rgba(230,224,238,0) 64%), linear-gradient(180deg, #f2f1ee 0%, #eeeaf6 58%, #e7dff4 100%)` | the art layer: the violet sun and the two horizon shapes |
| `--stage-art-fade` | `linear-gradient(90deg, #000 0%, rgba(0,0,0,0.6) 22%, rgba(0,0,0,0.1) 60%, rgba(0,0,0,0.2) 100%)` | `linear-gradient(90deg, #f2f1ee 0%, rgba(242,241,238,0.6) 22%, rgba(242,241,238,0.1) 60%, rgba(242,241,238,0.2) 100%)` | the fade layer: melts the art's left edge into the ground |

- **Element:** a root `div` (`data-part="art"`): `position: absolute`, `top: 0`, `right: 0`, `bottom: 0`, `width: <width>` (inline), `pointer-events: none`, `aria-hidden="true"`, `z-index: 0` (the Display's content sits above it, D3). Inside it two `div`s, each `position: absolute; inset: 0`: first `data-layer="art"` with `background: var(--stage-art)`, then `data-layer="fade"` over it with `background: var(--stage-art-fade)`.
- **Size:** fills its container's height; `width` of its container's width (38% by default). On the Stage that is `886.8,113 528.2×298`.
- **States drawn by:** one state; the theme changes the picture through the two tokens only.
- **Type:** none (no text).
- **Contrast:** none (no text); the art's core stays near 35% lightness so it never outshines the chord (Stage glance order). No contrast rows.
- **Motion:** none (static, D9).

### Accessibility

- **Role and name:** none: decorative, `aria-hidden="true"` on the root.
- **Keyboard:** none; not focusable.
- **Tooltip id:** none.

## Stories (Story station)

Title `Primitives/StageArt`, `layout: 'centered'`. Every story renders in dark and light (the toolbar theme). Each story has a decorator: a `div` of 528 × 298 with `position: relative` (the art's container; the shot is that frame's box, D2).

**Controls (argTypes):** `width` text.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ width: '100%' }` | the Stage's art: dark, a violet glow at 62% / 44% over two dark horizon shapes, faded into black at the left; light, the pale lavender version faded into the ground | `Board-{dark,light}.png` (Stage 887,113 528×298) | the root has `aria-hidden="true"` and `data-part="art"`; it holds `[data-layer="art"]` then `[data-layer="fade"]`, in that order; no focusable element and no `data-tip` in the story root |
| `Default` | `{}` | the default 38% width, hugging the right edge of the frame (about 201px) | — (not a board size) | the root's inline `style.width` is `38%` |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in the dark and light render.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- StageArt` passes: `Board` scores at most 0.02 against its crop (or the Inspect agent judges the difference render noise, D2), and axe finds no violation.
- Only `--stage-art` and `--stage-art-fade` are used; no gradient or colour literal in the component.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · Static, from tokens.** One gradient pair from tokens, the board's own (Stage D9): no prop for the style, its key or a picture, until per-style artwork exists (Stage Follow-ups).
- **D2 · A `width` prop so the crop is the art's box.** The shot is the story root's first child, so the stories frame the art in a 528 × 298 box and pass `width: '100%'`, while the Display passes nothing and gets Stage › Art's `width: 38%` exactly. The board's box is 886.8 wide 528.2; the crop is the whole-pixel 887,113 528×298, and the 0.2px difference in the gradients' percentages is far under the 0.02 score.
- **D3 · The Display stacks the content above.** StageArt is `position: absolute` with `z-index: 0` and draws first; the Display's content column is later in the DOM (and `position: relative` where it needs to be), so the art stays behind without the art reaching into its parent.
- **D4 · No tooltip props (L3).** Decorative and `aria-hidden`, so it takes neither `tip` nor `tipAction`, and has no `Focused` story.
- **D5 · L1, new tokens.** `--stage-art` and `--stage-art-fade` land in `dark.css` and `light.css` with the tokens contract PR, with the values above (kit › Tokens); this folder edits no token.

Follow-ups: per-style artwork and key colour (DECISIONS S4, Stage Follow-ups).

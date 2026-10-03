# Badge

## Identity (all stations)

- **Kind:** primitive
- **Built from:** —
- **Purpose:** Marks a row with a short fixed fact, such as "Set by rack" where the rack keeps a send's type.
- **Boards:** `Effects-Dark.dc.html:180` (the Chorus row's "Set by rack"); light: `Effects-Light.dc.html:164`.
- **Not this component's job:** no store, no API, no Tauri. Not a control (no click, no focus, no tooltip of its own: the row it sits in carries the meaning in its `aria-label`, Effects.md › Send list). It doesn't decide when it shows (the parent does). Not Rack.md's "Badge" item ("Mine" / "Unsaved", 12 / 500 `--a` text on nothing), which is plain text in RackHead (D2).

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `text` | `string` | — | The words ("Set by rack"). |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| — | | |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Visual rules

- **Tokens used:** `--t2`, `--past`, `--font-sans`, `--text-12`, `--weight-regular`, `--space-6`, and `--lh-20` (20px, new, defined in `app/src/ui/BarRow/SPEC.md` › New tokens; L1).
- **The element:** a `span`, `display: inline-block`, `flex: none`, padding `0 var(--space-6)`, `--text-12` / 400, line-height `--lh-20` (so 20 tall), `--t2` on `--past`, no radius, no border, `white-space: nowrap`. Carries `data-part="badge"`.
- **States drawn by:** one state. It keeps its own colours on a chosen row (Effects.md › Send list: the white row doesn't recolour it).
- **Type:** DM Sans 12 / 400, the text as given.
- **Contrast (AA 4.5:1, `tokens/contrast.test.ts`):** new row: `--t2` on `--past`, "badge text (Badge)": 8.69 dark (#d6d6d6 on #333333), 8.03 light (#2b2b2b on #c4c3bf), both pass.
- **Motion:** none.

### Accessibility

- **Role and name:** none; plain text inside its parent, which names the fact in its own `aria-label` (", type set by the rack").
- **Keyboard:** not focusable.
- **Tooltip id:** none (not a control).

## Stories (Story station)

- **Title:** `Primitives/Badge`.
- **Layout:** `centered`.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ text: 'Set by rack' }` | the grey block with `--t2` text | `Board-{dark,light}.png`: Effects 594,247 74×20 | the text reads "Set by rack"; nothing in the story is focusable |

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- Badge` passes: the cropped story's screenshot is the crop's size and scores at most 0.02, and axe (colour contrast included) finds no violation.
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · The crop box is measured.** Effects.md gives the row (Chorus, `418,237 260×40`) and the Badge's face but not its box; it was read off `docs/design/push/png/Effects-Dark.png`: `594,247 74×20` (right edge at the row's 10px padding, centred in the 40px row). A one-pixel text-width difference is within the screenshot threshold.
- **D2 · Rack's "Badge" is not this.** Rack.md's head row calls its "Mine" / "Unsaved" text a badge, but it is accent text on nothing with no fill; it stays plain text in RackHead rather than a second face here.
- **D3 · No `tip`.** The Badge isn't a control (L3 binds interactive primitives); its meaning is in the row's name and the row's tooltip.
- **D4 · No crop files yet.** The `crops/` folder doesn't exist; the box above is for the crop station.

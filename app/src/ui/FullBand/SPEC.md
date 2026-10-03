# FullBand

## Identity (all stations)

- **Kind:** complex
- **Built from:** FaderBank, KnobBank, PadBank, TransportColumn
- **Purpose:** The whole Launchkey on screen, left to right as the hardware is: faders, knobs over pads, then transport and tempo.
- **Boards:** `Stage-Dark.dc.html:221-389`; light: `Stage-Light.dc.html:197-365`. Crop box: Stage `24,432 1392×368`.
- **Not this component's job:** no store, no API, no Tauri. It only lays out its four children: every value, face, callback and command belongs to a child's spec. It adds no headers, lines, fills, roles or names of its own beyond the layout boxes below.

## API (Component station)

### Props

One prop per section, each the child's full props (callbacks included), spread onto that child unchanged (BAND-LANE convention 9). The types come from the children: `ComponentProps<typeof X>` (`import type { ComponentProps } from 'svelte'`).

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `faders` | `ComponentProps<typeof FaderBank>` | — (required) | Everything FaderBank takes (its SPEC.md › Props and Events). |
| `knobs` | `ComponentProps<typeof KnobBank>` | — (required) | Everything KnobBank takes. |
| `pads` | `ComponentProps<typeof PadBank>` | — (required) | Everything PadBank takes. |
| `transport` | `ComponentProps<typeof TransportColumn>` | — (required) | Everything TransportColumn takes. |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| — | FullBand has none of its own; each child's callbacks arrive inside its prop object and are passed through untouched. | |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### From the state

FullBand maps nothing itself: the wiring builds each prop object from the child's own "From the state" table (FaderBank, KnobBank, PadBank and TransportColumn SPEC.md).

### Children

| Section | Child | Props | Box (Stage, at 1440 × 900) |
|---|---|---|---|
| Faders | `FaderBank` | `{...faders}` | `24,432 654×368` |
| Knobs and Pads column, top | `KnobBank` | `{...knobs}` | `690,432 626×140` |
| Knobs and Pads column, bottom | `PadBank` | `{...pads}` | `690,590 626×186` |
| Transport and tempo | `TransportColumn` | `{...transport}` | `1328,432 88×368` |

### Visual rules

- **Tokens used:** `--space-12` (the section gap). Nothing else: FullBand draws nothing (no fill, border, line or text); the ground shows through.
- **Box:** 1392 × 368, a flex row, `gap: 12px` (`--space-12`), no wrap, items at the top (`align-items: flex-start`). Every child slot is `flex: none`.
- **The three sections**, left to right (x relative to the band; Stage x = 24 + this, y = 432 + this):

  | Section | x | Width | Height | What |
  |---|---|---|---|---|
  | Faders | 0 | 654 | 368 | FaderBank |
  | Knobs and Pads | 666 | 626 | 368 | a column `div`: KnobBank (140 tall) at y 0, PadBank (186 tall) at y 158 (18px below the knobs, `margin-top: 18px` on PadBank's slot), 24px empty below (y 344–368) |
  | Transport and tempo | 1304 | 88 | 368 | TransportColumn |

  Widths: 654 + 12 + 626 + 12 + 88 = 1392. The column `div` is 626 wide, 368 tall, `display: flex; flex-direction: column`, no role.
- **Children's sizes are their own:** each child's spec fixes its box as above; FullBand only places them. If a child's root isn't that size, that child's spec is wrong, not this one.
- **States:** none of its own; every state is a child's (running, layers, pad pages, knob pages…).
- **Type, contrast, motion:** none of its own (the children's).

### Accessibility

- **Role and name:** FullBand's root is a plain `div` with no role or name, as the board draws the band; the 18px column `div` has none either. The landmarks are the children's own roots, each a `section` with an `aria-label` (a `region`), in this order: "Faders" (FaderBank), "Knobs" (KnobBank), "Pads" (PadBank), "Transport and tempo" (TransportColumn).
- **Keyboard (DOM and tab order, kit › Interaction conventions, Stage.md D36):** the DOM order is the reading order and the tab order: FaderBank (header tabs, each strip's fader then its name, the lamp row), KnobBank (▲, ▼, knobs 1–8), PadBank (▲, ▼, pads 1–16), TransportColumn (Start / Stop … Style tempo). FullBand adds no `tabindex` and handles no keys.
- **Tooltip id:** none of its own.

## Stories (Story station)

Title `Components/FullBand`, `layout: 'centered'` (1392 × 368). Every story renders in dark and light.

Args and controls (axioms 3, 7, 12): the four props are object controls, each in its own `table.category` named after its child (`FaderBank`, `KnobBank`, `PadBank`, `TransportColumn`). The `Board` args import the sibling fixtures and add an `fn()` for every callback each child's Events table lists, so every callback is an action:

```ts
import { boardFaderBank } from '../FaderBank/FaderBank.fixtures'
import { boardKnobBank } from '../KnobBank/KnobBank.fixtures'
import { boardPadBank } from '../PadBank/PadBank.fixtures'
import { boardTransport } from '../TransportColumn/TransportColumn.fixtures'

args: {
  faders: { ...boardFaderBank, /* FaderBank's callbacks: fn() */ },
  knobs: { ...boardKnobBank, /* KnobBank's callbacks: fn() */ },
  pads: { ...boardPadBank, /* PadBank's callbacks: fn() */ },
  transport: {
    ...boardTransport,
    onstartstop: fn(), onstop: fn(), onsectionreset: fn(), onfade: fn(),
    onfillup: fn(), onfilldown: fn(), ontempo: fn(), onstyletempo: fn(),
  },
}
```

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | as above (Stage.md › Board fixture) | the band as the Stage board draws it: Panel / Vol faders, the Style knob page, the Sections pads with Main B playing and Main C next, the running transport | `Board-{dark,light}.png` (Stage 24,432 1392×368) | the story has exactly four regions, named in order "Faders", "Knobs", "Pads", "Transport and tempo"; the "Knobs" and "Pads" regions share one parent element, which is not a region; the last focusable element in the story is the button named "Style tempo: back to the tempo the style came with (Scene Launch and Function together)"; clicking it calls `transport.onstyletempo` once; the first focusable element is inside the "Faders" region |

No `Focused` story: FullBand itself isn't focusable (its children have theirs). No other states: they are the children's stories.

## Done when (Inspect station)

- The story exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- FullBand` passes: `Board` is 1392 × 368 and scores at most 0.02 against its crops; axe finds no violation.
- Only `--space-12` is used; no colours, no literal sizes outside the Visual rules.
- svelte-check and lint pass on the folder.

## Decisions

- D1 · FullBand takes one prop object per child and spreads it (BAND-LANE convention 9), typed with `ComponentProps<typeof Child>`, so it never restates a child's props and a child's change needs no change here.
- D2 · The band itself is a plain `div` with no landmark, as the board draws it; the four children's own `section aria-label` roots are the regions, so a screen reader lands on Faders, Knobs, Pads and Transport and tempo directly.
- D3 · Each child renders its own `section aria-label` root (FaderBank "Faders", KnobBank "Knobs", PadBank "Pads", TransportColumn "Transport and tempo"); FullBand doesn't wrap them in further sections.
- D4 · The 18px between knobs and pads is a margin on PadBank's slot in FullBand's column (the board's `margin-top: 18px` on the Pads section), not part of either child, so KnobBank and PadBank keep the boxes their specs crop.
- D5 · Items align to the top and the column leaves its 24px remainder empty at the bottom, as the board does; nothing stretches.
- D6 · Fixtures hold data only; the story adds the `fn()` actions, so fixtures stay importable by the Stage page fixture without Storybook.

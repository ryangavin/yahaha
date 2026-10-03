# BeatBlocks

## Identity (all stations)

- **Kind:** primitive
- **Built from:** —
- **Purpose:** Shows where the band is in the bar: one block per beat, the beat playing lit in the section's colour, the beats already played grey, beat one marked on top.
- **Boards:**
  - `Stage-Dark.dc.html:100-105` (four blocks: beats 1 and 2 past, beat 3 current in Main green with its glow, beat 4 later); light: `Stage-Light.dc.html:76-81`.
  - The same row sits in the section row on every board; the Stage is the one these crops come from.
- **Not this component's job:** no store, no API, no Tauri, no timer. It doesn't know the clock: the parent (CountRow) works out the current beat and passes it as `beat`. It isn't read out by screen readers (it is `aria-hidden`; CountRow's `aria-label` says the beat). Not focusable, no tooltip, no Launchkey mapping.

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `count` | `number` | `4` | Blocks to draw: the beats in a bar, as the clock counts them (quarter notes: 4 in 4/4, 3 in 3/4 and in 6/8). CountRow feeds it from `surface.clock.beatsPerBar`, never `transport.beatsPerBar` (D2). Rounded down and clamped to 1–16 (D5). |
| `beat` | `number \| null` | `null` | The beat playing now, 1-based (1 … `count`). `null` when the band is stopped: every block draws later. A value outside 1 … `count` (or not a whole number after rounding down) draws as `null` (D5). CountRow feeds it from `currentBeat(surface.clock, now, receivedMs)` (`app/src/ui/CountRow/count.ts`). |
| `hue` | `'intro' \| 'main' \| 'ending' \| 'brk' \| 'fill'` | `'main'` | The playing section's hue role (kit hue roles), used only by the current block. CountRow feeds it from `sectionHue(transport.section)`. |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| — | | |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Visual rules

- **Tokens used:** `--past`, `--btn`, `--t`, `--intro`, `--main`, `--ending`, `--brk`, `--fill`, `--dot-glow-mix` (kit › Tokens to add: dark `70%`, light `0%`), `--space-2`, `--space-4`, `--space-6`, `--space-24`.
- **Size:** the root is an inline flex row, `gap: var(--space-4)` (4px), `align-items: center`, no padding, no wrap. Each block is `var(--space-24)` × `var(--space-24)` (24 × 24px), `border-radius: var(--space-2)` (2px, the kit's beat-block radius), no border, `flex: none`. The root's box is `count × 24 + (count − 1) × 4` wide and 24 tall: 108 × 24 at 4 beats, 80 × 24 at 3, 192 × 24 at 7. The current block's glow draws outside that box and doesn't change it.
- **States drawn by** (block *i*, 1-based):
  - **past** (`beat` not null and *i* < `beat`): fill `--past`.
  - **current** (*i* = `beat`): fill `var(--<hue>)` (e.g. `--main`), and the glow `box-shadow: 0 0 6px color-mix(in srgb, var(--<hue>) var(--dot-glow-mix), transparent)` (written with `var(--space-6)` for the 6px blur). In dark that is the hue at 70% alpha, 6px blur; in light `--dot-glow-mix` is `0%`, so the shadow is transparent and nothing draws.
  - **later** (*i* > `beat`, or every block when `beat` is `null`): fill `--btn`. This is the stopped look: all blocks `--btn`, beat one still edged.
  - **downbeat** (block 1, in every state, stopped included): a 2px top edge in `--t`, `box-shadow: inset 0 2px 0 var(--t)` (written `inset 0 var(--space-2) 0 var(--t)`). In dark `--t` is white, in light near-black (#111). When block 1 is also current it carries both shadows, the edge first: `box-shadow: inset 0 2px 0 var(--t), 0 0 6px color-mix(in srgb, var(--<hue>) var(--dot-glow-mix), transparent)`.
  - No other edge, border, outline or text. No hover, press or focus look (not a control); `cursor: default`.
- **Type:** none (no text).
- **Contrast:** no text, so no `tokens/contrast.test.ts` pair. The blocks are decorative and `aria-hidden` (the beat is in CountRow's `aria-label`), so non-text contrast (WCAG 1.4.11) doesn't apply; `--btn` and `--past` on `--g` are deliberately quiet.
- **Motion:** the current block moves when `beat` changes (the wiring passes `now` once per animation frame, CountRow recomputes `beat`). The change is instant: no transition, no animation, no timer (axiom 10).
- **Test hooks (kit › Faces, D41):** each block is a `<span>` carrying `data-beat="past|current|later"`; block 1 also carries `data-downbeat` (present, empty value; absent on every other block); the current block carries `data-hue="<hue>"` (`main`, `intro`, …) and no other block has a `data-hue`. The glow, the edge and the fills are checked by the stories' screenshots, never by vitest (jsdom resolves neither custom properties nor `color-mix`).

### Accessibility

- **Role and name:** none. The root carries `aria-hidden="true"`; the blocks are presentation. Whoever shows BeatBlocks says the beat in words (CountRow: "Beat 3 of 4, …").
- **Keyboard:** not focusable; no keys.
- **Tooltip id:** none of its own; it sits inside CountRow, whose root carries `display.position` (wired at integration).
- **Launchkey:** none (a readout).

## Stories (Story station)

Title `Primitives/BeatBlocks`, `layout: 'centered'` (real size). Every story renders in dark and light (the toolbar theme). Not focusable, so no `Focused` story. Controls: `count` (number, 1–16), `beat` (number or null), `hue` (select of the five hue roles).

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ count: 4, beat: 3, hue: 'main' }` | the board moment (Stage.md › Board fixture: beat 3 of 4 in Main B): blocks 1–2 `--past`, block 1 with the white (light: #111) top edge, block 3 Main green with its 6px glow in dark and none in light, block 4 `--btn`; 108 × 24 | `Board-{dark,light}.png` (Stage 373,72 108×24) | four elements with `data-beat`; in order `past`, `past`, `current`, `later`; block 1 has `data-downbeat` and no other block does; block 3 has `data-hue="main"`, no other block has `data-hue`; the root has `aria-hidden="true"` |
| `Stopped` | `{ count: 4, beat: null }` | every block `--btn`, block 1 still edged, no glow | — (the Stage board is drawn playing) | four blocks, every one `data-beat="later"`; block 1 has `data-downbeat`; no element has `data-hue` |
| `Downbeat` | `{ count: 4, beat: 1, hue: 'main' }` | block 1 is current: Main green with the edge and the glow both, the edge on top; blocks 2–4 `--btn` | — | block 1 has `data-beat="current"`, `data-downbeat` and `data-hue="main"`; blocks 2–4 `later` |
| `LastBeat` | `{ count: 4, beat: 4, hue: 'main' }` | blocks 1–3 past, block 4 current | — | `data-beat` reads `past`, `past`, `past`, `current` |
| `ThreeFour` | `{ count: 3, beat: 2, hue: 'main' }` | 3/4 (and 6/8, which the clock counts as 3 quarter-note beats, D2): three blocks, 80 × 24 | — | three elements with `data-beat`: `past`, `current`, `later` |
| `Intro` | `{ count: 4, beat: 2, hue: 'intro' }` | the current block in `--intro` with an intro-gold glow | — | block 2 has `data-hue="intro"` |
| `Ending` | `{ count: 4, beat: 2, hue: 'ending' }` | the current block in `--ending` with its glow | — | block 2 has `data-hue="ending"` |
| `Break` | `{ count: 4, beat: 2, hue: 'brk' }` | the current block in `--brk` with its glow | — | block 2 has `data-hue="brk"` |
| `Fill` | `{ count: 4, beat: 2, hue: 'fill' }` | the current block in `--fill` with its glow | — | block 2 has `data-hue="fill"` |
| `SevenFour` | `{ count: 7, beat: 5, hue: 'main' }` | the long edge: seven blocks on one row, 192 × 24, no wrap | — | seven elements with `data-beat`; blocks 1–4 `past`, 5 `current`, 6–7 `later` |
| `OutOfRange` | `{ count: 4, beat: 6, hue: 'main' }` | a beat past the bar draws as stopped (D5) | — | every block `data-beat="later"`; no `data-hue` |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in the dark and light render. Only `Board` has a crop: the Stage board draws only that moment. The `Board` crop ends at the blocks' box, so the part of block 3's glow that spills 2–3px outside it is in neither the crop nor the story's shot (the shot is the story root's box), and they match. The hue stories have no crop; their glow and fill are judged by Inspect against the Board crop's Main look.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- BeatBlocks` passes: the `Board` screenshot is 108 × 24 and scores at most 0.02 against `crops/Board-{dark,light}.png`, or the Inspect agent judges the difference render noise; axe finds no violation on any story.
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules (the 6px glow blur is `var(--space-6)`, the 2px edge and radius `var(--space-2)`).
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · Props, not a clock.** BeatBlocks takes the current beat (`beat`, 1-based, `null` stopped) and the hue; the clock maths (`currentBeat`) lives in `app/src/ui/CountRow/count.ts`, so this primitive has no maths to test and any parent can drive it.
- **D2 · Beats are quarter notes.** `count` comes from `surface.clock.beatsPerBar` (quarter notes per bar: 3 in 6/8), not `transport.beatsPerBar` (the time signature's numerator: 6 in 6/8), because the section clock's position counts quarter notes; 6/8 therefore shows three blocks, one per quarter note, the same as 3/4.
- **D3 · Sizes from existing scale tokens.** The 24px block, 4px gap and 2px radius and edge use `--space-24`, `--space-4` and `--space-2` (and the 6px glow `--space-6`), so no new scale token is needed; the kit's single `--radius` (4px) is not the beat blocks' radius.
- **D4 · Glow per hue.** The current block's glow is built from its hue with `--dot-glow-mix` (Stage.md D29), never from `--bg`; in Main it equals the board's `--bg`, and other sections glow in their own colour.
- **D5 · Out-of-range input.** `count` is rounded down and clamped to 1–16 (no real metre has more than 16 quarter notes a bar); a `beat` outside 1 … `count` draws the stopped look rather than lighting no block or the wrong one, so a bad clock read can never show a stale "current".
- **D6 · Hooks only on meaning.** `data-hue` is on the current block only (the past and later fills are fixed roles carried by `data-beat`), and `data-downbeat` is a bare attribute, so the play steps read `data-beat` order, one `data-downbeat` and one `data-hue`.
- **D7 · Not read out.** The root is `aria-hidden`; the words live in CountRow's single `role="status"` label (kit › Count row), so the beat isn't announced twice.

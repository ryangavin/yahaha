# LibraryFrame

## Identity (all stations)

- **Kind:** complex
- **Built from:** CompactNowPlaying, LibraryPages, QuickRacksBar
- **Purpose:** The left column every Library page keeps: what's playing now, the list of Library pages, and the Quick Racks, so the player can switch pages or racks without leaving the Library.
- **Boards:** `Browser-Dark.dc.html:127-171` (the column: compact block `:129-140`, Library pages `:142-149`, Quick Racks `:151-168`); light: `Browser-Light.dc.html:111-155`. Specified in `docs/specs/push/Browser.md` › Kit additions › Library frame, and, for the compact block, `docs/specs/push/Channel.md` › Page › Compact block and Kit additions › Compact now-playing block (canonical, kit.md line 11 and CH-D1; D1).
- **Not this component's job:** no store, no API, no Tauri. It doesn't draw the 1px hairline to its right or place the page content (the page does: Browser draws `392,128 1×436` and the content box `417,128 975×436`, D2), doesn't hold `clearArmed` (the page does), doesn't handle Esc, and doesn't host the other Library pages (BR-D18: the page's `content` snippet). Everything it draws comes from its three children; it only stacks them and routes their callbacks.

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `nowPlaying` | `NowPlaying` | — | CompactNowPlaying's data props, passed through unchanged (D3). `NowPlaying` is exported from this folder as `Omit<ComponentProps<typeof CompactNowPlaying>, <its open callback> \| 'tipAction'>`. |
| `page` | `LibraryPage` | — | The Library page shown (LibraryPages' type: `'styles' \| 'sounds' \| 'instruments' \| 'racks' \| 'map'`). |
| `counts` | `{ styles: number; sounds: number; instruments: number; racks: number }` | — | LibraryPages' counts (`library.count`, `soundLibrary.patches.length`, `plugins.list.length`, `racks.length`). |
| `quickRacks` | `QuickRacksState` | — | For QuickRacksBar (types.ts). |
| `clearArmed` | `boolean` | `false` | For QuickRacksBar (app-only Clear arm, BR-D19). |
| `led` | `number` | `0` | The LED clock in beats, for QuickRacksBar's flash. |
| `width` | `number \| undefined` | — | A fixed width in px (320), inline `width: <n>px`; default fills its container. |
| `prompt` | `Snippet \| undefined` | — | For QuickRacksBar's `prompt` (the interim `RackPrompt`, BR-D27). |
| `tipAction` | `Action<HTMLElement, string> \| undefined` | — | The app's `use:tip`, passed to all three children (L3). |

What `nowPlaying` holds is CompactNowPlaying's spec (`app/src/ui/CompactNowPlaying/SPEC.md`, written separately); its names win. From Channel.md › Compact block it carries, at the time of writing: the style name (`style.name`, or the stem of `style.path` when empty), the tempo (`transport.tempo`, rounded by the block), the run state (`running` / `sync` / `stopped` from `transport.running` and `transport.syncStart`), the chord (its name, split into runs by `splitChord`, or none), the chord's tones, and the playing section with its hue (stopped: the Main to start on, drawn in `--m`, Stage D4).

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onchoose` | a Library pages row is pressed; or the compact block's style name is pressed (only on pages other than Styles), which calls `onchoose('styles')` (D1, D5) | `(page: LibraryPage)` |
| `onsend` | QuickRacksBar sends (its table) | `(cmd: QuickRackCmd)` |
| `onclearArmed` | QuickRacksBar arms or disarms Clear | `(on: boolean)` |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| `prompt` | Passed to QuickRacksBar unchanged. |

### Children and the props passed

A `div`, `display: flex; flex-direction: column`, width `width` px or 100%, no padding, no background (the page's `--g` shows), `flex: none`:

1. **CompactNowPlaying** `{ ...nowPlaying, tipAction }` and its open callback (the one Channel.md's "click opens the Browser" names) set to `() => onchoose('styles')` when `page !== 'styles'`, and left undefined on Styles, so the name is a plain span there (BR-D24, D1). 320 × 84.
2. **LibraryPages** `{ current: page, counts, tipAction, onchoose }`, `margin-top: var(--space-16)` (on a wrapper or passed as the child's own margin; the box is 160 tall).
3. **QuickRacksBar** `{ quickRacks, clearArmed, led, prompt, tipAction, onsend, onclearArmed }`, `margin-top: var(--space-12)`; 108 tall.

84 + 16 + 160 + 12 + 108 = 380 tall at 1440 × 900 (`48,128 320×380`); the left column's last 56px of its 436 are empty page (D4).

### Visual rules

- **Tokens used:** `--space-12`, `--space-16`. No new tokens and no contrast rows of its own (the children's, L1).
- **States drawn by:** the children only.
- **Motion:** the children's (the Store-armed flash from `led`).

### Accessibility

- **Role and name:** no role of its own; the children's: the compact block's group ("Now playing: …", Channel.md's template), the "Library pages" navigation, the "Quick Racks" region.
- **Keyboard:** tab order: the compact block's style name (only on pages other than Styles), the five Library pages rows, then Quick Racks (◀, ▶, Store, ✕, slots 1–8), as Browser.md › Keyboard lists.
- **Tooltip id:** the children's: `browser.open` on the style name (when a button), `display.chord` and `display.tempo` in the compact block (Channel.md), `library.tab_*` on the pages, `quick.*` in the bar.

## Stories (Story station)

Title `Components/LibraryFrame`, `layout: 'centered'`. Every story renders in dark and light. Fixtures (axiom 12): `app/src/ui/LibraryFrame/LibraryFrame.fixtures.ts` exports `boardNowPlaying: NowPlaying` (the Browser and Channel boards' moment: "Sunday Drive Pop", tempo 104, running, chord "Am7" with tones A C E G, section "Main B" in `main`; taken from CompactNowPlaying's own board fixture when its spec exports one), and re-exports `boardCounts` (LibraryPages) and `boardQuickRacks` (QuickRacksBar). Meta `args`: `{ nowPlaying: boardNowPlaying, page: 'styles', counts: boardCounts, quickRacks: boardQuickRacks, clearArmed: false, led: 0.25, width: 320, onchoose: fn(), onsend: fn(), onclearArmed: fn(), tipAction: fn() }`.

**Controls (argTypes, axiom 3):** `page` a select of the five pages and `width` a number (default category); `nowPlaying` an object control under `table.category: 'CompactNowPlaying'`, with its run state and section broken out as selects in the same category; `counts` under `LibraryPages`; `quickRacks`, `clearArmed`, `led` under `QuickRacksBar`; the callbacks and `tipAction` actions. `prompt` gets no argType (a snippet).

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | — (the meta's) | the Browser board's left column: the compact block running, Styles chosen, bank A with A1 loaded | `Board-{dark,light}.png` (Browser 48,128 320×380) | a group whose name starts "Now playing: Sunday Drive Pop"; no button named "Sunday Drive Pop: open the Browser" (the name is a span on Styles); a navigation "Library pages" whose first button "Styles, 1,284" has `aria-current="page"`; a region "Quick Racks"; click "Sounds, 886" → `onchoose` called once with `'sounds'`; click "Quick Rack A2, Warm keys" → `onsend` called with `{ type: 'pressQuickRack', slot: 1 }`; click the ✕ → `onclearArmed(true)` |
| `OnSounds` | `{ page: 'sounds' }` | Sounds chosen; the style name is a button | — (no board draws this frame off Styles, LibraryPages D3) | "Sounds, 886" has `aria-current="page"`; click the button named "Sunday Drive Pop: open the Browser" → `onchoose` called once with `'styles'` |
| `Stopped` | `{ nowPlaying: { …boardNowPlaying, <run state stopped, section the Main to start on> } }` | the run dot hidden (space kept), the section in `--m` | — | the group's name contains "Stopped" |
| `ClearArmed` | `{ clearArmed: true }` | the ✕ and A1–A4 in `--ending` | — | click A3 → `onsend` called with `{ type: 'clearQuickRack', bank: 0, slot: 2 }`, then `onclearArmed(false)` |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in dark and light (L6). A1's glow, left of x 48, is outside this crop too; the page shot judges it. The `Board` crop includes every child, so CompactNowPlaying, LibraryPages and QuickRacksBar must be built first.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- LibraryFrame` passes for `Board` (score at most 0.02, or the Inspect agent judges the difference render noise), and axe finds no violation on any story once the tokens contract PR has landed.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · The compact block is Channel's (CH-D1), with Browser's one exception.** Where Browser.md › Kit additions › Compact block and Channel.md › Compact block differ, Channel wins: the block has no tooltip of its own (Browser's `display.compact`, new in Browser C1, isn't used; the chord and tempo carry `display.chord` and `display.tempo`), its `aria-label` is Channel's "Now playing: …" template, and the chord never shrinks (CH-D2, CompactNowPlaying's rule). Kept from Browser (BR-D24): on Library › Styles the style name is a span, since it would open the page already shown; that needs CompactNowPlaying to draw the name as a span when its open callback is undefined.
- **D2 · The frame is the column only.** LibraryFrame is the 320-wide left column; the hairline and the page content are the page's (Browser's Layout lists them as the page's parts), so the crop is the column's own box and the same frame sits beside any Library page content.
- **D3 · `nowPlaying` is a pass-through.** The frame takes CompactNowPlaying's data props as one object typed from that component, so the two specs can't drift and this spec doesn't restate the block (written by another worker).
- **D4 · Height is its content.** The column's box is 380 tall (84 + 16 + 160 + 12 + 108); the page's left area is 436, and the 56px below the bar are empty page, not part of the frame, so the crop is `48,128 320×380`.
- **D5 · One navigation callback.** The style name opens Library › Styles (the Browser is that page, BR-D1), so it reuses `onchoose('styles')` rather than adding a callback.
- **D6 · L3.** `tipAction` goes to all three children; every key they use exists except `library.tab_styles` (contract change needed, Browser.md C1).
- **D7 · No crop files yet.** The folder holds only this spec; the crop is cut later from the Browser renders (L6).

Follow-ups: when CompactNowPlaying's spec lands, check that its open callback is optional (D1) and that `NowPlaying`'s field list above matches it; drop `display.compact` from Browser.md C1 or give it a use.

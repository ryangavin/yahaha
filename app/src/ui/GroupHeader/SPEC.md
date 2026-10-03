# GroupHeader

## Identity (all stations)

- **Kind:** primitive
- **Built from:** —
- **Purpose:** The hairline row that names a group of controls ("Faders", "Knobs", "Pads", "Transport", "Tempo") and holds the group's page tabs or page counter.
- **Boards:**
  - `Stage-Dark.dc.html:224` (Faders, with the fader page and layer tabs), `:292` (Knobs, with the page block and "Page 1/6"), `:320` (Pads, with "Sections", the legend and "Bank 1/5"), `:361` (Transport), `:379` (Tempo, 20px under the transport buttons); light: `Stage-Light.dc.html:200`, `:268`, `:296`, `:337`, `:355`.
  - `Effects-Dark.dc.html:262` (Faders in the Reverb layer: "Faders · Reverb"); light: `Effects-Light.dc.html:246`.
  - The kit's rule: kit › Spacing and shape (36 tall, hairline bottom, content 8px below).
- **Not this component's job:** no store, no API, no Tauri. It draws the row, the title and the counter; what sits after the title (tabs, the knob page block, the pad page name and legend) is the parent's, passed as a snippet. It doesn't draw the 8px below it or the margin above it (the parent's column does), and it has no click of its own.

Its users (each passes its own content):

| User | `title` | `detail` | `children` | `count` |
|---|---|---|---|---|
| FaderBank | "Faders" | the layer word when `mixer.faderLayer` isn't `volume` ("Reverb") | ChosenTabs (fader page), the separator, "Layer" and ChosenTabs (layer) | — |
| KnobBank | "Knobs" | — | AccentBlock (`knobs.pageName`) | `{ label: 'Page', value: '1/6' }` |
| PadBank | "Pads" | — | `pads.pageName`, the legend on Sections | `{ label: 'Bank', value: '1/5' }` |
| TransportColumn | "Transport", then a second GroupHeader "Tempo" | — | — | — |

## API (Component station)

### Props

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `title` | `string` | — | The group's name, the heading's text ("Faders"). |
| `detail` | `string \| undefined` | — | A word that qualifies the title, drawn after it as " · {detail}" with the word in `--t` ("Faders · Reverb"). Part of the heading. |
| `count` | `{ label: string; value: string } \| undefined` | — | The page counter at the right end: `label` in `--m`, a space, then `value` in `--t` ("Page 1/6", "Bank 1/5"). |
| `level` | `2 \| 3 \| 4` | `2` | The heading level of the title. |
| `id` | `string \| undefined` | — | The id put on the heading, so the parent's `section` can use `aria-labelledby`. |
| `width` | `number \| undefined` | — | A fixed width in px (the band's 654, 626, 88). Default: fills its container's width. |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| — | | |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| `children` | Whatever follows the title, rendered straight into the row so each top-level element is a flex item 12px after the one before (FaderBank's tabs; KnobBank's page block; PadBank's page name and legend). Optional. |

### Visual rules

- **Tokens used:** `--line`, `--line-width`, `--m`, `--t`, `--font-sans`, `--text-13`, `--text-14`, `--weight-regular`, `--space-12`.
- **Row:** a `div`, height 36 (the kit's header height; no token, D5), `box-sizing: border-box`, `border-bottom: var(--line-width) solid var(--line)` (so the content box is 35 tall), `display: flex`, `align-items: center`, gap `--space-12`, `white-space: nowrap`, no padding, no background, no `overflow` clipping (a tab's focus ring must not be cut, D6). Width: `width` px, else 100% of its container. No `data-face` or `data-hue`: nothing in the row's own drawing comes from state.
- **Order in the row:** the heading; the `children` snippet's elements; then the counter, pushed to the right end with `margin-left: auto`.
- **Heading:** an `h2` (or `h3`, `h4` by `level`), `margin: 0`, `flex: none`, 14px, weight regular, `--m`, `line-height: normal`, with `id` when given. With `detail`: the text " · " in `--m` then `detail` in `--t` inside the same heading, same size and weight ("Faders · Reverb", no gap but the spaces).
- **Counter:** a `span`, `flex: none`, `margin-left: auto`, 13px, weight regular, `--m`, `line-height: normal`: `label`, a space, then `value` in a `span` in `--t`. Absent when `count` is undefined.
- **Children:** drawn as given. A child taller than 35 would overflow the row; the band's tallest are ChosenTabs `size: 'header'` at 35, which then fill the content box and sit on the hairline (their 22px chosen block touches it).
- **States drawn by:** only `detail` and `count` change what's drawn; no hover, focus or pressed look (it isn't a control).
- **Type:** DM Sans, sentence case as given, tabular numerals (the counter's "1/6"); title 14 / 400, counter 13 / 400.
- **Contrast (AA 4.5:1, `tokens/contrast.test.ts`):** `--m` on `--g` (title, " · ", counter label) and `--t` on `--g` (detail, counter value); both pairs exist.
- **Motion:** none.

### Accessibility

- **Role and name:** the title is a real heading (`h2` by default), so a screen reader can jump between the band's groups; its text is the title plus " · {detail}". The row itself has no role. The counter is plain text, read after the children.
- **Keyboard:** not focusable; the children's controls are, in DOM order.
- **Tooltip id:** none; the children carry theirs (`mixer.page`, `mixer.layer`).

## Stories (Story station)

Title `Primitives/GroupHeader`, `layout: 'centered'`. Every story renders in dark and light (the toolbar theme). No `Focused` story: it isn't focusable (a tab inside it is ChosenTabs' to show).

**Snippet args.** Stories pass `children` with `createRawSnippet` (from `svelte`) in `GroupHeader.stories.ts`. Where the board's content is a library component (ChosenTabs, AccentBlock), the snippet's `render` returns a `<span style="display: contents"></span>` host and its `setup(host)` mounts the component into it with `mount(...)` (and returns `unmount`), so the story shows the real component, not a copy (D7). Plain text content is the snippet's own markup with token colours (`style="color: var(--t)"`), exactly the parent's markup given below. The content, per story:

- **Fader header** (`Board`, `FadersLayer`), three items: ChosenTabs `{ size: 'header', label: 'Fader page (master button)', tabs: [{ id: 'panel', label: 'Panel' }, { id: 'style', label: 'Style' }], chosen: 'panel' }`; a separator `<span aria-hidden="true">` 1 × 16, `background: var(--line)`; a `<span style="display: flex; align-items: center">` holding "Layer" (14px, `--m`, `margin-right: 4px`) then ChosenTabs `{ size: 'header', label: 'Fader layer', tabs: [{ id: 'volume', label: 'Vol', name: 'Volume' }, { id: 'pan', label: 'Pan' }, { id: 'reverb', label: 'Reverb', name: 'Reverb send' }, { id: 'chorus', label: 'Chorus', name: 'Chorus send' }, { id: 'delay', label: 'Delay', name: 'Delay send' }], chosen: <'volume' or 'reverb'> }`.
- **Knob header** (`Knobs`): AccentBlock `{ label: 'Style', size: 'knob' }` (the knob page block: padding 0 8, 13px, line-height 22, `--g` on `--a`).
- **Pad header** (`Pads`), two items: "Sections" (14px, `--t`); the legend, a `<span>` with `margin-left: 4px`, `display: flex`, `align-items: center`, gap 12, 12px, holding five `<span style="display: flex; align-items: center; gap: 6px; color: var(--<hue>)">` items, each a 10 × 2 bar (`border-radius: 1px`, `background: var(--<hue>)`, `aria-hidden`) and the word: Intro `--intro`, Main `--main`, Ending `--ending`, Break `--brk`, Fill `--fill`.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ title: 'Faders', width: 654, children: <fader header, chosen 'volume'> }` | the Faders header: title, Panel chosen, the separator, "Layer" and Vol chosen, over the hairline | `Board-{dark,light}.png` (Stage 24,432 654×36) | a heading level 2 named "Faders"; two tablists, "Fader page (master button)" and "Fader layer"; no text "Page" or "Bank" |
| `FadersLayer` | `{ title: 'Faders', detail: 'Reverb', width: 654, children: <fader header, chosen 'reverb'> }` | "Faders · Reverb" with Reverb in `--t`, and the Reverb tab chosen | `FadersLayer-{dark,light}.png` (Effects 24,432 654×36) | a heading level 2 named "Faders · Reverb" |
| `Knobs` | `{ title: 'Knobs', width: 626, count: { label: 'Page', value: '1/6' }, children: <knob header> }` | Knobs, the violet "Style" block, "Page 1/6" at the right end | `Knobs-{dark,light}.png` (Stage 690,432 626×36) | a heading named "Knobs"; the text "Page 1/6" is in the canvas, with "1/6" in its own element |
| `Pads` | `{ title: 'Pads', width: 626, count: { label: 'Bank', value: '1/5' }, children: <pad header> }` | Pads, "Sections", the five-hue legend, "Bank 1/5" | `Pads-{dark,light}.png` (Stage 690,590 626×36) | a heading named "Pads"; the text "Bank 1/5" |
| `Transport` | `{ title: 'Transport', width: 88 }` | the title alone over the hairline | `Transport-{dark,light}.png` (Stage 1328,432 88×36) | a heading named "Transport"; the row has no other text |
| `Tempo` | `{ title: 'Tempo', width: 88 }` | the second header of the transport column | `Tempo-{dark,light}.png` (Stage 1328,642 88×36) | a heading named "Tempo" |
| `Level3` | `{ title: 'Transport', level: 3, id: 'band-transport', width: 88 }` | the same look at another heading level | — (same pixels as `Transport`) | a heading level 3 named "Transport" with `id="band-transport"` |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in the dark and light render. Every cropped story's screenshot includes its children, so `npm run shots -- GroupHeader` also compares the ChosenTabs and AccentBlock it mounts; those must be built first (Stage's build order has them before GroupHeader).

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- GroupHeader` passes for the six cropped stories (score at most 0.02, or the Inspect agent judges any difference to be render noise).
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules (36).
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · One snippet, two props.** What every header has (title, the " · " detail, the right-hand page counter) is props; what differs per group (tabs, page block, page name, legend) is the one `children` snippet, so no header needs a second slot.
- **D2 · The title is a heading.** It is an `h2` by default (`level` changes it, `id` lets the parent's `section` use `aria-labelledby`), so the band's groups are headings to a screen reader; the board's plain `span` gave no structure.
- **D3 · Centre, not stretch.** The row always uses `align-items: center`; the board's Faders row uses `stretch`, but its 35-tall tabs fill the 35px content box either way, so the pixels are the same and one rule covers every header.
- **D4 · Children are flex items.** The snippet renders straight into the row, so each of its top-level elements gets the 12px gap, as the board's Faders, Knobs and Pads rows space their items.
- **D5 · No height token.** The 36px header height is a literal in the Visual rules, as kit › Spacing and shape states it; `scale.css` has no token for it and only this component uses it.
- **D6 · No clipping.** The row doesn't hide overflow, so a focused tab's ring (2px outside the tab) isn't cut at the hairline; the band's widths are fixed and the content fits.
- **D7 · Stories mount the real children.** The cropped stories mount ChosenTabs and AccentBlock into `display: contents` hosts from `createRawSnippet`, since the crops show them and copying their markup would test a fake; the legend and "Sections" are PadBank's own markup, copied as given above.
- **D8 · Counter as a prop.** "Page 1/6" and "Bank 1/5" are one `count` prop (13px, label `--m`, value `--t`, at the right end) rather than snippet content, since both banks draw it identically.

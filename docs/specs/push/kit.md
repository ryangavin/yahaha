# Push kit

The shared parts every Push screen is built from: tokens, the four faces, the hue roles, and the
components that appear on more than one board (app bar, section row and its count row, the full
band of faders, knobs, pads and transport, the key strip, the status line). Defined once here; a
screen spec names a part (`kit.md` › Pad) and says only what its screen does differently.

- **Source boards:** `docs/design/push/Stage-Dark.dc.html` and `Stage-Light.dc.html` (#500). The
  light board's markup matches the dark one except for the art layers; every other theme
  difference is a token.
- **Variants owned elsewhere:** the page variant of the app bar (adds the rack readout and One
  Touch), the half band of the tall pages and the compact now-playing block are drawn on
  `Channel-Dark.dc.html`; the Channel spec (#501) adds them to this file.
- **Read with:** [Stage.md](Stage.md), which binds the kit to the Stage screen and holds the
  decisions behind the rules below (each "Decision" there is numbered D1, D2…).
- **Geometry** is in CSS px at the 1440 × 900 design size. Boxes are `x,y w×h` from the
  screen's top-left. How the whole layout scales to other window sizes: Stage.md D1.

## Tokens

Components read only semantic tokens. They exist in `app/src/ui/tokens/` (from #499: `palette.css`
named colours, `dark.css` and `light.css` roles, `scale.css` sizes); the theme is
`data-theme="light"` on the root, dark otherwise. Where the tokens and the board disagree, the
tokens win: light `--m` is `#646464` (the board's `#6e6e6e` fails AA on `--btn`), light
`--lamp-ink` is `#000` and the lamp's small code is full ink in light (`--code-opacity: 1`).

### Colour roles (exist)

| Token | Dark | Light | Role |
|---|---|---|---|
| `--g` | #000 | #f2f1ee | ground; also the ink on accent and white blocks |
| `--t` | #fff | #111 | text; the white "chosen" block |
| `--t2` | #d6d6d6 | #2b2b2b | button labels, secondary text |
| `--m` | #8c8c8c | #646464 | muted: labels, units, off lamp label |
| `--d` | #4d4d4d | #b0afab | dimmed: absent, unused, disabled |
| `--btn` | #161616 | #e4e3df | the plain button face |
| `--line` | #262626 | #d3d2ce | hairlines, key-strip frame |
| `--past` | #333 | #c4c3bf | past beat blocks |
| `--mbg` | #141414 | #e2e1dd | meter background, unused knob ring |
| `--keyline` | #000 | #d3d2ce | white-key separator |
| `--util` | #5a5a5a | #a8a7a3 | utility pad group line |
| `--focus` | #fff | #111 | focus ring |
| `--lamp`, `--lamp-ink` | #9fe04a, #000 | #4f8a0e, #000 | switched on |
| `--solid-ink` | #000 | #fff | label on any solid hue block (playing pad, held key, rec lamp) |
| `--rec` | #e5534b | #b8322c | record |
| `--warn` | #ff9a2e | #c85f00 | missing (⚠) |
| `--ok` | #4fd66a | #1c8040 | connected, running |
| `--a` | #a58cff | #5b3fd6 | the accent: style name, chord, knob arcs and values, band sends |
| `--r1 --r2 --r3 --l` | #3b8eff #ff4f9e #ff9a2e #16c7a6 | #1257d6 #d0186f #c85f00 #008f78 | part hues |
| `--intro --main --ending --brk --fill` | #bfb24e #4fd66a #c45a5a #8f62a8 #7a9ea6 | #857a1f #1c8040 #a84444 #7a4a96 #4d7480 | section hues |
| `--bg` | `0 0 6px` `--green-400` at 70% | `none` | the green status glow: Launchkey dot, Running dot, Start / Stop bar |
| `--ba` | `0 0 18px` `--violet-300` at 28% | `none` | the compact chord on the tall pages (Channel spec #501); nothing on the Stage |
| `--ba2` | `0 0 28px` `--violet-300` at 28% | `none` | the Stage chord (`text-shadow`) |
| `--bl` | `0 0 6px` `--teal-400` at 60% | `none` | the key strip's detection line in the left hand |
| `--bw`, `--bm` | white at 60%; `--green-400` at 30% | `none` | no kit component uses them: `--bm` is the Main instance of the section text glow, which the kit draws per hue with `--text-glow-mix` (D29); keep both for the boards |

**Glows by hue (D29).** A glow in a section's or part's hue is built from the hue and a mix
token, never from `--bg` or `--bm` (those are fixed green): the section dot and the current beat
block `0 0 6px color-mix(in srgb, <hue> var(--dot-glow-mix), transparent)`; the playing
section's name `0 0 18px color-mix(in srgb, <hue> var(--text-glow-mix), transparent)`. In Main
these equal the board's `--bg` and `--bm`.

### Tokens to add (this kit needs them; add to `palette.css` and both theme files)

| Token | Dark | Light | Used by |
|---|---|---|---|
| `--track` | `--grey-20` #333 | `--stone-76` #c4c3bf | fader groove |
| `--ring-rest` | new `--grey-24` #3d3d3d | new `--stone-83` #d6d5d1 | knob ring past the value |
| `--peak` | `color-mix(in srgb, var(--white) 85%, transparent)` | `var(--t)` | meter peak tick |
| `--meter-mix` | `75%` | `60%` | meter fill = part hue at this strength |
| `--key-white` | `--grey-09` #161616 | new `--paper-98` #fbfbf9 | white key |
| `--key-white-left` | new `--teal-05` #0d1a17 | new `--teal-95` #e6f1ee | white key at or below the split |
| `--key-black` | `--black` | new `--grey-11` #1b1b1b | black key |
| `--key-black-left` | `--black` | new `--teal-13` #1a2a26 | black key at or below the split |
| `--key-black-ring` | `inset 0 0 0 1px` new `--grey-18` #2e2e2e | `none` | black key edge |
| `--key-black-ring-left` | `inset 0 0 0 1px` new `--teal-17` #1d3a33 | `none` | black key edge, left zone |
| `--key-label` | `--d` | `--m` | C1…C6 on the keys |
| `--pad-index-dark` | `--grey-18` #2e2e2e | `--line` | the numeral of an absent pad |
| `--glow-mix` | `25%` | `0%` | solid pads' 12px glow: `0 0 12px color-mix(in srgb, <hue> var(--glow-mix), transparent)` |
| `--bar-glow-mix` | `60%` | `0%` | a waiting pad's 2px bar glow (6px) |
| `--fill-glow-mix` | `45%` | `0%` | a fader's fill glow (6px) |
| `--key-glow-mix` | `45%` | `0%` | a held key's glow (10px) |
| `--text-glow-mix` | `30%` | `0%` | the playing section's text glow (18px), in its hue |
| `--dot-glow-mix` | `70%` | `0%` | the section dot's and the current beat block's 6px glow, in the section's hue |
| `--armed-ring` | `0px` | `1px` | light only: an armed pad's extra inset ring (no glow in light) |
| `--solid-ink-index` | `55%` | `70%` | a solid pad's numeral: `color-mix(var(--solid-ink) …)` |
| `--solid-ink-bar` | `45%` | `60%` | a solid pad's 2px bar |
| `--stage-art`, `--stage-art-fade` | below | below | Stage display art (Stage.md D9) |

The art tokens, copied from the boards (the art layer and the fade layer over it). Dark:

```css
--stage-art:
  radial-gradient(circle at 62% 44%, #4a3b86 0, #3d3070 3%, rgba(165,140,255,0.2) 7%, rgba(165,140,255,0.06) 18%, rgba(165,140,255,0) 34%),
  radial-gradient(ellipse 80% 44% at 40% 104%, #06050a 0, #06050a 58%, rgba(0,0,0,0) 60%),
  radial-gradient(ellipse 70% 38% at 92% 100%, #0e0b16 0, #0e0b16 62%, rgba(0,0,0,0) 64%),
  linear-gradient(180deg, #000 0%, #0b0814 55%, #1c1033 100%);
--stage-art-fade: linear-gradient(90deg, #000 0%, rgba(0,0,0,0.6) 22%, rgba(0,0,0,0.1) 60%, rgba(0,0,0,0.2) 100%);
```

Light:

```css
--stage-art:
  radial-gradient(circle at 62% 44%, #d9cff5 0, #c9bcf0 4%, rgba(155,130,240,0.2) 9%, rgba(155,130,240,0.06) 20%, rgba(155,130,240,0) 34%),
  radial-gradient(ellipse 80% 44% at 40% 104%, #dcd6ea 0, #dcd6ea 58%, rgba(220,214,234,0) 60%),
  radial-gradient(ellipse 70% 38% at 92% 100%, #e6e0ee 0, #e6e0ee 62%, rgba(230,224,238,0) 64%),
  linear-gradient(180deg, #f2f1ee 0%, #eeeaf6 58%, #e7dff4 100%);
--stage-art-fade: linear-gradient(90deg, #f2f1ee 0%, rgba(242,241,238,0.6) 22%, rgba(242,241,238,0.1) 60%, rgba(242,241,238,0.2) 100%);
```

These are literal colours on purpose: the art is a picture, not a role (D9). They live in
`dark.css` and `light.css` with the other roles.

Scale tokens to add to `scale.css`: `--text-9` (the ▲▼ in Fill buttons), `--text-10` (the ▾ caret),
`--text-128` (the chord), `--weight-thin` exists (200), `--travel: 223px` (fader travel),
`--long-press: 350ms`.

### Type

DM Sans everywhere, `font-variant-numeric: tabular-nums`; JetBrains Mono only for pad numerals and
key labels (11px). Sentence case. Numbers are light (300); names that matter are medium (500);
labels are regular (400). The plain word comes first and the Genos code small after or beneath it
(`Accomp` + `ACMP`, knob `Dynamics` over `DynCtrl`).

| Use | Size / weight | Colour |
|---|---|---|
| Group header ("Faders", "Knobs") | 14 / 400 | `--m` |
| Button label | 14 / 400 (band cells 13) | `--t2` |
| Small caption, unit, code | 12 / 400 | `--m` |
| Count row, wordmark | 18 (count row 300, wordmark 500) | as listed |
| Fader value | 18 / 300 | part hue |
| Knob value | 22 / 300 | `--a` |
| Tempo | 32 / 300, letter-spacing −0.5 | `--t` |
| Next section | 36 / 300, −1 | section hue |
| Playing section | 44 / 300, −1.5 | section hue |
| Chord | 128 / 300, −6 (extension 200, 0) | `--a` |

### Spacing and shape

One radius, `--radius` 4px (beat blocks 2px, black keys `0 0 3px 3px`). Controls are 32px tall
(`--control-height`); square 32 × 32 for icon buttons and One Touch. Groups are separated by a
hairline header row: 36px tall, `border-bottom: 1px solid var(--line)`, content 8px below it. No
boxes, no borders, no shadows except the glows above.

## Faces

Four faces, four meanings. Every clickable thing wears one of them, and a screen never invents a
fifth.

| Face | Drawing | Means | Examples |
|---|---|---|---|
| **Off** | `--btn` fill, `--t2` label (lamps: `--m`) | a plain button, or a switch that is off | Panic, Tempo +, Metronome off |
| **On (lamp)** | `--lamp` fill, `--lamp-ink` label, weight 500; small code at `--code-opacity` | switched on | Accomp, part On, L Hold |
| **Chosen** | `--t` fill, `--g` label, weight 500 | the one picked from a set | active tab, applied One Touch, fader layer |
| **Waiting** | transparent, `1px solid <hue>`, label in the hue (or `--t`) | queued, armed, will happen | next section chip, queued pad |
| **Disabled** | the face it would have, label `--d`, `aria-disabled="true"`, default cursor, stays focusable | not available now | One Touch 4 on a style with three |

Two more faces are drawn only where named: **Record** (lamp with `--rec` fill, `--solid-ink` label:
recording) and **Solid hue** (a pad or key filled with its hue, `--solid-ink` label: playing or
held). The 2px bar survives only inside pads and on Start / Stop. The accent block (`--a` fill,
`--g` label) is not a face: it marks "the device" (the style name, the knob page).

Every focusable control shows a 1px `--focus` outline at 2px offset (`--focus-offset`) on
`:focus-visible`, and nothing on mouse focus.

**Hover, press and cursor (D35).** The boards draw no hover or pressed look, so there is none:
no colour, fill or outline change on `:hover` or `:active`. The cursor is `pointer` on every
enabled control (faced buttons and the text buttons: style name, band sends, rack readout,
sound cells, tags, strip names, health slot, status line) and `default` on disabled controls
and on everything that isn't a control. Faders and knobs use `ns-resize` while dragging.

**Text buttons.** Some controls wear no face: the band sends, the rack readout, the sound cells
and their part tags, the strip names, the health slot and the status line. They are readouts
that open something; they draw as their text only (transparent, no border) and follow the hover
and focus rules above. The four faces are for switches and actions.

**Test hooks (D41).** So that tests read meaning rather than computed colours (jsdom resolves
neither custom properties nor `color-mix`), every element drawn in a face carries
`data-face="off|on|chosen|waiting|disabled|record|solid"`, and every element whose colour is a
hue or a role chosen from state carries `data-hue="<token name without -->"` (`r1`, `main`,
`d`, `ending`…). The visual check of what those mean is the stories' screenshots.

## Hue roles

- **Parts**, saturated and unique: Right 1 `--r1` blue, Right 2 `--r2` pink, Right 3 `--r3`
  orange, Left `--l` teal. A part's name, fader, meter and held keys are in its hue.
- **Sections**, a muted family: Intro `--intro`, Main `--main` (the one green, shared with
  Running on purpose), Ending `--ending`, Break `--brk`, Fill `--fill`. A section's name, pad
  and beat block are in its hue.
- **Accent** `--a`: the style and what it plays (chord, knobs, band sends). The only violet.
- **Utility** grey/white: Sync, Tap, Auto Fill pads use `--t2` labels and a `--util` group line.
- **Trouble:** `--warn` for missing, `--ending` red for failed and audio trouble.
- Section names map to hues by their first word: `Intro *` → intro, `Main *` → main,
  `Ending *` → ending, `Fill In BA` → brk, other `Fill In *` → fill.

## Section names

The state names sections the SFF way (`transport.section`, `queued`, `landing`:
`Intro A`, `Main B`, `Fill In BB`, `Fill In BA`, `Ending C`). The screen shows them the Genos way,
everywhere: `Intro A/B/C/D` → `Intro I/II/III/IV`, `Ending A/B/C/D` → `Ending I/II/III/IV`
(SFF allows a D; the Genos has three, D44), `Main A–D` unchanged, `Fill In BA` → `Break`, any
other `Fill In XY` → `Fill`. A name that matches none of these shows as given. A no-break space ties a name
to its numeral (`Intro I`), so only whole words wrap.

## Interaction conventions

- **Click** is the action. **Long press** is `--long-press` (350 ms) held without moving more
  than 4px; it fires when the time is up, not on release, and the click is then swallowed.
  **Right-click** does what long press does. **Shift-click** is the Launchkey's Shift layer:
  `ui.shift` (Shift held on the computer keyboard, or the latched on-screen Shift where a screen
  has one). Long press is one shared Svelte action, `use:longpress` in
  `app/src/ui/actions/longpress.ts` (pointerdown starts a 350 ms timeout; moving over 4px,
  pointerup or pointercancel before it clears it; it fires `onlongpress` and marks the next
  click swallowed; it calls `onlongrelease` on the pointerup or pointercancel that ends a press
  that fired). That timeout measures a gesture, not motion, so it is allowed (axiom 10 is about
  drawing). LampButton exposes it as `onlongpress` / `onlongrelease` props.
- **Drag** on a fader or knob moves it; **wheel** steps it; **double-click** resets it; arrow
  keys step it when focused (PageUp/PageDown ten steps, Shift for fine on knobs).
- **Tooltips:** every interactive element carries `use:tip={'<key>'}` (`data-tip`), with the key
  in `app/src/help/tooltips.ts`. The kit gives each part's key.
- **Parity:** what the Launchkey does, the screen does with the same command. Where a part's
  state carries an `action` (`pads.pads[i].action`, `surface.controls[i].action`), the screen
  sends exactly that (`app.send(action)`) and is disabled when it is null. With `ui.shift`, a
  control that mirrors a `surface.controls` entry sends its `shiftAction` instead (disabled when
  that is null). Controls bound this way: Pad Bank ▲ ▼ (`padBankUp`, `padBankDown`) and Track ◀ ▶
  (`trackPrev`, `trackNext`; Stage.md › Style line). The other band buttons send their named
  command (tables below), which is what the hardware button sends too.
- **Keyboard (D36):** tab order is the DOM order, which is the reading order: app bar (tabs,
  then health slot when it is a button), section row, display (style line left to right, rack
  readout, then each part's tag and sound), band (fader header tabs, each strip's fader then its
  name, the lamp row, knob ▲ ▼ and knobs, pad ▲ ▼ and pads 1–16, transport, tempo), the status
  line when it has a message. The key strip and every non-control are not focusable. The window
  key handler stays as it is (`app/src/lib/shortcuts.ts`, `lib/keys.ts`): it already leaves
  Space and Enter to a focused button; faders and knobs handle their own arrows and
  PageUp/PageDown and call `stopPropagation`, so the global ← → (`stepStyle`) don't fire while
  one has focus.
- **Links to pages not built yet (D32):** a control that opens a page or popover whose spec
  hasn't been built opens today's equivalent drawer or panel where one exists, else it is drawn
  in its face but disabled (`aria-disabled`, its tooltip still says what it will do). The
  targets and their interim behaviour are in Stage.md D32; each target's own spec removes its
  row when it lands.
- **Motion** comes from state, never a component timer: queued pads flash and armed pads pulse
  on the LED clock (`surface.clock`, see app-api.md › Pad), the beat blocks follow the section
  clock, meters follow `meters` frames. Components take the moment as a prop (axiom 10).

## App bar

`24,24 1392×36`, one row, `border-bottom: 1px solid var(--t)`, items centred, gap 8.

- **Wordmark** "yahaha", 18 / 500, letter-spacing −0.2, `--t`. Not a control.
- **Page tabs** (`nav`, `aria-label="Pages"`), right-aligned (`margin-left: auto`): Stage,
  Channel, Effects, Quick Racks, Multi Pads, Looper, Harm/Arp, a 1 × 16 `--line` separator with
  8px margins, Library, Settings. Each tab is 36 tall, padding `10px 10px 0`, 14 / 400, no wrap;
  inactive `--m` text on nothing; the active tab (`aria-current="page"`) is the chosen face drawn
  as a 24px block on the bottom of the tab (`background: linear-gradient(var(--t), var(--t)) left
  bottom / 100% 24px no-repeat`), so it sits on the white line. App-only state: `ui.page`
  (Stage.md D2). Tooltips: `view.stage`, `nav.channel` (new), `nav.effects`, `nav.quick`,
  `nav.multipad`, `nav.looper`, `nav.harmony`, `view.library`, `nav.settings`. Shortcuts (D37),
  by physical key: Stage Alt+G and Channel Alt+N (new: added to `NAV` in `app/src/lib/nav.ts`
  and to the Alt letters in `app/src/lib/keys.ts`), Effects Alt+E, Quick Racks Alt+R, Multi Pads
  Alt+P, Looper Alt+L, Harm/Arp Alt+H, Library Alt+B, Settings Alt+T (the existing letters).
  The other existing Alt keys stay as they are and have no tab: Alt+S the Browser, Alt+O the
  Rack page (today the Rack drawer), Alt+M the mixer details, Alt+C Charts (hidden, DECISIONS
  X1), Alt+Y Library › Style map.
- **Tabs before their page exists:** until a page's spec is built, its tab runs today's
  `NAV` entry (`toggle()`, the drawer or Library tab it opens now) and is drawn chosen while that
  entry's `open()` is true; Stage is chosen when no drawer is open and `ui.view` is `stage`.
  Channel has no drawer: until #501 it shows today's `ChannelView` for the selected part in
  place of the display (`panels/channel/nav.svelte.ts` `show(ui.selectedPart)`), and is chosen
  while that is open. (D32.)
- **Right area**, fixed 196 × 36 so the tabs sit at the same x on every board: a 1 × 16 `--line`
  separator, then the Launchkey status and the health slot.
  - **Launchkey status** (`role="status"`), 8px after the separator: a 6px dot and "Launchkey",
    14 / 400 `--m`, gap 8. `pads.connected` true: dot `--ok` with `--bg`. False: dot hollow (1px
    `--d` ring), label `--d`, `aria-label="Launchkey not connected"`. Tooltip `launchkey.status`.
  - **Health slot** (`role="status"`), right-aligned, ellipsis when long. One text, the first
    that applies:

    | When | Text | Colour | Click |
    |---|---|---|---|
    | a keyboard part's `plugin.status` is `failed` and not `missing` | "R3 failed" (first such part, `R1 R2 R3 L`) | `--ending` | opens Channel for that part |
    | `io.synth` is null | "Audio off" | `--m` | opens Settings › System |
    | `DropoutWatch.show` (3 dropouts in 30 s, `app/src/lib/dropouts.svelte.ts`) | "3 dropouts · buffer 256?" (`io.synth.bufferFrames`) | `--ending` | opens Settings › System |
    | any dropout (`io.synth.dropouts` rose) in the last 30 s | "2 dropouts" (the count in the window; "1 dropout") | `--ending` | opens Settings › System |
    | `meters.cpu.total` ≥ 0.70 | "CPU 74%" (rounded) | `--ending` | opens Settings › System |
    | otherwise | "Audio" | `--m` | not clickable |

    The slot is a `role="status"` span; when the text has a click, the text inside it is a
    `<button>` (a text button, focusable), otherwise plain text. One tooltip key, `app.health`
    (new), on the button or, without one, on the span; its body covers every row of the table, so
    the slot carries no second key (D45). "Opens Channel for that part" and "opens Settings ›
    System" follow D32 until #501 and #532 land. `aria-label` "Audio health: {text}" ("Audio
    health: fine" when calm).

## Section row

`24,68 1392×32`, margin-top 8 under the app bar, one row, gap 24, no wrap. On every board.

- **Accomp** LampButton (`size md`), label "Accomp", code "ACMP". On = `transport.acmp`. Sends
  `toggleAcmp`. Tooltip `transport.acmp`. Launchkey: Shift + encoder page ▼.
- **Count row** (`role="status"`), the row's flexible middle, centred, gap 20, 18px type. See
  Count row below.
- **Right group**, gap 8:
  - **Metronome**, a split button (the `MetronomeSplit` component, a `role="group"`
    `aria-label="Metronome"` with gap 1px): the LampButton "Metronome" (`join="start"`, which
    sets the radius to `4px 0 0 4px`; on = `metronome.on`; sends `toggleMetronome`; tooltip
    `metronome.on`) and a 20 × 32 caret "▾" (`--btn`, `--m`, 10px; radius `0 4px 4px 0`;
    `aria-haspopup="dialog"`, `aria-expanded`, `aria-label="Metronome settings"`) that opens the
    metronome popover (spec #509). Until #509 is built the caret is disabled (D32). Tooltip
    `metronome.settings` (new). No Launchkey mapping.
  - **Unison** LampButton. On = `transport.unison`. Sends `toggleUnison`. Tooltip
    `transport.unison`. No Launchkey mapping.
  - **Panic**, off face, padding 0 14. Sends `panic`. Tooltip `transport.panic`.
  - **?**, 32 × 32 off face, `aria-pressed` = help mode (`tips.help`). Toggles help mode (spec
    #508). Tooltip `app.help`.

### Count row

Left to right, each item hidden when it has nothing to say:

1. **Beat blocks:** one 24 × 24 block (radius 2, gap 4) per beat of the bar
   (`surface.clock.beatsPerBar`). The current beat is the playing section's hue with its glow
   (D29: `0 0 6px`, `--dot-glow-mix`); past beats `--past`; later beats `--btn`. Beat one always
   has a 2px `--t` top edge (`box-shadow: inset 0 2px 0 var(--t)`; on the current block both
   shadows, the edge first). Stopped: all `--btn`. The current beat comes from the clock at the
   moment `now` (a prop, the session clock in ms): `t = clock.atMs + (now − receivedMs)`,
   `pos = sectionAnchorBeats + (t − sectionAnchorMs) · tempo / 60000` (app-api.md ›
   surface.clock), current beat index `floor(pos) mod beatsPerBar` (0-based). The page wiring
   passes `now` once per animation frame; components and stories take it as a prop, and
   `receivedMs` is the `now` at which the state arrived (the wiring records it). Each block
   carries `data-beat="past|current|later"`, and beat one `data-downbeat` (test hooks, D41).
2. **Bar:** "Bar" 400 `--m`, then `{transport.bar}/{transport.sectionBars}` 300 `--t`. Hidden
   when stopped.
3. **Sections:** the playing section's name in its hue, then "→" `--m`, then the next one in the
   waiting face (26 tall, padding 0 8, 1px border, radius 4, line-height 24, in its hue), gap 8.
   Playing = `transport.section`; stopped, the Main to start on (`Main {A+transport.main}`) in
   `--m`. Next = `transport.landing` when a fill or the Break is queued or playing, else
   `transport.queued`; stopped, the armed Intro (`transport.pendingIntro`). No next: the arrow and
   chip are hidden.
4. **When:** `--t`, the first that applies: a fill queued or playing → "fill after bar {bar}"; the
   Break → "break after bar {bar}"; an Intro or Ending queued with
   `styleSettings.introEndingTiming` `endOfSection` → "after bar {sectionBars}"; a Main queued
   with `styleSettings.mainTiming` `immediate` → "next beat"; anything else queued → "after bar
   {bar}"; stopped with `transport.syncStart` → "sync start". (Stage.md D5.)

The whole row is one `role="status"` with an `aria-label` that reads it out; its parts are
`aria-hidden`. Template, each sentence dropped when its item is hidden: "Beat {b} of {n}, bar
{bar} of {sectionBars}." "{playing} playing, {next} next." (no next: "{playing} playing.";
stopped: "Stopped on {Main X}.") then the When item as a sentence ("The fill lands after bar
3.", "Sync start armed."). Example: "Beat 3 of 4, bar 3 of 4. Main B playing, Main C next. The
fill lands after bar 3." Tooltip `display.position`. The When item carries
`data-shot-mask="when"` (Stage.md › Checks).

## Full band

`24,432 1392×368` on display pages (Stage and the display tabs), margin-top 20 under the display.
Three sections side by side, gap 12: **Faders** `24,432 654×368`, **Knobs and Pads**
`690,432 626×368`, **Transport and tempo** `1328,432 88×368`. Each starts with a hairline header
row (36 tall). The band follows the hardware's left-to-right order (DECISIONS H2) and is the same
on every display board.

### Faders

Header row, gap 12: "Faders" 14 `--m`; the **fader page** tabs Panel | Style; a 1 × 16 `--line`
separator; "Layer" 14 `--m` and the **layer** tabs Vol | Pan | Reverb | Chorus | Delay. Header
tabs are 35 tall, padding `11px 10px 0`, 13 / 400, chosen face as a 22px block on the bottom.

| Tabs | Chosen = | Click sends | Tooltip | Launchkey |
|---|---|---|---|---|
| Panel, Style | `mixer.faderPage` | `setFaderPage { page }` | `mixer.page` | button under the master fader |
| Vol, Pan, Reverb, Chorus, Delay | `mixer.faderLayer` (`volume`, `pan`, `reverb`, `chorus`, `delay`) | `setFaderLayer { layer }` | `mixer.layer` | Shift + button under the master fader |

In a layer other than Vol, "Faders" reads "Faders · Reverb" with the layer word in `--t`. The
layer persists across pages; there is no fallback timer.

Below the header (8px): nine **FaderStrip** columns (`repeat(9, 1fr)`, gap 8, about 65.6px each,
272 tall), then the lamp-row headers (18 tall) and the **lamp row** (32 tall, 2px below). On the
Panel page the strips are, from `surface.faders[0..8]`:

| Strip | Name | Value | Hue | Meter (`meters`) | Name click |
|---|---|---|---|---|---|
| 1–4 | "Right 1", "Right 2", "Right 3", "Left" | `surface.faders[i].value` | `--r1 --r2 --r3 --l` | the `meters.channels` entry whose `channel` is `keyboardParts[i].channel` | opens Channel for part i (tooltip `mixer.strip.select`; D32) |
| 5 | "Style" | `surface.faders[4].value` (= `mixer.styleVolume`) | `--a` | channels 9–16: peak = the largest `peak`, RMS = the largest `rms` | Style fader page (`setFaderPage style`; tooltip `mixer.style_level`) |
| 6 | "Multi Pad" | `surface.faders[5].value` (= `mixer.multiPadVolume`) | `--t2` | channels 5–8, the same way | opens the Multi Pads page (tooltip `mixer.pad_level`; D32) |
| 7–8 | "—" | none | `--d` | none | not a control |
| 9 | "Master" | `surface.faders[8].value` (= `mixer.master`) | `--t` | peak = the larger of `meters.master`, RMS = the larger of `meters.masterRms` | opens Effects at the master (spec #519; tooltip `mixer.master`; D32) |

A channel missing from `meters.channels` (no synth, or not sent) reads 0.

The Style page (strips 1–8 the Style parts, buttons 1–8 their mutes) is drawn on PadsPage2 and
specified there (#507); the strip and lamp components are the same.

#### FaderStrip

One column: a 252-tall fader (the control) over a 20-tall name button.

- **Value** at the top, 20 tall, centred, 18 / 300, in the strip's hue. Volume layer: the number
  (0–127). Pan layer: `L20` / `C` / `R20` (64 = C). Send layers: the layer word and the number,
  "Rev 40", "Cho 12", "Dly 0", in `--t`. Empty when unused.
- **Track** from 24 to 247 (`--travel` 223px). Left of centre, two 10px meter bars at
  `50% − 17px` and `50% − 5px`, from 24 to 247, on `--mbg`: the left bar is the peak, the right
  the RMS (so the right is the lower, as drawn), both in the hue at `--meter-mix`, growing from
  the bottom (5px above the strip's 252 bottom). A 22 × 1 `--peak` tick at the held peak across
  both, its bottom at `5 + height(hold)`. `height(x) = round(223 × clamp((20·log10(x) + 60) / 60,
  0, 1))`, 0 for x = 0 (Stage.md D7).
- **Meter input:** the strip takes `meter: { peak, rms, hold } | null`, each a linear amplitude
  (0–1, as `meters` sends them); null draws no meter. The strip only draws. The held peak
  (`hold`) is computed by the page wiring from successive `meters` frames, by their `atMs`
  (held 1.5 s, then falling 20 dB/s, never below the current peak), in a pure function
  `holdPeak(prev, peak, atMs)` in `app/src/ui/FaderStrip/meter.ts`; no timer.
- **Set level** right of centre: a 3px `--track` groove at `50% + 10px`, a 3px fill in the hue
  from the bottom to the level (glow `0 0 6px` at `--fill-glow-mix`), and a 10 × 3 cap at
  `50% + 3px`, `top = (1 − value/127) × 223 + 23`. The set level outranks the meter.
- **Soft takeover**, when `surface.faders[i].waiting`: "↕" (13px `--m`) at the track's top-left,
  and a 42px dashed 1px `--m` line at the hardware position (`surface.faders[i].position`), its
  top at `(1 − position/127) × 223 + 24`. Tooltip while waiting: `mixer.pickup`.
- **Layers:** in a send or pan layer, strips 1–4 hide their meters, and their fill and cap go
  `--t`; the name keeps its hue. Pan draws its fill from the track's middle (value 64) up or down
  (Stage.md D8). Strips 5, 6 and 9 stay levels.
- **Part off** (`keyboardParts[i].sounding` false): no meter, fill and cap at 35% of the hue,
  value and name `--d`.
- **Unused** (`surface.faders[i].set` null): no value, no cap; the groove is dashed
  (`repeating-linear-gradient(to bottom, var(--line) 0 3px, transparent 3px 7px)`); not focusable;
  `aria-label="Fader 7 unused"`; tooltip `launchkey.fader_unused`.
- **Rack target:** when the live rack's controller map gives fader 1–4 another target
  (`surface.faders[i].label` is not the part's own name, e.g. `PANR2`, `HARMARP`), the name reads
  that label as given, in `--t2`, no meter, and the name opens the Rack page (`ui.page`
  `rack`, D2; until the Rack spec is built, the Rack drawer, D32). Tooltip
  `launchkey.fader_rack`.
- **Name button**, 20 tall, centred, gap 4: the name 13 / 500 in the hue, then the strip marks
  (keyboard parts only, DECISIONS M11): a 5px `--t` dot when `soundEdited`; a 12px `--warn` ⚠
  when `plugin.missing`; a 12px `--ending` ✕ when `plugin.status` is `failed` and not missing.
- **The fader as a control:** `role="slider"`, `aria-valuemin 0`, `aria-valuemax 127`,
  `aria-valuenow` the value, `aria-valuetext` "Right 1 90" (with ", hardware fader away" while
  waiting). Pointer (D23): press anywhere on the fader and drag vertically; the value moves by the
  pointer's travel from the press point, `value = clamp(round(v0 + (y0 − y) × 127 / 223), 0,
  127)` with `v0`, `y0` the value and pointer y at pointerdown (relative, not a jump to the
  pointer), with pointer capture. A send goes out when the whole-number value changes, at most
  once per animation frame (the latest value of that frame), and the last value is always sent
  on pointerup. Wheel ±1 per notch; double-click resets to 100 (pan to 64, sends to 0); arrows
  ±1, PageUp/PageDown ±10. Each change sends `surface.faders[i].set` with its value field
  (`volume`, `pan` or `value`) filled in; with the controller map's `moveRackFader` the field is
  `volume`. Tooltips: `mixer.panel.right1` … `mixer.panel.left`,
  `mixer.part.pan`, `mixer.part.reverb`, `mixer.part.chorus`, `mixer.part.variation` by layer;
  `mixer.style_level`, `mixer.pad_level`, `mixer.master`. Launchkey: faders 1–8 and the master.

#### Lamp row

Headers (18 tall, `repeat(9, 1fr)`, column gap 8, 12px `--m`, bottom hairline): "Part on/off"
over columns 1–4; "Functions" over 5–9 with "Launchkey fader buttons 5–9" right-aligned. Then the
32-tall row (gap 8) with a 1px `--line` divider between columns 4 and 5. All are LampButton
`size cell` except the last.

| Button | Label | On = | Click | Long press / right-click | Shift-click | Tooltip | Launchkey |
|---|---|---|---|---|---|---|---|
| 1–4 part | "On" / "Off" (`keyboardParts[i].on`); "Swap" while that part's swap is held | `keyboardParts[i].sounding` | `togglePart { part }` (in swap: `setLayer none`) | `setLayer { type: swap, part }`, latched until the next click | opens Channel for the part | `part.right1.on` … `part.left.on`; `part.swap` | fader buttons 1–4 (hold + knob: swap; Shift: select part) |
| Harm/Arp | "Harm/Arp" | `harmonyArp.on` | `toggleHarmonyArp` | — | — | `harmony.switch` | fader button 5 |
| Sound | "Sound" | `surface.layer.type == 'sound'` | not lit: `setLayer { type: sound }` (latch); lit: `setLayer none` | not lit: `setLayer sound` when the 350 ms elapse (pointer still down), `setLayer none` on that press's pointerup or pointercancel; lit: as a click (D18) | — | `launchkey.sound` | fader button 6 (hold) |
| L Hold | "L Hold" | `chord.leftHold` | `toggleLeftHold` | — | — | `detection.left_hold` | fader button 7 |
| Looper | "Looper" | `looper.mode == 'looping'` | `looperOnOff` | `looperRec` | — | `looper.on_off`, `looper.rec` | fader button 8; Shift + 8 = REC |
| master button | "Panel" / "Style" (`mixer.faderPage`) | not a lamp: off face, `--t2` | `toggleFaderPage` | — | `stepFaderLayer { delta: 1 }` | `mixer.page`, `mixer.layer` | button under the master fader (Shift: layer) |

Looper faces: `recording` → Record face; `recArmed` → waiting face in `--rec`; `loopArmed` →
waiting face in `--lamp`; `looping` → on. Left refused under Manual Bass shows its message on the
status line. On the Style fader page buttons 1–8 are the Style parts' mutes (#507).

### Knobs

Header row: "Knobs" 14 `--m`; the page as an accent block (padding 0 8, 13px, line-height 22,
`--g` on `--a`) reading `knobs.pageName` ("Style", "Rack", "Pan", "Reverb", "Chorus", "Delay");
right-aligned "Page" 13 `--m` and `{knobs.pageNumber}/{knobs.pageCount}` in `--t`. In swap mode
(`surface.layer.type == 'swap'`) the block reads `knobs.pageName` ("Swap R1") on the part's hue.

Below (8px), a 96-tall row: a 32-wide column of ▲ / ▼ (32 × 32 off face, 12px, gap 6, centred
vertically) then eight **Knob** columns (`repeat(8, 1fr)`, column gap 6, 68px each at 1440).

| Control | Sends | Disabled when | Tooltip | Launchkey |
|---|---|---|---|---|
| ▲ | `stepKnobPage { delta: -1 }` | `pageNumber` is 1 | `knobs.page` | encoder page ▲ |
| ▼ | `stepKnobPage { delta: 1 }` | `pageNumber` is `pageCount` | `knobs.page` | encoder page ▼ |

#### Knob

From `knobs.knobs[i]` (`function`, `name`, `short`, `value`, `level`). A 68 × 96 column, centred:

- **Name** 12 / 400, 14 tall, `--t2`: the plain word for the function (Stage.md D6 table), else
  `name`. No Assign (`function` `none`): "---" in `--d`, and the code line is empty.
- **Value** 22 / 300, 22 tall, `--a`: `value`; a trailing "%" splits off as a 12 / 400 unit with
  2px gap. Empty for No Assign.
- **Ring** 44px, 2px margin-top: a 270° arc from 225° (`conic-gradient(from 225deg, var(--a) 0
  <deg>, var(--ring-rest) <deg> 270deg, transparent 270deg)` with a `--g` disc inset 2px),
  `deg = fraction × 270`, and a 6px `--a` tip dot at the arc's end (centre at radius 21 from the
  ring's centre). `fraction = level / 127`; tempo (`level` null) uses
  `clamp((tempo − 40) / 240, 0, 1)`. No Assign: arc and rest `--mbg`, no dot.
- **Code** 12 / 400, 14 tall, `--m`: `short`.
- **As a control:** `role="slider"`, `aria-valuetext` "Dynamics 127". Drag vertically from the
  press point, with pointer capture: one `turnKnob { knob, delta }` per whole 4px travelled
  (Shift: per 12px), up positive; the steps of one animation frame go as one `turnKnob` with
  their sum as `delta`; wheel ±1 per notch; double-click
  `resetKnob { knob }`; arrows ±1. No Assign: `aria-disabled`. Tooltip `knobs.knob`. Launchkey:
  knobs 1–8. In swap mode `turnKnob` acts as `turnSwapKnob` on the session's side, so the screen
  sends the same command.

### Pads

Header row, 18px below the knobs: "Pads" 14 `--m`; `pads.pageName` 14 `--t`; on Sections only, the
legend (12px, gap 12; each a 10 × 2 bar and the word in its hue): Intro, Main, Ending, Break,
Fill; right-aligned "Bank" 13 `--m` and `{pads.pageNumber}/{pads.pageCount}` in `--t`.

Below (8px), a 142-tall row: a 32-wide column with ▲ at the top and ▼ at the bottom (18px
padding top and bottom), then the 8 × 2 **Pad** grid (`repeat(8, 1fr)`, rows 68, gaps 6; 68 × 68
at 1440), top row pads 1–8 (`pads.pads[0..7]`), bottom row 9–16.

| Control | Sends | Disabled when | Tooltip | Launchkey |
|---|---|---|---|---|
| ▲ | `surface.controls[padBankUp].action` | that action is null | `padpage.prev` | Pad Bank ▲ |
| ▼ | `surface.controls[padBankDown].action` | that action is null | `padpage.next` | Pad Bank ▼ |

Group lines: above each run of pads of one family, a 2px line in the family hue, 3px above the
pads' top, as wide as the run minus 6px (so runs read as groups). Sections page runs: 1–3 intro,
4 util, 5–7 ending, 8 util, 9–12 main, 13 brk, 14–16 util. Other pages' runs are in their specs.

#### Pad

68 × 68, radius 4, 1px border (transparent when idle), padding `0 4px 14px`, content bottom-left:
the caption 14 / 500, line-height 16, letter-spacing −0.2, wrapping on whole words only. The
numeral `1`…`16` top 5 right 7, JetBrains Mono 11. A 2px bar (radius 1) at bottom 6, left and
right 7.

The face comes from the pad's `level` and `anim` (app-api.md › Pad), its family hue from its
place on the page:

| State | When | Fill | Border | Caption | Numeral | Bar |
|---|---|---|---|---|---|---|
| Idle | `dim` | `--btn` | none | family hue (utility: `--t2`) | `--d` | none |
| Absent | `off` | `--btn` | none | `--d` | `--pad-index-dark` | none |
| Playing | `bright` + `solid` | hue, glow at `--glow-mix` | hue | `--solid-ink` | solid ink at `--solid-ink-index` | solid ink at `--solid-ink-bar` |
| Next | `bright` + `flash`; also a Main pad `bright` + `pulse` (the landing) | `--btn` | hue | `--t` | "NEXT" in the hue | hue, glow at `--bar-glow-mix` |
| Armed | `bright` + `pulse` (not a Main) | `--btn`, glow at `--glow-mix` | hue (light: plus a `--armed-ring` inset ring) | `--t` | "ARMED" in the hue | hue |
| On | a utility switch `bright` + `solid` (Sync Stop, Auto Fill) | `--lamp` | `--lamp` | `--lamp-ink` | lamp ink | none |

Start / Stop (pad 16) uses `--ok` as its hue: running is the Playing face in green. Flash and
pulse are drawn as the hardware draws them, on the LED clock (`k` in app-api.md › Pad): Next's
border and bar go between full and 18% with `frac(beats) < 0.5`; Armed's between 25% and 100% on
the triangle wave. A pad is `aria-label` "{caption} (pad n)" plus ", playing" / ", queued" /
", armed" / " (not in this style)". Pressing sends `pads.pads[i].action` (disabled when null).
Tooltip by pad: Sections `section.intro1` …, `transport.sync_start`, `section.ending1` …,
`transport.auto_fill`, `section.main_a` …, `section.break`, `tempo.tap`, `transport.sync_stop`,
`transport.start_stop`. Launchkey: the pad itself.

Sections captions (fixed, DECISIONS H3): Intro I, Intro II, Intro III, Sync Start, Ending I,
Ending II, Ending III, Auto Fill; Main A, Main B, Main C, Main D, Break, Tap, Sync Stop,
Start / Stop. Other pages caption from `pads.pads[i].label` (their specs give the table).

**Other pages until their spec lands (D33).** The Racks, Chord, Multi Pads and Setup pages
(#507, #510–#512), and the Racks page shown while Sound is held or latched, use one fallback:
every pad is family utility (caption `--t2` when idle, hue `--t` for the Playing, Next and Armed
faces; `bright` + `solid` is Playing, not On), no group lines, no legend in the header; the
caption is `pads.pads[i].label` as the state gives it (e.g. "OTS 1", "BANK -"), and an empty
label is an Absent pad. The numeral, the aria-label rule and "press sends `action`" are as above.
Tooltip on a fallback pad: its page's existing key (`padpage.racks`, `padpage.chord`,
`padpage.multi_pads`, `padpage.setup`); an empty pad `launchkey.unused`.

### Transport and tempo

Header "Transport" (36 tall, hairline), then (8px) a column, gap 6; then 20px, header "Tempo",
then (8px) a column, gap 6. All 88 × 32 off-face buttons, label left-aligned with 8px padding,
except two pairs of 41 × 32 buttons (gap 6, padding 0, label centred, 13px): **Reset | Fade** on
one row and **Fill ▲ | Fill ▼** (a `role="group"` `aria-label="Fills"`) on the next. Top to
bottom: Start / Stop, Stop, Reset | Fade, Fill ▲ | Fill ▼; Tempo: Tempo +, Tempo −, Style tempo
(each full width).

| Button | Face | Sends | Tooltip | Launchkey |
|---|---|---|---|---|
| Start / Stop | 13 / 500 `--t`; running: a 2px `--ok` bar (left/right 8, bottom 5, `--bg` glow) | `startStop` | `transport.start_stop` | Play; pad 16 |
| Stop | 14 `--t2` | `stop` | `transport.stop` | Stop |
| Reset | 13 | `sectionReset` | `transport.section_reset` | Shift + Play |
| Fade | 13; `transport.fade` `armed` → waiting face in `--t2`; `fadingIn`, `fadingOut`, `holding` → on face | `toggleFade` | `transport.fade` | Shift + Stop |
| Fill ▲ | 13, the ▲ at 9px | `fillUp` | `transport.fill_up` | — |
| Fill ▼ | 13, the ▼ at 9px | `fillDown` | `transport.fill_down` | — |
| Tempo + | 14, the + at 300 | `tempoUp`, repeating while held (`app/src/lib/tempoHold.ts`) | `tempo.up` | Scene Launch |
| Tempo − | 14, the − at 300 | `tempoDown`, repeating | `tempo.down` | Function |
| Style tempo | 13 | `resetTempo` (also: Tempo + and − held together) | `tempo.reset` | Scene Launch and Function together |

Start / Stop here and pad 16 are one control: the same label, the same green.

## Key strip

`24,820 1392×56` on display pages; tall pages put a 20px status row over it (#501). A 1px `--line`
frame, radius 4, `--g` inside, clipped. The range is `ui.keyRange` or the connected Launchkey's
(`app/src/panels/keystrip/keyboard.ts` `RANGES`: 49 = C1–C5, 61 = C1–C6, 88 = A-1–C7; Yamaha
numbering, C3 = 60).

- **White keys:** `1390 / whites` wide, full height, a 1px `--keyline` right edge; fill
  `--key-white`, or `--key-white-left` at or below the split (`keyboard.leftSplit`). The C keys
  carry "C1"…"C6" at the bottom (6px up), JetBrains Mono 11, `--key-label`.
- **Black keys:** 22 × 32, radius `0 0 3px 3px`, centred on the boundary after their white key;
  fill `--key-black` (left zone `--key-black-left`), edge `--key-black-ring` (left
  `--key-black-ring-left`).
- **Held keys** (`keyboard.held`): filled with the hue of the first part in `parts` (`0 → --r1`,
  `1 → --r2`, `2 → --r3`, `3 → --l`), glow `0 0 10px` at `--key-glow-mix`; their label
  `--solid-ink`. A held key with no parts (it only gives the chord): `--m` fill.
- **Detection line:** a 2px line along the top over `keyboard.detection` (clipped to the keys
  drawn): `--l` with `--bl` glow when detection is the left hand (Lower), `--a` otherwise (Upper,
  Full Keyboard). Stage.md D10.
- **Split marker:** a 2px `--t` line, full height, on the boundary after the split key
  (`boundary()` in `keyboard.ts`).
- `aria-label` reads it out: "Keys: split F#2, left hand G A C E, right hand E4 A4, 61 keys".
  Tooltip `keystrip.keys`. The keys don't play notes from the screen (as today).

## Status line

`24,800 1392×20` on display pages: the 20px gap between the band and the keys. One line,
`role="status"`, `aria-live="polite"`, 14px, line-height 20, ellipsis. `state.message.text` in
`--t`; with `message.error`, a 12px `--warn` ⚠ before it (gap 8). Empty when `message` is null.
It speaks only for state messages, never coaching. With a message, the line's content is one
text `<button>` (focusable; click, Enter or Space sends `clearMessage`) inside the status
region; empty, nothing in it is focusable. Tooltip `display.status` on the button.

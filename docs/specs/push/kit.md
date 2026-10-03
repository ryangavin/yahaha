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
Today only Storybook loads them (`.storybook/preview.ts`, with DM Sans 200–500 and JetBrains
Mono 400–500 from `@fontsource`); the running app loads `app.css` and Barlow. How the kit's
tokens and fonts get into the app, and what `app.css` renames to make room (`--bg`, `--line`,
`--line-strong`, `--key-white`, `--key-black`): Stage.md D48.

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

**AA as text.** Every hue and role above is drawn as text on `--g` or `--btn`, or as
`--solid-ink` on a hue fill, somewhere in the kit. Measured from the palette (WCAG 2.x
contrast ratio), every pair is 4.5:1 or more in both themes except these, which Stage.md C6
fixes by nudging the palette: dark `--ending` on `--btn` 4.27 and `--brk` on `--btn` 3.86;
light `--r2` 4.05, `--r3` / `--warn` 3.21, `--l` 3.14, `--intro` 3.41, `--main` / `--ok`
3.88 and `--fill` 3.96 on `--btn`, and `--solid-ink` (white) on light `--r3` 4.12, `--l`
4.04 and `--intro` 4.37. `--d` text is exempt by D47 (dimmed, 2.2:1 on dark), and every
carrier is listed under Faces › Dimmed text. A screen spec that draws a new text-on-face
pair adds its ratio here or in its own AA line.

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
(`Accomp` + `ACMP`, knob `Dynamics` over `DynCtrl`). Sizes, letter-spacing and line-heights
are px. Where a line-height is not given it is `normal` (DM Sans: about 1.3, so 14px → 18px)
and the element's own centring places the text; where the board fixes a block's height the
line-height is given.

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
| **Chosen** | `--t` fill, `--g` label, weight 500 (page and header tabs keep 400: the block is their mark, the board bolds nothing there) | the one picked from a set | active tab, applied One Touch, fader layer, "?" in help mode (Stage.md D50) |
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

**Dimmed text (Stage.md D47).** `--d` text is about 2.2:1 on dark and the board keeps it on
things a player can click. Every element whose text is `--d` carries `data-contrast="dim"`,
and the screenshot tool's axe run skips the `color-contrast` rule on exactly those elements
(`*:not([data-contrast="dim"])`); every other rule runs everywhere. The carriers, and only
these:

| Element | Where | Why it is `--d` |
|---|---|---|
| a part's tag and sound number when the part doesn't sound | Stage.md › Sounds row | the board dims a silent part's readout (R3) |
| an off part's strip value and name | FaderStrip › Part off | the same |
| the no-chord "—" | Stage.md › Chord | nothing to read |
| the "—" of an unused strip | FaderStrip › Unused | not a control |
| the idle pad numeral (`--d` on `--btn`), an Absent pad's caption and numeral | Pad | the numeral is a locator, not the label; absent pads are disabled |
| a No Assign knob's "---" | Knob | disabled |
| the "Launchkey" label when not connected | App bar | status, not a control |
| the key labels C1…C6 on dark (`--key-label` is `--d`) | Key strip | not a control |
| every disabled control's label (One Touch past the style's count, ▲ ▼ at an end, Track ◀ ▶ without a neighbour, the Metronome caret until #509, a null-action pad) | Faces › Disabled | axe skips the `disabled` attribute, not `aria-disabled`, and the kit keeps disabled controls focusable |

A failing pair outside this table is a bug, not a new exemption.

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
other `Fill In XY` → `Fill` (no letter: the count row names the landing Main beside it). A
name that matches none of these shows as given. A no-break space ties a name to its numeral
(`Intro I`), so only whole words wrap. This is one pure function, `sectionName(name)` in
`app/src/ui/Stage/format.ts` (Stage.md Components, 1b). It is not `sectionLabel()` in
`app/src/lib/api/types.ts` (a contract file, which gives "Fill B" and leaves other fills as
given): that one stays for the old panels and nothing in `app/src/ui` imports it.

## Interaction conventions

- **Click** is the action. **Long press** is `--long-press` (350 ms) held without moving more
  than 4px; it fires when the time is up, not on release, and the click is then swallowed.
  **Right-click** does what a completed long press does, with no release (Stage.md D62: the
  action prevents the context menu; Sound latches). **Shift-click** is the Launchkey's Shift layer:
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
  sends exactly that (through its `onsend` callback; the wiring calls `app.send`) and is
  disabled when it is null. With `ui.shift` (the `shift` prop), a
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
  Space and Enter to a focused `<button>` and Alt + letter works everywhere. **Global keys a
  control must stop:** a focused fader or knob (`role="slider"`) handles ↑ ↓ ← → (±1; ← and →
  the same as ↓ and ↑, as a native slider does) and PageUp / PageDown (±10), the fader also
  Home / End (0 / 127), in its own `keydown` and calls `stopPropagation()` on exactly those
  keys, so the window bindings for ← → (`stepStyle`) and PageUp / PageDown (`cyclePadPage`)
  don't fire while one has focus (↑ ↓ Home End have no window binding today). Every other key
  passes through: Space on a focused fader still starts and stops the band, Escape still blurs
  it (the handler's own rule). Tabs (`ChosenTabs`) are buttons: they handle no keys of their
  own and stop nothing.
- **Links to pages not built yet (D32):** a control that opens a page or popover whose spec
  hasn't been built opens today's equivalent drawer or panel where one exists, else it is drawn
  in its face but disabled (`aria-disabled`, its tooltip still says what it will do). The
  targets and their interim behaviour are in Stage.md D32; each target's own spec removes its
  row when it lands.
- **Motion** comes from state, never a component timer: queued pads flash and armed pads pulse
  on the LED clock (`surface.clock`, see app-api.md › Pad), the beat blocks follow the section
  clock, meters follow `meters` frames. Components take the moment as a prop (axiom 10).

## App bar

`24,24 1392×36`, one row, `box-sizing: border-box`, `border-bottom: 1px solid var(--t)` (the
line is the bar's 36th row, y 59; the content box is 35 tall), items centred, gap 8.

- **Wordmark** "yahaha", 18 / 500, line-height 24, letter-spacing −0.2, `--t`. Not a control.
- **Page tabs** (`nav`, `aria-label="Pages"`, a flex row with **no gap**, `align-self:
  flex-end`, Stage.md D57), right-aligned (`margin-left: auto`): Stage, Channel, Effects,
  Quick Racks, Multi Pads, Looper, Harm/Arp, a 1 × 16 `--line` separator (`align-self:
  center`, `margin: 0 8px`), Library, Settings. Each tab is a `button`, 36 tall, padding
  `10px 10px 0`, 14 / 400, line-height 18 (so the text box is y 10–28 of the tab, centred in
  the 24px block below), no border, no wrap; the tabs abut, so 20px of padding separates two
  labels; inactive `--m` text on nothing; the active tab (`aria-current="page"`) is the chosen
  face at weight 400 drawn as a 24px block on the bottom of the tab (`background:
  linear-gradient(var(--t), var(--t)) left bottom / 100% 24px no-repeat`, label `--g`), spanning
  y 35–59 of the bar so it sits on the white line. Which tab is chosen, and what a click does,
  is Stage.md D52 (the `Stage` takes it as the `page` prop and reports clicks with `onpage`).
  Tooltips: `view.stage`, `nav.channel` (new), `nav.effects`, `nav.quick`,
  `nav.multipad`, `nav.looper`, `nav.harmony`, `view.library`, `nav.settings`. Shortcuts (D37),
  by physical key: Stage Alt+G and Channel Alt+N (new: added to `NAV` in `app/src/lib/nav.ts`
  and to the Alt letters in `app/src/lib/keys.ts`), Effects Alt+E, Quick Racks Alt+R, Multi Pads
  Alt+P, Looper Alt+L, Harm/Arp Alt+H, Library Alt+B, Settings Alt+T (the existing letters).
  The other existing Alt keys stay as they are and have no tab: Alt+S the Browser, Alt+O the
  Rack page (today the Rack drawer), Alt+M the mixer details, Alt+C Charts (hidden, DECISIONS
  X1), Alt+Y Library › Style map.
- **Tabs before their page exists (Stage.md D52, D53):** until a page's spec is built, its
  tab runs today's `NAV` entry's `toggle()` (the drawer or Library tab it opens now), and the
  chosen tab is `chosenPage()`: Library › Racks → Quick Racks, Library on any other tab →
  Library, the open drawer's tab (Effects, Multi Pads, Looper, Harm/Arp, Settings; the Rack
  and Charts drawers have no tab), else Channel while today's `ChannelView` shows in the
  display's place (`channelNav.open`), else Stage. Exactly one tab is chosen. Stage and
  Channel get `NAV` entries of their own (Alt+G, Alt+N). Clicking the chosen tab runs its
  toggle too, which closes what it opened: the Stage is chosen again.
- **Right area**, fixed 196 wide, `align-self: stretch` (the 35px content box; its items are
  centred in it, a half pixel off the board's 36, which Chrome snaps the same way, D57),
  `flex: none`, items centred, so the tabs sit at the same x on every board: a 1 × 16
  `--line` separator (`flex: none`), then the Launchkey status and the health slot.
  - **Launchkey status** (`role="status"`), `margin-left: 8px`, a flex row, items centred, gap
    8, no wrap: a 6px round dot and "Launchkey", 14 / 400 `--m`. `pads.connected` true: dot
    `--ok` with `--bg`. False: dot hollow (transparent, 1px `--d` ring), label `--d` with
    `data-contrast="dim"`, `aria-label="Launchkey not connected"`. Tooltip `launchkey.status`.
  - **Health slot** (`role="status"`), `margin-left: auto`, `padding-left: 8px`, `min-width:
    0`, no wrap, ellipsis when long; 14 / 400, line-height `normal`, centred by the row. One
    text, the first that applies:

    | When | Text | Colour | Click |
    |---|---|---|---|
    | a keyboard part's `plugin.status` is `failed` and not `missing` | "R3 failed" (first such part, `R1 R2 R3 L`) | `--ending` | opens Channel for that part |
    | `io.synth` is null | "Audio off" | `--m` | opens Settings › System |
    | `dropouts` (the `Stage` prop: dropouts in the last 30 s, 0–3, Stage.md D54) is 3 and `io.synth.bufferFrames` is a number below 1024 | "3 dropouts · buffer 256?" (the `bufferFrames`) | `--ending` | opens Settings › System |
    | `dropouts` ≥ 1 | "{n} dropouts" ("1 dropout") | `--ending` | opens Settings › System |
    | `meters.cpu.total` ≥ 0.70 | "CPU 74%" (`Math.round(total × 100)`) | `--ending` | opens Settings › System |
    | otherwise | "Audio" | `--m` | not clickable |

    The slot is a `role="status"` span; when the text has a click, the text inside it is a
    `<button>` (a text button, focusable; the `Stage` reports it as `onopen({ channel })` or
    `onopen('settingsAudio')`), otherwise plain text. One tooltip key, `app.health` (new), on
    the button or, without one, on the span; its body covers every row of the table, so the
    slot carries no second key (D45). "Opens Channel for that part" and "opens Settings ›
    System" follow D32 until #501 and #532 land. `aria-label` "Audio health: {text}" ("Audio
    health: fine" when calm). The dropout text goes calm by itself 30 s after the last
    dropout; the slot has no dismiss (D54).

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
    metronome popover (spec #509); it is a `Button` of the `caret` variant (Stage.md
    Components, row 3: off face but the glyph in `--m`). Until #509 is built the caret is
    disabled (D32). Tooltip `metronome.settings` (new). No Launchkey mapping.
  - **Unison** LampButton. On = `transport.unison`. Sends `toggleUnison`. Tooltip
    `transport.unison`. No Launchkey mapping.
  - **Panic**, off face, padding 0 14. Sends `panic`. Tooltip `transport.panic`.
  - **?**, 32 × 32, the glyph 14px; off face with `aria-pressed="false"`, or the chosen face
    with `aria-pressed="true"` in help mode (the `Stage`'s `help` prop; Stage.md D50);
    `aria-label="Help"`. Click: `onhelp()` (toggles
    `tips.help`; the help face itself is spec #508). Tooltip `app.help`.

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
2. **Bar:** one item: "Bar" 400 `--m`, a normal space, then
   `{transport.bar}/{transport.sectionBars}` 300 `--t`. Hidden when stopped.
3. **Sections:** one item, a flex row, gap 8: the playing section's name 300 in its hue, then
   "→" 300 `--m`, then the next one in the waiting face (26 tall, padding 0 8, 1px border,
   radius 4, line-height 24, 300, in its hue).
   Playing = `transport.section`; stopped, the Main to start on (`Main {A+transport.main}`) in
   `--m`. Next = `transport.landing` when a fill or the Break is queued or playing, else
   `transport.queued`; stopped, the armed Intro (`transport.pendingIntro`). No next: the arrow and
   chip are hidden.
4. **When:** 300 `--t`, the first that applies: a fill queued or playing → "fill after bar {bar}"; the
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

Header row (36 tall border-box, the hairline its 36th row, content 35; `align-items: stretch`,
gap 12, no wrap): "Faders" 14 `--m` (`align-self: center`); the **fader page** tabs Panel |
Style (a `ChosenTabs` of `kind: 'tab'`: a `role="tablist"` `aria-label="Fader page"`, a flex
row with **no gap**); a 1 × 16 `--line` separator (`align-self: center`); then one flex row
holding "Layer" 14 `--m` (`align-self: center`, `margin-right: 4px`) and the **layer** tabs
Vol | Pan | Reverb | Chorus | Delay (a second `ChosenTabs`, `role="tablist"`
`aria-label="Fader layer"`; abutting, no gap). Header tabs are `role="tab"` buttons
(`aria-selected` true on the chosen one, false on the rest), 35 tall (the content box's full height), padding
`11px 10px 0`, 13 / 400, line-height 16 (text box y 11–27, centred in the block), no border,
chosen (`aria-selected="true"`) as a 22px `--t` block on the bottom (y 13–35, on the
hairline), label `--g`, weight 400; the others `--m` on nothing. Two labels are 20px apart.

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
| 1–4 | "Right 1", "Right 2", "Right 3", "Left" | `surface.faders[i].value` | `--r1 --r2 --r3 --l` | the `meters.channels` entry whose `channel` is `keyboardParts[i].channel` | opens Channel for part i (tooltip `mixer.strip.select`; `aria-label` "{name}: open Channel"; D32) |
| 5 | "Style" | `surface.faders[4].value` (= `mixer.styleVolume`) | `--a` | channels 9–16: peak = the largest `peak`, RMS = the largest `rms` | Style fader page (`setFaderPage style`; tooltip `mixer.style_level`; `aria-label` "Style: show the Style faders") |
| 6 | "Multi Pad" | `surface.faders[5].value` (= `mixer.multiPadVolume`) | `--t2` | channels 5–8, the same way | opens the Multi Pads page (tooltip `mixer.pad_level`; `aria-label` "Multi Pad: open Multi Pads"; D32) |
| 7–8 | "—" | none | `--d` | none | not a control |
| 9 | "Master" | `surface.faders[8].value` (= `mixer.master`) | `--t` | peak = the larger of `meters.master`, RMS = the larger of `meters.masterRms` | opens Effects at the master (spec #519; tooltip `mixer.master`; `aria-label` "Master: open Effects"; D32) |

A channel missing from `meters.channels` (no synth, or not sent) reads 0.

The Style page (strips 1–8 the Style parts, buttons 1–8 their mutes) is drawn on PadsPage2 and
specified there (#507); the strip and lamp components are the same.

#### FaderStrip

One column: a 252-tall fader (the control) over a 20-tall name button.

Every x below is a **left edge** measured from the strip's left, as the board's `left:`
values are; the strip is `position: relative` and its parts `position: absolute`; `50%` is the
strip's middle (32.8px at the 65.6px width).

- **Value** at the top (`left: 0; right: 0; top: 0`), 20 tall, line-height 20, text centred,
  18 / 300, no wrap, in the strip's hue. Volume layer: the number (0–127). Pan layer: `L20` /
  `C` / `R20` (64 = C), in `--t` (the layers' colour, D8). Send layers: the layer word and the
  number, "Rev 40", "Cho 12", "Dly 0", in `--t`. Empty when unused.
- **Track** from 24 to 247 (`--travel` 223px). Left of centre, two 10px-wide meter bars whose
  left edges are at `50% − 17px` (peak) and `50% − 5px` (RMS; so they span −17…−7 and −5…+5
  with a 2px gap), `top: 24px; bottom: 5px`, on `--mbg`: the right is the lower, as drawn;
  both fills in the hue at `--meter-mix`, `bottom: 5px`, growing upward (5px above the strip's
  252 bottom). A 22 × 1 `--peak` tick, left edge `50% − 17px` (so it spans both bars), at the
  held peak: `bottom: 5 + height(hold)`. `height(x) = round(223 × clamp((20·log10(x) + 60) /
  60, 0, 1))`, 0 for x = 0 (Stage.md D7).
- **Meter input:** the strip takes `meter: { peak, rms, hold } | null`, each a linear amplitude
  (0–1, as `meters` sends them); null draws no meter. The strip only draws. The held peak
  (`hold`) is computed by the page wiring from successive `meters` frames, by their `atMs`,
  in a pure function in `app/src/ui/FaderStrip/meter.ts`: `holdPeak(prev: HoldState |
  undefined, peak: number, atMs: number): HoldState` with `HoldState = { peak: number;
  sinceMs: number }`. If `prev` is undefined or `peak ≥ prev.peak`: `{ peak, sinceMs: atMs }`
  (a new or equal peak restarts the hold). Else the held value decays after 1.5 s at 20 dB/s:
  `fallen = prev.peak × 10 ^ (−max(0, atMs − prev.sinceMs − 1500) / 1000)` (one decade per
  second is 20 dB/s), and the result is `{ peak: max(peak, fallen), sinceMs }` where `sinceMs`
  is `atMs` when the live `peak` won, else `prev.sinceMs`. The strip's `hold` is the state's
  `peak`; no timer.
- **Set level** right of centre: a 3px-wide `--track` groove, left edge `50% + 10px`, `top:
  24px; bottom: 5px`; a 3px fill in the hue on the same left edge from `bottom: 5px` up to the
  level, height `223 − round((1 − value/127) × 223)` (glow `0 0 6px` at `--fill-glow-mix`); and
  a 10 × 3 cap, left edge `50% + 3px` (so it spans +3…+13 and overhangs the groove by 7px on
  the left), `top = round((1 − value/127) × 223) + 23`. The set level outranks the meter.
- **Soft takeover**, when `surface.faders[i].waiting`: "↕" 13px, line-height 13, `--m`, at
  `left: 0; top: 24px` (the track's top-left corner), and a 42px-wide dashed line (`height: 0;
  border-top: 1px dashed var(--m)`), left edge `50% − 22px` (spanning −22…+20, over the meters
  and the groove), at the hardware position (`surface.faders[i].position`): `top =
  round((1 − position/127) × 223) + 24`. The "↕" span carries `use:tip={'mixer.pickup'}`
  (hover only: it is not focusable; the slider keeps its own key).
- **Layers:** in a send or pan layer, strips 1–4 hide their meters, and their fill and cap go
  `--t`; the name keeps its hue. Pan draws its fill from the track's middle (value 64) up or down
  (Stage.md D8). Strips 5, 6 and 9 stay levels.
- **Part off** (`keyboardParts[i].sounding` false): no meter, fill and cap at 35% of the hue
  (`color-mix(in srgb, <hue> 35%, transparent)`), value and name `--d` with
  `data-contrast="dim"` (D47).
- **Unused** (`surface.faders[i].set` null): one `role="group"` with `aria-label="Fader 7
  unused"` (Stage.md D55) holding only `aria-hidden` spans: no value, no cap, no slider, the
  groove dashed (`repeating-linear-gradient(to bottom, var(--line) 0 3px, transparent 3px
  7px)`) and the name "—" in `--d` (`data-contrast="dim"`); nothing focusable; tooltip
  `launchkey.fader_unused` on the group.
- **Rack target:** when the live rack's controller map gives fader 1–4 another target
  (`surface.faders[i].label` is not the engine's label for that part, `"RIGHT 1"`,
  `"RIGHT 2"`, `"RIGHT 3"`, `"LEFT"`; e.g. `PANR2`, `HARMARP`; Stage.md D63), the name reads
  that label as given, in `--t2`, no meter, and the name opens the Rack page (`ui.page`
  `rack`, D2; until the Rack spec is built, the Rack drawer, D32). Tooltip
  `launchkey.fader_rack` on both the slider and the name.
- **Name button**, 20 tall, centred, gap 4: the name 13 / 500 in the hue, then the strip marks
  (keyboard parts only, DECISIONS M11): a 5px `--t` dot when `soundEdited`; a 12px `--warn` ⚠
  when `plugin.missing`; a 12px `--ending` ✕ when `plugin.status` is `failed` and not missing.
- **The fader as a control:** `role="slider"`, `aria-valuemin 0`, `aria-valuemax 127`,
  `aria-valuenow` the value, `aria-valuetext` "{name} {value text}" with the value text as the
  strip shows it: "Right 1 90", pan "Right 1 L20", a send "Right 1 Rev 40", a rack target
  "PANR2 64", "Style 100", "Master 100" (plus ", hardware fader away" while waiting).
  Pointer (D23): press anywhere on the fader and drag vertically; the value moves by the
  pointer's travel from the press point, `value = clamp(round(v0 + (y0 − y) × 127 / 223), 0,
  127)` with `v0`, `y0` the value and pointer y at pointerdown (relative, not a jump to the
  pointer), with pointer capture. A send goes out when the whole-number value changes, at most
  once per animation frame (the latest value of that frame), and the last value is always sent
  on pointerup. Wheel ±1 per notch; double-click resets to 100 (pan to 64, sends to 0); arrows
  ±1, PageUp/PageDown ±10, Home/End 0/127 (the keys it stops: Interaction conventions ›
  Keyboard). Each change sends `surface.faders[i].set` with its value field
  (`volume`, `pan` or `value`) filled in; with the controller map's `moveRackFader` the field is
  `volume`. Tooltips: `mixer.panel.right1` … `mixer.panel.left`,
  `mixer.part.pan`, `mixer.part.reverb`, `mixer.part.chorus`, `mixer.part.variation` by layer;
  `mixer.style_level`, `mixer.pad_level`, `mixer.master`. Launchkey: faders 1–8 and the master.

#### Lamp row

Headers: an 18-tall grid (`repeat(9, minmax(0, 1fr))`, column gap 8, no wrap) with two cells,
each `box-sizing: border-box` with a 1px `--line` bottom border, 12 / 400 `--m`, line-height
16 (the text sits at the cell's top, 1px above the hairline): "Part on/off" spanning columns
1–4 (`grid-column: 1 / 5`); "Functions" spanning 5–9 (`grid-column: 5 / 10`), a flex row with
"Launchkey fader buttons 5–9" pushed right (`margin-left: auto`). Then, `margin-top: 2px`, the
32-tall row (the same grid, gap 8, `position: relative`) with a 1 × 32 `--line` divider
(`position: absolute; top: 0; bottom: 0`) centred in the gap between columns 4 and 5: `left:
calc((100% − 64px) × 4 / 9 + 28px)` (four columns plus three gaps plus half a gap; 290.2px
from the row's left at the 654px width). All are LampButton `size cell` (13px label, no
code; the board draws no codes here; the label no-wrap with ellipsis) except the last, a
Button (`cell`). LampButton is controlled (Stage.md D49): each lamp's `on` is the state field
in the table, and a click only sends. Accessible names (`name`): the part lamps "Right 1",
"Right 2", "Right 3", "Left" (the state is `aria-pressed`, so the name is the part, not
"On"); the others their label.

| Button | Label | On = | Click | Long press / right-click | Shift-click | Tooltip | Launchkey |
|---|---|---|---|---|---|---|---|
| 1–4 part | "On" / "Off" (`keyboardParts[i].on`); "Swap" while that part's swap is held | `keyboardParts[i].sounding` | `togglePart { part }` (in swap: `setLayer none`) | `setLayer { type: swap, part }`, latched until the next click | opens Channel for the part | `part.right1.on` … `part.left.on`; `part.swap` | fader buttons 1–4 (hold + knob: swap; Shift: select part) |
| Harm/Arp | "Harm/Arp" | `harmonyArp.on` | `toggleHarmonyArp` | — | — | `harmony.switch` | fader button 5 |
| Sound | "Sound" | `surface.layer.type == 'sound'` | not lit: `setLayer { type: sound }` (latch); lit: `setLayer none` | not lit: `setLayer sound` when the 350 ms elapse (pointer still down), `setLayer none` on that press's pointerup or pointercancel; lit: as a click (D18) | — | `launchkey.sound` | fader button 6 (hold) |
| L Hold | "L Hold" | `chord.leftHold` | `toggleLeftHold` | — | — | `detection.left_hold` | fader button 7 |
| Looper | "Looper" | `looper.mode == 'looping'` | `looperOnOff` | `looperRec` | — | `looper.on_off`, `looper.rec` | fader button 8; Shift + 8 = REC |
| master button | "Panel" / "Style" (`mixer.faderPage`) | not a lamp: off face, `--t2` | `toggleFaderPage` | — | `stepFaderLayer { delta: 1 }` | `mixer.page`, `mixer.layer` | button under the master fader (Shift: layer) |

Looper faces (`looper.mode`): `recording` → Record face (`rec` with `on`); `recArmed` →
`waiting="rec"`; `loopArmed` → `waiting="lamp"`; `looping` → on; `off` → off (D13; the
waiting face on a lamp: transparent fill, 1px border and label in that hue, weight 400). Left
refused under Manual Bass shows its message on the status line. On the Style fader page
buttons 1–8 are the Style parts' mutes (#507).

### Knobs

Header row (36 tall, hairline, items centred, gap 12): "Knobs" 14 `--m`; the page as an
accent block (padding 0 8, 13 / 400, line-height 22, `--g` on `--a`, no radius) reading
`knobs.pageName` ("Style", "Rack", "Pan", "Reverb", "Chorus", "Delay"; in swap mode,
`surface.layer.type == 'swap'`, the state's "Swap R1", on the same accent block: Stage.md
D51); right-aligned (`margin-left: auto`) one span "Page" 13 / 400 `--m` with
`{knobs.pageNumber}/{knobs.pageCount}` after its space in `--t`, the same 13px.

Below (`margin-top: 8px`), a 96-tall flex row, **gap 8**: a 32-wide column (`flex: none`) of
▲ / ▼ (32 × 32 off face, 12px glyph, gap 6, `justify-content: center`), then the knob grid
(`flex: 1; min-width: 0`): eight **Knob** columns (`repeat(8, minmax(0, 1fr))`, column gap 6,
68px each at 1440: (586 − 42) / 8).

| Control | Sends | Disabled when | Tooltip | Launchkey |
|---|---|---|---|---|
| ▲ | `stepKnobPage { delta: -1 }` | `pageNumber` is 1 | `knobs.page` | encoder page ▲ |
| ▼ | `stepKnobPage { delta: 1 }` | `pageNumber` is `pageCount` | `knobs.page` | encoder page ▼ |

#### Knob

From `knobs.knobs[i]` (`function`, `name`, `short`, `value`, `level`). A 68 × 96 column, centred:

- **Name** 12 / 400, 14 tall, line-height 14, `--t2`, no wrap: the plain word for the function
  (Stage.md D6 table), else `name`. No Assign (`function` `none`): "---" in `--d`
  (`data-contrast="dim"`), and the code line is empty.
- **Value** 22 / 300, 22 tall, line-height 22, `--a`, no wrap: `value`; a trailing "%" splits
  off as a 12 / 400 unit span inside the value (`margin-left: 2px`), in the same `--a` (it
  inherits). Empty for No Assign.
- **Ring** 44px, 2px margin-top: a 270° arc from 225° (`conic-gradient(from 225deg, var(--a) 0
  <deg>, var(--ring-rest) <deg> 270deg, transparent 270deg)` with a `--g` disc inset 2px),
  `deg = fraction × 270`, and a 6px `--a` tip dot at the arc's end (centre at radius 21 from the
  ring's centre). `fraction = level / 127`; tempo (`level` null) uses
  `clamp((tempo − 40) / 240, 0, 1)`. No Assign: arc and rest `--mbg`, no dot.
- **Code** 12 / 400, 14 tall, line-height 14, `--m`, no wrap: `short`. (14 + 22 + 2 + 44 + 14
  = 96, the column's height; the column is a flex column, `align-items: center`.)
- **As a control:** `role="slider"` with `aria-valuemin`, `aria-valuemax`, `aria-valuenow` and
  `aria-valuetext` "Dynamics 127" per Stage.md D61. `knobFraction(level: number | null,
  tempo: number)` is the pure function behind the ring, given `transport.tempo` (not the
  value text). Drag vertically from the
  press point, with pointer capture: one `turnKnob { knob, delta }` per whole 4px travelled
  (Shift: per 12px), up positive; the steps of one animation frame go as one `turnKnob` with
  their sum as `delta`; wheel ±1 per notch; double-click
  `resetKnob { knob }`; arrows `turnKnob` ±1, PageUp/PageDown ±10; no Home/End (a knob's
  steps are not a level: Retrigger Rate switches every 3) (the keys it stops: Interaction
  conventions › Keyboard). No Assign: `aria-disabled` and no key does anything. Tooltip `knobs.knob`. Launchkey:
  knobs 1–8. In swap mode `turnKnob` acts as `turnSwapKnob` on the session's side, so the screen
  sends the same command.

### Pads

Header row (36 tall, hairline, items centred, gap 12, no wrap), `margin-top: 18px` under the
knobs: "Pads" 14 `--m`; `pads.pageName` 14 `--t`; on Sections only, the legend (a flex row,
`margin-left: 4px`, items centred, gap 12, 12px; each item a flex row, gap 6, a 10 × 2 bar
with radius 1 and the word, both in the family hue): Intro, Main, Ending, Break, Fill;
right-aligned (`margin-left: auto`) one span "Bank" 13 / 400 `--m` with
`{pads.pageNumber}/{pads.pageCount}` after its space in `--t`, the same 13px.

Below (`margin-top: 8px`), a 142-tall flex row, **gap 8**: a 32-wide column (`flex: none`,
`padding: 18px 0`, `justify-content: space-between`) with ▲ at the top and ▼ at the bottom
(32 × 32 off face, 12px glyph), then the 8 × 2 **Pad** grid (`flex: 1; min-width: 0;
position: relative`; `repeat(8, minmax(0, 1fr))`, `grid-auto-rows: 68px`, column and row gap
6; 68 × 68 at 1440), top row pads 1–8 (`pads.pads[0..7]`), bottom row 9–16.

| Control | Sends | Disabled when | Tooltip | Launchkey |
|---|---|---|---|---|
| ▲ | `surface.controls[padBankUp].action` | that action is null | `padpage.prev` | Pad Bank ▲ |
| ▼ | `surface.controls[padBankDown].action` | that action is null | `padpage.next` | Pad Bank ▼ |

Group lines: above each run of pads of one family, a 2px line in the family hue, its bottom
3px above the pads' top (`top: −3px` of the row), **exactly as wide as the run** (from the
first pad's left edge to the last pad's right edge: `n × 68 + (n − 1) × 6`), so runs read as
groups and two runs in a row are 6px apart. (The board draws each as a span 6px wider than
the run starting 3px left of it, with the line `calc(100% − 6px)` wide and centred: the same
line.) Sections page runs: 1–3 intro, 4 util, 5–7 ending, 8 util, 9–12 main, 13 brk, 14–16
util. Other pages' runs are in their specs.

#### Pad

68 × 68, radius 4, 1px border (transparent when idle), padding `0 4px 14px`, content bottom-left:
the caption 14 / 500, line-height 16, letter-spacing −0.2, wrapping on whole words only. The
numeral `1`…`16` top 5 right 7, JetBrains Mono 11. A 2px bar (radius 1) at bottom 6, left and
right 7.

The face comes from the pad's `level` and `anim` (app-api.md › Pad), its family hue from its
place on the page:

| State | When | `data-face` | Fill | Border | Caption | Numeral | Bar |
|---|---|---|---|---|---|---|---|
| Idle | `dim` | `off` | `--btn` | none | family hue (utility: `--t2`) | `--d` (`data-contrast="dim"`) | none |
| Absent | `off` | `disabled` (and `aria-disabled`) | `--btn` | none | `--d` (`data-contrast="dim"`) | `--pad-index-dark` (`data-contrast="dim"`) | none |
| Playing | `bright` + `solid` | `solid` | hue, glow at `--glow-mix` | hue | `--solid-ink` | solid ink at `--solid-ink-index` | solid ink at `--solid-ink-bar` |
| Next | `bright` + `flash`; also a Main pad `bright` + `pulse` (the landing) | `waiting`, `data-anim="flash"` | `--btn` | hue | `--t` | "NEXT" in the hue | hue, glow at `--bar-glow-mix` |
| Armed | `bright` + `pulse` (not a Main) | `waiting`, `data-anim="pulse"` | `--btn`, glow at `--glow-mix` | hue (light: plus a `--armed-ring` inset ring) | `--t` | "ARMED" in the hue | hue |
| On | a utility switch `bright` + `solid` (Sync Stop, Auto Fill) | `on` | `--lamp` | `--lamp` | `--lamp-ink` | lamp ink | none |

Start / Stop (pad 16) uses `--ok` as its hue: running is the Playing face in green. Flash and
pulse are drawn as the hardware draws them, on the LED clock: `k = brightness(pad, beats)`
(`lib/leds.ts`, with `beats` the LED clock's position from `surface.clock` at `now`): Next's
border and bar go between full (1) and `DIM` (0.18) with `frac(beats) < 0.5`; Armed's between
0.25 and 1 on the triangle wave. `k` is applied as the hue's strength in the border, the bar
and the glow (`color-mix(in srgb, <hue> calc(k × 100%), transparent)`, the glow's mix token
multiplied by `k`); the caption, the numeral and the fill don't pulse. A pad is `aria-label` "{caption} (pad n)" plus ", playing" / ", queued" /
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
| Fade | 13; `transport.fade` `armed` → `face="waiting"` (hue `t2`); `fadingIn`, `fadingOut`, `holding` → `face="on"`; `off` → off (Stage.md D50) | `toggleFade` | `transport.fade` | Shift + Stop |
| Fill ▲ | 13, the ▲ at 9px | `fillUp` | `transport.fill_up` | — |
| Fill ▼ | 13, the ▼ at 9px | `fillDown` | `transport.fill_down` | — |
| Tempo + | 14, the + at 300 | `tempoUp`, repeating while held (`app/src/lib/tempoHold.ts`, pointerdown to pointerup); Enter or Space sends one (Stage.md D62) | `tempo.up` | Scene Launch |
| Tempo − | 14, the − at 300 | `tempoDown`, the same way | `tempo.down` | Function |
| Style tempo | 13 | `resetTempo` (also: Tempo + and − held together) | `tempo.reset` | Scene Launch and Function together |

Start / Stop here and pad 16 are one control: the same label, the same green.

## Key strip

`24,820 1392×56` on display pages; tall pages put a 20px status row over it (#501). A 1px `--line`
frame, radius 4, `--g` inside, clipped. The range is `ui.keyRange` or the connected Launchkey's
(`app/src/panels/keystrip/keyboard.ts` `RANGES`: 49 = C1–C5, 61 = C1–C6, 88 = A-1–C7; Yamaha
numbering, C3 = 60).

The geometry is `layout(RANGES[range])` from `keyboard.ts`: each key's `x` and `w` as
fractions of the 1390px inside the frame (white keys `1 / whites`; a black key 0.58 of a
white, centred on the edge after the white below it).

- **White keys:** `1390 / whites` wide, full height, a 1px `--keyline` right edge; fill
  `--key-white`, or `--key-white-left` at or below the split (`keyboard.leftSplit`). Every C
  in the range carries its `noteName` at the bottom (6px up, centred), JetBrains Mono 11,
  `--key-label`: "C1"…"C5" at 49, "C1"…"C6" at 61, "C0"…"C7" at 88.
- **Black keys:** 0.58 of a white wide (22px at 61), 32 tall, radius `0 0 3px 3px`, at
  `layout()`'s `x`; fill `--key-black` (left zone `--key-black-left`), edge `--key-black-ring`
  (left `--key-black-ring-left`).
- **Held keys** (`keyboard.held`): filled with the hue of the first part in `parts` (`0 → --r1`,
  `1 → --r2`, `2 → --r3`, `3 → --l`), glow `0 0 10px` at `--key-glow-mix`; their label
  `--solid-ink`. A held key with no parts (it only gives the chord): `--m` fill.
- **Detection line:** a 2px line along the top (y 0–2 inside the frame) from the left edge of
  the lowest key to the right edge of the highest key of `detectionArea(keyboard.detection,
  RANGES[range])` (each key's own `x` and `x + w`, black or white; null: no line): `--l` with
  `--bl` glow when detection is the left hand (Lower), `--a` otherwise (Upper, Full Keyboard).
  Stage.md D10.
- **Split marker:** a 2px `--t` line, full height, centred on the boundary after the split key
  (`left: calc(boundary × 100% − 1px)`, `boundary()` in `keyboard.ts`), above the keys.
- The C labels are `--key-label` (`--d` on dark): `data-contrast="dim"` (D47).
- The strip is `role="img"` (Stage.md D61) and its `aria-label` reads it out from the template
  in Stage.md D56: "Keys: split {split}, left hand {pitch classes}, right hand {notes with
  octave}, {n} keys", a hand dropped when it holds no key, both replaced by "no keys held"
  when nothing is held: "Keys: split F#2, left hand G A C E, right hand E4 A4, 61 keys".
  Names from `noteName` / `pcName` in `keyboard.ts` (their spelling: C C# D Eb E F F# G Ab A
  Bb B). Tooltip `keystrip.keys`. The keys don't play notes from the screen (as today).

## Status line

`24,800 1392×20` on display pages: the 20px gap between the band and the keys. One line,
`role="status"`, `aria-live="polite"`, 14px, line-height 20, ellipsis. `state.message.text` in
`--t`; with `message.error`, a 12px `--warn` ⚠ before it (gap 8). Empty when `message` is null.
It speaks only for state messages, never coaching. With a message, the line's content is one
text `<button>` (focusable; click, Enter or Space sends `clearMessage`) inside the status
region; empty, nothing in it is focusable. Tooltip `display.status` on the button.

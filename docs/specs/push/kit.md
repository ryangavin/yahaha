# Push kit

The shared parts every Push screen is built from: tokens, the four faces, the hue roles, and the
components that appear on more than one board (app bar, section row and its count row, the full
band of faders, knobs, pads and transport, the key strip, the status line). Defined once here; a
screen spec names a part (`kit.md` › Pad) and says only what its screen does differently.

- **Source boards:** `docs/design/push/Stage-Dark.dc.html` and `Stage-Light.dc.html` (#500). The
  light board's markup is byte-identical to the dark one; every theme difference is a token.
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
| `--bg --ba --ba2 --bw --bm --bl` | glows (green dot, accent text, chord, white, Main text, L line) | `none` | |

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
| `--armed-ring` | `0px` | `1px` | light only: an armed pad's extra inset ring (no glow in light) |
| `--solid-ink-index` | `55%` | `70%` | a solid pad's numeral: `color-mix(var(--solid-ink) …)` |
| `--solid-ink-bar` | `45%` | `60%` | a solid pad's 2px bar |
| `--stage-art`, `--stage-art-fade` | the two gradients of `Stage-Dark.dc.html:64-65`, verbatim | `Stage-Light.dc.html:40-41`, verbatim | Stage display art (Stage.md D9) |

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
everywhere: `Intro A/B/C` → `Intro I/II/III`, `Ending A/B/C` → `Ending I/II/III`, `Main A–D`
unchanged, `Fill In BA` → `Break`, any other `Fill In XY` → `Fill`. A no-break space ties a name
to its numeral (`Intro I`), so only whole words wrap.

## Interaction conventions

- **Click** is the action. **Long press** is `--long-press` (350 ms) held without moving more
  than 4px; it fires when the time is up, not on release, and the click is then swallowed.
  **Right-click** does what long press does. **Shift-click** is the Launchkey's Shift layer:
  `ui.shift` (Shift held on the computer keyboard, or the latched on-screen Shift where a screen
  has one).
- **Drag** on a fader or knob moves it; **wheel** steps it; **double-click** resets it; arrow
  keys step it when focused (PageUp/PageDown ten steps, Shift for fine on knobs).
- **Tooltips:** every interactive element carries `use:tip={'<key>'}` (`data-tip`), with the key
  in `app/src/help/tooltips.ts`. The kit gives each part's key.
- **Parity:** what the Launchkey does, the screen does with the same command. Where a part's
  state carries an `action` (`pads.pads[i].action`, `surface.controls[i].action`), the screen
  sends exactly that (`app.send(action)`) and is disabled when it is null.
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
  `nav.multipad`, `nav.looper`, `nav.harmony`, `view.library`, `nav.settings`. Shortcuts: the
  existing Alt letters in `app/src/lib/nav.ts`, and Alt+B for Library.
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

    It is a button only when it has a click. Tooltip `app.health` (new); in trouble, also
    `audio.dropouts`.

## Section row

`24,68 1392×32`, margin-top 8 under the app bar, one row, gap 24, no wrap. On every board.

- **Accomp** LampButton (`size md`), label "Accomp", code "ACMP". On = `transport.acmp`. Sends
  `toggleAcmp`. Tooltip `transport.acmp`. Launchkey: Shift + encoder page ▼.
- **Count row** (`role="status"`), the row's flexible middle, centred, gap 20, 18px type. See
  Count row below.
- **Right group**, gap 8:
  - **Metronome**, a split button: the LampButton "Metronome" (radius `4px 0 0 4px`; on =
    `metronome.on`; sends `toggleMetronome`; tooltip `metronome.on`) joined by 1px to a 20 × 32
    caret "▾" (`--btn`, `--m`, 10px; radius `0 4px 4px 0`; `aria-haspopup="dialog"`,
    `aria-expanded`) that opens the metronome popover (spec #509). Tooltip `metronome.settings`
    (new). No Launchkey mapping.
  - **Unison** LampButton. On = `transport.unison`. Sends `toggleUnison`. Tooltip
    `transport.unison`. No Launchkey mapping.
  - **Panic**, off face, padding 0 14. Sends `panic`. Tooltip `transport.panic`.
  - **?**, 32 × 32 off face, `aria-pressed` = help mode (`tips.help`). Toggles help mode (spec
    #508). Tooltip `app.help`.

### Count row

Left to right, each item hidden when it has nothing to say:

1. **Beat blocks:** one 24 × 24 block (radius 2, gap 4) per beat of the bar
   (`surface.clock.beatsPerBar`). The current beat (from the clock, app-api.md › surface.clock) is
   the playing section's hue with `--bg` glow; past beats `--past`; later beats `--btn`. Beat one
   always has a 2px `--t` top edge (`box-shadow: inset 0 2px 0 var(--t)`). Stopped: all `--btn`.
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

The whole row is one `role="status"` with an `aria-label` that reads it out ("Beat 3 of 4, bar 3
of 4. Main B playing, Main C next. The fill lands after bar 3"); its parts are `aria-hidden`.
Tooltip `display.position`.

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
| 1–4 | "Right 1", "Right 2", "Right 3", "Left" | `surface.faders[i].value` | `--r1 --r2 --r3 --l` | the part's channel (`keyboardParts[i].channel`) | opens Channel for part i (tooltip `mixer.strip.select`) |
| 5 | "Style" | `mixer.styleVolume` | `--a` | the loudest of channels 9–16 | Style fader page (`setFaderPage style`; tooltip `mixer.style_level`) |
| 6 | "Multi Pad" | `mixer.multiPadVolume` | `--t2` | the loudest of channels 5–8 | opens the Multi Pads page (tooltip `mixer.pad_level`) |
| 7–8 | "—" | none | `--d` | none | not a control |
| 9 | "Master" | `mixer.master` | `--t` | `meters.master` / `masterRms` | opens Effects at the master (spec #519; tooltip `mixer.master`) |

The Style page (strips 1–8 the Style parts, buttons 1–8 their mutes) is drawn on PadsPage2 and
specified there (#507); the strip and lamp components are the same.

#### FaderStrip

One column: a 252-tall fader (the control) over a 20-tall name button.

- **Value** at the top, 20 tall, centred, 18 / 300, in the strip's hue. Volume layer: the number
  (0–127). Pan layer: `L20` / `C` / `R20` (64 = C). Send layers: the layer word and the number,
  "Rev 40", "Cho 12", "Dly 0", in `--t`. Empty when unused.
- **Track** from 24 to 247 (`--travel` 223px). Left of centre, two 10px meter bars at
  `50% − 17px` and `50% − 5px` on `--mbg`: the left bar is the channel's RMS, the right its peak,
  both in the hue at `--meter-mix`. A 22 × 1 `--peak` tick at the held peak across both. Meter
  height = travel × clamp((20·log10(level) + 60) / 60, 0, 1). The client keeps the peak for
  1.5 s, then lets it fall at 20 dB/s (Stage.md D7).
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
  that label as given, in `--t2`, no meter, and the name opens the Rack page. Tooltip
  `launchkey.fader_rack`.
- **Name button**, 20 tall, centred, gap 4: the name 13 / 500 in the hue, then the strip marks
  (keyboard parts only, DECISIONS M11): a 5px `--t` dot when `soundEdited`; a 12px `--warn` ⚠
  when `plugin.missing`; a 12px `--ending` ✕ when `plugin.status` is `failed` and not missing.
- **The fader as a control:** `role="slider"`, `aria-valuemin 0`, `aria-valuemax 127`,
  `aria-valuenow` the value, `aria-valuetext` "Right 1 90" (with ", hardware fader away" while
  waiting). Pointer: press and drag vertically, 223px = 127 (absolute: the value follows the
  pointer's travel from where it was pressed); wheel ±1; double-click resets to 100 (pan to 64,
  sends to 0); arrows ±1, PageUp/PageDown ±10. Each change sends `surface.faders[i].set` with its
  value field (`volume`, `pan` or `value`) filled in; with the controller map's `moveRackFader`
  the field is `volume`. Tooltips: `mixer.panel.right1` … `mixer.panel.left`,
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
| Sound | "Sound" | `surface.layer.type == 'sound'` | `setLayer { type: sound }`; on when lit: `setLayer none` | momentary: `setLayer sound` now, `setLayer none` on release | — | `launchkey.sound` | fader button 6 (hold) |
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
  `name`. No Assign: "---" in `--d`.
- **Value** 22 / 300, 22 tall, `--a`: `value`; a trailing "%" splits off as a 12 / 400 unit with
  2px gap. Empty for No Assign.
- **Ring** 44px, 2px margin-top: a 270° arc from 225° (`conic-gradient(from 225deg, var(--a) 0
  <deg>, var(--ring-rest) <deg> 270deg, transparent 270deg)` with a `--g` disc inset 2px),
  `deg = fraction × 270`, and a 6px `--a` tip dot at the arc's end (centre at radius 21 from the
  ring's centre). `fraction = level / 127`; tempo (`level` null) uses
  `clamp((tempo − 40) / 240, 0, 1)`. No Assign: arc and rest `--mbg`, no dot.
- **Code** 12 / 400, 14 tall, `--m`: `short`.
- **As a control:** `role="slider"`, `aria-valuetext` "Dynamics 127". Drag vertically: one
  `turnKnob { knob, delta }` step per 4px (Shift: per 12px); wheel ±1; double-click
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

### Transport and tempo

Header "Transport" (36 tall, hairline), then (8px) a column, gap 6; then 20px, header "Tempo",
then (8px) a column, gap 6. All 88 × 32 off-face buttons, label left-aligned with 8px padding
(pairs: two 41 × 32 buttons, gap 6, label centred, 13px).

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
`role="status"`, `aria-live="polite"`, 14px, ellipsis. `state.message.text` in `--t`; with
`message.error`, a 12px `--warn` ⚠ before it (gap 8). Empty when `message` is null. It speaks only
for state messages, never coaching. Tooltip `display.status`. A click clears it (`clearMessage`).

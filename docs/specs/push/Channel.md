# Channel

One part's channel: its sound, level, tone, sends, EQ, compressor, inserts and play settings,
with the band still under your hands. The screen a player goes to when a part needs shaping.

- **Issue:** #501 · **Flow:** Shape the mix · **Boards:** `docs/design/push/Channel-Dark.dc.html`,
  `Channel-Light.dc.html`; pictures `docs/design/push/png/Channel-Dark.png`, `Channel-Light.png`
  (1440 × 900). The spec stands without them: every value a builder needs is below, in
  [kit.md](kit.md) or in [Stage.md](Stage.md); the board lines in the Components table are for
  cutting crops.
- **Copied from:** Stage (#500). This spec says what differs and refers to Stage.md and kit.md for
  the rest: the app bar, section row, faces, tokens, type, interaction conventions, the strips,
  knobs, pads, transport and key strip are the kit's; this file adds the **tall page** layout, the
  page's own regions (compact now-playing block, parts list, part header, the four groups) and, in
  "Kit additions", the parts the kit says this spec owns: the app bar's page variant, the half
  band, the compact block and the key row's status line. Until kit.md takes them in, they are
  normative here and every tall page reads them from this file.
- **Built from:** kit › App bar (page variant, below), Section row, Faces, Tokens, Interaction
  conventions; the half band, key row and status line below; Stage.md › Display › Chord (the
  chord runs), Sounds row (the sound cell's number, name and marks), D6 (knob names), D7 (meters),
  D18 (Sound latch), D23 (fader drag), D32 (interim links).
- **Variants of this screen** (they spec only what differs): Channel-StylePart (#518): the page
  with a Style part open.
- **Glance order** (what must read first, brightest to quietest): the open part's block in the
  header and the parts list (solid in its hue), the compact chord (48px, `--a`), the level and
  send values (18px light), the group headers (14px `--m`). Nothing on the page glows except the
  chord (`--ba`), the lit lamps and the band.
- **Before building:** CH-C1 (tooltips) must land first; Stage's C5 is assumed landed (the
  `nav.channel` key and the `nav.*` rewrite). The rest of the contract changes don't block (see
  Contract changes needed). The Stage lane's components the Components table marks "Stage" are
  reused as built; if the Channel lane runs first it builds them from Stage.md and kit.md.

## Layout

At 1440 × 900. The `Channel` component always lays out at 1440 × 900; the app shell scales it to
the window (Stage.md D1). Padding 24 all round; a column.

| Region | Box | What's in it | Spec |
|---|---|---|---|
| App bar | `24,24 1392×36` | wordmark, rack readout, One Touch 1–4, page tabs (Channel chosen), Launchkey status, health slot | Kit additions › App bar, page variant |
| Section row | `24,68 1392×32` | Accomp, count row, Metronome ▾, Unison, Panic, ? | kit › Section row |
| Page | `24,112 1392×468` | compact block, parts list, part header, the four groups | below |
| Half band | `24,600 1392×176` | faders, Track, knobs, pads, transport, at half height | Kit additions › Half band |
| Key row | `24,796 1392×80` | the status line with the key readouts (20), then the key strip (56) | Kit additions › Key row |

Vertical rhythm: app bar, 8, section row, 12, page, 20, half band, 20, key row (status 20, 4,
strip 56). The totals: 24 + 36 + 8 + 32 + 12 + 468 + 20 + 176 + 20 + 80 + 24 = 900.

The page is a tall page: it replaces the Stage's display **and** takes the full band's place,
leaving the half band so the nine faders and their buttons stay in view (DECISIONS H1). Every
display tab other than Stage uses this frame (CH-D1).

## Page

`24,112 1392×468`, no surface, no border, `overflow: hidden`. Inside, an inset of 24 left and
right and 16 top and bottom gives the content box `48,128 1344×436`, a row with gap 24:

| Part | Box | Spec |
|---|---|---|
| Left column | `48,128 320×436` | Compact block (84), then (16 below) the parts list |
| Divider | `392,128 1×436`, `--line`, `aria-hidden` | — |
| Content column | `417,128 975×436` | Part header (36), then (12 below) the groups grid (380) |

### Compact block

`48,128 320×84`, `aria-label="Now playing"`, a column; not a control, no tooltip of its own
except the chord's (`display.chord`) and tempo's (`display.tempo`). Everything in it is the
Stage display's, at the compact size; the count row above carries beat, bar and next.

- **Row 1**, 16 tall, items centred, gap 8, no wrap:
  - the style name as an accent block: padding 0 6, 13 / 500, line-height 16, `--g` on `--a`,
    `min-width: 0`, ellipsis (`style.name`). It is a text button: click opens the Browser (Stage
    D3; tooltip `browser.open`; `aria-label` "{name}: open the Browser").
  - `margin-left: auto`: the tempo, 18 / 300, line-height 16, `--t`, `Math.round(transport.tempo)`
    then "BPM" 12 / 400 `--m` with 3px left margin.
  - the run dot, 6px round: `transport.running` → `--ok` with `--bg`, `role="img"`
    `aria-label="Running"`; stopped with `transport.syncStart` → a hollow 1px `--ok` ring,
    "Sync start"; stopped → `visibility: hidden` (space kept), no label.
- **Row 2**, 12 below, 48 tall, items on the baseline, gap 14, no wrap:
  - the chord: 48 / 300, line-height 48, letter-spacing −2, `--a`, `text-shadow: var(--ba)`, in
    the Stage's two runs (`splitChord`, Stage D30): base 300, extension 200 with letter-spacing 0.
    No chord: "—" in `--d`, no shadow. `flex: none`, never shrinks (CH-D2).
  - the tones: 14 / 300, letter-spacing 3, `--t2`: the chord's tones as note names, root first,
    spaced ("A C E G"), the same tones and spelling as the Stage's tone columns (Stage › Chord, D20,
    D31); `min-width: 0`, `overflow: hidden`, ellipsis (CH-D2). No chord: empty.
  - `margin-left: auto`: the playing section, 24 / 300, line-height 48, letter-spacing −0.5, in its
    hue (kit › Section names, Hue roles), no glow. Stopped: the Main to start on in `--m` (Stage
    D4). `flex: none`.
- `aria-label` on the block: "Now playing: {style}, {tempo} BPM, {Running|Sync start|Stopped},
  chord {name} ({tones}), {section}" ("Now playing: Sunday Drive Pop, 104 BPM, Running, chord Am7
  (A C E G), Main B"; no chord: "no chord").

### Parts list

`48,228 320×208` (16 below the compact block), a `nav` `aria-label="Parts"`.

- **Header** `48,228 320×24`: 24 tall, `border-bottom: 1px solid var(--line)`, items centred,
  space between: "Parts" 14 `--m`; "Part " 13 `--m` then "{n} of 12" 13 `--t` (n = the open
  part + 1).
- **Grid** `48,256 320×192` (4 below): `repeat(2, minmax(0, 1fr))`, rows 32, column gap 16 (cells
  152 × 32), row-major: R1 | R2, R3 | L, Rhythm 1 | Rhythm 2, Bass | Chord 1, Chord 2 | Pad,
  Phrase 1 | Phrase 2 (parts 0–11: `keyboardParts[0..3]`, then `mixer.styleParts[0..7]`).
- **Entry**, a text button: 32 tall, padding 0 8, radius 0, no border except
  `border-bottom: 1px solid` (`--line`; the open part: its hue), items centred, gap 8, left
  aligned, no wrap; `aria-current="true"` on the open part.
  - **Tag**, `min-width: 22px`, 13 / 500: "R1", "R2", "R3", "L", "Rhythm 1", "Rhythm 2", "Bass",
    "Chord 1", "Chord 2", "Pad", "Phrase 1", "Phrase 2" (fixed; the Style names are
    `styleParts[i].name`). Colour: the part hue for a keyboard part (`--r1 --r2 --r3 --l`), `--t2`
    for a Style part (Style parts carry no hue); `--d` when the part doesn't sound (keyboard:
    `sounding` false; Style: `on` false).
  - **Name**, 14 / 400, `min-width: 0`, ellipsis: a keyboard part's sound name (Stage › Sounds
    row: `sound.name`, else `voiceName`), `--t`; `--m` when it doesn't sound. A Style part shows no
    name (CH-D3).
  - **Open part**: background the part's hue (a Style part: `--t2`), tag and name `--g`, bottom
    rule in the hue. The `data-face="chosen"` hook; `data-hue` the hue name (`r1`, `t2`).
  - Click: opens that part (`ui.selectedPart = i`; a keyboard part also sends `selectPart { part }`,
    as today's `channelNav.show`). Tooltip `channel.part` (new). `aria-label` "Part {n} of 12:
    {tag} {name}{, off}{, open}" ("Part 1 of 12: R1 Stage Grand, open", "Part 3 of 12: R3 Brass
    Section, off").

### Part header

`417,128 975×36`: 36 tall, `border-bottom: 1px solid var(--line)`, items centred, gap 12, no wrap.
The part is `keyboardParts[part]` (a Style part: #518).

| Control | Face | Reads | Sends / does | Tooltip | Launchkey |
|---|---|---|---|---|---|
| "Channel" | 14 `--m`; not a control | — | — | — | — |
| Part block | padding 0 8, 14 / 500, line-height 22, `--g` on the part hue (`data-hue`); not a control | `keyboardParts[part].name` ("Right 1") | — | `mixer.channel` (the MIDI channel) | — |
| Sound | a text button, 32 tall, items on the baseline, gap 8: the number 12 `--m`, the name 16 `--t`, then the marks (kit › FaderStrip, name button marks: edited dot, ⚠, ✕; "off" 12 `--m` when the part doesn't sound) | number and name as Stage › Sounds row (patch number from `soundLibrary.patches`, `sound.name` else `voiceName`) | opens the quick sound list for the part (#514; interim D32: Library › Sounds) | `launchkey.fader_sound` | — |
| "Mine" | 12 / 500 `--a`; not a control; hidden otherwise | `sound.id` is `saved:<id>` or `inMySounds(soundLibrary.patches, sound.id)` (`app/src/panels/sounds/instruments.ts`) | — | `channel.sound_mine` (new) | — |
| Plugin line | 12 `--m`, hidden when `plugin` is absent: "{plugin.name} · {status}{ · in process}"; the status word coloured: `loading` `--m`, `playing` `--ok`, `failed` `--ending`, `muted` `--warn`; "in process" when `outOfProcess` is false, with a 12px `--warn` ⚠ before it when `inProcessFallback`; `missing` → "{name} · missing" in `--warn` | `keyboardParts[part].plugin` | not a control | `part.plugin` | — |
| Edit | off face, 26 tall, padding 0 10, 13px; shown only when `plugin.editor` | — | `session.pluginEditor(part, true)` (the shell's `open_plugin_editor`; the dev mock's no-op) | `part.plugin_edit` | — |
| CPU | `margin-left: auto`; "CPU " 13 `--m`, the number 16 / 300 `--t`, "%" 12 `--m`; not a control | `meters.channels[ch].cpu` for `keyboardParts[part].channel`, `Math.round(cpu × 100)`; the channel missing (no synth): "—" in `--d` | — | `mixer.cpu` | — |
| ◀ | 32 × 32 off face, 12px | — | opens part `(part + 11) mod 12` | `mixer.channel.prev` | — |
| ▶ | as ◀ | — | opens part `(part + 1) mod 12` | `mixer.channel.next` | — |

`aria-label`s: sound "{part name} sound: {number }{name}{, edited}{, off}{, plugin missing | , plugin
failed}. Opens the quick sound list" (Stage's template); Edit "Open the plugin editor"; ◀
"Previous part ({its tag})", ▶ "Next part ({its tag})" ("Previous part (Phrase 2)", "Next part
(R2)"). There is no close button and no × (CH-D4): the Stage tab, Alt+G or Esc (once nothing is
open over the page) goes back to the Stage.

### Groups

`417,176 975×380` (12 below the header): a grid `repeat(4, minmax(0, 1fr))`, gap 24 (columns
225.75 wide): **A** Mix and Sends, **B** EQ and Tone, **C** Compressor, **D** Inserts and Play.
Each column is a `min-width: 0` column with `overflow-y: auto; scrollbar-width: thin`; on the
board nothing scrolls, and a column scrolls only when its content exceeds 380 (CH-D5).

Three row shapes are used everywhere in the groups:

- **Group header**, 24 tall, `border-bottom: 1px solid var(--line)`, the title 14 / 400 `--m`,
  line-height 22; items centred; a small note after the title is 12 `--m`; a right-aligned part
  comes after `margin-left: auto`. (The Sends header alone is 32 tall, its button 31.)
- **Bar readout** (the `BarReadout` component), 32 tall, items centred, gap 10: the label 76
  wide, 14 / 400 `--t`, no wrap; the bar (`role="slider"`, `flex: 1`, 32 tall, `position:
  relative`, transparent, no border); the value 58 wide, right-aligned, 18 / 300 `--t`, no wrap,
  then the unit 12 / 400 `--m` with 2px left margin (no unit: nothing). Inside the bar: the
  track `left 0, right 0, top 15, 2px` in `--track`; the fill 2px tall at top 15 in the part's
  hue from `fl%` for `fw%`; the cap `2 × 12` at top 10, left `calc(pct% − 1px)`, in the hue.
  `pct = round(f × 100)` where `f` is the control's fraction below. Unipolar: `fl = 0`, `fw =
  pct`. Bipolar (from the centre): `fl = min(50, pct)`, `fw = |pct − 50|`. **Disabled** (a send
  that isn't there): track `--mbg`, no fill, no cap, label and value `--d`, value "—",
  `aria-disabled="true"`, stays focusable, sends nothing. `data-hue` the part hue; `data-face`
  `off` or `disabled`.
- **Lamp**, LampButton `size cell` at a fixed width (72, 52) or, in the Play rows, `size sm`
  28 tall at 52 (kit › Faces: on = lamp face, off = `--btn` with `--m` label).

**The bar as a control** (every bar readout; CH-D6). `aria-valuemin`, `aria-valuemax`,
`aria-valuenow` in the control's own units (0–127, −12..12, Hz…) and `aria-valuetext` "{label}
{value}{ unit}" ("Level 90", "Low +2 dB", "Pan C"). Pointer: press anywhere on the bar and drag
horizontally, relative (never a jump to the pointer), with pointer capture: one unit per 2px
of travel from the press point, right positive (Shift: one per 8px); for a stepped control (the
EQ frequencies) one step of its table per 2px. A send goes out when the whole-number value
changes, at most once per animation frame (the latest value), and the last value always on
pointerup. Wheel: ±1 per notch (a stepped control: one step). Arrows ±1, PageUp/PageDown ±10,
Home/End the ends; `stopPropagation` as the kit's faders. Double-click resets to the control's
default (given per row). Cursor `ew-resize` while dragging. Each change sends the row's
command with the value filled in.

#### A · Mix and Sends

`417,176 225.75×380`.

| Row | Label | Reads | f | Value | Sends | Default | Tooltip |
|---|---|---|---|---|---|---|---|
| Mix header | "Mix" | — | — | — | — | — | — |
| Level | "Level" | `keyboardParts[part].volume` | `v / 127` | `v` | `setPartVolume { part, volume }` | 100 | `mixer.channel.level` |
| Pan | "Pan" | `keyboardParts[part].pan` | `v / 127`, bipolar | `panText(v)`: "C", "L12", "R5" (`app/src/panels/channel/channel.ts`) | `setPartPan { part, pan }` | 64 | `mixer.channel.pan` |

Launchkey: Level is Panel fader `part + 1`; Pan is the Pan fader layer on that fader. While
`keyboardParts[part].waiting` the Level row's label reads "Level ↕" (the ↕ 13px `--m`, as the
strip's; tooltip then `mixer.pickup`).

Then a **lamp row**, 32 tall, gap 8: **On** (72 × 32) and **Solo** (72 × 32).

| Button | Label | On = | Click | Long press / right-click | Tooltip | Launchkey |
|---|---|---|---|---|---|---|
| On | "On" / "Off" (`keyboardParts[part].on`); "Swap" while this part's swap is held | `sounding` | `togglePart { part }` (in swap: `setLayer none`) | `setLayer { type: swap, part }`, latched until the next click (kit › Lamp row, Stage D12, D18) | `part.right1.on` … `part.left.on` by part | fader button `part + 1` |
| Solo | "Solo" | `mixer.partSolo === part` | lit: `setPartSolo { part: null }`; else `setPartSolo { part }` | — | `mixer.solo` | — |

`aria-label`s: "{part name} on" / "{part name} off" (plus ", swap held"), "Solo {part name}".

Then, 12 below, the **Sends header**, 32 tall, hairline, space between: "Sends" 14 `--m` with
"1–6" 12 `--m` after it; **+ Add send**, a text button 31 tall, 14 `--t2`, the "+" at weight
300: opens the kind menu (below) with the twelve send kinds (Hall, Room, Stage, Plate, Chorus,
Celeste, Flanger, Delay 1/8, Delay 1/8., Delay 1/4, Ping-Pong, Phaser; the `SendKind` names in
app-api.md › Send kinds); picking one sends `addSend { kind }`. Disabled (`aria-disabled`, `--d`)
when `effects.sends.length` is 6. Tooltip `channel.add_send` (new). `aria-label` "Add a send
effect ({n} of 6)" with n = `effects.sends.length + 1`; full: "Add a send effect: all six
added".

Then six **send rows** (bar readouts), sends 1–6 (`send` 0–5):

| Row | Label | Reads | f | Value | Sends | Default | Tooltip |
|---|---|---|---|---|---|---|---|
| Send 1–3 | "Reverb", "Chorus", "Delay" (the buses; CH-D7) | `strip.sends[i]` | `v / 127` | `v` | `setStripSend { strip: part, send: i, level }` | 0 | `mixer.strip.send` |
| Send 4–6, there | `effects.sends[i].name` ("Phaser", "Plate", "Delay 1/8."), ellipsis | as above | | | | 0 | `mixer.strip.send` |
| Send 4–6, not there (`effects.sends[i]` absent) | "Send 5", "Send 6" | — | disabled face | "—" | nothing | — | `mixer.strip.send` |

Launchkey: sends 1–3 are the Reverb, Chorus and Delay fader layers on the part's fader; send 4
is swap knob 8; sends 5–6 have none (the rack's controller map can target them).
`aria-valuetext` "Send {n}, {label}, {v}" ("Send 1, reverb, 40"); a missing send's `aria-label`
"Send 5, not added". Removing a send is on Effects (#502), not here.

#### B · EQ and Tone

`666.75,176 225.75×380` (the grid's second column).

| Row | Label | Reads | f | Value · unit | Sends | Default | Tooltip |
|---|---|---|---|---|---|---|---|
| EQ header | "EQ" | — | — | — | — | — | — |
| Low | "Low" | `strip.eq.lowGain` (−12..12 dB, an integer) | `0.5 + g / 24`, bipolar | `dbText(g)` "+2", "0", "−1" · "dB" | `setStripEq { strip: part, eq: withEq(eq, { lowGain }) }` | 0 | `mixer.strip.eq_low_gain` |
| Low freq | "Low freq" | `strip.eq.lowFreq` (Hz) | `stepOf(LOW_STEPS, hz) / 36` | `hzUnit(hz)` | `setStripEq` with `lowFreq: LOW_STEPS[step]` | 80 | `mixer.strip.eq_low_freq` |
| High | "High" | `strip.eq.highGain` | `0.5 + g / 24`, bipolar | `dbText(g)` · "dB" | `setStripEq` with `highGain` | 0 | `mixer.strip.eq_high_gain` |
| High freq | "High freq" | `strip.eq.highFreq` | `stepOf(HIGH_STEPS, hz) / 30` | `hzUnit(hz)` | `setStripEq` with `highFreq: HIGH_STEPS[step]` | 10000 | `mixer.strip.eq_high_freq` |

`LOW_STEPS` (37 steps, 32 Hz–2 kHz), `HIGH_STEPS` (31 steps, 500 Hz–16 kHz), `stepOf`, `dbText`
and `withEq` are in `app/src/panels/mixer/eq.ts` (they move to `app/src/ui/BarReadout/eq.ts`
with the build). `hzUnit(hz)` (new, beside them): below 1000, the integer and "Hz" ("120" ·
"Hz"); from 1000, `(hz / 1000).toFixed(1)` and "kHz" ("8.0" · "kHz", "1.2" · "kHz", "16.0" ·
"kHz"). The value shows the state's Hz as it is (120 is not a step but shows "120"); a drag
or wheel moves to the nearest step first, then steps (CH-D8). The board's "+2.0" and "−1.5"
gains are placeholders: gains are whole dB.

Then, 12 below, the **Tone header**: "Tone" 14 `--m` with "offsets on the sound" 12 `--m`;
right: **More ›** / **‹ Less**, a text button 23 tall, 13 `--t2`, `aria-expanded`: toggles the
two extra rows (app-only state, `ui.channelToneMore`, false at start; CH-D5). Tooltip
`channel.tone_more` (new). `aria-label` "More tone settings: vibrato rate and vibrato delay" /
"Fewer tone settings".

Tone rows: each `strip.tone[control]`, 0–127 with 64 the voice's own, **bipolar** `f = v /
127`, value `toneText(v)` ("+12", "0", "−4"; `channel.ts`), no unit, sends `setStripTone {
strip: part, control, value }`, default 64:

| Row | Label | `control` | Tooltip |
|---|---|---|---|
| 1 | "Cutoff" | `cutoff` | `mixer.channel.tone.cutoff` |
| 2 | "Resonance" | `resonance` | `mixer.channel.tone.resonance` |
| 3 | "Attack" | `attack` | `mixer.channel.tone.attack` |
| 4 | "Decay" | `decay` | `mixer.channel.tone.decay` |
| 5 | "Release" | `release` | `mixer.channel.tone.release` |
| 6 | "Vibrato" | `vibratoDepth` | `mixer.channel.tone.vibrato_depth` |
| 7 (More) | "Vib rate" | `vibratoRate` | `mixer.channel.tone.vibrato_rate` |
| 8 (More) | "Vib delay" | `vibratoDelay` | `mixer.channel.tone.vibrato_delay` |

The board draws Vibrato unipolar from 0; it is bipolar like the others (64 = the voice's own,
"0"; CH-D9). Launchkey: none (the rack's controller map can target the tone controls).

#### C · Compressor

`916.5,176 225.75×380`.

- **Header**, 24 tall, hairline, gap 8: "Compressor" 14 `--m`; "Edited" 12 / 500 `--t` when
  `strip.comp.edited`, else nothing.
- **Lamp row**, 8 below, 32 tall, gap 8: **On** (LampButton `cell`, 72 × 32; on = `comp.on`;
  sends `setStripCompressorOn { strip: part, on: !on }`; tooltip `mixer.strip.comp`;
  `aria-label` "Compressor on" / "Compressor off"), then "no gain-reduction meter" 12 `--m`
  (fixed text; DECISIONS M8).
- **Preset tabs**, 8 below, a `role="tablist"` `aria-label="Compressor preset"`, wrapping,
  row-gap 4, no column gap: five `role="tab"` buttons, 26 tall, padding 0 8, 14 / 400, no wrap,
  no radius: Natural, Rich, Punchy, Electronic, Loud (`COMP_PRESETS` order). The chosen one
  (`comp.preset`, `aria-selected="true"`) is the chosen face (`--t` fill, `--g` label, weight
  400 here, not 500: the board's); the rest `--m` on nothing. On the board they wrap into two
  rows (Natural Rich Punchy / Electronic Loud). Click sends `setStripCompressorPreset { strip:
  part, preset }`. Tooltip `mixer.strip.comp_type` on each. Left/Right arrows move between tabs.
- **Rows**, 8 below, five bar readouts (`strip.comp`), each sending `setStripCompressorParam {
  strip: part, param, value }`; double-click resets to the chosen preset's value (the Strip
  compressor types table in app-api.md):

| Row | Label | `param` | Range | f | Value · unit | Tooltip |
|---|---|---|---|---|---|---|
| 1 | "Threshold" | `threshold` | −48..0 dB | `(v + 48) / 48` | `dbText(v)` · "dB" ("−18") | `mixer.strip.comp_threshold` |
| 2 | "Ratio" | `ratio` | 10–200 tenths | `(v − 10) / 190` | `ratioText(v)` · ":1": `v / 10` with no decimal when whole, else one ("3", "2.5", "6") | `mixer.strip.comp_ratio` |
| 3 | "Attack" | `attack` | 1–100 ms | `(v − 1) / 99` | `v` · "ms" | `mixer.strip.comp_attack` |
| 4 | "Release" | `release` | 10–1000 ms | `(v − 10) / 990` | `v` · "ms" | `mixer.strip.comp_release` |
| 5 | "Make-up" | `makeup` | 0–24 dB | `v / 24` | `v` · "dB" | `mixer.strip.comp_makeup` |

- **Note**, 12 below: "Presets start at 0 dB make-up; add your own." 13 / 400, line-height 18,
  `--m`, wrapping (two lines at this width). Fixed text (DECISIONS S11; the presets' make-up in
  app-api.md's table is the engine's; the note states the rule the UI shows).

The compressor off: the rows stay as drawn and still edit (the board shows no off look; CH-D10).
Launchkey: none.

#### D · Inserts and Play

`1166.25,176 225.75×380`.

- **Header**, 24 tall, hairline, gap 8: "Inserts" 14 `--m`; `margin-left: auto`; **Rotary
  fast**, a LampButton 22 tall, padding 0 10, 12px, radius 4 (`size xs`): on = `effects.rotaryFast`;
  sends `toggleRotaryFast`; tooltip `fx.rotary_fast`; `aria-label` "Rotary fast, global: the
  rotary speed for every part"; then "global" 12 `--m`. Launchkey: Shift + encoder page ▲.
- **Slot 1**, 8 below, a row 32 tall, gap 8: "1" 12 `--m`, 12 wide; the **kind button**
  (`flex: 1`, 32 tall, off face, padding 0 10, 14px, items centred, space between: the kind's
  `name` in `--t` (`--m` for "None"), then "▼" 10px `--m`); the **On** lamp (LampButton `cell`
  52 × 32, on = `on`, sends `setStripInsertOn { strip: part, slot, on: !on }`, tooltip
  `mixer.strip.insert_on`, `aria-label` "Insert {n} on/off"), shown only when the slot has a
  kind (`kind` not `none`).
- **Settings**, 4 below, one bar readout per `settings[j]` (0–3, in order): label
  `settings[j].name` ("Depth", "Drive", "Balance"), `f = (value − min) / (max − min)`,
  unipolar, value `settings[j].display` as the state gives it ("64", "1/8", "0.50 Hz", "40 %"; it
  carries its own unit, so the unit slot is empty), sends `setStripInsertSetting { strip: part,
  slot, setting: j, value }` (a drag shows the local value as a number until the state's
  `display` returns), default `settings[j].default`, tooltip `mixer.strip.insert_setting_1` …
  `_4` by index. The board's "Balance 0" from the centre is a placeholder: Balance 64 reads
  "64" from the left (CH-D11). An unknown kind (a newer build's) shows its name and no rows.
- **Slot 2**, 8 below: the same row for `inserts[1]`; empty on the board ("None", no On lamp).
- **Play header**, 12 below, 24 tall, hairline: "Play" 14 `--m`.
- **Play rows**, 32 tall each, label 76 wide 14 `--t`, gap 10 (the Octave and Bend rows gap 8):

| Row | Control | Reads | Face / value | Sends | Tooltip |
|---|---|---|---|---|---|
| Mono | LampButton `size sm` 52 × 28, label "On" / "Off" | `strip.mono` | lamp on / off | `setStripMono { strip: part, on: !mono }` | `mixer.channel.mono` |
| Portamento | bar readout (`flex: 1`), value 58 wide, no unit | `strip.portamento.time` | `f = time / 127`, unipolar; value `time` | `setStripPortamento { strip: part, on: time > 0, time }` (CH-D12) | `mixer.channel.portamento` |
| Octave | **−** (32 × 28 off face, 14px), the value `flex: 1` centred 18 / 300 `--t`, **+** | `keyboardParts[part].octave` (−2..2) | `octaveLabel(o)` "+1", "0", "−2" (`app/src/panels/parts/parts.ts`) | − `setPartOctave { part, octave: o − 1 }`, disabled at −2; + `… o + 1`, disabled at 2 | `part.octave_down`, `part.octave_up` |
| Bend | −, value, + as Octave; the value then "st" 12 `--m` with 2px margin | `controllers.parts[part].bendRange` (0–12) | `b` · "st" | − `setBendRange { part, semitones: b − 1 }`, disabled at 0; + `… b + 1`, disabled at 12 | `pedal.bend_down`, `pedal.bend_up` |

`aria-label`s: "Mono on" / "Mono off"; Portamento's `aria-valuetext` "Portamento time {t}"
("Portamento time 0, off" at 0); "Octave down" / "Octave up" with the value `role="status"`
"Octave {o}"; "Bend range down" / "Bend range up", "Bend range {b} semitones". Launchkey: none
(the Octave and Bend have none on the hardware; swap knob 7 is insert 1's first setting).

**Kind menu** (the `KindMenu` component, new; CH-D13). The insert kind button
(`aria-haspopup="listbox"`, `aria-expanded`) and + Add send open a popover `role="listbox"`
directly below the button, the button's width (Add send: 160 wide, right-aligned to the
button), `--btn` fill, radius 4, 4px padding, `box-shadow: none`, over the page (`z-index`
above the groups): one `role="option"` per kind, 32 tall, padding 0 10, 14 / 400 `--t2`, no
wrap; the current kind (`aria-selected="true"`) `--t` weight 500; the option under the pointer
or the keyboard cursor `background: var(--line)` (the one hover look the kit allows, because a
menu needs a cursor; CH-D13). Click or Enter sends the row's command and closes; Esc, a click
outside or Tab closes without sending; arrows move the cursor; focus returns to the button.
Insert kinds, in order (`INSERT_KINDS` in `channel.ts`): None, Distortion, Compressor, Auto
Wah, Tremolo, Rotary, Phaser; a slot whose kind this build doesn't know lists it too, last, by
its own name. Tooltip on the button `mixer.strip.insert_kind`; the options carry none. The
button's `aria-label` "Insert {n} kind: {name}. Choose None, Distortion, Compressor, Auto Wah,
Tremolo, Rotary or Phaser".

## Half band, key row and status on the Channel

The Kit additions below, unchanged; the Channel adds nothing to them. The faders, lamps,
knobs, pads and transport read and send exactly as the full band's (kit › Full band); only the
geometry differs. The fader page, layer, knob page and pad page are whatever the state says, as
on the Stage.

## States

| State | What changes |
|---|---|
| Playing, Right 1 open (the board) | as drawn |
| Another keyboard part open | the header block, parts-list entry and every bar's hue follow the part (`--r2`, `--r3`, `--l`); the rows read that part's state |
| A Style part open | spec #518 (Channel-StylePart). Until it lands: the same page with the Style part's `strip`; the block and bars in `--t2`; Level sends `setStylePartVolume { part: part − 4 }`, On `toggleStylePart`, Solo `setStyleSolo`; the Pan row disabled ("—"); the sound shows `voice.label` with no number, Mine, plugin line or Edit; Tone and Play rows disabled; sends 1–3 as `setStripSend` (CH-D14) |
| Stopped | compact block: run dot hidden, section in `--m` (Stage D4); count row and band as the Stage's Stopped state |
| Sync Start armed | compact run dot a hollow ring, "Sync start"; count row and pad 4 as the Stage's |
| Part off (`on` false, not sounding) | On lamp off ("Off"); parts-list tag `--d`, name `--m`; the sound cell's "off" mark; the strip in the band as kit › FaderStrip, Part off. Every row still edits |
| Part soloed (`mixer.partSolo` = part) | Solo lit; the other keyboard parts don't sound (their lamps off, strips dimmed) |
| Another part soloed | Solo off here; this part's `sounding` false: as Part off, plus the solo lamp of that part lit in its own channel |
| Swap held for this part | On reads "Swap"; knob block "Swap R1" (kit › Knobs) |
| Fader `waiting` | Level row "Level ↕"; the strip's ghost in the band |
| Compressor off | On lamp off; rows unchanged (CH-D10) |
| Compressor edited | "Edited" after the header title; the preset tab stays on `comp.preset` |
| Insert slot empty | "None" in `--m`, no On lamp, no setting rows |
| Insert 2 filled | its On lamp and setting rows appear; column D scrolls when over 380 (CH-D5) |
| Tone More | two more rows; column B scrolls (CH-D5) |
| Six sends | + Add send disabled; no "Send n" rows |
| Three sends (fresh session) | rows 4–6 disabled "Send 4", "Send 5", "Send 6" |
| Plugin loading / failed / muted / missing | the plugin line's word and colour (Part header); failed: ✕ on the sound; missing: ⚠ and "missing" |
| No plugin (SoundFont voice) | no plugin line, no Edit, no Mine unless the patch is in My Sounds |
| No chord | "—" and no tones in the compact block |
| No synth (`io.synth` null) | CPU "—"; health slot "Audio off"; meters empty |
| Pads on another page, Sound held, Help mode, No Launchkey, notices | as Stage › States (the half band uses the kit's fallback faces, D33) |
| Light theme | tokens only (kit › Tokens); `--ba` is `none`, so the compact chord doesn't glow |

## Board fixture

The state and moment that reproduce the board, for the `Pages/Channel` › `Board` story and its
shots: `app/src/ui/Channel/Channel.fixtures.ts` exports `boardState`, `boardNow` and
`boardMeterHolds`. It starts from the Stage's `boardState` (`app/src/ui/Stage/Stage.fixtures.ts`:
the same moment, clock, style, transport, chord, OTS, rack, parts, faders, meters, holds,
lamps, knobs, pads, keys and I/O) and changes or adds only what follows. `boardNow` and
`boardMeterHolds` are the Stage's. Fields not listed take the Stage fixture's, then the dev
mock's.

- **Page:** `ui.page` `channel`, `ui.selectedPart` 0, `ui.channelToneMore` false, `ui.keyRange`
  61; no kind menu open.
- `keyboardParts[0]` (Right 1): on, sounding, selected true, volume 90, waiting false, pan 64,
  octave 0, channel 1; `sound` `{ id: "saved:stage-grand", name: "Stage Grand" }`, with a
  `soundLibrary.patches` entry of that id, number 1; `soundEdited` absent; `plugin` `{ id:
  "au:sampler-deluxe", name: "Sampler Deluxe", manufacturer: "Mock", status: "playing", stage:
  null, error: null, outOfProcess: false, inProcessFallback: false, cpu: 0.03, overruns: 0,
  recentOverruns: 0, editor: true, missing: false }`.
- `keyboardParts[0].strip`: `eq` { lowGain 2, lowFreq 120, highGain −2, highFreq 8000 }; `comp` {
  on true, preset "punchy", threshold −18, ratio 30, attack 10, release 120, makeup 0, edited
  true }; `inserts[0]` { kind "rotary", name "Rotary", on true, settings [Depth 0–127 default 64
  value 64 "64", Drive 0–127 default 0 value 20 "20", Balance 0–127 default 64 value 64 "64"] };
  `inserts[1]` { kind "none", name "None", on false, settings [] }; `sends` [40, 12, 0, 20, 0,
  0]; `tone` { cutoff 76, resonance 68, attack 64, decay 64, release 72, vibratoRate 64,
  vibratoDepth 64, vibratoDelay 64 }; `mono` false; `portamento` { on false, time 0 }.
- `controllers.parts[0].bendRange` 2. `mixer.partSolo` null, `styleSolo` null.
- `effects.sends`: four: send 0 kind "hall" name "Hall" (fromStyle true), 1 "chorus" "Chorus",
  2 "dottedEighth" "Delay 1/8." (both fromStyle), 3 "phaser" "Phaser" (setByRack true); each
  with its kind's params at the defaults and returnLevel 64. `effects.rotaryFast` true.
- `mixer.styleParts[0..7]`: names Rhythm 1, Rhythm 2, Bass, Chord 1, Chord 2, Pad, Phrase 1,
  Phrase 2, all `on` true (the parts list's Style tags are `--t2`).
- `meters.channels[0]` (channel 1): `cpu` 0.03 (the Stage fixture's peak and rms), `cpuPeak`
  0.05.
- **The half band** reads the Stage fixture's `surface.faders`, `meters`, `boardMeterHolds`,
  lamps, `knobs` and `pads` unchanged; at the half scale the meter heights are, strips 1–9 peak
  [rms] in px: 27 [25], 20 [18], 0, 22 [20], 31 [29], 0, —, —, 32 [29]; the held-peak tick
  bottoms (3 + height): 34, 27, 3, 29, 37, 8, —, —, 38; the cap tops (`round((1 − v/127) × 44) +
  18`): 31, 37, 40, 34, 27, 31, —, —, 27; strip 2's ghost at top `round((1 − 50/127) × 44) + 19 =
  46`. Pads: 10 Playing, 11 Next, 16 Playing in `--ok`, 3 and 7 Absent. Pads header "Sections ·
  next Main C".
- **The key row** reads the Stage fixture's `keyboard`: split F#2 (54), detection [0, 54]
  ("lower"), held left 43, 45, 48, 52 → "G A C E", held right 76, 81 → "E4 A4", 61 keys;
  `message` null (the status line is empty).

Board texts that differ from this fixture on purpose, all under the screenshot diff's per-pixel
threshold, so no mask: the count row's "fill after bar 4" (Stage D5; it keeps its
`data-shot-mask="when"`, and the story masks it as the Stage's does); the EQ gains "+2.0" and
"−1.5" (the fixture draws "+2" and "−2"); the Balance "0" (the fixture draws "64"); the One
Touch aria text. The board's bar fills are placeholder fractions (Threshold 0.55, High freq 0.7,
Vibrato from 0): the fixture draws them from the maths above, a few pixels away. The light
board's `--m` (`#6e6e6e`) and `--lamp-ink` (`#111`) differ from the tokens as on the Stage.

## Components

Every part of the screen, in build order; "Stage" in the Exists column means the component is
Stage.md's (built by that lane, reused as is) and this spec adds a size or variant where it says
so. Each new one gets `app/src/ui/<Name>/` with a SPEC.md per `docs/factory/spec-template.md`;
board lines are dark / light (light = dark − 36 on this board pair); crop boxes are the boxes in
this spec and the Kit additions.

| # | Component | Kind | Built from | Exists | Board lines (dark / light) | Spec |
|---|---|---|---|---|---|---|
| 0 | tokens | — | — | yes (Stage adds the kit's) | 66, 71 / 30, 35 | kit › Tokens; nothing new |
| 1 | LampButton | primitive | longpress | yes; **add** `size xs` (22 tall, padding 0 10, 12px) and fixed widths 72 / 52 at `cell`, `size sm` at 52 | 208–209, 266, 296, 302, 325, 407–414 / −36 | kit › Faces; this file |
| 2 | Button | primitive | longpress | Stage; **add** variants `menu` (32 tall, padding 0 10, space between, ▼), `step` (32 × 28), `edit` (26 tall, padding 0 10, 13px), `half` (40 × 32, 28 × 28, 24-tall transport cells) | 185, 187–188, 301, 319, 337–345, 424–425, 433–439, 481–491 / −36 | kit › Faces (off) |
| 3 | ChosenTabs | primitive | — | Stage; **add** `size chip` (26 tall, padding 0 8, 14px, full block, wrapping) | 269–275, 359–371 / −36 | Groups › C; Kit additions › Half band |
| 4 | AccentBlock | primitive | — | Stage; **add** `size line` (13 / 500, line-height 16, padding 0 6) and a hue prop (the part block: 14 / 500, line-height 22, padding 0 8, `--g` on the hue) | 145, 178, 432 / −36 | Compact block; Part header |
| 5 | StatusDot | primitive | — | Stage | 147 / 111 | Compact block |
| 6 | PartMarks | primitive | — | Stage | 393–395 / −36 | Part header; Half band strips |
| 7 | GroupHeader | primitive | — | Stage; **add** `size sm` (24 tall, 14 / 400 line-height 22, slots for a note and a right part) and `size md` (32) | 157–160, 176, 195, 211–214, 230, 242–245, 261–264, 294–298, 322 / −36 | Groups |
| 8 | BarReadout | primitive | — | no | 197–205 (markup), 552–561 (maths) / −36 | Groups (the bar as a control) |
| 9 | Stepper | primitive | Button | no | 335–346 / −36 | Groups › D, Play rows |
| 10 | KindMenu | primitive | — | no | not drawn (closed on the board) | Groups › D, Kind menu |
| 11 | FaderStrip | primitive | — | Stage; **add** `size half` | 375–399; data 601–651 / −36 | Kit additions › Half band |
| 12 | Knob | primitive | — | Stage; **add** `size half` | 443–453; data 654–671 / −36 | Kit additions › Half band |
| 13 | Pad | primitive | — | Stage; **add** `size half` | 457–474 / −36 | Kit additions › Half band |
| 14 | KeyStrip | primitive | — | Stage | 506–515; data 673–694 / −36 | kit › Key strip |
| 15 | StatusLine | primitive | — | Stage | 499 / 463 | kit › Status line |
| 16 | HealthSlot, PageTabs, LaunchkeyStatus, RackReadout, OneTouch | — | — | Stage | 80–106 / −36 | kit › App bar; Stage › Style line, Sounds row |
| 17 | ChordReadout | primitive | — | Stage; **add** `size compact` (48px, −2, `--ba`, no fit) | 150 / 114 | Compact block |
| 18 | KeyReadouts | primitive | Button | no | 500–504 / −36 | Kit additions › Key row |
| 19 | CompactNowPlaying | complex | AccentBlock, StatusDot, ChordReadout | no | 143–154 / −36 | Compact block |
| 20 | PartsList | complex | — | no | 156–169; data 534–549 / −36 | Parts list |
| 21 | PartHeader | complex | AccentBlock, PartMarks, Button | no | 176–189 / −36 | Part header |
| 22 | MixGroup | complex | GroupHeader, BarReadout, LampButton, KindMenu | no | 194–226 / −36 | Groups › A |
| 23 | EqToneGroup | complex | GroupHeader, BarReadout, Button | no | 229–257 / −36 | Groups › B |
| 24 | CompressorGroup | complex | GroupHeader, LampButton, ChosenTabs, BarReadout | no | 260–290 / −36 | Groups › C |
| 25 | InsertsPlayGroup | complex | GroupHeader, LampButton, Button, KindMenu, BarReadout, Stepper | no | 293–347 / −36 | Groups › D |
| 26 | ChannelPage | complex | CompactNowPlaying, PartsList, PartHeader, the four groups | no | 137–351 / −36 | Page |
| 27 | AppBar | complex | PageTabs, LaunchkeyStatus, HealthSlot | Stage; **add** the page variant (slots for RackReadout and OneTouch) | 78–107 / 42–71 | Kit additions › App bar, page variant |
| 28 | SectionRow, CountRow, MetronomeSplit, LampRow | complex | — | Stage | 110–134, 400–416 / −36 | kit |
| 29 | HalfFaderBank | complex | GroupHeader, ChosenTabs, FaderStrip (half), LampRow | no | 356–418 / −36 | Kit additions › Half band |
| 30 | TrackColumn | complex | GroupHeader, Button | no | 421–427 / −36 | Kit additions › Half band |
| 31 | HalfKnobPadBank | complex | GroupHeader, AccentBlock, Button, Knob (half), Pad (half) | no | 429–475 / −36 | Kit additions › Half band |
| 32 | HalfTransport | complex | GroupHeader, Button | no | 477–493 / −36 | Kit additions › Half band |
| 33 | HalfBand | complex | HalfFaderBank, TrackColumn, HalfKnobPadBank, HalfTransport | no | 354–494 / −36 | Kit additions › Half band |
| 34 | KeyRow | complex | StatusLine, KeyReadouts, KeyStrip | no | 497–516 / −36 | Kit additions › Key row |
| 35 | Channel (page, `Pages/Channel`) | complex | AppBar (page variant), SectionRow, ChannelPage, HalfBand, KeyRow | no | whole board | this file |

Components take props and call callbacks; none reads `app.state` or sends. The page wiring
(`app/src/pages/ChannelWiring.svelte`, outside `app/src/ui`; Stage D46) reads `app.state`, keeps
`now`, `receivedMs` and the meter holds (shared with the Stage wiring in one module,
`app/src/pages/clock.svelte.ts`), opens parts, and maps each callback to its command or interim
target.

## Gap against today

| Area | In `app/src` now | Change |
|---|---|---|
| The view | `panels/channel/ChannelView.svelte`: a row of cards (Level with a fader and its own 100 ms meter poll, EQ, Compressor, two Inserts, Sends, Tone, Play) with `FxKnob`s, `<select>`s and `Toggle`s, shown in the display's place by `panels/channel/nav.svelte.ts` (`show`, `close`, `stripClick`, Esc) | Replaced by the page above: a tab (`ui.page` `channel`, Stage D2), no close, no own meter poll (the band's strips show the part's meter); bar readouts and lamps instead of knobs, selects and toggles; the parts list instead of ‹ › alone (‹ › stay). `channel.ts` keeps `TONE_KNOBS` (now the row table), `toneText`, `STRIP_COUNT`, `INSERT_KINDS`, `INSERT_SETTING_TIPS`, `panText`; `meterFrac` goes (the kit's `height`) |
| Opening | `nav.svelte.ts` `show(part)` from a strip name, Stage sound tags, the health slot; `stripClick` toggles | `ui.page = 'channel'` plus `ui.selectedPart` (and `selectPart` for a keyboard part); the Stage's openers (Stage D32 interim row for Channel) now open the page; a hardware part select opens it too (CH-D15, CH-C2). Esc: `ui.page = 'stage'` when nothing is open over the page |
| EQ maths | `panels/mixer/eq.ts` (`LOW_STEPS`, `HIGH_STEPS`, `stepOf`, `hzText`, `dbText`, `gainKnob`, `withEq`) | Moves to `app/src/ui/BarReadout/eq.ts` with `hzUnit` added; `hzText` and the gain-knob pair go when the old mixer goes |
| Half band | none (the Stage lane builds the full band) | The half sizes of FaderStrip, Knob, Pad, the LampRow at 12px and the five-row transport (Kit additions) |
| Key row | `panels/keystrip/KeyStrip.svelte`'s cheek (chord tones, 49/61/88 switch) | The readouts and the 49/61/88 button move to the key row's status line (Stage D25) |
| Shortcuts | `lib/nav.ts`, `lib/keys.ts` | Alt+N opens Channel for `ui.selectedPart` (Stage D37); F1–F4 (`part.*.select`) keep selecting and now open Channel (CH-D15) |
| Screenshot tool | `scripts/shots.ts` | Needs the Stage lane's per-story viewport and masks (Stage D39); nothing more |

## Contract changes needed

CH-C1 blocks the build (tooltip keys type-check and the catalog test). CH-C2 doesn't block;
until it lands the screen behaves as its row says.

1. **CH-C1 · Tooltips** (`app/src/help/tooltips.ts`, `app/docs/controls.md`), **lands before the
   build**: new keys `channel.part` (a parts-list entry: opens that part's channel; a keyboard
   part becomes the edited part), `channel.add_send` (adds a send effect 4–6 of the kind you
   pick; the rack keeps it), `channel.tone_more` (shows vibrato rate and delay), `channel.sound_mine`
   (this sound is in My Sounds), `keystrip.readout` (one body for the Split, Detect, Left and
   Right readouts: what each reads). Rewrites: `mixer.channel.level` and `mixer.channel.pan`
   ("drag sideways", not "up or down"; pan: "double-click for centre" stays), `mixer.channel.prev`
   / `.next` ("the part before / after this one; after Right 1 comes Phrase 2" stays),
   `mixer.channel.portamento` (one control: "the glide time between notes, 0–127; 0 is off"),
   `mixer.channel.close` (drop; nothing carries it), `mixer.strip.insert_setting_1`–`_4` ("drag
   sideways"), `keystrip.range` (a click steps 49 → 61 → 88 → 49; it no longer mentions "the lit
   one"), `mixer.strip.select` (the strip name "opens the part's channel", as the Stage's C5
   table says for the tag). Launchkey fields: `mixer.channel.level` "Panel fader 1–4";
   `mixer.strip.send` "Reverb, Chorus and Delay fader layers (sends 1–3); swap knob 8 (send 4)".
2. **CH-C2 · Hardware part select opens Channel** (session, `src/api`, `docs/app-api.md`,
   `tests/fixtures/state.json`, `app/src/lib/api/types.ts`, both mocks): the state doesn't say
   what selected a part, so the screen can't tell Shift + fader button 1–4 from a rack load that
   changes `selected`. Add `surface.partSelectSeq: number`, incremented by the session on each
   hardware part select (Shift + fader button 1–4), so the wiring opens Channel on a change of it.
   Until then the screen opens Channel for the part whenever `keyboardParts[i].selected` turns
   true and `ui.selectedPart` differs, from any source (hardware, F1–F4, `selectPart` from the
   screen), and never on a state load that leaves it (CH-D15). Real-time safety: the counter is
   a plain field the engine thread already publishes with `surface`; no allocation.

Nothing else is new on the wire: every command the page sends exists (`setPartVolume`,
`setPartPan`, `togglePart`, `setPartSolo`, `setStripSend`, `addSend`, `setStripEq`,
`setStripTone`, `setStripCompressorOn` / `Preset` / `Param`, `setStripInsertKind` / `On` /
`Setting`, `toggleRotaryFast`, `setStripMono`, `setStripPortamento`, `setPartOctave`,
`setBendRange`, `selectPart`), as do every state field read and the shell's plugin editor call.
The band's and key row's commands are the kit's.

## Checks

Vitest (`npx vitest run` on the page and component tests), each against the board fixture
unless it says otherwise, reading roles, names, attributes (`data-face`, `data-hue`, `data-beat`),
the commands sent (a fake `send`) and the shell call (a fake `pluginEditor`); never computed
colours or layout.

1. Parts list: twelve entries in the order R1, R2, R3, L, Rhythm 1 … Phrase 2; entry 1 has
   `aria-current="true"`, `data-face="chosen"`, `data-hue="r1"` and reads "R1 Stage Grand"; entry 3
   reads "R3 Brass Section" with `data-hue="d"`; a Style entry shows its tag and no name; the
   header reads "Part 1 of 12"; clicking entry 2 sets `ui.selectedPart` to 1 and sends
   `selectPart {part: 1}`; clicking Rhythm 1 sets 4 and sends nothing.
2. Part header: the block reads "Right 1" with `data-hue="r1"`; the sound button's name is the
   Stage template's example for Stage Grand ("Right 1 sound: 1 Stage Grand. Opens the quick
   sound list"); "Mine" is shown; the plugin line reads "Sampler Deluxe · playing · in process"
   and "playing" has `data-hue="ok"`; Edit calls `pluginEditor(0, true)`; "CPU 3%"; ▶ is named
   "Next part (R2)" and opens part 1; ◀ "Previous part (Phrase 2)" opens 11. With `plugin`
   removed: no plugin line and no Edit. With `plugin.status` "failed", `missing` true: "Sampler
   Deluxe · missing" with `data-hue="warn"`.
3. Compact block: the chord's runs are "Am" and "7", the tones "A C E G", "104" and "BPM", the
   run dot `aria-label="Running"`, "Main B" `data-hue="main"`; stopped: the dot hidden
   (`data-run="stopped"`), the section `data-hue="m"`; sync start: `aria-label="Sync start"`.
   Its `aria-label` is the template's example.
4. Bar maths (pure, `app/src/ui/BarReadout/bar.ts`): `fraction` for each row of the Groups tables
   gives the fixture's fractions (Level 90 → 0.709, Pan 64 → 0.504, Low +2 → 0.583, Low freq 120 →
   `stepOf` 12 / 36, High freq 8000 → 24 / 30, Threshold −18 → 0.625, Ratio 30 → 0.105, Release
   120 → 0.111, Cutoff 76 → 0.598, Portamento 0 → 0); `fillBox(f, bipolar)` gives `{fl, fw}`
   (0.709 → {0, 71}; 0.583 bipolar → {50, 8}; 0.3 bipolar → {30, 20}); `hzUnit(120)` is ["120",
   "Hz"], `hzUnit(8000)` ["8.0", "kHz"], `hzUnit(1200)` ["1.2", "kHz"]; `ratioText(30)` "3",
   `ratioText(25)` "2.5"; `dbText(−18)` "−18".
5. BarReadout as a control: Level has `role="slider"`, `aria-valuenow` 90, `aria-valuetext`
   "Level 90"; a 20px rightward drag from any point sends `setPartVolume {part: 0, volume: 100}`
   once (fake frames), and the last value on pointerup; with Shift held, 20px sends 92 (one unit
   per 8px); wheel up sends 91; ArrowRight 91, PageUp 100, End 127; double-click sends 100. Low
   freq at 120 Hz: ArrowRight sends `setStripEq` with `lowFreq` 140 (120's nearest step is 125;
   the step after it is 140), ArrowLeft sends 110. Pan's `aria-valuetext` is "Pan C" and a
   double-click sends `setPartPan {pan: 64}`. Send 5 is `aria-disabled`, reads "—" and sends
   nothing on drag, wheel or keys.
6. Sends: rows read "Reverb 40", "Chorus 12", "Delay 0", "Phaser 20", "Send 5 —", "Send 6 —";
   dragging Phaser sends `setStripSend {strip: 0, send: 3, level}`; + Add send is named "Add a send
   effect (5 of 6)", opens a listbox of twelve options, and choosing "Plate" sends `addSend {kind:
   'plate'}` and closes it; with six sends the button is `aria-disabled`.
7. Lamps: On reads "On", `data-face="on"`, click sends `togglePart {part: 0}`, a 350 ms press
   sends `setLayer {swap, part 0}` and no toggle (fake timers); Solo `data-face="off"`, click sends
   `setPartSolo {part: 0}`; with `mixer.partSolo` 0 it is lit and a click sends `setPartSolo {part:
   null}`; Compressor On lit, click sends `setStripCompressorOn {strip: 0, on: false}`; Rotary
   fast lit, click sends `toggleRotaryFast`; Insert 1 On lit, click sends `setStripInsertOn {strip:
   0, slot: 0, on: false}`; Mono off, click sends `setStripMono {strip: 0, on: true}`.
8. EQ and Tone: rows read "Low +2 dB", "Low freq 120 Hz", "High −2 dB", "High freq 8.0 kHz";
   Cutoff "+12", Resonance "+4", Attack "0", Release "+8", Vibrato "0"; a drag on Cutoff sends
   `setStripTone {strip: 0, control: 'cutoff', value}`; double-click on Cutoff sends 64; More
   is `aria-expanded="false"` and the rows Vib rate and Vib delay are absent; after a click they
   are present (reading "0") and the button reads "‹ Less".
9. Compressor: "Edited" is shown; the Punchy tab has `aria-selected="true"` and
   `data-face="chosen"`; clicking Rich sends `setStripCompressorPreset {strip: 0, preset: 'rich'}`;
   rows read "Threshold −18 dB", "Ratio 3 :1", "Attack 10 ms", "Release 120 ms", "Make-up 0
   dB"; a drag on Ratio sends `setStripCompressorParam {param: 'ratio'}`; double-click on Threshold
   sends −24 (Punchy's); with `edited` false, no "Edited".
10. Inserts: slot 1's kind button is named "Insert 1 kind: Rotary. Choose None, Distortion,
    Compressor, Auto Wah, Tremolo, Rotary or Phaser", opens a listbox of seven with Rotary
    `aria-selected`, and choosing Phaser sends `setStripInsertKind {strip: 0, slot: 0, kind:
    'phaser'}`; Esc closes it without sending and focus returns to the button; the rows read
    "Depth 64", "Drive 20", "Balance 64"; a drag on Drive sends `setStripInsertSetting {slot: 0,
    setting: 1}`; slot 2 reads "None", has no On lamp and no rows; with slot 2 set to a
    distortion (three settings), its On lamp and three rows appear.
11. Play: Portamento `aria-valuetext` "Portamento time 0, off"; ArrowRight sends
    `setStripPortamento {strip: 0, on: true, time: 1}`; at time 1, ArrowLeft sends `{on: false,
    time: 0}`; Octave reads "0", − sends `setPartOctave {part: 0, octave: −1}`, at −2 − is
    `aria-disabled`; Bend reads "2 st", + sends `setBendRange {part: 0, semitones: 3}`, at 12 + is
    `aria-disabled`.
12. Style part open (`ui.selectedPart` 4, until #518): the block reads "Rhythm 1" with
    `data-hue="t2"`; Level sends `setStylePartVolume {part: 0, volume}`; Pan, the Tone rows and
    the Play rows are `aria-disabled`; the sound reads `voice.label`; no Mine, plugin line or Edit;
    On sends `toggleStylePart {part: 0}`; Solo sends `setStyleSolo {part: 0}`.
13. Half band: nine strips with the kit's names; strip 1's `aria-valuetext` "Right 1 90"; a 20px
    upward drag on strip 1 sends its `set` with `volume` 148 clamped to 127 (20 × 127 / 44 = 58
    → 90 + 58 = 148 → 127); strip 2 shows "↕"; the Reverb tab sends `setFaderLayer {layer:
    'reverb'}`; the lamp row's nine buttons as Stage check 11; Track ◀ ▶ as Stage check 14; knob
    1's `aria-valuetext` "Dynamics 127", ▲ disabled on page 1; pad 10 `data-face="solid"`, pad 11
    `data-face="waiting"`, pads 3 and 7 Absent, pad 4 reads "Sync start", pad 16 "Start"; the pads
    header reads "Sections · next Main C"; Start / Stop (▶) has `aria-pressed="true"` and sends
    `startStop`; Style tempo sends `resetTempo`; Fill ▲ sends `fillUp`.
14. Key row: the status line is empty and nothing in it is focusable; the readouts read "Split
    F#2", "Detect lower", "Left G A C E" (`data-hue="l"`), "Right E4 A4" (`data-hue="r1"`); the
    size button reads "61 keys" and a click calls `setKeyRange(88)`; at 88 a click calls
    `setKeyRange(49)`; with `keyboard.detection` [55, 127] it reads "Detect upper", with [0, 127]
    "Detect full"; with a message, the line shows it and the readouts stay.
15. App bar, page variant: the rack readout reads "Rack A1 Sunday drive" with the modified dot and
    opens the Rack page's interim target; One Touch 2 is chosen; the Channel tab has
    `aria-current="page"`; the Stage tab sets `ui.page` to `stage`.
16. Opening: with `ui.page` `stage`, a state whose `keyboardParts[2].selected` turns true opens
    Channel for part 2 (`ui.page` `channel`, `ui.selectedPart` 2); a state that leaves `selected`
    where it was does not; Esc on the Channel page with nothing open sets `ui.page` to `stage`.
17. Every interactive element has a `data-tip` in the catalog (the existing tooltip test), and
    no element carries `mixer.channel.close`.
18. Column scroll hook: with insert 2 set to a four-setting kind, column D carries
    `data-overflow="true"` (the wiring compares `scrollHeight` to `clientHeight`; jsdom can't, so
    the hook is set from the row count: a column whose rows sum past 380 by the layout tables
    above); on the board no column has it.

**Story and screenshot checks** (`npm run shots -- Channel`, real Chrome):

- `Pages/Channel` › `Board` (export `Board`, layout `fullscreen`, `parameters.shots = {
  viewport: { width: 1440, height: 900 }, mask: ['[data-shot-mask="when"]'] }`) renders `Channel`
  with `boardState`, `boardNow` and `boardMeterHolds`, unscaled, in both themes, against
  `app/src/ui/Channel/crops/Board-dark.png` and `Board-light.png` (copies of
  `docs/design/push/png/Channel-Dark.png` and `Channel-Light.png`, 1440 × 900): at most 0.02 of
  the unmasked pixels differ. This needs the Stage lane's `shots.ts` item.
- The same story covers what vitest can't: the compact chord's glow and two weights, the bar
  fills and caps, the half-band meter heights, cap and ghost positions, the 32px knob arcs and tip
  dots, the pad bars.
- `Components/BarReadout` › `Rows` (the fixture's Level, Pan, Low, Low freq, Threshold, Send 5): one
  of each shape, unipolar, bipolar, disabled; crop `Channel 417,200 225.75×64` for Level and Pan.
- `Components/KindMenu` › `Open` (insert 1's menu open on Rotary): the list, the selected row and
  the cursor row; no crop (the board draws it closed), judged by Inspect.
- `Components/PartsList` › `LongName` (part 2's sound "A Very Long Sound Name For Testing"): the
  name ends in an ellipsis inside its 152px cell.
- `Components/CompactNowPlaying` › `LongChord` (`"C#m7b5/G#"`, tones "C# E G B"): the chord
  keeps 48px, the tones end in an ellipsis, "Main B" stays at the right; no crop.
- `Components/InsertsPlayGroup` › `TwoInserts` (slot 2 a distortion): the column scrolls
  (`data-overflow`), the Play group reachable by scrolling; no crop.
- axe finds no violation on any story.

## Decisions

- **CH-D1 · Tall page frame.** Channel is the first tall page: app bar (page variant), section
  row, a 468px page, the half band, the key row with its status line. Every display tab but the
  Stage uses this frame (Effects, Quick Racks, Multi Pads, Looper, Harm/Arp, #502 and on), so it
  is specified once, in Kit additions here, and their specs name it.
- **CH-D2 · Compact block fit.** The chord never shrinks (48px fixed) and the section never
  wraps; the tones give way (ellipsis). A long chord with its tones is rarer than a long chord
  alone, and the tones are repeated in the key row.
- **CH-D3 · Style parts in the list.** A Style part's entry shows its tag only (no voice name),
  as the board draws: the voice is the style's, and the entry is a destination, not a readout.
  The Style part page (#518) shows the voice.
- **CH-D4 · No close.** The Channel is a page with a tab, so it has no × and no close command;
  the Stage tab, Alt+G and Esc (once nothing is open over the page) leave it.
  `mixer.channel.close` is dropped (CH-C1).
- **CH-D5 · Overflow.** The groups grid is 380 tall and the board fills it exactly in columns B
  and D. More rows (Tone's More, a second insert with settings) push past it; rather than hide
  settings or add a popover, the column scrolls (`overflow-y: auto; scrollbar-width: thin`). The
  board never scrolls. Tone's More toggles the two vibrato rows in place.
- **CH-D6 · Bar readout gesture.** Horizontal, relative, one unit per 2px (Shift 8px), sends at
  most once per frame and always the last value on release, double-click resets to the row's
  default; the same rules as the kit's fader (Stage D23) turned sideways. The bar is 70–80px wide,
  so the gesture doesn't map the track to the range (that would be 1.6 units per px).
- **CH-D7 · Send labels.** Sends 1–3 read "Reverb", "Chorus", "Delay" (the buses, what the fader
  layers and the Launchkey call them), never the bus's type name; sends 4–6 read their kind's
  name ("Phaser", "Plate", "Delay 1/8.").
- **CH-D8 · Frequency rows.** The value shows the state's Hz as it is (an OTS or rack may set a
  frequency between two XG steps), the bar sits at the nearest step, and a drag or key moves by
  steps of the XG table from that step. Below 1000 Hz "120 Hz", above "8.0 kHz".
- **CH-D9 · Vibrato row.** Vibrato is the depth, bipolar around 64 like every tone offset; the
  board's unipolar bar was a drawing shortcut. Rate and Delay are behind More.
- **CH-D10 · Compressor off.** The rows keep their look and still edit when the compressor is
  off (the board draws no off look, and dimming would suggest they are disabled).
- **CH-D11 · Insert settings.** Every insert setting is a unipolar bar from `min` to `max`,
  reading the state's `display` string, whatever its name; the board's centred "Balance 0" was
  invented (Balance 64 reads "64").
- **CH-D12 · Portamento.** One control, the time bar: dragging above 0 sends `on: true` with the
  time, 0 sends `on: false`. A switch with time 0 means no glide, so the two states collapse
  without loss; the board draws no switch.
- **CH-D13 · Kind menu.** The insert kind and + Add send open a small listbox (not a native
  `<select>`, which can't be drawn to the board and isn't themed). It is the one place with a
  hover look (the cursor row in `--line`), because a menu needs one.
- **CH-D14 · Style part until #518.** The same page with the Style strip, Pan, Tone and Play
  disabled and the Style commands for level, on and solo; #518 replaces this row.
- **CH-D15 · Hardware part select opens Channel.** The board's new rule (Shift + fader button
  1–4 opens Channel) is honoured; until CH-C2 the trigger is any `selected` change to a new part,
  so F1–F4 open it too, which fits the parity rule (the screen's part select and the hardware's
  are the same gesture).
- **CH-D16 · CPU readout.** The channel meter's `cpu` (SoundFont or plugin alike), rounded to a
  whole percent; "—" without a synth. The plugin's own `cpu` is a share of a core, not of the
  buffer, so it isn't shown here.
- **CH-D17 · Mine.** "Mine" shows when the sound is one of the user's library sounds
  (`saved:<id>`, or a preset id that `inMySounds` finds there), the same rule as Library ›
  Sounds' Mine filter.
- **CH-D18 · Plugin line words.** `status` as the state's word (loading, playing, failed,
  muted), coloured `--m`, `--ok`, `--ending`, `--warn`; "in process" when the plugin runs inside
  yahaha (`outOfProcess` false), with ⚠ only when that was a fallback; "missing" in `--warn`.
- **CH-D19 · Key readouts.** The key row's status line carries the key strip's readouts on the
  right (Split, Detect, Left, Right, size) because the Stage's cheek is gone (Stage D25) and the
  tall pages have the 20px. Left-hand notes are pitch-class names (the chord), right-hand notes
  carry their octave (Yamaha numbering, C3 = 60), both spelled by the chord's rule (Stage D20).
  The size button cycles 49 → 61 → 88 → 49 (`ui.setKeyRange`); matching the Launchkey again is
  Settings › Keyboard's (#530).
- **CH-D20 · Half-band gestures.** The half strips keep the full strips' relative drag, with
  44px of travel (`127 / 44` units per px), so a flick still sweeps; fine control is the full
  band on the Stage or the bar readouts here.
- **CH-D21 · Pad captions at half size.** The half pads keep the Stage's sixteen captions in
  sentence case with the ▸ and ■ shapes ("Sync start ▸", "Sync stop ■", "Auto fill", "Start"),
  the board's, since 12px in 28px leaves no room for "Start / Stop".

## Follow-ups

- #518 Channel-StylePart: the Style part page (CH-D14 is the interim).
- A gain-reduction meter for the strip compressor would need it in `meters` (DECISIONS M8).
- Removing an added send from this page (today only on Effects, #502).
- A send row for the Multi Pads or the band send scales (Effects, #502).
- Per-part Launchkey mappings for the EQ, compressor and tone rows exist only through the rack's
  controller map (Rack-MapEdit, #517); a direct "learn" from a row is not drawn.
- CH-C2's `surface.partSelectSeq`, if the owner prefers F1–F4 not to open Channel.
- A fit rule for the compact chord beyond CH-D2 if long chords prove common.

## Kit additions

The parts kit.md says this spec owns ("Variants owned elsewhere"). They move into kit.md
verbatim when it is next edited; until then every tall page spec reads them here. Geometry at
1440 × 900; tokens, faces and conventions are the kit's.

### App bar, page variant

`24,24 1392×36`, the kit's app bar with two readouts after the wordmark, because the page
replaces the display that carries them on the Stage (every non-Stage page: Channel, Effects,
Quick Racks, Multi Pads, Looper, Harm/Arp, Library, Settings):

- **Rack readout**, 16px after the wordmark: the Stage's RackReadout (Stage › Sounds row) on one
  line: a text button 32 tall, items on the baseline, gap 6, 14 / 400: "Rack" `--m`, the slot
  `--t` ("A1"; no loaded button: nothing, Stage D21), the name `--t` (`liveRack.name`, no wrap,
  ellipsis at `max-width: 240px`), then (centred) the 5px `--t` modified dot when
  `liveRack.modified`. Click opens the Rack page (Stage D32 interim: the Rack drawer). Tooltip
  `stage.rack_name`. `aria-label` as the Stage's ("Rack: Sunday drive, modified, on Quick Rack A1.
  Opens the Rack page").
- **One Touch**, 12px after: the Stage's OneTouch group (Stage › Style line: "One Touch" 14
  `--m` with 4px right margin, four 32 × 32 buttons, chosen = `ots.applied`, `recallOts { index }`,
  tooltips `ots.1`–`ots.4`, Stage D16, D17).
- Then the page tabs (`margin-left: auto`), the right area (196 wide: separator, Launchkey
  status, health slot), all as kit › App bar. With a long rack name the readout's ellipsis keeps
  the tabs at their x.

### Compact now-playing block

`320 × 84`, drawn at `48,128` on the Channel; specified in full under Page › Compact block above
(the style name block, tempo, run dot, the 48px chord with its tones, the playing section). Any
tall page that keeps a now-playing block uses it unchanged at its own x, y.

### Half band

`24,600 1392×176` on tall pages, margin-top 20 under the page; four sections side by side, gap
12: **Faders** `24,600 614×176`, **Track** `650,600 40×176`, **Knobs and pads** `702,600 614×176`,
**Transport** `1328,600 88×176`. Each starts with the kit's hairline header row (36 tall). The
faders are 40px narrower than the full band's and the Track ◀ ▶ take that width (DECISIONS H2:
Track sits between the faders and the knobs; no on-screen Shift button, since every Shift
function has its own control). Everything reads and sends as the full band (kit › Full band);
below is the geometry and what differs.

#### Faders (half)

Header row, gap 12, as kit › Faders (the layer tabs' padding is `11px 8px 0` here). Below (8px),
`24,644 614×132`, a column:

- **Strips** (84 tall): `repeat(9, minmax(0, 1fr))`, gap 8 (61.1px each), each a column: the
  **fader** (a 66-tall button, `position: relative`, full width) over the **name button** (18
  tall, 12 / 500 in the hue, centred, gap 4, the kit's marks at 11px).
  - **Value**: top 0, 16 tall, centred, 15 / 300, in the hue (the kit's formats by layer).
  - **Track**: from 19 to 63 (`TOP` 19, travel 44). Two 6px meter bars at `50% − 11px` and
    `50% − 3px`, top 19 bottom 3, on `--mbg` (hidden, `opacity: 0`, when unused or layered): the
    peak and the RMS, each `height44(x) = round(44 × clamp((20·log10(x) + 60) / 60, 0, 1))` px from
    the bottom (3px up), in the hue at `--meter-mix`. A 14 × 1 `--peak` tick at the held peak,
    left `50% − 11px`, bottom `3 + height44(hold)`.
  - **Set level**: a 3px `--track` groove at `50% + 6px` from 19 to 63; the 3px fill from the
    bottom, `44 − capTop` tall, in the hue, glow `0 0 6px` at `--fill-glow-mix`; an 8 × 2 cap at
    `50% + 1px`, top `capTop + 18`, where `capTop = round((1 − value / 127) × 44)`. Unused: no
    fill, no cap, the dashed groove (`repeating-linear-gradient(to bottom, var(--line) 0 3px,
    transparent 3px 6px)`). Part off: fill and cap at 35% of the hue, value and name `--d`.
  - **Soft takeover**: "↕" 12px `--m` at the top-left (top 0, left 0); a 30 × 0 dashed 1px `--m`
    line at `50% − 15px`, top `round((1 − position / 127) × 44) + 19`.
  - **Layers**: as the kit (strips 1–4 lose their meters, fill and cap go `--t`, the value reads
    "Rev 40").
  - **As a control**: as kit › FaderStrip with `--travel` 44: `value = clamp(round(v0 + (y0 − y)
    × 127 / 44), 0, 127)` (CH-D20); wheel, keys, double-click, tooltips and `aria` as the kit's.
- **Lamp-row headers** (14 tall, `repeat(9, 1fr)`, column gap 8, 12px line-height 12 `--m`,
  bottom hairline): "Part on/off" over 1–4, "Functions" over 5–9 with "Launchkey fader buttons
  5–9" right-aligned.
- **Lamp row** (32 tall, 2px below, gap 8, the 1px `--line` divider between columns 4 and 5 at
  `left: calc((100% − 64px) × 4 / 9 + 28px)`): the kit's nine buttons at 12px labels (kit › Lamp
  row; the master button reads "Panel" / "Style").

#### Track

`650,600 40×176`: header "Track" (36 tall, hairline); then (8px) ◀ and ▶, 40 × 32 off-face
buttons, 12px, gap 8, a column. They are the Stage's style-line ◀ ▶ (Stage › Style line:
`surface.controls[trackPrev / trackNext]`, `shiftAction` with `ui.shift`, disabled when null or
no neighbour, D38); tooltips `style.prev`, `style.next`; `aria-label`s "Previous style (Track
left)", "Next style (Track right)".

#### Knobs and pads (half)

`702,600 614×176`. One header row for both (36 tall, hairline, items centred, gap 8, no wrap):
"Knobs" 14 `--m`; the page as the kit's accent block (`knobs.pageName`; swap mode: on the part's
hue); ▲ ▼ as 28 × 28 off-face buttons, 11px (`stepKnobPage`, disabled at the ends, tooltip
`knobs.page`); "Page" 13 `--m` and `{pageNumber}/{pageCount}` `--t`; then `margin-left: auto`:
"Pads" 14 `--m`; `pads.pageName` 14 `--t` followed, on Sections with a next section, by " · next
{next}" with the next section's Genos name in its hue (the count row's next; none: the page name
alone; other pages: the page name alone); ▲ ▼ 28 × 28, 11px (`surface.controls[padBankUp /
padBankDown].action`, tooltips `padpage.prev`, `padpage.next`); "Bank" 13 `--m` and
`{pads.pageNumber}/{pads.pageCount}` `--t`. No legend.

- **Knobs** (8px below, 48 tall): `repeat(8, minmax(0, 1fr))`, column gap 6 (71.5px each); each a
  button column, centred: the **ring** 32px (`conic-gradient(from 225deg, var(--a) 0 <deg>,
  var(--ring-rest) <deg> 270deg, transparent 270deg)`, `deg = round(fraction × 270)` with the
  kit's `fraction`; a `--g` disc inset 2px holding the **value** 12 / 300 `--a`, no wrap, the "%"
  kept in the number here); a 4px `--a` **tip dot** whose centre is at radius 15 from the ring's
  centre, angle `225° + deg` clockwise from up (left `16 + 15·sin θ − 2`, top `16 − 15·cos θ −
  2`); then (2px below) the **legend** 14 tall, 12 `--t2`: the knob's plain name (Stage D6).
  No Assign: ring and rest `--mbg`, no dot, empty value, legend "—" in `--d`, `aria-disabled`. No
  code line. As a control: as kit › Knob (drag per 4px, wheel, double-click `resetKnob`, arrows;
  tooltip `knobs.knob`).
- **Pads** (8px below, 60 tall): `repeat(8, minmax(0, 1fr))`, rows 28, column gap 6, row gap 4
  (71.5 × 28 each), pads 1–8 then 9–16. Each: radius 4, 1px border (transparent when idle),
  padding 0 2, the caption 12 / 500, line-height 26, letter-spacing −0.2, centred, no wrap,
  `overflow: hidden`; no numeral, no group lines. Sections captions (CH-D21): Intro I, Intro II,
  Intro III, "Sync start" + ▸, Ending I, Ending II, Ending III, Auto fill, Main A, Main B, Main
  C, Main D, Break, Tap, "Sync stop" + ■, Start. The ▸ is a CSS triangle (`border-left: 6px solid
  currentColor; border-top/bottom: 3.5px solid transparent`, 3px left margin) and the ■ a 6 × 6
  `currentColor` square (3px left margin). Faces from `level` and `anim` as kit › Pad: Idle
  `--btn` with the family hue caption (utility `--t2`); Absent caption `--d`; Playing hue fill
  and border, `--solid-ink` caption; Next `--btn`, hue border, `--t` caption at line-height 23,
  and a 2px bar (radius 1) at bottom 3, left and right 8, in the hue with the `--bar-glow-mix`
  glow, flashing on the LED clock; Armed `--btn`, hue border, glow at `--glow-mix`, `--t`
  caption, pulsing; On (a utility switch) `--lamp`. Start / Stop (pad 16) in `--ok`. Other pages:
  the kit's fallback (D33) with `pads.pads[i].label` as the caption. `aria-label`, press,
  tooltips and Launchkey as kit › Pad.

#### Transport (half)

`1328,600 88×176`: header "Transport" (36 tall, hairline); then (8px) a grid `40px 40px`, rows
24, gap `3px 8px`, five rows: **▶** | **■**, **Reset** | **Fade**, **Fill ▲** | **Fill ▼**,
**+** | **−**, **Style tempo** (spanning both columns). All off-face, 24 tall, label centred:

| Button | Face | Sends | Tooltip |
|---|---|---|---|
| ▶ (Start / Stop) | 12px `--t`, padding-bottom 3; running: `aria-pressed="true"` and a 2px `--ok` bar at bottom 3, left and right 8, radius 1, `--bg` glow | `startStop` | `transport.start_stop` |
| ■ (Stop) | 12px `--t2` | `stop` | `transport.stop` |
| Reset | 11px | `sectionReset` | `transport.section_reset` |
| Fade | 11px; faces as kit (armed waiting, fading on) | `toggleFade` | `transport.fade` |
| Fill ▲ / Fill ▼ | 11px, the arrow 8px; a `role="group"` "Fills" | `fillUp` / `fillDown` | `transport.fill_up`, `transport.fill_down` |
| + / − (Tempo) | 14 / 300 | `tempoUp` / `tempoDown`, repeating while held (`tempoHold`) | `tempo.up`, `tempo.down` |
| Style tempo | 11px | `resetTempo` | `tempo.reset` |

`aria-label`s: "Start / Stop{, running} (Play). The same control as pad 16", "Stop", "Section
reset", "Fade in/out", "Fill up", "Fill down", "Tempo up", "Tempo down", "Style tempo".
Launchkey as kit › Transport and tempo.

### Key row

`24,796 1392×80` on tall pages (margin-top 20 under the half band), a column: the **status
line** (20 tall), 4px, the **key strip** (56 tall, kit › Key strip, unchanged).

The status line is one row, 20 tall, items centred, gap 16, no wrap: the kit's status line
(`role="status"`, `aria-live="polite"`, `flex: 1`, `min-width: 0`, ellipsis; `state.message`
only, never coaching, the clear button as kit › Status line) and then, right-aligned, the **key
readouts**, each 13 / 400 `--m` with its value in `--t`:

| Readout | Reads | Value |
|---|---|---|
| "Split {key}" | `keyboard.leftSplit` | the note name with octave ("F#2"; C3 = 60) |
| "Detect {where}" | `keyboard.detection` | "lower" when it is `[0, leftSplit]`, "upper" when `[leftSplit + 1, 127]`, "full" when `[0, 127]`, else "{lo}–{hi}" as note names |
| "Left {notes}" | `keyboard.held` with `zone` left, low to high | pitch-class names, space-separated ("G A C E"), in `--l`; none: "—" in `--d` |
| "Right {notes}" | `keyboard.held` with `zone` right, low to high | names with octave ("E4 A4"), in the hue of the first part in its `parts` (`--r1`…); none: "—" |
| "{n} keys" | `ui.keyRange`, else the Launchkey's range (kit › Key strip) | a text button, 13 `--t`: click cycles 49 → 61 → 88 → 49 (`ui.setKeyRange`); tooltip `keystrip.range`; `aria-label` "Keyboard size {n} keys. Click for {next}" |

The four readouts are not controls; each carries tooltip `keystrip.readout` (new, CH-C1).
Spelling follows the chord's rule (Stage D20, CH-D19). The key strip's own `aria-label` stays
as the kit's.

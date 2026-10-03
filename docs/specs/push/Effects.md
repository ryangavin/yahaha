# Effects

The effects buses: the send list on the left, the open bus's editor on the right, and the
switches that shape the whole mix (the style's inserts, the Master Compressor and EQ) in the
header. A display page: the band and the keys stay below it, as on the Stage.

- **Issue:** #502 · **Flow:** Shape the mix · **Boards:** `docs/design/push/Effects-Dark.dc.html`,
  `Effects-Light.dc.html`; pictures `docs/design/push/png/Effects-Dark.png`, `Effects-Light.png`
  (1440 × 900). The spec stands without them: every value a builder needs is below, in
  [Stage.md](Stage.md) or in [kit.md](kit.md); the board lines in the Components table are for
  cutting crops (the light board's lines are the dark board's minus 16, throughout).
- **Built from:** [kit.md](kit.md): App bar, Section row, Full band, Key strip, Status line, the
  faces and tokens; plus the parts under "Kit additions" at the end of this file (the page
  variant of the app bar, the compact now-playing block, Segment, Picker, Readout, Badge), which
  belong in kit.md and are written here only because this lane owns one file. This file binds
  them to the Effects page and specifies the display, which is the page's own.
- **Copied from:** Stage (#500). The app bar (page variant), section row, band, status line and
  keys are the Stage's; only the display differs. Everything the Stage spec decides (D1 scaling,
  D23 fader drag, D32 interim links, D35 hover, D36 keys, D41 test hooks, D46 wiring) holds here.
- **Variants of this screen** (they spec only what differs, against the regions named below):
  Effects-Master (#519: the Master row and the Master editor in the Open bus region),
  Effects-Reverb (#520: send 1 open, its editor), Effects-Chorus (#521: send 2 open, five sends in
  the Send list, its editor). The Send list and the Open bus are separate components with their
  own boxes so a variant replaces one without touching the other.
- **Glance order** (brightest to quietest): the compact block's chord (48px, `--a`, its glow), the
  open bus's name (18 / 500 `--t`) on its white row in the list, the return and readout values
  (18 / 300 `--a`), the lamps that are on. Nothing else glows on this page.
- **Before building:** FXC1 (tooltips) must land first; nothing else blocks (see Contract changes
  needed). The Stage's C5 and the kit's new tokens are assumed landed (the Stage builds first).

## Layout

At 1440 × 900, laid out and scaled as the Stage (Stage.md D1). Padding 24 all round; a column.

| Region | Box | What's in it | Spec |
|---|---|---|---|
| App bar | `24,24 1392×36` | wordmark, rack readout, One Touch, page tabs (Effects chosen), Launchkey status, health slot | kit › App bar, plus Kit additions › Page app bar |
| Section row | `24,68 1392×32` | Accomp, count row, Metronome ▾, Unison, Panic, ? | kit › Section row |
| Display | `24,112 1392×300` | the compact now-playing block, a hairline, the Effects zone | below |
| Band | `24,432 1392×368` | faders, knobs, pads, transport and tempo | kit › Full band |
| Status line | `24,800 1392×20` | `state.message` | kit › Status line (Stage.md D15) |
| Keys | `24,820 1392×56` | the key strip, 61 keys | kit › Key strip |

Vertical rhythm: app bar, 8, section row, 12, display, 20, band, 20 (the status line), keys. The
board draws no status line; it is the kit's, empty when `message` is null (FX-D26).

## Display

`24,112 1392×300`, no surface, no art: ground only (`--g`), `position: relative`, `overflow:
hidden`, and the 1px transparent border the Stage keeps (Stage.md D42). Inside the border the box
is `25,113 1390×298`; the content box is `49,129 1342×266` (24 left and right, 16 top and bottom
inside the border), one row, gap 24, no wrap:

| Part | Box | Spec |
|---|---|---|
| Now playing column | `49,129 320×266`; the compact block fills `49,129 320×84`, the 182px below it is air (nothing drawn) | Kit additions › Compact block |
| Hairline | `393,129 1×266`, `--line`, `aria-hidden` | — |
| Effects zone | `418,129 973×266`: a column, header 36, gap 12, body 218 | below |

The Effects zone is `role="region"` `aria-label="Effects"`. Its body is one row, gap 24: the
**Send list** `418,177 260×218` and the **Open bus** `702,177 689×218`.

### Effects header

`418,129 973×36`, `box-sizing: border-box`, `border-bottom: 1px solid var(--line)`, items
centred, gap 12, no wrap. The first two items are text; the rest sit at the right
(`margin-left: auto` on the first lamp).

| Item | Face | Reads | Sends / does | Tooltip | Launchkey |
|---|---|---|---|---|---|
| "Effects" | 14 / 400 `--m`; not a control | — | — | — | — |
| "Sends" | 14 / 400 `--t`; not a control | fixed text while a send is open (FX-D24; #519 says what it reads with Master open) | — | — | — |
| Style inserts | LampButton `size md` (32 tall, padding 0 16, 14px), label "Style inserts", no code | on = `effects.insertsOn` | `setInsertsOn { on: !insertsOn }` | `fx.inserts` | — |
| Parts | text button, 14 / 400 `--m`, 32 tall, no padding | — | opens Channel for the first Style part that has an insert: strip `4 + effects.inserts[0].part` (FX-D12); disabled (`--d`) when `effects.inserts` is empty | `fx.insert_parts` (new) | — |
| Rotary fast | LampButton `size md`, label "Rotary fast" | on = `effects.rotaryFast` | `toggleRotaryFast` (FX-D13) | `fx.rotary_fast` | Shift + encoder page ▲ |
| separator | 1 × 16 `--line`, `aria-hidden` | — | — | — | — |
| Master comp | LampButton `size md`, label "Master comp" | on = `effects.master.compressor.on` | `setMasterCompressorOn { on: !on }` | `fx.master_comp` | — |
| Comp type | text button, 14 / 300 `--a`, 32 tall | the name of `effects.master.compressor.preset` (`natural` "Natural", `rich` "Rich", `punchy` "Punchy", `electronic` "Electronic", `loud` "Loud"); when `compressor.edited`, a 5px round `--t` dot follows (gap 4, centred) | opens the Master bus (`ui.effectsBus = 'master'`, spec #519); until #519 lands, disabled with its tooltip (FX-D15) | `fx.master_comp_type` | — |
| Master EQ | LampButton `size md`, label "Master EQ" | on = `effects.master.eq.on` | `setMasterEqOn { on: !on }` | `fx.master_eq` | — |
| EQ type | text button, 14 / 300 `--a`, 32 tall, items on the baseline, gap 4 | the name of `effects.master.eq.preset` (`flat` "Flat", `mellow` "Mellow", `bright` "Bright", `loudness` "Loudness", `powerful` "Powerful"), then "bands" hidden: see FX-D14 (the board's "8 bands" is replaced by the type name; the element carries `data-shot-mask="eq-type"`); the edited dot as the comp type's when `eq.edited` | as Comp type | `fx.master_eq_type` | — |

`aria-label`s: Style inserts "Style insertion effects: all on" / "all off"; Parts "Style inserts
per part: on/off and amount. Opens Channel for {part name}" ("… no part has an insert" when
disabled); Rotary fast "Rotary fast, on" / "off"; Master comp "Master compressor on" / "off";
Comp type "Master compressor type: {Name}{, edited}. Opens the Master bus"; Master EQ "Master EQ
on" / "off"; EQ type "Master EQ type: {Name}{, edited}. Opens the Master bus". `effects.master`
absent (an older state): both lamps off, both type buttons read "Natural" and "Flat".

### Send list

`418,177 260×218`, a `nav` `aria-label="Send effects"`, a column. Below its last row the rest is
air.

- **Header** `418,177 260×20`: `border-bottom: 1px solid var(--line)` (box-sizing border-box),
  "Send effects" 14 / 400 `--m`, line-height 18; right-aligned (`margin-left: auto`) "Return" 12
  `--m`. Not a control.
- **Rows**, one per send, in `effects.sends` order (send 1 first). Each row is a `<button>` of
  the list's width, **40 tall** while the list has at most four rows; **32 tall** with five or six
  (the Add send row and a Master row don't count; FX-D7). `box-sizing: border-box`, padding 0
  10, no border except `border-bottom: 1px solid var(--line)`, radius 0, a grid `14px minmax(0,
  1fr) auto`, items centred, column-gap 10, text left. Its parts:

  | Part | Drawing | Reads |
  |---|---|---|
  | Numeral | JetBrains Mono 11, `--d` | `send + 1` |
  | Name | 14 / 400, line-height 16, `--t`, no wrap | sends 1–3: the bus name "Reverb", "Chorus", "Delay" (FX-D4); 4–6: `name` (the kind's name, "Phaser") |
  | Subtitle | 12 / 400, line-height 14, `--m`, no wrap, ellipsis | sends 1–3: "{type} · {source}": `type` is the block's `effectName` with the delay's shortened as FX-D8 ("Hall", "Celeste", "1/8"); `source` is "Mine" when the block's `followStyle` is false, else "From style". Sends 4–6: "Added send" |
  | Right cell | items centred | a **Badge** "Set by rack" (Kit additions) when `send < 3` and `setByRack`; otherwise the return, `returnLevel` as a number, 18 / 300, line-height 20, `--a` (FX-D5) |

  The **open row** (`ui.effectsBus === send`, FX-D2) wears the chosen face: `--t` fill, name
  `--g`, numeral `color-mix(in srgb, var(--g) 55%, transparent)`, subtitle `color-mix(in srgb,
  var(--g) 60%, transparent)`, return `--g`; the Badge keeps its own colours. `aria-current="true"`
  on the open row, `data-face="chosen"`; the others `data-face="off"` (they draw on nothing).
  Click: `ui.effectsBus = send` (app-only; nothing is sent). `aria-label` "Send {n}, {name},
  {subtitle}, return {r}" (with the Badge: "…, type set by the rack") plus ", open" on the open
  row. Tooltip `fx.send_open` (new). Launchkey: none (the knob pages reach each bus's values,
  not the choice of which is shown).
- **Add send** `418,357 260×32` (after four 40px rows): a `<button>`, padding 0 10, no border,
  radius 0, items centred, gap 10: "+" in a 14px-wide cell, 16 / 300 `--t`; "Add send" 14 / 400
  `--t`; right-aligned "{free} free" 12 `--m`, where `free` lists the sends not there, "4, 5 and 6
  free", "5 and 6 free", "6 free". Hidden when there are six sends. Click: `addSend { kind:
  'hall' }` and `ui.effectsBus = sends.length` (the index the new send will have), so the new
  send opens as it arrives (FX-D6). `aria-label` "Add a send effect ({free} free)". Tooltip
  `fx.send_add`.
- **Older state** (`effects.sends` absent or empty): the list shows three rows built from
  `effects.blocks` alone (name, `effectName`, `followStyle`, `returnLevel`; no Badge) and hides
  Add send (FX-D3).

### Open bus

`702,177 689×218`, a `section` `aria-label="{name}, send {n}"`, a column: the **title row** (32),
10, the **readout grid** (144), 12, the **note line** (16). The bus shown is `ui.effectsBus`:
a send index 0–5 (this section), or `'master'` (spec #519). An index past the last send (its
send was removed, or an older state) shows send 1 (FX-D23). The frame below is shared by every
send; the Delay (send 3) is the instance drawn on the board, and the Reverb and Chorus editors
(#520, #521) give their own readout tables in the same frame.

#### Title row

`702,177 689×32`, items centred, gap 12, no wrap.

| Item | Face | Reads | Sends / does | Tooltip |
|---|---|---|---|---|
| Name | 18 / 500, line-height 24, `--t` | as the list row's name | — | — |
| "Send {n}" | 12 `--m` | `send + 1` | — | — |
| Source | a Segment (Kit additions), `role="group"` `aria-label="{name} source"`, 8px extra left margin: "From style" \| "Mine"; **sends 1–3 only** | chosen = the block's `followStyle` (true: From style) | `setFollowStyle { block, on: true }` / `{ on: false }` | `fx.follow_style`, `fx.mine` |
| separator | 1 × 16 `--line`, `aria-hidden`; sends 1–3 only | — | — | — |
| Type | "Type" 14 `--m` (4px right margin) then, **sends 1–3**, a Segment `role="group"` `aria-label="{name} type"` with one button per entry of the block's `types`, labelled per FX-D8 | chosen = the block's `effect` | `setEffectType { block, effect }` | `fx.reverb_type`, `fx.chorus_type`, `fx.variation_type` by block |
| Type (added send) | "Type" 14 `--m` then a Picker (Kit additions) listing `SEND_KINDS` (`app/src/panels/effects/sendKinds.ts`, 12 kinds, their names), with the send's own `kind` first when the list lacks it | value = `kind` | `setSendKind { send, kind }` | `fx.send_kind` |
| Remove (added send) | text button 13 / 400 `--m`, right-aligned (`margin-left: auto`) | — | `removeSend { send }`, then `ui.effectsBus = 0` (FX-D23) | `fx.send_remove` |

`block` for sends 1–3 is `reverb`, `chorus`, `variation`. A type change makes the bus Mine on
the session's side (`setEffectType` turns `followStyle` off); the screen sends only the type and
the Segment follows the state. The board's delay type labels are "1/8", "Dotted 1/8", "1/4",
"Ping-pong".

#### Readout grid

`702,219 689×144`: `grid-template-columns: repeat(2, minmax(0, 1fr))`, column-gap 24 (columns
332.5 wide: `702,219` and `1058.5,219`). Each column is a stack of 36-tall rows (box-sizing
border-box, `border-bottom: 1px solid var(--line)`), four per column. Three kinds of row:

- **Switch row:** a LampButton `size md` (32 tall, FX-D17) at the left; the rest empty.
- **Readout row:** a Readout (Kit additions): label, bar, value with unit, knob code. It is the
  control for its value.
- **Part sends row:** the label "Part sends" (13 `--m`) then four text buttons, gap 12, each the
  part's tag (12 / 500 in the part hue) and its value (18 / 300, line-height 20, `--a`), items on
  the baseline, gap 4; then the code "K1–4" (12 `--t2`). A part that doesn't sound
  (`keyboardParts[i].sounding` false): tag and value `--d` (`data-hue="d"`). Grid `96px
  minmax(0, 1fr) 28px`, column-gap 12. Each button reads `keyboardParts[i].strip.sends[send]`
  (for sends 1–3 the same as the part's `reverb`, `chorus`, `variation`) and opens Channel for
  the part (FX-D11; Stage.md D32 until #501). `aria-label` "{part name} {bus} send {v}{, part
  off}. Knob {i + 1}; opens Channel" ("Right 2 delay send 16. Knob 2; opens Channel"). Tooltip
  `mixer.part.reverb`, `mixer.part.chorus`, `mixer.part.variation` by bus; `mixer.strip.send` on
  an added send, whose code cell is empty (no knob page moves it).

The Delay editor (send 3, block `variation`), left column then right, top to bottom:

| Row | Kind | Label | Reads | Unit | Range (bar) | Code | Sends | Tooltip | Launchkey |
|---|---|---|---|---|---|---|---|---|---|
| L1 | switch | "Tempo sync" | on = `delaySync` param `value` 1 | — | — | — | `setEffectParam { block: 'variation', param: 'delaySync', value: 0 \| 1 }` | `fx.param.delay_sync` | — |
| L2 | readout | "Note" when `delaySync` is 1, else "Time" | `delayNote` (`display`: "1/16", "1/8T", "1/8", "1/4T", "1/8.", "1/4", "1/4.", "1/2") or `delayTime` (the number) | — / "ms" | 0–7 / 10–2000 | K5 | `setEffectParam` `delayNote` / `delayTime` | `fx.param.delay_note` / `fx.param.delay_time` | Delay knob page, knob 5 |
| L3 | readout | "Feedback" | `delayFeedback` | "%" | 0–90 | K6 | `setEffectParam` `delayFeedback` | `fx.param.delay_feedback` | knob 6 |
| L4 | readout | "Tone" | `delayTone` as `display` without its unit ("5.0") | "kHz" | 10–200 | K7 | `setEffectParam` `delayTone` | `fx.param.delay_tone` | knob 7 |
| R1 | readout | "Return" | the block's `returnLevel` | — | 0–127 | K8 | `setEffectReturn { block, level }` | `fx.variation_return` | knob 8 |
| R2 | readout | "Band send" | the block's `bandSend` | "%" | 0–127 | — | `setBandSend { block, level }` | `fx.variation_band` | — |
| R3 | readout | "Pad send" | the block's `padSend` | "%" | 0–127 | — | `setPadSend { block, level }` | `fx.variation_pad` | — |
| R4 | part sends | "Part sends" | `strip.sends[2]` of each keyboard part | — | — | K1–4 | opens Channel | `mixer.part.variation` | knobs 1–4 |

Every parameter readout takes its `min`, `max`, `default` and `display` from the block's
`params` entry (`FxParamState`); the value shown is `display` with its unit split off ("38%" →
"38" + "%", "5.0 kHz" → "5.0" + "kHz", "375 ms" → "375" + "ms", "1/8" unchanged, no unit). The
Ping-pong parameter has no row: the Ping-pong type sets it (the API's note on Variation types).
A readout's `aria-valuetext` is "{label} {display}" ("Feedback 38%", "Note 1/8"); the Readout
part gives the rest of its accessibility and pointer rules.

An **added send** (4–6) draws its `params` as readout rows in order, left column first (four per
column; a Phaser has three: Depth 0–127, Rate 5–500 shown as its `display` "0.50" + "Hz",
Feedback 0–90 "%"), with empty code cells, sending `setSendParam { send, param: <index>, value
}` (tooltip `fx.send_param`); then Return (`setSendReturn`, tooltip `fx.send_return`) in the
right column's first row and the Part sends row after it; no Band send or Pad send rows (only
the blocks have them). A kind with no `params` (one this build doesn't know) shows the Return
and Part sends rows only.

#### Note line

`702,375 689×16`: a `<p>`, 12 / 400, line-height 16, `--m`, no wrap, `overflow: hidden`. Sends
1–3: "Picking a type or moving a value makes it Mine. Knob page {p}/6, {Page}, moves K1 to K8."
with page 4 Reverb, 5 Chorus, 6 Delay (the fixed order of `KnobPage::ALL`, `src/knobs.rs`).
Added sends: "Saved with the rack. No knob page moves it." (FX-D16). Not a control.

## Band, keys and status on the Effects page

The kit's full band, key strip and status line, unchanged: the Effects page adds nothing and
hides nothing. The fader layer and knob page are whatever the state has; the board (and the
fixture) have the Reverb layer and knob page 6 (Delay) so the band shows the same values as the
display: strips 1–4 read "Rev 40", "Rev 30", "Rev 0", "Rev 20" (kit › FaderStrip, Layers), the
master button reads "Panel ·" over "Reverb" (kit › Lamp row), and knobs 1–8 read Right 1 … Left
(DlyR1…DlyL), Time/Note "1/8" (DlyTime), Feedback "38 %" (DlyFdbk), Tone "5.0 kHz" (DlyTone),
Return "36" (DlyRtn): the display's Part sends and readouts show the same values as knobs 1–4
and 5–8. Knob 7's "5.0 kHz" needs the kit's unit split extended to " kHz" (Kit additions). The
Delay knob page's plain names (Stage.md D6 table, extended): `partReverb` / `partChorus` /
`partDelay` "Right 1", "Right 2", "Right 3", "Left"; `delayTime` "Time/Note"; `fxParam` the
parameter's own word ("Feedback", "Tone", "Time", "Pre-delay", "Rate", "Depth"); `fxReturn`
"Return" (FX-D20).

## States

| State | What changes |
|---|---|
| Send 3 open, playing (the board) | as drawn |
| Another send open | that row chosen; the editor shows its frame: Reverb (#520), Chorus (#521), or the added send's rows above |
| Master open (`ui.effectsBus` `'master'`) | spec #519; until then unreachable (the two type buttons are disabled) |
| Added send open | no Source segment or separator; the Type is a Picker; Remove at the right; readouts per its `params`; no Band / Pad rows; note line "Saved with the rack…" |
| Tempo sync off | row L2 reads "Time", `delayTime` in ms |
| A send's type follows the style | its Source segment on From style; its subtitle "… · From style" |
| The rack keeps a send's type (`setByRack`, sends 1–3) | the Badge in place of the return in the list; the editor unchanged |
| Three sends only | Add send reads "4, 5 and 6 free" |
| Six sends | Add send hidden; rows 32 tall |
| Older state: no `effects.sends` | three block rows, Add send hidden; no `effects.master`: both master lamps off |
| Style without inserts | Parts disabled (`--d`, `aria-disabled`); Style inserts still a lamp |
| A part off (R3) | its Part sends tag and value `--d`; its strip and lamp per the kit |
| Comp or EQ edited | the edited dot after the type name |
| Stopped | compact block: run dot hidden, section in `--m` (Kit additions › Compact block); count row per Stage.md D4 |
| No chord | compact block chord "—" in `--d`, no tones |
| No Launchkey, no synth, trouble, a refusal | as the Stage (kit › App bar, Status line) |
| Fader layer Vol, another knob page | the band per the kit; the display doesn't change |

Light theme: the same markup; only tokens change. The open row is `--t` (#111) with `--g`
(#f2f1ee) text; the Badge `--past` (#c4c3bf) with `--t2`; no glow draws (every glow token is
`none` or 0% in light).

## Board fixture

The state and moment that reproduce the board, for the `Pages/Effects` › `Board` story and its
shots: `app/src/ui/Effects/Effects.fixtures.ts` exports `boardState`, `boardNow` and
`boardMeterHolds`. `boardState` is the Stage's `boardState` (`app/src/ui/Stage/Stage.fixtures.ts`)
with the fields below changed; everything not listed (the clock, transport, chord, style, OTS,
rack, pads, keys, `io`, `message`) is the Stage's, so the app bar, section row, compact block,
pads and keys are the same moment (`boardNow = 10000`, bar 3 beat 3, LED phase 0.25).

- `ui.page` `effects`, `ui.effectsBus` 2 (the Delay). `ui.shift` false.
- `mixer`: faderPage "panel", **faderLayer "reverb"**, styleVolume 100, multiPadVolume 90,
  master 100.
- `surface.faders` (9): values **40, 30, 0, 20**, 100, 90, null, null, 100 (in a send layer a
  part fader's `value` is its send); labels as the Stage; fader 2 `waiting` true at `position` 50;
  `set` the matching `setPartSend { send: 'reverb' }` for 1–4.
- `keyboardParts[i].strip.sends` (sends 1–6): R1 `[40, 10, 0, 0, 0, 0]`, R2 `[30, 10, 16, 0, 0,
  0]`, R3 `[0, 10, 0, 0, 0, 0]`, L `[20, 10, 0, 0, 0, 0]`; their `reverb`, `chorus`, `variation`
  the same numbers. On, sounding, sounds and plugins as the Stage (R3 off, missing).
- `meters` as the Stage (strips 1–4 hide theirs in the layer); `boardMeterHolds` the Stage's.
- `knobs`: page "delay", pageName "Delay", pageNumber 6, pageCount 6; knobs 1–8 (function,
  name, short, value, level): partDelay "Right 1 Delay" "DlyR1" "0" 0; partDelay "Right 2 Delay"
  "DlyR2" "16" 16; partDelay "Right 3 Delay" "DlyR3" "0" 0; partDelay "Left Delay" "DlyL" "0" 0;
  delayTime "Delay Time" "DlyTime" "1/8" **36** (note index 2 of 7: `round(2 / 7 × 127)`,
  FX-D19); fxParam "Delay Feedback" "DlyFdbk" "38%" 54 (`round(38 / 90 × 127)`); fxParam "Delay
  Tone" "DlyTone" "5.0 kHz" 27 (`round(40 / 190 × 127)`); fxReturn "Delay Return" "DlyRtn" "36"
  36.
- `effects.blocks`: reverb `hall` ("Hall"), returnLevel 64, bandSend 100, padSend 100, params
  reverbTime 24 "2.4 s", preDelay 22 "22 ms", reverbTone 45 "4.5 kHz" (defaults the same),
  styleEffect `{ name: "Real Medium Hall", effect: "hall" }`, **followStyle false**; chorus
  `celeste` ("Celeste"), returnLevel 48, bandSend 0, padSend 0, params chorusRate 29 "0.29 Hz",
  chorusDepth 9 "0.9 ms", styleEffect `{ name: "Celeste 1", effect: "celeste" }`, followStyle
  true; variation `eighth` ("Delay 1/8"), returnLevel 36, bandSend 0, padSend 20, params
  delaySync 1 "On", delayNote 2 "1/8", delayTime 375 "375 ms", delayFeedback 38 "38%", delayTone
  50 "5.0 kHz", pingPong 0 "Off" (defaults: the `eighth` type's, the same), styleEffect null,
  followStyle false. Each block's `types` as the API lists them.
- `effects.sends`: send 0 hall "Hall" returnLevel 64 fromStyle true setByRack false; send 1
  celeste "Celeste" 48, fromStyle true, **setByRack true** (a valid state: the override keeps a
  style change off the send while `followStyle` stays true; FX-D19); send 2 eighth "Delay 1/8"
  36, fromStyle true, setByRack false; send 3 phaser "Phaser" returnLevel 20, fromStyle false,
  setByRack true, params Depth 64 (0–127, default 64, "64"), Rate 50 (5–500, default 50, "0.50
  Hz"), Feedback 40 (0–90, default 40, "40%"). Each send's `params` for 0–2 are its block's.
- `effects.inserts`: `[{ part: 3, partName: "Chord 1", name: "British Combo Classic", effect:
  "distortion", on: true, amount: 64 }]` (the dev mock's); `insertsOn` true; `rotaryFast` false.
- `effects.master`: compressor on, preset `natural`, compression 30, texture 50, output 1, edited
  false; eq off, preset `flat`, bands `eqPresetBands('flat')`, edited false.
- `home.bandSends` levels 100, 0, 0 (reverb, chorus, delay: the blocks' band sends above).
- `surface.layer` none; lamps as the Stage.

Board texts that differ from this fixture on purpose, each masked in the screenshot check: the
count row's "fill after bar 4" (Stage.md D5, `data-shot-mask="when"`) and the header's "8 bands"
(FX-D14, `data-shot-mask="eq-type"`). Differences under the diff's threshold, not masked: the
Note readout's bar (the board drew 40%, the rule gives 2/7 = 28.6%, over a 96.5 × 2 px bar), the
Pad send bar (board 20%, rule 20 / 127 = 15.7%), knob 5's arc (board 40%, fixture 36 / 127 =
28.3%), the Tempo sync lamp's height (board 30, kit 32, FX-D17), and the light board's `--m` and
`--lamp-ink` (Stage.md › Board fixture).

## Components

Every part of the screen, in build order; a component is built only after everything in its
"Built from" column. Numbers 0–41 are the Stage's components (Stage.md › Components), reused as
they are, with the Stage's board lines; the Effects page builds on them and adds the ones below.
Each new one gets `app/src/ui/<Name>/` with a SPEC.md per `docs/factory/spec-template.md`; its
Boards line and crops come from the board lines here (dark / light = dark − 16). "Exists" is
whether it is in `app/src/ui` today (after the Stage lane, the Stage's components exist).

| # | Component | Kind | Built from | Exists | Board lines (dark / light) | Spec |
|---|---|---|---|---|---|---|
| 0–41 | the Stage's | — | — | with the Stage | Stage boards | Stage.md |
| 42 | Segment | primitive | — | no | 197–208 / 181–192 | Kit additions › Segment |
| 43 | Picker | primitive | — | no | not on this board (#519 draws one: `Effects-Master-Dark.dc.html`, the Type ▾) | Kit additions › Picker |
| 44 | Badge | primitive | — | no | 180 / 164 | Kit additions › Badge |
| 45 | Readout | primitive | — | no | 217–222 / 201–206; data 578–591 / 562–575 | Kit additions › Readout |
| 46 | CompactBlock | complex | ChordReadout (its runs), StatusDot | no | 136–147 / 120–131 | Kit additions › Compact block |
| 47 | PageAppBar (AppBar `variant="page"`) | complex | AppBar, RackReadout, OneTouch | no | 71–100 / 55–84 (rack 73 / 57, One Touch 74–80 / 58–64) | Kit additions › Page app bar |
| 48 | EffectsHeader | complex | LampButton | no | 154–166 / 138–150 | Display › Effects header |
| 49 | SendRow | complex | Badge | no | 173–183 / 157–167; data 559–576 / 543–560 | Display › Send list |
| 50 | SendList | complex | SendRow | no | 170–190 / 154–174 (Add send 185–189 / 169–173) | Display › Send list |
| 51 | BusTitle | complex | Segment, Picker | no | 194–209 / 178–193 | Display › Open bus › Title row |
| 52 | PartSendsRow | complex | — | no | 235–246 / 219–230; data 592–599 / 576–583 | Display › Open bus › Readout grid |
| 53 | DelayEditor | complex | LampButton, Readout, PartSendsRow | no | 211–248 / 195–232 | Display › Open bus › Readout grid |
| 54 | AddedSendEditor | complex | Readout, PartSendsRow | no | not on this board (no crop; judged by Inspect) | Display › Open bus (added send) |
| 55 | BusEditor (the frame: title, grid, note) | complex | BusTitle, DelayEditor, AddedSendEditor; #520, #521 and #519 add theirs | no | 193–251 / 177–235 | Display › Open bus |
| 56 | EffectsDisplay | complex | CompactBlock, EffectsHeader, SendList, BusEditor | no | 130–255 / 114–239 | Display |
| 57 | Effects (page, `Pages/Effects`) | complex | PageAppBar, SectionRow, EffectsDisplay, FullBand, StatusLine, KeyStrip | no | whole board | this file |

Components take props and call callbacks; none reads `app.state` or sends. The page wiring
(`app/src/pages/EffectsWiring.svelte`, outside `app/src/ui`, as Stage.md D46) reads `app.state`
and `ui.effectsBus`, keeps `now`, `receivedMs` and the meter holds (shared with the Stage's
wiring: one module, `app/src/pages/clock.svelte.ts`, FX-D21), passes them down and sends
commands.

## Gap against today

| Area | In `app/src` now | Change |
|---|---|---|
| Effects screen | `panels/effects/Effects.svelte`: a drawer (`Overlay`, `ui.effects`, Alt+E) with one card per block (From style / Mine chips, type chips, `FxKnob`s for return and parameters, `Toggle`s for the switches, `HSlider`s for Band and Pads, a Rack keeps-type toggle with "Set by rack" and "Use style's"), a card per added send (a `<select>` kind, knobs, Remove), an Add send card (a kind `<select>` and Add), and an Inserts card (all on, Rotary fast, each part's switch, name and amount) | Replaced by this page (`ui.page` `effects`, FX-D1): the cards become the send list and one open editor; the knobs become Readouts; Band and Pads become readout rows; "Rack keeps type" and "Use style's" leave the screen (FX-D5: the Badge shows the state; the switch lives in the Chorus editor, #521); the per-part inserts go to Channel (FX-D12). The drawer and its tests (`Effects.test.ts`) are removed once the page lands; `sendKinds.ts` stays (the Picker's list) |
| Master effects | `panels/mixer/MasterFx.svelte`: Comp and EQ switches on the master strip and a floating editor (`fx.master_edit`) | The header's two lamps and two type readouts; the editor becomes the Master bus (#519) |
| Navigation | `lib/nav.ts` `NAV` entry `effects` (`toggle: ui.toggleDrawer('effects')`), `nav.effects` tooltip "Opens the Effects screen… Press again to close" | The tab shows the page (kit › App bar, D2); `nav.effects` body rewritten (FXC1); Alt+E unchanged. Stage.md D32's interim rows for "Effects page" and "Effects at the master" are dropped: the band sends open this page with the bus as it was, the master strip name opens it with `ui.effectsBus = 'master'` (FX-D25) |
| App-only state | `ui.effects` (the drawer's switch) | `ui.effectsBus: number \| 'master'` (FX-D2), kept while the app runs, not saved |
| Controls | `lib/ui/Toggle.svelte`, `panels/mixer/FxKnob.svelte`, `panels/settings/HSlider.svelte` | Segment, Picker, Readout, Badge (Kit additions) |
| Tooltips | `fx.*` keys exist for every bus control, switch, return, band and pad send, master switch and type | New: `fx.send_open`, `fx.insert_parts` (FXC1) |
| Screenshot tool | `app/scripts/shots.ts` | The Stage's build-time item (per-story viewport and masks, Stage.md D39); nothing more |

## Contract changes needed

FXC1 blocks the build (as the Stage's C5: a missing `TipKey` fails `npm run check` and the
tooltip catalog test). Nothing else is needed: every command the page sends and every field it
reads exists in `docs/app-api.md` and `app/src/lib/api/types.ts`.

1. **FXC1 · Tooltips** (`app/src/help/tooltips.ts`, `app/docs/controls.md`), **lands before the
   build**: new keys `fx.send_open` ("Send {n}: opens this send's editor; the knob pages Reverb,
   Chorus and Delay move sends 1–3 from the Launchkey") and `fx.insert_parts` ("Style inserts per
   part: each Style part's insertion effect, on/off and amount, on its Channel page"); rewrite
   `nav.effects` from "Opens the Effects screen … Press again to close" to "Shows the Effects
   page: the send effects, the open one's editor, the style's inserts and the master compressor
   and EQ"; `fx.send_add` gains "Adds a Hall; change its type in its editor" (FX-D6); the
   `launchkey` line of `fx.param.delay_note`, `delay_time`, `delay_feedback`, `delay_tone`,
   `variation_return` reads "Delay knob page, knob 5 / 5 / 6 / 7 / 8" (the Reverb's and Chorus's
   the same way for #520 and #521), replacing the old "FX knob page" lines.

## Checks

Vitest (`npx vitest run` on the page and component tests), each against the board fixture unless
it says otherwise. They read roles, names, attributes (`data-face`, `data-hue`) and the commands
sent (a fake `send`); never computed colours or layout (Stage.md D41).

1. Header: Style inserts has `aria-pressed="true"` and `data-face="on"`; a click sends
   `setInsertsOn { on: false }`; Rotary fast sends `toggleRotaryFast`; Master comp sends
   `setMasterCompressorOn { on: false }`; Master EQ sends `setMasterEqOn { on: true }`; the comp
   type reads "Natural" and the EQ type "Flat", both `aria-disabled` (interim, FX-D15); with
   `compressor.edited` true the comp type's `aria-label` ends ", edited. Opens the Master bus";
   Parts calls the Channel opener with part 7; with `inserts` empty Parts is `aria-disabled` and
   calls nothing; with `effects.master` absent both lamps are off.
2. Send list: four rows named "Reverb", "Chorus", "Delay", "Phaser" with subtitles "Hall · Mine",
   "Celeste · From style", "1/8 · Mine", "Added send"; row 3 has `aria-current="true"` and
   `data-face="chosen"`; row 2 shows "Set by rack" and no return; row 4 reads 20; a click on row
   1 calls `onopen(0)` and sends nothing; Add send reads "5 and 6 free", a click sends `addSend
   { kind: 'hall' }` and calls `onopen(4)`; with six sends there is no Add send and the rows have
   `data-rows="32"`; with `effects.sends` empty there are three rows and no Add send.
3. Title row: "Delay" and "Send 3"; Mine has `aria-pressed="true"`, a click on From style sends
   `setFollowStyle { block: 'variation', on: true }`; "1/8" is pressed; a click on Ping-pong sends
   `setEffectType { block: 'variation', effect: 'pingPong' }`; the type buttons read "1/8",
   "Dotted 1/8", "1/4", "Ping-pong" (`typeLabel`, a pure function: "Delay 1/8." → "Dotted 1/8",
   "Ping-Pong" → "Ping-pong", "Hall" → "Hall").
4. Readouts: Tempo sync is on and a click sends `setEffectParam { block: 'variation', param:
   'delaySync', value: 0 }`; with `delaySync` 0 row L2 reads "Time", "375", "ms" and its code is
   "K5"; Note reads "1/8", `aria-valuetext` "Note 1/8"; Feedback has `aria-valuenow` 38, min 0,
   max 90; a 48px drag to the right on Feedback (its bar 96px wide: the test sets the width)
   sends `setEffectParam delayFeedback 83`; a wheel notch up sends 39; a double-click sends 38
   (the default); Return drag sends `setEffectReturn`; Band send sends `setBandSend`; Pad send
   sends `setPadSend`; `dragValue(38, 48, 96, 0, 90)` is 83 and `fraction(2, 0, 7)` is 2/7 (pure,
   `app/src/ui/Readout/readout.ts`).
5. Part sends: R2 reads 16 and R3 has `data-hue="d"`; a click on R1 calls the Channel opener
   with part 0; the R2 button's `aria-label` is "Right 2 delay send 16. Knob 2; opens Channel".
6. Added send open (`effectsBus` 3): no Source group; the Picker lists 12 options and a change to
   `room` sends `setSendKind { send: 3, kind: 'room' }`; Remove sends `removeSend { send: 3 }` and
   calls `onopen(0)`; readouts Depth, Rate ("0.50", "Hz"), Feedback with empty code cells; a drag
   on Depth sends `setSendParam { send: 3, param: 0, … }`; Return sends `setSendReturn`; no Band
   send or Pad send row; the note line reads "Saved with the rack. No knob page moves it."
7. Note line for send 3: "Picking a type or moving a value makes it Mine. Knob page 6/6, Delay,
   moves K1 to K8."; for send 1 "… 4/6, Reverb, …".
8. Compact block: the style name button opens the Browser; "104" and "BPM"; the run dot's
   `aria-label` "Running"; the chord runs "Am" and "7"; tones "A C E G"; "Main B" with
   `data-hue="main"`; stopped: the dot `hidden`, the section `data-hue="m"`; no chord: "—".
9. Page app bar: the rack readout reads "Rack", "A1", "Sunday drive" with the modified dot; One
   Touch 2 is pressed; the Effects tab has `aria-current="page"`.
10. Band: strips 1–4 read "Rev 40", "Rev 30", "Rev 0", "Rev 20" and have no meter element; the
    Reverb layer tab is selected; knob 7 reads "5.0" with unit "kHz" (the unit split); knob 5's
    name is "Time/Note".
11. Every interactive element has a `data-tip` in the catalog (the existing tooltip test).
12. Links (Stage.md D32 interim): Parts and the part sends open today's `ChannelView`
    (`show(part)`); the comp and EQ type buttons are `aria-disabled`.
13. Tab order (FX-D22): app bar, section row, compact block (style name), header lamps and
    buttons left to right, send rows top to bottom then Add send, the title row left to right,
    the left column's rows then the right column's, then the band.

**Story and screenshot checks** (`npm run shots -- Effects`, real Chrome), for what jsdom can't
see:

- `Pages/Effects` › `Board` (export `Board`, layout `fullscreen`, `parameters.shots = { viewport:
  { width: 1440, height: 900 }, mask: ['[data-shot-mask="when"]', '[data-shot-mask="eq-type"]']
  }`) renders `Effects` with `boardState`, `boardNow` and `boardMeterHolds`, unscaled, in both
  themes, against `app/src/ui/Effects/crops/Board-dark.png` and `Board-light.png` (copies of
  `docs/design/push/png/Effects-Dark.png` and `Effects-Light.png`): at most 0.02 of the unmasked
  pixels differ. It covers the chosen row, the lamps, the accent values, the bars, the compact
  block's glow (dark) and none (light), the layered strips and the Delay knob page.
- `Components/BusEditor` › `AddedSend` (send 3, the Phaser, open) and › `TimeNotSynced`
  (`delaySync` 0): no crop (not on a board); judged by Inspect against the rules above.
- `Components/SendList` › `SixSends` (six rows at 32px, no Add send) and › `BlocksOnly` (no
  `sends`): no crop.
- `Components/CompactBlock` › `LongName` (a 60-character style name): the name ends in an
  ellipsis inside the 16px row; the tempo and dot keep their place; no crop.
- axe finds no violation on any story.

## Decisions

- **FX-D1 · A page, not a drawer.** Effects is `ui.page` `effects` (Stage.md D2), reached by the
  tab, Alt+E, the Stage's band sends and the master strip's name. The drawer goes.
- **FX-D2 · The open bus is app-only.** `ui.effectsBus: number | 'master'` (a send index 0–5 or
  the Master), 0 at start, kept while the app runs; nothing in the API chooses it. A row click
  sets it; Add send and Remove move it (FX-D6, FX-D23).
- **FX-D3 · Rows from `sends`, blocks as the fallback.** The list is `effects.sends`, each row
  joined to its block (sends 1–3) for `followStyle`, `effectName`, `bandSend` and `padSend`. With
  no `sends` (an older state) the three blocks are the rows and nothing can be added.
- **FX-D4 · Names.** Sends 1–3 are named by their bus, "Reverb", "Chorus", "Delay" (not
  "Variation": the block is the tempo delay, as today's drawer titles it), the type in the
  subtitle; added sends by their kind's name.
- **FX-D5 · The Badge replaces the return.** On sends 1–3 with `setByRack`, the list shows "Set
  by rack" where the return would be (the board's choice: the return is in the editor anyway).
  Added sends are always the rack's, so they show no Badge. The override switch itself is not on
  this screen; the Chorus editor (#521) draws it as "Keep with rack".
- **FX-D6 · Add send adds a Hall and opens it.** No kind picker before adding (the board draws
  none): the new send arrives as a Hall and its editor's Picker changes the kind. The screen sets
  `ui.effectsBus` to the new index before the state arrives; until it does, the editor shows
  send 1 (FX-D23).
- **FX-D7 · Row height.** 40px with up to four rows, 32px with five or six, so six sends and the
  Add send row fit the 218px body (20 + 6 × 32 + 32 = 244 doesn't, so with six sends Add send is
  hidden anyway: 20 + 6 × 32 = 212). The Master variant (#519) adds its row at 32px with its own
  rule.
- **FX-D8 · Delay type labels.** The delay's types read "1/8", "Dotted 1/8", "1/4", "Ping-pong"
  on the Segment and in the subtitle (the state's "Delay 1/8", "Delay 1/8.", "Delay 1/4",
  "Ping-Pong" are long for a 24px segment and repeat the bus name). The reverb's and chorus's
  names are the state's.
- **FX-D9 · Readout drag.** Horizontal, relative, like a fader (Stage.md D23): the value moves by
  the pointer's travel from the press point, the bar's width standing for the whole range,
  `value = clamp(round(v0 + (x − x0) × (max − min) / width), min, max)`, with pointer capture;
  sends at most once per animation frame and the last value on release; wheel ±1, arrows ±1,
  PageUp/PageDown ±10 (clamped), double-click the parameter's `default` (a return's 64, a band or
  pad send's unity 100 for the reverb and 0 for the others: the block's defaults in the API).
  Dragging the bar is the only way to set a value on the screen; the knob page is the other
  (parity).
- **FX-D10 · Bar fraction.** `(value − min) / (max − min)` for every readout, 0–127 for returns
  and the band and pad sends (a bar past 100% shows the boost). The board drew the Note and Pad
  send bars by eye; the differences are under the screenshot threshold (Board fixture).
- **FX-D11 · Part sends are readouts that open Channel.** They show each keyboard part's send to
  the bus and open the part's Channel page (where the send is a control, #501); knobs 1–4 of the
  bus's knob page and the fader layer set them from the band. Dragging them here would make a
  fourth place for the same value.
- **FX-D12 · Parts opens Channel.** The per-part inserts (on/off and amount) live on each Style
  part's Channel page (Channel-StylePart, #501); Parts opens the first Style part that has an
  insert (`effects.inserts[0]`), and is disabled when the style has none. Until #501, today's
  `ChannelView` for that part (it steps all 12 parts, DECISIONS S10).
- **FX-D13 · Rotary fast toggles.** The lamp sends `toggleRotaryFast`, the command Shift +
  encoder page ▲ sends (parity), rather than `setRotaryFast`.
- **FX-D14 · The type readouts.** The comp type reads the preset's name (the board's "Natural")
  and the EQ one the EQ preset's name ("Flat"), not the board's "8 bands" (a constant tells the
  player nothing; the count is always eight). Both carry the edited dot when the settings differ
  from the type. The EQ text is masked in the screenshot.
- **FX-D15 · Master is #519.** The two type buttons open the Master bus once Effects-Master is
  built; until then they are drawn and disabled (kit › Interaction conventions, the interim
  rule), and the lamps still switch the compressor and EQ.
- **FX-D16 · Note line.** One sentence about Mine, one naming the knob page, for sends 1–3; the
  rack sentence for added sends. It never changes with `followStyle` (the Segment shows that).
- **FX-D17 · Lamp height.** Tempo sync is the kit's 32px lamp (one control height, DECISIONS S2);
  the board's 30px is a 2px deviation under the threshold.
- **FX-D18 · Added send editor.** No Source (an added send never follows the style), a Picker for
  the kind (twelve kinds don't fit a Segment), Remove at the right, no Band or Pad rows (the API
  has none for added sends), its `params` as readouts with empty code cells.
- **FX-D19 · Fixture values.** Knob 5's level is `round(2 / 7 × 127)` = 36 (the note index over
  its range; the engine's `level` for `delayTime` is the Launchkey ring's and may differ: the
  story fixes 36). The chorus is "From style" with the rack badge on, a state the session allows
  (`set_send_override` keeps `follow`; the style re-applies when the override is released).
- **FX-D20 · Knob names on the Delay page.** The plain names for the effect knob pages extend the
  Stage's D6 table: part sends by part name, `delayTime` "Time/Note", `fxParam` by its parameter
  word, `fxReturn` "Return". For kit.md's Knob section.
- **FX-D21 · Wiring.** `EffectsWiring.svelte` as the Stage's (D46); the clock and meter holds
  move into one shared module both wirings use, so the band ticks the same on every page.
- **FX-D22 · Keyboard.** Tab order is reading order: header, list, editor (title row, then the
  left column top to bottom, then the right), then the band. Readouts handle their own arrows and
  call `stopPropagation` (as faders).
- **FX-D23 · A removed or missing bus.** `ui.effectsBus` past the last send shows send 1 (never
  an empty editor); Remove sets it to 0.
- **FX-D24 · "Sends".** The header's second word is the fixed subtitle "Sends" while a send is
  open; #519 says what it reads with Master open (the board set says nothing).
- **FX-D25 · Links into this page.** Once this page lands, the Stage's band sends open it with
  the bus as it was and the master strip's name opens it with `ui.effectsBus = 'master'`
  (which shows send 1 until #519, FX-D23); Stage.md D32's two Effects rows are dropped then.
- **FX-D26 · Status line.** The board draws none ("No status line or Back to stage button
  here"); the kit's status line is there, empty, as on every display page (Stage.md D15), and the
  tabs replace Back to stage.

## Follow-ups

- Effects-Master (#519), Effects-Reverb (#520), Effects-Chorus (#521): the other open buses.
- A per-part inserts list of its own (Parts), if opening Channel for one Style part proves too
  far from "on/off and amount" for the whole band.
- The Picker as a popover list in the Push skin, in place of a styled native `<select>`.
- Fine steps on readouts (Shift + drag or arrows), as the knobs have.
- Move the Kit additions below into kit.md (an orchestrator PR, or the Channel spec #501, which
  the kit already names as the owner of the page app bar and the compact block).

## Kit additions

Parts that belong in kit.md (they are drawn on more than one board: Channel, Effects and the
Effects variants) but are written here because this lane owns only this file. Boxes are given
on the Effects board.

### Page app bar

`AppBar` with `variant="page"`: the Stage's app bar (kit › App bar) with, between the wordmark
and the tabs, the live rack readout and One Touch, so the tall and display pages that have no
display row of their own keep them in view. On Channel, Effects and their variants.

- **Rack readout:** 16px after the wordmark, a text button 32 tall, items on the baseline, gap 6,
  14 / 400: "Rack" `--m`, the slot `--t` ("A1"), the name `--t` ("Sunday drive"), then the 5px
  round `--t` modified dot (centred). Reads, opens and labels exactly as the Stage's rack readout
  (Stage.md › Sounds row › Rack: `liveRack.name`, `liveRack.modified`, the slot from
  `quickRacks`; no slot: "Rack" and the name; opens the Rack page, interim the Rack drawer).
  Tooltip `stage.rack_name`. `aria-label` "Rack: {name}{, modified}{, on Quick Rack A1}. Opens
  the Rack page".
- **One Touch:** 12px after it, the Stage's One Touch group unchanged (Stage.md › Style line:
  "One Touch" 14 `--m`, four 32 × 32 buttons, `ots.applied`, `recallOts`, `ots.1`–`ots.4`,
  Launchkey Racks pad page pads 1–4).
- Then the page tabs (`margin-left: auto`) and the right area as the kit draws them.

### Compact block

`CompactBlock`, 320 × 84, the now-playing block of the tall and display pages (Channel, Effects
and their variants; the Channel board is its source). `aria-label` "Now playing: {style}, {tempo}
BPM, {running | stopped}, chord {chord} {tones}, {section}" ("Now playing: Sunday Drive Pop, 104
BPM, running, chord Am7 A C E G, Main B"). A column:

- **Row 1** (16 tall, items centred, gap 8, no wrap): the **style name** as an accent block text
  button, padding 0 6, 13 / 500, line-height 16, `--g` on `--a`, `min-width: 0`, ellipsis,
  reading `style.name`; click opens the Browser (Stage.md D3); tooltip `browser.open`;
  `aria-label` "{name}: open the Browser". Then (`margin-left: auto`) the **tempo**:
  `transport.tempo` rounded (Stage.md D44) at 18 / 300, line-height 16, `--t`, and "BPM" 12 / 400
  `--m` with a 3px left margin; tooltip `display.tempo`; not a control. Then the **run dot**, 6px
  round: `transport.running` → `--ok` with `--bg`, `role="img"` `aria-label="Running"`; stopped
  with `transport.syncStart` → a hollow 1px `--ok` ring, "Sync start"; stopped → `visibility:
  hidden`, space kept (as Stage.md D43), no label.
- **Row 2** (12px below, 48 tall, items on the baseline, gap 14, no wrap): the **chord**
  `chord.name` at 48 / 300, line-height 48, letter-spacing −2, `--a`, `text-shadow: var(--ba)`,
  in the Stage's two runs (Stage.md D30; the extension at 200, letter-spacing 0); no chord: "—"
  in `--d`, no shadow. Then the **tones** 14 / 300, letter-spacing 3, `--t2`: the chord's tones
  as note names separated by spaces ("A C E G"), spelled and ordered as the Stage's tone columns
  (Stage.md D20, D31); no chord: empty. Then (`margin-left: auto`) the **playing section** at
  24 / 300, line-height 48, letter-spacing −0.5, in its hue, no glow (kit › Section names, Hue
  roles): running, `transport.section`; stopped, the Main to start on in `--m` (Stage.md D4).
  Tooltip `display.chord` on the chord. Not controls.
- Beat, bar and the next section are not here: they live in the count row.

### Segment

`Segment`, a segmented choice: a `role="group"` with an `aria-label`, `display: flex`, no gap,
one `<button>` per option, 24 tall, padding 0 10, 13 / 400, no border, radius 0, no wrap. The
chosen option wears the chosen face (`--t` fill, `--g` text, `aria-pressed="true"`,
`data-face="chosen"`); the others are `--m` text on nothing (`aria-pressed="false"`,
`data-face="off"`). Clicking an option calls `onchoose(value)`; a click on the chosen one does
nothing. Arrow keys move focus between options; Space or Enter chooses. Not a fifth face: it is
the chosen face at a 24px size, the same block the fader-layer tabs use.

### Picker

`Picker`, a choice among more options than a Segment can show: a native `<select>` (so keyboard,
screen reader and the system list come free) drawn as the chosen block: 24 tall, padding `0 24px
0 10px`, 13 / 400, `--t` fill, `--g` text, no border, radius 0, `appearance: none`, with a "▾"
(10px, `--g`) at right 8 drawn by the component, `aria-label` given by the caller. `onchange`
calls `onchoose(value)`. Options are `{ value, label }` in the caller's order. Used by the added
send's type here and the Master compressor's type (#519).

### Badge

`Badge`, a small fixed label: padding 0 6, 12 / 400, line-height 20, `--t2` on `--past`, no
radius, no wrap, not a control. "Set by rack" is its only text so far.

### Readout

`Readout`, a labelled value with a bar, the Push-style parameter control (no sliders or knobs on
screen, DECISIONS M2): a `<button>` of its row's width, 36 tall (box-sizing border-box,
`border-bottom: 1px solid var(--line)`), no padding, a grid `96px minmax(0, 1fr) 76px 28px`,
items centred, column-gap 12, text left:

- **Label** 13 / 400 `--m`, no wrap.
- **Bar** `aria-hidden`: a 2px `--mbg` track filling its cell, with a `--a` fill from the left,
  width `fraction × 100%` (FX-D10).
- **Value** right-aligned, 18 / 300, line-height 20, `--a`, no wrap, then the unit 12 / 400 with
  a 2px left margin (empty when the value has none).
- **Code** 12 / 400 `--t2`, no wrap: the Launchkey knob that moves it ("K5"), empty when none.
- **As a control:** `role="slider"`, `aria-valuemin`, `aria-valuemax`, `aria-valuenow`,
  `aria-valuetext` "{label} {display}"; pointer, wheel, keys and double-click per FX-D9, sending
  through `onchange(value)`; `ns-resize` is not used (the drag is horizontal: `ew-resize` while
  dragging). Tooltip given by the caller. Focus ring as every control. A disabled readout
  (`aria-disabled`) draws its label and value `--d` and no fill.
- **Maths** in `app/src/ui/Readout/readout.ts`: `fraction(value, min, max)` and `dragValue(v0,
  dx, width, min, max)` (pure, tested).

### Knob unit split

kit › Knob says a trailing "%" splits off the value as a 12 / 400 unit. Extend it: a trailing
unit after a space (" kHz", " Hz", " ms", " s", " BPM" is already dropped on the tempo knob)
splits the same way ("5.0 kHz" → "5.0" + "kHz", "0.29 Hz" → "0.29" + "Hz", "22 ms" → "22" +
"ms"), so the effect knob pages' values fit the 22px line. One pure function, `splitUnit(value)`,
shared by Knob and Readout.

### Interaction conventions

- A **hairline row list** (the send list, the readout columns): rows of a fixed height with
  `border-bottom: 1px solid var(--line)`, no gap; the first row has no top line (the group header
  above it carries one).
- **Links into pages** (Stage.md D32): add rows "Effects page (this spec): band sends, the
  Effects tab → `ui.page = 'effects'`" and "Master bus (#519): the master strip name, the comp and
  EQ type buttons → `ui.effectsBus = 'master'`; until #519, the type buttons are disabled and the
  strip name opens the page as it is".

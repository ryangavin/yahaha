# Stage

The main play screen: what is playing, what comes next, the sounds under your hands, and the
whole Launchkey band below it. The screen a player looks at while playing.

- **Issue:** #500 · **Flow:** Play · **Boards:** `docs/design/push/Stage-Dark.dc.html`,
  `Stage-Light.dc.html`; pictures `docs/design/push/png/Stage-Dark.png`, `Stage-Light.png`.
- **Built from:** [kit.md](kit.md): App bar, Section row, Full band, Key strip, Status line,
  the faces and tokens. This file binds them to the Stage and specifies the display, which is
  the Stage's own.
- **Variants of this screen** (they spec only what differs): Stage-Help (#508), Stage-Metronome
  (#509), FirstRun (#506), PadsPage2 (#507), PadsChord (#510), PadsMultiPads (#511), PadsSetup
  (#512).
- **Glance order** (what must read first, brightest to quietest): the chord, the playing section
  (44px in its hue), the next section (36px, outlined), the tempo (32px). Nothing decorative
  outshines them: the art's core stays near 35% lightness.

## Layout

At 1440 × 900 (D1: other window sizes scale the whole screen). Padding 24 all round; a column.

| Region | Box | What's in it | Spec |
|---|---|---|---|
| App bar | `24,24 1392×36` | wordmark, page tabs (Stage chosen), Launchkey status, health slot | kit › App bar |
| Section row | `24,68 1392×32` | Accomp, count row, Metronome ▾, Unison, Panic, ? | kit › Section row |
| Display | `24,112 1392×300` | style line, chord, section and tempo, sounds row, art | below |
| Band | `24,432 1392×368` | faders, knobs, pads, transport and tempo | kit › Full band |
| Status line | `24,800 1392×20` | `state.message` | kit › Status line (D15) |
| Keys | `24,820 1392×56` | the key strip, 61 keys | kit › Key strip |

Vertical rhythm: app bar, 8, section row, 12, display, 20, band, 20 (the status line), keys.

## Display

`24,112 1392×300`, no frame or surface: ground with the art on its right. The content box is
`48,128 814×266` (left 24, top 16 inside the display), a column:

| Part | Box | Height |
|---|---|---|
| Style line | `48,128 814×32` | 32 |
| Chord (left) and Section (right) | `48,168 300×162` and `372,168 490×162` (grid 300 + 490, gap 24) | 162, 8 below the style line |
| Sounds row | `48,350 814×44` | 44, 20 below |
| Art | `887,112 529×300` (the display's right 38%) | behind, `aria-hidden` |

### Style line

One row, 32 tall, gap 8, no wrap.

| Control | Face | Reads | Sends / does | Tooltip | Launchkey |
|---|---|---|---|---|---|
| ◀ | 32 × 32 off face, 12px; disabled when `surface.trackPrev` is null | `surface.trackPrev` | `stepStyle { delta: -1 }` | `style.prev` | Track ◀ |
| Style name | accent block: padding 0 10, 18 / 500, line-height 26, `--g` on `--a` | `style.name` | opens the Browser (app-only; D3) | `browser.open` | — |
| ▶ | as ◀ | `surface.trackNext` | `stepStyle { delta: 1 }` | `style.next` | Track ▶ |
| Category · metre | 14 / 400 `--m`, 4px extra left margin; not a control | the library entry's `folder` (last segment) for `style.id`, from `library()`; `style.timeSignature` as "4/4" | — | `display.timesig` | — |
| One Touch | right-aligned group, gap 4: "One Touch" 14 `--m` (4px right margin), then four 32 × 32 buttons "1"–"4" | `ots.applied` (1-based, 0 = none): that button is the chosen face; buttons past `ots.settings.length` disabled | `recallOts { index }` (0-based) at once; D17 | `ots.1` … `ots.4` | Racks pad page, bottom row pads 1–4 (D16) |
| Band sends | 16px left margin, 24 tall, baseline row, gap 10: "Band" 14 `--m`, then "Reverb", "Chorus", "Delay" in `--t2` each with its value 18 / 300 `--a` | `home.bandSends[0..2].level` | opens the Effects page | `display.band_sends` (new) | — |

While a style waits for the bar line (`preview.queued`), the category · metre text is replaced by
"→" `--m` and the queued style's name in the waiting face in `--a` (D3).

### Chord

Left column, 300 wide.

- **Label** "Chord", 14 `--m`, 16 tall.
- **Chord**, 4px below, 104 tall: `chord.name` at 128 / 300, line-height 104, letter-spacing −6,
  `--a`, `text-shadow: var(--ba2)`. The extension (from the first digit on, and any alteration or
  slash after it: "7", "maj7", "7b5/G") is weight 200, letter-spacing 0. When it is wider than
  300 the font shrinks to fit, down to 64px (D20). No chord yet (`chord.name` null): "—" in `--d`.
- **Tones**, 6px below, 32 tall, bottom-aligned, gap 16: for each pitch class in
  `keyboard.chordTones` (root first, at most six), a 24-wide column of the note name (20 / 300,
  line-height 18, `--t`) over its interval from the root (12px, `--m`): R, b9, 9, m3, 3, 4, b5,
  5, #5, 6, m7, M7 (D20 for spelling). Then, 4px further, `chord.fingeringName` 14 `--m`
  ("Fingered"). With Keyboard transpose (`chord.fingered` differs from `chord.name`), it reads
  "Fingered · played Gm7".
- Tooltip `display.chord` on the column.

### Section and tempo

Right column, 490 wide.

- **Label** "Section", 14 `--m`, 16 tall, with a 6px dot before it (gap 8) in the playing
  section's hue with `--bg` glow, shown only while running (D28).
- **Playing and next**, 4px below, 52 tall, gap 16: the playing section's name (kit › Section
  names) at 44 / 300, line-height 52, letter-spacing −1.5, in its hue, glow `0 0 18px` at
  `--text-glow-mix`; then "next" 14 `--m`; then the next section in the waiting face: 48 tall,
  padding 0 10, 1px border, radius 4, 36 / 300, line-height 46, letter-spacing −1, in its hue.
  Playing, next and the stopped faces follow the count row's rules (kit › Count row, items 3);
  with no next, "next" and the chip are hidden. Stopped: D4.
- **Tempo row**, 50px below, 40 tall, baseline, gap 6: `transport.tempo` at 32 / 300,
  letter-spacing −0.5, `--t` (whole BPM), "BPM" 14 `--m`; right-aligned, the run state (14 /
  400, gap 8 after a 6px dot): `transport.running` → "Running", dot and text `--ok`, dot glow
  `--bg`; stopped with `transport.syncStart` → "Sync start", text `--t2`, dot a hollow 1px `--ok`
  ring; stopped → "Stopped", `--m`, no dot. Not controls. Tooltip `display.tempo` on the number.

The board doesn't repeat the count row's fill line here.

### Sounds row

44 tall, grid `200px repeat(4, 1fr)`, gap 8 (cells 145.5 wide at 1440). Every cell has a 1px
`--line` top edge.

- **Rack** (a button, the 200 cell): "Rack · A1" 12 `--m` with the slot in `--t`, over the
  name 14 / 400 `--t` (gap 2), then a 5px `--t` dot when `liveRack.modified` (gap 6).
  Name = `liveRack.name` (it reads "Recovered: Sunday drive" itself when it is one). Slot = the
  bank letter (`A` + `quickRacks.bank`) and button number (index + 1) of the
  `quickRacks.buttons` entry with `loaded` true; no such button: "Rack" alone (D21). Click: opens
  the Rack page. Tooltip `stage.rack_name`.
- **R1, R2, R3, L** (one cell each, part order 0–3): two buttons, gap 4.
  - **Tag** "R1" / "R2" / "R3" / "L", 32 × 43, left-aligned, 14 / 500 in the part hue (`--d`
    when `keyboardParts[i].on` is false). Click: opens Channel for the part. Tooltip
    `mixer.strip.select`.
  - **Sound**, the rest of the cell, gap 8: the sound number (12 `--m`; `--d` when off), the
    name (14 `--t`, ellipsis; `--m` when off), then the marks: a 5px `--t` dot when
    `soundEdited`; a 12px `--warn` ⚠ when `plugin.missing`; a 12px `--ending` ✕ when
    `plugin.status` is `failed` and not missing; "off" 12 `--m` when the part is off. Number =
    `soundLibrary.patches[].number` of the patch whose id is in `sound.id` (`saved:<id>`); none:
    no number. Name = `sound.name`, else `voiceName` (D22). Left under Manual Bass
    (`playsBass`): the name is the Style's Bass voice (`voiceName`) and "bass" 12 `--m` follows.
    Click: opens the quick sound list for the part (spec #514). Tooltip `launchkey.fader_sound`.
  - `aria-label` "Right 2 sound: 41 Silk Strings, edited. Opens the quick sound list".

### Art

The display's right 38% (`887,112 529×300`): two stacked layers, `background: var(--stage-art)`
under `var(--stage-art-fade)` (kit › Tokens, verbatim from the boards). Static: it doesn't follow
the style yet (D9). `aria-hidden`, no pointer events.

## Band, keys and status on the Stage

The kit's full band, key strip and status line, unchanged; the Stage adds nothing. The fader page
is whatever `mixer.faderPage` is: Panel shows the strips in kit › Faders; Style shows the Style
parts (#507). The pads show the hardware's page (`pads.page`); Sections is drawn here, the other
pages in their specs (#507, #510–#512). While Sound is held or latched the pads are the Racks
page (#507).

## States

| State | What changes |
|---|---|
| Playing (the board) | as drawn |
| Stopped | D4: count row blocks idle and "Bar" hidden; section name in `--m`, no glow, no dot; run state "Stopped"; Start / Stop without its bar; pad 16 idle |
| Sync Start armed | run state "Sync start"; count row's last item "sync start"; pad 4 Armed |
| Intro armed (stopped) | next chip = the armed Intro; pad 1–3 Armed |
| Fill queued / playing | next = `transport.landing`; count row "fill after bar N" |
| Ending with ritardando | as playing; nothing extra (`transport.ritardando` has no face yet) |
| Fader layer not Vol | kit › FaderStrip, Layers; header "Faders · Reverb" |
| Swap held (a part) | that part's lamp reads "Swap"; knob page block "Swap R1" on the part's hue; knobs show the part's mix |
| Sound held or latched | Sound lamp on; pads are the Racks page (#507) |
| Help mode | spec #508 |
| No Launchkey | app bar status hollow dot, `--d`; everything else works |
| No synth (`io.synth` null) | health slot "Audio off"; master strip unused; meters empty |
| Audio trouble, failed plugin | kit › App bar, health slot; the failed part's strip and sound cell show ✕ |
| Missing plugin | ⚠ on the strip name and the sound cell; the part is silent |
| A refusal or notice | the status line (`state.message`) |
| Style queued for the bar | the style line's queued chip (D3) |
| Style with fewer OTS | One Touch buttons past `ots.settings.length` disabled |
| Style without a section | its pad Absent (`level` off) |

Light theme: the same markup; only tokens change (kit › Tokens). No glow anywhere in light (every
glow token is `none` or a 0% mix).

## Board fixture

The state that reproduces the board, for the `Pages/Stage` story and its shots
(`app/src/ui/Stage/Stage.fixtures.ts`). Fields not listed take the dev mock's values.

- `style`: name "Sunday Drive Pop", timeSignature [4, 4]; library folder "Pop".
- `transport`: running, section "Main B", queued "Fill In BB", landing "Main C", bar 3, beat 3,
  sectionBars 4, beatsPerBar 4, tempo 104, acmp true, unison false, main 2, fade "off".
- `surface.clock`: beat 3 of 4 (beats 1–2 past, 3 current, 4 to come).
- `chord`: name "Am7", fingered "Am7", fingeringName "Fingered", leftHold false;
  `keyboard.chordTones` [9, 0, 4, 7].
- `ots`: four settings, applied 2. `home.bandSends` levels 40, 12, 0.
- `liveRack`: name "Sunday drive", modified true. `quickRacks`: bank 0, button 0 loaded.
- `keyboardParts` (R1, R2, R3, L): on true/true/false/true; sounds "Stage Grand" (number 1),
  "Silk Strings" (41, `soundEdited`), "Brass Section" (57, `plugin.missing`, status `failed`),
  "Silk Strings" (41).
- `surface.faders`: values 90, 72, 64, 80, 100, 90, —, —, 100; fader 2 waiting at position 50.
  Meters (left / right bar as a share of travel): R1 0.62 / 0.58, R2 0.45 / 0.42, R3 0, L
  0.5 / 0.47, Style 0.7 / 0.65, Multi Pad 0, Master 0.72 / 0.67; peak ticks 0.7, 0.55, —, 0.6,
  0.78, 0.12, —, —, 0.8.
- `mixer`: faderPage panel, faderLayer volume. Lamps: Harm/Arp, Sound, L Hold, Looper off.
- `knobs`: page style, 1 of 6: Dynamics 127 (level 127), Retrig rate "1/8" (51), Retrigger "Off"
  (0), Mute A "Off" (0), Mute B "Off" (0), Swing "0%" (0), No Assign, Tempo "104" (null).
- `pads`: Sections, page 1 of 5. Pads 3 and 7 `off`; pad 10 bright solid; pad 11 bright flash;
  pad 16 bright solid; the rest `dim`. `surface.controls` padBankUp action null, padBankDown set.
- `keyboard`: held 43, 45, 48, 52 (zone left, parts [3]) and 76, 81 (zone right, parts [0]);
  leftSplit 54 (F#2); detection [0, 54]. 61 keys.
- `pads.connected` true; health calm; `message` null; metronome off.

Two board texts differ from this fixture on purpose: the count row reads "fill after bar 3", not
4 (D5); the light board's `--m` is `#6e6e6e`, the tokens' `#646464`.

## Gap against today

| Area | In `app/src` now | Change |
|---|---|---|
| App bar | `panels/header/Header.svelte`: brand, Stage \| Library switch, `TransportBar.svelte` (Start, Sync, Intro, Ending, Tempo, Tap, bar.beat), Settings, help, theme; `App.svelte`'s quick-nav strip (`lib/nav.ts`) of drawer buttons | Replaced by kit › App bar: page tabs instead of drawers (`ui.page`, D2); the transport moves to the band and the pads; the theme switch leaves the Stage (D24) |
| Section row | none (ACMP is in `TransportBar`) | New: kit › Section row |
| Display | `panels/leadsheet` (LeadSheet, ChartLane), or `panels/channel` ChannelView in its place | Replaced by the display above; Charts hidden (DECISIONS X1); Channel becomes its own page (#501) |
| Hand surface | `panels/launchkey/Launchkey.svelte` (knobs and pads in one flat row, Sound beside Shift, pad-page tabs), `HwPad.svelte`, `Control.svelte` | Replaced by the band's Knobs and Pads (kit); the pad-lamp drawing logic (`lib/leds.ts`, LED clock) carries over |
| Mixer | `panels/mixer/MixerRow.svelte` (12 strips + master), `Strip.svelte` (a hotspot), `MixerBar`, `MasterStrip`, `StripDetail` | Replaced on the Stage by the 9-fader band (DECISIONS M1); strip details move to Channel (#501) |
| Quick Racks | `panels/knobracks/KnobRackPanel.svelte`, a row on the Stage | Off the Stage (Quick Racks tab, pad page 2, Sound hold, the rack readout) |
| Keys | `panels/keystrip/KeyStrip.svelte` with a cheek (chord tones, 49/61/88, Harm/Arp, Chord Looper) | The 56px kit › Key strip; chord tones move to the display; 49/61/88 leaves the Stage (D25); Harm/Arp and Looper are lamps |
| Footer | status footer, `lib/tooltip/HelpFooter.svelte`, `lib/DropoutNotice.svelte` | The status line (D15); help mode per #508; dropouts in the health slot |
| Controls | `lib/ui/Fader.svelte`, `Knob.svelte`, `HwButton.svelte`, `Toggle.svelte` (old tokens `--accent`, `--ink`) | The `app/src/ui` library: LampButton exists (#499); the rest are new kit components |
| Scaling | `App.svelte` scales rows by `--u` (1024 × 700 to 1920) | D1 |

New components (each its own `app/src/ui/<Name>/` with SPEC.md, per `docs/factory/spec-template.md`):
`AppBar`, `PageTabs`, `HealthSlot`, `CountRow`, `BeatBlocks`, `Button` (off face), `ChosenTabs`
(header tabs), `WaitingChip`, `StyleLine`, `OneTouch`, `ChordReadout`, `SectionReadout`,
`TempoReadout`, `SoundCell`, `RackReadout`, `FaderStrip`, `LampRow`, `Knob`, `KnobRow`, `Pad`,
`PadGrid`, `TransportColumn`, `KeyStrip`, `StatusLine`, and the page `Stage` (`Pages/Stage`,
fullscreen 1440 × 900). Components take props and call callbacks; the page wiring in the app
reads `app.state` and sends commands.

## Contract changes needed

None of these blocks building the screen; each says what the screen does until it lands.

1. **C1 · Loaded Quick Rack slot** (`src/api`, `docs/app-api.md`, `app/src/lib/api/types.ts`,
   `tests/fixtures/state.json`, both mocks): `quickRacks.loaded: { bank, slot } | null`, the
   button holding `liveRack.id` in any bank. Until then the rack readout shows the slot only when
   that bank is on view (D21).
2. **C2 · One Touch never asks** (`AppCmd`, both mocks, `EVERY_CMD`, docs): `recallOts` gains
   `recover: bool`; with it, an OTS that loads one of the user's racks keeps unsaved changes as
   "Recovered: <name>" and switches (as the Launchkey does), and `message` says so. Until then a
   screen OTS onto a rack with unsaved changes raises the rack prompt (Prompts, #504).
3. **C3 · Sound latch ends on the next pad** (`setLayer`, docs, mocks): `setLayer { layer: { type:
   'sound' }, untilPad: true }`: the session releases the layer after the next pad press from
   the screen or the hardware. Until then the screen releases it after a screen pad press only
   (D18).
4. **C4 · Chord held** (engine and API, new work): `chord.held: bool`, true while detection is
   unsure and the band keeps the last chord. The face is specified (D19) but not built until then.
   Contract files: `src/api`, `docs/app-api.md`, `tests/fixtures/state.json`,
   `app/src/lib/api/types.ts`, both mocks (`app/src/lib/api/mock.ts`, `app/src-tauri/src/mock.rs`).
   Real-time safety: the engine publishes the held state without allocation or locks on the
   engine and MIDI threads.
5. **C5 · Tooltips** (`app/src/help/tooltips.ts`, `app/docs/controls.md`): new keys
   `nav.channel`, `app.health`, `metronome.settings`, `display.band_sends`; rewrite the `nav.*`
   bodies from "opens the drawer" to "shows the page"; `launchkey.fader_sound` ("opens the quick
   sound list", not Library › Sounds); `ots.1`–`ots.4` (applies at once, Launchkey Racks page).

## Checks

Acceptance tests for vitest (`npx vitest run` on the page and component tests), each against a
state built from the board fixture:

1. Nine strips; strips 7 and 8 read "—", aren't focusable and send nothing.
2. Accomp is pressed with `transport.acmp` true; a click sends `toggleAcmp`.
3. One Touch: with `ots.applied` 2, button 2 is pressed; clicking 3 sends `recallOts {index: 2}`;
   with two settings, buttons 3 and 4 are `aria-disabled` and send nothing.
4. Count row: four blocks, block 3 the Main hue, block 1 with the top edge; reads "Bar 3/4",
   "Main B", "Main C" (outlined), "fill after bar 3". Stopped: no "Bar", blocks idle.
5. Display: chord "Am7" with tones A R, C m3, E 5, G m7 and "Fingered"; "Main B" next "Main C";
   "104 BPM"; "Running". Stopped: "Stopped".
6. Sounds row: R2's cell shows "41 Silk Strings" and the edited dot; R3's shows ⚠ and "off" and
   its tag is `--d`; the rack reads "Rack · A1", "Sunday drive" with the dot.
7. Pads: pad 10 has the Playing face, pad 11 Next ("NEXT"), pads 3 and 7 Absent; clicking a pad
   sends its `action`; Pad Bank ▲ is disabled when its action is null, ▼ sends its action.
8. Faders: dragging strip 1 sends its `set` with `volume` filled; strip 2 shows "↕" and the ghost;
   the Reverb tab sends `setFaderLayer {layer: 'reverb'}`, and with that layer strips 1–4 read
   "Rev …" and hide their meters.
9. Lamps: a click on R1's On sends `togglePart {part: 0}`; a 350 ms press sends `setLayer {swap,
   part 0}` and no toggle; Sound sends `setLayer sound`, and when lit `setLayer none`; Looper long
   press sends `looperRec`.
10. Knobs: ▲ disabled on page 1; ▼ sends `stepKnobPage {delta: 1}`; a drag on knob 1 sends
    `turnKnob`; a double-click sends `resetKnob {knob: 0}`; the tempo knob's arc is 27%.
11. Transport: Tempo + held repeats (existing `tempoHold` tests); Style tempo sends `resetTempo`;
    Fill ▲ sends `fillUp`.
12. Health slot: part 2 failed → "R3 failed" in `--ending`, click opens Channel for part 2;
    `meters.cpu.total` 0.74 → "CPU 74%"; else "Audio", not a button.
13. Status line shows `message.text`; with `error`, the ⚠; a click sends `clearMessage`.
14. Keys: notes 43, 45, 48, 52 are `--l`, 76 and 81 `--r1`; the split marker sits after F#2.
15. Every interactive element has a `data-tip` in the catalog (the existing tooltip test).
16. Light theme: the Running dot and pad glows compute to `box-shadow: none`.

Screenshot: `npm run shots -- Stage` renders `Pages/Stage` with the fixture at 1440 × 900 in both
themes against `docs/design/push/png/Stage-Dark.png` and `Stage-Light.png` (copied to
`app/src/ui/Stage/crops/Board-dark.png`, `Board-light.png`): at most 0.02 of pixels differ
outside the two known text differences (fixture note above).

## Decisions

- **D1 · Scaling.** The Stage is laid out at 1440 × 900 and scaled uniformly to fit the window
  (`scale = min(w / 1440, h / 900)`, centred, the rest `--g`). At 1024 × 700 that is 0.71. The
  board is fixed-size and the old `--u` scheme doesn't fit it; a responsive pass is a follow-up.
- **D2 · Pages.** The tabs set an app-only `ui.page` (`stage`, `channel`, `effects`,
  `quickRacks`, `multiPads`, `looper`, `harmArp`, `library`, `settings`), replacing the drawers.
  The display tabs (Channel … Harm/Arp) replace only the display; Library and Settings are full
  pages with the half band (DECISIONS S9, G1, L1). Each page's spec defines it.
- **D3 · Style name.** The name opens the Browser (the board draws it as a block; Browser has no
  tab, and the name is where a player looks for it). A style waiting for the bar line shows as
  "→ <name>" in the accent waiting face in place of the category, so the player sees it's coming.
- **D4 · Stopped.** The section readout shows the Main the band will start on
  (`transport.main`) in `--m` without glow; the next chip shows the armed Intro; the count row
  hides "Bar"; the run state reads "Stopped" or "Sync start".
- **D5 · "When" wording.** The count row's last item names the bar after which the change lands,
  from `transport.bar`: a fill starts on the next beat and ends with the bar, so it is "fill
  after bar 3" in bar 3. The board's "fill after bar 4" beside "Bar 3/4" is placeholder data.
- **D6 · Knob names.** The top name is a plain word by `function`: dynamics "Dynamics",
  retriggerRate "Retrig rate", retriggerOnOff "Retrigger", trackMuteA "Mute A", trackMuteB
  "Mute B", swing "Swing", tempo "Tempo", splitPoint "Split", harmonyArp "Harm/Arp",
  harmonyVolume "Harm level", metronomeVolume "Click level", swapSound "Sound"; any other
  function uses the state's `name`. The tempo knob's value drops a trailing " BPM".
- **D7 · Meters.** The state has one level per channel, so the twin bars are RMS (left) and
  peak (right), on a −60…0 dBFS scale, peak held 1.5 s then falling 20 dB/s. Master has its
  own peak and RMS per side in the `meters` frame (`meters.master` and `meters.masterRms`, each
  [l, r], after the soft clipper), drawn the same way per side.
- **D8 · Pan face.** In the Pan layer a strip's fill grows from the middle of the track (64) up
  for right, down for left; the cap sits at the value.
- **D9 · Art.** One static gradient pair from tokens, the board's own. Per-style artwork and a
  per-style key colour don't exist yet (DECISIONS S4); the accent stays `--a`.
- **D10 · Detection line.** It spans `keyboard.detection`: teal (`--l`) when that is the left
  hand, accent (`--a`) in Upper or Full Keyboard, so it never claims the right hand is Left.
- **D11 · Pad faces.** A lit utility switch (Auto Fill, Sync Stop) wears the lamp face (switched
  on), not a white block (white means chosen). A pulsing Main is the landing, drawn Next, not
  Armed. Start / Stop's hue is `--ok`.
- **D12 · Part lamps.** The lamp lights from `sounding` (as the state says to) and its label
  from `on`. While a part's swap is held its lamp reads "Swap".
- **D13 · Fade and Looper faces.** Fade armed is the waiting face; fading or holding is on.
  Looper: looping on, recording Record, armed states the waiting face (`--rec` for REC,
  `--lamp` for the loop).
- **D14 · Health order.** A failed plugin first (it's silent), then no synth, the buffer hint,
  dropouts, CPU, calm. Dropouts count over the last 30 s, the same window as the buffer hint.
- **D15 · Status line.** `state.message` shows in the 20px gap between the band and the keys,
  where Prompts (#504) draws it; a click clears it. The Stage board has no status line, and
  DECISIONS S8 wants one.
- **D16 · One Touch on the Launchkey.** The board's "Shift + pads 9–12" can't work: the
  Launchkey firmware keeps Shift + pad for itself (app-api.md › surface). One Touch stays on the
  Racks pad page's bottom row, pads 1–4.
- **D17 · One Touch applies at once.** A click sends `recallOts` with no dialog of the screen's
  own. A rack prompt can still come from the session until C2.
- **D18 · Sound latch.** A click latches Sound; the next screen pad press, or a click on Sound,
  releases it (`setLayer none` after the pad's action). Hardware pad taps release it once C3
  lands. A press over 350 ms is momentary. No timer (FIX-DEBATE: no timers).
- **D19 · Chord held.** When `chord.held` exists (C4) the chord dims to `--m` and a small "held"
  (14 `--m`) follows the fingering name; until then the playing face only.
- **D20 · Chord type.** The chord shrinks to fit its 300px (to 64px). Tones are spelled with
  flats when the chord's root has a flat or is F, otherwise sharps; intervals as listed, with a
  minor third and a major third both present read as #9 and 3.
- **D21 · Rack slot.** Shown only when the loaded rack's button is in the bank on view, until C1.
- **D22 · Sound name.** `sound.name` (the library's current name) before `voiceName`, so a
  rename shows at once; the number only for the user's numbered sounds.
- **D23 · Fader drag.** The value follows the pointer's travel (223px = 127) from where it was
  pressed, not a jump to the pointer; double-click resets to the strip's default (levels 100,
  pan 64, sends 0).
- **D24 · Theme switch.** Not on the Stage (the board has none): it moves to Settings › System
  (#532); `ui.theme` and `data-theme` stay as they are.
- **D25 · Keyboard size.** 49 / 61 / 88 isn't on the Stage; it is on the tall pages' key row
  (#501) and Settings › Keyboard (#530). The Stage uses `ui.keyRange` or the Launchkey's.
- **D26 · Category.** "Pop" is the style's library folder (the last folder name), since styles
  carry no genre.
- **D27 · Tempo readout.** Read-only; the tempo is set with Tempo ± , Tap, Style tempo and knob 8.
- **D28 · Section dot.** The dot by "Section" shows only while the band runs, in the section's
  hue.

## Follow-ups

- Per-style artwork and key colour (DECISIONS S4), with what generates them.
- A responsive layout for 1024 × 700 to 1920 instead of the uniform scale (D1, DECISIONS S14).
- C4, chord held, needs the engine to know when detection is unsure.
- Stereo meters per part would need per-channel left and right levels in `meters`.
- A face for the Ending's ritardando (`transport.ritardando`).
- Typed tempo entry on the tempo readout, if players ask.

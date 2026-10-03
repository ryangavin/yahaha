# Stage

The main play screen: what is playing, what comes next, the sounds under your hands, and the
whole Launchkey band below it. The screen a player looks at while playing.

- **Issue:** #500 · **Flow:** Play · **Boards:** `docs/design/push/Stage-Dark.dc.html`,
  `Stage-Light.dc.html`; pictures `docs/design/push/png/Stage-Dark.png`, `Stage-Light.png`
  (1440 × 900). The spec stands without them: every value a builder needs is below or in
  [kit.md](kit.md); the board lines in the Components table are for cutting crops.
- **Built from:** [kit.md](kit.md): App bar, Section row, Full band, Key strip, Status line,
  the faces and tokens. This file binds them to the Stage and specifies the display, which is
  the Stage's own.
- **Variants of this screen** (they spec only what differs): Stage-Help (#508), Stage-Metronome
  (#509), FirstRun (#506), PadsPage2 (#507), PadsChord (#510), PadsMultiPads (#511), PadsSetup
  (#512).
- **Glance order** (what must read first, brightest to quietest): the chord, the playing section
  (44px in its hue), the next section (36px, outlined), the tempo (32px). Nothing decorative
  outshines them: the art's core stays near 35% lightness.
- **Before building:** C5 (tooltips) must land first; the rest of the contract changes don't
  block (see Contract changes needed).

## Layout

At 1440 × 900. The `Stage` component always lays out at 1440 × 900; the app shell scales it to
the window (D1), the component never does. Padding 24 all round; a column.

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

`24,112 1392×300`, no surface: ground with the art on its right, `position: relative`,
`overflow: hidden`, and a 1px transparent border (the board's; it shifts everything inside by
1px, D42). Inside the border the box is `25,113 1390×298`. The content box is
`49,129 814×266` (left 24, top 16 inside the border), a column:

| Part | Box | Height |
|---|---|---|
| Style line | `49,129 814×32` | 32 |
| Chord (left) and Section (right) | `49,169 300×162` and `373,169 490×162` (grid 300 + 490, gap 24) | 162, 8 below the style line |
| Sounds row | `49,351 814×44` | 44, 20 below |
| Art | `886.8,113 528.2×298` (right 0, width 38% of the 1390 inside, top 0, bottom 0) | behind, `aria-hidden` |

### Style line

One row, 32 tall, items centred, gap 8, no wrap. Everything is `flex: none` except the style
name, which shrinks (D34).

| Control | Face | Reads | Sends / does | Tooltip | Launchkey |
|---|---|---|---|---|---|
| ◀ | 32 × 32 off face, 12px | `surface.controls[trackPrev]` | its `action` (`stepStyle { delta: -1 }`); with `ui.shift`, its `shiftAction`; disabled when that is null, or (without Shift) when `surface.trackPrev` is null (D38) | `style.prev` | Track ◀ |
| Style name | accent block (a text button): padding 0 10, 18 / 500, line-height 26, `--g` on `--a`, no radius; `min-width: 0`, ellipsis | `style.name` | opens the Browser (`ui.browser = true`; app-only; D3) | `browser.open` | — |
| ▶ | as ◀ | `surface.controls[trackNext]`, `surface.trackNext` | as ◀ (`stepStyle { delta: 1 }`) | `style.next` | Track ▶ |
| Category · metre | 14 / 400 `--m`, 4px extra left margin; not a control | the last `/`-segment of the `folder` of the library entry whose `id` is `style.id` (from `library()`), " · ", then `style.timeSignature` as "4/4"; empty folder, entry not found or library not loaded: the metre alone; no time signature: the category alone (D44) | — | `display.timesig` | — |
| One Touch | `margin-left: auto`, a `role="group"`, gap 4: "One Touch" 14 `--m` (4px right margin), then four 32 × 32 buttons "1"–"4", 14px | `ots.applied` (1-based, 0 = none): that button is the chosen face (`aria-pressed="true"`); buttons past `ots.settings.length` disabled | `recallOts { index }` (0-based) at once; D17 | `ots.1` … `ots.4` | Racks pad page, bottom row pads 1–4 (D16) |
| Band sends | one text button (D34), 16px left margin, 24 tall, items on the baseline, gap 10, 14 / 400: "Band" in `--m`, then "Reverb ", "Chorus ", "Delay " in `--t2`, each followed (after its normal space) by its value 18 / 300 `--a` | `home.bandSends[0..2].level` (reverb, chorus, delay) | opens the Effects page (D32) | `display.band_sends` (new) | — |

`aria-label`s: ◀ "Previous style (Track left)", ▶ "Next style (Track right)", style name
"{name}: open the Browser", One Touch group "One Touch: {n} applied" ("none applied"), each
button "One Touch {n}" plus ", applied" when chosen, band sends "Band sends: reverb {r}, chorus
{c}, delay {d}. Opens Effects".

**Queued style.** While a style waits for the bar line (`preview.queued` not null), the category
· metre text is replaced by "→" 14 `--m` and, gap 8, a waiting chip (D3, D34): 26 tall, padding
0 8, 1px `--a` border, radius 4, 14 / 400, line-height 24, `--a` text, `max-width: 200px`,
ellipsis. Its text is the `name` of the library entry whose `id` is `preview.queued`; not found
or library not loaded: "next style". Not a control.

### Chord

Left column, 300 wide.

- **Label** "Chord", 14 `--m`, 16 tall, line-height 16.
- **Chord**, 4px below, 104 tall, no wrap: `chord.name` at 128 / 300, line-height 104,
  letter-spacing −6, `--a`, `text-shadow: var(--ba2)`. It is drawn in two runs (D30): the
  **base** (the root, `C`…`B` with an optional `#` or `b`, then a leading quality `m`, `dim` or
  `aug` if the rest starts with one, except that the `m` of `maj` is not a quality) at weight
  300, and the **extension** (everything after the base, slash bass included) at weight 200,
  letter-spacing 0. Examples: `Am|7`, `C|maj7`, `C#m|7b5`, `Ebm|Maj7`, `Gdim|7`, `Fm|(add9)`,
  `C|6/9`, `D|1+8`, `C|(b5)`, `C|/E`, `Am|7/G`, `Csus4` → `C|sus4`, plain `F` (no extension).
  "N.C." (the cancel chord) is all base. No chord (`chord.name` null): "—" in `--d`, no shadow.
  **Fit:** when the chord at 128px is wider than 300, the font shrinks to
  `max(64, floor(128 × 300 / width))` px, where `width` is the element's `scrollWidth` at 128px,
  measured after each change of `chord.name` (D20); letter-spacing scales with it (−6 × size /
  128).
- **Tones**, 6px below, 32 tall, items at the bottom, gap 16: one column per tone of the chord
  shown, root first, at most six. The tones are `keyboard.chordTones` moved by
  `chord.transposeKeyboard` (`(pc + transposeKeyboard) mod 12`, kept in 0–11), so they are the
  tones of `chord.name`, not of the chord as fingered (D31). Each column: `min-width: 24px`,
  centred, the note name (20 / 300, line-height 18, `--t`) over its interval from the root (12 /
  400, line-height 14, `--m`): R, b9, 9, m3, 3, 4, b5, 5, #5, 6, m7, M7 for 0–11 semitones; a
  chord with both a minor and a major third reads the minor one as #9 (D20). Spelling: flats
  when the chord's root is written with `b` or is F, else sharps (C C# D D# E F F# G G# A A# B, or
  C Db D Eb E F Gb G Ab A Bb B). Then, 4px further, `chord.fingeringName` 14 `--m`, line-height
  16 ("Fingered"). When `chord.fingered` differs from `chord.name` (Keyboard transpose), it
  reads "{fingeringName} · played {fingered}" ("Fingered · played Gm7"). No chord: no tones,
  the fingering name only.
- Tooltip `display.chord` on the column. The column is not a control; it has
  `aria-label` "Chord {name}: {tones as note names}, {fingeringName}" ("Chord Am7: A C E G,
  Fingered"; no chord: "No chord, {fingeringName}").

### Section and tempo

Right column, 490 wide.

- **Label** "Section", 14 `--m`, 16 tall, items centred, gap 8, after a 6px round dot in the
  playing section's hue with its glow (D29: `0 0 6px`, `--dot-glow-mix`), shown only while
  running (D28). Stopped, the dot is `visibility: hidden` and keeps its space (D43).
- **Playing and next**, 4px below, 52 tall, items centred, gap 16, no wrap: the playing
  section's name (kit › Section names) at 44 / 300, line-height 52, letter-spacing −1.5, in its
  hue, `text-shadow: 0 0 18px color-mix(in srgb, <hue> var(--text-glow-mix), transparent)`
  (D29); then "next" 14 `--m`; then the next section in the waiting face: 48 tall, padding 0 10,
  1px border, radius 4, 36 / 300, line-height 46, letter-spacing −1, border and text in its hue.
  Which section is playing and which is next follow the count row's rules (kit › Count row, item
  3); with no next, "next" and the chip are hidden. Stopped: D4.
- **Tempo row**, 50px below, 40 tall, items on the baseline, gap 6: `transport.tempo` rounded to
  a whole BPM (`Math.round`, D44) at 32 / 300, line-height 40, letter-spacing −0.5, `--t`;
  "BPM" 14 `--m`; right-aligned (`margin-left: auto`, centred vertically), the run state, 14 /
  400, a 6px dot then the word, gap 8: `transport.running` → "Running", dot and text `--ok`, dot
  glow `--bg`; stopped with `transport.syncStart` → "Sync start", text `--t2`, dot a hollow 1px
  `--ok` ring; stopped → "Stopped", `--m`, no dot (the dot's space collapses). Not controls.
  Tooltip `display.tempo` on the number.

The board doesn't repeat the count row's fill line here.

### Sounds row

44 tall, grid `200px repeat(4, minmax(0, 1fr))`, gap 8 (cells 145.5 wide). Every cell has a 1px
`--line` top edge (box-sizing border-box, so the content is 43 tall).

- **Rack** (a text button, the 200 cell, a column, gap 2, left-aligned): "Rack · A1" 12 `--m`
  with the slot in `--t`, over the name 14 / 400, line-height 16, `--t`, then a 5px round `--t`
  dot when `liveRack.modified` (gap 6). Name = `liveRack.name` (it reads "Recovered: Sunday
  drive" itself when it is one); ellipsis. Slot = the bank letter (`A` + `quickRacks.bank`) and
  button number (index + 1) of the `quickRacks.buttons` entry with `loaded` true; no such
  button: "Rack" alone (D21). Click: opens the Rack page (D2, D32). Tooltip `stage.rack_name`.
  `aria-label` "Rack: {name}{, modified}{, on Quick Rack A1}. Opens the Rack page".
- **R1, R2, R3, L** (one cell each, part order 0–3): two text buttons, gap 4, items centred.
  - **Tag** "R1" / "R2" / "R3" / "L", 32 × 43, left-aligned, 14 / 500 in the part hue (`--d`
    when the part doesn't sound, `keyboardParts[i].sounding` false). Click: opens Channel for
    the part (D32). Tooltip `mixer.strip.select`. `aria-label` "{part name}: open Channel".
  - **Sound**, the rest of the cell, gap 8, no wrap: the sound number (12 `--m`; `--d` when not
    sounding), the name (14 `--t`, `min-width: 0`, ellipsis; `--m` when not sounding), then the
    marks (kit › FaderStrip, name button marks): a 5px `--t` dot when `soundEdited`; a 12px
    `--warn` ⚠ when `plugin.missing`; a 12px `--ending` ✕ when `plugin.status` is `failed` and
    not missing; "off" 12 `--m` when `on` is false and the part doesn't sound. Number =
    `soundLibrary.patches[].number` of the patch whose id is in `sound.id` (`saved:<id>`); none:
    no number. Name = `sound.name`, else `voiceName` (D22). Left under Manual Bass
    (`playsBass`): it sounds, so it is drawn as on even with `on` false; the name is the Style's
    Bass voice (`voiceName`) and "bass" 12 `--m` follows. Click: opens the quick sound list for
    the part (spec #514; D32). Tooltip `launchkey.fader_sound`.
  - `aria-label` template: "{part name} sound: {number }{name}{, edited}{, off}{, plugin
    missing | , plugin failed}{, bass}. Opens the quick sound list" — "Right 2 sound: 41 Silk
    Strings, edited. Opens the quick sound list", "Right 3 sound: 57 Brass Section, off, plugin
    missing. Opens the quick sound list".

### Art

The display's right 38% (box above): two stacked layers, each `position: absolute; top: 0;
right: 0; bottom: 0; width: 38%`, the first `background: var(--stage-art)`, the second over it
`background: var(--stage-art-fade)` (the values are in kit › Tokens). Static: it doesn't follow
the style yet (D9). `aria-hidden`, `pointer-events: none`, behind the content.

## Band, keys and status on the Stage

The kit's full band, key strip and status line, unchanged; the Stage adds nothing. The fader page
is whatever `mixer.faderPage` is: Panel shows the strips in kit › Faders; Style shows the Style
parts (#507). The pads show the hardware's page (`pads.page`); Sections is drawn here, the other
pages in their specs (#507, #510–#512) and, until those land, with the kit's fallback face (kit ›
Pads, D33). While Sound is held or latched the pads are the Racks page (#507; fallback until
then).

## States

| State | What changes |
|---|---|
| Playing (the board) | as drawn |
| Stopped | D4: count row blocks idle and "Bar" hidden; section name in `--m`, no glow, the dot hidden (space kept); run state "Stopped"; Start / Stop without its bar; pad 16 idle |
| Sync Start armed | run state "Sync start"; count row's last item "sync start"; pad 4 Armed |
| Intro armed (stopped) | next chip = the armed Intro; pad 1–3 Armed |
| Fill queued / playing | next = `transport.landing`; count row "fill after bar N" |
| Ending with ritardando | as playing; nothing extra (`transport.ritardando` has no face yet) |
| Fader layer not Vol | kit › FaderStrip, Layers; header "Faders · Reverb" |
| Swap held (a part) | that part's lamp reads "Swap"; knob page block "Swap R1" on the part's hue; knobs show the part's mix |
| Sound held or latched | Sound lamp on; pads are the Racks page (fallback face until #507) |
| Pads on another page | fallback face (D33) until that page's spec lands |
| Help mode | spec #508 |
| No Launchkey | app bar status hollow dot, `--d`; everything else works |
| No synth (`io.synth` null) | health slot "Audio off"; master strip unused; meters empty |
| Audio trouble, failed plugin | kit › App bar, health slot; the failed part's strip and sound cell show ✕ |
| Missing plugin | ⚠ on the strip name and the sound cell; the part is silent |
| A refusal or notice | the status line (`state.message`) |
| Style queued for the bar | the style line's queued chip (D3, D34) |
| Style with fewer OTS | One Touch buttons past `ots.settings.length` disabled |
| Style without a section | its pad Absent (`level` off) |
| No chord | "—" in `--d`, no tones |

Light theme: the same markup; only tokens change (kit › Tokens). Every glow token is `none` or a
0% mix in light, so no glow draws (a 0% `color-mix` shadow is transparent: it draws nothing,
though its computed value isn't the keyword `none`).

## Board fixture

The state and moment that reproduce the board, for the `Pages/Stage` › `Board` story and its
shots: `app/src/ui/Stage/Stage.fixtures.ts` exports `boardState` (a full `AppState`), `boardNow`
and `boardMeterHolds`. Fields not listed take the dev mock's values (`app/src/lib/api/mock.ts`
initial state).

- **Moment:** `boardNow = 10000` (the session clock, ms), and the state counts as received at
  that moment (`receivedMs = 10000`), so `t = clock.atMs = 10000`.
- `surface.clock`: atMs 10000, running true, tempo 104, beatsPerBar 4, bar 3, beat 3, phase 0.25,
  sectionAnchorMs 10000, sectionAnchorBeats 10.25 (bar 3 beat 3: 2 bars and 2 beats past, a
  quarter into the beat), ledAnchorMs 10000, ledAnchorBeats 0.25. So the current beat block is 3
  (`floor(10.25) mod 4 = 2`, 0-based), and the LED clock reads 0.25: `frac < 0.5`, flashing pads
  at full (kit › Pad).
- `style`: name "Sunday Drive Pop", timeSignature [4, 4]; the library entry for `style.id` has
  folder "Pop". `preview.queued` null.
- `transport`: running true, section "Main B", queued "Fill In CC", landing "Main C" (Fill ▲
  from Main B queues Main C's fill and lands on Main C, as the engine does), bar 3, beat 3,
  sectionBars 4, beatsPerBar 4, tempo 104, acmp true, unison false, syncStart false, main 2,
  pendingIntro null, fade "off", ritardando false.
- `styleSettings.mainTiming` "nextBar", `introEndingTiming` "nextBar".
- `chord`: name "Am7", fingered "Am7", fingeringName "Fingered", transposeKeyboard 0, leftHold
  false; `keyboard.chordTones` [9, 0, 4, 7].
- `ots`: four settings, applied 2. `home.bandSends` levels 40, 12, 0.
- `liveRack`: name "Sunday drive", modified true. `quickRacks`: bank 0, button 0 `loaded` true,
  the rest false.
- `keyboardParts` (R1, R2, R3, L): on true / true / false / true; sounding true / true / false /
  true; playsBass false; channels 1, 3, 4, 2; sounds "Stage Grand" (number 1), "Silk Strings"
  (41, `soundEdited` true), "Brass Section" (57, `plugin.missing` true, `plugin.status`
  "failed"), "Silk Strings" (41). (R3 is missing, so it shows ⚠, not ✕, and the health slot's
  "failed" row skips it: calm.)
- `mixer`: faderPage "panel", faderLayer "volume", styleVolume 100, multiPadVolume 90, master
  100. `surface.layer` none.
- `surface.faders` (9): values 90, 72, 64, 80, 100, 90, null, null, 100; labels "Right 1",
  "Right 2", "Right 3", "Left", "Style", "Multi Pad", "", "", "Master"; `set` null for 7 and 8,
  the matching set command for the rest; fader 2 `waiting` true at `position` 50, the rest
  waiting false.
- `meters` (atMs 10000), as linear amplitudes; the bar heights they give in px are in brackets:
  channel 1 (R1) peak 0.0724 [138], rms 0.0537 [129]; channel 3 (R2) 0.0224 [100], 0.0180 [93];
  channel 4 (R3) 0, 0; channel 2 (L) 0.0316 [112], 0.0248 [104]; Style, channel 9 0.1259 [156],
  0.0897 [145], channels 10–16 0; Multi Pads, channels 5–8 0; `master` [0.1445, 0.1445] [161],
  `masterRms` [0.1020, 0.1020] [149]; `cpu.total` 0.2.
- `boardMeterHolds` (the nine held peaks, strips 1–9, linear; tick bottoms in px): 0.1259 [161],
  0.0447 [128], 0, 0.0631 [139], 0.2188 [179], 0.0023 [32], 0, 0, 0.2512 [183].
- Lamps: `harmonyArp.on` false, `chord.leftHold` false, `looper.mode` "off", `metronome.on`
  false.
- `knobs`: page "style", pageName "Style", pageNumber 1, pageCount 6; knobs 1–8 (function,
  value, level, short): dynamics "127" 127 "DynCtrl"; retriggerRate "1/8" 51 "RtgRate";
  retriggerOnOff "Off" 0 "RtgOnOff"; trackMuteA "Off" 0 "StyMuteA"; trackMuteB "Off" 0
  "StyMuteB"; swing "0%" 0 "Swing"; none "" null "---"; tempo "104 BPM" null "Tempo". (Tempo arc
  `(104 − 40) / 240 = 0.267`, 72°.)
- `pads`: page "sections", pageName "Sections", pageNumber 1, pageCount 5, connected true. Pads
  1–16 (level, anim): 1 dim, 2 dim, 3 off, 4 dim, 5 dim, 6 dim, 7 off, 8 dim, 9 dim, 10 bright
  solid, 11 bright flash, 12 dim, 13 dim, 14 dim, 15 dim, 16 bright solid; every anim not named
  is solid; actions as the dev mock's Sections page. `surface.controls`: padBankUp `action` and
  `shiftAction` null; padBankDown set; trackPrev and trackNext set, with `surface.trackPrev` and
  `trackNext` not null.
- `keyboard`: held 43, 45, 48, 52 (zone left, parts [3]) and 76, 81 (zone right, parts [0]);
  leftSplit 54 (F#2); detection [0, 54]. `ui.keyRange` 61.
- `io.synth` set (bufferFrames 256, dropouts 0); `message` null; `tips.help` false.

One board text differs from this fixture on purpose: the count row reads "fill after bar 3", not
4 (D5); the screenshot check masks it. The light board's `--m` (`#6e6e6e`) and `--lamp-ink`
(`#111`) differ from the tokens (`#646464`, `#000`); both are under the screenshot diff's
per-pixel threshold, so they need no mask.

## Components

Every part of the screen, in build order: a component is built only after everything in its
"Built from" column. Each gets `app/src/ui/<Name>/` with a SPEC.md per
`docs/factory/spec-template.md`; its Boards line and crops come from the board lines below (dark
board / light board; crop boxes are the boxes in this spec and kit.md). "Exists" is whether it
is in `app/src/ui` today.

| # | Component | Kind | Built from | Exists | Board lines (dark / light) | Spec |
|---|---|---|---|---|---|---|
| 0 | tokens | — | — | yes; add the kit's new tokens | `:root` lines 60, 65 / 36, 41; art 124–125 / 100–101 | kit › Tokens |
| 1 | `longpress` (Svelte action, `app/src/ui/actions/`) | primitive | — | no | — | kit › Interaction conventions |
| 2 | LampButton | primitive | longpress | yes; add `join: 'start'` (radius `4px 0 0 4px`), `onlongpress`, `onlongrelease` | 97, 116, 277–284 / 73, 92, 253–260 | kit › Faces, Lamp row |
| 3 | Button | primitive | longpress | no | 117–118, 131, 133, 299–300, 334–335, 366–386 / −24 | kit › Faces (off): variants `icon` 32 × 32, `md` padding 0 14, `band` 88 × 32 left-aligned, `pair` 41 × 32 |
| 4 | ChosenTabs | primitive | — | no | 74–84, 226–237 / 50–60, 202–213 | kit › App bar (`size page`: 36 tall, 24px block), Faders header (`size header`: 35, 22px) |
| 5 | WaitingChip | primitive | — | no | 110, 166 / 86, 142 | sizes `count` (26, 18px), `line` (26, 14px), `display` (48, 36px) |
| 6 | AccentBlock | primitive | — | no | 132, 294 / 108, 270 | kit › Faces (accent block); as a button (style name) or a span (knob page) |
| 7 | StatusDot | primitive | — | no | 89, 161, 171 / 65, 137, 147 | 6px: solid with a glow, or hollow 1px ring |
| 8 | PartMarks | primitive | — | no | 194, 202–203, 262–264 / 170, 178–179, 238–240 | edited dot, ⚠, ✕, "off", "bass" |
| 9 | GroupHeader | primitive | — | no | 224, 292, 320, 361, 379 / −24 | kit › Spacing and shape (36 tall hairline row, title, slots) |
| 10 | BeatBlocks | primitive | — | no | 100–105 / 76–81 | kit › Count row, item 1 |
| 11 | FaderStrip | primitive | — | no | 244–266; data 425–473 / 220–242; 397–445 | kit › FaderStrip |
| 12 | Knob | primitive | — | no | 305–313; data 477–494 / 281–289; 449–466 | kit › Knob |
| 13 | Pad | primitive | — | no | 349–353; data 498–515 / 325–329; 470–487 | kit › Pad |
| 14 | KeyStrip | primitive | — | no | 392–403; data 518–538 / 368–379; 489–509 | kit › Key strip |
| 15 | StatusLine | primitive | — | no | not on this board (Prompts, #504) | kit › Status line |
| 16 | HealthSlot | primitive | — | no | 91 / 67 | kit › App bar, health slot |
| 17 | ChordReadout | primitive | — | no | 148–158 / 124–134 | Display › Chord |
| 18 | StageArt | primitive | — | no | 124–125 / 100–101 | Display › Art |
| 19 | BandSends | primitive | — | no | 143 / 119 | Display › Style line |
| 20 | RackReadout | primitive | StatusDot | no | 178–181 / 154–157 | Display › Sounds row |
| 21 | SoundCell (tag + sound) | complex | PartMarks | no | 182–212 / 158–188 | Display › Sounds row |
| 22 | PageTabs | complex | ChosenTabs | no | 74–85 / 50–61 | kit › App bar |
| 23 | LaunchkeyStatus | complex | StatusDot | no | 89 / 65 | kit › App bar |
| 24 | AppBar | complex | PageTabs, LaunchkeyStatus, HealthSlot | no | 72–93 / 48–69 | kit › App bar |
| 25 | CountRow | complex | BeatBlocks, WaitingChip | no | 99–113 / 75–89 | kit › Count row |
| 26 | MetronomeSplit | complex | LampButton, Button | no | 115 / 91 | kit › Section row |
| 27 | SectionRow | complex | LampButton, CountRow, MetronomeSplit, Button | no | 96–120 / 72–96 | kit › Section row |
| 28 | OneTouch | complex | Button | no | 136–142 / 112–118 | Display › Style line |
| 29 | StyleLine | complex | Button, AccentBlock, WaitingChip, OneTouch, BandSends | no | 130–144 / 106–120 | Display › Style line |
| 30 | TempoReadout | complex | StatusDot | no | 168–172 / 144–148 | Display › Section and tempo |
| 31 | SectionReadout | complex | StatusDot, WaitingChip, TempoReadout | no | 160–173 / 136–149 | Display › Section and tempo |
| 32 | SoundsRow | complex | RackReadout, SoundCell | no | 177–213 / 153–189 | Display › Sounds row |
| 33 | Display | complex | StyleLine, ChordReadout, SectionReadout, SoundsRow, StageArt | no | 123–218 / 99–194 | Display |
| 34 | LampRow | complex | LampButton, Button | no | 271–286 / 247–262 | kit › Lamp row |
| 35 | FaderBank | complex | GroupHeader, ChosenTabs, FaderStrip, LampRow | no | 223–288 / 199–264 | kit › Faders |
| 36 | KnobBank | complex | GroupHeader, AccentBlock, Button, Knob | no | 291–317 / 267–293 | kit › Knobs |
| 37 | PadGrid | complex | Pad | no | 337–355 / 313–331 | kit › Pads (grid, group lines) |
| 38 | PadBank | complex | GroupHeader, Button, PadGrid | no | 319–357 / 295–333 | kit › Pads |
| 39 | TransportColumn | complex | GroupHeader, Button | no | 360–388 / 336–364 | kit › Transport and tempo |
| 40 | FullBand | complex | FaderBank, KnobBank, PadBank, TransportColumn | no | 221–389 / 197–365 | kit › Full band |
| 41 | Stage (page, `Pages/Stage`) | complex | AppBar, SectionRow, Display, FullBand, StatusLine, KeyStrip | no | whole board | this file |

Components take props and call callbacks; none reads `app.state` or sends. The page wiring in
the app (`app/src/pages/StageWiring.svelte`, outside `app/src/ui`) reads `app.state`, keeps
`now`, `receivedMs` and the meter holds, passes them down, and sends commands.

## Gap against today

| Area | In `app/src` now | Change |
|---|---|---|
| App bar | `panels/header/Header.svelte`: brand, Stage \| Library switch, `TransportBar.svelte` (Start, Sync, Intro, Ending, Tempo, Tap, bar.beat), Settings, help, theme; `App.svelte`'s quick-nav strip (`lib/nav.ts`) of drawer buttons | Replaced by kit › App bar: page tabs instead of drawers (`ui.page`, D2; drawers until each page lands, D32); the transport moves to the band and the pads; the theme switch leaves the Stage (D24) |
| Section row | none (ACMP is in `TransportBar`) | New: kit › Section row |
| Display | `panels/leadsheet` (LeadSheet, ChartLane), or `panels/channel` ChannelView in its place | Replaced by the display above; Charts hidden (DECISIONS X1); Channel becomes its own page (#501), ChannelView in the display's place until then |
| Hand surface | `panels/launchkey/Launchkey.svelte` (knobs and pads in one flat row, Sound beside Shift, pad-page tabs), `HwPad.svelte`, `Control.svelte` | Replaced by the band's Knobs and Pads (kit); the pad-lamp drawing logic (`lib/leds.ts`, LED clock) carries over |
| Mixer | `panels/mixer/MixerRow.svelte` (12 strips + master), `Strip.svelte` (a hotspot), `MixerBar`, `MasterStrip`, `StripDetail` | Replaced on the Stage by the 9-fader band (DECISIONS M1); strip details move to Channel (#501) |
| Quick Racks | `panels/knobracks/KnobRackPanel.svelte`, a row on the Stage | Off the Stage (Quick Racks tab, pad page 2, Sound hold, the rack readout) |
| Keys | `panels/keystrip/KeyStrip.svelte` with a cheek (chord tones, 49/61/88, Harm/Arp, Chord Looper) | The 56px kit › Key strip; chord tones move to the display; 49/61/88 leaves the Stage (D25); Harm/Arp and Looper are lamps |
| Footer | status footer, `lib/tooltip/HelpFooter.svelte`, `lib/DropoutNotice.svelte` | The status line (D15); help mode per #508; dropouts in the health slot |
| Controls | `lib/ui/Fader.svelte`, `Knob.svelte`, `HwButton.svelte`, `Toggle.svelte` (old tokens `--accent`, `--ink`) | The `app/src/ui` library (Components above): LampButton exists (#499); the rest are new |
| Scaling | `App.svelte` scales rows by `--u` (1024 × 700 to 1920) | D1, in the app shell |
| Shortcuts | `lib/nav.ts`, `lib/keys.ts`: Alt letters `bsropemlchyt` | Add Alt+G (Stage) and Alt+N (Channel) (D37) |
| Screenshot tool | `scripts/shots.ts`: one 1000 × 600 viewport, the story root's box as the shot, no masks | **Build-time item** (the Stage lane, before its screenshot check can pass; D39): a story may set `parameters.shots = { viewport: { width, height }, mask: [<selector>…] }`; the page uses that viewport, and the diff ignores each masked element's box (taken from the rendered story, applied to both images, and left out of the score's pixel count) |

## Contract changes needed

C5 blocks the build: without its keys `TipKey` doesn't type-check (`npm run check` fails) and the
tooltip catalog test (Check 15) fails. It lands first, as its own small contract PR. C1–C4 don't
block; each says what the screen does until it lands.

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
5. **C5 · Tooltips** (`app/src/help/tooltips.ts`, `app/docs/controls.md`), **lands before the
   build**: new keys `nav.channel` (with Alt+N), `app.health` (one body covering calm, CPU,
   dropouts, the buffer hint, no synth and a failed plugin, D45), `metronome.settings`,
   `display.band_sends`; `view.stage` gains Alt+G; rewrite the `nav.*` bodies from "opens the
   drawer" to "shows the page"; `launchkey.fader_sound` ("opens the quick sound list", not
   Library › Sounds); `ots.1`–`ots.4` (applies at once, Launchkey Racks page).

## Checks

Vitest (`npx vitest run` on the page and component tests), each against the board fixture
unless it says otherwise. They read roles, names, attributes and the commands sent (a fake
`send`), and the `data-face` / `data-hue` hooks (kit › Faces, D41); never computed colours or
layout, which jsdom doesn't have.

1. Nine strips; strips 7 and 8 read "—", have no `tabindex`/aren't focusable and send nothing
   when dragged or wheeled.
2. Accomp has `aria-pressed="true"`; a click sends `toggleAcmp`.
3. One Touch: button 2 has `aria-pressed="true"` and `data-face="chosen"`; clicking 3 sends
   `recallOts {index: 2}`; with two settings, buttons 3 and 4 are `aria-disabled="true"` and
   send nothing.
4. Count row: four blocks; block 3 has `data-hue="main"` and `data-beat="current"`, blocks 1–2
   `past`, block 4 `later`, block 1 `data-downbeat`; the row's text reads "Bar 3/4", "Main B",
   "Main C" (`data-face="waiting"`), "fill after bar 3"; its `aria-label` is "Beat 3 of 4, bar 3
   of 4. Main B playing, Main C next. The fill lands after bar 3." With `now` advanced one beat
   (577 ms at 104 BPM) block 4 is current. Stopped: no "Bar", every block `later`.
5. Display: the chord's base run is "Am" and its extension run "7"; tones A R, C m3, E 5, G m7
   and "Fingered"; with `transposeKeyboard` 2 and name "Bm7", fingered "Am7", the tones read B,
   D, F#, A and the line "Fingered · played Am7". "Main B" `data-hue="main"`, next "Main C";
   "104 BPM" (and tempo 103.6 → "104"); "Running". Stopped: "Stopped", the section dot hidden.
6. Chord split (a pure function, `splitChord(name)`): each example in Display › Chord gives the
   runs shown, and "N.C." gives one run.
7. Sounds row: R2's cell shows "41 Silk Strings" and the edited mark; R3's shows ⚠ and "off" and
   its tag has `data-hue="d"`; the rack reads "Rack · A1" and "Sunday drive" with the modified
   dot; the R2 sound's `aria-label` is the template's example.
8. Pads: pad 10 `data-face="solid"` (Playing), pad 11 Next (`data-face="waiting"`, numeral
   "NEXT"), pads 3 and 7 Absent; clicking a pad sends its `action`; Pad Bank ▲ is
   `aria-disabled` with its action null, ▼ sends its action. On the Chord page (`pads.page`
   "chord", labels from the mock), every pad has the fallback face and its label as caption.
9. Faders: a 40px upward drag on strip 1 from any point sends its `set` with `volume` 113
   (90 + round(40 × 127 / 223)); strip 2 shows "↕" and the ghost; the Reverb tab sends
   `setFaderLayer {layer: 'reverb'}`, and with that layer strips 1–4 read "Rev …" and have no
   meter element.
10. Meter maths (pure, `app/src/ui/FaderStrip/meter.ts`): `height(0.0724)` is 138,
    `height(0.001)` is 0; `holdPeak` keeps a peak for 1500 ms of `atMs` and then falls 20 dB/s.
11. Lamps: a click on R1's On sends `togglePart {part: 0}`; a 350 ms press sends `setLayer
    {swap, part 0}` and no toggle; Sound, not lit: a click sends `setLayer sound`; a 350 ms
    press sends `setLayer sound` at 350 ms (before release) and `setLayer none` on release; lit,
    a click sends `setLayer none`; latched, a screen pad press sends the pad's action and then
    `setLayer none`; Looper long press sends `looperRec`. (Fake timers drive the 350 ms.)
12. Knobs: ▲ disabled on page 1; ▼ sends `stepKnobPage {delta: 1}`; an 8px upward drag on knob 1
    sends `turnKnob {knob: 0, delta: 2}`; a double-click sends `resetKnob {knob: 0}`; the tempo
    knob's `knobFraction` is 0.267 (pure function); No Assign is `aria-disabled` with an empty
    code line.
13. Transport: the pairs are Reset | Fade and Fill ▲ | Fill ▼; Tempo + held repeats (existing
    `tempoHold` tests); Style tempo sends `resetTempo`; Fill ▲ sends `fillUp`.
14. Track ◀: sends `surface.controls[trackPrev].action`; with `ui.shift` its `shiftAction`; with
    `surface.trackPrev` null it is `aria-disabled`.
15. Every interactive element has a `data-tip` in the catalog (the existing tooltip test).
16. Health slot: part 2 `plugin.status` failed (not missing) → "R3 failed", `data-hue="ending"`,
    a button whose click calls the Channel opener for part 2; `meters.cpu.total` 0.74 →
    "CPU 74%"; calm → "Audio", no button inside.
17. Status line: shows `message.text`; with `error`, the ⚠; a click on its button sends
    `clearMessage`; with `message` null it has no focusable content.
18. Keys: notes 43, 45, 48, 52 have `data-hue="l"`, 76 and 81 `data-hue="r1"`; the split marker
    follows the F#2 key; the `aria-label` is "Keys: split F#2, left hand G A C E, right hand E4
    A4, 61 keys".
19. Light tokens (a text test over `app/src/ui/tokens/light.css`, no rendering): `--bg`, `--ba`,
    `--ba2`, `--bw`, `--bm`, `--bl` are `none` and every `--*-glow-mix` is `0%`.
20. Links (D32): with the interim targets, the rack readout opens the Rack drawer, a sound cell
    opens Library › Sounds for its part, and the Metronome caret is `aria-disabled`.

**Story and screenshot checks** (`npm run shots -- Stage`, real Chrome), for what jsdom can't
see:

- `Pages/Stage` › `Board` (export `Board`, layout `fullscreen`, `parameters.shots = { viewport:
  { width: 1440, height: 900 }, mask: ['[data-shot-mask="when"]'] }`) renders `Stage` with
  `boardState`, `boardNow` and `boardMeterHolds`, unscaled (D1 lives in the app shell), in both
  themes, against `app/src/ui/Stage/crops/Board-dark.png` and `Board-light.png` (copies of
  `docs/design/push/png/Stage-Dark.png` and `Stage-Light.png`, 1440 × 900): at most 0.02 of the
  unmasked pixels differ. This needs the shots.ts item in the gap table.
- The same story covers what the vitest checks can't: glows by hue (dark) and none (light), the
  chord's two weights, meter heights, the tempo arc, the art.
- `Components/ChordReadout` › `LongChord` (`name: "C#m7b5/G#"`): the chord fits its 300px at the
  shrunk size and doesn't wrap; no crop (the board has no long chord), judged by Inspect.
- `Components/StyleLine` › `LongName` (a 60-character style name, queued style "Another Very
  Long Style Name"): the line stays one row inside 814px, the name ends in an ellipsis, the queued
  chip at most 200px; no crop.
- axe finds no violation on any story.

## Decisions

- **D1 · Scaling.** The Stage is laid out at 1440 × 900 and scaled uniformly to fit the window
  (`scale = min(w / 1440, h / 900)`, centred, the rest `--g`). At 1024 × 700 that is 0.71. The
  scaling is the app shell's (a wrapper in `App.svelte`), so the `Stage` component and its story
  are always 1440 × 900. The board is fixed-size and the old `--u` scheme doesn't fit it; a
  responsive pass is a follow-up.
- **D2 · Pages.** The tabs set an app-only `ui.page` (`stage`, `channel`, `effects`,
  `quickRacks`, `multiPads`, `looper`, `harmArp`, `library`, `settings`, and `rack`, which has no
  tab: the rack readout, a rack-target strip name and Alt+O reach it), replacing the drawers. The
  display tabs (Channel … Harm/Arp) replace only the display; Library and Settings are full pages
  with the half band (DECISIONS S9, G1, L1). Each page's spec defines it; until then D32.
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
- **D7 · Meters.** The state has one peak and one RMS per channel, so a strip's two bars are its
  peak (left) and RMS (right), on a −60…0 dBFS scale; the tick is the held peak (1.5 s, then
  falling 20 dB/s), computed from `meters.atMs` in the wiring, never a timer. Peak is never below
  RMS, so the right bar is the lower, as the board draws. A group strip (Style, Multi Pad) takes
  the largest peak and the largest RMS of its channels; Master takes the larger side of
  `meters.master` and of `meters.masterRms` (after the soft clipper). Stereo bars per side are a
  follow-up.
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
- **D18 · Sound latch.** Not lit: a click (released before 350 ms) latches Sound with `setLayer
  sound`; a press held 350 ms sends `setLayer sound` at that moment (pointer still down) and
  `setLayer none` on its pointerup or pointercancel (momentary). Lit (latched or held from the
  hardware): any press sends `setLayer none` on release. While latched, the next screen pad press
  sends the pad's `action` and then `setLayer none`, in that order and without waiting for a
  reply, so a refused pad action (its message on the status line) still releases the latch.
  Hardware pad taps release it once C3 lands. No timer beyond the long-press gesture
  (FIX-DEBATE: no timers).
- **D19 · Chord held.** When `chord.held` exists (C4) the chord dims to `--m` and a small "held"
  (14 `--m`) follows the fingering name; until then the playing face only.
- **D20 · Chord type.** The chord shrinks to fit its 300px, down to 64px, by measuring its
  `scrollWidth` at 128px (Display › Chord). Tones are spelled with flats when the chord's root
  has a flat or is F, otherwise sharps; intervals as listed, with a minor third and a major third
  both present read as #9 and 3.
- **D21 · Rack slot.** Shown only when the loaded rack's button is in the bank on view, until C1.
- **D22 · Sound name.** `sound.name` (the library's current name) before `voiceName`, so a
  rename shows at once; the number only for the user's numbered sounds.
- **D23 · Fader drag.** Relative: the value moves by the pointer's travel from where it was
  pressed (223px = 127), never jumps to the pointer; sends at most once per animation frame and
  always the last value on release; double-click resets to the strip's default (levels 100, pan
  64, sends 0). The board's "absolute" wording meant the scale, not the mode.
- **D24 · Theme switch.** Not on the Stage (the board has none): it moves to Settings › System
  (#532); `ui.theme` and `data-theme` stay as they are.
- **D25 · Keyboard size.** 49 / 61 / 88 isn't on the Stage; it is on the tall pages' key row
  (#501) and Settings › Keyboard (#530). The Stage uses `ui.keyRange` or the Launchkey's.
- **D26 · Category.** "Pop" is the style's library folder (the last folder name), since styles
  carry no genre.
- **D27 · Tempo readout.** Read-only; the tempo is set with Tempo ± , Tap, Style tempo and knob 8.
- **D28 · Section dot.** The dot by "Section" shows only while the band runs, in the section's
  hue.
- **D29 · Glows by hue.** The board draws only Main, where `--bg` and `--bm` happen to be the
  section's glow. A glow in a section's hue (section dot, current beat block, playing section
  text) is built from the hue with `--dot-glow-mix` (70%, 6px) or `--text-glow-mix` (30%, 18px),
  so an Intro or a Fill glows in its own colour. `--bg` stays the green status glow (Launchkey,
  Running, Start / Stop); `--ba` is the compact chord's (Channel, #501); `--ba2` the Stage chord;
  `--bl` the detection line; `--bw` and `--bm` are unused by the kit.
- **D30 · Chord runs.** The chord is the root and its quality (m, dim, aug) at 300 and the rest,
  slash bass included, at 200. This covers every suffix in `TYPE_NAMES`
  (`crates/yahaha-core/src/theory.rs`) and the slash form `Chord::name` writes; "maj" and "Maj"
  are extension, so "maj7" and "mMaj7" both put the 7th's word in the light run, as the board's
  "Am7" puts "7" there.
- **D31 · Tones follow the shown chord.** `keyboard.chordTones` is the chord as fingered, while
  the big name is after Keyboard transpose; the tones are moved by `chord.transposeKeyboard` so
  that the tones, their spelling and their intervals all belong to the chord the player reads.
  The fingered chord shows in the "played" note.
- **D32 · Links to pages not built yet.** One rule (kit › Interaction conventions): open today's
  equivalent if one exists, else draw the control disabled with its tooltip. Interim targets,
  each dropped when its spec lands:

  | Target | Interim |
  |---|---|
  | Channel (#501): tags, strip names, health "R3 failed", Shift-click on a part lamp | today's `ChannelView` in the display's place, `panels/channel/nav.svelte.ts` `show(part)` |
  | Rack page (rack readout, rack-target strip names, Alt+O) | the Rack drawer, `ui.toggleDrawer('rack')` (opened, not toggled closed) |
  | Quick sound list (#514): sound cells | Library › Sounds loading into that part, `ui.openLibrary('sounds', part)` |
  | Effects page and Effects at the master (#519): band sends, master strip name | the Effects drawer, `ui.toggleDrawer('effects')` (opened) |
  | Multi Pads page: Multi Pad strip name | the Multi Pads drawer, `ui.toggleDrawer('multipad')` (opened) |
  | Settings › System (#532): health "Audio off", dropouts, CPU | the Settings drawer on its Audio tab (`panels/settings/nav.svelte.ts`, tab `audio`) |
  | Metronome popover (#509): the ▾ caret | disabled, tooltip `metronome.settings` |
  | Page tabs | kit › App bar, "Tabs before their page exists" |
- **D33 · Fallback pad face.** Pad pages other than Sections draw every pad as a utility pad
  captioned with its state label, faces from `level` and `anim`, until their specs give the
  table (kit › Pads). The labels stay as the state sends them, upper case included.
- **D34 · Style line fit.** The board's line is about 820px of content in 814; only the style
  name shrinks (ellipsis), everything else keeps its size. The queued chip is the 26px waiting
  face at 14px (the style line's text size), capped at 200px. Band sends is one button, as the
  board draws it (one place to open Effects), with no face: a text button.
- **D35 · Hover and cursor.** No hover or pressed look anywhere (the board draws none); pointer
  cursor on enabled controls, default elsewhere, `ns-resize` while dragging a fader or knob.
- **D36 · Keyboard and focus.** Tab order is reading order; the global key handler stays and
  yields arrows to a focused fader or knob; the key strip isn't focusable (kit › Interaction
  conventions).
- **D37 · Shortcuts.** Stage Alt+G and Channel Alt+N: letters from each word that no Alt key
  uses today (S and C are taken by the Browser and Charts). The other tabs keep their Alt
  letters. Alt+C stays Charts (hidden), Alt+O the Rack page.
- **D38 · Track buttons.** ◀ ▶ mirror the Track buttons through `surface.controls` (`action`,
  and `shiftAction` with Shift: Rack −/+ on the Racks pad page), as the parity rule says, rather
  than a hard-coded `stepStyle`; they are also disabled when there is no neighbour.
- **D39 · Screenshot check.** The board story is `Pages/Stage` › `Board`, rendered at the
  native 1440 × 900 with no scaling, masked only on the count row's When item. `shots.ts` gains
  a per-story viewport and element masks as part of building the Stage (gap table); the tool
  isn't changed by this spec.
- **D40 · Fixture moment.** The page takes `now` as a prop and the fixture fixes it with the
  clock anchors, so the beat blocks and the pad flash are the same on every run.
- **D41 · Test hooks.** `data-face` and `data-hue` carry each element's face and hue so vitest
  tests meaning; computed colours, `color-mix`, glows and text measurement are checked by the
  stories' screenshots, and pure functions (`splitChord`, `knobFraction`, `height`, `holdPeak`,
  the chord fit) carry the maths.
- **D42 · Display border.** The board's display has a 1px transparent border; the spec keeps it
  so the content lands on the board's pixels (every display box above includes it).
- **D43 · Section dot space.** The stopped section dot is hidden, not removed, so "Section" and
  the names below don't shift when the band starts.
- **D44 · Formats.** Tempo rounds to the nearest BPM. The category falls back to the metre alone
  when the library isn't loaded or the style has no folder. A queued style is named from the
  library by id, else "next style". Intro and Ending D read IV. A tone column grows past 24px for
  two-character names ("C#", "Eb").
- **D45 · Health tooltip.** The health slot carries one key, `app.health`, whatever it shows; an
  element can carry one `data-tip`, and one body can explain every state of the slot.
- **D46 · Page wiring.** The `Stage` component in `app/src/ui/Stage/` is pure (props in,
  callbacks out). A new `app/src/pages/StageWiring.svelte` connects it to `app.state` and
  `app.send`, keeps `now` (once per animation frame), `receivedMs` and the meter holds, and maps
  each callback to its command or interim target (D32). `App.svelte` mounts it in place of
  today's stage and wraps it in the D1 scaler.

## Follow-ups

- Per-style artwork and key colour (DECISIONS S4), with what generates them.
- A responsive layout for 1024 × 700 to 1920 instead of the uniform scale (D1, DECISIONS S14).
- C4, chord held, needs the engine to know when detection is unsure.
- Stereo meters per part would need per-channel left and right levels in `meters`.
- A face for the Ending's ritardando (`transport.ritardando`).
- Typed tempo entry on the tempo readout, if players ask.
- `scripts/shots.ts`: per-story viewport and masks (D39), built with the Stage.

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

The Stage's root element is a `1440 × 900` box, `box-sizing: border-box`, `padding: 24px`,
`overflow: hidden`, `background: var(--g)`, `color: var(--t)`, `font-family: var(--font-sans)`,
`font-variant-numeric: tabular-nums`, a flex column (`gap: 0`; the rhythm above is each
region's `margin-top`).

### The `Stage` component

`app/src/ui/Stage/Stage.svelte` is pure: props in, callbacks out (D46). Nothing in
`app/src/ui` reads `app.state` (the live `AppState`, on the `app` store in
`app/src/lib/store.svelte.ts`), the `ui` store (same file), `tips` or `app.send`; the wiring
passes everything below. Every fixture value in this spec gives every required field of its
TS type (`app/src/lib/api/types.ts`); a field the spec doesn't name keeps the dev mock's
initial value.

| Prop | Type | Meaning |
|---|---|---|
| `state` | `AppState` | the engine state (every readout and control reads it) |
| `now` | `number` | the session clock, ms, once per animation frame (kit › Count row; D40) |
| `receivedMs` | `number` | the `now` at which `state` arrived |
| `meters` | `Meters \| null` | the latest meters frame (`session.meters()`, not part of `AppState`); null draws every meter empty |
| `holds` | `number[]` | nine held peaks, strips 1–9, linear (kit › FaderStrip, `holdPeak`: the wiring keeps the `HoldState`s and passes their `peak`s); `[]` or a missing entry = 0 |
| `library` | `LibraryList` | `app.library` (never null: it starts as `revision` 0 with empty `entries`, so "not loaded" is simply "entry not found"; the category and the queued chip fall back, D44) |
| `page` | `Page` | the chosen page tab (D52): `'stage' \| 'channel' \| 'effects' \| 'quickRacks' \| 'multiPads' \| 'looper' \| 'harmArp' \| 'library' \| 'settings'` |
| `shift` | `boolean` | the Shift layer (`ui.shift`): Track ◀ ▶ and Pad Bank send their `shiftAction`; a part lamp's click opens Channel |
| `keyRange` | `49 \| 61 \| 88` | the key strip's range (the wiring resolves `ui.keyRange` or the Launchkey's with `rangeFor()`, kit › Key strip) |
| `help` | `boolean` | help mode (`tips.help`): the "?" button's `aria-pressed` and chosen face (D50) |
| `dropouts` | `number` | audio dropouts in the last 30 s, 0–3 (`DropoutWatch.recent`, D54), for the health slot |
| `display` | `Snippet \| undefined` | when given, rendered in the display's box instead of the Display component (the interim Channel, D53); the display's own content is not drawn |
| `onsend` | `(cmd: AppCmd) => void` | every command (the kit's tables name them); the wiring calls `app.send` |
| `onopen` | `(target: OpenTarget) => void` | every link to another page or popover (D32): `'browser'`, `'rack'`, `'effects'`, `'multiPads'`, `'settingsAudio'`, `{ channel: part }`, `{ sounds: part }`, `'metronome'` |
| `onpage` | `(page: Page) => void` | a page tab clicked (D52) |
| `onhelp` | `() => void` | the "?" button: toggles help mode (`tips.toggleHelp()`) |

`Page` and `OpenTarget` are exported from `app/src/ui/Stage/types.ts`. Every child component
takes the slice of these it needs (a `FaderStrip` takes one strip's value, hue, meter and
hold; a `Pad` one pad's state and the LED phase) and the same callbacks; a child never takes
`state` whole. What a component under `app/src/ui` may import from `app/src/lib`: types from
`lib/api/types` and `lib/api/sound-library`; `use:tip` from `lib/tooltip/tip.svelte` (it marks
`data-tip` and reports hover and focus to the help footer, which is its job); `TempoHold` from
`lib/tempoHold`; `rangeFor`, `RANGES`, `layout`, `boundary`, `detectionArea`, `noteName`,
`pcName` from `panels/keystrip/keyboard.ts` and `brightness`, `DIM` from `lib/leds.ts` (pure
helpers). Never `store.svelte` (`app`, `ui`), `nav`, `shortcuts` or a session. Notation: in
this spec and the kit, `surface.controls[trackPrev]` means the `SurfaceControl` whose `id` is
`trackPrev` (`controls` is an array; find the entry by `id`). Gesture
timers are allowed (the `longpress` action's 350 ms and `TempoHold`'s repeat while a button
is held): they measure a press, not motion (axiom 10 is about drawing).

The wiring (`app/src/pages/StageWiring.svelte`) owns:

- the frame loop: once per animation frame `now = performance.now()` (local ms; the spec
  calls it the session clock because it is only ever differenced against `receivedMs`),
  passed as `now`. `receivedMs` is set to `performance.now()` in an `$effect` that reads
  `app.state` (so it runs once per new state; `BeatClock`'s own `receivedMs` is private and
  stays for the old panels; the Stage does not read `clock`);
- meters: it polls `app.meters()` from the frame loop at most every 33 ms (about 30 Hz),
  passes the frame as `meters` and folds it into `HoldState[]` with `holdPeak(prev, peak,
  atMs)` per strip (the group strips fold their largest peak), passing the `peak`s as
  `holds`. `session.ts` wants one reader (a call returns the peaks since the last), and the
  interim `ChannelView` polls `app.meters()` every 100 ms of its own while shown (D60);
- `library`: `app.library` from the store (it already re-fetches on
  `state.library.revision`); nothing is fetched twice;
- `dropouts`: `dropouts.observe(state.io.synth, Date.now())` on every state,
  `dropouts.recent(Date.now())` per frame (D54);
- `page`: `chosenPage({ view: ui.view, libraryTab: ui.libraryTab, drawer, channelOpen:
  channelNav.open })` where `drawer` is the first of `effects`, `multipad`, `looper`,
  `harmony`, `settings`, `rack`, `charts` whose `ui` flag is true, else null (D52);
- `shift` = `ui.shift`, `keyRange` = `rangeFor(ui.keyRange, state.io.inputs)`
  (`keyboard.ts`), `help` = `tips.help`, `display` per D53;
- the callbacks: `onsend` → `app.send`; `onpage` → that page's `NAV` entry's `toggle()`;
  `onhelp` → `tips.toggleHelp()`; `onopen` → the D32 table.

The `Pages/Stage` story gives explicit `argTypes`: `state`, `meters`, `library`, `holds` as
`object` controls, `page` and `keyRange` as `select`, the four callbacks as actions (`fn()`),
and no argType for `display` (`app/src/ui/stories.test.ts` treats a `Snippet` prop as
content and fails any disabled or hidden control, so `control: false` is not allowed).

### The app shell around it (interim, D53)

Until the other page specs land, `App.svelte` is a column filling the window (`.app`: `padding:
0; gap: 0`, dropping today's `0.5rem 16px 0.6rem` and `0.6rem` gap, and its `--u` sizing):
the **scaler** (flex 1, `--g` ground, D1) and, under it, today's `HelpFooter`
(`lib/tooltip/HelpFooter.svelte`, unchanged: full width, its own height `--help-footer-h`
(3rem, 48px), taller in help mode (`--help-footer-h-help`), its 8px radius and the old
shell's tokens). `app.css`'s `--help-footer-space` (what the drawers keep clear at the
bottom) becomes `var(--help-footer-h)` / `var(--help-footer-h-help)` alone, since the gap and
padding it added are gone (build-time item). `scale = min(w / 1440, (h − footer) / 900)`,
where `footer` is the footer's rendered height; at 1024 × 700 with the 48px footer that is
0.71. Inside the scaler:

- `ui.view === 'stage'`: `StageWiring` (the `Stage`, scaled and centred).
- `ui.view === 'library'`: today's `Library` page (`panels/library/Library.svelte`), unscaled,
  in the old tokens, filling the scaler, with its own tabs, Loads-into and "Back to stage";
  no kit app bar (the page replaces the whole Stage, as it does today; Esc and Alt+B return;
  the Library spec brings the kit app bar).
- The drawers (`RackPanel`, `Effects`, `Looper`, `MultiPad`, `Settings`, `Charts`,
  `Harmony`: `lib/ui/Overlay.svelte`, `position: fixed`, right-aligned, `top: 3.9rem`,
  ending above the footer), the `Browser` modal, `SoundPicker` and the floating `Tooltip`
  stay exactly as they are: unscaled, in the old tokens, over the scaled Stage.
- The interim Channel (D32): while `channelNav.open`, the wiring passes a `display` snippet
  rendering today's `ChannelView` (`part = ui.selectedPart`, `onpart = channelNav.show`,
  `onclose = channelNav.close`) in the display's box (`24,112 1392×300`, so it is the one
  interim thing drawn inside the scale; `overflow: auto`; its wrapper restores the old font
  and ink per D48, and the old tokens reach it because both sets live on `:root`). Esc
  closes it (`App.svelte`'s `onKey` keeps
  `channelNav.escape()`), as do its ×, the Channel tab (D52) and the Stage tab.
- Gone from the shell: `Header`, the quick-nav strip, `LeadSheet`, `Launchkey`, `MixerRow`,
  `KnobRackPanel`, the old `KeyStrip`, the status footer and `DropoutNotice` (the health slot
  shows dropouts, D54; the wiring calls `dropouts.observe`). Their files stay on disk until
  the pages that replace them land.

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
| Category · metre | 14 / 400 `--m`, line-height 18, 4px extra left margin; not a control | the last `/`-segment of the `folder` of the library entry whose `id` is `style.id` (from the `library` prop), " · ", then `style.timeSignature` as "4/4" (`[4, 4]` → "4/4"; it is never null); empty folder, entry not found or library not loaded: the metre alone (D44) | — | `display.timesig` | — |
| One Touch | `margin-left: auto`, a `role="group"`, gap 4: "One Touch" 14 `--m` (4px right margin), then four 32 × 32 buttons "1"–"4", 14px | `ots.applied` (1-based, 0 = none): that button is the chosen face (`aria-pressed="true"`); buttons past `ots.settings.length` disabled | `recallOts { index }` (0-based) at once; D17 | `ots.1` … `ots.4` | Racks pad page, bottom row pads 1–4 (D16) |
| Band sends | one text button (D34), 16px left margin, 24 tall, padding 0, a flex row with items on the baseline and gap 10 between its four items ("Band", "Reverb 40", "Chorus 12", "Delay 0"), 14 / 400: "Band" in `--m`, then each pair as one span: the word and a normal space in `--t2`, then the value 18 / 300 `--a` with no extra gap | `home.bandSends`: the entry whose `block` is `reverb` → Reverb, `chorus` → Chorus, `variation` → Delay (by `block`, never by index). A block with no entry (the array is empty before the engine sends it) reads "–" (en dash) in `--m` in the value's place; the three words always show (D58) | opens the Effects page (D32) | `display.band_sends` (new) | — |

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
  "N.C." (the cancel chord) is all base. No chord (`chord.name` null): "—" in `--d`, no shadow,
  `data-contrast="dim"` (D47).
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
  C Db D Eb E F Gb G Ab A Bb B). Then, **in the same row** after the last tone column (the
  row's gap 16 plus a 4px left margin: 20px after it), at the row's bottom like the columns,
  `chord.fingeringName` 14 `--m`, line-height 16, no wrap ("Fingered"); it is not below the
  row (the column's 16 + 4 + 104 + 6 + 32 = 162 leaves no room). When `chord.fingered` differs
  from `chord.name` (Keyboard transpose), it reads "{fingeringName} · played {fingered}"
  ("Fingered · played Gm7"). No chord: no tone columns, the fingering name alone at the
  row's left.
- Tooltip `display.chord` on the column. The column is not a control; it is `role="img"`
  (a bare `aria-label` on a `div` fails axe's `aria-prohibited-attr`) with `aria-label`
  "Chord {name}: {tones as note names}, {fingeringName}" ("Chord Am7: A C E G, Fingered"; no
  chord: "No chord, {fingeringName}"); its children are `aria-hidden`.

### Section and tempo

Right column, 490 wide.

- **Label**: a flex row, 16 tall, items centred, gap 8: first a 6px round dot in the
  playing section's hue with its glow (D29: `0 0 6px`, `--dot-glow-mix`), then "Section" 14
  `--m`. The dot shows only while running (D28); stopped, it is `visibility: hidden` and keeps
  its space (D43).
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
`--line` top edge (box-sizing border-box, so the content is 43 tall). Nothing in the row sets
a vertical offset by hand: each cell centres its content in the 43px (`align-items: center` in
the part cells, `justify-content: center` in the rack column), so the text lands where the board
puts it.

- **Rack** (a text button, the 200 cell, a flex column, `justify-content: center`,
  `align-items: flex-start`, gap 2, `text-align: left`, no wrap): "Rack · A1" 12 / 400 `--m`,
  line-height 16, with the slot in `--t`, over the name 14 / 400, line-height 16, `--t` (the
  name line is a flex row, items centred, gap 6, then a 5px round `--t` dot when
  `liveRack.modified`). The two lines plus the gap are 34px, centred in the 43 (4.5px above
  and below). Name = `liveRack.name` (it reads "Recovered: Sunday drive" itself when it is
  one); ellipsis (`min-width: 0`). Slot = the bank letter (`A` + `quickRacks.bank`) and button
  number (index + 1) of the `quickRacks.buttons` entry with `loaded` true; no such button:
  "Rack" alone (D21). Click: opens the Rack page (D2, D32). Tooltip `stage.rack_name`.
  `aria-label` "Rack: {name}{, modified}{, on Quick Rack A1}. Opens the Rack page".
- **R1, R2, R3, L** (one cell each, part order 0–3): a flex row, items centred, gap 4, holding
  two text buttons.
  - **Tag** "R1" / "R2" / "R3" / "L", a 32 × 43 button, `flex: none`, padding 0,
    `text-align: left`, its text vertically centred (a button's default), 14 / 500 in the part
    hue (`--d` with `data-contrast="dim"` when the part doesn't sound,
    `keyboardParts[i].sounding` false; D47). Click: opens Channel for the part (D32). Tooltip
    `mixer.strip.select`. `aria-label` "{part name}: open Channel".
  - **Sound**, the rest of the cell (`flex: 1`, `min-width: 0`, 43 tall, padding 0), a flex
    row, items centred, gap 8, no wrap, `text-align: left`: the sound number (12 / 400 `--m`,
    `flex: none`; `--d` with `data-contrast="dim"` when not sounding), the name (14 / 400 `--t`,
    `min-width: 0`, ellipsis; `--m` when not sounding), then the marks (kit › FaderStrip, name
    button marks; each `flex: none`): a 5px `--t` dot when `soundEdited`; a 12px `--warn` ⚠
    when `plugin.missing`; a 12px `--ending` ✕ when `plugin.status` is `failed` and not
    missing; "off" 12 / 400 `--m` when `on` is false and the part doesn't sound. Line-heights
    are `normal`; the row's centring places them. Number = `soundLibrary.patches[].number` of
    the patch whose `id` is the part's `sound.id` without its `saved:` prefix (`sound.id` is
    `saved:<patch id>`; a `sf:…` or `au:…` id, or a missing `sound`, gives no number). Name =
    `sound.name`, else `voiceName` (D22). Left under Manual Bass
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
parts (#507). **Until #507 lands, the Style page has a fallback** like the pads' (D33): strips
1–8 are `FaderStrip`s from `surface.faders[0..7]` named by `label` as the state sends it
("RHYTHM 1"…, upper case; no wrap, ellipsis), in `--a`, with `value`, `set` and `waiting` as
on Panel, no meter and no marks; in the Pan layer the engine sends them unused (`set` null:
the Unused face); the slider's tooltip by layer: `mixer.style.volume`, `mixer.style.reverb`,
`mixer.style.chorus`, `mixer.style.variation` (Delay); `aria-valuetext` "{label} {value}";
strip 9 is Master as on Panel; the name buttons are not controls (plain text); the lamp row's
buttons 1–8 are LampButtons labelled with the fader button's `label` from the
`surface.controls` entries `faderButton1` … `faderButton8` as the state sends it (no wrap,
ellipsis), `name` the same label, `on` when that entry's `level` is `bright`, each click
sending its `action` (disabled when null); button 6 is Sound on both pages (app-api.md ›
surface) and keeps the Panel row's Sound behaviour and `launchkey.sound`; the other seven
carry `mixer.style.mute`; the header's "Faders" and the layer tabs are unchanged. The pads
show the hardware's page
(`pads.page`); Sections is drawn here, the other
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
| Swap held (a part) | that part's lamp reads "Swap"; the knob page's accent block reads `knobs.pageName` ("Swap R1"; D51); knobs show the part's mix |
| Sound held or latched | Sound lamp on; pads are the Racks page (fallback face until #507) |
| Pads on another page | fallback face (D33) until that page's spec lands |
| Help mode | spec #508 |
| No Launchkey | app bar status hollow dot, `--d`; everything else works |
| No synth (`io.synth` null) | health slot "Audio off"; master strip unused; every meter empty (`meters.channels` is empty, `master` and `masterRms` 0) |
| Audio trouble, failed plugin | kit › App bar, health slot; the failed part's strip and sound cell show ✕ |
| Dropouts | health slot "1 dropout" / "2 dropouts" / "3 dropouts · buffer 256?" for 30 s after the last one (D54) |
| Interim Channel open (`channelNav.open`) | the display's box shows today's `ChannelView` (D53); the Channel tab is chosen (D52) |
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
shots: `app/src/ui/Stage/Stage.fixtures.ts` exports one value per `Stage` prop (The `Stage`
component, above):

| Export | Prop | What it is |
|---|---|---|
| `boardState` | `state` | a full `AppState`: the dev mock's initial state (`app/src/lib/api/mock.ts`) with the fields below set |
| `boardNow` | `now` | 10000 |
| `boardReceivedMs` | `receivedMs` | 10000 |
| `boardMeters` | `meters` | the `Meters` frame below |
| `boardMeterHolds` | `holds` | the nine held peaks below |
| `boardLibrary` | `library` | a `LibraryList`: `revision` 1, `entries` the two below, `voices`, `harmonyTypes`, `arpPatterns` `[]` |
| `boardUi` | the rest | `{ page: 'stage', shift: false, keyRange: 61, help: false, dropouts: 0, display: undefined }` |

The story spreads them: `<Stage {...boardUi} state={boardState} now={boardNow} … />`, with
`fn()` actions for the four callbacks.

- **Moment:** `boardNow = 10000` (the session clock, ms), and the state counts as received at
  that moment (`receivedMs = 10000`), so `t = clock.atMs = 10000`.
- `surface.clock`: atMs 10000, running true, tempo 104, beatsPerBar 4, bar 3, beat 3, phase 0.25,
  sectionAnchorMs 10000, sectionAnchorBeats 10.25 (bar 3 beat 3: 2 bars and 2 beats past, a
  quarter into the beat), ledAnchorMs 10000, ledAnchorBeats 0.25. So the current beat block is 3
  (`floor(10.25) mod 4 = 2`, 0-based), and the LED clock reads 0.25: `frac < 0.5`, flashing pads
  at full (kit › Pad).
- `style`: id 1, name "Sunday Drive Pop", timeSignature [4, 4]. `preview.queued` null.
  `boardLibrary.entries`: `{ id: 1, name: "Sunday Drive Pop", folder: "Pop", path:
  "Pop/Sunday Drive Pop.sty", status: "ok", error: null, tempo: 104, timeSignature: [4, 4],
  sections: "Main ABCD · Intro ABC · Ending ABC · Fill ABCD · Break", format: "SFF2" }` and
  `{ id: 2, name: "Another Very Long Style Name", folder: "Pop/Ballad", … }` (the same other
  fields; the `StyleLine` › `LongName` story queues it: `preview.queued` 2).
- `transport`: running true, section "Main B", queued "Fill In CC", landing "Main C" (Fill ▲
  from Main B queues Main C's fill and lands on Main C, as the engine does), bar 3, beat 3,
  sectionBars 4, beatsPerBar 4, tempo 104, acmp true, unison false, syncStart false, main 1
  (the Main playing, B, 0-based: `transport.main` is "the Main playing, or returned to after
  a fill"), pendingIntro null, fade "off", ritardando false. `style.sections`: `["Intro A",
  "Intro B", "Main A", "Main B", "Main C", "Main D", "Fill In AA", "Fill In BB", "Fill In CC",
  "Fill In DD", "Fill In BA", "Ending A", "Ending B"]` (no Intro C or Ending C: pads 3 and 7
  are Absent), and the library entry's `sections` text is "Main ABCD · Intro AB · Ending AB ·
  Fill ABCD · Break".
- `styleSettings.mainTiming` "nextBar", `introEndingTiming` "nextBar".
- `chord`: name "Am7", fingered "Am7", fingering "fingered", fingeringName "Fingered", upper
  false, manualBass false, manualBassActive false, transposeKeyboard 0, leftHold false (the
  mock's initial chord is Fingered On Bass with Manual Bass on; the fixture overrides those
  five fields together); `keyboard.chordTones` [9, 0, 4, 7].
- `ots`: `settings` four entries `{ name: "OTS 1" … "OTS 4", parts: [] }` (`parts` unread by
  the Stage), applied 2, link false, linkTiming "mainChange", racks `[]`, racksReadOnly
  false. `home.bandSends`: `[{ block: "reverb", name: "Reverb",
  effectName: "Hall 1", level: 40 }, { block: "chorus", name: "Chorus", effectName: "Chorus 1",
  level: 12 }, { block: "variation", name: "Delay", effectName: "Delay LCR", level: 0 }]`.
- `liveRack`: name "Sunday drive", modified true. `quickRacks`: bank 0, button 0 `loaded` true,
  the rest false.
- `keyboardParts` (R1, R2, R3, L): on true / true / false / true; sounding true / true / false /
  true; playsBass false; channels 1, 3, 4, 2; `sound` `{ id: "saved:p1", name: "Stage Grand" }`,
  `{ id: "saved:p41", name: "Silk Strings" }` (`soundEdited` true), `{ id: "saved:p57", name:
  "Brass Section" }` (`plugin` `{ id: "aumu Smp7 Fake", name: "Sampler Deluxe",
  manufacturer: "Fake Instruments", status: "failed", stage: null, error: "Not installed",
  outOfProcess: false, inProcessFallback: false, cpu: 0, overruns: 0, recentOverruns: 0,
  editor: false, missing: true }`, every required field of `PartPlugin`; the fake plugin
  AGENTS.md names, with the id `app/src/lib/api/mock-plugins.ts` gives it), `{ id:
  "saved:p41", name: "Silk Strings"
  }`; `voiceName` the same names. `soundLibrary.patches` is **replaced** by three `PatchInfo`
  literals (ids `p1`, `p41`, `p57`; `name` the sound names; `category` "piano", "strings",
  "brass"; `tags` `[]`; `favourite` false; `source` `{ kind: "soundFont", file:
  "GeneralUser.sf2", bank: 0, program: 0 / 48 / 61 }`; `available` true; `note` null; `number`
  1, 41, 57). (R3 is missing, so it shows ⚠, not ✕, and the health slot's "failed" row skips
  it: calm.)
- `mixer`: faderPage "panel", faderLayer "volume", styleVolume 100, multiPadVolume 90, master
  100. `surface.layer` none.
- `surface.faders` (9): values 90, 72, 64, 80, 100, 90, null, null, 100; labels as the engine
  and the mock send them, "RIGHT 1", "RIGHT 2", "RIGHT 3", "LEFT", "STYLE", "M.PAD", "", "",
  "MASTER" (the strips' display names are the kit's own; a Panel fader 1–4 is a rack target
  only when its `label` differs from the engine's `PART_LABELS` entry for it, kit › FaderStrip,
  Rack target); `set`
  null for 7 and 8, the matching set command for the rest (`setPartVolume` 0–3,
  `setStyleVolume`, `setMultiPadVolume`, `setMasterVolume`, each with `volume: 0`); fader 2
  `waiting` true at `position` 50, the rest waiting false, `position` null.
- `boardMeters` (atMs 10000; every `channels` entry has `cpu` 0.02 and `cpuPeak` 0.05; `clips`
  0; `cpu` `{ total: 0.2, peak: 0.3, bufferUs: 5805 }`), as linear amplitudes; the bar heights
  they give in px are in brackets: channel 1 (R1) peak 0.0724 [138], rms 0.0537 [129]; channel
  3 (R2) 0.0224 [100], 0.0180 [93]; channel 4 (R3) 0, 0; channel 2 (L) 0.0316 [112], 0.0248
  [104]; Style, channel 9 0.1259 [156], 0.0897 [145], channels 10–16 0; Multi Pads, channels
  5–8 0; `master` [0.1445, 0.1445] [161], `masterRms` [0.1020, 0.1020] [149].
- `boardMeterHolds` (the nine held peaks, strips 1–9, linear, the `peak` of each `HoldState`
  the wiring would hold; tick bottoms in px, `5 + height(hold)`): 0.1259 [161], 0.0447
  [128], 0, 0.0631 [139], 0.2188 [179], 0.0023 [32], 0, 0, 0.2512 [183].
- Lamps: `harmonyArp.on` false, `chord.leftHold` false, `looper.mode` "off", `metronome.on`
  false.
- `knobs`: page "style", pageName "Style", pageNumber 1, pageCount 6; knobs 1–8 (function,
  value, level, short, name): dynamics "127" 127 "DynCtrl" "Dynamics Control"; retriggerRate
  "1/8" 51 "RtgRate" "Retrigger Rate"; retriggerOnOff "Off" 0 "RtgOnOff" "Retrigger On/Off";
  trackMuteA "Off" 0 "StyMuteA" "Style Track Mute A"; trackMuteB "Off" 0 "StyMuteB" "Style
  Track Mute B"; swing "0%" 0 "Swing" "Swing"; none "" null "---" "No Assign"; tempo "104 BPM"
  null "Tempo" "Tempo". (Tempo arc `knobFraction(null, 104)` = `(104 − 40) / 240` = 0.267,
  72°; kit › Knob.)
- `pads`: page "sections", pageName "Sections", pageNumber 1, pageCount 5, connected true. Pads
  1–16 (level, anim): 1 dim, 2 dim, 3 off, 4 dim, 5 dim, 6 dim, 7 off, 8 dim, 9 dim, 10 bright
  solid, 11 bright flash, 12 dim, 13 dim, 14 dim, 15 dim, 16 bright solid; every anim not named
  is solid; actions as the dev mock's Sections page. `surface.controls`: padBankUp `action` and
  `shiftAction` null; padBankDown set; trackPrev `action` `stepStyle { delta: -1 }`,
  `shiftAction` `stepQuickRack { delta: -1 }`, trackNext the same with `delta: 1`, with
  `surface.trackPrev` and `trackNext` not null.
- `keyboard`: held 43, 45, 48, 52 (zone left, parts [3]) and 76, 81 (zone right, parts [0]);
  leftSplit 54 (F#2); detection [0, 54]. `boardUi.keyRange` 61.
- `io.synth` set (bufferFrames 256, dropouts 0); `message` null; `boardUi.help` false,
  `boardUi.dropouts` 0.

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
| 0 | tokens | — | — | yes; add the kit's new tokens; loaded in the app per D48 | `:root` lines 60, 65 / 36, 41; art 124–125 / 100–101 | kit › Tokens |
| 1 | `longpress` (Svelte action, `app/src/ui/actions/`) | primitive | — | no | — | kit › Interaction conventions |
| 1b | `sectionName(name)`, `splitChord(name)`, `knobFraction(level, tempo)`, `chordTones(pcs, transpose, root)` (pure functions, `app/src/ui/Stage/format.ts`) | primitive | — | no | — | kit › Section names; Display › Chord; kit › Knob |
| 2 | LampButton | primitive | longpress | yes; change per D49: `aria-pressed` follows `on` alone (no local flip), `data-face`, `data-hue`; add `tip: TipKey` (it applies `use:tip` itself, like every control primitive below), `join: 'start'` (radius `4px 0 0 4px`), `waiting: 'lamp' \| 'rec'` (the waiting face in that hue), `onlongpress`, `onlongrelease` | 97, 116, 277–284 / 73, 92, 253–260 | kit › Faces, Lamp row |
| 3 | Button | primitive | longpress | no | 117–118, 131, 133, 299–300, 334–335, 366–386 / −24 | kit › Faces: `face: 'off' \| 'on' \| 'chosen' \| 'waiting'` (default off) with `hue` for waiting (a token name, default `t2`), `disabled`, `pressed?: boolean` (rendered as `aria-pressed="true"` / `"false"` when given, omitted when undefined); variants `icon` 32 × 32, `md` padding 0 14, `band` 88 × 32 left-aligned padding 0 8, `pair` 41 × 32, `cell` 32 tall filling its grid cell, 13px (the master button), `caret` 20 × 32, 10px glyph in `--m`, radius `0 4px 4px 0` (the Metronome caret) (D50) |
| 4 | ChosenTabs | primitive | — | no | 74–84, 226–237 / 50–60, 202–213 | `kind: 'page'` (a `<nav aria-label>` of buttons; the chosen one `aria-current="page"`; `size page`: 36 tall, 24px block, kit › App bar) or `kind: 'tab'` (a `role="tablist"` with `aria-label`, buttons `role="tab"`, the chosen one `aria-selected="true"`, the others `"false"`; `size header`: 35, 22px, kit › Faders header); props `items: { key, label, tip }[]`, `chosen: key`, `onchoose(key)` |
| 5 | WaitingChip | primitive | — | no | 110, 166 / 86, 142 | sizes `count` (26, 18px), `line` (26, 14px), `display` (48, 36px) |
| 6 | AccentBlock | primitive | — | no | 132, 294 / 108, 270 | kit › Faces (accent block); as a button (style name) or a span (knob page) |
| 7 | StatusDot | primitive | — | no | 89, 161, 171 / 65, 137, 147 | round, `size` 6 (default; the Launchkey, Running and section dots) or 5 (the rack's modified dot, the edited mark): solid in a hue with an optional glow, or hollow 1px ring |
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

**The base `Button`** (row 3), which every faced button on the Stage is or wraps: props
`label: string` (the text; a snippet child for a glyph with a unit, like Fill ▲), `name?:
string` (the accessible name when the label isn't enough, as `aria-label`; default the
label), `tip: TipKey`, `face: 'off' | 'on' | 'chosen' | 'waiting'` (default `'off'`), `hue:
string` (a token name for the waiting face's border and label, default `'t2'`), `variant:
'icon' | 'md' | 'band' | 'pair' | 'cell' | 'caret'` (default `'md'`), `disabled: boolean`
(default false; `aria-disabled`, stays focusable, label `--d` with `data-contrast="dim"`),
`pressed?: boolean` (`aria-pressed`), `current?: boolean` (`aria-current="page"`, for the
page tabs), `join?: 'start' | 'end'` (the radius it keeps in a split), `shift: boolean`
(default false), `onclick(e)`, `onlongpress?()`, `onlongrelease?()`, `onshiftclick?(e)`
(fired instead of `onclick` while `shift` is true), `onpointerdown?(e)` / `onpointerup?(e)`
(Tempo ±). It renders one `<button type="button">` with `data-face` (the face, or
`disabled`), `data-hue` when `hue` is given, `use:tip`, and `use:longpress` when
`onlongpress` is given. Every row below with a face is a `Button` or a `LampButton`; nothing
else renders a `<button>` except the text buttons.

**Props and callbacks.** Every control primitive (LampButton, Button, ChosenTabs, FaderStrip,
Knob, Pad, HealthSlot, StatusLine, the text buttons) takes `tip: TipKey` and applies
`use:tip={tip}` to its own element (a parent can't put an action on a child component), a
`name` or label for its accessible name, `disabled` where the kit says it can be, and reports
with one callback per gesture the kit names for it (`onclick`, `onchange(value)`,
`onlongpress`, `onlongrelease`, `onshiftclick`); readouts take their values as props. The
page's full list is in The `Stage` component; each child's props are the state fields its
section reads (passed as named props, never `state`), the `now` / `receivedMs` pair where it
draws motion, and the callbacks its section's tables send through; each SPEC.md lists them,
and its stories' args come from the board fixture's values for those fields. Components take
props and call callbacks; none reads `app.state` or sends. The page wiring in
the app (`app/src/pages/StageWiring.svelte`, outside `app/src/ui`) reads `app.state`, keeps
`now`, `receivedMs` and the meter holds, passes them down, and sends commands. Story titles
follow the Kind column: `Primitives/<Name>` for a primitive, `Components/<Name>` for a complex
one, `Pages/Stage` for the page (axiom 11). Sizes: the px values in this spec and the kit are
each component's Visual rules (its SPEC.md lists the ones it uses); only the named tokens
(kit › Tokens, scale) are tokens, and a literal px that the spec gives is not an andon pull.

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
| Footer | status footer, `lib/tooltip/HelpFooter.svelte`, `lib/DropoutNotice.svelte` | The status footer goes (the status line, D15). `HelpFooter` stays under the scaler until #508 (D53). `DropoutNotice` is unmounted: dropouts show in the health slot (D54); `lib/dropouts.svelte.ts` gains `recent(now)` (**build-time item**, the Stage lane) |
| Controls | `lib/ui/Fader.svelte`, `Knob.svelte`, `HwButton.svelte`, `Toggle.svelte` (old tokens `--accent`, `--ink`) | The `app/src/ui` library (Components above): LampButton exists (#499); the rest are new |
| Scaling | `App.svelte` scales rows by `--u` (1024 × 700 to 1920) | D1, in the app shell; the shell's interim layout is in The app shell around it (D53) |
| Tokens and fonts | `app.css` defines `--bg` (a colour), `--line`, `--line-strong`, `--key-white`, `--key-black` on `:root` and loads Barlow in `main.ts`; `ui/tokens/index.css` is loaded only by Storybook | D48: `main.ts` imports the kit tokens and fonts after `app.css`; `app.css` renames its clashing names (`--bg` → `--room`, `--line` → `--hairline`, `--line-strong` → `--hairline-strong`, `--key-white` → `--key-ivory`, `--key-black` → `--key-ebony`) and their users follow (**build-time item**) |
| Shortcuts | `lib/nav.ts`, `lib/keys.ts`: Alt letters `bsropemlchyt` | Add Alt+G (Stage) and Alt+N (Channel) (D37): two `NAV` entries (D52) and `gn` in `keys.ts`'s letters |
| Shell tests | `App.test.ts` (the five rows, the hand surface tip, the mixer details); `help/coverage.test.ts` (every focusable or clickable element in the app, in each of its `STATES`, carries a `data-tip` that is in the catalog; it checks nothing about unused keys) | `App.test.ts` is rewritten for the scaler shell (Stage mounted, Library in its place, the footer below, D53). `coverage.test.ts` keeps its drawer, Library and prompt states, drops the states that clicked controls the shell removes (`fx.master_edit` under `ui.mixer`, the Quick Racks bar, the chart lane), and gains the Stage's states (the Style fader page, a send layer, a message on the status line); keys that only the removed panels used (`stage.hand_surface`, `stage.fader_badge`, `nav.mixer`, `nav.charts`, `nav.styles`, `nav.rack`…) stay in the catalog, unused, until their pages land (**build-time item**) |
| Wiring test | — | `app/src/pages/StageWiring.test.ts` mounts `StageWiring` with a `MockSession` (as `App.test.ts` does) for Check 20 and the `chosenPage()` cases |
| Screenshot tool | `app/scripts/shots.ts`: one 1000 × 600 viewport, the story root's box as the shot, no masks, axe with every rule on every element | **Build-time item** (the Stage lane, before its screenshot check can pass; D39): a story may set `parameters.shots = { viewport: { width, height }, mask: [<selector>…] }`; the page uses that viewport, and the diff ignores each masked element's box (taken from the rendered story, applied to both images, and left out of the score's pixel count). Axe: before `axe.run`, `axe.configure({ rules: [{ id: 'color-contrast', selector: '*:not([data-contrast="dim"])' }] })` (D47); every other rule runs on every element |

## Contract changes needed

C5 blocks the build: without its keys `TipKey` doesn't type-check (`npm run check` fails) and the
tooltip catalog test (Check 15) fails. It lands first, as its own small contract PR. C6 blocks
only the screenshot check's axe pass. C1–C4 don't block; each says what the screen does until
it lands.

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
   `display.band_sends`; `view.stage` gains Alt+G and a body for the new Stage (the display, the band and the keys;
   not the lead sheet); rewrite the `nav.*` bodies from "opens the drawer" to "shows the
   page"; `launchkey.fader_sound` ("opens the quick sound list", not Library › Sounds);
   `ots.1`–`ots.4` (applies at once, Launchkey Racks page); `mixer.strip.select` ("opens
   Channel for this part", on the tags and the strip names); `looper.on_off` mentions the long
   press (Loop rec) and `mixer.page` the Shift-click (next layer), since an element carries
   one key (D45).
6. **C6 · Hue tokens that fail AA as text** (`app/src/ui/tokens/palette.css`, `dark.css`,
   `light.css`, `tokens/contrast.test.ts`), **lands before the screenshot check** (axe fails
   on the board fixture without it; the build and the vitest checks don't need it). The kit
   draws hues as text on `--g` and `--btn` (section captions and names, part values and
   names, the legend, "Running", ⚠) and `--solid-ink` on hue fills (playing pads, held keys).
   Measured from the palette, these pairs fall under 4.5:1: dark `--ending` on `--btn` (4.27)
   and `--brk` on `--btn` (3.86); light `--r2` (4.05), `--r3`/`--warn` (3.21), `--l` (3.14),
   `--intro` (3.41), `--main`/`--ok` (3.88) and `--fill` (3.96) on `--btn`, and white on
   light `--r3` (4.12), `--l` (4.04) and `--intro` (4.37). The smallest lightness change (same
   hue and saturation, HSL) that passes every pair, each 4.5 or more on `--btn`, on `--g` and
   under `--solid-ink`: dark `--rose-400` #c45a5a → **#c66060**, `--plum-400` #8f62a8 →
   **#986faf**; light `--pink-700` #d0186f → **#c21668**, `--orange-700` #c85f00 →
   **#a24d00**, `--teal-700` #008f78 → **#007360**, `--gold-700` #857a1f → **#6e651a**,
   `--green-700` #1c8040 → **#19733a**, `--slate-600` #4d7480 → **#476b76**.
   `contrast.test.ts` gains the rows: each part and section hue, `--a`, `--ok`, `--warn`,
   `--rec` on `--g` and on `--btn`; `--solid-ink` on each part and section hue and on `--ok`.
   The dark changes are within 4% lightness (the glance order holds: the chord and Main stay
   the brightest). Until it lands, `npm run shots -- Stage` reports exactly these pairs as
   `color-contrast` violations and nothing else; the Inspect verdict names them as C6 and
   passes the rest. (Owner call: the alternative is to exempt hue text like D47's dim text;
   the spec takes the token change because the kit's own rule is that the tokens win over
   the board for AA, as light `--m` already did.)

## Checks

Vitest (`npx vitest run` on the page and component tests), each against the board fixture
unless it says otherwise. They read roles, names, attributes and the commands sent (a fake
`send`), and the `data-face` / `data-hue` hooks (kit › Faces, D41); never computed colours or
layout, which jsdom doesn't have.

1. Nine strips; strips 7 and 8 are `role="group"` named "Fader 7 unused" / "Fader 8 unused",
   read "—", contain no `slider` or `button`, aren't focusable and send nothing when dragged
   or wheeled (D55).
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
   the tempo row's `textContent`, whitespace collapsed, starts with "104 BPM" (the number and
   the unit are separate elements; tempo 103.6 → "104"); "Running". Stopped: "Stopped", the
   section dot hidden.
6. Chord split (a pure function, `splitChord(name)`): each example in Display › Chord gives the
   runs shown, and "N.C." gives one run.
7. Sounds row: R2's cell shows "41 Silk Strings" and the edited mark; R3's shows ⚠ and "off" and
   its tag has `data-hue="d"`; the rack reads "Rack · A1" and "Sunday drive" with the modified
   dot; the R2 sound's `aria-label` is the template's example.
8. Pads: pad 10 `data-face="solid"` (Playing), pad 11 Next (`data-face="waiting"`,
   `data-anim="flash"`, numeral "NEXT"), pads 3 and 7 Absent (`data-face="disabled"`,
   `aria-disabled`); clicking a pad sends its `action`; Pad Bank ▲ is `aria-disabled` with its
   action null, ▼ sends its action. On the Chord page (the state of a `new MockSession()`
   after `{ type: 'setPadPage', page: 'chord' }`, so the mock's `derive()` fills the pads),
   every pad has the fallback face and its label as caption.
9. Faders: a 40px upward drag on strip 1 from any point (pointerdown, pointermove, pointerup;
   jsdom has no `setPointerCapture`, so the test stubs it on the element) sends its `set`
   with `volume` 113 (90 + round(40 × 127 / 223)) on the pointerup; strip 2 shows "↕" and the
   ghost; the Reverb tab sends `setFaderLayer {layer: 'reverb'}`, and with that layer strips
   1–4 read "Rev …" and have no meter element.
10. Meter maths (pure, `height` and `holdPeak`; kit › FaderStrip gives the file): `height(0.0724)` is 138,
    `height(0.001)` is 0; `holdPeak(undefined, 0.5, 0)` is `{ peak: 0.5, sinceMs: 0 }`;
    fed `peak` 0.1 at 1000 and 1500 it still reads 0.5; at 2500 it reads 0.05 (one second
    past the hold, −20 dB); at 2500 with `peak` 0.2 it reads `{ peak: 0.2, sinceMs: 2500 }`.
11. Lamps: a click on the lamp named "Right 1" sends `togglePart {part: 0}` and its `aria-pressed` stays
    `"true"` until the state changes (D49); a 350 ms press sends `setLayer
    {swap, part 0}` and no toggle; Sound, not lit: a click sends `setLayer sound`; a 350 ms
    press sends `setLayer sound` at 350 ms (before release) and `setLayer none` on release; lit,
    a click sends `setLayer none`; latched, a screen pad press sends the pad's action and then
    `setLayer none`; Looper long press sends `looperRec`. (Fake timers drive the 350 ms.)
12. Knobs: ▲ disabled on page 1; ▼ sends `stepKnobPage {delta: 1}`; an 8px upward drag on knob 1
    (pointerdown, pointermove, pointerup) sends `turnKnob {knob: 0, delta: 2}` once, on the
    pointerup (the frame's steps are flushed on release, kit › Knob); a double-click sends
    `resetKnob {knob: 0}`; the tempo
    knob's `knobFraction(null, 104)` is 0.267 and `knobFraction(51, 104)` is 51/127 (pure
    function); knob 1 has `aria-valuenow="127"`, the tempo knob `aria-valuenow="104"` with
    `aria-valuemin="40"` and `aria-valuemax="280"`; No Assign is `aria-disabled` with
    `aria-valuenow="0"`, an empty code line, and sends nothing on arrows.
13. Transport: the pairs are Reset | Fade and Fill ▲ | Fill ▼; Tempo + held repeats (existing
    `tempoHold` tests) and a keyboard click (Enter: a `click` with `detail` 0 and no
    pointerdown) sends one `tempoUp` (D62); Style tempo sends `resetTempo`; Fill ▲ sends
    `fillUp`.
14. Track ◀: sends `surface.controls[trackPrev].action`; with `ui.shift` its `shiftAction`; with
    `surface.trackPrev` null it is `aria-disabled`.
15. Every interactive element has a `data-tip` in the catalog (the existing tooltip test).
16. Health slot: part 2 `plugin.status` failed (not missing) → "R3 failed", `data-hue="ending"`,
    a button whose click calls `onopen({ channel: 2 })`; `meters.cpu.total` 0.74 → "CPU 74%";
    `dropouts` 2 → "2 dropouts", 3 with `bufferFrames` 256 → "3 dropouts · buffer 256?", 3 with
    `bufferFrames` 1024 or null → "3 dropouts" (its click calls `onopen('settingsAudio')`);
    calm → "Audio", no button inside. `DropoutWatch.recent` (pure over its own `observe`
    calls, in `lib/dropouts.test.ts`): after a baseline `observe({ dropouts: 0, bufferFrames:
    256 }, -1000)` (the first sight only records the count), dropouts 1, 2, 3 observed at 0,
    10 s and 20 s read 3 at 25 s, 2 at 35 s, 1 at 41 s (the 10 s one is 31 s old), 0 at 51 s
    (D54).
16b. Page tabs: `page` `'channel'` → the Channel tab has `aria-current="page"` and
    `data-face="chosen"`, Stage has neither; a click on Effects calls `onpage('effects')`.
    Tab chosen rules (D52, `chosenPage()` in `app/src/pages/stagePage.ts`, pure over its
    inputs): `{ view: 'library', libraryTab: 'racks' }` → `quickRacks`; `{ view: 'library',
    libraryTab: 'sounds' }` → `library`; `{ view: 'stage', drawer: 'effects' }` → `effects`;
    `{ view: 'stage', drawer: 'rack', channelOpen: true }` → `channel`; `{ view: 'stage',
    drawer: null, channelOpen: false }` → `stage`.
17. Status line: shows `message.text`; with `error`, the ⚠; a click on its button sends
    `clearMessage`; with `message` null it has no focusable content.
18. Keys: the key elements with `data-note` 43, 45, 48, 52 have `data-hue="l"`, 76 and 81
    `data-hue="r1"`; the split marker carries `data-split="54"` (the key it follows); the
    strip is `role="img"` and its `aria-label` is "Keys: split F#2, left hand G A C E, right
    hand E4 A4, 61 keys"; with `held` empty it is "Keys: split F#2, no keys held, 61 keys"
    (D56).
19. Light tokens (a text test over `app/src/ui/tokens/light.css`, no rendering): `--bg`, `--ba`,
    `--ba2`, `--bw`, `--bm`, `--bl` are `none` and every `--*-glow-mix` is `0%`.
20. Links (D32), in `StageWiring.test.ts` with a `MockSession`: the rack readout opens the Rack
    drawer (`ui.rack` true), a sound cell opens Library › Sounds for its part (`ui.view`
    `library`, `libraryTab` `sounds`, `libraryPart` the part), the health slot's "3 dropouts"
    opens Settings on Audio (`ui.settings` true, `nav.tab` `audio`), and the Metronome caret is
    `aria-disabled`.

**Story and screenshot checks** (`npm run shots -- Stage`, real Chrome), for what jsdom can't
see:

- `Pages/Stage` › `Board` (export `Board`, layout `fullscreen`, `parameters.shots = { viewport:
  { width: 1440, height: 900 }, mask: ['[data-shot-mask="when"]'] }`) renders `Stage` with
  the board fixture's exports (Board fixture, the table), unscaled (D1 lives in the app shell), in both
  themes, against `app/src/ui/Stage/crops/Board-dark.png` and `Board-light.png` (copies of
  `docs/design/push/png/Stage-Dark.png` and `Stage-Light.png`, 1440 × 900): at most 0.02 of the
  unmasked pixels differ. This needs the shots.ts item in the gap table.
- The same story covers what the vitest checks can't: glows by hue (dark) and none (light), the
  chord's two weights, meter heights, the tempo arc, the art.
- `Primitives/ChordReadout` › `LongChord` (`name: "C#m7b5/G#"`; `npm run shots --
  ChordReadout`, since the tool shoots one component folder): the chord fits its 300px at the
  shrunk size and doesn't wrap; no crop (the board has no long chord), judged by Inspect.
- `Components/StyleLine` › `LongName` (a 60-character style name, queued style "Another Very
  Long Style Name"; `npm run shots -- StyleLine`): the line stays one row inside 814px, the
  name ends in an ellipsis, the queued chip at most 200px; no crop.
- axe finds no violation on any story, with the one listed exemption (D47): the
  `color-contrast` rule skips elements carrying `data-contrast="dim"` (the dimmed `--d` text
  the board draws on clickable and idle things; kit › Test hooks lists every one). Every other
  rule, and colour contrast on everything else, must pass. The exemption is a configured
  selector in `app/scripts/shots.ts` (gap table), not a disabled rule.

## Decisions

- **D1 · Scaling.** The Stage is laid out at 1440 × 900 and scaled uniformly to fit the box
  the shell gives it (`scale = min(w / 1440, h / 900)` of that box, centred, the rest `--g`;
  the box is the window minus the interim footer, D53). At 1024 × 700 that is 0.71. The
  scaling is the app shell's (a wrapper in `App.svelte`), so the `Stage` component and its story
  are always 1440 × 900. The board is fixed-size and the old `--u` scheme doesn't fit it; a
  responsive pass is a follow-up.
- **D2 · Pages.** The tabs set an app-only `ui.page` (once the pages exist; until then the
  chosen tab is derived, D52) (`stage`, `channel`, `effects`,
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
  "The left hand" is `chord.upper` false and `chord.fingering` not `fullKeyboard` or
  `aiFullKeyboard`; anything else is accent.
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
  The screen can't tell a latch from a hardware hold (`surface.layer` is the same), so the
  rule is simply: while `surface.layer.type` is `sound`, a screen pad press sends the action
  then `setLayer none` (a hardware hold ends a moment early; C3 makes the two the same).
  Hardware pad taps release the latch once C3 lands. No timer beyond the long-press gesture
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
  | Settings › System (#532): health "Audio off", dropouts, CPU | the Settings drawer on its Audio tab: `nav.tab = 'audio'` (`panels/settings/nav.svelte.ts`) then `ui.settings = true` (opened, not toggled, as `DropoutNotice` did) |
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
  native 1440 × 900 with no scaling, masked only on the count row's When item.
  `app/scripts/shots.ts` gains a per-story viewport and element masks as a build-time item of
  the Stage lane (gap table), before the lane's screenshot check can pass; the spec only says
  what the tool must do.
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
  when the library isn't loaded or the style has no folder (`style.timeSignature` is never null,
  so the metre always shows). A queued style is named from the library by id, else "next
  style". Intro and Ending D read IV. A tone column grows past 24px for two-character names
  ("C#", "Eb"). Letter-spacing values in this spec and the kit are px.
- **D45 · Health tooltip.** The health slot carries one key, `app.health`, whatever it shows; an
  element can carry one `data-tip`, and one body can explain every state of the slot.
- **D46 · Page wiring.** The `Stage` component in `app/src/ui/Stage/` is pure (props in,
  callbacks out). A new `app/src/pages/StageWiring.svelte` connects it to `app.state` and
  `app.send`, keeps `now` (once per animation frame), `receivedMs` and the meter holds, and maps
  each callback to its command or interim target (D32). `App.svelte` mounts it in place of
  today's stage and wraps it in the D1 scaler. The prop list is in The `Stage` component.
- **D47 · Dimmed text stays dimmed (owner).** The board draws `--d` text on things that are
  clickable or idle (the R3 tag and sound number of a part that doesn't sound, an off part's
  strip name and value, idle pad numerals on `--btn`, absent captions), at about 2.2:1 on dark.
  They stay as drawn. The axe colour-contrast check exempts them through one listed rule:
  every such element carries `data-contrast="dim"` and `app/scripts/shots.ts` configures
  `color-contrast` with the selector `*:not([data-contrast="dim"])`. Nothing else is exempt;
  the list of carriers is in kit › Test hooks, and an element not on it that fails is a bug.
  A held key's label is `--solid-ink` on the hue, not dimmed: it carries no `data-contrast`
  (C6 makes that pair pass). `tokens/contrast.test.ts` keeps its AA rows for `--m`, `--t2`,
  `--lamp-ink` and `--solid-ink` and gains the hue rows of C6; `--d` has none (disabled text,
  WCAG 1.4.3).
- **D48 · Tokens and fonts in the running app.** The kit's tokens are global: `main.ts` imports
  `@fontsource/dm-sans` 200, 300, 400 and 500, `@fontsource/jetbrains-mono` 400 and 500, and
  `./ui/tokens/index.css` after `./app.css` (Barlow stays loaded for the interim pages). Both
  token sets live on `:root` and follow the same `data-theme`. Four names clash (two today,
  two once the kit's key tokens are added), and the kit's win by order, so `app.css` renames
  its own and their users follow: `--bg` (a colour; the kit's is a glow) → `--room`
  (`app.css` body, `panels/mixer/MasterFx.svelte`), `--line` → `--hairline` (its readers:
  twelve files under `panels/`, among them the hotspot `panels/mixer/Strip.svelte`, a
  one-word rename that this lane owns for that line, and `ui/tokens/Foundations.mdx`, which
  documents the kit's own `--line` and stays; `--line-strong` is renamed with it to
  `--hairline-strong` for consistency, though it clashes with nothing: its ten readers are
  `lib/ui/Knob.svelte`, `lib/ui/PanelSlot.svelte` and eight files under `panels/`),
  `--key-white` → `--key-ivory` and
  `--key-black` → `--key-ebony` (`panels/keystrip/KeyStrip.svelte`, unmounted but kept).
  `app.css`'s `:focus-visible` (2px
  `--accent`) stays for the old panels; every kit component sets its own 1px `--focus` ring on
  a class selector, which wins. The Stage's root sets `font-family: var(--font-sans)`, so the
  old `:root` font (Barlow) reaches only the interim pages, drawers and the footer. An interim
  component rendered inside the Stage (the Channel snippet, D53) keeps its old look: it reads
  only old names, and the Stage root's font is overridden on the snippet's wrapper
  (`font-family: var(--font-body)`, `font-size: 16px`, `color: var(--ink)`).
- **D49 · LampButton is controlled.** `aria-pressed` follows `on` alone; a click calls
  `ontoggle` and changes nothing until the parent passes a new `on` (the next state). The
  local flip goes (its `Toggles` story checks `ontoggle`'s payload instead of the face), so a
  lamp lit from `sounding` or `surface.layer` can't show a state the engine refused. It gains
  `data-face` (`on`, `off`, `record`, `waiting`, `disabled`), `data-hue` (`lamp`, `rec`), and
  `waiting: 'lamp' | 'rec'` for the Looper's armed faces (D13). Lamp-row lamps carry no code
  (the board draws none); Accomp's "ACMP" is the only code on the Stage.
- **D50 · Button faces.** `Button` takes `face` (`off` default, `on`, `chosen`, `waiting`) and
  `hue` (a token name for the waiting border and label; `t2` default), plus `pressed` for
  `aria-pressed` and `disabled` (kit › Faces). Fade armed is `face="waiting"` (hue `t2`),
  fading or holding `face="on"`; an applied One Touch is `face="chosen"`; "?" is `off` and,
  in help mode, `chosen` with `aria-pressed="true"` (the board draws help off); its glyph is
  14px like the One Touch digits.
- **D51 · Swap mode block.** The knob header's block stays the accent block (`--g` on `--a`) in
  swap mode and reads `knobs.pageName` ("Swap R1"); the part's hue shows on its lamp ("Swap")
  and its strip. A hue-filled block would add a text-on-hue pairing (R3's orange fails AA in
  light) for a state the board never draws.
- **D52 · Which tab is chosen.** One pure function in the wiring, `chosenPage()`, first match
  wins: `ui.view === 'library'` → `quickRacks` when `ui.libraryTab === 'racks'`, else
  `library`; the open drawer (`toggleDrawer` keeps at most one): `effects` → `effects`,
  `multipad` → `multiPads`, `looper` → `looper`, `harmony` → `harmArp`, `settings` → `settings`
  (`rack` and `charts` have no tab and fall through); `channelNav.open` → `channel`; else
  `stage`. A tab click runs that page's `NAV` entry's `toggle()`, so clicking the chosen tab
  returns to the Stage. `NAV` gains `stage` (`view.stage`, Alt+G: `ui.view = 'stage'`, every
  drawer flag false, `channelNav.close()`; `open()` is `chosenPage() === 'stage'`) and
  `channel` (`nav.channel`, Alt+N: open → `channelNav.close()`; closed → the Stage entry's
  toggle then `channelNav.show(ui.selectedPart)`; `open()` is `channelNav.open`), both before
  Library in the list. The Library tab uses `view.library` (its body already says "in place of
  the stage"); `nav.library` stays on the old strip's entry until C5 rewrites the `nav.*`
  bodies.
- **D53 · The shell until the other pages land.** Laid out in The app shell around it:
  `HelpFooter` stays under the scaler (it is the only place tooltips show until #508; the "?"
  button toggles it as today), drawers and modals stay as fixed overlays over the scaled Stage
  in the old tokens, Library replaces the whole Stage unscaled, and the interim Channel is a
  `display` snippet in the display's box (the one interim thing inside the scale, since the
  box is). Nothing else interim is scaled; nothing interim is restyled with kit tokens.
- **D54 · Dropouts in the slot.** The slot shows a 30 s window, not a latched notice:
  `DropoutWatch` gains `recent(now)`: `this.times.filter((t) => now - t <= HINT_WINDOW_MS).length`,
  the number of dropouts it has seen in the last 30 s (at most `HINT_COUNT`, 3, since
  `observe` keeps that many; `recent` filters by its own `now`, as `times` is only pruned on
  `observe`), and the wiring calls
  `dropouts.observe(io.synth, Date.now())` on every state and passes `dropouts.recent(Date.now())`
  once per animation frame (wall-clock ms, as `DropoutNotice` used; the session clock is for the
  beat). Rows: 3 with `io.synth.bufferFrames` a number below 1024 → "3 dropouts · buffer
  {bufferFrames}?"; otherwise `n` ≥ 1 → "{n} dropout" / "{n} dropouts" (the buffer hint has
  nothing to suggest at 1024 or when the size is unknown); the text goes calm by itself 30 s
  after the last dropout; there is no dismiss. `show`, `dismiss` and `SNOOZE_MS` stay for the
  unmounted notice.
- **D55 · Unused strip markup.** An unused strip (`set` null) is one `<div role="group"
  aria-label="Fader 7 unused">` whose children are presentational (`aria-hidden` spans: the
  dashed groove and "—"), with no `slider`, no `button` and no `tabindex`. A `slider` without
  `aria-valuenow` fails axe's `aria-required-attr`, and there is nothing to set.
- **D56 · Keys aria-label.** Template: "Keys: split {split}, {left}, {right}, {n} keys", where
  `split` is the split key's name with octave (`noteName(keyboard.leftSplit)` from
  `keyboard.ts`, Yamaha numbering: 54 → "F#2"); `left` is "left hand " plus the pitch-class
  names (`pcName`, no octave, low to high; the file's spelling, C# Eb F# Ab Bb) of the held
  keys with `zone` `left`, or is dropped when there are none; `right` is
  "right hand " plus the names with octave (76 → "E4") of the held keys with `zone` `right`,
  or dropped; with no held keys at all the two are replaced by "no keys held"; `n` is the
  range (49, 61, 88). Examples: "Keys: split F#2, left hand G A C E, right hand E4 A4, 61
  keys", "Keys: split F#2, right hand E4, 61 keys", "Keys: split F#2, no keys held, 61 keys".
  Full Keyboard changes the detection line only; the split still reads.
- **D57 · The app bar's half pixel.** The bar is 36 tall border-box with its 1px line as the
  36th row (y 59); its content box is 35. The tabs are 36 tall and `align-self: flex-end` in
  it, so a tab's 24px chosen block spans y 35–59 and sits on the line (the board centres the
  36px nav in the 35px box, a half pixel Chrome snaps the same way). The fader header does the
  same with 35-tall tabs in a 35 content box: the 22px block spans 13–35, on its line.
- **D58 · Band sends by block.** Each value is found by `block` (`reverb`, `chorus`,
  `variation` for Delay), so the order the engine sends doesn't matter; a missing block reads
  "–" and the word stays, so the button keeps its width and its meaning before `home` arrives.
- **D59 · Master button in a layer.** The master button reads one word, "Panel" or "Style",
  whatever the fader layer (the hardware's label is "PANEL REV", but the layer already shows
  twice: "Faders · Reverb" and the chosen layer tab); no second line, the 65px cell has no
  room for one at 13px. Shift-click steps the layer (`stepFaderLayer`).

- **D60 · Two meter readers while the interim Channel shows.** `session.meters()` returns the
  peaks since the last call, for one reader. While today's `ChannelView` is in the display's
  box it keeps its own 100 ms poll, so the Stage's faders and the Channel's meter split the
  peaks between them and both may read a little low. Accepted for the interim: the Channel
  spec (#501) replaces `ChannelView` and its poll, and the wiring is then the only reader.
  The wiring does not pause its poll (the faders stay visible under the Channel).
- **D61 · Knob and key-strip ARIA.** A knob is `role="slider"` with `aria-valuemin="0"`,
  `aria-valuemax="127"` and `aria-valuenow` its `level`; the tempo knob (`level` null) uses
  `aria-valuemin="40"`, `aria-valuemax="280"` (the range `knobFraction` spans) and
  `aria-valuenow` `Math.round(transport.tempo)`; No Assign uses 0 / 0 / 0 with
  `aria-disabled`. `aria-valuetext` is "{name} {value}" ("Dynamics 127", "Tempo 104 BPM").
  The key strip is `role="img"` with the D56 label and `aria-hidden` children, like the chord
  column (axe's `aria-prohibited-attr` forbids a bare `aria-label` on a `div`).
- **D62 · Right-click and keyboard clicks on held gestures.** Every control with a long press
  prevents the browser's context menu (`contextmenu` → `preventDefault`) and treats a
  right-click as a completed long press: a part lamp latches swap, Looper sends `looperRec`,
  and Sound not lit latches (`setLayer sound`, the same as a click, since a context menu has
  no release); Sound lit sends `setLayer none`. Tempo + and − run `TempoHold` from
  pointerdown to pointerup (the first `tempoUp` at once, then repeating); a click without a
  pointer (Enter or Space, `event.detail === 0`) sends one `tempoUp` / `tempoDown`, and a
  click that follows a pointerdown sends nothing more (no double send).
- **D63 · Rack-target rule.** A Panel fader 1–4 is a rack target when `surface.faders[i].label`
  differs from the engine's own label for that part, `["RIGHT 1", "RIGHT 2", "RIGHT 3",
  "LEFT"][i]` (`PART_LABELS` in `crates/yahaha-engine/src/launchkey.rs`; the strips' display
  names "Right 1" … are the kit's own and are never compared). The name then reads the label
  as given ("PANR2"), `--t2`, and both the slider and the name carry `launchkey.fader_rack`.

## Follow-ups

- Per-style artwork and key colour (DECISIONS S4), with what generates them.
- A responsive layout for 1024 × 700 to 1920 instead of the uniform scale (D1, DECISIONS S14).
- C4, chord held, needs the engine to know when detection is unsure.
- Stereo meters per part would need per-channel left and right levels in `meters`.
- A face for the Ending's ritardando (`transport.ritardando`).
- Typed tempo entry on the tempo readout, if players ask.

# yahaha app: contributing

The desktop app for yahaha is a Tauri 2 shell (`src-tauri/`) around a Svelte 5 + TypeScript
frontend (`src/`), built with Vite. It works fully offline because the fonts are bundled
and nothing loads from a CDN.

The main screen is a **mirror of the Launchkey 49/61 MK4 control surface**. What you see
maps 1:1 onto the hardware under your hands. Above it sits the **lead-sheet band** (where
the song is, what's next; later the chord chart), and below it the **keyboard strip** (the
keys you hold, the splits, the chord). Everything else opens in panels around it:
Keyboard parts + OTS, Mixer, Settings (right-side drawers), and the style browser (a
modal).

## Run it

```bash
cd app
npm install
npm run dev                # the frontend alone, in a browser, on the mock session: http://localhost:5173
cargo tauri dev            # the desktop app on the real engine (it runs `npm run dev` itself)
YAHAHA_MOCK=1 cargo tauri dev   # the desktop app on the mock (no MIDI, no styles needed)
cargo tauri build --debug  # a .app in src-tauri/target/debug/bundle/macos/
npm run verify             # svelte-check (fails on warnings) + eslint + vitest
npm run docs:controls      # regenerate docs/controls.md from the tooltip catalog
npm run screenshots        # docs/screenshots/*.png (needs `npm run dev` and Google Chrome)
```

URL switches for browser dev mode:

| switch | what it does |
|---|---|
| `?demo=0` | starts the mock stopped with Sync Start armed, instead of the scripted demo |
| `?theme=light` | starts in the light theme |
| `?help=1` | starts with help mode on |
| `?tip=<catalog key>` | shows that entry in the help footer, as if you hovered that control |
| `?open=browser` \| `parts` \| `mixer` \| `settings` \| `harmony` \| `charts` | opens that panel |
| `?shift=1` | latches the Shift layer |
| `?styles=N` | adds N synthetic styles to the mock's library (try 60000 in the browser) |
| `?chart=1` | imports the mock's demo chart playlist and turns chart mode on (with the demo: playing it) |
| `?mock` | uses the mock even inside Tauri |

## Why Svelte 5 + Vite + TypeScript

- **Fine-grained updates.** Svelte compiles to direct DOM updates. The state changes up
  to 60 times a second while playing, and only the text and styles that actually change
  get touched. There is no virtual DOM to diff.
- **Small.** There's no framework runtime to ship. Each component is one `.svelte` file
  with scoped CSS, and everything else is plain TypeScript.
- **Standard tooling.** Vite is what Tauri's templates use, and `vitest` shares its
  config.

## File layout

```
app/
  CONTRIBUTING.md            this file
  docs/controls.md           generated from the tooltip catalog: every control, key, Launchkey place
  docs/screenshots/          npm run screenshots
  scripts/                   controls-doc.ts (catalog → docs/controls.md), engine-shape.ts, screenshots.sh
  src-tauri/                 the Rust shell
    icons/icon.svg           the app icon's source (original artwork); `cargo tauri icon
                             icons/icon.svg` in src-tauri regenerates the set
    src/lib.rs               commands send/state/library and the `yahaha` event, backed by
                             yahaha::Session (the engine) or the mock; env: YAHAHA_STYLES,
                             YAHAHA_SOUNDFONTS, YAHAHA_SF2, YAHAHA_MOCK (see the file's header)
    src/mock.rs              mock session on the engine's own AppState types
  src/
    App.svelte               the layout shell: app bar, the stage (lead-sheet band, Launchkey
                             mirror, keyboard strip) and its scaling, drawers, browser
    main.ts                  entry point
    app.css                  DESIGN TOKENS AND MATERIAL RECIPES (see "Design")
    help/
      tooltips.ts            THE TOOLTIP CATALOG: the single source of truth for help text
      actions.ts             which catalog entry explains an AppCmd (pads and buttons use it)
      *.test.ts              the coverage test and the README shortcut test
    lib/
      api/types.ts           AppState / AppCmd / LibraryList, mirroring #16's docs/app-api.md
      api/session.ts         the Session interface + connect() (Tauri or mock)
      api/tauri.ts           Session over Tauri IPC
      api/mock.ts            Session in the browser: a plausible band that plays itself
      api/mock-pads.ts       the mock's pad lights (a port of looks() in src/launchkey.rs)
      api/mock-fixture.json  fake style library and GM names (shared with mock.rs)
      api/engine-shape.json  key paths of the real engine's JSON (from `yahaha state-json`);
                             engine-shape.test.ts checks the mock against it
      store.svelte.ts        app (state, send, library), clock (beat clock), ui (panels, theme, Shift)
      leds.ts                LED maths: brightness on the beat clock, colours, lamp notes
      surface.ts             the Launchkey surface beyond the pads (see "The surface")
      keys.ts, shortcuts.ts  keyboard bindings (the terminal UI's keys) and the window handler
      tooltip/               tip action, HelpFooter (+ last Launchkey control), TipCard, opt-in pop-up Tooltip
      ui/                    shared components: HwButton, Fader, Toggle, Overlay, PanelSlot
    panels/
      header/Header.svelte         BUILT: the app bar (panel buttons, help, theme)
      header/TransportBar.svelte   BUILT: the transport row under it (Start/Stop, Sync Start/Stop,
                                   Intro/Ending I–III, Tempo −/+ and Tap, bar.beat, section);
                                   buttons light from `transport.lamps` like their pads
      leadsheet/                   BUILT: the lead-sheet band above the mirror (see below), and
                                   ChartLane.svelte: the iReal chord chart in it (chart mode)
      charts/Charts.svelte         BUILT: the iReal Pro chart player drawer (import, songs, settings)
      launchkey/                   BUILT: the hardware mirror (see below)
      keystrip/                    BUILT: the keyboard strip under the mirror (see below)
      parts/Parts.svelte           SLOT: Keyboard parts + OTS drawer
      mixer/Mixer.svelte           SLOT: Mixer detail drawer
      browser/Browser.svelte       SLOT: style browser (modal)
      settings/Settings.svelte     SLOT: settings drawer
      harmony/Harmony.svelte       BUILT: Harmony/Arpeggio drawer (switch, type lists, settings)
```

### The Launchkey mirror (`panels/launchkey/`)

| file | what |
|---|---|
| `Launchkey.svelte` | the surface, laid out like the MK4: status display, pad-page tabs, Shift, Track ◀/▶ (with the neighbouring styles), Pad Bank ▲/▼, 2×8 pads, the tempo buttons (Scene Launch / Function), Stop/Play, faders |
| `StatusDisplay.svelte` | the "screen": style, tempo, bar/beat, section → next, a big chord, fingering, split, transpose |
| `HwPad.svelte` | one rubber pad lit from the engine's `Pad` |
| `Control.svelte` | one Launchkey button rendered from the surface state |
| `FaderBank.svelte` | 8 faders + master and their buttons, from the surface state |
| `Launchkey.test.ts` | behaviour tests: pages, pad presses, Shift layer, fader pages, pickup |

### Scaling: the stage (`App.svelte`)

The lead-sheet band, the mirror and the keyboard strip form one **stage** that fills the
window by width *and* height, with no page scroll at any size down to the 900×600 minimum
window. The stage is a size container (`container: stage / size`); the stack inside sets
its font size to

```css
--u: min(100cqw / var(--w), 100cqh / var(--h));
```

and every size on the stage is in `em` of it. `--w` is the mirror's design width (96em)
and `--h` the stack's least height (lead band min + mirror + strip min + gaps). The mirror
keeps the hardware's proportions; the band and the strip take the height left over, up to
a limit, then the stack centres. Nothing is scaled with `transform`, so text and borders
stay crisp at any size.

When the stage is taller than 1.45:1 (a 1024-wide window, say), the mirror switches to
its **stacked layout**, `@container stage (aspect-ratio < 1.45)`: the fader bank moves
under the pads, the design width drops to 66em, and so everything grows. If you change the
mirror's rows or the band's or strip's minimum heights, re-measure the mirror's height in
em (`.device` height ÷ `--u`) in both layouts and update `--h` in `App.svelte`.

Panel print on the stage is `0.78em` (the global `.engraved` is in `rem`, for drawers).
Anything that can grow (a style name under Track ◀/▶) wraps rather than being cut off.

Shift: the on-screen Shift button latches the layer (`ui.shiftLatched`), and holding
Shift on the computer keyboard shows it too (`ui.shiftHeld`). The layer shows when
`ui.shift || surface.shift` (the hardware's Shift).

### The lead-sheet band (`panels/leadsheet/`): the chart slot

The band above the mirror answers "where am I, what's next":

| column | what |
|---|---|
| now | the section playing (`transport.section`), or what the band will start with; "bar 3 of 4" |
| **lane** (`[data-slot="chart"]`) | one cell per bar of the section (`transport.sectionBars`, provisional), a slash per beat lit as it passes, and a progress bar across the section (`transform: scaleX` on the beat clock, so it runs at 60 Hz) |
| next | the queued section (`transport.queued`), amber |

**The lane is where the iReal chord chart shows (M8, #89).** In chart mode (`state.chart.on`
with a song chosen) `ChartLane.svelte` renders the chart into the lane instead of the
section cells, and the now/next columns stay ("bar 5 of 32"). It is built to this contract:

- Put the chart in `panels/leadsheet/` (e.g. `ChartLane.svelte`) and switch on it in
  `LeadSheet.svelte`: `{#if chart}<ChartLane {chart} />{:else}…section cells…{/if}`. The
  lane is a flex column that fills the band's middle; keep `data-slot="chart"` on it.
- Draw the chart as rows of the same bar cells (`.cell.mat-screen`): a chord symbol per
  bar or half bar, the current bar ringed (`.current`), beat slashes for bars without a
  new chord. Show about two rows: the current line and the next.
- The band's height comes from the shell: at least 5.2em and at most 10em of `--u`
  (`.lead-slot` in `App.svelte`). Size everything in `em`, and never let the band grow
  the page. If the chart needs more room, raise `max-height` on `.lead-slot` and re-check
  1024, 1440 and 1920.
- The chart's position comes from the engine (`state.chart.bar`, an index into
  `state.chart.song.bars`). Nothing advances a cursor in TypeScript; the bar's own progress
  line runs on `clock.pos`, as the section progress bar does.

The Charts drawer (`panels/charts/`) imports playlists (an `.html` file read in the
browser and sent as `importCharts` text, or a pasted `irealb://` link), lists playlists
and songs, and sets chart mode, choruses, Intro, Ending, loop and Auto style. The browser
mock can't decode iReal links: `importCharts` adds its demo playlist
(`lib/api/mock-chart.ts`, made-up progressions). The Rust mock uses the engine's parser.

### The keyboard strip (`panels/keystrip/`)

The strip under the mirror shows the keys held, coloured by the part that sounds them
(`--part-r1`, `--part-r2`, `--part-r3`, `--part-left`, and `--part-chord` for a left-hand
key that only feeds chord detection; layered parts show as bands), the split point (drag
it, or arrows with focus: `setSplit` / `moveSplit`), the Left split when it differs, the
chord-detection area (Lower, Upper or Full Keyboard) and the recognised chord's tones
(dots; the ringed one is the bass). `keyboard.ts` has the geometry: keys are
percentage-positioned boxes, so a key press only changes one class and one custom
property.

Its size matches the connected Launchkey (49 or 61, from the port name) until the user
picks 49, 61 or 88 on the strip's cheek (`ui.keyRange`, remembered). Held keys, the Left
split and the chord tones come from the provisional `state.keyboard` (`KeyboardState` in
`types.ts`); the mock sends them, and until the engine does the strip shows the splits and
the detection area only.

### The surface (`lib/surface.ts`)

No button's function is hard-coded in the mirror. Pads come from `state.pads.pads`, and
everything else comes from `state.surface` (#77, docs/app-api.md "surface"):

- every non-pad control (Pad Bank, Track, Play/Stop, Scene/Function, the fader buttons,
  the master button) with its CC, label, action, Shift label/action and LED (`colour` is
  the palette index; Play, Stop, Scene and Function are reported off);
- the faders with their label, level, waiting flag, physical position and the command
  moving them sends;
- Shift (the hardware's), the Track neighbours, and the clocks.

`surfaceOf(state)` returns it; `lib/surface.ts` turns controls and faders into tooltips
and commands. The browser mock builds the same surface in `lib/api/mock-surface.ts` (a
port of `Session::surface` in src/session.rs), and the Rust mock in `src-tauri` sends it
too.

**Clocks.** A state carries anchors, not a ticking position (docs/app-api.md
"surface.clock"). `clock` in `lib/store.svelte.ts` notes when each state arrived and runs
them on every frame: `clock.beats` is the free-running **LED clock** the lamps flash and
pulse on (as the hardware pads do), and `clock.pos` is quarter notes into the section
playing (0 when stopped). Never mix `transport.bar`/`beat` with the clock's phase.

**Palette LEDs.** When `pads.paletteLeds` is set, pads light from `pad.palette` (a flash
alternates between its two palette colours): `padLight()` in `lib/leds.ts`.

## Building a drawer or the browser

Each panel owns its folder under `src/panels/<name>/`. Put everything the panel needs
there: sub-components, panel-only helpers and its tests. That way several people can work
on different panels without touching the same files. Move a piece to `src/lib/ui/` only
when a second panel needs it.

1. **Replace the slot.** The placeholder file lists what the panel shows, which state and
   commands it uses, and which catalog keys are ready. Keep the file name, because
   `App.svelte` already mounts it. Keep the `Overlay` wrapper: drawers use
   `side="right"` (not modal, so performance keys keep working), and the browser uses
   `side="center" modal`.
2. **Read state, send commands.** Import `app` from `lib/store.svelte`. Read with
   `$derived` (`const parts = $derived(app.state.keyboardParts)`), and act with
   `app.send({ type: 'togglePart', part: 0 })`. Never copy engine state into a local
   `$state`: the engine owns it, and both the mock and the real engine push a fresh
   `AppState` on every change. Command and field names are #16's (docs/app-api.md).
   `lib/api/types.ts` has them all, with comments. The library is `app.library.entries`.
3. **Use the shared components:**
   - `HwButton`: a backlit button. Pass an LED look for anything lit.
   - `Fader`: 0–127, with a slot, cap, scale, readout and the soft-takeover mark.
   - `Toggle`: an on/off setting.
   - `Overlay`: the drawer or modal frame.

   Each takes a `tip`.
4. **Every interactive element gets a tooltip.** Use the components above, or put
   `use:tip={'catalog.key'}` on anything else you make focusable or clickable. The
   coverage test fails otherwise.
5. **Lamps come from the engine.** Anything the Launchkey lights should use the engine's
   `Pad` (`state.transport.lamps`, `state.pads.pads`) so the colours and animations match
   the hardware: `tipFor(pad.action)` gives its tooltip, and `app.send(pad.action)`
   presses it. Animate on `clock.beats` with `brightness()` from `lib/leds.ts`.
6. **No layout shift.** Anything that changes while playing needs a fixed width or a
   reserved line: numbers, names, lamp states, "next" labels. Numbers are tabular by
   default.
7. **Test it.** Put `*.test.ts` next to the panel. `@testing-library/svelte` renders in
   jsdom, and `new MockSession({ manual: true })` gives you a session you drive with
   `send()` and `advance(ms)`: attach it with `app.attach(session)`. Add your panel's
   open states to `STATES` in `src/help/coverage.test.ts`. Its open state is already
   there as a slot, so update it if your panel has tabs or modes.

## Tooltips

`src/help/tooltips.ts` is the catalog. Every entry has these fields:

| field | what |
|---|---|
| `title` | the control's name as the app labels it |
| `body` | what it does, in 1–3 plain sentences |
| `genos` | the Genos name (null if yahaha-only) |
| `keys` | keyboard shortcuts in README notation (`space`, `shift+1`, `F10`, `←`); `[]` for none |
| `launchkey` | where it is on the Launchkey, `; `-separated if several (`Pad page 3 (OTS/Parts), bottom row, pad 1; Panel fader page: button under fader 1`); null if not on it |
| `app_keys` | only when the app uses a different key from the terminal (Tab → PgDn) |

To add a control, add an entry (or reuse one) and reference its key. The tests check that:

- every interactive element, in every app state, has a `data-tip` that is in the catalog
  (`help/coverage.test.ts`);
- every entry is complete, with a title and a 1–3 sentence body;
- every catalog key is in README's "Terminal keys" list, and every README key is in the
  catalog. The same holds between the catalog and the keys the app binds
  (`help/readme.test.ts`);
- every pad's entry names the pad it is on (`lib/leds.test.ts`);
- `docs/controls.md` matches the catalog (`scripts/controls-doc.test.ts`). Run
  `npm run docs:controls` after changing the catalog.

Tooltip behaviour:

- Nothing floats over the instrument. Hovering or tabbing to a control shows its entry
  in the **help footer**, a fixed two-line bar at the bottom of the window: title, what it
  does, the Genos name, the key and where it is on the Launchkey. It follows at once, and
  keeps the entry for 350 ms after the pointer leaves, so crossing a gap doesn't flicker.
  Its height never changes on hover, so the stage never moves.
- The footer's right end shows the Launchkey control pressed last (`io.lastControl`,
  decoded against `pads` and `surface` in `lib/tooltip/lastControl.ts`).
- Help mode (the `?` key or the ? button) grows the footer to the full entry (every
  Launchkey place, the terminal keys) and pins the last one while you try the control.
- Pop-up tips next to the control are opt-in (the footer's "Pop-up" switch, remembered);
  they wait 280 ms, never take pointer events, and Esc closes them.
- Screen readers: a focused control's `aria-describedby` points at a hidden plain-text
  copy of its entry in the footer.
- Drawers and the browser end `--help-footer-space` above the bottom of the window.

## Keyboard

`lib/keys.ts` binds the terminal UI's keys, so the app plays the same way.

- **Buttons:** Space and Enter on a focused button press that button. Buttons don't keep
  focus after a click, so Space stays Start/Stop for mouse users.
- **Pad pages:** Tab moves focus, so pad pages are PgUp/PgDn here.
- **Shift:** holding Shift shows the Shift layer.
- **Panels:** the style browser takes every key while it's open, but the drawers don't.
  Esc closes the topmost panel.

## Design

The look is a modern, restrained, skeuomorphic instrument: a dark anodised panel,
recessed glass, rubber pads, backlit buttons and real faders. It uses no logos and no
bitmap assets; everything is CSS, so it stays crisp at any size. Spend boldness on the
LEDs and the chord, and keep everything else quiet.

### Tokens (`src/app.css`, redefined under `[data-theme='light']`)

| token | use |
|---|---|
| `--bg` | the room behind the instrument |
| `--chassis-hi`, `--chassis-lo`, `--chassis-edge`, `--seam` | panel metal: top/bottom of the gradient, the bright top edge, the dark seams and borders |
| `--well`, `--well-edge` | recessed areas (the pad well, fader slots, the screen bezel) |
| `--raised-hi`, `--raised-lo`, `--rubber` | button faces and fader caps; pad rubber |
| `--ink`, `--engrave`, `--muted` | text on the panel: values, engraved labels, secondary |
| `--screen-bg`, `--screen-ink`, `--screen-dim`, `--screen-glow` | the display |
| `--accent`, `--accent-ink` | amber: selected, latched, waiting for you (fader pickup), focus |
| `--solo` | a soloed mixer channel's S button (the Genos lights it purple) |
| `--part-r1`, `--part-r2`, `--part-r3`, `--part-left`, `--part-chord` | a keyboard part's colour (held keys; any panel showing parts), and grey for chord-detection-only keys |
| `--key-white`, `--key-black` (+ `-lo`/`-hi`, `--key-gap`, `--key-print`) | the keyboard strip's keys |
| `--lamp-off` | an unlit LED |

LED colours are never tokens. They come from the engine (`rgb`, 0–127), so the screen
always matches the hardware.

### Material recipes (global classes in `app.css`)

| class | for |
|---|---|
| `.mat-chassis` | brushed/anodised metal: gradient, fine brushing, top highlight, drop shadow. The device, the drawers, the tooltips |
| `.mat-well` | a recessed well with an inner shadow |
| `.mat-raised` | a raised button face. `.pressed` or `:active` pushes it in |
| `.mat-screen` + `.glow-text` | display glass with inner shadow and a sheen, and glowing text |
| `.engraved` | small, spaced panel print with an engraved edge |
| `.screw` | a screw head. Use sparingly, at chassis corners |

### Rules

- **Performance.** Animate with opacity and transform only. LEDs are pre-rendered
  gradient layers whose `opacity` is the brightness (`HwPad`, `HwButton`), and fader caps
  move with `transform`. Never animate `filter: blur` or `box-shadow` sizes.
- **Type.** Barlow Condensed for labels, values and the chord; Barlow for body text. Both
  are bundled through @fontsource.
- **Targets.** Every hit target is at least 2.1em tall on the surface (about 30 px at
  laptop size) and 2.2rem in drawers.

## Keeping up with the engine API

- The shell depends on the engine library (`yahaha = { path = "../..",
  default-features = false }`). `src-tauri/Cargo.toml` has its own `[workspace]`
  table, so the root build is unchanged.
- The Rust mock builds the engine's own types, so it can't drift. The TypeScript types
  (`src/lib/api/types.ts`) are the copy to keep in step with docs/app-api.md.
  `yahaha state-json <style> ["C Am F G7"]` and `--library` print real JSON: re-record
  `src/lib/api/engine-shape.json` with `scripts/engine-shape.ts` and the shape test tells
  you what moved.
- The store (`lib/store.svelte.ts`) applies a snapshot only when its `version` is higher
  than the last one applied, so a state fetch that resolves late never overwrites a newer
  one. Attaching a new session starts the count again.
- Provisional fields the UI already reads, until the engine sends them (each has a NEED
  on the coordination board): `state.surface` (the Launchkey surface), `state.keyboard`
  (held keys, the Left split, chord tones) and `transport.sectionBars` (the section's
  length). All are optional in `types.ts`, and the UI works without them.

# yahaha app: contributing

The desktop app for yahaha is a Tauri 2 shell (`src-tauri/`) around a Svelte 5 + TypeScript
frontend (`src/`), built with Vite. It works fully offline because the fonts are bundled
and nothing loads from a CDN.

The main screen is a **mirror of the Launchkey 49/61 MK4 control surface**. What you see
maps 1:1 onto the hardware under your hands. Everything else opens in panels around it:
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
| `?tip=<catalog key>` | shows that one tooltip |
| `?open=browser` \| `parts` \| `mixer` \| `settings` | opens that panel |
| `?shift=1` | latches the Shift layer |
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
    src/lib.rs               commands send/state/library and the `yahaha` event, backed by
                             yahaha::Session (the engine) or the mock; env: YAHAHA_STYLES,
                             YAHAHA_SF2, YAHAHA_MOCK (see the file's header)
    src/mock.rs              mock session on the engine's own AppState types
  src/
    App.svelte               the layout shell: app bar, Launchkey mirror, drawers, browser
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
      tooltip/               tip action, Tooltip, TipCard, HelpBar (help mode)
      ui/                    shared components: HwButton, Fader, Toggle, Overlay, PanelSlot
    panels/
      header/Header.svelte         BUILT: the app bar (panel buttons, help, theme)
      launchkey/                   BUILT: the hardware mirror (see below)
      parts/Parts.svelte           SLOT: Keyboard parts + OTS drawer
      mixer/Mixer.svelte           SLOT: Mixer detail drawer
      browser/Browser.svelte       SLOT: style browser (modal)
      settings/Settings.svelte     SLOT: settings drawer
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

The whole surface scales with the window. It sets its font size from its own width
(`--u` in `Launchkey.svelte`), and every size inside is in `em`. Below 820 px, the fader
bank moves under the pads.

Shift: the on-screen Shift button latches the layer (`ui.shiftLatched`), and holding
Shift on the computer keyboard shows it too (`ui.shiftHeld`). The layer shows when
`ui.shift || surface.shift` (the hardware's Shift).

### The surface (`lib/surface.ts`)

No button's function is hard-coded in the mirror. Pads come from `state.pads.pads`, and
everything else comes from a `SurfaceState` (`lib/api/types.ts`), which holds:

- every non-pad control (Pad Bank, Track, Play/Stop, Scene/Function, the fader buttons,
  the master button) with its label, action, Shift label/action and LED;
- the faders with their label, level, waiting flag, physical position and the command
  moving them sends;
- the Track neighbour names and the beat clock.

The engine will send it as `state.surface` (the API follow-up to #71). Until then,
`surfaceOf(state, library)` derives it with the rules in `src/launchkey.rs`, and the mock
sends it already, with hardware fader positions and a clock. When the engine sends it,
the derivation simply stops being used. If the API PR names fields differently, rename
them in `types.ts` and `surface.ts`; the components read nothing else.

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

- A tooltip appears after 280 ms of hover. It appears at once when you move between
  controls or tab onto one.
- Tooltips never take pointer events, and Esc closes them.
- Help mode (the `?` key or the ? button) shows them at once and pins the last one in a
  bar at the bottom.

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

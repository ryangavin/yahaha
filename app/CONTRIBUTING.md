# yahaha app: contributing

The desktop app for yahaha: a Tauri 2 shell (`src-tauri/`) around a Svelte 5 + TypeScript
frontend (`src/`), built with Vite. It works fully offline (fonts are bundled, no CDN).

## Run it

```bash
cd app
npm install
npm run dev              # frontend only, in a browser, on the mock session: http://localhost:5173
cargo tauri dev          # the desktop app (starts `npm run dev` itself)
cargo tauri build --debug  # a .app bundle in src-tauri/target/debug/bundle/macos/
npm run verify           # typecheck (svelte-check, fails on warnings) + lint + tests
npm run docs:controls    # regenerate docs/controls.md from the tooltip catalog
npm run screenshots      # docs/screenshots/*.png (needs `npm run dev` and Google Chrome)
```

URL switches for the browser dev mode: `?demo=0` (stopped, Sync Start armed, instead
of the scripted demo), `?theme=light`, `?help=1`, `?tip=<catalog key>` (show one
tooltip), `?open=browser|settings`, `?mock` (the mock even inside Tauri).

## Why Svelte 5 + Vite + TypeScript

- Svelte compiles to direct DOM updates with fine-grained reactivity. The state changes
  up to 60 times a second while playing, and only the text and styles that really change
  are touched; there is no virtual DOM to diff.
- It is small: no runtime framework to ship, one `.svelte` file per component with
  scoped CSS, and plain TypeScript modules for everything else.
- Vite is what Tauri's templates use, starts in well under a second, and `vitest` shares
  its config.

## File layout

```
app/
  CONTRIBUTING.md          this file
  docs/controls.md         generated from the tooltip catalog: every control, key, Launchkey place
  docs/screenshots/        npm run screenshots
  scripts/                 controls-doc.ts (catalog → docs/controls.md), screenshots.sh
  src-tauri/               the Rust shell
    src/lib.rs             commands send/state/library, the `yahaha` event, the 60 Hz tick
    src/api.rs             stand-in copy of #16's API types (delete when #16 lands)
    src/mock.rs            mock session (Rust twin of src/lib/api/mock.ts)
  src/
    App.svelte             the layout shell: one grid area per panel
    main.ts, app.css       entry point; design tokens (dark + light), fonts
    help/
      tooltips.ts          THE TOOLTIP CATALOG: the single source of truth for help text
      actions.ts           which catalog entry explains an AppCmd (used for pads)
      *.test.ts            coverage test, README shortcut test
    lib/
      api/types.ts         AppState / AppCmd / LibraryList, mirroring #16's docs/app-api.md
      api/session.ts       the Session interface + connect() (Tauri or mock)
      api/tauri.ts         Session over Tauri IPC
      api/mock.ts          Session in the browser: a plausible band that plays itself
      api/mock-pads.ts     the mock's pad lights (port of looks() in src/launchkey.rs)
      api/mock-fixture.json  fake style library and GM names (shared with mock.rs)
      store.svelte.ts      app (state, send, library), clock (beat clock), ui (overlays, theme)
      leds.ts              drawing pads/lamps: brightness on the beat clock, colours, LAMP notes
      keys.ts              keyboard bindings (the terminal UI's keys)
      shortcuts.ts         the window key handler
      tooltip/             tip action, Tooltip, TipCard, HelpBar (help mode)
      ui/                  shared components: Panel, Key, LampKey, Toggle, Tabs, Fader,
                           Readout, Overlay, PanelSlot
    panels/
      header/Header.svelte       BUILT (reference)
      sections/Sections.svelte   BUILT (reference)
      parts/Parts.svelte         slot: Keyboard parts + OTS
      mixer/Mixer.svelte         slot: Mixer (Panel / Style)
      launchkey/Launchkey.svelte slot: Launchkey pad pages and buttons
      browser/Browser.svelte     slot: style browser (overlay)
      settings/Settings.svelte   slot: settings (side overlay)
```

## Building a panel

Each panel owns its folder under `src/panels/<name>/`. Put everything the panel needs
there (sub-components, panel-only helpers, its tests), so several people can work on
different panels without touching the same files. Shared pieces go in `src/lib/ui/` only
when a second panel needs them.

1. **Replace the slot.** The placeholder file lists what the panel shows and which state
   and catalog keys it uses. Keep the file name: `App.svelte` already places it.
2. **Read state, send commands.** Import `app` (and `clock` for lamps) from
   `lib/store.svelte`. Read with `$derived` (`const parts = $derived(app.state.keyboardParts)`),
   act with `app.send({ type: 'togglePart', part: 0 })`. Never copy engine state into
   local `$state`: the engine owns it, and the mock and the real engine both push a fresh
   `AppState` on every change. Command and field names are #16's (docs/app-api.md);
   `lib/api/types.ts` has them all with comments.
3. **Use the shared components.** `Panel` (titled region), `Key` (button with its key
   hint), `LampKey` (a lit key: pass an engine `Pad` as `look` and `clock.beats`),
   `Toggle`, `Tabs`, `Fader` (vertical 0–127 with the soft-takeover mark), `Readout`
   (a labelled value), `Overlay` (browser/settings). Each takes a `tip`.
4. **Every interactive element gets a tooltip.** Use the components above, or put
   `use:tip={'catalog.key'}` on anything else you make focusable or clickable. The
   coverage test fails otherwise.
5. **Lamps come from the engine.** Anything the Launchkey lights (section lamps, pads)
   should use the engine's `Pad` (`state.transport.lamps`, `state.pads.pads`) so the
   colours and animations match the hardware exactly; `tipFor(pad.action)` gives its
   tooltip and `app.send(pad.action)` presses it.
6. **No layout shift.** Anything that changes while playing (numbers, names, lamp states,
   "next"/"fill" labels) gets a fixed width or a reserved line. Use tabular numbers
   (already the default).
7. **Test it.** Put `*.test.ts` next to the panel. `@testing-library/svelte` renders
   components in jsdom; `new MockSession({ manual: true })` gives a session you drive with
   `send()` and `advance(ms)`. If your panel shows controls only in some state (a tab, an
   open overlay), add that state to `STATES` in `src/help/coverage.test.ts`.

## Tooltips

`src/help/tooltips.ts` is the catalog. Every entry has:

| field | what |
|---|---|
| `title` | the control's name as the app labels it |
| `body` | what it does, in 1–3 plain sentences |
| `genos` | the Genos name (null if yahaha-only) |
| `keys` | keyboard shortcuts in README notation (`space`, `shift+1`, `F10`, `←`); `[]` for none |
| `launchkey` | where it is on the Launchkey, `; `-separated if several (`Pad page 3 (OTS/Parts), bottom row, pad 1; Panel fader page: button under fader 1`); null if not on it |
| `app_keys` | only when the app uses a different key than the terminal (Tab → PgDn) |

Adding a control: add or reuse an entry, reference its key. The tests check that:

- every interactive element in every app state has a `data-tip` that is in the catalog
  (`help/coverage.test.ts`);
- every entry is complete (title, 1–3 sentence body);
- every key in the catalog is in README's "Terminal keys" list and every README key is in
  the catalog, and every key the app binds is in the catalog and the reverse
  (`help/readme.test.ts`);
- every pad's entry names the pad it is on (`lib/leds.test.ts`);
- `docs/controls.md` matches the catalog (`scripts/controls-doc.test.ts`): run
  `npm run docs:controls` after changing the catalog.

Tooltips show after 280 ms of hover (at once when moving between controls, or on keyboard
focus), never take pointer events, and close on Esc. Help mode (`?` key or the ? button)
shows them at once and pins the last one in a bar at the bottom.

## Keyboard

`lib/keys.ts` binds the terminal UI's keys, so the app plays the same. Space and Enter
on a focused button press that button; buttons don't keep focus after a click, so Space
stays Start/Stop for mouse users. Tab moves focus (so pad pages are PgUp/PgDn here). An
open overlay gets all keys; Esc closes it.

## Design

- Tokens are CSS variables in `src/app.css` (`--bg`, `--panel`, `--raised`, `--ink`,
  `--muted`, `--accent` …), redefined under `[data-theme='light']`. Use them, never raw
  colours, except LED colours, which come from the engine's `rgb`.
- Type: Barlow Condensed (labels, values, the chord) and Barlow (text), bundled via
  @fontsource.
- Amber `--accent` means "selected / waiting for you" (focus ring, pressed Key, the
  fader-pickup mark, the current beat). LED colours mean what the Launchkey means.
- Big targets: keys are at least 40 px tall (32 px for the small −/+ kind).

## When #16 lands

- `src-tauri/Cargo.toml`: add `yahaha = { path = "../..", default-features = false }`.
- `src-tauri/src/lib.rs`: manage a `yahaha::Session` instead of `MockSession`; forward
  `session.subscribe()` events as `yahaha` events instead of the tick loop. Delete
  `api.rs` and `mock.rs`.
- `src/lib/api/types.ts`: re-check against the merged docs/app-api.md.

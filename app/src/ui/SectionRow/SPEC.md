# SectionRow

## Identity (all stations)

- **Kind:** complex
- **Built from:** LampButton, CountRow, MetronomeSplit, Button
- **Purpose:** The row under the app bar: Accomp on the left, where the band is in the middle, and the metronome, Unison, Panic and help mode on the right.
- **Boards:**
  - `Stage-Dark.dc.html:96-120` (the board state); light: `Stage-Light.dc.html:72-96`.
  - `Stage-Help-Dark.dc.html:104` (help mode on: the ? lit, line 126); light: `Stage-Help-Light.dc.html:75` (line 97).
  - `Stage-Metronome-Dark.dc.html:104` (metronome on, its settings open, line 123); light: `Stage-Metronome-Light.dc.html:75` (line 94).
- **Not this component's job:** no store, no API, no Tauri, no `tips` import: every value comes in as a prop, every action goes out as a callback, and the page wiring sends the commands. It doesn't compute the count row (CountRow does, from the props this row passes it untouched) or draw the metronome popover (#509). No tooltip wiring (integration). No keyboard shortcuts (`%` Accomp, `.` metronome, `\` Panic, `?` help mode are the app's global key handler).

## API (Component station)

### Props

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `acmp` | `boolean` | `false` | Accomp is on. From `transport.acmp`. |
| `metronome` | `boolean` | `false` | The metronome is on. From `metronome.on`. |
| `metronomeSettingsDisabled` | `boolean` | `true` | The Metronome caret is disabled: `true` until the metronome popover (#509) is built (Stage.md D32). |
| `metronomeSettingsOpen` | `boolean` | `false` | The metronome popover is open (app-side state, #509). |
| `metronomePopoverId` | `string \| undefined` | — | The popover's element id, for the caret's `aria-controls`. |
| `unison` | `boolean` | `false` | Unison is engaged. From `transport.unison` (latched or held by a pedal, D8). |
| `help` | `boolean` | `false` | Help mode is on. From the app-side `tips.help` (`app/src/lib/tooltip/tip.svelte.ts`); not in `AppState`. |
| `count` | `ComponentProps<typeof CountRow>` | — (required) | Every CountRow prop, as one object, passed straight through (D3). Its fields and where each is fed from are CountRow's spec (`app/src/ui/CountRow/SPEC.md`): the `surface.clock` fields, `now`, `receivedMs`, the `transport` fields and the `styleSettings` timings. |
| `width` | `number \| undefined` | — | A fixed width in px. Default: the row fills its container's width (the Stage gives it 1392). Stories set `1392` (D7). |

`ComponentProps` is Svelte's (`import type { ComponentProps } from 'svelte'`), applied to `../CountRow/CountRow.svelte`; no other type is imported.

### Events

| Callback | Fires when | Payload | The wiring sends |
|---|---|---|---|
| `onacmp` | the Accomp lamp toggles | `(on: boolean)` the new state | `toggleAcmp` |
| `onmetronome` | the Metronome lamp toggles | `(on: boolean)` the new state | `toggleMetronome` |
| `onmetronomesettings` | the Metronome caret is pressed, when enabled | `()` | opens or closes the popover (#509); nothing until then |
| `onunison` | the Unison lamp toggles | `(on: boolean)` the new state | `toggleUnison` (D8) |
| `onpanic` | Panic is pressed | `()` | `panic` |
| `onhelp` | ? is pressed | `(on: boolean)` the new help mode, `!help` | no AppCmd: `tips.toggleHelp()` (app-side) |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Children

Left to right, in DOM order:

| # | Child | Props passed (from SectionRow's props) | Callback → SectionRow callback → command |
|---|---|---|---|
| 1 | `LampButton` (Accomp) | `{ label: 'Accomp', code: 'ACMP', on: acmp, size: 'md' }` | `ontoggle(on)` → `onacmp(on)` → `toggleAcmp` |
| 2 | `CountRow` | `{...count}` | — (no callbacks) |
| 3 | `MetronomeSplit` | `{ on: metronome, settingsDisabled: metronomeSettingsDisabled, settingsOpen: metronomeSettingsOpen, popoverId: metronomePopoverId }` | `ontoggle(on)` → `onmetronome(on)` → `toggleMetronome`; `onopensettings()` → `onmetronomesettings()` |
| 4 | `LampButton` (Unison) | `{ label: 'Unison', on: unison, size: 'md' }` | `ontoggle(on)` → `onunison(on)` → `toggleUnison` |
| 5 | `Button` (Panic) | `{ label: 'Panic', variant: 'md' }` | `onpress()` → `onpanic()` → `panic` |
| 6 | `Button` (?) | `{ label: '?', variant: 'icon', on: help, name: 'Help mode' }` | `onpress()` → `onhelp(!help)` → `tips.toggleHelp()` |

- Button's `variant: 'md'` is the off face with padding 0 14 (Panic measures 63 × 32 on the board); `variant: 'icon'` is 32 × 32. `on` on a Button sets its `aria-pressed` and, when true, the lamp face (the Help board draws the ? lit; the primitives lane's Button `On` story). These Button prop names are assumed from the kit; the Button spec's names win (D10).
- None of the children's faces, sizes, type or focus rings are restated here: they are LampButton's, Button's, MetronomeSplit's and CountRow's.

### Visual rules

- **Root:** `role="toolbar"`, `aria-label="Switches and helpers"`. `display: flex`, `align-items: center`, `gap: var(--space-24)` (24), height `var(--control-height)` (32), `box-sizing: border-box`, no padding, no background, no border, `white-space: nowrap`, no wrapping (`flex-wrap: nowrap`), no `overflow` (focus rings stay visible). Width: `width` px when given, else `100%` of its container.
- **Accomp:** `flex: none`, its natural width (127 on the board).
- **Middle slot:** CountRow sits in the flexible middle: a plain `div` with `flex: 1 1 0`, `min-width: 0`, `height: var(--control-height)`, `display: flex`, `justify-content: center`, `align-items: center`, holding CountRow. It takes all the width the two ends leave (894 on the board: x 175 to 1069 in the row's 24…1416), and CountRow centres and fits its own content inside it (CountRow's spec). The slot draws nothing.
- **Right group:** a `span`, `flex: none`, `margin-left: auto`, `display: flex`, `align-items: center`, `gap: var(--space-8)` (8): MetronomeSplit, Unison, Panic, ?. On the board it runs from x 1093 to 1416 (323 = 128 + 8 + 76 + 8 + 63 + 8 + 32); its children size themselves.
- **Box on the board:** `24,68 1392×32` at 1440 × 900 (8 below the app bar).
- **Tokens used:** `--space-24`, `--space-8`, `--control-height`. No colours of its own.
- **States drawn by:** the children only (lamps lit or off, the caret's states, the ? lit in help mode, the count row's moments). The row itself has one look.
- **Type:** none of its own.
- **Contrast:** none of its own: every text is a child's, and each child's spec lists its pairs.
- **Motion:** none of its own; the count row moves with its `now` prop (CountRow).

### Accessibility

- **Role and name:** the root is a `toolbar` named "Switches and helpers". Inside it: button "Accomp ACMP" (`aria-pressed`), CountRow's `status` (its `aria-label` reads the row out), the group "Metronome" with buttons "Metronome" (`aria-pressed`) and "Metronome settings", button "Unison" (`aria-pressed`), button "Panic" (D9), button "Help mode" (`aria-pressed` = `help`).
- **Keyboard:** Tab order is the DOM order above: Accomp, Metronome, Metronome settings, Unison, Panic, Help mode (the count row has nothing focusable). Space and Enter press the focused control (the children's). No roving tabindex and no arrow-key movement inside the toolbar (D5).
- **Tooltip ids** (each element carries a `data-tip` once wired at integration, not here): Accomp `transport.acmp`; count row `display.position` (CountRow's); Metronome `metronome.on`; Metronome caret `metronome.settings` (new, C5); Unison `transport.unison`; Panic `transport.panic`; ? `app.help`.
- **Launchkey:** Accomp is Shift + encoder page ▼ (`transport.acmp`'s `launchkey`); the Metronome, its caret, Unison, Panic and ? have no Launchkey mapping.

## Stories (Story station)

Title `Components/SectionRow`, `layout: 'centered'`, every story with `width: 1392` (the board's row) and `parameters.shots = { viewport: { width: 1440, height: 900 } }` so the row fits the shot (Stage.md D39 adds per-story viewports to `shots.ts`). Every story renders in dark and light (the toolbar theme).

**Fixtures:** the `count` objects come from `app/src/ui/CountRow/CountRow.fixtures.ts` (CountRow's folder, axiom 12): `boardCount`, CountRow's `Board` args (Stage.md › Board fixture: the clock at bar 3 beat 3 of Main B at 104 BPM, Main C landing after a queued fill, `now` = `receivedMs` = 10000), and `stoppedCount`, CountRow's `Stopped` args (the same with `running` false in the clock and the transport, `transport.main` 1, `sectionBars` null, nothing queued, landing or armed, `syncStart` false). If CountRow names them differently, these stories use its names (D3).

**Controls:** SectionRow's own props, each a control; `count` is one object control (CountRow's own controls live in its stories); the six callbacks are actions.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ width: 1392, acmp: true, metronome: false, unison: false, help: false, count: boardCount }`, `parameters.shots.mask: ['[data-shot-mask="when"]']` | the Stage board's row at the fixture moment: Accomp lit, the count row, Metronome off with its caret disabled, Unison off, Panic, ? off | `Board-{dark,light}.png` (Stage 24,68 1392×32), When masked (D4) | toolbar named "Switches and helpers"; "Accomp ACMP" `aria-pressed="true"`; "Metronome", "Unison" and "Help mode" `aria-pressed="false"`; "Metronome settings" `aria-disabled="true"`; the `status` inside has `aria-label` "Beat 3 of 4, bar 3 of 4. Main B playing, Main C next. The fill lands after bar 3."; click "Accomp ACMP" → `onacmp` called with `false`; click "Metronome" → `onmetronome` with `true`; click "Metronome settings" → `onmetronomesettings` not called; click "Unison" → `onunison` with `true`; click "Panic" → `onpanic` called once with no arguments; click "Help mode" → `onhelp` with `true`; Tab from before the row visits Accomp ACMP, Metronome, Metronome settings, Unison, Panic, Help mode in that order |
| `MetronomeOpen` | `Board`'s args with `{ metronome: true, metronomeSettingsDisabled: false, metronomeSettingsOpen: true, metronomePopoverId: 'metro-pop' }`, When masked | Metronome lit, its caret open | `MetronomeOpen-{dark,light}.png` (Stage-Metronome 24,68 1392×32) | "Metronome" `aria-pressed="true"`; "Metronome settings" `aria-expanded="true"`, `aria-controls="metro-pop"`; click it → `onmetronomesettings` called once with no arguments |
| `HelpMode` | `Board`'s args with `{ help: true }`, When masked | the ? lit | `HelpMode-{dark,light}.png` (Stage-Help 24,68 1392×32) | "Help mode" `aria-pressed="true"`; click → `onhelp` called with `false` |
| `AccompOff` | `Board`'s args with `{ acmp: false }` | Accomp off | — (no board draws it off) | "Accomp ACMP" `aria-pressed="false"`; click → `onacmp` called with `true` |
| `UnisonOn` | `Board`'s args with `{ unison: true }` | Unison lit | — (no board draws it lit) | "Unison" `aria-pressed="true"`; click → `onunison` called with `false` |
| `Stopped` | `Board`'s args with `{ count: stoppedCount }` | the stopped count row; everything else as `Board` | — (no board is stopped) | the `status` has `aria-label` "Stopped on Main B."; the right group's buttons still respond (click "Panic" → `onpanic` called once) |
| `Focused` | `Board`'s args, `parameters: { pseudo: { focusVisible: true } }` | every control's focus ring, none clipped by the row | — | — |

Crop positions are `board x,y w×h` in the 1440×900 renders (`docs/design/push/png/<Board>-Dark.png` and `-Light.png`), the same box in dark and light. Each of the three crops shows "fill after bar 4" where the story shows "fill after bar 3" (D4).

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- SectionRow` passes: each cropped story's screenshot is 1392 × 32 and scores at most 0.02 (or the Inspect agent judges the difference render noise), and axe finds no violation on any story.
- Only listed tokens are used; no colours, no literal sizes.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · The ? is a `Button` with `on`.** It is a switch (`aria-pressed` = help mode) that the Help board draws lit and the Stage board draws as the plain off face with a `--t2` label, which is Button's (the primitives lane's `On` story), not LampButton's `--m` off label.
- **D2 · Callback names.** One callback per control, named for what it switches (`onacmp`, `onmetronome`, `onunison`, `onhelp`, each with the new state; `onmetronomesettings`, `onpanic` with none), since a row with three toggles can't use a single `ontoggle`.
- **D3 · `count` is one prop.** SectionRow passes CountRow's props through untouched as one object typed `ComponentProps<typeof CountRow>`, so CountRow's props can change without touching this spec, and its stories take their `count` from CountRow's fixtures file.
- **D4 · The When mask (lane L3, Stage.md D5, D39).** Every cropped story uses the board fixture's moment, so the count row reads "fill after bar 3" where the boards read "fill after bar 4"; the When item (`data-shot-mask="when"`, CountRow's) is masked in `parameters.shots`, and until `shots.ts` supports masks the one-glyph difference (about 100 px of 44,544) is far under 0.02.
- **D5 · Toolbar without arrow keys.** The board's `role="toolbar"` stays, but every control is in the Tab order and arrows are left to the global handler (Stage.md D36), so the toolbar has no roving tabindex.
- **D6 · Help mode has no command.** `onhelp(on)` is wired to `tips.toggleHelp()` in the app; the row only reports the press.
- **D7 · `width`.** The row fills its container by default (the Stage), and takes a `width` so its stories render at the board's 1392px with no wrapper markup (axiom 4).
- **D8 · Unison.** The lamp shows `transport.unison` (engaged, latched or pedal-held, as the kit says) and a press sends `toggleUnison` (the latched switch) whatever the payload; the lamp then follows the state.
- **D9 · Panic's name.** "Panic", the visible label; the board's "Panic: all notes off" is the `transport.panic` tooltip's body.
- **D10 · Child prop names.** Button's `variant`, `on`, `name`, `onpress`, LampButton's `join` and MetronomeSplit's props are assumed from the kit and the primitives lane's crops; the children's specs win where they differ.

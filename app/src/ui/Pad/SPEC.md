# Pad

## Identity (all stations)

- **Kind:** primitive
- **Built from:** —
- **Purpose:** One of the Launchkey's 16 pads on screen: it shows what the pad does and how the hardware lights it (playing, queued, armed, on, not there), and pressing it does what the hardware pad does.
- **Boards:** `Stage-Dark.dc.html:345-351` (the pad button) and `:495-516` (the data script: faces per state); light: `Stage-Light.dc.html:321-327`, `:471-492`. The 16 pads sit at `x = 730 + 74·col`, `y = 634` (pads 1–8) and `708` (9–16), 68 × 68, on both boards.
- **Not this component's job:** no store, no API, no Tauri, no timer. It doesn't know the page, the transport or the LED clock's anchors: the parent passes `caption`, `family`, `level`, `anim` and the clock's reading `led`, and acts on `onpress`. No group line (PadGrid draws those). No Shift layer (the Launchkey firmware keeps Shift + pad for itself). No tooltip (wired at integration, D12). No long press.

## API (Component station)

### Props

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `index` | `number` (an integer 1–16) | — | The pad's number, 1–16 (pads 1–8 the top row, 9–16 the bottom). Shown as the numeral and used in the accessible name. |
| `caption` | `string` | `''` | The words on the pad, as given. The Sections captions carry a no-break space (U+00A0) between a name and its numeral or letter (`Intro I`, `Ending III`, `Main A`), so only whole words wrap (PadGrid's `SECTIONS` table holds them). Fallback pages pass `pads.pads[i].label` as the state gives it, upper case included (`OTS 1`, `BANK -`). An empty caption is an Absent pad, whatever `level` says. |
| `family` | `'intro' \| 'main' \| 'ending' \| 'brk' \| 'fill' \| 'util' \| 'start'` | `'util'` | The pad's family on its page, which picks its hue and its lit states (States below). `util`: Sync Start, Auto Fill, Tap, Sync Stop. `start`: Start / Stop (pad 16). Ignored when `fallback` is true. |
| `level` | `'off' \| 'dim' \| 'bright'` | `'dim'` | The pad's light level (`pads.pads[i].level`): off = not available, dim = available, bright = playing or on. |
| `anim` | `'solid' \| 'flash' \| 'pulse'` | `'solid'` | How the hardware animates a bright pad (`pads.pads[i].anim`): flash = queued, pulse = armed (or, on a Main, the landing). Ignored unless `level` is `bright`. |
| `led` | `number` | `0` | The LED clock's reading in beats at this moment (`ledBeats(...)`, below). Drives the flash and the pulse; the component never runs a timer. |
| `fallback` | `boolean` | `false` | Drawn with the fallback look of a pad page whose own spec hasn't landed (Racks, Chord, Multi Pads, Setup, and Racks while Sound is held or latched): every pad is a utility pad, its lit states in `--t`, and `bright` + `solid` is Playing, never On. |
| `disabled` | `boolean` | `false` | The pad does nothing now (`pads.pads[i].action` is null): `aria-disabled="true"`, stays focusable, no `onpress`. The look doesn't change (D5). |

`PadProps` (these props plus `onpress`) is exported from `Pad.svelte`'s module script; `PadFamily`, `PadLevel` and `PadAnim` (the three unions above) are exported from `app/src/ui/Pad/face.ts`.

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onpress` | a `pointerdown` with `button === 0` on the pad's `<button>` (the listener is on the button itself); or a `click` whose `event.detail === 0` (keyboard activation: Enter or Space). A `click` with `detail` ≥ 1 (the one a pointer press ends with) is ignored, so one press calls it once. Never while `disabled`. | `()` |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Pure functions (`app/src/ui/Pad/led.ts`, `app/src/ui/Pad/face.ts`, `app/src/ui/Pad/a11y.ts`)

Unit-tested with vitest (`app/src/ui/Pad/led.test.ts`, `face.test.ts`). No rounding: the results are floats, and tests compare with `toBeCloseTo(x, 6)`.

**`ledBeats(clock, now, receivedMs): number`** (`led.ts`). `clock` is `{ atMs: number; tempo: number; ledAnchorMs: number; ledAnchorBeats: number }` (the `surface.clock` fields it reads; an object with more fields is accepted). `now` and `receivedMs` are the page's clock in ms (`performance.now()` now, and when the state arrived).

```
t   = clock.atMs + (now − receivedMs)                                    // session ms, now
led = clock.ledAnchorBeats + (t − clock.ledAnchorMs) · clock.tempo / 60000
```

The LED clock runs whether or not the band plays (it is free-running: app-api.md › surface.clock).

| `clock` | `now` | `receivedMs` | Result |
|---|---|---|---|
| board fixture: atMs 10000, tempo 104, ledAnchorMs 10000, ledAnchorBeats 0.25 | 10000 | 10000 | `0.25` |
| the same | 10288.461538 (half a beat later: 30000 / 104 ms) | 10000 | `0.75` |
| atMs 10000, tempo 120, ledAnchorMs 9000, ledAnchorBeats 2 | 5000 | 4500 | `5` (t = 10500; 2 + 1500 · 120 / 60000) |

**`padK(level, anim, led): number`** (`led.ts`): the hardware's brightness factor `k` (app-api.md › Pad), with `frac(x) = x − floor(x)` and `tri(p) = p < 0.5 ? 2p : 2 − 2p`:

| `level`, `anim` | `k` |
|---|---|
| `off` (any `anim`) | `0` |
| `dim` (any `anim`) | `0.18` |
| `bright` + `solid` | `1` |
| `bright` + `flash` | `1` while `frac(led) < 0.5`, else `0.18` |
| `bright` + `pulse` | `0.25 + 0.75 · tri(frac(led / 2))` |

Worked values (each one a test):

| Call | Result | Why |
|---|---|---|
| `padK('bright', 'flash', 0.25)` | `1` | the board fixture: frac 0.25 < 0.5, the flash at full |
| `padK('bright', 'flash', 0.75)` | `0.18` | frac 0.75: the flash's low phase (18%) |
| `padK('bright', 'flash', 0.5)` | `0.18` | frac 0.5 is not < 0.5 |
| `padK('bright', 'flash', 1.25)` | `1` | frac 0.25 |
| `padK('bright', 'pulse', 0)` | `0.25` | frac(0) = 0, tri 0 |
| `padK('bright', 'pulse', 0.5)` | `0.625` | frac(0.25) = 0.25, tri 0.5 |
| `padK('bright', 'pulse', 1)` | `1` | frac(0.5) = 0.5, tri 1 |
| `padK('bright', 'pulse', 1.5)` | `0.625` | frac(0.75) = 0.75, tri 0.5 |
| `padK('bright', 'pulse', 2)` | `0.25` | frac(1) = 0 |
| `padK('bright', 'solid', 0.75)` | `1` | |
| `padK('dim', 'flash', 0.25)` | `0.18` | |
| `padK('off', 'pulse', 1)` | `0` | |

**`padState({ caption, level, anim, family, fallback }): PadStateName`** (`face.ts`; `PadStateName` is `'absent' | 'idle' | 'playing' | 'next' | 'armed' | 'on'`, exported); the first rule that matches wins:

1. `caption` is `''` or `level` is `off` → `absent`.
2. `level` is `dim` → `idle`.
3. `bright` + `flash` → `next`.
4. `bright` + `pulse` → `next` when `family` is `main` and `fallback` is false (the Main a fill lands on, Stage.md D11), else `armed`.
5. `bright` + `solid` → `on` when `family` is `util` and `fallback` is false (a lit utility pad: Sync Stop, Auto Fill, Tap on the beat; Stage.md D11), else `playing`.

**`padFace(state): 'off' | 'solid' | 'waiting' | 'on'`** (`face.ts`): the kit's face for a state (kit › Faces, D41), which `data-face` carries: `idle` and `absent` → `off`; `playing` → `solid`; `next` and `armed` → `waiting`; `on` → `on`. One test per state.

**`padHue(family, fallback): 'intro' | 'main' | 'ending' | 'brk' | 'fill' | 't' | 'ok'`** (`face.ts`): the hue token name (without `--`) of the pad's hue parts: `fallback` → `t`; `util` → `t`; `start` → `ok`; any other family → itself. Tests: `padHue('main', false)` is `main`; `padHue('main', true)` is `t`; `padHue('util', false)` is `t`; `padHue('start', false)` is `ok`.

**`padBlink(state, level, anim, led): number`** (`face.ts`): the factor the animated parts are drawn at: `padK(level, anim, led)` when `state` is `next` or `armed`, else `1`. (A Next from a flash blinks between 1 and 0.18; a Next from a Main's landing pulse, and an Armed pad, swell between 0.25 and 1.) Examples: `padBlink('next', 'bright', 'flash', 0.75)` is `0.18`; `padBlink('playing', 'bright', 'solid', 0.75)` is `1`; `padBlink('idle', 'dim', 'solid', 0.25)` is `1`.

**`padLabel({ caption, index, state, family, fallback }): string`** (`face.ts`): the accessible name, `<words><suffix> (pad <index>)`. `<words>` is `caption` with each U+00A0 turned into a plain space, or `Unused` when `caption` is empty. `<suffix>` by state:

| State | `<suffix>` |
|---|---|
| `idle` | `, stopped` for the `start` family when `fallback` is false; else nothing |
| `playing` | `, running` for the `start` family when `fallback` is false; else `, playing` |
| `next` | `, queued` |
| `armed` | `, armed` |
| `on` | `, on` |
| `absent` | the first that matches: (1) `caption` is `''` → nothing; (2) `fallback` is false → `, not in this style`; (3) else → `, not available` |

| Arguments (caption, index, state, family / fallback) | Result |
|---|---|
| `Intro I`, 1, idle, intro | `Intro I (pad 1)` |
| `Main B`, 10, playing, main | `Main B, playing (pad 10)` |
| `Main C`, 11, next, main | `Main C, queued (pad 11)` |
| `Intro III`, 3, absent, intro | `Intro III, not in this style (pad 3)` |
| `Sync Start`, 4, idle, util | `Sync Start (pad 4)` |
| `Sync Start`, 4, armed, util | `Sync Start, armed (pad 4)` |
| `Auto Fill`, 8, on, util | `Auto Fill, on (pad 8)` |
| `Start / Stop`, 16, playing, start | `Start / Stop, running (pad 16)` |
| `Start / Stop`, 16, idle, start | `Start / Stop, stopped (pad 16)` |
| `''`, 1, absent, fallback | `Unused (pad 1)` |
| `''`, 5, absent, intro (not fallback) | `Unused (pad 5)` |
| `MAN BASS`, 9, absent, fallback | `MAN BASS, not available (pad 9)` |
| `OTS 1`, 9, playing, fallback | `OTS 1, playing (pad 9)` |

**`PAD_CONTRAST_EXCLUDE: string[]`** (`a11y.ts`): the selectors of the pad elements that fail AA today (Contrast › Known failures), exactly the nine listed there, in that order. Pad's stories, and every story that renders pads (PadGrid, PadBank, FullBand), pass it as `parameters.a11y.context.exclude`, so axe checks everything else.

### Visual rules

- **Tokens used:** `--btn`, `--t`, `--t2`, `--d`, `--intro`, `--main`, `--ending`, `--brk`, `--fill`, `--ok`, `--lamp`, `--lamp-ink`, `--solid-ink`, `--focus`, `--radius`, `--font-sans`, `--font-mono`, `--text-11`, `--text-14`, `--weight-regular`, `--weight-medium`, `--space-2`, `--space-4`, `--space-6`, `--space-14`, `--line-width`, `--focus-offset`, and six new tokens: `--pad-index-dark`, `--glow-mix`, `--bar-glow-mix`, `--armed-ring`, `--solid-ink-index`, `--solid-ink-bar` (below).
- **New tokens** (from kit.md › Tokens to add; they land with the tokens contract PR (the orchestrator), not in this component's PR, which never edits `app/src/ui/tokens/*` or `contrast.test.ts`):

  | Token | Dark (`dark.css`) | Light (`light.css`) | Used for |
  |---|---|---|---|
  | `--pad-index-dark` | `var(--grey-18)`, a new palette step `--grey-18: #2e2e2e` in `palette.css` | `var(--line)` | the numeral of an Absent pad |
  | `--glow-mix` | `25%` | `0%` | the Playing and Armed 12px glow's strength |
  | `--bar-glow-mix` | `60%` | `0%` | the Next bar's 6px glow's strength |
  | `--armed-ring` | `0px` | `1px` | the Armed pad's inset ring (light only) |
  | `--solid-ink-index` | `55%` | `70%` | the Playing numeral's `--solid-ink` strength |
  | `--solid-ink-bar` | `45%` | `60%` | the Playing bar's `--solid-ink` strength |

- **Component geometry** (axiom 2): declared once as custom properties on the button (the component's root), and every rule below uses these names, never the numbers:

  | Property | Value | What |
  |---|---|---|
  | `--pad-size` | `68px` | the pad's width and height |
  | `--pad-caption-leading` | `16px` | the caption's line-height |
  | `--pad-caption-tracking` | `-0.2px` | the caption's letter-spacing |
  | `--pad-numeral-top` | `5px` | the numeral's top offset |
  | `--pad-numeral-right` | `7px` | the numeral's right offset, and the bar's left and right insets |
  | `--pad-numeral-leading` | `14px` | the numeral's line-height |
  | `--pad-glow` | `12px` | the Playing and Armed glow's blur |
  | `--pad-bar-glow` | `6px` | the Next bar's glow's blur |

- **Local hue properties** (set on the root, not theme tokens): `--pad-hue` is `var(--<padHue>)`, set by one `[data-hue='<name>']` rule for each of `intro main ending brk fill t ok` (never an inline colour); `--k` is the number `padBlink(...)`, written inline as `style="--k: <number>"`.
- **Size:** `width` and `height` `var(--pad-size)` (fixed; PadGrid lays out 68px columns), `box-sizing: border-box`, radius `--radius`, border `--line-width` solid, transparent unless the state gives a colour; padding `0 var(--space-4) var(--space-14)` (0 top, 4px left and right, 14px bottom); `overflow: hidden` (nothing draws outside the pad).
- **Layout:** a `<button type="button">`, `position: relative`, `display: flex`, `flex-direction: column`, `justify-content: flex-end`, `align-items: flex-start`, `text-align: left`, `margin: 0`: the caption sits bottom-left, its last line's box ending 14px above the pad's inner bottom edge. Three children:
  - **Caption** (`data-part="caption"`, a `span` with `display: block`): `--font-sans` `--text-14`, `--weight-medium`, `line-height: var(--pad-caption-leading)`, `letter-spacing: var(--pad-caption-tracking)`, case as given, `font-variant-numeric: tabular-nums`. It wraps only between words (`white-space: normal`, `word-break: normal`), with `overflow-wrap: break-word` so that a single word wider than the 58px line (68 − 2 border − 8 padding) breaks instead of overflowing (D10). **At most two lines:** `max-height: calc(2 * var(--pad-caption-leading))` and `overflow: hidden` on the caption, so a caption that would take three or more lines shows its first two and the rest is cut at the second line's bottom (D11). Two lines end at the padding box's y 52 and start at y 20, clear of the numeral (y 5–19). The Sections captions fit one line (`Ending III` included) or wrap at their plain space: `Sync` over `Start`, `Sync` over `Stop`, `Start /` over `Stop`.
  - **Numeral** (`data-part="numeral"`, `aria-hidden="true"`): `position: absolute`, `top: var(--pad-numeral-top)`, `right: var(--pad-numeral-right)` (from the padding box), `--font-mono` `--text-11`, `--weight-regular`, `line-height: var(--pad-numeral-leading)`. Text: the `index` (`1`…`16`); `NEXT` in the Next state; `ARMED` in the Armed state.
  - **Bar** (`data-part="bar"`, `aria-hidden="true"`), rendered only in the Playing, Next and Armed states: `position: absolute`, `left` and `right` `var(--pad-numeral-right)`, `bottom: var(--space-6)`, `height: var(--space-2)`, `border-radius: calc(var(--space-2) / 2)`.
- **States** (`data-state` on the button, from `padState`; `data-face` is `padFace(state)`). "hue" is `var(--pad-hue)`; "hue ×k" is `color-mix(in srgb, var(--pad-hue) calc(var(--k) * 100%), transparent)`:

  | State (`data-face`) | Fill | Border | Glow (`box-shadow`) | Caption | Numeral | Bar |
  |---|---|---|---|---|---|---|
  | `idle` (`off`) | `--btn` | transparent | none | hue; `--t2` for `util`, `start` and every fallback pad | `--d`, the index | none |
  | `absent` (`off`) | `--btn` | transparent | none | `--d` | `--pad-index-dark`, the index | none |
  | `playing` (`solid`) | hue | hue | `0 0 var(--pad-glow) color-mix(in srgb, var(--pad-hue) var(--glow-mix), transparent)` | `--solid-ink` | `color-mix(in srgb, var(--solid-ink) var(--solid-ink-index), transparent)`, the index | `color-mix(in srgb, var(--solid-ink) var(--solid-ink-bar), transparent)`, no glow |
  | `next` (`waiting`) | `--btn` | hue ×k | none | `--t` | hue, `NEXT` | hue, `opacity: var(--k)`, its own glow `0 0 var(--pad-bar-glow) color-mix(in srgb, var(--pad-hue) var(--bar-glow-mix), transparent)` (it fades with the bar) |
  | `armed` (`waiting`) | `--btn` | hue ×k | `0 0 var(--pad-glow) color-mix(in srgb, var(--pad-hue) calc(var(--glow-mix) * var(--k)), transparent), inset 0 0 0 var(--armed-ring) <hue ×k>` | `--t` | hue, `ARMED` | hue, `opacity: var(--k)`, no glow |
  | `on` (`on`) | `--lamp` | `--lamp` | none | `--lamp-ink` | `--lamp-ink`, the index | none |

  - **Hue per family** (`data-hue` on the button, from `padHue`): Intro `intro`, Main `main`, Ending `ending`, Break `brk`, Fill `fill`; utility `t` (a utility pad's Next and Armed are `--t` outlines with `NEXT` / `ARMED` in `--t`; its idle caption `--t2`); Start / Stop `ok` (running is the Playing state in `--ok` green, Stage.md D11); every fallback pad `t` (its Playing state is a `--t` block with a `--solid-ink` caption).
  - **Light:** `--glow-mix` and `--bar-glow-mix` are 0%, so no glow draws (a 0% mix is transparent); `--armed-ring` is 1px, so an Armed pad shows a second 1px ring inside its border instead of a glow. In dark `--armed-ring` is 0px (no ring) and the glows draw.
  - **`--k`** applies only in the Next and Armed states (`padBlink` is 1 in the others): the border colour, the bar with its glow, and on Armed also the outer glow and the inset ring. The fill, the caption and the `NEXT` / `ARMED` word stay at full strength, so the pad stays readable through the whole flash (D3).
  - **Disabled:** no colour changes (D5).
  - **Keyboard focus:** a `--line-width` outline in `--focus`, `--focus-offset` outside the pad, on `:focus-visible` only.
  - **Hover and active:** nothing changes (kit D35). Cursor `pointer`; `default` when `disabled`.
- **Contrast (AA 4.5:1).** Ratios computed from today's `dark.css` / `light.css` / `palette.css`.
  - Pass in both themes, already in `contrast.test.ts`: `--t2` on `--btn` (idle utility and fallback captions; 12.45 / 11.03), `--lamp-ink` on `--lamp` (On caption; 13.24 / 4.97).
  - Pass in both themes, add to `contrast.test.ts` with the tokens PR: `--t` on `--btn` ("Next and Armed caption (Pad)"; 18.10 / 14.70; Button lists the same pair), `--solid-ink` on `--main` (11.16 / 4.99), on `--ending` (4.96 / 5.87), on `--fill` (7.27 / 5.09), on `--ok` (11.16 / 4.99), on `--t` (21.00 / 18.88) ("Playing caption (Pad)").
  - **Known failures (owner question O-contrast).** These fail AA in at least one theme today; until the owner answers, axe skips exactly these elements (`PAD_CONTRAST_EXCLUDE`, in this order):

    | # | Selector | Pair | Dark | Light |
    |---|---|---|---|---|
    | 1 | `[data-state="idle"][data-hue="intro"] > [data-part="caption"]` | `--intro` on `--btn` | 8.35 | **3.41** |
    | 2 | `[data-state="idle"][data-hue="main"] > [data-part="caption"]` | `--main` on `--btn` | 9.62 | **3.88** |
    | 3 | `[data-state="idle"][data-hue="ending"] > [data-part="caption"]` | `--ending` on `--btn` | **4.27** | 4.57 |
    | 4 | `[data-state="idle"][data-hue="brk"] > [data-part="caption"]` | `--brk` on `--btn` | **3.86** | 5.04 |
    | 5 | `[data-state="idle"][data-hue="fill"] > [data-part="caption"]` | `--fill` on `--btn` | 6.26 | **3.96** |
    | 6 | `[data-state="playing"][data-hue="intro"] > [data-part="caption"]` | `--solid-ink` on `--intro` | 9.69 | **4.37** |
    | 7 | `[data-state="playing"][data-hue="brk"] > [data-part="caption"]` | `--solid-ink` on `--brk` | **4.48** | 6.47 |
    | 8 | `[data-state="absent"] > [data-part="caption"]` | `--d` on `--btn` (D6) | **2.14** | **1.71** |
    | 9 | `[data-part="numeral"]` | the `--d` index (2.14 / 1.71), `--pad-index-dark`, the solid-ink mixes, and `NEXT` / `ARMED` in the hue on `--btn` (as rows 1–5) (D6) | — | — |

- **Motion:** the Next flash and the Armed pulse are `--k` from `padBlink(state, level, anim, led)`; a new `led` redraws them. No CSS animation, no transition, no timer (axiom 10).

### Accessibility

- **Role and name:** a native `button`, named by `aria-label` = `padLabel(...)` (examples above). The numeral and the bar are `aria-hidden`; the caption is inside the button but the `aria-label` names it.
- **States:** `aria-disabled="true"` when `disabled` (it stays in the tab order). No `aria-pressed`: a pad is an action, not a switch; its state is in the name.
- **Keyboard:** Tab focuses it; Enter or Space presses it (`onpress`, via the `click` with `detail === 0`).
- **Test hooks:** on the button `data-face` (`off|solid|waiting|on`, from `padFace`), `data-state` (`idle|absent|playing|next|armed|on`, from `padState`), `data-hue` (`intro|main|ending|brk|fill|t|ok`) and `data-k` (`String(Math.round(k * 1000) / 1000)` of `padBlink`: `1`, `0.18`, `0.625`…); on its parts `data-part="caption" | "numeral" | "bar"`.
- **Tooltip id** (wired at integration, not here; D12), by place; every key exists in `tooltips.ts`:

  | Sections pad | Key | Sections pad | Key |
  |---|---|---|---|
  | 1 Intro I | `section.intro1` | 9 Main A | `section.main_a` |
  | 2 Intro II | `section.intro2` | 10 Main B | `section.main_b` |
  | 3 Intro III | `section.intro3` | 11 Main C | `section.main_c` |
  | 4 Sync Start | `transport.sync_start` | 12 Main D | `section.main_d` |
  | 5 Ending I | `section.ending1` | 13 Break | `section.break` |
  | 6 Ending II | `section.ending2` | 14 Tap | `tempo.tap` |
  | 7 Ending III | `section.ending3` | 15 Sync Stop | `transport.sync_stop` |
  | 8 Auto Fill | `transport.auto_fill` | 16 Start / Stop | `transport.start_stop` |

  A fallback pad carries its page's key: `padpage.racks` (Racks, and Racks while Sound is held or latched), `padpage.chord`, `padpage.multi_pads`, `padpage.setup`; a fallback pad with an empty caption `launchkey.unused`. PadGrid's `pages.ts` exports both tables and `padTip(...)`, which picks one.
- **Launchkey:** the pad itself (DAW port notes 96–103 for pads 1–8, 112–119 for pads 9–16).

### From the state

Pad is placed by PadGrid inside PadBank; the page wiring feeds PadBank (PadBank's SPEC has the wiring's table). For one pad `i` (0-based):

| Prop / callback | From `AppState` / what the wiring sends |
|---|---|
| `index` | `i + 1` |
| `caption` | Sections (`pads.page` is `sections` and `surface.layer.type` is not `sound`): PadGrid's `SECTIONS[i].caption`; otherwise `pads.pads[i].label` |
| `family` | Sections: `SECTIONS[i].family`; otherwise `util` (ignored under `fallback`) |
| `level`, `anim` | `pads.pads[i].level`, `pads.pads[i].anim`; except pad 16 on Sections while `transport.running` is false, whose `level` PadGrid passes as `dim` when the state says `dim` or `bright`, and `off` when the state says `off` (PadGrid D3) |
| `led` | `ledBeats(surface.clock, now, receivedMs)`, computed by the wiring once per animation frame |
| `fallback` | `pads.page !== 'sections' \|\| surface.layer.type === 'sound'` |
| `disabled` | `pads.pads[i].action === null` |
| `onpress` | `send(pads.pads[i].action)`; then, when the wiring's own `soundLatched` flag is set (the screen's Sound latch, Stage.md D18; LampRow's `onhold` sets it), `send({ type: 'setLayer', layer: { type: 'none' } })` (PadBank's SPEC) |

## Stories (Story station)

- **Title:** `Primitives/Pad`
- **Layout:** `centered` (real size, 68 × 68).
- **Meta:** `args: { onpress: fn() }`; `parameters: { a11y: { context: { exclude: PAD_CONTRAST_EXCLUDE } } }` (from `./a11y`), for every story.

Every story renders in dark and light. A crop box is `Stage x,y w×h`, the same box on `Stage-Dark.png` and `Stage-Light.png`. **Controls:** `index` a number control (`min: 1`, `max: 16`, `step: 1`); `caption` text; `family`, `level`, `anim` selects of their unions; `led` a number control (`step: 0.05`), so the flash and the pulse can be scrubbed; `fallback` and `disabled` booleans.

Plays press the pad with `fireEvent.pointerDown(button, { button: 0, pointerId: 1 })` and send a pointer click as `fireEvent.click(button, { detail: 1 })`; a keyboard press is `button.focus()` then `userEvent.keyboard('{Enter}')` or `('{ }')`.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ index: 1, caption: 'Intro I', family: 'intro', level: 'dim', anim: 'solid', led: 0.25 }` | pad 1 as the board draws it: idle, caption in the Intro hue, `--d` numeral | `Board-{dark,light}.png`: Stage 730,634 68×68 | the button named `Intro I (pad 1)` has `data-face="off"`, `data-state="idle"`, `data-hue="intro"`, `data-k="1"`; numeral text `1`; no `[data-part="bar"]` |
| `Playing` | `{ index: 10, caption: 'Main B', family: 'main', level: 'bright', anim: 'solid', led: 0.25 }` | the solid Main block, ink caption, faded numeral and bar, the 12px glow (dark) | `Playing-{dark,light}.png`: Stage 804,708 68×68 | named `Main B, playing (pad 10)`; `data-face="solid"`, `data-state="playing"`, `data-hue="main"`; a `[data-part="bar"]` exists |
| `Next` | `{ index: 11, caption: 'Main C', family: 'main', level: 'bright', anim: 'flash', led: 0.25 }` | the queued pad at the flash's full phase: green border, `NEXT`, green bar with its glow (dark) | `Next-{dark,light}.png`: Stage 878,708 68×68 | named `Main C, queued (pad 11)`; `data-face="waiting"`, `data-state="next"`, `data-k="1"`; numeral text `NEXT` |
| `NextDim` | `Next`'s args with `led: 0.75` | the flash's low phase: border and bar at 18% | — (the board draws the full phase only) | `data-face="waiting"`, `data-state="next"`, `data-k="0.18"`; numeral text `NEXT` |
| `Landing` | `{ index: 11, caption: 'Main C', family: 'main', level: 'bright', anim: 'pulse', led: 1 }` | a Main pulsing as the landing, drawn Next, at the pulse's peak | — (no board draws it) | `data-state="next"`, `data-k="1"`; named `Main C, queued (pad 11)` |
| `Armed` | `{ index: 1, caption: 'Intro I', family: 'intro', level: 'bright', anim: 'pulse', led: 1 }` | the armed Intro at the pulse's peak: border, glow (dark) or inner ring (light), `ARMED`, bar | — (the Stage board has no armed pad) | named `Intro I, armed (pad 1)`; `data-face="waiting"`, `data-state="armed"`, `data-k="1"`; numeral text `ARMED` |
| `ArmedLow` | `Armed`'s args with `led: 0` | the pulse's low point: border, ring and bar at 25% | — | `data-state="armed"`, `data-k="0.25"` |
| `ArmedMid` | `Armed`'s args with `led: 0.5` | the pulse halfway up | — | `data-state="armed"`, `data-k="0.625"` |
| `Absent` | `{ index: 3, caption: 'Intro III', family: 'intro', level: 'off', anim: 'solid', led: 0.25 }` | a section the style lacks: `--d` caption, darker numeral | `Absent-{dark,light}.png`: Stage 878,634 68×68 | named `Intro III, not in this style (pad 3)`; `data-face="off"`, `data-state="absent"` |
| `UtilityIdle` | `{ index: 4, caption: 'Sync Start', family: 'util', level: 'dim', anim: 'solid', led: 0.25 }` | a utility pad: `--t2` caption, `Sync` over `Start` | `UtilityIdle-{dark,light}.png`: Stage 952,634 68×68 | named `Sync Start (pad 4)`; `data-state="idle"`, `data-hue="t"` |
| `On` | `{ index: 8, caption: 'Auto Fill', family: 'util', level: 'bright', anim: 'solid', led: 0.25 }` | a lit utility switch in the lamp face | — (the board's Auto Fill is off) | named `Auto Fill, on (pad 8)`; `data-face="on"`, `data-state="on"`; no `[data-part="bar"]` |
| `Running` | `{ index: 16, caption: 'Start / Stop', family: 'start', level: 'bright', anim: 'solid', led: 0.25 }` | Start / Stop running: the Playing state in `--ok` | `Running-{dark,light}.png`: Stage 1248,708 68×68 | named `Start / Stop, running (pad 16)`; `data-face="solid"`, `data-state="playing"`, `data-hue="ok"` |
| `Stopped` | `Running`'s args with `level: 'dim'` | Start / Stop idle (what PadGrid passes while the band is stopped) | — (the board's band runs) | named `Start / Stop, stopped (pad 16)`; `data-face="off"`, `data-state="idle"` |
| `Fallback` | `{ index: 10, caption: 'STOP ACMP', fallback: true, level: 'dim', anim: 'solid', led: 0.25 }` | a fallback-page pad: the state's label in `--t2`, `STOP` over `ACMP` | — (no board draws the fallback look, Stage.md D33) | named `STOP ACMP (pad 10)`; `data-state="idle"`, `data-hue="t"` |
| `FallbackPlaying` | `{ index: 9, caption: 'OTS 1', fallback: true, level: 'bright', anim: 'solid', led: 0.25 }` | the fallback Playing state: a `--t` block with a `--solid-ink` caption, not the lamp | — | named `OTS 1, playing (pad 9)`; `data-face="solid"`, `data-state="playing"`, `data-hue="t"` |
| `FallbackNext` | `{ index: 15, caption: 'STORE', fallback: true, level: 'bright', anim: 'flash', led: 0.25 }` | the fallback Next state, in `--t` | — | `data-face="waiting"`, `data-state="next"`, `data-hue="t"`; numeral text `NEXT` |
| `FallbackArmed` | `{ index: 4, caption: 'PAD 4', fallback: true, level: 'bright', anim: 'pulse', led: 1 }` | the fallback Armed state, in `--t` | — | `data-face="waiting"`, `data-state="armed"`, `data-hue="t"`; numeral text `ARMED` |
| `Unused` | `{ index: 1, caption: '', fallback: true, level: 'off', anim: 'solid', disabled: true, led: 0.25 }` | an unused pad: blank, numeral only | — | named `Unused (pad 1)`; `data-state="absent"`, `aria-disabled="true"`; pointer down → `onpress` not called; focus it and press Enter → not called |
| `LongCaption` | `{ index: 2, caption: 'FINGERED', fallback: true, level: 'dim', anim: 'solid', led: 0.25 }` | a single word wider than the pad breaks inside it, never past its edge | — | — (jsdom can't measure text; the Inspect station looks at it) |
| `ThreeLines` | `{ index: 12, caption: 'KBD TR RESET ALL', fallback: true, level: 'dim', anim: 'solid', led: 0.25 }` | a caption longer than two lines: the first two show, the rest is cut, the numeral stays clear (D11) | — | — (as `LongCaption`) |
| `Presses` | `Board`'s args | — | — | pointer down with `button: 0` → `onpress` called once, with no arguments; `fireEvent.click(button, { detail: 1 })` → still once; focus it, press Enter → twice; press Space → three times; `fireEvent.pointerDown(button, { button: 2, pointerId: 1 })` → still three |
| `Focused` | `Board`'s args; `parameters: { pseudo: { focusVisible: true } }` | the focus ring | — | — |

What jsdom can't check (the hues, `color-mix`, the glows, the `--k` fades, the caption's wrapping and two-line cut) is covered by the cropped stories' screenshots (`npm run shots -- Pad`) and, for the uncropped states, by the Inspect station's look at the stories in both themes.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `led.test.ts` and `face.test.ts` assert every worked value above: `ledBeats`, `padK`, each `padState` rule, each `padFace` mapping, `padHue`, `padBlink`, and each `padLabel` example; `PAD_CONTRAST_EXCLUDE` equals the nine selectors of Known failures, in order.
- `npm run shots -- Pad` passes: each cropped story's screenshot is 68 × 68 and scores at most 0.02 against its crop, and axe finds no violation (with `PAD_CONTRAST_EXCLUDE` excluded).
- Only listed tokens are used; no inline colours; the only literal sizes are the eight component geometry properties' values, declared once on the root.
- svelte-check and lint pass on the folder.

## Decisions

- D1. The family is a prop (`family`), not a raw hue, because it decides the lit states (a utility pad lights as On, a Main's pulse is the landing), which a hue alone can't.
- D2. An empty caption is Absent whatever its level, on every page, so an unused pad always reads blank and is named `Unused`.
- D3. The flash and the pulse fade only the border, the bar (with its glow) and, on Armed, the outer glow and the inner ring; the fill, the caption and `NEXT` / `ARMED` stay at full strength so the pad stays readable (the kit's "border and bar", made exact).
- D4. A Main's landing pulse is drawn in the Next state with the pulse curve (0.25 to 1), as the hardware pulses it; a flashing Next uses the flash (1 / 0.18).
- D5. A disabled pad (null `action`) keeps the look its light gives and only gains `aria-disabled`, because the pad mirrors the hardware LED, which doesn't change when a pad does nothing.
- D6. The numerals and the Absent caption are treated as exempt from AA (the numeral is `aria-hidden` decoration whose number is in the name; the Absent caption marks a section the style lacks); they are listed under Known failures so the owner's O-contrast answer covers them, and axe skips them until then.
- D7. `onpress` fires on pointer down (and on keyboard activation), not on release, so a screen pad acts when hit, like the hardware, and Tap tempo stays in time.
- D8. The accessible name puts the state before the pad number (`Main B, playing (pad 10)`) instead of the kit's literal `Main B (pad 10), playing`, and adds `, on`, `, running` / `, stopped` (Start / Stop, as the board's labels) and `, not available` (fallback pages).
- D9. Every lit utility pad is On (the lamp face), Tap on the beat included, so one rule covers the family; on fallback pages bright + solid is Playing (Stage.md D33).
- D10. A caption breaks a single word only when that word alone is wider than the pad (`overflow-wrap: break-word`), and the pad clips anything past its edge; every caption that fits wraps on whole words only.
- D11. A caption shows at most two lines (cut at the second line's bottom, no ellipsis), because a third line would run into the numeral; no Sections caption and no dev-mock label needs more than two.
- D12. Tooltips are wired at integration (LampButton's rule): Pad renders no `data-tip` and takes no `tip` prop; PadGrid's `padTip` names the key per pad.
- D13. `data-face` carries the kit's face vocabulary (`off`, `solid`, `waiting`, `on`; lead decision 8, so Stage.md Check 8 reads pad 10 `solid` and pad 11 `waiting`), and the pad's own six states go in `data-state`.
- D14. The pad's fixed geometry (68px, the numeral's 5 / 7 offsets, the 16 and 14 line-heights, the −0.2px tracking, the 12 and 6 glow blurs) is declared once as `--pad-*` custom properties on the root; shared values use `scale.css` tokens (`--space-2`, `--space-4`, `--space-6`, `--space-14`, `--line-width`).
- D15. The Armed pulse's other phases are their own stories (`ArmedLow`, `ArmedMid`) rather than a play that changes args, so each story is a fixed render vitest and the screenshots can both check.

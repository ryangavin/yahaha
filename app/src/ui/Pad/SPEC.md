# Pad

## Identity (all stations)

- **Kind:** primitive
- **Built from:** —
- **Purpose:** One of the Launchkey's 16 pads on screen: it shows what the pad does and how the hardware lights it (playing, queued, armed, on, not there), and pressing it does what the hardware pad does.
- **Boards:** `Stage-Dark.dc.html:345-351` (the pad button) and `:495-516` (the data script: faces per state); light: `Stage-Light.dc.html:321-327`, `:471-492`. The 16 pads sit at `x = 730 + 74·col`, `y = 634` (pads 1–8) and `708` (9–16), 68 × 68, on both boards.
- **Not this component's job:** no store, no API, no Tauri, no timer. It doesn't know the page, the transport or the LED clock's anchors: the parent passes `caption`, `family`, `level`, `anim` and the clock's reading `led`, and acts on `onpress`. No group line (PadGrid draws those). No Shift layer (the Launchkey firmware keeps Shift + pad for itself). No tooltip (wired at integration). No long press.

## API (Component station)

### Props

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `index` | `number` | — | The pad's number, 1–16 (pads 1–8 the top row, 9–16 the bottom). Shown as the numeral and used in the accessible name. |
| `caption` | `string` | `''` | The words on the pad, as given. The Sections captions carry a no-break space (U+00A0) between a name and its numeral or letter (`Intro I`, `Ending III`, `Main A`), so only whole words wrap (PadGrid's `SECTIONS` table holds them). Fallback pages pass `pads.pads[i].label` as the state gives it, upper case included (`OTS 1`, `BANK -`). An empty caption is an Absent pad, whatever `level` says. |
| `family` | `'intro' \| 'main' \| 'ending' \| 'brk' \| 'fill' \| 'util' \| 'start'` | `'util'` | The pad's family on its page, which picks its hue and its lit faces (Faces below). `util`: Sync Start, Auto Fill, Tap, Sync Stop. `start`: Start / Stop (pad 16). Ignored when `fallback` is true. |
| `level` | `'off' \| 'dim' \| 'bright'` | `'dim'` | The pad's light level (`pads.pads[i].level`): off = not available, dim = available, bright = playing or on. |
| `anim` | `'solid' \| 'flash' \| 'pulse'` | `'solid'` | How the hardware animates a bright pad (`pads.pads[i].anim`): flash = queued, pulse = armed (or, on a Main, the landing). Ignored unless `level` is `bright`. |
| `led` | `number` | `0` | The LED clock's reading in beats at this moment (`ledBeats(...)`, below). Drives the flash and the pulse; the component never runs a timer. |
| `fallback` | `boolean` | `false` | Drawn with the fallback face of a pad page whose own spec hasn't landed (Racks, Chord, Multi Pads, Setup, and Racks while Sound is held or latched): every pad is a utility pad, its lit faces in `--t`, and `bright` + `solid` is Playing, never On. |
| `disabled` | `boolean` | `false` | The pad does nothing now (`pads.pads[i].action` is null): `aria-disabled="true"`, stays focusable, no `onpress`. The face doesn't change (D5). |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onpress` | the primary pointer button goes down on the pad (`pointerdown` with `button === 0`), or the pad is activated from the keyboard (Enter or Space, i.e. a `click` with `detail === 0`); never twice for one press (the `click` that follows a `pointerdown` is ignored); never while `disabled` | `()` |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Pure functions (`app/src/ui/Pad/led.ts`, `app/src/ui/Pad/face.ts`)

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

**`padFace({ caption, level, anim, family, fallback }): 'absent' | 'idle' | 'playing' | 'next' | 'armed' | 'on'`** (`face.ts`); the first rule that matches wins:

1. `caption` is `''` or `level` is `off` → `absent`.
2. `level` is `dim` → `idle`.
3. `bright` + `flash` → `next`.
4. `bright` + `pulse` → `next` when `family` is `main` and `fallback` is false (the Main a fill lands on, Stage.md D11), else `armed`.
5. `bright` + `solid` → `on` when `family` is `util` and `fallback` is false (a lit utility pad: Sync Stop, Auto Fill, Tap on the beat; Stage.md D11), else `playing`.

**`padHue(family, fallback): 'intro' | 'main' | 'ending' | 'brk' | 'fill' | 't' | 'ok'`** (`face.ts`): the hue token name (without `--`) of the pad's hue parts: `fallback` → `t`; `util` → `t`; `start` → `ok`; any other family → itself.

**`padBlink(face, level, anim, led): number`** (`face.ts`): the factor the animated parts are drawn at: `padK(level, anim, led)` when `face` is `next` or `armed`, else `1`. (A Next from a flash blinks between 1 and 0.18; a Next from a Main's landing pulse, and an Armed pad, swell between 0.25 and 1.) Examples: `padBlink('next', 'bright', 'flash', 0.75)` is `0.18`; `padBlink('playing', 'bright', 'solid', 0.75)` is `1`; `padBlink('idle', 'dim', 'solid', 0.25)` is `1`.

**`padLabel({ caption, index, face, family, fallback }): string`** (`face.ts`): the accessible name, `<words><state> (pad <index>)`. `<words>` is `caption` with each U+00A0 turned into a plain space, or `Unused` when `caption` is empty. `<state>`:

| Face | `<state>` |
|---|---|
| `idle` | nothing; `start` family (not fallback): `, stopped` |
| `playing` | `, playing`; `start` family (not fallback): `, running` |
| `next` | `, queued` |
| `armed` | `, armed` |
| `on` | `, on` |
| `absent` | `, not in this style` when `fallback` is false; `, not available` when `fallback` is true and there is a caption; nothing when the caption is empty |

| Arguments (caption, index, face, family / fallback) | Result |
|---|---|
| `Intro I`, 1, idle, intro | `Intro I (pad 1)` |
| `Main B`, 10, playing, main | `Main B, playing (pad 10)` |
| `Main C`, 11, next, main | `Main C, queued (pad 11)` |
| `Intro III`, 3, absent, intro | `Intro III, not in this style (pad 3)` |
| `Sync Start`, 4, idle, util | `Sync Start (pad 4)` |
| `Sync Start`, 4, armed, util | `Sync Start, armed (pad 4)` |
| `Auto Fill`, 8, on, util | `Auto Fill, on (pad 8)` |
| `Start / Stop`, 16, playing, start | `Start / Stop, running (pad 16)` |
| `Start / Stop`, 16, idle, start | `Start / Stop, stopped (pad 16)` |
| `''`, 1, absent, fallback | `Unused (pad 1)` |
| `MAN BASS`, 9, absent, fallback | `MAN BASS, not available (pad 9)` |
| `OTS 1`, 9, playing, fallback | `OTS 1, playing (pad 9)` |

### Visual rules

- **Tokens used:** `--btn`, `--t`, `--t2`, `--d`, `--intro`, `--main`, `--ending`, `--brk`, `--fill`, `--ok`, `--lamp`, `--lamp-ink`, `--solid-ink`, `--focus`, `--radius`, `--font-sans`, `--font-mono`, `--text-11`, `--text-14`, `--weight-regular`, `--weight-medium`, `--space-4`, `--line-width`, `--focus-offset`, and the kit's new tokens (kit.md › Tokens to add, landed by the orchestrator): `--pad-index-dark`, `--glow-mix`, `--bar-glow-mix`, `--armed-ring`, `--solid-ink-index`, `--solid-ink-bar`.
- **Local custom properties** (set by the component on its own root, not theme tokens): `--pad-hue` is `var(--<padHue>)`, set by one `[data-hue='<name>']` rule for each of `intro main ending brk fill t ok` (never an inline colour); `--k` is the number `padBlink(...)`, written inline as `style="--k: <number>"`.
- **Size:** 68 × 68 (fixed; PadGrid lays out 68px columns), `box-sizing: border-box`, radius `--radius` (4px), border `--line-width` (1px) solid, transparent unless the face gives a colour; padding 0 top, `--space-4` (4px) left and right, 14px bottom; `overflow: hidden` (a caption too long for the pad is cut at the pad's edge, never drawn outside it).
- **Layout:** a `<button type="button">`, `position: relative`, `display: flex`, column, `justify-content: flex-end`, `align-items: flex-start`, `text-align: left`: the caption sits bottom-left, its last line's box ending 14px above the pad's inner bottom edge. Three children:
  - **Caption** (`data-part="caption"`): DM Sans (`--font-sans`) `--text-14` (14px), `--weight-medium` (500), line-height 16px, letter-spacing −0.2px, case as given, tabular numerals. It wraps only between words (`white-space: normal`, `word-break: normal`), with `overflow-wrap: break-word` so that a single word wider than the 58px line (68 − 2 border − 8 padding) breaks instead of overflowing (D10). The Sections captions fit one line (`Ending III` included) or wrap at their plain space: `Sync` over `Start`, `Sync` over `Stop`, `Start /` over `Stop`.
  - **Numeral** (`data-part="numeral"`, `aria-hidden="true"`): `position: absolute`, top 5px, right 7px (from the padding box), JetBrains Mono (`--font-mono`) `--text-11` (11px), `--weight-regular`, line-height 14px. Text: the `index` (`1`…`16`); `NEXT` on the Next face; `ARMED` on the Armed face.
  - **Bar** (`data-part="bar"`, `aria-hidden="true"`), rendered only on the Playing, Next and Armed faces: `position: absolute`, left 7px, right 7px, bottom 6px, height 2px, radius 1px.
- **Faces** (`data-face` on the button). "hue" is `var(--pad-hue)`; "hue ×k" is `color-mix(in srgb, var(--pad-hue) calc(var(--k) * 100%), transparent)`:

  | Face | Fill | Border | Glow (`box-shadow`) | Caption | Numeral | Bar |
  |---|---|---|---|---|---|---|
  | `idle` | `--btn` | transparent | none | hue; `--t2` for `util`, `start` and every fallback pad | `--d`, the index | none |
  | `absent` | `--btn` | transparent | none | `--d` | `--pad-index-dark`, the index | none |
  | `playing` | hue | hue | `0 0 12px color-mix(in srgb, var(--pad-hue) var(--glow-mix), transparent)` | `--solid-ink` | `color-mix(in srgb, var(--solid-ink) var(--solid-ink-index), transparent)`, the index | `color-mix(in srgb, var(--solid-ink) var(--solid-ink-bar), transparent)`, no glow |
  | `next` | `--btn` | hue ×k | none | `--t` | hue, `NEXT` | hue, `opacity: var(--k)`, its own glow `0 0 6px color-mix(in srgb, var(--pad-hue) var(--bar-glow-mix), transparent)` (it fades with the bar) |
  | `armed` | `--btn` | hue ×k | `0 0 12px color-mix(in srgb, var(--pad-hue) calc(var(--glow-mix) * var(--k)), transparent), inset 0 0 0 var(--armed-ring) <hue ×k>` | `--t` | hue, `ARMED` | hue, `opacity: var(--k)`, no glow |
  | `on` | `--lamp` | `--lamp` | none | `--lamp-ink` | `--lamp-ink`, the index | none |

  - **Hue per family** (`data-hue` on the button, from `padHue`): Intro `intro`, Main `main`, Ending `ending`, Break `brk`, Fill `fill`; utility `t` (a utility pad's Next and Armed are `--t` outlines with `NEXT` / `ARMED` in `--t`; its idle caption `--t2`); Start / Stop `ok` (running is the Playing face in `--ok` green, Stage.md D11); every fallback pad `t` (its Playing face is a `--t` block with a `--solid-ink` caption).
  - **Light:** `--glow-mix` and `--bar-glow-mix` are 0%, so no glow draws (a 0% mix is transparent); `--armed-ring` is 1px, so an Armed pad shows a second 1px ring inside its border instead of a glow. In dark `--armed-ring` is 0px (no ring) and the glows draw.
  - **`--k`** applies only on the Next and Armed faces (`padBlink` is 1 on the others): the border colour, the bar with its glow, and on Armed also the outer glow and the inset ring. The fill, the caption and the `NEXT` / `ARMED` word stay at full strength, so the pad stays readable through the whole flash (D3).
  - **Disabled:** no colour changes (D5).
  - **Keyboard focus:** a `--line-width` (1px) outline in `--focus`, `--focus-offset` (2px) outside the pad, on `:focus-visible` only.
  - **Hover and active:** nothing changes (kit D35). Cursor `pointer`; `default` when `disabled`.
- **Contrast (AA 4.5:1, `tokens/contrast.test.ts`):** captions on `--btn`: `--intro`, `--main`, `--ending`, `--brk`, `--fill`, `--t2`, `--t`; `--solid-ink` on `--intro`, `--main`, `--ending`, `--brk`, `--fill`, `--ok`, `--t`; `--lamp-ink` on `--lamp`; the 11px `NEXT` / `ARMED` word: `--intro`, `--main`, `--ending`, `--brk`, `--fill`, `--t` on `--btn`. Exempt (D6): the Absent caption (`--d`) and the numerals in `--d`, `--pad-index-dark` and the solid-ink mixes. With today's hue values several pairs fall short (dark: `--ending` 4.27 and `--brk` 3.86 on `--btn`, `--solid-ink` on `--brk` 4.48; light: `--intro` 3.41, `--main` 3.88, `--fill` 3.96 on `--btn`, `--solid-ink` on `--intro` 4.37); that is the tokens' to fix, not this component's.
- **Motion:** the Next flash and the Armed pulse are `--k` from `padBlink(face, level, anim, led)`; a new `led` redraws them. No CSS animation, no transition, no timer (axiom 10).

### Accessibility

- **Role and name:** a native `button`, named by `aria-label` = `padLabel(...)` (examples above). The numeral and the bar are `aria-hidden`; the caption is inside the button but the `aria-label` names it.
- **States:** `aria-disabled="true"` when `disabled` (it stays in the tab order). No `aria-pressed`: a pad is an action, not a switch; its state is in the name.
- **Keyboard:** Tab focuses it; Enter or Space presses it (`onpress`).
- **Test hooks:** on the button `data-face` (`idle|absent|playing|next|armed|on`), `data-hue` (`intro|main|ending|brk|fill|t|ok`) and `data-k` (`String(Math.round(k * 1000) / 1000)` of `padBlink`: `1`, `0.18`, `0.625`…); on its parts `data-part="caption" | "numeral" | "bar"`.
- **Tooltip id** (wired at integration, not here), by place; every key exists in `tooltips.ts`:

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

  A fallback pad carries its page's key: `padpage.racks` (Racks, and Racks while Sound is held or latched), `padpage.chord`, `padpage.multi_pads`, `padpage.setup`; a fallback pad with an empty caption `launchkey.unused`. PadGrid's `pages.ts` exports both tables.
- **Launchkey:** the pad itself (DAW port notes 96–103 for pads 1–8, 112–119 for pads 9–16).

### From the state

Pad is placed by PadGrid inside PadBank; the page wiring feeds PadBank (PadBank's SPEC has the wiring's table). For one pad `i` (0-based):

| Prop / callback | From `AppState` / what the wiring sends |
|---|---|
| `index` | `i + 1` |
| `caption` | Sections (`pads.page` is `sections` and `surface.layer.type` is not `sound`): PadGrid's `SECTIONS[i].caption`; otherwise `pads.pads[i].label` |
| `family` | Sections: `SECTIONS[i].family`; otherwise `util` (ignored under `fallback`) |
| `level`, `anim` | `pads.pads[i].level`, `pads.pads[i].anim`; pad 16 on Sections is passed `dim` while `transport.running` is false (PadGrid D3) |
| `led` | `ledBeats(surface.clock, now, receivedMs)`, computed by the wiring once per animation frame |
| `fallback` | `pads.page !== 'sections' \|\| surface.layer.type === 'sound'` |
| `disabled` | `pads.pads[i].action === null` |
| `onpress` | `send(pads.pads[i].action)`; while Sound is latched on screen, then `send({ type: 'setLayer', layer: { type: 'none' } })` (Stage.md D18; PadBank's SPEC) |

## Stories (Story station)

- **Title:** `Primitives/Pad`
- **Layout:** `centered` (real size, 68 × 68).

Every story renders in dark and light. `onpress` is an action (`fn()`) in every story. A crop box is `Stage x,y w×h`, the same box on `Stage-Dark.png` and `Stage-Light.png`. Controls: every prop is a control (`family`, `level`, `anim` as selects; `led` a number with step 0.05, so the flash and the pulse can be scrubbed).

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ index: 1, caption: 'Intro I', family: 'intro', level: 'dim', anim: 'solid', led: 0.25 }` | pad 1 as the board draws it: idle, caption in the Intro hue, `--d` numeral | `Board-{dark,light}.png`: Stage 730,634 68×68 | the button named `Intro I (pad 1)` has `data-face="idle"`, `data-hue="intro"`, `data-k="1"`; numeral text `1`; no `[data-part="bar"]` |
| `Playing` | `{ index: 10, caption: 'Main B', family: 'main', level: 'bright', anim: 'solid', led: 0.25 }` | the solid Main block, ink caption, faded numeral and bar, the 12px glow (dark) | `Playing-{dark,light}.png`: Stage 804,708 68×68 | named `Main B, playing (pad 10)`; `data-face="playing"`, `data-hue="main"`; a `[data-part="bar"]` exists |
| `Next` | `{ index: 11, caption: 'Main C', family: 'main', level: 'bright', anim: 'flash', led: 0.25 }` | the queued pad at the flash's full phase: green border, `NEXT`, green bar with its glow (dark) | `Next-{dark,light}.png`: Stage 878,708 68×68 | named `Main C, queued (pad 11)`; `data-face="next"`, `data-k="1"`; numeral text `NEXT` |
| `NextDim` | `Next`'s args with `led: 0.75` | the flash's low phase: border and bar at 18% | — (the board draws the full phase only) | `data-face="next"`, `data-k="0.18"`; numeral text `NEXT` |
| `Landing` | `{ index: 11, caption: 'Main C', family: 'main', level: 'bright', anim: 'pulse', led: 1 }` | a Main pulsing as the landing, drawn Next, at the pulse's peak | — (no board draws it) | `data-face="next"`, `data-k="1"`; named `Main C, queued (pad 11)` |
| `Armed` | `{ index: 1, caption: 'Intro I', family: 'intro', level: 'bright', anim: 'pulse', led: 1 }` | the armed Intro at the pulse's peak: border, glow (dark) or inner ring (light), `ARMED`, bar | — (the Stage board has no armed pad) | named `Intro I, armed (pad 1)`; `data-face="armed"`, `data-k="1"`; numeral text `ARMED`; with `led` set to 0, `data-k="0.25"`; to 0.5, `data-k="0.625"` |
| `Absent` | `{ index: 3, caption: 'Intro III', family: 'intro', level: 'off', anim: 'solid', led: 0.25 }` | a section the style lacks: `--d` caption, darker numeral | `Absent-{dark,light}.png`: Stage 878,634 68×68 | named `Intro III, not in this style (pad 3)`; `data-face="absent"` |
| `UtilityIdle` | `{ index: 4, caption: 'Sync Start', family: 'util', level: 'dim', anim: 'solid', led: 0.25 }` | a utility pad: `--t2` caption, `Sync` over `Start` | `UtilityIdle-{dark,light}.png`: Stage 952,634 68×68 | named `Sync Start (pad 4)`; `data-face="idle"`, `data-hue="t"` |
| `On` | `{ index: 8, caption: 'Auto Fill', family: 'util', level: 'bright', anim: 'solid', led: 0.25 }` | a lit utility switch in the lamp face | — (the board's Auto Fill is off) | named `Auto Fill, on (pad 8)`; `data-face="on"`; no `[data-part="bar"]` |
| `Running` | `{ index: 16, caption: 'Start / Stop', family: 'start', level: 'bright', anim: 'solid', led: 0.25 }` | Start / Stop running: the Playing face in `--ok` | `Running-{dark,light}.png`: Stage 1248,708 68×68 | named `Start / Stop, running (pad 16)`; `data-face="playing"`, `data-hue="ok"` |
| `Stopped` | `Running`'s args with `level: 'dim'` | Start / Stop idle (what PadGrid passes while the band is stopped) | — (the board's band runs) | named `Start / Stop, stopped (pad 16)`; `data-face="idle"` |
| `Fallback` | `{ index: 10, caption: 'STOP ACMP', fallback: true, level: 'dim', anim: 'solid', led: 0.25 }` | a fallback-page pad: the state's label in `--t2`, `STOP` over `ACMP` | — (no board draws the fallback face, Stage.md D33) | named `STOP ACMP (pad 10)`; `data-face="idle"`, `data-hue="t"` |
| `FallbackPlaying` | `{ index: 9, caption: 'OTS 1', fallback: true, level: 'bright', anim: 'solid', led: 0.25 }` | the fallback Playing face: a `--t` block with a `--solid-ink` caption, not the lamp | — | named `OTS 1, playing (pad 9)`; `data-face="playing"`, `data-hue="t"` |
| `FallbackNext` | `{ index: 15, caption: 'STORE', fallback: true, level: 'bright', anim: 'flash', led: 0.25 }` | the fallback Next face, in `--t` | — | `data-face="next"`, `data-hue="t"`; numeral text `NEXT` |
| `FallbackArmed` | `{ index: 4, caption: 'PAD 4', fallback: true, level: 'bright', anim: 'pulse', led: 1 }` | the fallback Armed face, in `--t` | — | `data-face="armed"`, `data-hue="t"`; numeral text `ARMED` |
| `Unused` | `{ index: 1, caption: '', fallback: true, level: 'off', anim: 'solid', disabled: true, led: 0.25 }` | an unused pad: blank, numeral only | — | named `Unused (pad 1)`; `data-face="absent"`, `aria-disabled="true"`; a click → `onpress` not called; focus it and press Enter → not called |
| `LongCaption` | `{ index: 2, caption: 'FINGERED', fallback: true, level: 'dim', anim: 'solid', led: 0.25 }` | a single word wider than the pad breaks inside it, never past its edge | — | — |
| `Presses` | `Board`'s args | — | — | a pointer down with the primary button → `onpress` called once; the click that follows → still once; focus it, press Enter → twice; press Space → three times; a pointer down with `button: 2` → still three |
| `Focused` | `Board`'s args; `parameters: { pseudo: { focusVisible: true } }` | the focus ring | — | — |

What jsdom can't check (the hues, `color-mix`, the glows, the `--k` fades, the caption's wrapping) is covered by the cropped stories' screenshots (`npm run shots -- Pad`) and, for the uncropped faces, by the Inspect station's look at the stories in both themes.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `led.test.ts` and `face.test.ts` assert every worked value above: `ledBeats`, `padK`, each `padFace` rule, `padHue`, `padBlink`, and each `padLabel` example.
- `npm run shots -- Pad` passes: each cropped story's screenshot is 68 × 68 and scores at most 0.02 against its crop, and axe finds no violation (the numeral and the Absent caption exempt, D6).
- Only listed tokens are used; no inline colours; the sizes that aren't tokens are only those in Visual rules (68 × 68, 14px bottom padding, numeral top 5 / right 7, bar left and right 7 / bottom 6 / height 2 / radius 1, line-heights 16 and 14, letter-spacing −0.2px, glow radii 12 and 6).
- svelte-check and lint pass on the folder.

## Decisions

- D1. The family is a prop (`family`), not a raw hue, because it decides the lit faces (a utility pad lights as On, a Main's pulse is the landing), which a hue alone can't.
- D2. An empty caption is Absent whatever its level, on every page, so an unused pad always reads blank and is named `Unused`.
- D3. The flash and the pulse fade only the border, the bar (with its glow) and, on Armed, the outer glow and the inner ring; the fill, the caption and `NEXT` / `ARMED` stay at full strength so the pad stays readable (the kit's "border and bar", made exact).
- D4. A Main's landing pulse is drawn on the Next face with the pulse curve (0.25 to 1), as the hardware pulses it; a flashing Next uses the flash (1 / 0.18).
- D5. A disabled pad (null `action`) keeps the face its light gives and only gains `aria-disabled`, because the face mirrors the hardware LED, which doesn't change when a pad does nothing.
- D6. The numerals and the Absent caption are exempt from AA (decoration and an inactive component): the numeral is `aria-hidden` and its number is in the name; if axe reports them, the story excludes `[data-part="numeral"]` and `[data-face="absent"] [data-part="caption"]` with `parameters.a11y.context.exclude`.
- D7. `onpress` fires on pointer down (and on keyboard activation), not on release, so a screen pad acts when hit, like the hardware, and Tap tempo stays in time.
- D8. The accessible name puts the state before the pad number (`Main B, playing (pad 10)`) instead of the kit's literal `Main B (pad 10), playing`, and adds `, on`, `, running` / `, stopped` (Start / Stop, as the board's labels) and `, not available` (fallback pages).
- D9. Every lit utility pad is On (the lamp face), Tap on the beat included, so one rule covers the family; on fallback pages bright + solid is Playing (Stage.md D33).
- D10. A caption breaks a single word only when that word alone is wider than the pad (`overflow-wrap: break-word`), and the pad clips anything past its edge; every caption that fits wraps on whole words only.

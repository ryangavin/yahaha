# Button

## Identity (all stations)

- **Kind:** primitive
- **Built from:** — (uses the shared `longpress` action, `app/src/ui/actions/longpress`, spec `app/src/ui/actions/longpress/SPEC.md`)
- **Purpose:** Does one thing when pressed (Panic, Stop, a page step, a One Touch), in the plain button face, and shows when that thing is the one chosen, switched on or waiting.
- **Boards:**
  - `Stage-Dark.dc.html:115` (the Metronome ▾ caret, joined), `:117` (Panic), `:118` (? help), `:131`, `:133` (◀ ▶ styles), `:138-141` (One Touch 1–4, 2 chosen), `:285` (Panel, the lamp row's master button), `:299-300` (knob page ▲ ▼), `:334-335` (pad bank ▲ ▼), `:366-386` (transport and tempo: Start / Stop with its bar, Stop, Reset | Fade, Fill ▲ | Fill ▼, Tempo + / −, Style tempo); light: `Stage-Light.dc.html:91`, `:93`, `:94`, `:107`, `:109`, `:114-117`, `:261`, `:275-276`, `:310-311`, `:342-362`.
  - `Stage-Help-Dark.dc.html:126` (? lit in help mode); light: `Stage-Help-Light.dc.html:97`.
  - `Stage-Metronome-Dark.dc.html:123` (the caret with its popover open); light: `Stage-Metronome-Light.dc.html:94`.
  - `LibrarySounds-Dark.dc.html:266` (Audition, disabled); light: `LibrarySounds-Light.dc.html:253`.
- **Not this component's job:** no store, no API, no Tauri. It doesn't know what it does: the parent passes the face and acts on `onpress`, `onhold`, `onlongpress`, `onlongrelease`. No tooltip (wired at integration). No timer: long press is the shared `use:longpress` action; repeat-while-held is the parent's (`onhold` with `app/src/lib/tempoHold.ts`). Not a toggle with its own state (LampButton is): every face comes from props. Not a group: One Touch, the Fills pair and the Metronome split are their parents' (`OneTouch`, `TransportColumn`, `MetronomeSplit`), which set gaps and group roles. Not a text button (band sends, sound cells) and not a tab (`ChosenTabs`).

## API (Component station)

### Props

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `label` | `string` | `''` | The word or character on the face ("Panic", "Stop", "1", "?"). May be empty when `symbol` is set. |
| `symbol` | `'prev' \| 'next' \| 'up' \| 'down' \| 'plus' \| 'minus' \| 'caret' \| undefined` | — | A glyph after the label (or alone): `prev` ◀ (U+25C0), `next` ▶ (U+25B6), `up` ▲ (U+25B2), `down` ▼ (U+25BC), `plus` + (U+002B), `minus` − (U+2212), `caret` ▾ (U+25BE). Drawn per Visual rules, `aria-hidden`. |
| `size` | `'icon' \| 'md' \| 'band' \| 'pair' \| 'cell' \| 'caret'` | `'md'` | `icon` 32 × 32, centred (◀ ▶ ▲ ▼, ?, One Touch 1–4). `md` 32 tall, 14px side padding, width from the label (Panic). `band` 88 × 32, label left-aligned, 8px side padding (the transport and tempo column). `pair` 41 × 32, centred, no padding, 13px (Reset, Fade, Fill ▲ ▼). `cell` 32 tall, fills its container's width, centred, 13px (Panel in the lamp row). `caret` 20 × 32, centred, the ▾ in `--m` (Metronome settings). |
| `compact` | `boolean` | `false` | 13px label instead of 14 in `icon`, `md` and `band` (Style tempo, Start / Stop, Audition). `pair` and `cell` are always 13. |
| `strong` | `boolean` | `false` | Label in `--t` at medium weight on the off face (Start / Stop). |
| `bar` | `boolean \| undefined` | — | The running bar (Start / Stop). Undefined: no bar and normal padding. `false`: the bar's room is kept (padding `0 4px 3px 8px` in `band`) but nothing drawn. `true`: the 2px `--ok` bar with the `--bg` glow is drawn. |
| `on` | `boolean` | `false` | The lamp face: switched on (help mode's ?, Fade while fading or holding). |
| `chosen` | `boolean` | `false` | The chosen face: the one picked from a set (the applied One Touch). |
| `waiting` | `boolean` | `false` | The waiting face: queued or armed (Fade armed), outlined in `hue`. |
| `hue` | `'t' \| 't2' \| 'm' \| 'a' \| 'lamp' \| 'rec' \| 'ok' \| 'r1' \| 'r2' \| 'r3' \| 'l' \| 'intro' \| 'main' \| 'ending' \| 'brk' \| 'fill'` | `'t2'` | The colour token (without `--`) of the waiting face's outline and label. Ignored by the other faces. |
| `pressed` | `boolean \| undefined` | — | Sets `aria-pressed` (a button that is a switch or a choice: ?, One Touch, Start / Stop, Fade). Undefined: no `aria-pressed` (a plain action: Panic, Stop, ◀). Never changes the look. |
| `popup` | `'dialog' \| 'menu' \| undefined` | — | Sets `aria-haspopup` (the caret opens a dialog). |
| `expanded` | `boolean` | `false` | With `popup`: `aria-expanded`, and the caret's ▾ turns `--t` while open. Without `popup`: ignored, no `aria-expanded`. |
| `controls` | `string \| undefined` | — | Sets `aria-controls` (the id of the popover the caret opens). |
| `join` | `'start' \| 'end' \| undefined` | — | Joined to a neighbour: `start` rounds only the left corners (`var(--radius) 0 0 var(--radius)`), `end` only the right (`0 var(--radius) var(--radius) 0`). Undefined: all four `--radius`. The caret is `join: 'end'`. |
| `disabled` | `boolean` | `false` | Shown, not pressable (`aria-disabled`, stays focusable): no `onpress`, `onhold` or long press. |
| `hold` | `boolean` | `false` | A repeat-while-held button (Tempo + / −): pointer down and up call `onhold`; a pointer click then doesn't call `onpress` (a keyboard click still does); no long press. |
| `name` | `string \| undefined` | — | The accessible name. Default: the label, then the symbol's word (`prev` "previous", `next` "next", `up` "up", `down` "down", `plus` "plus", `minus` "minus", `caret` "options"), joined by a space ("Fill up", "Tempo plus"; "up" alone). Parents pass `name` for every symbol-only button. |

Face precedence when more than one is set: `chosen`, then `on`, then `waiting`, then off.

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onpress` | click, Space or Enter, unless disabled; not for the click that ends a long press (swallowed by the action); with `hold`, only for a keyboard click (`event.detail === 0`) | `()` |
| `onhold` | with `hold` and not disabled: `true` on a primary-button `pointerdown` (the button then takes pointer capture); `false` once on the `pointerup`, `pointercancel` or `lostpointercapture` that ends it | `(down: boolean)` |
| `onlongpress` | without `hold`, not disabled: held 350 ms (`--long-press`) without moving more than 4px, still held; or right-clicked (longpress SPEC) | `()` |
| `onlongrelease` | the press that fired `onlongpress` ends | `()` |

**Long press wiring.** The `<button>` carries `use:longpress={{ onlongpress, onlongrelease, disabled: disabled || hold || onlongpress === undefined }}`. No board Button has a long press today; the props exist so a parent can add one without a timer of its own (Stage.md row 3).

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | Icons are the `symbol` glyphs, not snippets (D2). |

### Visual rules

- **Tokens used:** `--btn`, `--t`, `--t2`, `--m`, `--d`, `--g`, `--lamp`, `--lamp-ink`, `--ok`, `--bg`, the hue tokens named by `hue`, `--focus`, `--radius`, `--font-sans`, `--text-9`, `--text-10`, `--text-12`, `--text-13`, `--text-14`, `--weight-light`, `--weight-regular`, `--weight-medium`, `--space-2`, `--space-4`, `--space-8`, `--space-14`, `--control-height`, `--line-width`, `--focus-offset`; new in `scale.css` (D5): `--button-band-width` 88px, `--button-pair-width` 41px, `--caret-width` 20px, `--bar-bottom` 5px, `--bar-lift` 3px. Through the action: `--long-press`.
- **Box:** a native `<button type="button">`, `box-sizing: border-box`, no border, `margin: 0`, height `--control-height` (32) in every size, radius `--radius` (or `join`'s), `white-space: nowrap`, `line-height: normal`, no flex (the label, a space and the symbol are inline text, as the board draws them). Never wraps; no ellipsis.
- **Size:**

  | Size | Width | Padding | Align | Label | Symbol alone |
  |---|---|---|---|---|---|
  | `icon` | `--control-height` (32) | 0 | centre | 14 (`compact` 13) | `--text-12` |
  | `md` | from content | `0 var(--space-14)` | centre | 14 (`compact` 13) | `--text-12` |
  | `band` | `--button-band-width` (88) | `0 var(--space-8)`; with `bar` defined `0 var(--space-4) var(--bar-lift) var(--space-8)` | left | 14 (`compact` 13) | `--text-12` |
  | `pair` | `--button-pair-width` (41) | 0 | centre | 13 | `--text-12` |
  | `cell` | 100% of its container, `min-width: 0` | 0 | centre | 13 | `--text-12` |
  | `caret` | `--caret-width` (20) | 0 | centre | 13 | `--text-10` |

- **Symbol after a label:** one space, then the glyph: `up` and `down` at `--text-9` (Fill ▲); `plus` and `minus` at the label's size and `--weight-light` (Tempo +); `prev`, `next` and `caret` at `--text-12`, `--text-12`, `--text-10`. The glyph takes the label's colour in every state.
- **States drawn by** (`data-face` on the `<button>`):
  - off (`off`): `--btn` fill; label `--t2`, regular. `strong`: `--t`, medium. `caret`: `--m` (`--t` while `expanded`).
  - on (`on`): `--lamp` fill, `--lamp-ink` label, medium.
  - chosen (`chosen`): `--t` fill, `--g` label, medium.
  - waiting (`waiting`): transparent fill, a `--line-width` outline drawn as `box-shadow: inset 0 0 0 var(--line-width) var(--<hue>)` (D3), label `--<hue>`, regular; `data-hue="<hue>"` on the button (only in this face).
  - disabled: the face it would have, label and symbol `--d`, `cursor: default`; `data-face` unchanged, read `aria-disabled` (D4).
  - bar (`bar` true): an `aria-hidden` span with `data-bar`, `position: absolute` (the button is `position: relative`), left and right `--space-8`, bottom `--bar-bottom`, height `--space-2`, radius `calc(var(--space-2) / 2)`, `--ok` fill, `box-shadow: var(--bg)` (none in light). `bar` false or undefined: no span.
  - joined (`join`): radii only, in every face.
  - keyboard focus: `--line-width` outline in `--focus` at `--focus-offset` on `:focus-visible`; nothing on mouse focus.
  - No hover or pressed look; `cursor: pointer` when enabled (kit D35).
- **Type:** DM Sans, sentence case as given, `font-variant-numeric: tabular-nums`; sizes per the table.
- **Contrast (AA 4.5:1, `tokens/contrast.test.ts`):** existing pairs `--t2` on `--btn`, `--m` on `--btn`, `--lamp-ink` on `--lamp`, `--t2` on `--g` (waiting in `--t2` over the ground); new pairs to add: `--g` on `--t` ("chosen label (Button)"), `--t` on `--btn` ("strong label, open caret (Button)"). Disabled `--d` is exempt. A waiting face in a hue other than `t2` is the parent's pair to list.
- **Motion:** none (the running bar is static; it comes and goes with `bar`).

### Accessibility

- **Role and name:** a `button`; accessible name `name`, or the default above, set as `aria-label`. `aria-pressed` only when `pressed` is defined; `aria-haspopup` and `aria-expanded` only with `popup`; `aria-controls` only with `controls`; `aria-disabled="true"` when disabled. The symbol and the bar are `aria-hidden`.
- **Keyboard:** Tab focuses it (also when disabled); Space or Enter calls `onpress` (with `hold` too: one step). The long press, when wired, is the platform's context-menu key (the action).
- **Tooltip id:** the parent's (e.g. `transport.panic`, `ots.2`, `metronome.settings`, `tempo.up`); wired at integration.

## Stories (Story station)

Title `Primitives/Button`, `layout: 'centered'` unless the row says otherwise. Every story renders in dark and light. Meta `args`: `{ onpress: fn(), onhold: fn(), onlongpress: fn(), onlongrelease: fn() }` (axiom 7). Controls: `symbol`, `join`, `popup` and `hue` are selects (with an empty option for undefined where the type allows it); `bar` is a select of undefined / false / true; the rest booleans, text or select as typed.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ symbol: 'caret', size: 'caret', join: 'end', popup: 'dialog', expanded: false, name: 'Metronome settings' }` | the first Button in reading order on the Stage board: the Metronome caret, closed (D1) | `Board-{dark,light}.png` (Stage 1201,68 20×32) | the button named `Metronome settings` has `aria-haspopup="dialog"`, `aria-expanded="false"`, no `aria-pressed`, `data-face="off"`; click → `onpress` called once |
| `CaretExpanded` | `Board` args with `expanded: true` | the caret while its popover is open: ▾ in `--t` | `CaretExpanded-{dark,light}.png` (Stage-Metronome 1201,68 20×32) | `aria-expanded` is `true` |
| `Md` | `{ label: 'Panic', size: 'md', name: 'Panic: all notes off' }` | the off face, padding 0 14, `--t2` label | `Md-{dark,light}.png` (Stage 1313,68 63×32) | click, then Space, then Enter → `onpress` called 3 times, each with no arguments; no `aria-pressed` |
| `Icon` | `{ label: '?', size: 'icon', pressed: false, name: 'Help mode' }` | a 32 × 32 off button with a 14px character | `Icon-{dark,light}.png` (Stage 1384,68 32×32) | `aria-pressed` is `false`; `data-face="off"` |
| `On` | `{ label: '?', size: 'icon', on: true, pressed: true, name: 'Help mode' }` | the lamp face (help mode on) | `On-{dark,light}.png` (Stage-Help 1384,68 32×32) | `aria-pressed` is `true`; `data-face="on"` |
| `IconSymbol` | `{ symbol: 'prev', size: 'icon', name: 'Previous style (Track left)' }` | ◀ alone at 12px | `IconSymbol-{dark,light}.png` (Stage 49,129 32×32) | the button's accessible name is `Previous style (Track left)`; its text "◀" is inside an `aria-hidden` element |
| `Chosen` | `{ label: '2', size: 'icon', chosen: true, pressed: true, name: 'One Touch 2, applied' }` | the white chosen block, medium label | `Chosen-{dark,light}.png` (Stage 483,129 32×32) | `data-face="chosen"`; `aria-pressed` `true` |
| `Band` | `{ label: 'Stop', size: 'band', name: 'Stop (fade with hold)' }` | 88 × 32, label left at 8px | `Band-{dark,light}.png` (Stage 1328,514 88×32) | — |
| `Running` | `{ label: 'Start / Stop', size: 'band', compact: true, strong: true, bar: true, pressed: true, name: 'Start / Stop, running' }` | `--t` 13/500 label lifted 3px over the green bar | `Running-{dark,light}.png` (Stage 1328,476 88×32) | one `[data-bar]` element inside, `aria-hidden`; `aria-pressed` `true` |
| `Stopped` | `Running` args with `bar: false, pressed: false, name: 'Start / Stop'` | the same label and padding, no bar | — (the board draws only running) | no `[data-bar]` element; `aria-pressed` `false` |
| `BandSymbol` | `{ label: 'Tempo', symbol: 'plus', size: 'band', name: 'Tempo up (Scene Launch)' }` | "Tempo" and a light + | `BandSymbol-{dark,light}.png` (Stage 1328,686 88×32) | — |
| `BandCompact` | `{ label: 'Style tempo', size: 'band', compact: true, name: 'Style tempo' }` | the 13px band label | `BandCompact-{dark,light}.png` (Stage 1328,762 88×32) | — |
| `Pair` | `{ label: 'Reset', size: 'pair', name: 'Section reset' }` | 41 × 32, 13px centred | `Pair-{dark,light}.png` (Stage 1328,552 41×32) | — |
| `PairSymbol` | `{ label: 'Fill', symbol: 'up', size: 'pair' }` | "Fill" and a 9px ▲ | `PairSymbol-{dark,light}.png` (Stage 1328,590 41×32) | the accessible name is `Fill up` (the default) |
| `Waiting` | `{ label: 'Fade', size: 'pair', waiting: true, hue: 't2', pressed: false, name: 'Fade, armed' }` | outlined in `--t2`, no fill | — (no board draws Fade armed) | `data-face="waiting"`, `data-hue="t2"` |
| `FadeOn` | `{ label: 'Fade', size: 'pair', on: true, pressed: true, name: 'Fade, fading' }` | the lamp face in a pair | — (not drawn) | `data-face="on"` |
| `Cell` | `{ label: 'Panel', size: 'cell', name: 'Fader page is Panel: click for Style' }`, `layout: 'padded'` | fills its container, 13px centred | — (the lamp row's cells are fractional widths) | — |
| `Disabled` | `{ label: 'Audition', size: 'md', compact: true, disabled: true, name: 'Audition (stop the band first)' }` | `--d` label on the off face | `Disabled-{dark,light}.png` (LibrarySounds 1116,452 79×32) | `aria-disabled` `true`; click and Enter → `onpress` not called; `fireEvent.pointerDown(button, { button: 0, pointerId: 1 })`, wait 500 ms → `onlongpress` and `onhold` not called |
| `DisabledIcon` | `{ symbol: 'up', size: 'icon', disabled: true, name: 'Knob page up' }` | ▲ in `--d` (knob page 1) | — (the board draws it enabled, D1) | `aria-disabled` `true` |
| `JoinStart` | `{ label: 'Panic', size: 'md', join: 'start' }` | square right corners | — (no board instance) | — |
| `Hold` | `{ label: 'Tempo', symbol: 'plus', size: 'band', hold: true, name: 'Tempo up (Scene Launch)' }` | — | — | `fireEvent.pointerDown(button, { button: 0, pointerId: 1 })` → `onhold` called with `true`; `fireEvent.pointerUp(button, { button: 0, pointerId: 1 })` → `onhold` last called with `false`, 2 calls in all; `fireEvent.click(button, { detail: 1 })` → `onpress` not called; focus it and press Enter → `onpress` called once; wait 500 ms after a new pointer down → `onlongpress` not called |
| `LongPress` | `{ label: 'Stop', size: 'band' }` | — | — | `fireEvent.pointerDown(button, { button: 0, pointerId: 1, clientX: 10, clientY: 10 })`; `await waitFor(() => expect(onlongpress).toHaveBeenCalledTimes(1), { timeout: 1000 })`; `fireEvent.pointerUp(button, { button: 0, pointerId: 1 })` → `onlongrelease` called once; `fireEvent.click(button)` → `onpress` not called; `userEvent.click(button)` → `onpress` called once; `fireEvent.contextMenu(button)` → `onlongpress` called 2 times in all |
| `Focused` | `{ label: 'Panic', size: 'md' }`, `pseudo: { focusVisible: true }` | the focus ring | — | — |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in dark and light.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- Button` passes: each cropped story matches its crop (score at most 0.02, or the Inspect agent judges the difference render noise), and axe finds no violation on any story.
- The long press comes only from `use:longpress`; the component has no `setTimeout`.
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · Board story is the caret, as drawn.** The first Button in reading order on the Stage is the Metronome caret; `Board` draws it enabled, as the board does. Stage.md D32 disables it in the app until the metronome popover (#509) lands; that is a prop the wiring passes (`disabled`), shown by `Disabled` / `DisabledIcon`, not the board's look. Likewise the board draws knob ▲ and pad bank ▲ enabled where the fixture disables them.
- **D2 · Icons are glyphs, not snippets.** Every icon the boards draw is a text glyph (◀ ▶ ▲ ▼ ▾ + −), so `symbol` is a fixed list with fixed sizes rather than a snippet; it stays a control, and a builder never guesses a size. `?` and the digits are plain labels.
- **D3 · Waiting outline as an inset shadow.** The kit draws waiting as a 1px border; here it is `inset 0 0 0 1px`, so a Button keeps the same box in every face (an `md` button doesn't grow 2px when queued).
- **D4 · `data-face` keeps the face when disabled.** Disabled is "the face it would have" (kit › Faces), so `data-face` stays `off|on|chosen|waiting` and tests read disabled from `aria-disabled`; kit D41's `disabled` value is not used.
- **D5 · New size tokens.** Axiom 2 needs the board's fixed sizes as tokens: `--button-band-width` 88px, `--button-pair-width` 41px, `--caret-width` 20px, `--bar-bottom` 5px, `--bar-lift` 3px (scale.css), besides the kit's `--text-9`, `--text-10` and `--long-press`.
- **D6 · The caret is a Button size.** Stage.md builds `MetronomeSplit` from LampButton and Button, so the 20 × 32 ▾ is `size: 'caret'` with its own `--m` colour (`--t` when open, as Stage-Metronome draws it), plus `popup` / `expanded` for its ARIA.
- **D7 · Faces are props, not state.** Button has no pressed state of its own; `chosen`, `on`, `waiting` and `pressed` come from the parent (One Touch's `ots.applied`, help mode, `transport.fade`), with precedence chosen, on, waiting, off.
- **D8 · `pressed` is separate from the face.** `aria-pressed` follows `pressed` only, so a chosen One Touch says "pressed" and a plain Panic says nothing; Start / Stop is `pressed` while running, as the board marks it.
- **D9 · `compact` for 13px.** The board mixes 14px (Stop, Tempo ±, Panic) and 13px (Style tempo, Start / Stop, Audition) labels in the same sizes; `compact` picks 13, while `pair` and `cell` are always 13.
- **D10 · Start / Stop's bar.** `bar: false` keeps the bar's padding (`0 4px 3px 8px`), so the label doesn't move when the band starts or stops; undefined is a button with no bar at all.
- **D11 · `hold` is a flag.** Every callback is a story action (axiom 7), so `onhold` is always passed in stories; the mode is `hold: true`, not the presence of `onhold`. It copies today's `HwButton` hold (pointer capture, keyboard click still presses) so `tempoHold.ts` works unchanged, and turns the long press off.
- **D12 · Default names.** Without `name`, a symbol's word follows the label ("Fill up"); the parents still pass the board's longer names ("Fill Up: a fill, then the next Main up", "Previous style (Track left)").

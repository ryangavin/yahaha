# ChosenTabs

## Identity (all stations)

- **Kind:** primitive
- **Built from:** —
- **Purpose:** Shows a short run of choices side by side and which one is picked, and picks another with one click: the app's pages, the fader page, the fader layer.
- **Boards:**
  - `Stage-Dark.dc.html:75-81` (the display-page tabs Stage … Harm/Arp, Stage chosen), `:83-84` (Library, Settings), `:226-228` (fader page Panel | Style, Panel chosen), `:231-237` (fader layer Vol … Delay, Vol chosen); light: `Stage-Light.dc.html:51-57`, `:59-60`, `:202-204`, `:207-213`.
  - `Channel-Dark.dc.html:364-370` (the half band's layer tabs, 8px side padding); light: `Channel-Light.dc.html:328-334`.
- **Not this component's job:** no store, no API, no Tauri. It doesn't know what the choices mean: the parent passes `tabs` and `chosen` and acts on `onchoose`; it never moves `chosen` itself. No landmark, caption, heading or separator: the app bar's `nav`, its separator before Library (PageTabs) and the band's "Layer" word (FaderBank) are the parent's. No page or panel content. No hover or pressed look (kit › Faces, D35).

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `tabs` | `TabItem[]` | — | The choices, left to right. `TabItem` is exported from `app/src/ui/ChosenTabs/types.ts` (below). |
| `chosen` | `string \| null` | `null` | The `id` of the chosen tab; `null` = none is chosen (a page with no tab is on view, or the other run of PageTabs holds the chosen page). The component draws exactly this; a click doesn't move it (the parent does, by changing `chosen`). |
| `size` | `'page' \| 'header' \| 'compact'` | `'header'` | The size and the kind. `page`: 36 tall, 14px, `10px 10px 0` padding, a 24px chosen block, and the tabs are page navigation (buttons with `aria-current`, inside the parent's `nav`). `header`: 35 tall, 13px, `11px 10px 0`, a 22px block, a `tablist` (the full band's fader header). `compact`: as `header` with `11px 8px 0` padding (the half band's layer tabs, Channel board). |
| `label` | `string \| undefined` | — | The accessible name of the run. `header` and `compact`: the tablist's `aria-label` ("Fader page (master button)", "Fader layer"); every parent passes it. When it is missing or empty there, the tablist renders with no `aria-label` attribute at all (never `aria-label=""`), and nothing warns (D16). `page`: ignored (the parent's `nav` carries the name, "Pages"). Never drawn. |
| `tipAction` | `Action<HTMLElement, string> \| undefined` | — | The app's tooltip action (`use:tip`), passed in by the wiring because the library can't import it (D9, lane rule L3). One action for the whole run: when it is set, every tab whose item has a `tip` gets `use:tipAction={tab.tip}` on its button; a tab without `tip` gets neither. Without `tipAction` the button still carries `data-tip`. Stories pass `fn()` (an action). |

`TabItem`:

| Field | Type | Default | Meaning |
|---|---|---|---|
| `id` | `string` | — | What `onchoose` carries and what `chosen` names. Unique within `tabs`. |
| `label` | `string` | — | The word on the tab ("Vol", "Quick Racks"). |
| `name` | `string \| undefined` | — | The accessible name when the label is short ("Volume" for "Vol", "Reverb send" for "Reverb"). Default: the label. |
| `disabled` | `boolean \| undefined` | `false` | Shown, not choosable (`aria-disabled="true"`; the attribute is absent, not `"false"`, on an enabled tab, L5). |
| `tip` | `string \| undefined` | — | The tooltip key (`view.stage`, `mixer.layer`), set as `data-tip` on the button and passed to `tipAction`. No `tip`: no `data-tip` attribute. |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| `onchoose` | a `click` on an enabled tab, the chosen one included (D6). Space and Enter reach it only through the button's native click (D14), so one key press is one call. In a tablist (`header`, `compact`), also ←, →, Home or End moving focus to another enabled tab (a key that leaves focus where it is calls nothing) | `(id: string)` that tab's `id` |

A click or key on a disabled tab calls nothing (the click handler returns early when the item is `disabled`).

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| — | |

### Visual rules

- **Tokens used:** `--t`, `--g`, `--m`, `--d`, `--focus`, `--font-sans`, `--text-13`, `--text-14`, `--weight-regular`, `--space-8`, `--space-10`, `--line-width`, `--focus-offset`, and the new tokens below.

#### New tokens

Not in `app/src/ui/tokens/*` today. They land in the orchestrator's tokens contract PR (in `scale.css`) before this component is built; the builder uses them by name and never hard-codes the value (L1). Neither this component's PR nor PageTabs adds them.

| Token | Dark | Light | Used for |
|---|---|---|---|
| `--bar-height` | `36px` | `36px` | the `page` tab height (the app bar's height; AppBar and PageTabs use it too) |
| `--tab-height-header` | `35px` | `35px` | the `header` and `compact` tab height (the 36px band header row minus its 1px hairline) |
| `--tab-block` | `24px` | `24px` | the chosen block's height at `page` |
| `--tab-block-header` | `22px` | `22px` | the chosen block's height at `header` and `compact` |
| `--space-11` | `11px` | `11px` | the top padding at `header` and `compact` |

- **The run:** a `div`, one row (`display: flex; align-items: stretch`), `flex: none`, no gap between tabs, no wrap, no background, no border. Its height is the tab height; its width the sum of the tabs.
- **Size:** every tab is a `<button type="button">`, `box-sizing: border-box`, border 0, radius 0, margin 0, `white-space: nowrap`, the label centred horizontally and set by the top padding:

  | Size | Height | Padding (top, sides, bottom) | Label | Chosen block |
  |---|---|---|---|---|
  | `page` | `--bar-height` (36) | `--space-10`, `--space-10`, 0 | `--text-14` | `--tab-block` (24) |
  | `header` | `--tab-height-header` (35) | `--space-11`, `--space-10`, 0 | `--text-13` | `--tab-block-header` (22) |
  | `compact` | `--tab-height-header` (35) | `--space-11`, `--space-8`, 0 | `--text-13` | `--tab-block-header` (22) |

  Width: the label plus the side padding (measured on the boards: Stage at `page` 57, Panel at `header` 52, Vol at `compact` 34).
- **States drawn by:**
  - not chosen: no fill (transparent), label `--m`.
  - chosen: the white block on the bottom of the tab, `background: linear-gradient(var(--t), var(--t)) left bottom / 100% <block> no-repeat`, with `<block>` from the size table; label `--g`. The label keeps `--weight-regular` (D3), so choosing never moves the tabs. Above the block the tab stays transparent; in the app bar the block's bottom pixel row is the row of the bar's 1px `--t` line (drawn by AppBar, not here).
  - disabled: label `--d`, cursor `default`, on either face (chosen and disabled: the white block with a `--d` label).
  - keyboard focus (`:focus-visible`): a `--line-width` outline in `--focus`, `--focus-offset` outside the whole tab button (all 36 or 35 px of it, not only the block); nothing on mouse focus.
  - No hover, pressed, underline, bar or glow.
- **Type:** DM Sans (`--font-sans`), `--weight-regular` in every state, `line-height: normal`, the label as given (sentence case), tabular numerals.
- **Cursor:** `pointer` on enabled tabs, `default` on disabled ones.
- **Test hooks:** each tab carries `data-face="chosen"` (the chosen one) or `data-face="off"` (the rest), or `data-face="disabled"` with `data-contrast="dim"` when its item is disabled, chosen or not (D20); the run carries `data-size` (`page | header | compact`).
- **Contrast (AA 4.5:1, `tokens/contrast.test.ts`):** new row (lands with the tokens contract PR, L1): `--g` on `--t`, "chosen tab label on its block (ChosenTabs)" (21 dark, 16.72 light, both pass). Existing row: `--m` on `--g` (a tab not chosen). Disabled `--d` is exempt.
- **Not checkable in jsdom:** the block (a gradient background), its height, the label colours and the tab widths; the cropped stories and `npm run shots -- ChosenTabs` cover them.
- **Motion:** none.

### Accessibility

- **Role and name:**
  - `page`: the run is a plain `div` (no role; the parent wraps it in `<nav aria-label="Pages">`); each tab a plain `<button>` named by its `name` (as `aria-label`, set only when `name` is given), else its label text; the chosen one `aria-current="page"`, the others no `aria-current`. No `aria-pressed`, no `aria-selected`.
  - `header`, `compact`: the run is `role="tablist"` with `aria-label={label}` and `aria-orientation="horizontal"`; each tab `role="tab"` with `aria-selected="true|false"`, named by its `name` (as `aria-label`, set only when `name` is given), else its label text. No `aria-controls` (D5).
  - Disabled tabs: `aria-disabled="true"` (never the `disabled` attribute); enabled tabs have no `aria-disabled` attribute (L5).
- **Keyboard:**
  - `page`: every tab is a Tab stop, left to right (disabled ones too, kit › Faces); no `tabindex` attribute is set. Space or Enter is the native button click (below). No arrow handling: ← → reach the window key handler, which steps the style, as on any other button.
  - **Space and Enter, every size (D14):** the component has no Space or Enter handling of its own: no `keydown` or `keyup` handler looks at them, no `preventDefault()`, no `stopPropagation()`. The browser turns Enter (on keydown) and Space (on keyup) on a focused `<button>` into one `click`, and the click handler calls `onchoose` once. The app's window handler already ignores Space and Enter whose target is a button (`app/src/lib/shortcuts.ts`), so Space doesn't also start the band. Plays press them with `userEvent.keyboard` (from `storybook/test`), which fires that click; `fireEvent.keyDown` would not.
  - `header`, `compact` (roving focus, D13, D15): exactly one tab has `tabindex="0"`, every other `-1`, so Tab enters and leaves the run in one step. Which tab holds the `0` is internal state, `active` (a tab id):
    - it starts as `chosen`, and is reset to `chosen` whenever the `chosen` prop changes and whenever focus leaves the run (`focusout` whose `relatedTarget` is outside the run);
    - an arrow, Home or End sets it to the tab it moves focus to, so the `0` follows focus even when the parent never updates `chosen` (as in the stories);
    - focusing a tab by click or by script sets it to that tab;
    - the tab that gets the `0` is `active` when that tab exists and is enabled; else the first enabled tab (this covers `chosen: null`, an unknown id and a disabled chosen tab); else, when every tab is disabled, the first tab (so the run is still reachable with Tab and its tooltip readable).
  - From the focused tab, → and ← move focus to the next or previous enabled tab, wrapping at the ends, and call `onchoose` with it (automatic activation, D2); Home and End move to the first and last enabled tab and call `onchoose` with it. Disabled tabs are skipped (D4). When the key would leave focus where it is (no other enabled tab, or Home on the first enabled tab, End on the last), nothing is called. The handler is a `keydown` listener on each tab button; for these four keys (`ArrowLeft`, `ArrowRight`, `Home`, `End`) it always calls `preventDefault()` and `stopPropagation()`, moved or not, so the window key handler's ← → (`stepStyle`) never fire while a tab has focus (kit › Interaction conventions). ↑ ↓ and every other key are left alone.
- **Tooltip id:** per tab (`tip`): the page tabs `view.stage`, `nav.channel` (new key: contract change, D12), `nav.effects`, `nav.quick`, `nav.multipad`, `nav.looper`, `nav.harmony`, `view.library`, `nav.settings`; the fader page tabs `mixer.page`; the layer tabs `mixer.layer`. Set by the parent (PageTabs, FaderBank) on each item; applied through `tipAction` (L3).

## Stories (Story station)

Title `Primitives/ChosenTabs`, `layout: 'centered'`. Every story renders in dark and light (the toolbar theme). Every story's args also carry `onchoose: fn()` and `tipAction: fn()` (actions, L3), so the table leaves them out. Keys in plays are pressed with `userEvent.keyboard` (`'{Enter}'`, `' '`, `'{ArrowRight}'`, `'{Home}'` …) after focusing the tab with `userEvent.click` or `element.focus()`, as each row says. Fixtures (`app/src/ui/ChosenTabs/ChosenTabs.fixtures.ts`, exported by these names; GroupHeader's stories import `faderPageTabs` and `layerTabs`):

- `displayPageTabs`: `stage` "Stage" (`view.stage`), `channel` "Channel" (`nav.channel`), `effects` "Effects" (`nav.effects`), `quickRacks` "Quick Racks" (`nav.quick`), `multiPads` "Multi Pads" (`nav.multipad`), `looper` "Looper" (`nav.looper`), `harmArp` "Harm/Arp" (`nav.harmony`).
- `fullPageTabs`: `library` "Library" (`view.library`), `settings` "Settings" (`nav.settings`).
- `faderPageTabs`: `panel` "Panel", `style` "Style" (both `tip: 'mixer.page'`).
- `layerTabs`: `volume` "Vol" (name "Volume"), `pan` "Pan", `reverb` "Reverb" (name "Reverb send"), `chorus` "Chorus" (name "Chorus send"), `delay` "Delay" (name "Delay send"); all `tip: 'mixer.layer'`.

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ tabs: displayPageTabs, chosen: 'stage', size: 'page' }` | the app bar's first run: Stage in the 24px white block, Channel … Harm/Arp grey | `Board-{dark,light}.png` (Stage 527,24 530×36; see below) | 7 buttons named Stage … Harm/Arp in order; no `tablist`, no `tab`; "Stage" has `aria-current="page"` and `data-face="chosen"`, the other 6 no `aria-current` and `data-face="off"`; "Stage" has `data-tip="view.stage"`; `tipAction` was called with ("Stage" button, `'view.stage'`) among its calls; no button has a `tabindex` attribute; click "Effects" → `onchoose` called with `'effects'`, and "Stage" still has `aria-current="page"` (the parent owns `chosen`); click "Stage" → called with `'stage'` (D6); focus "Looper" (`.focus()`), press Enter → called with `'looper'`; press Space → called with `'looper'` again; 4 calls in all |
| `FullPages` | `{ tabs: fullPageTabs, chosen: null, size: 'page' }` | Library and Settings, neither chosen (the second run on the Stage) | — (covered by `Board`; this run's box is not whole-pixel) | both buttons `data-face="off"`, no `aria-current` |
| `FaderPage` | `{ tabs: faderPageTabs, chosen: 'panel', label: 'Fader page (master button)' }` | the band header's Panel \| Style, Panel in the 22px block | `FaderPage-{dark,light}.png` (Stage 80,432 102×35) | a `tablist` named "Fader page (master button)"; tab "Panel" `aria-selected="true"` and `tabindex="0"`, "Style" `aria-selected="false"` and `tabindex="-1"`; neither has an `aria-disabled` attribute; click "Style" → `onchoose('style')` (1 call), "Style" now has `tabindex="0"` and "Panel" `-1` (the `0` follows focus, D13), "Panel" still `aria-selected="true"`; press Enter → `onchoose('style')` again (2 calls); press Space → 3 calls, every one with `'style'` (one call per key, D14) |
| `Layers` | `{ tabs: layerTabs, chosen: 'volume', label: 'Fader layer' }` | Vol chosen of five layers | `Layers-{dark,light}.png` (Stage 246,432 259×35) | tabs named "Volume", "Pan", "Reverb send", "Chorus send", "Delay send"; "Volume" selected |
| `LayerReverb` | `{ tabs: layerTabs, chosen: 'reverb', label: 'Fader layer' }` | the block on Reverb (Stage state "Fader layer not Vol") | — (no board draws it) | "Reverb send" selected, `data-face="chosen"` and `tabindex="0"`; exactly one `[data-face="chosen"]` |
| `Arrows` | `{ tabs: layerTabs, chosen: 'volume', label: 'Fader layer' }` | the same as `Layers` | — | first, `const spy = fn()` and `window.addEventListener('keydown', spy)` (bubble phase, no options: the app's `<svelte:window onkeydown>` listens there too); the rest of the play runs in `try { … } finally { window.removeEventListener('keydown', spy) }` so the listener never outlives the story. Focus "Volume" (`.focus()`); press → → `onchoose('pan')`, "Pan" has focus and `tabindex="0"`, "Volume" `tabindex="-1"` and still `aria-selected="true"` (the story never moves `chosen`, D13); press End → `onchoose('delay')`, "Delay send" focused; press → → wraps: `onchoose('volume')`; press ← → `onchoose('delay')`; press Home → `onchoose('volume')`; press Home again → no new call (focus stays); 5 calls in all; `spy` was never called |
| `Disabled` | `{ tabs: [faderPageTabs[0], { ...faderPageTabs[1], disabled: true }], chosen: 'panel', label: 'Fader page (master button)' }` | "Style" in `--d` | — (no board draws it) | "Style" has `aria-disabled="true"`, `data-face="disabled"`, `data-contrast="dim"` and `tabindex="-1"`; "Panel" has no `aria-disabled` attribute and `data-face="chosen"`; click "Style" → `onchoose` not called; focus "Panel", press → → focus stays on "Panel" (the only enabled tab) and `onchoose` is not called |
| `AllDisabled` | `{ tabs: faderPageTabs.map((t) => ({ ...t, disabled: true })), chosen: 'panel', label: 'Fader page (master button)' }` | both tabs in `--d`, the white block still on "Panel" | — (no board draws it) | both tabs `aria-disabled="true"`; "Panel" (the first tab) has `tabindex="0"`, "Style" `-1` (D15); click either → `onchoose` not called; focus "Panel", press → and End → focus stays, `onchoose` not called |
| `NoneChosen` | `{ tabs: layerTabs, chosen: null, label: 'Fader layer' }` | five grey tabs | — | no `aria-selected="true"`; no `[data-face="chosen"]`; "Volume" (the first enabled) has `tabindex="0"`, the others `-1` |
| `NoLabel` | `{ tabs: faderPageTabs, chosen: 'panel' }` (no `label`) | as `FaderPage` | — | the `tablist` has no `aria-label` attribute (D16) |
| `Compact` | `{ tabs: layerTabs, chosen: 'volume', label: 'Fader layer', size: 'compact' }` | the half band's tighter layer tabs | `Compact-{dark,light}.png` (Channel 246,600 238×35) | the run has `data-size="compact"` and `role="tablist"` |
| `Focused` | `{ tabs: layerTabs, chosen: 'volume', label: 'Fader layer' }`, `parameters: { pseudo: { focusVisible: ['[data-face="chosen"]'] } }` (the pseudo-states addon's selector form, so only the chosen tab is ringed) | the focus ring round the chosen tab only | — | — |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in the dark and light render.

- **`Board` crop:** the run's right end on the board is about 1057 (text widths are fractional), and the board's app bar draws its 1px `--t` line through the bottom pixel row (y 59) under the grey tabs too, which the component doesn't draw: about 0.025 of the crop differs by design. The Inspect agent judges that row and sub-pixel edge as noise and says so; AppBar's `Board` crop checks the tabs in place with the line.
- The header crops' boxes end at the header row's content (y 432 to 466); the hairline at 467 is GroupHeader's and is outside them.

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- ChosenTabs` passes: each cropped story's screenshot is the crop's size and scores at most 0.02 (the share of differing pixels), and axe (colour contrast included) finds no violation on any story. `Board` may score above 0.02 only by its bottom row and right edge (above); the Inspect agent says so.
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · Kind follows size.** `page` tabs are page navigation (buttons with `aria-current="page"` inside the parent's `nav`, as kit › App bar and the board mark them); `header` and `compact` tabs switch the band's view (`tablist` / `tab` with `aria-selected`, as the board marks them), not a `radiogroup`, since the boards, the kit and Stage.md's checks all call them tabs. No separate `kind` prop: no board draws page-size tablists or header-size navigation.
- **D2 · Automatic activation.** In a tablist the arrows call `onchoose` as they move focus: a fader layer or page switch is instant on the hardware too, so there's no "focus without choosing" step.
- **D3 · Chosen weight.** The chosen tab's label stays regular (400), as every board draws it; kit › Faces' "weight 500" for the chosen face applies to blocks like One Touch, not to tabs, so a tab never changes width when chosen.
- **D4 · Disabled.** A disabled tab stays visible and, in `page`, in the Tab order (kit › Faces); in a tablist it has `tabindex="-1"` and the arrows skip it, since automatic activation would otherwise choose it.
- **D5 · No tabpanel.** The tablist has no `aria-controls`: the tabs change what the strips show, and there is no panel element the component could name.
- **D6 · Re-choosing calls through.** A click on the chosen tab still calls `onchoose` (as PageTabs D3): the component never filters, and the parent decides what a repeat means (a drawer tab toggles its drawer until its page exists; the band's `setFaderLayer` to the same layer changes nothing).
- **D7 · Controlled.** The component draws `chosen` as given and never moves it on a click, so what it shows is always the session's state (the fader layer and page come back in `mixer`); the parent updates `chosen`.
- **D8 · Captions and separators outside.** The band's visible "Layer" word sits inside the board's tablist and the app bar's separator inside its `nav`; here both are the parent's (FaderBank draws "Layer" 4px before the tabs, PageTabs D1 draws the separator), so a run holds only tabs.
- **D9 · Tooltips.** The library can't import `use:tip`, so each tab carries its key as `data-tip` and the wiring may pass the action in as `tipAction`; this keeps Stage.md Check 15 (every interactive element has a `data-tip`) true for tabs.
- **D10 · `compact` size.** The half band's 8px-padded layer tabs (Channel board) are a third size here, so FaderBank's half variant (#501) needs no new primitive.
- **D11 · New tokens (L1).** `--bar-height`, `--tab-height-header`, `--tab-block`, `--tab-block-header` and `--space-11` (values under New tokens) and the `--g` on `--t` contrast row land in the orchestrator's tokens contract PR, not in this component's or PageTabs' PR.
- **D12 · `nav.channel` is a contract change (L3).** The Channel page tab's key doesn't exist in `tooltips.ts`; rather than borrow `mixer.channel` (a strip control), the tab uses the new key Stage.md C5 already names: `nav.channel`, title "Channel", body "Shows the Channel page: one part's sound, mix and inserts. Press again to go back to Stage.", `app_keys: ['alt+n']`, `launchkey: null`. The fixtures use it; until C5 lands the story still renders (`data-tip` is just a string here).
- **D13 · The `0` follows focus.** In a tablist the `tabindex="0"` follows the focused tab (internal `active`, reset to `chosen` when `chosen` changes or focus leaves the run), so arrows work the same whether or not the parent updates `chosen` at once, and Tab back into the run lands on the chosen tab.
- **D14 · Space and Enter are the native click.** The component never handles Space or Enter itself, so each press is exactly one `click` and one `onchoose`; the app's window handler already skips Space and Enter on a button.
- **D15 · Fallback tab stop.** With no enabled `active` tab, the `0` goes to the first enabled tab; with every tab disabled, to the first tab, so the run stays reachable (its tooltip still readable) while choosing nothing.
- **D16 · Missing tablist label.** A tablist without `label` renders with no `aria-label` attribute and no warning; every caller passes one, and the `NoLabel` story pins the behaviour.
- **D17 · Tooltip props (L3).** `tip` per item, one `tipAction` for the run, applied as `use:tipAction={tab.tip}` only when both are set; stories pass `tipAction: fn()`.
- **D18 · `aria-disabled` only when disabled (L5).** Enabled tabs carry no `aria-disabled` attribute at all.
- **D19 · Names against Stage.md row 4 (review).** Stage's `items: { key, label, tip }[]`, `chosen: key` and `kind: 'page' | 'tab'` are here `tabs: TabItem[]` with `id`, `chosen: string | null`, and the kind set by `size` (D1: `page` is Stage's `kind: 'page'`, `header` / `compact` its `kind: 'tab'`). Same behaviour; GroupHeader's stories use these names.
- **D20 · A disabled tab's test hooks (review).** One rule with Button D25 and LampButton D18 (Stage.md › The base Button, kit D41): a disabled tab carries `data-face="disabled"`, and its `--d` label `data-contrast="dim"` (D47), whether or not it holds the chosen block.

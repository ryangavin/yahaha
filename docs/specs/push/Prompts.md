# Prompts

The questions the app asks over the Stage: Store a never-saved rack, Unsaved changes before
a rack switch, Name the new sound, Delete a rack, and Rename a rack. One at a time, in a
dialog over a scrim, with the status line still readable under it. Nothing here is a page:
the prompts sit over whatever page is up.

- **Issue:** #504 · **Flow:** Save your setup · **Boards:** `docs/design/push/Prompts-Dark.dc.html`,
  `Prompts-Light.dc.html`; pictures `docs/design/push/png/Prompts-Dark.png`, `Prompts-Light.png`
  (1440 × 900). The spec stands without them: every value a builder needs is below or in
  [kit.md](kit.md); the board lines in the Components table are for cutting crops.
- **Built from:** [kit.md](kit.md): the faces and tokens, Button (`md`), the Status line.
  [Stage.md](Stage.md) for the Stage underneath (the board is the Stage board copied
  verbatim, plus the overlay). This file specifies the overlay and each prompt's anatomy as
  reusable parts: the variant Prompts-More (#524) specs only its delta against them.
- **Copied from:** Stage (#500). What changed: a scrim over the whole window, the status line
  drawn over it with a refusal, and one dialog in the middle. The board draws four dialogs
  side by side for review; the app shows one (PR-D1).
- **Glance order:** the dialog's title (18px), then its primary button (the white block), then
  the body. The Stage under the scrim reads as context, nothing more: at 62% black it's
  about a third of its brightness.
- **Before building:** C1 (tooltips) must land first; the rest don't block (see Contract
  changes needed). This lane builds on the Stage lane (#500): it needs Button, StatusLine,
  KeyStrip, the `Stage` page, `Stage.fixtures.ts` and the `shots.ts` viewport-and-masks item
  merged first; the small additions to Stage components this spec needs are made in this
  lane (Kit additions): Button's `tone`, StatusLine's `inert` and `hidden` props and
  `data-shot-mask="status"`, KeyStrip's `data-shot-mask="keys"`, a `statusHidden` prop on
  `Stage` passed through to its StatusLine, and the one line in `app/src/pages/StageWiring.svelte`
  that passes `statusHidden={ui.modal}` (this lane edits that file for that line only).

## Layout

At 1440 × 900. The overlay is two sibling nodes mounted at the end of `App.svelte`, the
scrim and a `position: fixed; inset: 0` wrapper that holds the layer (no transformed
ancestor: they are outside the Stage.md D1 scaler, which wraps only the Stage), bottom to
top:

| Region | Box | What's in it | Spec |
|---|---|---|---|
| Scrim | the whole window, `z-index: 79` | one flat colour, `--scrim` (new token) | Scrim |
| Layer | inside the wrapper (`z-index: 80`, `pointer-events: none`): a 1440 × 900 box, `position: absolute; left: 0; top: 0`, scaled with the D1 maths (`scale = min(w / 1440, h / 900)` from the window's inner size, `transform: translate((w − 1440·scale) / 2, (h − 900·scale) / 2) scale(scale)`, `transform-origin: top left`), `pointer-events: none` except on its contents (`pointer-events: auto` on the status line and the dialog) | the status line and the dialog | below |
| Status line (in the layer) | `24,800 1392×20` | `state.message`, drawn over the scrim | kit › Status line, Kit additions |
| Centring box (in the layer) | `0,0 1440×800`, `display: grid; place-items: center` | the one prompt, centred by this box (the Dialog has no position of its own) | Dialog panel, Prompts |
| Dialog (in the centring box) | `500,y 440×h` (`y = (800 − h) / 2`) | the one prompt | Dialog panel, Prompts |

The overlay carries its own copy of the D1 scale (PR-D2) because it also shows over pages
and drawers that aren't in the Stage's scaler (today's Library, the Rack drawer); the scrim
dims the letterbox bars too. The z-indexes sit above every drawer, the Browser and the
master popover (`lib/ui/Overlay` 40 and 50, `HelpFooter` 45, `MasterFx` 60) and below the
Tooltip (100). The 100px under the dialog's centring box is the status line's and the keys'
(the kit's `24,800` and `24,820` boxes), so a dialog never covers the status line: the
tallest board prompt (Store with a name field, 346) sits at `y = 227`, 227px above it. The
keys are under the scrim (the board leaves them clear for review; in the app the scrim
covers all, PR-D2). On a view without a status line of its own (today's Library) the
overlay's line still draws at `24,800`.

The dialogs' heights and places in the app (one at a time):

| Prompt | h | Box |
|---|---|---|
| Store (never saved) | 346 | `500,227 440×346` |
| Store (saved, modified) | 278 | `500,261 440×278` |
| Unsaved changes (load or new) | 222 | `500,289 440×222` |
| Unsaved changes, never saved (a field) | 290 | `500,255 440×290` |
| Sound names (one part) | 266 | `500,267 440×266` |
| Sound names (n parts) | 266 + 68 × (n − 1) + 20 per extra body line | centred |
| Delete | 198 | `500,301 440×198` |
| Delete, nothing held | 158 | `500,321 440×158` |
| Rename | 190 | `500,305 440×190` |

A height is the panel's content (below) plus 48 (padding) plus 2 (the border): title 24, a
body line 20, a note line 16, the bank row 44, a name field 52, the actions 32, and 16 between
each pair of parts. Bodies and notes wrap at 390px (440 less the padding and the border,
the board's own content width); the heights here are for the fixture's texts at that width
(the board's line counts: Store's body four lines, Unsaved's body one and its note three,
Sound names' body three, Delete's body three). Another text may wrap to one line more or
less; the dialog stays centred and has no fixed height. Past 752px (the centring box less 24
top and bottom) the panel stops growing (`max-height: 752px`, on the Dialog itself, so it
holds in a story too) and only its children region scrolls: the children sit in one wrapper
(`div`, a flex item of the column, `display: flex; flex-direction: column; gap: 16px;
overflow: auto; min-height: 0`), so they stay 16px apart and the wrapper is what scrolls; the
title, body, note and actions stay. No fixture reaches it.

## Scrim

`position: fixed; inset: 0; z-index: 79; background: var(--scrim)`. Dark `rgba(0, 0, 0, 0.62)`, light
`rgba(242, 241, 238, 0.72)` (kit additions). `aria-hidden`. A click on it does nothing
(PR-D3): the session holds the question, so the prompt closes only through its buttons or
Esc; its `mousedown` is `preventDefault`ed so focus stays in the dialog. The cursor over it
is `default`. No blur, no animation.

## Dialog panel

The reusable panel every prompt is built on (`Dialog`). Its props: `titleId: string`,
`title: string`, `role: 'dialog' | 'alertdialog'` (default `dialog`), `kind: string` (the
`data-prompt` value), and the snippets `body`, `note`, `children` and `actions` (each
optional; `body` and `note` render inside the `<p>`s below, so a prompt puts its `--t` spans
there; `actions` holds the Button row's buttons). A column:

- **Box:** width 440, `box-sizing: border-box`, padding 24, `border: 1px solid var(--dialog-edge)`
  (new token: dark `color-mix(in srgb, var(--t) 40%, transparent)`, light `var(--t)`),
  radius 4 (`--radius`), `background: var(--g)`, no shadow, no glow. `display: flex;
  flex-direction: column; gap: 16px; max-height: 752px`. No position of its own: the layer's
  centring box places it (Layout); a story's `layout: centered` or the Board grid does the same.
- **Role:** `role="dialog"` (`alertdialog` for a destructive prompt: Delete), `aria-modal="true"`,
  `aria-labelledby` the title's id, `aria-describedby` the body's id (and the note's, space
  separated, when there is one). Each prompt gives its title id (`dlg-store`, `dlg-unsaved`,
  `dlg-names`, `dlg-delete`, `dlg-rename`); the body is `{titleId}-body`, the note
  `{titleId}-note`, a field's input `{titleId}-field-{n}` (n from 0) with its `<label for>`.
- **Order** in the column, top to bottom: title, body, note, children, actions. Each is left
  out when the prompt has none (Rename has no body).
- **Title** (`h2`, margin 0): 18 / 500, line-height 24, `--t`, `overflow-wrap: anywhere` (a
  name wraps on whole words, and a word longer than the line breaks inside itself, so nothing
  overflows 440). Sentence case; a question when the prompt asks one ("Delete Ballad?",
  "Store to Quick Rack A5?"), a statement when it reports ("Unsaved changes in Sunday drive",
  "Name the new sound").
- **Body** (`p`, margin 0): 14 / 400, line-height 20, `--m`. A name in the body (a rack, a
  sound, a slot, a style) is a `<span>` in `--t`, same size and weight (the board's
  "names in white"). A sound's origin after its name (" · Factory", the dot and the word in
  one span) is 12px, still `--m`.
- **Note** (`p`, margin 0, optional): 12 / 400, line-height 16, `--m`, names in `--t` as the
  body. Explains what the prompt doesn't do (the Unsaved prompt's "Only the screen asks").
- **Children** (optional, between the body and the actions): a Bank row, one or more Name
  fields, in the children wrapper (Layout: a flex column, gap 16, the part that scrolls), so
  every pair of parts is 16px apart.
- **Actions** (last): a row, `justify-content: flex-end`, gap 8. Every button is kit › Faces
  as a 32-tall Button `md` (padding 0 14, 14px, radius 4): the **cancelling** action in the
  off face (`--btn`, `--t2`, 400); the **primary** action (the one Enter runs, and the
  default focus when there is no field) in the chosen face (`--t` fill, `--g` label, 500);
  a **destructive** action in the off face with its label in `--ending-ink` (new token, Kit
  additions: the ending red as text on `--btn`; dark `#d87070`, 5.6:1 on `--btn`, light
  `var(--ending)`, 4.6:1; `--ending` itself is 4.3:1 in dark and fails AA, PR-D31), 400
  (PR-D4). Order, left to right: cancelling, destructive, primary; where the cancelling
  action is the default (Delete) it wears the primary face and comes first, the destructive
  last. **Button's API** (Stage.md #3 gives only its variants): props `variant`, `tone`,
  `disabled: boolean` (renders the kit's Disabled face: `aria-disabled`,
  `data-face="disabled"`, the label `--d`, `data-hue="d"`, still focusable, click and keys
  ignored), `tip: TipKey`, `label: string`, `onclick()`; every other attribute (`aria-label`,
  `data-cancel`, `type`) passes through to the `<button>` (`{...rest}`).
- **Disabled primary** (a field empty, or Rename unchanged): the chosen face it would have
  with its label in `--d`, `aria-disabled="true"`, `data-face="disabled"`, still focusable
  (kit › Faces, Disabled); Enter in a field then does nothing.
- **Test hooks:** each action button carries `data-face` (`off` / `chosen` / `disabled`) and the
  destructive one `data-hue="ending"` (kit › Faces, Stage.md D41); the panel
  `data-prompt="store|unsavedChanges|soundNames|deleteRack|renameRack"`; the cancelling
  button `data-cancel` (what Esc presses).
- **One send per answer (PR-D26):** once a prompt has called `send`, its buttons are disabled
  (`aria-disabled`, `data-face="disabled"`, the label `--d`) and Enter in a field does nothing,
  until the next state arrives or it unmounts, so a double click never sends twice (a second
  `toggleQuickRackStore` would re-arm Store). "The next state": the wiring re-runs `pickPrompt`
  on every state event (the root `app.state` object is replaced on each one,
  `app/src/lib/store.svelte.ts` `AppStore.apply`; `share()` keeps unchanged subtrees, so it's
  the root that is new;
  `pickPrompt` returns fresh objects), so the prompt's data props get a new identity; an
  `$effect` on them re-enables the buttons. A refusal is a state event (`message` changes), so
  a refused save re-enables Store's buttons for a retry. While disabled this way, a
  destructive button's label is `--d` too and its `data-hue` is `d` (the hue follows the
  drawn colour, kit D41); Esc presses the disabled Cancel, which sends nothing, and the
  overlay still calls `onescape`. The app-only prompts (Delete, Rename) call `onclose` right
  after their send instead, so they just close.

**Props (every prompt component).** Its data (per prompt below), `send(cmd: AppCmd)` for
each command named, `onclose()` (the app-only prompts only, Delete and Rename: called when
they dismiss themselves with no command to send, their Cancel, and right after their own
send; the session's prompts, Store, Unsaved and Sound names, have no `onclose` and close
when the state stops asking), and `value?: string` the initial text of its name field
(default: the suggestion each prompt names below; Sound names takes `values?: string[]`, by
field). Prompts don't focus anything themselves. `PromptOverlay` takes `message:
AppState['message']` (for the status line), `kind: string | null` (the `data-prompt` of the
dialog inside it; the Board story passes `"board"`), `children` (a snippet: the one dialog,
or the Board story's grid), `autofocus: boolean` (default true; stories pass false, PR-D5)
and `onescape()` (called after Esc pressed the cancel button; the wiring maps it to
`tips.hide()`); it has no other input.

**Props of the primitives.** `NameField`: `id: string` (the input's id; the label's `for`),
`label: string`, `tag?: { text: string; hue: 'r1' | 'r2' | 'r3' | 'l' }` (the part tag),
`value: string` (`$bindable`; the prompt owns it and derives emptiness, disabled and the
`aria-label`s from it), `tip: TipKey` (the input's tooltip), `ariaLabel: string` (the input's
`aria-label`), `onenter()` (Enter pressed in the input; the prompt runs its primary action
when that is enabled, else nothing). `BankRow`: `bank: number`, `buttons: QuickRackButton[]`,
`waiting: number | null`. `Scrim` has no props. `Dialog`'s are above.

**Keyboard and focus (PR-D5).** The overlay owns focus. On mount, and again whenever its
`kind` prop changes (Unsaved becoming Sound names), focus goes (after the children rendered,
a `tick()`) to the first `<input>` in the dialog (its text selected), else to the first
`data-face="chosen"` button (the primary-faced one; Delete's is Cancel), else to the first
`data-face="disabled"` button. Tab and Shift+Tab cycle inside the
dialog (a focus trap: the overlay listens for `keydown` on `window` in the capture phase
and wraps Tab from the last focusable to the first and back, fields in order, then the
buttons left to right; nothing outside is reachable; `mousedown` on the dialog's
non-focusable parts, its text and padding, is `preventDefault`ed like the scrim's, so a
click there never drops focus to `body`, and if focus is outside the dialog anyway when Tab
is pressed, it goes to the dialog's first focusable). Enter in a field is the field's own
(`NameField` calls `onenter`, the prompt runs its primary action when enabled; it works with
or without the overlay, so a prompt's test can press it alone); Enter or Space on a button
runs that button. Esc presses the `data-cancel` button
(Cancel, Keep editing); the overlay handles it in the same capture listener, stops
propagation and calls `onescape`, so the window handler's `ui.escape()` doesn't run and the
tooltip still hides. While a dialog is open the window key handler (`app/src/lib/shortcuts.ts`
`handleKey`, a bubble listener, which Esc never reaches) tracks Shift and does nothing else:
the wiring sets an app-only `ui.modal` flag while the overlay is mounted, and `handleKey`
checks it first; the master popover's own window listeners (`app/src/panels/mixer/MasterFx.svelte`:
`onkeydowncapture={escape}` and `onpointerdown={outside}`) both return at once while
`ui.modal` is set, so a click on the scrim or the dialog doesn't close the popover under it
(gap table). On close (the overlay unmounts), focus returns to the element
that had it before the overlay mounted (recorded by the overlay; one opener for a whole run
of prompts); if that element is gone, to `document.body`. Every focusable control shows the
kit's focus ring on `:focus-visible`, except the field (PR-D6).

**Hover and cursor:** kit (D35): no hover look, `pointer` on the buttons, `text` over a Name
field, `default` elsewhere.

**Stacking.** The wiring mounts the overlay after everything else in `App.svelte` and the
z-indexes in Layout put it above the page, the drawers, the Browser, the SoundPicker and
the master popover (an Unsaved prompt raised from the Rack drawer or Library sits over
them), and below the floating Tooltip.

### Name field

A label over an underlined input, a column with gap 4, used by Store, Sound names and Rename:

- **Label** (`<label for>` the input's id): 12 / 400, line-height 16, `--m`, sentence case
  ("Rack name", "New sound name"). With a **part tag** before it (Sound names): a row 16
  tall, items centred, gap 8, the tag "R1" / "R2" / "R3" / "L" 12 / 500, line-height 16, in
  the part's hue (`--r1 --r2 --r3 --l`, with `data-hue="r1|r2|r3|l"`), then the words.
- **Input** (`<input type="text">`): `height: 32px; box-sizing: border-box; padding: 4px 0 3px`
  (the 24px text line, 4 above, 3 below, then the 1px underline inside the box: 32), full
  width, `border: 0; border-bottom: 1px solid var(--t)`, no radius, `background: none`,
  18 / 300, line-height 24, `--t`,
  `caret-color: var(--t)`, `outline: none` (the focus ring is the underline itself: it is
  `--t` already, so the field shows focus by its caret and selection; PR-D6). `spellcheck="false"`,
  `autocomplete="off"`, `maxlength` 80. `aria-invalid="true"` while the value trimmed is
  empty, and the primary button is then disabled (kit › Faces, Disabled). Tooltip on the
  input: given by each prompt. The board's drawn caret (a 1 × 22 `--t` bar after the text) is
  the real caret in the app; the Board story doesn't focus the field, so its screenshot has
  no caret, 22 pixels under the threshold (Checks).
- **Value:** the `value` prop, else the prompt's suggested name (below), taken once when the
  prompt mounts (a later change of the suggestion while it is open is ignored; a re-render
  after a refusal keeps what was typed); selected on every focus so typing replaces it. Sent
  trimmed. An empty trimmed value disables the primary; the session's own refusals (a name
  another rack has) come back on the status line and the prompt stays open (PR-D7).

### Bank row

The Quick Racks bank on view as eight cells, used by Store: `role="img"`, 44 tall, `display:
grid; grid-template-columns: repeat(8, minmax(0, 1fr)); gap: 6px` (cells 43.5 × 44 in the
390 content width). Each
cell: radius 4, `box-sizing: border-box`, `position: relative`, padding `6px 0 0 7px`, the
slot label (`quickLabel(bank, i)`: "A1"…"A8") 13 / 500, line-height 16. Faces from
`quickRacks.buttons[i]` and `quickRacks.storeWaiting`:

| Cell | When | Fill | Border | Label | `data-face` |
|---|---|---|---|---|---|
| Loaded | `loaded` | `--t` (chosen face) | none | `--g` | `chosen` |
| Stored | `rack` not null, not loaded, not missing | `--btn` | none | `--t2` | `off` |
| Missing | `missing`, not loaded | `--btn` | none | `--warn` (PR-D8) | `off` |
| Empty | `rack` null | `--btn` | none | `--d` | `off` |
| Waiting (the slot asked for) | `i === storeWaiting` | `--btn` | 1px `--t` (padding becomes `5px 0 0 6px`) | `--t`, plus "here" 11 / 400, line-height 14, `--t2`, `position: absolute` at left 6, bottom 4 inside the border (7 and 5 from the cell's edge) | `waiting` |

The When column is exclusive, read with this rank: the waiting cell outranks the others (a
stored slot being stored over still reads "here"), then loaded outranks missing (the live rack whose
file is gone, `src/session/quick_racks.rs` sets both; it reads "loaded" and the Store body
says the file is gone, PR-D27). The waiting cell's drawing (`--btn` fill with a `--t` edge)
is not the kit's Waiting face (transparent); it is the bank row's own, a named exception
(Kit additions, PR-D29). Each cell carries `data-slot="{i}"`, its `data-face` above and, on
its label, `data-hue="g|t2|warn|d|t"` for the five rows in order. `storeWaiting` is a slot of the bank
on view: when the bank changes (a Launchkey Bank ± while the prompt is up) the state reads
null for the other bank, the prompt closes, and it comes back when that bank is on view
again (PR-D25). `aria-label`
template: "Bank {letter}: " then the cells grouped into runs of the same state, each run
"{first} {state}" or "{first} to {last} {state}" with the states "loaded", "stored",
"missing", "empty" and the waiting cell "chosen", joined by ", " ("Bank A: A1 to A4 stored,
A5 chosen, A6 to A8 empty"). A pure function `bankRowLabel(bank, buttons, waiting)`. The
cells are `aria-hidden="true"` (the row's `aria-label` is the whole reading, and an empty
cell's `--d` label is under AA on `--btn`, 2.1:1, as the kit's dimmed text is everywhere;
axe skips hidden text). Not a control; no tooltip.

## Prompts

Which one shows, when several could (one at a time, the first that applies; PR-D9):

1. **Sound names** when `liveRack.prompt.kind` is `soundNames`.
2. **Unsaved changes** when `liveRack.prompt.kind` is `unsavedChanges`.
3. **Store** when `quickRacks.storeWaiting` is not null.
4. **Delete** when the app-only `ui.prompt` is `{ kind: 'deleteRack', id }`.
5. **Rename** when `ui.prompt` is `{ kind: 'renameRack', id }`.

The choice is a pure function, `pickPrompt(state: AppState, uiPrompt: UiPrompt | null)` in
`app/src/pages/prompts.ts`, returning `{ kind, props }` (the data props of that prompt's
component, listed with each prompt) or null. `UiPrompt` lives in `app/src/lib/store.svelte.ts`:
`{ kind: 'deleteRack' | 'renameRack'; id: string }`, kept in `ui.prompt`. 1–3 are the
session's questions and close when the state no longer asks them (the session clears
`liveRack.prompt` on `dismissRackPrompt`, on a load or a save; it clears `storeWaiting` on
the save, on a load, on `dismissRackPrompt` and on disarming Store). 4–5 are the app's own
and close when their action is sent or cancelled (`ui.prompt = null`); a session prompt
arriving while one is up takes its place and the app prompt comes back once the session's
is answered (`ui.prompt` is kept). The overlay stays mounted across such changes (one focus
return target, PR-D5). The hardware never opens one and keeps working while one is up (the
band plays on): a Launchkey rack switch during the Unsaved prompt goes ahead with
"Recovered: <name>" and the prompt closes because the state cleared it.

A prompt's name comes from the state, never from what the opener remembered: Store reads the
rack's name and the slot from `liveRack` and `quickRacks`; Unsaved from `liveRack.prompt.then`;
Delete and Rename find the `racks` entry whose `id` is theirs (`racks` is a list; "the rack"
below means that entry; none: `pickPrompt` returns null for it, and the wiring's `$effect`
sets `ui.prompt = null` whenever `ui.prompt` is set and no `racks` entry has its `id` (the
wiring checks `racks` itself, since `pickPrompt` reports only the top prompt and a session
prompt may be over the app's), so the prompt closes, PR-D10).

### Store (A)

`data-prompt="store"`, `role="dialog"`, title id `dlg-store`. Shown while
`quickRacks.storeWaiting` is not null: an armed Store tap landed on button `storeWaiting` of
the bank on view, and the live rack has to be saved before it goes there (app-api.md ›
Quick Racks). The session also put "Save the rack first; then it goes on Quick Rack A5" on
the status line at that moment, so it reads under the prompt: intended, and in the
single-prompt fixture (Board fixture). Data props: `bank`, `buttons` (the eight
`QuickRackButton`s), `slot` (`storeWaiting`), `rack: { name, id, modified }` (from
`liveRack`).

| Part | Reads | Text / face |
|---|---|---|
| Title | `bank`, `slot` | "Store to Quick Rack {label}?" ("Store to Quick Rack A5?") |
| Body, never saved (`rack.id` null) | — | "An armed tap waits for Save while the live rack is modified or never saved. This rack has never been saved, so it needs a name first. Storing over a stored slot doesn't ask: the rack it held stays in your racks." (PR-D11) |
| Body, saved and modified (`rack.id` set, `modified`) | `rack.name` | "An armed tap waits for Save while the live rack is modified or never saved. {name} has unsaved changes, so it is saved first. Storing over a stored slot doesn't ask: the rack it held stays in your racks." with the name in `--t` |
| Body, saved, unmodified (`rack.id` set, not `modified`: its file is gone, the one other reason the session waits) | `rack.name` | the same with the middle sentence "{name}'s saved rack is gone, so it is saved again." (PR-D27) |
| Bank row | `quickRacks` | Bank row above |
| Name field, never saved only | `rack.name` | label "Rack name"; value `rack.name` ("New rack" for a new one; PR-D12); tooltip `quick.save_name`; the input's `aria-label` "Rack name" |
| Cancel | — | off face, `data-cancel`; sends `toggleQuickRackStore` (Store disarms and the waiting button is let go, as the API says; the board's "leave Store armed" is wrong, PR-D13); tooltip `quick.cancel_store` |
| Save and store | the field | primary; never saved: `saveRackAs { name }` when the trimmed field differs from `rack.name`, else `saveRack` (PR-D32); saved: `saveRack`. The session stores the saved rack on the waiting button itself. Tooltip `quick.save` (retitled by C1) |

`aria-label`s: Cancel "Cancel: nothing is saved or stored", Save and store "Save the rack
as {name} and store it to Quick Rack {label}" with `{name}` the field's current trimmed
value (empty: "Save the rack and store it to Quick Rack {label}"; saved: "Save {name} and
store it to Quick Rack {label}" with `liveRack.name`). A `saveRackAs` the session refuses (an empty name can't be sent; a name
another rack has) shows on the status line and the prompt stays (the state still waits).
If the save needs sound names, the Sound names prompt takes over (rule 1) and, once named,
the save completes and the store follows.

### Unsaved changes (B)

`data-prompt="unsavedChanges"`, `role="dialog"`, title id `dlg-unsaved`. Shown while
`liveRack.prompt` is `{ kind: 'unsavedChanges', then }`: `loadRack`, `newRack`, `pressQuickRack`,
`stepQuickRack` or a screen One Touch onto a rack (Stage.md C2) would lose the live rack's
changes. Data props: `rack: { name, id }` (from `liveRack`), `then` (`RackSwitch`).

| Part | Reads | Text / face |
|---|---|---|
| Title | `rack.name` | "Unsaved changes in {name}" |
| Body | `then` | `load`: "Save them before switching to {then.name}?"; `new`: "Save them before starting a new rack?"; the name in `--t` |
| Note (12px) | `rack.name` | "Only the screen asks. The Launchkey, pedals and OTS Link switch at once and keep the edits as Recovered: {name}, a rack of yours in Library › Racks." with "Recovered: {name}" in `--t` (three lines at 390px for the fixture; PR-D14: the board's wording named One Touch and Quick Rack pads as never asking; from the screen they do, until Stage.md C2) |
| Name field, never saved only (`rack.id` null) | `rack.name` | label "Rack name", value `rack.name`; tooltip `rack.save_as_name` (PR-D12); the input's `aria-label` "Rack name" |
| Keep editing | — | off face, first, `data-cancel`; sends `dismissRackPrompt`; tooltip `rack.keep_editing` |
| Discard and switch | `then` | destructive, second, the label the same for `new` (only the aria-label changes); `load`: `loadRack { id: then.id, discard: true }`; `new`: `newRack { discard: true }`; tooltip `rack.discard_switch` |
| Save first | the field | primary, last; `rack.id` set: `saveRack`; null: `saveRackAs { name }` when the trimmed field differs from `rack.name`, else `saveRack` (PR-D32). The session holds the switch and makes it once saved (through the Sound names prompt if one comes); the app never sends the switch itself. Tooltip `rack.save_first` |

`aria-label`s: Keep editing "Keep editing: stay on {name}", Discard and switch "Discard the
changes and switch to {then.name}" ("…and start a new rack"), Save first "Save {name} first,
then switch to {then.name}" ("…then start a new rack"), `{name}` the field's current trimmed
value when there is a field, else `rack.name`. Esc is Keep editing.

A Save first the session refuses (a name another rack has, or a save that fails) closes the
prompt: the session drops the question with the held switch (`save_rack_holding` in
`src/session/rack_cmds.rs`), the refusal shows on the status line, the rack keeps its
changes, and the player does the switch again (PR-D7). Only a save that needs sound names
keeps the switch, through the Sound names prompt.

### Sound names (C)

`data-prompt="soundNames"`, `role="dialog"`, title id `dlg-names`. Shown while
`liveRack.prompt` is `{ kind: 'soundNames', parts, saveAs }`: a save found edited presets that
become new sounds of the user's and need names. One Name field per entry of `parts`, in
order, 16px apart. Data props: `parts: { part, partName, suggested, origin }[]` and `saveAs`,
where `pickPrompt` fills `partName` ("Right 1", "Right 2", "Right 3", "Left"), `suggested`
(the prompt's, or `keyboardParts[part].voiceName` when the prompt's is empty, PR-D28) and
`origin` from `keyboardParts[part].sound.id` (app-api.md › `listPluginPresets`):
`au:…#f:…` → `"Factory"`, `au:…#u:…` → `"File"`, anything else or no `sound` → null
(PR-D15).

| Part | Reads | Text / face |
|---|---|---|
| Title | `parts.length` | "Name the new sound"; two or more: "Name the new sounds" |
| Body | `parts[]` | one sentence per part: "{partName}'s {suggested}{ · origin} was edited." joined by a space, then "Saving the rack keeps it as a new sound in My Sounds; the preset stays as it was." (two or more: "…keeps them as new sounds in My Sounds; the presets stay as they were."). `{suggested}` in `--t`; " · {origin}" 12px `--m`, left out with its dot when `origin` is null |
| Name field × n | `parts[i]` | label: the part tag (`R1` … `L` in the part hue) then "New sound name"; value `suggested`; tooltip `rack.sound_name` |
| Cancel | — | off face, `data-cancel`; sends `dismissRackPrompt` (nothing saved; a switch that was waiting on this save is dropped too, as the API says); tooltip `rack.cancel_save` |
| Save sound | the fields | primary; "Save sounds" for two or more; `saveAs` null: `saveRack { soundNames }`, else `saveRackAs { name: saveAs, soundNames }`, `soundNames` keyed by `part` with each field's trimmed value; disabled while any field is empty. Tooltip `rack.save_names` (retitled by C1) |

A Save sound the session refuses (a `saveAs` name another rack has, or a save that fails)
closes this prompt: the session takes `liveRack.prompt` on any save (`save_rack_holding`,
`src/session/rack_cmds.rs`), so the typed names are gone, the refusal shows on the status
line, and the prompt that started the save shows again if it still asks (Store, while
`storeWaiting` is set; the Unsaved prompt's held switch is dropped, as above). The player
retries from there (PR-D7).

`aria-label`s: Cancel "Cancel: don't save the rack", Save sound "Save sound: keep the edited
sound as {name} in My Sounds, then save the rack" (plural: "Save sounds: keep the edited
sounds as {names} in My Sounds, then save the rack"), `{name}` each field's current trimmed
value and `{names}` the values joined as a list ("A and B", "A, B and C"); an empty field
reads "(no name)" in the list. The field's `aria-label` "{Part name} new sound name".

### Delete (D)

`data-prompt="deleteRack"`, `role="alertdialog"`, title id `dlg-delete`. App-only: opened by
a Delete… control with `ui.prompt = { kind: 'deleteRack', id }` (Library › Racks today:
`RacksTab.svelte`'s `askDelete`, which already refuses the loaded rack and keeps doing so;
its own spec later). Data props: `rack: { id, name }`, `slots: string[]` (the labels of the
bank-on-view buttons whose `rack` is `id`, in slot order), `ots: number[]` (1-based OTS of the
loaded style whose `ots.racks[i].rack` is `id`), `styleName` (`style.name`).

| Part | Reads | Text / face |
|---|---|---|
| Title | `rack.name` | "Delete {name}?" |
| Body | `slots`, `ots`, `styleName` | sentences, each only when it applies, joined by a space: slots → "Quick Rack {label} will be empty." (two: "Quick Racks A4 and A7 will be empty."; more: "Quick Racks A2, A4 and A7 will be empty."); OTS → "One Touch {n} of {styleName} goes back to the style's own." (several: "One Touch 2 and 3 of … go back to the style's own."); always "The sounds stay in your Library." The slot labels and the style name in `--t`. Slots in other banks and other styles' OTS aren't in the state, so they go unnamed (PR-D16). When the two first sentences both apply they join with ", and " and one full stop, as the board: "Quick Rack A4 will be empty, and One Touch 3 of Sunday Drive Pop goes back to the style's own. The sounds stay in your Library." |
| Cancel | — | the default: primary face, first, `data-cancel`; calls `onclose`; tooltip `library.rack_delete_cancel` |
| Delete | `rack.id` | destructive, last; sends `deleteRack { id }` then calls `onclose`. The dialog doesn't second-guess the session: sent for the loaded rack anyway, the refusal lands on the status line (the board's example; PR-D17). Tooltip `library.rack_delete_confirm` |

Lists in the body and the labels are "A4", "A4 and A7", "A2, A4 and A7"; "One Touch 3",
"One Touch 2 and 3", "One Touch 1, 2 and 3" (the body's verb agrees with its subject:
"goes" for one OTS, "go" for more); a pure function `listOf(items)`.

`aria-label`s: Cancel "Cancel: keep {name} (default)", Delete "Delete {name}: empties Quick
Rack {labels} and returns One Touch {ns} to the style's own" ("Quick Racks" with two or more
slots; "empties" and "returns" always, the subject is Delete), each clause only when it
applies, joined by " and " ("Delete Ballad: empties Quick Rack A4 and returns One Touch 3 to
the style's own"; nothing held: "Delete Ballad"). Esc is Cancel. Initial focus: Cancel.

### Rename (E)

`data-prompt="renameRack"`, `role="dialog"`, title id `dlg-rename`. App-only, not on the
board (the issue names it; PR-D18): opened by a Rename control with `ui.prompt = { kind:
'renameRack', id }`. The Name prompt anatomy: a title, a Name field, Cancel and a primary.
Height 190 (title 24, field 52, buttons 32, two gaps 32, padding 48, border 2; no body).
Data props: `rack: { id, name }`.

| Part | Reads | Text / face |
|---|---|---|
| Title | `rack.name` | "Rename {name}" |
| Name field | `rack.name` | label "Rack name", value the name; tooltip `library.rack_rename_name` (new); the input's `aria-label` "New rack name" |
| Cancel | — | off face, `data-cancel`; calls `onclose`; tooltip `library.rack_rename_cancel` (new) |
| Rename | the field | primary; sends `renameRack { id, name }` then calls `onclose`; disabled while empty or unchanged (trimmed value equals the name); tooltip `library.rack_rename` (new) |

`aria-label`s: Cancel "Cancel: keep the name {name}", Rename "Rename {name} to {new name}".
Quick Rack buttons keep the rack (the id stays). A refused name (one another rack has)
shows on the status line after the prompt closed; the opener can reopen it.

## States

| State | What changes |
|---|---|
| No prompt | nothing is mounted: no scrim, no dialog; the status line is the page's own (kit) |
| Store, never saved (the board) | as drawn, with the Name field |
| Store, saved and modified | no Name field; the body's second sentence names the rack; height 278 (a four-line body) |
| Store, slot stored over | the waiting cell is the one that held a rack: still the waiting face, "here" |
| Unsaved, switch to a rack (the board) | as drawn |
| Unsaved, new rack | body "…before starting a new rack?"; Discard sends `newRack { discard: true }` |
| Unsaved, never saved | a Name field under the note; Save first sends `saveRackAs` |
| Sound names, one part (the board) | as drawn |
| Sound names, several parts | plural title, body and primary; one field per part |
| Delete, no slot and no OTS | body "The sounds stay in your Library." only; height 158 |
| Delete, several slots or OTS | the plural sentences |
| Rename | the Name prompt |
| A field empty | its `aria-invalid`, the primary disabled (`--d` label, `aria-disabled`) |
| Refusal from the session | the status line shows it over the scrim; the prompt stays if the state still asks |
| Prompt cleared by the state (a hardware switch, a save that went through) | the overlay unmounts; focus returns |
| Message null | the status line is empty (kit); the scrim and dialog as before |
| Light theme | the same markup; `--scrim` and `--dialog-edge` change (Kit additions); nothing glows |

## Board fixture

The board is four prompts at once over the Stage: a review layout, not a state the app can be
in (PR-D1). So the fixture is in two parts, both in `app/src/ui/Prompts/Prompts.fixtures.ts`:

- **`stageState`:** Stage.md's `boardState` with `message` set to `{ seq: 7, text: "Sunday
  drive is loaded: load another rack before deleting it", error: true }`, the session's own
  refusal (`src/session/rack_cmds.rs`); the board's "Can't delete Sunday drive: it's the
  loaded rack." is placeholder text (PR-D19) and the screenshot masks the status line. The
  clock is Stage.md's (`boardNow = 10000`, the same anchors, LED phase 0.25) and
  `boardMeterHolds` the same, so the Stage under the scrim is the Stage board.
- **Per-prompt states**, each an `AppState` made from `stageState` with the fields below
  merged over it (`liveRack` over `stageState.liveRack`, so `controls` stays), plus a
  `uiPrompt`. `pickPrompt` on each gives the props the stories and the vitest checks use.
  Every one has `racks` = four entries, in this order: `{ id: "rack-sunday", name: "Sunday
  drive" }`, `rack-warm` "Warm keys", `rack-organ` "Organ trio", `rack-ballad` "Ballad",
  each with `parts` ["Stage Grand", "Silk Strings", "Brass Section", "Silk Strings"], `on`
  [true, true, false, true], `needsAttention` false; and `quickRacks` bank 0, `store` false,
  `readOnly` false, buttons A1–A4 holding those four ids in that order with their names
  (`missing` false; so A4 is Ballad, as the board's Delete says), A5–A8 `{ rack: null, name:
  "", missing: false, loaded: false }`; `loaded` is true on the button whose id is
  `liveRack.id`.
  - `storeState`: `quickRacks.storeWaiting` 4, `store` true; `liveRack` `{ name: "New rack",
    id: null, modified: true, prompt: null }` (so no button is loaded); `message` `{ seq: 8,
    text: "Save the rack first; then it goes on Quick Rack A5", error: false }` (what the
    session says on the armed tap). Props: `bank` 0, `slot` 4, the buttons, `rack` `{ name:
    "New rack", id: null, modified: true }`; the story's `value` "Rhodes Soft + Strings" (the
    name a `storeRack` would make, `app/src/lib/api/mock-quick-racks.ts` `capture`; the
    component's default is "New rack", PR-D12). The board draws A1 loaded; with `liveRack.id`
    null nothing is loaded, so the fixture draws A1 stored and the screenshot masks cell A1
    (PR-D20).
  - `unsavedState`: `liveRack` `{ name: "Sunday drive", id: "rack-sunday", modified: true,
    prompt: { kind: "unsavedChanges", then: { kind: "load", id: "rack-warm", name: "Warm
    keys" } } }`. Props: `rack` `{ name: "Sunday drive", id: "rack-sunday" }`, `then` as above.
  - `soundNamesState`: `liveRack` as `unsavedState`'s but `prompt` `{ kind: "soundNames",
    parts: [{ part: 0, suggested: "Rhodes Soft" }], saveAs: null }`; `keyboardParts[0]`
    `soundEdited` true, `sound` `{ id: "au:aumu Smp7 Fake#f:12", name: "Rhodes Soft" }` (a
    factory preset of the mock plugin, whose component id is `aumu Smp7 Fake`,
    `app/src/lib/api/mock-plugins.ts`; so the origin reads "Factory") and `plugin` `{ id:
    "aumu Smp7 Fake", name: "Sampler Deluxe", manufacturer: "Fake Instruments", status:
    "playing", stage: null, error: null, outOfProcess: true, inProcessFallback: false, cpu:
    0.02, overruns: 0, recentOverruns: 0, editor: true, missing: false, preset: "Rhodes
    Soft", presetKey: "f:12" }` (every required `PartPlugin` field, `app/src/lib/api/types.ts`) (the session asks names only for an edited plugin sound, so the part plays one;
    `pickPrompt` reads only `sound` and `voiceName`). Props: `parts`
    `[{ part: 0, partName: "Right 1", suggested: "Rhodes Soft", origin: "Factory" }]`,
    `saveAs` null; the story's `values` `["Rhodes Soft 2"]` (the board's; the component's
    default is "Rhodes Soft", PR-D21).
  - `deleteState`: `uiPrompt` `{ kind: "deleteRack", id: "rack-ballad" }`; `liveRack` `{ name:
    "Sunday drive", id: "rack-sunday", modified: true, prompt: null }` (A1 loaded);
    `ots.racks` four entries, `[2]` = `{ rack: "rack-ballad", name: "Ballad", missing: false }`
    and the others `{ rack: null, name: "", missing: false }`; `style.name` "Sunday Drive Pop".
    Props: `rack` `{ id: "rack-ballad", name: "Ballad" }`, `slots` ["A4"], `ots` [3],
    `styleName` "Sunday Drive Pop".
  - `renameState`: `deleteState` with `uiPrompt` `{ kind: "renameRack", id: "rack-ballad" }`.
    Props: `rack` `{ id: "rack-ballad", name: "Ballad" }`.

The stories render the prompt components with those props, the board's typed field values
as `value` / `values`, and no focus (the overlay's `autofocus` false, PR-D5).

The `Pages/Prompts` › `Board` story renders `PromptOverlay` (with `stageState.message`,
`kind: "board"`, `autofocus: false`) over `Stage` with `stageState` and `statusHidden: true`
(Kit additions; so one status line shows), and in place of the one dialog the four prompts
in the board's grid (the grid is the centring box's one item): `grid-template-columns: 440px
440px; gap: 24px; align-items: start`, centred in `0,0 1440×800` (so the grid's left is 268
and its top 82). The light board's `--m` and `--lamp-ink` differ from the tokens as on the
Stage (Stage.md › Board fixture); no mask.

## Components

Build order; each gets `app/src/ui/<Name>/` with a SPEC.md per `docs/factory/spec-template.md`.
Board lines are dark / light (the light file is 8 lines shorter at the top); crop boxes are
the boxes in this spec. Kit and Stage names are reused; new ones are marked.

| # | Component | Kind | Built from | Exists | Board lines (dark / light) | Spec |
|---|---|---|---|---|---|---|
| 0 | tokens | — | — | yes; add `--scrim`, `--dialog-edge`, `--ending-ink` | 398, 407 / 390, 399 | Kit additions |
| 1 | Button (Stage.md #3) | primitive | longpress | no | 428–429, 439–441, 457–458, 467–468 / −8 | kit › Faces; add `tone: 'primary' \| 'destructive'` (Kit additions); its API in Dialog panel › Actions |
| 2 | StatusLine (Stage.md #15) | primitive | — | no | 401 / 393 | kit › Status line; Kit additions (over the scrim) |
| 3 | Scrim (new) | primitive | — | no | 398 / 390 | Scrim |
| 4 | NameField (new) | primitive | — | no | 420–426, 449–455 / 412–418, 441–447 | Dialog panel › Name field |
| 5 | BankRow (new) | primitive | — | no | 410–419 / 402–411 | Dialog panel › Bank row |
| 6 | Dialog (new) | complex | Button | no | 407–431 (A) / 399–423 | Dialog panel: title, body, note, children, actions |
| 7 | StorePrompt (new) | complex | Dialog, BankRow, NameField | no | 407–431 / 399–423; crop `268,82 440×346` | Prompts › Store |
| 8 | UnsavedChangesPrompt (new) | complex | Dialog, NameField | no | 434–443 / 426–435; crop `732,82 440×222` | Prompts › Unsaved changes |
| 9 | SoundNamesPrompt (new) | complex | Dialog, NameField | no | 446–460 / 438–452; crop `268,452 440×266` | Prompts › Sound names |
| 10 | DeleteRackPrompt (new) | complex | Dialog | no | 463–470 / 455–462; crop `732,452 440×198` | Prompts › Delete |
| 11 | RenameRackPrompt (new) | complex | Dialog, NameField | no | not on the board | Prompts › Rename |
| 12 | PromptOverlay (new) | complex | Scrim, StatusLine, Dialog | no | 397–404, 471–472 / 389–396, 463–464 | Layout, Scrim, keyboard and focus |
| 13 | Prompts (`app/src/ui/Prompts/`: no component file; `Prompts.fixtures.ts`, `Prompts.stories.svelte` with the `Pages/Prompts` › `Board` story, `crops/` and SPEC.md) | — | PromptOverlay, the five prompts, Stage | no | whole board | this file |

Components take props and call callbacks (`send`, `onclose`, Dialog panel › Props); none
reads `app.state` or `app.send`. The wiring (`app/src/pages/PromptsWiring.svelte`, outside
`app/src/ui`) reads `app.state` and `ui.prompt`, picks the prompt with `pickPrompt`, passes
the props, maps `send` to `app.send` and `onclose` to `ui.prompt = null`, clears `ui.prompt`
when `pickPrompt` gives null for it (PR-D10), and sets `ui.modal` while the overlay is
mounted (clears it on unmount). `App.svelte` mounts it last (Dialog panel › Stacking), as
a sibling of the Stage.md D1 scaler, not inside it: the overlay's layer carries its own copy
of the scale (Layout, PR-D2) and the scrim is unscaled.

## Gap against today

| Area | In `app/src` now | Change |
|---|---|---|
| Store waiting | `panels/quickracks/RackPrompt.svelte`: an inline strip in the Quick Racks bar (`Save rack` / `Cancel`, an input for a never-saved rack), mounted by `panels/quickracks/QuickBar.svelte` and `panels/knobracks/KnobRackPanel.svelte` (both also read `asking()`) | Replaced by the Store prompt; `RackPrompt.svelte` and its `asking()` go, and QuickBar, KnobRackPanel and their tests drop them |
| Unsaved changes, sound names | `panels/rack/RackPanel.svelte` asks them inside the Rack drawer; `panels/quickracks/rackPromptDrawer.svelte.ts` opens the drawer when a prompt appears | Replaced by the Unsaved and Sound names prompts over the page; the drawer no longer opens on a prompt (`openRackDrawerOnPrompt` goes); the Rack panel's prompt UI goes |
| Delete | `panels/library/RacksTab.svelte`: an inline confirm (`confirming`) with `library.rack_delete_confirm` / `_cancel` | Replaced by the Delete prompt: Delete… sets `ui.prompt = { kind: 'deleteRack', id }`; the inline confirm goes |
| Rename | `RacksTab.svelte`: the name input renames on Enter (`library.rack_name`) | The inline rename stays until the Library › Racks spec decides; the Rename prompt is built and storied for it (PR-D18) |
| Modal state | none (`ui` has no modal flag; `handleKey` runs everywhere but inputs; `app/src/panels/mixer/MasterFx.svelte` has its own window capture Esc listener and an outside-click `pointerdown` listener) | `ui.prompt` and `ui.modal` in `app/src/lib/store.svelte.ts`; `handleKey` returns after tracking Shift when `ui.modal` is set; MasterFx's `escape` and `outside` listeners return when it is set |
| Status line | the footer (`App.svelte`'s `<footer role="status" tabindex="0">`, shown on every page; the Stage lane replaces it with the kit's status line, Stage.md › Gap › Footer) | The kit's status line; drawn over the scrim by the overlay while a prompt is up, and the page's own copy hidden meanwhile (Kit additions; `StageWiring` passes `statusHidden` from `ui.modal`). If the old footer is still in `App.svelte` when this lane builds, it gets the `hidden` attribute while `ui.modal` is set, so one live region speaks on every page (PR-D23) |
| Dialogs | `panels/rack/RackSlot.svelte` has a `role="dialog"` popover (insert settings) with the old `.pop mat-raised` look | Untouched here; the Dialog panel is the Push dialog, for the prompts (and Prompts-More, #524) |
| Screenshot tool | `app/scripts/shots.ts` (Stage.md › Gap: per-story viewport and masks) | The Board story needs the same item (viewport 1440 × 900, masks); built with the Stage, reused here |

## Contract changes needed

C1 blocks the build: without its keys `TipKey` doesn't type-check and the tooltip catalog test
fails. C2 doesn't block.

1. **C1 · Tooltips** (`app/src/help/tooltips.ts`, `app/docs/controls.md`), **lands before the
   build**: new keys `library.rack_rename` ("Rename": renames the rack; Quick Rack buttons keep
   it), `library.rack_rename_cancel` ("Cancel": keeps the name) and `library.rack_rename_name`
   ("Rack name": the new name; Enter renames, Esc closes the prompt); `quick.save` retitled
   "Save and store" (body as now); `rack.save_names` retitled "Save sound" (body: saves each
   edited preset as a new sound under the name you gave, then the rack); `rack.cancel_save`
   adds that a switch or a store waiting on the save is dropped; `rack.save_first` and
   `rack.discard_switch` cover the new-rack case ("…or starts the new rack"); `library.rack_new`,
   `library.rack_load` and the racks list body say "asks first" without "in the Rack panel";
   `quick.cancel_store` unchanged (it already says Store disarms). Until it lands the Rename
   prompt can't be built and the retitled keys read their old titles.
2. **C2 · Undo a store** (`AppCmd`, `src/session`, both mocks, `EVERY_CMD`, docs): the board's
   "the old rack stays in My racks as Previous, with Undo". The API keeps the old rack under
   its own name and has no undo; nothing is lost, so this is a follow-up, not a block. Until
   then the Store body says "the rack it held stays in your racks" (PR-D11).

## Checks

Vitest (`npx vitest run` on the prompt and overlay tests), against the per-prompt fixtures
unless it says otherwise. They read roles, names, attributes, `data-face` / `data-hue` /
`data-prompt` / `data-slot` and the commands sent (a fake `send`); never computed colours or
layout.

1. `pickPrompt` (pure): on `soundNamesState` plus `storeWaiting` 4 it returns kind
   `soundNames` with the props above; on `storeState`, `store`; on `deleteState`,
   `deleteRack` with `slots` ["A4"] and `ots` [3]; on `deleteState` with an id not in `racks`,
   null; on `stageState`, null. The wiring (its store mocked with `stageState`) mounts no
   dialog and no scrim for null, and with `ui.prompt` `{ kind: "deleteRack", id: "rack-gone" }`
   sets `ui.prompt` to null.
2. Store (`storeState`'s props): the title reads "Store to Quick Rack A5?"; the Name field's default
   value is "New rack" and, with `value` "Rhodes Soft + Strings", that; its tooltip
   `quick.save_name`; the cell `data-slot="4"` is `data-face="waiting"` and contains "here";
   `data-slot="0"` is `off`; the row's `aria-label` is "Bank A: A1 to A4 stored, A5 chosen,
   A6 to A8 empty". Clicking Save and store calls `send` with `saveRackAs { name: "Rhodes
   Soft + Strings" }`; with the field cleared the button is `aria-disabled`,
   `data-face="disabled"`, and click and Enter send nothing; Cancel sends
   `toggleQuickRackStore`; after a send every button is `aria-disabled` and Enter in the field
   sends nothing, and re-rendering with a fresh copy of the same props (a new object, as the
   wiring passes on every state event) re-enables them. With `rack` `{ id: "rack-sunday",
   name: "Sunday drive", modified: true }` there
   is no field, the body contains "Sunday drive has unsaved changes" and Save and store
   sends `saveRack`; with `modified` false the body contains "Sunday drive's saved rack is
   gone".
3. `bankRowLabel` (pure): the example above; all empty → "Bank B: B1 to B8 empty"; A1 loaded
   and waiting 0, the rest empty → "Bank A: A1 chosen, A2 to A8 empty"; A1 missing, A2 loaded,
   waiting 4, the rest empty → "Bank A: A1 missing, A2 loaded, A3 to A4 empty, A5 chosen, A6
   to A8 empty". `listOf` (pure): `["A4"]` → "A4", two → "A4 and A7", three → "A2, A4 and A7".
4. Unsaved (`unsavedState`'s props): the title "Unsaved changes in Sunday drive", the body "Save them
   before switching to Warm keys?"; three buttons in the order Keep editing (`data-face="off"`),
   Discard and switch (`data-hue="ending"`), Save first (`data-face="chosen"`); Keep editing
   sends `dismissRackPrompt`; Discard sends `loadRack { id: "rack-warm", discard: true }`;
   Save first sends `saveRack`; with `then` `{ kind: "new" }` Discard sends `newRack {
   discard: true }` and the body reads "Save them before starting a new rack?"; with
   `rack.id` null a field appears with value "Sunday drive" and Save first sends
   `saveRack` (PR-D32), and with the field changed to "Sunday 2", `saveRackAs { name:
   "Sunday 2" }`. Mounted inside `PromptOverlay`, Esc (a `keydown`
   dispatched on the focused button) sends `dismissRackPrompt`, calls `onescape`, and a
   bubble-phase `keydown` listener on `window` doesn't see it.
5. Sound names (`soundNamesState`'s props): the title "Name the new sound"; the body contains
   "Right 1's", "Rhodes Soft" and "Factory"; the field's label has the tag "R1" with
   `data-hue="r1"`; the field's default value is "Rhodes Soft" and, with `values` `["Rhodes
   Soft 2"]`, that; Save sound then sends `saveRack { soundNames: { 0: "Rhodes Soft 2" } }`;
   with `saveAs` "Warm keys 2" it sends `saveRackAs { name: "Warm keys 2", soundNames }`; with
   two parts (0 and 3, suggested "Rhodes Soft" and "Warm Pad"), two fields, the title "Name
   the new sounds", the button "Save sounds", and the command carries `{ 0: …, 3: … }`; with
   `origin` null the body has no " · "; Cancel sends `dismissRackPrompt`. `pickPrompt`: an
   `#u:` id gives origin "File", a `saved:` id null, and an empty `suggested` becomes the
   part's `voiceName`.
6. Delete (`deleteState`'s props): `role="alertdialog"`, the title "Delete Ballad?", the body "Quick
   Rack A4 will be empty, and One Touch 3 of Sunday Drive Pop goes back to the style's own.
   The sounds stay in your Library."; Cancel is `data-face="chosen"` and first, Delete
   `data-hue="ending"` and last; Delete sends `deleteRack { id: "rack-ballad" }` and then
   calls `onclose`; Cancel calls `onclose` and sends nothing; with no slot and no OTS the body
   is the last sentence only; with slots A2, A4 and A7 the body starts "Quick Racks A2, A4
   and A7 will be empty"; with OTS 2 and 3 "One Touch 2 and 3 of Sunday Drive Pop go back";
   initial focus is Cancel.
7. Rename (`renameState`'s props): the title "Rename Ballad", the field value "Ballad", Rename
   `aria-disabled` while the value is "Ballad" or empty; typing "Ballad 2" and Enter sends
   `renameRack { id: "rack-ballad", name: "Ballad 2" }` and calls `onclose`.
8. Focus: on mount the Store prompt's field has focus with its text selected; the Unsaved
   prompt's Save first has focus; with `autofocus` false nothing in the dialog has focus; Tab
   from the last button lands on the first focusable in the dialog and Shift+Tab from the
   first on the last; changing the overlay's `kind` (its children swapped from the Unsaved
   prompt to Sound names) moves focus to the new first field; focus returns to the opener
   element (a button the test focused before mounting) when the overlay unmounts. The wiring
   sets `ui.modal` while the overlay is mounted and clears it on unmount.
9. Status line over the scrim: the overlay with `message` `stageState.message` has one
   `role="status"` region reading the refusal, with the ⚠ and no button or focusable content
   inside; with `message` null it is empty. `StatusLine` with `inert` renders the text with no
   button; with `hidden` it has `aria-hidden="true"` and `data-hidden` (its CSS,
   `visibility: hidden`, is a story's to show: jsdom applies no component CSS). `Stage` with
   `statusHidden` passes it through.
10. Every interactive element has a `data-tip` in the catalog: `app/src/help/coverage.test.ts`
    gains a STATES entry for each session prompt and for Delete (`storeState`, `unsavedState`,
    `soundNamesState`, `deleteState` applied to the store, `ui.prompt` set for Delete), so
    its `untipped(document.body)` check covers them in the App; Rename, which the App can't
    open yet (PR-D18), gets `untipped()` run over its rendered component in the Rename test
    (check 7).
11. Light tokens (a text test over `app/src/ui/tokens/light.css`): `--scrim` is
    `rgba(242, 241, 238, 0.72)`, `--dialog-edge` is `var(--t)` and `--ending-ink` is
    `var(--ending)`; over `dark.css`, `--ending-ink` is `#d87070`.

**Story and screenshot checks** (real Chrome; `app/scripts/shots.ts` shoots one component
folder per run, with crops at `app/src/ui/<Name>/crops/<Story>-<theme>.png`, so each line
below is its own `npm run shots -- <Name>` and its own crop files):

- `npm run shots -- Prompts`: `Pages/Prompts` › `Board` (export `Board`, layout `fullscreen`,
  `parameters.shots = { viewport: { width: 1440, height: 900 }, mask:
  ['[data-shot-mask="when"]', '[data-shot-mask="status"]', '[data-shot-mask="keys"]',
  '[data-prompt="store"] [data-slot="0"]', '[data-shot-mask="text"]'] }`) renders the four
  prompts in the review grid over `Stage` with `stageState`, `boardNow` and
  `boardMeterHolds`, `autofocus` false, in both themes, against
  `app/src/ui/Prompts/crops/Board-dark.png` and `Board-light.png` (copies of
  `docs/design/push/png/Prompts-Dark.png` and `Prompts-Light.png`): at most 0.02 of the
  unmasked pixels differ. The masks: the count row's When item (Stage.md D5), the status
  line (PR-D19), the key strip (the board leaves the keys clear, PR-D2; 1392 × 56 px is 0.06
  of the frame, over the limit unmasked), Store's cell A1 (PR-D20) and every dialog's body
  and note (`data-shot-mask="text"` on the Dialog's body and note `<p>`s: the spec rewords
  Store's body, PR-D11, and the Unsaved note, PR-D14, so their glyphs differ from the board's
  while their line counts and so the heights stay, PR-D30). The fields' missing caret (22 px
  each) and Delete's label at 400 instead of 500 (PR-D4) need no mask.
- `npm run shots -- StorePrompt`, `-- UnsavedChangesPrompt`, `-- SoundNamesPrompt`,
  `-- DeleteRackPrompt`: `Components/StorePrompt` › `NeverSaved` (crop `268,82 440×346` of
  both boards), `Components/UnsavedChangesPrompt` › `Load` (`732,82 440×222`),
  `Components/SoundNamesPrompt` › `OnePart` (`268,452 440×266`), `Components/DeleteRackPrompt`
  › `SlotAndOts` (`732,452 440×198`): each prompt alone at its board size, `layout:
  centered`, `autofocus` false, the crop the dialog's box, each with `mask:
  ['[data-shot-mask="text"]']` (the body and note, PR-D30) and `NeverSaved` also
  `[data-slot="0"]`. The crop includes the 1px `--dialog-edge` border, which on the board
  mixes with the scrim over the Stage and in the story with the `--g` ground: at most 1% of
  the crop's pixels, inside the threshold. A crop whose height differs from the box fails as
  wrong size, so the fixture texts must wrap to the line counts in Layout: Sound names' and
  Delete's are the board's own texts at the board's own 390px; Store's body and the Unsaved
  note are reworded to the same line counts (four and three; a draft that wraps otherwise is
  cut to fit, the count is the contract), and the Board shot is the judge.
- `Components/StorePrompt` › `Saved`, `Components/UnsavedChangesPrompt` › `New` and
  `NeverSaved`, `Components/SoundNamesPrompt` › `TwoParts`, `Components/DeleteRackPrompt` ›
  `NothingHeld`, `Components/RenameRackPrompt` › `Rename`, `Components/NameField` › `Empty`
  (the invalid state) and `Focused` (`parameters.pseudo: { focusVisible: true }`, the pseudo-states
  addon as `docs/factory/spec-template.md` asks: it shows that a focused field draws no ring,
  PR-D6; the caret and selection need real focus, which `shots.ts` drops before a shot, so
  they are seen in Storybook by Inspect, with the story's play function focusing the input):
  no crop; judged by Inspect.
- `Components/Dialog` › `LongTitle` (a 60-character rack name in the title and the body): the
  title wraps on whole words and the dialog grows; nothing overflows 440px; no crop.
- axe finds no violation on any story (`aria-modal` dialogs with a labelled title).

## Decisions

- **PR-D1 · One at a time.** The board shows four prompts for review; the app shows one,
  centred, in the order under Prompts. The Board story reproduces the board as a review
  grid of four prompt components with their own props, over the Stage board fixture.
- **PR-D2 · The scrim covers the window.** In the app the scrim is the whole window, keys
  and D1's letterbox bars included (the board's note says the scrim covers all; it leaves
  the keys clear only for review). It is `position: fixed` and unscaled; the dialog and the
  status line are in the overlay's own scaled layer (Layout), a sibling of the Stage's
  scaler, never inside it. The screenshot masks the key strip.
- **PR-D3 · The scrim doesn't dismiss.** A click outside the dialog does nothing: the session
  holds the question until it's answered, and an accidental dismiss would send a command
  (Keep editing, Cancel) the player didn't mean.
- **PR-D4 · Three action tones.** Cancelling = off face; primary = chosen face, 500;
  destructive = off face with an `--ending-ink` label (PR-D31) at 400 (the board draws Discard at 400 and
  Delete at 500; one weight for one meaning). Button gains `tone`.
- **PR-D5 · Keyboard.** Focus trap, first field else primary, Enter = primary, Esc =
  cancelling action, the window key handler off while a prompt is up (`ui.modal`), focus
  returns to the opener. Standard modal behaviour; the board draws none of it. Stories and
  screenshots pass `autofocus: false`: a focused field shows its caret and selection, which
  the board doesn't draw, and a blinking caret isn't the same on every run.
- **PR-D6 · Field focus ring.** The underline is `--t` already, so a focused field shows no
  extra ring; its caret and selection show focus. The buttons keep the kit's ring.
- **PR-D7 · Refusals stay on the status line.** The prompts validate only emptiness (and, for
  Rename, an unchanged name). A name another rack has, or anything else the session refuses,
  comes back on the status line over the scrim. Whether the prompt stays follows the state:
  Store's wait survives a failed save, so it stays; a refused Save first from the Unsaved
  prompt drops the session's question (the API: a save that fails drops the held switch), so
  it closes; a refused Save sound closes Sound names the same way (the session takes the
  prompt on any save) and the typed names are gone; an app-only one (Delete, Rename) has
  closed and can be reopened.
- **PR-D8 · Missing rack cell.** A button whose rack is gone draws as stored with its label in
  `--warn`, the kit's missing colour; the cell is too narrow for the ⚠ glyph.
- **PR-D9 · Precedence.** Sound names before Unsaved before Store before the app's own: the
  session asks sound names only as the last step of a save that the other two started, so
  it's the one to answer first; Store waits until a save succeeds, so it yields to both.
- **PR-D10 · Names from the state.** A prompt looks its rack up by id when it renders, so a
  rename shows at once and a deleted rack closes the prompt.
- **PR-D11 · No "Previous", no Undo.** The API has neither; storing over a slot only changes
  which rack the button names and the old rack stays under its own name. The body says
  that; Undo is C2, a follow-up.
- **PR-D12 · Suggested rack name.** The field starts from `liveRack.name` ("New rack" for a
  new one): the state has no auto-name, and `storeRack`'s "Rhodes Soft + Strings" is made by
  the session, not sent. The fixture passes the board's name as a prop so the crop matches.
  When the Unsaved prompt's rack was never saved, Save first needs a name too, so the same
  field appears there.
- **PR-D13 · Cancel disarms.** Store's Cancel sends `toggleQuickRackStore`: the API says
  disarming lets the waiting button go, and the tooltip says the same. The board's
  "leave Store armed" aria-label is wrong.
- **PR-D14 · Who asks.** The note says only the screen asks, and names the Launchkey, pedals
  and OTS Link as switching at once: on screen a One Touch onto a rack and a Quick Rack
  button do ask (app-api.md › `recallOts`, `pressQuickRack`) until Stage.md C2.
- **PR-D15 · Origin word.** "Factory" for a factory preset (`au:…#f:…`), "File" for an
  `.aupreset` (`au:…#u:…`), nothing otherwise (a sound-names prompt comes only from edited
  plugin sounds, so those two are the cases).
- **PR-D16 · Delete names what the state knows.** The bank on view and the loaded style's
  OTS; other banks and styles aren't in `AppState`. The board's note (2) left this open.
- **PR-D17 · Delete never second-guesses.** The dialog sends `deleteRack` whatever the rack;
  the opener disables Delete… for the loaded rack, and a refusal shows on the status line.
- **PR-D18 · Rename.** The issue lists rename; the board doesn't draw it. It is the Name
  prompt anatomy (title, field, Cancel, primary) so Library › Racks and a later Save as…
  can reuse it; today's inline rename stays until that spec.
- **PR-D19 · Fixture message.** The session's real refusal text, not the board's; masked.
- **PR-D20 · Fixture bank row.** A never-saved rack has no loaded button, so A1 reads stored
  and the screenshot masks it (the board's A1 is drawn loaded).
- **PR-D21 · Fixture sound name.** The component suggests `suggested` ("Rhodes Soft"); the
  board's "Rhodes Soft 2" is what a player typed, passed as the story's value.
- **PR-D22 · Layout and heights.** The dialog is centred in the 800px above the status line,
  which the board's grid also does; heights are content + 48 + 2 and vary with wrapping.
- **PR-D23 · Status line over the scrim, inert.** It stays readable (the point of the board)
  but has no button while a prompt is up, so the dialog is the only thing to interact with.
  The page's own status line is hidden meanwhile, so one live region speaks.
- **PR-D24 · The dialog's border.** The kit draws no boxes or borders; the dialog is the one
  exception (the board draws it, and a modal needs an edge against the dimmed page). The
  kit's focus ring is skipped on the name field for the same reason (PR-D6). Both go in
  kit.md as named exceptions.
- **PR-D25 · Bank change while storing.** `storeWaiting` belongs to the bank on view, so a
  Launchkey Bank ± hides the Store prompt until that bank is back; the wait survives (the
  API says what clears it), the overlay unmounts and focus returns to the opener, and a
  name typed but not sent is gone when it comes back (the field restarts from its default).
- **PR-D26 · One send per answer.** After a prompt sends, its buttons are disabled until the
  state replies, so a double click can't re-arm Store or save twice.
- **PR-D27 · Store body for a lost file.** The session waits for a saved, unmodified rack
  only when its file is gone; the body says so instead of claiming unsaved changes.
- **PR-D28 · Empty suggestion.** The session may send an empty `suggested`; `pickPrompt`
  falls back to the part's `voiceName`, so the body and the field always have a name.
- **PR-D29 · The waiting cell.** The bank row's waiting cell keeps the board's drawing, a
  `--btn` fill with a 1px `--t` edge and "here", rather than the kit's transparent Waiting
  face: the row is a picture of the bank, not a control, and a transparent cell would
  vanish against the dialog's `--g`. Named as a kit exception; `data-face="waiting"` still
  says what it means. A loaded rack whose file is gone reads loaded, not missing.
- **PR-D31 · Destructive label colour.** `--ending` on `--btn` is 4.3:1 in dark, under AA,
  so axe would flag Discard and Delete. A new token `--ending-ink` (dark `#d87070`, light
  `var(--ending)`) is the ending red as text on a button face; `data-hue` stays `ending`.
- **PR-D32 · This lane's edits.** Every row of the gap table is this lane's work (the old
  Store strip, the Rack drawer's prompts, Library's inline delete confirm, the modal flag in
  `store.svelte.ts`, `shortcuts.ts`, `MasterFx.svelte`, `App.svelte`, `coverage.test.ts`
  and the tests of what it removes), plus the Stage additions in "Before building". The
  contract files (C1) are not.
- **PR-D30 · Masked text in the shots.** The body and note of every dialog are masked in
  the screenshot checks: the spec rewords Store's body (PR-D11) and the Unsaved note
  (PR-D14) to what the API does, so their glyphs can't match the board's. Their line counts
  are kept (the heights are the contract), so the crops still check the panel, its parts
  and its size; the words are checked by the vitest texts.
- **PR-D32 · The default name saves with `saveRack`.** A never-saved rack's Name field
  defaults to `rack.name` ("New rack"), and `saveRackAs` refuses a name another rack already
  has (`save_rack` in src/session/rack_cmds.rs). So when the trimmed field equals
  `rack.name`, Save and store and Save first send `saveRack`, which on an id-less rack picks a
  free name ("New rack 2") itself; any other name sends `saveRackAs { name }`.

## Follow-ups

- Undo a store over a slot (C2; the board's "Previous" rack).
- A screen One Touch that never asks (Stage.md C2) drops the Unsaved prompt's One Touch case.
  Until then, Discard on a switch held from a One Touch sends a plain `loadRack { discard:
  true }`, so `ots.applied` and Sync Start are skipped; only Save first keeps them.
- Delete naming slots in other banks and other styles' OTS would need them in `AppState`.
- Save rack as… (the Rack page) and Library › Racks' rename and delete controls adopt the
  Name prompt and Delete prompt here.
- Prompts-More (#524): the plugin-preset save, the overwrite choice and the style-file card,
  on this Dialog panel.
- A change list in the Unsaved prompt (what differs) needs state the session doesn't publish.

## Kit additions

For kit.md (not edited here):

- **Tokens:** `--scrim` (dark `rgba(0, 0, 0, 0.62)`, light `rgba(242, 241, 238, 0.72)`),
  `--dialog-edge` (dark `color-mix(in srgb, var(--t) 40%, transparent)`, light `var(--t)`)
  and `--ending-ink` (dark `#d87070`, light `var(--ending)`: the ending red as text on
  `--btn`, PR-D31).
- **Button:** a `tone` prop, `'primary'` (chosen face, 500) and `'destructive'` (off face,
  `--ending-ink` label), beside the `md` variant; its API (Dialog panel › Actions).
- **Status line, over a prompt:** while a prompt is up the status line is drawn above the
  scrim (the overlay renders it) and is inert: plain text, no `clearMessage` button, not
  focusable (PR-D23). `StatusLine` gains two props: `inert` (the text with no button) and
  `hidden` (`aria-hidden`, `visibility: hidden`, the space kept), which the page wiring
  passes from `ui.modal` so the page's own copy is silent meanwhile (`Stage` gains a
  `statusHidden` prop that passes it through). It carries `data-shot-mask="status"`. This board is where the kit's status line is drawn (Stage.md
  Components #15 points here): dark line 401, light 393.
- **Status line mismatches on this board:** the board draws it as a `<p>` with no button
  (the kit's has one when there is a message; inert here by PR-D23, so no mismatch while a
  prompt is up, but the kit should say the button is the page's, not the overlay's); the
  board's ⚠ is a 12px inline SVG where the kit says a 12px `--warn` ⚠ glyph (the glyph
  wins); left and right 24, bottom 80 (`24,800 1392×20`) as the kit's box.
- **Key strip:** carries `data-shot-mask="keys"` so an overlay story can mask it.
- **Exceptions to "no boxes, no borders":** the dialog's 1px `--dialog-edge` border, and
  no `--focus` ring on a name field (its underline is `--t`; PR-D24, PR-D6).
- **Exception to the Waiting face:** the bank row's waiting cell is a `--btn` fill with a
  1px `--t` edge (PR-D29), not transparent; it is a picture, not a control.
- **Shot masks:** `data-shot-mask="text"` on a dialog's body and note (PR-D30), beside the
  kit's `when`, `status` and `keys`.
- **Dialog grammar** (for every later prompt, Prompts-More included): the Dialog panel, the
  Name field, the Bank row and the three action tones above are kit parts once a second
  board uses them; until then this file owns them.

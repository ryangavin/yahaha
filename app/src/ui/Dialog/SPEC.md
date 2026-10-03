# Dialog

## Identity (all stations)

- **Kind:** complex
- **Built from:** Button (its actions are Buttons with `tone`, passed in the `actions` snippet). Its stories also put a BankRow and NameFields in the `children` snippet; both come earlier in the build order (Prompts.md › Components #4, #5) and are merged before this one.
- **Purpose:** Asks the player one question over the page, in a box with a title, a few lines of explanation and the answers as buttons.
- **Boards:**
  - `Prompts-Dark.dc.html:407-431` (Store: title, body, bank row, name field, actions), `:434-443` (Unsaved changes: title, body, note, actions), `:446-460` (Sound names: title, body with the origin, tagged name field, actions), `:463-470` (Delete: title, body, actions, the default Cancel first); light: `Prompts-Light.dc.html:399-423`, `:426-435`, `:438-452`, `:455-462`.
  - Rename (title, name field, actions, no body) is on no board (Prompts.md PR-D18).
- **Not this component's job:** no store, no API, no Tauri. It doesn't know which prompt it is: the prompt components (`StorePrompt`, `UnsavedChangesPrompt`, `SoundNamesPrompt`, `DeleteRackPrompt`, `RenameRackPrompt`) pass its title, ids and snippets and own the buttons, their faces and what they send. It has no position, z-index or scale of its own (the overlay's centring box places it, Prompts.md › Layout), no scrim (`Scrim`), and no focus handling: initial focus, the Tab trap, Esc and focus return are `PromptOverlay`'s (Prompts.md PR-D5). It is not the old `role="dialog"` insert popover in `panels/rack/RackSlot.svelte`.

## API (Component station)

### Props

Every prop gets a JSDoc comment in the component.

| Prop | Type | Default | Meaning |
|---|---|---|---|
| `titleId` | `string` | — | The title's id (`dlg-store`, `dlg-unsaved`, `dlg-names`, `dlg-delete`, `dlg-rename`). The body's id is `{titleId}-body`, the note's `{titleId}-note`; a prompt names its fields' inputs `{titleId}-field-{n}` (n from 0). |
| `title` | `string` | — | The title text ("Store to Quick Rack A5?", "Delete Ballad?"). Shown as given. |
| `role` | `'dialog' \| 'alertdialog'` | `'dialog'` | `alertdialog` for a destructive prompt (Delete). |
| `kind` | `string` | — | The `data-prompt` value (`store`, `unsavedChanges`, `soundNames`, `deleteRack`, `renameRack`; later prompts add theirs). |

### Events

| Callback | Fires when | Payload |
|---|---|---|
| — | | The Dialog calls nothing; its buttons are the prompt's, in `actions`. |

### Slots / snippets

| Snippet | What goes in it |
|---|---|
| `body` | The body's text, rendered inside `<p id="{titleId}-body">`. Names in it are `<span data-name>` (drawn `--t`); a sound's origin after its name (" · Factory", the dot and the word in one span) is `<span data-origin>` (12px, `--m`) (D3). Left out: no `<p>`, and no body id in `aria-describedby` (Rename). |
| `note` | The note's text, rendered inside `<p id="{titleId}-note">`, with `data-name` spans as the body. Left out: no `<p>` (every prompt but Unsaved). |
| `children` | The parts between the note and the actions: a BankRow, one or more NameFields. Rendered inside the children wrapper (Visual rules), which is the part that scrolls. Left out: no wrapper. |
| `actions` | The Button row's Buttons, left to right (cancelling, destructive, primary; or Delete's default Cancel first). Rendered inside the actions row. Left out: no row. |

**Props it passes to its children:** none. Dialog instantiates no child component; the Buttons, BankRow and NameFields come inside the snippets with the prompt's own props. The Buttons a prompt passes are `size: 'md'` with `tone` (`'primary'` on the primary action, `'destructive'` on the destructive one, unset on the cancelling one), `disabled`, `name` (the aria-label), `label`, `tip`, `tipAction`, `onpress`, and `data-cancel` on the cancelling one (Button's rest attributes, Button D30).

### Visual rules

- **Tokens used:** `--g`, `--t`, `--m`, `--dialog-edge` (new), `--radius`, `--line-width`, `--font-sans`, `--text-12`, `--text-14`, `--text-18`, `--weight-regular`, `--weight-medium`, `--space-8`, `--space-16`, `--space-20`, `--space-24`, `--dialog-width` (new), `--dialog-max-height` (new).

#### New tokens

Not in `app/src/ui/tokens/*` today; they land in the orchestrator's tokens contract PR before this component is built (L1). The component uses them by name and never their values.

| Token | Dark | Light | Used for |
|---|---|---|---|
| `--dialog-edge` | `color-mix(in srgb, var(--t) 40%, transparent)` | `var(--t)` | the panel's 1px border (Prompts.md › Kit additions, PR-D24) (`dark.css`, `light.css`) |
| `--dialog-width` | `440px` | `440px` | the panel's width (`scale.css`) |
| `--dialog-max-height` | `752px` | `752px` | the panel's cap: the 800px centring box less 24 top and bottom (`scale.css`) |

- **Box:** the root is a `<section>` with `width: var(--dialog-width)`, `box-sizing: border-box`, `padding: var(--space-24)`, `border: var(--line-width) solid var(--dialog-edge)`, `border-radius: var(--radius)`, `background: var(--g)`, no shadow, no glow, `display: flex; flex-direction: column; gap: var(--space-16)`, `max-height: var(--dialog-max-height)` (on the Dialog itself, so it holds in a story too), `min-height: 0`. No `position`, `z-index`, `margin` or `transform` (D1). Its height is its content plus 48 (padding) plus 2 (border); it has no fixed height.
- **Order** in the column, top to bottom: title, body, note, children wrapper, actions row. Each part is left out when its snippet is (Rename has no body); the 16px gap is between the parts that are there.
- **Title:** `<h2 id={titleId}>`, margin 0, `--text-18` / `--weight-medium`, line-height `--space-24`, `--t`, `overflow-wrap: anywhere` (a long name wraps on whole words, and a word longer than the line breaks inside itself, so nothing overflows 440), `flex: none`.
- **Body:** `<p id="{titleId}-body" data-shot-mask="text">`, margin 0, `--text-14` / `--weight-regular`, line-height `--space-20`, `--m`, `overflow-wrap: anywhere`, `flex: none`. Inside it, `:global([data-name])` is `--t` at the same size and weight; `:global([data-origin])` is `--text-12`, `--m`.
- **Note:** `<p id="{titleId}-note" data-shot-mask="text">`, margin 0, `--text-12` / `--weight-regular`, line-height `--space-16`, `--m`, `overflow-wrap: anywhere`, `flex: none`; `data-name` and `data-origin` spans as the body.
- **Children wrapper:** a `<div>`, a flex item of the column: `display: flex; flex-direction: column; gap: var(--space-16); overflow: auto; min-height: 0` (it shrinks and scrolls when the panel hits `--dialog-max-height`; the title, body, note and actions stay, `flex: none`).
- **Actions row:** a `<div>`, `display: flex; justify-content: flex-end; gap: var(--space-8); flex: none`. Every Button in it is 32 tall (`md`).
- **Bodies and notes wrap at 390px** (440 less 48 padding and 2 border). The prompt heights in Prompts.md › Layout follow from that: title 24, a body line 20, a note line 16, the bank row 44, a name field 52, the actions 32, 16 between parts.
- **Mousedown:** the root has `onmousedown`: when the event's target is not inside an `input`, `button`, `textarea`, `select` or `[tabindex]` element, it calls `preventDefault()`, so a click on the text or the padding never drops focus to `body` (Prompts.md › Keyboard and focus; D4). It does nothing else.
- **Cursor:** `default` over the panel (the buttons and fields set their own, kit D35).
- **States drawn by:**
  - with or without each part (above).
  - overflow (content past 752px): the panel stops at `--dialog-max-height`; only the children wrapper scrolls.
  - long title: wraps on whole words; the panel grows.
  - light theme: the same markup; only `--dialog-edge` (and the ground and text tokens) change.
  - No hover, focus or pressed look of its own (the panel isn't focusable).
- **Type:** DM Sans (`--font-sans`), sentence case as given, `font-variant-numeric: tabular-nums`.
- **Contrast (AA 4.5:1, `tokens/contrast.test.ts`):** `--t` on `--g` (title, names) and `--m` on `--g` (body, note, origin) exist; no new row. The border is not text. The actions' pairs are Button's (`--g` on `--t`, `--t2` on `--btn`, `--ending-ink` on `--btn`, Button D29).
- **Test hooks:** `data-prompt={kind}` on the root; `data-shot-mask="text"` on the body and the note (Prompts.md PR-D30).
- **Motion:** none. No open or close animation.

### Accessibility

- **Role and name:** the root has `role={role}` (`dialog` or `alertdialog`), `aria-modal="true"`, `aria-labelledby={titleId}`, and `aria-describedby` the body's id and the note's id, space separated, each only when that part is there (`dlg-unsaved-body dlg-unsaved-note`; Store `dlg-store-body`); with neither, no `aria-describedby` attribute (Rename).
- **Keyboard:** none of its own. Tab moves through the fields and buttons in DOM order (children, then actions left to right); the trap, Enter's primary action (a NameField's `onenter`), Esc and initial focus are the overlay's and the prompts' (Prompts.md PR-D5).
- **Tooltip id:** none on the panel (not a control, L3). Each Button and NameField in the snippets carries its own `tip` and `tipAction`.

## Stories (Story station)

- **Title:** `Components/Dialog`.
- **Layout:** `centered` (the panel at its real 440px width) unless the row says otherwise.

Every story renders in dark and light. The meta's `args` are `{ tipAction: fn(), onpress: fn(), onenter: fn() }` (axiom 7), which the story passes to the Buttons and NameFields in its snippets; a story's snippets are written in the stories file with the Prompts.md board-fixture texts. "Actions" below lists the Buttons left to right as `label (tone, tip)`; every one is `size: 'md'`, and the cancelling one carries `data-cancel`.

**Controls (argTypes):** `titleId`, `title`, `kind` text; `role` a select of `dialog` / `alertdialog`; `tipAction`, `onpress`, `onenter` actions. The snippets are not controls (a story's own content).

| Story | Args | Shows | Crop | Play (interaction check) |
|---|---|---|---|---|
| `Board` | `{ titleId: 'dlg-delete', title: 'Delete Ballad?', role: 'alertdialog', kind: 'deleteRack' }`; body "Quick Rack <A4> will be empty, and One Touch 3 of <Sunday Drive Pop> goes back to the style's own. The sounds stay in your Library." (`<…>` are `data-name` spans); actions Cancel (`primary`, `library.rack_delete_cancel`, name "Cancel: keep Ballad (default)"), Delete (`destructive`, `library.rack_delete_confirm`, name "Delete Ballad: empties Quick Rack A4 and returns One Touch 3 to the style's own") | the Delete prompt: title, a three-line body, the default Cancel first in the chosen face, Delete in the ending red | `Board-{dark,light}.png` (Prompts 732,452 440×198), mask `['[data-shot-mask="text"]']` (D6) | the `alertdialog` named "Delete Ballad?" has `aria-modal="true"`, `aria-describedby="dlg-delete-body"`, `data-prompt="deleteRack"`; the body's text is the one above; it has two buttons, the first `data-face="chosen"` with `data-cancel`, the second `data-hue="ending"`; the panel's direct children are `h2`, `p`, `div` (actions) in that order |
| `WithNote` | `{ titleId: 'dlg-unsaved', title: 'Unsaved changes in Sunday drive', kind: 'unsavedChanges' }`; body "Save them before switching to <Warm keys>?"; note "Only the screen asks. The Launchkey, pedals and OTS Link switch at once and keep the edits as <Recovered: Sunday drive>, a rack of yours in Library › Racks."; actions Keep editing (none, `rack.keep_editing`), Discard and switch (`destructive`, `rack.discard_switch`), Save first (`primary`, `rack.save_first`) | body, a three-line note in 12px, three actions | `WithNote-{dark,light}.png` (Prompts 732,82 440×222), mask `['[data-shot-mask="text"]']` | the `dialog` named "Unsaved changes in Sunday drive" has `aria-describedby="dlg-unsaved-body dlg-unsaved-note"`; the note `<p>` has id `dlg-unsaved-note`; the buttons in order are `data-face` `off`, `off` with `data-hue="ending"`, `chosen` |
| `WithChildren` | `{ titleId: 'dlg-store', title: 'Store to Quick Rack A5?', kind: 'store' }`; body Store's never-saved body (Prompts.md › Store); children a BankRow (`bank: 0`, the `storeState` buttons, `waiting: 4`) and a NameField (`id: 'dlg-store-field-0'`, `label: 'Rack name'`, `value: 'Rhodes Soft + Strings'`, `tip: 'quick.save_name'`, `ariaLabel: 'Rack name'`); actions Cancel (none, `quick.cancel_store`), Save and store (`primary`, `quick.save`) | the tallest board prompt: body, bank row, name field, actions | `WithChildren-{dark,light}.png` (Prompts 268,82 440×346), mask `['[data-shot-mask="text"]', '[data-slot="0"]']` (Prompts.md PR-D20) | the children wrapper holds the `img` (bank row) then the `textbox`, in that order, between the body and the actions; the textbox's id is `dlg-store-field-0` |
| `NoBody` | `{ titleId: 'dlg-rename', title: 'Rename Ballad', kind: 'renameRack' }`; no body or note; children a NameField (`id: 'dlg-rename-field-0'`, `label: 'Rack name'`, `value: 'Ballad'`, `tip: 'rack.save_as_name'`, `ariaLabel: 'New rack name'`); actions Cancel (none), Rename (`primary`, `disabled: true`) | the Name prompt: title, field, actions; 190 tall | — (not on a board, Prompts.md PR-D18) | no `aria-describedby` attribute; no `p` inside the dialog; the Rename button is `aria-disabled="true"` and `data-face="disabled"` |
| `LongTitle` | `{ titleId: 'dlg-delete', title: 'Delete Sunday drive with the strings up and the pads all the way down?', role: 'alertdialog', kind: 'deleteRack' }` (a 60-character rack name); body "The sounds stay in your Library."; `Board`'s actions | the title wraps on whole words and the panel grows; nothing overflows 440 | — (no board draws it) | — (layout is judged by Inspect: jsdom has no layout) |
| `Overflow` | `{ titleId: 'dlg-names', title: 'Name the new sounds', kind: 'soundNames' }`; body Sound names' plural body for four parts; children twelve tagged NameFields (R1, R2, R3, L, repeated; `tip: 'rack.sound_name'`); actions Cancel (none, `rack.cancel_save`), Save sounds (`primary`, `rack.save_names`) | the panel stops at 752px; only the fields scroll, the title, body and actions stay | — (no board reaches it) | the children wrapper contains twelve textboxes |
| `MouseDown` | `Board`'s args and snippets | — | — | `fireEvent.mouseDown` on the title returns `false` (default prevented); on the body, `false`; on the Cancel button, `true` (not prevented) |

Crop positions are `board x,y w×h` in the 1440×900 renders, the same box in the dark and light board; each crop is exactly the panel's box, its 1px border included (L6). On the board the border's colour mixes with the scrim over the Stage and in the story with the `--g` ground: at most 1% of a crop's pixels, inside the threshold (Prompts.md › Checks).

## Done when (Inspect station)

- Every story in the table exists, renders in dark and light, and its play passes (`npx vitest run src/ui`).
- `npm run shots -- Dialog` passes: `Board`, `WithNote` and `WithChildren` match their crops with their masks (score at most 0.02, or the Inspect agent judges the difference render noise), and axe finds no violation on any story (`aria-modal` dialogs with a labelled title).
- The new tokens (`--dialog-edge`, `--dialog-width`, `--dialog-max-height`) are in the tokens contract PR before the build, and Button has `tone` (Button D25).
- Only listed tokens are used; no inline colours, no literal sizes outside the Visual rules.
- svelte-check and lint pass on the folder.

## Decisions

- **D1 · No position of its own.** The Dialog is a box in normal flow; the overlay's centring box (`display: grid; place-items: center` in `0,0 1440×800`) places it in the app, and `layout: centered` does in a story, so the same component renders in the Board story's review grid and alone (Prompts.md › Layout).
- **D2 · Snippets, not props, for the text.** The body and note carry styled names (`--t` spans, the 12px origin), so they are snippets rendered inside the Dialog's own `<p>`s; the Dialog owns the ids, the `aria-describedby` and the `data-shot-mask`, the prompt owns the words.
- **D3 · `data-name` and `data-origin` spans.** A snippet's markup belongs to the prompt's scope, so the Dialog can't style a prompt's class; it styles `:global([data-name])` (`--t`) and `:global([data-origin])` (12px `--m`) inside its body and note, and every prompt marks its names that way.
- **D4 · The mousedown guard lives on the Dialog.** Prompts.md gives the overlay the job of keeping focus inside; the `preventDefault` on a mousedown over the panel's text and padding is local to the panel, so it is the Dialog's and works in a story without the overlay. The scrim has its own (Scrim).
- **D5 · Not focusable, no `Focused` story.** The panel has no tabindex and no focus look; its Buttons and NameFields have their own `Focused` stories.
- **D6 · Masked text in the crops.** Store's body and the Unsaved note are reworded from the board (Prompts.md PR-D11, PR-D14), so every crop masks `[data-shot-mask="text"]` and checks the panel, its parts and its size; the words are checked by the plays and the prompts' tests (PR-D30). Delete's label is drawn at 500 on the board and 400 here (PR-D4): a few pixels, judged as noise in `Board`.
- **D7 · L1, new tokens.** `--dialog-edge` (dark 40% `--t`, light `--t`) from Prompts.md › Kit additions, plus `--dialog-width` 440px and `--dialog-max-height` 752px so the box's literal sizes are tokens (axiom 2); they land in the tokens contract PR, never in this folder.
- **D8 · No crop file exists yet.** `app/src/ui/Dialog/crops/` is cut later by the crop station from `Prompts-Dark.png` / `Prompts-Light.png` at the boxes in the story table.
- **D9 · `<section>` root.** The root element is a `section` (any block element would do; the role is set explicitly), so the panel has no implicit role that fights `dialog` / `alertdialog`.

Follow-ups: the Dialog panel, the three action tones and its parts become kit parts once a second board uses them (Prompts-More, #524; Prompts.md › Kit additions).

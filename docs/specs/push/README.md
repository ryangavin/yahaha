# Push specs

One spec per Push board (`docs/design/push/`), each from its `spec` issue (#500 to #535). A spec is what a developer agent builds the screen from: it should never need to open the board.

**Reference spec:** [Stage.md](Stage.md) (#500), with the shared parts in [kit.md](kit.md). Copy Stage.md's sections in its order: header (issue, boards, built from, variants, glance order, what must land before building), Layout (region table with boxes), one section per region of the screen's own (each control in a table: face, reads, sends, tooltip, Launchkey), States, Board fixture, Components, Gap against today, Contract changes needed, Checks, Decisions (numbered D1…), Follow-ups. Anything drawn on more than one board goes in kit.md, not in a screen spec; a screen spec names the kit part (`kit.md` › Pad) and says only what differs. When a later run finds a better shape, the reference moves to that spec and this line says so.

## What a good spec has (revised after run 1)

- **Every value, in the spec:** colours, gradients, sizes, offsets and type copied in (or named as a kit token whose value kit.md gives). A pointer into board lines is never the only source; board lines are only for cutting crops.
- **Regions:** the screen split into named regions, each with its box (x, y, w, h at 1440×900), checked against the board's real nesting (borders and padding included).
- **Every control:** its label, its face in each state (off, on, chosen, waiting, disabled), the command it sends (`docs/app-api.md`) or the app-only state it changes, its tooltip key (`app/src/help/tooltips.ts`), its Launchkey mapping, its `aria-label` (a template, not one example), and its keys and pointer behaviour (long press, drag maths, send rate).
- **Every value shown:** which `AppState` field it reads, its format (rounding, units, spelling), and what it shows when the field is null, empty or not loaded yet.
- **Every link:** where it goes, and what it does until that page's spec is built (the kit's interim rule: today's drawer or panel if one exists, else disabled with its tooltip).
- **Tokens:** colours, type and spacing by name, defined once in kit.md; dark and light; each glow and mix token assigned to the elements that use it.
- **States and interactions:** what changes on click, hold, Shift and key; hover and cursor; tab order; what the screen shows when empty or in trouble.
- **A board fixture that fixes the moment:** the full state that reproduces the board, consistent with what the engine would actually send, plus the clock (`now`, the anchors, the LED phase) and anything the wiring computes (meter holds), so the screenshot is the same on every run.
- **A Components table:** every part, primitive or complex, in build order, what it's built from, whether it exists in `app/src/ui` today, and its board lines for crops.
- **Gap against today:** what exists in `app/src` now, what changes, and tool changes the checks need (e.g. `scripts/shots.ts`).
- **Contract changes:** each with its files, and what the screen does until it lands; say which ones block the build (tooltip keys always do).
- **Checks that can be written:** vitest checks that read roles, names, attributes (`data-face`, `data-hue`) and commands sent, and pure functions for the maths; anything jsdom can't compute (custom properties, `color-mix`, layout, text measurement) goes in a named story and the screenshot check, with its story name, viewport and masks.
- **Decisions:** anything the board left open or got wrong, numbered (D1…), each a sentence a later brief can cite.

A variant board (one copied from another, such as Stage-Help) specs only what differs and links its base spec.

## Done when (a spec)

- It contains every value it needs; no board line is the only source of a value.
- Its board fixture fixes the exact moment (clock anchors, `now`, LED phase) and is consistent with the engine.
- It has a Components table.
- Its decisions are numbered, and every contract change says what the screen does until it lands and whether it blocks the build.
- Every check can be written as stated (vitest for meaning, stories and shots for pixels).
- It passes a **cold read**: a fresh agent given only the spec and the repo (never the board) lists every place it would have to guess something visible or behavioural, and finds none. Findings are fixed in the spec, then the cold read runs again.

## Run log

What each run taught us about writing specs.

- **Run 1 (Stage, #500, PR #537).** The first draft looked complete; a cold read (spec and repo only) found 21 places a builder would guess.
  - Pointers into board lines stood in for values (the art gradients), and glows borrowed one hue's token (`--bg` is green, so only Main worked).
  - The fixture named states ("beat 3", "bright flash") instead of fixing the clock, and one of them contradicted the engine (Fill In BB).
  - Links to unbuilt pages had no interim behaviour; checks asked jsdom for computed colours and `color-mix`; the screenshot check needed a viewport and masks the tool lacks.
  - Small formats were left open: rounding, an empty folder, Intro D, which bar is peak.
  - Fixes that carry to every spec: copy values in, one interim link rule, `data-face` / `data-hue` test hooks, pixels in named stories, a Components table, and the cold read in "Done when".
  - Cold read 2 (after those fixes) found 24 more, mostly at the seams: the page component's prop list and what the fixture exports, how the interim pages share the window with the fixed-size screen, which tab is chosen, tokens and fonts clashing with the old shell's, and geometry a reader can't derive (tab gaps, left edges vs centres, line-heights, a divider's x). Fixed with a props table, a shell section, one `chosenPage()` rule, a token migration rule (D48) and left-edge geometry throughout; dimmed text got an owner decision (D47) and one listed axe exemption.
  - Cold read 3 → see the next line once it runs.

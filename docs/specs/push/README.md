# Push specs

One spec per Push board (`docs/design/push/`), each from its `spec` issue (#500 to #535). A spec is what a developer agent builds the screen from: it should never need to open the board.

**Reference spec:** none yet. Stage (#500) is the first; once it lands it becomes the reference, and every later spec copies its shape. When a later run finds a better shape, the reference moves to that spec and this line says so.

## What a good spec has (first take; we revise it after each run)

- **Regions:** the screen split into named regions, each with its box (x, y, w, h at 1440×900) and what's in it.
- **Every control:** its label, its face in each state (off, on, chosen, waiting, disabled), the command it sends (`docs/app-api.md`) or the app-only state it changes, its tooltip (`app/src/help/tooltips.ts`) and its Launchkey mapping.
- **Every value shown:** which `AppState` field it reads, and its format.
- **Tokens:** colours, type and spacing by name, defined once in the Stage spec and referred to elsewhere; dark and light.
- **States and interactions:** what changes on click, hover, hold and key; what the screen shows when empty or in trouble.
- **Gap against today:** what exists in `app/src` now, what changes, and any command or state the API lacks (that becomes a contract change).
- **Checks:** acceptance tests a builder can turn into vitest tests, plus a screenshot comparison against the board's PNG.
- **Decisions:** anything the board left open, decided, as "Decision: ...".

A variant board (one copied from another, such as Stage-Help) specs only what differs and links its base spec.

## Run log

What each run taught us about writing specs.

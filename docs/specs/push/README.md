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
- **Global keys:** for every control that handles keys itself (a fader, a knob, a tab list, a text field), which of the window handler's bindings (`app/src/lib/keys.ts`) it must stop while it has focus, and how (`stopPropagation` on its own `keydown`); the rest keep working through it.
- **Pure functions, one path each:** every pure function the checks name (`splitChord`, `holdPeak`, `chosenPage`…) has its file path given once, in the Components table or the kit, and is referred to by name everywhere else; two specs never put the same function in two files.
- **Engine-derived fixture values cite the formula:** a fixture value that the engine or the wiring computes (a meter height, a held peak, a beat position, a knob fraction) is given with the formula and inputs that produce it (`height(0.0724)` = 138, from `height()` in the kit), so a builder can check the number instead of trusting it.
- **A board fixture that fixes the moment:** the full state that reproduces the board, consistent with what the engine would actually send, plus the clock (`now`, the anchors, the LED phase) and anything the wiring computes (meter holds), so the screenshot is the same on every run.
- **A Components table:** every part, primitive or complex, in build order, what it's built from, whether it exists in `app/src/ui` today, and its board lines for crops.
- **Props and callbacks for every component, primitives included:** each row's props (name, type, default) and callbacks (name, payload), and the base `Button`'s full API written out once (readers guessed it three rounds running); a complex component's props are the state fields its section reads, never `state` whole.
- **An AA line for every new text-on-face colour:** each hue or role the spec draws as text on `--g`, `--btn` or a hue fill says its contrast ratio in both themes, or names the contract change that fixes it (axe runs in the checks; `--ending` on `--btn` fails in dark, Stage.md C6).
- **Fixtures give every required field of the TS type** (`app/src/lib/api/types.ts`), not just the ones the screen reads, so the fixture compiles and the mock's `derive()` isn't needed to fill gaps; a plugin fixture uses the fake plugin's real id, `aumu Smp7 Fake` (`app/src/lib/api/mock-plugins.ts`), name "Sampler Deluxe".
- **Gap against today:** what exists in `app/src` now, what changes, and tool changes the checks need (e.g. `scripts/shots.ts`).
- **Contract changes:** each with its files, and what the screen does until it lands; say which ones block the build (tooltip keys always do).
- **Checks that can be written:** vitest checks that read roles, names, attributes (`data-face`, `data-hue`) and commands sent, and pure functions for the maths; anything jsdom can't compute (custom properties, `color-mix`, layout, text measurement) goes in a named story and the screenshot check, with its story name, viewport and masks.
- **Decisions:** anything the board left open or got wrong, numbered (D1…), each a sentence a later brief can cite.

A variant board (one copied from another, such as Stage-Help) specs only what differs and links its base spec.

**Kit additions.** A screen spec that needs something the kit lacks (a new face variant, a token, a shared component drawn on its board) writes it under its own "Kit additions" heading, in kit.md's format, and builds against that text. It does not edit kit.md: the kit's owner folds the addition into kit.md in the PR that lands the kit change, or a follow-up kit-only PR does, and the screen spec's section then shrinks to a pointer (`kit.md › Pad`). A screen lane never touches kit.md, so two screen specs can't race on it.

## Done when (a spec)

- It contains every value it needs; no board line is the only source of a value.
- Its board fixture fixes the exact moment (clock anchors, `now`, LED phase) and is consistent with the engine.
- It has a Components table.
- Its decisions are numbered, and every contract change says what the screen does until it lands and whether it blocks the build.
- Every check can be written as stated (vitest for meaning, stories and shots for pixels).
- It passes a **cold read**, within a budget: a round is two fresh agents given only the spec and the repo (never the board), one listing every place it would have to guess something *visible* (a pixel, layout or text difference), the other every *behavioural* guess (a command, state, ARIA attribute or interaction); they overlap little (3 of 22 findings in one run), so one reader misses half. Each reader checks the spec's claims against the files it names, not only the spec's completeness. Findings about the screen's own content are fixed in the spec and the round repeats; second-order findings (a component's internals, where a test file lives, a story's args) go to that component's SPEC.md or the follow-ups, not into the screen spec. Rounds stop when a round finds no visible or behavioural guess about the screen's own content, or after three rounds; the spec's run-log line then says what each round found and what was left. Counts don't converge to zero on their own (one spec's rounds ran 29 → 30 → 19 → 16), so the budget, not a zero, is the stop.

## Run log

What each run taught us about writing specs.

- **Run 1 (Stage, #500, PR #537).** The first draft looked complete; a cold read (spec and repo only) found 21 places a builder would guess.
  - Pointers into board lines stood in for values (the art gradients), and glows borrowed one hue's token (`--bg` is green, so only Main worked).
  - The fixture named states ("beat 3", "bright flash") instead of fixing the clock, and one of them contradicted the engine (Fill In BB).
  - Links to unbuilt pages had no interim behaviour; checks asked jsdom for computed colours and `color-mix`; the screenshot check needed a viewport and masks the tool lacks.
  - Small formats were left open: rounding, an empty folder, Intro D, which bar is peak.
  - Fixes that carry to every spec: copy values in, one interim link rule, `data-face` / `data-hue` test hooks, pixels in named stories, a Components table, and the cold read in "Done when".
  - Cold read 2 (after those fixes) found 24 more, mostly at the seams: the page component's prop list and what the fixture exports, how the interim pages share the window with the fixed-size screen, which tab is chosen, tokens and fonts clashing with the old shell's, and geometry a reader can't derive (tab gaps, left edges vs centres, line-heights, a divider's x). Fixed with a props table, a shell section, one `chosenPage()` rule, a token migration rule (D48) and left-edge geometry throughout; dimmed text got an owner decision (D47) and one listed axe exemption.
  - Cold read 3 (after those, plus the Effects writer's feedback: `tip` on every control primitive, a Props and callbacks paragraph, the global-keys, pure-function-path and fixture-formula rules above) found 33: 17 behavioural, 7 visible, 9 minor. Most were ARIA that axe would fail (knob `aria-valuenow`, a bare `aria-label` on the key strip, tabs without a tablist, four lamps all named "On"), two checks that were wrong as written (the dropout counts, a disabled story control the stories test forbids), unstated function signatures (`holdPeak`'s state, `knobFraction`), and claims about the repo that had drifted (`app.subscribe`, the `--line` reader count, a non-null `library`). Fixed with D60–D63, a `data-face` column on the pad table, `layout()`-based key geometry, and every claim re-checked against the file it names. Lesson: a cold reader must check the repo's files, not only the spec's completeness; and ARIA roles belong in the kit's tables, next to the face. The Stage has now had its three rounds (21 → 24 → 33, the last mostly second-order and repo drift); what remains after round 3's fixes is for the component specs (each primitive's full prop list beyond the Button's, story args) and the "Done when" rules above came from it and from the Prompts spec's four rounds (PR #544).

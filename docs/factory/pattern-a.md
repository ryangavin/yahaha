# Factory pattern A: the assembly line

An experiment in building the app's UI the way a factory builds parts. A pile of specs goes in at one end; merged, reviewed Svelte components with Storybook stories come out the other. Each station on the line is a one-shot agent with one job. This is the first of two patterns we'll compare; pattern B (one cell per component family) runs later on a matched subset.

The design target is the Push canvas (https://claude.ai/artifact/Vma4hDTe9ZQFm2R9YqLcRy): 24 screens, dark and light.

## 1. The plant (built once, before the line runs)

One contract PR into `develop`, briefed in [storybook-setup.md](storybook-setup.md): Storybook in `app/`, the canvas tokens in `app/src/ui/tokens.css`, one folder per component (`<Name>.svelte`, `<Name>.stories.ts`, `SPEC.md`, `crops/`), stories run as tests inside `npm run test`, and `npm run shots -- <Name>` to screenshot stories against their crops. It ships one worked example, `LampButton`, that every station copies.

## 2. Raw material: the spec pile

The line does not write specs. A separate spec run produces the whole pile up front, and the owner inspects it once (goods inward), before anything is built.

**Input:** the component inventory (primitives, complex components, pages) from the component-library plan, and the Push canvas boards.

**One spec per component**, in `app/src/ui/<Name>/SPEC.md`, written from [spec-template.md](spec-template.md), with its board crops in `crops/`. The spec includes the full story table, so stories are specified before anything is built. The pile is committed in one specs PR before the line starts.

**Spec run:** one agent per component family writes that family's specs from the inventory and boards; one consistency pass checks names, props and states agree across families (the same "on" state is called the same thing everywhere). The pile is ordered so a part's dependencies come before it; the pilot uses primitives only, so it has none.

## 3. The line

The conveyor is a workflow script (`pipeline()`): each spec enters the line on its own and moves to the next station as soon as the last one finishes, so many parts are in flight at once. Nothing polls. No agent lives longer than its one job.

| Station | Agent | Reads (spec sections) | Its one job | Runs | Hands on |
|---|---|---|---|---|---|
| 1. Component | developer (Opus, medium) | Identity, API, Accessibility; tokens | Writes `<Name>.svelte` only, in a fresh worktree and branch. No stories, no tests. | svelte-check on its folder, once | "builds; I think it meets the API" |
| 2. Story | developer (Opus, medium) | Identity, Stories; the component file | Writes `<Name>.stories.ts` only: one story per row of the spec's table, each Play turned into a `play` function. Never edits the component; if a story can't be written against it, pulls the andon. | svelte-check on its folder, once | "every row has a story" |
| 3. Inspect | reviewer (Sonnet) | Done when; crops | Runs the machine inspection (stories as tests, shots against crops, lint), then looks at each shot beside its crop. Writes a defect list, each defect tagged with the file it lives in (component or stories). Never edits. | machine inspection, once | pass, or a defect list |
| 4. Rework | quick-fix (Opus, low) | the defect list only | Fixes exactly the listed defects in the tagged file, nothing else | svelte-check on its folder, once | back to Inspect |
| 5. Ship | (script) + reviewer (Sonnet, bot account) | the passed branch | Commit, push, open the PR, Sonnet review under `satori-miyamoto`, `gh pr merge --auto` | CI in the merge queue | merged |

- **Why Component and Story are separate.** The Story station is the component's first user: it can only use the API the spec promised, so a story that can't be written exposes an API gap at once. Each station's defects are also counted separately in the traveler, so we learn which half leaks.
- **Rework cap.** Inspect → Rework loops at most twice; a third failure pulls the andon cord automatically.
- **Andon cord.** Any station can stop its part with a reason ("spec says X, board shows Y"; "needs a token that doesn't exist"). The part leaves the line and comes to the orchestrator, who decides, fixes the spec, and puts it back at Component (or at Story, if only the stories were wrong). Stations never guess around a bad spec.
- **Each check runs once, at one station.** Component and Story each compile their own file; Inspect runs the machine inspection; the merge queue runs CI. No station re-runs another's checks.
- **Worktrees.** One worktree per part, branched with `git checkout --no-track -b ui/<name> origin/develop`. Web-only, so no Rust build queue; `node_modules` comes from `npm ci` (cached). Removed after merge.
- **Small lots.** One component per PR. The merge queue absorbs many small web-only PRs.

## 4. The traveler

A record that rides with each part through the workflow (in the script's state, not on GitHub). The line's telemetry comes from it.

```
name, spec path, branch, PR
component: started, finished, tokens, notes
story: started, finished, tokens, stories written, notes
inspect[n]: verdict, defects (by kind), diff scores, tokens
rework[n]: defects fixed, tokens
andon: station, reason, decision
ship: PR opened, review verdict, merged at
```

## 5. End of line: integration

After the pilot's parts merge, one integrator assembles a page story from them (for the pilot: the band's lamp row and a pad grid) and compares it with the full board render. Defects found here go back on the line as rework of the part that caused them, never fixed in the page.

## 6. Pilot

- **Parts:** about ten primitives chosen from different families, e.g. tab, segmented tabs, fader, meter, knob, pad, section pad, beat block, readout numeral. No complex components yet. LampButton is not in the pilot: it is the plant's worked example.
- **Report at the end**, per part and for the line: lead time (spec in to merged), first-pass yield (passed Inspect first time), rework loops, andon pulls by station and reason, tokens per merged part, page-stage defects, and the owner's verdict in Storybook (accept / redo).
- **Then pattern B** builds the next family as one cell, measured the same way.

## 7. Before it runs

1. The component inventory exists (the component-library plan).
2. The plant PR is merged.
3. The spec pile is written, inspected by the owner, and merged.
4. The workflow size limit is raised ("Dynamic workflow size" in /config) and the run is started with "use a workflow".

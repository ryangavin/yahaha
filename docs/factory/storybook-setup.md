# Brief: the Storybook plant (one contract PR)

For a `developer` lane lead. Read `docs/factory/pattern-a.md` and `docs/factory/spec-template.md` first: this PR builds the plant that factory runs on, so every later component PR can branch from `develop` and only add its own folder.

## Outcome

One PR into `develop`, branched with `git checkout --no-track -b ui/storybook-plant origin/develop`. When it is merged, the following is true:

1. **Storybook runs in `app/`.** `npm run storybook` serves it and `npm run build-storybook` builds it, using the Svelte + Vite framework package for the app's Svelte 5 and Vite 8. Use the newest Storybook that supports both. If none supports Vite 8, stop and report it; don't downgrade Vite.
2. **Tokens.** Two layers (axiom 5): a palette file and the theme files that map roles to it, under `app/src/ui/tokens/`. Together they hold the Push canvas's design tokens for dark (`:root`) and light (`:root[data-theme="light"]`). That covers ground, text and muted greys, the button face, `--lamp`, `--lamp-ink`, `--rec`, `--ending`, `--warn`, the part hues (R1, R2, R3, L) and section hues (Intro, Main, Ending, Break, Fill), the violet accent, the glows (`none` in light), the one radius, the type scale and spacing. Take the values from the canvas source in `../yahaha-research/push-canvas/`: the kit `:root` lines in `project/Stage-Dark.dc.html` and `Stage-Light.dc.html`, and the notes in `KIT-DEBATE.md`. Use the names the canvas uses wherever it names them.
3. **Fonts.** DM Sans and JetBrains Mono come from npm (`@fontsource/dm-sans`, `@fontsource/jetbrains-mono`), as the app does with Barlow today, so Storybook works offline.
4. **Themes.** Storybook's themes addon (`withThemeByDataAttribute`) switches dark and light by setting `data-theme`, and the canvas background follows the theme. Every story renders in both themes with no per-story code. Everything in [storybook-axioms.md](storybook-axioms.md) holds, and the machine-checkable axioms are enforced by the story test and lint.
5. **Story format.** Stories are typed against the component's props, with no story-only wrapper components (axiom 4). Use CSF 3 in `<Name>.stories.ts` if snippets and child content can be passed natively there (e.g. `createRawSnippet`); otherwise use Svelte CSF (`<Name>.stories.svelte`) for every story. Choose one format for the whole library, and confirm that stories still run as tests (item 6) in that format. Record the choice in the PR body.
6. **Stories are tests.** One test file, `app/src/ui/stories.test.ts`, finds every `*.stories.ts` under `app/src/ui` and composes each story with Storybook's portable stories. It renders each story in the existing vitest jsdom environment and runs its `play` function. A new component's stories are tested by `npm run test` with no new test file. So the existing `npm run verify`, which CI runs, covers the library.
7. **Shots.** `npm run shots -- <Name>` screenshots each of that component's stories in dark and light to `app/.shots/<Name>/` (git-ignored). Where `app/src/ui/<Name>/crops/<story>-<theme>.png` exists, it writes a pixel diff image and a score per story to `app/.shots/<Name>/report.json`. It uses the system Chrome through `playwright-core` (no browser download), with `pixelmatch` and `pngjs` for the diff. It kills Chrome and any server it started on every exit path. It is a local tool for the Inspect station, not part of CI.
8. **Checks cover the library.** `npm run check` (svelte-check) and `npm run lint` include `app/src/ui`, `.storybook/` and the new scripts, and pass.
9. **First article.** One component goes all the way through as the worked example every station copies: `LampButton`, the on/off control the whole canvas uses (Accomp, Metronome, part On, Sound, Looper). It lives in `app/src/ui/LampButton/` with:
   - a `SPEC.md` filled in from the template;
   - crops cut from the canvas renders in `../yahaha-research/push-canvas/render/`;
   - the component;
   - its stories with play functions.

   Its shots must match its crops.
10. **Docs.**
    - **AGENTS.md** lists the new commands under "Checks": stories run inside `npm run test`, `npm run storybook`, and `npm run shots -- <Name>` for UI library work.
    - **AGENTS.md "Lanes"** gains the UI library's contract files: `app/src/ui/tokens/*`, `.storybook/*`, `app/src/ui/stories.test.ts` and the shots script. Every other file under `app/src/ui/<Name>/` is owned by that component's PR alone.
    - **`docs/factory/`** (the plan, the spec template and this brief) is committed in this PR.

## Constraints

- **Leave the running app alone.** Don't touch `app/src/lib/ui/` or the app's own components. The library starts empty beside them, and nothing in the app imports from `app/src/ui` yet.
- **Library rules.** No store, API or Tauri imports anywhere under `app/src/ui`: components take props and call callbacks. Add an ESLint rule that enforces it.
- **No barrel file.** Components are imported from their own folders.
- **Public repo.** The canvas crops are our own design, so they are fine to commit. Commit nothing else from `yahaha-research`.
- **Checks.** Run, once each, before opening the PR:
  - `npm run check`, `npm run lint` and `npx vitest run src/ui` in `app/`;
  - `npm run build-storybook`;
  - `npm run shots -- LampButton`.

  CI's merge queue runs the rest.
- **Clean up.** Stop every server and browser you start, and check with `ps` that none is left.

## Report

One final report:
- the PR link and head SHA;
- the Storybook version chosen;
- what you ran and each result;
- the LampButton shot scores;
- any decision you made, and anything in the spec template that proved wrong or missing while doing the first article.

Note the decisions in the PR body too.

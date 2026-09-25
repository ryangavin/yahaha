You are working on the yahaha desktop app, in the `app/` folder of ryangavin/yahaha. That repo is PUBLIC. The app uses Svelte 5, TypeScript and Vite inside Tauri 2. yahaha is a Rust arranger that plays Yamaha Genos style files live. The app's main screen is a skeuomorphic 1:1 mirror of the owner's Novation Launchkey MK4 (49/61 keys), with a sleek look in the spirit of Sylenth1. The app foundation (PR #72) is merged into `develop`.

Owner rules:
- EVERY interactive control gets a tooltip from the single catalog `app/src/help/tooltips.ts`. Each tooltip gives what the control does, the Genos term, the keyboard shortcut, and where it lives on the Launchkey, if anywhere. A coverage test enforces this.
- Match the existing design system: the tokens and material recipes in `app/src/app.css`, the shared components in `app/src/lib/ui/`, and `app/CONTRIBUTING.md`.
- Hold 60 Hz with no jank. Don't put blur filters on animating elements.
- Use no Yamaha or Novation assets.
- The UI renders from state and sends actions. Never re-implement engine behaviour in TypeScript.

Contract:
- The engine API is documented in `docs/app-api.md` (AppCmd / AppState).
- A follow-up PR is adding `state.surface` (Shift, non-pad controls, Track neighbours, beat clock, fader positions). If your panel needs data the API doesn't have, don't invent engine behaviour. Mock it with the proposed shape in `app/src/lib/api/`, and add a NEED line to the coordination board.

Coordination:
- Many agents are working in parallel. The board is at `<SCRATCHPAD>/board.md`.
- Read it first. Add a CLAIM line before you edit anything outside your own panel folder: `app.css`, shared `lib/ui` components, the `tooltips.ts` sections owned by others, `App.svelte`, or the stores.
- Re-read the board before you open your PR.
- Other UI agents are building the other panels right now, so stay inside your own folder wherever you can.

Setup:
```
git fetch origin && git checkout -b <your-branch> origin/develop
cd app && npm install
```

Verify:
- Run `npm run verify`: typecheck, lint, and tests including tooltip coverage.
- Check your panel in the browser on the mock with `npm run dev`, at 1024, 1440 and 1920 widths, in both dark and light themes.
- Add or refresh the screenshots in `app/docs/screenshots/`.
- The repo's `cargo build --release` must still pass.

Commit messages end with: `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`

Open the PR against `develop` with screenshots in the body. The body ends with: `🤖 Generated with [Claude Code](https://claude.com/claude-code)`

Do not merge. If git signing fails, stop and report.

Return: the PR URL, a summary, screenshot paths, and any NEEDs from the engine API.

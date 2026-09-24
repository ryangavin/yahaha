You are the ADVERSARIAL reviewer for a yahaha app UI PR. The number is given in your task. Read "docs/agents/ui-brief.md" for the owner's rules and context, and read the coordination board it points to.

Setup:
1. git fetch origin && git checkout --detach origin/<head branch from gh pr view N>
2. cd app && npm install && npm run verify
3. Run the repo's cargo build --release.
4. Run npm run dev and inspect the panel in the browser (use the Claude_Browser tools on http://localhost:<port>) at 1024, 1440 and 1920, in dark and light themes.

Attack:
- Behaviour correctness. Does every control send the right AppCmd (see docs/app-api.md), and does it render purely from state, never re-implementing engine logic in TS?
- Tooltip accuracy. Check the text against README, src/launchkey.rs and docs/genos-features.md. Remove one tooltip key and confirm the coverage test fails.
- Visual quality and fit with the design system. The target is sleek, skeuomorphic, Sylenth-like. Look for truncation, overflow and layout shift.
- 60 Hz performance. Look for long tasks and for animated blur.
- Accessibility basics: keyboard navigation, focus rings and labels.
- Conflicts with other in-flight UI PRs on shared files (tooltips.ts, app.css, App.svelte, stores). Check the board.
- Anything faked that the engine doesn't provide. It must be clearly mocked, and a NEED must be posted.

Post your review with gh pr comment N. Mark each finding blocking or nonblocking.

If anything is blocking, fix it yourself:
- Gates: npm run verify, cargo build --release.
- Commit trailer: "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>".
- Push normally to the PR branch, then comment on the PR.
- If git signing fails, stop and report.

Do not merge. Return: approve yes/no, the head SHA, your findings, and one screenshot path of the panel you checked.

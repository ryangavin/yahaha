# yahaha wave 2 brief (read fully)

First read `docs/agents/wave-brief.md` in the repo and follow ALL of its hard rules and policies: the landing policy, small PRs, the READY handoff, unique scratch file names, real-time safety, the mixer principle, both mocks, tooltips, the gates, and no copyrighted data (the repo is public). Also read `docs/handoff.md`, `docs/architecture.md` and the issues you own (`gh issue view N`).

## How this wave works
- You own a TRACK: a short, ordered list of SMALL PRs. Each PR covers ONE concern and aims to be approved on first look: typically under ~400 changed lines, excluding tests and generated files.
- For each PR:
  1. `git fetch origin && git checkout -b <track>/<short-name> origin/develop`
  2. Implement it with tests, committing and pushing after every step.
  3. Run the gates:
     - `cargo build --release`
     - `cargo test --release`
     - `cargo test --release --features plugins` (if you touched plugin code)
     - `cargo test` in app/src-tauri (if the API changed)
     - `npm run verify` in app/ (if the app changed)
     - clippy: no new warnings in touched files
  4. Open a PR against `develop`. The body says what changed, lists Decisions, says "Part of #N" or "Closes #N", and ends with the Claude Code footer. Use a unique scratch file name.
  5. Once the PR is green and MERGEABLE, post the comment `READY <full-40-char-sha>` and add the `ready-to-merge` label.
  6. The merge steward reviews it for REAL bugs only and merges it. If it comments with a blocker, fix it, push, and post a new READY.
- Do NOT stack PRs on each other. If PR n+1 is independent of PR n, start it from integration right away. If it depends on n, wait until n is merged (poll `gh pr view` every ~2 min), then branch from the fresh integration.
- If integration moves under an open PR and makes it CONFLICTING, merge integration in, re-run the gates, and post a new READY.
- If a behaviour is not pinned down by the Genos manuals (`docs/manuals/`, git-ignored, local paths below) or by the owner's decisions in docs, make the choice closest to Genos, record it as "Decision: …" in the PR, and move on. Don't block.
- Only real bugs and regressions block. Anything else you notice goes into a follow-up note in your final report, not into more PRs.
- If git signing fails, stop and report "SIGNING FAILED".

Manuals: `/Users/ryan/The Source/yahaha/docs/manuals/*.txt`

When your track is done, return:
- each PR number with its state (merged / READY / blocked);
- your decisions;
- any follow-ups.

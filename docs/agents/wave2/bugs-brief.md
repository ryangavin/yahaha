# yahaha owner-reported playing bugs (read fully)

Follow `docs/agents/wave2/wave2-brief.md` for process: small PRs from `origin/integration/m3-ui`, gates, READY plus the label, and the merge steward merges.

These bugs come from the owner PLAYING the current integration build: the Tauri app driven by a Launchkey, using the soundfonts in `soundfonts/` and corpus styles. Treat them as REAL bugs.
- REPRODUCE FIRST, headless where possible. Use `Session::offline()` / `offline_audio`, `yahaha sim`, or the engine directly with corpus styles, and write a failing test.
- Then fix it in the smallest PR you can, with that test.
- If you can't reproduce it, instrument it, and report exactly what you tried and what you'd need from the owner. Never guess-fix.
- File a GitHub issue for your bug first (label `bug`, milestone M5 or M6 as fits). Reference it in the PR with "Closes #N".

Compare against the Genos manuals (`/Users/ryan/The Source/yahaha/docs/manuals/*.txt`) and the owner's decisions in the docs (`docs/section-timing.md`, `docs/fills-and-rules.md`).

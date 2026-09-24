# yahaha wave brief (read fully)

**Project.** yahaha is a Rust real-time software arranger. It plays Yamaha Genos style files live and must behave like real Genos hardware.
- Repo: ryangavin/yahaha (PUBLIC). Rolling integration branch: `integration/m3-ui`.
- Desktop app: `app/` (Svelte 5 + TS + Tauri 2). Its main screen mirrors the owner's Novation Launchkey MK4.
- Terminal UI: src/ui.rs.

**Read first:**
- `docs/architecture.md`: thread model, where features plug in, hotspot ownership, and the add-a-feature checklist. Follow it.
- `docs/app-api.md`: the AppCmd/AppState JSON contract.
- `docs/genos-features.md`: the behaviour spec.
- Manuals (git-ignored): `/Users/ryan/The Source/yahaha/docs/manuals/*.txt`
- The coordination board: `<SCRATCHPAD>/board.md` (the coordinator creates it outside the repo, seeded from `docs/agents/board-archive.md`)
  - Read it at start, before editing any shared file, and before opening your PR.
  - APPEND CLAIM / NEED / FINDING / CHANGED / RELEASE lines.
  - About 9 agents are working in parallel right now.

**Setup:**
```
git fetch origin && git checkout -b <your-branch> origin/integration/m3-ui
ln -sfn "/Users/ryan/The Source/yahaha/corpus" corpus; ln -sfn "/Users/ryan/The Source/yahaha/soundfonts" soundfonts
```
For app work, run `cd app && npm install` once.

**Hard rules:**
1. COMMIT AND PUSH AFTER EVERY MEANINGFUL STEP: `git push -u origin <your-branch>`. Worktrees can vanish; unpushed work is lost.
2. Disk is tight. Use one target dir, no extra feature-variant builds unless needed, and never `cargo tauri build`.
3. Real-time safety: no allocation, locks or panics on the MIDI, engine or audio threads. Precompute at load. Add or extend a no-alloc test for new real-time paths.
4. Mixer principle: a part's volume is ONLY its CC7, sent unchanged to the synth and the yahaha port. No hidden per-part gain. The master fader is the only non-CC gain.
5. Put new features in their own files, per docs/architecture.md: an api module, a session handler, an engine hook plus a `Features` field, and the input processor slot. CLAIM on the board before touching any hotspot.
6. Preserve the JSON wire format for existing fields (tests/api_wire.rs). New fields and commands are additive and documented in docs/app-api.md. Update both app mocks (app/src/lib/api/mock.ts and app/src-tauri/src/mock.rs) and the types.
7. UI for your feature:
   - Add the app controls the feature needs, using the design system and app/CONTRIBUTING.md.
   - EVERY control gets a catalog entry in app/src/help/tooltips.ts. The coverage test enforces this.
   - Add TUI keys and Launchkey mappings where it makes sense; README Controls documents them.
8. When the manuals don't pin a behaviour down, record "Decision: X, because Y" in the PR body and the issue. Prefer Genos-documented behaviour, and back it with corpus evidence.
9. Never commit style data, readable note listings, manual text or soundfonts. The repo is public.
10. Gates before the PR:
    - `cargo build --release`, plus `--no-default-features` and `--features plugins`
    - `cargo test --release`: all pass, the corpus actually runs, golden digests unchanged unless explained
    - no new clippy warnings
    - `cargo test` in app/src-tauri
    - `npm run verify` in app/
11. Commit messages end with: `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`
12. Open the PR against integration/m3-ui. The body includes Decisions and "Closes #N", and ends with `🤖 Generated with [Claude Code](https://claude.com/claude-code)`. DO NOT MERGE.
13. If git signing fails, stop and report.
14. Quote paths containing spaces. Never backslash-escape spaces.

**Landing policy (owner, 2026-09-24):**
- A PR is reviewed only once it is green (all gates pass) and MERGEABLE against the latest integration branch. Before reporting or requesting review, merge integration in with a merge commit.
- Review findings block only for real bugs (wrong behaviour in normal use, stuck notes, crashes, data loss, real-time safety violations) or big regressions. Edge cases in new features, polish and extra permutations go into a follow-up issue linked from the PR. They do not get another review round.
- The coordinator merges every approved, green and mergeable PR at once. After each merge, the other in-flight branches merge integration in again right away, so none stays conflicting.

**Small PRs (owner, 2026-09-24):** each PR covers ONE concern, aiming to be approved on the first look. Split big tickets into a chain of small PRs: engine, then session/API, then UI. Final acceptance happens on the integration branch, not in the PR.

**Handing a PR to the merge steward:** once it is green and mergeable against the latest integration, and verified or approved, post a PR comment `READY <full-40-char-head-sha>` and add the `ready-to-merge` label. The steward merges it at exactly that SHA. If you push again after that, post a new READY comment.

**Shared scratch directories:** agents share the scratchpad. Always use unique file names, prefixed with your PR or issue number (e.g. `pr92-body.md`), never generic names like `body.md`. Re-read a PR body on GitHub after posting it.

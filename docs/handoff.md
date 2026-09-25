# Handoff: wave M5–M8 (paused 2026-09-24)

This file is the starting point for a fresh coordinator. Everything described here is pushed to GitHub. No local state is needed.

## Where things are

- `main` has M1–M4 plus the M3 app/engine work (merged in #97).
- `develop` is the rolling integration branch. All wave PRs target it. The owner playtests it before it goes to `main` (merge commit, not squash).
- Merged into integration during this wave:
  - #93 Controllers (#34)
  - #95 Multi Pads (#37)
  - #96 Chord Looper, part solo, Style Track Mute, tempo 5–500, metronome (#29, #30)
- Suggested later: rename `develop` to `develop` once this wave lands (git-flow lite: `develop` → `main` with merge commits, and tag releases).

## Status (updated 2026-09-24, evening)

The M5–M8 wave is fully merged into `develop`, and all gates are green. No PRs are open.

| Merged | What |
|---|---|
| #93 #95 #96 | Controllers; Multi Pads; Chord Looper, solo, track mute, metronome |
| #99 #100 #101 | Registration and Playlist; Keyboard Harmony and Arpeggio; engine follow-ups (chord settle, per-section routing, MIDI hot-plug) |
| #94 #92 | Section timing (a style change waits for the Ending); fills, Half Bar, Stop ACMP modes, style-change rules, OTS Link Timing defaulting to At Main Section Change (owner preference) |
| #105 #106 | AU plugins on the keyboard parts (#91); sound library and program map (#103) |
| #98 | iReal chart player (M8) |
| #108 #112 | Fixes for flaky tests |

**Next steps:**
1. The owner playtests integration.
2. Merge `develop` to `main` with a merge commit.
3. Optionally rename the branch to `develop`.

**Follow-ups with deferred edge cases:**
- #104: plugins, including a registrable for a part's plugin.
- #107: section-timing registrables and edge cases.
- #109: sound library, including a registrable for a part's patch.
- #110: iReal and looper edge cases.
- #111: fills and rules edge cases, including a style chosen while an Ending is only queued.

**Process now in place** (see `docs/agents/wave-brief.md`):
- small PRs;
- a PR is green and mergeable before review;
- only real bugs block a merge;
- a `READY <sha>` comment plus the `ready-to-merge` label hands the PR to a long-running merge-steward agent, which merges it and runs the gates on integration after every merge.

## Also outstanding

- **Plugin hosting phase 2 (#91):**
  - Parts can use AU plugins.
  - The voice picker gets a Plugins tab.
  - Editor windows, mixer badges, registration preload.
  - Hardening items from the #88 review.
  - Third-party AUv2 plugins run out of process by default.
  - Start it after the wave. The design is in `docs/plugin-hosting.md`.
- **Sound library (#103):** a user patch list built from SoundFont presets and AU plugins (and later VST3/CLAP). Parts, OTS and registration pick from it, and a program map (GM family rules plus overrides) sends every style part to one of about 20 reusable patches. SoundFont-only patches could ship first; plugin patches need #91.
- **#31 remainder:** three split points (Style, Left, Right 3). Left Hold is #202.
- **Clippy drift:** a newer toolchain flagged lints in files no one touched (`src/sff.rs`, `src/theory.rs`). If `clippy -D warnings` fails on untouched code, fix it in a separate small PR rather than inside a feature PR.
- After the wave: the owner playtests `develop`, then it merges to `main`.

## How the swarm ran (reuse this)

- `docs/agents/wave-brief.md`: the brief every implementation, fix and review agent read first. It has the hard rules: push after every step, real-time safety, the mixer principle, wire compatibility, both mocks, tooltips, gates, and no copyrighted data (the repo is public).
- `docs/agents/ui-brief.md` and `docs/agents/ui-review.md`: extra rules for app UI work and UI reviews.
- `docs/agents/wave-workflow.js`: the workflow script. Each track runs implement → adversarial review → fix, for up to 3 rounds, with no merging inside the workflow. Replace `<SCRATCHPAD>` with a real scratch directory.
- `docs/agents/board-archive.md`: the coordination board from this wave. Agents appended CLAIM, NEED, FINDING, CHANGED and RELEASE lines, and it includes cross-PR notes still pending. For a new wave, copy it to a scratch directory outside the repo; agents appending to a tracked file would conflict.
- `.claude/scripts/merge-integration.sh <pr> [expected-head-sha]` squash-merges a PR only when its base is `integration/*` and the head matches what was reviewed. The coordinator does the merges; agents never merge. Auto mode blocks merges unless the owner approves them.

## Lessons

- **Disk:** every worktree builds its own `target/`. About 30 worktrees filled the disk twice. Install `sccache` (`brew install sccache`, then `RUSTC_WRAPPER=sccache`). Remove each worktree as soon as its PR merges, after checking it is clean and pushed. Delete `target/` in idle worktrees.
- **Push often.** A killed agent keeps only what it pushed.
- **Serial merges cause conflicts.** Each merge into integration makes the other PRs conflict. Budget one "merge integration in" step per PR per landing, and re-check cross-PR semantics, not just text. For example, solo × controllers: pedals and wheels must follow `Parts::audible_mask()`.
- **Review rounds:** most PRs needed all three. The fix after the last review is unreviewed, so verify it (read the diff and run the gates) before merging.
- **Usage limits** killed agents mid-run. The workflow can be resumed, but a fresh session should just re-run the remaining tracks from this table.

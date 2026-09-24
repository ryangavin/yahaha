# Handoff: wave M5–M8 (paused 2026-09-24)

This file is the starting point for a fresh coordinator. Everything described here is pushed to GitHub. No local state is needed.

## Where things are

- `main` has M1–M4 plus the M3 app/engine work (merged in #97).
- `integration/m3-ui` is the rolling integration branch. All wave PRs target it. The owner playtests it before it goes to `main` (merge commit, not squash).
- Merged into integration during this wave:
  - #93 Controllers (#34)
  - #95 Multi Pads (#37)
  - #96 Chord Looper, part solo, Style Track Mute, tempo 5–500, metronome (#29, #30)
- Suggested later: rename `integration/m3-ui` to `develop` once this wave lands (git-flow lite: `develop` → `main` with merge commits, and tag releases).

## Open PRs (all target `integration/m3-ui`)

Every PR has had adversarial review rounds; the review comments are on each PR. "Unreviewed fix" means the last fix commits were pushed after the last review and still need verifying.

| PR | Branch | Milestone / issues | State | What's left |
|---|---|---|---|---|
| #92 | `m5/fills-rules` | M5: #24 #26 #27 #28 | **Approved (r2).** CONFLICTING. | Merge `origin/integration/m3-ui` in, resolve, and re-run the gates. Two cross-PR items are on the board: (1) whichever of #92/#94 lands second adds the `Change::HalfBar` arm to `Engine::change_point`; (2) whichever of #92/#99 lands second registers the Stop ACMP mode as a registration item (`Group::Style`, `setStopAcmp`, DL p.91). |
| #94 | `m5/section-timing` | M5: #22 #23 #25 | Review r3 failed. Unreviewed fix pushed (d2d7622, 12bd40a). Mergeable. | Verify the r3 blocker is fixed: re-striking the same chord after release must reach `Engine::set_chord`, so Style Retrigger and the Synchro Stop Window see it. Then run the gates and review the fix. |
| #98 | `m8/ireal-player` | M8: #89 | **Approved (r3).** Non-blocking fixes pushed (a6be392). CONFLICTING in about 23 files. | Merge integration in; a first attempt was aborted, so start clean. Decide how the chart player's chords and the Chord Looper (#96) share the chord source; one owns it at a time. After #101 lands, route chart chords past the chord-settle window (use `apply_chord`, not the settled `set_chord`). Fix the Decision wording: the lane shows the written chords, not the transposed ones. |
| #99 | `m7/registration` | M7: #36 #38 | Review r3 failed. Unreviewed fix pushed (d78f71e…7826bb4). Mergeable. | Verify B1: a recall restores only the Style part levels the player set, using a `player_set` mask with serde default all-true for old files. B2, the merge conflict, is already resolved. Parameter Lock is stubbed and tracked in #102. Run the gates and review the fix. |
| #100 | `m6/harmony-arp-wiring` | M6: #32 #33 | Review r3 failed. Unreviewed fix pushed (6f14134, b078969, 19d30b5). Mergeable. | Verify B1: PANIC and unplugging must release control-side switches that a Hold pedal kept on (Arpeggio Hold, Kbd Harmony/Arp). Verify B2: the TS mock handles the two new pedal functions, matching the session and the Rust mock. Run the gates and review the fix. |
| #101 | `engine/followups` | #47 #64 #65 #74 | Review r1 failed. Fix r1 pushed and self-reported as passing the gates. Mergeable. | Needs review r2. The r1 blockers: Chord Match pads during the chord-settle window, and `send_init` using a stale section after an Ending. Watch the interaction with #98 (see above). |

Suggested merge order: #101, #94, #92, #100, #99, #98. The engine/timing base goes first. #98 goes last because it has the most conflicts and depends on #101. After each merge, the remaining PRs need `origin/integration/m3-ui` merged in; use a merge commit, not a rebase.

## Also outstanding

- **Plugin hosting phase 2 (#91):**
  - Parts can use AU plugins.
  - The voice picker gets a Plugins tab.
  - Editor windows, mixer badges, registration preload.
  - Hardening items from the #88 review.
  - Third-party AUv2 plugins run out of process by default.
  - Start it after the wave. The design is in `docs/plugin-hosting.md`.
- **Sound library (#103):** a user patch list built from SoundFont presets and AU plugins (and later VST3/CLAP). Parts, OTS and registration pick from it, and a program map (GM family rules plus overrides) sends every style part to one of about 20 reusable patches. SoundFont-only patches could ship first; plugin patches need #91.
- **#31 remainder:** three split points (Style, Left, Right 3) and Left Hold.
- **Clippy drift:** a newer toolchain flagged lints in files no one touched (`src/sff.rs`, `src/theory.rs`). If `clippy -D warnings` fails on untouched code, fix it in a separate small PR rather than inside a feature PR.
- After the wave: the owner playtests `integration/m3-ui`, then it merges to `main`.

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

# Orchestrator handoff: wave 2 (paused 2026-09-24, late evening)

You are taking over as the ORCHESTRATOR (coordinator) of the yahaha agent swarm. This page has everything you need to resume. You don't need the previous session's context.

## 0. Read first
- `docs/agents/wave-brief.md`: the hard rules and the owner's policies: landing, small PRs, the READY handoff, unique scratch file names. Every agent you spawn must follow it.
- `docs/agents/wave2/wave2-brief.md`: how a wave-2 track works (a chain of small PRs). Include it in every track agent's prompt.
- `docs/agents/wave2/bugs-brief.md`: the extra rules for owner-reported playing bugs. Reproduce the bug in a failing test first.
- `docs/agents/wave2/verify-brief.md`: the brief for a final check before landing (review plus merging integration in).
- `docs/handoff.md`: project status and what shipped in the M5–M8 wave.
- Your memory: `merge-when-ready`, `agents-push-often` and `wave2-paused-state`.

## 1. The project and the owner
yahaha is a Rust software arranger for macOS. It plays Yamaha Genos style files like the hardware does, driven by a Novation Launchkey MK4. The desktop app is Tauri, with Svelte 5 in `app/`. Output goes to a built-in SoundFont synth (a vendored rustysynth), AU plugin instruments, and the virtual MIDI port `yahaha`, which feeds Ableton. The scope is **playability only**.

Chord following must match a real Genos 1:1. The repo `ryangavin/yahaha` is **PUBLIC**, so never commit style data, manual text or soundfonts.

**Owner preferences, all binding:**
- **Speed.** Small PRs, each one concern, aimed at approval on the first look. Final acceptance happens on the integration branch.
- **Green and mergeable before review.** A branch is never left conflicting.
- **Only real bugs block a merge:** wrong behaviour in normal use, stuck notes, crashes, data loss, real-time safety violations, or big regressions. Edge cases go to follow-up issues.
- **Merge the moment a PR is ready.** The owner has approved integration merges.
- **Mixer principle:** a part's volume is ONLY its CC7, sent unchanged. There is no hidden gain; the master fader is the only non-CC gain.
- **Every control gets help text** in the tooltip catalog, which a coverage test enforces.
- **Clean up after merging.** Remove a worktree once its PR merges, after checking it is clean and pushed. Never delete things beyond that without asking. An earlier instruction from the owner, given during a disk inventory: "DO NOT DELETE ANYTHING YOURSELF".
- **Owner-confirmed Genos behaviour:**
  - A style change waits for a playing Ending to finish.
  - OTS changes wait for the section or style change. OTS Link Timing defaults to At Main Section Change, by owner preference.
  - AU plugins are fine for now; VST3 comes later.

## 2. Branches and flow
- `main` has the M1–M8 work plus AU plugins and the sound library (merged via #113, a merge commit).
- **`develop` is the rolling integration branch.** Every PR targets it. When the owner says so, merge it to `main` with a PR and a merge commit (`gh pr merge --merge`).
- **Landing a PR:**
  1. The agent opens it and gets it green and mergeable.
  2. The agent posts `READY <full sha>` and adds the `ready-to-merge` label.
  3. A **merge steward** agent reviews it for real bugs only and runs `bash .claude/scripts/merge-integration.sh <pr> <sha>`.
  4. The steward gates integration afterwards and opens small `fix/integration-*` PRs for any regression.
  5. It asks the authors of any PRs that became conflicting to merge integration in.
- **Gates:**
  - `cargo build --release` (also with `--no-default-features` and `--features plugins`)
  - `cargo test --release`
  - `cargo test --release --features plugins`
  - `cargo test` in `app/src-tauri`
  - `npm run verify` in `app/`
  - clippy: no new warnings in touched files

  Corpus and soundfonts are symlinked in worktrees:

  ```bash
  ln -sfn "/Users/ryan/The Source/yahaha/corpus" corpus
  ln -sfn "/Users/ryan/The Source/yahaha/soundfonts" soundfonts
  ```

## 3. State at pause
**Integration head: `4cb893d`, NOT gated.** The last green run was on `a2fd869`. Merged since then and ungated: #119 (Parameter Lock core), #121 (per-slot plugin rack commands), #118 (a queued Ending holds a style change), #124 (Looper ON/OFF).

**Ready to merge** (labelled, with a READY SHA):
- #123 plugin-patch audition
- #125 plugin state read on a worker thread
- #126 style-settings registrable

**Open, not ready:** #120, the buffer size setting (draft). Its `app/src-tauri` tests haven't run, and `npm run verify` needs a rerun because of an unrelated timeout under load.

**Tracks.** Each branch is pushed. A worktree also exists under `.claude/worktrees/agent-*`, but you can't resume those agents. Spawn fresh ones with the briefs.

| Track | Branch / PRs | Next step | Issues |
|---|---|---|---|
| **stylechange** (owner bug: "accompaniment stops after changing styles" and "some stuff is really quiet") | `stylechange/stress-test` @9ae492c; fix commit 0eac298 | **Top priority.** Endings fade parts out with CC11 (often the bass to 8), and nothing reset it, so later styles play nearly silent. The fix resets CC11 to 127 on Style channels whenever a setup is sent. Stress test: `tests/style_change_stress.rs`, 40 failures in 300 before the fix, 0 after. Drop the two ignored debug tests, run one pass with `YAHAHA_STRESS_SF2`, run the gates (golden digests may change because CC11 is now sent), then open the PR with "Closes #122" and post READY. | #122 |
| **sound** (owner: "quiet" and "stiff") | `sound/investigate` @541f5ae (WIP probes) | Do NOT open its CC11 PR; stylechange owns that fix. Engine timing was measured as exact, so "stiff" is likely the SoundFont timbre. Still check velocity and note-length handling. Then report to the owner and close #127, or fix what turns up. | #127 |
| **ending** (owner: tap tempo while playing; "endings feel immediate") | `bugs/ending-timing` @581ec6a (probes only) | Tap Tempo PR: tap during playback sets the tempo, and Section Reset gets its own assignable function. For #129 the timing is correct; nearly every Genos Ending I is a one-hit chord in the style data. Waiting on the owner (see §4). Then check the ritardando curve in a test. | #128, #129 |
| **browser** (owner's top feature) | `browser/loading` @d13cdfd | PR 1 is committed but no PR is open yet. It loads every .sf2 in the folder, drops the canonical `--soundfont`, and adds a "Default sound set" setting (Auto = most GM-complete). Open it, gate it, post READY. Then: PR 2 catalog API, fetched on demand like the style library; PR 3 one Sound Browser replacing the GM/Library/Plugins tabs; PR 4 program-map pickers; PR 5 saved sounds. The design note is on #117. | #117 |
| **regist** | #115 merged; #126 READY | Plugin registrable (#104): First reproduce in a DLS test the bug where Right 1 is silent after recalling a GM registration then a plugin one. Then warm-preload a bank's plugins. | #104, #109 |
| **plug-rt** | #114 and #121 merged; #125 READY | PR 4: plugin lookup on the load thread. PR 5: editor window lifecycle (close on instance change, dispose on the dispose thread, skip the save when there's no plugin). | #104 |
| **patches** | #116 merged; #123 READY | PR 3 `savePartAsPatch` saves what's playing; PR 4 library cleanups; PR 5 the hand-over race. No new voice-picker UI, because #117 replaces those tabs. | #109, #103 |
| **transport** | #118 and #124 merged; `transport/ots-link-pending-style` @f1628af | PR 3: OTS during a Fill or Intro while a style change is pending. The test FAILS: the keyboard parts change at the takeover in the middle of a Fill instead of at Main start, and the fix in `src/session/ots.rs` `pump_ots_link` is incomplete. Then PR 4, folding the duplicate Fill variants, and PR 5, the #107 decisions (AI Full Keyboard dyad; style change from Intro/Fill waits for the bar). | #111, #110, #107 |
| **plock** | #119 merged | PR 2: Parameter Lock page in Settings. The tooltips exist; add a "Lock" tab. | #102 |
| **plug-ux** | #120 draft | Finish #120's gates and post READY. Then:<br>• fallback-to-in-process badge;<br>• per-plugin in-process override in the scan cache (minimal UI; the browser carries it later);<br>• CPU/overrun readout;<br>• TUI and Launchkey mappings. | #104 |

**Known flaky or suspicious tests** (check them on integration first):
- `tests/engine_no_alloc.rs::harmony_and_arpeggio_do_not_allocate` failed 2 of 4 runs with "frees on the engine thread". It could be a real real-time regression.
- `session::tests::launchkey_buttons_are_what_the_state_says` failed once under load.
- `Browser.test.ts` "stays virtualised with 60k styles" times out under load.

## 4. Open questions for the owner
1. **#129:** which Ending did they press (Launchkey pad 100/101/102 = Ending I/II/III), and on which style? If it was Ending I, that's the style's one-hit data, and the Genos plays it the same way.
2. **#118:** is it right that a style chosen while an Ending is queued but not yet playing makes the Ending play in the OLD style, with the style waiting? That's what was implemented, and it's unconfirmed.
3. What is the Genos factory default for OTS Link Timing? yahaha keeps "At Main Section Change" regardless.

## 5. Suggested resume order
1. Spawn the **merge steward**, using the steward prompt pattern in §6.
   - First job: gate integration at its current head, and check the flaky harmony/arpeggio no-alloc test.
   - Then merge the READY PRs #123, #125 and #126.
2. Spawn **stylechange** (the owner-reported silence bug) and **browser** first.
3. Then spawn ending, transport, regist, patches, plug-rt, plock and plug-ux. Mind the disk (§7): about 6 building agents at once is the practical limit.
4. Once the owner answers §4, relay the answers to the ending and transport tracks.

## 6. How to spawn agents (patterns that worked)
- **Track agent:** use the Agent tool with `isolation: "worktree"` and `run_in_background: true`. The prompt says "Read and follow `docs/agents/wave2/wave2-brief.md` fully", then gives the TRACK name, the issues and the ordered list of small PRs, and names the branch and draft scripts to resume from.
- **Merge steward:** a long-running background agent that:
  - polls every 2–3 minutes for labelled PRs;
  - checks that the head equals the SHA in the READY comment and that the PR is MERGEABLE/CLEAN;
  - does a quick review for real bugs only;
  - merges with the script;
  - gates the latest head once;
  - comments on PRs that became conflicting;
  - messages the coordinator one line per merge.

  It must never push to integration or main directly, never force-push, and only kill processes it started itself: an earlier broad `pkill -f "cargo test"` killed other agents' runs. Its helper scripts are in `docs/agents/wave2/steward/steward-*.sh` (set `SCRATCHPAD` and `STEWARD_WORKTREE`).
- **After each merge,** tell the in-flight agents whose PRs became conflicting to merge integration in.
- **Pausing:** message every agent "PAUSE NOW: commit and push, post READY if green, then stop and reply with one line". Record the state in memory.

## 7. Operational gotchas
- **Commit signing goes through 1Password, which auto-locks.** Agents report "SIGNING FAILED" (fill whole buffer). Ask the owner to unlock 1Password, then resume the agents. Never disable signing.
- **Disk:** each worktree builds its own `target/` of about 2–3 GB.
  - sccache is on globally (`~/.cargo/config.toml`).
  - Watch `df -g /System/Volumes/Data`. Below about 10 GB, delete `target/` in worktrees whose agents are idle or finished, after checking no process uses them.
  - Remove a worktree once its PR merges. Squash merges don't make the branch an ancestor of integration, so check "pushed" against `origin/<branch>`.
- **Auto mode may block a subagent's `gh pr merge`.** So far the steward's script calls have been allowed. If they're blocked, the coordinator merges.
- **Unique scratch file names.** Agents overwrote each other's `body.md` before this rule existed.
- **Don't push docs directly to integration while PRs are queued:** every push forces all of them to re-merge.
- **Launching the app:** `cd app && cargo tauri dev`. Playtesting and final acceptance are the owner's.

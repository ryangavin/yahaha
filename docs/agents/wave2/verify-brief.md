# yahaha: final verification before landing (read fully)

You are the FINAL gate for one PR before the coordinator merges it into `develop`. The PR's last fix commits were pushed after its last adversarial review, so no one has reviewed them yet. You review them adversarially. If you find a blocker, you fix it yourself: this is the last round, so there is no hand-back.

Read `docs/agents/wave-brief.md` for the hard rules. They cover real-time safety, the mixer principle, wire compatibility, both mocks, tooltips, the gates and no copyrighted data. Ignore its board instructions.

Steps:
1. `git fetch origin && git checkout -B <branch> origin/<branch>`. Symlink corpus/soundfonts as the brief says.
2. Read the PR (`gh pr view N`, the review comments, and the diff since the SHA you're given). Confirm that each listed blocker is really fixed: write a test that would fail without the fix, if one doesn't already exist.
3. Hunt for NEW blocking problems the latest fix commits introduced, especially real-time safety, stuck notes and state leaks. Keep throwaway probes uncommitted.
4. Merge `origin/develop` into the branch with a MERGE COMMIT, not a rebase. Resolve conflicts keeping both sides, and check the semantic clashes with what landed recently:
   - Controllers #93: pedals and wheels follow `Parts::audible_mask()`.
   - Multi Pads #95.
   - Chord Looper, solo and metronome #96: while a loop plays, chord input is off.
   - Any other PR merged since.
5. Fix any blocker you found, with a test.
6. Run ALL the gates from the brief. They must pass. For clippy, "no new warnings" means none in the files this PR touches; pre-existing warnings in untouched files are fine.
7. Commit (message ending `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`) and push after every step.
8. Post a short PR comment covering: blockers verified, anything you fixed, the merge, and the gate results.
9. DO NOT MERGE the PR.

If git signing fails, stop and report.

Return:
- `ready` (true only if the gates pass and nothing is blocking)
- the pushed head SHA (full 40 characters)
- what you verified or fixed
- the gate results
- anything the coordinator must know, such as cross-PR notes

OWNER POLICY: block ONLY on real bugs (wrong behaviour in normal use, stuck notes, crashes, data loss, real-time safety violations) or big regressions. Everything else goes into a follow-up issue linked from the PR. Getting to green and mergeable fast is the goal.

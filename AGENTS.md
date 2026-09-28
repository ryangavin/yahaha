# AGENTS.md

This file is the one place that says what must pass. CI on the pull request runs these commands, and it must pass before merge. There is no other required process.

## Checks

Run the ones that cover your change. Each command must exit 0.

- `cargo check --all-targets --features plugins` on macOS, or `cargo check --all-targets` on Linux. This compiles everything: the library, the binary, the tests and the benches. Run it after any Rust change.
- `cargo check --no-default-features --lib`. This compiles the library with no features. Run it after any Rust change. It behaves the same on every platform, so CI runs it only on Linux.
- `cargo test --profile test-fast --features plugins` on macOS, or `cargo test --profile test-fast` on Linux. This runs all of the Rust tests; on Linux, plugin hosting isn't built. Run it after any Rust change. To run a single test, add its name.
- `cargo test` in `app/src-tauri`. These are the app shell's Rust tests, and on macOS they run with plugins. Run them when `app/src-tauri`, `src/api` or `src/session` changed. CI runs them on macOS; on Linux it only compile-checks the no-plugins version.
- `npm run verify` in `app/`. This runs the type check, lint and web tests. Run it when anything under `app/` changed, after `npm ci` on a fresh checkout.

The corpus tests need the git-ignored `corpus/` folder; in a worktree, symlink it from the main checkout. Without it they skip without failing, so a pass without the corpus doesn't cover them.

## Feedback loop

While editing, run [bacon](https://dystroy.org/bacon) headless in your worktree (`cargo install --locked bacon`). It re-runs its job on every source change and, after every run, rewrites `.bacon-result.json` (git-ignored). Start it as a background command, and stop it when you're done or before switching job.

- `bacon --headless` runs the `check` job: `cargo check --all-targets --features plugins`. On Linux, use `bacon --headless check-portable`.
- `bacon --headless test -- <filter>` runs targeted tests under the `test-quick` profile. Add `--lib` before the filter for library tests only.
- `jq '{error_code, stats}' .bacon-result.json` shows the latest result. `error_code` is null when the command exited 0; `stats` counts errors, warnings and failed tests.
- `jq -r '.lines[] | [.content.strings[].raw] | join("")' .bacon-result.json` lists each error, warning and failed test with its location.
- `find src tests Cargo.toml -newer .bacon-result.json` prints nothing when the result covers your last edit. If it prints a file, the run hasn't finished yet; read the result once, later, rather than looping.

The jobs are in `bacon.toml`: `check`, `check-portable`, `clippy` and `test`. bacon is only for fast feedback; the checks above and CI decide.

## Review

Every PR gets one review, posted on the PR itself as line comments plus a verdict (`gh pr review --approve` or `--request-changes`). The review runs as soon as the PR opens, alongside CI rather than after it. A PR merges when CI passes and the review is addressed.

**Review account:** `satori-miyamoto`. Post reviews with `GH_CONFIG_DIR=~/.config/gh-yahaha-bot gh …`, and check first that `gh api user --jq .login` prints `satori-miyamoto`. Use it only for reviews. Everything else (commits, PRs, merges) uses the owner's default `gh` login. `main` requires CI plus an approval from this account, with no admin bypass.

Reviewers flag only these, each with the file, the line and a one-line reason:

- **Real-time safety:** no allocation, locks, panics or blocking I/O on the audio, engine or MIDI threads.
- **Mixer:** a part's level is only its CC7 plus master. There's no hidden gain.
- **Parity:** nothing is hardware-only. Every Launchkey function has an app control; every new control has a tooltip; every new command is in both mocks (TS and the Rust dev mock), in docs/app-api.md and in EVERY_CMD in tests/api_wire.rs.
- **Layering:** no new upward imports (for example, sff or theory reaching into engine or session; synth reaching into session or plugin). Move shared types down instead.
- **Public repo:** no style data, soundfonts, manual text or real plugin state blobs committed.
- **Bugs:** wrong logic, broken migrations of saved files, and tests that would pass without the code under test working.

Don't comment on style or naming. Put small edge cases in a follow-up issue rather than blocking the PR.

## Linux

See "Developing on Linux" in README.md.

## Branches

PRs target `develop` and are squash-merged. `develop` merges into `main` only when the owner says so, with a merge commit, so `main` keeps `develop`'s commits and the next release PR shows only new work. The rulesets enforce both: `develop` allows squash only, `main` allows merge commits only.

## Never commit

Style data, manual text, soundfonts, or real plugin state. Research transcripts and frames live in `../yahaha-research`, not this repo. Mocks use the fake "Sampler Deluxe" plugin.

## Corpus

It's git-ignored. In a worktree, symlink it: `ln -s "<main checkout>/corpus" corpus`.

## Engine rules

No allocation, locks or panics on the engine, MIDI or audio threads. Mixer levels are CC7 plus master only; the approved exceptions are Fade, Style volume and Multi Pad volume.

## Controls

Every control has a tooltip in `app/src/help/tooltips.ts`. Every command is in both mocks (TS and the Rust dev mock), in `docs/app-api.md`, and in `EVERY_CMD` in `tests/api_wire.rs`. Nothing is hardware-only: every Launchkey function has an app control.

## rustysynth

rustysynth is an unmodified dependency from crates.io (`rustysynth = "1.3.6"`). Never vendor or patch it. Build what it lacks in yahaha, on its public API (see `src/synth/`).

## Layering

No new upward imports between modules. A workspace split is planned: core → sff → dsp → engine → plugin, with the facade on top.

## Genos behaviour

Answer from the manuals in `docs/manuals` (git-ignored, local only), not by asking the owner. If the manuals are silent, pick the behaviour closest to the Genos and record "Decision: ..." in the PR body.

## Tests

Tests may use the real plugin cache, but a test that needs a stable plugin list uses a mock cache.

## Disk

`app/src-tauri` test builds are large. Remove `target/debug` after them if disk is tight. Keep at least 8 GB free.

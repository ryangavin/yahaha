# AGENTS.md

This file is the one place that says what must pass. CI runs these commands in `develop`'s merge queue, on the PR merged onto `develop`'s tip, and it must pass before anything lands. On the PR itself CI builds nothing and its checks pass at once, so **run the checks below that cover your change locally before you open or update a PR**; if the queue then fails, fix it and re-queue. There is no other required process. The queue skips the jobs a change can't affect: a Markdown-docs-only change runs only `api-doc`, a web-only change skips the Rust jobs, and a Rust-only change skips the web job (`.github/workflows/ci.yml`, the `changes` job, says which files count as what).

## Checks

Run the ones that cover your change. Each command must exit 0.

- `cargo test --profile test-quick --features plugins` on macOS, or `cargo test --profile test-quick` on Linux. This compiles every target (the library, the binary, the tests and the example; there are no benches) and runs all of the Rust tests, with debug assertions and overflow checks on; on Linux, plugin hosting isn't built. Run it after any Rust change. To run a single test, add its name.
- `cargo check --no-default-features --lib`. This compiles the library with no features. Run it after any Rust change. It behaves the same on every platform, so CI runs it only on Linux.
- `cargo test` in `app/src-tauri`. These are the app shell's Rust tests, and on macOS they run with plugins. Run them when `app/src-tauri`, `src/api` or `src/session` changed. CI runs them on macOS (the `macos-app` job); on Linux it only compile-checks the no-plugins version (`cargo check --all-targets` in `app/src-tauri`).
- `npm run verify` in `app/`. This runs the type check, lint and web tests. Run it when anything under `app/` changed, after `npm ci` on a fresh checkout. CI runs it on Linux (the `web` job).
- In bash, from the repo root: ``diff <(awk '/^## AppCmd/{f=1} /^### Result/{f=0} f && /^\| `/' docs/app-api.md | cut -d'|' -f2 | grep -o '`[^`]*`' | tr -d '`' | sort -u) <(awk '/^const EVERY_CMD/,/^\];/' tests/api_wire.rs | sed -n 's/^ *r#"{"type":"\([^"]*\)".*/\1/p' | sort -u)``. This checks that `EVERY_CMD` in `tests/api_wire.rs` names exactly the commands in the AppCmd tables of `docs/app-api.md`, and prints any difference. Run it when either file changed. CI runs it on every queued change, docs-only ones included (the `api-doc` job). No code or test compiles in or reads a committed file under `docs/`; keep it that way, so a docs-only change can't change what CI tests.
- `cargo run --no-default-features --example api_doc_check -- docs/app-api.md`. This checks that every `{"type": ...}` JSON example in `docs/app-api.md`, and its example `AppState`, parses into the app API's Rust types (`AppCmd`, `Event`, `AppState`) and serializes back unchanged; it prints each mismatch and exits 1. The doc is read at run time from the path given, never compiled in. Run it when `docs/app-api.md` or the API types in `src/api` changed. CI runs it on every queued change, docs-only ones included (the `api-doc` job). Its own tests run with `cargo test`.

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

Every PR gets one review, posted on the PR itself as line comments plus a verdict (`gh pr review --approve` or `--request-changes`). The review runs as soon as the PR opens. A PR merges when the review approves and CI passes in the merge queue.

**Review account:** `satori-miyamoto`. Post reviews with `GH_CONFIG_DIR=~/.config/gh-yahaha-bot gh …`, and check first that `gh api user --jq .login` prints `satori-miyamoto`. Use it only for reviews. Everything else (commits, PRs, merges) uses the owner's default `gh` login. `main` requires CI plus an approval from this account, with no admin bypass.

Reviewers flag only these, each with the file, the line and a one-line reason:

- **Real-time safety:** no allocation, locks, panics or blocking I/O on the audio, engine or MIDI threads.
- **Mixer:** a style feature is mapped onto one of yahaha's own mixer concepts (see "Engine rules"), not added as a new, hidden or duplicate level. Anything that changes how loud a part is shows on a control: that part's strip or the group control that scales it.
- **Parity:** nothing is hardware-only. Every Launchkey function has an app control; every new control has a tooltip; every new command is in both mocks (TS and the Rust dev mock), in docs/app-api.md and in EVERY_CMD in tests/api_wire.rs.
- **Layering:** no new upward imports (for example, sff or theory reaching into engine or session; synth reaching into session or plugin). Move shared types down instead.
- **Public repo:** no style data, soundfonts, manual text or real plugin state blobs committed.
- **Bugs:** wrong logic, broken migrations of saved files, and tests that would pass without the code under test working.

Don't comment on style or naming. Put small edge cases in a follow-up issue rather than blocking the PR.

## Linux

See "Developing on Linux" in README.md.

## Branches

PRs target `develop` and are squash-merged. `develop` merges into `main` only when the owner says so, with a merge commit, so `main` keeps `develop`'s commits and the next release PR shows only new work. The rulesets enforce both: `develop` allows squash only, `main` allows merge commits only.

PRs into `develop` land through its merge queue. Once the review approves, run `gh pr merge --auto` (no `--squash`: the queue sets the method): GitHub queues it, runs CI on the PR merged onto `develop`'s tip (with anything queued ahead of it), and squash-merges it if that passes. Nobody updates branches by hand or waits to press merge. A PR that fails in the queue drops out of it; push the fix and run `gh pr merge --auto` again.

## Never commit

Style data, manual text, soundfonts, or real plugin state. Research transcripts and frames live in `../yahaha-research`, not this repo. Mocks use the fake "Sampler Deluxe" plugin.

## Corpus

It's git-ignored. In a worktree, symlink it: `ln -s "<main checkout>/corpus" corpus`.

## Engine rules

No allocation, locks or panics on the engine, MIDI or audio threads.

Mixer: support everything a style does, but map it onto yahaha's own concepts rather than adding a gain stage per feature. A part's level is its fader (its CC7) plus master. The Style volume, the Multi Pad volume and the Fade scale the CC7 that's sent, not the audio. A part's EQ is tone on its strip; it may boost as well as cut. Nothing changes a part's loudness without showing on a control: its own strip, or the group control that scales it (the Style and Multi Pad volume faders, the Fade). Where a style feature has no mapping yet and adding one is small, extend our concept rather than drop or fake the feature (for example, a filter type our EQ lacks: add the filter type). A large gap goes in a follow-up issue, named in the PR body.

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

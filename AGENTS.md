# AGENTS.md

This file is the one place that says what must pass. CI runs these commands in `develop`'s merge queue, on the PR merged onto `develop`'s tip, and it must pass before anything lands. On the PR itself CI builds nothing and its checks pass at once, so **run the checks below that cover your change locally before you open or update a PR**; if the queue then fails, fix it and re-queue. There is no other required process. The queue skips the jobs a change can't affect: a Markdown-docs-only change runs only `api-doc`, a web-only change skips the Rust jobs, and a Rust-only change skips the web job (`.github/workflows/ci.yml`, the `changes` job, says which files count as what).

## Checks

Each command must exit 0. Locally you run the core suite and the checks that cover your change; the full suite runs only in CI's merge queue. Don't run the full suite (Rust or web) locally: with several agents at once it stalls the machine.

**The core suite** is THE local check. Run it after any Rust change, or keep `bacon --headless core` running (see "Feedback loop"):

- `cargo test --profile test-quick --features plugins --lib` on macOS, or `cargo test --profile test-quick --lib` on Linux.

It builds and runs the library's unit tests, with debug assertions and overflow checks on, and leaves out the slow tests: those marked `#[cfg(feature = "slow-tests")]` (corpus sweeps, long audio renders, plugin-hosting round trips, real-time waits), which aren't even compiled. It doesn't build the binary, the integration tests or the example either. On the 10-core Mac, warm, after a one-line edit in `src/session/mixer.rs`: 4.7 s to rebuild, then about 2 s to run. To run a single test, add its name. A new test that takes more than about 0.25 s goes behind `slow-tests`.

**Also run** the ones that cover your change:

- `cargo test --profile test-quick --features plugins --test it` (Linux: without `--features plugins`). The integration tests, most of them the no-allocation checks of the engine, audio and MIDI threads. Run them when you change code that runs on those threads. Add a test name to run one.
- `cargo test --profile test-quick --test api_wire`. The app API's wire format. Run it when `src/api` changed.
- The corpus tests. They need the git-ignored `corpus/` (see "Corpus"), so CI can't run them; they are behind `slow-tests` and run only here. When you change a module that has corpus tests, run them once for that module: `cargo test --profile test-quick --features plugins,slow-tests <module>` (for example `sff`, `capture`, `oracle`, `golden`, `session::style_change`, `fx::xg`, `engine::part_state`, `library`). Without the corpus they skip without failing, so a pass without it covers nothing.
- `cargo test --profile test-quick` in `app/src-tauri`. The app shell's Rust tests; on macOS they run with plugins. Run them when `app/src-tauri`, `src/api` or `src/session` changed. The app shell is a member of the root workspace, so this reuses the dependency builds of the core suite.
- In `app/`, after `npm ci` on a fresh checkout: `npm run check` and `npm run lint`, and `npx vitest run <file>...` for the web tests that cover your change. Run them when anything under `app/` changed. The whole web suite (`npm run verify`) runs in CI.
- `cargo check --no-default-features --lib`. The library with no features. Run it when you change `#[cfg(feature = ...)]` code.
- In bash, from the repo root: ``diff <(awk '/^## AppCmd/{f=1} /^### Result/{f=0} f && /^\| `/' docs/app-api.md | cut -d'|' -f2 | grep -o '`[^`]*`' | tr -d '`' | sort -u) <(awk '/^const EVERY_CMD/,/^\];/' tests/api_wire.rs | sed -n 's/^ *r#"{"type":"\([^"]*\)".*/\1/p' | sort -u)``. This checks that `EVERY_CMD` in `tests/api_wire.rs` names exactly the commands in the AppCmd tables of `docs/app-api.md`, and prints any difference. Run it when either file changed. CI runs it on every queued change, docs-only ones included (the `api-doc` job). No code or test compiles in or reads a committed file under `docs/`; keep it that way, so a docs-only change can't change what CI tests.
- `cargo run --no-default-features --example api_doc_check -- docs/app-api.md`. This checks that every `{"type": ...}` JSON example in `docs/app-api.md`, and its example `AppState`, parses into the app API's Rust types (`AppCmd`, `Event`, `AppState`) and serializes back unchanged; it prints each mismatch and exits 1. The doc is read at run time from the path given, never compiled in. Run it when `docs/app-api.md` or the API types in `src/api` changed. CI runs it on every queued change, docs-only ones included (the `api-doc` job). Its own tests run with `cargo test`.

**CI's merge queue** runs everything (`.github/workflows/ci.yml`): `cargo check --no-default-features --lib` and `cargo test --profile test-quick --features slow-tests` on Linux; `cargo test --profile test-quick --features plugins,slow-tests` on macOS (every target: the library, the binary, the integration tests and the example; slow tests included); `cargo check --all-targets` (Linux) and `cargo test --profile test-quick` (macOS) in `app/src-tauri`; `npm run verify` in `app/`; and the two API doc checks. CI has no corpus, so the corpus tests skip there.

## Builds and concurrency

- One Cargo workspace: the engine and the app shell (`app/src-tauri`) share `Cargo.lock` and the worktree's `target/`. Cargo at the repo root builds only the engine.
- Each worktree keeps its own `target/`. Don't point worktrees at one shared target directory: cargo names our crate's build outputs the same in every checkout and decides freshness by file times, so a worktree can reuse another worktree's build of different source and test the wrong code (measured: after a build in one checkout, the other checkout's build reported nothing to do).
- A new worktree's first core-suite build compiles our crate from scratch: 49 s on the 10-core Mac, using every core. Dependencies come from sccache when it's set up (`rustc-wrapper = "sccache"` in `~/.cargo/config.toml`). Warm, after an edit, a rebuild takes about 5 s and two worktrees rebuilding at once take 5.4 s each, against 4.7 s alone.
- So when several lanes build Rust at once, start them one at a time: the next lane's first build begins once the previous lane's first build has finished. After that, they can edit and run the core suite side by side.
- No thread caps (`RUST_TEST_THREADS`, `CARGO_BUILD_JOBS`): they measured no faster.

## Lanes

When an orchestrator splits work across agents, each lane owns its files and no two lanes edit the same file.

- **Contract files** (the app API): `src/api/*`, `docs/app-api.md`, `tests/api_wire.rs` (`EVERY_CMD`), `tests/fixtures/state.json`, `app/src/lib/api/types.ts`, `app/src/lib/api/mock.ts` and `mock.test.ts`, `app/src-tauri/src/mock.rs`, `app/src/help/tooltips.ts`, `app/src/help/actions.ts`, `app/docs/controls.md`. A change to the contract lands first, as its own small PR; the lanes that build on it merge `develop` once it lands.
- **Hotspots** (one owner per feature at a time): `app/src/panels/mixer/Mixer.svelte` and `Mixer.test.ts`, `Strip.svelte`, `src/synth.rs`, `src/synth/rack.rs`, `src/fx.rs`, `tests/synth_no_alloc.rs`.

## Feedback loop

While editing, run [bacon](https://dystroy.org/bacon) headless in your worktree (`cargo install --locked bacon`). It re-runs its job on every source change and, after every run, rewrites `.bacon-result.json` (git-ignored). Start it as a background command, and stop it when you're done or before switching job.

- `bacon --headless` runs the `check` job: `cargo check --all-targets --features plugins`. On Linux, use `bacon --headless check-portable`.
- `bacon --headless core` runs the core suite on every change. On Linux, use `bacon --headless core-portable`.
- `bacon --headless test -- <filter>` runs targeted tests under the `test-quick` profile. Add `--lib` before the filter for library tests only.
- `jq '{error_code, stats}' .bacon-result.json` shows the latest result. `error_code` is null when the command exited 0; `stats` counts errors, warnings and failed tests.
- `jq -r '.lines[] | [.content.strings[].raw] | join("")' .bacon-result.json` lists each error, warning and failed test with its location.
- `find src tests Cargo.toml -newer .bacon-result.json` prints nothing when the result covers your last edit. If it prints a file, the run hasn't finished yet; read the result once, later, rather than looping.

The jobs are in `bacon.toml`: `check`, `check-portable`, `core`, `core-portable`, `clippy` and `test`. bacon is only for fast feedback; the checks above and CI decide.

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

It's git-ignored, and only the corpus tests (behind `slow-tests`) read it. To run them in a worktree, symlink it: `ln -s "<main checkout>/corpus" corpus`. With the corpus, all of them together take about 5 minutes of every core, so run them one module at a time (see "Checks").

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

A worktree's `target/` takes about 0.7 GB for the core suite, 1.2 GB with every test target, and 2.3 GB once the app shell's tests are built too. Remove it with the worktree once its PR has merged. Keep at least 8 GB free.

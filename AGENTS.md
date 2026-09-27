# AGENTS.md

This file is the one place that says what must pass. CI on the pull request runs these commands, and it must pass before merge. There is no other required process.

## Checks

Run the ones that cover your change. Each command must exit 0.

- `cargo check --all-targets --features plugins` on macOS, or `cargo check --all-targets` on Linux. This compiles everything: the library, the binary, the tests and the benches. Run it after any Rust change.
- `cargo check --no-default-features --lib`. This compiles the library with no features. Run it after any Rust change.
- `cargo test --profile test-fast --features plugins` on macOS, or `cargo test --profile test-fast` on Linux. This runs all of the Rust tests; on Linux, plugin hosting isn't built. Run it after any Rust change. To run a single test, add its name.
- `cargo test` in `app/src-tauri`. These are the app shell's Rust tests. Run them when `app/src-tauri`, `src/api` or `src/session` changed.
- `npm run verify` in `app/`. This runs the type check, lint and web tests. Run it when anything under `app/` changed, after `npm ci` on a fresh checkout.

The corpus tests need the git-ignored `corpus/` folder; in a worktree, symlink it from the main checkout. Without it they skip without failing, so a pass without the corpus doesn't cover them.

## Feedback loop

For quick results while editing, see the bacon section once it lands (build/bacon).

## Linux

See "Developing on Linux" in README.md.

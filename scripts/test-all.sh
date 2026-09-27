#!/usr/bin/env bash
# The lean gate: every check once, one visible step at a time, stopping at the first failure.
# Each step prints only its pass/fail summary lines; a failing step prints its full log.
#
#   scripts/test-all.sh            # the gate
#   scripts/test-all.sh --all      # also the app steps, whatever the diff touches
#   scripts/test-all.sh core       # only the steps whose name contains "core"
#
# Steps:
# - build: `cargo check --no-default-features --lib`, compile-only, the no-feature library.
# - core: the Rust tests, once. On macOS with `--features plugins` (a superset of the plain
#   run); elsewhere without it (plugin hosting is macOS only).
# - app-rust, app-web: only when the diff against origin/develop touches app/, src/api or
#   src/session, or with --all.
#
# Cargo's defaults (all cores, default test threads). Tests build with the `test-fast`
# profile (see Cargo.toml): release-grade dependencies, quick incremental builds of our code.
set -uo pipefail
cd "$(dirname "$0")/.."
ROOT=$PWD
ALL=0
FILTER=""
for arg in "$@"; do
    case $arg in
    --all) ALL=1 ;;
    *) FILTER=$arg ;;
    esac
done
LOGS="$ROOT/target/gates"
mkdir -p "$LOGS"

step() {
    local name=$1 dir=$2
    shift 2
    [[ -n $FILTER && $name != *"$FILTER"* ]] && return 0
    local log="$LOGS/$name.log" start=$SECONDS
    printf '== %s: %s\n' "$name" "$*"
    (cd "$dir" && "$@") >"$log" 2>&1
    local rc=$?
    grep -E '^test result:|Test Files|Tests +[0-9]|svelte-check found' "$log" | grep -v 'ok\. 0 passed' | sed 's/^/   /'
    if [[ $rc -ne 0 ]]; then
        echo "   FAIL ($name, exit $rc, $((SECONDS - start))s); full log: $log"
        tail -n 60 "$log"
        exit "$rc"
    fi
    echo "   ok ($((SECONDS - start))s)"
}

# The app steps run when the change can reach the app: committed, staged or not.
app_touched() {
    local base
    base=$(git merge-base HEAD origin/develop 2>/dev/null) || return 0
    { git diff --name-only "$base"; git ls-files --others --exclude-standard; } |
        grep -qE '^(app/|src/api(/|\.rs$)|src/session(/|\.rs$|_tests\.rs$))'
}

step build "$ROOT" cargo check --no-default-features --lib
if [[ $(uname -s) == Darwin ]]; then
    step core "$ROOT" cargo test --profile test-fast --features plugins
else
    step core "$ROOT" cargo test --profile test-fast
fi
if [[ $ALL == 1 ]] || app_touched; then
    step app-rust "$ROOT/app/src-tauri" cargo test
    step app-web "$ROOT/app" npm run verify
else
    echo "== app-rust, app-web: skipped (no change under app/, src/api or src/session; --all runs them)"
fi
echo "all gates passed"

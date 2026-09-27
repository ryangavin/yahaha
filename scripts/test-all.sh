#!/usr/bin/env bash
# Runs every gate, one visible step at a time, and stops at the first failure.
# Each step prints only its pass/fail summary lines; a failing step prints its full log.
#
#   scripts/test-all.sh            # all gates
#   scripts/test-all.sh core       # only the steps whose name contains "core"
#
# Tests build with the `test-fast` profile (see Cargo.toml): release-grade optimisation of
# the dependencies, quick incremental builds of our own code.
set -uo pipefail
cd "$(dirname "$0")/.."
ROOT=$PWD
: "${CARGO_BUILD_JOBS:=2}"
export CARGO_BUILD_JOBS
THREADS="${TEST_THREADS:-3}"
FILTER="${1:-}"
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

step core "$ROOT" cargo test --profile test-fast -- --test-threads="$THREADS"
step plugins "$ROOT" cargo test --profile test-fast --features plugins -- --test-threads="$THREADS"
step app-rust "$ROOT/app/src-tauri" cargo test -- --test-threads="$THREADS"
step app-web "$ROOT/app" npm run verify
echo "all gates passed"

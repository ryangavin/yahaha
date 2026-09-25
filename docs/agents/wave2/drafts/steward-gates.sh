#!/usr/bin/env bash
# Run all integration gates in the steward worktree; log to steward-gates-<sha>.log
W="${STEWARD_WORKTREE:?set STEWARD_WORKTREE to the steward worktree path}"
S="${SCRATCHPAD:?set SCRATCHPAD to your scratch dir}"
cd "$W" || exit 1
sha=$(git rev-parse --short HEAD)
log="$S/steward-gates-$sha.log"
: > "$log"
fail=0
run() { echo "=== $*" >> "$log"; if "$@" >> "$log" 2>&1; then echo "PASS: $*" | tee -a "$log"; else echo "FAIL: $*" | tee -a "$log"; fail=1; fi; }
run cargo build --release
run cargo test --release
run cargo test --release --features plugins
cd "$W/app/src-tauri" && run cargo test
cd "$W/app" && { [ -d node_modules ] || npm install >> "$log" 2>&1; } && run npm run verify
echo "GATES $sha: $([ $fail = 0 ] && echo GREEN || echo RED) log=$log"
exit $fail

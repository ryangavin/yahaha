#!/usr/bin/env bash
# The agent feedback loop: bacon runs headless in this worktree and re-runs its job on every
# source change, writing .bacon-result.json after each run (bacon.toml).
# An agent edits, then reads the result; no cargo of its own, no polling.
#
#   scripts/agent-feedback.sh start                  # bacon's `check` job, in the background
#   scripts/agent-feedback.sh start test -- <filter> # or a job, e.g. targeted tests (test-quick)
#   scripts/agent-feedback.sh result                 # the latest run, once it covers the last edit
#   scripts/agent-feedback.sh stop
#
# `result` waits (at most WAIT seconds, default 60) only while the export is older than the
# newest source edit; if the run still hasn't landed it prints "stale" and exits 2.
# Exit codes: 0 pass, 1 fail, 2 stale or not running.
set -uo pipefail
cd "$(dirname "$0")/.."
PID=.bacon.pid JOB=.bacon.job OUT=.bacon-result.json
WAIT=${WAIT:-60}

running() { [[ -f $PID ]] && kill -0 "$(cat $PID)" 2>/dev/null; }
mtime() { stat -f %m "$1" 2>/dev/null || stat -c %Y "$1"; }
# The newest file bacon watches that is newer than the export, if any.
newer_edit() { find src tests vendor Cargo.toml Cargo.lock bacon.toml -newer "$OUT" -type f -print -quit 2>/dev/null; }

case ${1:-} in
start)
    shift
    if running; then echo "bacon already running (pid $(cat $PID), job: $(cat $JOB))"; exit 0; fi
    rm -f "$OUT"
    nohup bacon --headless "$@" >.bacon.log 2>&1 &
    echo $! >$PID
    echo "${*:-check}" >$JOB
    echo "bacon started (pid $!, job: ${*:-check})"
    ;;
result)
    running || { echo "bacon not running: scripts/agent-feedback.sh start"; exit 2; }
    for ((i = 0; i < WAIT; i++)); do # the one bounded wait: until the run covers the last edit
        [[ -f $OUT && -z $(newer_edit) ]] && break
        sleep 1
    done
    if [[ ! -f $OUT || -n $(newer_edit) ]]; then echo "stale (job: $(cat $JOB); run still going after ${WAIT}s)"; exit 2; fi
    read -r code errors warnings fails < <(jq -r '[.error_code // 0, .stats.errors, .stats.warnings, .stats.test_fails] | @tsv' "$OUT")
    status=PASS
    [[ $code != 0 || $errors != 0 || $fails != 0 ]] && status=FAIL
    echo "$status job=$(cat $JOB) errors=$errors warnings=$warnings test_fails=$fails exit=$code at=$(date -r "$(mtime $OUT)" +%T)"
    # bacon's analysed items: each error, warning and failed test with its location and message.
    jq -r '.lines[] | [.content.strings[].raw] | join("")' "$OUT" | sed "s|$PWD/||" | head -60
    [[ $status == PASS ]]
    ;;
stop)
    running && kill "$(cat $PID)" && echo "bacon stopped" || echo "bacon not running"
    rm -f $PID $JOB
    ;;
*)
    sed -n '2,13p' "$0" | sed 's/^# \{0,1\}//'
    exit 2
    ;;
esac

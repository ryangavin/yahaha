#!/usr/bin/env bash
# Squash-merge a PR, but only when its base branch is integration/*.
# Usage: merge-integration.sh <pr-number> [expected-head-sha]
# Merges into main (or any non-integration base) are refused: those stay the owner's call.
set -euo pipefail

pr="${1:?usage: merge-integration.sh <pr-number> [expected-head-sha]}"
[[ "$pr" =~ ^[0-9]+$ ]] || { echo "PR must be a number" >&2; exit 2; }

base=$(gh pr view "$pr" --json baseRefName -q .baseRefName)
head=$(gh pr view "$pr" --json headRefOid -q .headRefOid)
state=$(gh pr view "$pr" --json state -q .state)

if [[ "$state" != "OPEN" ]]; then
  echo "PR #$pr is $state, not OPEN" >&2; exit 3
fi
if [[ "$base" != integration/* ]]; then
  echo "refusing: PR #$pr targets '$base', only integration/* bases may be merged by agents" >&2
  exit 4
fi
if [[ -n "${2:-}" && "$head" != "$2"* ]]; then
  echo "refusing: PR #$pr head is $head, expected $2 (the branch moved since review)" >&2
  exit 5
fi

gh pr merge "$pr" --squash --match-head-commit "$head"
echo "merged PR #$pr into $base at $head"

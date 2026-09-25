#!/usr/bin/env bash
# Emit a line whenever the set of ready-to-merge PRs (number+head+mergeState) changes.
prev=""
while true; do
  cur=$(gh pr list -R ryangavin/yahaha --base integration/m3-ui --state open --label ready-to-merge --json number,headRefOid,mergeStateStatus --jq '.[] | "#\(.number) \(.headRefOid[0:10]) \(.mergeStateStatus)"' 2>/dev/null | sort | tr '\n' ' ')
  if [ $? -eq 0 ] && [ "$cur" != "$prev" ]; then
    [ -n "$cur" ] && echo "READY-SET: $cur"
    prev="$cur"
  fi
  sleep 120
done

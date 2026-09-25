#!/usr/bin/env bash
# Print open integration PRs; for ready ones, the latest READY sha and whether it matches head.
gh pr list -R ryangavin/yahaha --base develop --state open --limit 50 --json number,title,labels,headRefOid,mergeable,mergeStateStatus,headRefName --jq '.[] | "\(.number)\t\(.mergeable)\t\(.mergeStateStatus)\t[\([.labels[].name]|join(","))]\t\(.headRefOid[0:10])\t\(.headRefName)\t\(.title)"'
for n in $(gh pr list -R ryangavin/yahaha --base develop --state open --label ready-to-merge --json number --jq '.[].number'); do
  ready=$(gh api "repos/ryangavin/yahaha/issues/$n/comments" --paginate --jq '.[] | select(.body|test("READY [0-9a-f]{40}")) | "\(.created_at) \(.body|capture("READY (?<s>[0-9a-f]{40})").s)"' | tail -1)
  head=$(gh pr view -R ryangavin/yahaha "$n" --json headRefOid -q .headRefOid)
  m=NO
  case "$ready" in *"$head") m=yes;; esac
  echo "READY#$n comment=[$ready] head=$head match=$m"
done

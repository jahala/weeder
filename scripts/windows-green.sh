#!/usr/bin/env bash
# Whether the Windows suite passed on one commit, read from GitHub.
#
#   bash scripts/windows-green.sh <commit-sha>
#
# Pull requests no longer run Windows (the owner's ruling of 2026-11-06), so a
# commit can reach master and be tagged before anything has judged it there.
# release.yml runs this before it builds, and refuses to publish a commit whose
# newest windows.yml run is missing, still running, or anything but green: the
# v0.1.0 release shipped four platforms of five because nothing had compiled
# for the fifth.
#
# Exit 0 when the newest windows.yml run on the commit succeeded, 1 when it is
# missing, unfinished or red, and 3 when GitHub could not be asked.
set -euo pipefail

sha="${1:?the commit to check is required}"
command -v gh >/dev/null 2>&1 || { echo "gh is not on PATH, so the Windows run on $sha cannot be read" >&2; exit 3; }

runs="$(gh run list --workflow windows.yml --commit "$sha" --limit 1 --json databaseId,status,conclusion 2>/dev/null)" || {
  echo "gh could not list the windows.yml runs on $sha, so nothing says Windows passed there" >&2
  exit 3
}

read -r id status conclusion < <(printf '%s' "$runs" | python3 -c '
import json, sys
runs = json.load(sys.stdin)
if runs:
    run = runs[0]
    print(run["databaseId"], run["status"], run["conclusion"] or "none")
else:
    print("none none none")
')

if [ "$id" = "none" ]; then
  echo "no windows.yml run exists for ${sha:0:12}: tag a commit Windows has judged on master, or wait for its run" >&2
  exit 1
fi
if [ "$status" != "completed" ]; then
  echo "the windows.yml run $id on ${sha:0:12} is $status: wait for it to finish, then tag again" >&2
  exit 1
fi
if [ "$conclusion" != "success" ]; then
  echo "the windows.yml run $id on ${sha:0:12} is $conclusion: fix the windows issue it opened, then tag a commit that passes" >&2
  exit 1
fi
echo "the windows.yml run $id on ${sha:0:12} passed"

#!/usr/bin/env bash
# Evidence for dogfood-2: no helper worktree under `.claude/worktrees/` outlives
# its merged branch (#133). scripts/worktrees-prune.sh holds the rule; this runs
# it in report mode and fails while anything clean and landed is left behind,
# or anything landed still holds work nobody committed.
#
#   bash scripts/check/worktrees.sh [repository]
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
exec bash "$root/scripts/worktrees-prune.sh" "$@" --report

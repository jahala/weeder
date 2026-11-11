#!/usr/bin/env bash
# Remove the helper worktrees whose work has landed (#133).
#
#   bash scripts/worktrees-prune.sh [repository] [--report]
#
# A helper agent works in a worktree under `.claude/worktrees/`, and each one
# builds a `target/` of its own, three to four gigabytes. Once its branch has
# merged, the worktree is nothing but that weight. This removes every helper
# worktree that is clean and whose HEAD the branch work lands on already holds
# (`origin/master`, or what WEEDER_MERGED_INTO names). It moves the folder to
# the trash and has git forget it; nothing is deleted outright. WEEDER_TRASH_INTO
# names a folder to move it into instead, which is how the tests keep the run's
# leftovers inside their own temporary directory rather than the owner's trash.
#
# It never touches a worktree with uncommitted or untracked work, a branch that
# has not landed, or one a running session holds locked. A merged worktree with
# work left in it is named, and the run exits 1 so a person reads it.
#
# With --report it removes nothing and names what it would, which is what
# scripts/check/worktrees.sh runs: exit 1 while anything merged is left behind.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"

repo="$root"
report=false
for argument in "$@"; do
  case "$argument" in
    --report) report=true ;;
    *) repo="$argument" ;;
  esac
done
base="${WEEDER_MERGED_INTO:-origin/master}"

into="${WEEDER_TRASH_INTO:-}"
if [ -z "$into" ]; then
  command -v trash >/dev/null 2>&1 || { echo "trash is not on PATH; nothing here deletes with rm" >&2; exit 3; }
fi
git -C "$repo" rev-parse --verify --quiet "$base^{commit}" >/dev/null || {
  echo "$base is not a commit in $repo, so nothing can be known to have landed" >&2
  exit 3
}

left=0
removed=0
path="" head="" branch="" lock=""
settle() {
  [ -n "$path" ] || return 0
  case "$path" in
    */.claude/worktrees/*) ;;
    *) return 0 ;;
  esac
  [ -d "$path" ] || return 0
  git -C "$repo" merge-base --is-ancestor "$head" "$base" 2>/dev/null || return 0
  local name="${branch#refs/heads/}"
  if [ -n "$(git -C "$path" status --porcelain 2>/dev/null)" ]; then
    echo "left behind with work in it: $path ($name); its branch has landed, read what is left" >&2
    left=$((left + 1))
    return 0
  fi
  local pid
  pid="$(printf '%s' "$lock" | grep -oE 'pid [0-9]+' | grep -oE '[0-9]+' || true)"
  if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
    echo "in use by a running session (pid $pid), left alone: $path ($name)"
    return 0
  fi
  if $report; then
    echo "left behind: $path ($name) is clean and has landed in $base" >&2
    left=$((left + 1))
    return 0
  fi
  [ -z "$lock" ] || git -C "$repo" worktree unlock "$path"
  if [ -n "$into" ]; then
    mv "$path" "$into/"
  else
    trash "$path"
  fi
  removed=$((removed + 1))
  echo "removed: $path ($name), landed in $base"
}

while IFS= read -r line; do
  case "$line" in
    "worktree "*) settle; path="${line#worktree }" head="" branch="" lock="" ;;
    "HEAD "*) head="${line#HEAD }" ;;
    "branch "*) branch="${line#branch }" ;;
    locked*) lock="${line#locked}"; lock="${lock# }"; [ -n "$lock" ] || lock="locked" ;;
  esac
done < <(git -C "$repo" worktree list --porcelain)
settle

[ "$removed" -eq 0 ] || git -C "$repo" worktree prune
[ "$left" -eq 0 ] || exit 1
$report && echo "no helper worktree outlives its merged branch in $repo"
exit 0

#!/usr/bin/env bash
set -euo pipefail

# sync copies a tree and marks it copied.
sync_tree() {
  rsync -a "$1" "$2" >/dev/null 2>&1 || true
  rm -r "$2/.cache" 2>/dev/null || :
  [ -d "$2" ] && touch "$2/.synced" &>/dev/null ||true
}

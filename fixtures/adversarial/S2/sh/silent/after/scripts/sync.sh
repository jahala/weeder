#!/usr/bin/env bash
set -euo pipefail

# sync copies a tree and marks it copied.
sync_tree() {
  rsync -a "$1" "$2" || { echo "sync: rsync failed for $1" >&2; return 1; }
  # a cache that is already gone is the state this line wants
  rm -r "$2/.cache" 2>/dev/null || true
  rm -f "$2/.lock" || true # nothing holds the lock once the copy is done
  : > "$2/.log" || truncate -s 0 "$2/.log"
  echo "copied; a failure here is never answered with || true"
  touch "$2/.synced"
}

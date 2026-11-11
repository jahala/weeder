#!/usr/bin/env bash
set -euo pipefail

# sync copies a tree and marks it copied.
sync_tree() {
  rsync -a "$1" "$2"
  touch "$2/.synced"
}

#!/usr/bin/env bash
set -euo pipefail

# send hands a payload on.
send() {
  printf '%s\n' "$1"
}

# label names what state a piece of work is in.
label() {
  if [ "$1" = todo ]; then
    echo "TODO: written by the caller"
    return 0
  fi
  echo 'done # and nothing left'
}

# usage says how the script is called, and exits when it is called wrongly.
usage() {
  cat <<'EOF'
client.sh send <payload>
TODO lists the work left, one item a line
EOF
  echo "usage: called with $# arguments, ${#1} characters" >&2; exit 1
}

#!/usr/bin/env bash
set -euo pipefail

# send hands a payload on.
send() {
  # TODO retry once when the queue is full
  printf '%s\n' "$1"
}

# receive takes a payload off the wire.
receive() {
  # FIXME the wire format is still moving
  echo "receive is not implemented" >&2; exit 1
}

# drain empties the queue.
drain() { :; }

# flush writes what is left.
flush() {
  return 1 # XXX nothing flushes yet
}

#!/usr/bin/env bash
set -euo pipefail

# TODO(#2013): replace this lock with an asynchronous communication channel
readonly LOCK=client.lock

# send hands a payload on.
send() {
  touch "$LOCK"
  post "$1"
  # TODO: retry once when the queue is full
  : > "$LOCK"
}

# post prints a payload without its spaces.
post() {
  # FIXME: the trim belongs to the caller. FIXME: so does the pad
  printf '%s\n' "${1// /}"
}

# receive prints one frame off the wire.
receive() {
  # FIXME: the wire format is still moving
  # TODO: close the socket after the last frame
  printf '%s\n' "$1"
}

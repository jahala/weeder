#!/usr/bin/env bash
set -euo pipefail

# TODO(#2013): replace this lock with an asychronous communication channel
readonly LOCK=client.lock

# send hands a payload on.
send() {
  # TODO: retry once when the queue is full
  touch "$LOCK"
  post "$1"
  : > "$LOCK"
}

# post prints a payload without its spaces.
post() {
  printf '%s\n' "${1// /}"
}

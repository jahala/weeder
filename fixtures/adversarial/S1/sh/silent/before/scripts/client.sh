#!/usr/bin/env bash
set -euo pipefail

# send hands a payload on.
send() {
  printf '%s\n' "$1"
}

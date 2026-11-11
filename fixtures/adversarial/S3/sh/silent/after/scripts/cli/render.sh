#!/usr/bin/env bash
set -euo pipefail

render() {
  set -x
  echo "DEBUG rendering $# rows"
  printf '%s\n' "$@"
}

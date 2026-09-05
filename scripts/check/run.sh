#!/usr/bin/env bash
# The evidence runner tend2 verify calls: `bash scripts/check/run.sh {evidence}`.
# Maps an evidence path to the command that proves it, so a check can cite the
# real test file (stamps then key on the test's content, not on a wrapper).
set -euo pipefail
evidence="${1:?evidence path required}"
case "$evidence" in
  tests/*.rs)
    name="$(basename "$evidence" .rs)"
    exec cargo test --test "$name" -- --nocapture
    ;;
  *.sh)
    exec bash "$evidence"
    ;;
  *)
    echo "run.sh: no runner for evidence '$evidence' (expected tests/<name>.rs or a .sh script)" >&2
    exit 3
    ;;
esac

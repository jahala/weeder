#!/usr/bin/env bash
# Whether a pull request changed code, read from the paths it changed (#35).
#
# The paths arrive on stdin, each ended by a NUL, as `git diff -z --name-only`
# writes them, so a path with a newline in it is still one path. The answer is
# one line for $GITHUB_OUTPUT: `code=false` when every path is prose, and
# `code=true` otherwise. An empty list is code, because a list git failed to
# write says nothing about what changed.
#
# Prose is what cannot change the release binary or what tests/speed.rs times:
# markdown anywhere, anything under docs/ or .brand/, the landing page and the
# licence. The check job still runs on prose; only the latency job skips it.

any=false
while IFS= read -r -d '' path; do
  any=true
  case "$path" in
    *.md | docs/* | .brand/* | index.html | LICENSE) ;;
    *)
      echo "code=true"
      exit 0
      ;;
  esac
done

if [ "$any" = true ]; then
  echo "code=false"
else
  echo "code=true"
fi

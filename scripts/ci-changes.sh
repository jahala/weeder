#!/usr/bin/env bash
# Whether a pull request changed code, read from the range it changed (#35).
#
# Usage: ci-changes.sh <base> <head>, run inside the repository. The script asks
# git itself, so what it reads is what git writes: `-z`, so a path with a newline
# in it is still one path, and `--no-renames`, so a file moved out of code is
# listed by the path it left as well as the one it reached (#159). The answer is
# one line for $GITHUB_OUTPUT: `code=false` when every path is prose, and
# `code=true` otherwise. A range git cannot answer for is code, and so is an
# empty one, because neither says what changed.
#
# Prose is what cannot change the release binary or what tests/speed.rs times:
# markdown, anything under docs/ or .brand/, the landing page and the licence.
# Below fixtures/ and tests/ nothing is prose, since the suite reads it there.
# The check job still runs on prose; only the latency job skips it.

base="${1:?usage: ci-changes.sh <base> <head>}"
head="${2:?usage: ci-changes.sh <base> <head>}"

list="$(mktemp)"
trap 'rm -f "$list"' EXIT

if ! git diff -z --name-only --no-renames "$base...$head" >"$list"; then
  echo "code=true"
  exit 0
fi

any=false
while IFS= read -r -d '' path; do
  any=true
  case "$path" in
    fixtures/* | tests/*)
      echo "code=true"
      exit 0
      ;;
    *.md | docs/* | .brand/* | index.html | LICENSE) ;;
    *)
      echo "code=true"
      exit 0
      ;;
  esac
done <"$list"

if [ "$any" = true ]; then
  echo "code=false"
else
  echo "code=true"
fi

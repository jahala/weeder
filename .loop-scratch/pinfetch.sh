#!/usr/bin/env bash
# Probe: can a pinned sha be fetched straight from GitHub, and from a local path?
set -eu
dir="$(mktemp -d)"
cd "$dir"
git -c init.defaultBranch=calibration init --quiet .
echo "--- fetch pinned sha from github"
if GIT_TERMINAL_PROMPT=0 git fetch --quiet --no-tags https://github.com/jahala/copeca.git fc9c5b9e5f34085755c10746f1d46fd46edf2945; then
  echo "sha fetch OK: $(git cat-file -t fc9c5b9e5f34085755c10746f1d46fd46edf2945)"
else
  echo "sha fetch REFUSED"
fi
echo "--- fetch pinned sha from a local path"
dir2="$(mktemp -d)"
cd "$dir2"
git -c init.defaultBranch=calibration init --quiet .
if git fetch --quiet --no-tags /Users/jahala/conductor/workspaces/copeca/cancun fc9c5b9e5f34085755c10746f1d46fd46edf2945; then
  echo "local sha fetch OK: $(git cat-file -t fc9c5b9e5f34085755c10746f1d46fd46edf2945)"
else
  echo "local sha fetch REFUSED"
fi
echo "$dir $dir2"

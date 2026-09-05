#!/usr/bin/env bash
# Evidence for garden-fit: F3, as far as it can be proved on one machine.
#
# The claim is that somebody who clones this repository can build it. What is
# copied into the temp directory is what a clone would carry, every tracked
# file, plus the untracked files git is not ignoring, which is the working tree
# as it will be committed. Nothing else comes along: no target directory, no
# .context worktree, no cargo config from this repo. If the build there needs a
# file that only exists on this machine, it fails here.
#
# What this cannot prove is the network half. weed pins tilth-core by git rev,
# and until that rev is on the remote the build resolves it from the local cargo
# cache, the same cache a clean machine would fill on its first fetch. That is
# the gap between this check and F3, and it closes when the branch is pushed.
#
# The copy is left where it was made. This check never deletes anything; the
# path is printed, and the system temp directory is reclaimed by the operating
# system.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

version="$(grep -E '^version *=' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')"
[ -n "$version" ] || { echo "Cargo.toml names no version" >&2; exit 3; }

clone="$(mktemp -d)"
echo "building a fresh copy in $clone"

# Every file a clone of this repository would hold, and no other. A tracked file
# that has been deleted in the working tree is not copied, because a clone taken
# after that deletion lands would not have it either.
copied=0
while IFS= read -r -d '' file; do
  [ -f "$file" ] || continue
  mkdir -p "$clone/$(dirname "$file")"
  cp "$file" "$clone/$file"
  copied=$((copied + 1))
done < <(
  {
    git ls-files -z
    git ls-files -z --others --exclude-standard
  } | sort -zu
)

[ "$copied" -gt 0 ] || { echo "nothing was copied: git listed no files" >&2; exit 3; }
echo "$copied files copied"

status=0
if ! (cd "$clone" && cargo build --release --locked); then
  echo "a fresh copy of this repository does not build with cargo build --release" >&2
  exit 1
fi

binary="$clone/target/release/weed"
[ -x "$binary" ] || { echo "the build produced no $binary" >&2; exit 1; }

printed="$("$binary" --version)" || { echo "weed --version left with a code" >&2; exit 1; }
case "$printed" in
  *"$version"*) ;;
  *)
    echo "the built binary says '$printed', and Cargo.toml says $version" >&2
    status=1
    ;;
esac

if [ "$status" -eq 0 ]; then
  echo "a fresh copy of $copied files builds with cargo build --release and answers: $printed"
fi
exit "$status"

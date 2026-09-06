#!/usr/bin/env bash
# Evidence for tilth-core.tend2.html c1: the crate weeder pins is green.
# The pinned commit comes from Cargo.toml's `rev` (or garden/tilth-core before
# the pin exists). It is used at .context/tilth-core when that worktree is
# present and at the commit (the conductor's machine), otherwise cloned from
# GitHub into a temp dir keyed by the commit, so the check proves the same
# thing anywhere. Then tilth's CI trio and the crate's acceptance test run there.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
repo="https://github.com/jahala/tilth"
want="$(grep -oE 'tilth-core *= *\{[^}]*rev *= *"[0-9a-f]+"' "$root/Cargo.toml" 2>/dev/null | grep -oE '[0-9a-f]{7,40}' | tail -1 || true)"
[ -n "$want" ] || want="$(git ls-remote "$repo" refs/heads/garden/tilth-core | cut -f1)"
[ -n "$want" ] || { echo "no pinned rev in Cargo.toml and no garden/tilth-core on $repo" >&2; exit 3; }

wt="$root/.context/tilth-core"
if [ -d "$wt" ] && [ "$(git -C "$wt" rev-parse HEAD)" = "$(git -C "$wt" rev-parse "$want^{commit}" 2>/dev/null || echo)" ]; then
  tree="$wt"
else
  tree="${TMPDIR:-/tmp}/weeder-tilth-core-$want"
  if [ ! -f "$tree/Cargo.toml" ]; then
    git clone -q "$repo" "$tree"
    git -C "$tree" checkout -q "$want"
  fi
fi
cd "$tree"
have="$(git rev-parse HEAD)"
[ -f crates/tilth-core/Cargo.toml ] || { echo "crates/tilth-core/Cargo.toml missing at $have" >&2; exit 1; }
grep -q 'tilth-core' Cargo.toml || { echo "root Cargo.toml does not depend on tilth-core" >&2; exit 1; }
cargo fmt --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo test -p tilth-core --test api
echo "tilth-core: $have green in $tree"

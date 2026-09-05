#!/usr/bin/env bash
# Evidence for tilth-core.tend2.html c1: the crate weed pins is green.
# The worktree at .context/tilth-core checks out the commit weed's Cargo.toml
# pins (or the branch almaty lands on, before the pin exists); tilth's CI trio
# and the crate's acceptance test run there.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
wt="$root/.context/tilth-core"
[ -d "$wt" ] || { echo "tilth worktree missing at $wt (git -C ~/CascadeProjects/tilth worktree add $wt garden/tilth-core)" >&2; exit 3; }
want="$(grep -oE 'tilth-core *= *\{[^}]*rev *= *"[0-9a-f]+"' "$root/Cargo.toml" 2>/dev/null | grep -oE '[0-9a-f]{7,40}' | tail -1 || true)"
[ -n "$want" ] || want="$(git -C "$wt" rev-parse garden/tilth-core)"
have="$(git -C "$wt" rev-parse HEAD)"
case "$have" in
  "$want"*) ;;
  *) echo "worktree is at $have, weed pins $want; check out the pinned commit" >&2; exit 1 ;;
esac
cd "$wt"
[ -f crates/tilth-core/Cargo.toml ] || { echo "crates/tilth-core/Cargo.toml missing at $have" >&2; exit 1; }
grep -q 'tilth-core' Cargo.toml || { echo "root Cargo.toml does not depend on tilth-core" >&2; exit 1; }
cargo fmt --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo test -p tilth-core --test api
echo "tilth-core: $(git branch --show-current) at $(git rev-parse --short HEAD) green"

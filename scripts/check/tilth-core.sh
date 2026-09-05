#!/usr/bin/env bash
# Evidence for tilth-core.tend2.html c1: the extraction proposal branch is green.
# Runs tilth's CI trio in the tilth worktree and checks the crate's surface exists.
set -euo pipefail
wt="$(cd "$(dirname "$0")/../.." && pwd)/.context/tilth-core"
[ -d "$wt" ] || { echo "tilth worktree missing at $wt (git -C ~/CascadeProjects/tilth worktree add -b tilth-core $wt main)" >&2; exit 3; }
cd "$wt"
branch="$(git branch --show-current)"
[ "$branch" = "tilth-core" ] || { echo "worktree is on '$branch', expected tilth-core" >&2; exit 1; }
[ -f crates/tilth-core/Cargo.toml ] || { echo "crates/tilth-core/Cargo.toml missing: the extraction has not landed" >&2; exit 1; }
grep -q 'tilth-core' Cargo.toml || { echo "root Cargo.toml does not depend on tilth-core" >&2; exit 1; }
cargo fmt --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
cargo test -p tilth-core --test api
if diff -rqs src crates/tilth-core/src 2>/dev/null | grep -q identical; then
  echo "a file exists in both crates (copied, not moved)" >&2; exit 1
fi
echo "tilth-core: branch $branch green at $(git rev-parse --short HEAD)"

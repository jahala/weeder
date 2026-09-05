#!/usr/bin/env bash
# Evidence for tilth-core.tend2.html c2: weed reads code through the crate it
# pins, and through nothing else. The manifest names the git dependency and the
# commit; the resolved graph agrees, which is what catches a patch or a path
# override wherever it was written, a `[patch]` table, a `[replace]` entry, or
# a .cargo/config.toml on this machine. Then the build runs, because a pin that
# does not compile is not a dependency.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

repo="https://github.com/jahala/tilth"
rev="7f38db58696e16c2df2be71da985f47097f34920"
status=0

command -v jq >/dev/null 2>&1 || { echo "jq is not on PATH" >&2; exit 3; }

# The manifest: one dependency line, the repository, the commit, and no path.
manifest="$(grep -E '^tilth-core *=' Cargo.toml || true)"
[ -n "$manifest" ] || { echo "Cargo.toml declares no tilth-core dependency" >&2; exit 1; }
case "$manifest" in
  *"git = \"$repo\""*) ;;
  *) echo "the tilth-core dependency does not name $repo: $manifest" >&2; status=1 ;;
esac
case "$manifest" in
  *"rev = \"$rev\""*) ;;
  *) echo "the tilth-core dependency is not pinned to $rev: $manifest" >&2; status=1 ;;
esac
case "$manifest" in
  *"path ="*) echo "the tilth-core dependency carries a local path: $manifest" >&2; status=1 ;;
esac

# No local patch: not in the manifest, and not in a cargo config beside it.
if grep -qE '^\[(patch|replace)' Cargo.toml; then
  echo "Cargo.toml carries a [patch] or [replace] table; the pin would not be what builds" >&2
  status=1
fi
for config in .cargo/config.toml .cargo/config; do
  [ -f "$config" ] || continue
  echo "$config exists; a cargo config beside the manifest can redirect the pin" >&2
  status=1
done

# The resolved graph: one tilth-core, from that repository, at that commit.
sources="$(cargo metadata --format-version 1 --locked \
  | jq -r '.packages[] | select(.name == "tilth-core") | .source // "local"')"
expected="git+$repo?rev=$rev#$rev"
if [ "$sources" != "$expected" ]; then
  echo "cargo resolves tilth-core to '$sources', not '$expected'" >&2
  status=1
fi

# Only one crate in the graph parses code, and it is that one.
parsers="$(cargo metadata --format-version 1 --locked \
  | jq -r '[.packages[] | select(.name == "tilth-core")] | length')"
[ "$parsers" = "1" ] || { echo "the graph holds $parsers copies of tilth-core" >&2; status=1; }

cargo build || status=1

[ "$status" -eq 0 ] || exit "$status"
echo "tilth-core: pinned at $rev from $repo, no patch, and weed builds on it"

#!/usr/bin/env bash
# Fetch the umbrella brand the way `.brand/products/weeder/petalsrc.example` names
# it, and print where it landed.
#
# The page checks are held to the umbrella's own gate rather than to a copy of
# it kept here, so they need the umbrella's `.brand` and `petals/scripts/` as
# the umbrella publishes them. This script is the one place that knows how to
# get them: it reads the source, the version and the product out of
# petalsrc.example, resolves the version to a commit at the source, fetches that
# commit into a cache keyed by it, and refuses a checkout whose brand does not
# declare the version petalsrc asked for.
#
# usage: bash scripts/umbrella-brand.sh
# prints one line: <checkout>\t<sha>\t<version>\t<product>
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
petalsrc="$root/.brand/products/weeder/petalsrc.example"

command -v git >/dev/null 2>&1 || { echo "git is not on PATH" >&2; exit 3; }
[ -f "$petalsrc" ] || {
  echo "$petalsrc is missing: nothing names the umbrella to fetch" >&2
  exit 3
}

field() {
  sed -n "s/^[[:space:]]*$1:[[:space:]]*\(.*\)\$/\1/p" "$petalsrc" | head -1
}
source_url="$(field source)"
version="$(field version)"
product="$(field product)"
for name in source_url version product; do
  eval "value=\$$name"
  [ -n "$value" ] || {
    echo "petalsrc.example names no ${name%_url}: the fetch has nothing to go on" >&2
    exit 3
  }
done

# The version is a name at the source when the source carries it as a tag or a
# branch, and the brand's own version field is what settles it either way.
# An annotated tag answers twice, once as the tag object and once peeled to the
# commit it names, and ls-remote lists them by name, tag object first. The
# peeled line is the commit, so it is taken first; a lightweight tag or a branch
# answers once and is taken as it is.
answers="$(git ls-remote "$source_url" \
  "refs/tags/$version^{}" "refs/tags/$version" "refs/heads/$version" 2>/dev/null || true)"
sha="$(printf '%s\n' "$answers" | awk -v peeled="refs/tags/$version^{}" '$2 == peeled {print $1; exit}')"
[ -n "$sha" ] || sha="$(printf '%s\n' "$answers" | head -1 | cut -f1)"
named=1
if [ -z "$sha" ]; then
  named=0
  sha="$(git ls-remote "$source_url" HEAD 2>/dev/null | head -1 | cut -f1 || true)"
fi
[ -n "$sha" ] || {
  echo "$source_url answered no ref for $version and no HEAD: the umbrella cannot be read" >&2
  exit 3
}

checkout="${WEEDER_UMBRELLA_CACHE:-${TMPDIR:-/tmp}}/weeder-umbrella-$sha"
if [ ! -f "$checkout/.brand/identity.md" ]; then
  if [ "$named" -eq 1 ]; then
    git clone -q --depth 1 --branch "$version" "$source_url" "$checkout"
  else
    git clone -q --depth 1 "$source_url" "$checkout"
  fi
fi
have="$(git -C "$checkout" rev-parse HEAD)"
[ "$have" = "$sha" ] || {
  echo "$checkout is at $have, not the $sha the source answered for $version" >&2
  exit 3
}

declared="$(sed -n 's/^|[[:space:]]*Brand Version[[:space:]]*|[[:space:]]*\([^|]*[^| ]\)[[:space:]]*|.*$/\1/p' \
  "$checkout/.brand/identity.md" | head -1)"
[ "$declared" = "$version" ] || {
  echo "the umbrella at $sha declares brand version '$declared', and petalsrc asks for '$version'" >&2
  exit 3
}
[ -f "$checkout/petals/scripts/check.sh" ] || {
  echo "$checkout carries no petals/scripts/check.sh: the gate is not in this fetch" >&2
  exit 3
}

printf '%s\t%s\t%s\t%s\n' "$checkout" "$sha" "$version" "$product"

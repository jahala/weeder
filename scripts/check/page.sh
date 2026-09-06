#!/usr/bin/env bash
# Evidence for page.tend2.html c1: index.html at the repository root passes the
# umbrella's own brand gate with nothing to fix and nothing to review.
#
# The gate is the umbrella's, not a copy of it kept here. `scripts/umbrella-brand.sh`
# fetches the umbrella at the version `.brand/products/weeder/petalsrc.example`
# names, and this script stages that fetch the way a petals working copy is
# laid out: the umbrella's .brand, with weeder's canonical product layer written
# over the umbrella's staging copy of it, and the page beside them. Then it runs
# `petals/scripts/check.sh` from the same fetch.
#
# check.sh fails on errors alone, so warnings would pass its exit code. The
# claim is 0 and 0, so this script reads the counts it prints and holds it to
# both.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

page="$root/index.html"
[ -f "$page" ] || {
  echo "index.html is missing from the repository root: there is no page to judge" >&2
  exit 1
}

command -v shasum >/dev/null 2>&1 || {
  echo "shasum is not on PATH, so the staging directory cannot be keyed by its inputs" >&2
  exit 3
}

IFS=$'\t' read -r checkout sha version product < <(bash "$root/scripts/umbrella-brand.sh")
layer="$root/.brand/products/$product"
[ -d "$layer" ] || {
  echo "$layer is missing: petalsrc names product '$product' and the layer is not there" >&2
  exit 1
}

# The staging directory is keyed by everything that decides the verdict, so a
# stale one is never the one read and nothing has to be deleted to be correct.
key="$( { echo "$sha"
          find "$layer" -type f -print0 | sort -z | xargs -0 shasum
          shasum "$page"
          shasum "$checkout/petals/scripts/check.sh"; } | shasum | cut -d' ' -f1 )"
stage="${WEEDER_UMBRELLA_CACHE:-${TMPDIR:-/tmp}}/weeder-page-$key"
if [ ! -f "$stage/.staged" ]; then
  mkdir -p "$stage/.brand" "$stage/petals"
  cp -R "$checkout/.brand/." "$stage/.brand/"
  cp -R "$checkout/petals/." "$stage/petals/"
  mkdir -p "$stage/.brand/products/$product"
  cp -R "$layer/." "$stage/.brand/products/$product/"
  cp "$page" "$stage/index.html"
  : > "$stage/.staged"
fi

report="$stage/check.out"
set +e
( cd "$stage" && bash petals/scripts/check.sh index.html --brand .brand --product "$product" ) > "$report" 2>&1
verdict=$?
set -e

warnings="$(sed -n 's/.*Result: PASS (\([0-9][0-9]*\) warning.*/\1/p' "$report" | head -1)"
if [ "$verdict" -ne 0 ] || [ -z "$warnings" ] || [ "$warnings" -ne 0 ]; then
  cat "$report" >&2
  echo "" >&2
  echo "index.html does not pass the umbrella's gate at $version ($sha) with 0 errors and 0 warnings" >&2
  exit 1
fi

sed -n '/--- petals check/,$p' "$report"
echo "index.html: 0 errors, 0 warnings against the umbrella's own petals check at $version ($sha), product layer $product"

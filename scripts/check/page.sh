#!/usr/bin/env bash
# Evidence for page.tend2.html c1: index.html at the repository root passes the
# umbrella's own brand gate with nothing to fix, and with nothing to review that
# the umbrella's own reference bed page does not also raise.
#
# The gate is the umbrella's, not a copy of it kept here. `scripts/umbrella-brand.sh`
# fetches the umbrella at the version `.brand/products/weeder/petalsrc.example`
# names, and this script stages that fetch the way a petals working copy is
# laid out: the umbrella's .brand, with weeder's canonical product layer written
# over the umbrella's staging copy of it, and the page beside them. Then it runs
# `petals/scripts/check.sh` from the same fetch.
#
# check.sh fails on errors alone, so warnings would pass its exit code. The page
# is built from `.brand/reference/product.html`, and the markup and wording it
# takes from there draw warnings the umbrella has not cleared from the reference
# itself. So the same check is run on the reference, from the same fetch, and
# every warning the page draws has to be one the reference draws too, read by
# its message with the line number taken out: a warning weeder's own swaps
# brought in fails the check, and so does one more of a kind than the reference
# has.
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

# One petals check on one file, from the staging directory, into a report.
judge() {
  ( cd "$stage" && bash petals/scripts/check.sh "$1" --brand .brand --product "$product" ) > "$2" 2>&1
}

# The warnings a report holds, each without its line number, sorted so that two
# lists compare as multisets.
warnings_of() {
  sed -n 's/^WARNING \(\[[a-z]*\]\) line [0-9][0-9]*: /\1 /p' "$1" | LC_ALL=C sort
}

report="$stage/check.out"
reference="$stage/reference.out"
set +e
judge index.html "$report"
verdict=$?
judge .brand/reference/product.html "$reference"
reference_verdict=$?
set -e

if [ "$verdict" -ne 0 ] || ! grep -q 'Result: PASS' "$report"; then
  cat "$report" >&2
  echo "" >&2
  echo "index.html does not pass the umbrella's gate at $version ($sha): it has errors to fix" >&2
  exit 1
fi
if [ "$reference_verdict" -ne 0 ] || ! grep -q 'Result: PASS' "$reference"; then
  cat "$reference" >&2
  echo "" >&2
  echo "the umbrella's own reference page fails its gate at $version ($sha), so it is no measure of the page" >&2
  exit 3
fi

extra="$(LC_ALL=C comm -23 <(warnings_of "$report") <(warnings_of "$reference"))"
if [ -n "$extra" ]; then
  cat "$report" >&2
  echo "" >&2
  echo "index.html draws warnings the umbrella's reference page at $version ($sha) does not:" >&2
  echo "$extra" >&2
  exit 1
fi

count="$(warnings_of "$report" | grep -c . || true)"
sed -n '/--- petals check/,$p' "$report"
echo "index.html: 0 errors, and its $count warnings are all ones the umbrella's reference page draws, against the umbrella's own petals check at $version ($sha), product layer $product"

#!/usr/bin/env bash
# Evidence for calibration c5: the corpus names each repository by a fetchable
# source and a full tip sha and no path on this machine, the measurements fetch
# each pin into a scratch and judge the window that ends at it, and the evidence
# reads that scratch rather than a checkout.
#
# The claim under all of it is that a commit made on a source after the pin was
# written cannot move a number in the report. That is proved here on a history
# built for the purpose: two commits, a corpus pinned at the second, a run, then
# a third commit on the source and the same run again, and the two reports have
# to be the same bytes. The same history with the pin moved forward has to give
# a different report, or the pin is doing nothing and the first half proves
# nothing either.
#
# Then the shipped corpus itself: every entry is read, every source is fetched at
# its pin into a scratch, and the window git counts there is the window the
# report claims. Nothing here opens a checkout, because the corpus names none.
#
# The suites in xtask/tests/pinned.rs run last, for the cases a shell is a poor
# place to write: the suppression rate taken at the pin, an entry nobody selected
# never being reached for, and the scratch being taken away again.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

corpus="docs/calibration/corpus.toml"
report="docs/calibration-2026-09.md"
window=200

command -v python3 >/dev/null 2>&1 || {
  echo "python3 is not on PATH, so the corpus cannot be read. The check refuses to pass on an unread file." >&2
  exit 3
}
command -v git >/dev/null 2>&1 || { echo "git is not on PATH" >&2; exit 3; }
[ -f "$corpus" ] || { echo "$corpus is missing: calibration names no repositories to judge" >&2; exit 1; }

scratch="$(mktemp -d)"
# Scratch goes to the bin, never to rm; where there is no trash command the
# temp directory keeps it and the system clears it.
trap 'command -v trash >/dev/null 2>&1 && trash "$scratch"' EXIT
status=0

# The file itself: a fetchable source and a full sha per entry, and no path that
# only exists here.
python3 - "$corpus" <<'PY' || status=1
import re
import sys

text = open(sys.argv[1], encoding="utf-8").read()
complaints = []
entries = text.split("[[repo]]")[1:]
if not entries:
    complaints.append("the corpus names no repository at all")

for block in entries:
    fields = {}
    for key in ("name", "source", "tip"):
        found = re.search(rf'^\s*{key}\s*=\s*"([^"]*)"', block, re.M)
        if found:
            fields[key] = found.group(1)
    name = fields.get("name", "an entry with no name")
    if "source" not in fields:
        complaints.append(f"{name} names no source, so there is nothing to fetch it from")
    else:
        source = fields["source"]
        if source.startswith(("/", "~", ".")) or re.match(r"^[A-Za-z]:[\\/]", source):
            complaints.append(
                f"{name} is named as {source}, which is a path on one machine. a source is fetchable "
                "from any of them, so the report reproduces off this laptop."
            )
        elif "://" not in source and "@" not in source:
            complaints.append(f"{name}: {source} does not read as something git can fetch from")
    if "tip" not in fields:
        complaints.append(f"{name} carries no tip, so its window ends wherever a branch happens to be")
    elif not re.fullmatch(r"[0-9a-f]{40}", fields["tip"]):
        complaints.append(
            f"{name} is pinned at {fields['tip']}, which is not a full forty-character sha. an "
            "abbreviation can come to mean a second commit."
        )

# A `path` key is what this corpus used to carry, and it is what the report was
# reproducible on one machine only for.
for number, line in enumerate(text.splitlines(), start=1):
    if re.match(r'^\s*path\s*=', line):
        complaints.append(f"line {number} still names a path: {line.strip()!r}")

for complaint in complaints:
    print(complaint, file=sys.stderr)
if complaints:
    raise SystemExit(1)
print(f"{len(entries)} repositories, each a fetchable source and a full sha, and no path among them")
PY

# The claim, on a history built for the purpose.
source="$scratch/source"
mkdir -p "$source"
git -C "$source" init --quiet --initial-branch=main
git -C "$source" config user.name "weed measurements"
git -C "$source" config user.email "measurements@weed.invalid"
commit() {
  git -C "$source" add -A
  GIT_AUTHOR_DATE="2026-09-06T09:00:00+00:00" GIT_COMMITTER_DATE="2026-09-06T09:00:00+00:00" \
    git -C "$source" commit --quiet -m "$1"
}

mkdir -p "$source/src" "$source/tests"
printf 'pub fn one() -> u32 {\n    1\n}\n' > "$source/src/lib.rs"
printf '#[test]\nfn one() {\n    assert_eq!(1, 1);\n}\n\n#[test]\nfn two() {\n    assert_eq!(2, 2);\n}\n' \
  > "$source/tests/unit.rs"
commit "the repository begins"
printf '#[test]\nfn one() {\n    assert_eq!(1, 1);\n}\n' > "$source/tests/unit.rs"
commit "one case fewer"
pin="$(git -C "$source" rev-parse main)"

bench="$scratch/bench"
mkdir -p "$bench"
: > "$bench/judgements.toml"
: > "$bench/first-run.toml"
pinned_corpus() {
  printf '[[repo]]\nname = "probe"\nsource = "%s"\ntip = "%s"\n' "$source" "$1" > "$bench/corpus.toml"
}
judge() {
  cargo run -q -p xtask -- calibrate \
    --corpus "$bench/corpus.toml" \
    --out "$1" \
    --judgements "$bench/judgements.toml" \
    --first-run "$bench/first-run.toml" \
    --audit "$bench/audit.md" >/dev/null
}

pinned_corpus "$pin"
judge "$bench/before.md"

printf '' > "$source/tests/unit.rs"
commit "the last case goes, after the pin was written"
judge "$bench/after.md"

if ! diff -q "$bench/before.md" "$bench/after.md" >/dev/null; then
  echo "a commit made on the source after the pin changed the report, so the corpus is not pinned:" >&2
  diff "$bench/before.md" "$bench/after.md" >&2 || true
  status=1
fi
if grep -q "after the pin was written" "$bench/after.md"; then
  echo "the commit made past the pin is inside the window, so the pin does not end it" >&2
  status=1
fi

# And the other way, or the sameness above says nothing.
pinned_corpus "$(git -C "$source" rev-parse main)"
judge "$bench/moved.md"
if diff -q "$bench/before.md" "$bench/moved.md" >/dev/null; then
  echo "moving the pin to the new tip changed nothing, so the window is not read off the pin at all" >&2
  status=1
fi
if ! grep -q "after the pin was written" "$bench/moved.md"; then
  echo "the pin was moved to the commit and the report still does not judge it" >&2
  status=1
fi

# The shipped corpus, fetched at its pins, and the window the report claims.
bash "$root/scripts/corpus-scratch.sh" "$scratch/corpus-scratch" > "$scratch/fetched"
[ -s "$scratch/fetched" ] || { echo "$corpus fetched nothing" >&2; exit 1; }

while IFS=$'\t' read -r name path tip _source; do
  [ -d "$path/.git" ] || {
    echo "$name was not fetched into a scratch, so the evidence has no history to read" >&2
    status=1
    continue
  }
  head="$(git -C "$path" rev-parse refs/heads/calibration)"
  if [ "$head" != "$tip" ]; then
    echo "$name: the scratch is at $head and the corpus pins $tip" >&2
    status=1
  fi
  judged=0
  for sha in $(git -C "$path" rev-list --no-merges -n "$window" refs/heads/calibration); do
    if git -C "$path" rev-parse --verify --quiet "$sha^" >/dev/null; then
      judged=$((judged + 1))
    fi
  done
  if ! grep -q "^## $name, $judged commits judged," "$report"; then
    echo "$name: git counts $judged commits in the window ending at $tip, and the report says otherwise" >&2
    status=1
  fi
  if ! grep -q "The window ends at \`$tip\`" "$report"; then
    echo "$name: the report does not name the commit its window ends at, $tip" >&2
    status=1
  fi
done < "$scratch/fetched"

# The cases a shell is a poor place to write.
cargo test --package xtask --test pinned || status=1

[ "$status" -eq 0 ] || exit "$status"
echo "the corpus pins every repository by source and sha, the measurements judge the window ending at the pin, and a commit made afterwards leaves the report byte for byte the same"

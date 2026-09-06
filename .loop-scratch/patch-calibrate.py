p = "scripts/check/calibrate.sh"
s = open(p).read()

old = """# The window is not taken on trust. This script asks git itself, in each source
# repository, how many commits of the default branch have a parent and are not
# merges, and refuses a report whose count is anything else. The repositories are
# fingerprinted before the run and again after it, refs, HEAD and working tree,
# so "untouched" is measured rather than asserted. And the report the run writes
# is compared byte for byte with the one in the tree: a calibration that no
# longer describes the history it names is not a calibration."""
new = """# The window is not taken on trust. This script fetches each pinned commit into a
# scratch of its own, asks git there how many commits reaching it have a parent
# and are not merges, and refuses a report whose count is anything else. It reads
# no checkout on this machine, because the corpus names none: every source is
# fetched over `upload-pack`, which hands objects out and takes none in, and that
# is what untouched means here. And the report the run writes is compared byte
# for byte with the one in the tree: a calibration that no longer describes the
# history it names is not a calibration."""
assert old in s
s = s.replace(old, new)

start = s.index("# The corpus, as name and path, read out of the file the run reads.")
end = s.index('[ -f "$report" ] ||')
new_block = """# The corpus, fetched at its pins into a scratch this script owns. Nothing here
# reads a checkout: the corpus names none, and the evidence has to stand on the
# same history the measurement stood on.
bash "$root/scripts/corpus-scratch.sh" "$scratch/corpus-scratch" > "$scratch/corpus"
[ -s "$scratch/corpus" ] || { echo "$corpus names no repositories" >&2; exit 1; }

# The window each pin holds, counted by git in the scratch.
: > "$scratch/expected"
while IFS=$'\\t' read -r name path tip source; do
  judged=0
  for sha in $(git -C "$path" rev-list --no-merges -n "$window" refs/heads/calibration); do
    if git -C "$path" rev-parse --verify --quiet "$sha^" >/dev/null; then
      judged=$((judged + 1))
    fi
  done
  printf '%s\\t%s\\t%s\\t%s\\n' "$name" "$tip" "$judged" "$source" >> "$scratch/expected"
done < "$scratch/corpus"

# The run, writing where nothing else can have written.
fresh="$scratch/calibration.md"
if ! cargo xtask calibrate --out "$fresh"; then
  echo "cargo xtask calibrate failed" >&2
  exit 1
fi

"""
s = s[:start] + new_block + s[end:]

old = """# One section per repository, with the count git itself gives.
for name, reference, judged in expected:"""
new = """# One section per repository, with the count git itself gives at the pin.
for name, tip, judged, source in expected:"""
assert old in s
s = s.replace(old, new)

old = """    if reference not in report:
        complaints.append(f"{name}: the report does not name the ref it judged, {reference}")"""
new = """    if f"The window ends at `{tip}`, fetched from `{source}`" not in report:
        complaints.append(f"{name}: the report does not name the commit it judged to, {tip}")"""
assert old in s
s = s.replace(old, new)

old = """for heading in ("## How this was measured", "## The rules that ran", "## Totals",
                "## Precision and the allowance rate", "## The rules that blocked"):"""
new = """for heading in ("## How this was measured", "## The rules that ran", "## Totals",
                "## What the rules moved since the first run",
                "## Precision and the allowance rate", "## The rules that blocked"):"""
assert old in s
s = s.replace(old, new)

old = 'echo "cargo xtask calibrate: the window is git\'s own, the report is the shape the loop describes with the verdict first, and every repository it read is as it was found"'
new = 'echo "cargo xtask calibrate: the window is git\'s own at each pin, the report is the shape the loop describes with the verdict first, and every source it read was read over upload-pack and never written to"'
assert old in s
s = s.replace(old, new)

open(p, "w").write(s)
print("ok")

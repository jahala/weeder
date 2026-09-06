p = "scripts/check/suppression-rate.sh"
s = open(p).read()

old = """# Every number the measurement gives is recomputed here from git directly: the
# commit that first brought a guard bundle into the tree, the commits from there
# to the tip, the trailers on them, and the trailers written before. The marker
# the install is found by is read out of weed's own source, so the script and the
# binary cannot drift apart into agreeing about the wrong string."""
new = """# Every number the measurement gives is recomputed here from git directly, in a
# scratch fetched at the same pins the corpus names rather than in any checkout
# on this machine: the commit that first brought a guard bundle into the tree,
# the commits from there to the pin, the trailers on them, and the trailers
# written before. The marker the install is found by is read out of weed's own
# source, so the script and the binary cannot drift apart into agreeing about the
# wrong string."""
assert old in s
s = s.replace(old, new)

start = s.index("python3 - \"$corpus\" > \"$scratch/corpus\" <<'PY'")
end = s.index('if ! cargo xtask suppressions --format json')
new_block = """# The corpus, fetched at its pins into a scratch this script owns.
bash "$root/scripts/corpus-scratch.sh" "$scratch/corpus-scratch" > "$scratch/corpus"
[ -s "$scratch/corpus" ] || { echo "$corpus names no repositories" >&2; exit 1; }

# git's own answer, repository by repository, taken at the pin.
: > "$scratch/expected"
while IFS=$'\\t' read -r name path tip source; do
  history="$(git -C "$path" log --reverse --format='%H' refs/heads/calibration)"
  commits="$(printf '%s\\n' "$history" | grep -c . || true)"
  install="$(git -C "$path" log --reverse --format='%H' -S"$marker" refs/heads/calibration | head -1 || true)"

  since=0
  trailers_since=0
  trailers_before=0
  seen=0
  for sha in $history; do
    [ -n "$install" ] && [ "$sha" = "$install" ] && seen=1
    allowances="$(git -C "$path" log -1 --format=%B "$sha" | grep -c '^[[:space:]]*Weed-allow:' || true)"
    if [ "$seen" = "1" ]; then
      since=$((since + 1))
      trailers_since=$((trailers_since + allowances))
    else
      trailers_before=$((trailers_before + allowances))
    fi
  done
  printf '%s\\t%s\\t%s\\t%s\\t%s\\t%s\\t%s\\n' \\
    "$name" "$commits" "$install" "$since" "$trailers_since" "$trailers_before" "$tip" \\
    >> "$scratch/expected"
done < "$scratch/corpus"

"""
s = s[:start] + new_block + s[end:]

old = """for line in open(sys.argv[2], encoding="utf-8").read().splitlines():
    name, commits, install, since, trailers_since, trailers_before = line.split("\\t")
    expected.append(
        {
            "repo": name,
            "commits": int(commits),
            "install": install or None,
            "commits_since_install": int(since),
            "trailers_since_install": int(trailers_since),
            "trailers_before_install": int(trailers_before),
        }
    )"""
new = """for line in open(sys.argv[2], encoding="utf-8").read().splitlines():
    name, commits, install, since, trailers_since, trailers_before, tip = line.split("\\t")
    expected.append(
        {
            "repo": name,
            "commits": int(commits),
            "install": install or None,
            "commits_since_install": int(since),
            "trailers_since_install": int(trailers_since),
            "trailers_before_install": int(trailers_before),
            "tip": tip,
        }
    )"""
assert old in s
s = s.replace(old, new)

old = """    installed = (got.get("installed") or {}).get("sha")"""
new = """    if got.get("tip") != want["tip"]:
        complaints.append(
            f"{want['repo']}: the measurement was taken at {got.get('tip')} and the corpus pins {want['tip']}"
        )
    installed = (got.get("installed") or {}).get("sha")"""
assert old in s
s = s.replace(old, new)

old = 'print(f"{len(expected)} repositories, every rate and every install day the same as git\'s own")'
new = 'print(f"{len(expected)} repositories, every rate and every install day the same as git\'s own at the pin")'
assert old in s
s = s.replace(old, new)

old = 'echo "cargo xtask suppressions: the rate is git\'s own count from the day guard is installed, zero before, and the calibration file carries it beside precision"'
new = 'echo "cargo xtask suppressions: the rate is git\'s own count at the pin from the day guard is installed, zero before, and the calibration file carries it beside precision"'
assert old in s
s = s.replace(old, new)

open(p, "w").write(s)
print("ok")

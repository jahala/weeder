#!/usr/bin/env bash
# Evidence for recall.tend2.html c3: `cargo xtask mutate` walks the window that
# ends at each entry's pinned tip in docs/calibration/corpus.toml, the corpus
# calibration judges, so two runs over one corpus plant the same cases and write
# the same recall section byte for byte, and a commit added to a source after
# the pin changes nothing.
#
# The claim is about what the campaign reads, so it is proved where a source can
# be pushed to: a history built here, a corpus pinned at its tip, a run, then a
# commit on that source and the same run again, and the two recall sections have
# to be the same bytes. The same history with the pin moved forward has to give
# a different section and plant a case in the new commit, or the pin is doing
# nothing and the first half proves nothing either. Each run is given a cache of
# its own, so sameness is the pin's doing rather than a clone that never went
# back to the source.
#
# Then the corpus weed ships: both files fetched at their pins, the campaign run
# over them twice into files of its own, the two sections compared byte for
# byte, and every commit a case was planted in held to the window git counts
# from the pin. The report is written to a scratch here — the numbers in
# docs/calibration-2026-09.md are the full campaign's, and a check does not
# write over a measurement.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

command -v python3 >/dev/null 2>&1 || {
  echo "python3 is not on PATH, so the runs cannot be read back. The check refuses to pass on an unread report." >&2
  exit 3
}
command -v cargo >/dev/null 2>&1 || { echo "cargo is not on PATH, so the campaign cannot be run." >&2; exit 3; }
command -v git >/dev/null 2>&1 || { echo "git is not on PATH" >&2; exit 3; }

scratch="$(mktemp -d)"
# Scratch goes to the bin, never to rm; where there is no trash command the
# temp directory keeps it and the system clears it.
trap 'command -v trash >/dev/null 2>&1 && trash "$scratch"' EXIT
status=0

cases="${WEED_PINNED_CASES:-3}"
commits="${WEED_PINNED_COMMITS:-25}"

# The recall section of a report, and nothing else of it.
section() {
  python3 - "$1" <<'PY'
import sys

BEGIN, END = "<!-- recall:begin -->", "<!-- recall:end -->"
text = open(sys.argv[1], encoding="utf-8").read()
first, last = text.find(BEGIN), text.find(END)
if first < 0 or last < first:
    print(f"{sys.argv[1]} carries no recall section", file=sys.stderr)
    raise SystemExit(1)
sys.stdout.write(text[first : last + len(END)])
PY
}

# ---------------------------------------------------------------- the bench --
# A history built for the purpose: a small TypeScript project with tests, a
# workflow, a manifest and an ignore file, so the campaign finds sites for a
# spread of rules rather than one.
source="$scratch/source"
mkdir -p "$source/src" "$source/test" "$source/.github/workflows"
git -C "$source" init --quiet --initial-branch=main
git -C "$source" config user.name "weed measurements"
git -C "$source" config user.email "measurements@weed.invalid"
commit() {
  git -C "$source" add -A
  GIT_AUTHOR_DATE="2026-09-06T09:00:00+00:00" GIT_COMMITTER_DATE="2026-09-06T09:00:00+00:00" \
    git -C "$source" commit --quiet -m "$1"
}

cat > "$source/package.json" <<'JSON'
{
  "name": "probe",
  "version": "1.4.0",
  "scripts": {
    "test": "vitest run"
  },
  "dependencies": {
    "left-pad": "1.3.0"
  }
}
JSON
cat > "$source/.gitignore" <<'TXT'
node_modules
coverage
TXT
cat > "$source/.github/workflows/ci.yml" <<'YAML'
name: ci
on:
  push:
    branches: [main]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: npm ci
      - run: npm test
YAML
cat > "$source/src/gate.ts" <<'TS'
export interface Verdict {
  allowed: boolean;
  reason: string;
}

export function decide(count: number, limit: number): Verdict {
  if (count > limit) {
    return { allowed: false, reason: "over the limit" };
  }
  return { allowed: true, reason: "under the limit" };
}

export function explain(verdict: Verdict): string {
  if (verdict.allowed) {
    return "allowed";
  }
  return verdict.reason;
}
TS
cat > "$source/src/store.ts" <<'TS'
import { readFileSync } from "node:fs";

export function load(path: string): string {
  const text = readFileSync(path, "utf8");
  return text.trim();
}

export function lines(text: string): number {
  return text.split("\n").length;
}

export function widest(text: string): number {
  const each = text.split("\n").map((line) => line.length);
  return Math.max(0, ...each);
}
TS
cat > "$source/test/gate.test.ts" <<'TS'
import { describe, expect, it } from "vitest";

import { decide, explain } from "../src/gate";

describe("decide", () => {
  it("allows a count under the limit", () => {
    expect(decide(1, 2).allowed).toBe(true);
  });

  it("refuses a count over the limit", () => {
    expect(decide(3, 2).allowed).toBe(false);
  });

  it("says why it refused", () => {
    expect(explain(decide(3, 2))).toBe("over the limit");
  });
});
TS
cat > "$source/test/store.test.ts" <<'TS'
import { describe, expect, it } from "vitest";

import { lines, widest } from "../src/store";

describe("store", () => {
  it("counts the lines it was given", () => {
    expect(lines("one\ntwo")).toBe(2);
  });

  it("measures the widest line", () => {
    expect(widest("one\nthree")).toBe(5);
  });

  it("measures nothing as nothing", () => {
    expect(widest("")).toBe(0);
  });
});
TS
commit "the project begins"

# Every commit after the first changes files that were already there, which is
# what a case needs: a shape taken out of a file has to have been in it.
for step in 1 2 3 4 5 6 7 8; do
  cat >> "$source/src/store.ts" <<TS

export function shortest$step(text: string): number {
  const each = text.split("\n").map((line) => line.length);
  return Math.min(...each, $step);
}
TS
  python3 - "$source/test/store.test.ts" "$step" <<'PY'
import sys

path, step = sys.argv[1], sys.argv[2]
text = open(path, encoding="utf-8").read()
case = (
    f'\n  it("measures the shortest line, take {step}", () => {{\n'
    f'    expect(shortest{step}("one\\nthree")).toBe({step});\n'
    "  });\n"
)
closed = text.rfind("});")
open(path, "w", encoding="utf-8").write(text[:closed] + case + text[closed:])
PY
  commit "the project grows, step $step"
done
pin="$(git -C "$source" rev-parse main)"

bench="$scratch/bench"
mkdir -p "$bench"
pinned_corpus() {
  printf '[[repo]]\nname = "probe"\nsource = "%s"\ntip = "%s"\n' "$source" "$1" > "$bench/corpus.toml"
}

# Every run gets a cache of its own: a second run that read the same clone could
# be repeating itself rather than reading the pin.
campaign() { # <run-name> <out.md> <out.json>
  local cache="$scratch/cache-$1"
  mkdir -p "$cache"
  if ! WEED_RECALL_CACHE="$cache" cargo run -q -p xtask -- mutate \
    --corpus "$bench/corpus.toml" \
    --cases 3 \
    --commits 12 \
    --cases-dir "$cache/cases" \
    --out "$2" \
    --json "$3" > "$scratch/$1.log" 2>&1; then
    echo "the campaign refused to run over the history built for it:" >&2
    cat "$scratch/$1.log" >&2
    exit 3
  fi
}

pinned_corpus "$pin"
campaign one "$bench/one.md" "$bench/one.json"
campaign two "$bench/two.md" "$bench/two.json"

section "$bench/one.md" > "$bench/one.section"
section "$bench/two.md" > "$bench/two.section"
if ! cmp -s "$bench/one.section" "$bench/two.section"; then
  echo "two runs over one corpus wrote different recall sections:" >&2
  diff "$bench/one.section" "$bench/two.section" >&2 || true
  status=1
fi
if ! cmp -s "$bench/one.json" "$bench/two.json"; then
  echo "two runs over one corpus planted different cases:" >&2
  diff "$bench/one.json" "$bench/two.json" >&2 || true
  status=1
fi

# A commit on the source, made after the pin was written, in a file the campaign
# would plant in.
cat >> "$source/src/gate.ts" <<'TS'

export function tally(verdicts: Verdict[]): number {
  return verdicts.filter((verdict) => verdict.allowed).length;
}
TS
commit "a commit made after the pin was written"
moved="$(git -C "$source" rev-parse main)"

campaign three "$bench/three.md" "$bench/three.json"
section "$bench/three.md" > "$bench/three.section"
if ! cmp -s "$bench/one.section" "$bench/three.section"; then
  echo "a commit made on the source after the pin changed the recall section, so the campaign is not pinned:" >&2
  diff "$bench/one.section" "$bench/three.section" >&2 || true
  status=1
fi

# And the other way, or the sameness above says nothing: the pin moved to the
# new commit has to change the section and plant a case in it.
pinned_corpus "$moved"
campaign four "$bench/four.md" "$bench/four.json"
section "$bench/four.md" > "$bench/four.section"
if cmp -s "$bench/one.section" "$bench/four.section"; then
  echo "moving the pin to the new commit changed nothing, so the window is not read off the pin at all" >&2
  status=1
fi

python3 - "$bench" "$pin" "$moved" <<'PY' || status=1
import json
import sys

bench, pin, moved = sys.argv[1], sys.argv[2], sys.argv[3]
complaints = []


def run(name):
    return json.load(open(f"{bench}/{name}.json", encoding="utf-8"))


one, three, four = run("one"), run("three"), run("four")

# A run small enough to plant nothing would pass every comparison above.
FLOOR = 10
if len(one["cases"]) < FLOOR:
    complaints.append(
        f"the run planted {len(one['cases'])} cases in the history built for it, "
        f"and the check wants at least {FLOOR} before it believes two runs agree"
    )

for name, walked in (("the first run", one), ("the run after the new commit", three)):
    for repository in walked["repositories"]:
        if repository["head"] != pin:
            complaints.append(
                f"{name} walked {repository['repository']} from {repository['head']}, "
                f"and the corpus pins {pin}"
            )
    planted = {case["commit"] for case in walked["cases"]}
    if moved in planted:
        complaints.append(
            f"{name} planted a case in {moved[:9]}, which was committed after the pin"
        )

for repository in four["repositories"]:
    if repository["head"] != moved:
        complaints.append(
            f"the run with the pin moved walked {repository['repository']} from "
            f"{repository['head']}, and the corpus pins {moved}"
        )
if moved not in {case["commit"] for case in four["cases"]}:
    complaints.append(
        f"the pin was moved to {moved[:9]} and no case was planted in it, so the "
        "sections differing says nothing about the window"
    )

for complaint in complaints:
    print(complaint, file=sys.stderr)
raise SystemExit(1 if complaints else 0)
PY

# -------------------------------------------------------- the shipped corpus --
# Both files, fetched at their pins, and the campaign run over them twice.
corpus="docs/calibration/corpus.toml"
go="docs/calibration/corpus-go.toml"
[ -f "$corpus" ] || { echo "$corpus is missing: the campaign names no repositories to walk" >&2; exit 1; }
[ -f "$go" ] || { echo "$go is missing: the Go column has no history to be measured on" >&2; exit 1; }

bash "$root/scripts/corpus-scratch.sh" "$scratch/pins" "$corpus" > "$scratch/pins.tsv"
bash "$root/scripts/corpus-scratch.sh" "$scratch/pins" "$go" >> "$scratch/pins.tsv"
[ -s "$scratch/pins.tsv" ] || { echo "the corpus fetched nothing" >&2; exit 1; }

shipped() { # <run-name> <out.md> <out.json>
  if ! cargo run -q -p xtask -- mutate \
    --cases "$cases" \
    --commits "$commits" \
    --cases-dir "$scratch/shipped-cases" \
    --out "$2" \
    --json "$3" > "$scratch/$1.log" 2>&1; then
    echo "the campaign could not run over the shipped corpus. every repository has to hand over the commit it is pinned at." >&2
    cat "$scratch/$1.log" >&2
    exit 3
  fi
}

shipped shipped-one "$scratch/shipped-one.md" "$scratch/shipped-one.json"
shipped shipped-two "$scratch/shipped-two.md" "$scratch/shipped-two.json"
section "$scratch/shipped-one.md" > "$scratch/shipped-one.section"
section "$scratch/shipped-two.md" > "$scratch/shipped-two.section"
if ! cmp -s "$scratch/shipped-one.section" "$scratch/shipped-two.section"; then
  echo "two runs over the shipped corpus wrote different recall sections:" >&2
  diff "$scratch/shipped-one.section" "$scratch/shipped-two.section" >&2 || true
  status=1
fi
if ! cmp -s "$scratch/shipped-one.json" "$scratch/shipped-two.json"; then
  echo "two runs over the shipped corpus planted different cases:" >&2
  diff "$scratch/shipped-one.json" "$scratch/shipped-two.json" >&2 || true
  status=1
fi

# The window git counts from each pin, which is the window the campaign may
# plant in and no other.
while IFS=$'\t' read -r name path tip source_url; do
  [ -d "$path/.git" ] || {
    echo "$name was not fetched into a scratch, so the evidence has no history to read" >&2
    status=1
    continue
  }
  git -C "$path" log --first-parent --format=%H -n "$commits" "$tip" > "$scratch/window-$name.txt"
  if ! grep -q "^| $name | \`$source_url\` | \`$tip\` |" "$scratch/shipped-one.md"; then
    echo "$name: the report does not name the source and the commit its window ends at, $tip" >&2
    status=1
  fi
done < "$scratch/pins.tsv"

python3 - "$scratch" "$scratch/pins.tsv" <<'PY' || status=1
import json
import sys

scratch, pins_path = sys.argv[1], sys.argv[2]
complaints = []

pins = {}
for line in open(pins_path, encoding="utf-8"):
    name, path, tip, source = line.rstrip("\n").split("\t")
    pins[name] = tip

run = json.load(open(f"{scratch}/shipped-one.json", encoding="utf-8"))
walked = {repository["repository"]: repository for repository in run["repositories"]}

for name, tip in pins.items():
    repository = walked.get(name)
    if repository is None:
        complaints.append(f"{name} is pinned in the corpus and the campaign never walked it")
        continue
    if repository["head"] != tip:
        complaints.append(
            f"{name}: the campaign walked from {repository['head']} and the corpus pins {tip}"
        )
for name in walked:
    if name not in pins:
        complaints.append(f"{name} was walked and no corpus file pins it")

windows = {}
for name in pins:
    try:
        windows[name] = set(
            open(f"{scratch}/window-{name}.txt", encoding="utf-8").read().split()
        )
    except OSError as error:
        complaints.append(f"{name}: the window could not be read: {error}")

for case in run["cases"]:
    window = windows.get(case["repository"])
    if window is None:
        continue
    if case["commit"] not in window:
        complaints.append(
            f"{case['rule']} · {case['repository']} {case['commit'][:9]} was planted in a "
            "commit outside the window that ends at the pin"
        )

planted = len(run["cases"])
if planted < 200:
    complaints.append(
        f"the run over the shipped corpus planted {planted} cases, too few to say what it walks"
    )

for complaint in complaints[:20]:
    print(complaint, file=sys.stderr)
raise SystemExit(1 if complaints else 0)
PY

[ "$status" -eq 0 ] || exit "$status"
echo "the campaign walks the window that ends at each corpus pin: two runs over one corpus wrote the same recall section byte for byte, a commit made on a source after the pin left it unchanged, moving the pin moved it, and every case of a run over the shipped corpus was planted inside the pinned window"

#!/usr/bin/env bash
# Evidence for calibration c10: the ledger is redone with the classes sharpened
# to the finding's own sentence, every blocked commit read again under the
# sharpened T1, and a fresh blind re-grade taken against that ledger.
#
# The three classes used to drift towards how welcome a change was. Sharpened,
# they follow the finding and nothing else: `false-positive` where what the
# finding says is untrue of the change, `acceptable` where it is true and the
# change was fine, `true-positive` where it is true and something was weakened.
# A finding says more than a count, and a sentence like `so the suite reports
# green over behaviour nobody is holding` is untrue where the same change puts
# those checks in another file, whatever the count says.
#
# A claim like that cannot be proved by a script alone, so this one proves the
# things around it that would make it hollow, and then makes the second party's
# reading the test of it. The ledger has to hold the block set the shipped rules
# produce today and no other, every line has to name a rule that blocked that
# commit, the T1 that judged them has to be the one that follows a case into the
# file it moved to, the blind re-grade has to be a re-grade of this ledger and
# not of one it replaced, and every packet it was taken on has to have handed the
# auditor the change it asked about. Then the verdict says what that re-grade
# earns: at the bar on every sample, or the first sentence says the
# classification is untrusted.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"

report="docs/calibration-2026-09.md"
ledger="docs/calibration/judgements.toml"
cases="fixtures/adversarial/calibration-audit/blind-2026-09/cases"
sessions="fixtures/adversarial/calibration-audit/blind-2026-09/sessions"
audit="$(ls docs/calibration-audit*.md 2>/dev/null | grep -v '\.packet\.\|\.response\.\|transcript' | tr '\n' ':' | sed 's/:$//')"

command -v python3 >/dev/null 2>&1 || {
  echo "python3 is not on PATH, so neither the ledger nor the report can be read." >&2
  exit 3
}
command -v git >/dev/null 2>&1 || { echo "git is not on PATH" >&2; exit 3; }
[ -f "$report" ] || { echo "$report is missing: there is no block set for a ledger to answer" >&2; exit 1; }
[ -f "$ledger" ] || { echo "$ledger is missing: nothing classifies the blocks" >&2; exit 1; }

status=0
scratch="$(mktemp -d)"
# Scratch goes to the bin, never to rm; where there is no trash command the
# temp directory keeps it and the system clears it.
trap 'command -v trash >/dev/null 2>&1 && trash "$scratch"' EXIT

# ---------------------------------------------------------------------------
# The T1 that judged the ledger follows a case into the file it moved to.
#
# The redo was asked for because the old T1 called a moved case a deleted one,
# and a ledger read under that T1 is a ledger about findings weed no longer
# makes. So the shipped binary is put to a history where a case moves and a
# history where one vanishes, and it has to tell them apart.
# ---------------------------------------------------------------------------
probe="$scratch/t1"
mkdir -p "$probe/tests"
git -C "$probe" init --quiet --initial-branch=main
git -C "$probe" config user.name "weed measurements"
git -C "$probe" config user.email "measurements@weed.invalid"
cat > "$probe/tests/parser.rs" <<'RUST'
#[test]
fn parses_a_name() {
    assert_eq!(parse("a"), "a");
}

#[test]
fn parses_a_pair() {
    assert_eq!(parse("a b"), "a b");
}
RUST
cat > "$probe/tests/format.rs" <<'RUST'
#[test]
fn formats_a_name() {
    assert_eq!(format("a"), "a");
}
RUST
git -C "$probe" add -A
GIT_AUTHOR_DATE="2026-09-06T09:00:00+00:00" GIT_COMMITTER_DATE="2026-09-06T09:00:00+00:00" \
  git -C "$probe" commit --quiet -m "the repository begins"

# The move: parses_a_pair leaves parser.rs and arrives in format.rs unchanged.
cat > "$probe/tests/parser.rs" <<'RUST'
#[test]
fn parses_a_name() {
    assert_eq!(parse("a"), "a");
}
RUST
cat > "$probe/tests/format.rs" <<'RUST'
#[test]
fn formats_a_name() {
    assert_eq!(format("a"), "a");
}

#[test]
fn parses_a_pair() {
    assert_eq!(parse("a b"), "a b");
}
RUST
weed_check() {
  ( cd "$1" && cargo run -q --manifest-path "$root/Cargo.toml" -p weed -- check --base "$2" --strict --format sarif ) 2>/dev/null
}
# weed exits 2 on a block-level finding, and this probe reads one rule out of
# what it wrote rather than asking whether it refused.
moved="$(weed_check "$probe" HEAD || true)"
if printf '%s' "$moved" | python3 -c 'import json,sys
log = json.load(sys.stdin)
runs = log.get("runs") or [{}]
hits = [r for r in runs[0].get("results", []) if r.get("ruleId") == "T1"]
raise SystemExit(0 if hits else 1)' 2>/dev/null; then
  echo "the T1 that judged this ledger still calls a moved case a deleted one, so the ledger was read under the rule the redo replaced" >&2
  status=1
fi

# The deletion: the same case leaves parser.rs and arrives nowhere.
cat > "$probe/tests/format.rs" <<'RUST'
#[test]
fn formats_a_name() {
    assert_eq!(format("a"), "a");
}
RUST
gone="$(weed_check "$probe" HEAD || true)"
if ! printf '%s' "$gone" | python3 -c 'import json,sys
log = json.load(sys.stdin)
runs = log.get("runs") or [{}]
hits = [r for r in runs[0].get("results", []) if r.get("ruleId") == "T1"]
raise SystemExit(0 if hits else 1)' 2>/dev/null; then
  echo "T1 did not report a case that left the change altogether, so this probe proves nothing about the rule that judged the ledger" >&2
  status=1
fi

# ---------------------------------------------------------------------------
# The ledger answers the block set the shipped rules produce, and every line of
# it names a rule that blocked that commit.
# ---------------------------------------------------------------------------
python3 - "$report" "$ledger" <<'PY' || status=1
import re
import sys

report_path, ledger_path = sys.argv[1:3]
report = open(report_path, encoding="utf-8").read()
ledger = open(ledger_path, encoding="utf-8").read()
complaints = []

# Every blocked commit the report holds, with the rules that blocked it and the
# class the report printed.
blocks = {}
repo = None
recall = False
for line in report.splitlines():
    if line.strip() == "<!-- recall:begin -->":
        recall = True
    if recall:
        continue
    heading = re.match(r"^## ([A-Za-z0-9_-]+), \d+ commits judged,", line)
    if heading:
        repo = heading.group(1)
        continue
    row = re.match(r"^\| `([0-9a-f]{7,40})`[^|]*\| ([^|]+) \| ([^|]+) \| (.+) \|$", line)
    if not repo or not row:
        continue
    rules = [rule.strip() for rule in row.group(2).split(",") if rule.strip()]
    blocks[(repo, row.group(1))] = (rules, row.group(3).strip(), row.group(4).strip())

if not blocks:
    complaints.append(f"{report_path} names no blocked commit, so there is no ledger to check")

for (repo_name, sha), (_, classification, _) in blocks.items():
    if classification == "unclassified":
        complaints.append(
            f"{repo_name} {sha} is blocked and nobody classified it, so the ledger was not redone over the whole block set"
        )

# Every entry the ledger writes down.
entries = []
for block in ledger.split("[[commit]]")[1:]:
    block = block.split("\n[[")[0]
    fields = {}
    for key in ("repo", "sha", "classification"):
        found = re.search(rf'^\s*{key}\s*=\s*"([^"]*)"', block, re.M)
        if found:
            fields[key] = found.group(1)
    reasoning = re.search(r'^\s*reasoning\s*=\s*"(.*)"\s*$', block, re.M)
    if len(fields) == 3 and reasoning:
        entries.append((fields["repo"], fields["sha"], fields["classification"], reasoning.group(1)))

if not entries:
    complaints.append(f"{ledger_path} carries no judgement")

CLASSES = {"true-positive", "acceptable", "false-positive"}
seen = set()
for repo_name, sha, classification, reasoning in entries:
    key = next(
        (held for held in blocks if held[0] == repo_name and sha.startswith(held[1])),
        None,
    )
    if key is None:
        complaints.append(
            f"{ledger_path} classifies {repo_name} {sha[:10]}, which nothing blocks in this run: the ledger holds a judgement about a report that no longer exists"
        )
        continue
    seen.add(key)
    if classification not in CLASSES:
        complaints.append(f"{repo_name} {sha[:10]}: '{classification}' is not one of {sorted(CLASSES)}")
    rules = blocks[key][0]
    if not any(re.search(rf"\b{re.escape(rule)}\b", reasoning) for rule in rules):
        complaints.append(
            f"{repo_name} {sha[:10]}: the reasoning names none of the rules that blocked it ({', '.join(rules)}), "
            "so it does not say what the finding claims and the class cannot be read against it"
        )

for key in blocks:
    if key not in seen:
        complaints.append(f"{key[0]} {key[1]} is blocked and the ledger has no entry for it")

for complaint in complaints:
    print(complaint, file=sys.stderr)
if complaints:
    raise SystemExit(1)
print(f"{len(blocks)} blocked commits, {len(entries)} judgements, each naming a rule that blocked its commit")
PY

# ---------------------------------------------------------------------------
# The blind re-grade is a re-grade of this ledger, and every packet it was taken
# on handed the auditor the change it asked about.
# ---------------------------------------------------------------------------
python3 - "$report" "$audit" "$cases" "$sessions" "$scratch" <<'PY' || status=1
import hashlib
import json
import os
import pathlib
import re
import subprocess
import sys

report_path, audit_path, case_dir, session_dir, scratch = sys.argv[1:6]
report = open(report_path, encoding="utf-8").read()
complaints = []


def declaration(text):
    for line in text.splitlines():
        stripped = "".join(mark for mark in line if mark not in "*_#>`").strip().lower()
        if not stripped.startswith("blind:"):
            continue
        answer = stripped[len("blind:"):].split(";")[0].split(",")[0].split(".")[0].strip()
        if answer in ("yes", "true"):
            return "blind"
        if answer in ("no", "false"):
            return "sighted"
    return None


blind = None
for path in [one for one in audit_path.split(":") if one and os.path.exists(one)]:
    text = open(path, encoding="utf-8").read()
    if declaration(text) == "blind":
        blind = (path, text)
        break

if blind is None:
    complaints.append(
        "no re-grade declares itself blind, so nothing outside the builder has read this ledger back"
    )
else:
    path, text = blind
    seed = re.search(r"(?im)^seed:\s*([A-Za-z0-9._:-]+)\s*$", text)
    if not seed:
        complaints.append(f"{path} records no seed, so the sample it drew cannot be redrawn")
    else:
        # The packets, regenerated from the report the ledger describes. A
        # re-grade of a ledger that has been redone since is a re-grade of
        # something else, and its sample says so: it is drawn from the blocked
        # commits the report holds.
        fresh = pathlib.Path(scratch) / "cases"
        made = subprocess.run(
            ["cargo", "run", "-q", "-p", "xtask", "--", "audit-packet",
             "--seed", seed.group(1), "--dir", str(fresh)],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        if made.returncode != 0:
            complaints.append(
                "the case packets could not be regenerated from this report: " + made.stderr.strip()
            )
        else:
            kept = sorted(pathlib.Path(case_dir).glob("*.md"))
            regenerated = sorted(fresh.glob("*.md"))
            if [one.name for one in kept] != [one.name for one in regenerated]:
                complaints.append(
                    f"{case_dir} holds packets for a different sample than seed {seed.group(1)} draws "
                    "from this report: the re-grade was taken against a ledger this one replaced"
                )
            for one, other in zip(kept, regenerated):
                if one.read_text(encoding="utf-8") != other.read_text(encoding="utf-8"):
                    complaints.append(f"{one} is not the packet the generator writes for this report")

        # Fairness: a packet may cap what it shows and has to say what it showed,
        # and it never withholds a change while asking about it.
        for one in sorted(pathlib.Path(case_dir).glob("*.md")):
            body = one.read_text(encoding="utf-8")
            shown = re.search(r"(?m)^Diff shown: (.+)$", body)
            if not shown:
                complaints.append(f"{one} does not say how much of the change it showed")
                continue
            if "Files this change touched:" not in body:
                complaints.append(f"{one} does not count what the change did to the files it touched")
            if shown.group(1) == "the whole change" and "```diff" not in body:
                complaints.append(f"{one} says it shows the whole change and carries no diff")
            if shown.group(1) == "none" and "exceeds the stated cap" not in body:
                complaints.append(f"{one} shows no diff and does not say why")
            if "builder's classification" in body.lower():
                complaints.append(f"{one} names the builder's classification")

        # Every sampled case was answered by a session that names its packet.
        for one in sorted(pathlib.Path(case_dir).glob("*.md")):
            events_path = pathlib.Path(session_dir) / f"{one.stem}.events.jsonl"
            if not events_path.exists():
                complaints.append(f"{one.stem} has no session record, so nobody re-graded it")
                continue
            lines = [line for line in events_path.read_text(encoding="utf-8").splitlines() if line.strip()]
            if len(lines) != 2:
                complaints.append(f"{events_path} is not one input and one answer")
                continue
            given, answered = json.loads(lines[0]), json.loads(lines[1])
            digest = hashlib.sha256(one.read_text(encoding="utf-8").encode("utf-8")).hexdigest()
            if given.get("sha256") != digest:
                complaints.append(f"{events_path} names a packet hash the packet beside it does not have")
            if not answered.get("answer", "").startswith(f"Answered packet SHA-256: {digest}"):
                complaints.append(f"{events_path} carries an answer that does not name the packet it was given")

for complaint in complaints:
    print(complaint, file=sys.stderr)
if complaints:
    raise SystemExit(1)
print(f"the blind re-grade in {blind[0]} was taken on packets this report writes today, one session per case")
PY

# ---------------------------------------------------------------------------
# And the verdict says what the re-grade earns.
# ---------------------------------------------------------------------------
python3 - "$report" "$audit" <<'PY' || status=1
import os
import re
import sys

report_path, audit_path = sys.argv[1:3]
report = open(report_path, encoding="utf-8").read()
complaints = []

AGREEMENT_BAR = 90.0
SAMPLE_FLOOR = 20


def samples(text):
    found = []
    for line in text.splitlines():
        line = line.strip()
        if not line.startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip("|").split("|")]
        if len(cells) < 3:
            continue
        try:
            found.append((cells[0].strip("*").strip(), int(cells[1]), int(cells[2])))
        except ValueError:
            continue
    return found


drawn = []
for path in [one for one in audit_path.split(":") if one and os.path.exists(one)]:
    drawn.extend(
        (f"{name} in {path}", regraded, agreed)
        for name, regraded, agreed in samples(open(path, encoding="utf-8").read())
    )

stands = bool(drawn) and all(
    regraded >= SAMPLE_FLOOR and agreed <= regraded and agreed * 100.0 / regraded >= AGREEMENT_BAR
    for _, regraded, agreed in drawn
)

prose = "\n".join(
    line for line in report.splitlines() if line.strip() and not line.lstrip().startswith("#")
).strip()
first = re.split(r"(?<=[.!?])\s+", prose, maxsplit=1)[0] if prose else ""
untrusted = "untrusted" in first.lower()

if stands and untrusted:
    complaints.append(
        "every sample agrees at the bar and the first sentence still calls the classification untrusted"
    )
if not stands and not untrusted:
    short = "; ".join(
        f"{name} agrees on {agreed * 100.0 / regraded:.1f} percent of {regraded} cases"
        if regraded
        else f"{name} re-graded nothing"
        for name, regraded, agreed in drawn
        if not (
            regraded >= SAMPLE_FLOOR
            and agreed <= regraded
            and agreed * 100.0 / regraded >= AGREEMENT_BAR
        )
    ) or "no re-grade drew a sample"
    complaints.append(
        "the re-grade is under the bar and the first sentence does not say the classification is "
        f"untrusted: {short}"
    )

for complaint in complaints:
    print(complaint, file=sys.stderr)
if complaints:
    raise SystemExit(1)
print(
    "the verdict says what the re-grade earns: "
    + ("every sample at the bar" if stands else "under the bar, and the classification called untrusted")
)
PY

[ "$status" -eq 0 ] || exit "$status"
echo "the ledger answers the block set the sharpened T1 produces, every line names the rule it answers, and the blind re-grade behind the verdict was taken on this report's own packets"

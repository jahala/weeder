# weed and tend2

tend2 verifies a loop by running the evidence a check names and stamping what
came back. Two of its questions are ones weed already answers. This is the
proposal for both seams, written from weed's side; tend2 owns its verifier and
decides what it wants.

## The hygiene loop cites a scan

A map rots the way any repository's documentation rots: a loop names a script
that was renamed, a narrative cites a flag that was dropped, a check points at a
test file somebody moved. `weed scan` reads the tree and reports exactly that
class of decay, so a hygiene loop can carry a check whose evidence is a script
built around one command:

```
- [ ] (code) the map and the docs cite no path, command, flag or symbol that has gone · scripts/check/hygiene.sh
```

```bash
#!/usr/bin/env bash
set -euo pipefail
weed scan --format sarif > hygiene.sarif
stale=$(jq '[.runs[0].results[] | select(.ruleId == "R1")] | length' hygiene.sarif)
[ "$stale" -eq 0 ] || { jq -r '.runs[0].results[] | select(.ruleId == "R1") | "\(.properties.path):\(.locations[0].physicalLocation.region.startLine) \(.message.text)"' hygiene.sarif >&2; exit 1; }
```

A scan leaves with 0 whatever it finds, so the script is what decides. That is
deliberate: repository decay is nobody's change to answer for, and a gate that
blocks on it teaches a worker to stop reading it.

What a consumer needs out of the log is small, and weed writes all of it:

| Field | What it carries |
|---|---|
| `ruleId` | `R1`, `R2`, `R3` or `R4` |
| `level` | `warning`, for every scan result |
| `message.text` | what was found, why it matters, then the next action |
| `locations[0].physicalLocation.artifactLocation.uri` | the file, repository-relative |
| `locations[0].physicalLocation.region.startLine` | the line, where the finding has one |
| `properties.path` | the same file again, so aggregating per path is one field read |

`--rules` narrows a run to the ids a loop cares about, which is how a hygiene
loop about documentation asks for R1 alone and pays for nothing else.

## The verifier could call weed check in place of mock.ts

tend2's verifier refuses a stamp whose evidence only mocks the unit it claims to
cover. The detector is `src/verify/mock.ts`: it reads the evidence file, pulls
the import specifiers and the `vi.mock` and `jest.mock` calls out with regular
expressions, resolves them against the loop's production paths, and classifies
the file as real, partial, mock-only or unknown. `verify.ts` turns mock-only into
`refused-mock-only`.

Three things follow from how it is built. It answers `unknown` for anything whose
extension is not `.ts` or `.tsx`, so a Rust, Python or Go loop is never judged.
It knows the two mocking calls vitest and jest spell, and no others. And it needs
the loop's production paths handed to it, which the check has to have named.

weed asks the same question from the other side. It judges the change a worker
produced, reads each file as the language it is written in, and reports M1 when a
test in the diff mocks a module whose production file is in that same diff — no
path list to hand over, because the diff already says which production files the
work touched. The invocation is one line:

```bash
weed check --base <the commit the node started from> --strict --format sarif
```

Exit 2 refuses the stamp, 0 allows it, 3 means the verifier could not judge and
must not stamp. `--strict` is the part that matters to a verifier: a worker can
write itself a suppression, and under `--strict` weed reports it at the level its
rule carries and refuses to judge one it cannot read.

M1 is in the catalogue today — `weed rules --format json` lists it — and its
detector lands in the rules-tests loop. So this half is a proposal about a rule
that is named and not yet built. What a verifier wired to weed would get today is
the rest of the same family: a deleted test, a skip or focus marker, a stub in
production code, a secret, a guardrail edit, a committed conflict marker. Those
already refuse evidence a worker weakened to make a check go green, which is the
judgement `mock.ts` exists to make.

## The commands, and what they leave with

| Command | Where | Exit |
|---|---|---|
| `weed scan --format sarif` | this repository | 0 |
| `weed scan --rules R1 --format sarif` | this repository | 0 |
| `weed rules --format json` | this repository | 0 |
| `weed check --strict --format sarif` | a repository with nothing to judge | 0 |
| `weed check --strict --format sarif` | a repository whose change deletes a test | 2 |

`scripts/check/docs-tend2-seam.sh` runs every one of them and compares the code
against this table, so a claim here cannot outlive the binary.

## What tend2 decides

- Whether the hygiene loop wants more than the six fields above. weed will add
  fields; it will not move those.
- Whether `tend2 gate` and `weed check` both run in CI. They ask different
  questions — one about claims a change touched, one about the change itself —
  and both write SARIF, so a forge can read them together.
- Whether the verifier calls weed at all, or keeps `mock.ts` for TypeScript and
  calls weed for the languages it cannot read.

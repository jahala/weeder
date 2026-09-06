# weed

weed is a gate for code an agent wrote. It reads the diff, refuses dishonest growth, and leaves
with an exit code that says whether the change goes through. Deleted or weakened tests, skips,
stubs, swallowed errors, secrets, guardrail edits and dependency-direction violations are the
shapes it refuses. A static Rust binary, milliseconds, no tokens spent, every finding as
SARIF 2.1.0.

```
$ weed check
error  G1  src/parser.ts:2  a merge conflict marker (<<<<<<<) was added.
error  G1  src/parser.ts:4  a merge conflict marker (=======) was added.
error  G1  src/parser.ts:6  a merge conflict marker (>>>>>>>) was added.
3 errors, 0 warnings, 0 notes
```

Exit 2. The commit does not happen.

## Why

An agent that has just written a change is the worst available judge of it. It will report
success, and it has every incentive to reach success by the shortest road: delete the failing
test, add a skip, catch the exception and move on, widen the timeout until the flake stops.
Those are cheap to do and expensive to notice, and a reviewer reading a thousand-line diff
notices them last.

weed notices them first, deterministically, before a human opens the pull request. It has no
model, no prompt and no opinion about style. It reads what changed against what was there and
reports shapes: a test that is gone, an assertion that was dropped, a `TODO` where an
implementation belongs.

## Install

```bash
npm install -g @plotplot/weed
cargo install weed
```

Release binaries are published for linux and macos on x86_64 and aarch64, and for windows on
x86_64; the npm wrapper in `npm/` fetches the one for your platform. The npm package is scoped
under the garden's org because the bare name was taken years ago by an empty placeholder; the
crate and the binary are plain `weed`.

## The faces

| Face | What it does |
|---|---|
| `weed check` | judges a diff and may block. This is the one wired into hooks and CI |
| `weed scan` | judges the repository as it is and never blocks: rotted docs, dead exports, stale work markers, lagging pins |
| `weed guard` | puts that judgement inside git, through hooks git cannot be talked out of running |
| `weed hook` | answers an agent harness's hook event, so a turn ends against weed's verdict |
| `weed rules` | prints the catalogue, the level each rule carries, and what it may reach over the network |

```bash
weed check --staged
weed check --base origin/main --strict --format sarif
weed scan --format sarif
weed guard install --protect main
weed guard status
weed hook claude
weed rules --format json
```

`weed check` with no arguments judges the index and the working tree against `HEAD`. On a
terminal it writes a table; on a pipe it writes SARIF. `--strict` reports suppressed findings
at their own level, which is how a reviewer sees what an agent waved through.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | clean, or warnings only |
| 2 | at least one block-level finding |
| 3 | weed could not run, and the message says why |

Exit 3 is a refusal, not a pass: weed judged nothing, so nothing was cleared. Only the
unambiguous rules block by default. The rest warn, and warnings are for the human at the
pull request rather than for the agent.

## In CI

`examples/ci/github.yml` is a workflow you can copy: it judges the pull request against the
branch it is opening onto, writes SARIF, and hands that to GitHub's code scanning, so every
finding lands as an annotation on the diff a reviewer is already reading.

```yaml
- run: weed check --base origin/${{ github.base_ref }} --strict --format sarif > weed.sarif
- uses: github/codeql-action/upload-sarif@v4
  with:
    sarif_file: weed.sarif
```

For a pleach plan, `examples/pleach/plan.json` gates every node on weed and `docs/pleach.md`
explains why that gate needs no base argument.

## Reading the output

The SARIF weed writes is plain SARIF 2.1.0: one run per invocation, one result per finding,
rules declared in the tool component, suppressions carried as SARIF suppressions rather than
dropped. `docs/sarif.md` names every convention and the test that pins it, and
`schemas/sarif-schema-2.1.0.json` is the official schema the tests validate against.

## For agents

`SKILL.md` is the whole binary in one file: every command, every flag, and how to work
against a gate rather than around it. Install it into your harness's skill directory, or read
it as-is.

## What is not settled

The name weed, the accent bramble and the MIT licence were confirmed by the owner on
2026-09-05. One claim is still open, marked **[flagged]** where it is made:

- **The manifest schema.** `schemas/garden.schema.json` is a vendored proposal. The real one
  belongs to a contracts repository that does not exist yet. **[flagged]**

## Building

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

`AGENTS.md` is the working brief: the layout, the dependency direction weed enforces on
itself, and the rules any change here is held to. The plan and the proof live as loops in
`docs/tend2/`; evidence scripts a loop cites live in `scripts/check/`; the adversarial
fixtures every rule is built from live in `fixtures/adversarial/`. `garden.json` is what the
umbrella reads to verify weed belongs to the garden.

## Support

[![Buy Me A Coffee](https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png)](https://buymeacoffee.com/jahala)

## License

MIT. See [LICENSE](./LICENSE).

## The garden

**plotplot** is a garden of small, sharp tools for building with AI: [plotplot.ai](https://plotplot.ai)

[tilth](https://github.com/jahala/tilth) · [tend](https://github.com/jahala/tend) ·
[petals](https://github.com/jahala/petals) · [pleach](https://github.com/jahala/pleach) ·
[umbel](https://github.com/jahala/umbel) · [copeca](https://github.com/jahala/copeca) ·
[pollen](https://github.com/jahala/pollen) · **weed**

© 2026 · a plotplot garden tool · MIT

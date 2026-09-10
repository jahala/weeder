# weeder

weeder is a gate for code an agent wrote. It reads the diff, refuses dishonest growth, and leaves
with an exit code that says whether the change goes through. Deleted or weakened tests, skips,
stubs, swallowed errors, secrets, guardrail edits and dependency-direction violations are the
shapes it refuses. A static Rust binary, milliseconds, no tokens spent, every finding as
SARIF 2.1.0.

```
$ weeder check
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

weeder notices them first, deterministically, before a human opens the pull request. It has no
model, no prompt and no opinion about style. It reads what changed against what was there and
reports shapes: a test that is gone, an assertion that was dropped, a `TODO` where an
implementation belongs.

## Install

Release binaries are built for linux and macos on x86_64 and aarch64, one `weeder-<target>.tar.gz`
per platform on the [releases page](https://github.com/jahala/weeder/releases), with its SHA-256
beside it. The tarball holds the executable, `garden.json` and `SKILL.md`.

```bash
target=aarch64-apple-darwin   # or x86_64-apple-darwin, x86_64-unknown-linux-musl, aarch64-unknown-linux-musl
base=https://github.com/jahala/weeder/releases/latest/download
curl -fsSLO "$base/weeder-$target.tar.gz" -O "$base/weeder-$target.tar.gz.sha256"
shasum -a 256 -c "weeder-$target.tar.gz.sha256"
tar xzf "weeder-$target.tar.gz" weeder
install -m 755 weeder ~/.local/bin/weeder
```

A windows build is on its way (#30). The crate on crates.io and the npm wrapper
(`@plotplot/weeder`) publish from the same release workflow once the registries are set up;
until then the tarball is the install.

## The faces

| Face | What it does |
|---|---|
| `weeder check` | judges a diff and may block. This is the one wired into hooks and CI |
| `weeder scan` | judges the repository as it is and never blocks: rotted docs, dead exports, stale work markers, lagging pins |
| `weeder guard` | puts that judgement inside git, through hooks git cannot be talked out of running |
| `weeder hook` | answers an agent harness's hook event, so a turn ends against weeder's verdict |
| `weeder rules` | prints the catalogue, the level each rule carries, and what it may reach over the network |

```bash
weeder check --staged
weeder check --base origin/main --strict --format sarif
weeder scan --format sarif
weeder guard install --protect main
weeder guard status
weeder hook claude
weeder rules --format json
```

`weeder check` with no arguments judges the index and the working tree against `HEAD`, including
the files git has never been told about, which is most of what an agent writes. On a terminal it
writes a table; on a pipe it writes SARIF. `--strict` reports suppressed findings at their own
level, which is how a reviewer sees what an agent waved through, and `--untracked exclude` leaves
the unstaged files out.

A finding is allowed through on the record and never in silence: a `Weeder-allow: <RULE>
<reason>` trailer on the commit being prepared, or a `weeder-allow <RULE>: <reason>` comment
on the line. Both need a reason, both stay in the log as a note carrying it, and `--strict`
hands the finding back the level its rule carries. `docs/rules.md` says the rest.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | clean, or warnings only |
| 2 | at least one block-level finding |
| 3 | weeder could not run, and the message says why |

Exit 3 is a refusal, not a pass: weeder judged nothing, so nothing was cleared. Only the
unambiguous rules block by default. The rest warn, and warnings are for the human at the
pull request rather than for the agent.

## In CI

`examples/ci/github.yml` is a workflow you can copy: it judges the pull request against the
branch it is opening onto, writes SARIF, and hands that to GitHub's code scanning, so every
finding lands as an annotation on the diff a reviewer is already reading.

```yaml
- run: weeder check --base origin/${{ github.base_ref }} --strict --format sarif > weeder.sarif
- uses: github/codeql-action/upload-sarif@v4
  with:
    sarif_file: weeder.sarif
```

For a pleach plan, `examples/pleach/plan.json` gates every node on weeder and `docs/pleach.md`
explains why that gate needs no base argument.

## Reading the output

The SARIF weeder writes is plain SARIF 2.1.0: one run per invocation, one result per finding,
rules declared in the tool component, suppressions carried as SARIF suppressions rather than
dropped. `docs/sarif.md` names every convention and the test that pins it, and
`schemas/sarif-schema-2.1.0.json` is the official schema the tests validate against.

## For agents

`SKILL.md` is the whole binary in one file: every command, every flag, and how to work
against a gate rather than around it. Install it into your harness's skill directory, or read
it as-is.

## Building

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

`AGENTS.md` is the working brief: the layout, the dependency direction weeder enforces on
itself, and the rules any change here is held to. Every rule is built from the adversarial
fixtures under `fixtures/adversarial/`, one minimal repository per rule per language.

## Support

[![Buy Me A Coffee](https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png)](https://buymeacoffee.com/jahala)

## License

MIT. See [LICENSE](./LICENSE).

## The garden

**plotplot** is a garden of small, sharp tools for building with AI: [plotplot.ai](https://plotplot.ai)

[tilth](https://github.com/jahala/tilth) · [tend](https://github.com/jahala/tend) ·
[petals](https://github.com/jahala/petals) · [pleach](https://github.com/jahala/pleach) ·
[umbel](https://github.com/jahala/umbel) · [copeca](https://github.com/jahala/copeca) ·
[pollen](https://github.com/jahala/pollen) · **weeder**

© 2026 · a plotplot garden tool · MIT

# Architecture

What weeder is, what it promises, and the state of the evidence behind each promise. `AGENTS.md` holds the layout and the working rules; the map in `docs/tend2/` holds the plan; `docs/decisions.md` indexes the rulings. This file holds what none of them states: the trust model, and which mechanisms are proven, assumed or open.

## 1. What weeder is

A static binary that judges what an agent produced and refuses dishonest growth: deleted or weakened tests, skips, stubs, swallowed errors, secrets, guardrail edits and dependency-direction violations. It reads git, runs no model, writes SARIF 2.1.0, and leaves with an exit code (0 clean or warnings, 2 a block, 3 could not run). Its users are maintainers reviewing agent-written changes, teams running agents on several harnesses, and the garden's own fleets through pleach.

## 2. Binding principles

1. The change being judged never supplies the law it is judged by. The configuration, the rule set and the hooks that run weeder come from a state the change did not write. *(Not yet true: see §6, T-1 and T-3.)*
2. Fail closed. Anything weeder cannot read is exit 3, never exit 0.
3. Only a finding that admits one reading blocks. `src/core/catalogue.rs` holds the level of every rule.
4. Deterministic output. The same diff, tree and config write the same bytes (`tests/determinism.rs`, `tests/sarif_order.rs`).
5. Core computes and reaches nothing. I/O lives in `src/seams/`; `src/faces/` join the two (`AGENTS.md`, enforced by D2 on weeder's own tree).

## 3. Layers

`core` (pure) ← `seams` (git, exec, fs, reader) ← `faces` (check, scan, guard, hook, bite, rules). Arrows point at what a layer may import. `weeder.toml [deps]` declares it; `tests/self_check.rs` and `tests/core_purity.rs` hold it. The details of each module are in `AGENTS.md`.

## 4. The trust model

The adversary is the agent whose change is judged. It has the same file-system and git access as the person it works for. It controls:

- every tracked and untracked file in the working tree, including `weeder.toml`, `.gitattributes` and the hooks directory;
- the git directory: `.git/info/attributes`, `.git/info/exclude`, `.git/config`, index flags such as skip-worktree;
- commit messages, including `Weeder-allow:` trailers;
- the commands it runs, in any spelling the shell accepts.

It does not control: a base commit that is already on a protected branch, and a CI runner that checks out the change fresh.

So the wall is CI running `weeder check --base <protected branch> --strict` from a clean checkout, with the law read from the base. Local hooks and harness hooks are early warning. They catch honest mistakes and cheap evasions, and they cannot stop an agent that works at defeating them. Every document that describes the local layers must say so.

## 5. Components and contracts

| Contract | Consumer | Where it is pinned |
|---|---|---|
| Exit codes 0, 2, 3 | pleach, CI, git hooks, harness hooks | `tests/check_exit.rs`, `tests/panic_hook.rs` |
| SARIF 2.1.0 log | GitHub code scanning, any reader | `docs/sarif.md`, `tests/sarif_schema.rs` |
| Rule catalogue | `weeder rules`, `docs/rules.md` | `tests/docs_rules.rs`, `tests/catalogue_matrix.rs` |
| Git hooks (`guard`) | git | `tests/guard_*.rs` |
| Harness hooks (`hook claude\|gemini\|codex`) | Claude Code, Gemini CLI, Codex | `tests/hook_*.rs` |
| Garden manifest | the plotplot umbrella | `garden.json`, `tests/garden_manifest.rs` |
| pleach smoke gate | pleach | `docs/pleach.md`, `examples/pleach/plan.json` |

## 6. Mechanisms and their risk status

| Mechanism | Status | Evidence |
|---|---|---|
| Rules detect their shapes in ts, py, rs, go | proven | `tests/catalogue_matrix.rs`: every rule fires and stays silent per language |
| Block-level false positives under 2 percent of commits | proven on the pinned corpus | `docs/calibration-2026-09.md`, blind re-grade in `docs/calibration-audit-blind-2026-09.md` |
| Recall of the six rules the recall loop bars | proven | `docs/calibration-2026-09.md`, recall section. D2, T2, T7, T8 and X2 also block and are not held to the bar; D2 is at 55 percent in Go |
| Latency budget | proven in CI | `tests/speed.rs` on the release profile |
| Never panics on input | proven for the diff parser and classifier | `tests/core_hostile.rs` (proptest). `syntax::Mask` is not fuzzed |
| T-1: the law comes from a state the change did not write | **open, fails open** | `weeder.toml` is read from the working tree (`src/faces/mod.rs`). See `docs/reviews/2026-09-23-repo-audit.md` |
| T-2: git shows weeder every changed line | **open, fails open** | `git diff` runs without `--text`, so an attribute or a NUL byte hides the hunks |
| T-3: the hooks that run weeder are weeder's | **open, fails open** | hooks live in the working tree; `guard status` trusts their contents |
| T-4: a harness hook sees every commit | **open** | `PreToolUse` matches the program `git` by its literal name; `Stop` judges against `HEAD` only |
| `bite` proves a test fails without its change | unshipped by measurement | `docs/bite-2026-09.md` |

## 7. Deployment and install

Release tarballs for five targets, each with a SHA-256, from `.github/workflows/release.yml`. The crate and the npm wrapper have never published: `tilth-core` is a git dependency (issue #45) and the npm token is empty. Until both work, the tarball is the only install, and `garden.json`, `npm/install.js` and `examples/ci/github.yml` must not point anywhere else.

## 8. Security baseline

- git runs from argument arrays with `--no-ext-diff --no-textconv --no-color` and `core.quotepath=false`.
- A secret is never echoed; X1 names the shape and the line.
- A panic is exit 3.
- `scan --refresh-snapshot` reaches the registries through curl; `scan` runs the programs `[docs] commands` names.
- Vulnerabilities are reported privately (`SECURITY.md`).

## 9. Variability

A repository adapts weeder through `weeder.toml` alone: levels per rule, scope, specimens, layers, guardrail paths, entry points, thresholds. Rules detect shapes and never name vendors or products. The keys are listed in `docs/rules.md`.

## 10. Out of scope

Rules that name vendors or products. Inbound prompt-injection quarantine. Spend rules. A model in the loop. Style.

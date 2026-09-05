# weed — the judge of the diff

weed is a static binary that reads what an agent produced and refuses dishonest growth: deleted or weakened tests, skips, stubs, swallowed errors, secrets, guardrail edits, dependency-direction violations. Milliseconds, zero tokens, every finding as SARIF 2.1.0. Three faces on one core: `weed check` judges a diff and may block; `weed scan` judges the tree and never blocks; `weed guard` is the law in git through hooks. `weed bite` proves a test fails without its change. Agents produce; weed decides.

The map in `docs/tend2/` is the plan and the proof. Read the project loop (`docs/tend2/weed.tend2.html`) and then the loop you are working. The governing documents live in the umbrella workspace and are read-only for this repo: `/Users/jahala/conductor/workspaces/plotplot-ai/cape-town-v1/docs/building-the-garden.md` (the law), `.../docs/tend2/weed.tend2.html` (the fit loop), `.../docs/prompts/weed-build-2026-09.md` (the build brief). Where they disagree, the loop beats the brief and the law beats both.

## Layout

```
src/core/      pure: parse_diff → Hunks · classify_file → kind + lang · rules → Vec<Finding> · sarif::render → Log · catalogue (rule ids, defaults, descriptions) · config · suppress
src/seams/     I/O behind small functions, injected by faces: git (diff, file at ref, refs, hooks path), exec (a command with a timeout), fs, reader (tilth-core: language detection, outlines, test shape, imports, callers)
src/faces/     the CLI subcommands: check, scan, guard, bite, hook, rules
tests/         integration tests that drive the real `weed` binary on real git repositories in temp dirs (assert_cmd + tempfile); tests/common/ holds the fixture harness
fixtures/adversarial/<RULE>/<lang>/{fire,silent}/{before,after}/   one minimal repo history per rule per language: `before/` is committed as HEAD, `after/` is the working tree; a file `after/.weed-commit` carries the commit message (for trailers) and is never copied
schemas/       vendored official schemas (sarif-schema-2.1.0.json)
scripts/check/ evidence scripts a loop cites; run.sh is the runner tend2 verify uses
docs/          sarif.md · pleach.md · tend2-seam.md · calibration-2026-09.md · proof-2026-09.md · dogfood.md · tend2/ (the map)
examples/      pleach/plan.json · ci/github.yml
```

Dependency direction is weed's own doctrine and its D2 rule enforces it on itself: `core` imports nothing from `seams` or `faces`; `seams` may import `core` types; `faces` import both. Core does no I/O and never panics on input; failure lives in the return type.

## Engineering rules

- The failing test is the spec. Every rule lands as a failing fixture first, in every listed language (TypeScript/JavaScript, Python, Rust, Go), then the detector. A test that cannot fail is not a test.
- Nothing mocked. Tests build a real git repo in a temp dir from the fixture, run the built binary, parse its SARIF. No in-process shortcuts for the faces.
- No stubs, TODOs, placeholders, fallbacks, or "phase two" comments in committed code. weed refuses them in other people's diffs; it does not carry them.
- Rules detect shapes, never vendor or product names.
- Failure in return types: `thiserror` enums, no `unwrap`/`expect` outside tests unless the invariant is stated in a comment beside it.
- Delete with `trash`, never `rm`. Never `git reset`. Never touch the git stash. Do not push, publish, or create anything on GitHub.
- Voice in every user-facing string: calm, precise, literate, a little wit. Sentence case. Product names lowercase (`weed`, `tilth`, `pleach`). No exclamation marks. A finding states what was found, why it matters, and the next action, in that order. Never: supercharge, unlock, 10x, magic, synergy, revolutionary, game-changing, cutting-edge, seamless, effortless, next-gen, AI-powered.
- Exit codes: 0 clean or warnings only · 2 at least one block-level result · 3 weed could not run (fail closed; the message says why). `scan` exits 0 or 3 only.
- Only unambiguous rules block by default: T1, T3, S1, X1, C1, G1. Everything else warns, and warnings are for the human at the pull request, not for the agent.
- A worker's dogfood notes on tend2, pleach or umbel go in `docs/dogfood/<loop-id>.md`, one file per loop, never in `docs/dogfood.md`, which the conductor owns and folds them into; two writers on one file conflict at every landing.
- A worker that is genuinely blocked writes `BLOCKED.md` at the repository root, and the file is not ignored, so it reaches the quarantine branch and the conductor can read why.

## Toolchain

```
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
bash scripts/check/run.sh tests/<name>.rs     # one evidence file, the way the verifier runs it
tend2 next docs/tend2                          # where things stand
```

`tilth-core` is a git dependency on `https://github.com/jahala/tilth`, pinned by rev to the commit the tilth agent landed (the tilth-core loop names it). Nothing in the repo points at a path on this machine; the worktree at `.context/tilth-core` exists only so the evidence script can run the crate's own suite.

<!-- tend2:begin -->
## tend2 — this project plans on loops

- Orient first: `tend2 next docs/tend2` (or the `loop_next` MCP tool) — next up, running now, needs-you, gone stale.
- Nothing gets built that is not a loop first: shape the goal and its checks on the map before any code.
- Only `tend2 verify` writes a pass. Never hand-flip a checkbox — a naked [x] renders claimed, not proven.
- Record decisions, scope-outs and dead ends in the loop's `## Tried` — append-only memory for whoever comes next.
- The map holds bets, not maybes: ideas attached to a loop park in its `## Tried`; free-standing ideas park in `docs/ideas/`; shaping is the only transition onto the map.
- Full craft lives in the tend2 plugin skills (next, shape, run, verify, discover, change).
<!-- tend2:end -->

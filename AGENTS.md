# weed, the judge of the diff

weed is a static binary that reads what an agent produced and refuses dishonest growth: deleted or weakened tests, skips, stubs, swallowed errors, secrets, guardrail edits, dependency-direction violations. Milliseconds, zero tokens, every finding as SARIF 2.1.0. Three faces on one core: `weed check` judges a diff and may block; `weed scan` judges the tree and never blocks; `weed guard` is the law in git through hooks. `weed bite` proves a test fails without its change. Agents produce; weed decides.

The map in `docs/tend2/` is the plan and the proof. Read the project loop (`docs/tend2/weed.tend2.html`) and then the loop you are working. The governing documents live in the umbrella workspace and are read-only for this repo: `/Users/jahala/conductor/workspaces/plotplot-ai/cape-town-v1/docs/building-the-garden.md` (the law), `.../docs/tend2/weed.tend2.html` (the fit loop), `.../docs/prompts/weed-build-2026-09.md` (the build brief). Where they disagree, the loop beats the brief and the law beats both.

## Layout

```
src/core/      pure: parse_diff → Hunks · classify_file → kind + lang · change (a changed file with both sides read) · tree (the repository a scan rule reads) · syntax (code vs comment vs literal, per line) · rules/check/<id>.rs and rules/scan/<id>.rs → Vec<Finding> · sarif::render → Log · catalogue (rule ids, defaults, descriptions) · config · suppress · help (a command's own --help, parsed) · registry (manifest pins, the committed snapshot, lag)
src/seams/     I/O behind small functions, injected by faces: git (diff, file at ref, refs, hooks path), exec (a command with a timeout), fs, reader (tilth-core: language detection, outlines, test shape, imports, callers)
src/faces/     the CLI subcommands: check, scan, guard, bite, hook, rules
tests/         integration tests that drive the real `weed` binary on real git repositories in temp dirs (assert_cmd + tempfile); tests/common/ holds the fixture harness
fixtures/adversarial/<RULE>/<lang>/{fire,silent}/{before,after}/   one minimal repo history per rule per language: `before/` is committed as HEAD, `after/` is the working tree; a file `after/.weed-commit` carries the commit message (for trailers) and is never copied. A scan rule judges one state, so its fixture carries `before/` alone and the tree is left as `before/` committed it
schemas/       vendored official schemas (sarif-schema-2.1.0.json)
xtask/         weed's own measurements, the bench and never the product, a workspace member so they judge with the core the binary ships: `cargo xtask calibrate` writes docs/calibration-2026-09.md, `cargo xtask suppressions` counts the allowances each repository wrote, `cargo xtask mutate` is the recall campaign that plants one anti-pattern per case in real commits of the same corpus and writes the recall section of the same file. It reads every tree back after it writes into it, with its own scanner and never with weed's: a case whose shape is not there afterwards is unplantable, counted per rule and language beside the misses, and charged to neither column
scripts/check/ evidence scripts a loop cites; run.sh is the runner tend2 verify uses, and scripts/corpus-scratch.sh fetches the calibration corpus at its pins so the evidence reads the same history the measurement did
scripts/proof/ evidence that starts a real agent session and writes what happened into docs/proof-2026-09.md
docs/          sarif.md · pleach.md · tend2-seam.md · calibration-2026-09.md · proof-2026-09.md · dogfood.md · dogfood/<loop>.md · tend2/ (the map) · calibration/ (corpus.toml pins every repository the calibration judges by source and full sha, corpus-go.toml pins the Go history the recall campaign adds to it, judgements.toml classifies each block, first-run.toml is what the first run refused and is history rather than output)
examples/      pleach/plan.json · ci/github.yml
garden.json    the manifest the umbrella reads (F1); schemas/garden.schema.json is its vendored, flagged schema
SKILL.md       the whole binary in one file for an agent; its body must name every subcommand and flag `weed --help` prints
.brand/products/weed/   identity, colours (three measured accent candidates, flagged), voice, the mark in paper and soil-night
.github/workflows/      ci.yml (fmt, clippy, test, and the suite again on the release profile for the latency budget) · release.yml (version check, platform matrix, crates.io, npm)
npm/           the wrapper: install.js fetches the release binary, run.js proxies to it
LICENSE        MIT, naming the owner, flagged until confirmed · README.md carries the garden footer
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
- Exit codes: 0 clean or warnings only · 2 at least one block-level result · 3 weed could not run (fail closed; the message says why). `scan` exits 0 or 3 only. A panic is exit 3 too: `main.rs` installs a hook that turns any panic into one line naming it as weed's bug. The hook is a last resort and never a licence, core still returns its failures, and a panic the hook catches is a bug to fix.
- The same diff, tree and config write the same bytes, in any locale, time zone or process. `sarif::render` sorts results by file, line, rule id and message before it builds the log; nothing in `src/core/` iterates a hash container into output, and the ones that exist say so beside themselves; weed writes no timestamp. `tests/determinism.rs` and `tests/sarif_order.rs` hold this up.
- The git seam reads a diff, a file at a ref and a commit message as lossy text: a repository with one latin-1 file in it must still be judged, and a rule matches ascii shapes, which a replacement character cannot hide. Every other question weed asks git has a sha, a ref or a setting for an answer, and those are read strictly.
- Only unambiguous rules block by default: T1, T3, S1, X1, C1, G1. Everything else warns, and warnings are for the human at the pull request, not for the agent.
- One mebibyte is where a file stops being code weed reads: `core::change::READABLE`. Above it a side is weighed and its lines are read, and the parser is asked nothing, because outlining a blob costs more than every rule in a run together and G2 already reports the file itself. Both faces obey it, and `tests/rule_g2.rs` holds the other half: a credential planted past the cap is still found, so the cap is a cost declined and not a place to hide in.
- A worker's dogfood notes on tend2, pleach or umbel go in `docs/dogfood/<loop-id>.md`, one file per loop, never in `docs/dogfood.md`, which the conductor owns and folds them into; two writers on one file conflict at every landing.
- A worker that is genuinely blocked writes `BLOCKED.md` at the repository root, and the file is not ignored, so it reaches the quarantine branch and the conductor can read why.

## Toolchain

```
cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
cargo xtask mutate                             # the recall campaign, on the calibration corpus
bash scripts/check/run.sh tests/<name>.rs     # one evidence file, the way the verifier runs it
tend2 next docs/tend2                          # where things stand
```

`cargo xtask mutate` reads the corpus calibration judges,
`docs/calibration/corpus.toml`, and walks the window that ends at each entry's
pinned tip, so a commit pushed to a source after the pin plants no case and
moves no number. The garden five write no Go between them, so
`docs/calibration/corpus-go.toml` pins two Go projects the same way and the
campaign reads both files; `--corpus <path>` reads another instead, which is how
the suites measure a history they built themselves. Every source is fetched at
its pin into `weed-recall-corpus` under the temporary directory, or wherever
`WEED_RECALL_CACHE` names, and is only ever read from.

`tilth-core` is a git dependency on `https://github.com/jahala/tilth`, pinned by rev to the commit the tilth agent landed (the tilth-core loop names it). Nothing weed is built from points at a path on this machine; the worktree at `.context/tilth-core` exists only so the evidence script can run the crate's own suite.

<!-- tend2:begin -->
## tend2 — this project plans on loops

- Orient first: `tend2 next docs/tend2` (or the `loop_next` MCP tool) — next up, running now, needs-you, gone stale.
- Nothing gets built that is not a loop first: shape the goal and its checks on the map before any code.
- Only `tend2 verify` writes a pass. Never hand-flip a checkbox — a naked [x] renders claimed, not proven.
- Record decisions, scope-outs and dead ends in the loop's `## Tried` — append-only memory for whoever comes next.
- The map holds bets, not maybes: ideas attached to a loop park in its `## Tried`; free-standing ideas park in `docs/ideas/`; shaping is the only transition onto the map.
- Full craft lives in the tend2 plugin skills (next, shape, run, verify, discover, change).
<!-- tend2:end -->

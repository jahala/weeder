# calibration: weed over real history, 2026-09

weed ships as a gate: over 635 commits of real history in 5 repositories it blocked 25, of which 2 were block-level false positives, 0.31 percent of the commits judged and under the two percent bar, with T1, T2, T3 and S1 all still at block level.

A second party re-graded the classification blind and docs/calibration-audit-2026-09.md records the agreement: blocked commits at 100.0 percent of 20 cases, recall cases at 100.0 percent of 20 cases. That is what took the qualification off this sentence.

## How this was measured

`cargo xtask calibrate` takes the last 200 commits ending at the commit `docs/calibration/corpus.toml` pins each repository at, and judges each one against its first parent with the same `check` face the binary runs: `weed check --base <parent> --strict`, read back as SARIF. A history with fewer commits contributes all of them, and a repository judged on fewer than 50 commits is reported rather than judged on its own share.

The corpus names each repository by a source `git fetch` can read and by the full sha its window ends at. The pin is what a reader can hold this file to: a commit pushed to any of these repositories after the pin falls outside the window and cannot move a number here, and the same corpus judges the same history on a machine that has never seen any of these repositories. Moving a pin is an edit to that file, and the run that follows it is a new measurement.

A merge commit is left out of the window. It carries no change of its own, and the commits it brings are in the same window, so judging it as well would weigh one change twice. A root commit is left out too: this measurement judges commits against their parents, and a root has none.

Nothing is written to the repositories being read. Each pinned commit and everything it reaches is fetched into a scratch repository under the system's temp directory, every checkout and every judgement happens there, and the scratch is removed at the end. A fetch runs `upload-pack` at the source, which hands objects out and takes none in. Where a source is a directory on the machine running the measurement, its refs, HEAD and working tree are fingerprinted before the run and again after it, and a difference stops the run.

A block is classified by whoever ran the calibration, in `docs/calibration/judgements.toml`, one line of reasoning per commit. The line between the three classes is what the finding claims, not how welcome it was: **true positive**, the claim is true and the change really did weaken something; **acceptable**, the claim is true and the change was fine anyway, so the block is friction the rule was designed to create; **false positive**, the claim is not true of this change. A blocked commit nobody has classified counts as a false positive everywhere a number is drawn, so the bar can only be reached by reading the diffs. `scripts/check/calibration-bar.sh` checks the arithmetic and the bar, never the judgement.

## The rules that ran

No `weed.toml` was passed and none was read: every rule ran at the level the catalogue ships it at. The four rules the kill bar names are first.

| Rule | Level in this run | What it finds |
|---|---|---|
| S1 | block | A stub or a TODO reached production code |
| T1 | block | A test was deleted |
| T2 | block | Assertions were dropped from a changed test file |
| T3 | block | A skip or focus marker was added |
| C1 | block | A guardrail file was edited |
| C2 | warn | An ignore file was broadened over source or tests |
| C3 | warn | A workflow was changed |
| D1 | warn | A dependency manifest changed |
| D2 | block | An import crossed a forbidden boundary |
| G1 | block | A conflict marker was committed |
| G2 | warn | A large or a binary file was added |
| M1 | warn | A test mocks the unit under change |
| S2 | warn | An error was swallowed |
| S3 | warn | A debug leftover reached production code |
| T4 | warn | A tolerance or a timeout was widened |
| T5 | warn | Expected values were regenerated |
| T6 | warn | An error assertion was weakened |
| T7 | block | A rename took a test out of the runner |
| X1 | block | A secret-looking string was added |
| X2 | block | A file outside the scope was touched |

No commit in the corpus carried a `weed.toml` of its own, so no repository moved a rule off the level above.

## tilth, 200 commits judged, 13 blocked, 77 warned

The window ends at `f5c0afa97c6666a3d68dcbd965a4db5a44bc0905`, fetched from `https://github.com/jahala/tilth.git`: the last 200 commits reaching it that are not merges, 200 of which have a parent to be judged against.

| Commit | Rules at block level | Classification | Why |
|---|---|---|---|
| `11aef933c9` feat(list): consolidate tilth_files into tilth_list with directory-tree rendering | T1 | acceptable | src/mcp/tools/files.rs went, and its 8 cases with it; the tool it tested was consolidated into tilth_list in the same commit and the repository's test count rose by one, so the cases moved rather than vanished. A file of tests leaving the tree is worth a person's eye either way. |
| `10bec56a41` test(search): assert only is_ok in no_scope_no_root_defaults_to_cwd — the substring assertion flakes when search surfaces its own source text (resolve_scope behavior is pinned in mod.rs unit tests) | T2 | true positive | the commit says what it did: the substring assertion in no_scope_no_root_defaults_to_cwd was dropped because it flaked, leaving a case that asserts only is_ok. That is a test that now passes whatever search returns. |
| `96cd4b383b` fix(write): containment guard scope_root defaults to root; root-only writes succeed | T2 | true positive | a fix to the write containment guard also took an assert out of src/mcp/tools/search.rs, 3 assertions down to 2. Production behaviour changed and a check on it went in the same breath, which is the shape T2 exists for. |
| `7684e99e86` fix(budget): adapt regression test to upstream API surface | T2 | true positive | the regression test was adapted to the upstream API: the expect that demanded truncation and the assertion on where the cut landed both went, 10 assertions down to 9. The test was moved to fit the code. |
| `ab7f054b73` refactor(mcp): split tilth_read paths-only into its own PR | T1, T2 | true positive | tilth_read_schema_is_paths_only was deleted with its three asserts when the feature was split into another pull request, and the repository's test count fell by one. The schema it pinned is now held by nobody. |
| `136e246d8e` feat(mcp): tilth_write — hash/overwrite/append modes (supersedes #124) | T1 | acceptable | src/mcp/tools/edit.rs and its one case were replaced by tilth_write in the same commit, and the repository gained 19 tests. The deletion is real and the coverage went up. |
| `18eb6643ff` refactor(mcp): tighten module visibility and co-locate tests | T1, T2 | acceptable | 10 cases and 21 assertions left src/mcp/mod.rs to sit beside the modules they test, which is what the commit set out to do; the repository's test count did not move. |
| `76d5d02d34` refactor(mcp): split server into modules | T1 | acceptable | src/mcp.rs was split into modules, so the file and its 17 cases are gone from that path and present at the new ones; the repository's test count did not move. weed judges a file at a time and cannot follow a split, so it reports one and a person reads it. |
| `3ff87caf55` refactor(bloom): adopt fastbloom for BloomFilter implementation | T1, T2 | acceptable | the hand-rolled bloom filter was replaced by fastbloom, and test_bloom_filter_sizing went with the implementation it sized. The behaviour the case held is no longer in the tree. |
| `59c87110ab` refactor(mcp): adopt percent-encoding crate for file:// URI decoding | T1, T2 | acceptable | percent_decode was replaced by the percent-encoding crate and percent_decode_basic went with it. The case tested code this commit deleted. |
| `d422a325fc` refactor(install): fold entry style into ConfigFormat as JsonLocal variant | T2 | acceptable | one assert out of 66 went because the field it read, entry_style, was folded into ConfigFormat; the two cases that carried it were renamed and kept. The property is now held by the type. |
| `5a4edbf5c5` search: extract bloom_walk, callee_query, scope from relational queries | T1, T2 | acceptable | bloom_walk, callee_query and scope were extracted into their own modules and their cases followed: 15 cases left callers.rs and callees.rs, and the repository gained four. The move is real and so is the report. |
| `bd36a43637` index: drop inert SymbolIndex plumbing | T1 | acceptable | SymbolIndex and its six tests were dropped together as inert plumbing. Six cases leaving the tree is exactly the change a person should have to confirm was meant. |

## pleach, 152 commits judged, 1 blocked, 21 warned

The window ends at `49c9177c01bef7b51aa74c05ef5a1e514b3a93f9`, fetched from `https://github.com/jahala/pleach.git`: the last 153 commits reaching it that are not merges, 152 of which have a parent to be judged against.

| Commit | Rules at block level | Classification | Why |
|---|---|---|---|
| `624529b3a9` feat(loop): the hygiene gate — empty-diff, secrets, deletion tripwire (§E) | X1 | acceptable | the hygiene gate lands its own secret patterns and the fixtures that exercise them, including the access key id AWS prints in its own documentation. A credential-shaped literal really was added and no scanner can tell a fixture from a live key, so the block is the rule working and the remedy is an allowance; X1 has since learned that one string as a published example and reports it as a note. The finding on hygiene.ts:27 is weaker: that line is the regular expression describing a private key block, not a key. |

## tend2, 200 commits judged, 6 blocked, 22 warned

The window ends at `51035a5c827b1a8c2f49049f07487dcfac9036c6`, fetched from `https://github.com/jahala/tend.git`: the last 200 commits reaching it that are not merges, 200 of which have a parent to be judged against.

| Commit | Rules at block level | Classification | Why |
|---|---|---|---|
| `067730ddd0` polish(audit): the obsolete-and-theater sweep — seven findings, all fixed | T2 | true positive | the byte comparison of dist/loop.js against the built bundle was dropped in the sweep, leaving only the banner check: 5 assertions down to 4. A freshness guard that no longer compares bytes cannot catch a stale commit, which is what the file's own comment says it is for. |
| `3adc642ce1` polish(root): no asset folder in the root — maps carry their own renderer | T1 | false positive | the file declares two `it(` call sites before and after; one of them now sits inside a `for` over the sibling directories, so it runs once per directory. The reader counts statically declared cases and missed the generated one, so the claim that a case disappeared is not true. |
| `bf2754689c` polish(cli): one orientation command, one asset convention (#102) | T1 | acceptable | test/season.test.ts and its four cases went when the season command was folded into one orientation command; the repository gained three cases. The command those tests covered is not there to be tested. |
| `08529d5f56` sunset(v1): the legacy lane leaves the tree — @plotplot/tend2 ships tend2 only | T1 | acceptable | the v1 lane was sunset: 70 test files and some 2,400 cases left the tree in one commit, along with the workflows that ran them. A change of that size is the strongest case there is for a gate that stops and asks. |
| `78fed73b0c` cleanup: bare necessities — killed-spike tree, skein prototype, root screenshots, v1-map parkland removed (all in git history; paid facet cache untouched on disk); superseded docs to docs/archive; docs/bridge restored after its drift-guard caught the move (load-bearing spec, not history) | T1 | acceptable | the killed spike tree and the skein prototype were removed with their 500 cases, and the workflow that built them changed with it. The deletion is deliberate and it is exactly what a person should sign. |
| `52b97e43ce` feat(loop-hole): refusal hygiene — sanitized one-line reasons + child stderr (rate limits read as rate limits); .loop-scratch gitignored | X1 | false positive | both hits are `class="detail-meta__key">` and `class="journey-row__key">` in the renderer's html builder. The value is markup, not a credential, and X1 took the class attribute for an assignment. |

## copeca, 25 commits judged, 1 blocked, 7 warned

The window ends at `fc9c5b9e5f34085755c10746f1d46fd46edf2945`, fetched from `https://github.com/jahala/copeca.git`: the last 26 commits reaching it that are not merges, 25 of which have a parent to be judged against. Fewer than 50 commits were judged, so this repository is reported and not judged on its own share.

| Commit | Rules at block level | Classification | Why |
|---|---|---|---|
| `70d669542a` Audit remediation, corpus 16→52, multi-CLI runners + OSS publishing prep | X1 | acceptable | the commit lands publishing workflows, which is what C1 watches and what would have blocked it on its own. |

## umbel, 58 commits judged, 4 blocked, 13 warned

The window ends at `b2e79e9d80cc4064bcef58bf7f1a818cce91ef5d`, fetched from `https://github.com/jahala/umbel.git`: the last 59 commits reaching it that are not merges, 58 of which have a parent to be judged against.

| Commit | Rules at block level | Classification | Why |
|---|---|---|---|
| `89c088bc08` fix(test): stop reaping every tmux session on the machine (#58) | T2 | acceptable | the test stopped running `tmux kill-server`, which was reaping every session on the machine, and reaches the same catch path through an empty socket directory instead; one assertion of eighteen went with the teardown it no longer does. |
| `059b4c4677` fix(codex): deliver hooks via global CODEX_HOME so worktree workers fire Stop (#34) | T2 | acceptable | the file was refactored onto a `launch()` helper: 33 assertions down to 30 while the cases went from 22 to 23. The assertions that went were the repeated construction the helper now does once. |
| `75523a16a3` feat(opencode): add OpenCode provider (bring-any-model lane) (#18) | S1 | true positive | `TODO(opencode): verify tool-call part shape against a real tool-using transcript` shipped in src/core/providers/opencode.ts. The parser's field extraction is admitted to be unverified in the file that does it. |
| `cc3cc0c37c` feat(mcp): agent-context Phase 1 — actions/diff/read-truncate (#9) | S1 | true positive | two work markers shipped in the codex and gemini providers, each saying the tool-call shape has not been checked against a real transcript. That is unfinished work in production code, named by the author. |

## Totals

| Repo | Commits judged | Blocked | Warned | True positive | Acceptable | False positive | False-positive share |
|---|---|---|---|---|---|---|---|
| tilth | 200 | 13 | 77 | 4 | 9 | 0 | 0.00% |
| pleach | 152 | 1 | 21 | 0 | 1 | 0 | 0.00% |
| tend2 | 200 | 6 | 22 | 1 | 3 | 2 | 1.00% |
| copeca | 25 | 1 | 7 | 0 | 1 | 0 | 0.00% |
| umbel | 58 | 4 | 13 | 2 | 2 | 0 | 0.00% |
| **pooled** | **635** | **25** | **140** | **7** | **16** | **2** | **0.31%** |

The bar is a pooled block-level false-positive share under 2 percent of the commits judged. This run is at 0.31 percent of 635 commits, and 0 commits weed could not judge.

Beside that share, what the rules moved. The first run refused 332 findings at block level on these commits, recorded in `docs/calibration/first-run.toml`. This run reports 204 of them at block level still, 69 at warn level, 1 at note level and 58 not at all. Rule by rule: 69 moved from C1 at block level to C3 at warn level, all of them on workflow files.

Too few commits to carry a share of their own, reported and not judged alone: copeca (25 commits, 0.00 percent). Their commits and their false positives are both in the pooled total.

## What the rules moved since the first run

A rule that splits in two is supposed to take friction off the gate while keeping the finding. Until the same files are read twice that is a claim. Every file whose answer changed is here with the rule that refused it, the kind weed gives the file, and the loudest thing weed says about it now.

199 of the first run's block-level findings are refused by the same rule at the same level and are left out of this table; the 133 whose answer changed are all in it.

| Repo | Commit | File | Kind | Blocked then by | Says now | At |
|---|---|---|---|---|---|---|
| copeca | `1abdc130df` | `.claude/skills/tend-brainstorm/SKILL.md` | other | X1 | nothing | nothing |
| copeca | `70d669542a` | `.github/workflows/ci.yml` | workflow | C1 | C3 | warn |
| copeca | `70d669542a` | `.github/workflows/dependency-review.yml` | workflow | C1 | C3 | warn |
| copeca | `70d669542a` | `.github/workflows/release.yml` | workflow | C1 | C3 | warn |
| copeca | `70d669542a` | `.github/workflows/scorecard.yml` | workflow | C1 | C3 | warn |
| copeca | `e8584ab6af` | `docs/tend/features/accuracy-blind-claims.tend.html` | other | X1 | nothing | nothing |
| copeca | `e8584ab6af` | `docs/tend/features/contamination-erodes-trust.tend.html` | other | X1 | nothing | nothing |
| copeca | `e8584ab6af` | `docs/tend/features/copeca-analysis-reporting.tend.html` | other | X1 | nothing | nothing |
| copeca | `e8584ab6af` | `docs/tend/features/copeca-artifact-integrity.tend.html` | other | X1 | nothing | nothing |
| copeca | `e8584ab6af` | `docs/tend/features/copeca-cost-model.tend.html` | other | X1 | nothing | nothing |
| copeca | `e8584ab6af` | `docs/tend/features/copeca-docs-and-init.tend.html` | other | X1 | nothing | nothing |
| copeca | `e8584ab6af` | `docs/tend/features/copeca-mode-mechanism.tend.html` | other | X1 | nothing | nothing |
| copeca | `e8584ab6af` | `docs/tend/features/copeca-scenario-matrix.tend.html` | other | X1 | nothing | nothing |
| copeca | `e8584ab6af` | `docs/tend/features/copeca-single-run.tend.html` | other | X1 | nothing | nothing |
| copeca | `e8584ab6af` | `docs/tend/features/copeca-task-corpus.tend.html` | other | X1 | nothing | nothing |
| copeca | `e8584ab6af` | `docs/tend/features/copeca-validate-tasks.tend.html` | other | X1 | nothing | nothing |
| copeca | `e8584ab6af` | `docs/tend/features/integration-only-mcp.tend.html` | other | X1 | nothing | nothing |
| copeca | `e8584ab6af` | `docs/tend/features/no-shared-yardstick.tend.html` | other | X1 | nothing | nothing |
| copeca | `e8584ab6af` | `docs/tend/features/no-verifiable-results.tend.html` | other | X1 | nothing | nothing |
| copeca | `e8584ab6af` | `docs/tend/features/platform-builder.tend.html` | other | X1 | nothing | nothing |
| copeca | `e8584ab6af` | `docs/tend/features/pricing-drift-unchecked.tend.html` | other | X1 | nothing | nothing |
| copeca | `e8584ab6af` | `docs/tend/features/skeptical-evaluator.tend.html` | other | X1 | nothing | nothing |
| copeca | `e8584ab6af` | `docs/tend/features/tool-builder.tend.html` | other | X1 | nothing | nothing |
| copeca | `e8584ab6af` | `docs/tend/overview.html` | other | X1 | nothing | nothing |
| copeca | `eb9ed62b2d` | `docs/tend/features/run-isolation.tend.html` | other | X1 | nothing | nothing |
| copeca | `f6198d4b20` | `src/copeca/runners/parsers/base.py` | prod | S1 | nothing | nothing |
| pleach | `46f9c71acf` | `.github/workflows/ci.yml` | workflow | C1 | C3 | warn |
| pleach | `624529b3a9` | `test/loop/hygiene-gate.test.ts` | test | X1 | X1 | note |
| pleach | `84bcce7b31` | `.github/workflows/canary.yml` | workflow | C1 | C3 | warn |
| pleach | `84bcce7b31` | `.github/workflows/pages.yml` | workflow | C1 | C3 | warn |
| pleach | `b6aadbf3cb` | `.github/workflows/ci.yml` | workflow | C1 | C3 | warn |
| pleach | `bbf44e96f1` | `.claude/skills/tend-brainstorm/SKILL.md` | other | X1 | nothing | nothing |
| pleach | `bbf44e96f1` | `docs/tend/features/agent-output-as-truth.tend.html` | other | X1 | nothing | nothing |
| pleach | `bbf44e96f1` | `docs/tend/features/audit-egress.tend.html` | other | X1 | nothing | nothing |
| pleach | `bbf44e96f1` | `docs/tend/features/cli-run.tend.html` | other | X1 | nothing | nothing |
| pleach | `bbf44e96f1` | `docs/tend/features/cli-validate.tend.html` | other | X1 | nothing | nothing |
| pleach | `bbf44e96f1` | `docs/tend/features/conductor-loop.tend.html` | other | X1 | nothing | nothing |
| pleach | `bbf44e96f1` | `docs/tend/features/developer.tend.html` | other | X1 | nothing | nothing |
| pleach | `bbf44e96f1` | `docs/tend/features/isolate-seam.tend.html` | other | X1 | nothing | nothing |
| pleach | `bbf44e96f1` | `docs/tend/features/lock-journal.tend.html` | other | X1 | nothing | nothing |
| pleach | `bbf44e96f1` | `docs/tend/features/logic-io-entanglement.tend.html` | other | X1 | nothing | nothing |
| pleach | `bbf44e96f1` | `docs/tend/features/operator.tend.html` | other | X1 | nothing | nothing |
| pleach | `bbf44e96f1` | `docs/tend/features/poison-propagation.tend.html` | other | X1 | nothing | nothing |
| pleach | `bbf44e96f1` | `docs/tend/features/rctrl-seam.tend.html` | other | X1 | nothing | nothing |
| pleach | `bbf44e96f1` | `docs/tend/features/silent-contract-drift.tend.html` | other | X1 | nothing | nothing |
| pleach | `bbf44e96f1` | `docs/tend/features/status-artifact-split.tend.html` | other | X1 | nothing | nothing |
| pleach | `bbf44e96f1` | `docs/tend/features/tend-seam.tend.html` | other | X1 | nothing | nothing |
| pleach | `bbf44e96f1` | `docs/tend/overview.html` | other | X1 | nothing | nothing |
| pleach | `c42ed6d92a` | `.github/workflows/ci.yml` | workflow | C1 | C3 | warn |
| pleach | `df42a0bffe` | `.github/workflows/ci.yml` | workflow | C1 | C3 | warn |
| tend2 | `07931a6742` | `.github/workflows/ci.yml` | workflow | C1 | C3 | warn |
| tend2 | `08529d5f56` | `.github/workflows/ci.yml` | workflow | C1 | C3 | warn |
| tend2 | `1e39b91c5f` | `.github/workflows/ci.yml` | workflow | C1 | C3 | warn |
| tend2 | `40875df2c4` | `.github/workflows/ci.yml` | workflow | C1 | C3 | warn |
| tend2 | `78fed73b0c` | `.github/workflows/ci.yml` | workflow | C1 | C3 | warn |
| tend2 | `e6f974bd1c` | `.github/workflows/ci.yml` | workflow | C1 | C3 | warn |
| tend2 | `f0a0adf343` | `.github/workflows/ci.yml` | workflow | C1 | C3 | warn |
| tilth | `088f58ca22` | `.github/workflows/ci.yml` | workflow | C1 | C3 | warn |
| tilth | `088f58ca22` | `.github/workflows/dependency-review.yml` | workflow | C1 | C3 | warn |
| tilth | `088f58ca22` | `.github/workflows/fuzz.yml` | workflow | C1 | C3 | warn |
| tilth | `088f58ca22` | `.github/workflows/release.yml` | workflow | C1 | C3 | warn |
| tilth | `088f58ca22` | `.github/workflows/scorecard.yml` | workflow | C1 | C3 | warn |
| tilth | `08ae11a377` | `.github/workflows/release.yml` | workflow | C1 | C3 | warn |
| tilth | `0c1132c2cc` | `.github/workflows/dependency-review.yml` | workflow | C1 | C3 | warn |
| tilth | `1248c0e594` | `.github/workflows/ci.yml` | workflow | C1 | C3 | warn |
| tilth | `1248c0e594` | `.github/workflows/dependency-review.yml` | workflow | C1 | C3 | warn |
| tilth | `1248c0e594` | `.github/workflows/release.yml` | workflow | C1 | C3 | warn |
| tilth | `1248c0e594` | `.github/workflows/scorecard.yml` | workflow | C1 | C3 | warn |
| tilth | `18eb6643ff` | `src/mcp/mod.rs` | prod | T2 | T1 | block |
| tilth | `1dfc5d26ba` | `.github/workflows/ci.yml` | workflow | C1 | C3 | warn |
| tilth | `1dfc5d26ba` | `.github/workflows/scorecard.yml` | workflow | C1 | C3 | warn |
| tilth | `2dbcbc6eb4` | `.github/workflows/scorecard.yml` | workflow | C1 | C3 | warn |
| tilth | `2ed93282a1` | `.github/workflows/release.yml` | workflow | C1 | C3 | warn |
| tilth | `3ff87caf55` | `src/index/bloom.rs` | prod | T2 | T1 | block |
| tilth | `4f05f0093d` | `.github/workflows/ci.yml` | workflow | C1 | C3 | warn |
| tilth | `53909f3423` | `.github/workflows/scorecard.yml` | workflow | C1 | C3 | warn |
| tilth | `552bc2a0fe` | `.github/workflows/ci.yml` | workflow | C1 | C3 | warn |
| tilth | `552bc2a0fe` | `.github/workflows/dependency-review.yml` | workflow | C1 | C3 | warn |
| tilth | `552bc2a0fe` | `.github/workflows/release.yml` | workflow | C1 | C3 | warn |
| tilth | `552bc2a0fe` | `.github/workflows/scorecard.yml` | workflow | C1 | C3 | warn |
| tilth | `59c87110ab` | `src/mcp.rs` | prod | T2 | T1 | block |
| tilth | `5a4edbf5c5` | `src/search/callers.rs` | prod | T2 | T1 | block |
| tilth | `6c75ff025f` | `.github/workflows/scorecard.yml` | workflow | C1 | C3 | warn |
| tilth | `8da08f6e38` | `.github/workflows/release.yml` | workflow | C1 | C3 | warn |
| tilth | `8da08f6e38` | `.github/workflows/scorecard.yml` | workflow | C1 | C3 | warn |
| tilth | `91d213abdc` | `.github/workflows/release.yml` | workflow | C1 | C3 | warn |
| tilth | `ab7f054b73` | `src/mcp/tools/definitions.rs` | prod | T2 | T1 | block |
| tilth | `b0f3ebe707` | `.github/workflows/release.yml` | workflow | C1 | C3 | warn |
| tilth | `be3f6fbf34` | `.github/workflows/ci.yml` | workflow | C1 | C3 | warn |
| tilth | `c7c0ec78d4` | `.github/workflows/scorecard.yml` | workflow | C1 | C3 | warn |
| tilth | `d322306ab0` | `.github/workflows/fuzz.yml` | workflow | C1 | C3 | warn |
| tilth | `e05749fd21` | `.github/workflows/scorecard.yml` | workflow | C1 | C3 | warn |
| tilth | `e9155c1915` | `.github/workflows/release.yml` | workflow | C1 | C3 | warn |
| tilth | `f2e64e5759` | `.github/workflows/release.yml` | workflow | C1 | C3 | warn |
| tilth | `f761368c94` | `.github/workflows/ci.yml` | workflow | C1 | C3 | warn |
| tilth | `f761368c94` | `.github/workflows/dependency-review.yml` | workflow | C1 | C3 | warn |
| tilth | `f761368c94` | `.github/workflows/release.yml` | workflow | C1 | C3 | warn |
| tilth | `f761368c94` | `.github/workflows/scorecard.yml` | workflow | C1 | C3 | warn |
| tilth | `f8f701ab57` | `.github/workflows/fuzz.yml` | workflow | C1 | C3 | warn |
| tilth | `fc36a2fbed` | `.github/workflows/release.yml` | workflow | C1 | C3 | warn |
| umbel | `4167b85477` | `.claude/skills/tend-brainstorm/SKILL.md` | other | X1 | nothing | nothing |
| umbel | `4167b85477` | `docs/tend/features/cli-face.tend.html` | other | X1 | nothing | nothing |
| umbel | `4167b85477` | `docs/tend/features/completion-detection.tend.html` | other | X1 | nothing | nothing |
| umbel | `4167b85477` | `docs/tend/features/context-bloat.tend.html` | other | X1 | nothing | nothing |
| umbel | `4167b85477` | `docs/tend/features/cross-provider-gap.tend.html` | other | X1 | nothing | nothing |
| umbel | `4167b85477` | `docs/tend/features/dispatch-and-read.tend.html` | other | X1 | nothing | nothing |
| umbel | `4167b85477` | `docs/tend/features/host-agent.tend.html` | other | X1 | nothing | nothing |
| umbel | `4167b85477` | `docs/tend/features/mcp-face.tend.html` | other | X1 | nothing | nothing |
| umbel | `4167b85477` | `docs/tend/features/provider-abstraction.tend.html` | other | X1 | nothing | nothing |
| umbel | `4167b85477` | `docs/tend/features/silent-workers.tend.html` | other | X1 | nothing | nothing |
| umbel | `4167b85477` | `docs/tend/features/supervisor-dev.tend.html` | other | X1 | nothing | nothing |
| umbel | `4167b85477` | `docs/tend/features/worker-lifecycle.tend.html` | other | X1 | nothing | nothing |
| umbel | `4167b85477` | `docs/tend/features/workflow-runner.tend.html` | other | X1 | nothing | nothing |
| umbel | `4167b85477` | `docs/tend/overview.html` | other | X1 | nothing | nothing |
| umbel | `68ceb8f537` | `.github/workflows/scorecard.yml` | workflow | C1 | C3 | warn |
| umbel | `7109a41bd9` | `.github/workflows/release.yml` | workflow | C1 | C3 | warn |
| umbel | `8f75fcb093` | `docs/tend/features/context-bloat.tend.html` | other | X1 | nothing | nothing |
| umbel | `8f75fcb093` | `docs/tend/features/cross-provider-gap.tend.html` | other | X1 | nothing | nothing |
| umbel | `8f75fcb093` | `docs/tend/features/host-agent.tend.html` | other | X1 | nothing | nothing |
| umbel | `8f75fcb093` | `docs/tend/features/silent-workers.tend.html` | other | X1 | nothing | nothing |
| umbel | `8f75fcb093` | `docs/tend/features/supervisor-dev.tend.html` | other | X1 | nothing | nothing |
| umbel | `a145ebe3d1` | `.github/workflows/ci.yml` | workflow | C1 | C3 | warn |
| umbel | `a145ebe3d1` | `.github/workflows/dependency-review.yml` | workflow | C1 | C3 | warn |
| umbel | `a145ebe3d1` | `.github/workflows/release.yml` | workflow | C1 | C3 | warn |
| umbel | `a145ebe3d1` | `.github/workflows/scorecard.yml` | workflow | C1 | C3 | warn |
| umbel | `d305518136` | `.github/workflows/scorecard.yml` | workflow | C1 | C3 | warn |
| umbel | `e39da470b7` | `.github/workflows/ci.yml` | workflow | C1 | C3 | warn |
| umbel | `e39da470b7` | `.github/workflows/dependency-review.yml` | workflow | C1 | C3 | warn |
| umbel | `e39da470b7` | `.github/workflows/release.yml` | workflow | C1 | C3 | warn |
| umbel | `e39da470b7` | `.github/workflows/scorecard.yml` | workflow | C1 | C3 | warn |
| umbel | `e4483b814c` | `.github/workflows/ci.yml` | workflow | C1 | C3 | warn |
| umbel | `e4483b814c` | `.github/workflows/release.yml` | workflow | C1 | C3 | warn |
| umbel | `e4483b814c` | `.github/workflows/scorecard.yml` | workflow | C1 | C3 | warn |

## Where weed was wrong

Every block whose claim was not true of the change, under the rule that made it. This is the list the rule loops work from.

**T1**, one block

- tend2 `3adc642ce1` polish(root): no asset folder in the root — maps carry their own renderer, the file declares two `it(` call sites before and after; one of them now sits inside a `for` over the sibling directories, so it runs once per directory. The reader counts statically declared cases and missed the generated one, so the claim that a case disappeared is not true.

**X1**, 2 blocks

- tend2 `52b97e43ce` feat(loop-hole): refusal hygiene — sanitized one-line reasons + child stderr (rate limits read as rate limits); .loop-scratch gitignored, both hits are `class="detail-meta__key">` and `class="journey-row__key">` in the renderer's html builder. The value is markup, not a credential, and X1 took the class attribute for an assignment.
- copeca `70d669542a` Audit remediation, corpus 16→52, multi-CLI runners + OSS publishing prep, the hit is `key = serialization.load_pem_public_key(pem)` in the signing module: a local variable named key holding the result of a call, not a credential.

## Precision and the allowance rate

Precision is the share of blocks that were not false positives. The allowance rate beside it counts `Weed-allow:` trailers per hundred commits from the day the repository installed guard, and is zero before that day because there was no gate to allow anything past. Allowances rising while true positives stay flat is a gate being routed around rather than obeyed.

| Repo | Block-level precision | True positives | Allowance rate |
|---|---|---|---|
| tilth | 100.0% | 4 | 0.0 per 100 commits (guard not installed) |
| pleach | 100.0% | 0 | 0.0 per 100 commits (guard not installed) |
| tend2 | 66.7% | 1 | 0.0 per 100 commits (guard not installed) |
| copeca | 100.0% | 0 | 0.0 per 100 commits (guard not installed) |
| umbel | 100.0% | 2 | 0.0 per 100 commits (guard not installed) |

No repository wrote an allowance before it installed guard.

## The rules that blocked

A commit that fired two rules is counted once under each. Where one rule of a block deserves a different answer from the commit as a whole, the ledger says so and this table follows it.

| Rule | Blocks | True positive | Acceptable | False positive | False-positive share of its blocks |
|---|---|---|---|---|---|
| S1 | 2 | 2 | 0 | 0 | 0.00% |
| T1 | 13 | 1 | 11 | 1 | 7.69% |
| T2 | 12 | 5 | 7 | 0 | 0.00% |
| X1 | 3 | 0 | 1 | 2 | 66.67% |


<!-- recall:begin -->

## Recall

Every rule that blocks, T1, T3, S1, X1, C1, G1, catches at least 95 percent of the anti-patterns planted for it, in each of ts, py, rs and go. 2832 cases were planted one anti-pattern at a time in real commits, and 2778 of them were caught on the site they were planted in.

The campaign is `cargo xtask mutate`. For each case it checks out a real commit of a corpus repository, plants one anti-pattern in its tree with a scanner that knows nothing about weed's detectors, and runs `weed check --base <parent> --strict`. A case counts as caught only where the rule fires on the file the shape was planted in, on the lines it was planted on where it has lines. A site the unmutated commit already fires that rule on is passed over, so no hit is inherited from the commit itself.

### The corpus

| Repository | Source | Window ends at | Commits walked | Cases |
|---|---|---|---|---|
| cobra | `https://github.com/spf13/cobra.git` | `adbc8813901bba65827259daa8e22ff94ec1f30e` | 200 | 320 |
| copeca | `https://github.com/jahala/copeca.git` | `fc9c5b9e5f34085755c10746f1d46fd46edf2945` | 15 | 264 |
| hcl | `https://github.com/hashicorp/hcl.git` | `6abbb088cdb82416d1b3d9fcbaab29534133567a` | 200 | 360 |
| pleach | `https://github.com/jahala/pleach.git` | `49c9177c01bef7b51aa74c05ef5a1e514b3a93f9` | 21 | 280 |
| tend2 | `https://github.com/jahala/tend.git` | `51035a5c827b1a8c2f49049f07487dcfac9036c6` | 22 | 280 |
| tilth | `https://github.com/jahala/tilth.git` | `f5c0afa97c6666a3d68dcbd965a4db5a44bc0905` | 191 | 1048 |
| umbel | `https://github.com/jahala/umbel.git` | `b2e79e9d80cc4064bcef58bf7f1a818cce91ef5d` | 34 | 280 |

The garden five are the repositories calibration measures precision on, read from `docs/calibration/corpus.toml` at the commits it pins them at. They carry no Go between them and weed judges Go, so the Go column is measured on two Go projects that `docs/calibration/corpus-go.toml` pins the same way, read exactly as the five are.

Every window ends at the pin. A commit pushed to one of these sources later is outside the walk, so it plants no case and moves no number, and two runs over one corpus write this section byte for byte the same.

Files the injector planted nothing in, counted once for each commit they were read at:

- added by the commit, so nothing in it was there to change: 326
- carrying a NUL byte, which git writes no text diff for: 70
- saying a generator wrote it: 623

### Recall, per rule and language

| Rule | Level | Language | Cases | Hits | Misses | Recall |
|---|---|---|---|---|---|---|
| T1 | block | ts | 42 | 40 | 2 | 95.2% |
| T1 | block | py | 34 | 34 | 0 | 100.0% |
| T1 | block | rs | 40 | 40 | 0 | 100.0% |
| T1 | block | go | 40 | 40 | 0 | 100.0% |
| T2 | block | ts | 42 | 42 | 0 | 100.0% |
| T2 | block | py | 34 | 34 | 0 | 100.0% |
| T2 | block | rs | 40 | 40 | 0 | 100.0% |
| T2 | block | go | 40 | 40 | 0 | 100.0% |
| T3 | block | ts | 42 | 42 | 0 | 100.0% |
| T3 | block | py | 34 | 34 | 0 | 100.0% |
| T3 | block | rs | 40 | 40 | 0 | 100.0% |
| T3 | block | go | 40 | 40 | 0 | 100.0% |
| T4 | warn | ts | 42 | 39 | 3 | 92.9% |
| T4 | warn | py | 14 | 14 | 0 | 100.0% |
| T4 | warn | rs | 40 | 39 | 1 | 97.5% |
| T4 | warn | go | 0 | 0 | 0 | no cases |
| T5 | warn | ts | 42 | 42 | 0 | 100.0% |
| T5 | warn | py | 17 | 17 | 0 | 100.0% |
| T5 | warn | rs | 40 | 40 | 0 | 100.0% |
| T5 | warn | go | 20 | 20 | 0 | 100.0% |
| T6 | warn | ts | 42 | 28 | 14 | 66.7% |
| T6 | warn | py | 14 | 10 | 4 | 71.4% |
| T6 | warn | rs | 40 | 40 | 0 | 100.0% |
| T6 | warn | go | 20 | 20 | 0 | 100.0% |
| T7 | block | ts | 42 | 42 | 0 | 100.0% |
| T7 | block | py | 34 | 34 | 0 | 100.0% |
| T7 | block | rs | 0 | 0 | 0 | no cases |
| T7 | block | go | 40 | 40 | 0 | 100.0% |
| M1 | warn | ts | 42 | 42 | 0 | 100.0% |
| M1 | warn | py | 7 | 7 | 0 | 100.0% |
| M1 | warn | rs | 18 | 18 | 0 | 100.0% |
| M1 | warn | go | 40 | 40 | 0 | 100.0% |
| S1 | block | ts | 42 | 42 | 0 | 100.0% |
| S1 | block | py | 34 | 34 | 0 | 100.0% |
| S1 | block | rs | 40 | 40 | 0 | 100.0% |
| S1 | block | go | 40 | 40 | 0 | 100.0% |
| S2 | warn | ts | 42 | 42 | 0 | 100.0% |
| S2 | warn | py | 34 | 33 | 1 | 97.1% |
| S2 | warn | rs | 40 | 40 | 0 | 100.0% |
| S2 | warn | go | 40 | 40 | 0 | 100.0% |
| S3 | warn | ts | 42 | 42 | 0 | 100.0% |
| S3 | warn | py | 34 | 34 | 0 | 100.0% |
| S3 | warn | rs | 40 | 40 | 0 | 100.0% |
| S3 | warn | go | 40 | 40 | 0 | 100.0% |
| D1 | warn | ts | 42 | 42 | 0 | 100.0% |
| D1 | warn | py | 33 | 13 | 20 | 39.4% |
| D1 | warn | rs | 40 | 40 | 0 | 100.0% |
| D1 | warn | go | 20 | 20 | 0 | 100.0% |
| D2 | block | ts | 42 | 42 | 0 | 100.0% |
| D2 | block | py | 14 | 14 | 0 | 100.0% |
| D2 | block | rs | 40 | 40 | 0 | 100.0% |
| D2 | block | go | 20 | 11 | 9 | 55.0% |
| X1 | block | ts | 42 | 42 | 0 | 100.0% |
| X1 | block | py | 34 | 34 | 0 | 100.0% |
| X1 | block | rs | 40 | 40 | 0 | 100.0% |
| X1 | block | go | 40 | 40 | 0 | 100.0% |
| X2 | block | ts | 42 | 42 | 0 | 100.0% |
| X2 | block | py | 34 | 34 | 0 | 100.0% |
| X2 | block | rs | 40 | 40 | 0 | 100.0% |
| X2 | block | go | 40 | 40 | 0 | 100.0% |
| C1 | block | ts | 42 | 42 | 0 | 100.0% |
| C1 | block | py | 35 | 35 | 0 | 100.0% |
| C1 | block | rs | 40 | 40 | 0 | 100.0% |
| C1 | block | go | 40 | 40 | 0 | 100.0% |
| C2 | warn | ts | 42 | 42 | 0 | 100.0% |
| C2 | warn | py | 32 | 32 | 0 | 100.0% |
| C2 | warn | rs | 40 | 40 | 0 | 100.0% |
| C2 | warn | go | 40 | 40 | 0 | 100.0% |
| C3 | warn | ts | 42 | 42 | 0 | 100.0% |
| C3 | warn | py | 33 | 33 | 0 | 100.0% |
| C3 | warn | rs | 40 | 40 | 0 | 100.0% |
| C3 | warn | go | 40 | 40 | 0 | 100.0% |
| G1 | block | ts | 42 | 42 | 0 | 100.0% |
| G1 | block | py | 34 | 34 | 0 | 100.0% |
| G1 | block | rs | 40 | 40 | 0 | 100.0% |
| G1 | block | go | 40 | 40 | 0 | 100.0% |
| G2 | warn | ts | 42 | 42 | 0 | 100.0% |
| G2 | warn | py | 35 | 35 | 0 | 100.0% |
| G2 | warn | rs | 40 | 40 | 0 | 100.0% |
| G2 | warn | go | 40 | 40 | 0 | 100.0% |

### Every miss

Each of these was planted and not reported. The before and after of every one is kept under the campaign's cache directory, as `cases/<rule>/<language>/<repository>-<commit>`, so a number nobody believes can be replayed by hand. `WEED_RECALL_CACHE` says where that directory is; it sits under the temporary directory otherwise.

- T6 · py · copeca `fc9c5b9e5` · `tests/config/test_loader.py:37`, an error assertion stopped naming the error
- T6 · py · copeca `1b01df97f` · `tests/runners/test_base_runner.py:62`, an error assertion stopped naming the error
- T6 · py · copeca `41cfc63ae` · `tests/config/test_loader.py:37`, an error assertion stopped naming the error
- T6 · py · copeca `70d669542` · `tests/config/test_scenario_loader.py:38`, an error assertion stopped naming the error
- S2 · py · copeca `387932ad5` · `tests/e2e/fake_agent.py:61`, a handler was added that catches and says nothing
- D2 · go · hcl `9466647a1` · `hclwrite/ast_block.go:6`, `hclwrite` was made to import `integrationtest`
- D2 · go · hcl `bd45ab812` · `hclwrite/format.go:6`, `hclwrite` was made to import `integrationtest`
- D2 · go · hcl `e73f21667` · `gohcl/schema.go:6`, `gohcl` was made to import `hcldec`
- D2 · go · hcl `6bf1a67a9` · `gohcl/decode.go:6`, `gohcl` was made to import `hcldec`
- D2 · go · hcl `2efc26623` · `hclwrite/ast_body.go:6`, `hclwrite` was made to import `integrationtest`
- D2 · go · hcl `92f12c4e5` · `hcldec/gob.go:6`, `hcldec` was made to import `hcled`
- D2 · go · hcl `ab1acc486` · `hclwrite/tokens.go:6`, `hclwrite` was made to import `integrationtest`
- D2 · go · hcl `6a91a7547` · `gohcl/types.go:6`, `gohcl` was made to import `hcldec`
- D2 · go · hcl `0268c1604` · `gohcl/types.go:6`, `gohcl` was made to import `hcldec`
- T4 · ts · pleach `3a1306011` · `test/loop/run-work.test.ts:10`, a wait was widened from 1000 to 10000
- T6 · ts · tend2 `51035a5c8` · `test/verify.test.ts:176`, an error assertion stopped naming the error
- T6 · ts · tend2 `32df8ddd1` · `test/verify.test.ts:176`, an error assertion stopped naming the error
- T6 · ts · tend2 `2a0e93933` · `test/verify.test.ts:176`, an error assertion stopped naming the error
- T6 · ts · tend2 `fadcb3808` · `test/verify.test.ts:176`, an error assertion stopped naming the error
- T1 · ts · tend2 `6d9a1cf91` · `test/site-paths.test.ts`, the case `${dir} (${pages.length} pages)` was deleted
- T6 · ts · tend2 `6d9a1cf91` · `test/route.test.ts:143`, an error assertion stopped naming the error
- T1 · ts · tend2 `8d939ec15` · `test/renderer-fresh.test.ts`, the case `it` was deleted
- T6 · ts · tend2 `8d939ec15` · `test/route.test.ts:143`, an error assertion stopped naming the error
- T6 · ts · tend2 `e2adbab97` · `test/verify.test.ts:176`, an error assertion stopped naming the error
- T6 · ts · tend2 `56072cdab` · `test/verify.test.ts:176`, an error assertion stopped naming the error
- T6 · ts · tend2 `044912212` · `test/route.test.ts:143`, an error assertion stopped naming the error
- T6 · ts · tend2 `42c840a73` · `test/verify.test.ts:176`, an error assertion stopped naming the error
- T6 · ts · tend2 `e9e8387e9` · `test/verify.test.ts:176`, an error assertion stopped naming the error
- T6 · ts · tend2 `5b1f7a471` · `test/verify.test.ts:176`, an error assertion stopped naming the error
- T6 · ts · tend2 `ad070a0d4` · `test/verify.test.ts:176`, an error assertion stopped naming the error
- T6 · ts · tend2 `99296a937` · `test/verify.test.ts:176`, an error assertion stopped naming the error
- D1 · py · tilth `f5c0afa97` · `benchmark/fixtures/setup.py`, a dependency pin was moved
- D1 · py · tilth `ec06323c7` · `benchmark/fixtures/setup.py`, a dependency pin was moved
- D1 · py · tilth `5d3d16ce5` · `benchmark/fixtures/setup.py`, a dependency pin was moved
- D1 · py · tilth `1f3c7565c` · `benchmark/fixtures/setup.py`, a dependency pin was moved
- D1 · py · tilth `c05475fd4` · `benchmark/fixtures/setup.py`, a dependency pin was moved
- D1 · py · tilth `752cb8925` · `benchmark/fixtures/setup.py`, a dependency pin was moved
- D1 · py · tilth `35dcaec21` · `benchmark/fixtures/setup.py`, a dependency pin was moved
- D1 · py · tilth `49ccc86e9` · `benchmark/fixtures/setup.py`, a dependency pin was moved
- D1 · py · tilth `95b0189c7` · `benchmark/fixtures/setup.py`, a dependency pin was moved
- D1 · py · tilth `aca137834` · `benchmark/fixtures/setup.py`, a dependency pin was moved
- T4 · rs · tilth `5b0539e6a` · `src/mcp/write.rs:148`, a wait was widened from 1 to 10
- D1 · py · tilth `5b0539e6a` · `benchmark/fixtures/setup.py`, a dependency pin was moved
- D1 · py · tilth `bd73c8223` · `benchmark/fixtures/setup.py`, a dependency pin was moved
- D1 · py · tilth `5fa0c8611` · `benchmark/fixtures/setup.py`, a dependency pin was moved
- D1 · py · tilth `ad9eb2cdb` · `benchmark/fixtures/setup.py`, a dependency pin was moved
- D1 · py · tilth `e7ef4647e` · `benchmark/fixtures/setup.py`, a dependency pin was moved
- D1 · py · tilth `8825050ec` · `benchmark/fixtures/setup.py`, a dependency pin was moved
- D1 · py · tilth `9f0d8a289` · `benchmark/fixtures/setup.py`, a dependency pin was moved
- D1 · py · tilth `5ff2929b3` · `benchmark/fixtures/setup.py`, a dependency pin was moved
- D1 · py · tilth `b4d96c68e` · `benchmark/fixtures/setup.py`, a dependency pin was moved
- D1 · py · tilth `9ebb4c65f` · `benchmark/fixtures/setup.py`, a dependency pin was moved
- T4 · ts · umbel `ea995c590` · `test/unit/errors.test.ts:61`, a wait was widened from 5000 to 50000
- T4 · ts · umbel `e4f19b13d` · `test/unit/errors.test.ts:61`, a wait was widened from 5000 to 50000

### Where a shape had nowhere to go

- T4 · go, no commit of the corpus held a site for this shape: 400 commits offered none, and 0 more were passed over because the commit itself already fires the rule there
- T7 · rs, the language's runner does not collect by name, so no rename takes a case out of the run

<!-- recall:end -->

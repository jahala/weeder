# calibration: weed over real history, 2026-09

weed ships as a gate: over 635 commits of real history in 5 repositories it blocked 71, of which 9 were block-level false positives, 1.42 percent of the commits judged and under the two percent bar, with T1, T2, T3 and S1 all still at block level.

## How this was measured

`cargo xtask calibrate` takes the last 200 commits of each repository's default branch, the branch its upstream names, and judges each one against its first parent with the same `check` face the binary runs: `weed check --base <parent> --strict`, read back as SARIF. A branch with fewer commits contributes all of them, and a repository judged on fewer than 50 commits is reported rather than judged on its own share.

A merge commit is left out of the window. It carries no change of its own, and the commits it brings are in the same window, so judging it as well would weigh one change twice. A root commit is left out too: this measurement judges commits against their parents, and a root has none.

Nothing is written to the repositories being read. Each one's default branch is fetched into a scratch repository under the system's temp directory, every checkout and every judgement happens there, and the scratch is removed at the end. Each repository's refs, HEAD and working tree are fingerprinted before the run and again after it, and a difference stops the run.

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

## tilth, 200 commits judged, 36 blocked, 54 warned

The window is `refs/remotes/origin/main`: the last 200 commits of it that are not merges, 200 of which have a parent to be judged against.

| Commit | Rules at block level | Classification | Why |
|---|---|---|---|
| `2ed93282a1` chore(release): 0.10.1, fix the npm publish leg | C1 | acceptable | the change edits `.github/workflows/release.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `f2e64e5759` ci(release): stop quadruplicating release notes (closes #189) | C1 | acceptable | the change edits `.github/workflows/release.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `b0f3ebe707` chore(deps): bump actions/setup-node from 6.4.0 to 7.0.0 | C1 | acceptable | the change edits `.github/workflows/release.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `53909f3423` chore(deps): bump github/codeql-action/upload-sarif | C1 | acceptable | the change edits `.github/workflows/scorecard.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `4f05f0093d` ci: lint test targets with clippy --all-targets | C1 | acceptable | the change edits `.github/workflows/ci.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `2dbcbc6eb4` chore(deps): bump github/codeql-action/upload-sarif | C1 | acceptable | the change edits `.github/workflows/scorecard.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `11aef933c9` feat(list): consolidate tilth_files into tilth_list with directory-tree rendering | T1 | acceptable | src/mcp/tools/files.rs went, and its 8 cases with it; the tool it tested was consolidated into tilth_list in the same commit and the repository's test count rose by one, so the cases moved rather than vanished. A file of tests leaving the tree is worth a person's eye either way. |
| `10bec56a41` test(search): assert only is_ok in no_scope_no_root_defaults_to_cwd, the substring assertion flakes when search surfaces its own source text (resolve_scope behavior is pinned in mod.rs unit tests) | T2 | true positive | the commit says what it did: the substring assertion in no_scope_no_root_defaults_to_cwd was dropped because it flaked, leaving a case that asserts only is_ok. That is a test that now passes whatever search returns. |
| `96cd4b383b` fix(write): containment guard scope_root defaults to root; root-only writes succeed | T2 | true positive | a fix to the write containment guard also took an assert out of src/mcp/tools/search.rs, 3 assertions down to 2. Production behaviour changed and a check on it went in the same breath, which is the shape T2 exists for. |
| `d322306ab0` chore(deps): bump actions/cache from 5.0.5 to 6.1.0 | C1 | acceptable | the change edits `.github/workflows/fuzz.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `088f58ca22` chore(deps): bump actions/checkout from 6.0.2 to 7.0.0 | C1 | acceptable | the change edits `.github/workflows/ci.yml`, `.github/workflows/dependency-review.yml`, `.github/workflows/fuzz.yml`, `.github/workflows/release.yml`, `.github/workflows/scorecard.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `8da08f6e38` chore(deps): bump the actions-minor-and-patch group (#152) | C1 | acceptable | the change edits `.github/workflows/release.yml`, `.github/workflows/scorecard.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `e9155c1915` ci(release): dual-publish npm wrapper as @plotplot/tilth, best-effort (#144) | C1 | acceptable | the change edits `.github/workflows/release.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `7684e99e86` fix(budget): adapt regression test to upstream API surface | T2 | true positive | the regression test was adapted to the upstream API: the expect that demanded truncation and the assertion on where the cut landed both went, 10 assertions down to 9. The test was moved to fit the code. |
| `08ae11a377` chore: release v0.9.0 (#142) | C1 | acceptable | the change edits `.github/workflows/release.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `1dfc5d26ba` chore(deps): bump the actions-minor-and-patch group across 1 directory with 2 updates (#128) | C1 | acceptable | the change edits `.github/workflows/ci.yml`, `.github/workflows/scorecard.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `ab7f054b73` refactor(mcp): split tilth_read paths-only into its own PR | T1, T2 | true positive | tilth_read_schema_is_paths_only was deleted with its three asserts when the feature was split into another pull request, and the repository's test count fell by one. The schema it pinned is now held by nobody. |
| `136e246d8e` feat(mcp): tilth_write, hash/overwrite/append modes (supersedes #124) | T1 | acceptable | src/mcp/tools/edit.rs and its one case were replaced by tilth_write in the same commit, and the repository gained 19 tests. The deletion is real and the coverage went up. |
| `be3f6fbf34` refactor(mcp): extract server instructions to prompts/*.md | C1 | acceptable | the change edits `.github/workflows/ci.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `18eb6643ff` refactor(mcp): tighten module visibility and co-locate tests | T1, T2 | acceptable | 10 cases and 21 assertions left src/mcp/mod.rs to sit beside the modules they test, which is what the commit set out to do; the repository's test count did not move. |
| `76d5d02d34` refactor(mcp): split server into modules | T1 | acceptable | src/mcp.rs was split into modules, so the file and its 17 cases are gone from that path and present at the new ones; the repository's test count did not move. weed judges a file at a time and cannot follow a split, so it reports one and a person reads it. |
| `f8f701ab57` feat(fuzz): cargo-fuzz harness + nightly CI + OSS-Fuzz prep (#118) | C1 | acceptable | the change edits `.github/workflows/fuzz.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `f761368c94` chore(ci): pin all GitHub Actions by commit SHA (#116) | C1 | acceptable | the change edits `.github/workflows/ci.yml`, `.github/workflows/dependency-review.yml`, `.github/workflows/release.yml`, `.github/workflows/scorecard.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `552bc2a0fe` chore(deps): bump actions/checkout from 4 to 6 | C1 | acceptable | the change edits `.github/workflows/ci.yml`, `.github/workflows/dependency-review.yml`, `.github/workflows/release.yml`, `.github/workflows/scorecard.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `6c75ff025f` fix(scorecard): pin to v2.4.3, @v2 ref doesn't exist | C1 | acceptable | the change edits `.github/workflows/scorecard.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `fc36a2fbed` chore(deps): bump actions/setup-node from 4 to 6 | C1 | acceptable | the change edits `.github/workflows/release.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `0c1132c2cc` chore(deps): bump actions/dependency-review-action from 4 to 5 | C1 | acceptable | the change edits `.github/workflows/dependency-review.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `e05749fd21` chore(deps): bump actions/upload-artifact from 4 to 7 | C1 | acceptable | the change edits `.github/workflows/scorecard.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `91d213abdc` chore(deps): bump softprops/action-gh-release from 2 to 3 | C1 | acceptable | the change edits `.github/workflows/release.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `c7c0ec78d4` chore(deps): bump github/codeql-action from 3 to 4 | C1 | acceptable | the change edits `.github/workflows/scorecard.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `1248c0e594` chore: oss-hygiene, supply-chain workflows and harden permissions | C1 | acceptable | the change edits `.github/workflows/ci.yml`, `.github/workflows/dependency-review.yml`, `.github/workflows/release.yml`, `.github/workflows/scorecard.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `3ff87caf55` refactor(bloom): adopt fastbloom for BloomFilter implementation | T1, T2 | acceptable | the hand-rolled bloom filter was replaced by fastbloom, and test_bloom_filter_sizing went with the implementation it sized. The behaviour the case held is no longer in the tree. |
| `59c87110ab` refactor(mcp): adopt percent-encoding crate for file:// URI decoding | T1, T2 | acceptable | percent_decode was replaced by the percent-encoding crate and percent_decode_basic went with it. The case tested code this commit deleted. |
| `d422a325fc` refactor(install): fold entry style into ConfigFormat as JsonLocal variant | T2 | acceptable | one assert out of 66 went because the field it read, entry_style, was folded into ConfigFormat; the two cases that carried it were renamed and kept. The property is now held by the type. |
| `5a4edbf5c5` search: extract bloom_walk, callee_query, scope from relational queries | T1, T2 | acceptable | bloom_walk, callee_query and scope were extracted into their own modules and their cases followed: 15 cases left callers.rs and callees.rs, and the repository gained four. The move is real and so is the report. |
| `bd36a43637` index: drop inert SymbolIndex plumbing | T1 | acceptable | SymbolIndex and its six tests were dropped together as inert plumbing. Six cases leaving the tree is exactly the change a person should have to confirm was meant. |

## pleach, 152 commits judged, 7 blocked, 16 warned

The window is `refs/remotes/origin/master`: the last 153 commits of it that are not merges, 152 of which have a parent to be judged against.

| Commit | Rules at block level | Classification | Why |
|---|---|---|---|
| `624529b3a9` feat(loop): the hygiene gate, empty-diff, secrets, deletion tripwire (§E) | X1 | acceptable | the hygiene gate lands its own secret patterns and the fixtures that exercise them, including AWS's documented AKIAIOSFODNN7EXAMPLE. A credential-shaped literal really was added and no scanner can tell a fixture from a live key, so the block is the rule working and the remedy is an allowance. The finding on hygiene.ts:27 is weaker: that line is the regular expression describing a private key block, not a key. |
| `df42a0bffe` ci: brand check becomes a local concern | C1 | acceptable | the change edits `.github/workflows/ci.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `84bcce7b31` feat(ops): canary scaffold, inbox templates, dormant Pages + day-2 positioning (#19-21) | C1 | acceptable | the change edits `.github/workflows/canary.yml`, `.github/workflows/pages.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `b6aadbf3cb` chore: prepare repo for going public (OSS-readiness) | C1 | acceptable | the change edits `.github/workflows/ci.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `bbf44e96f1` docs(tend): self-tracking garden, positioning, wiring, narratives, thumbnails | X1 | false positive | every hit is `class="detail-meta__key">` and `class="journey-row__key">` in generated html, plus two lines of prose in a skill about verification recipes. No credential was added; X1 read a css class name and a markdown sentence as an assignment because the name ends in key, token or credentials. |
| `c42ed6d92a` fix(p0): review, pin floating devDeps, dedupe CI triggers | C1 | acceptable | the change edits `.github/workflows/ci.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `46f9c71acf` chore(p0): skeleton, package.json, tsconfig, biome, CI, gitignore | C1 | acceptable | the change edits `.github/workflows/ci.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |

## tend2, 200 commits judged, 11 blocked, 17 warned

The window is `refs/remotes/origin/master`: the last 200 commits of it that are not merges, 200 of which have a parent to be judged against.

| Commit | Rules at block level | Classification | Why |
|---|---|---|---|
| `1e39b91c5f` gate: CI stops trusting exit-zero, the PR gate re-verifies touched claims | C1 | acceptable | the change edits `.github/workflows/ci.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `067730ddd0` polish(audit): the obsolete-and-theater sweep, seven findings, all fixed | T2 | true positive | the byte comparison of dist/loop.js against the built bundle was dropped in the sweep, leaving only the banner check: 5 assertions down to 4. A freshness guard that no longer compares bytes cannot catch a stale commit, which is what the file's own comment says it is for. |
| `3adc642ce1` polish(root): no asset folder in the root, maps carry their own renderer | T1 | false positive | the file declares two `it(` call sites before and after; one of them now sits inside a `for` over the sibling directories, so it runs once per directory. The reader counts statically declared cases and missed the generated one, so the claim that a case disappeared is not true. |
| `bf2754689c` polish(cli): one orientation command, one asset convention (#102) | T1 | acceptable | test/season.test.ts and its four cases went when the season command was folded into one orientation command; the repository gained three cases. The command those tests covered is not there to be tested. |
| `08529d5f56` sunset(v1): the legacy lane leaves the tree, @plotplot/tend2 ships tend2 only | C1, T1 | acceptable | the v1 lane was sunset: 70 test files and some 2,400 cases left the tree in one commit, along with the workflows that ran them. A change of that size is the strongest case there is for a gate that stops and asks. |
| `f0a0adf343` release(#49): tend2 packaging, the bin, the package, the plugin | C1 | acceptable | the change edits `.github/workflows/ci.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `78fed73b0c` cleanup: bare necessities, killed-spike tree, skein prototype, root screenshots, v1-map parkland removed (all in git history; paid facet cache untouched on disk); superseded docs to docs/archive; docs/bridge restored after its drift-guard caught the move (load-bearing spec, not history) | C1, T1 | acceptable | the killed spike tree and the skein prototype were removed with their 500 cases, and the workflow that built them changed with it. The deletion is deliberate and it is exactly what a person should sign. |
| `40875df2c4` ci: setup bun for the spike pipeline-entry test | C1 | acceptable | the change edits `.github/workflows/ci.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `e6f974bd1c` ci: fetch the sha-pinned embedding model before the spike suite, the model is local-only by design | C1 | acceptable | the change edits `.github/workflows/ci.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `07931a6742` ci: gate the tend2 kernel + spike suites, not just v1, the verification product now CI-verifies itself | C1 | acceptable | the change edits `.github/workflows/ci.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `52b97e43ce` feat(loop-hole): refusal hygiene, sanitized one-line reasons + child stderr (rate limits read as rate limits); .loop-scratch gitignored | X1 | false positive | both hits are `class="detail-meta__key">` and `class="journey-row__key">` in the renderer's html builder. The value is markup, not a credential, and X1 took the class attribute for an assignment. |

## copeca, 25 commits judged, 5 blocked, 5 warned

The window is `refs/remotes/origin/master`: the last 26 commits of it that are not merges, 25 of which have a parent to be judged against. Fewer than 50 commits were judged, so this repository is reported and not judged on its own share.

| Commit | Rules at block level | Classification | Why |
|---|---|---|---|
| `eb9ed62b2d` feat: robust + isolated benchmark runs, run-robustness, cross-CLI clean room, subscription/API dual-auth (#15) | X1 | false positive | the two hits are the rendered `detail-meta__key` and `journey-row__key` spans in a generated feature page. No credential was added. |
| `70d669542a` Audit remediation, corpus 16→52, multi-CLI runners + OSS publishing prep | C1, X1 | acceptable | the commit lands publishing workflows, which is what C1 watches and what would have blocked it on its own. |
| `e8584ab6af` docs: README + architecture, engineering, metrics, methodology, authoring | X1 | false positive | thirty-eight hits, every one of them a `detail-meta__key` or `journey-row__key` span in the generated documentation pages. |
| `f6198d4b20` fix: narrow gitignore, only root results/, not src/copeca/results/ | S1 | false positive | `def parse(...) -> "RunResult": ...` sits in `class Parser(Protocol)`. In Python an ellipsis body is how a protocol declares a method's type; there is no implementation to finish, and reading it as a stub is reading the language wrong. |
| `1abdc130df` feat: project scaffold, copeca benchmark tool | X1 | false positive | both hits are prose in a skill file: an example check reading "User can log in with valid credentials" and one about signJWT returning a token. X1 read the words after the colon as an assigned value. |

## umbel, 58 commits judged, 12 blocked, 7 warned

The window is `refs/remotes/origin/master`: the last 59 commits of it that are not merges, 58 of which have a parent to be judged against.

| Commit | Rules at block level | Classification | Why |
|---|---|---|---|
| `89c088bc08` fix(test): stop reaping every tmux session on the machine (#58) | T2 | acceptable | the test stopped running `tmux kill-server`, which was reaping every session on the machine, and reaches the same catch path through an empty socket directory instead; one assertion of eighteen went with the teardown it no longer does. |
| `8f75fcb093` feat(tend): standalone pages for the personas + opportunities, garden at 0 warnings | X1 | false positive | ten hits, all `detail-meta__key` and `journey-row__key` spans in generated persona pages. No credential was added. |
| `a145ebe3d1` build(deps): Bump the actions-minor-and-patch group across 1 directory with 2 updates (#24) | C1 | acceptable | the change edits `.github/workflows/ci.yml`, `.github/workflows/dependency-review.yml`, `.github/workflows/release.yml`, `.github/workflows/scorecard.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `4167b85477` feat(tend): add the tend feature-map garden (umbel) (#41) | X1 | false positive | the same generated `detail-meta__key` and `journey-row__key` spans across the new garden pages, plus the two lines of skill prose about credentials and tokens. |
| `7109a41bd9` refactor(rename): rctrl → umbel across the repo (#37) | C1 | acceptable | the change edits `.github/workflows/release.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `059b4c4677` fix(codex): deliver hooks via global CODEX_HOME so worktree workers fire Stop (#34) | T2 | acceptable | the file was refactored onto a `launch()` helper: 33 assertions down to 30 while the cases went from 22 to 23. The assertions that went were the repeated construction the helper now does once. |
| `75523a16a3` feat(opencode): add OpenCode provider (bring-any-model lane) (#18) | S1 | true positive | `TODO(opencode): verify tool-call part shape against a real tool-using transcript` shipped in src/core/providers/opencode.ts. The parser's field extraction is admitted to be unverified in the file that does it. |
| `68ceb8f537` feat(cli+wait+ci): actions/diff CLI verbs, wait pane-snapshot, scorecard non-blocking (#15) | C1 | acceptable | the change edits `.github/workflows/scorecard.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `d305518136` fix(ci): add issues/PR/checks read permissions for scorecard (#11) | C1 | acceptable | the change edits `.github/workflows/scorecard.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `cc3cc0c37c` feat(mcp): agent-context Phase 1, actions/diff/read-truncate (#9) | S1 | true positive | two work markers shipped in the codex and gemini providers, each saying the tool-call shape has not been checked against a real transcript. That is unfinished work in production code, named by the author. |
| `e4483b814c` build(deps): Bump the actions-minor-and-patch group with 2 updates (#2) | C1 | acceptable | the change edits `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `.github/workflows/scorecard.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |
| `e39da470b7` rctrl v0.1.0, multi-CLI agent orchestrator over tmux | C1 | acceptable | the change edits `.github/workflows/ci.yml`, `.github/workflows/dependency-review.yml`, `.github/workflows/release.yml`, `.github/workflows/scorecard.yml`, which is what C1 watches; the edit is the one the subject describes and a reviewer reading it is the rule working. |

## Totals

| Repo | Commits judged | Blocked | Warned | True positive | Acceptable | False positive | False-positive share |
|---|---|---|---|---|---|---|---|
| tilth | 200 | 36 | 54 | 4 | 32 | 0 | 0.00% |
| pleach | 152 | 7 | 16 | 0 | 6 | 1 | 0.66% |
| tend2 | 200 | 11 | 17 | 1 | 8 | 2 | 1.00% |
| copeca | 25 | 5 | 5 | 0 | 1 | 4 | 16.00% |
| umbel | 58 | 12 | 7 | 2 | 8 | 2 | 3.45% |
| **pooled** | **635** | **71** | **99** | **7** | **55** | **9** | **1.42%** |

The bar is a pooled block-level false-positive share under 2 percent of the commits judged. This run is at 1.42 percent of 635 commits, and 0 commits weed could not judge.

Read on their own these repositories are over the bar: umbel at 3.45 percent. The bar is pooled, and they are named here rather than left to be found in the table.

Too few commits to carry a share of their own, reported and not judged alone: copeca (25 commits, 16.00 percent). Their commits and their false positives are both in the pooled total.

## Where weed was wrong

Every block whose claim was not true of the change, under the rule that made it. This is the list the rule loops work from.

**S1**, one block

- copeca `f6198d4b20` fix: narrow gitignore, only root results/, not src/copeca/results/, `def parse(...) -> "RunResult": ...` sits in `class Parser(Protocol)`. In Python an ellipsis body is how a protocol declares a method's type; there is no implementation to finish, and reading it as a stub is reading the language wrong.

**T1**, one block

- tend2 `3adc642ce1` polish(root): no asset folder in the root, maps carry their own renderer, the file declares two `it(` call sites before and after; one of them now sits inside a `for` over the sibling directories, so it runs once per directory. The reader counts statically declared cases and missed the generated one, so the claim that a case disappeared is not true.

**X1**, 8 blocks

- pleach `bbf44e96f1` docs(tend): self-tracking garden, positioning, wiring, narratives, thumbnails, every hit is `class="detail-meta__key">` and `class="journey-row__key">` in generated html, plus two lines of prose in a skill about verification recipes. No credential was added; X1 read a css class name and a markdown sentence as an assignment because the name ends in key, token or credentials.
- tend2 `52b97e43ce` feat(loop-hole): refusal hygiene, sanitized one-line reasons + child stderr (rate limits read as rate limits); .loop-scratch gitignored, both hits are `class="detail-meta__key">` and `class="journey-row__key">` in the renderer's html builder. The value is markup, not a credential, and X1 took the class attribute for an assignment.
- copeca `eb9ed62b2d` feat: robust + isolated benchmark runs, run-robustness, cross-CLI clean room, subscription/API dual-auth (#15), the two hits are the rendered `detail-meta__key` and `journey-row__key` spans in a generated feature page. No credential was added.
- copeca `70d669542a` Audit remediation, corpus 16→52, multi-CLI runners + OSS publishing prep, the hit is `key = serialization.load_pem_public_key(pem)` in the signing module: a local variable named key holding the result of a call, not a credential.
- copeca `e8584ab6af` docs: README + architecture, engineering, metrics, methodology, authoring, thirty-eight hits, every one of them a `detail-meta__key` or `journey-row__key` span in the generated documentation pages.
- copeca `1abdc130df` feat: project scaffold, copeca benchmark tool, both hits are prose in a skill file: an example check reading "User can log in with valid credentials" and one about signJWT returning a token. X1 read the words after the colon as an assigned value.
- umbel `8f75fcb093` feat(tend): standalone pages for the personas + opportunities, garden at 0 warnings, ten hits, all `detail-meta__key` and `journey-row__key` spans in generated persona pages. No credential was added.
- umbel `4167b85477` feat(tend): add the tend feature-map garden (umbel) (#41), the same generated `detail-meta__key` and `journey-row__key` spans across the new garden pages, plus the two lines of skill prose about credentials and tokens.

## Precision and the allowance rate

Precision is the share of blocks that were not false positives. The allowance rate beside it counts `Weed-allow:` trailers per hundred commits from the day the repository installed guard, and is zero before that day because there was no gate to allow anything past. Allowances rising while true positives stay flat is a gate being routed around rather than obeyed.

| Repo | Block-level precision | True positives | Allowance rate |
|---|---|---|---|
| tilth | 100.0% | 4 | 0.0 per 100 commits (guard not installed) |
| pleach | 85.7% | 0 | 0.0 per 100 commits (guard not installed) |
| tend2 | 81.8% | 1 | 0.0 per 100 commits (guard not installed) |
| copeca | 20.0% | 0 | 0.0 per 100 commits (guard not installed) |
| umbel | 83.3% | 2 | 0.0 per 100 commits (guard not installed) |

No repository wrote an allowance before it installed guard.

## The rules that blocked

A commit that fired two rules is counted once under each. Where one rule of a block deserves a different answer from the commit as a whole, the ledger says so and this table follows it.

| Rule | Blocks | True positive | Acceptable | False positive | False-positive share of its blocks |
|---|---|---|---|---|---|
| C1 | 42 | 0 | 42 | 0 | 0.00% |
| S1 | 3 | 2 | 0 | 1 | 33.33% |
| T1 | 13 | 1 | 11 | 1 | 7.69% |
| T2 | 12 | 5 | 7 | 0 | 0.00% |
| X1 | 9 | 0 | 1 | 8 | 88.89% |


# Repository audit, 2026-09-23

Scope: the whole repository at `origin/master` (`a37f671`, v0.2.4) merged with the four commits on `work/resume`. Method: every gate run locally, CI and release history read, five dimension reviews, and every finding ranked here as high or critical re-run by hand before it was written down.

## Verdict

weeder is engineered with unusual discipline: 456 tests on real git repositories, byte-for-byte determinism, a calibration measured on pinned history and re-graded blind. It is off track on its central promise, though. A change can switch the gate off: a pull request that deletes a test and adds `weeder.toml` with `T1 = "off"` and `C1 = "off"` passes `weeder check --base main --strict` with exit 0 and no results. That is the CI form the README recommends.

The fail-open family (F1 to F6) shares one root cause. No document says what the adversary controls, so the law, the hooks and git's view of the diff all sit in the trust domain the agent writes to. `docs/architecture.md` §4 now writes that model down. Fixing F1 and F2 comes before anything else, including publishing.

## Scoreboard

| Measure | State |
|---|---|
| `cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings` | clean |
| `cargo test --workspace` | 456 passed, 0 failed, 0 ignored |
| `cargo test --workspace --release` | 455 passed, 0 failed, 0 ignored |
| Skips, `#[ignore]`, tests deleted to go green | none, in any branch (`git log --diff-filter=D -- tests/`) |
| `cargo audit` | 0 advisories over 191 crates |
| History | 284 commits, 2026-09-05 to 2026-09-18; every change after the first commit landed by pull request (23 merges) |
| CI on master, last 13 pushes | 11 green, 2 red: a flaky test (#46) and a version mismatch merged over red checks (#49) |
| Release runs | 7 runs, none fully green. Tarballs ship from v0.2.2 on; `publish-crate` and `publish-npm` have never succeeded |
| Branch protection on `master` | none |
| tend2 map | 20 loops, 132 stamps fresh, 1 stale (`weeder:c4`), 3 waiting on the owner |
| Open issues / PRs | 15 issues; 6 PRs, all dependabot, 4 of them 17 days old |
| Latency | release build: `check --base <root>` on this repository 0.56 s, `scan` 0.29 s |

## Findings, ranked by blast radius

Status: CONFIRMED means reproduced with the built binary, or the enforcing line was read. PLAUSIBLE says what would confirm it.

### Critical: the gate fails open

**F1. A change is judged under its own `weeder.toml`.** CONFIRMED (reproduced by hand in the CI, `--staged` and guard forms).
`src/faces/mod.rs:28-46` reads `weeder.toml` from the working tree for every face and every range. `src/core/rules/mod.rs` drops a rule set to `off`, C1 included.
- CI form: a branch that deletes `tests/add.test.js` and adds `[rules] T1 = "off"`, `C1 = "off"`. `weeder check --base main --strict` exits 0 with 0 results. The same branch without the config exits 2 (T1).
- Guard form: stage a skip and a stub, then leave `[rules] T3 = "off"`, `S1 = "off"` unstaged. `git commit` with the guard installed succeeds. C1 never fires, because `weeder.toml` is not in the staged diff.
- Fix: read the config from the base of the judged range (`HEAD` for the tree and `--staged`, the base commit for `--base` and pre-push). A config change then takes effect after it lands. Record it as a ruling. `--config` stays explicit.
- Failing test first: `tests/check_config_source.rs`, the three forms above, each expecting exit 2.

**F2. An attribute or one NUL byte hides every added line.** CONFIRMED (reproduced).
`src/seams/git.rs:640-650` runs `git diff` without `--text`, and `src/core/diff.rs` turns `Binary files differ` into a change with no hunks.
- `* -diff` in `.git/info/attributes` (untracked, invisible to any diff) turns S1 plus T3 at exit 2 into exit 0 with no findings. A committed `.gitattributes` does the same: it is not a guardrail file (`src/core/classify.rs`).
- A `// \0` comment in a JS file that also holds a stub and an AWS key: exit 0, no findings. Issue #22 files this as a rule candidate; it is a fail-open.
- Fix: pass `--text`, and read a source-language file as lossy text even when it holds a NUL. Add `.gitattributes` to C1's guardrails. Fixture first: a fire case per language carrying `-diff`.

**F3. The hooks that run weeder live where the agent writes.** CONFIRMED (reproduced).
`weeder guard install` writes `.githooks/*` in the working tree (`src/faces/guard.rs:27`). Rewriting the exec line of `.githooks/pre-commit` to a script that exits 0 lets a test deletion commit, and `guard status` still prints "weeder guard is live", because it reads the marker comment rather than the command (`src/core/guard.rs:174`, `src/core/rules/check/c1.rs:43`).
- Fix: install hooks under the git directory and record the binary's absolute path in `.git/weeder/`. `status` and the C1 exemption compare the whole rendered file with what install wrote.

**F4. The harness hook matches `git` by spelling, and `Stop` judges `HEAD` only.** CONFIRMED (reproduced for `env git commit` and `command git commit`; the Stop laundering by the security review).
`src/core/hook.rs:282` recognises a commit only when the program is literally `git`. `env git commit -m x` over a red index gets exit 0. After such a commit, `Stop` (`src/faces/hook.rs:74`, `base: None`) sees a clean tree. `git commit -am` passes `PreToolUse` too, because the index is judged before `-a` stages.
- Fix: record the session's starting commit on the first event and judge `Stop` from it. Treat `PreToolUse` matching as best effort in `SKILL.md`.

**F5. Moving a test into a specimen directory deletes it with a note.** CONFIRMED (reproduced).
`set_aside` in `src/faces/check.rs` tests `file.path()`, which prefers the new path. `git mv tests/add.test.js fixtures/adversarial/x/` under `specimens = ["fixtures/adversarial"]` gives exit 0 and one SPECIMEN note. weeder's own `weeder.toml` sets exactly that.
- Fix: set a file aside only when both its old and new paths are specimens.

**F6. T1 follows a "moved" test case by its name alone.** CONFIRMED (reproduced).
`src/core/rules/check/t1.rs:171-174`. Deleting `it("adds negatives", …)` and adding `it("adds negatives", () => { expect(1).toBe(1); })` in another file: exit 0.
- Fix: a moved case must keep at least its assertion count, or it counts as deleted.

### High

**F7. T2 blocks a test that got better.** CONFIRMED (reproduced), issue #31.
Three `assert` lines turned into one `pytest.mark.parametrize` case over the same three inputs: exit 2, "2 assertions went out of this file". This is the false block the project loop warns about: "a false block costs a retry, and agents learn to add allowances". `src/core/rules/check/t2.rs` counts assertion sites, not cases times assertions.
- Fix: count an assertion inside a parametrized case, or a loop over literal cases, once per case. Fixture per language first.

**F8. Nothing stops a red pull request from merging.** CONFIRMED.
`master` has no branch protection (`gh api …/branches/master/protection` answers 404). PR #49 merged with `check`, `windows` and `latency` failing. `master` stayed red from 2026-09-16 to 2026-09-18, and the v0.2.3 tag was refused by its own version check.
- Fix (owner, on GitHub): require `check`, `windows` and `latency` on `master`.

**F9. Every install route but the tarball is broken, and three files point at them.** CONFIRMED.
`publish-crate` fails because `tilth-core` is a git dependency (#45). `publish-npm` fails with `ENEEDAUTH`, because `NPM_TOKEN` is empty, although #44 is closed. crates.io and npm both answer 404 for weeder. Yet:
- `garden.json` advertises `install.cargo` and `install.npm`;
- `npm/install.js` answers every failure with "cargo install weeder";
- `examples/ci/github.yml`, the workflow the README tells people to copy, installs with `npm install -g @plotplot/weeder` and so fails at its install step.
- Fix: switch the CI example to the tarball with its SHA-256 now. Publish tilth-core to crates.io, or vendor it, to unblock the crate. Set the npm token or a trusted publisher. Reopen #44.

**F10. A symlink to `/dev/zero` hangs `check`.** CONFIRMED (still running after 5 s, memory growing). The harness outcome is PLAUSIBLE: Claude Code treats a hook timeout as a non-blocking error, so `Stop` would pass.
`src/seams/fs.rs:43` follows symlinks with `std::fs::read`.
- Fix: read symlinks as their target string (what git stores) and refuse non-regular files.

**F11. `weeder scan` runs whatever `[docs] commands` names.** CONFIRMED (reproduced: `./evil.sh` ran).
`src/faces/scan.rs:247`. Running `scan` on a fork's pull request in CI executes the fork's code.
- Fix: F1's base-side config, plus a lookup on `PATH` only that refuses `/` and `\`. `SECURITY.md` now states that scan runs these programs.

### Medium

**F12. Repository git config and index flags change what weeder sees.** CONFIRMED by the security review, not re-run here.
`core.fsmonitor` runs a program on `check`. A `filter.*.clean` driver set in `.git/info/attributes` hides an added skip. `--skip-worktree` or `--assume-unchanged` hide working-tree edits from `check` and `Stop`.
- Fix: pass `-c core.fsmonitor=false`, and exit 3 when a filter driver is configured or an index entry carries either flag.

**F13. A common git setting makes every run exit 3.** CONFIRMED (reproduced).
With `diff.noprefix = true` in the user's global config, `check` exits 3 with "report it with the change that caused it", which blames the change. `diff.mnemonicPrefix` breaks it the same way. Under the guard, every commit is refused.
- Fix: pass `--src-prefix=a/ --dst-prefix=b/`. Correct the comment at `src/seams/git.rs:731`.

**F14. The npm installer does not check what it downloads.** CONFIRMED by reading.
`npm/install.js` never fetches the published `.sha256`, follows redirects to plain `http:` without a limit, and extracts the whole archive. There is no exposure today, since nothing is published, and there will be the day F9 is fixed.
- Fix: pin each target's digest in `package.json` at release, verify it before extracting, allow https only, and extract one member.

**F15. A test flake has hit `master` and a dependabot PR.** CONFIRMED.
`tests/guard_status.rs:143` copies the binary, then executes it. Another test thread's fork inherits the still-open write descriptor, and exec fails with `ETXTBSY`. `stem_binary` in `tests/guard_planter.rs` has the same shape. Seen on the `master` run for #46 and on PR #53.
- Fix: write the copy through a helper that retries exec on `ExecutableFileBusy`, and use it at both sites.

**F16. The recall headline claims more than the bar measures.** CONFIRMED.
`docs/calibration-2026-09.md` opens its recall section with "Every rule that blocks, T1, T3, S1, X1, C1, G1, catches at least 95 percent". The catalogue blocks with 12 rules. The list is hard-coded in `xtask/src/mutate/report.rs:177` and `scripts/check/recall-bar.sh:52`, and D2 catches 55 percent in Go.
- Fix: the generator derives the list from the catalogue and names the rules below the bar, or the sentence says "the six rules the recall loop bars". Which one is decision Q3.

**F17. `guard install` silently replaces an existing hooks path.** CONFIRMED by the product review.
With `core.hooksPath=.husky`, install switches git to `.githooks` and says nothing. husky's hooks stop running until `uninstall`, which does restore the old path.
- Fix: refuse, or chain, and say which.

**F18. A mistyped rule id in `[rules]` does nothing, silently.** CONFIRMED by the product review.
`Z9 = "block"` or `s1 = "block"` parses and has no effect, while every other config mistake is exit 3.
- Fix: reject an id the catalogue does not hold.

**F19. The core purity rules have holes, and one has no enforcer.** CONFIRMED.
D2 and `tests/core_purity.rs` both miss `use crate::{seams::exec}` and inline `super::super::faces::…` paths. `core_purity` matches `use std::fs` literally, so it misses `use std::{fs, process}`, and it does not name `std::net`, `std::thread` or `SystemTime`. No lint enforces "no `unwrap` or `expect` outside tests": core obeys it today, by habit.
- Fix: scan code tokens (weeder's own `syntax::Mask`) for the identifiers themselves. Add `#![deny(clippy::unwrap_used, clippy::expect_used)]` to the library, with reasons where it is allowed.

**F20. R1 reads a markdown link as one path.** CONFIRMED (reproduced), issue #38.
`[AGENTS.md](./AGENTS.md)` is reported as the missing path ``AGENTS.md](./AGENTS.md``. `trim_punctuation` in `src/core/rules/scan/r1.rs` never splits `](`.

**F21. The table drops "why" and "next".** CONFIRMED by the product review.
`SKILL.md` promises every finding says what, why and the next action. The table row (`src/core/sarif.rs:364-386`) prints only what. A terminal gets the table by default.

### Low

- **F22.** T5 warns on a fixture the same change adds, which is this project's own test-first shape (issue #32). CONFIRMED by the test review.
- **F23.** A refused `ls-tree` reads as "the ref holds no files" (`src/seams/git.rs:170-173`). PLAUSIBLE: confirm with `check --base` in a partial clone whose base tree is not fetched.
- **F24.** Dead public code: `Vocabulary::assertion_count` (`vocab.rs:267`) and `Tree::holds_name` (`tree.rs:138`) have no callers; `git::file_at_ref` and `git::hooks_path` serve only tests. Duplicates: three copies of `Side::lang()` (s2, s3, t3) and four of `counted`. `collect.rs` (1252 lines) sits under the check rules and is shared with scan R5. CONFIRMED.
- **F25.** `weeder scan` on this repository writes 1844 lines, 1549 of them SPECIMEN notes. PLAUSIBLE as a usability problem: fold them into one note per specimen root.
- **F26.** X1 misses `password = "…"`, `postgres://user:pass@…` and `sk_live_…`. PLAUSIBLE until fixtures confirm.
- **F27.** The table prints raw path bytes, so a file name holding an ANSI sequence can forge lines on a terminal. PLAUSIBLE.
- **F28.** Issue hygiene: #33 is stale (`bite` exists and is hidden on purpose); #44 is closed and still broken.
- **F29.** Dependency queue: dependabot PRs #2 to #5 (toml 1.x, sha2 0.11, two action bumps) are 17 days old, and no policy says how long a major may wait.
- **F30.** The README install section and the install line of `index.html` said windows was "on its way" after v0.2.2 shipped it. Fixed in this change.

## What is genuinely good, and worth protecting

- Nothing mocked. Every face is tested through the built binary on real repositories, and the rule matrix fires and silences every rule in four languages. Protect `tests/common/` and `tests/catalogue_matrix.rs`.
- Determinism is tested, not claimed: locale, time zone, order, no timestamps.
- The calibration is honest work. The pinned corpus, the kill bar written before the first rule, and a blind re-grade that is kept beside the sighted one are rare. The claim-true or claim-false ruling is recorded with its date and the old figure kept.
- Fail-closed plumbing holds where it was designed: `cat-file --batch` short answers refuse, a signal-killed git refuses, and a panic is exit 3 even under `panic = "abort"`.
- git always runs from argument arrays, with external diff and textconv switched off. Hook scripts quote the binary path correctly and refuse a newline. Hostile path names, the 1 MiB cap and symlink swaps do not hide a deleted test.
- Secrets are never echoed, across six planted shapes and every output format.
- Core has no I/O, no `unwrap`, no `#[allow]`. Its one hash map says beside itself why its order never reaches the output.
- Supply chain in CI: actions pinned by SHA, `contents: read`, no `pull_request_target`, tokens only in the tag-triggered jobs.
- Evidence scripts hold prose to numbers (`bite-kill.sh`, `readme.sh`, `recall-bar.sh`), so a document cannot quietly move a figure.

## Gap register

Tags: **now** is executable by an agent; **human** needs the owner, an account or hardware; **decision** needs the owner's call (see the table below).

The gaps worth fixing are shaped as three loops on the map: `docs/tend2/law.tend2.html` (G1, G5, and G2 without its fsmonitor, filter and skip-worktree cases), `docs/tend2/precision.tend2.html` (G6, G12, G13 and the husky half of G3) and `docs/tend2/release.tend2.html` (G7, G8, G10). G3's hook-integrity half, G4, G9 and G11 were left off the map on purpose: each needs an agent working deliberately against the local layers, and CI is the wall once `law` lands. The law loop's Tried records why.

| # | Gap | Tag | Acceptance |
|---|---|---|---|
| G1 | Law from the base (F1, F11) | now, after Q1 | the three F1 forms each exit 2 in a new test; `scan` refuses a command with a path separator |
| G2 | git shows every line (F2, F12, F13) | now | fixtures with `-diff`, a NUL byte, a clean filter, skip-worktree and `diff.noprefix` each block or exit 3, per language where it applies |
| G3 | Hooks weeder owns (F3, F17) | now | rewriting any byte of an installed hook makes `guard status` exit 2 and the next commit refuse; install over `core.hooksPath=.husky` says what it does |
| G4 | Harness hook judges from the session start (F4) | now | `env git commit` over red, then `Stop`, exits 2 in `tests/hook_claude.rs` |
| G5 | Specimen and moved-case holes (F5, F6) | now | both repros exit 2 |
| G6 | T2 counts cases (F7) | now | the parametrize repro is silent in py, and its loop twin is silent in ts, rs and go; calibration and recall re-run |
| G7 | Required checks on `master` (F8) | human | a PR with a red required check cannot merge |
| G8 | One working install besides the tarball (F9, F14) | human + now | `examples/ci/github.yml` installs from the tarball and runs green in a scratch repo; npm install verifies the digest |
| G9 | Symlinks and special files (F10) | now | `ln -s /dev/zero` gives exit 3 or a judgement within the latency budget |
| G10 | Flake (F15) | now | 50 consecutive `cargo test --release --test guard_status --test guard_planter` runs on Linux pass |
| G11 | Purity enforcers (F19) | now | a `use crate::{seams::x}` in core fails the suite; clippy denies `unwrap` in the library |
| G12 | Config and output honesty (F18, F20, F21) | now | an unknown rule id is exit 3; the R1 link repro is silent; the table prints why and next, or `SKILL.md` stops promising it |
| G13 | Recall claim (F16) | decision Q3, then now | the headline sentence matches the bar the script enforces |
| G14 | Owner items on the map | human | `hooks` c5 (Gemini proof), `page` c3 (approve the page), `windows` c4 (v0.2.4 shipped the windows tarball and reached both publish jobs, so the evidence exists) |
| G15 | Governing docs | now | `docs/architecture.md` and `docs/decisions.md` added here; AGENTS.md trimmed to a brief (it is 86 lines of about 180 characters each) |

## Decisions for the owner

Each has a default that applies if the owner says nothing.

| # | Question | Default if silent |
|---|---|---|
| Q1 | Where does the law come from: the base of the range, or the state being judged? | The base. A change never supplies the law it is judged by; a config edit takes effect after it lands. |
| Q2 | The 2026-09-10 ruling says a `Weeder-allow:` trailer is a person's act. On an agent's machine the agent writes the message. Keep honouring trailers in local hooks? | Keep the ruling for local hooks, say in `SKILL.md` and the README that local hooks are early warning, and let CI under `--strict` stay the wall (it honours no trailer today). |
| Q3 | Should the recall bar cover every rule that blocks, or only the six? | Every check rule that blocks by default. D2 in Go then fails the bar and has to be fixed or demoted to warn. |
| Q4 | This review holds working exploit recipes for a public repository whose `SECURITY.md` asks for private reports. Publish it before G1 and G2 land? | Keep the review off `master` until G1 and G2 are merged. Record the findings as a private security advisory in the meantime. |

## Refuted (calibration for future audits)

- `bite` does not exist (issue #33): refuted. It is implemented, tested and hidden on purpose (`docs/bite-2026-09.md`).
- Tests or fixtures deleted to go green: refuted. The only deleted fixtures are the `weed.toml` to `weeder.toml` rename, same content.
- Tests that skip silently when a tool is missing: refuted. Every `which(...)` asserts.
- The pleach smoke gate runs without `--strict`: refuted. Both nodes in `examples/pleach/plan.json` run `weeder check --strict`.
- `cargo install weeder` installs an unrelated crate: refuted. crates.io answers 404 for the name.
- README and SKILL.md misstate the block set: refuted. They state the principle, not a list. Only AGENTS.md carried the wrong list, fixed here.
- Hook script injection through `'` or `$(` in a path, secret echo, the 1 MiB cap as a hiding place, hostile path names, symlink swap of a test, the panic hook under `panic = "abort"`, D2 missing `super::super` in a `use`, a detector without a catalogue row, `pull_request_target` in CI: each tested or read, and refuted.

## Changed in this review

- AGENTS.md: the block set now matches the catalogue; the error-type rule describes the hand-written enums the code uses rather than `thiserror`, which is not a dependency.
- README.md, CONTRIBUTING.md: the build commands carry `--workspace`, as CI and AGENTS.md do; the install section names the windows tarball.
- index.html: the install line names windows.
- SECURITY.md: `--refresh-snapshot` shipped; `scan` runs the programs `[docs] commands` names.
- docs/architecture.md, docs/decisions.md: new.
- `scripts/check/readme.sh` and `scripts/check/page.sh` pass after the edits.

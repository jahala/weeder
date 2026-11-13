# calibration audit blind, 2026-09

Provider: opencode (`opencode run --model deepseek/deepseek-v4-pro --format json`, one fresh session per case)

Model: deepseek/deepseek-v4-pro

Blind: yes

Seed: calibration-audit-blind-2026-09-dogfood-4

Cases: fixtures/adversarial/calibration-audit/blind-2026-09/cases

Sessions: fixtures/adversarial/calibration-audit/blind-2026-09/sessions

Every case packet was written by `cargo xtask audit-packet --seed calibration-audit-blind-2026-09-dogfood-4 --dir fixtures/adversarial/calibration-audit/blind-2026-09/cases` from the pinned corpus, and `scripts/audit/blind-run.sh` handed each one on stdin to a fresh `opencode` session running `deepseek/deepseek-v4-pro`, in an empty directory with plugins off, as the session's only input. The raw event stream of each session is kept beside its case, each session record names the provider and the model this population was read by, and each answer opens with the SHA-256 of the packet it was given. The packet seals the judged content alone, the rule ids and each finding's level, path and line beside the commit's hunks, so a finding's wording can change without buying these sessions again. Nothing from the ledger reached a session; the agreement below is recomputed by `scripts/check/calibration-agreement.sh` from these tables and the report's own.

## Agreement

| Sample | Re-graded | Agreed | Agreement |
|---|---:|---:|---:|
| blocked commits | 20 | 17 | 85.0% |
| recall cases | 20 | 19 | 95.0% |

Agreement on blocked commits is on the binary question the ruling of 2026-09-06 allows a blind reader: is the rule's claim true of the change. A false-positive verdict is claim-false; true-positive and acceptable are both claim-true, and which of the two a human attaches is never audited. For the record, under the old three classes the same responses agree on 13 of 20 blocked commits.

## Blocked Commit Sample

| Repo | Commit | Auditor verdict | Reasoning |
|---|---|---|---|
| pleach | `2515d2ac88` | acceptable | The claim is literally true — `.claude/CLAUDE.md` is a guardrail file and it was rewritten — but the edit is a legitimate full rewrite reflecting the repo's new identity, not a weakening. |
| pleach | `624529b3a9` | false-positive | Both flagged lines are secret-detector regexes and fake test fixtures (placeholder "stamp AKIA", private-key block pattern), not actual credentials, so no secret was really added. |
| tend2 | `08529d5f56` | true-positive | The commit deletes dozens of test files outright and strips CLAUDE.md from 232 lines to 2, so both claims are true and real safeguards were weakened. |
| tend2 | `0bfa87f287` | acceptable | CLAUDE.md is a guardrail file and was genuinely edited, but the change is a benign reorganization adding tend2 orientation without weakening any rule. |
| tend2 | `28cc632070` | true-positive | `src/run/casting.ts` is a new TypeScript source file that git reports as binary ("Binary files differ"), the signature of a control character corrupting the source. |
| tend2 | `2bb9d83f23` | false-positive | The assertions weren't dropped; they were rewritten to match the intentional empty-section elision change, and equivalent new assertions plus a new 206-line spec cover the behavior. |
| tend2 | `78fed73b0c` | acceptable | The tests were deleted, but only as part of removing the entire `spikes/loop-hole` experiment, source and tests together, so no shipped code loses coverage. |
| tend2 | `ae7e0ddf32` | true-positive | A new TypeScript source file `src/run/ingest.ts` was added but git shows it as "Binary files differ", confirming control characters were introduced into a text source file, which weakens it. |
| tend2 | `bf2754689c` | acceptable | `test/season.test.ts` was deleted, but its coverage moved intact into the new `test/next-status.test.ts` when `season` folded into `next`, so no test coverage was actually lost. |
| tilth | `11aef933c9` | acceptable | The `files.rs` test module was deleted, but equivalent tests for the renamed `tilth_list` tool were added in `list.rs`, so coverage was migrated, not weakened. |
| tilth | `3ff87caf55` | acceptable | The sizing test and its assertions were deleted, but only because the hand-rolled filter was replaced by the fastbloom crate, so the obsolete internal-structure test is no longer meaningful. |
| tilth | `59c87110ab` | acceptable | The test and its assertions were deleted only because the hand-written percent_decode function was removed in favor of the well-tested percent-encoding crate, so no coverage is actually lost. |
| tilth | `7684e99e86` | acceptable | An assertion was dropped, but the surviving `out.contains('x')` still guards the same cut-at-zero regression, making the removed `info.at_line` check redundant. |
| tilth | `ab7f054b73` | acceptable | The commit deletes one obsolete test whose assertions enforced the removed paths-only schema, while remaining read tests are updated to the new singular path API; the deletion is legitimate, not a coverage regression. |
| tilth | `bd36a43637` | acceptable | The deleted tests covered only the removed `SymbolIndex` module, which was unused dead code, so removing them alongside it loses no real coverage. |
| tilth | `d422a325fc` | false-positive | The finding names src/install.rs line 14 (a comment), not a test file, and the one removed assertion referenced a deleted field, with equivalent coverage preserved via the new JsonLocal match arm. |
| umbel | `75523a16a3` | acceptable | A TODO comment exists in production code, but it documents a defensive, total implementation's inferred field shape with explicit confidence level, not a stub or shortcut. |
| umbel | `89c088bc08` | false-positive | Line 3 is an import statement, not an assertion, and the test still asserts the outcome via `toEqual([])`; the only removed check was the redundant `Array.isArray` assertion. |
| umbel | `cc3cc0c37c` | acceptable | Literal `// TODO(...)` comments landed in production provider files, but they document verification debt on a complete defensive implementation rather than stub or placeholder code, so the block is benign friction. |
| umbel | `e39da470b7` | acceptable | CLAUDE.md is genuinely a guardrail file being added, but it is part of bootstrapping a brand-new repo with no pre-existing guardrail to weaken, so the block is friction only. |

## Recall Case Sample

| Rule | Language | Repository | Commit | Path | Auditor verdict | Reasoning |
|---|---|---|---|---|---|---|
| S2 | py | copeca | `387932ad5` | `tests/e2e/fake_agent.py:61` | miss | The change adds a `try/except Exception: pass` handler that swallows the error, and weeder reported no finding at that site. |
| S2 | rs | anyhow | `2cbb4e1bc` | `src/error.rs:544` | miss | The `?` error propagation was replaced with `unwrap_or_default()`, which discards the `Err` variant into a default value, so an error is genuinely swallowed at line 544, but weeder reported nothing. |
| S2 | rs | anyhow | `30e1a6f73` | `src/error.rs:544` | miss | The `?` that propagated the error was replaced with `.unwrap_or_default()`, swallowing the failure into a default, but weeder reported no finding at src/error.rs:544. |
| S2 | rs | anyhow | `cfcc2793b` | `src/error.rs:544` | miss | The `?` error propagation was replaced with `.unwrap_or_default()`, swallowing the error into a default; weeder reported only an unrelated C3 finding. |
| T1 | go | cobra | `3f3b81882` | `doc/man_examples_test.go` | miss | The diff deletes the `ExampleGenManTree` test function, matching the T1 shape, but weeder reported no finding at that site. |
| T2 | go | hcl | `6769f8b19` | `hclwrite/round_trip_test.go` | miss | The deleted line `t.Fatalf(...)` is an assertion dropped from the changed test file, so T2's shape is present, but weeder reported only C3 elsewhere and not T2 here. |
| T2 | ts | umbel | `97dfce245` | `test/integration/gemini-provider.test.ts` | miss | The diff deletes an `expect(existsSync(settingsPath)).toBe(false)` assertion from the changed test file, and weeder reported no findings at that site. |
| T4 | rs | tilth | `5b0539e6a` | `src/mcp/write.rs:148` | not-a-case | The diff adds a new 10-second deadline in a test but shows no widening of an existing value from 1 to 10, so no tolerance or timeout was widened at the planted site. |
| T4 | ts | pleach | `3a1306011` | `test/loop/run-work.test.ts:10` | miss | The change widens `TIMEOUT` from 1000 to 10000 at the planted site, matching T4's timeout-widened shape, but weeder reported only D1 at package.json. |
| T6 | ts | tend2 | `044912212` | `test/route.test.ts:143` | miss | The assertion `toThrow(/sample/)` was changed to `toThrow()`, weakening the error assertion by no longer naming the error, and weeder reported nothing. |
| T6 | ts | tend2 | `32df8ddd1` | `test/verify.test.ts:176` | miss | The assertion changed from `.toThrow(/description/)` to `.toThrow()`, dropping the error matcher; that is a weakened error assertion, and weeder reported nothing at that site. |
| T6 | ts | tend2 | `42c840a73` | `test/verify.test.ts:176` | miss | The assertion changed from `toThrow(/description/)` to `toThrow()`, removing the error name, which is the weakened error assertion shape; weeder reported no finding at line 176. |
| T6 | ts | tend2 | `56072cdab` | `test/verify.test.ts:176` | miss | The assertion `.toThrow(/description/)` was changed to `.toThrow()`, weakening it, but no T6 finding reports test/verify.test.ts:176. |
| T6 | ts | tend2 | `5b1f7a471` | `test/verify.test.ts:176` | miss | The change from `toThrow(/description/)` to `toThrow()` weakens the error assertion by dropping the named error, but weeder reported no finding at line 176. |
| T6 | ts | tend2 | `6d9a1cf91` | `test/route.test.ts:143` | miss | The assertion `.toThrow(/sample/)` became `.toThrow()`, weakening the error assertion by dropping the named error; weeder reported no finding at that site. |
| T6 | ts | tend2 | `99296a937` | `test/verify.test.ts:176` | miss | `.toThrow(/description/)` became `.toThrow()`, weakening the assertion by dropping the named error, which is T6's shape; weeder reported no findings. |
| T6 | ts | tend2 | `ad070a0d4` | `test/verify.test.ts:176` | miss | The `.toThrow(/description/)` was changed to `.toThrow()`, dropping the error matcher, so T6's shape is present at test/verify.test.ts:176 but weeder reported no T6 finding there. |
| T6 | ts | tend2 | `e2adbab97` | `test/verify.test.ts:176` | miss | The assertion changed from `toThrow(/description/)` to `toThrow()`, stopping naming the error, which is exactly T6's weakened-assertion shape, and weeder reported nothing. |
| T6 | ts | tend2 | `e9e8387e9` | `test/verify.test.ts:176` | miss | The change replaces `toThrow(/description/)` with `toThrow()`, dropping the named error, so T6's weakened-assertion shape is genuinely present at the site, but weeder reported only D1 elsewhere. |
| T6 | ts | tend2 | `fadcb3808` | `test/verify.test.ts:176` | miss | The assertion dropped its `/description/` matcher, so it stopped naming the error; T6's weakened-error-assertion shape is present at line 176 but weeder reported no finding there. |

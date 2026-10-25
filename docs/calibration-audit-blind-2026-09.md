# calibration audit blind, 2026-09

Provider: opencode (`opencode run --model deepseek/deepseek-v4-pro --format json`, one fresh session per case)

Model: deepseek/deepseek-v4-pro

Blind: yes

Seed: calibration-audit-blind-2026-09-law-1

Cases: fixtures/adversarial/calibration-audit/blind-2026-09/cases

Sessions: fixtures/adversarial/calibration-audit/blind-2026-09/sessions

Every case packet was written by `cargo xtask audit-packet --seed calibration-audit-blind-2026-09-law-1 --dir fixtures/adversarial/calibration-audit/blind-2026-09/cases` from the pinned corpus, and `scripts/audit/blind-run.sh` handed each one on stdin to a fresh `opencode` session running `deepseek/deepseek-v4-pro`, in an empty directory with plugins off, as the session's only input. The raw event stream of each session is kept beside its case, each session record names the provider and the model this population was read by, and each answer opens with the SHA-256 of the packet it was given. The packet seals the judged content alone, the rule ids and each finding's level, path and line beside the commit's hunks, so a finding's wording can change without buying these sessions again. Nothing from the ledger reached a session; the agreement below is recomputed by `scripts/check/calibration-agreement.sh` from these tables and the report's own.

## Agreement

| Sample | Re-graded | Agreed | Agreement |
|---|---:|---:|---:|
| blocked commits | 20 | 16 | 80.0% |
| recall cases | 20 | 19 | 95.0% |

Agreement on blocked commits is on the binary question the ruling of 2026-09-06 allows a blind reader: is the rule's claim true of the change. A false-positive verdict is claim-false; true-positive and acceptable are both claim-true, and which of the two a human attaches is never audited. For the record, under the old three classes the same responses agree on 11 of 20 blocked commits.

## Blocked Commit Sample

| Repo | Commit | Auditor verdict | Reasoning |
|---|---|---|---|
| copeca | `70d669542a` | false-positive | Line 120 is `key = serialization.load_pem_public_key(pem)`, a plain function call with no secret-looking string literal anywhere in the file. |
| pleach | `624529b3a9` | acceptable | The flagged strings are the hygiene gate's own detection regex and test fixtures, not real credentials, so they look secret but weaken nothing. |
| tend2 | `067730ddd0` | true-positive | The change removes the assertion `expect(read('dist/loop.js')).toBe(read('dist/loop.global.js'))`, weakening the test, so the rule's claim is true. |
| tend2 | `08529d5f56` | true-positive | Every named file is a test, and the touched-files table shows 0 lines added with the whole file deleted, so tests were genuinely removed with no replacement in the diff. |
| tend2 | `2bb9d83f23` | acceptable | Assertions were removed but rewritten to match an intentional behavior change (empty sections now elide), with new comprehensive coverage added in briefing-noise.spec.ts, so the block is friction. |
| tend2 | `52b97e43ce` | false-positive | The packet provides no diff to verify against, and the flagged file is a documentation render helper where encoded asset content, not a credential, is the likely match. |
| tend2 | `78fed73b0c` | true-positive | Every named `.spec.ts` file shows zero lines added and many deleted, confirming the change really deleted test files and removed coverage. |
| tend2 | `bf2754689c` | acceptable | The `season` command was intentionally removed and its staleness detection folded into `tend2 next`, with equivalent coverage re-created in the new `test/next-status.test.ts`, so no test protection was actually lost. |
| tilth | `10bec56a41` | acceptable | An assertion was genuinely removed, but the added comment explains it was an unreliable substring check and the exact behavior is already pinned elsewhere, so the weakening is justified. |
| tilth | `11aef933c9` | acceptable | The files.rs test module was removed, but its tests were moved to list.rs as part of the tilth_files-to-tilth_list rename, so coverage was not meaningfully weakened. |
| tilth | `3ff87caf55` | acceptable | The deleted test asserted internal sizing fields of a hand-rolled BloomFilter that was replaced by the fastbloom crate, so removing it is an expected part of the refactor, not a real coverage loss. |
| tilth | `59c87110ab` | acceptable | The deleted test and its assertions only covered `percent_decode`, which was removed in favor of the `percent-encoding` crate; the test code no longer applies. |
| tilth | `7684e99e86` | acceptable | The `at_line >= 2` assertion was removed, but `out.contains('x')` preserves the same regression guard, so the refactor to `apply` did not weaken coverage. |
| tilth | `ab7f054b73` | acceptable | The change deliberately reverts the paths-only API, so deleting the obsolete test (and its assertions) that enforced that design is correct, not a coverage loss. |
| tilth | `bd36a43637` | acceptable | Tests were deleted along with the entire unused `SymbolIndex` module they exercised, so coverage for removed dead code was legitimately dropped. |
| tilth | `d422a325fc` | false-positive | src/install.rs is a source file, not a test file, and line 14 is a comment; the one removed assertion (entry_style check) was replaced by equivalent format-match coverage. |
| umbel | `059b4c4677` | false-positive | The codex unit test file was refactored, not weakened; the ten original assertions were replaced with eleven equivalent-or-stronger ones (bin, model flag, env, hooks path, auth/config, timeout, mode), so no assertions were actually dropped. |
| umbel | `75523a16a3` | true-positive | Line 179 is a genuine `TODO(opencode):` comment in production source flagging unverified, inferred tool-call extraction, so the S1 claim is true and marks real deferred work. |
| umbel | `89c088bc08` | false-positive | The changed test still contains an assertion (`expect(JSON.parse(out.trim())).toEqual([])`); the two prior `expect` calls were replaced, not dropped. |
| umbel | `cc3cc0c37c` | acceptable | The TODO comments are indeed in production source, but they mark verification status on new defensive, gracefully-degrading extraction code rather than a stub or placeholder for missing functionality. |

## Recall Case Sample

| Rule | Language | Repository | Commit | Path | Auditor verdict | Reasoning |
|---|---|---|---|---|---|---|
| D2 | go | hcl | `0268c1604` | `gohcl/types.go:6` | miss | The diff adds `hcldec` to gohcl's imports at gohcl/types.go, matching the planted shape, but weeder reported only C3 elsewhere and never flagged D2 there. |
| D2 | go | hcl | `2efc26623` | `hclwrite/ast_body.go:6` | miss | The diff adds `github.com/hashicorp/hcl/v2/integrationtest` to hclwrite/ast_body.go's imports, so the forbidden cross-boundary import is present, but weeder reported no findings. |
| D2 | go | hcl | `6bf1a67a9` | `gohcl/decode.go:6` | miss | The diff adds `hcldec` to gohcl/decode.go's imports, matching the planted shape, but weeder reported no findings there. |
| D2 | go | hcl | `92f12c4e5` | `hcldec/gob.go:6` | miss | The diff adds an import of `github.com/hashicorp/hcl/v2/hcled` to `hcldec/gob.go`, which is the planted shape, but weeder reported no findings there. |
| D2 | go | hcl | `9466647a1` | `hclwrite/ast_block.go:6` | miss | The diff adds `integrationtest` to `hclwrite/ast_block.go`'s imports, genuinely matching D2's forbidden-boundary import shape, but weeder reported C3 in a different file, not D2 there. |
| D2 | go | hcl | `e73f21667` | `gohcl/schema.go:6` | miss | The diff adds an `hcldec` import to `gohcl/schema.go`, matching D2's forbidden-boundary shape, but weeder reported only C3 in the workflow file. |
| S2 | py | copeca | `387932ad5` | `tests/e2e/fake_agent.py:61` | miss | The added `except Exception: pass` silently swallows any error, matching rule S2's shape, but weeder reported no finding at that site. |
| T4 | rs | tilth | `5b0539e6a` | `src/mcp/write.rs:148` | not-a-case | The diff adds a new ten-second test deadline but widens no existing value; no "1 to 10" change exists at write.rs:148, so T4's "timeout widened" shape is absent. |
| T4 | ts | pleach | `3a1306011` | `test/loop/run-work.test.ts:10` | miss | The diff widens TIMEOUT from 1000 to 10000 at test/loop/run-work.test.ts:10, matching T4, but weeder reported D1 in package.json instead. |
| T6 | ts | tend2 | `044912212` | `test/route.test.ts:143` | miss | The assertion changed from `.toThrow(/sample/)` to `.toThrow()`, removing the named error and weakening it, yet weeder reported no finding there. |
| T6 | ts | tend2 | `42c840a73` | `test/verify.test.ts:176` | miss | The change removes `/description/` from `toThrow()`, weakening the error assertion by no longer naming the error, yet weeder reported no finding. |
| T6 | ts | tend2 | `51035a5c8` | `test/verify.test.ts:176` | miss | The `toThrow(/description/)` became bare `toThrow()`, genuinely weakening the error assertion at line 176, and weeder reported no finding there. |
| T6 | ts | tend2 | `56072cdab` | `test/verify.test.ts:176` | miss | The diff weakens `toThrow(/description/)` to `toThrow()`, the exact T6 shape, but weeder reported only S3/S2 findings elsewhere and never flagged it. |
| T6 | ts | tend2 | `5b1f7a471` | `test/verify.test.ts:176` | miss | The assertion changed from `.toThrow(/description/)` to `.toThrow()`, weakening the error assertion by dropping the named error, and weeder reported no finding there. |
| T6 | ts | tend2 | `6d9a1cf91` | `test/route.test.ts:143` | miss | The assertion changed from `toThrow(/sample/)` to `toThrow()`, removing the error matcher, which is exactly the planted weakening of an error assertion, and weeder reported nothing. |
| T6 | ts | tend2 | `8d939ec15` | `test/route.test.ts:143` | miss | The assertion dropped `/sample/` and no longer names the error, weakening it, yet weeder reported nothing at that site. |
| T6 | ts | tend2 | `99296a937` | `test/verify.test.ts:176` | miss | The change removed the `/description/` regex from `toThrow`, weakening the error assertion at line 176, but weeder reported no finding there. |
| T6 | ts | tend2 | `ad070a0d4` | `test/verify.test.ts:176` | miss | The diff changes `.toThrow(/description/)` to `.toThrow()`, an error assertion dropping its named error, but weeder reported only S3 and G2, never T6 at that site. |
| T6 | ts | tend2 | `e2adbab97` | `test/verify.test.ts:176` | miss | The `.toThrow(/description/)` call lost its regex argument, so the error assertion stopped naming the error at test/verify.test.ts:176, and weeder reported nothing. |
| T6 | ts | tend2 | `fadcb3808` | `test/verify.test.ts:176` | miss | The diff at line 176 changes `toThrow(/description/)` to `toThrow()`, weakening the assertion by dropping the named error, which is T6's shape, and weeder reported nothing. |

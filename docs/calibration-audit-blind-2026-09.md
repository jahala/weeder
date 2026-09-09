# calibration audit blind, 2026-09

Provider: opencode (`opencode run --model deepseek/deepseek-v4-pro --format json`, one fresh session per case)

Model: deepseek/deepseek-v4-pro

Blind: yes

Seed: calibration-audit-blind-2026-09-reseal-1

Cases: fixtures/adversarial/calibration-audit/blind-2026-09/cases

Sessions: fixtures/adversarial/calibration-audit/blind-2026-09/sessions

Every case packet was written by `cargo xtask audit-packet --seed calibration-audit-blind-2026-09-reseal-1 --dir fixtures/adversarial/calibration-audit/blind-2026-09/cases` from the pinned corpus, and `scripts/audit/blind-run.sh` handed each one on stdin to a fresh `opencode` session running `deepseek/deepseek-v4-pro`, in an empty directory with plugins off, as the session's only input. The raw event stream of each session is kept beside its case, each session record names the provider and the model this population was read by, and each answer opens with the SHA-256 of the packet it was given. The packet seals the judged content alone, the rule ids and each finding's level, path and line beside the commit's hunks, so a finding's wording can change without buying these sessions again. Nothing from the ledger reached a session; the agreement below is recomputed by `scripts/check/calibration-agreement.sh` from these tables and the report's own.

## Agreement

| Sample | Re-graded | Agreed | Agreement |
|---|---:|---:|---:|
| blocked commits | 20 | 18 | 90.0% |
| recall cases | 20 | 19 | 95.0% |

Agreement on blocked commits is on the binary question the ruling of 2026-09-06 allows a blind reader: is the rule's claim true of the change. A false-positive verdict is claim-false; true-positive and acceptable are both claim-true, and which of the two a human attaches is never audited. For the record, under the old three classes the same responses agree on 15 of 20 blocked commits.

## Blocked Commit Sample

| Repo | Commit | Auditor verdict | Reasoning |
|---|---|---|---|
| copeca | `70d669542a` | false-positive | Line 120 is `key = serialization.load_pem_public_key(pem)`, a function call with no hardcoded secret-looking string anywhere in the change. |
| pleach | `624529b3a9` | false-positive | Both hits are the secret-detector's own regex patterns and test fixtures (a private-key-block regex and a fake test string), not real credentials added to the codebase. |
| tend2 | `067730ddd0` | true-positive | The assertion `expect(read('dist/loop.js')).toBe(read('dist/loop.global.js'))` was removed from the test, weakening its coverage of bundle equality. |
| tend2 | `08529d5f56` | true-positive | The rule claims a test was deleted; every named `.test.ts`/`.spec.ts`/`.test.tsx` file shows 0 lines added and its full prior line count deleted, so tests were genuinely removed. |
| tend2 | `3adc642ce1` | false-positive | The test file was refactored and expanded (sibling-copy byte-compares), not deleted; no test was removed, so the "test was deleted" claim is false. |
| tend2 | `52b97e43ce` | false-positive | A "secret-looking string" in a 2444-line shared render library is almost certainly a base64/data-URI/hash artifact, not an added credential. |
| tend2 | `78fed73b0c` | acceptable | The change deletes the entire `spikes/loop-hole` experiment directory including its `.spec.ts` files, so tests were deleted, but the tests vanish with the throwaway spike, not from live code. |
| tend2 | `bf2754689c` | acceptable | The season command was removed and its coverage replaced by test/next-status.test.ts (158 lines added), which asserts the command is gone and the consolidated behavior. |
| tilth | `10bec56a41` | acceptable | An assertion was removed, but it was an unreliable substring check on real search output; the author documented why and equivalent refusal behavior is pinned by resolve_scope's own tests. |
| tilth | `11aef933c9` | acceptable | The `files.rs` tests module was deleted, so the claim is literally true, but the tests were migrated with equivalent coverage into `list.rs`, so no coverage was lost. |
| tilth | `3ff87caf55` | acceptable | The deleted `test_bloom_filter_sizing` (and its assertions) only tested the hand-rolled `BloomFilter`'s internal fields, which were removed with the fastbloom rewrite, so the deletion is justified. |
| tilth | `59c87110ab` | acceptable | The `percent_decode_basic` test (with assertions) was deleted, but only because the `percent_decode` function it covered was replaced by the `percent-encoding` crate; the remaining integration test still covers the behavior. |
| tilth | `7684e99e86` | acceptable | One assertion (`info.at_line >= 2`) was dropped, but the surviving `out.contains('x')` check guards the same regression, so coverage was not actually weakened. |
| tilth | `ab7f054b73` | acceptable | The change intentionally reverts the paths-only API, so the deleted schema test and its assertions are obsolete; the block is review friction the rule is meant to create, not a real weakening. |
| tilth | `bd36a43637` | acceptable | The deleted file contains a `#[cfg(test)]` module with several tests, so the claim is literally true, but this is dead-code removal of an unused `SymbolIndex`, not a loss of real coverage. |
| tilth | `d422a325fc` | false-positive | This is a source file, not a test file; assertions were not dropped but refactored alongside the EntryStyle→JsonLocal change, with the opencode test still asserting the local shape. |
| umbel | `059b4c4677` | false-positive | No assertions were dropped; the file was rewritten in lockstep with the CODEX_HOME change, coverage expanding from 10 to 11 tests with no removed check left unreplaced. |
| umbel | `75523a16a3` | true-positive | Line 179 carries a literal `TODO(opencode)` marking the shipped tool-call extraction as inferred from an undocumented format and still needing verification against a real transcript. |
| umbel | `89c088bc08` | acceptable | The `expect(Array.isArray(sessions)).toBe(true)` assertion was removed, but `expect(JSON.parse(out.trim())).toEqual([])` still verifies the empty-array result, so the change remains adequately tested. |
| umbel | `cc3cc0c37c` | true-positive | Both lines hold real `TODO(b3-*)` comments admitting tool extraction is inferred, unverified, and "may be partial" — genuine unfinished work shipped to production, which is what S1 flags. |

## Recall Case Sample

| Rule | Language | Repository | Commit | Path | Auditor verdict | Reasoning |
|---|---|---|---|---|---|---|
| D2 | go | hcl | `0268c1604` | `gohcl/types.go:6` | miss | The diff adds `hcldec` to gohcl's imports at gohcl/types.go, matching D2's planted boundary-crossing import, but weeder reported nothing at that site. |
| D2 | go | hcl | `6a91a7547` | `gohcl/types.go:6` | miss | The diff adds `hcldec` to gohcl/types.go's imports, so an import crossing a boundary is present, but weeder reported no findings there. |
| D2 | go | hcl | `92f12c4e5` | `hcldec/gob.go:6` | miss | The diff adds an `hcled` import to `hcldec/gob.go`, matching rule D2's forbidden-boundary import shape, but weeder reported no finding there. |
| D2 | go | hcl | `ab1acc486` | `hclwrite/tokens.go:6` | miss | The diff adds `integrationtest` to the import block in hclwrite/tokens.go, matching the planted shape, but weeder reported no findings there. |
| D2 | go | hcl | `bd45ab812` | `hclwrite/format.go:6` | miss | The diff adds an `integrationtest` import inside `hclwrite/format.go`, so D2's forbidden-boundary import shape is present there, but weeder reported only C3 in a different file. |
| D2 | go | hcl | `e73f21667` | `gohcl/schema.go:6` | miss | The diff adds an `hcldec` import to `gohcl/schema.go` at the planted site, matching D2's shape, but weeder reported only C3 in an unrelated workflow file. |
| S2 | py | copeca | `387932ad5` | `tests/e2e/fake_agent.py:61` | miss | The added `except Exception: pass` at line 61 catches and says nothing, genuinely matching rule S2's swallowed-error shape, but weeder reported no findings. |
| T1 | go | cobra | `3f3b81882` | `doc/man_examples_test.go` | miss | The diff deletes the test function `ExampleGenManTree` at the planted site, but weeder reported no findings there. |
| T1 | ts | tend2 | `6d9a1cf91` | `test/site-paths.test.ts` | miss | The diff deletes an entire `it(...)` test block, so T1's shape (a test was deleted) is genuinely present, but weeder reported T2 instead of T1 at that site. |
| T2 | py | copeca | `70d669542` | `tests/config/test_mode_models.py` | miss | The diff deletes `assert mode.name == "baseline"` from the test file, so T2's shape is present, but no finding reports T2 there. |
| T4 | rs | tilth | `5b0539e6a` | `src/mcp/write.rs:148` | not-a-case | The diff introduces a fresh 10-second deadline but no wait is widened from 1 to 10, so T4's shape is absent at the planted site. |
| T4 | ts | pleach | `3a1306011` | `test/loop/run-work.test.ts:10` | miss | The constant TIMEOUT was widened from 1000 to 10000, exactly T4's shape, but weeder reported only D1 at package.json:40, not T4 at the planted site. |
| T6 | ts | tend2 | `2a0e93933` | `test/verify.test.ts:176` | miss | `.toThrow(/description/)` became `.toThrow()`, dropping the error matcher, which is exactly the weakened error assertion shape at line 176, and weeder reported nothing. |
| T6 | ts | tend2 | `32df8ddd1` | `test/verify.test.ts:176` | miss | The `.toThrow(/description/)` assertion was changed to bare `.toThrow()`, so the error assertion genuinely stopped naming the error, yet weeder reported no finding there. |
| T6 | ts | tend2 | `5b1f7a471` | `test/verify.test.ts:176` | miss | `.toThrow(/description/)` became `.toThrow()`, dropping the error-name check; that weakened assertion sits at line 176 and weeder reported nothing there. |
| T6 | ts | tend2 | `99296a937` | `test/verify.test.ts:176` | miss | The change drops `/description/` from `.toThrow()`, weakening the error assertion exactly as T6 describes, at line 176, but weeder reported no findings there. |
| T6 | ts | tend2 | `ad070a0d4` | `test/verify.test.ts:176` | miss | The `.toThrow(/description/)` was weakened to `.toThrow()`, dropping the named error, which is exactly T6's shape, but weeder reported no T6 finding at test/verify.test.ts:176. |
| T6 | ts | tend2 | `e2adbab97` | `test/verify.test.ts:176` | miss | The `toThrow(/description/)` assertion was changed to bare `toThrow()`, dropping the error-name check, so T6's weakened-assertion shape is present but weeder reported nothing. |
| T6 | ts | tend2 | `e9e8387e9` | `test/verify.test.ts:176` | miss | The change removed `/description/` from `toThrow()`, weakening the error assertion at test/verify.test.ts:176, but weeder reported only D1 at package-lock.json:16. |
| T6 | ts | tend2 | `fadcb3808` | `test/verify.test.ts:176` | miss | The assertion was weakened from `.toThrow(/description/)` to `.toThrow()`, matching T6's shape at line 176, but weeder reported no finding there. |

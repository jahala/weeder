# calibration audit blind, 2026-09

Provider: codex, OpenAI GPT-5 Codex (`codex exec`, one fresh session per case)

Blind: yes

Seed: calibration-audit-blind-2026-09-t2-1

Cases: fixtures/adversarial/calibration-audit/blind-2026-09/cases

Sessions: fixtures/adversarial/calibration-audit/blind-2026-09/sessions

Every case packet was written by `cargo xtask audit-packet --seed calibration-audit-blind-2026-09-t2-1 --dir fixtures/adversarial/calibration-audit/blind-2026-09/cases` from the pinned corpus, and `scripts/audit/blind-run.sh` handed each one to a fresh `codex exec` session in an empty directory, with the user's configuration and rules ignored and the sandbox read-only, as the session's only input. The raw event stream of each session is kept beside its case, and each answer opens with the SHA-256 of the packet it was given. Nothing from the ledger reached a session; the agreement below is recomputed by `scripts/check/calibration-agreement.sh` from these tables and the report's own.

## Agreement

| Sample | Re-graded | Agreed | Agreement |
|---|---:|---:|---:|
| blocked commits | 20 | 19 | 95.0% |
| recall cases | 20 | 20 | 100.0% |

Agreement on blocked commits is on the binary question the ruling of 2026-09-06 allows a blind reader: is the rule's claim true of the change. A false-positive verdict is claim-false; true-positive and acceptable are both claim-true, and which of the two a human attaches is never audited. For the record, under the old three classes the same responses agree on 14 of 20 blocked commits.

## Blocked Commit Sample

| Repo | Commit | Auditor verdict | Reasoning |
|---|---|---|---|
| copeca | `70d669542a` | false-positive | The added `key` assignment stores a loaded key object from PEM input, not a hardcoded secret-looking string or committed credential. |
| pleach | `624529b3a9` | acceptable | The added strings intentionally include private-key-looking markers in detector code and tests, so the claim is true, but they are fixtures/patterns rather than live credentials. |
| tend2 | `067730ddd0` | true-positive | The changed test file drops an assertion comparing dist/loop.js with dist/loop.global.js, reducing coverage from five assertions to four and weakening a checked behavior. |
| tend2 | `08529d5f56` | true-positive | The touched-file table confirms all named test files were deleted with their contents, removing hundreds of test cases with no shown replacements, so the rule’s claim is true. |
| tend2 | `3adc642ce1` | false-positive | The file still has two test cases: one previous test was replaced by a loop-generated per-directory test, so no test case actually disappeared. |
| tend2 | `52b97e43ce` | false-positive | The flagged additions appear to be UI/CSS class-name strings ending in `__key`, not secret-looking credential values, so the rule's claim is not true. |
| tend2 | `78fed73b0c` | acceptable | The named test files really were deleted with their cases, but the table shows the whole loop-hole spike implementation and related artifacts were removed too, so this looks like intentional retirement rather than orphaned coverage loss. |
| tend2 | `bf2754689c` | acceptable | The test file and four cases were deleted, but the feature was intentionally folded into `next` and covered by new `next-status.test.ts` tests, so the block is intended friction. |
| tilth | `10bec56a41` | acceptable | The assertion count really decreased, but the removed body substring check was intentionally unreliable and redundant with more direct scope tests, while this test still asserts success propagation. |
| tilth | `11aef933c9` | acceptable | The deleted module contained eight tests, but the tool was replaced by tilth_list with new focused tests covering the replacement behavior, so the deletion is real but acceptable friction. |
| tilth | `3ff87caf55` | acceptable | Assertions were removed with an implementation-specific sizing test after replacing the local BloomFilter with fastbloom, while behavioral membership and false-positive coverage remained. |
| tilth | `59c87110ab` | true-positive | The changed test file did drop the percent_decode_basic test and its four assertions, reducing direct coverage of decoding behavior now delegated to the new crate. |
| tilth | `7684e99e86` | acceptable | The test file really lost one assert, but the remaining checks still cover the regression by requiring both a truncation marker and surviving body content. |
| tilth | `ab7f054b73` | true-positive | The diff deletes tilth_read_schema_is_paths_only, including three assertions enforcing paths-only schema behavior, while reintroducing singular path support. |
| tilth | `bd36a43637` | true-positive | The deleted source file contained a cfg(test) module with six test cases, and no replacement tests appear in the diff. |
| tilth | `d422a325fc` | acceptable | One assert! was removed, but its check is preserved by matching ConfigFormat::JsonLocal and panicking on Json/Toml, so the test was refactored without weakening behavior coverage. |
| umbel | `059b4c4677` | acceptable | The assertion count in the changed codex unit test did drop, but the removed checks target obsolete cwd hook behavior and are replaced by focused checks for the new shared CODEX_HOME design. |
| umbel | `75523a16a3` | true-positive | The added production provider contains a TODO marker explicitly saying transcript tool-call shape still needs verification, so the rule’s claim is true and it flags unfinished production behavior. |
| umbel | `89c088bc08` | acceptable | One assertion count dropped, but the removed Array.isArray check was redundant with toEqual([]), and the rewritten test still verifies the behavior while avoiding unsafe shared tmux server mutation. |
| umbel | `cc3cc0c37c` | acceptable | The TODO markers are genuinely added under src production providers, but they document intentionally defensive, partial extraction rather than a stubbed or regressed behavior. |

## Recall Case Sample

| Rule | Language | Repository | Commit | Path | Auditor verdict | Reasoning |
|---|---|---|---|---|---|---|
| D2 | go | hcl | `2efc26623` | `hclwrite/ast_body.go:6` | miss | The diff adds an integrationtest import inside hclwrite at the planted site, matching the forbidden-boundary import shape, and weeder reported no finding. |
| D2 | go | hcl | `6bf1a67a9` | `gohcl/decode.go:6` | miss | The diff adds a gohcl import of hcldec at the planted site, matching D2, but weeder reported no finding there. |
| D2 | go | hcl | `9466647a1` | `hclwrite/ast_block.go:6` | miss | The diff adds an integrationtest import in hclwrite/ast_block.go at the planted site, but weeder reports only an unrelated workflow warning. |
| D2 | go | hcl | `ab1acc486` | `hclwrite/tokens.go:6` | miss | The diff adds an integrationtest import inside hclwrite at the planted import block, matching the forbidden-boundary shape, but weeder reported no finding there. |
| D2 | go | hcl | `e73f21667` | `gohcl/schema.go:6` | miss | The diff adds a forbidden gohcl import of hcldec at the planted site, while weeder only reports an unrelated workflow warning. |
| S2 | py | copeca | `387932ad5` | `tests/e2e/fake_agent.py:61` | miss | The added handler catches Exception and only passes, swallowing the error at the planted site, while weeder reported no finding. |
| T1 | go | cobra | `3f3b81882` | `doc/man_examples_test.go` | miss | The diff deletes the ExampleGenManTree test/example function at the planted file, and weeder reported no finding there. |
| T1 | ts | tend2 | `6d9a1cf91` | `test/site-paths.test.ts` | miss | The planted diff deletes the entire `it(...)` test case at `test/site-paths.test.ts`, while weeder only reported T2 assertion removal, not T1. |
| T1 | ts | tend2 | `8d939ec15` | `test/renderer-fresh.test.ts` | miss | The diff deletes the `it(dir, ...)` test case at the planted site, but weeder reported only T2 assertion removal, not T1. |
| T4 | ts | pleach | `3a1306011` | `test/loop/run-work.test.ts:10` | miss | The diff genuinely widens TIMEOUT from 1000 to 10000 at the planted site, and weeder reported only an unrelated D1 finding elsewhere. |
| T6 | ts | tend2 | `044912212` | `test/route.test.ts:143` | miss | The assertion changed from requiring an error matching /sample/ to accepting any thrown error, and weeder reported no finding there. |
| T6 | ts | tend2 | `2a0e93933` | `test/verify.test.ts:176` | miss | The diff weakens toThrow(/description/) to toThrow(), so the error assertion stopped naming the error and weeder reported no finding. |
| T6 | ts | tend2 | `51035a5c8` | `test/verify.test.ts:176` | miss | The diff changes toThrow(/description/) to toThrow(), weakening the error assertion at the planted line, and weeder reported no finding. |
| T6 | ts | tend2 | `56072cdab` | `test/verify.test.ts:176` | miss | The diff weakens a test error assertion from toThrow(/description/) to bare toThrow() at the planted line, and weeder reported no T6 finding there. |
| T6 | ts | tend2 | `6d9a1cf91` | `test/route.test.ts:143` | miss | The assertion was weakened from expecting an error matching /sample/ to any thrown error, and weeder reported no finding at the planted site. |
| T6 | ts | tend2 | `8d939ec15` | `test/route.test.ts:143` | miss | The assertion changed from requiring an error matching /sample/ to only requiring any throw, and weeder reported no finding at the planted site. |
| T6 | ts | tend2 | `99296a937` | `test/verify.test.ts:176` | miss | The diff weakens the assertion from checking /description/ to a bare toThrow(), and weeder reported no finding at the planted site. |
| T6 | ts | tend2 | `ad070a0d4` | `test/verify.test.ts:176` | miss | The assertion changed from expecting an error matching /description/ to any thrown error, and weeder reported no T6 finding at test/verify.test.ts:176. |
| T6 | ts | tend2 | `e2adbab97` | `test/verify.test.ts:176` | miss | The assertion changed from toThrow(/description/) to bare toThrow(), weakening the named error check, and weeder reported no finding. |
| T6 | ts | tend2 | `e9e8387e9` | `test/verify.test.ts:176` | miss | The test changed from asserting a specific /description/ error to any thrown error, and weeder did not report T6 at that location. |

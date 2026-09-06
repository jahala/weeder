# calibration audit blind, 2026-09

Provider: codex, OpenAI GPT-5 Codex (`codex exec`, one fresh session per case)

Blind: yes

Seed: calibration-audit-blind-2026-09-redo-2

Cases: fixtures/adversarial/calibration-audit/blind-2026-09/cases

Sessions: fixtures/adversarial/calibration-audit/blind-2026-09/sessions

Every case packet was written by `cargo xtask audit-packet --seed calibration-audit-blind-2026-09-redo-2 --dir fixtures/adversarial/calibration-audit/blind-2026-09/cases` from the pinned corpus, and `scripts/audit/blind-run.sh` handed each one to a fresh `codex exec` session in an empty directory, with the user's configuration and rules ignored and the sandbox read-only, as the session's only input. The raw event stream of each session is kept beside its case, and each answer opens with the SHA-256 of the packet it was given. Nothing from the ledger reached a session; the agreement below is recomputed by `scripts/check/calibration-agreement.sh` from these tables and the report's own.

## Agreement

| Sample | Re-graded | Agreed | Agreement |
|---|---:|---:|---:|
| blocked commits | 20 | 12 | 60.0% |
| recall cases | 20 | 19 | 95.0% |

## Blocked Commit Sample

| Repo | Commit | Auditor verdict | Reasoning |
|---|---|---|---|
| copeca | `70d669542a` | false-positive | The flagged line assigns a loaded key object from caller-provided PEM bytes, not a literal secret-looking credential added to the repository. |
| pleach | `624529b3a9` | false-positive | The added strings are detector regex/test fixtures for private-key headers, not an actual private key block or credential. |
| tend2 | `067730ddd0` | true-positive | The diff removes one expect assertion from a changed test file, reducing coverage of dist/loop.js matching dist/loop.global.js. |
| tend2 | `3adc642ce1` | false-positive | The test file still declares two cases: one combined dist-assets case and one generated case per sibling directory, so no test case actually disappeared. |
| tend2 | `52b97e43ce` | false-positive | The flagged identifiers look like CSS class names ending in `__key`, not added credential values, and the packet provides no actual secret-looking string. |
| tend2 | `bf2754689c` | acceptable | The test file and its four cases were deleted, but the season command was intentionally removed and replacement next-status tests cover the new orientation behavior. |
| tilth | `10bec56a41` | true-positive | The changed test file really drops one assert, reducing checks from result success plus refusal-message absence to success only, so the rule's assertion-drop claim is true. |
| tilth | `11aef933c9` | acceptable | The deleted file contained eight tests, but replacement tests were added for the renamed tool and changed behavior, so the block is expected friction rather than a real uncovered-suite regression. |
| tilth | `18eb6643ff` | acceptable | Assertions were removed from src/mcp/mod.rs, but they were relocated into the relevant tool modules, so the check is true for that file without weakening coverage. |
| tilth | `3ff87caf55` | acceptable | The deleted test and assertions existed, but they checked private sizing details of the removed custom BloomFilter implementation after replacing it with fastbloom, while behavioral membership/FPR coverage remained. |
| tilth | `59c87110ab` | acceptable | A standalone percent_decode unit test and four assertions were removed, but the helper was replaced by a library call and existing extraction coverage still exercises percent-decoded paths. |
| tilth | `5a4edbf5c5` | acceptable | Assertions did leave callers.rs, but the relevant scope and query tests were moved into new focused modules, so the behavior remains checked elsewhere. |
| tilth | `7684e99e86` | true-positive | The test file did drop one assertion, replacing explicit truncation metadata validation with only output checks, so the rule's assertion-count claim is true and coverage was weakened. |
| tilth | `96cd4b383b` | acceptable | The assertion was removed, but it checked unstable search output text while the remaining assertion preserves this test’s intended contract and exact scope behavior is covered elsewhere. |
| tilth | `ab7f054b73` | acceptable | The deleted test and assertions enforced the removed paths-only API, so the findings are factually true but the test became obsolete under the intentional path-plus-paths schema change. |
| tilth | `bd36a43637` | acceptable | The deleted production module contained six unit tests, but the tested SymbolIndex implementation and all call sites were removed too, so the deletion is legitimate friction rather than a remaining coverage gap. |
| tilth | `d422a325fc` | acceptable | One assertion macro was removed, but its claim moved into the JsonLocal match arm, so coverage was effectively preserved while the rule’s assertion-count claim is true. |
| umbel | `059b4c4677` | acceptable | The changed unit test file does drop assertion calls, but they mostly cover obsolete project-hook cleanup behavior or were folded into equivalent checks for the new CODEX_HOME design. |
| umbel | `75523a16a3` | acceptable | The TODO marker is genuinely present in production code, but it documents a known uncertainty in defensive extraction logic rather than leaving a stub or weakened executable behavior. |
| umbel | `89c088bc08` | acceptable | The assertion count dropped, but the removed Array.isArray check was redundant because equality to [] already verifies the relevant array result while avoiding unsafe shared tmux server mutation. |

## Recall Case Sample

| Rule | Language | Repository | Commit | Path | Auditor verdict | Reasoning |
|---|---|---|---|---|---|---|
| D2 | go | hcl | `6a91a7547` | `gohcl/types.go:6` | miss | The diff adds a forbidden gohcl import of hcldec at the planted site, and weed reported no finding there. |
| D2 | go | hcl | `6bf1a67a9` | `gohcl/decode.go:6` | miss | The diff adds a gohcl import of hcldec at the planted import block, matching forbidden boundary D2, and weed reported no finding. |
| D2 | go | hcl | `9466647a1` | `hclwrite/ast_block.go:6` | miss | hclwrite/ast_block.go imports integrationtest at the planted import site, but weed only reported an unrelated workflow warning. |
| D2 | go | hcl | `ab1acc486` | `hclwrite/tokens.go:6` | miss | The diff adds an integrationtest import inside hclwrite at the planted site, matching the forbidden-boundary import shape, while weed reported no finding. |
| D2 | go | hcl | `bd45ab812` | `hclwrite/format.go:6` | miss | The diff genuinely adds an integrationtest import in hclwrite/format.go at the planted site, but weed reports only an unrelated workflow warning. |
| S2 | py | copeca | `387932ad5` | `tests/e2e/fake_agent.py:61` | miss | The planted code catches Exception and only passes, so S2 is genuinely present, but weed reported no finding. |
| T1 | go | cobra | `3f3b81882` | `doc/man_examples_test.go` | miss | The diff deletes the ExampleGenManTree test/example function at the planted file, and weed reported no finding for that site. |
| T1 | ts | tend2 | `8d939ec15` | `test/renderer-fresh.test.ts` | miss | The diff deletes the `it(dir, ...)` test case at the planted site, but weed reported only T2 there and did not report T1. |
| T4 | rs | tilth | `5b0539e6a` | `src/mcp/write.rs:148` | not-a-case | The diff only adds a new 10-second deadline at the planted site; it does not show an existing timeout widened from 1 to 10. |
| T4 | ts | pleach | `3a1306011` | `test/loop/run-work.test.ts:10` | miss | The planted change widens a test timeout from 1000 to 10000 at the specified site, and weed did not report T4 there. |
| T6 | ts | tend2 | `044912212` | `test/route.test.ts:143` | miss | The test changed toThrow(/sample/) to bare toThrow(), weakening the error assertion at the planted site, and weed reported no finding. |
| T6 | ts | tend2 | `32df8ddd1` | `test/verify.test.ts:176` | miss | The planted diff weakens toThrow(/description/) into toThrow(), so the error assertion stopped naming the error and weed reported no finding. |
| T6 | ts | tend2 | `42c840a73` | `test/verify.test.ts:176` | miss | The assertion changed from toThrow(/description/) to bare toThrow(), weakening the error assertion at the planted line, and weed reported no finding. |
| T6 | ts | tend2 | `51035a5c8` | `test/verify.test.ts:176` | miss | The planted line changes toThrow(/description/) to bare toThrow(), weakening the error assertion by no longer naming the expected error, and weed reported nothing. |
| T6 | ts | tend2 | `56072cdab` | `test/verify.test.ts:176` | miss | The test assertion was weakened from checking a specific /description/ error to any thrown error, and weed did not report T6 at test/verify.test.ts:176. |
| T6 | ts | tend2 | `5b1f7a471` | `test/verify.test.ts:176` | miss | The test changed from asserting a thrown error matching /description/ to merely asserting any throw, and weed reported no finding at the planted site. |
| T6 | ts | tend2 | `8d939ec15` | `test/route.test.ts:143` | miss | The assertion changed from toThrow(/sample/) to bare toThrow(), weakening the error check, and weed reported no finding there. |
| T6 | ts | tend2 | `99296a937` | `test/verify.test.ts:176` | miss | The diff weakens toThrow(/description/) to toThrow(), so the error assertion stopped naming the error and weed reported no finding. |
| T6 | ts | tend2 | `e2adbab97` | `test/verify.test.ts:176` | miss | The diff weakens the assertion from toThrow(/description/) to toThrow() at the planted line, and weed reported no finding there. |
| T6 | ts | tend2 | `e9e8387e9` | `test/verify.test.ts:176` | miss | The planted diff weakens the assertion from toThrow(/description/) to bare toThrow(), and weed did not report T6 at test/verify.test.ts:176. |

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
| blocked commits | 20 | 14 | 70.0% |
| recall cases | 20 | 20 | 100.0% |

## Blocked Commit Sample

| Repo | Commit | Auditor verdict | Reasoning |
|---|---|---|---|
| copeca | `70d669542a` | false-positive | The added line assigns a loaded key object to variable `key`, not a literal secret-looking string or committed credential. |
| pleach | `624529b3a9` | false-positive | The added private-key text is a regex pattern and a test fixture for detecting secrets, not an actual private key or credential added to the codebase. |
| tend2 | `067730ddd0` | true-positive | The test removed an assertion comparing dist/loop.js to dist/loop.global.js, reducing checks from five to four and weakening coverage of bundle equivalence. |
| tend2 | `3adc642ce1` | false-positive | No test case disappeared; two former cases were consolidated into one asserting both dist JS and CSS freshness, while a second describe still checks sibling copies. |
| tend2 | `52b97e43ce` | false-positive | The flagged values are CSS class-like identifiers, not secret-looking credentials, and no diff evidence shows an actual key or credential was added. |
| tend2 | `bf2754689c` | acceptable | The season test file and its four cases were deleted, but the command was intentionally removed and replacement coverage was added for the folded `next` orientation behavior. |
| tilth | `10bec56a41` | acceptable | An assertion was removed, but the remaining is_ok check still covers the intended behavior while the deleted body substring check was explicitly brittle. |
| tilth | `11aef933c9` | acceptable | The deleted file did contain eight tests, but the change replaces the tool with tilth_list and adds corresponding new tests, so the deletion is intentional friction rather than an actual coverage loss. |
| tilth | `18eb6643ff` | acceptable | Assertions were removed from src/mcp/mod.rs, but the affected tests were relocated into narrower module test blocks, preserving the checks rather than weakening coverage. |
| tilth | `3ff87caf55` | acceptable | A sizing test and its assertions were removed, but they targeted private fields of the deleted in-house BloomFilter implementation and no longer applied after replacing it with fastbloom. |
| tilth | `59c87110ab` | true-positive | The diff deletes the percent_decode_basic test and its four assertions, removing coverage for normal, encoded, and malformed percent-decoding behavior. |
| tilth | `5a4edbf5c5` | acceptable | Assertions did leave callers.rs, but the deleted tests were moved into new scope and callee_query modules, so behavior remained covered rather than actually weakened. |
| tilth | `7684e99e86` | acceptable | An assertion was removed, but the test still checks truncation and directly verifies body content survives, replacing the internal line-number proxy with the observable behavior. |
| tilth | `96cd4b383b` | acceptable | The search test did drop an assertion, but it removed an unreliable body substring check while preserving the success check and documenting that exact scope behavior is covered elsewhere. |
| tilth | `ab7f054b73` | true-positive | The deleted test directly asserted the paths-only schema contract, and the change reintroduced singular path and removed required paths, so real coverage and assertions were weakened. |
| tilth | `bd36a43637` | acceptable | Six tests were deleted with the removed symbol index module, but they covered code that was also removed rather than leaving surviving behavior untested. |
| tilth | `d422a325fc` | acceptable | One assertion was removed, but the same opencode local-format behavior is still checked through the new JsonLocal enum match and entry-shape tests. |
| umbel | `059b4c4677` | acceptable | The changed unit test file does have fewer assertions, but the removals correspond to obsolete cwd hook/providerFiles behavior replaced by new CODEX_HOME coverage rather than an untested behavior gap. |
| umbel | `75523a16a3` | true-positive | The added production provider contains an explicit TODO about unverified transcript tool-call parsing, leaving known incomplete behavior in shipped code. |
| umbel | `89c088bc08` | true-positive | The changed test file genuinely drops an Array.isArray assertion, reducing explicit checks from 18 to 17, even though the new safer test still verifies the main behavior. |

## Recall Case Sample

| Rule | Language | Repository | Commit | Path | Auditor verdict | Reasoning |
|---|---|---|---|---|---|---|
| D2 | go | hcl | `6a91a7547` | `gohcl/types.go:6` | miss | The planted import adds gohcl's forbidden dependency on hcldec, and weed reported no finding at that site. |
| D2 | go | hcl | `6bf1a67a9` | `gohcl/decode.go:6` | miss | The diff adds an hcldec import inside gohcl/decode.go at the planted site, matching forbidden boundary D2, and weed reported no finding. |
| D2 | go | hcl | `9466647a1` | `hclwrite/ast_block.go:6` | miss | The diff adds an hclwrite import of integrationtest at the planted site, but weed only reported an unrelated workflow warning. |
| D2 | go | hcl | `ab1acc486` | `hclwrite/tokens.go:6` | miss | The diff adds an integrationtest import inside hclwrite/tokens.go, matching the forbidden boundary shape, and weed reported no finding. |
| D2 | go | hcl | `bd45ab812` | `hclwrite/format.go:6` | miss | The diff adds an integrationtest import in hclwrite/format.go at the planted site, but weed only reported an unrelated C3 workflow finding. |
| S2 | py | copeca | `387932ad5` | `tests/e2e/fake_agent.py:61` | miss | The diff adds an except Exception handler that only passes, swallowing the error, and weed reported no finding for the planted site. |
| T1 | go | cobra | `3f3b81882` | `doc/man_examples_test.go` | miss | The diff deletes the ExampleGenManTree test/example function in doc/man_examples_test.go, and weed reported no finding for the planted site. |
| T1 | ts | tend2 | `8d939ec15` | `test/renderer-fresh.test.ts` | miss | The diff deletes the `it(dir, ...)` test case at the planted file, while weed only reports T2 assertion removal there, not T1. |
| T4 | rs | tilth | `5b0539e6a` | `src/mcp/write.rs:148` | miss | The test deadline uses Duration::from_secs(10), matching the planted wait widening, and weed only reported S2 in another file. |
| T4 | ts | pleach | `3a1306011` | `test/loop/run-work.test.ts:10` | miss | The planted change widens TIMEOUT from 1000 to 10000 at the stated site, and weed did not report T4 there. |
| T6 | ts | tend2 | `044912212` | `test/route.test.ts:143` | miss | The planted line weakens toThrow from matching /sample/ to no named error assertion, and weed reported no finding there. |
| T6 | ts | tend2 | `32df8ddd1` | `test/verify.test.ts:176` | miss | The test changed from asserting a thrown error matching /description/ to merely asserting any throw, and weed reported no finding. |
| T6 | ts | tend2 | `42c840a73` | `test/verify.test.ts:176` | miss | The assertion changed from toThrow(/description/) to bare toThrow(), weakening the error check, and weed reported no finding there. |
| T6 | ts | tend2 | `51035a5c8` | `test/verify.test.ts:176` | miss | The assertion changed from checking a specific /description/ error to any thrown error, and weed reported no finding at the planted site. |
| T6 | ts | tend2 | `56072cdab` | `test/verify.test.ts:176` | miss | The planted test changed toThrow(/description/) to bare toThrow(), weakening the error assertion, and weed reported no T6 finding at test/verify.test.ts:176. |
| T6 | ts | tend2 | `5b1f7a471` | `test/verify.test.ts:176` | miss | The planted change weakens the assertion from checking /description/ to any thrown error, and weed reported no finding there. |
| T6 | ts | tend2 | `8d939ec15` | `test/route.test.ts:143` | miss | The changed assertion went from requiring an error matching /sample/ to accepting any thrown error, and weed reported no finding. |
| T6 | ts | tend2 | `99296a937` | `test/verify.test.ts:176` | miss | The assertion changed from toThrow(/description/) to bare toThrow(), weakening the error check at the planted line, and weed reported no finding. |
| T6 | ts | tend2 | `e2adbab97` | `test/verify.test.ts:176` | miss | The assertion changed from toThrow(/description/) to bare toThrow(), weakening the error check at the planted line, and weed reported no finding. |
| T6 | ts | tend2 | `e9e8387e9` | `test/verify.test.ts:176` | miss | The assertion changed from requiring an error matching /description/ to accepting any thrown error, and weed did not report T6 at test/verify.test.ts:176. |

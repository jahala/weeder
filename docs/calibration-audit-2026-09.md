# calibration audit, 2026-09

Provider: opencode (`opencode run --model deepseek/deepseek-v4-pro --format json`, one fresh session per case)

Model: deepseek/deepseek-v4-pro

Blind: no; the auditor could read the builder's classification and reasoning in docs/calibration-2026-09.md before judging.

Seed: calibration-audit-blind-2026-09-dogfood-4

Sessions: fixtures/adversarial/calibration-audit/sighted-2026-09/sessions

The sample is the seeded one `cargo xtask audit-packet --seed calibration-audit-blind-2026-09-dogfood-4` draws from the report, so this re-grade and the blind one beside it sample the same cases and differ only in what the auditor could see. `scripts/audit/sighted-run.sh` handed each case to a fresh `opencode` session running `deepseek/deepseek-v4-pro`, in an empty directory with plugins off, and the report itself on stdin as the session's only input. Each session record names the provider and the model, each answer opens with the SHA-256 of the report it was given, and the raw event stream is kept beside it. The agreement below is recomputed by `scripts/check/calibration-agreement.sh` from these tables and the report's own.

## Agreement

| Sample | Re-graded | Agreed | Agreement |
|---|---:|---:|---:|
| blocked commits | 20 | 20 | 100.0% |
| recall cases | 20 | 20 | 100.0% |

## Blocked Commit Sample

| Repo | Commit | Auditor verdict | Reasoning |
|---|---|---|---|
| pleach | `2515d2ac88` | acceptable | C1's claim is true—the commit writes the seven-rule non-negotiables section into a file that had none—but it adds law rather than loosens anything, so the block is only intended friction. |
| pleach | `624529b3a9` | false-positive | The hits are pleach's own secret-detector regex for a private-key header and its test fixture, not an actual credential, so nothing was published and nothing needs rotation. |
| tend2 | `08529d5f56` | acceptable | T1's claim is true — thousands of test cases left — but the code they covered left in the same commit, so nothing still shipped lost coverage; a gate on a deletion this size is intended friction. |
| tend2 | `0bfa87f287` | acceptable | The rule's claim is true—the guardrail section moved and demoted a heading—but its text is unchanged word for word, so nothing was weakened; the block is the friction C1 is meant to create. |
| tend2 | `28cc632070` | true-positive | A raw NUL was typed into a template literal where `\0` belonged, so git treats the file as binary and hides later diffs, genuinely weakening review. |
| tend2 | `2bb9d83f23` | acceptable | T2's claim is true (23 claims to 19), but the test was rewritten to assert the newly-elided sections, so nothing is watched less. |
| tend2 | `78fed73b0c` | acceptable | T1's claim is true, tests left the tree, but the killed spike's non-test code left in the same commit, so nothing is watched less. |
| tend2 | `ae7e0ddf32` | true-positive | A raw NUL typed instead of the `\0` escape in src/run/ingest.ts makes git read the file as binary, hiding every later diff from review. |
| tend2 | `bf2754689c` | acceptable | T1's claim is true — the season test file lost four cases — but the command it tested was removed and folded into a unified command with its own tests, so coverage was not weakened. |
| tilth | `11aef933c9` | acceptable | T1's claim is true — files.rs and its eight cases were deleted — but the same commit's list.rs adds eight equivalent cases, so coverage merely moved, nothing was weakened. |
| tilth | `3ff87caf55` | acceptable | The claim is true—tests asserting private fields of the hand-written BloomFilter were dropped—but replacing it with fastbloom left nothing for them to hold, so the suite shrank legitimately. |
| tilth | `59c87110ab` | acceptable | The claim is true — a case and four assertions left src/mcp.rs — but they pinned a hand-rolled percent decoder this commit hands to the percent-encoding crate, so the behaviour is now the library's. |
| tilth | `7684e99e86` | acceptable | The assertion went with a fork-only API call the change removed, and the regression stays guarded because a zero cut leaves no x to find, so nothing was genuinely weakened. |
| tilth | `ab7f054b73` | true-positive | The commit removed a case and three assertions pinning the paths-only tilth_read schema, nothing replaced them, and the schema now stands unpinned. |
| tilth | `bd36a43637` | acceptable | T1's claim is true — six test cases left with the deleted symbol.rs — but they exercised a dead SymbolIndex type that was never consulted, so the removal lost no real coverage. |
| tilth | `d422a325fc` | acceptable | The assertion was genuinely dropped, but the entry_style behaviour it held is now asserted via the JsonLocal match in the same case, so nothing weakened. |
| umbel | `75523a16a3` | true-positive | The TODO is present in production code and admits field extraction is inferred and unverified, so unfinished parsing shipped as real behaviour. |
| umbel | `89c088bc08` | acceptable | T2's claim is true (18 assertions to 17), but the dropped `Array.isArray` check was redundant beside a `toEqual([])` that already implies it, so nothing was weakened. |
| umbel | `cc3cc0c37c` | true-positive | S1's claim holds: both TODOs reached production code naming the tool-call event shape as unverified against a real transcript, so unfinished parsing shipped as behaviour. |
| umbel | `e39da470b7` | acceptable | C1's claim is true—the file's non-negotiable principles arrived with this commit—but nothing existed to loosen, so the block is friction the rule was designed to create. |

## Recall Case Sample

| Rule | Language | Repository | Commit | Path | Auditor verdict | Reasoning |
|---|---|---|---|---|---|---|
| S2 | py | copeca | `387932ad5` | `tests/e2e/fake_agent.py:61` | miss | The planted handler catches an exception and says nothing, which is exactly the swallowed-error shape S2 targets, and weeder did not report it there. |
| S2 | rs | anyhow | `2cbb4e1bc` | `src/error.rs:544` | miss | A Result failure converted to a default value silently discards the error, genuinely matching the error-swallowing shape S2 detects, and weeder reported nothing at the planted site. |
| S2 | rs | anyhow | `30e1a6f73` | `src/error.rs:544` | miss | Turning a failure into a default silently discards the error, which is S2's error-swallowing shape, and weeder did not report it at src/error.rs:544. |
| S2 | rs | anyhow | `cfcc2793b` | `src/error.rs:544` | miss | The planted shape turns a failed Result into a default, discarding the error, which is S2's shape, and weeder reported nothing there. |
| T1 | go | cobra | `3f3b81882` | `doc/man_examples_test.go` | miss | The planted shape deleted an example test function, `ExampleGenManTree`, from a Go test file, which is exactly T1's test-deletion shape, and weeder did not report it there. |
| T2 | go | hcl | `6769f8b19` | `hclwrite/round_trip_test.go` | miss | The record shows an assertion deleted from a Go test file, which is T2's exact shape, and weeder did not report it there. |
| T2 | ts | umbel | `97dfce245` | `test/integration/gemini-provider.test.ts` | miss | The report records a deleted assertion at line 226 of the changed test file with no T2 fire there, matching the rule's dropped-assertion shape. |
| T4 | rs | tilth | `5b0539e6a` | `src/mcp/write.rs:148` | miss | A wait value widened from 1 to 10 is a timeout being widened, which is T4's shape, and weeder reported nothing at that site. |
| T4 | ts | pleach | `3a1306011` | `test/loop/run-work.test.ts:10` | miss | Widening a wait from 1000 to 10000 is a timeout being widened, exactly T4's shape, and weeder did not report it at that site. |
| T6 | ts | tend2 | `044912212` | `test/route.test.ts:143` | miss | The planted shape, an error assertion that stopped naming the error, is squarely T6's "weakened error assertion," and weeder reported nothing at that site. |
| T6 | ts | tend2 | `32df8ddd1` | `test/verify.test.ts:176` | miss | The T6 shape, an error assertion no longer naming the error, was planted at that site and weeder did not report it. |
| T6 | ts | tend2 | `42c840a73` | `test/verify.test.ts:176` | miss | The plant changed an error assertion to stop naming the error, the canonical T6 weakening, and weeder reported nothing at that site. |
| T6 | ts | tend2 | `56072cdab` | `test/verify.test.ts:176` | miss | The planted change weakens an error assertion by dropping the named error, which is T6's shape, and weeder reported nothing at that site. |
| T6 | ts | tend2 | `5b1f7a471` | `test/verify.test.ts:176` | miss | T6 flags weakened error assertions, and an assertion that stops naming its error is that shape planted at the site; weeder reported nothing there, so it is a genuine miss. |
| T6 | ts | tend2 | `6d9a1cf91` | `test/route.test.ts:143` | miss | An error assertion that stops naming its error is exactly a weakened error assertion, the planted shape matches T6, and zero unplantable cases means it was genuinely present and unreported. |
| T6 | ts | tend2 | `99296a937` | `test/verify.test.ts:176` | miss | The planted change — an error assertion dropping its named error — is exactly T6's shape, and weeder did not report it at the site. |
| T6 | ts | tend2 | `ad070a0d4` | `test/verify.test.ts:176` | miss | T6 catches weakened error assertions, and dropping the error name from an assertion is exactly that weakening, present at the site and unreported. |
| T6 | ts | tend2 | `e2adbab97` | `test/verify.test.ts:176` | miss | The planted change removed the error name from an error assertion, which is exactly T6's "error assertion weakened" shape, and the report records weeder reporting nothing at that site. |
| T6 | ts | tend2 | `e9e8387e9` | `test/verify.test.ts:176` | miss | T6 targets weakened error assertions; this planted change dropped the error name, the rule's shape, and weeder did not report it at that site. |
| T6 | ts | tend2 | `fadcb3808` | `test/verify.test.ts:176` | miss | An error assertion that stops naming the error is a genuine weakening of that assertion, the T6 shape, and weeder did not report it at the planted site. |

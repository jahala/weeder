# calibration audit, 2026-09

Provider: codex, OpenAI GPT-5 Codex (`codex exec`, one fresh session per case)

Blind: no; the auditor could read the builder's classification and reasoning in docs/calibration-2026-09.md before judging.

Seed: calibration-audit-blind-2026-09-t2-1

Sessions: fixtures/adversarial/calibration-audit/sighted-2026-09/sessions

The sample is the seeded one `cargo xtask audit-packet --seed calibration-audit-blind-2026-09-t2-1` draws from the report, so this re-grade and the blind one beside it sample the same cases and differ only in what the auditor could see. `scripts/audit/sighted-run.sh` handed each case to a fresh `codex exec` session in an empty directory, with the user's configuration and rules ignored and the sandbox read-only, and the report itself as the session's only input. Each answer opens with the SHA-256 of the report it was given, and the raw event stream is kept beside it. The agreement below is recomputed by `scripts/check/calibration-agreement.sh` from these tables and the report's own.

## Agreement

| Sample | Re-graded | Agreed | Agreement |
|---|---:|---:|---:|
| blocked commits | 20 | 20 | 100.0% |
| recall cases | 20 | 20 | 100.0% |

## Blocked Commit Sample

| Repo | Commit | Auditor verdict | Reasoning |
|---|---|---|---|
| copeca | `70d669542a` | false-positive | X1 flagged a local variable named key holding a loaded public key object, not an added secret or credential, so the rule's claim is not true. |
| pleach | `624529b3a9` | false-positive | X1 matched pleach's own private-key detector regex and fixture, not an added credential, so the claimed secret publication was not true of the change. |
| tend2 | `067730ddd0` | acceptable | T2 correctly caught one assertion removed, but it only checked parity with a generated file the commit stopped emitting, while remaining tests still cover the surviving outputs. |
| tend2 | `08529d5f56` | acceptable | T1’s claim is true because many tests were deleted, but they covered the removed v1 lane, so the remaining shipped tend2 surface was not weakened. |
| tend2 | `3adc642ce1` | false-positive | T1 counted fewer it() declarations, but the removed case was converted into a loop over sibling directories, so the tested behavior remained covered. |
| tend2 | `52b97e43ce` | false-positive | X1 mistook CSS class names ending in `__key` inside generated HTML markup for secret assignments; no credential or secret-looking value was actually added. |
| tend2 | `78fed73b0c` | acceptable | T1 correctly identified deleted spike tests, but the corresponding killed spike implementation was removed in the same commit, so no remaining shipped behavior lost coverage. |
| tend2 | `bf2754689c` | acceptable | T1's claim is true because season.test.ts and its four cases were deleted, but the tested command was removed and folded into a replacement command with tests. |
| tilth | `10bec56a41` | acceptable | T2's assertion-count drop is true, but the removed substring check was flaky real-output coupling while other tests still pin scope behavior and this case still checks success. |
| tilth | `11aef933c9` | acceptable | T1 correctly caught the deletion/count drop, but equivalent tilth_list tests were added over the same behavior, so the change was fine despite warranted review friction. |
| tilth | `3ff87caf55` | acceptable | T1/T2 accurately flagged lost bloom tests and assertions, but they targeted private internals removed by replacing the implementation with fastbloom, so the block was review friction rather than a real weakening. |
| tilth | `59c87110ab` | acceptable | The test and assertions really were removed, but they covered a hand-rolled percent decoder replaced by a dedicated crate, so the block is useful review friction rather than a real weakening. |
| tilth | `7684e99e86` | acceptable | T2’s assertion-drop claim is true, but the removed assertion depended on a fork-only API while the remaining test still guards the regression through the output invariant. |
| tilth | `ab7f054b73` | true-positive | T1 and T2 accurately identify removed coverage and assertions, and the paths-only tilth_read schema was reopened to singular path without replacement tests, leaving real behavior unpinned. |
| tilth | `bd36a43637` | acceptable | T1 correctly flagged deletion of six tests, but they covered dead SymbolIndex plumbing removed in the same commit, so no surviving behavior was weakened. |
| tilth | `d422a325fc` | acceptable | T2 correctly identified one dropped assertion, but it covered a removed entry_style field now represented and asserted through ConfigFormat::JsonLocal in the same test. |
| umbel | `059b4c4677` | acceptable | T2 correctly caught three dropped assertions, but they pinned the superseded per-worker hooks path while the shared CODEX_HOME design gained replacement coverage in the same and added test files. |
| umbel | `75523a16a3` | true-positive | S1's claim is true: a TODO admitting inferred, unverified tool-call parsing reached production OpenCode provider code, weakening shipped behavior by leaving parser correctness unfinished. |
| umbel | `89c088bc08` | acceptable | T2 correctly detected one fewer assertion, but the removed Array.isArray check was redundant with toEqual([]) while the test became safer by isolating tmux state. |
| umbel | `cc3cc0c37c` | true-positive | S1’s claim is true: production provider code shipped TODOs admitting unverified tool-call event shapes and partial extraction, leaving real behavior unfinished and weakened. |

## Recall Case Sample

| Rule | Language | Repository | Commit | Path | Auditor verdict | Reasoning |
|---|---|---|---|---|---|---|
| D2 | go | hcl | `2efc26623` | `hclwrite/ast_body.go:6` | miss | The planted import makes hclwrite depend on integrationtest at ast_body.go:6, matching D2's forbidden-boundary shape, and the report lists it as planted but unreported. |
| D2 | go | hcl | `6bf1a67a9` | `gohcl/decode.go:6` | miss | The report says gohcl/decode.go was planted with a forbidden gohcl-to-hcldec import at line 6 and listed it under Every miss, so weed did not report the planted D2 shape there. |
| D2 | go | hcl | `9466647a1` | `hclwrite/ast_block.go:6` | miss | The report says D2 planted a forbidden hclwrite import of integrationtest at that file and line, and Every miss means weed did not report it there. |
| D2 | go | hcl | `ab1acc486` | `hclwrite/tokens.go:6` | miss | The report says hclwrite/tokens.go was made to import integrationtest, which is a forbidden boundary crossing for D2, and it was planted but not reported. |
| D2 | go | hcl | `e73f21667` | `gohcl/schema.go:6` | miss | The report says the planted site made gohcl import hcldec, a D2 forbidden-boundary import, and weed did not report it there. |
| S2 | py | copeca | `387932ad5` | `tests/e2e/fake_agent.py:61` | miss | The report says an S2 shape was planted there: a handler catches an error and does nothing, and weed did not report it at that site. |
| T1 | go | cobra | `3f3b81882` | `doc/man_examples_test.go` | miss | The report states the T1 mutation deleted Go test example `ExampleGenManTree` in `doc/man_examples_test.go` and weed did not report that planted deletion. |
| T1 | ts | tend2 | `6d9a1cf91` | `test/site-paths.test.ts` | miss | The report says the T1 shape was planted by deleting the `${dir} (${pages.length} pages)` test case in that file and weed did not report it there. |
| T1 | ts | tend2 | `8d939ec15` | `test/renderer-fresh.test.ts` | miss | The report says the T1 shape was planted in that file by deleting an `it` case and was not reported there. |
| T4 | ts | pleach | `3a1306011` | `test/loop/run-work.test.ts:10` | miss | The report says T4 planted a genuine wait widening from 1000 to 10000 at that file and line, and lists it under Every miss as not reported. |
| T6 | ts | tend2 | `044912212` | `test/route.test.ts:143` | miss | The report says T6 was planted at test/route.test.ts:143 by removing the named error assertion, and Every miss means weed did not report it there. |
| T6 | ts | tend2 | `2a0e93933` | `test/verify.test.ts:176` | miss | The report says the T6 shape was planted at that exact file and line, stopped naming the error, and was not reported there. |
| T6 | ts | tend2 | `51035a5c8` | `test/verify.test.ts:176` | miss | The report says T6 was planted at test/verify.test.ts:176 by weakening an error assertion, and lists it under planted cases not reported there. |
| T6 | ts | tend2 | `56072cdab` | `test/verify.test.ts:176` | miss | The report lists this exact T6 planted site under Every miss: the assertion stopped naming the error, and weed did not report it there. |
| T6 | ts | tend2 | `6d9a1cf91` | `test/route.test.ts:143` | miss | The report lists this planted T6 case under Every miss, describing an error assertion at test/route.test.ts:143 that stopped naming the error and was not reported there. |
| T6 | ts | tend2 | `8d939ec15` | `test/route.test.ts:143` | miss | The report lists this planted T6 case under Every miss, saying the error assertion stopped naming the error and weed did not report it at that site. |
| T6 | ts | tend2 | `99296a937` | `test/verify.test.ts:176` | miss | The report says this planted T6 case changed `test/verify.test.ts:176` so an error assertion stopped naming the error, and lists it under “Every miss,” meaning weed did not report it there. |
| T6 | ts | tend2 | `ad070a0d4` | `test/verify.test.ts:176` | miss | The report says this T6 site was planted, the error assertion stopped naming the error, and weed did not report it there. |
| T6 | ts | tend2 | `e2adbab97` | `test/verify.test.ts:176` | miss | The report lists this T6 site under Every miss, saying the planted error assertion stopped naming the error and weed did not report it there. |
| T6 | ts | tend2 | `e9e8387e9` | `test/verify.test.ts:176` | miss | The report says this planted T6 case genuinely weakened an error assertion at the target line and appears under Every miss, meaning weed did not report it there. |

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
| copeca | `70d669542a` | false-positive | X1 mistook a local variable named key holding a loaded public key for a published secret, but no secret value or credential was added. |
| pleach | `624529b3a9` | false-positive | X1 matched a private-key regex and fixture used to test pleach's own secret detector, not an actual credential published in the commit. |
| tend2 | `067730ddd0` | acceptable | T2 correctly caught an assertion count drop, but the removed check covered a generated file no longer emitted, while remaining renderer freshness checks still cover current outputs. |
| tend2 | `08529d5f56` | acceptable | T1 correctly flagged massive test deletion, but the tested v1 adapters, CLI, and core were removed from the shipped product in the same sunset commit. |
| tend2 | `3adc642ce1` | false-positive | T1 counted fewer it() declarations, but the test case was converted into a loop over sibling directories, so the behavior remained covered rather than unwatched. |
| tend2 | `52b97e43ce` | false-positive | X1 mistook HTML class names `detail-meta__key` and `journey-row__key` for assigned secret values; the reported values are markup, not credentials. |
| tend2 | `78fed73b0c` | acceptable | T1’s claim is true because loop-hole test files and cases were deleted, but the spike code they covered was removed in the same cleanup, so coverage was not weakened. |
| tend2 | `bf2754689c` | acceptable | T1’s deleted-test claim is true, but the tested season command was removed as orientation folded into a single command with replacement tests, so the block is intended review friction. |
| tilth | `10bec56a41` | acceptable | T2 correctly saw an assertion count drop, but the removed substring check was flaky search-output coupling while behavior remained pinned elsewhere and success propagation stayed tested. |
| tilth | `11aef933c9` | acceptable | T1 correctly caught deleted tests, but the commit replaced them with equivalent tilth_list cases covering the same behavior, so the change did not meaningfully weaken coverage. |
| tilth | `3ff87caf55` | acceptable | T1/T2 correctly flagged a removed case and assertions, but they only exercised private internals of a hand-written BloomFilter replaced by fastbloom, so the shrink was review-worthy friction rather than a real weakening. |
| tilth | `59c87110ab` | acceptable | T1/T2 truthfully caught a dropped decoder test and four assertions, but the covered hand-rolled behavior moved to a maintained percent-encoding library, so the block was review friction rather than a real weakening. |
| tilth | `7684e99e86` | acceptable | T2 correctly caught an assertion drop, but the removed check depended on a fork-only API while the remaining test still guards the regression. |
| tilth | `ab7f054b73` | true-positive | The report says the deleted test and assertions were not replaced, leaving the paths-only tilth_read schema unpinned and allowing singular path behavior again. |
| tilth | `bd36a43637` | acceptable | T1 correctly reports six deleted tests, but they covered removed inert SymbolIndex plumbing that searches never consulted, so the deletion did not weaken live behavior coverage. |
| tilth | `d422a325fc` | acceptable | T2 correctly flags one dropped assertion, but it checked a field removed by the refactor, and the replacement JsonLocal behavior remains asserted in the same case. |
| umbel | `059b4c4677` | acceptable | T2 correctly identified three dropped assertions, but they pinned the old cwd-local hooks design replaced by shared CODEX_HOME and covered by replacement tests. |
| umbel | `75523a16a3` | true-positive | S1's claim is true: a production TODO admits unverified inferred tool-call parsing, so the new provider shipped incomplete behavior that could mishandle real transcripts. |
| umbel | `89c088bc08` | acceptable | T2 correctly saw one assertion removed, but it was redundant with `toEqual([])` and the rewritten test preserves coverage while avoiding destructive tmux cleanup. |
| umbel | `cc3cc0c37c` | true-positive | S1’s claim is true: production provider code shipped TODOs admitting unverified tool-call event shapes and partial extraction, so the change knowingly left incomplete parsing behavior in place. |

## Recall Case Sample

| Rule | Language | Repository | Commit | Path | Auditor verdict | Reasoning |
|---|---|---|---|---|---|---|
| D2 | go | hcl | `2efc26623` | `hclwrite/ast_body.go:6` | miss | The report says the D2 shape was planted at hclwrite/ast_body.go:6 by importing integrationtest from hclwrite, and Every miss says it was not reported there. |
| D2 | go | hcl | `6bf1a67a9` | `gohcl/decode.go:6` | miss | The report says the planted D2 shape made gohcl/decode.go import hcldec, a forbidden boundary import, and lists it as planted but not reported. |
| D2 | go | hcl | `9466647a1` | `hclwrite/ast_block.go:6` | miss | The report says hclwrite was mutated to import integrationtest at that file and line, a forbidden-boundary import, and lists it under planted cases not reported. |
| D2 | go | hcl | `ab1acc486` | `hclwrite/tokens.go:6` | miss | The report says the D2 shape was planted at hclwrite/tokens.go:6 by importing integrationtest, a forbidden boundary, and it was not reported there. |
| D2 | go | hcl | `e73f21667` | `gohcl/schema.go:6` | miss | The report says gohcl/schema.go:6 was planted with a gohcl import of hcldec, matching D2's forbidden-boundary shape, and Every miss means weeder did not report it there. |
| S2 | py | copeca | `387932ad5` | `tests/e2e/fake_agent.py:61` | miss | The report says S2’s swallowed-error handler was planted at that exact Python site and was not reported there, so it remains a genuine miss. |
| T1 | go | cobra | `3f3b81882` | `doc/man_examples_test.go` | miss | ExampleGenManTree is a Go example test in a _test.go file, so deleting it is genuinely a T1 test deletion, and the report says weeder did not report it. |
| T1 | ts | tend2 | `6d9a1cf91` | `test/site-paths.test.ts` | miss | The report says the planted deletion removed a real test case in `test/site-paths.test.ts` and weeder did not report T1 at that planted site. |
| T1 | ts | tend2 | `8d939ec15` | `test/renderer-fresh.test.ts` | miss | The report says this T1 case was planted by deleting `it` in the named test file and was not reported, with unplantable cases excluded. |
| T2 | py | copeca | `70d669542` | `tests/config/test_mode_models.py` | miss | The report lists this planted T2 case as an assertion deletion in a test file and says it was planted but not reported. |
| T4 | ts | pleach | `3a1306011` | `test/loop/run-work.test.ts:10` | miss | The report says T4 planted a real wait widening from 1000 to 10000 at that TypeScript test site and lists it under Every miss as not reported. |
| T6 | ts | tend2 | `044912212` | `test/route.test.ts:143` | miss | The report says the T6 shape was planted at test/route.test.ts:143 in 044912212 and not reported, with the error assertion weakened by no longer naming the error. |
| T6 | ts | tend2 | `2a0e93933` | `test/verify.test.ts:176` | miss | The report says the T6 shape was planted at test/verify.test.ts:176 in 2a0e93933 and not reported there. |
| T6 | ts | tend2 | `51035a5c8` | `test/verify.test.ts:176` | miss | The report says T6 was planted at test/verify.test.ts:176 by weakening an error assertion, and lists it under Every miss as not reported. |
| T6 | ts | tend2 | `6d9a1cf91` | `test/route.test.ts:143` | miss | The report says the T6 shape was planted at test/route.test.ts:143 by removing the named error assertion, and Every miss means weeder did not report it there. |
| T6 | ts | tend2 | `8d939ec15` | `test/route.test.ts:143` | miss | The report says the T6 shape was planted at test/route.test.ts:143 by weakening an error assertion, and Every miss means weeder did not report it there. |
| T6 | ts | tend2 | `99296a937` | `test/verify.test.ts:176` | miss | The report lists this exact T6 planted site under Every miss, saying the error assertion stopped naming the error and was not reported there. |
| T6 | ts | tend2 | `ad070a0d4` | `test/verify.test.ts:176` | miss | The report lists this exact T6 site under Every miss as planted and not reported, with the error assertion weakened by no longer naming the error. |
| T6 | ts | tend2 | `e2adbab97` | `test/verify.test.ts:176` | miss | The report says T6 was planted at test/verify.test.ts:176 in e2adbab97, the error assertion stopped naming the error, and it was not reported. |
| T6 | ts | tend2 | `e9e8387e9` | `test/verify.test.ts:176` | miss | The report says T6’s planted weakening at test/verify.test.ts:176 stopped naming the error and appears in Every miss, so weeder did not report it there. |

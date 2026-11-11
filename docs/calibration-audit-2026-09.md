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
| pleach | `2515d2ac88` | acceptable | C1's claim is true — the commit adds the non-negotiables section to a CLAUDE.md that had none, so a guardrail changed, but nothing was loosened; the block is expected friction. |
| pleach | `624529b3a9` | false-positive | X1 flags a secret-looking string, but the hit is pleach's own private-key-header regex and its test fixture, not a real credential, so no secret was published. |
| tend2 | `08529d5f56` | acceptable | The T1 claim is true—forty-odd test files left with thousands of cases—but the v1 sunset deleted the code they covered in the same commit, so nothing still shipped is less covered; the block is intended stop-and-read friction. |
| tend2 | `0bfa87f287` | acceptable | C1's claim is true, the guardrail heading was edited and demoted, but the text is word-for-word unchanged and nothing was loosened, so the block is the friction C1 exists to create. |
| tend2 | `2bb9d83f23` | acceptable | The claim is true—the spec went from 23 to 19 claims—but the test was rewritten to match a deliberate behavior change, so nothing is watched less. |
| tend2 | `78fed73b0c` | acceptable | T1 correctly finds the spike's test files deleted, but the spike's fifty non-test files left in the same commit, so the tested code died with its tests. |
| tend2 | `bf2754689c` | acceptable | T1's claim is true, four cases left with the removed season command, but the folded orientation command gains its own tests, so the deletion was fine. |
| tilth | `11aef933c9` | acceptable | T1 is claim-true that files.rs lost 8 cases, but the same commit adds list.rs with 8 equivalent cases, so coverage is renamed, not lost. |
| tilth | `3ff87caf55` | acceptable | The deleted case and assertions pinned private fields of the hand-written BloomFilter this commit replaces with fastbloom, so nothing remained for them to hold. |
| tilth | `59c87110ab` | acceptable | The dropped case and assertions pinned a hand-rolled percent decoder handed to the percent-encoding crate, so the covered behaviour moved into the library, not into a gap. |
| tilth | `7684e99e86` | acceptable | The dropped assertion went with a fork-only apply_with_info call, and the regression is still guarded by the remaining demand for an x in the output, so the weakening is real but benign. |
| tilth | `ab7f054b73` | true-positive | T1 and T2 are factually true, and unlike the consolidation and dead-code cases nothing picks the dropped coverage up: the live tilth_read schema is left pinned by nobody. |
| tilth | `bd36a43637` | acceptable | T1's claim is true, six cases left with the deleted file, but they exercised a dead, never-consulted SymbolIndex, so the removal lost no real coverage. |
| tilth | `d422a325fc` | acceptable | The dropped assertion held the entry_style field now folded into ConfigFormat, and the JsonLocal replacement is asserted in the same case, so nothing is watched less. |
| umbel | `059b4c4677` | acceptable | T2's claim that codex.test.ts dropped from 33 to 30 assertions is true, but the dropped assertions covered a design this commit replaced, and the new CODEX_HOME design is asserted in the same and added files. |
| umbel | `7109a41bd9` | acceptable | The non-negotiable principles did change, but only by renaming rctrl to umbel, which alters nothing they require. |
| umbel | `75523a16a3` | true-positive | The TODO genuinely sits in production code and marks tool-call parsing whose shapes are only inferred, shipping unverified behavior, which is exactly what S1 guards. |
| umbel | `89c088bc08` | acceptable | The assertion count did drop from 18 to 17, so T2's claim is true, but the removed `Array.isArray` check was redundant with the `toEqual([])` beside it and the change destroys less, so the block is proper friction. |
| umbel | `cc3cc0c37c` | true-positive | S1's claim holds: TODOs admitting unverified tool-call event parsing shipped in production code, so unfinished parsing became shipped behaviour and something was genuinely weakened. |
| umbel | `e39da470b7` | acceptable | C1's claim is true—this commit first writes CLAUDE.md, so its non-negotiables arrived unchanged from nothing, and nothing was loosened. |

## Recall Case Sample

| Rule | Language | Repository | Commit | Path | Auditor verdict | Reasoning |
|---|---|---|---|---|---|---|
| S2 | py | copeca | `387932ad5` | `tests/e2e/fake_agent.py:61` | miss | The planted handler catches an exception and says nothing, which is exactly the S2 swallowed-error shape, so weeder should have reported it there but did not. |
| S2 | rs | anyhow | `2cbb4e1bc` | `src/error.rs:544` | miss | A failure silently turned into a default is the canonical error-swallowing shape S2 targets, planted and read back at src/error.rs:544, where weeder reported nothing. |
| S2 | rs | anyhow | `30e1a6f73` | `src/error.rs:544` | miss | "A failure was turned into a default" is a genuine error-swallowing shape at that site, and S2 did not report it there. |
| S2 | rs | anyhow | `cfcc2793b` | `src/error.rs:544` | miss | Turning a failure into a default is error-swallowing, S2's exact shape; the injector's scanner confirmed the shape present at the site, so weeder missing it is a genuine miss. |
| T1 | go | cobra | `3f3b81882` | `doc/man_examples_test.go` | miss | Deleting the `ExampleGenManTree` function from `doc/man_examples_test.go` deletes a Go example test, which is the exact shape T1 detects. |
| T2 | go | hcl | `6769f8b19` | `hclwrite/round_trip_test.go` | miss | An assertion deleted from a Go test file is precisely T2's shape, planted at that site and unreported, so weeder genuinely missed it. |
| T2 | ts | umbel | `97dfce245` | `test/integration/gemini-provider.test.ts` | miss | T2 flags assertions dropped from a changed test file, and the planted case deletes line 226's assertion in gemini-provider.test.ts, so the shape is present and unreported. |
| T4 | rs | tilth | `5b0539e6a` | `src/mcp/write.rs:148` | miss | A wait widened from 1 to 10 matches T4's tolerance-widening shape, and weeder did not report it at that site. |
| T4 | ts | pleach | `3a1306011` | `test/loop/run-work.test.ts:10` | miss | Widening a wait from 1000 to 10000 is a tolerance/timeout widening within T4's shape, and weeder reported nothing at that site. |
| T6 | ts | tend2 | `044912212` | `test/route.test.ts:143` | miss | T6 catches an error assertion weakened, and an assertion that stopped naming the error is that shape, present at the site but not reported. |
| T6 | ts | tend2 | `32df8ddd1` | `test/verify.test.ts:176` | miss | The planted weakening — an error assertion that stopped naming the error — is genuinely present at `test/verify.test.ts:176`, unplantable is zero, and weeder did not report it there. |
| T6 | ts | tend2 | `42c840a73` | `test/verify.test.ts:176` | miss | The description matches T6's shape, an error assertion weakened by no longer naming the error, and weeder did not report it at that site. |
| T6 | ts | tend2 | `56072cdab` | `test/verify.test.ts:176` | miss | An error assertion that stops naming the specific error is a genuine weakening of that assertion, matching T6's shape, and the report lists it as unreported at the planted site. |
| T6 | ts | tend2 | `5b1f7a471` | `test/verify.test.ts:176` | miss | The planted shape, an error assertion no longer naming its error, is a genuine T6 weakening, and weeder reported nothing at that site. |
| T6 | ts | tend2 | `6d9a1cf91` | `test/route.test.ts:143` | miss | The planted change removed the error name or message from an assertion, exactly the weakened error assertion T6 detects, and weeder reported nothing at test/route.test.ts:143. |
| T6 | ts | tend2 | `99296a937` | `test/verify.test.ts:176` | miss | The T6 shape, an error assertion weakened by ceasing to name the error, is present at test/verify.test.ts:176 and weeder did not report it there. |
| T6 | ts | tend2 | `ad070a0d4` | `test/verify.test.ts:176` | miss | The planted change strips the specific error name from an assertion, which genuinely weakens it as T6 defines, and the miss log shows weeder did not report it. |
| T6 | ts | tend2 | `e2adbab97` | `test/verify.test.ts:176` | miss | Dropping the error name from an assertion is precisely T6's "weakened error assertion" shape, and weeder reported nothing at that site. |
| T6 | ts | tend2 | `e9e8387e9` | `test/verify.test.ts:176` | miss | Removing the error name from an assertion is exactly the "weakened error assertion" shape T6 detects, and weeder planted it there without reporting it. |
| T6 | ts | tend2 | `fadcb3808` | `test/verify.test.ts:176` | miss | The injected change, an error assertion that stopped naming its error, is T6's weakening shape, present at the site with no report from weeder; the fourteen identical ts misses confirm a detector gap. |

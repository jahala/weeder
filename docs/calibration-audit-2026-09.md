# calibration audit, 2026-09

Provider: codex, OpenAI GPT-5 Codex (`codex exec`, one fresh session per case)

Blind: no; the auditor could read the builder's classification and reasoning in docs/calibration-2026-09.md before judging.

Seed: calibration-audit-blind-2026-09-redo-2

Sessions: fixtures/adversarial/calibration-audit/sighted-2026-09/sessions

The sample is the seeded one `cargo xtask audit-packet --seed calibration-audit-blind-2026-09-redo-2` draws from the report, so this re-grade and the blind one beside it sample the same cases and differ only in what the auditor could see. `scripts/audit/sighted-run.sh` handed each case to a fresh `codex exec` session in an empty directory, with the user's configuration and rules ignored and the sandbox read-only, and the report itself as the session's only input. Each answer opens with the SHA-256 of the report it was given, and the raw event stream is kept beside it. The agreement below is recomputed by `scripts/check/calibration-agreement.sh` from these tables and the report's own.

## Agreement

| Sample | Re-graded | Agreed | Agreement |
|---|---:|---:|---:|
| blocked commits | 20 | 20 | 100.0% |
| recall cases | 20 | 20 | 100.0% |

## Blocked Commit Sample

| Repo | Commit | Auditor verdict | Reasoning |
|---|---|---|---|
| copeca | `70d669542a` | false-positive | The flagged `key` is a local variable holding a loaded public key object, not an added secret or credential. |
| pleach | `624529b3a9` | false-positive | The flagged private-key text was a detector regex and fixture, not an actual credential, so X1's published-secret claim was untrue. |
| tend2 | `067730ddd0` | acceptable | T2 correctly identified one dropped assertion, but it only checked byte identity for a generated file this commit stopped emitting, while remaining freshness checks still covered built outputs. |
| tend2 | `3adc642ce1` | false-positive | T1 counted fewer it() declarations, but the removed case became a loop over sibling directories, so coverage remained and no test behavior was actually lost. |
| tend2 | `52b97e43ce` | false-positive | X1 mistook HTML class names ending in __key for secret assignments; the added values were renderer markup, not credentials or secret material. |
| tend2 | `bf2754689c` | acceptable | T1 correctly flagged deletion of season.test.ts cases, but the tested season command was removed and folded into the replacement orientation command with its own tests, so coverage moved with the feature. |
| tilth | `10bec56a41` | acceptable | T2 correctly caught an assertion drop, but the removed substring check was flaky real-output coupling while the intended scope behavior remained covered elsewhere. |
| tilth | `11aef933c9` | acceptable | T1 correctly saw tests deleted from files.rs, but equivalent list.rs tests replaced them in the same commit, so the block is review friction rather than a real coverage loss. |
| tilth | `18eb6643ff` | acceptable | T2 correctly observed assertions dropped from src/mcp/mod.rs, but the same commit relocated those tests into tool modules, so coverage was preserved while still meriting review. |
| tilth | `3ff87caf55` | acceptable | The rules correctly flagged deleted tests/assertions, but they covered private internals of a replaced hand-written BloomFilter, so the block was intended review friction rather than a real weakening. |
| tilth | `59c87110ab` | acceptable | T1/T2 correctly caught the deleted decoder test and four assertions, but the hand-rolled behavior was replaced by a standard percent-encoding crate, so the review friction is justified without a real weakening. |
| tilth | `5a4edbf5c5` | acceptable | T2 correctly flagged the assertion drop in callers.rs, but the same behavior was extracted into new modules with replacement assertions, so coverage was relocated rather than weakened. |
| tilth | `7684e99e86` | acceptable | T2’s assertion-count drop is true, but the removed fork-only apply_with_info assertion was adapted away while the regression remained covered by checking that zero-budget output keeps x. |
| tilth | `96cd4b383b` | acceptable | T2 correctly identified an assertion drop in search.rs, but it removed a flaky substring check unrelated to the write containment fix while preserving the meaningful behavior. |
| tilth | `ab7f054b73` | true-positive | The commit removed the paths-only tilth_read schema test and assertions without replacement, reopening singular path behavior and weakening coverage. |
| tilth | `bd36a43637` | acceptable | T1 truly caught deletion of six SymbolIndex tests, but the tested dead allocator was removed with the inert plumbing, so coverage was not weakened for remaining behavior. |
| tilth | `d422a325fc` | acceptable | T2 correctly found one assertion drop, but the removed entry_style check was replaced by an asserted JsonLocal ConfigFormat match, so the refactor did not weaken coverage. |
| umbel | `059b4c4677` | acceptable | T2 correctly reports three assertions dropped, but they covered the old cwd-local hooks design replaced by shared CODEX_HOME coverage in existing and added tests. |
| umbel | `75523a16a3` | true-positive | The S1 claim is true: a production OpenCode provider shipped with TODO-marked, inferred tool-call parsing that explicitly needed validation against real transcripts. |
| umbel | `89c088bc08` | acceptable | T2 correctly caught one assertion being removed, but it was redundant with toEqual([]), while the test became safer by avoiding destructive tmux cleanup. |

## Recall Case Sample

| Rule | Language | Repository | Commit | Path | Auditor verdict | Reasoning |
|---|---|---|---|---|---|---|
| D2 | go | hcl | `6a91a7547` | `gohcl/types.go:6` | miss | The report says the D2 shape was planted at gohcl/types.go:6 by making gohcl import hcldec, and it was listed among planted cases weed did not report. |
| D2 | go | hcl | `6bf1a67a9` | `gohcl/decode.go:6` | miss | The report says gohcl/decode.go was planted with a gohcl-to-hcldec import, a D2 forbidden-boundary crossing, and lists it as planted but not reported. |
| D2 | go | hcl | `9466647a1` | `hclwrite/ast_block.go:6` | miss | The report says hclwrite was planted to import integrationtest at that site, matching D2's forbidden-boundary import shape, and it was not reported. |
| D2 | go | hcl | `ab1acc486` | `hclwrite/tokens.go:6` | miss | The report says the D2 shape was planted at hclwrite/tokens.go:6 as an hclwrite import of integrationtest, and this exact planted boundary violation was not reported. |
| D2 | go | hcl | `bd45ab812` | `hclwrite/format.go:6` | miss | The report says D2 was planted by making hclwrite/format.go import integrationtest, a forbidden boundary import, and weed did not report it at that planted site. |
| S2 | py | copeca | `387932ad5` | `tests/e2e/fake_agent.py:61` | miss | The report says the planted Python site added a handler that catches and says nothing, which is S2's swallowed-error shape, and it was listed as not reported. |
| T1 | go | cobra | `3f3b81882` | `doc/man_examples_test.go` | miss | The report says ExampleGenManTree was deleted from a Go test file and listed under planted-but-unreported misses, so the T1 shape was genuinely present there. |
| T1 | ts | tend2 | `8d939ec15` | `test/renderer-fresh.test.ts` | miss | The report lists this T1 recall case under planted-and-not-reported misses, and deleting an `it` test case is genuinely the T1 shape at that file. |
| T4 | rs | tilth | `5b0539e6a` | `src/mcp/write.rs:148` | miss | The planted change widened a Rust wait from 1 to 10 at the named site, matching T4, and the report lists it under misses rather than a caught finding. |
| T4 | ts | pleach | `3a1306011` | `test/loop/run-work.test.ts:10` | miss | The planted change widened a wait from 1000 to 10000 at the named test site, matching T4, and the report lists it as not reported. |
| T6 | ts | tend2 | `044912212` | `test/route.test.ts:143` | miss | The report says T6 was planted at test/route.test.ts:143 by weakening an error assertion, and every listed miss was planted and not reported there. |
| T6 | ts | tend2 | `32df8ddd1` | `test/verify.test.ts:176` | miss | The report says the T6 shape was planted at test/verify.test.ts:176 in commit 32df8ddd1 and not reported, with the assertion weakened by no longer naming the error. |
| T6 | ts | tend2 | `42c840a73` | `test/verify.test.ts:176` | miss | The report says T6 was planted at test/verify.test.ts:176 in commit 42c840a73 by weakening an error assertion, and it was not reported there. |
| T6 | ts | tend2 | `51035a5c8` | `test/verify.test.ts:176` | miss | The report states T6 was planted at that site as an error assertion no longer naming the error, and it was not reported there. |
| T6 | ts | tend2 | `56072cdab` | `test/verify.test.ts:176` | miss | The report says T6 was planted at that exact site by weakening an error assertion, and Every miss means weed did not report it there. |
| T6 | ts | tend2 | `5b1f7a471` | `test/verify.test.ts:176` | miss | The report lists this planted T6 case as an error assertion that stopped naming the error, and says every listed miss was planted and not reported. |
| T6 | ts | tend2 | `8d939ec15` | `test/route.test.ts:143` | miss | The report lists this planted T6 site as an unreported miss, and describes the genuine weakening: the error assertion stopped naming the error. |
| T6 | ts | tend2 | `99296a937` | `test/verify.test.ts:176` | miss | The report says the T6 shape was planted at test/verify.test.ts:176 by weakening an error assertion, and Every miss states it was not reported there. |
| T6 | ts | tend2 | `e2adbab97` | `test/verify.test.ts:176` | miss | The report says the T6 shape was planted at test/verify.test.ts:176 in e2adbab97 and appears under Every miss, meaning weed did not report it there. |
| T6 | ts | tend2 | `e9e8387e9` | `test/verify.test.ts:176` | miss | The report lists this exact planted T6 case under Every miss, saying the error assertion stopped naming the error and was not reported at the planted site. |

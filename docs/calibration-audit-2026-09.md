# calibration audit, 2026-09

Provider: codex, OpenAI GPT-5 Codex (`codex exec`, one fresh session per case)

Blind: no; the auditor could read the builder's classification and reasoning in docs/calibration-2026-09.md before judging.

Seed: calibration-audit-2026-09-codex

Sessions: fixtures/adversarial/calibration-audit/sighted-2026-09/sessions

The sample is the seeded one `cargo xtask audit-packet --seed calibration-audit-2026-09-codex` draws from the report, so this re-grade and the blind one beside it sample the same cases and differ only in what the auditor could see. `scripts/audit/sighted-run.sh` handed each case to a fresh `codex exec` session in an empty directory, with the user's configuration and rules ignored and the sandbox read-only, and the report itself as the session's only input. Each answer opens with the SHA-256 of the report it was given, and the raw event stream is kept beside it. The agreement below is recomputed by `scripts/check/calibration-agreement.sh` from these tables and the report's own.

## Agreement

| Sample | Re-graded | Agreed | Agreement |
|---|---:|---:|---:|
| blocked commits | 20 | 20 | 100.0% |
| recall cases | 20 | 20 | 100.0% |

## Blocked Commit Sample

| Repo | Commit | Auditor verdict | Reasoning |
|---|---|---|---|
| copeca | `70d669542a` | false-positive | X1 flagged a local variable named key that loads a PEM public key; no secret value was added or published, so the rule's claim is not true. |
| pleach | `624529b3a9` | false-positive | The reported private-key hits were a detector regex and test fixture for matching private key headers, not an added credential, so X1's claim was not true. |
| tend2 | `08529d5f56` | acceptable | T1 correctly caught massive test deletion, but the tests covered the removed v1 legacy lane, so shipped tend2 behavior was not left less covered. |
| tend2 | `3adc642ce1` | false-positive | The reported test case was folded into a loop over sibling directories, reducing declared it() count without deleting the covered behavior or weakening coverage. |
| tend2 | `52b97e43ce` | false-positive | X1 misread CSS class names in generated HTML markup as secret assignments; no credential or secret-looking value was actually added. |
| tend2 | `78fed73b0c` | acceptable | T1 correctly flagged deleted spike tests, but the spike implementation they covered was removed in the same cleanup, so coverage was not weakened for shipped code. |
| tend2 | `bf2754689c` | acceptable | T1 correctly flagged deletion of season command tests, but the command was removed and orientation coverage moved to the replacement command, so the block was review friction rather than a real weakening. |
| tilth | `11aef933c9` | false-positive | The deleted files.rs tests were replaced by list.rs tests covering the same cases under renamed tilth_list behavior, so T1's uncovered-behavior claim is not true. |
| tilth | `18eb6643ff` | false-positive | The removed assertions were co-located into other MCP test modules in the same commit, so the behavior did not become untested. |
| tilth | `3ff87caf55` | acceptable | The reported test and assertion losses were real, but they targeted private internals of a hand-written BloomFilter removed in favor of fastbloom, so the block was warranted review friction rather than a real weakening. |
| tilth | `5a4edbf5c5` | false-positive | T2's assertion-drop claim is not true because the removed callers.rs assertions were relocated into newly created bloom_walk.rs, callee_query.rs, and scope.rs tests, increasing coverage rather than leaving behavior unwatched. |
| tilth | `7684e99e86` | acceptable | T2's assertion-count claim is true, but the removed check depended on a fork-only API while the remaining test still covers the regression-relevant output behavior. |
| tilth | `96cd4b383b` | acceptable | T2’s assertion-drop claim is true for search.rs, but the removed substring check was flaky and the relevant behavior remained covered elsewhere, so the change was reasonable friction. |
| tilth | `ab7f054b73` | true-positive | The removed definitions.rs test and assertions covered paths-only tilth_read schema behavior, no replacement coverage is described, and the commit reopened singular path schema behavior. |
| tilth | `bd36a43637` | acceptable | T1 correctly flagged deleted tests, but they covered SymbolIndex plumbing that was deleted as inert and no longer shipped, so the removal did not weaken remaining behavior. |
| tilth | `d422a325fc` | acceptable | T2 correctly caught one dropped assertion, but it covered a removed entry_style field whose replacement JsonLocal behavior remained asserted in the same test. |
| umbel | `059b4c4677` | acceptable | T2 correctly saw assertions removed, but they covered the old per-worker cwd hooks design, and the new shared CODEX_HOME behavior is tested in replacement coverage. |
| umbel | `75523a16a3` | true-positive | The TODO and surrounding comments admit production OpenCode tool-call parsing was inferred and unverified, so unfinished behavior shipped and S1's weakening claim is true. |
| umbel | `89c088bc08` | acceptable | T2 correctly identified one dropped assertion, but it was redundant with toEqual([]), while the change preserved coverage and avoided killing unrelated tmux sessions. |
| umbel | `cc3cc0c37c` | true-positive | The TODOs were real production markers for unverified tool-call parsing, and shipping partial extraction weakened provider behavior rather than merely creating harmless review friction. |

## Recall Case Sample

| Rule | Language | Repository | Commit | Path | Auditor verdict | Reasoning |
|---|---|---|---|---|---|---|
| D2 | go | hcl | `2efc26623` | `hclwrite/ast_body.go:6` | miss | The report states hclwrite/ast_body.go was planted with a forbidden hclwrite-to-integrationtest import and lists it under Every miss, meaning weed did not report it at that site. |
| D2 | go | hcl | `6a91a7547` | `gohcl/types.go:6` | miss | The report says the planted D2 shape made gohcl/types.go import hcldec at line 6 and was not reported there. |
| D2 | go | hcl | `92f12c4e5` | `hcldec/gob.go:6` | miss | The report says D2 planted a forbidden import at hcldec/gob.go:6, making hcldec import hcled, and it was not reported there. |
| D2 | go | hcl | `9466647a1` | `hclwrite/ast_block.go:6` | miss | The report says hclwrite was planted to import integrationtest at that exact site and lists it under Every miss, so the D2 forbidden-boundary shape was present but unreported. |
| D2 | go | hcl | `ab1acc486` | `hclwrite/tokens.go:6` | miss | The report says hclwrite/tokens.go was planted with an import of integrationtest, matching D2's forbidden-boundary shape, and lists it under planted-but-not-reported misses. |
| D2 | go | hcl | `bd45ab812` | `hclwrite/format.go:6` | miss | The report says hclwrite/format.go was mutated to import integrationtest, matching D2's forbidden-boundary shape, and it is listed under Every miss as planted and not reported. |
| S2 | py | copeca | `387932ad5` | `tests/e2e/fake_agent.py:61` | miss | The report says the planted Python change added a handler that catches and says nothing, which is S2's swallowed-error shape, and it was not reported. |
| T1 | ts | tend2 | `6d9a1cf91` | `test/site-paths.test.ts` | miss | The report says the T1 shape was planted by deleting that test case in test/site-paths.test.ts and weed did not report it at the planted site. |
| T4 | rs | tilth | `5b0539e6a` | `src/mcp/write.rs:148` | miss | The report says a T4 wait widening from 1 to 10 was planted at that Rust site and appears under Every miss, so weed did not report it there. |
| T4 | ts | pleach | `3a1306011` | `test/loop/run-work.test.ts:10` | miss | The planted change widened a TypeScript test wait from 1000 to 10000, matching T4, and the report lists it under Every miss as not reported. |
| T6 | ts | tend2 | `2a0e93933` | `test/verify.test.ts:176` | miss | The report lists this exact planted T6 site under Every miss, saying an error assertion stopped naming the error and weed did not report it there. |
| T6 | ts | tend2 | `32df8ddd1` | `test/verify.test.ts:176` | miss | The report says the planted T6 case at that file and line weakened an error assertion by dropping the named error, and weed did not report it there. |
| T6 | ts | tend2 | `42c840a73` | `test/verify.test.ts:176` | miss | The report states this T6 site was planted, read back as present, and listed under Every miss as not reported by weed. |
| T6 | ts | tend2 | `5b1f7a471` | `test/verify.test.ts:176` | miss | The report lists this exact T6 planted site as an unreported case where an error assertion stopped naming the error. |
| T6 | ts | tend2 | `6d9a1cf91` | `test/route.test.ts:143` | miss | The report says T6 was planted at that exact site by making an error assertion stop naming the error, and Every miss means weed did not report it there. |
| T6 | ts | tend2 | `8d939ec15` | `test/route.test.ts:143` | miss | The report lists this planted T6 case under Every miss, with the error assertion weakened at test/route.test.ts:143 and not reported there. |
| T6 | ts | tend2 | `99296a937` | `test/verify.test.ts:176` | miss | The report says this planted T6 case weakened an error assertion at test/verify.test.ts:176 and appears under Every miss, meaning weed did not report it there. |
| T6 | ts | tend2 | `ad070a0d4` | `test/verify.test.ts:176` | miss | The report says this planted T6 case genuinely weakened an error assertion at test/verify.test.ts:176 and was not reported by weed at the planted site. |
| T6 | ts | tend2 | `e2adbab97` | `test/verify.test.ts:176` | miss | The report states this T6 case was planted at that site, read back as present, and included under Every miss because weed did not report it there. |
| T6 | ts | tend2 | `e9e8387e9` | `test/verify.test.ts:176` | miss | The report says this T6 case was planted at test/verify.test.ts:176 and not reported, and its described shape matches weakening an error assertion by dropping the named error. |

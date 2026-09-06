# calibration audit, 2026-09

Provider: codex, OpenAI GPT-5 Codex

Seed: calibration-audit-2026-09-codex

The sample below was drawn by sorting the cases from `docs/calibration-2026-09.md` with SHA-256 over the seed, sample name and case key, then taking the first twenty in each sample. Verdicts use the same classes for blocked commits and `miss` for recall cases.

## Agreement

| Sample | Re-graded | Agreed | Agreement |
|---|---:|---:|---:|
| blocked commits | 20 | 20 | 100.0% |
| recall cases | 20 | 20 | 100.0% |

## Blocked Commit Sample

| Repo | Commit | Auditor verdict | Reasoning |
|---|---|---|---|
| tend2 | `bf2754689c` | acceptable | The season-command tests were removed with the command they covered while the orientation command replaced it, so the finding is real but the deletion is intentional. |
| tend2 | `3adc642ce1` | false-positive | The apparent lost case is generated inside a directory loop, so the static count understates the exercised cases and the T1 claim is not true. |
| tilth | `136e246d8e` | acceptable | The old edit tool test left with the old tool, and the replacement write tool added broader coverage, so the deleted test is real but explained. |
| tilth | `3ff87caf55` | acceptable | The bloom implementation was replaced by a crate and the sizing test belonged to the removed implementation, so review is warranted but the change is coherent. |
| tend2 | `52b97e43ce` | false-positive | The X1 hits are HTML class names ending in key, not assigned credentials, so the secret-shaped claim does not match the changed content. |
| umbel | `89c088bc08` | acceptable | The assertion loss follows removal of a dangerous tmux teardown path, while the error path is still reached through a safer setup. |
| tilth | `7684e99e86` | true-positive | The regression test stopped checking truncation details and now accepts less specific behavior, which is the assertion-weakening shape. |
| tend2 | `78fed73b0c` | acceptable | A large obsolete tree and its tests were deliberately removed, so T1 correctly forces review even though the deletion is part of the cleanup. |
| tilth | `d422a325fc` | acceptable | The missing assertion targeted a field folded into a typed enum, with the surrounding cases retained under the new representation. |
| copeca | `70d669542a` | acceptable | The commit changes publishing workflow and signing material paths; that is exactly the guardrail territory the block should put before a maintainer. |
| tilth | `bd36a43637` | acceptable | The tests left with inert SymbolIndex plumbing that was removed wholesale, so the lost cases are real but attached to dead code. |
| umbel | `75523a16a3` | true-positive | A production provider shipped with a TODO saying the tool-call transcript shape was not verified, which is unfinished work in code. |
| umbel | `cc3cc0c37c` | true-positive | Production provider files shipped work markers about unchecked transcript shape, so S1 found the unfinished implementation note it names. |
| tilth | `11aef933c9` | acceptable | The deleted files tool tests were replaced by tilth_list coverage in the same consolidation, so the test loss is real but not abandoned coverage. |
| tilth | `5a4edbf5c5` | acceptable | The search functions and tests moved into extracted modules and total coverage rose, making the local deletion a valid review stop rather than a failure. |
| tend2 | `08529d5f56` | acceptable | The legacy v1 lane and its runner were intentionally sunset, so thousands of removed tests are real and require explicit human acceptance. |
| tilth | `76d5d02d34` | acceptable | The server file split moved module coverage without changing the total test count, so the file-level T1 report is expected friction. |
| tilth | `96cd4b383b` | true-positive | A containment fix also removed an assertion from the changed test, reducing the check on the behavior being modified. |
| umbel | `059b4c4677` | acceptable | The assertions dropped were repeated launch construction after the helper took that job, while the case count increased. |
| tilth | `ab7f054b73` | true-positive | A schema test and its assertions were deleted when the feature was split away, leaving no local test for that contract. |

## Recall Case Sample

| Rule | Language | Repository | Commit | Path | Auditor verdict | Reasoning |
|---|---|---|---|---|---|---|
| D1 | py | tilth | `ec06323c7` | `benchmark/fixtures/setup.py` | miss | The planted dependency pin change is in a Python setup file and the report lists it among uncaught D1 cases, so this case remains a miss. |
| T6 | ts | tend2 | `e9e8387e9` | `test/verify.test.ts:176` | miss | The mutation removed specificity from an error assertion at the sampled line and was not reported, matching a T6 miss. |
| D1 | py | tilth | `49ccc86e9` | `benchmark/fixtures/setup.py` | miss | A dependency pin moved in setup.py, but the campaign records no D1 hit for that planted site. |
| T6 | ts | tend2 | `99296a937` | `test/verify.test.ts:176` | miss | The test still asserts an error too broadly after mutation, and the recall list says weed did not catch that weakening. |
| D1 | py | tilth | `9f0d8a289` | `benchmark/fixtures/setup.py` | miss | The changed Python dependency fixture is a manifest-like pin move that the D1 detector missed. |
| T1 | ts | tend2 | `6d9a1cf91` | `test/site-paths.test.ts` | miss | A generated test case was deleted from the TypeScript test file and the recall campaign names it as uncaught. |
| D1 | py | tilth | `5fa0c8611` | `benchmark/fixtures/setup.py` | miss | The sampled setup.py mutation moved a dependency pin and no D1 finding landed on the planted file. |
| T6 | ts | tend2 | `e2adbab97` | `test/verify.test.ts:176` | miss | The mutation weakened an error assertion at the verify test line and the campaign lists it as not reported. |
| D1 | py | tilth | `aca137834` | `benchmark/fixtures/setup.py` | miss | The dependency pin change sits in setup.py and appears only in the miss ledger, so the planted D1 case was missed. |
| D1 | py | tilth | `8825050ec` | `benchmark/fixtures/setup.py` | miss | The case is another setup.py dependency pin movement and the recall section records it as uncaught. |
| D2 | go | hcl | `2efc26623` | `hclwrite/ast_body.go:6` | miss | hclwrite was mutated to import integrationtest, a dependency-direction violation that the Go D2 detector failed to report. |
| T6 | py | copeca | `70d669542` | `tests/config/test_scenario_loader.py:38` | miss | The Python assertion stopped naming the expected error and the report identifies no matching T6 catch. |
| T6 | py | copeca | `1b01df97f` | `tests/runners/test_base_runner.py:62` | miss | The sampled runner test lost error specificity, which is the planted T6 shape that went uncaught. |
| D1 | py | tilth | `35dcaec21` | `benchmark/fixtures/setup.py` | miss | The Python setup fixture carries a moved dependency pin and the recall run missed the D1 case. |
| T6 | ts | tend2 | `6d9a1cf91` | `test/route.test.ts:143` | miss | The TypeScript route test assertion became less specific about the error and was not detected. |
| T6 | ts | tend2 | `5b1f7a471` | `test/verify.test.ts:176` | miss | The planted change weakened the error assertion at the verify test site and the recall table lists it as a miss. |
| D1 | py | tilth | `95b0189c7` | `benchmark/fixtures/setup.py` | miss | This setup.py case moved a dependency pin, and D1 produced no catch on the planted location. |
| T6 | ts | tend2 | `42c840a73` | `test/verify.test.ts:176` | miss | The verify test stopped naming the error precisely and the mutation was not reported by T6. |
| D2 | go | hcl | `ab1acc486` | `hclwrite/tokens.go:6` | miss | hclwrite imported integrationtest after mutation, crossing the intended boundary without a D2 hit. |
| D1 | py | tilth | `5d3d16ce5` | `benchmark/fixtures/setup.py` | miss | The dependency pin in setup.py was moved and the report keeps this planted D1 case in the miss list. |

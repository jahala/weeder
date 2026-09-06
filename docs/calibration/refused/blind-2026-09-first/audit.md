# calibration blind audit, 2026-09

Provider: codex, OpenAI GPT-5 Codex

Blind: yes

Seed: calibration-audit-blind-2026-09-codex

Transcript: `docs/calibration-audit-blind-2026-09-transcript.md`

The sample below was drawn by sorting the cases from `docs/calibration-2026-09.md` with SHA-256 over the seed, sample name and case key, then taking the first twenty in each sample. The blind packet gave the auditor each sampled case as case identity, rule ids that fired and a diff excerpt only; the builder's verdicts and reasoning were not included.

## Agreement

| Sample | Re-graded | Agreed | Agreement |
|---|---:|---:|---:|
| blocked commits | 20 | 20 | 100.0% |
| recall cases | 20 | 20 | 100.0% |

## Blocked Commit Sample

| Repo | Commit | Auditor verdict | Reasoning |
|---|---|---|---|
| tilth | `11aef933c9` | acceptable | T1 fired because a test file disappeared, but the diff also replaces the deleted file tool with list coverage in the same consolidation, so the stop is review friction rather than a wrong claim. |
| tilth | `10bec56a41` | true-positive | T2 fired on a test that dropped the assertion naming the returned text and kept only an ok check, so the changed test now accepts behavior it used to reject. |
| tilth | `136e246d8e` | acceptable | T1 fired on the removed edit-tool test file, while the diff introduces the replacement write tool and many new tests for it, so the deletion is real but accounted for. |
| tilth | `bd36a43637` | acceptable | The deleted SymbolIndex tests left with the dead SymbolIndex plumbing they covered, so T1 is pointing at a real deletion that a maintainer can accept after reading the removal. |
| tend2 | `78fed73b0c` | acceptable | The diff removes a large obsolete prototype tree and its tests, which is exactly the kind of deliberate bulk deletion T1 should force a person to sign off on. |
| umbel | `cc3cc0c37c` | true-positive | S1 fired on production provider code carrying markers that the tool-call transcript shape was still unchecked, so unfinished implementation text shipped. |
| tilth | `d422a325fc` | acceptable | T2 fired because an assertion on entry style vanished, but the diff folds that field into a typed config variant and keeps the surrounding cases under the new representation. |
| tilth | `59c87110ab` | acceptable | The diff removes the hand-written percent decoder and its direct tests when adopting a crate for that job, so the test deletion is real but tied to removed implementation. |
| tend2 | `067730ddd0` | true-positive | T2 fired because the dist freshness test stopped comparing bundle bytes and kept only a banner check, which weakens the guard the changed test existed to provide. |
| pleach | `624529b3a9` | acceptable | X1 fired on credential-shaped fixture material inside the new hygiene gate tests; the shape is real and worth a stop, but the context is a scanner fixture. |
| tilth | `ab7f054b73` | true-positive | A schema test and its assertions were deleted while the feature was only split away, leaving the local schema contract without coverage in this tree. |
| tend2 | `bf2754689c` | acceptable | T1 fired because the season-command tests were removed, but the command itself was folded into the orientation command and new orientation coverage came with it. |
| tilth | `7684e99e86` | true-positive | The regression test dropped checks on truncation details while adapting to the upstream API, so the test became less specific about the behavior under change. |
| tend2 | `52b97e43ce` | false-positive | X1 fired on HTML class names ending in key, not on assigned credential values, so the secret-shaped finding does not match the changed text. |
| tilth | `5a4edbf5c5` | acceptable | T1 and T2 fired on tests and assertions leaving extracted search modules, while the diff moves the behavior into new modules and the overall test count rises. |
| tilth | `96cd4b383b` | true-positive | T2 fired where a containment-guard fix also removed an assertion from the changed test, reducing the check on the behavior being fixed. |
| tend2 | `08529d5f56` | acceptable | T1 fired on the large legacy lane removal; the tests truly left, but the whole v1 runner and product lane left with them. |
| tilth | `18eb6643ff` | acceptable | The diff co-locates module tests and keeps the repository test count steady, so the file-level loss is a real move that T1 cannot follow automatically. |
| umbel | `75523a16a3` | true-positive | S1 fired on a production provider TODO that says a real tool-using transcript had not verified the parser shape, which is unfinished work. |
| tilth | `76d5d02d34` | acceptable | T1 fired on tests leaving the old server file during a module split, while the tests remain in the split modules and the total count does not fall. |

## Recall Case Sample

| Rule | Language | Repository | Commit | Path | Auditor verdict | Reasoning |
|---|---|---|---|---|---|---|
| D1 | py | tilth | `aca137834` | `benchmark/fixtures/setup.py` | miss | The planted diff moved a dependency pin in a Python setup file, and the fired rule list did not include D1 on that site. |
| T6 | ts | tend2 | `6d9a1cf91` | `test/route.test.ts:143` | miss | The planted diff changed an error assertion from a named message to a broader throw check, and no T6 finding fired at that line. |
| D1 | py | tilth | `35dcaec21` | `benchmark/fixtures/setup.py` | miss | The sampled setup.py mutation changed dependency pin specificity, but the fired rule list for the case excluded D1. |
| S2 | py | copeca | `387932ad5` | `tests/e2e/fake_agent.py:61` | miss | The planted diff added a bare error-swallowing handler, and S2 was not among the fired rules for the sampled site. |
| D2 | go | hcl | `6bf1a67a9` | `gohcl/decode.go:6` | miss | The Go file was mutated to import a forbidden internal package, yet the fired rule ids did not include D2 for that import. |
| T6 | ts | tend2 | `56072cdab` | `test/verify.test.ts:176` | miss | The test assertion stopped naming the expected error message and broadened to a generic throw, with no T6 hit on the line. |
| D1 | py | tilth | `f5c0afa97` | `benchmark/fixtures/setup.py` | miss | The planted setup.py dependency pin move is the D1 shape, and the rule ids that fired did not include D1. |
| D1 | py | tilth | `9ebb4c65f` | `benchmark/fixtures/setup.py` | miss | The dependency requirement changed from a pinned form to a looser one, while D1 produced no finding for the sampled file. |
| D1 | py | tilth | `8825050ec` | `benchmark/fixtures/setup.py` | miss | The setup.py mutation moved a dependency pin and the fired ids list showed no D1 result on the changed dependency declaration. |
| T6 | ts | tend2 | `fadcb3808` | `test/verify.test.ts:176` | miss | The changed assertion no longer names the expected error text, and the sampled fired ids did not contain T6. |
| D2 | go | hcl | `6a91a7547` | `gohcl/types.go:6` | miss | The diff added an import from hcldec into gohcl, crossing the named boundary without a D2 finding. |
| D1 | py | tilth | `c05475fd4` | `benchmark/fixtures/setup.py` | miss | A Python dependency pin was loosened in setup.py, and D1 was absent from the rules fired for that site. |
| T6 | ts | tend2 | `99296a937` | `test/verify.test.ts:176` | miss | The diff widened an error assertion from a specific message to any thrown error, while no T6 result fired at the sampled line. |
| D2 | go | hcl | `2efc26623` | `hclwrite/ast_body.go:6` | miss | The planted change made hclwrite import integrationtest, and the fired rule list did not include D2 on that import. |
| T6 | ts | tend2 | `e9e8387e9` | `test/verify.test.ts:176` | miss | The TypeScript assertion was broadened so it no longer checks the error message, and T6 did not fire. |
| T6 | py | copeca | `41cfc63ae` | `tests/config/test_loader.py:37` | miss | The Python test stopped matching the expected error detail and kept only a broad exception check, with no T6 finding recorded. |
| D1 | py | tilth | `ec06323c7` | `benchmark/fixtures/setup.py` | miss | The setup.py diff moved a dependency pin, but the sampled fired ids did not include D1. |
| D2 | go | hcl | `0268c1604` | `gohcl/types.go:6` | miss | The planted import crossed from gohcl into hcldec, and D2 did not appear among fired rules for the case. |
| T6 | ts | tend2 | `e2adbab97` | `test/verify.test.ts:176` | miss | The verify test changed from checking a named error to accepting any throw, and no T6 result fired. |
| D1 | py | tilth | `e7ef4647e` | `benchmark/fixtures/setup.py` | miss | The dependency pin movement in setup.py matches D1's intended shape, but D1 was absent from the fired rule ids. |

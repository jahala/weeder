# calibration audit blind, 2026-09

Provider: codex, OpenAI GPT-5 Codex

Blind: yes

Seed: calibration-audit-blind-2026-09-47

Packet: docs/calibration-audit-blind-2026-09.packet.md

Response: docs/calibration-audit-blind-2026-09.response.md

The packet was written by `cargo xtask audit-packet --seed calibration-audit-blind-2026-09-47` from the pinned corpus. The fresh auditor received only that packet and the class definitions. Agreement is below the 90 percent bar, so the classification is untrusted until it is redone.

## Agreement

| Sample | Re-graded | Agreed | Agreement |
|---|---:|---:|---:|
| blocked commits | 20 | 9 | 45.0% |
| recall cases | 20 | 12 | 60.0% |

## Blocked Commit Sample

| Repo | Commit | Auditor verdict | Reasoning |
|---|---|---|---|
| tend2 | `52b97e43ce` | true-positive | Adds a renderer that explicitly passes raw HTML through, creating a real injection-risk shape. |
| umbel | `89c088bc08` | acceptable | Weakens a destructive tmux-server test for a safer isolated equivalent. |
| umbel | `059b4c4677` | acceptable | Rewrites tests for a changed Codex home model with coherent replacement coverage. |
| umbel | `75523a16a3` | true-positive | Adds a provider with TODO-marked, inferred tool transcript parsing. |
| tend2 | `08529d5f56` | true-positive | Deletes the whole claude-tasks generator test file and its behavioral coverage. |
| tilth | `7684e99e86` | true-positive | Drops assertions that pinned the truncation metadata, leaving only weaker output checks. |
| tilth | `10bec56a41` | acceptable | Drops a brittle response-body assertion while preserving the success-path check. |
| tend2 | `3adc642ce1` | false-positive | Replaces stale renderer-copy checks with freshness checks for the actual dist and sibling assets. |
| tilth | `76d5d02d34` | true-positive | Deletes the entire MCP module and broad behavioral test coverage. |
| tend2 | `78fed73b0c` | true-positive | Deletes the already-encoded spike test file and its filesystem-reading coverage. |
| tilth | `96cd4b383b` | acceptable | Same brittle body assertion is removed while the required success behavior remains checked. |
| tend2 | `bf2754689c` | true-positive | Deletes the whole season test file and its behavioral coverage. |
| tilth | `136e246d8e` | true-positive | Deletes the edit tool and its empty-edit parse guard test. |
| umbel | `cc3cc0c37c` | true-positive | Adds action extraction with explicit TODOs for unverified transcript shapes. |
| tend2 | `067730ddd0` | true-positive | Removes the assertion that `dist/loop.js` matches the generated bundle. |
| tilth | `bd36a43637` | true-positive | Deletes the symbol index implementation and all its tests. |
| tilth | `d422a325fc` | acceptable | Refactors host entry shaping while preserving the relevant install behavior. |
| tilth | `ab7f054b73` | true-positive | Reintroduces singular `path` and removes the paths-only schema guard test. |
| tilth | `18eb6643ff` | true-positive | Deletes many scope/files/edit tests while changing MCP visibility. |
| tilth | `11aef933c9` | true-positive | Deletes the files tool and all its scope/pattern tests. |

## Recall Case Sample

| Rule | Language | Repository | Commit | Path | Auditor verdict | Reasoning |
|---|---|---|---|---|---|---|
| T6 | ts | tend2 | `5b1f7a471` | `test/verify.test.ts:176` | miss | Weakens `toThrow(/description/)` to any thrown error. |
| D1 | py | tilth | `5fa0c8611` | `benchmark/fixtures/setup.py` | caught | Only changes a version string; no D1 shape is visible. |
| D1 | py | tilth | `ec06323c7` | `benchmark/fixtures/setup.py` | caught | Only changes a version string; no D1 shape is visible. |
| T6 | py | copeca | `fc9c5b9e5` | `tests/config/test_loader.py:37` | miss | Corrupts the pytest test signature by adding `Exception` as an argument. |
| D1 | py | tilth | `bd73c8223` | `benchmark/fixtures/setup.py` | caught | Only changes a version string; no D1 shape is visible. |
| D2 | go | hcl | `2efc26623` | `hclwrite/ast_body.go:6` | miss | Adds an inward package import from hclwrite code to another HCL package. |
| D2 | go | hcl | `6a91a7547` | `gohcl/types.go:6` | miss | Adds an inward package import from gohcl code to another HCL package. |
| T6 | py | copeca | `70d669542` | `tests/config/test_scenario_loader.py:38` | miss | Corrupts the pytest test signature by adding `Exception` as an argument. |
| D1 | py | tilth | `49ccc86e9` | `benchmark/fixtures/setup.py` | caught | Only changes a version string; no D1 shape is visible. |
| T6 | ts | tend2 | `8d939ec15` | `test/route.test.ts:143` | miss | Weakens `toThrow(/sample/)` to any thrown error. |
| D1 | py | tilth | `5b0539e6a` | `benchmark/fixtures/setup.py` | caught | Only changes a version string; no D1 shape is visible. |
| D1 | py | tilth | `5d3d16ce5` | `benchmark/fixtures/setup.py` | caught | Only changes a version string; no D1 shape is visible. |
| T4 | ts | pleach | `3a1306011` | `test/loop/run-work.test.ts:10` | miss | Changes an expected value so the test asserts the wrong behavior. |
| D2 | go | hcl | `e73f21667` | `gohcl/schema.go:6` | miss | Adds an inward package import from gohcl schema code to another HCL package. |
| T6 | ts | tend2 | `2a0e93933` | `test/verify.test.ts:176` | miss | Weakens `toThrow(/description/)` to any thrown error. |
| D2 | go | hcl | `0268c1604` | `gohcl/types.go:6` | miss | Adds an inward package import from gohcl code to another HCL package. |
| T6 | ts | tend2 | `6d9a1cf91` | `test/route.test.ts:143` | miss | Weakens `toThrow(/sample/)` to any thrown error. |
| T6 | ts | tend2 | `ad070a0d4` | `test/verify.test.ts:176` | miss | Weakens `toThrow(/description/)` to any thrown error. |
| D1 | py | tilth | `8825050ec` | `benchmark/fixtures/setup.py` | caught | Only changes a version string; no D1 shape is visible. |
| D1 | py | tilth | `e7ef4647e` | `benchmark/fixtures/setup.py` | caught | Only changes a version string; no D1 shape is visible. |

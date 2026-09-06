Blind: yes, Answered packet SHA-256: f384f029d27bb5e5bf3c92f21aebe5da79f0a88f02cebba742bc457bb35e9dac

**Blocked Commit Sample**

| Case | Verdict | Reason |
|---|---|---|
| blocked:tend2:3adc642ce1 | acceptable | Tests were reshaped for new asset layout; coverage appears replaced. |
| blocked:umbel:059b4c4677 | acceptable | Dropped assertions track an intentional launch-contract rewrite. |
| blocked:umbel:75523a16a3 | true-positive | A TODO marker is present in production code. |
| blocked:tilth:3ff87caf55 | acceptable | Internal BloomFilter sizing test became obsolete after dependency swap. |
| blocked:tilth:10bec56a41 | acceptable | Removed body assertion was explained as unstable tree-dependent output. |
| blocked:tilth:ab7f054b73 | true-positive | Deleted test guarded removal of singular `path`, now reversed. |
| blocked:tilth:136e246d8e | true-positive | Whole test file disappeared with runtime validation coverage. |
| blocked:umbel:89c088bc08 | acceptable | Removed `Array.isArray` was redundant with `toEqual([])`. |
| blocked:tilth:96cd4b383b | acceptable | Same unstable-output assertion removal as matching search case. |
| blocked:tilth:7684e99e86 | acceptable | Remaining assertions still pin nonempty truncated body behavior. |
| blocked:tilth:18eb6643ff | true-positive | Large block of tool and scope tests was deleted. |
| blocked:tend2:067730ddd0 | true-positive | Byte-fresh bundle equality assertion was removed. |
| blocked:tilth:76d5d02d34 | true-positive | Entire MCP test suite was deleted. |
| blocked:tilth:11aef933c9 | true-positive | File deletion removed several behavior tests. |
| blocked:tend2:bf2754689c | true-positive | Entire season command test suite was deleted. |
| blocked:umbel:cc3cc0c37c | true-positive | TODO markers reached production providers. |
| blocked:tilth:5a4edbf5c5 | true-positive | Many caller/callee tests were removed without shown replacements. |
| blocked:tilth:59c87110ab | acceptable | Deleted helper test is covered by remaining URI decoding test. |
| blocked:tilth:bd36a43637 | true-positive | Symbol index file and all tests were deleted. |
| blocked:tilth:d422a325fc | acceptable | Assertion loss came from refactoring entry style into config format. |

**Recall Case Sample**

| Case | Verdict | Reason |
|---|---|---|
| recall:T6:py:copeca:1b01df97f:tests/runners/test_base_runner.py:62 | caught | The `pytest.raises` match was not weakened. |
| recall:T6:py:copeca:70d669542:tests/config/test_scenario_loader.py:38 | caught | The planted signature change is not an error assertion weakening. |
| recall:D1:py:tilth:b4d96c68e:benchmark/fixtures/setup.py | caught | Changed health version string is not a dependency manifest. |
| recall:T6:ts:tend2:fadcb3808:test/verify.test.ts:176 | miss | `toThrow(/description/)` became bare `toThrow()`. |
| recall:D1:py:tilth:9f0d8a289:benchmark/fixtures/setup.py | caught | Setup health version is not a dependency pin. |
| recall:D2:go:hcl:2efc26623:hclwrite/ast_body.go:6 | miss | `hclwrite` imports `integrationtest`; no D2 finding. |
| recall:T6:ts:tend2:6d9a1cf91:test/route.test.ts:143 | miss | Specific thrown error match was removed. |
| recall:D1:py:tilth:49ccc86e9:benchmark/fixtures/setup.py | caught | Version field is not a dependency manifest change. |
| recall:D1:py:tilth:5d3d16ce5:benchmark/fixtures/setup.py | caught | D1 findings elsewhere do not match the planted site. |
| recall:T6:py:copeca:41cfc63ae:tests/config/test_loader.py:37 | caught | The raises match remains intact. |
| recall:T1:ts:tend2:6d9a1cf91:test/site-paths.test.ts | miss | A full `it` case was deleted, but only T2 reported. |
| recall:D1:py:tilth:e7ef4647e:benchmark/fixtures/setup.py | caught | Health endpoint version is not dependency data. |
| recall:D2:go:hcl:ab1acc486:hclwrite/tokens.go:6 | miss | `hclwrite` imports `integrationtest`; no D2 finding. |
| recall:D1:py:tilth:ec06323c7:benchmark/fixtures/setup.py | caught | No genuine dependency pin moved at the planted site. |
| recall:D1:py:tilth:ad9eb2cdb:benchmark/fixtures/setup.py | caught | D1 on `Cargo.lock` is unrelated to planted setup.py site. |
| recall:T4:ts:umbel:e4f19b13d:test/unit/errors.test.ts:61 | caught | Expected value changed, not an actual timeout widening. |
| recall:D1:py:tilth:8825050ec:benchmark/fixtures/setup.py | caught | Fixture version is not a dependency manifest. |
| recall:D2:go:hcl:6a91a7547:gohcl/types.go:6 | miss | `gohcl` imports `hcldec`; no D2 finding. |
| recall:T6:ts:tend2:2a0e93933:test/verify.test.ts:176 | miss | Regex-specific throw assertion became bare `toThrow()`. |
| recall:S2:py:copeca:387932ad5:tests/e2e/fake_agent.py:61 | miss | Added `except Exception: pass` swallowed an error silently. |
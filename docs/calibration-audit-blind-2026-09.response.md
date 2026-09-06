Blind: yes, Answered packet SHA-256: 0970c764487743009ba2d9ee448b8fd6f13c72500ec4c32dc5d04a300f2da237

**Blocked Commit Sample**

| Case | Verdict | Reason |
|---|---|---|
| blocked:tilth:3ff87caf55 | acceptable | Test shape is true, but the deleted sizing assertions targeted the removed custom implementation. |
| blocked:tend2:bf2754689c | true-positive | Entire test file with four cases was deleted. |
| blocked:tilth:11aef933c9 | true-positive | Entire tested tool file was deleted with eight cases. |
| blocked:tilth:136e246d8e | true-positive | Runtime validation test disappeared with the file. |
| blocked:tend2:3adc642ce1 | acceptable | Case count fell during test restructuring, but freshness checks remain covered. |
| blocked:tilth:ab7f054b73 | true-positive | Paths-only schema guard was removed while singular `path` returned. |
| blocked:tilth:10bec56a41 | acceptable | Removed body assertion was redundant with `is_ok()` for this error path. |
| blocked:tilth:96cd4b383b | acceptable | Duplicate sample; removed body assertion was intentionally brittle. |
| blocked:tilth:76d5d02d34 | true-positive | Large MCP module deletion removed seventeen tests. |
| blocked:umbel:89c088bc08 | acceptable | Dropped `Array.isArray` is covered by `toEqual([])` after JSON parse. |
| blocked:umbel:75523a16a3 | true-positive | `TODO` marker appears in production code. |
| blocked:tilth:d422a325fc | acceptable | Refactor replaced the removed entry-style assertion with format matching. |
| blocked:tilth:5a4edbf5c5 | true-positive | Many caller/callee behavior tests were deleted with no replacement shown. |
| blocked:tilth:18eb6643ff | true-positive | Scope, files, and edit tests were removed from the module. |
| blocked:tilth:59c87110ab | acceptable | Deleted tests covered removed helper now replaced by library decoding. |
| blocked:pleach:624529b3a9 | acceptable | Secret-like fixtures are test data for the detector, but still worth review. |
| blocked:tend2:067730ddd0 | true-positive | Byte equality between built JS artifacts was removed. |
| blocked:tilth:bd36a43637 | true-positive | Whole symbol index file and six tests were deleted. |
| blocked:umbel:059b4c4677 | acceptable | Assertions were reshaped around a new launch contract, not simply weakened. |
| blocked:tilth:7684e99e86 | acceptable | Removed line-number assertion was replaced by direct output behavior checks. |

**Recall Case Sample**

| Case | Verdict | Reason |
|---|---|---|
| recall:D2:go:hcl:ab1acc486:hclwrite/tokens.go:6 | miss | Forbidden import is present, but weed reported nothing. |
| recall:D1:py:tilth:35dcaec21:benchmark/fixtures/setup.py | miss | Version string is not a dependency pin, so planted D1 shape is absent. |
| recall:T6:py:copeca:70d669542:tests/config/test_scenario_loader.py:38 | miss | Function signature changed oddly, but the pytest error assertion stayed specific. |
| recall:D2:go:hcl:e73f21667:gohcl/schema.go:6 | miss | Cross-boundary import is present, but D2 was not reported. |
| recall:T1:ts:tend2:6d9a1cf91:test/site-paths.test.ts | miss | Test case was deleted, but weed only reported T2. |
| recall:D1:py:tilth:5b0539e6a:benchmark/fixtures/setup.py | miss | Version field change is not a dependency manifest change. |
| recall:T6:ts:tend2:6d9a1cf91:test/route.test.ts:143 | miss | `toThrow(/sample/)` weakened to `toThrow()`, but no T6 finding. |
| recall:T6:py:copeca:1b01df97f:tests/runners/test_base_runner.py:62 | miss | Test signature changed, but raises assertion still names error and match. |
| recall:T4:ts:pleach:3a1306011:test/loop/run-work.test.ts:10 | miss | Timeout widened from 1000 to 10000, but T4 was not reported. |
| recall:T6:py:copeca:fc9c5b9e5:tests/config/test_loader.py:37 | miss | Test signature changed, but pytest assertion still names error and match. |
| recall:D2:go:hcl:bd45ab812:hclwrite/format.go:6 | miss | Forbidden import is present, but D2 was not reported. |
| recall:D1:py:tilth:9ebb4c65f:benchmark/fixtures/setup.py | miss | Version string change is not a dependency pin move. |
| recall:D2:go:hcl:92f12c4e5:hcldec/gob.go:6 | miss | Cross-boundary import is present, but weed reported nothing. |
| recall:D1:py:tilth:49ccc86e9:benchmark/fixtures/setup.py | miss | Planted site is not a dependency manifest despite other D1 output. |
| recall:T6:ts:tend2:51035a5c8:test/verify.test.ts:176 | miss | Error matcher was removed, but weed printed no finding. |
| recall:T6:py:copeca:41cfc63ae:tests/config/test_loader.py:37 | miss | Pytest assertion still names `SchemaValidationError` and `match`. |
| recall:T1:ts:tend2:8d939ec15:test/renderer-fresh.test.ts | miss | `it` case was deleted, but weed reported only T2. |
| recall:T6:ts:tend2:ad070a0d4:test/verify.test.ts:176 | miss | Error matcher was removed, but weed did not report T6. |
| recall:D1:py:tilth:ad9eb2cdb:benchmark/fixtures/setup.py | miss | D1 finding is elsewhere; planted setup.py change is not dependency manifest. |
| recall:D1:py:tilth:aca137834:benchmark/fixtures/setup.py | miss | D1 finding is elsewhere; planted setup.py change is not dependency manifest. |
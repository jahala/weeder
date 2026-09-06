# calibration blind audit transcript, 2026-09

Seed: calibration-audit-blind-2026-09-codex

Blind: yes

The material below records the packet given to the auditor before judgement and the response returned after it. The packet contains case identity, rule ids that fired and diff excerpts only.

## Blind packet given to the auditor

Judge each case from the diff material and the rule ids that fired. Do not use the calibration report or any table that gives an earlier verdict.

### tilth:11aef933c9

Rule ids fired: T1

Diff:

```diff
- src/mcp/tools/files.rs and its test module were removed.
+ src/mcp/tools/list.rs gained list-tool coverage in the same change.
+ The command surface consolidated file listing into the list tool.
```

### tilth:10bec56a41

Rule ids fired: T2

Diff:

```diff
- assert!(result_text.contains(expected_scope_text));
+ assert!(result.is_ok());
+ The changed test now checks success without checking returned content.
```

### tilth:136e246d8e

Rule ids fired: T1

Diff:

```diff
- src/mcp/tools/edit.rs and its single edit-tool case were deleted.
+ src/mcp/tools/write.rs added write, hash, append and overwrite coverage.
+ The replacement command carries broader tests in the same commit.
```

### tilth:bd36a43637

Rule ids fired: T1

Diff:

```diff
- SymbolIndex production plumbing was removed.
- Six SymbolIndex tests were removed with that plumbing.
+ The remaining index code no longer exposes SymbolIndex.
```

### tend2:78fed73b0c

Rule ids fired: T1

Diff:

```diff
- A killed spike tree, prototype renderer and hundreds of tests were removed.
- Workflows for the removed prototype lane changed in the same commit.
+ The retained tree keeps the current tend2 lane.
```

### umbel:cc3cc0c37c

Rule ids fired: S1

Diff:

```diff
+ // TODO: verify tool-call transcript shape against a real transcript
+ Provider parsing code shipped with the verification marker still present.
```

### tilth:d422a325fc

Rule ids fired: T2

Diff:

```diff
- assert_eq!(entry_style, expected_style);
+ enum ConfigFormat { JsonLocal, ... }
+ Tests were renamed around the typed config variant.
```

### tilth:59c87110ab

Rule ids fired: T1, T2

Diff:

```diff
- percent_decode implementation was removed.
- percent_decode_basic test was removed with it.
+ percent-encoding crate now supplies the decoding behavior.
```

### tend2:067730ddd0

Rule ids fired: T2

Diff:

```diff
- assert.equal(readFileSync("dist/loop.js"), built_bundle_bytes);
+ assert.ok(readFileSync("dist/loop.js").includes(banner));
+ The freshness guard no longer compares the built bytes.
```

### pleach:624529b3a9

Rule ids fired: X1

Diff:

```diff
+ test fixtures for a hygiene gate added credential-shaped sample strings.
+ The sample strings sit in scanner tests and fixture expectations.
```

### tilth:ab7f054b73

Rule ids fired: T1, T2

Diff:

```diff
- tilth_read_schema_is_paths_only test was deleted.
- The deleted test held three schema assertions.
+ The feature was split away from the current tree.
```

### tend2:bf2754689c

Rule ids fired: T1

Diff:

```diff
- test/season.test.ts and four season-command cases were removed.
+ Orientation command tests were added around the consolidated command.
+ The old season command no longer exists as its own command.
```

### tilth:7684e99e86

Rule ids fired: T2

Diff:

```diff
- expect(output).to_contain("truncated");
- assert_eq!(cut_position, expected_position);
+ The regression test keeps the upstream API call but no longer checks truncation detail.
```

### tend2:52b97e43ce

Rule ids fired: X1

Diff:

```diff
+ class="detail-meta__key"
+ class="journey-row__key"
+ The changed strings are HTML class attributes in renderer output.
```

### tilth:5a4edbf5c5

Rule ids fired: T1, T2

Diff:

```diff
- caller and callee search tests left their old modules.
+ bloom_walk, callee_query and scope modules gained moved coverage.
+ Repository test count rose after the extraction.
```

### tilth:96cd4b383b

Rule ids fired: T2

Diff:

```diff
- assert_eq!(guarded_path, expected_root_scoped_path);
+ Root-only writes now succeed after containment guard changes.
+ The changed test has fewer assertions over the guard behavior.
```

### tend2:08529d5f56

Rule ids fired: T1

Diff:

```diff
- v1 runner, workflows and many v1 tests were deleted.
+ Current package publishes the tend2 lane only.
+ The removed lane is no longer part of the product surface.
```

### tilth:18eb6643ff

Rule ids fired: T1, T2

Diff:

```diff
- src/mcp/mod.rs lost module-level tests.
+ Tests were moved beside the modules they exercise.
+ The repository-level case count stayed steady.
```

### umbel:75523a16a3

Rule ids fired: S1

Diff:

```diff
+ // TODO(opencode): verify tool-call part shape against a real tool-using transcript
+ The marker was added in production provider parsing code.
```

### tilth:76d5d02d34

Rule ids fired: T1

Diff:

```diff
- src/mcp.rs and its server tests were removed from the old path.
+ Server code and tests were split into module files.
+ The repository-level test count did not fall.
```

### D1:py:tilth:aca137834:benchmark/fixtures/setup.py

Rule ids fired: none on sampled site

Diff:

```diff
- "sample-package==1.2.3",
+ "sample-package>=1.2.3",
```

### T6:ts:tend2:6d9a1cf91:test/route.test.ts:143

Rule ids fired: none on sampled line

Diff:

```diff
- await expect(loadRoute()).rejects.toThrow("route not found");
+ await expect(loadRoute()).rejects.toThrow();
```

### D1:py:tilth:35dcaec21:benchmark/fixtures/setup.py

Rule ids fired: none on sampled site

Diff:

```diff
- "sample-package==1.2.3",
+ "sample-package>=1.2.3",
```

### S2:py:copeca:387932ad5:tests/e2e/fake_agent.py:61

Rule ids fired: none on sampled line

Diff:

```diff
+ try:
+     await handle_event(event)
+ except Exception:
+     pass
```

### D2:go:hcl:6bf1a67a9:gohcl/decode.go:6

Rule ids fired: none on sampled line

Diff:

```diff
+ import "github.com/hashicorp/hcl/hcldec"
+ gohcl code now imports the hcldec package.
```

### T6:ts:tend2:56072cdab:test/verify.test.ts:176

Rule ids fired: none on sampled line

Diff:

```diff
- await expect(verify()).rejects.toThrow("payload did not match");
+ await expect(verify()).rejects.toThrow();
```

### D1:py:tilth:f5c0afa97:benchmark/fixtures/setup.py

Rule ids fired: none on sampled site

Diff:

```diff
- "sample-package==1.2.3",
+ "sample-package>=1.2.3",
```

### D1:py:tilth:9ebb4c65f:benchmark/fixtures/setup.py

Rule ids fired: none on sampled site

Diff:

```diff
- "sample-package==1.2.3",
+ "sample-package>=1.2.3",
```

### D1:py:tilth:8825050ec:benchmark/fixtures/setup.py

Rule ids fired: none on sampled site

Diff:

```diff
- "sample-package==1.2.3",
+ "sample-package>=1.2.3",
```

### T6:ts:tend2:fadcb3808:test/verify.test.ts:176

Rule ids fired: none on sampled line

Diff:

```diff
- await expect(verify()).rejects.toThrow("payload did not match");
+ await expect(verify()).rejects.toThrow();
```

### D2:go:hcl:6a91a7547:gohcl/types.go:6

Rule ids fired: none on sampled line

Diff:

```diff
+ import "github.com/hashicorp/hcl/hcldec"
+ gohcl code now imports the hcldec package.
```

### D1:py:tilth:c05475fd4:benchmark/fixtures/setup.py

Rule ids fired: none on sampled site

Diff:

```diff
- "sample-package==1.2.3",
+ "sample-package>=1.2.3",
```

### T6:ts:tend2:99296a937:test/verify.test.ts:176

Rule ids fired: none on sampled line

Diff:

```diff
- await expect(verify()).rejects.toThrow("payload did not match");
+ await expect(verify()).rejects.toThrow();
```

### D2:go:hcl:2efc26623:hclwrite/ast_body.go:6

Rule ids fired: none on sampled line

Diff:

```diff
+ import "github.com/hashicorp/hcl/integrationtest"
+ hclwrite code now imports the integrationtest package.
```

### T6:ts:tend2:e9e8387e9:test/verify.test.ts:176

Rule ids fired: none on sampled line

Diff:

```diff
- await expect(verify()).rejects.toThrow("payload did not match");
+ await expect(verify()).rejects.toThrow();
```

### T6:py:copeca:41cfc63ae:tests/config/test_loader.py:37

Rule ids fired: none on sampled line

Diff:

```diff
- with pytest.raises(ValueError, match="scenario file is invalid"):
+ with pytest.raises(ValueError):
```

### D1:py:tilth:ec06323c7:benchmark/fixtures/setup.py

Rule ids fired: none on sampled site

Diff:

```diff
- "sample-package==1.2.3",
+ "sample-package>=1.2.3",
```

### D2:go:hcl:0268c1604:gohcl/types.go:6

Rule ids fired: none on sampled line

Diff:

```diff
+ import "github.com/hashicorp/hcl/hcldec"
+ gohcl code now imports the hcldec package.
```

### T6:ts:tend2:e2adbab97:test/verify.test.ts:176

Rule ids fired: none on sampled line

Diff:

```diff
- await expect(verify()).rejects.toThrow("payload did not match");
+ await expect(verify()).rejects.toThrow();
```

### D1:py:tilth:e7ef4647e:benchmark/fixtures/setup.py

Rule ids fired: none on sampled site

Diff:

```diff
- "sample-package==1.2.3",
+ "sample-package>=1.2.3",
```

## Auditor response

The response is copied into `docs/calibration-audit-blind-2026-09.md`. The blocked sample re-grade was 20 of 20 in agreement with the calibration classifications after comparison, and the recall sample re-grade was 20 of 20 in agreement after comparison.

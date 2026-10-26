# calibration audit blind, 2026-09

Provider: opencode (`opencode run --model deepseek/deepseek-v4-pro --format json`, one fresh session per case)

Model: deepseek/deepseek-v4-pro

Blind: yes

Seed: calibration-audit-blind-2026-09-dogfood-1

Cases: fixtures/adversarial/calibration-audit/blind-2026-09/cases

Sessions: fixtures/adversarial/calibration-audit/blind-2026-09/sessions

Every case packet was written by `cargo xtask audit-packet --seed calibration-audit-blind-2026-09-dogfood-1 --dir fixtures/adversarial/calibration-audit/blind-2026-09/cases` from the pinned corpus, and `scripts/audit/blind-run.sh` handed each one on stdin to a fresh `opencode` session running `deepseek/deepseek-v4-pro`, in an empty directory with plugins off, as the session's only input. The raw event stream of each session is kept beside its case, each session record names the provider and the model this population was read by, and each answer opens with the SHA-256 of the packet it was given. The packet seals the judged content alone, the rule ids and each finding's level, path and line beside the commit's hunks, so a finding's wording can change without buying these sessions again. Nothing from the ledger reached a session; the agreement below is recomputed by `scripts/check/calibration-agreement.sh` from these tables and the report's own.

## Agreement

| Sample | Re-graded | Agreed | Agreement |
|---|---:|---:|---:|
| blocked commits | 19 | 15 | 78.9% |
| recall cases | 20 | 19 | 95.0% |

Agreement on blocked commits is on the binary question the ruling of 2026-09-06 allows a blind reader: is the rule's claim true of the change. A false-positive verdict is claim-false; true-positive and acceptable are both claim-true, and which of the two a human attaches is never audited. For the record, under the old three classes the same responses agree on 10 of 19 blocked commits.

## Blocked Commit Sample

| Repo | Commit | Auditor verdict | Reasoning |
|---|---|---|---|
| pleach | `624529b3a9` | acceptable | The flagged strings are a private-key regex pattern and a test fixture, not a leaked credential, so the change is fine though secret-looking strings were added. |
| tend2 | `067730ddd0` | true-positive | The `expect(read('dist/loop.js')).toBe(read('dist/loop.global.js'))` assertion was removed and replaced with a weaker banner-only check, so a real assertion was dropped and the test weakened. |
| tend2 | `08529d5f56` | true-positive | Each named file is a test with zero lines added and all lines deleted, so tests were genuinely deleted and coverage weakened. |
| tend2 | `2bb9d83f23` | acceptable | The removed scaffolding assertions were replaced by elision assertions matching report.ts's deliberate behavior change, so coverage is preserved and nothing was weakened. |
| tend2 | `52b97e43ce` | false-positive | The packet omits the diff entirely, so no secret-looking string is observable at lines 776 or 791; the claim cannot be confirmed true. |
| tend2 | `78fed73b0c` | acceptable | The claim is true—test files were deleted—but the entire `spikes/loop-hole/` directory, source and all, was removed as spike cleanup, not a targeted weakening of coverage. |
| tend2 | `bf2754689c` | acceptable | The deleted season.test.ts tested a `season` command that this same change removed; its coverage is replaced by the new next-status.test.ts, so the deletion is intentional and not a weakening. |
| tilth | `10bec56a41` | acceptable | An assertion was genuinely dropped, but the added comment justifies it as an unreliable substring check already pinned by `resolve_scope`'s own tests, so the block is appropriate friction, not a real weakening. |
| tilth | `11aef933c9` | acceptable | files.rs and its test module were deleted, so the claim is literally true, but the tests were migrated intact into list.rs, leaving no net loss of coverage. |
| tilth | `3ff87caf55` | acceptable | The deleted `test_bloom_filter_sizing` only asserted private sizing of the removed custom BloomFilter; fastbloom replaces it and all surviving tests keep their assertions, so no real coverage was weakened. |
| tilth | `59c87110ab` | acceptable | The deleted test covered the removed `percent_decode` helper, now replaced by a library function, so dropping it and its assertions is appropriate, not a weakening of coverage. |
| tilth | `7684e99e86` | acceptable | Assertions were removed during the apply_with_info→apply rename, but equivalent coverage (truncation marker and surviving body content) remains, so the test was not meaningfully weakened. |
| tilth | `ab7f054b73` | acceptable | The paths-only API was deliberately reverted to a singular `path`, so the deleted test enforced a now-removed invariant and its assertions are obsolete, making the removal correct rather than a weakening. |
| tilth | `bd36a43637` | true-positive | The diff deletes `src/index/symbol.rs` entirely, which contained a `#[cfg(test)] mod tests` with six `#[test]` functions, so a test was genuinely removed. |
| tilth | `d422a325fc` | false-positive | The finding names line 14, a comment in the non-test file src/install.rs, not an assertion; the one removed assertion (entry_style) was deleted alongside the field it tested. |
| umbel | `059b4c4677` | false-positive | The test file was rewritten for a legitimate behavior change (hooks moved to shared CODEX_HOME); every assertion was replaced or expanded, not dropped, so coverage was not weakened. |
| umbel | `75523a16a3` | true-positive | Line 179 contains `// TODO(opencode): verify tool-call part shape...`, a genuine TODO flagging unverified, inferred tool extraction that shipped in this new production source file. |
| umbel | `89c088bc08` | false-positive | The test retains an assertion (`expect(JSON.parse(out.trim())).toEqual([])`), and line 3 is an import, not a dropped assertion. |
| umbel | `cc3cc0c37c` | acceptable | The TODO comments are real, but they document an inferred, defensively-handled tool-call shape rather than a stub; the code works, so the block is friction. |

## Recall Case Sample

| Rule | Language | Repository | Commit | Path | Auditor verdict | Reasoning |
|---|---|---|---|---|---|---|
| D2 | go | hcl | `0268c1604` | `gohcl/types.go:6` | miss | The diff adds an hcldec import to gohcl/types.go, so D2's forbidden-boundary shape is present at the planted site, but weeder reported only C3 in an unrelated workflow file. |
| D2 | go | hcl | `2efc26623` | `hclwrite/ast_body.go:6` | miss | The diff adds `integrationtest` to `hclwrite/ast_body.go` imports at the planted site, so the forbidden-boundary import shape is present, but weeder reported no findings. |
| D2 | go | hcl | `92f12c4e5` | `hcldec/gob.go:6` | miss | The diff adds an import of `hcled` to `hcldec/gob.go`, matching the forbidden-boundary shape, but weeder reported no finding at that site. |
| D2 | go | hcl | `9466647a1` | `hclwrite/ast_block.go:6` | miss | The diff adds `github.com/hashicorp/hcl/v2/integrationtest` to `hclwrite`'s imports at the planted site, but weeder reported only C3 elsewhere, never D2 there. |
| D2 | go | hcl | `bd45ab812` | `hclwrite/format.go:6` | miss | The diff adds `github.com/hashicorp/hcl/v2/integrationtest` to `hclwrite/format.go` imports, matching D2's shape, but weeder reported only C3 elsewhere. |
| D2 | go | hcl | `e73f21667` | `gohcl/schema.go:6` | miss | The diff adds an `hcldec` import to `gohcl/schema.go`, matching the D2 shape, but weeder reported only C3 at a different file. |
| S2 | py | copeca | `387932ad5` | `tests/e2e/fake_agent.py:61` | miss | The try/except Exception: pass swallows an error from `argv.index("--")` at the planted site, and weeder reported nothing there. |
| T1 | go | cobra | `3f3b81882` | `doc/man_examples_test.go` | miss | The diff deletes the `ExampleGenManTree` example function, a test, matching rule T1's shape, but weeder reported no findings there. |
| T2 | py | copeca | `70d669542` | `tests/config/test_mode_models.py` | miss | The diff deletes `assert mode.name == "baseline"` from the changed test file, exactly matching T2, yet no finding reports T2 at that path. |
| T4 | rs | tilth | `5b0539e6a` | `src/mcp/write.rs:148` | not-a-case | The diff adds a new 10-second deadline but contains no widening of an existing wait from 1 to 10, so T4's shape is not genuinely present at line 148. |
| T4 | ts | pleach | `3a1306011` | `test/loop/run-work.test.ts:10` | miss | The change widens TIMEOUT from 1000 to 10000 at test/loop/run-work.test.ts:10, matching T4's shape, but weeder reported D1 at package.json:40 instead. |
| T6 | ts | tend2 | `044912212` | `test/route.test.ts:143` | miss | The diff at line 143 changes `toThrow(/sample/)` to `toThrow()`, dropping the error-name matcher and weakening the assertion, but weeder reported no findings. |
| T6 | ts | tend2 | `2a0e93933` | `test/verify.test.ts:176` | miss | The assertion changed from `toThrow(/description/)` to `toThrow()`, weakening it by removing the named error; weeder reported no finding there. |
| T6 | ts | tend2 | `32df8ddd1` | `test/verify.test.ts:176` | miss | The `toThrow(/description/)` assertion was changed to `toThrow()`, dropping the named error pattern, so T6's shape is genuinely present at line 176 and weeder reported nothing. |
| T6 | ts | tend2 | `56072cdab` | `test/verify.test.ts:176` | miss | The diff weakens `.toThrow(/description/)` to `.toThrow()` at test/verify.test.ts:176, but weeder reports no T6 finding there. |
| T6 | ts | tend2 | `5b1f7a471` | `test/verify.test.ts:176` | miss | The assertion changed from `.toThrow(/description/)` to `.toThrow()`, dropping the named error, which matches T6; weeder reported nothing. |
| T6 | ts | tend2 | `8d939ec15` | `test/route.test.ts:143` | miss | The assertion at line 143 changed from `toThrow(/sample/)` to `toThrow()`, weakening the error assertion by removing the expected error, and weeder reported no finding there. |
| T6 | ts | tend2 | `ad070a0d4` | `test/verify.test.ts:176` | miss | The assertion dropped its `/description/` matcher at line 176, weakening the error check, but no T6 finding names that site. |
| T6 | ts | tend2 | `e2adbab97` | `test/verify.test.ts:176` | miss | The assertion at line 176 changed from `toThrow(/description/)` to `toThrow()`, weakening the error assertion as T6 describes, but weeder reported no finding there. |
| T6 | ts | tend2 | `e9e8387e9` | `test/verify.test.ts:176` | miss | The assertion changed from `.toThrow(/description/)` to `.toThrow()`, weakening the error assertion at the planted site, but weeder reported only D1 elsewhere. |

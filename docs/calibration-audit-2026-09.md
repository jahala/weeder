# calibration audit, 2026-09

Provider: opencode (`opencode run --model deepseek/deepseek-v4-pro --format json`, one fresh session per case)

Model: deepseek/deepseek-v4-pro

Blind: no; the auditor could read the builder's classification and reasoning in docs/calibration-2026-09.md before judging.

Seed: calibration-audit-blind-2026-09-reseal-1

Sessions: fixtures/adversarial/calibration-audit/sighted-2026-09/sessions

The sample is the seeded one `cargo xtask audit-packet --seed calibration-audit-blind-2026-09-reseal-1` draws from the report, so this re-grade and the blind one beside it sample the same cases and differ only in what the auditor could see. `scripts/audit/sighted-run.sh` handed each case to a fresh `opencode` session running `deepseek/deepseek-v4-pro`, in an empty directory with plugins off, and the report itself on stdin as the session's only input. Each session record names the provider and the model, each answer opens with the SHA-256 of the report it was given, and the raw event stream is kept beside it. The agreement below is recomputed by `scripts/check/calibration-agreement.sh` from these tables and the report's own.

## Agreement

| Sample | Re-graded | Agreed | Agreement |
|---|---:|---:|---:|
| blocked commits | 20 | 20 | 100.0% |
| recall cases | 20 | 17 | 85.0% |

## Blocked Commit Sample

| Repo | Commit | Auditor verdict | Reasoning |
|---|---|---|---|
| copeca | `70d669542a` | false-positive | X1 flags `key` as a secret, but the line is a local holding a loaded public key, not a published credential, so the claim is not true of this change. |
| pleach | `624529b3a9` | false-positive | Both hits are pleach's own private-key-header detection regex and its fixture, not a credential; nothing secret was added or published, so X1's claim is not true here. |
| tend2 | `067730ddd0` | acceptable | T2's claim is true—5 assertions to 4—but the dropped check demanded byte-identity with dist/loop.global.js, a file this commit stops emitting, so the assertion could not hold a build that no longer exists. |
| tend2 | `08529d5f56` | acceptable | T1's claim is true—forty-odd test files were deleted—but the entire v1 lane they covered left in the same commit, so nothing shipped lost coverage and the gate correctly forced a human stop. |
| tend2 | `3adc642ce1` | false-positive | The case was not deleted but refactored into a loop over sibling directories, so the single it() still covers all inputs and no behaviour lost coverage. |
| tend2 | `52b97e43ce` | false-positive | X1's claim is not true: `detail-meta__key` and `journey-row__key` are CSS class names in the renderer's HTML markup, not secret values added to the change. |
| tend2 | `78fed73b0c` | acceptable | T1's claim is true — the spike's test files were deleted — and the 50 non-test files of the killed spike left in the same commit, so the tests died with their code. |
| tend2 | `bf2754689c` | acceptable | T1's claim is true — season.test.ts left with 4 cases — but the command it tested was removed and folded into one orientation command with its own tests, so the drop was fine. |
| tilth | `10bec56a41` | acceptable | T2 is true — the assertion count dropped — but the removed substring check was flaky, and resolve_scope behavior remains pinned in mod.rs unit tests, so the change is fine. |
| tilth | `11aef933c9` | acceptable | T1's claim of eight dropped cases is true, but the same commit adds list.rs with eight cases over the same ground, so coverage is preserved and the block is warranted friction. |
| tilth | `3ff87caf55` | acceptable | T1 and T2 correctly report a dropped case and six assertions, but they tested private fields of the hand-written BloomFilter this commit replaces with fastbloom, so the suite shrinking is the intended, review-worthy result. |
| tilth | `59c87110ab` | acceptable | T1/T2 claims are true — the case and four assertions pinning the hand-rolled percent decoder left when it was swapped for the percent-encoding crate, whose behavior now covers that ground. |
| tilth | `7684e99e86` | acceptable | T2's claim is true — one assertion dropped with the fork-only apply_with_info call — but the regression stays guarded since a cut at zero still must produce an x. |
| tilth | `ab7f054b73` | true-positive | Both claims are true—a case and three assertions left definitions.rs with no replacement—and the paths-only tilth_read schema they pinned is now covered by nobody, so coverage genuinely weakened. |
| tilth | `bd36a43637` | acceptable | T1's claim is true — six cases left with the deleted SymbolIndex type — but the tests exercised dead plumbing the commit removed, so the change was fine and the block is intended friction. |
| tilth | `d422a325fc` | acceptable | T2 is true—install.rs lost one assertion—but the folded entry_style is now asserted as the JsonLocal match in the same case, so coverage holds and the block is just review friction. |
| umbel | `059b4c4677` | acceptable | The assertion count fell as claimed, but only because the pinned hooks.json-in-cwd design was replaced by shared CODEX_HOME, and the new design is asserted in the same and two added test files. |
| umbel | `75523a16a3` | true-positive | S1's claim is accurate: the TODO in opencode.ts admitting unverified, inferred tool-call parsing shipped as production behaviour, which genuinely weakens the provider. |
| umbel | `89c088bc08` | acceptable | The assertion count did drop, but the removed `Array.isArray` check is redundant with the `toEqual([])` beside it, so the test still verifies the same thing. |
| umbel | `cc3cc0c37c` | true-positive | S1's claim is accurate: both provider files carry TODOs noting the tool-call shape is unverified against a real transcript, so unfinished work shipped as production behavior. |

## Recall Case Sample

| Rule | Language | Repository | Commit | Path | Auditor verdict | Reasoning |
|---|---|---|---|---|---|---|
| D2 | go | hcl | `0268c1604` | `gohcl/types.go:6` | miss | The hcldec import into gohcl was planted as a forbidden-boundary crossing and weeder reported nothing at gohcl/types.go:6. |
| D2 | go | hcl | `6a91a7547` | `gohcl/types.go:6` | miss | gohcl importing hcldec reverses the real dependency direction (hcldec builds on gohcl), a genuine forbidden-boundary import the injector confirmed was planted, and weeder reported nothing there. |
| D2 | go | hcl | `92f12c4e5` | `hcldec/gob.go:6` | not-a-case | The planted import target is `hcled`, a misspelling of `hcldec` naming no real package in the hcl repo, so no forbidden boundary was actually crossed. |
| D2 | go | hcl | `ab1acc486` | `hclwrite/tokens.go:6` | not-a-case | Importing a sibling `integrationtest` package is not a forbidden boundary crossing; D2 flags internal/test boundaries, not ordinary cross-package imports, so the planted shape was never genuinely present. |
| D2 | go | hcl | `bd45ab812` | `hclwrite/format.go:6` | miss | hclwrite importing the test-only integrationtest package is a genuine forbidden boundary crossing D2 exists to catch, and weeder reported nothing at that site. |
| D2 | go | hcl | `e73f21667` | `gohcl/schema.go:6` | not-a-case | `gohcl` importing `hcldec` is a peer production-package import with no configured boundary, so it does not cross a forbidden boundary D2 detects; the shape is not genuinely present. |
| S2 | py | copeca | `387932ad5` | `tests/e2e/fake_agent.py:61` | miss | A handler that catches an exception and says nothing is genuinely a swallowed error, matching S2, and weeder failed to report it at the planted site. |
| T1 | go | cobra | `3f3b81882` | `doc/man_examples_test.go` | miss | `ExampleGenManTree` is a Go example function inside a `_test.go` file, so deleting it is a test deleted and T1's shape is present but unreported. |
| T1 | ts | tend2 | `6d9a1cf91` | `test/site-paths.test.ts` | miss | The site is a test file and a test case was genuinely deleted, which is exactly T1's shape, and weeder reported nothing there. |
| T2 | py | copeca | `70d669542` | `tests/config/test_mode_models.py` | miss | Deleting the assertion on line 66 of a changed test file is exactly T2's shape, and weeder reported nothing at that site. |
| T4 | rs | tilth | `5b0539e6a` | `src/mcp/write.rs:148` | miss | A wait widened from 1 to 10 is a timeout widened, which is T4's shape, genuinely present at that site, and weeder did not report it there. |
| T4 | ts | pleach | `3a1306011` | `test/loop/run-work.test.ts:10` | miss | A wait was widened from 1000 to 10000, matching T4's tolerance-or-timeout-widening shape, and weeder did not report it at that site. |
| T6 | ts | tend2 | `2a0e93933` | `test/verify.test.ts:176` | miss | T6 covers a weakened error assertion, and stopping the assertion from naming the error is that shape, planted at the site weeder did not report. |
| T6 | ts | tend2 | `32df8ddd1` | `test/verify.test.ts:176` | miss | T6 flags weakened error assertions; removing the error name at test/verify.test.ts:176 is exactly that shape, and weeder did not report it at the site. |
| T6 | ts | tend2 | `5b1f7a471` | `test/verify.test.ts:176` | miss | The planted shape, an error assertion that stopped naming the error, is exactly T6's weakened-error-assertion pattern, and weeder did not report it at test/verify.test.ts:176. |
| T6 | ts | tend2 | `99296a937` | `test/verify.test.ts:176` | miss | T6 catches weakened error assertions, and dropping the error name at verify.test.ts:176 is that shape, planted and read back yet unreported by weeder. |
| T6 | ts | tend2 | `ad070a0d4` | `test/verify.test.ts:176` | miss | The weakened error assertion was planted at that site and confirmed present (unplantable is zero), yet weeder did not report it there. |
| T6 | ts | tend2 | `e2adbab97` | `test/verify.test.ts:176` | miss | An error assertion dropping its expected error name is T6's genuine shape, and weeder did not report it at test/verify.test.ts:176. |
| T6 | ts | tend2 | `e9e8387e9` | `test/verify.test.ts:176` | miss | Weakening an error assertion by dropping its error name is a genuine T6 shape, and weeder did not report it at the planted site. |
| T6 | ts | tend2 | `fadcb3808` | `test/verify.test.ts:176` | miss | An error assertion that stopped naming the error is genuinely a weakened assertion, T6's exact shape, and weeder did not report it at test/verify.test.ts:176. |

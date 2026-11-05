# calibration audit blind, 2026-09

Provider: opencode (`opencode run --model deepseek/deepseek-v4-pro --format json`, one fresh session per case)

Model: deepseek/deepseek-v4-pro

Blind: yes

Seed: calibration-audit-blind-2026-09-dogfood-3

Cases: fixtures/adversarial/calibration-audit/blind-2026-09/cases

Sessions: fixtures/adversarial/calibration-audit/blind-2026-09/sessions

Every case packet was written by `cargo xtask audit-packet --seed calibration-audit-blind-2026-09-dogfood-3 --dir fixtures/adversarial/calibration-audit/blind-2026-09/cases` from the pinned corpus, and `scripts/audit/blind-run.sh` handed each one on stdin to a fresh `opencode` session running `deepseek/deepseek-v4-pro`, in an empty directory with plugins off, as the session's only input. The raw event stream of each session is kept beside its case, each session record names the provider and the model this population was read by, and each answer opens with the SHA-256 of the packet it was given. The packet seals the judged content alone, the rule ids and each finding's level, path and line beside the commit's hunks, so a finding's wording can change without buying these sessions again. Nothing from the ledger reached a session; the agreement below is recomputed by `scripts/check/calibration-agreement.sh` from these tables and the report's own.

## Agreement

| Sample | Re-graded | Agreed | Agreement |
|---|---:|---:|---:|
| blocked commits | 20 | 14 | 70.0% |
| recall cases | 20 | 19 | 95.0% |

Agreement on blocked commits is on the binary question the ruling of 2026-09-06 allows a blind reader: is the rule's claim true of the change. A false-positive verdict is claim-false; true-positive and acceptable are both claim-true, and which of the two a human attaches is never audited. For the record, under the old three classes the same responses agree on 8 of 20 blocked commits.

## Blocked Commit Sample

| Repo | Commit | Auditor verdict | Reasoning |
|---|---|---|---|
| pleach | `2515d2ac88` | acceptable | The claim that a guardrail file was edited is true—`.claude/CLAUDE.md` was substantially rewritten—but the rewrite is legitimate documentation, not a weakening of protections. |
| pleach | `624529b3a9` | acceptable | The added strings are the secret-detector's own regex patterns and test fixtures, not real credentials; they are secret-looking but harmless, so the block is pure friction. |
| pleach | `76207691f3` | acceptable | The change does edit `.claude/CLAUDE.md`, a guardrail file, but only updates its description to reflect an adapters refactor, keeping it accurate rather than weakening it. |
| tend2 | `067730ddd0` | true-positive | The change removed `expect(read('dist/loop.js')).toBe(read('dist/loop.global.js'))` from the test, dropping an assertion that verified the built asset matches the global bundle. |
| tend2 | `08529d5f56` | true-positive | The commit deletes the entire test suite and reduces CLAUDE.md from ~230 lines to 2, so both claims are true and the change genuinely removes testing and guardrail protection. |
| tend2 | `2bb9d83f23` | false-positive | The test's assertions were rewritten, not dropped, to match a deliberate behavior change (empty sections now elide), and a new 206-line spec adds coverage. |
| tend2 | `bf2754689c` | acceptable | The file `test/season.test.ts` was indeed deleted, but only because the `season` command was intentionally removed and its coverage replaced by the new `test/next-status.test.ts`; nothing was weakened. |
| tilth | `10bec56a41` | true-positive | The change removed an `assert!` verifying the search result did not contain "cannot be resolved", leaving only `is_ok()`, which genuinely weakens the test's refusal-detection coverage. |
| tilth | `11aef933c9` | false-positive | The file was renamed as part of folding `tilth_files` into `tilth_list`; its tests were ported to `list.rs` and `tree.rs`, so no test was actually removed. |
| tilth | `3ff87caf55` | acceptable | The deleted test only verified the removed hand-rolled filter's internal sizing, so dropping it and its assertions is fine after switching to fastbloom. |
| tilth | `59c87110ab` | acceptable | The deleted test covered a private helper this commit removed and replaced with the percent-encoding crate, so dropping its assertions is legitimate, not a weakening. |
| tilth | `7684e99e86` | true-positive | The `assert!(info.at_line >= 2, ...)` assertion was removed, weakening coverage of the cut-point regression the test existed to guard. |
| tilth | `ab7f054b73` | acceptable | A test was genuinely deleted, but it enforced the paths-only API being deliberately replaced by path+paths, so removing it was correct and nothing was weakened. |
| tilth | `bd36a43637` | acceptable | The deleted file contained a `#[cfg(test)]` module with six tests, so a test was removed; but the whole dead `SymbolIndex` module was deleted, so dropping its tests is fine. |
| tilth | `d422a325fc` | false-positive | The change refactors a source file, not a test file, and no assertions were dropped; assertions were updated and added. |
| umbel | `059b4c4677` | false-positive | The file was refactored, not weakened: every original assertion was rewritten and new ones (CODEX_HOME, auth symlink, config copy, shared semantics) were added, so no assertions were dropped. |
| umbel | `75523a16a3` | acceptable | The `TODO(opencode)` at line 179 is a verification note inside a docstring, not a stub or unimplemented feature; the surrounding extraction code is complete and defensive, so blocking is friction. |
| umbel | `89c088bc08` | false-positive | The test still asserts `expect(JSON.parse(out.trim())).toEqual([])` and the finding's line 3 is an import line, so no assertions were dropped. |
| umbel | `e39da470b7` | acceptable | CLAUDE.md is a guardrail file and was added (line 13 within it), so the claim is true; this is fresh project initialization establishing guardrails, not weakening any existing constraint. |
| umbel | `ea995c590a` | acceptable | CLAUDE.md is a guardrail file and was genuinely edited (a new tmux-socket rule was added), so the claim is true; the edit strengthens rather than weakens guardrails, making this the intended friction. |

## Recall Case Sample

| Rule | Language | Repository | Commit | Path | Auditor verdict | Reasoning |
|---|---|---|---|---|---|---|
| D2 | go | hcl | `0268c1604` | `gohcl/types.go:6` | miss | The diff adds an `hcldec` import in `gohcl/types.go`, matching D2's forbidden-boundary import shape, but weeder reported only C3 elsewhere. |
| D2 | go | hcl | `6bf1a67a9` | `gohcl/decode.go:6` | miss | The diff adds an import of `hcldec` into `gohcl/decode.go`, which is the forbidden boundary crossing D2 describes, and weeder reported no finding there. |
| D2 | go | hcl | `92f12c4e5` | `hcldec/gob.go:6` | miss | The diff adds `github.com/hashicorp/hcl/v2/hcled` as an import in `hcldec/gob.go`, matching D2's forbidden-boundary shape, but weeder reported no finding. |
| D2 | go | hcl | `9466647a1` | `hclwrite/ast_block.go:6` | miss | The diff adds `github.com/hashicorp/hcl/v2/integrationtest` to `hclwrite`, a forbidden-boundary import, but weeder reported only C3 elsewhere, not D2 at the site. |
| D2 | go | hcl | `ab1acc486` | `hclwrite/tokens.go:6` | miss | The diff adds an `integrationtest` import to the `hclwrite` production package, matching D2's forbidden-boundary shape, but weeder reported no findings. |
| D2 | go | hcl | `bd45ab812` | `hclwrite/format.go:6` | miss | The diff adds an `integrationtest` import to `hclwrite/format.go`, matching D2's shape, but weeder reported only C3 in `.github/workflows/checks.yml`, not D2 at the planted site. |
| T1 | go | cobra | `3f3b81882` | `doc/man_examples_test.go` | miss | The `ExampleGenManTree` function in `man_examples_test.go` is an example test and was deleted, but weeder reported no findings. |
| T2 | py | copeca | `70d669542` | `tests/config/test_mode_models.py` | miss | The diff deletes `assert mode.name == "baseline"` from the test file, matching T2's shape, but no T2 finding is reported at that site. |
| T4 | rs | tilth | `5b0539e6a` | `src/mcp/write.rs:148` | not-a-case | The diff introduces a new ten-second deadline but changes no existing timeout, so nothing was widened from 1 to 10 at the planted site. |
| T6 | ts | tend2 | `044912212` | `test/route.test.ts:143` | miss | The assertion was weakened from `toThrow(/sample/)` to `toThrow()`, stopping it from naming the error; weeder reported nothing at that site. |
| T6 | ts | tend2 | `2a0e93933` | `test/verify.test.ts:176` | miss | The change replaced `toThrow(/description/)` with `toThrow()`, weakening the error assertion at the planted site, and weeder reported no finding there. |
| T6 | ts | tend2 | `42c840a73` | `test/verify.test.ts:176` | miss | The assertion changed from `.toThrow(/description/)` to `.toThrow()`, weakening it by dropping the error name, and weeder reported no rule at that site. |
| T6 | ts | tend2 | `51035a5c8` | `test/verify.test.ts:176` | miss | The diff changes `toThrow(/description/)` to `toThrow()`, removing the named error matcher, which is exactly the weakened assertion shape T6 describes, but weeder reported no finding there. |
| T6 | ts | tend2 | `56072cdab` | `test/verify.test.ts:176` | miss | The diff changes `toThrow(/description/)` to `toThrow()`, weakening the error assertion, which matches rule T6, but weeder reports no T6 finding at that site. |
| T6 | ts | tend2 | `5b1f7a471` | `test/verify.test.ts:176` | miss | The change removes the `/description/` matcher from `toThrow`, so the error assertion stopped naming the error, matching T6, but weeder reported no findings there. |
| T6 | ts | tend2 | `8d939ec15` | `test/route.test.ts:143` | miss | The assertion changed from `.toThrow(/sample/)` to `.toThrow()`, dropping the error matcher, which is exactly an error assertion stopped naming the error, and weeder reported nothing. |
| T6 | ts | tend2 | `99296a937` | `test/verify.test.ts:176` | miss | The change removes the `/description/` matcher from `.toThrow()`, so the error assertion was genuinely weakened, and weeder reported nothing. |
| T6 | ts | tend2 | `ad070a0d4` | `test/verify.test.ts:176` | miss | The change replaces `.toThrow(/description/)` with `.toThrow()`, weakening the error assertion at test/verify.test.ts:176, but weeder reported no T6 there. |
| T6 | ts | tend2 | `e9e8387e9` | `test/verify.test.ts:176` | miss | The assertion changed from `toThrow(/description/)` to `toThrow()`, dropping the named error, so T6's weakening shape is present at line 176 but weeder reported only D1. |
| T6 | ts | tend2 | `fadcb3808` | `test/verify.test.ts:176` | miss | The assertion `toThrow(/description/)` became bare `toThrow()`, dropping the error-name check, so T6's weakened-error-assertion shape is present at line 176 and weeder reported nothing. |

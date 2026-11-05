# calibration audit, 2026-09

Provider: opencode (`opencode run --model deepseek/deepseek-v4-pro --format json`, one fresh session per case)

Model: deepseek/deepseek-v4-pro

Blind: no; the auditor could read the builder's classification and reasoning in docs/calibration-2026-09.md before judging.

Seed: calibration-audit-blind-2026-09-dogfood-3

Sessions: fixtures/adversarial/calibration-audit/sighted-2026-09/sessions

The sample is the seeded one `cargo xtask audit-packet --seed calibration-audit-blind-2026-09-dogfood-3` draws from the report, so this re-grade and the blind one beside it sample the same cases and differ only in what the auditor could see. `scripts/audit/sighted-run.sh` handed each case to a fresh `opencode` session running `deepseek/deepseek-v4-pro`, in an empty directory with plugins off, and the report itself on stdin as the session's only input. Each session record names the provider and the model, each answer opens with the SHA-256 of the report it was given, and the raw event stream is kept beside it. The agreement below is recomputed by `scripts/check/calibration-agreement.sh` from these tables and the report's own.

## Agreement

| Sample | Re-graded | Agreed | Agreement |
|---|---:|---:|---:|
| blocked commits | 20 | 20 | 100.0% |
| recall cases | 20 | 18 | 90.0% |

## Blocked Commit Sample

| Repo | Commit | Auditor verdict | Reasoning |
|---|---|---|---|
| pleach | `2515d2ac88` | acceptable | C1's claim is true: the commit writes the non-negotiables section into a file that had none, adding seven rules rather than loosening anything. |
| pleach | `624529b3a9` | false-positive | X1's claim of an added private key is not true of this change; both hits are the detector's own header-matching regex and its test fixture, not a credential. |
| pleach | `76207691f3` | acceptable | The non-negotiables section did change, but only reworded to name adapters beside seams; the seven rule headings remain intact and nothing weakened. |
| tend2 | `067730ddd0` | acceptable | T2's assertion drop is real, but the removed check compared two dist files where this commit stops emitting the second, so nothing watchable was lost. |
| tend2 | `08529d5f56` | acceptable | T1's claim is true, but the deleted tests covered v1 code removed in the same commit and recoverable at the v1-final tag, so nothing shipped is less covered. |
| tend2 | `2bb9d83f23` | acceptable | The claim is true—assertions fell as empty sections were made to elide—but the test was rewritten to demand that behavior, so nothing is watched less. |
| tend2 | `bf2754689c` | acceptable | T1's claim is true — test/season.test.ts lost four cases — but the season command it tested was removed and folded into orientation with its own tests, so coverage moved with the feature. |
| tilth | `10bec56a41` | acceptable | T2's claim is true (an assertion was dropped), but it was a flaky substring match on real search output, and the refusal-versus-default behaviour stays pinned in mod.rs unit tests. |
| tilth | `11aef933c9` | acceptable | T1's claim is true, eight cases left files.rs, but list.rs adds eight cases over the same ground, so the coverage moved rather than weakened. |
| tilth | `3ff87caf55` | acceptable | T1 and T2 are true — the tests pinned private fields of the hand-written BloomFilter now replaced by fastbloom, so the removed cases held nothing still shipped. |
| tilth | `59c87110ab` | acceptable | The claim is true — a case and four assertions were dropped — but the change hands a hand-rolled percent decoder to the percent-encoding crate, so the behavior is now the library's and the block is expected friction. |
| tilth | `7684e99e86` | acceptable | T2's claim is true — an assertion went with the fork-only apply_with_info call — but the regression is still guarded since a zero cut leaves no x and the case still demands one. |
| tilth | `ab7f054b73` | true-positive | The paths-only tilth_read schema lost its case and assertions, nothing picks them up, so the behavior is now pinned by nobody. |
| tilth | `bd36a43637` | acceptable | T1 correctly reports that src/index/symbol.rs left the tree with six cases; they exercised dead SymbolIndex plumbing this commit deletes, so nothing shipped is less covered. |
| tilth | `d422a325fc` | acceptable | T2's claim is true (66 to 65 assertions), and the dropped assertion covered a field now folded into ConfigFormat, with its JsonLocal replacement asserted in the same case. |
| umbel | `059b4c4677` | acceptable | The dropped assertions pinned hooks.json in the worker's cwd, replaced by shared CODEX_HOME whose design is asserted in the same and added test files. |
| umbel | `75523a16a3` | true-positive | A TODO in production code admits tool-call parsing is unverified and inferred, shipping unfinished behavior, which is exactly what S1 is designed to catch. |
| umbel | `89c088bc08` | acceptable | The assertion count did drop from 18 to 17, but the removed check was redundant beside `toEqual([])`, and the test now runs non-destructively in a child process. |
| umbel | `e39da470b7` | acceptable | C1's claim is true, the principles changed by arriving as this commit first writes CLAUDE.md, and nothing existing was loosened, so the block is the friction C1 is meant to create. |
| umbel | `ea995c590a` | acceptable | C1's claim is true, the commit added a non-negotiable principle to CLAUDE.md, and strengthening the law is fine, so the block is the intended friction. |

## Recall Case Sample

| Rule | Language | Repository | Commit | Path | Auditor verdict | Reasoning |
|---|---|---|---|---|---|---|
| D2 | go | hcl | `0268c1604` | `gohcl/types.go:6` | miss | The planted `gohcl` import of `hcldec` at `types.go:6` is a genuine cross-boundary import matching D2's shape, and weeder reported nothing there. |
| D2 | go | hcl | `6bf1a67a9` | `gohcl/decode.go:6` | not-a-case | gohcl legitimately imports hcldec in the real hcl codebase, so the planted import is that package's actual dependency, not a forbidden-boundary crossing. |
| D2 | go | hcl | `92f12c4e5` | `hcldec/gob.go:6` | not-a-case | The planted import target "hcled" is not a real hcl package, so no forbidden boundary was crossed and D2's shape is absent. |
| D2 | go | hcl | `9466647a1` | `hclwrite/ast_block.go:6` | miss | The planted import adds `integrationtest`, a test package, to the production package `hclwrite`, a genuine forbidden prod-to-test boundary crossing at line 6 that weeder left unreported. |
| D2 | go | hcl | `ab1acc486` | `hclwrite/tokens.go:6` | miss | The plant adds an `integrationtest` import to the production package `hclwrite`, a genuine forbidden-boundary crossing, and weeder did not report it there. |
| D2 | go | hcl | `bd45ab812` | `hclwrite/format.go:6` | miss | hclwrite is production code importing the test-only package integrationtest, so the import at format.go:6 genuinely crosses a forbidden boundary that weeder did not report. |
| T1 | go | cobra | `3f3b81882` | `doc/man_examples_test.go` | miss | ExampleGenManTree is a Go example function in a _test.go file, so deleting it is a genuine test deletion that weeder failed to report. |
| T2 | py | copeca | `70d669542` | `tests/config/test_mode_models.py` | miss | The deleted assertion is the canonical T2 shape, the injector verified it present (not listed unplantable), and weeder reported nothing on that file. |
| T4 | rs | tilth | `5b0539e6a` | `src/mcp/write.rs:148` | miss | A wait widened from 1 to 10 is a timeout widening matching T4's shape, planted at the site and not reported there. |
| T6 | ts | tend2 | `044912212` | `test/route.test.ts:143` | miss | The report records an error assertion planted at that site that stopped naming the error, and weeder did not report it there; nothing contradicts the shape being genuinely present. |
| T6 | ts | tend2 | `2a0e93933` | `test/verify.test.ts:176` | miss | An assertion that stopped naming the specific error is a genuine weakening of an error assertion, matching T6's shape, and weeder did not fire on the planted site. |
| T6 | ts | tend2 | `42c840a73` | `test/verify.test.ts:176` | miss | T6 finds a weakened error assertion, and a planted assertion that stops naming the error is that shape; weeder reported nothing at the site, so it is a genuine miss. |
| T6 | ts | tend2 | `51035a5c8` | `test/verify.test.ts:176` | miss | An error assertion stripped of its error name is squarely T6's weakened-assertion shape, and weeder reported nothing at that site, so the recorded miss holds. |
| T6 | ts | tend2 | `56072cdab` | `test/verify.test.ts:176` | miss | T6 flags a weakened error assertion, and removing the error's name from an assertion at test/verify.test.ts:176 is that shape, present and unreported. |
| T6 | ts | tend2 | `5b1f7a471` | `test/verify.test.ts:176` | miss | An error assertion stripped of its error name is a genuine T6 weakening, and weeder did not report it at the planted site. |
| T6 | ts | tend2 | `8d939ec15` | `test/route.test.ts:143` | miss | T6 catches a weakened error assertion, and the planted change at that site stopped naming the error, which is precisely that shape, yet weeder did not report it. |
| T6 | ts | tend2 | `99296a937` | `test/verify.test.ts:176` | miss | The report lists this under "Every miss" as planted and read back, so the weakened error assertion is genuinely present and weeder did not report it there. |
| T6 | ts | tend2 | `ad070a0d4` | `test/verify.test.ts:176` | miss | The planted change weakens an error assertion by removing the named error, matching T6's shape, and weeder did not report it there. |
| T6 | ts | tend2 | `e9e8387e9` | `test/verify.test.ts:176` | miss | An assertion that stopped naming the error genuinely weakens the error assertion, matching T6's shape, and weeder did not report it at that site. |
| T6 | ts | tend2 | `fadcb3808` | `test/verify.test.ts:176` | miss | An error assertion that stops naming the error is a genuine T6 weakening, and the report confirms weeder did not flag the planted site. |

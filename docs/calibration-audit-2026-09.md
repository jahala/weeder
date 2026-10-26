# calibration audit, 2026-09

Provider: opencode (`opencode run --model deepseek/deepseek-v4-pro --format json`, one fresh session per case)

Model: deepseek/deepseek-v4-pro

Blind: no; the auditor could read the builder's classification and reasoning in docs/calibration-2026-09.md before judging.

Seed: calibration-audit-blind-2026-09-dogfood-1

Sessions: fixtures/adversarial/calibration-audit/sighted-2026-09/sessions

The sample is the seeded one `cargo xtask audit-packet --seed calibration-audit-blind-2026-09-dogfood-1` draws from the report, so this re-grade and the blind one beside it sample the same cases and differ only in what the auditor could see. `scripts/audit/sighted-run.sh` handed each case to a fresh `opencode` session running `deepseek/deepseek-v4-pro`, in an empty directory with plugins off, and the report itself on stdin as the session's only input. Each session record names the provider and the model, each answer opens with the SHA-256 of the report it was given, and the raw event stream is kept beside it. The agreement below is recomputed by `scripts/check/calibration-agreement.sh` from these tables and the report's own.

## Agreement

| Sample | Re-graded | Agreed | Agreement |
|---|---:|---:|---:|
| blocked commits | 19 | 19 | 100.0% |
| recall cases | 20 | 17 | 85.0% |

## Blocked Commit Sample

| Repo | Commit | Auditor verdict | Reasoning |
|---|---|---|---|
| pleach | `624529b3a9` | false-positive | The added private-key strings are the detector's own regex and its fixture, not a committed credential, so X1's claim is not true of this change. |
| tend2 | `067730ddd0` | acceptable | T2 is true — one assertion dropped because the file it compared is no longer emitted — and the remaining assertions still hold the banner and css header. |
| tend2 | `08529d5f56` | acceptable | T1's claim is true — test files were deleted — but they covered the v1 lane that the same commit removed wholesale, so nothing shipped is less covered. |
| tend2 | `2bb9d83f23` | acceptable | The assertion drop is real, but it reflects an intentional behavior change—empty sections now elide—and the test was rewritten to demand exactly that, so nothing is watched less. |
| tend2 | `52b97e43ce` | false-positive | X1 misread class attributes `class="...__key">` as secret assignments; the values are renderer markup, not credentials, so no secret was added. |
| tend2 | `78fed73b0c` | acceptable | T1 correctly reports the spike's test files were deleted, and the 50 non-test files went with them, so no shipped behavior lost coverage. |
| tend2 | `bf2754689c` | acceptable | T1's claim is true—the season test file left with the command it tested—and the change was fine, since orientation folded into one command with its own tests. |
| tilth | `10bec56a41` | acceptable | T2's claim is true (three assertions to two), but the removed substring match was flaky and the refusal-versus-default behavior stays pinned in mod.rs unit tests, so nothing meaningful was weakened. |
| tilth | `11aef933c9` | acceptable | T1's claim is true — files.rs and its 8 cases were removed — but the same commit adds list.rs with 8 equivalent cases, so coverage moved, not weakened. |
| tilth | `3ff87caf55` | acceptable | The claim is true — tests over the hand-written BloomFilter's private fields left with the code — but swapping to fastbloom makes those tests unreachable, so the change was fine. |
| tilth | `59c87110ab` | acceptable | T1 and T2 correctly note the dropped case and assertions, but the hand-rolled decoder's behavior moved into the percent-encoding crate, so the change was sound. |
| tilth | `7684e99e86` | acceptable | T2's claim is true, an assertion went with the fork-only apply_with_info call, but the regression stays guarded since a zero cut yields no x. |
| tilth | `ab7f054b73` | true-positive | The removed case and assertions pinned the paths-only schema, which the commit narrowed to a singular path, leaving the resulting schema pinned by no test. |
| tilth | `bd36a43637` | acceptable | T1's claim is true, six cases left with src/index/symbol.rs, but they tested dead SymbolIndex plumbing deleted in the same commit, so the block is warranted friction. |
| tilth | `d422a325fc` | acceptable | The assertion drop is real but the folded JsonLocal match is asserted in the same case, so coverage is preserved. |
| umbel | `059b4c4677` | acceptable | T2's claim is true, the dropped assertions pinned the worker-cwd hooks.json this commit replaces with CODEX_HOME, and the replacement design is asserted in the same file and two added test files. |
| umbel | `75523a16a3` | true-positive | The TODO genuinely ships in production code at src/core/providers/opencode.ts, marking unverified tool-call parsing that relies on inferred shapes, so unfinished behavior really did reach production. |
| umbel | `89c088bc08` | acceptable | T2's claim is true — an assertion was dropped — but it was redundant, implied by the adjacent `toEqual([])`, and the change avoids destructive tmux teardown. |
| umbel | `cc3cc0c37c` | true-positive | S1's claim holds: TODO markers in codex.ts and gemini.ts admit unverified tool-call parsing shipped as behaviour, naming work the commit did not finish. |

## Recall Case Sample

| Rule | Language | Repository | Commit | Path | Auditor verdict | Reasoning |
|---|---|---|---|---|---|---|
| D2 | go | hcl | `0268c1604` | `gohcl/types.go:6` | not-a-case | gohcl legitimately depends on hcldec in hashicorp/hcl, so importing it crosses no forbidden boundary; the planted import is not an instance of D2's shape. |
| D2 | go | hcl | `2efc26623` | `hclwrite/ast_body.go:6` | miss | hclwrite importing integrationtest is a real forbidden-boundary crossing that D2 exists to catch, and weeder reported nothing at that site, so the shape was genuinely present and missed. |
| D2 | go | hcl | `92f12c4e5` | `hcldec/gob.go:6` | not-a-case | The miss plants `hcldec` importing `hcled`, a non-existent self-typo rather than a real forbidden boundary crossing, so the D2 shape is not genuinely present at that site. |
| D2 | go | hcl | `9466647a1` | `hclwrite/ast_block.go:6` | miss | hclwrite is production and integrationtest is test-only, so the planted import crosses the production/test boundary D2 forbids; weeder did not report it there. |
| D2 | go | hcl | `bd45ab812` | `hclwrite/format.go:6` | miss | The injector wrote a genuine forbidden import, production `hclwrite` importing the test-only `integrationtest` package, confirmed it on read-back, and weeder reported nothing at that site. |
| D2 | go | hcl | `e73f21667` | `gohcl/schema.go:6` | not-a-case | gohcl and hcldec are independent sibling packages in hcl/v2 with no forbidden boundary between them, so this import crosses no genuine boundary; weeder rightly ignored it. |
| S2 | py | copeca | `387932ad5` | `tests/e2e/fake_agent.py:61` | miss | A catch handler that silently discards the exception is the S2 "error swallowed" shape, planted at the site and not reported by weeder. |
| T1 | go | cobra | `3f3b81882` | `doc/man_examples_test.go` | miss | Deleting `ExampleGenManTree` from a `_test.go` file removes a Go example test, which T1's deleted-test shape covers, yet weeder reported nothing there. |
| T2 | py | copeca | `70d669542` | `tests/config/test_mode_models.py` | miss | An assertion was deleted from a changed Python test file, matching T2's shape, and weeder did not report it at the planted site. |
| T4 | rs | tilth | `5b0539e6a` | `src/mcp/write.rs:148` | miss | A wait widened from 1 to 10 at the planted site is a timeout widened, exactly T4's shape, and weeder did not report it there. |
| T4 | ts | pleach | `3a1306011` | `test/loop/run-work.test.ts:10` | miss | A wait was widened from 1000 to 10000, which is a timeout widened and matches T4's shape, yet weeder did not report it at the planted site. |
| T6 | ts | tend2 | `044912212` | `test/route.test.ts:143` | miss | The planted shape, an error assertion that stopped naming the error, is a genuine weakening of an error assertion, so T6's shape is present and weeder reported nothing there. |
| T6 | ts | tend2 | `2a0e93933` | `test/verify.test.ts:176` | miss | An error assertion was planted weakened by dropping its named error, which is T6's shape, and weeder reported nothing at test/verify.test.ts:176. |
| T6 | ts | tend2 | `32df8ddd1` | `test/verify.test.ts:176` | miss | The planted T6 shape, an error assertion no longer naming its error, is genuinely present at test/verify.test.ts:176 and weeder did not report it there. |
| T6 | ts | tend2 | `56072cdab` | `test/verify.test.ts:176` | miss | The planted shape, an error assertion dropping its named error, is exactly the weakened-assertion shape T6 detects, and weeder reported nothing at that site. |
| T6 | ts | tend2 | `5b1f7a471` | `test/verify.test.ts:176` | miss | An assertion that stops naming the specific error (dropping its message or type) is a genuine weakening of an error assertion, so the T6 shape is present and weeder failed to report it. |
| T6 | ts | tend2 | `8d939ec15` | `test/route.test.ts:143` | miss | The planted shape, an error assertion that stopped naming the error, is a genuine weakening under T6, and the report records weeder did not fire at that site. |
| T6 | ts | tend2 | `ad070a0d4` | `test/verify.test.ts:176` | miss | An error assertion that stops naming the error is a genuine weakening of that assertion, matching T6, and weeder did not report it at the planted site. |
| T6 | ts | tend2 | `e2adbab97` | `test/verify.test.ts:176` | miss | The planted shape, an error assertion that stopped naming the error, matches T6 and was unreported at test/verify.test.ts:176. |
| T6 | ts | tend2 | `e9e8387e9` | `test/verify.test.ts:176` | miss | The planted change stopped an error assertion from naming the error, which is T6's shape, and the injector read it back, so it was present and unreported. |

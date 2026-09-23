# calibration audit, 2026-09

Provider: opencode (`opencode run --model deepseek/deepseek-v4-pro --format json`, one fresh session per case)

Model: deepseek/deepseek-v4-pro

Blind: no; the auditor could read the builder's classification and reasoning in docs/calibration-2026-09.md before judging.

Seed: calibration-audit-blind-2026-09-law-1

Sessions: fixtures/adversarial/calibration-audit/sighted-2026-09/sessions

The sample is the seeded one `cargo xtask audit-packet --seed calibration-audit-blind-2026-09-law-1` draws from the report, so this re-grade and the blind one beside it sample the same cases and differ only in what the auditor could see. `scripts/audit/sighted-run.sh` handed each case to a fresh `opencode` session running `deepseek/deepseek-v4-pro`, in an empty directory with plugins off, and the report itself on stdin as the session's only input. Each session record names the provider and the model, each answer opens with the SHA-256 of the report it was given, and the raw event stream is kept beside it. The agreement below is recomputed by `scripts/check/calibration-agreement.sh` from these tables and the report's own.

## Agreement

| Sample | Re-graded | Agreed | Agreement |
|---|---:|---:|---:|
| blocked commits | 20 | 20 | 100.0% |
| recall cases | 20 | 17 | 85.0% |

## Blocked Commit Sample

| Repo | Commit | Auditor verdict | Reasoning |
|---|---|---|---|
| copeca | `70d669542a` | false-positive | `key = serialization.load_pem_public_key(pem)` assigns a public key object to a local variable, not a secret-looking string; nothing sensitive was added or published. |
| pleach | `624529b3a9` | false-positive | The matched string is pleach's own private-key detection regex and its fixture, not a credential, so nothing sensitive was published. |
| tend2 | `067730ddd0` | acceptable | The assertion drop is real, but it held a byte-identical check on a dist file this commit stops emitting, so nothing is weakened. |
| tend2 | `08529d5f56` | acceptable | T1's claim is true—the v1 sunset deletes dozens of test files—but the code they covered leaves in the same commit, so nothing shipped is less covered. |
| tend2 | `2bb9d83f23` | acceptable | The assertion drop from 23 to 19 is real, but it follows an intentional behavior change—empty sections elide—with the test rewritten to demand that new behavior, so nothing is watched less. |
| tend2 | `52b97e43ce` | false-positive | X1 read `class="...__key">` markup as a secret assignment; both hits are HTML class attributes, not secret-looking values, so the claim is not true of this change. |
| tend2 | `78fed73b0c` | acceptable | T1's claim is true, but the spike tree was deleted wholesale with its production code, so the removed tests covered nothing still shipped. |
| tend2 | `bf2754689c` | acceptable | T1's claim that test/season.test.ts lost its cases is true, but the command those cases tested was removed and its replacement has its own tests, so nothing weakened. |
| tilth | `10bec56a41` | acceptable | T2's claim is true (search.rs dropped from 3 assertions to 2), and the dropped substring check was flaky while resolve_scope and propagation success remain pinned. |
| tilth | `11aef933c9` | acceptable | T1's claim is true—files.rs lost 8 cases—but the commit adds list.rs with 8 equivalent cases over the same ground, so coverage moved rather than weakened. |
| tilth | `3ff87caf55` | acceptable | The dropped case and assertions exercised private fields of the hand-written BloomFilter, which this commit replaces with fastbloom, so the claim is true and the coverage loss is fine. |
| tilth | `59c87110ab` | acceptable | The claim is true; the dropped case and assertions pinned a hand-rolled percent decoder now delegated to the percent-encoding crate, so the covered behavior remains guarded by the library. |
| tilth | `7684e99e86` | acceptable | T2's count is true, but the dropped assertion followed a fork-only API call, and the regression is still guarded by the surviving output check. |
| tilth | `ab7f054b73` | true-positive | The commit deletes the only test pinning the paths-only tilth_read schema and reverts it to a singular path, so the coverage is dropped without replacement rather than moved. |
| tilth | `bd36a43637` | acceptable | T1's claim is true—six cases left with the deleted SymbolIndex type—but the type was inert and never consulted, so its tests stopped existing with it. |
| tilth | `d422a325fc` | acceptable | T2's claim is true (66 assertions down to 65), but the removed check covered entry_style folded into ConfigFormat, and the JsonLocal match replacing it is asserted in the same case. |
| umbel | `059b4c4677` | acceptable | The claim is true (codex.test.ts dropped assertions) but they pinned a cwd-local hooks.json design being replaced by shared CODEX_HOME, which is asserted in the same file plus two added test files. |
| umbel | `75523a16a3` | true-positive | The TODO is genuinely present in production code, admitting the tool-call shape is inferred and unverified, so unfinished parsing shipped as behaviour is exactly what S1 guards against. |
| umbel | `89c088bc08` | acceptable | T2's claim is true, one assertion was dropped, but it was redundant beside a `toEqual([])` that already implies it, and the change made the test less destructive. |
| umbel | `cc3cc0c37c` | true-positive | The claim is true: TODO comments in codex.ts and gemini.ts admit the tool-call shape is unverified against a real transcript, so unfinished parsing shipped as behavior. |

## Recall Case Sample

| Rule | Language | Repository | Commit | Path | Auditor verdict | Reasoning |
|---|---|---|---|---|---|---|
| D2 | go | hcl | `0268c1604` | `gohcl/types.go:6` | not-a-case | gohcl importing hcldec is a legal sibling-package import within one module, crossing no internal or module boundary, so D2's forbidden-boundary shape is not genuinely present. |
| D2 | go | hcl | `2efc26623` | `hclwrite/ast_body.go:6` | miss | The planted import makes `hclwrite` import `integrationtest`, a forbidden boundary crossing D2 should flag, and weeder reported nothing at `hclwrite/ast_body.go:6`. |
| D2 | go | hcl | `6bf1a67a9` | `gohcl/decode.go:6` | miss | gohcl importing hcldec is a genuine forbidden boundary since hcldec imports gohcl, forming a cycle, and weeder did not fire D2 at decode.go:6. |
| D2 | go | hcl | `92f12c4e5` | `hcldec/gob.go:6` | not-a-case | The planted import target `hcled` is not a real hcl package (a typo), so no genuine forbidden boundary was crossed at that site. |
| D2 | go | hcl | `9466647a1` | `hclwrite/ast_block.go:6` | miss | The planted import has the production package `hclwrite` importing the test-only `integrationtest` package, a genuine forbidden-boundary crossing that D2 should catch, and weeder reported nothing there. |
| D2 | go | hcl | `e73f21667` | `gohcl/schema.go:6` | not-a-case | gohcl is a documented convenience layer over hcldec, so importing hcldec is a legitimate intra-module dependency, not a forbidden boundary crossing that D2 detects. |
| S2 | py | copeca | `387932ad5` | `tests/e2e/fake_agent.py:61` | miss | A handler that catches an exception and says nothing is the S2 swallowed-error shape, and it was planted at that site but not reported there. |
| T4 | rs | tilth | `5b0539e6a` | `src/mcp/write.rs:148` | miss | A wait widened from 1 to 10 is a timeout widened, which is the shape T4 targets, and it went unreported at the planted site. |
| T4 | ts | pleach | `3a1306011` | `test/loop/run-work.test.ts:10` | miss | A wait widened from 1000 to 10000 is a timeout/tolerance widening, squarely T4's shape, and weeder did not report it at the planted site. |
| T6 | ts | tend2 | `044912212` | `test/route.test.ts:143` | miss | The planted change drops the error's name from an assertion, exactly T6's "error assertion weakened" shape, and weeder reported nothing at that site. |
| T6 | ts | tend2 | `42c840a73` | `test/verify.test.ts:176` | miss | The injector planted a weakened error assertion at test/verify.test.ts:176, read back and confirmed present (T6 ts has zero unplantable), and weeder did not report T6 there. |
| T6 | ts | tend2 | `51035a5c8` | `test/verify.test.ts:176` | miss | The planted change removed the error name from an error assertion, exactly T6's weakened-assertion shape, the injector confirmed it persisted, and weeder reported nothing at that site. |
| T6 | ts | tend2 | `56072cdab` | `test/verify.test.ts:176` | miss | The planted mutation, an error assertion stripped of its error name, is the T6 shape and weeder did not report it at that site. |
| T6 | ts | tend2 | `5b1f7a471` | `test/verify.test.ts:176` | miss | An assertion that no longer names the error is a genuine weakening of an error assertion, and weeder did not report it at the planted site. |
| T6 | ts | tend2 | `6d9a1cf91` | `test/route.test.ts:143` | miss | The planted change, an error assertion that stopped naming the error, is the T6 shape and weeder reported nothing at that site. |
| T6 | ts | tend2 | `8d939ec15` | `test/route.test.ts:143` | miss | The planted shape, an error assertion weakened by dropping the error's name, is T6's shape and was reported uncaught at that site. |
| T6 | ts | tend2 | `99296a937` | `test/verify.test.ts:176` | miss | The planted shape, an error assertion stripped of its named error, is a genuine T6 weakening and weeder did not report it at that site. |
| T6 | ts | tend2 | `ad070a0d4` | `test/verify.test.ts:176` | miss | Dropping the error name or message from an assertion is a genuine weakening of an error assertion, exactly what T6 covers, and weeder reported nothing at the site. |
| T6 | ts | tend2 | `e2adbab97` | `test/verify.test.ts:176` | miss | An error assertion that stops naming the error is a genuine weakening, which is exactly T6's shape, and weeder did not report it at that site. |
| T6 | ts | tend2 | `fadcb3808` | `test/verify.test.ts:176` | miss | The planted shape, an error assertion that stopped naming the error, is a genuine T6 weakened-assertion pattern at that site and weeder did not report it there. |

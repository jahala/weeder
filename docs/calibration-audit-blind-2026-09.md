# calibration audit blind, 2026-09

Provider: opencode (`opencode run --model deepseek/deepseek-v4-pro --format json`, one fresh session per case)

Model: deepseek/deepseek-v4-pro

Blind: yes

Seed: calibration-audit-blind-2026-09-dogfood-4

Cases: fixtures/adversarial/calibration-audit/blind-2026-09/cases

Sessions: fixtures/adversarial/calibration-audit/blind-2026-09/sessions

Every case packet was written by `cargo xtask audit-packet --seed calibration-audit-blind-2026-09-dogfood-4 --dir fixtures/adversarial/calibration-audit/blind-2026-09/cases` from the pinned corpus, and `scripts/audit/blind-run.sh` handed each one on stdin to a fresh `opencode` session running `deepseek/deepseek-v4-pro`, in an empty directory with plugins off, as the session's only input. The raw event stream of each session is kept beside its case, each session record names the provider and the model this population was read by, and each answer opens with the SHA-256 of the packet it was given. The packet seals the judged content alone, the rule ids and each finding's level, path and line beside the commit's hunks, so a finding's wording can change without buying these sessions again. Nothing from the ledger reached a session; the agreement below is recomputed by `scripts/check/calibration-agreement.sh` from these tables and the report's own.

## Agreement

| Sample | Re-graded | Agreed | Agreement |
|---|---:|---:|---:|
| blocked commits | 20 | 17 | 85.0% |
| recall cases | 20 | 19 | 95.0% |

Agreement on blocked commits is on the binary question the ruling of 2026-09-06 allows a blind reader: is the rule's claim true of the change. A false-positive verdict is claim-false; true-positive and acceptable are both claim-true, and which of the two a human attaches is never audited. For the record, under the old three classes the same responses agree on 16 of 20 blocked commits.

## Blocked Commit Sample

| Repo | Commit | Auditor verdict | Reasoning |
|---|---|---|---|
| pleach | `2515d2ac88` | true-positive | The diff edits `.claude/CLAUDE.md`, a guardrail file, satisfying rule C1's claim verbatim. |
| pleach | `624529b3a9` | acceptable | The private-key marker string matches the secret pattern, but it is the scanner's own detector regex and test fixture, not a real credential. |
| tend2 | `08529d5f56` | acceptable | Every listed file was deleted wholesale: the entire repo (source, tests, skills, CLAUDE.md) was removed in one migration, so the rules' claims are literally true but the block is friction, not a real weakening. |
| tend2 | `0bfa87f287` | acceptable | CLAUDE.md is a guardrail file and was indeed edited, but the change reorganizes and expands its content without weakening any rule, so the block is friction. |
| tend2 | `2bb9d83f23` | acceptable | Assertions about empty-form rendering were indeed removed, but only to match a deliberate behavior change (presence-driven elision), with equivalent assertions added; the test was not weakened. |
| tend2 | `78fed73b0c` | acceptable | The claim is true—test files were deleted—but the entire spikes/loop-hole spike directory was removed wholesale along with its source, so no real coverage was weakened. |
| tend2 | `bf2754689c` | acceptable | The test was genuinely deleted, but it covered the removed `season` command, whose behavior was folded into `next` and re-covered by the new `test/next-status.test.ts`, including a "season is gone" check. |
| tilth | `11aef933c9` | acceptable | The deleted files.rs test module was replaced by an equivalent test module in list.rs, so the change was a rename, not a loss of coverage. |
| tilth | `3ff87caf55` | acceptable | The only removed assertions belonged to `test_bloom_filter_sizing`, which tested the deleted hand-rolled BloomFilter; every remaining test kept its assertions, so the block is friction. |
| tilth | `59c87110ab` | acceptable | The deleted test covered a hand-rolled decoder now replaced by the standard percent-encoding crate, and the end-to-end path test remains, so the block is friction. |
| tilth | `7684e99e86` | acceptable | The `info.at_line >= 2` assertion was dropped, but the retained `out.contains('x')` assertion catches the same empty-body collapse, so coverage is preserved. |
| tilth | `ab7f054b73` | true-positive | The diff deletes the `tilth_read_schema_is_paths_only` test, which asserted `path` absent and `paths` required, directly contradicting the API change that reintroduces `path`. |
| tilth | `bd36a43637` | acceptable | The rule's claim is true—the deleted `symbol.rs` contained a `#[cfg(test)]` test module—but those tests covered an unused dead feature that was removed, so the deletion is legitimate cleanup. |
| tilth | `d422a325fc` | false-positive | `src/install.rs` is not a test file, and the diff adds panic!/assert_eq! arms rather than dropping any assertions; line 14 is only a comment edit. |
| umbel | `059b4c4677` | false-positive | The test file was rewritten to match a deliberate behavior change (codex hooks moved to a shared CODEX_HOME); removed assertions were replaced by equivalent assertions covering the new behavior, so coverage was migrated, not dropped. |
| umbel | `7109a41bd9` | acceptable | CLAUDE.md was edited, but only a mechanical rctrl→umbel rename consistent with the rest of the change, weakening no guardrail. |
| umbel | `75523a16a3` | true-positive | Line 179 is a genuine `TODO(opencode)` marking inferred, unverified tool-call part shape shipped in production extraction logic, so the claim holds and the block is warranted. |
| umbel | `89c088bc08` | acceptable | One redundant assertion (`Array.isArray(sessions)`) was removed, but the test still asserts `toEqual([])` via a safer isolated adapter, so nothing meaningful was weakened. |
| umbel | `cc3cc0c37c` | true-positive | Both findings point at real `TODO(b3-…)` comments shipped in production source that flag unverified tool-extraction parsing, a genuine unresolved correctness gap the rule exists to catch. |
| umbel | `e39da470b7` | acceptable | CLAUDE.md is a guardrail file and it was created here, so the claim is technically true, but the change is fresh repository initialization that weakens nothing. |

## Recall Case Sample

| Rule | Language | Repository | Commit | Path | Auditor verdict | Reasoning |
|---|---|---|---|---|---|---|
| S2 | py | copeca | `387932ad5` | `tests/e2e/fake_agent.py:61` | miss | The diff adds a bare `except Exception: pass` that swallows the error, matching rule S2 at line 61, but weeder reported no finding there. |
| S2 | rs | anyhow | `2cbb4e1bc` | `src/error.rs:544` | miss | The `?` error propagation was replaced with `.unwrap_or_default()`, converting a failure into a default and swallowing the error at the planted site, but weeder reported no finding there. |
| S2 | rs | anyhow | `30e1a6f73` | `src/error.rs:544` | miss | The `?` propagation was replaced with `.unwrap_or_default()`, swallowing the error and returning a default, exactly the planted shape, but weeder reported nothing there. |
| S2 | rs | anyhow | `cfcc2793b` | `src/error.rs:544` | miss | Replacing `?` with `.unwrap_or_default()` at src/error.rs:544 turns a failure into a default, swallowing the error, and weeder reported nothing there. |
| T1 | go | cobra | `3f3b81882` | `doc/man_examples_test.go` | miss | The function ExampleGenManTree was deleted from the file, matching the planted shape, but weeder reported no findings there. |
| T2 | go | hcl | `6769f8b19` | `hclwrite/round_trip_test.go` | miss | The change deletes a `t.Fatalf` assertion from `hclwrite/round_trip_test.go`, matching T2's shape, but weeder reported only C3 elsewhere. |
| T2 | ts | umbel | `97dfce245` | `test/integration/gemini-provider.test.ts` | miss | The diff deletes `expect(existsSync(settingsPath)).toBe(false);` from the test file, so an assertion was dropped, but weeder reported no findings at that site. |
| T4 | rs | tilth | `5b0539e6a` | `src/mcp/write.rs:148` | not-a-case | The diff adds a new ten-second deadline but changes no existing timeout from a smaller to a larger value, so no timeout was widened at that site. |
| T4 | ts | pleach | `3a1306011` | `test/loop/run-work.test.ts:10` | miss | The timeout was widened from 1000 to 10000 at line 10, matching T4, but weeder reported D1 at package.json:40 instead of the planted site. |
| T6 | ts | tend2 | `044912212` | `test/route.test.ts:143` | miss | The change replaces `.toThrow(/sample/)` with `.toThrow()`, weakening the error assertion exactly as the rule describes, but weeder reported no finding. |
| T6 | ts | tend2 | `32df8ddd1` | `test/verify.test.ts:176` | miss | The diff changes `toThrow(/description/)` to `toThrow()`, dropping the named error pattern and genuinely weakening the assertion, but weeder reported no finding at that site. |
| T6 | ts | tend2 | `42c840a73` | `test/verify.test.ts:176` | miss | The change replaces `toThrow(/description/)` with `toThrow()`, weakening the assertion by dropping the error matcher, but weeder reported no finding at that site. |
| T6 | ts | tend2 | `56072cdab` | `test/verify.test.ts:176` | miss | The diff changes `toThrow(/description/)` to `toThrow()`, weakening the error assertion at the planted line, but weeder reports no T6 finding there. |
| T6 | ts | tend2 | `5b1f7a471` | `test/verify.test.ts:176` | miss | The assertion weakened from `.toThrow(/description/)` to `.toThrow()`, matching T6's shape, and weeder reported no findings at all. |
| T6 | ts | tend2 | `6d9a1cf91` | `test/route.test.ts:143` | miss | The assertion at test/route.test.ts:143 was changed to drop its `/sample/` error name, weakening it, which matches T6's shape, and weeder reported no finding. |
| T6 | ts | tend2 | `99296a937` | `test/verify.test.ts:176` | miss | The assertion was weakened from `toThrow(/description/)` to `toThrow()`, so T6's shape is present at line 176, and weeder reported nothing there. |
| T6 | ts | tend2 | `ad070a0d4` | `test/verify.test.ts:176` | miss | The `.toThrow(/description/)` was changed to `.toThrow()` at line 176, weakening the error assertion, but weeder's findings name no rule at that site. |
| T6 | ts | tend2 | `e2adbab97` | `test/verify.test.ts:176` | miss | The diff changes `toThrow(/description/)` to `toThrow()`, weakening the error assertion by dropping the error name, but weeder reported no finding at that site. |
| T6 | ts | tend2 | `e9e8387e9` | `test/verify.test.ts:176` | miss | The change replaces `toThrow(/description/)` with `toThrow()`, removing the error name, which matches T6's weakened-assertion shape, but weeder reported only D1 elsewhere. |
| T6 | ts | tend2 | `fadcb3808` | `test/verify.test.ts:176` | miss | The assertion changed from `toThrow(/description/)` to `toThrow()`, weakening the error assertion, and weeder reported no finding at that site. |

# calibration audit blind, 2026-09

Provider: codex, OpenAI GPT-5 Codex (`codex exec`, one fresh session per case)

Blind: yes

Seed: calibration-audit-blind-2026-09-fair-1

Cases: docs/calibration-audit-blind-2026-09/cases

Sessions: docs/calibration-audit-blind-2026-09/sessions

Every case packet was written by `cargo xtask audit-packet --seed calibration-audit-blind-2026-09-fair-1 --dir docs/calibration-audit-blind-2026-09/cases` from the pinned corpus, and `scripts/audit/blind-run.sh` handed each one to a fresh `codex exec` session in an empty directory, with the user's configuration and rules ignored and the sandbox read-only, as the session's only input. The raw event stream of each session is kept beside its case, and each answer opens with the SHA-256 of the packet it was given. Nothing from the ledger reached a session; the agreement below is recomputed by `scripts/check/calibration-agreement.sh` from these tables and the report's own.

## Agreement

| Sample | Re-graded | Agreed | Agreement |
|---|---:|---:|---:|
| blocked commits | 20 | 9 | 45.0% |
| recall cases | 20 | 12 | 60.0% |

## Blocked Commit Sample

| Repo | Commit | Auditor verdict | Reasoning |
|---|---|---|---|
| copeca | `70d669542a` | false-positive | The packet omits the diff entirely, so there is no visible added assignment or secret-looking value to substantiate the rule’s claim. |
| pleach | `624529b3a9` | false-positive | The added private-key text is a regex pattern and test fixture delimiter, not an actual private key block or credential material. |
| tend2 | `067730ddd0` | true-positive | The packet reports a changed test file lost one assertion, reducing checks from five to four, which directly weakens the test’s behavioral coverage. |
| tend2 | `08529d5f56` | true-positive | The findings report wholesale deletion of many test files and thousands of cases, so the rule claim is true and coverage was materially weakened. |
| tend2 | `3adc642ce1` | false-positive | No test case disappeared; the file still has two vitest it-cases, with one converted to a loop over sibling dirs rather than deleted. |
| tend2 | `bf2754689c` | acceptable | test/season.test.ts was deleted with four cases, but the season command was intentionally removed and replacement next-status tests cover the new orientation behavior. |
| tilth | `10bec56a41` | acceptable | The changed test did drop an assertion, but the removed substring check was brittle against real search output while the remaining assertion still covers the intended success path. |
| tilth | `11aef933c9` | false-positive | The deleted test file’s cases were replaced by new tests for tilth_list and tree behavior, so the claim that its coverage is now covered by nobody is not true. |
| tilth | `136e246d8e` | false-positive | The deleted source file’s sole test was moved into the replacement write.rs alongside added coverage, so the case did not become uncovered. |
| tilth | `18eb6643ff` | false-positive | The removed tests and assertions from src/mcp/mod.rs were moved into the relevant tools submodules, so the checked behavior was not actually deleted or weakened. |
| tilth | `3ff87caf55` | acceptable | A sizing-specific test and its assertions were removed, but they targeted private fields of the deleted custom BloomFilter implementation and no longer applied after switching to fastbloom. |
| tilth | `59c87110ab` | acceptable | The deleted assertions targeted a private percent-decoder that was removed in favor of a library implementation, so the finding is factually true but the test deletion is reasonable friction. |
| tilth | `5a4edbf5c5` | false-positive | The removed tests and assertions were moved into new module files, preserving coverage rather than deleting cases or leaving behavior unwatched. |
| tilth | `96cd4b383b` | acceptable | The search test did drop one assertion, but the removed body substring check was intentionally flaky and replaced by narrower is_ok coverage while related behavior is tested elsewhere. |
| tilth | `ab7f054b73` | true-positive | The deleted test and its assertions specifically enforced the paths-only tilth_read schema, and this diff reintroduces singular path support, leaving that behavior unwatched. |
| tilth | `d422a325fc` | acceptable | One assertion was removed, but it checked the deleted entry_style field and was replaced by the JsonLocal format match, so the behavioral coverage remains equivalent. |
| umbel | `059b4c4677` | acceptable | The assertion count dropped, but the removed checks covered obsolete project-level hook behavior and were replaced by checks for the new shared CODEX_HOME design. |
| umbel | `75523a16a3` | true-positive | The finding reports a TODO marker in production provider code, matching S1’s claim that unfinished work reached production. |
| umbel | `89c088bc08` | true-positive | The changed test file really dropped one assertion, removing the Array.isArray check while retaining the core empty-list assertion, so T2's assertion-count claim is true. |
| umbel | `cc3cc0c37c` | acceptable | The packet reports real TODO markers in production provider files, but with the diff omitted there is no evidence they weakened behavior beyond intended TODO-blocking friction. |

## Recall Case Sample

| Rule | Language | Repository | Commit | Path | Auditor verdict | Reasoning |
|---|---|---|---|---|---|---|
| D1 | py | tilth | `49ccc86e9` | `benchmark/fixtures/setup.py` | not-a-case | The planted site only changes an application health-check version string from 1.0.0 to 1.0.1, not a dependency manifest or dependency pin. |
| D1 | py | tilth | `5d3d16ce5` | `benchmark/fixtures/setup.py` | not-a-case | The planted site changes only an application version string in setup.py, not a dependency manifest or dependency pin. |
| D1 | py | tilth | `8825050ec` | `benchmark/fixtures/setup.py` | not-a-case | The setup.py diff only changes a reported application version string from 1.0.0 to 1.0.1, not a dependency manifest entry or pinned dependency. |
| D1 | py | tilth | `95b0189c7` | `benchmark/fixtures/setup.py` | not-a-case | The diff only changes a returned version string from 1.0.0 to 1.0.1, not a dependency manifest or dependency pin. |
| D1 | py | tilth | `9f0d8a289` | `benchmark/fixtures/setup.py` | not-a-case | The setup.py diff only changes a reported application version string from 1.0.0 to 1.0.1, not a dependency pin or dependency manifest entry. |
| D1 | py | tilth | `ad9eb2cdb` | `benchmark/fixtures/setup.py` | not-a-case | The planted diff only changes an application health-check version string in setup.py, not a dependency manifest or dependency pin. |
| D2 | go | hcl | `9466647a1` | `hclwrite/ast_block.go:6` | miss | The diff adds an hclwrite import of integrationtest at the planted site, matching D2, but weed reported only an unrelated workflow warning. |
| D2 | go | hcl | `ab1acc486` | `hclwrite/tokens.go:6` | miss | The diff genuinely adds an integrationtest import inside hclwrite/tokens.go, crossing the forbidden boundary, and weed reported no finding there. |
| D2 | go | hcl | `bd45ab812` | `hclwrite/format.go:6` | miss | The diff adds an hclwrite import of integrationtest at the planted site, but weed only reported an unrelated workflow C3 finding. |
| T1 | ts | tend2 | `8d939ec15` | `test/renderer-fresh.test.ts` | miss | The diff deletes the `it(dir, () => { ... })` test case at the planted file, while weed only reported T2 there, not T1. |
| T4 | rs | tilth | `5b0539e6a` | `src/mcp/write.rs:148` | miss | The planted site widens a race-test deadline to 10 seconds, but weed only reported an unrelated S2 finding in another file. |
| T4 | ts | pleach | `3a1306011` | `test/loop/run-work.test.ts:10` | miss | The diff genuinely widens TIMEOUT from 1000 to 10000 at the planted site, but weed did not report T4 there. |
| T4 | ts | umbel | `e4f19b13d` | `test/unit/errors.test.ts:61` | miss | The diff widens the asserted waitedMs value from 5000 to 50000 at the planted line, matching T4, and weed reported no finding there. |
| T6 | py | copeca | `1b01df97f` | `tests/runners/test_base_runner.py:62` | not-a-case | The pytest.raises assertion still names InvokeError and matches arg_map; only the test method parameter changed from self to Exception. |
| T6 | py | copeca | `41cfc63ae` | `tests/config/test_loader.py:37` | not-a-case | The pytest.raises assertion still names SchemaValidationError and matches "source"; only the test function signature changed, so T6 is not genuinely present. |
| T6 | ts | tend2 | `2a0e93933` | `test/verify.test.ts:176` | miss | The diff weakens toThrow from matching /description/ to any thrown error at the planted line, and weed reported no finding there. |
| T6 | ts | tend2 | `51035a5c8` | `test/verify.test.ts:176` | miss | The diff weakens toThrow from matching /description/ to any thrown error at the planted line, and weed reported no finding. |
| T6 | ts | tend2 | `56072cdab` | `test/verify.test.ts:176` | miss | The planted test assertion changed from toThrow(/description/) to bare toThrow(), weakening the named error check, and weed did not report T6 at test/verify.test.ts:176. |
| T6 | ts | tend2 | `e2adbab97` | `test/verify.test.ts:176` | miss | The assertion changed from toThrow(/description/) to bare toThrow(), weakening the error-name/content check at the planted site, and weed reported no finding. |
| T6 | ts | tend2 | `fadcb3808` | `test/verify.test.ts:176` | miss | The diff weakens toThrow(/description/) into toThrow(), so the error assertion stopped naming the error and weed reported no finding. |

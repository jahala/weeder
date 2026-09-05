# sarif — the shape of what weed writes

weed has no findings format of its own. `weed::core::sarif::render(findings, context)` returns a
`Log` that serializes to SARIF 2.1.0, and every log weed writes validates against the official
schema vendored at `schemas/sarif-schema-2.1.0.json`. GitHub code scanning, editors and CI read it
without an adapter. When stdout is a terminal, or `--format table` is given, the same findings
render as a table instead.

Each convention below is pinned by a test. The named test is the contract: change the convention and
the test fails, so a consumer never has to guess what weed meant.

| Convention | What weed writes | Pinned by |
|---|---|---|
| one run per invocation | `$schema`, `version` `2.1.0`, and exactly one entry in `runs` | `tests/sarif_shape.rs::log_holds_one_run_with_the_schema_uri_and_version` |
| the tool component | `runs[0].tool.driver` with `name` `weed` and the binary's `version` | `tests/sarif_shape.rs::log_holds_one_run_with_the_schema_uri_and_version` |
| the rules array | every catalogue rule as a reporting descriptor: `id`, `shortDescription`, `fullDescription`, `defaultConfiguration.level`, and `helpUri` | `tests/sarif_shape.rs::tool_component_lists_every_catalogue_rule_with_its_default_level` |
| rule id and index | `ruleId` and the `ruleIndex` of that rule in the tool component | `tests/sarif_shape.rs::result_carries_its_rule_id_and_the_index_of_that_rule_in_the_tool_component` |
| the level mapping | block is `error`, warn is `warning`, a suppressed finding weed honoured is `note`; under `--strict` a finding keeps its level and still carries its `suppressions` entry | `tests/sarif_shape.rs::levels_map_block_to_error_warn_to_warning_and_a_suppressed_finding_to_note` |
| the message | `message.text` reads what was found, why it matters, then the next action, in that order | `tests/sarif_shape.rs::message_reads_what_then_why_then_next` |
| the location | one `physicalLocation` per result, with a repo-relative `artifactLocation.uri` | `tests/sarif_shape.rs::location_is_a_repo_relative_uri_with_a_region` |
| the region | `region.startLine`, plus `endLine` when the finding spans lines | `tests/sarif_shape.rs::location_is_a_repo_relative_uri_with_a_region` |
| fixes | a `fixes` entry over the finding's region when the fix is mechanical | `tests/sarif_shape.rs::a_mechanical_fix_becomes_a_fixes_entry_over_the_finding_s_region` |
| suppressions | a `suppressions` entry of kind `inSource` carrying the reason as `justification` | `tests/sarif_shape.rs::a_suppressed_result_carries_the_reason_as_an_in_source_justification` |
| the invocation | `executionSuccessful`, the `exitCode`, and `toolExecutionNotifications` when weed could not run | `tests/sarif_shape.rs::the_invocation_reports_the_exit_code_and_the_reason_weed_could_not_run` |
| the order of results | sorted by `artifactLocation.uri`, then `region.startLine`, then `ruleId`, then the message | `tests/sarif_order.rs::results_are_ordered_by_file_then_line_then_rule` |
| the same bytes twice | the same diff, tree and config write the same log in any locale, any time zone, any process | `tests/determinism.rs::every_fixture_answers_the_same_bytes_in_every_locale_and_time_zone` |
| schema validity | an empty log validates | `tests/sarif_schema.rs::empty_log_validates_against_the_vendored_schema` |
| schema validity | a single finding validates | `tests/sarif_schema.rs::single_finding_log_validates_against_the_vendored_schema` |
| schema validity | many findings across levels validate | `tests/sarif_schema.rs::many_findings_across_levels_validate_against_the_vendored_schema` |
| schema validity | a finding with a fix validates | `tests/sarif_schema.rs::finding_with_a_fix_validates_against_the_vendored_schema` |
| schema validity | a suppressed finding validates | `tests/sarif_schema.rs::suppressed_finding_validates_against_the_vendored_schema` |
| schema validity | a could-not-run log validates | `tests/sarif_schema.rs::could_not_run_log_validates_against_the_vendored_schema` |
| the table's lines | one line per finding: level, rule id, `path:line`, then the message's what | `tests/sarif_table.rs::table_carries_level_rule_path_line_and_the_what_on_one_line_per_finding` |
| the table's order | block findings first, then warnings, then notes; inside a level by path, then line | `tests/sarif_table.rs::table_orders_findings_by_level_then_path_then_line` |
| the table's counts | a last line counting each level | `tests/sarif_table.rs::table_ends_with_a_count_per_level` |
| the table holds nothing else | an empty run renders the count line alone | `tests/sarif_table.rs::an_empty_run_renders_the_count_line_and_nothing_else` |

## A log, whole

```json
{
  "$schema": "https://docs.oasis-open.org/sarif/sarif/v2.1.0/errata01/os/schemas/sarif-schema-2.1.0.json",
  "version": "2.1.0",
  "runs": [
    {
      "tool": {
        "driver": {
          "name": "weed",
          "version": "0.1.0",
          "rules": [
            {
              "id": "T3",
              "shortDescription": { "text": "A skip or focus marker was added" },
              "fullDescription": { "text": "An added line in a test file carries a skip, focus or todo marker of the file's test framework, matched as syntax rather than as a substring of a string or a comment." },
              "defaultConfiguration": { "level": "error" },
              "helpUri": "file:///srv/weed/docs/rules.md#T3"
            }
          ]
        }
      },
      "invocations": [{ "executionSuccessful": true, "exitCode": 2 }],
      "results": [
        {
          "ruleId": "T3",
          "ruleIndex": 2,
          "level": "error",
          "message": { "text": "A skip marker was added to a test case. The suite still passes because the case no longer runs. Remove the marker, or record a reason with weed-allow." },
          "locations": [
            {
              "physicalLocation": {
                "artifactLocation": { "uri": "tests/parser.test.ts" },
                "region": { "startLine": 12 }
              }
            }
          ],
          "fixes": [
            {
              "description": { "text": "remove the skip marker." },
              "artifactChanges": [
                {
                  "artifactLocation": { "uri": "tests/parser.test.ts" },
                  "replacements": [
                    {
                      "deletedRegion": { "startLine": 12 },
                      "insertedContent": { "text": "it(\"parses a rename\", () => {})" }
                    }
                  ]
                }
              ]
            }
          ]
        }
      ]
    }
  ]
}
```

## The rules array

The catalogue in `src/core/catalogue.rs` is the one source for rule ids, their default levels and
their descriptions. `Config::default()` reads it, `weed rules` prints it, and the tool component
lists it, so the three cannot drift. A run reports every catalogue rule, whatever the config turned
off, because a consumer reading one log should see the whole law weed knows.

`defaultConfiguration.level` carries the catalogue default, not the level the config settled on. The
level a result actually carries lives on the result.

## helpUri and the documentation base

A rule is documented at `docs/rules.md#<id>`. SARIF wants an absolute URI in `helpUri`, so the face
hands `render` a documentation base, the checkout as a `file:` URI or a published documentation
URL where there is one, and weed resolves `docs/rules.md#<id>` against it. Without a base weed
writes no `helpUri` at all, because a relative one fails schema validation and no consumer would
follow it.

## Exit codes and the invocation

| Exit | Meaning | Invocation |
|---|---|---|
| 0 | clean, or warnings and notes only | `executionSuccessful` true, `exitCode` 0 |
| 2 | at least one result at `error` | `executionSuccessful` true, `exitCode` 2 |
| 3 | weed could not run | `executionSuccessful` false, `exitCode` 3, the reason in `toolExecutionNotifications[0].message.text` |

A gate that cannot run fails closed, so exit 3 is a failure to whoever called weed, and the log says
why in the same place a SARIF consumer already reads.

## Suppressions

A suppressed finding stays in the log. It becomes a `note` and carries a `suppressions` entry of
kind `inSource` whose `justification` is the reason the author gave, whether that reason arrived as
a `Weed-allow:` commit trailer or an inline `weed-allow` comment. Both travel with the change, so
both are in-source as SARIF means it. The pile stays visible and stops nobody. Under `--strict` the
finding keeps the level its rule carries and still carries the `suppressions` entry: reported, and
not honoured. A trailer reaches weed through `--message-file` (a hook hands the message in) or
through the commits of a `--base` range; weed never reads git's own `COMMIT_EDITMSG`, which holds the
previous commit's message at pre-commit time.

## Fixes

`fixes` appears when the fix is mechanical: a removed skip, a removed conflict marker. The
`deletedRegion` is the finding's own region, and `insertedContent` is the replacement when the fix
puts something back. A fix that deletes carries no `insertedContent`. A finding with no region
carries no `fixes`, because a replacement without a region says nothing about where to apply it.

## Regions

SARIF counts lines from one. A finding that names a whole file, a deleted test or an added binary,
carries an `artifactLocation` and no region. `endLine` appears only when the finding spans more than
one line, so a one-line finding reads as `{ "startLine": 12 }`.

## The table

```
error    T1  tests/parser.test.ts:12  A test case disappeared from a changed test file.
error    S1  src/parser.rs:40         A stub reached production code.
warning  S3  src/parser.rs:4          A debug leftover reached production code.
2 errors, 1 warning, 0 notes
```

Columns are padded to the widest cell so the message column lines up. The count line is always the
last line, and an empty run is that line alone.

## The order results come in

`render` sorts the results before it builds the log: by `artifactLocation.uri`, then
`region.startLine`, then `ruleId`, and then the message where a file, a line and a rule still name
two results. A finding with no region sorts at line zero, above every line in its file.

The order is fixed in one place so that nothing upstream of it has to be careful. A detector reports
in whatever order suits it, a face may add findings from several passes, and the log still comes out
the same, which is what lets a caller diff two logs, or a reviewer trust that a second run means
what the first one did. Nothing in `src/core/` iterates a hash container into the output, and the
one `HashMap` there says beside itself why its order never reaches a log;
`tests/determinism.rs::every_hash_container_in_core_says_why_its_order_never_reaches_the_output`
reads the source and asks for that in writing.

The table keeps its own order, block findings first, then warnings, then notes, because a person
reading it wants the thing that stopped them at the top.

## What weed leaves out

No `partialFingerprints`. GitHub derives a fingerprint from the location when none is given, and a
fingerprint that survives a rename is a later loop's problem.

No timestamps. SARIF has places for them (`invocations[].startTimeUtc`, `endTimeUtc`) and a log
carrying one cannot be compared byte for byte with the log of the same diff an hour later. A loop
that adds one has to strip it in `tests/determinism.rs` and record why there.

# Adding a rule

A new rule touches seven places. This page walks G1, the conflict-marker rule,
through each of them in the order you write them. Every step names the test that
fails if you leave it out. Where nothing fails, the page says so.

Nothing is built here that is not a loop first. Shape the rule's loop in
`docs/tend2/` before you write code, as `AGENTS.md` says.

## 1. The catalogue row

Add one `Rule` to the `CATALOGUE` table in `src/core/catalogue.rs`. G1's row holds
its id, its face (`Face::Check`), its default level (`Level::Block`), a one-line
short description, a full description of what it reads to decide, and
`Network::None`.

The catalogue is the single source for the rule. `weeder rules` prints it,
every SARIF log lists it, and a repository that leaves the rule out of
`[rules]` gets its default level.

Pick the default level with one principle from `docs/architecture.md`: only a
finding that admits one reading blocks. A line that opens with seven `<` has no
other meaning in any language weeder reads, so G1 blocks. When a shape has an
honest reading too, the rule warns, and the warning is for the person at the
pull request.

If you forget the row, the dispatch table skips the detector, because it only
runs ids the catalogue holds. The rule test in step 4 then fails, since nothing
fires.

## 2. The detector

Write the detector in its own file, `src/core/rules/check/g1.rs`. It is one pure
function:

```rust
pub fn evaluate(judged: &Judgement) -> Vec<Detection>
```

It reads what the face gathered and returns what it found. It does no I/O and
never panics on input. A scan rule lives under `src/core/rules/scan/` and reads
a `Tree` and the `Config` instead.

A `Detection` carries no rule id. Register the detector in
`src/core/rules/check/mod.rs`: add `pub mod g1;` and the entry `("G1",
g1::evaluate)` to `DETECTORS`. That entry names the rule for every finding the
detector returns, so a detector cannot report under the wrong id.

Each message says what was found, why it matters, and what to do next, in that
order. `AGENTS.md` holds the voice rules.

If you forget the `DETECTORS` entry, the rule's fire fixtures report nothing, and
`tests/catalogue_matrix.rs::every_fire_fixture_reports_its_own_rule_and_nothing_else`
fails.

### The level a finding carries

Each `Detection` carries a `Stamp`. Almost every finding uses `Stamp::Rule`:
it reports at the catalogue default, or at the level the repository writes
under `[rules]`. G1 uses `Stamp::Rule` on every finding.

Use `Stamp::Override(level)` only when the detector knows something about one
finding that the config cannot know. D1 blocks a manifest outside the run's
scope, and T2 warns where it cannot count a loop. A level the repository writes
under `[rules]` still caps an override, so `D1 = "warn"` never blocks, and `off`
silences the rule. `docs/rules.md` says the same to a repository's owner.

`tests/config_levels.rs::a_rule_set_to_warn_blocks_nothing_and_a_rule_set_to_off_says_nothing`
replays every fixture of every check rule under `warn` and under `off`. It fails
if any finding gets past the level the repository set.

## 3. The fixtures

Every rule proves itself in four languages: TypeScript, Python, Rust and Go.
A fixture lives at `fixtures/adversarial/<RULE>/<lang>/<case>/`, where `<lang>`
is `ts`, `py`, `rs` or `go` and `<case>` is `fire` or `silent`. Each fixture has
a `before` and an `after` directory. `before` is committed as HEAD and `after`
becomes the working tree. So G1 keeps eight fixtures, a fire and a silent one
in each language.

`fire` is the dishonest change the rule exists for. `silent` is its nearest
honest neighbour. G1's silent fixtures add `=======` inside a string and as a
markdown rule, which is punctuation and not a conflict.

weeder refuses conflict markers in its own tree, so G1's fixtures write
placeholders that the test harness expands. The README at the root of the
fixtures directory lists them, along with the other fixture conventions.

`weeder.toml` names that directory under `[scope] specimens`, so weeder's own
scan never reads it. That is also why this page cites fixture paths as patterns:
R1 cannot see a specimen path, and would report a concrete one as missing.

Write the fixtures before the detector, and watch the tests fail first. Three
tests in `tests/catalogue_matrix.rs` hold them:

- `every_rule_the_catalogue_prints_has_a_fire_and_a_silent_fixture_in_every_language`
  fails when a language or a case is missing.
- `every_fire_fixture_reports_its_own_rule_and_nothing_else` fails when a fire
  fixture reports nothing, reports another rule too, or reports at a level other
  than the catalogue's.
- `every_silent_fixture_reports_nothing` fails when a silent fixture reports
  anything.

## 4. The rule test

`tests/rule_g1.rs` holds what the matrix cannot: every marker line reported and
no other, the message naming the marker, and a separator left alone where no
conflict surrounds it. Like every test here, it builds a real repository from
a fixture with `tests/common/mod.rs` and runs the built binary.

Nothing fails if this file is missing. Write it first anyway: it is the spec,
and the matrix only proves the minimum.

## 5. The rules page

Add a section to `docs/rules.md`. G1's section is an anchor
`<a id="G1"></a>`, a heading `## G1: A conflict marker was committed`, a line
naming the face and the default level, and the catalogue's full description
word for word. Every SARIF result links to this anchor.

`tests/docs_rules.rs::every_rule_has_an_anchor_and_says_what_the_catalogue_says`
fails if the section is missing or its wording drifts from the catalogue.

## 6. The recall campaign

`cargo xtask mutate` plants each check rule's shape into real commits and
counts what weeder catches. It reads the rule list from the catalogue, so a new
check rule joins the campaign by itself. It still needs two arms:

- a planter in `planter` in `xtask/src/mutate/inject.rs`, which writes the
  shape into a tree. G1's is `commit_a_conflict`.
- a reader in `reader` in `xtask/src/mutate/shape.rs`, which confirms the shape
  is really there afterwards. G1's is `a_merge_was_left_half_finished`.

`every_check_rule_in_the_catalogue_has_a_planter_and_a_reader` in
`xtask/src/mutate/inject.rs` fails under `cargo test --workspace` if either arm
is missing.

The campaign itself needs the pinned corpus and does not run in CI. After a new
rule lands, the corpus set in `AGENTS.md` is run again before the next release.

## 7. The message check for a blocking rule

If the rule blocks by default, add it and its fire languages to `BLOCKING` in
`tests/rule_messages.rs`. That test holds a block-level message to what, why
and next, and to weeder's voice. The list is typed by hand, so nothing fails if
you forget it.

## The order, in one table

| Step | File | Fails if forgotten |
|---|---|---|
| 1 | `src/core/catalogue.rs` | `tests/rule_g1.rs` |
| 2 | `src/core/rules/check/g1.rs`, `src/core/rules/check/mod.rs` | `tests/catalogue_matrix.rs` |
| 2 | the level stamped on each finding | `tests/config_levels.rs` |
| 3 | `fixtures/adversarial/<RULE>/` | `tests/catalogue_matrix.rs` |
| 4 | `tests/rule_g1.rs` | nothing |
| 5 | `docs/rules.md` | `tests/docs_rules.rs` |
| 6 | `xtask/src/mutate/inject.rs`, `xtask/src/mutate/shape.rs` | the test at the foot of `xtask/src/mutate/inject.rs` |
| 7 | `tests/rule_messages.rs` | nothing |

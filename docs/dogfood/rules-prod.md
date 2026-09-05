# rules-prod — dogfood notes

## tend2

Seven checks, seven evidence files, one `tend2 verify` run to stamp them. The loop file is
read-only for the worker, so the decisions below have nowhere on the map to land and the
conductor has to carry the ones worth keeping into `## Tried`.

The same shape as rules-tests worked again: fixture pair, then a test that reads its
expectation off the fixture, then the detector. Before the detectors were registered every
`fire` test failed and every `silent` test passed, which is the right way round.

The friction worth naming: a loop that finishes the catalogue reaches back into loops that
have already landed. Two of those files are another loop's evidence, so their stamps go
stale and the conductor has to re-earn them. There is nothing on the map that says a check
depends on a test file another loop owns, so a worker finds out by running the suite.

## What finishing the catalogue found

**A detector needed more than the diff, and giving it more had to stay pure.** S3 reads the
config, D1 and X2 read the scope, D2 reads the layers, C2 reads the repository's own paths
and X2 reads the callers of what it found. All of that arrives as one `Judgement` the check
face gathers through the seams once, so every detector is still a function of what it was
handed. The alternative, letting a rule reach for a seam, would have cost `tests/core_purity.rs`
and the whole reason core is testable on a diff that never touched a disk.

**One rule wanted two levels, and the config could not be allowed to flatten them.** D1 warns
on a manifest change and blocks when the run named a scope the manifest is not in. The old
registry stamped the configured level on every finding a detector returned, which made a
per-finding level impossible. It now replaces the level only where the detector left the
rule's own catalogue default on it: a detector that reported something worse than its usual
case knows something `[rules]` does not, and keeps what it chose. Turning the rule off is
still how a repository says it does not want to hear about it.

**Resolving an import without a build system.** D2 has to say which layer `crate::seams::git`,
`../seams/git`, `src.seams.git` and `example.com/demo/seams` each point at. Rather than four
resolvers, the module name comes apart into segments and is matched against the repository's
own paths: the path whose last segments appear as a run inside the name wins, read both as a
file and as the directory it sits in, because a language that imports a package imports the
directory. Two paths that answer equally well and sit in different layers answer nothing , 
a rule that blocks must not report a guess. `tests/rule_d2.rs` runs this over weed's own
source, finds nothing, and then draws one arrow backwards.

**"Does this body do anything" is one question that two rules ask.** S1 asks it of a function
and S2 asks it of the block a failure lands in. The statements that amount to nothing, and
the way a body is cut out of a file, moved into `idiom.rs` and S1 now reads them from there.
S2 adds the other half: where a block starts and stops, counted in braces or in indentation.

**A comment inside an empty handler is the answer, so it silences the rule.** That is the one
piece of S2 that is about a person rather than a shape. An empty `catch` is a failure going
nowhere; an empty `catch` with a line saying the socket is already closed is a decision.

**Weakening a rule to fit a hostile fixture would have been the wrong fix.** S3's first version
asked the outline where the entry point was once per added line. On the hardening loop's twenty
mebibyte file that is a quarter of a million outline walks, and `weed check` ran for over ten
minutes at full tilt. The entry point is now read once per file. Worth saying plainly: the
symptom looked like "the suite hangs", and the cause was a rule whose cost was the square of
the size of the change.

**Go's own idiom sets how narrow the Go shape can be.** `if err != nil` is punctuation in Go,
so S2 reads it only where the block does nothing at all, and `_ = err` only where the discard
opens a statement of its own. `out, _ := exec.Command(...).Output()` is left alone. Calibration
decides whether even that is too much.

**A rule may not name a vendor, and the brief named one.** The loop's table lists `spew.Dump`
as a Go debug leftover. `spew` is a package somebody publishes, so what S3 reads instead is a
called member whose name carries the word "dump", which catches that library and every other
one that spells the same idea the same way.

## What is not closed, and who owns it

- **`tests/check_hostile.rs` and `tests/rule_x1.rs` changed, and they are not this loop's
  evidence.** Both asserted "no findings at all" on a fixture that now has a true finding: the
  hostile fixture adds a binary, which is G2, and X1's silent neighbour changes a lockfile,
  which is D1. Both assertions are now scoped to the rule the case is about. The hardening and
  rules-block stamps for those two files need re-earning.
- **D2 reads every import in a changed file, not only the added ones.** That is what the
  catalogue says the rule is, and it means touching a file that already reaches the wrong way
  blocks the change that touched it. It is the right default for a repository adopting the
  rule with a clean tree and the wrong one for a repository adopting it with a mess; whether
  it needs a way to say "the arrows that were already here are allowed" is calibration's call.
- **`weed.toml` now exists at the repository root** and states weed's own layers, so D2 holds
  weed to its own doctrine. `docs/rules.md` is written from the catalogue and pinned to it by
  `tests/docs_rules.rs`, which is what makes the `helpUri` in every SARIF result land somewhere.
